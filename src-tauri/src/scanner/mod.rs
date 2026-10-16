//! Scanners turn a directory on disk into an [`Index`].

mod walk;

pub use walk::{file_rec, scan_dir, stat_path, Entry, WalkScanner};

use crate::index::Index;
use serde::Serialize;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// Live counters a scanner updates while it runs.
#[derive(Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub bytes: AtomicU64,
    pub current: Mutex<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProgressSnapshot {
    pub files: u64,
    pub bytes: u64,
    pub current: String,
}

impl Progress {
    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            files: self.files.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            current: self.current.lock().map(|c| c.clone()).unwrap_or_default(),
        }
    }
}

/// Allocation unit of the volume holding `path` (4 KiB if unknown).
pub fn cluster_size(path: &Path) -> u64 {
    crate::win::cluster_size(path)
}

pub trait Scanner: Send + Sync {
    /// Scans `root`. Returns `Ok(None)` if `cancel` was set before the scan finished.
    fn scan(&self, root: &Path, progress: &Progress, cancel: &AtomicBool) -> io::Result<Option<Index>>;
}
