//! Prints the top apps by CPU, to compare with Task Manager: `cargo run --release --example activity`

use disklens_lib::activity::Activity;
use std::time::Duration;

const GIB: f64 = (1u64 << 30) as f64;
const MIB: f64 = (1u64 << 20) as f64;

fn main() {
    let activity = Activity::new();
    for round in 0..3 {
        // Polling keeps the sampler running; give it time for a couple of samples.
        std::thread::sleep(Duration::from_millis(1100));
        let Some(snap) = activity.snapshot() else {
            println!("(no sample yet)");
            continue;
        };
        let s = &snap.system;
        println!(
            "\n#{round} CPU {:.0}% ({} cores)  RAM {:.1}/{:.1} GB  GPU {}  GPU mem {}",
            s.cpu,
            s.cores,
            s.ram_used as f64 / GIB,
            s.ram_total as f64 / GIB,
            s.gpu.map(|g| format!("{g:.0}%")).unwrap_or("-".into()),
            match (s.gpu_mem_used, s.gpu_mem_total) {
                (Some(u), Some(t)) => format!("{:.1}/{:.1} GB", u as f64 / GIB, t as f64 / GIB),
                (Some(u), None) => format!("{:.1} GB", u as f64 / GIB),
                _ => "-".into(),
            }
        );
        for app in snap.apps.iter().take(10) {
            let name = if app.name.is_empty() { "Windows processes" } else { &app.name };
            println!(
                "  {:<34} {:>3} procs  CPU {:>5.1}%  RAM {:>7.0} MB  GPU {}",
                name,
                app.processes.len(),
                app.cpu,
                app.ram as f64 / MIB,
                app.gpu.map(|g| format!("{g:.1}%")).unwrap_or("-".into())
            );
        }
        if let Some(windows) = snap.apps.iter().find(|a| a.name.is_empty()) {
            let mut by_ram = windows.processes.clone();
            by_ram.sort_by_key(|p| std::cmp::Reverse(p.ram));
            println!("  Windows processes by memory:");
            for p in by_ram.iter().take(6) {
                println!("    {:<30} {:>7.0} MB  exe {}", p.name, p.ram as f64 / MIB, p.exe.as_ref().map(|e| e.display().to_string()).unwrap_or("-".into()));
            }
        }
    }
}
