//! System Data: what Windows and common apps keep on disk, measured directly.

pub mod catalog;
pub mod expand;
pub(crate) mod measure;
pub mod probe;
mod windows;

use catalog::{Group, ItemDef, Verdict};
use expand::Lookup;
use probe::Ctx;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemState {
    Pending,
    Ok,
    /// Some folders below could not be read; the real size may be larger.
    Partial,
    NeedsAdmin,
    Missing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Note {
    SharedWithWindows,
    MeasuringWithDism,
    MeasuredByDism,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measured {
    pub state: ItemState,
    pub alloc: u64,
    pub files: u64,
    /// What *Open in Explore* and *Reveal* point at.
    pub path: Option<String>,
    pub is_dir: bool,
    pub note: Option<Note>,
}

impl Measured {
    pub fn with_state(state: ItemState) -> Measured {
        Measured { state, alloc: 0, files: 0, path: None, is_dir: false, note: None }
    }
}

/// One row as the UI sees it.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    pub id: &'static str,
    pub group: Group,
    pub verdict: Verdict,
    #[serde(flatten)]
    pub measured: Measured,
}

/// Measures the catalogue in the background and keeps the latest results.
pub struct SystemData {
    catalog: &'static [ItemDef],
    lookup: Arc<Lookup>,
    elevated: bool,
    items: Arc<Mutex<Vec<ItemView>>>,
    /// Bumped by every measurement; results from older ones are dropped.
    run: Arc<AtomicU64>,
}

impl SystemData {
    pub fn new(catalog: &'static [ItemDef], lookup: Arc<Lookup>, elevated: bool) -> SystemData {
        SystemData { catalog, lookup, elevated, items: Arc::default(), run: Arc::default() }
    }

    /// The current rows. Starts measuring when nothing was measured yet or `refresh` is set.
    pub fn get(&self, refresh: bool) -> Vec<ItemView> {
        let mut items = self.items.lock().unwrap();
        if items.is_empty() || refresh {
            *items = self.start();
        }
        items.clone()
    }

    /// Spawns one thread per item and returns the rows, all pending. The caller
    /// holds the lock, so no result lands before the rows are in place.
    fn start(&self) -> Vec<ItemView> {
        let run = self.run.fetch_add(1, Ordering::SeqCst) + 1;
        for (slot, def) in self.catalog.iter().enumerate() {
            let (items, current, lookup, elevated) = (self.items.clone(), self.run.clone(), self.lookup.clone(), self.elevated);
            thread::spawn(move || {
                let ctx = Ctx { lookup: &*lookup, elevated };
                probe::run(&def.probe, &ctx, &mut |measured| {
                    let mut items = items.lock().unwrap();
                    if current.load(Ordering::SeqCst) == run {
                        if let Some(item) = items.get_mut(slot) {
                            item.measured = measured;
                        }
                    }
                });
            });
        }
        self.catalog
            .iter()
            .map(|def| ItemView {
                id: def.id,
                group: def.group,
                verdict: def.verdict,
                measured: Measured::with_state(ItemState::Pending),
            })
            .collect()
    }
}

#[tauri::command]
pub fn system_data(state: tauri::State<'_, SystemData>, refresh: bool) -> Vec<ItemView> {
    state.get(refresh)
}

#[cfg(test)]
mod tests {
    use super::catalog::Probe;
    use super::*;
    use std::fs;
    use std::time::{Duration, Instant};

    static TEST_CATALOG: &[ItemDef] = &[
        ItemDef { id: "cache", group: Group::Developer, verdict: Verdict::SafeToClear, probe: Probe::Paths(&[r"%ROOT%\cache"]) },
        ItemDef { id: "gone", group: Group::TempLogs, verdict: Verdict::SafeToClear, probe: Probe::Paths(&[r"%ROOT%\gone"]) },
    ];

    fn wait_until_measured(data: &SystemData) -> Vec<ItemView> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let items = data.get(false);
            if items.iter().all(|i| i.measured.state != ItemState::Pending) {
                return items;
            }
            assert!(Instant::now() < deadline, "measuring timed out");
            thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn measures_every_item_in_the_background() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("cache")).unwrap();
        fs::write(dir.path().join("cache").join("f.bin"), vec![0u8; 3000]).unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let data = SystemData::new(TEST_CATALOG, Arc::new(move |name: &str| (name == "ROOT").then(|| root.clone())), false);

        assert!(data.get(false).iter().all(|i| i.measured.state == ItemState::Pending));
        let items = wait_until_measured(&data);
        assert_eq!(items[0].measured.state, ItemState::Ok);
        assert_eq!(items[0].measured.files, 1);
        assert_eq!(items[1].measured.state, ItemState::Missing);

        assert!(data.get(true).iter().all(|i| i.measured.state == ItemState::Pending));
        assert_eq!(wait_until_measured(&data)[0].measured.files, 1);
    }

    #[test]
    fn rows_serialize_flat_for_the_ui() {
        let row = ItemView {
            id: "npmCache",
            group: Group::Developer,
            verdict: Verdict::SafeToClear,
            measured: Measured::with_state(ItemState::NeedsAdmin),
        };
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["group"], "developer");
        assert_eq!(json["verdict"], "safeToClear");
        assert_eq!(json["state"], "needsAdmin");
        assert_eq!(json["isDir"], false);
    }
}
