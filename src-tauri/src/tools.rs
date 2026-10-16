//! Windows' own space-saving tools, run from System Data.

use crate::apps::uninstall::{launch, Launch};
use crate::elevation::run_elevated;
use crate::system_data::expand::{expand, Lookup};
use crate::win;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolId {
    Hibernation,
    ComponentCleanup,
    CompactOs,
    SystemProtection,
    VirtualMemory,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolView {
    pub id: ToolId,
    /// Worth offering right now (e.g. hibernation is still on).
    pub available: bool,
    /// For CompactOS: whether Windows files are already compressed.
    pub on: Option<bool>,
    /// Runs a command as administrator, rather than opening a Windows window.
    pub elevated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolPhase {
    Running,
    Done,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub phase: ToolPhase,
    pub message: Option<String>,
}

/// What the machine looks like, as far as the tools care.
#[derive(Clone, Copy, Debug, Default)]
pub struct Machine {
    pub hiberfil: bool,
    pub compact_os: bool,
}

pub fn views(machine: Machine) -> Vec<ToolView> {
    let tool = |id, available, on, elevated| ToolView { id, available, on, elevated };
    vec![
        tool(ToolId::Hibernation, machine.hiberfil, None, true),
        tool(ToolId::ComponentCleanup, true, None, true),
        tool(ToolId::CompactOs, !machine.compact_os, Some(machine.compact_os), true),
        tool(ToolId::SystemProtection, true, None, false),
        tool(ToolId::VirtualMemory, true, None, false),
    ]
}

/// The program, its arguments, and whether it runs elevated and is waited for.
pub fn command(id: ToolId) -> (&'static str, &'static str, bool) {
    match id {
        ToolId::Hibernation => ("powercfg.exe", "/hibernate off", true),
        ToolId::ComponentCleanup => ("dism.exe", "/Online /Cleanup-Image /StartComponentCleanup", true),
        ToolId::CompactOs => ("compact.exe", "/CompactOS:always", true),
        // These dialogs ask for administrator rights themselves.
        ToolId::SystemProtection => ("SystemPropertiesProtection.exe", "", false),
        ToolId::VirtualMemory => ("SystemPropertiesPerformance.exe", "", false),
    }
}

pub struct Tools {
    lookup: Arc<Lookup>,
    jobs: Arc<Mutex<HashMap<ToolId, ToolStatus>>>,
}

impl Tools {
    pub fn new(lookup: Arc<Lookup>) -> Tools {
        Tools { lookup, jobs: Arc::default() }
    }

    pub fn list(&self) -> Vec<ToolView> {
        views(self.machine())
    }

    pub fn run(&self, id: ToolId) -> Result<(), String> {
        let (file, args, elevated) = command(id);
        if !elevated {
            launch(&Launch { file: file.into(), args: args.into() })?;
            self.jobs.lock().unwrap().insert(id, ToolStatus { phase: ToolPhase::Done, message: None });
            return Ok(());
        }
        {
            let mut jobs = self.jobs.lock().unwrap();
            if jobs.get(&id).is_some_and(|j| j.phase == ToolPhase::Running) {
                return Err("Already running.".into());
            }
            jobs.insert(id, ToolStatus { phase: ToolPhase::Running, message: None });
        }
        let jobs = self.jobs.clone();
        thread::spawn(move || {
            let status = match run_elevated(file, args) {
                Ok(0) => ToolStatus { phase: ToolPhase::Done, message: None },
                Ok(code) => ToolStatus { phase: ToolPhase::Failed, message: Some(format!("{file} ended with code {code}.")) },
                Err(message) => ToolStatus { phase: ToolPhase::Failed, message: Some(message) },
            };
            jobs.lock().unwrap().insert(id, status);
        });
        Ok(())
    }

    pub fn status(&self, id: ToolId) -> Option<ToolStatus> {
        self.jobs.lock().unwrap().get(&id).cloned()
    }

    fn machine(&self) -> Machine {
        let hiberfil = expand(r"%SystemDrive%\", &*self.lookup).is_some_and(|root| listed(&root, "hiberfil.sys"));
        let compact_os = expand(r"%SystemRoot%\System32\ntoskrnl.exe", &*self.lookup).is_some_and(|kernel| is_compressed(&kernel));
        Machine { hiberfil, compact_os }
    }
}

/// Whether `dir` lists `name`. Works for files Windows keeps locked.
fn listed(dir: &Path, name: &str) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| entries.flatten().any(|e| e.file_name().eq_ignore_ascii_case(name)))
}

/// CompactOS stores Windows files compressed; the kernel is one of them.
fn is_compressed(path: &Path) -> bool {
    match (fs::metadata(path), win::compressed_size(path)) {
        (Ok(meta), Some(on_disk)) => meta.len() > 0 && on_disk < meta.len() * 9 / 10,
        _ => false,
    }
}

#[tauri::command]
pub fn tools_list(tools: tauri::State<'_, Tools>) -> Vec<ToolView> {
    tools.list()
}

#[tauri::command]
pub fn tool_run(tools: tauri::State<'_, Tools>, id: ToolId) -> Result<(), String> {
    tools.run(id)
}

#[tauri::command]
pub fn tool_status(tools: tauri::State<'_, Tools>, id: ToolId) -> Option<ToolStatus> {
    tools.status(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_only_what_applies() {
        let ids = |m: Machine| views(m).into_iter().filter(|v| v.available).map(|v| v.id).collect::<Vec<_>>();
        let all = Machine { hiberfil: true, compact_os: false };
        assert_eq!(ids(all), [ToolId::Hibernation, ToolId::ComponentCleanup, ToolId::CompactOs, ToolId::SystemProtection, ToolId::VirtualMemory]);
        let done = Machine { hiberfil: false, compact_os: true };
        assert_eq!(ids(done), [ToolId::ComponentCleanup, ToolId::SystemProtection, ToolId::VirtualMemory]);
        assert_eq!(views(done)[2].on, Some(true));
    }

    #[test]
    fn dialogs_open_and_commands_run_elevated() {
        assert!(command(ToolId::Hibernation).2);
        assert!(!command(ToolId::SystemProtection).2);
        assert_eq!(command(ToolId::CompactOs).1, "/CompactOS:always");
    }

    #[test]
    fn reads_this_machine() {
        let tools = Tools::new(Arc::new(crate::system_data::expand::env_lookup));
        assert_eq!(tools.list().len(), 5);
    }
}
