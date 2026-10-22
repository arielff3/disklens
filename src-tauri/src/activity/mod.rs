//! Activity: CPU, memory and GPU per app and for the whole system, and ending processes.
//! A sampler thread runs only while someone polls `snapshot`, and stops 5 s after the last poll.

pub mod end;
pub mod gpu;
pub mod group;
pub mod guard;
mod names;
pub mod procs;

use end::EndResult;
use gpu::GpuQuery;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Seconds of system history kept for the graphs.
pub const HISTORY: usize = 60;
const TICK: Duration = Duration::from_secs(1);
const IDLE_STOP: Duration = Duration::from_secs(5);
const NAME_BUDGET: Duration = Duration::from_millis(150);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Process {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub exe: Option<PathBuf>,
    /// Percent of the whole machine, 0 to 100.
    pub cpu: f32,
    /// Private working set, bytes.
    pub ram: u64,
    /// `None` when the GPU counters aren't available.
    pub gpu: Option<f32>,
    pub gpu_mem: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct App {
    pub key: String,
    /// Empty for the Windows processes group; the UI names it.
    pub name: String,
    pub exe: Option<PathBuf>,
    pub processes: Vec<Process>,
    pub cpu: f32,
    pub ram: u64,
    pub gpu: Option<f32>,
    pub gpu_mem: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemNow {
    pub cpu: f32,
    pub cores: usize,
    pub ram_used: u64,
    pub ram_total: u64,
    pub gpu: Option<f32>,
    pub gpu_mem_used: Option<u64>,
    pub gpu_mem_total: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemPoint {
    pub cpu: f32,
    pub ram_used: u64,
    pub gpu: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// Milliseconds since the Unix epoch.
    pub taken_at: u64,
    pub system: SystemNow,
    pub history: Vec<SystemPoint>,
    pub apps: Vec<App>,
}

/// Appends a point, keeping the last `HISTORY`.
pub fn push_history(history: &mut Vec<SystemPoint>, point: SystemPoint) {
    history.push(point);
    if history.len() > HISTORY {
        history.drain(..history.len() - HISTORY);
    }
}

struct Inner {
    latest: Option<Snapshot>,
    /// App names by executable, kept across sampler restarts: reading them is the slow part.
    names: HashMap<PathBuf, Option<String>>,
    history: Vec<SystemPoint>,
    last_poll: Instant,
    running: bool,
}

pub struct Activity {
    inner: Arc<Mutex<Inner>>,
}

impl Default for Activity {
    fn default() -> Self {
        Self::new()
    }
}

impl Activity {
    pub fn new() -> Activity {
        Activity {
            inner: Arc::new(Mutex::new(Inner { latest: None, names: HashMap::new(), history: Vec::new(), last_poll: Instant::now(), running: false })),
        }
    }

    /// The latest sample (None until the first one lands); starts the sampler if it isn't running.
    pub fn snapshot(&self) -> Option<Snapshot> {
        let mut inner = self.inner.lock().unwrap();
        inner.last_poll = Instant::now();
        if !inner.running {
            inner.running = true;
            let shared = self.inner.clone();
            std::thread::Builder::new()
                .name("activity".into())
                .spawn(move || sample_loop(shared))
                .expect("spawn the activity sampler");
        }
        inner.latest.clone()
    }

    /// Ends these processes, looking their names up fresh for the guard.
    pub fn end(&self, pids: &[u32]) -> Vec<EndResult> {
        let running: HashMap<u32, String> = procs::list().into_iter().map(|p| (p.pid, p.name)).collect();
        end::end(pids, |pid| running.get(&pid).cloned())
    }
}

/// What the sampler remembers about a process between ticks.
struct Seen {
    create_time: i64,
    cpu_time: i64,
    exe: Option<PathBuf>,
}

fn sample_loop(shared: Arc<Mutex<Inner>>) {
    let cores = procs::cores();
    let mut gpu = GpuQuery::open();
    let gpu_total = gpu.as_ref().and(gpu::dedicated_total());
    let windows_dir = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let mut seen: HashMap<u32, Seen> = HashMap::new();
    let mut last = Instant::now();
    // CPU use is a difference between two readings: take the first one now.
    for p in procs::list() {
        seen.insert(p.pid, Seen { create_time: p.create_time, cpu_time: p.cpu_time, exe: procs::exe_path(p.pid) });
    }
    std::thread::sleep(Duration::from_millis(300));

    loop {
        let now = Instant::now();
        let elapsed = (now - last).as_nanos() as i64 / 100;
        last = now;
        let listed = procs::list();
        let sample = gpu.as_mut().map(GpuQuery::sample);
        let (ram_used, ram_total) = procs::memory();

        let mut next: HashMap<u32, Seen> = HashMap::with_capacity(listed.len());
        let mut idle = 0.0f32;
        let mut processes = Vec::with_capacity(listed.len());
        for raw in listed {
            let previous = seen.remove(&raw.pid).filter(|s| s.create_time == raw.create_time);
            let cpu = previous.as_ref().map(|s| procs::cpu_percent(s.cpu_time, raw.cpu_time, elapsed, cores)).unwrap_or(0.0);
            let exe = match previous {
                Some(s) => s.exe,
                None => procs::exe_path(raw.pid),
            };
            next.insert(raw.pid, Seen { create_time: raw.create_time, cpu_time: raw.cpu_time, exe: exe.clone() });
            if raw.pid == 0 {
                idle = cpu; // the Idle process is idle time, not a process
                continue;
            }
            let gpu_entry = sample.as_ref().map(|s| s.per_pid.get(&raw.pid).copied().unwrap_or((0.0, 0)));
            processes.push(Process {
                pid: raw.pid,
                parent: (raw.parent != 0).then_some(raw.parent),
                name: raw.name,
                exe,
                cpu,
                ram: raw.memory,
                gpu: gpu_entry.map(|(percent, _)| percent),
                gpu_mem: gpu_entry.map(|(_, bytes)| bytes),
            });
        }
        seen = next;

        let mut names = std::mem::take(&mut shared.lock().unwrap().names);
        // Reading version info is slow the first time (hundreds of executables): spread it over a
        // few ticks so the first numbers show right away; unread apps show their file name meanwhile.
        let budget = Instant::now();
        for exe in processes.iter().filter_map(|p| p.exe.as_ref()) {
            if budget.elapsed() > NAME_BUDGET {
                break;
            }
            if !names.contains_key(exe) {
                names.insert(exe.clone(), names::describe(exe));
            }
        }
        let apps = group::group(processes, |exe| names.get(exe).cloned().flatten(), &windows_dir);
        let system = SystemNow {
            cpu: (100.0 - idle).clamp(0.0, 100.0),
            cores,
            ram_used,
            ram_total,
            gpu: sample.as_ref().map(|s| s.total),
            gpu_mem_used: sample.as_ref().map(|s| s.mem_used),
            gpu_mem_total: gpu_total,
        };
        let taken_at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);

        {
            let mut inner = shared.lock().unwrap();
            inner.names = names;
            if inner.last_poll.elapsed() > IDLE_STOP {
                inner.running = false;
                inner.latest = None;
                inner.history.clear();
                return;
            }
            push_history(&mut inner.history, SystemPoint { cpu: system.cpu, ram_used: system.ram_used, gpu: system.gpu });
            let history = inner.history.clone();
            inner.latest = Some(Snapshot { taken_at, system, history, apps });
        }
        std::thread::sleep(TICK);
    }
}

#[tauri::command]
pub fn activity_snapshot(activity: tauri::State<'_, Activity>) -> Option<Snapshot> {
    activity.snapshot()
}

#[tauri::command]
pub fn activity_end(activity: tauri::State<'_, Activity>, pids: Vec<u32>) -> Vec<EndResult> {
    activity.end(&pids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_keeps_the_last_minute() {
        let mut history = Vec::new();
        for i in 0..70 {
            push_history(&mut history, SystemPoint { cpu: i as f32, ram_used: 0, gpu: None });
        }
        assert_eq!(history.len(), HISTORY);
        assert_eq!(history[0].cpu, 10.0);
        assert_eq!(history[HISTORY - 1].cpu, 69.0);
    }
}
