//! Shrinking WSL and Docker virtual disks, which grow but never shrink on their own.

use crate::cleanup::running::process_names;
use crate::elevation::run_elevated;
use crate::scanner::file_rec;
use crate::system_data::expand::{expand, Lookup};
use crate::win;
use serde::Serialize;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DOCKER_DESKTOP: &str = "docker desktop.exe";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiskKind {
    Wsl,
    Docker,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VDisk {
    pub path: String,
    pub kind: DiskKind,
    /// The WSL distribution using it, when known.
    pub distro: Option<String>,
    pub alloc: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CompactPhase {
    Trimming,
    ShuttingDown,
    Compacting,
    Done,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskResult {
    pub path: String,
    pub before: u64,
    pub after: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactStatus {
    pub phase: CompactPhase,
    pub message: Option<String>,
    pub results: Vec<DiskResult>,
}

/// The diskpart commands that compact each disk, attached read-only so nothing inside changes.
pub fn diskpart_script(paths: &[PathBuf]) -> String {
    let mut script = String::new();
    for path in paths {
        script.push_str(&format!(
            "select vdisk file=\"{}\"\nattach vdisk readonly\ncompact vdisk\ndetach vdisk\n",
            path.display()
        ));
    }
    script.push_str("exit\n");
    script
}

/// WSL distributions and their folders, from the registry.
fn distros() -> Vec<(String, PathBuf)> {
    let Ok(lxss) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Lxss") else {
        return Vec::new();
    };
    lxss.enum_keys()
        .flatten()
        .filter_map(|guid| {
            let key = lxss.open_subkey(&guid).ok()?;
            let name: String = key.get_value("DistributionName").ok()?;
            let base: String = key.get_value("BasePath").ok()?;
            Some((name, PathBuf::from(base.trim_start_matches(r"\\?\"))))
        })
        .collect()
}

/// Every WSL and Docker virtual disk on this machine.
pub fn list(lookup: &Lookup) -> Vec<VDisk> {
    let mut disks: Vec<VDisk> = Vec::new();
    let mut add = |path: PathBuf, kind: DiskKind, distro: Option<String>| {
        let key = path.to_string_lossy().to_lowercase();
        if disks.iter().any(|d| d.path.to_lowercase() == key) {
            return;
        }
        if let Some(alloc) = size_on_disk(&path) {
            disks.push(VDisk { path: path.display().to_string(), kind, distro, alloc });
        }
    };
    for (name, base) in distros() {
        let kind = if name.starts_with("docker-desktop") { DiskKind::Docker } else { DiskKind::Wsl };
        add(base.join("ext4.vhdx"), kind, Some(name));
    }
    for (base, kind, depth) in [
        (r"%LOCALAPPDATA%\Docker", DiskKind::Docker, 4),
        (r"%LOCALAPPDATA%\Packages", DiskKind::Wsl, 3),
        (r"%LOCALAPPDATA%\wsl", DiskKind::Wsl, 3),
    ] {
        if let Some(base) = expand(base, lookup) {
            for path in find_vhdx(&base, depth) {
                add(path, kind, None);
            }
        }
    }
    disks.sort_by_key(|d| std::cmp::Reverse(d.alloc));
    disks
}

fn find_vhdx(dir: &Path, depth: usize) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let Ok(t) = entry.file_type() else { continue };
        let path = entry.path();
        if t.is_dir() && !t.is_symlink() && depth > 0 {
            found.extend(find_vhdx(&path, depth - 1));
        } else if t.is_file() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("vhdx")) {
            found.push(path);
        }
    }
    found
}

fn size_on_disk(path: &Path) -> Option<u64> {
    let parent = path.parent()?;
    let name = path.file_name()?;
    let entry = fs::read_dir(parent).ok()?.flatten().find(|e| e.file_name().eq_ignore_ascii_case(name))?;
    let meta = entry.metadata().ok()?;
    Some(file_rec(String::new(), &meta, path, win::cluster_size(path)).alloc)
}

fn hidden(program: &str, args: &[&str]) -> bool {
    Command::new(program).args(args).creation_flags(CREATE_NO_WINDOW).status().is_ok_and(|s| s.success())
}

pub struct VDisks {
    lookup: Arc<Lookup>,
    status: Arc<Mutex<Option<CompactStatus>>>,
}

impl VDisks {
    pub fn new(lookup: Arc<Lookup>) -> VDisks {
        VDisks { lookup, status: Arc::default() }
    }

    pub fn list(&self) -> Vec<VDisk> {
        list(&*self.lookup)
    }

    /// Compacts the given disks in the background; follow it with [`VDisks::status`].
    pub fn compact(&self, paths: Vec<String>) -> Result<(), String> {
        if process_names().contains(DOCKER_DESKTOP) {
            return Err("Quit Docker Desktop first: it restarts WSL while it runs.".into());
        }
        let known = self.list();
        let disks: Vec<VDisk> = known.into_iter().filter(|d| paths.iter().any(|p| p.eq_ignore_ascii_case(&d.path))).collect();
        if disks.is_empty() {
            return Err("None of these disks were found.".into());
        }
        {
            let mut status = self.status.lock().unwrap();
            if status.as_ref().is_some_and(|s| !matches!(s.phase, CompactPhase::Done | CompactPhase::Failed)) {
                return Err("Already compacting.".into());
            }
            *status = Some(CompactStatus { phase: CompactPhase::Trimming, message: None, results: Vec::new() });
        }
        let shared = self.status.clone();
        thread::spawn(move || {
            let set = |phase: CompactPhase, message: Option<String>, results: Vec<DiskResult>| {
                *shared.lock().unwrap() = Some(CompactStatus { phase, message, results });
            };
            // Trim first, so blocks freed inside Linux are zeroed and can be reclaimed.
            // A distro that can't trim (no fstrim, or it fails to start) is still compacted.
            for distro in disks.iter().filter(|d| d.kind == DiskKind::Wsl).filter_map(|d| d.distro.as_deref()) {
                hidden("wsl.exe", &["-d", distro, "-u", "root", "--", "fstrim", "-a"]);
            }
            set(CompactPhase::ShuttingDown, None, Vec::new());
            hidden("wsl.exe", &["--shutdown"]);

            set(CompactPhase::Compacting, None, Vec::new());
            let paths: Vec<PathBuf> = disks.iter().map(|d| PathBuf::from(&d.path)).collect();
            let script = std::env::temp_dir().join(format!("disklens-compact-{}.txt", std::process::id()));
            let outcome = fs::write(&script, diskpart_script(&paths))
                .map_err(|e| e.to_string())
                .and_then(|_| run_elevated("diskpart.exe", &format!("/s \"{}\"", script.display())));
            let _ = fs::remove_file(&script);

            let results: Vec<DiskResult> = disks
                .iter()
                .map(|d| DiskResult { path: d.path.clone(), before: d.alloc, after: size_on_disk(Path::new(&d.path)).unwrap_or(d.alloc) })
                .collect();
            match outcome {
                Ok(0) => set(CompactPhase::Done, None, results),
                Ok(code) => set(CompactPhase::Failed, Some(format!("diskpart ended with code {code}.")), results),
                Err(message) => set(CompactPhase::Failed, Some(message), results),
            }
        });
        Ok(())
    }

    pub fn status(&self) -> Option<CompactStatus> {
        self.status.lock().unwrap().clone()
    }
}

#[tauri::command]
pub fn vdisks(vdisks: tauri::State<'_, VDisks>) -> Vec<VDisk> {
    vdisks.list()
}

#[tauri::command]
pub fn vdisk_compact(vdisks: tauri::State<'_, VDisks>, paths: Vec<String>) -> Result<(), String> {
    vdisks.compact(paths)
}

#[tauri::command]
pub fn vdisk_status(vdisks: tauri::State<'_, VDisks>) -> Option<CompactStatus> {
    vdisks.status()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_diskpart_script_per_disk() {
        let script = diskpart_script(&[PathBuf::from(r"C:\a\ext4.vhdx"), PathBuf::from(r"C:\b c\docker_data.vhdx")]);
        assert_eq!(
            script,
            "select vdisk file=\"C:\\a\\ext4.vhdx\"\nattach vdisk readonly\ncompact vdisk\ndetach vdisk\n\
             select vdisk file=\"C:\\b c\\docker_data.vhdx\"\nattach vdisk readonly\ncompact vdisk\ndetach vdisk\nexit\n"
        );
    }

    #[test]
    fn finds_vhdx_files_a_bounded_depth_down() {
        let dir = tempfile::tempdir().unwrap();
        let shallow = dir.path().join("a").join("disk.VHDX");
        let deep = dir.path().join("a").join("b").join("c").join("deep.vhdx");
        for file in [&shallow, &deep] {
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"x").unwrap();
        }
        assert_eq!(find_vhdx(dir.path(), 1), vec![shallow]);
    }

    #[test]
    fn lists_this_machines_disks() {
        for disk in list(&crate::system_data::expand::env_lookup) {
            assert!(disk.path.to_lowercase().ends_with(".vhdx"));
        }
    }
}
