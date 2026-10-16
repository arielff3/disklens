//! Deleting big folder trees quickly.
//!
//! `std::fs::remove_dir_all` deletes one file at a time; each delete is a
//! synchronous round trip through NTFS and antivirus filters, so a
//! `node_modules` folder takes about a minute while the disk sits mostly idle.
//! This removes the files of each folder in parallel instead.

use rayon::prelude::*;
use std::fs;
use std::io;
use std::path::Path;

/// Permanently deletes `path` and everything below it. Junctions and symbolic
/// links are removed as links: their targets are never touched.
pub fn remove_tree(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return remove_link(path);
    }
    if !meta.is_dir() {
        return remove_file(path);
    }
    let entries: Vec<fs::DirEntry> = fs::read_dir(path)?.flatten().collect();
    entries.par_iter().for_each(|entry| {
        let child = entry.path();
        // Errors surface below: the folder can't be removed while anything is left in it.
        let _ = match entry.file_type() {
            Ok(t) if t.is_symlink() => remove_link(&child),
            Ok(t) if t.is_dir() => remove_tree(&child),
            _ => remove_file(&child),
        };
    });
    fs::remove_dir(path).or_else(|e| {
        clear_read_only(path);
        fs::remove_dir(path).map_err(|_| e)
    })
}

fn remove_file(path: &Path) -> io::Result<()> {
    fs::remove_file(path).or_else(|e| {
        clear_read_only(path);
        fs::remove_file(path).map_err(|_| e)
    })
}

/// A junction or directory symlink is removed with `remove_dir`, a file symlink with `remove_file`.
fn remove_link(path: &Path) -> io::Result<()> {
    fs::remove_dir(path).or_else(|_| fs::remove_file(path))
}

fn clear_read_only(path: &Path) {
    if let Ok(meta) = fs::symlink_metadata(path) {
        let mut permissions = meta.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        let _ = fs::set_permissions(path, permissions);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn removes_nested_trees_and_read_only_files() {
        let dir = tempfile::tempdir().unwrap();
        let tree = dir.path().join("node_modules");
        for i in 0..20 {
            let sub = tree.join(format!("pkg{i}")).join("lib");
            fs::create_dir_all(&sub).unwrap();
            fs::write(sub.join("index.js"), b"x").unwrap();
        }
        let locked = tree.join("pkg0").join("readonly.txt");
        fs::write(&locked, b"x").unwrap();
        let mut perms = fs::metadata(&locked).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&locked, perms).unwrap();

        remove_tree(&tree).unwrap();
        assert!(!tree.exists());
    }

    #[test]
    fn never_follows_junctions() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();
        fs::write(store.join("shared.js"), b"keep me").unwrap();
        let tree = dir.path().join("node_modules");
        fs::create_dir_all(&tree).unwrap();
        let out = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(tree.join("linked"))
            .arg(&store)
            .output()
            .unwrap();
        assert!(out.status.success(), "mklink failed: {out:?}");

        remove_tree(&tree).unwrap();
        assert!(!tree.exists());
        assert_eq!(fs::read(store.join("shared.js")).unwrap(), b"keep me");
    }

    #[test]
    fn missing_paths_are_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(remove_tree(&dir.path().join("nope")).is_err());
    }
}
