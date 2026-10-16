//! Compares deleting a node_modules-like tree with std::fs::remove_dir_all and
//! with Disklens' parallel remove_tree: `cargo run --release --example delete_speed`

use disklens_lib::remove::remove_tree;
use std::fs;
use std::path::Path;
use std::time::Instant;

const FILES: usize = 60_000;

fn make(root: &Path) {
    for i in 0..FILES {
        let dir = root.join(format!("pkg{}", i / 200)).join("lib");
        if i % 200 == 0 {
            fs::create_dir_all(&dir).unwrap();
        }
        fs::write(dir.join(format!("f{i}.js")), b"module.exports = 1;").unwrap();
    }
}

fn main() {
    let dir = tempfile::tempdir().unwrap();
    for (name, delete) in [
        ("std remove_dir_all", (|p: &Path| fs::remove_dir_all(p)) as fn(&Path) -> std::io::Result<()>),
        ("parallel remove_tree", remove_tree),
        ("remove_tree, 64 threads", |p: &Path| rayon::ThreadPoolBuilder::new().num_threads(64).build().unwrap().install(|| remove_tree(p))),
    ] {
        let tree = dir.path().join("node_modules");
        make(&tree);
        let started = Instant::now();
        delete(&tree).unwrap();
        let elapsed = started.elapsed();
        println!("{name:<22} {FILES} files in {elapsed:.1?}  ({:.0} files/s)", FILES as f64 / elapsed.as_secs_f64());
    }
}
