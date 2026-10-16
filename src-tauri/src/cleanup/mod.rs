//! Clean up: moves the contents of safe-to-clear caches to the Recycle Bin.

pub mod recycle;
pub mod rules;
pub mod running;
pub mod safety;
pub mod targets;

use crate::scanner::{file_rec, scan_dir};
use crate::system_data::catalog::{Group, ItemDef};
use crate::system_data::expand::Lookup;
use crate::system_data::measure::totals;
use crate::system_data::{ItemState, ItemView, SystemData};
use crate::win;
use rules::Rule;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

/// Lists the running processes' executable names (lowercase).
pub type Running = dyn Fn() -> HashSet<String> + Send + Sync;
/// Removes paths and returns the ones still on disk.
pub type Recycler = fn(&[PathBuf]) -> Vec<PathBuf>;

/// One row of the checklist.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanItem {
    pub id: &'static str,
    pub group: Group,
    pub state: ItemState,
    pub alloc: u64,
    pub files: u64,
    pub default_on: bool,
    /// The app to close first, if it is running.
    pub app_open: Option<&'static str>,
    /// Folders (or files) whose contents would be moved.
    pub roots: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupStatus {
    pub running: bool,
    pub finished: bool,
    pub total: usize,
    pub done: usize,
    pub current: Option<String>,
    /// Bytes moved to the Recycle Bin.
    pub moved: u64,
    /// Entries still on disk afterwards, usually because they were in use.
    pub in_use: u64,
    pub in_use_examples: Vec<String>,
    /// Items skipped because their app was open.
    pub apps_open: Vec<String>,
    /// Folders the safety check refused.
    pub refused: Vec<String>,
}

const MAX_EXAMPLES: usize = 5;

pub struct Cleanup {
    catalog: &'static [ItemDef],
    rules: &'static [Rule],
    lookup: Arc<Lookup>,
    running: Arc<Running>,
    recycler: Recycler,
    status: Arc<Mutex<CleanupStatus>>,
}

impl Cleanup {
    pub fn new(
        catalog: &'static [ItemDef],
        rules: &'static [Rule],
        lookup: Arc<Lookup>,
        running: Arc<Running>,
        recycler: Recycler,
    ) -> Cleanup {
        Cleanup { catalog, rules, lookup, running, recycler, status: Arc::default() }
    }

    /// The rows worth offering, from System Data's latest measurements.
    pub fn plan(&self, measured: &[ItemView]) -> Vec<CleanItem> {
        let running = (self.running)();
        self.rules
            .iter()
            .filter_map(|rule| {
                let row = measured.iter().find(|r| r.id == rule.id)?;
                let def = self.def(rule.id)?;
                let m = &row.measured;
                let worth = match m.state {
                    ItemState::Pending | ItemState::NeedsAdmin => true,
                    ItemState::Missing => false,
                    ItemState::Ok | ItemState::Partial => m.alloc > 0,
                };
                worth.then(|| CleanItem {
                    id: rule.id,
                    group: row.group,
                    state: m.state,
                    alloc: m.alloc,
                    files: m.files,
                    default_on: rule.default_on,
                    app_open: rule.open_app(&running),
                    roots: targets::roots(def, &*self.lookup).iter().map(|p| p.display().to_string()).collect(),
                })
            })
            .collect()
    }

    /// Cleans the given items in the background; follow it with [`Cleanup::status`].
    /// `quiet` gets the folders about to be emptied; its guard is held until the job ends.
    pub fn start<G: Send + 'static>(&self, ids: Vec<String>, quiet: impl FnOnce(Vec<PathBuf>) -> G) -> Result<(), String> {
        let mut status = self.status.lock().unwrap();
        if status.running {
            return Err("A clean-up is already running.".into());
        }
        let jobs: Vec<(&'static Rule, &'static ItemDef)> = ids
            .iter()
            .filter_map(|id| {
                let rule = self.rules.iter().find(|r| r.id == id)?;
                Some((rule, self.def(rule.id)?))
            })
            .collect();
        *status = CleanupStatus { running: true, total: jobs.len(), ..Default::default() };
        drop(status);

        let (shared, lookup, running, recycler) = (self.status.clone(), self.lookup.clone(), self.running.clone(), self.recycler);
        let guard = quiet(jobs.iter().flat_map(|(_, def)| targets::roots(def, &*lookup)).collect());
        thread::spawn(move || {
            let _quiet = guard;
            for (rule, def) in jobs {
                clean_item(rule, def, &*lookup, &*running, recycler, &shared);
                shared.lock().unwrap().done += 1;
            }
            let mut status = shared.lock().unwrap();
            status.running = false;
            status.finished = true;
            status.current = None;
        });
        Ok(())
    }

    pub fn status(&self) -> CleanupStatus {
        self.status.lock().unwrap().clone()
    }

    fn def(&self, id: &str) -> Option<&'static ItemDef> {
        self.catalog.iter().find(|d| d.id == id)
    }
}

fn clean_item(rule: &Rule, def: &ItemDef, lookup: &Lookup, running: &Running, recycler: Recycler, status: &Mutex<CleanupStatus>) {
    // Checked again: the app may have started since the plan was shown.
    if rule.open_app(&running()).is_some() {
        status.lock().unwrap().apps_open.push(rule.id.to_string());
        return;
    }
    for root in targets::roots(def, lookup) {
        if safety::check(&root, lookup).is_err() {
            status.lock().unwrap().refused.push(root.display().to_string());
            continue;
        }
        status.lock().unwrap().current = Some(root.display().to_string());
        let cluster = win::cluster_size(&root);
        let entries = targets::entries(&root);
        let sizes: Vec<u64> = entries.iter().map(|e| size_on_disk(e, cluster)).collect();
        let left: HashSet<PathBuf> = recycler(&entries).into_iter().collect();
        let mut status = status.lock().unwrap();
        for (entry, size) in entries.iter().zip(sizes) {
            if left.contains(entry) {
                status.in_use += 1;
                if status.in_use_examples.len() < MAX_EXAMPLES {
                    status.in_use_examples.push(entry.display().to_string());
                }
            } else {
                status.moved += size;
            }
        }
    }
}

fn size_on_disk(path: &Path, cluster: u64) -> u64 {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => totals(&scan_dir(path, cluster)).0,
        Ok(meta) => file_rec(String::new(), &meta, path, cluster).alloc,
        Err(_) => 0,
    }
}

#[tauri::command]
pub fn cleanup_plan(system: tauri::State<'_, SystemData>, cleanup: tauri::State<'_, Cleanup>) -> Vec<CleanItem> {
    cleanup.plan(&system.get(false))
}

#[tauri::command]
pub fn cleanup_start(
    session: tauri::State<'_, crate::session::Session>,
    cleanup: tauri::State<'_, Cleanup>,
    ids: Vec<String>,
) -> Result<(), String> {
    cleanup.start(ids, |folders| session.quiet(folders))
}

#[tauri::command]
pub fn cleanup_status(cleanup: tauri::State<'_, Cleanup>) -> CleanupStatus {
    cleanup.status()
}

#[tauri::command]
pub fn open_recycle_bin() -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg("shell:RecycleBinFolder")
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::rules::App;
    use super::*;
    use crate::system_data::catalog::{Probe, Verdict};
    use crate::system_data::Measured;
    use std::time::{Duration, Instant};

    const NOTEPAD: App = App { name: "Notepad", exes: &["notepad.exe"] };

    const fn def(id: &'static str, path: &'static [&'static str]) -> ItemDef {
        ItemDef { id, group: Group::AppCaches, verdict: Verdict::SafeToClear, probe: Probe::Paths(path) }
    }

    static CATALOG: &[ItemDef] = &[
        def("cache", &[r"%ROOT%\cache"]),
        def("app", &[r"%ROOT%\app"]),
        def("danger", &[r"%USERPROFILE%"]),
        def("gone", &[r"%ROOT%\gone"]),
    ];

    static RULES: &[Rule] = &[
        Rule { id: "cache", default_on: true, apps: &[] },
        Rule { id: "app", default_on: true, apps: &[NOTEPAD] },
        Rule { id: "danger", default_on: false, apps: &[] },
        Rule { id: "gone", default_on: true, apps: &[] },
    ];

    fn delete(paths: &[PathBuf]) -> Vec<PathBuf> {
        for p in paths {
            let _ = if p.is_dir() { fs::remove_dir_all(p) } else { fs::remove_file(p) };
        }
        paths.iter().filter(|p| p.exists()).cloned().collect()
    }

    fn keep(paths: &[PathBuf]) -> Vec<PathBuf> {
        paths.to_vec()
    }

    fn write(path: &Path, len: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![0u8; len]).unwrap();
    }

    fn setup(recycler: Recycler) -> (tempfile::TempDir, Cleanup) {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("cache").join("a.bin"), 5000);
        write(&dir.path().join("cache").join("sub").join("b.bin"), 100);
        write(&dir.path().join("app").join("c.bin"), 10);
        write(&dir.path().join("profile").join("keep.txt"), 10);
        let root = dir.path().to_string_lossy().into_owned();
        let lookup = move |name: &str| match name {
            "ROOT" => Some(root.clone()),
            "USERPROFILE" => Some(format!(r"{root}\profile")),
            _ => None,
        };
        let running = || HashSet::from(["notepad.exe".to_string()]);
        let cleanup = Cleanup::new(CATALOG, RULES, Arc::new(lookup), Arc::new(running), recycler);
        (dir, cleanup)
    }

    fn run(cleanup: &Cleanup, ids: &[&str]) -> CleanupStatus {
        cleanup.start(ids.iter().map(|s| s.to_string()).collect(), |_| ()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let status = cleanup.status();
            if status.finished {
                return status;
            }
            assert!(Instant::now() < deadline, "clean-up timed out");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn empties_folders_but_keeps_them() {
        let (dir, cleanup) = setup(delete);
        let status = run(&cleanup, &["cache"]);
        let cache = dir.path().join("cache");
        assert!(cache.is_dir());
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
        assert!(status.moved >= 5100);
        assert_eq!((status.done, status.total, status.in_use), (1, 1, 0));
        assert!(!status.running);
    }

    #[test]
    fn skips_items_whose_app_is_open() {
        let (dir, cleanup) = setup(delete);
        let status = run(&cleanup, &["app"]);
        assert!(dir.path().join("app").join("c.bin").exists());
        assert_eq!(status.apps_open, vec!["app"]);
    }

    #[test]
    fn refuses_protected_folders() {
        let (dir, cleanup) = setup(delete);
        let status = run(&cleanup, &["danger"]);
        assert!(dir.path().join("profile").join("keep.txt").exists());
        assert_eq!(status.refused.len(), 1);
        assert_eq!(status.moved, 0);
    }

    #[test]
    fn reports_what_stayed_on_disk() {
        let (_dir, cleanup) = setup(keep);
        let status = run(&cleanup, &["cache"]);
        assert_eq!(status.in_use, 2);
        assert_eq!(status.in_use_examples.len(), 2);
        assert_eq!(status.moved, 0);
    }

    /// Ignored by default: it puts two tiny test entries in the user's Recycle Bin.
    #[test]
    #[ignore]
    fn cleans_through_the_real_recycle_bin() {
        let (dir, cleanup) = setup(recycle::recycle);
        let status = run(&cleanup, &["cache"]);
        assert!(dir.path().join("cache").is_dir());
        assert_eq!(fs::read_dir(dir.path().join("cache")).unwrap().count(), 0);
        assert_eq!(status.in_use, 0);
        assert!(status.moved >= 5100);
    }

    #[test]
    fn plan_offers_rows_worth_cleaning() {
        let (dir, cleanup) = setup(delete);
        let row = |id: &'static str, state: ItemState, alloc: u64| ItemView {
            id,
            group: Group::AppCaches,
            verdict: Verdict::SafeToClear,
            measured: Measured { alloc, ..Measured::with_state(state) },
        };
        let measured = vec![
            row("cache", ItemState::Ok, 5100),
            row("app", ItemState::Partial, 10),
            row("danger", ItemState::Ok, 0),
            row("gone", ItemState::Missing, 0),
        ];
        let plan = cleanup.plan(&measured);
        let ids: Vec<_> = plan.iter().map(|i| i.id).collect();
        assert_eq!(ids, ["cache", "app"]);
        assert_eq!(plan[1].app_open, Some("Notepad"));
        assert_eq!(plan[0].roots, vec![dir.path().join("cache").display().to_string()]);
    }
}
