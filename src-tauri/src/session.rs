//! The live state behind the UI: the current index, the scan in progress,
//! the watcher and the snapshot store. Tauri-free; events go through a callback.
//!
//! Lock order: `control` before `live`, never the other way around. Everything
//! that writes `live` holds `control`, so while `control` is held the index can
//! only change by the holder's own hand.

use crate::history;
use crate::index::Index;
use crate::scanner::{self, Progress, ProgressSnapshot, Scanner};
use crate::snapshot::Store;
use crate::watcher::{self, Batch, Watcher};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

pub enum Event {
    /// Foreground scans only.
    Progress(ProgressSnapshot),
    /// A foreground scan finished, failed or was cancelled.
    ScanDone(ScanDone),
    /// The index changed: patched by the watcher, or replaced by a background scan.
    IndexUpdated(IndexUpdated),
}

pub type Emit = Arc<dyn Fn(Event) + Send + Sync>;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanDone {
    pub root: String,
    pub ok: bool,
    pub error: Option<String>,
    pub cancelled: bool,
    pub files: u64,
    pub unreadable: u64,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexUpdated {
    pub root: String,
    pub files: u64,
    pub unreadable: u64,
    /// A background scan is running; another update follows when it ends.
    pub refreshing: bool,
    pub scanned_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    pub root: String,
    /// True when a saved index was loaded and is ready to browse. False means a
    /// foreground scan started; wait for `ScanDone`.
    pub loaded: bool,
    pub scanned_at: i64,
    pub files: u64,
    pub unreadable: u64,
}

#[derive(Clone)]
pub struct Session {
    inner: Arc<Inner>,
}

struct Inner {
    store: Option<Store>,
    scanner: Arc<dyn Scanner>,
    emit: Emit,
    control: Mutex<Control>,
    live: RwLock<Option<Live>>,
    /// Folders Disklens is removing itself (lowercased). See [`Session::quiet`].
    quiet: Mutex<Vec<PathBuf>>,
}

struct Live {
    root: String,
    index: Index,
    scanned_at: i64,
    cluster: u64,
    /// Patched since it was last saved.
    dirty: AtomicBool,
}

#[derive(Default)]
struct Control {
    /// Bumped by every scan start; a scan thread whose number is stale drops its result.
    generation: u64,
    scan: Option<RunningScan>,
    watcher: Option<RootWatcher>,
    /// Bumped whenever the watcher is replaced, so a stale callback can tell.
    watch_generation: u64,
    /// Changes seen while a scan runs, applied to its result.
    pending: Vec<PathBuf>,
    pending_overflow: bool,
}

struct RunningScan {
    cancel: Arc<AtomicBool>,
    background: bool,
}

struct RootWatcher {
    root: String,
    _watcher: Watcher,
}

impl Session {
    pub fn new(store: Option<Store>, scanner: Arc<dyn Scanner>, emit: Emit) -> Session {
        Session {
            inner: Arc::new(Inner {
                store,
                scanner,
                emit,
                control: Mutex::new(Control::default()),
                live: RwLock::new(None),
                quiet: Mutex::default(),
            }),
        }
    }

    /// Reopens the root of the most recently saved snapshot. `None` if nothing was saved.
    pub fn restore(&self) -> Result<Option<Opened>, String> {
        match self.inner.store.as_ref().and_then(Store::latest) {
            Some(root) => self.open_root(&root).map(Some),
            None => Ok(None),
        }
    }

    /// Shows `root`: from its snapshot when there is one (then refreshes in the
    /// background), otherwise with a foreground scan.
    pub fn open_root(&self, root: &str) -> Result<Opened, String> {
        let root = normalize_root(root);
        if let Some(opened) = self.opened_if_current(&root) {
            return Ok(opened);
        }
        self.save_if_dirty();
        let loaded = match self.inner.store.as_ref().map(|s| s.load(&root)) {
            Some(Ok(Some(loaded))) => Some(loaded),
            Some(Err(discarded)) => {
                eprintln!("discarded snapshot of {root}: {}", discarded.reason);
                None
            }
            _ => None,
        };
        match loaded {
            Some(loaded) => {
                // Snapshots saved before history existed seed it, so the refresh
                // that starts below already has something to compare with.
                if let Some(store) = &self.inner.store {
                    let history_path = history::path_for(&store.path_for(&root));
                    if history::load(&history_path).is_empty() {
                        let _ = history::append(&history_path, history::summarize(&loaded.index, loaded.scanned_at));
                    }
                }
                let opened = {
                    let mut control = self.inner.control.lock().unwrap();
                    let mut live = self.inner.live.write().unwrap();
                    let new = Live {
                        cluster: scanner::cluster_size(Path::new(&root)),
                        root: root.clone(),
                        index: loaded.index,
                        scanned_at: loaded.scanned_at,
                        dirty: AtomicBool::new(false),
                    };
                    let opened = new.opened(true);
                    *live = Some(new);
                    drop(live);
                    self.start_scan_locked(&mut control, root, true);
                    opened
                };
                Ok(opened)
            }
            None => {
                self.scan(&root)?;
                Ok(Opened { root, loaded: false, scanned_at: 0, files: 0, unreadable: 0 })
            }
        }
    }

    /// Starts a foreground scan of `root`, cancelling any scan in progress.
    /// Progress arrives as `Event::Progress`, the result as `Event::ScanDone`.
    pub fn scan(&self, root: &str) -> Result<(), String> {
        let root = normalize_root(root);
        let mut control = self.inner.control.lock().unwrap();
        {
            // Free the old tree while the new one is built.
            let mut live = self.inner.live.write().unwrap();
            if live.as_ref().is_some_and(|l| !same_root(&l.root, &root)) {
                *live = None;
            }
        }
        self.start_scan_locked(&mut control, root, false);
        Ok(())
    }

    pub fn cancel_scan(&self) {
        let mut control = self.inner.control.lock().unwrap();
        control.generation += 1;
        if let Some(scan) = control.scan.take() {
            scan.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Runs `f` on the current index, if there is one.
    pub fn with_index<R>(&self, f: impl FnOnce(&Index) -> R) -> Option<R> {
        self.inner.live.read().unwrap().as_ref().map(|l| f(&l.index))
    }

    /// Writes the snapshot again if the watcher changed the index since it was saved.
    pub fn save_if_dirty(&self) {
        let live = self.inner.live.read().unwrap();
        if let (Some(live), Some(store)) = (live.as_ref(), &self.inner.store) {
            if live.dirty.swap(false, Ordering::Relaxed) {
                if let Err(e) = store.save(&live.index, live.scanned_at) {
                    eprintln!("could not save the index of {}: {e}", live.root);
                    live.dirty.store(true, Ordering::Relaxed);
                }
            }
        }
    }

    fn opened_if_current(&self, root: &str) -> Option<Opened> {
        let live = self.inner.live.read().unwrap();
        live.as_ref().filter(|l| same_root(&l.root, root)).map(|l| l.opened(true))
    }

    fn start_scan_locked(&self, control: &mut Control, root: String, background: bool) {
        control.generation += 1;
        let generation = control.generation;
        if let Some(previous) = control.scan.take() {
            previous.cancel.store(true, Ordering::Relaxed);
        }
        // Watch first, so changes made while the scan runs are queued, not lost.
        self.ensure_watcher(control, &root);
        control.pending.clear();
        control.pending_overflow = false;
        let cancel = Arc::new(AtomicBool::new(false));
        control.scan = Some(RunningScan { cancel: cancel.clone(), background });

        let session = self.clone();
        let spawned = thread::Builder::new()
            .name("scan".into())
            .stack_size(64 << 20)
            .spawn(move || session.run_scan(root, cancel, generation, background));
        if let Err(e) = spawned {
            eprintln!("could not start the scan thread: {e}");
            control.scan = None;
        }
    }

    fn ensure_watcher(&self, control: &mut Control, root: &str) {
        if control.watcher.as_ref().is_some_and(|w| same_root(&w.root, root)) {
            return;
        }
        control.watcher = None;
        control.watch_generation += 1;
        let generation = control.watch_generation;
        let weak = Arc::downgrade(&self.inner);
        match Watcher::start(Path::new(root), move |batch| on_batch(&weak, generation, batch)) {
            Ok(w) => control.watcher = Some(RootWatcher { root: root.to_string(), _watcher: w }),
            // Network shares and some file systems can't be watched; browsing still works.
            Err(e) => eprintln!("not watching {root}: {e}"),
        }
    }

    fn run_scan(&self, root: String, cancel: Arc<AtomicBool>, generation: u64, background: bool) {
        let inner = &self.inner;
        let started = Instant::now();
        let progress = Arc::new(Progress::default());
        let finished = Arc::new(AtomicBool::new(false));
        let ticker = (!background).then(|| {
            let (emit, progress, finished) = (inner.emit.clone(), progress.clone(), finished.clone());
            thread::spawn(move || {
                while !finished.load(Ordering::Relaxed) {
                    emit(Event::Progress(progress.snapshot()));
                    thread::sleep(PROGRESS_INTERVAL);
                }
            })
        });
        let result = inner.scanner.scan(Path::new(&root), &progress, &cancel);
        finished.store(true, Ordering::Relaxed);
        if let Some(t) = ticker {
            let _ = t.join();
        }

        let mut done = ScanDone {
            root: root.clone(),
            ok: false,
            error: None,
            cancelled: false,
            files: 0,
            unreadable: 0,
            elapsed_ms: started.elapsed().as_millis() as u64,
        };
        let mut control = inner.control.lock().unwrap();
        let current = control.generation == generation && !cancel.load(Ordering::Relaxed);
        match result {
            Ok(Some(mut index)) if current => {
                control.scan = None;
                let pending = std::mem::take(&mut control.pending);
                let overflow = std::mem::take(&mut control.pending_overflow);
                let cluster = scanner::cluster_size(Path::new(&root));
                let patched = !pending.is_empty() && watcher::apply(&mut index, &pending, cluster);
                done.ok = true;
                done.files = index.node(index.root()).map_or(0, |n| n.files);
                done.unreadable = index.unreadable_dirs();
                *inner.live.write().unwrap() = Some(Live {
                    root: root.clone(),
                    index,
                    scanned_at: unix_now(),
                    cluster,
                    dirty: AtomicBool::new(patched),
                });
                if overflow {
                    self.start_scan_locked(&mut control, root.clone(), true);
                }
                drop(control);
                self.save();
            }
            Ok(_) => {
                drop(control);
                done.cancelled = true;
            }
            Err(e) => {
                if current {
                    control.scan = None;
                }
                drop(control);
                done.error = Some(e.to_string());
            }
        }
        if !background {
            (inner.emit)(Event::ScanDone(done));
        } else if !done.cancelled {
            self.emit_updated();
        }
    }

    /// Saves the snapshot after a full scan, and records the scan in the history.
    fn save(&self) {
        let live = self.inner.live.read().unwrap();
        if let (Some(live), Some(store)) = (live.as_ref(), &self.inner.store) {
            live.dirty.store(false, Ordering::Relaxed);
            match store.save(&live.index, live.scanned_at) {
                Ok(path) => {
                    let summary = history::summarize(&live.index, live.scanned_at);
                    if let Err(e) = history::append(&history::path_for(&path), summary) {
                        eprintln!("could not record the history of {}: {e}", live.root);
                    }
                }
                Err(e) => eprintln!("could not save the index of {}: {e}", live.root),
            }
        }
    }

    /// Recorded scans of the current root, oldest first.
    pub fn history(&self) -> Vec<history::Summary> {
        let live = self.inner.live.read().unwrap();
        match (live.as_ref(), &self.inner.store) {
            (Some(live), Some(store)) => history::load(&history::path_for(&store.path_for(&live.root))),
            _ => Vec::new(),
        }
    }

    /// The current root, when it was scanned, and its folder sizes right now (live updates included).
    pub fn current_summary(&self) -> Option<(String, i64, history::Summary)> {
        let live = self.inner.live.read().unwrap();
        live.as_ref().map(|l| (l.root.clone(), l.scanned_at, history::summarize(&l.index, unix_now())))
    }

    fn emit_updated(&self) {
        let refreshing = {
            let control = self.inner.control.lock().unwrap();
            control.scan.as_ref().is_some_and(|s| s.background)
        };
        let update = {
            let live = self.inner.live.read().unwrap();
            live.as_ref().map(|l| {
                let opened = l.opened(true);
                IndexUpdated {
                    root: opened.root,
                    files: opened.files,
                    unreadable: opened.unreadable,
                    refreshing,
                    scanned_at: opened.scanned_at,
                }
            })
        };
        if let Some(update) = update {
            (self.inner.emit)(Event::IndexUpdated(update));
        }
    }
}

/// Keeps the watcher quiet about folders Disklens is removing itself, until dropped.
pub struct Quiet {
    session: Session,
    folders: Vec<PathBuf>,
}

impl Session {
    /// Starts removing `folders` (a Projects delete, a clean-up). Deleting
    /// thousands of files overflows the watcher, and each overflow used to
    /// rescan the whole drive while the job was still running, over and over.
    /// Until the returned guard drops, reports inside these folders and lost
    /// events are ignored; then only these folders are patched in the index.
    pub fn quiet(&self, folders: Vec<PathBuf>) -> Quiet {
        self.inner.quiet.lock().unwrap().extend(folders.iter().map(|f| lowered(f)));
        Quiet { session: self.clone(), folders }
    }
}

impl Drop for Quiet {
    fn drop(&mut self) {
        {
            let mut quiet = self.session.inner.quiet.lock().unwrap();
            for folder in &self.folders {
                if let Some(i) = quiet.iter().position(|q| *q == lowered(folder)) {
                    quiet.remove(i);
                }
            }
        }
        let generation = self.session.inner.control.lock().unwrap().watch_generation;
        on_batch(&Arc::downgrade(&self.session.inner), generation, Batch::Paths(std::mem::take(&mut self.folders)));
    }
}

fn lowered(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_lowercase())
}

/// Watcher callback: patch the index now, or queue the change for the scan in progress.
fn on_batch(weak: &Weak<Inner>, generation: u64, batch: Batch) {
    let Some(inner) = weak.upgrade() else { return };
    let session = Session { inner };
    let mut control = session.inner.control.lock().unwrap();
    if control.watch_generation != generation {
        return;
    }
    let batch = {
        let quiet = session.inner.quiet.lock().unwrap();
        if quiet.is_empty() {
            batch
        } else {
            match batch {
                // Most likely our own removals; the guard patches those folders when the job ends.
                Batch::Overflow => return,
                Batch::Paths(paths) => {
                    let rest: Vec<PathBuf> = paths
                        .into_iter()
                        .filter(|p| {
                            let p = lowered(p);
                            !quiet.iter().any(|q| p.starts_with(q))
                        })
                        .collect();
                    if rest.is_empty() {
                        return;
                    }
                    Batch::Paths(rest)
                }
            }
        }
    };
    if control.scan.is_some() {
        match batch {
            Batch::Overflow => control.pending_overflow = true,
            Batch::Paths(paths) => control.pending.extend(paths),
        }
        return;
    }
    match batch {
        Batch::Overflow => {
            let root = session.inner.live.read().unwrap().as_ref().map(|l| l.root.clone());
            let Some(root) = root else { return };
            eprintln!("the watcher lost events in {root}; rescanning in the background");
            session.start_scan_locked(&mut control, root, true);
        }
        Batch::Paths(paths) => {
            // Read the disk under a read lock so the UI keeps browsing meanwhile;
            // holding `control` guarantees nobody replaces the index in between.
            let changes = {
                let live = session.inner.live.read().unwrap();
                let Some(live) = live.as_ref() else { return };
                watcher::probe(&live.index, &paths, live.cluster)
            };
            if changes.is_empty() {
                return;
            }
            let mut live = session.inner.live.write().unwrap();
            let Some(live) = live.as_mut() else { return };
            watcher::commit(&mut live.index, changes);
            live.dirty.store(true, Ordering::Relaxed);
        }
    }
    drop(control);
    session.emit_updated();
}

impl Live {
    fn opened(&self, loaded: bool) -> Opened {
        let root = self.index.node(self.index.root());
        Opened {
            root: self.root.clone(),
            loaded,
            scanned_at: self.scanned_at,
            files: root.map_or(0, |n| n.files),
            unreadable: self.index.unreadable_dirs(),
        }
    }
}

/// `C:` alone means "the current directory on C:", so always scan from the drive root.
pub fn normalize_root(root: &str) -> String {
    let root = root.trim();
    if root.len() == 2 && root.ends_with(':') {
        format!("{root}\\")
    } else {
        root.to_string()
    }
}

fn same_root(a: &str, b: &str) -> bool {
    let trim = |s: &str| s.trim_end_matches(['\\', '/']).to_lowercase();
    trim(a) == trim(b)
}

pub fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::WalkScanner;
    use std::sync::mpsc;

    #[test]
    fn bare_drive_letter_becomes_drive_root() {
        assert_eq!(normalize_root("C:"), r"C:\");
        assert_eq!(normalize_root(r"C:\"), r"C:\");
        assert_eq!(normalize_root(r" D:\Games "), r"D:\Games");
    }

    #[test]
    fn roots_compare_case_and_separator_insensitively() {
        assert!(same_root(r"C:\", "c:"));
        assert!(same_root(r"C:\Users\", r"c:\users"));
        assert!(!same_root(r"C:\Users", r"C:\Users2"));
    }

    #[test]
    fn overflow_marks_the_index_stale_and_rescans_in_the_background() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("tree");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.bin"), [1u8; 10]).unwrap();
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let emit: Emit = Arc::new(move |e| {
            let _ = tx.lock().unwrap().send(e);
        });
        let session = Session::new(Some(Store::new(dir.path().join("store"))), Arc::new(WalkScanner), emit);
        session.open_root(&root.to_string_lossy()).unwrap();
        let wait = |want: fn(&Event) -> bool| loop {
            let e = rx.recv_timeout(Duration::from_secs(10)).expect("timed out");
            if want(&e) {
                return e;
            }
        };
        wait(|e| matches!(e, Event::ScanDone(_)));

        std::fs::write(root.join("b.bin"), [2u8; 10]).unwrap();
        let generation = session.inner.control.lock().unwrap().watch_generation;
        on_batch(&Arc::downgrade(&session.inner), generation, Batch::Overflow);
        wait(|e| matches!(e, Event::IndexUpdated(u) if u.refreshing));
        wait(|e| matches!(e, Event::IndexUpdated(u) if !u.refreshing));
        let files = session.with_index(|i| i.node(i.root()).unwrap().files).unwrap();
        assert_eq!(files, 2);
    }

    #[test]
    fn own_removals_do_not_trigger_rescans() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("tree");
        let build = root.join("proj").join("node_modules");
        std::fs::create_dir_all(&build).unwrap();
        for i in 0..50 {
            std::fs::write(build.join(format!("f{i}.js")), [1u8; 10]).unwrap();
        }
        std::fs::write(root.join("proj").join("keep.txt"), [1u8; 10]).unwrap();
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let emit: Emit = Arc::new(move |e| {
            let _ = tx.lock().unwrap().send(e);
        });
        let session = Session::new(Some(Store::new(dir.path().join("store"))), Arc::new(WalkScanner), emit);
        session.open_root(&root.to_string_lossy()).unwrap();
        loop {
            if matches!(rx.recv_timeout(Duration::from_secs(10)).expect("timed out"), Event::ScanDone(_)) {
                break;
            }
        }

        let quiet = session.quiet(vec![build.clone()]);
        std::fs::remove_dir_all(&build).unwrap();
        let generation = session.inner.control.lock().unwrap().watch_generation;
        // What the watcher reports while thousands of files disappear.
        on_batch(&Arc::downgrade(&session.inner), generation, Batch::Overflow);
        on_batch(&Arc::downgrade(&session.inner), generation, Batch::Paths(vec![build.join("f1.js")]));
        assert!(session.inner.control.lock().unwrap().scan.is_none(), "a rescan started for our own deletion");

        drop(quiet);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let files = session.with_index(|i| i.node(i.root()).unwrap().files).unwrap();
            if files == 1 {
                break;
            }
            assert!(Instant::now() < deadline, "the index still counts {files} files");
            thread::sleep(Duration::from_millis(20));
        }
        assert!(session.inner.control.lock().unwrap().scan.is_none(), "a rescan started after the job ended");
    }
}
