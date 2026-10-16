//! Apps: installed programs and Store apps, their footprint, and uninstalling them.

pub mod leftovers;
pub mod names;
pub mod registry;
pub mod store;
pub mod uninstall;

use crate::cleanup::{safety, Recycler};
use crate::scanner::{file_rec, scan_dir};
use crate::system_data::expand::{expand, Lookup};
use crate::system_data::measure::totals;
use crate::win;
use leftovers::Identity;
use rayon::prelude::*;
use registry::Program;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use store::Package;

const POLL: Duration = Duration::from_secs(2);
/// Stop waiting for an uninstaller after this long.
const GIVE_UP: Duration = Duration::from_secs(30 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AppKind {
    Program,
    Store,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderSize {
    pub path: String,
    pub alloc: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppView {
    pub id: String,
    pub kind: AppKind,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub install_location: Option<String>,
    pub install_size: u64,
    /// The install size is the registry's estimate, not a measurement.
    pub estimated: bool,
    pub data_folders: Vec<FolderSize>,
    pub data_size: u64,
    pub measured: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppsList {
    /// Still discovering apps; more rows may appear.
    pub loading: bool,
    pub apps: Vec<AppView>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UninstallPhase {
    Running,
    Done,
    Failed,
    /// The user stopped waiting, or the uninstaller took too long.
    Stopped,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallStatus {
    pub phase: UninstallPhase,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecycleReport {
    pub moved: u64,
    /// Still on disk afterwards (in use).
    pub left: Vec<String>,
    /// Not offered by Disklens, or refused by the safety check.
    pub refused: Vec<String>,
}

#[derive(Clone)]
enum Source {
    Program(Program),
    Store(Package),
}

#[derive(Clone)]
struct Entry {
    view: AppView,
    source: Source,
}

#[derive(Default)]
struct State {
    started: bool,
    loading: bool,
    entries: Vec<Entry>,
    /// Apps uninstalled this session, kept to find their leftovers.
    uninstalled: HashMap<String, Entry>,
}

pub struct Apps {
    lookup: Arc<Lookup>,
    recycler: Recycler,
    state: Arc<Mutex<State>>,
    run: Arc<AtomicU64>,
    jobs: Arc<Mutex<HashMap<String, UninstallStatus>>>,
    stop: Arc<Mutex<HashSet<String>>>,
    /// Leftover paths shown to the user (lowercased); only these can be recycled.
    offered: Mutex<HashSet<PathBuf>>,
}

impl Apps {
    pub fn new(lookup: Arc<Lookup>, recycler: Recycler) -> Apps {
        Apps {
            lookup,
            recycler,
            state: Arc::default(),
            run: Arc::default(),
            jobs: Arc::default(),
            stop: Arc::default(),
            offered: Mutex::default(),
        }
    }

    /// The current list. Discovers and measures in the background the first time, or when `refresh` is set.
    pub fn list(&self, refresh: bool) -> AppsList {
        let mut state = self.state.lock().unwrap();
        if !state.started || refresh {
            state.started = true;
            state.loading = true;
            state.entries.clear();
            let run = self.run.fetch_add(1, Ordering::SeqCst) + 1;
            let (shared, current, lookup) = (self.state.clone(), self.run.clone(), self.lookup.clone());
            thread::spawn(move || discover_and_measure(run, &shared, &current, &*lookup));
        }
        AppsList { loading: state.loading, apps: state.entries.iter().map(|e| e.view.clone()).collect() }
    }

    /// Starts uninstalling `id`; follow it with [`Apps::uninstall_status`].
    pub fn uninstall(&self, id: &str) -> Result<(), String> {
        let entry = {
            let state = self.state.lock().unwrap();
            state.entries.iter().find(|e| e.view.id == id).cloned().ok_or("This app is no longer listed.")?
        };
        {
            let mut jobs = self.jobs.lock().unwrap();
            if jobs.get(id).is_some_and(|j| j.phase == UninstallPhase::Running) {
                return Err("Already uninstalling.".into());
            }
            jobs.insert(id.to_string(), UninstallStatus { phase: UninstallPhase::Running, message: None });
        }
        self.stop.lock().unwrap().remove(id);
        let (state, jobs, stop, id) = (self.state.clone(), self.jobs.clone(), self.stop.clone(), id.to_string());
        thread::spawn(move || {
            let result = match &entry.source {
                Source::Program(program) => wait_for_uninstaller(program, &id, &stop),
                Source::Store(package) => store::remove(&package.full_name).map(|_| UninstallPhase::Done),
            };
            let status = match result {
                Ok(phase) => UninstallStatus { phase, message: None },
                Err(message) => UninstallStatus { phase: UninstallPhase::Failed, message: Some(message) },
            };
            if status.phase == UninstallPhase::Done {
                let mut state = state.lock().unwrap();
                state.entries.retain(|e| e.view.id != id);
                state.uninstalled.insert(id.clone(), entry);
            }
            jobs.lock().unwrap().insert(id, status);
        });
        Ok(())
    }

    pub fn uninstall_status(&self, id: &str) -> Option<UninstallStatus> {
        self.jobs.lock().unwrap().get(id).cloned()
    }

    pub fn stop_waiting(&self, id: &str) {
        self.stop.lock().unwrap().insert(id.to_string());
    }

    /// What an uninstalled program left behind: its install folder if still
    /// there, and its data folders. Store apps leave nothing.
    pub fn leftovers(&self, id: &str) -> Vec<FolderSize> {
        let (entry, others) = {
            let state = self.state.lock().unwrap();
            let others: Vec<PathBuf> = state.entries.iter().filter_map(|e| e.view.install_location.as_ref().map(PathBuf::from)).collect();
            (state.uninstalled.get(id).cloned(), others)
        };
        let Some(Entry { source: Source::Program(program), .. }) = entry else { return Vec::new() };
        let install = program
            .install_location
            .as_deref()
            .map(PathBuf::from)
            .filter(|p| safety::check(p, &*self.lookup).is_ok())
            .filter(|p| !others.iter().any(|o| lower(o).starts_with(lower(p))));
        let identity = Identity { name: &program.name, publisher: program.publisher.as_deref(), install_location: install.as_deref() };
        let mut paths: Vec<PathBuf> = install.iter().filter(|p| p.exists()).cloned().collect();
        paths.extend(leftovers::data_folders(&identity, &*self.lookup, &others));
        let found: Vec<FolderSize> = paths.iter().map(|p| FolderSize { path: p.display().to_string(), alloc: size_on_disk(p) }).collect();
        self.offered.lock().unwrap().extend(paths.iter().map(|p| lower(p)));
        found
    }

    /// Moves offered leftovers to the Recycle Bin. Anything not offered, or
    /// refused by the safety check, is left alone.
    pub fn recycle(&self, paths: &[String]) -> RecycleReport {
        let offered = self.offered.lock().unwrap().clone();
        let mut report = RecycleReport::default();
        let mut allowed = Vec::new();
        for path in paths.iter().map(PathBuf::from) {
            if offered.contains(&lower(&path)) && safety::check(&path, &*self.lookup).is_ok() {
                allowed.push(path);
            } else {
                report.refused.push(path.display().to_string());
            }
        }
        let sizes: Vec<u64> = allowed.iter().map(|p| size_on_disk(p)).collect();
        let left: HashSet<PathBuf> = (self.recycler)(&allowed).into_iter().collect();
        for (path, size) in allowed.iter().zip(sizes) {
            if left.contains(path) {
                report.left.push(path.display().to_string());
            } else {
                report.moved += size;
            }
        }
        report
    }
}

/// Runs the uninstaller, then waits for its registry entry to disappear.
fn wait_for_uninstaller(program: &Program, id: &str, stop: &Mutex<HashSet<String>>) -> Result<UninstallPhase, String> {
    uninstall::launch(&uninstall::command_for(program))?;
    let started = Instant::now();
    loop {
        thread::sleep(POLL);
        if !registry::exists(&program.id) {
            return Ok(UninstallPhase::Done);
        }
        if stop.lock().unwrap().contains(id) || started.elapsed() > GIVE_UP {
            return Ok(UninstallPhase::Stopped);
        }
    }
}

fn discover_and_measure(run: u64, shared: &Mutex<State>, current: &AtomicU64, lookup: &Lookup) {
    let publish = |entries: Vec<Entry>, loading: bool| {
        let mut state = shared.lock().unwrap();
        if current.load(Ordering::SeqCst) == run {
            state.entries.extend(entries);
            state.loading = loading;
        }
    };
    publish(registry::read_all().into_iter().map(|p| program_entry(p, lookup)).collect(), true);
    publish(store::list().into_iter().map(store_entry).collect(), false);

    let entries: Vec<Entry> = {
        let mut state = shared.lock().unwrap();
        drop_shared_installs(&mut state.entries);
        state.entries.clone()
    };
    let installs: Vec<PathBuf> = entries.iter().filter_map(|e| e.view.install_location.as_ref().map(PathBuf::from)).collect();
    entries.par_iter().for_each(|entry| {
        let measured = measure(entry, lookup, &installs);
        let mut state = shared.lock().unwrap();
        if current.load(Ordering::SeqCst) != run {
            return;
        }
        if let Some(e) = state.entries.iter_mut().find(|e| e.view.id == entry.view.id) {
            e.view = measured;
        }
    });
}

/// Several apps sometimes register one parent folder (e.g. all of Adobe) as
/// their install location. A location that equals or contains another app's
/// would count, and later offer to clean, the other apps too: drop it.
fn drop_shared_installs(entries: &mut [Entry]) {
    let installs: Vec<(String, PathBuf)> = entries
        .iter()
        .filter_map(|e| e.view.install_location.as_ref().map(|l| (e.view.id.clone(), lower(Path::new(l)))))
        .collect();
    for entry in entries.iter_mut() {
        let Some(own) = entry.view.install_location.as_ref().map(|l| lower(Path::new(l))) else { continue };
        if installs.iter().any(|(id, other)| *id != entry.view.id && other.starts_with(&own)) {
            entry.view.install_location = None;
        }
    }
}

fn program_entry(program: Program, lookup: &Lookup) -> Entry {
    // An install location like C:\ or C:\Program Files would count (or clean) far too much.
    let location = program.install_location.clone().filter(|l| safety::check(Path::new(l), lookup).is_ok());
    let view = AppView {
        id: program.id.clone(),
        kind: AppKind::Program,
        name: program.name.clone(),
        publisher: program.publisher.clone(),
        version: program.version.clone(),
        install_location: location,
        install_size: 0,
        estimated: false,
        data_folders: Vec::new(),
        data_size: 0,
        measured: false,
    };
    Entry { view, source: Source::Program(program) }
}

fn store_entry(package: Package) -> Entry {
    let view = AppView {
        id: format!("store:{}", package.full_name),
        kind: AppKind::Store,
        name: package.name.clone(),
        publisher: package.publisher_name(),
        version: package.version.clone(),
        install_location: package.location.clone(),
        install_size: 0,
        estimated: false,
        data_folders: Vec::new(),
        data_size: 0,
        measured: false,
    };
    Entry { view, source: Source::Store(package) }
}

fn measure(entry: &Entry, lookup: &Lookup, installs: &[PathBuf]) -> AppView {
    let mut view = entry.view.clone();
    let install = view.install_location.as_deref().map(PathBuf::from).filter(|p| p.is_dir());
    view.install_size = install.as_deref().map(size_on_disk).unwrap_or(0);
    let data: Vec<PathBuf> = match &entry.source {
        Source::Program(program) => {
            if install.is_none() {
                if let Some(bytes) = program.estimated_bytes {
                    view.install_size = bytes;
                    view.estimated = true;
                }
            }
            let others: Vec<PathBuf> = installs.iter().filter(|p| install.as_ref() != Some(*p)).cloned().collect();
            let identity = Identity { name: &program.name, publisher: program.publisher.as_deref(), install_location: install.as_deref() };
            leftovers::data_folders(&identity, lookup, &others)
        }
        Source::Store(package) => expand(&format!(r"%LOCALAPPDATA%\Packages\{}", package.family), lookup)
            .filter(|p| p.is_dir())
            .into_iter()
            .collect(),
    };
    view.data_folders = data.iter().map(|p| FolderSize { path: p.display().to_string(), alloc: size_on_disk(p) }).collect();
    view.data_size = view.data_folders.iter().map(|f| f.alloc).sum();
    view.measured = true;
    view
}

fn size_on_disk(path: &Path) -> u64 {
    let cluster = win::cluster_size(path);
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => totals(&scan_dir(path, cluster)).0,
        Ok(meta) => file_rec(String::new(), &meta, path, cluster).alloc,
        Err(_) => 0,
    }
}

fn lower(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_lowercase())
}

#[tauri::command]
pub fn apps_list(apps: tauri::State<'_, Apps>, refresh: bool) -> AppsList {
    apps.list(refresh)
}

#[tauri::command]
pub fn app_uninstall(apps: tauri::State<'_, Apps>, id: String) -> Result<(), String> {
    apps.uninstall(&id)
}

#[tauri::command]
pub fn app_uninstall_status(apps: tauri::State<'_, Apps>, id: String) -> Option<UninstallStatus> {
    apps.uninstall_status(&id)
}

#[tauri::command]
pub fn app_stop_waiting(apps: tauri::State<'_, Apps>, id: String) {
    apps.stop_waiting(&id)
}

#[tauri::command]
pub fn app_leftovers(apps: tauri::State<'_, Apps>, id: String) -> Vec<FolderSize> {
    apps.leftovers(&id)
}

#[tauri::command]
pub fn recycle_leftovers(apps: tauri::State<'_, Apps>, paths: Vec<String>) -> RecycleReport {
    apps.recycle(&paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delete(paths: &[PathBuf]) -> Vec<PathBuf> {
        for p in paths {
            let _ = if p.is_dir() { fs::remove_dir_all(p) } else { fs::remove_file(p) };
        }
        paths.iter().filter(|p| p.exists()).cloned().collect()
    }

    #[test]
    fn recycles_only_offered_paths() {
        let dir = tempfile::tempdir().unwrap();
        let offered = dir.path().join("Roaming").join("OldApp");
        let other = dir.path().join("Roaming").join("Keep");
        for folder in [&offered, &other] {
            fs::create_dir_all(folder).unwrap();
            fs::write(folder.join("f.bin"), vec![0u8; 3000]).unwrap();
        }
        let apps = Apps::new(Arc::new(|_: &str| None), delete);
        apps.offered.lock().unwrap().insert(lower(&offered));

        let report = apps.recycle(&[offered.display().to_string().to_uppercase(), other.display().to_string()]);
        assert!(!offered.exists());
        assert!(other.join("f.bin").exists());
        assert!(report.moved >= 3000);
        assert_eq!(report.refused, vec![other.display().to_string()]);
        assert!(report.left.is_empty());
    }

    #[test]
    fn drops_install_locations_shared_with_other_apps() {
        let entry = |id: &str, location: &str| {
            let mut e = program_entry(
                Program {
                    id: id.into(),
                    key: id.into(),
                    name: id.into(),
                    version: None,
                    publisher: None,
                    install_location: Some(location.into()),
                    uninstall_string: "x".into(),
                    estimated_bytes: None,
                    msi: false,
                },
                &|_: &str| None,
            );
            e.view.install_location = Some(location.into());
            e
        };
        let mut entries = vec![
            entry("premiere", r"C:\Program Files\Adobe"),
            entry("aftereffects", r"C:\Program Files\Adobe"),
            entry("photoshop", r"C:\Program Files\Adobe\Photoshop"),
            entry("parent", r"C:\Games"),
            entry("game", r"C:\Games\Big Game"),
            entry("alone", r"C:\Tools\Alone"),
        ];
        drop_shared_installs(&mut entries);
        let kept: Vec<_> = entries.iter().filter(|e| e.view.install_location.is_some()).map(|e| e.view.id.as_str()).collect();
        assert_eq!(kept, ["photoshop", "game", "alone"]);
    }

    #[test]
    fn ignores_install_locations_that_are_too_broad() {
        let lookup = |name: &str| match name {
            "ProgramFiles" => Some(r"C:\Program Files".to_string()),
            _ => None,
        };
        let program = |location: &str| Program {
            id: r"HKLM\x".into(),
            key: "x".into(),
            name: "X".into(),
            version: None,
            publisher: None,
            install_location: Some(location.into()),
            uninstall_string: "x".into(),
            estimated_bytes: None,
            msi: false,
        };
        assert_eq!(program_entry(program(r"C:\Program Files"), &lookup).view.install_location, None);
        assert_eq!(program_entry(program(r"C:\"), &lookup).view.install_location, None);
        assert_eq!(
            program_entry(program(r"C:\Program Files\X"), &lookup).view.install_location.as_deref(),
            Some(r"C:\Program Files\X")
        );
    }
}
