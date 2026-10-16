//! Lists installed apps by footprint: `cargo run --release --example apps`

use disklens_lib::apps::Apps;
use disklens_lib::cleanup::recycle::recycle;
use disklens_lib::system_data::expand::env_lookup;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const GIB: f64 = (1u64 << 30) as f64;

fn main() {
    let started = Instant::now();
    let apps = Apps::new(Arc::new(env_lookup), recycle);
    let list = loop {
        let list = apps.list(false);
        if !list.loading && list.apps.iter().all(|a| a.measured) {
            break list;
        }
        thread::sleep(Duration::from_millis(100));
    };
    let mut rows = list.apps;
    rows.sort_by_key(|a| std::cmp::Reverse(a.install_size + a.data_size));
    println!("{} apps, measured in {:.1?}", rows.len(), started.elapsed());
    for app in rows.iter().take(25) {
        println!(
            "{:>7.2} GB  {:<40} {:<8?} install {:.2} GB{}",
            (app.install_size + app.data_size) as f64 / GIB,
            app.name.chars().take(40).collect::<String>(),
            app.kind,
            app.install_size as f64 / GIB,
            if app.estimated { " (estimate)" } else { "" }
        );
        for folder in &app.data_folders {
            println!("{:>14} data {:.2} GB  {}", "", folder.alloc as f64 / GIB, folder.path);
        }
    }
}
