//! Live updates: edit a real folder after scanning it and check the patched
//! index matches a fresh scan.

use disklens_lib::index::Index;
use disklens_lib::scanner::{self, Progress, Scanner, WalkScanner};
use disklens_lib::watcher;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

fn write(path: &Path, len: usize) {
    fs::write(path, vec![7u8; len]).unwrap();
}

fn scan(root: &Path) -> Index {
    WalkScanner
        .scan(root, &Progress::default(), &AtomicBool::new(false))
        .unwrap()
        .unwrap()
}

/// The outline without folder modification times: those change on disk with
/// every edit inside the folder, and tests call `apply` only with the paths
/// they touched.
fn shape(index: &Index) -> Vec<String> {
    index
        .outline()
        .into_iter()
        .map(|line| {
            if !line.contains(" dir=true ") {
                return line;
            }
            let start = line.find(" modified=").unwrap();
            let end = line[start + 1..].find(' ').unwrap() + start + 1;
            format!("{}{}", &line[..start], &line[end..])
        })
        .collect()
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    index: Index,
    cluster: u64,
}

impl Fixture {
    /// root { top.bin, a\ { one.mp4, b\ { two.jpg } } }
    fn new() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("tree");
        fs::create_dir_all(root.join("a").join("b")).unwrap();
        write(&root.join("top.bin"), 1000);
        write(&root.join("a").join("one.mp4"), 5000);
        write(&root.join("a").join("b").join("two.jpg"), 300);
        let index = scan(&root);
        let cluster = scanner::cluster_size(&root);
        Fixture {
            _dir: dir,
            root,
            index,
            cluster,
        }
    }

    fn apply(&mut self, paths: &[PathBuf]) -> bool {
        watcher::apply(&mut self.index, paths, self.cluster)
    }

    fn assert_matches_disk(&self) {
        assert_eq!(shape(&self.index), shape(&scan(&self.root)));
    }
}

#[test]
fn new_file() {
    let mut f = Fixture::new();
    let p = f.root.join("a").join("new.zip");
    write(&p, 40_000);
    assert!(f.apply(&[p]));
    f.assert_matches_disk();
}

#[test]
fn grown_and_shrunk_file() {
    let mut f = Fixture::new();
    let p = f.root.join("a").join("one.mp4");
    write(&p, 900_000);
    assert!(f.apply(std::slice::from_ref(&p)));
    f.assert_matches_disk();
    write(&p, 10);
    f.apply(&[p]);
    f.assert_matches_disk();
}

#[test]
fn deleted_file() {
    let mut f = Fixture::new();
    let p = f.root.join("a").join("b").join("two.jpg");
    fs::remove_file(&p).unwrap();
    assert!(f.apply(&[p]));
    f.assert_matches_disk();
}

#[test]
fn new_folder_with_files_reported_child_first() {
    let mut f = Fixture::new();
    let d = f.root.join("a").join("c").join("d");
    fs::create_dir_all(&d).unwrap();
    write(&d.join("x.bin"), 12_345);
    write(&f.root.join("a").join("c").join("y.txt"), 99);
    // Only the deepest file is reported: the missing folders above it are walked.
    assert!(f.apply(&[d.join("x.bin")]));
    f.assert_matches_disk();
}

#[test]
fn deleted_folder_with_many_events() {
    let mut f = Fixture::new();
    let a = f.root.join("a");
    let events = vec![
        a.join("b").join("two.jpg"),
        a.join("one.mp4"),
        a.join("b"),
        a.clone(),
    ];
    fs::remove_dir_all(&a).unwrap();
    assert!(f.apply(&events));
    f.assert_matches_disk();
}

#[test]
fn renamed_file_and_folder() {
    let mut f = Fixture::new();
    let (from, to) = (f.root.join("top.bin"), f.root.join("a").join("moved.iso"));
    fs::rename(&from, &to).unwrap();
    let (dfrom, dto) = (f.root.join("a").join("b"), f.root.join("bee"));
    fs::rename(&dfrom, &dto).unwrap();
    assert!(f.apply(&[from, to, dfrom, dto]));
    f.assert_matches_disk();
}

#[test]
fn file_replaced_by_folder() {
    let mut f = Fixture::new();
    let p = f.root.join("top.bin");
    fs::remove_file(&p).unwrap();
    fs::create_dir(&p).unwrap();
    write(&p.join("inner.dat"), 777);
    assert!(f.apply(&[p]));
    f.assert_matches_disk();
}

#[test]
fn folder_event_refreshes_only_its_modified_time() {
    let mut f = Fixture::new();
    let a = f.root.join("a");
    let tmp = a.join("tmp");
    write(&tmp, 1);
    fs::remove_file(&tmp).unwrap();
    f.apply(std::slice::from_ref(&a));
    let prefix = format!("{} ", a.display());
    let line = |index: &Index| {
        index
            .outline()
            .into_iter()
            .find(|l| l.starts_with(&prefix))
            .unwrap()
    };
    assert_eq!(line(&f.index), line(&scan(&f.root)));
}

#[test]
fn paths_outside_root_and_root_itself_are_ignored() {
    let mut f = Fixture::new();
    let before = f.index.outline();
    let outside = f.root.parent().unwrap().join("elsewhere.txt");
    write(&outside, 10);
    assert!(!f.apply(&[outside, f.root.clone()]));
    assert_eq!(f.index.outline(), before);
}

#[test]
fn unchanged_file_is_not_a_change() {
    let mut f = Fixture::new();
    assert!(!f.apply(&[f.root.join("top.bin")]));
}

#[test]
fn junction_is_not_counted() {
    let mut f = Fixture::new();
    let link = f.root.join("link");
    let out = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(f.root.join("a"))
        .output()
        .unwrap();
    assert!(out.status.success());
    f.apply(&[link]);
    f.assert_matches_disk();
}

/// A burst like unpacking an archive: thousands of new files in a folder that
/// already holds thousands. Lookups must not be linear in the folder size.
#[test]
fn many_new_files_in_a_big_folder_apply_quickly() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("tree");
    let big = root.join("big");
    fs::create_dir_all(&big).unwrap();
    let name = |i: usize| big.join(format!("file-with-a-fairly-long-name-{i:05}.dat"));
    const N: usize = 3000;
    for i in 0..N {
        write(&name(i), 1);
    }
    let mut index = scan(&root);
    for i in N..2 * N {
        write(&name(i), 1);
    }
    let paths: Vec<PathBuf> = (N..2 * N).map(name).collect();
    let started = std::time::Instant::now();
    assert!(watcher::apply(&mut index, &paths, scanner::cluster_size(&root)));
    let took = started.elapsed();
    assert_eq!(shape(&index), shape(&scan(&root)));
    assert!(took < std::time::Duration::from_secs(2), "apply took {took:?}");
}

#[test]
fn watcher_reports_a_new_file() {
    use std::sync::mpsc;
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = mpsc::channel();
    let _watcher = watcher::Watcher::start_with(
        dir.path(),
        Duration::from_millis(100),
        Duration::from_millis(500),
        move |b| {
            let _ = tx.send(b);
        },
    )
    .unwrap();
    let p = dir.path().join("hello.txt");
    write(&p, 10);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        match rx.recv_timeout(left) {
            Ok(watcher::Batch::Paths(paths)) if paths.contains(&p) => break,
            Ok(_) => continue,
            Err(_) => panic!("no batch with {} within 5 s", p.display()),
        }
    }
}

mod session {
    use super::write;
    use disklens_lib::index::Index;
    use disklens_lib::scanner::{Progress, Scanner, WalkScanner};
    use disklens_lib::session::{Event, Session};
    use disklens_lib::snapshot::Store;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::{self, Receiver};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    const WAIT: Duration = Duration::from_secs(10);

    struct Env {
        _dir: tempfile::TempDir,
        root: PathBuf,
        store_dir: PathBuf,
    }

    impl Env {
        fn new() -> Env {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("tree");
            fs::create_dir_all(root.join("a")).unwrap();
            write(&root.join("top.bin"), 1000);
            write(&root.join("a").join("one.mp4"), 5000);
            let store_dir = dir.path().join("store");
            Env { _dir: dir, root, store_dir }
        }

        fn root_str(&self) -> String {
            self.root.to_string_lossy().into_owned()
        }

        fn session(&self, scanner: Arc<dyn Scanner>) -> (Session, Receiver<Event>) {
            let (tx, rx) = mpsc::channel();
            let tx = std::sync::Mutex::new(tx);
            let emit = Arc::new(move |e: Event| {
                let _ = tx.lock().unwrap().send(e);
            });
            (Session::new(Some(Store::new(&self.store_dir)), scanner, emit), rx)
        }
    }

    fn walk() -> Arc<dyn Scanner> {
        Arc::new(WalkScanner)
    }

    /// Waits for the first event `pick` accepts, skipping others.
    fn wait_for<T>(rx: &Receiver<Event>, what: &str, mut pick: impl FnMut(Event) -> Option<T>) -> T {
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(e) => {
                    if let Some(t) = pick(e) {
                        return t;
                    }
                }
                Err(_) => panic!("timed out waiting for {what}"),
            }
        }
    }

    fn scan_done(rx: &Receiver<Event>) {
        wait_for(rx, "scan-done", |e| match e {
            Event::ScanDone(d) => {
                assert!(d.ok, "scan failed: {:?}", d.error);
                Some(())
            }
            _ => None,
        });
    }

    fn updated(rx: &Receiver<Event>, refreshing: bool) {
        wait_for(rx, "index-updated", |e| match e {
            Event::IndexUpdated(u) if u.refreshing == refreshing => Some(()),
            _ => None,
        });
    }

    fn has(session: &Session, path: &Path) -> bool {
        session.with_index(|i: &Index| i.find_path(&path.to_string_lossy()).is_some()).unwrap_or(false)
    }

    fn files(session: &Session) -> u64 {
        session.with_index(|i| i.node(i.root()).unwrap().files).unwrap()
    }

    #[test]
    fn first_open_scans_and_saves() {
        let env = Env::new();
        let (session, rx) = env.session(walk());
        let opened = session.open_root(&env.root_str()).unwrap();
        assert!(!opened.loaded);
        scan_done(&rx);
        assert_eq!(files(&session), 2);
        assert!(Store::new(&env.store_dir).load(&env.root_str()).unwrap().is_some());
    }

    #[test]
    fn restore_loads_instantly_then_refreshes() {
        let env = Env::new();
        {
            let (session, rx) = env.session(walk());
            session.open_root(&env.root_str()).unwrap();
            scan_done(&rx);
        }
        // Changed while the app was closed: only the background refresh sees it.
        write(&env.root.join("later.zip"), 3000);

        let (session, rx) = env.session(walk());
        let opened = session.restore().unwrap().expect("nothing restored");
        assert!(opened.loaded);
        assert_eq!(opened.root, env.root_str());
        assert_eq!(opened.files, 2);
        assert_eq!(files(&session), 2);
        updated(&rx, false);
        assert_eq!(files(&session), 3);
    }

    #[test]
    fn restore_with_nothing_saved_is_none() {
        let env = Env::new();
        let (session, _rx) = env.session(walk());
        assert!(session.restore().unwrap().is_none());
    }

    #[test]
    fn corrupt_snapshot_means_a_foreground_scan() {
        let env = Env::new();
        {
            let (session, rx) = env.session(walk());
            session.open_root(&env.root_str()).unwrap();
            scan_done(&rx);
        }
        let path = Store::new(&env.store_dir).path_for(&env.root_str());
        let mut bytes = fs::read(&path).unwrap();
        let n = bytes.len();
        bytes.truncate(n - 20);
        fs::write(&path, bytes).unwrap();

        let (session, rx) = env.session(walk());
        let opened = session.restore().unwrap().unwrap();
        assert!(!opened.loaded);
        assert_eq!(opened.root, env.root_str());
        scan_done(&rx);
        assert_eq!(files(&session), 2);
    }

    #[test]
    fn watcher_patches_the_index_and_save_on_exit_keeps_it() {
        let env = Env::new();
        let (session, rx) = env.session(walk());
        session.open_root(&env.root_str()).unwrap();
        scan_done(&rx);
        let new = env.root.join("a").join("new.iso");
        write(&new, 70_000);
        let deadline = Instant::now() + WAIT;
        while !has(&session, &new) {
            updated(&rx, false);
            assert!(Instant::now() < deadline);
        }
        session.save_if_dirty();
        let loaded = Store::new(&env.store_dir).load(&env.root_str()).unwrap().unwrap();
        assert!(loaded.index.find_path(&new.to_string_lossy()).is_some());
    }

    /// Walks the tree, then adds a file the walk has already passed and holds
    /// the scan open until the watcher has reported it.
    struct LateWriter(PathBuf);

    impl Scanner for LateWriter {
        fn scan(&self, root: &Path, progress: &Progress, cancel: &AtomicBool) -> io::Result<Option<Index>> {
            let index = WalkScanner.scan(root, progress, cancel)?;
            write(&self.0, 4242);
            std::thread::sleep(Duration::from_millis(1500));
            Ok(index)
        }
    }

    #[test]
    fn changes_during_a_scan_are_replayed() {
        let env = Env::new();
        let late = env.root.join("late.bin");
        let (session, rx) = env.session(Arc::new(LateWriter(late.clone())));
        session.open_root(&env.root_str()).unwrap();
        scan_done(&rx);
        assert!(has(&session, &late), "change made during the scan was lost");
    }

    #[test]
    fn opening_another_root_switches_index() {
        let env = Env::new();
        let (session, rx) = env.session(walk());
        session.open_root(&env.root_str()).unwrap();
        scan_done(&rx);
        let sub = env.root.join("a").to_string_lossy().into_owned();
        session.open_root(&sub).unwrap();
        scan_done(&rx);
        assert_eq!(session.with_index(|i| i.path_of(i.root())).unwrap(), sub);
        assert_eq!(files(&session), 1);
    }
}
