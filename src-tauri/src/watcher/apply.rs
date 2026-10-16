//! Turns a batch of changed paths into index patches by re-reading each path.
//!
//! Two steps, so the slow part doesn't block readers: [`probe`] reads the disk
//! and works out the changes against an unchanged index; [`commit`] applies them.

use crate::index::{DirTree, FileRec, Index, NodeId};
use crate::scanner::{scan_dir, stat_path, Entry};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// What a batch does to the index, worked out from the disk.
#[derive(Default)]
pub struct Changes {
    ops: Vec<Op>,
}

enum Op {
    Remove(NodeId),
    SetFile(NodeId, FileRec),
    SetModified(NodeId, i64),
    AddFile(NodeId, FileRec),
    AddDir(NodeId, DirTree),
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// Re-stats `paths` and patches `index` to match the disk. Paths outside the
/// scanned root, and the root itself, are ignored. Returns true if the index changed.
pub fn apply(index: &mut Index, paths: &[PathBuf], cluster: u64) -> bool {
    let changes = probe(index, paths, cluster);
    commit(index, changes)
}

/// Reads the disk for `paths` and decides how the index must change. Ids in
/// the result refer to `index` and stay valid until it is replaced.
pub fn probe(index: &Index, paths: &[PathBuf], cluster: u64) -> Changes {
    // Parents before children, so a removed or newly walked folder covers
    // every event below it.
    let mut paths: Vec<(String, String)> = paths
        .iter()
        .map(|p| {
            let s = p.to_string_lossy().replace('/', "\\");
            (fold(&s), s)
        })
        .collect();
    paths.sort_by(|a, b| a.0.len().cmp(&b.0.len()).then_with(|| a.0.cmp(&b.0)));
    paths.dedup_by(|a, b| a.0 == b.0);

    let mut covered: HashSet<String> = HashSet::new();
    let mut lookup = Lookup::default();
    let mut ops = Vec::new();
    for (key, path) in paths {
        if is_covered(&key, &covered) {
            continue;
        }
        let path = Path::new(&path);
        let Some((near, missing)) = lookup.locate(index, path) else { continue };
        let Some(node) = index.node(near) else { continue };
        match missing {
            None if near == index.root() => {}
            None => {
                if probe_existing(index, near, path, cluster, &mut ops) {
                    covered.insert(key);
                }
            }
            Some(_) if !node.is_dir => {}
            Some(part) => {
                // The first missing component decides: add it whole, or nothing.
                let child = Path::new(&index.path_of(near)).join(&part);
                match stat_path(&child, cluster) {
                    Ok(Some(Entry::File(rec))) => ops.push(Op::AddFile(near, rec)),
                    Ok(Some(Entry::Dir { .. })) => ops.push(Op::AddDir(near, scan_dir(&child, cluster))),
                    Ok(None) | Err(_) => continue,
                }
                covered.insert(fold(&child.to_string_lossy()));
            }
        }
    }
    Changes { ops }
}

/// Applies changes worked out by [`probe`] on this same index. Returns true if
/// anything changed.
pub fn commit(index: &mut Index, changes: Changes) -> bool {
    let changed = !changes.ops.is_empty();
    for op in changes.ops {
        match op {
            Op::Remove(id) => index.remove(id),
            Op::SetFile(id, rec) => index.set_file(id, &rec),
            Op::SetModified(id, modified) => index.set_modified(id, modified),
            Op::AddFile(parent, rec) if index.is_live(parent) => {
                index.add_file(parent, rec);
            }
            Op::AddDir(parent, tree) if index.is_live(parent) => {
                index.graft_dir(parent, tree);
            }
            Op::AddFile(..) | Op::AddDir(..) => {}
        }
    }
    changed
}

/// `path` is in the index as `id`; queue what brings that node up to date.
/// Returns true if everything below `path` is now handled.
fn probe_existing(index: &Index, id: NodeId, path: &Path, cluster: u64, ops: &mut Vec<Op>) -> bool {
    let Some(node) = index.node(id) else { return false };
    match stat_path(path, cluster) {
        Ok(None) => {
            ops.push(Op::Remove(id));
            true
        }
        Ok(Some(Entry::File(rec))) if !node.is_dir => {
            if (node.size, node.alloc, node.modified) != (rec.size, rec.alloc, rec.modified) {
                ops.push(Op::SetFile(id, rec));
            }
            false
        }
        Ok(Some(Entry::File(rec))) => {
            ops.push(Op::Remove(id));
            ops.push(Op::AddFile(node.parent, rec));
            true
        }
        Ok(Some(Entry::Dir { modified })) if node.is_dir => {
            // Folder events come with an event per changed child; just refresh its time.
            if node.modified != modified {
                ops.push(Op::SetModified(id, modified));
            }
            false
        }
        Ok(Some(Entry::Dir { .. })) => {
            ops.push(Op::Remove(id));
            ops.push(Op::AddDir(node.parent, scan_dir(path, cluster)));
            true
        }
        // Can't read it right now (locked, access denied): keep what we have.
        Err(_) => false,
    }
}

/// True if an ancestor of `key` was already removed or walked in this batch.
fn is_covered(key: &str, covered: &HashSet<String>) -> bool {
    key.match_indices('\\').any(|(i, _)| covered.contains(&key[..i]))
}

/// Path lookups for one batch. A burst of events usually lands in a few
/// folders, possibly huge ones, so each folder is resolved once and its
/// children are indexed by name the first time it is touched; walking the
/// children for every event would be quadratic.
#[derive(Default)]
struct Lookup {
    /// Lowercase folder path → node.
    dirs: HashMap<String, Option<NodeId>>,
    /// Folder → lowercase child name → node.
    names: HashMap<NodeId, HashMap<String, NodeId>>,
}

impl Lookup {
    /// Same answer as [`Index::locate`], fast for paths whose folder is in the index.
    fn locate(&mut self, index: &Index, path: &Path) -> Option<(NodeId, Option<String>)> {
        let fast = (|| {
            let name = path.file_name()?.to_string_lossy();
            let dir = self.dir(index, path.parent()?)?;
            let children = self.names.entry(dir).or_insert_with(|| {
                let children = index.node(dir).map_or(&[][..], |n| &n.children[..]);
                children.iter().filter_map(|&c| Some((fold(&index.node(c)?.name), c))).collect()
            });
            Some(match children.get(&fold(&name)) {
                Some(&child) => (child, None),
                None => (dir, Some(name.into_owned())),
            })
        })();
        // The root, paths outside it, and folders not in the index.
        fast.or_else(|| index.locate(&path.to_string_lossy()))
    }

    /// The folder node at exactly `path`, if the index has one.
    fn dir(&mut self, index: &Index, path: &Path) -> Option<NodeId> {
        let key = fold(&path.to_string_lossy());
        *self.dirs.entry(key).or_insert_with(|| match index.locate(&path.to_string_lossy())? {
            (id, None) if index.node(id).is_some_and(|n| n.is_dir) => Some(id),
            _ => None,
        })
    }
}

/// Case-folds a name or path for lookups (no Unicode tables for ASCII).
fn fold(s: &str) -> String {
    if s.is_ascii() {
        s.to_ascii_lowercase()
    } else {
        s.to_lowercase()
    }
}
