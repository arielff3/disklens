//! Prints System Data for this machine: `cargo run --release --example system_data`

use disklens_lib::system_data::catalog::CATALOG;
use disklens_lib::system_data::expand::env_lookup;
use disklens_lib::system_data::{ItemState, SystemData};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const GIB: f64 = (1u64 << 30) as f64;

fn main() {
    let started = Instant::now();
    let data = SystemData::new(CATALOG, Arc::new(env_lookup), false);
    let items = loop {
        let items = data.get(false);
        if items.iter().all(|i| i.measured.state != ItemState::Pending) {
            break items;
        }
        thread::sleep(Duration::from_millis(50));
    };
    for item in &items {
        let m = &item.measured;
        println!(
            "{:<22} {:<11} {:>9.2} GB {:>9} files  {}",
            item.id,
            format!("{:?}", m.state),
            m.alloc as f64 / GIB,
            m.files,
            m.path.as_deref().unwrap_or("")
        );
    }
    println!("measured in {:.2?}", started.elapsed());
}
