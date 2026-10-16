//! Lists build folders under a folder: `cargo run --release --example projects -- C:\path`

use disklens_lib::projects::find;
use disklens_lib::scanner::{Progress, Scanner, WalkScanner};
use disklens_lib::system_data::expand::env_lookup;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

const GIB: f64 = (1u64 << 30) as f64;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("pass a folder"));
    let index = WalkScanner.scan(&root, &Progress::default(), &AtomicBool::new(false)).unwrap().unwrap();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let projects = find(&index, &env_lookup);
    let total: u64 = projects.iter().map(|p| p.total).sum();
    println!("{} projects, {:.2} GB of build folders", projects.len(), total as f64 / GIB);
    for p in projects.iter().take(20) {
        println!("{:>7.2} GB  {:<40} {:>4} days idle", p.total as f64 / GIB, p.path, (now - p.last_active) / 86_400);
        for f in &p.folders {
            println!("{:>18} {:?} {:.2} GB", "", f.kind, f.alloc as f64 / GIB);
        }
    }
}
