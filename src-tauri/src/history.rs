//! Scan history: folder sizes after each full scan, to show what changed since.

use crate::index::{Index, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Smaller folders are not recorded.
pub const MIN_FOLDER: u64 = 4 << 20;
/// Entries kept per root.
pub const KEEP: usize = 12;
/// Smaller changes are not reported.
pub const MIN_CHANGE: u64 = 50 << 20;
/// A folder whose change is mostly explained by its subfolders is not reported itself.
const EXPLAINED: f64 = 0.7;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub scanned_at: i64,
    pub total: u64,
    /// Every folder of at least [`MIN_FOLDER`]: path relative to the root ("" is the root) and bytes on disk.
    pub folders: Vec<(String, u64)>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    /// Relative to the root.
    pub path: String,
    pub before: u64,
    pub after: u64,
}

/// Folder sizes of an index, down to [`MIN_FOLDER`].
pub fn summarize(index: &Index, scanned_at: i64) -> Summary {
    let mut folders = Vec::new();
    let mut stack: Vec<(NodeId, String)> = vec![(index.root(), String::new())];
    while let Some((id, path)) = stack.pop() {
        let Some(node) = index.node(id) else { continue };
        if node.removed || !node.is_dir || node.alloc < MIN_FOLDER {
            continue;
        }
        for &child in &node.children {
            if let Some(c) = index.node(child) {
                if c.is_dir && !c.removed && c.alloc >= MIN_FOLDER {
                    let child_path = if path.is_empty() { c.name.to_string() } else { format!(r"{path}\{}", c.name) };
                    stack.push((child, child_path));
                }
            }
        }
        folders.push((path, node.alloc));
    }
    let total = index.node(index.root()).map_or(0, |n| n.alloc);
    Summary { scanned_at, total, folders }
}

/// The most specific folders that explain the change from `before` to `after`, largest change first.
pub fn diff(before: &Summary, after: &Summary) -> Vec<Change> {
    let old: HashMap<&str, u64> = before.folders.iter().map(|(p, s)| (p.as_str(), *s)).collect();
    let new: HashMap<&str, u64> = after.folders.iter().map(|(p, s)| (p.as_str(), *s)).collect();
    let delta = |p: &str| new.get(p).copied().unwrap_or(0) as i128 - old.get(p).copied().unwrap_or(0) as i128;

    let mut paths: Vec<&str> = old.keys().chain(new.keys()).copied().collect();
    paths.sort_unstable();
    paths.dedup();
    let big: Vec<&str> = paths.into_iter().filter(|p| delta(p).unsigned_abs() >= MIN_CHANGE as u128).collect();

    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for &p in &big {
        if !p.is_empty() {
            children.entry(parent(p)).or_default().push(p);
        }
    }
    let mut changes: Vec<Change> = big
        .iter()
        .filter(|&&p| {
            let d = delta(p);
            let same_sign: i128 = children.get(p).map_or(0, |kids| kids.iter().map(|k| delta(k)).filter(|k| k.signum() == d.signum()).sum());
            (same_sign.unsigned_abs() as f64) < EXPLAINED * d.unsigned_abs() as f64
        })
        .filter(|p| !p.is_empty())
        .map(|&p| Change { path: p.to_string(), before: old.get(p).copied().unwrap_or(0), after: new.get(p).copied().unwrap_or(0) })
        .collect();
    changes.sort_by_key(|c| std::cmp::Reverse((c.after as i128 - c.before as i128).unsigned_abs()));
    changes.truncate(100);
    changes
}

fn parent(path: &str) -> &str {
    path.rsplit_once('\\').map_or("", |(p, _)| p)
}

/// The history file next to a snapshot.
pub fn path_for(snapshot: &Path) -> PathBuf {
    snapshot.with_extension("history")
}

pub fn load(path: &Path) -> Vec<Summary> {
    let Ok(bytes) = fs::read(path) else { return Vec::new() };
    zstd::decode_all(bytes.as_slice())
        .ok()
        .and_then(|raw| bincode::serde::decode_from_slice::<Vec<Summary>, _>(&raw, bincode::config::standard()).ok())
        .map(|(all, _)| all)
        .unwrap_or_default()
}

/// Adds an entry and keeps the last [`KEEP`].
pub fn append(path: &Path, summary: Summary) -> io::Result<()> {
    let mut all = load(path);
    all.push(summary);
    if all.len() > KEEP {
        all.drain(..all.len() - KEEP);
    }
    let raw = bincode::serde::encode_to_vec(&all, bincode::config::standard()).map_err(io::Error::other)?;
    let tmp = path.with_extension("history.tmp");
    fs::write(&tmp, zstd::encode_all(raw.as_slice(), 3)?)?;
    fs::rename(&tmp, path)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub scanned_at: i64,
    pub total: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryList {
    pub root: Option<String>,
    /// When the index being shown was scanned.
    pub current_at: Option<i64>,
    pub current_total: u64,
    /// Earlier scans to compare with, newest first.
    pub points: Vec<HistoryPoint>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDiff {
    pub before_at: i64,
    pub before_total: u64,
    pub after_total: u64,
    /// Paths are absolute.
    pub changes: Vec<Change>,
}

#[tauri::command]
pub fn history_list(session: tauri::State<'_, crate::session::Session>) -> HistoryList {
    let Some((root, current_at, now)) = session.current_summary() else {
        return HistoryList { root: None, current_at: None, current_total: 0, points: Vec::new() };
    };
    let mut points: Vec<HistoryPoint> = session
        .history()
        .into_iter()
        .filter(|s| s.scanned_at < current_at)
        .map(|s| HistoryPoint { scanned_at: s.scanned_at, total: s.total })
        .collect();
    points.reverse();
    HistoryList { root: Some(root), current_at: Some(current_at), current_total: now.total, points }
}

#[tauri::command]
pub fn history_diff(session: tauri::State<'_, crate::session::Session>, scanned_at: i64) -> Result<HistoryDiff, String> {
    let (root, _, now) = session.current_summary().ok_or("Nothing scanned yet.")?;
    let before = session.history().into_iter().find(|s| s.scanned_at == scanned_at).ok_or("That scan is no longer in the history.")?;
    let changes = diff(&before, &now)
        .into_iter()
        .map(|c| Change { path: Path::new(&root).join(&c.path).display().to_string(), ..c })
        .collect();
    Ok(HistoryDiff { before_at: before.scanned_at, before_total: before.total, after_total: now.total, changes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{DirTree, FileRec};

    const MB: u64 = 1 << 20;

    fn file(name: &str, alloc: u64) -> FileRec {
        FileRec { name: name.into(), size: alloc, alloc, modified: 0 }
    }

    fn dir(name: &str, files: Vec<FileRec>, dirs: Vec<DirTree>) -> DirTree {
        DirTree { name: name.into(), files, dirs, ..Default::default() }
    }

    fn summary(at: i64, folders: &[(&str, u64)]) -> Summary {
        Summary { scanned_at: at, total: folders.iter().find(|f| f.0.is_empty()).map_or(0, |f| f.1), folders: folders.iter().map(|(p, s)| (p.to_string(), *s)).collect() }
    }

    #[test]
    fn summarizes_folders_down_to_the_threshold() {
        let index = Index::build(dir(
            r"C:\",
            vec![file("big.bin", 10 * MB)],
            vec![dir("Users", vec![file("a", 6 * MB)], vec![dir("tiny", vec![file("b", MB)], vec![])])],
        ));
        let mut s = summarize(&index, 7);
        s.folders.sort();
        assert_eq!(s.total, 17 * MB);
        assert_eq!(s.folders, vec![(String::new(), 17 * MB), ("Users".to_string(), 7 * MB)]);
    }

    #[test]
    fn reports_the_most_specific_folders() {
        let before = summary(1, &[("", 1000 * MB), ("Users", 600 * MB), (r"Users\a", 500 * MB), (r"Users\a\Docker", 100 * MB), ("Games", 300 * MB)]);
        let after = summary(2, &[("", 1400 * MB), ("Users", 1000 * MB), (r"Users\a", 900 * MB), (r"Users\a\Docker", 480 * MB), ("Games", 200 * MB), ("New", 200 * MB)]);
        let changes = diff(&before, &after);
        let paths: Vec<_> = changes.iter().map(|c| c.path.as_str()).collect();
        // Users and Users\a are explained by Docker; Games shrank; New appeared.
        assert_eq!(paths, [r"Users\a\Docker", "New", "Games"]);
        assert_eq!((changes[0].before, changes[0].after), (100 * MB, 480 * MB));
    }

    #[test]
    fn keeps_a_folder_whose_growth_is_spread_out() {
        let before = summary(1, &[("", 0), ("Photos", 100 * MB)]);
        let after = summary(2, &[("", 0), ("Photos", 400 * MB)]);
        assert_eq!(diff(&before, &after)[0].path, "Photos");
    }

    #[test]
    fn appends_and_keeps_the_last_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("root.history");
        for at in 0..15 {
            append(&path, summary(at, &[("", at as u64)])).unwrap();
        }
        let all = load(&path);
        assert_eq!(all.len(), KEEP);
        assert_eq!(all[0].scanned_at, 3);
        assert_eq!(all.last().unwrap().scanned_at, 14);
        assert!(load(&dir.path().join("missing.history")).is_empty());
    }
}
