//! Parallel directory walk. Works on any file system and needs no admin rights.
//! On Windows, `read_dir` returns size and attributes from the directory
//! listing itself, so there is no extra system call per file.

use super::{Progress, Scanner};
use crate::index::{DirTree, FileRec, Index};
use crate::win;
use rayon::prelude::*;
use std::fs::{self, Metadata};
use std::io;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

// Attributes whose on-disk size differs from the cluster-rounded logical size.
const COMPRESSED: u32 = 0x800;
const SPARSE: u32 = 0x200;
const OFFLINE: u32 = 0x1000;
const RECALL_ON_OPEN: u32 = 0x4_0000;
const RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
const NEEDS_REAL_ALLOC: u32 = COMPRESSED | SPARSE | OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS;

pub struct WalkScanner;

struct Ctx<'a> {
    cluster: u64,
    progress: &'a Progress,
    cancel: &'a AtomicBool,
}

impl Scanner for WalkScanner {
    fn scan(&self, root: &Path, progress: &Progress, cancel: &AtomicBool) -> io::Result<Option<Index>> {
        let meta = fs::metadata(root)?;
        if !meta.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} is not a folder", root.display()),
            ));
        }
        let ctx = Ctx { cluster: win::cluster_size(root), progress, cancel };
        // Deep trees recurse deeply; give the workers generous stacks.
        let pool = rayon::ThreadPoolBuilder::new()
            .stack_size(16 << 20)
            .build()
            .map_err(io::Error::other)?;
        let mut tree = pool.install(|| walk_dir(root, &ctx));
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        tree.name = root.to_string_lossy().into_owned();
        tree.modified = unix_secs(&meta);
        Ok(Some(Index::build(tree)))
    }
}

fn walk_dir(path: &Path, ctx: &Ctx) -> DirTree {
    let mut tree = DirTree {
        name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        ..Default::default()
    };
    if ctx.cancel.load(Ordering::Relaxed) {
        return tree;
    }
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => {
            tree.unreadable = true;
            return tree;
        }
    };

    let mut subdirs: Vec<(PathBuf, i64)> = Vec::new();
    let mut bytes = 0;
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        // Junctions and symlinks point at data counted elsewhere: never follow or count them.
        if file_type.is_symlink() {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if file_type.is_dir() {
            subdirs.push((entry.path(), unix_secs(&meta)));
            continue;
        }
        let rec = file_rec(entry.file_name().to_string_lossy().into_owned(), &meta, &entry.path(), ctx.cluster);
        bytes += rec.alloc;
        tree.files.push(rec);
    }
    ctx.progress.files.fetch_add(tree.files.len() as u64, Ordering::Relaxed);
    ctx.progress.bytes.fetch_add(bytes, Ordering::Relaxed);
    if let Ok(mut current) = ctx.progress.current.try_lock() {
        *current = path.to_string_lossy().into_owned();
    }

    tree.dirs = subdirs
        .into_par_iter()
        .map(|(sub, modified)| {
            let mut d = walk_dir(&sub, ctx);
            d.modified = modified;
            d
        })
        .collect();
    tree
}

/// One path as the scanner would see it, without walking into folders.
pub enum Entry {
    File(FileRec),
    Dir { modified: i64 },
}

/// Re-reads one path. `None` if it is gone, or is a junction or symlink
/// (which the scanner never counts).
pub fn stat_path(path: &Path, cluster: u64) -> io::Result<Option<Entry>> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if meta.file_type().is_symlink() {
        return Ok(None);
    }
    if meta.is_dir() {
        return Ok(Some(Entry::Dir { modified: unix_secs(&meta) }));
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Some(Entry::File(file_rec(name, &meta, path, cluster))))
}

/// Walks one folder and everything below it, like a scan of that folder.
pub fn scan_dir(path: &Path, cluster: u64) -> DirTree {
    let progress = Progress::default();
    let cancel = AtomicBool::new(false);
    let mut tree = walk_dir(path, &Ctx { cluster, progress: &progress, cancel: &cancel });
    tree.modified = fs::metadata(path).map(|m| unix_secs(&m)).unwrap_or(0);
    tree
}

pub fn file_rec(name: String, meta: &Metadata, path: &Path, cluster: u64) -> FileRec {
    let size = meta.len();
    let alloc = if meta.file_attributes() & NEEDS_REAL_ALLOC != 0 {
        win::compressed_size(path).unwrap_or(size)
    } else {
        round_up(size, cluster)
    };
    FileRec { name, size, alloc, modified: unix_secs(meta) }
}

fn round_up(size: u64, cluster: u64) -> u64 {
    size.div_ceil(cluster) * cluster
}

fn unix_secs(meta: &Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::round_up;

    #[test]
    fn rounds_up_to_whole_clusters() {
        assert_eq!(round_up(0, 4096), 0);
        assert_eq!(round_up(1, 4096), 4096);
        assert_eq!(round_up(4096, 4096), 4096);
        assert_eq!(round_up(4097, 4096), 8192);
    }
}
