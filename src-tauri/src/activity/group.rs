//! Processes grouped into apps, the way you think of them: Chrome, not 31 chrome.exe.

use super::guard::CRITICAL;
use super::{App, Process};
use std::collections::HashMap;
use std::path::Path;

/// Key of the group holding Windows' own processes.
pub const WINDOWS_KEY: &str = "windows";

/// Windows components whose path a non-admin app can't read, besides the critical ones.
const SYSTEM_NAMES: [&str; 18] = [
    "idle",
    "registry",
    "memory compression",
    "secure system",
    "lsaiso.exe",
    "msmpeng.exe",
    "nissrv.exe",
    "securityhealthservice.exe",
    "spoolsv.exe",
    "wmiprvse.exe",
    "sgrmbroker.exe",
    "audiodg.exe",
    "wudfhost.exe",
    "conhost.exe",
    "dllhost.exe",
    "taskhostw.exe",
    "runtimebroker.exe",
    "searchindexer.exe",
];

fn is_windows_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    CRITICAL.contains(&lower.as_str()) || SYSTEM_NAMES.contains(&lower.as_str())
}

/// Groups processes by executable path. `describe` names an app from its executable.
/// Executables under `windows_dir`, and Windows components whose path can't be read, form one
/// group; any other process without a readable path is grouped by its name.
pub fn group(processes: Vec<Process>, describe: impl Fn(&Path) -> Option<String>, windows_dir: &Path) -> Vec<App> {
    let windows = windows_dir.to_string_lossy().to_lowercase();
    let mut groups: HashMap<String, Vec<Process>> = HashMap::new();
    for p in processes {
        let key = match &p.exe {
            Some(exe) => {
                let lower = exe.to_string_lossy().to_lowercase();
                if lower.starts_with(&windows) {
                    WINDOWS_KEY.to_string()
                } else {
                    lower
                }
            }
            None if is_windows_name(&p.name) => WINDOWS_KEY.to_string(),
            None => format!("name:{}", p.name.to_lowercase()),
        };
        groups.entry(key).or_default().push(p);
    }
    let mut apps: Vec<App> = groups
        .into_iter()
        .map(|(key, mut processes)| {
            processes.sort_by(|a, b| b.cpu.total_cmp(&a.cpu).then(a.pid.cmp(&b.pid)));
            let name = if key == WINDOWS_KEY {
                String::new()
            } else {
                processes[0].exe.as_deref().and_then(&describe).unwrap_or_else(|| processes[0].name.clone())
            };
            let known_gpu: Vec<f32> = processes.iter().filter_map(|p| p.gpu).collect();
            let known_mem: Vec<u64> = processes.iter().filter_map(|p| p.gpu_mem).collect();
            App {
                name,
                exe: processes.iter().find_map(|p| p.exe.clone()).filter(|_| key != WINDOWS_KEY),
                cpu: processes.iter().map(|p| p.cpu).sum(),
                ram: processes.iter().map(|p| p.ram).sum(),
                gpu: (!known_gpu.is_empty()).then(|| known_gpu.iter().sum()),
                gpu_mem: (!known_mem.is_empty()).then(|| known_mem.iter().sum()),
                key,
                processes,
            }
        })
        .collect();
    apps.sort_by(|a, b| b.cpu.total_cmp(&a.cpu).then_with(|| a.key.cmp(&b.key)));
    apps
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(pid: u32, exe: Option<&str>, cpu: f32, ram: u64, gpu: Option<f32>) -> Process {
        let exe = exe.map(PathBuf::from);
        let name = exe.as_ref().and_then(|e| e.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "System".into());
        Process { pid, parent: None, name, exe, cpu, ram, gpu, gpu_mem: None }
    }

    fn describe(exe: &Path) -> Option<String> {
        (exe.file_name()? == "chrome.exe").then(|| "Google Chrome".to_string())
    }

    const WIN: &str = r"C:\Windows";

    #[test]
    fn same_exe_is_one_app_with_totals() {
        let chrome = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
        let apps = group(vec![p(1, Some(chrome), 5.0, 100, None), p(2, Some(&chrome.to_uppercase()), 2.5, 50, None)], describe, Path::new(WIN));
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Google Chrome");
        assert_eq!(apps[0].cpu, 7.5);
        assert_eq!(apps[0].ram, 150);
        assert_eq!(apps[0].processes.len(), 2);
    }

    #[test]
    fn falls_back_to_file_name() {
        let apps = group(vec![p(1, Some(r"D:\tools\tool.exe"), 1.0, 1, None)], describe, Path::new(WIN));
        assert_eq!(apps[0].name, "tool.exe");
    }

    fn unreadable(pid: u32, name: &str) -> Process {
        Process { name: name.into(), ..p(pid, None, 1.0, 1, None) }
    }

    #[test]
    fn windows_folder_and_system_names_group_together() {
        let apps = group(
            vec![p(1, Some(r"C:\WINDOWS\System32\svchost.exe"), 1.0, 1, None), unreadable(2, "Memory Compression")],
            describe,
            Path::new(WIN),
        );
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].key, WINDOWS_KEY);
        assert_eq!(apps[0].processes.len(), 2);
        assert!(apps[0].exe.is_none());
    }

    #[test]
    fn other_unreadable_processes_group_by_name() {
        let apps = group(vec![unreadable(1, "vmmemWSL"), unreadable(2, "avp.exe"), unreadable(3, "avp.exe")], describe, Path::new(WIN));
        let mut found: Vec<(String, usize)> = apps.iter().map(|a| (a.name.clone(), a.processes.len())).collect();
        found.sort();
        assert_eq!(found, [("avp.exe".to_string(), 2), ("vmmemWSL".to_string(), 1)]);
    }

    #[test]
    fn gpu_sums_only_known_values() {
        let app = r"C:\a\app.exe";
        let apps = group(vec![p(1, Some(app), 0.0, 0, Some(10.0)), p(2, Some(app), 0.0, 0, None)], describe, Path::new(WIN));
        assert_eq!(apps[0].gpu, Some(10.0));
        let apps = group(vec![p(3, Some(app), 0.0, 0, None)], describe, Path::new(WIN));
        assert_eq!(apps[0].gpu, None);
    }

    #[test]
    fn apps_sorted_by_cpu_desc() {
        let apps = group(vec![p(1, Some(r"C:\a\low.exe"), 1.0, 0, None), p(2, Some(r"C:\a\high.exe"), 9.0, 0, None)], describe, Path::new(WIN));
        assert_eq!(apps[0].name, "high.exe");
    }
}
