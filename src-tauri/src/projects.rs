//! Build folders across projects (node_modules, target, …), found in the scan index.

use crate::cleanup::safety;
use crate::index::{Index, Node, NodeId};
use crate::remove::remove_tree;
use crate::scanner::scan_dir;
use crate::system_data::expand::{expand, Lookup};
use crate::system_data::measure::totals;
use crate::win;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

/// Smaller build folders are not worth listing.
const MIN_SIZE: u64 = 1 << 20;
/// Deletes in flight at once; see [`crate::remove`].
const DELETE_THREADS: usize = 64;

/// Folders whose projects are tools, not the user's code: never searched.
const EXCLUDED: &[&str] = &[
    "%APPDATA%",
    "%LOCALAPPDATA%",
    "%ProgramFiles%",
    "%ProgramFiles(x86)%",
    "%ProgramData%",
    "%SystemRoot%",
];

const JUNK_NAMES: &[&str] = &["node_modules", "target", "build", ".gradle", "bin", "obj", ".venv", "venv", ".next"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum JunkKind {
    Node,
    Rust,
    Gradle,
    Dotnet,
    Python,
    Next,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JunkFolder {
    pub path: String,
    pub kind: JunkKind,
    pub alloc: u64,
    pub files: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub path: String,
    pub name: String,
    /// Newest file outside build folders and hidden folders (Unix seconds).
    pub last_active: i64,
    pub folders: Vec<JunkFolder>,
    pub total: u64,
}

fn lower(name: &str) -> String {
    name.to_lowercase()
}

/// What kind of build folder `dir` is, judging by its name, its siblings and its contents.
fn classify(index: &Index, dir: &Node, siblings: &[&Node]) -> Option<JunkKind> {
    let has_sibling = |names: &[&str]| siblings.iter().any(|s| !s.is_dir && names.contains(&lower(&s.name).as_str()));
    let has_sibling_ext = |exts: &[&str]| {
        siblings
            .iter()
            .any(|s| !s.is_dir && Path::new(&*s.name).extension().is_some_and(|e| exts.contains(&lower(&e.to_string_lossy()).as_str())))
    };
    let contains = |name: &str| {
        dir.children
            .iter()
            .filter_map(|&c| index.node(c))
            .any(|c| !c.removed && lower(&c.name) == name)
    };
    let gradle = ["build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts"];
    match lower(&dir.name).as_str() {
        "node_modules" if has_sibling(&["package.json"]) => Some(JunkKind::Node),
        ".next" if has_sibling(&["package.json"]) => Some(JunkKind::Next),
        "target" if has_sibling(&["cargo.toml"]) => Some(JunkKind::Rust),
        "build" if has_sibling(&gradle[..2]) => Some(JunkKind::Gradle),
        ".gradle" if has_sibling(&gradle) => Some(JunkKind::Gradle),
        "bin" | "obj" if has_sibling_ext(&["csproj", "fsproj", "vbproj"]) => Some(JunkKind::Dotnet),
        ".venv" | "venv" if contains("pyvenv.cfg") => Some(JunkKind::Python),
        _ => None,
    }
}

/// Every project with build folders, largest first.
pub fn find(index: &Index, lookup: &Lookup) -> Vec<Project> {
    let excluded: Vec<String> = EXCLUDED.iter().filter_map(|t| expand(t, lookup)).map(|p| lower(&p.to_string_lossy())).collect();
    let is_excluded = |path: &str| {
        let path = lower(path);
        excluded.iter().any(|e| path == *e || path.starts_with(&format!("{}\\", e.trim_end_matches('\\'))))
    };

    let mut projects: Vec<Project> = Vec::new();
    let mut stack: Vec<(NodeId, String)> = vec![(index.root(), index.path_of(index.root()))];
    while let Some((id, path)) = stack.pop() {
        let Some(node) = index.node(id) else { continue };
        if is_excluded(&path) {
            continue;
        }
        let children: Vec<(NodeId, &Node)> = node.children.iter().filter_map(|&c| index.node(c).map(|n| (c, n))).filter(|(_, n)| !n.removed).collect();
        let siblings: Vec<&Node> = children.iter().map(|(_, n)| *n).collect();
        let mut folders = Vec::new();
        for &(child_id, child) in &children {
            if !child.is_dir {
                continue;
            }
            let child_path = join(&path, &child.name);
            if let Some(kind) = classify(index, child, &siblings) {
                if child.alloc >= MIN_SIZE {
                    folders.push(JunkFolder { path: child_path, kind, alloc: child.alloc, files: child.files });
                }
            } else if !child.name.starts_with('.') {
                stack.push((child_id, child_path));
            }
        }
        if !folders.is_empty() {
            let junk: HashSet<&str> = folders.iter().map(|f| f.path.as_str()).collect();
            projects.push(Project {
                name: node.name.rsplit('\\').next().unwrap_or(&node.name).to_string(),
                last_active: last_active(index, id, &path, &junk),
                total: folders.iter().map(|f| f.alloc).sum(),
                path: path.clone(),
                folders,
            });
        }
    }
    projects.retain(|p| p.folders.iter().all(|f| safety::check(Path::new(&f.path), lookup).is_ok()));
    projects.sort_by_key(|p| std::cmp::Reverse(p.total));
    projects
}

/// Newest file under `id`, skipping build folders and hidden folders (like .git).
fn last_active(index: &Index, id: NodeId, path: &str, junk: &HashSet<&str>) -> i64 {
    let mut newest = 0;
    let mut stack = vec![(id, path.to_string())];
    while let Some((id, path)) = stack.pop() {
        let Some(node) = index.node(id) else { continue };
        for &c in &node.children {
            let Some(child) = index.node(c) else { continue };
            if child.removed {
                continue;
            }
            if child.is_dir {
                let child_path = join(&path, &child.name);
                if !child.name.starts_with('.') && !junk.contains(child_path.as_str()) {
                    stack.push((c, child_path));
                }
            } else {
                newest = newest.max(child.modified);
            }
        }
    }
    newest
}

fn join(parent: &str, name: &str) -> String {
    if parent.ends_with('\\') {
        format!("{parent}{name}")
    } else {
        format!(r"{parent}\{name}")
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteStatus {
    pub running: bool,
    pub finished: bool,
    pub total: usize,
    pub done: usize,
    pub current: Option<String>,
    pub freed: u64,
    /// Could not be deleted completely (files in use).
    pub failed: Vec<String>,
    /// Not offered, not a build folder, or refused by the safety check.
    pub refused: Vec<String>,
}

pub struct Projects {
    lookup: Arc<Lookup>,
    /// Folders shown to the user (lowercased path → bytes on disk); only these can be deleted.
    offered: Mutex<HashMap<String, u64>>,
    status: Arc<Mutex<DeleteStatus>>,
}

impl Projects {
    pub fn new(lookup: Arc<Lookup>) -> Projects {
        Projects { lookup, offered: Mutex::default(), status: Arc::default() }
    }

    pub fn list(&self, index: &Index) -> Vec<Project> {
        let projects = find(index, &*self.lookup);
        self.offered.lock().unwrap().extend(projects.iter().flat_map(|p| p.folders.iter().map(|f| (lower(&f.path), f.alloc))));
        projects
    }

    /// Deletes build folders permanently, in the background. `quiet` is called
    /// with the folders about to go and its guard is held until the job ends
    /// (see `Session::quiet`).
    pub fn delete<G: Send + 'static>(&self, paths: Vec<String>, quiet: impl FnOnce(Vec<PathBuf>) -> G) -> Result<(), String> {
        let mut status = self.status.lock().unwrap();
        if status.running {
            return Err("Already deleting.".into());
        }
        let offered = self.offered.lock().unwrap().clone();
        let mut allowed = Vec::new();
        let mut refused = Vec::new();
        for path in paths {
            let p = PathBuf::from(&path);
            let is_build_name = p.file_name().is_some_and(|n| JUNK_NAMES.contains(&lower(&n.to_string_lossy()).as_str()));
            let is_dir = fs::symlink_metadata(&p).is_ok_and(|m| m.is_dir());
            match offered.get(&lower(&path)) {
                Some(&size) if is_build_name && is_dir && safety::check(&p, &*self.lookup).is_ok() => allowed.push((p, size)),
                _ => refused.push(path),
            }
        }
        *status = DeleteStatus { running: true, total: allowed.len(), refused, ..Default::default() };
        drop(status);
        let shared = self.status.clone();
        let guard = quiet(allowed.iter().map(|(p, _)| p.clone()).collect());
        thread::spawn(move || {
            let _quiet = guard;
            // Deleting is bound by per-file latency (NTFS, antivirus filters), not
            // by CPU or disk: many deletes in flight are ~8x faster than one.
            let pool = rayon::ThreadPoolBuilder::new().num_threads(DELETE_THREADS).thread_name(|i| format!("delete-{i}")).build();
            for (path, size) in allowed {
                shared.lock().unwrap().current = Some(path.display().to_string());
                let _ = match &pool {
                    Ok(pool) => pool.install(|| remove_tree(&path)),
                    Err(_) => remove_tree(&path),
                };
                // Something stayed (files in use): measure what's left before taking the lock.
                let left = path.exists().then(|| totals(&scan_dir(&path, win::cluster_size(&path))).0);
                let mut status = shared.lock().unwrap();
                if let Some(left) = left {
                    status.failed.push(path.display().to_string());
                    status.freed += size.saturating_sub(left);
                } else {
                    status.freed += size;
                }
                status.done += 1;
            }
            let mut status = shared.lock().unwrap();
            status.running = false;
            status.finished = true;
            status.current = None;
        });
        Ok(())
    }

    pub fn status(&self) -> DeleteStatus {
        self.status.lock().unwrap().clone()
    }
}

/// `None` when nothing is scanned yet.
#[tauri::command]
pub fn projects_list(session: tauri::State<'_, crate::session::Session>, projects: tauri::State<'_, Projects>) -> Option<Vec<Project>> {
    session.with_index(|index| projects.list(index))
}

#[tauri::command]
pub fn projects_delete(
    session: tauri::State<'_, crate::session::Session>,
    projects: tauri::State<'_, Projects>,
    paths: Vec<String>,
) -> Result<(), String> {
    projects.delete(paths, |folders| session.quiet(folders))
}

#[tauri::command]
pub fn projects_status(projects: tauri::State<'_, Projects>) -> DeleteStatus {
    projects.status()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{DirTree, FileRec};
    use std::time::{Duration, Instant};

    const MB: u64 = 1 << 20;

    fn file(name: &str, alloc: u64, modified: i64) -> FileRec {
        FileRec { name: name.into(), size: alloc, alloc, modified }
    }

    fn dir(name: &str, files: Vec<FileRec>, dirs: Vec<DirTree>) -> DirTree {
        DirTree { name: name.into(), files, dirs, ..Default::default() }
    }

    fn lookup(name: &str) -> Option<String> {
        (name == "LOCALAPPDATA").then(|| r"C:\Users\a\AppData\Local".to_string())
    }

    fn sample() -> Index {
        let web = dir(
            "web",
            vec![file("package.json", 1, 100), file("index.js", 1, 500)],
            vec![
                dir("node_modules", vec![file("big.js", 300 * MB, 900)], vec![dir("dep", vec![file("package.json", 1, 1)], vec![dir("node_modules", vec![file("x", 5 * MB, 1)], vec![])])]),
                dir(".git", vec![file("pack", 10 * MB, 999)], vec![]),
            ],
        );
        let rust = dir("tool", vec![file("Cargo.toml", 1, 50)], vec![dir("target", vec![file("app.exe", 2000 * MB, 60)], vec![])]);
        let dotnet = dir("api", vec![file("Api.csproj", 1, 70)], vec![dir("bin", vec![file("a.dll", 50 * MB, 1)], vec![]), dir("obj", vec![file("o", 20 * MB, 1)], vec![])]);
        let py = dir("ml", vec![file("main.py", 1, 80)], vec![dir(".venv", vec![file("pyvenv.cfg", 1, 1), file("torch", 900 * MB, 1)], vec![])]);
        let not_a_project = dir("stuff", vec![], vec![dir("target", vec![file("t", 50 * MB, 1)], vec![]), dir("bin", vec![file("b", 50 * MB, 1)], vec![])]);
        let tool_install = dir(
            "Local",
            vec![],
            vec![dir("ext", vec![file("package.json", 1, 1)], vec![dir("node_modules", vec![file("m", 100 * MB, 1)], vec![])])],
        );
        let hidden = dir(".vscode", vec![], vec![dir("extensions", vec![], vec![dir("e", vec![file("package.json", 1, 1)], vec![dir("node_modules", vec![file("m", 80 * MB, 1)], vec![])])])]);
        Index::build(dir(
            r"C:\",
            vec![],
            vec![dir(
                "Users",
                vec![],
                vec![dir(
                    "a",
                    vec![],
                    vec![
                        dir("work", vec![], vec![web, rust, dotnet, py, not_a_project]),
                        dir("AppData", vec![], vec![tool_install]),
                        hidden,
                    ],
                )],
            )],
        ))
    }

    #[test]
    fn finds_build_folders_next_to_their_project_files() {
        let projects = find(&sample(), &lookup);
        let found: Vec<(&str, Vec<(&str, JunkKind)>)> = projects
            .iter()
            .map(|p| (p.name.as_str(), p.folders.iter().map(|f| (f.path.rsplit('\\').next().unwrap(), f.kind)).collect()))
            .collect();
        assert_eq!(
            found,
            vec![
                ("tool", vec![("target", JunkKind::Rust)]),
                ("ml", vec![(".venv", JunkKind::Python)]),
                ("web", vec![("node_modules", JunkKind::Node)]),
                ("api", vec![("bin", JunkKind::Dotnet), ("obj", JunkKind::Dotnet)]),
            ]
        );
        assert_eq!(projects[0].path, r"C:\Users\a\work\tool");
    }

    #[test]
    fn last_activity_ignores_build_and_hidden_folders() {
        let projects = find(&sample(), &lookup);
        let web = projects.iter().find(|p| p.name == "web").unwrap();
        assert_eq!(web.last_active, 500);
    }

    #[test]
    fn deletes_only_offered_build_folders() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("proj");
        fs::create_dir_all(project.join("node_modules").join("x")).unwrap();
        fs::write(project.join("node_modules").join("x").join("f.js"), vec![0u8; 4096]).unwrap();
        fs::write(project.join("package.json"), b"{}").unwrap();
        fs::create_dir_all(project.join("src")).unwrap();

        let projects = Projects::new(Arc::new(|_: &str| None));
        let node_modules = project.join("node_modules").display().to_string();
        projects.offered.lock().unwrap().insert(lower(&node_modules), 4096);
        projects.delete(vec![node_modules, project.join("src").display().to_string()], |_| ()).unwrap();

        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            let s = projects.status();
            if s.finished {
                break s;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        };
        assert!(!project.join("node_modules").exists());
        assert!(project.join("src").exists());
        assert_eq!(status.refused.len(), 1);
        assert!(status.freed >= 4096);
        assert!(status.failed.is_empty());
    }
}
