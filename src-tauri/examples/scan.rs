//! Times a real scan, then saving and loading its snapshot and a search:
//! `cargo run --release --example scan -- C:\`

use disklens_lib::scanner::{Progress, Scanner, WalkScanner};
use disklens_lib::search;
use disklens_lib::snapshot::Store;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

const GIB: f64 = (1u64 << 30) as f64;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| r"C:\".into()));
    let started = Instant::now();
    let index = WalkScanner
        .scan(&root, &Progress::default(), &AtomicBool::new(false))
        .expect("scan failed")
        .expect("scan cancelled");
    let r = index.node(index.root()).unwrap();
    println!(
        "{}: {} files, {:.2} GB on disk ({:.2} GB logical), {} nodes, {} unreadable folders, {:.2?}",
        root.display(),
        r.files,
        r.alloc as f64 / GIB,
        r.size as f64 / GIB,
        index.len(),
        index.unreadable_dirs(),
        started.elapsed()
    );

    let dir = std::env::temp_dir().join("disklens-example-store");
    let store = Store::new(&dir);
    let started = Instant::now();
    let path = store.save(&index, 0).expect("save failed");
    let bytes = std::fs::metadata(&path).map_or(0, |m| m.len());
    println!("saved {:.1} MB in {:.2?}", bytes as f64 / (1 << 20) as f64, started.elapsed());
    let started = Instant::now();
    let loaded = store.load(&root.to_string_lossy()).expect("load failed").expect("snapshot missing");
    println!("loaded {} nodes in {:.2?}", loaded.index.len(), started.elapsed());
    let _ = std::fs::remove_dir_all(&dir);

    for q in ["size:>100mb", "ext:dll", "node_modules"] {
        let r = search::run(&loaded.index, &search::parse(q).unwrap(), &[], disklens_lib::session::unix_now(), 500);
        println!("search {q:?}: {} matches in {} ms", r.total, r.elapsed_ms);
    }
}
