use disklens_lib::scanner::{Progress, Scanner, WalkScanner};
use disklens_lib::snapshot::Store;
use std::fs;
use std::sync::atomic::AtomicBool;

#[test]
fn a_real_scan_survives_save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("tree");
    fs::create_dir_all(root.join("a").join("b")).unwrap();
    fs::write(root.join("top.bin"), vec![1u8; 1000]).unwrap();
    fs::write(root.join("a").join("one.mp4"), vec![2u8; 5000]).unwrap();
    fs::write(root.join("a").join("b").join("two.jpg"), vec![3u8; 300]).unwrap();
    let index = WalkScanner
        .scan(&root, &Progress::default(), &AtomicBool::new(false))
        .unwrap()
        .unwrap();

    let store = Store::new(dir.path().join("store"));
    store.save(&index, 99).unwrap();
    let root_str = root.to_string_lossy().into_owned();
    assert_eq!(store.latest().as_deref(), Some(root_str.as_str()));
    let loaded = store.load(&root_str).unwrap().unwrap();
    assert_eq!(loaded.index.outline(), index.outline());
    assert_eq!(loaded.scanned_at, 99);
}
