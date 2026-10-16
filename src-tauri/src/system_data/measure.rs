//! Measures files and folders on disk for System Data probes.

use super::expand::{expand, Lookup};
use super::{ItemState, Measured};
use crate::index::DirTree;
use crate::scanner::{file_rec, scan_dir};
use crate::win;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Running totals for one item.
#[derive(Default)]
pub struct Acc {
    alloc: u64,
    files: u64,
    /// Folders below a measured folder that could not be read.
    unreadable: u64,
    /// Something we were asked to measure could not be read at all.
    denied: bool,
    /// What *Open* and *Reveal* point at: the first thing found.
    path: Option<PathBuf>,
    is_dir: bool,
}

impl Acc {
    /// Adds a file, or a folder and everything below it, if it exists.
    pub fn add_path(&mut self, path: &Path) {
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else { return };
        // Read the entry from its parent's listing: that works even for files
        // Windows keeps locked, such as pagefile.sys.
        let entries = match fs::read_dir(parent) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return,
            Err(_) => {
                self.denied = true;
                return;
            }
        };
        let Some(entry) = entries.flatten().find(|e| e.file_name().eq_ignore_ascii_case(name)) else { return };
        let Ok(file_type) = entry.file_type() else { return };
        if file_type.is_symlink() {
            return;
        }
        let cluster = win::cluster_size(path);
        if file_type.is_dir() {
            let tree = scan_dir(path, cluster);
            if tree.unreadable {
                self.denied = true;
            }
            let (alloc, files, unreadable) = totals(&tree);
            self.alloc += alloc;
            self.files += files;
            self.unreadable += unreadable;
            self.found(path, true);
        } else if let Ok(meta) = entry.metadata() {
            self.alloc += file_rec(String::new(), &meta, path, cluster).alloc;
            self.files += 1;
            self.found(path, false);
        }
    }

    /// Adds every file with extension `ext` in `dir`, looking at most `depth` folders down.
    pub fn find(&mut self, dir: &Path, ext: &str, depth: usize) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else { continue };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                if depth > 0 {
                    self.find(&path, ext, depth - 1);
                }
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
                if let Ok(meta) = entry.metadata() {
                    self.alloc += file_rec(String::new(), &meta, &path, win::cluster_size(&path)).alloc;
                    self.files += 1;
                    self.found(&path, false);
                }
            }
        }
    }

    fn found(&mut self, path: &Path, is_dir: bool) {
        if self.path.is_none() {
            self.path = Some(path.to_path_buf());
            self.is_dir = is_dir;
        }
    }

    pub fn finish(self) -> Measured {
        let state = match (self.path.is_some(), self.denied) {
            (false, false) => ItemState::Missing,
            (_, true) if self.alloc == 0 => ItemState::NeedsAdmin,
            (_, true) => ItemState::Partial,
            _ if self.unreadable > 0 => ItemState::Partial,
            _ => ItemState::Ok,
        };
        Measured {
            state,
            alloc: self.alloc,
            files: self.files,
            path: self.path.map(|p| p.to_string_lossy().into_owned()),
            is_dir: self.is_dir,
            note: None,
        }
    }
}

/// The given subfolders of every folder inside `base`; *Open* points at `base`.
pub fn measure_each(base: &str, subs: &[&str], lookup: &Lookup) -> Measured {
    let Some(base) = expand(base, lookup) else { return Measured::with_state(ItemState::Missing) };
    let mut acc = Acc::default();
    match fs::read_dir(&base) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink()) {
                    for sub in subs {
                        acc.add_path(&entry.path().join(sub));
                    }
                }
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Measured::with_state(ItemState::Missing),
        Err(_) => acc.denied = true,
    }
    if acc.path.is_some() {
        acc.path = Some(base);
        acc.is_dir = true;
    }
    acc.finish()
}

/// Allocated bytes, files, and unreadable folders below the root.
pub(crate) fn totals(tree: &DirTree) -> (u64, u64, u64) {
    let mut alloc: u64 = tree.files.iter().map(|f| f.alloc).sum();
    let mut files = tree.files.len() as u64;
    let mut unreadable = 0;
    for dir in &tree.dirs {
        let (a, f, u) = totals(dir);
        alloc += a;
        files += f;
        unreadable += u + u64::from(dir.unreadable);
    }
    (alloc, files, unreadable)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, len: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![1u8; len]).unwrap();
    }

    #[test]
    fn missing_paths_are_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut acc = Acc::default();
        acc.add_path(&dir.path().join("nope"));
        acc.add_path(&dir.path().join("also").join("nope"));
        assert_eq!(acc.finish().state, ItemState::Missing);
    }

    #[test]
    fn sums_files_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.bin");
        write(&file, 5000);
        write(&dir.path().join("cache").join("x").join("y.bin"), 100);
        let mut acc = Acc::default();
        acc.add_path(&file);
        acc.add_path(&dir.path().join("cache"));
        let m = acc.finish();
        assert_eq!(m.state, ItemState::Ok);
        assert_eq!(m.files, 2);
        assert!(m.alloc >= 5100);
        assert_eq!(m.path.as_deref(), Some(&*file.to_string_lossy()));
        assert!(!m.is_dir);
    }

    #[test]
    fn matches_names_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("Cache").join("f.bin"), 10);
        let mut acc = Acc::default();
        acc.add_path(&dir.path().join("cache"));
        assert_eq!(acc.finish().files, 1);
    }

    #[test]
    fn each_measures_the_subfolders_of_every_profile() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("User Data");
        write(&base.join("Default").join("Cache").join("a"), 1000);
        write(&base.join("Profile 1").join("Cache").join("b"), 2000);
        write(&base.join("Profile 1").join("Bookmarks").join("c"), 9000);
        let root = base.to_string_lossy().into_owned();
        let lookup = move |name: &str| (name == "BASE").then(|| root.clone());
        let m = measure_each("%BASE%", &["Cache"], &lookup);
        assert_eq!(m.state, ItemState::Ok);
        assert_eq!(m.files, 2);
        assert_eq!(m.path.as_deref(), Some(&*base.to_string_lossy()));
        assert!(m.is_dir);
        assert_eq!(measure_each("%NOPE%", &["Cache"], &lookup).state, ItemState::Missing);
    }

    #[test]
    fn find_looks_a_bounded_number_of_levels_down() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("a").join("b").join("disk.VHDX"), 10);
        write(&dir.path().join("a").join("b").join("c").join("d").join("deep.vhdx"), 10);
        write(&dir.path().join("a").join("notes.txt"), 10);
        let mut acc = Acc::default();
        acc.find(dir.path(), "vhdx", 2);
        let m = acc.finish();
        assert_eq!(m.files, 1);
        assert!(m.path.unwrap().ends_with("disk.VHDX"));
    }
}
