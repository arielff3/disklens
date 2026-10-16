use disklens_lib::scanner::{Progress, Scanner, WalkScanner};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;

fn write(path: &Path, len: usize) {
    fs::write(path, vec![0u8; len]).unwrap();
}

fn scan(root: &Path) -> disklens_lib::index::Index {
    WalkScanner
        .scan(root, &Progress::default(), &AtomicBool::new(false))
        .expect("scan failed")
        .expect("scan cancelled")
}

#[test]
fn scans_sizes_counts_and_paths() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("a").join("b")).unwrap();
    write(&root.join("top.bin"), 1000);
    write(&root.join("a").join("one.mp4"), 5000);
    write(&root.join("a").join("b").join("two.jpg"), 300);

    let progress = Progress::default();
    let index = WalkScanner.scan(root, &progress, &AtomicBool::new(false)).unwrap().unwrap();

    let r = index.node(index.root()).unwrap();
    assert_eq!(r.size, 6300);
    assert_eq!(r.files, 3);
    assert!(r.alloc > 0);
    assert_eq!(index.path_of(index.root()), root.to_string_lossy());

    let b = index.find_path(&root.join("a").join("b").to_string_lossy()).unwrap();
    assert_eq!(index.node(b).unwrap().files, 1);
    assert_eq!(progress.snapshot().files, 3);
}

#[test]
fn does_not_follow_junctions() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir(root.join("real")).unwrap();
    write(&root.join("real").join("data.bin"), 2000);
    let out = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(root.join("link"))
        .arg(root.join("real"))
        .output()
        .unwrap();
    assert!(out.status.success(), "mklink failed: {out:?}");

    let index = scan(root);
    assert_eq!(index.node(index.root()).unwrap().size, 2000);
    assert!(index.find_path(&root.join("link").to_string_lossy()).is_none());
}

#[test]
fn missing_root_is_an_error() {
    let result = WalkScanner.scan(
        Path::new(r"C:\definitely\not\here\disklens"),
        &Progress::default(),
        &AtomicBool::new(false),
    );
    assert!(result.is_err());
}

#[test]
fn cancelled_scan_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let result = WalkScanner.scan(dir.path(), &Progress::default(), &AtomicBool::new(true));
    assert!(result.unwrap().is_none());
}
