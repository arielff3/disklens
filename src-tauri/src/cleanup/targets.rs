//! Where an item's data lives on disk, and what inside it gets removed.

use crate::system_data::catalog::{ItemDef, Probe};
use crate::system_data::expand::{expand, Lookup};
use std::fs;
use std::path::{Path, PathBuf};

/// The existing files and folders an item is measured from.
pub fn roots(def: &ItemDef, lookup: &Lookup) -> Vec<PathBuf> {
    let candidates: Vec<PathBuf> = match &def.probe {
        Probe::Paths(templates) => templates.iter().filter_map(|t| expand(t, lookup)).collect(),
        Probe::Each { base, subs } => match expand(base, lookup).map(fs::read_dir) {
            Some(Ok(profiles)) => profiles
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink()))
                .flat_map(|e| subs.iter().map(move |sub| e.path().join(sub)))
                .collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    candidates
        .into_iter()
        .filter(|p| fs::symlink_metadata(p).is_ok_and(|m| !m.file_type().is_symlink()))
        .collect()
}

/// What to remove for one root: everything directly inside a folder (the folder
/// itself stays, so its app keeps working), or the file itself.
pub fn entries(root: &Path) -> Vec<PathBuf> {
    match fs::symlink_metadata(root) {
        Ok(meta) if meta.is_dir() => fs::read_dir(root)
            .map(|items| items.flatten().map(|e| e.path()).collect())
            .unwrap_or_default(),
        Ok(_) => vec![root.to_path_buf()],
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_data::catalog::{Group, Verdict};

    fn write(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }

    fn lookup_for(root: &Path) -> impl Fn(&str) -> Option<String> + Send + Sync {
        let root = root.to_string_lossy().into_owned();
        move |name: &str| (name == "ROOT").then(|| root.clone())
    }

    static PATHS: ItemDef = ItemDef {
        id: "t",
        group: Group::TempLogs,
        verdict: Verdict::SafeToClear,
        probe: Probe::Paths(&[r"%ROOT%\cache", r"%ROOT%\dump.dmp", r"%ROOT%\missing", r"%NOPE%\x"]),
    };

    static EACH: ItemDef = ItemDef {
        id: "e",
        group: Group::AppCaches,
        verdict: Verdict::SafeToClear,
        probe: Probe::Each { base: r"%ROOT%\User Data", subs: &["Cache"] },
    };

    #[test]
    fn roots_keep_only_what_exists() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("cache").join("a"));
        write(&dir.path().join("dump.dmp"));
        let lookup = lookup_for(dir.path());
        assert_eq!(roots(&PATHS, &lookup), vec![dir.path().join("cache"), dir.path().join("dump.dmp")]);
    }

    #[test]
    fn roots_of_each_are_the_profiles_caches() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("User Data");
        write(&base.join("Default").join("Cache").join("a"));
        write(&base.join("Profile 1").join("Cache").join("b"));
        write(&base.join("Profile 2").join("Bookmarks"));
        let mut found = roots(&EACH, &lookup_for(dir.path()));
        found.sort();
        assert_eq!(found, vec![base.join("Default").join("Cache"), base.join("Profile 1").join("Cache")]);
    }

    #[test]
    fn entries_are_a_folders_children_or_the_file_itself() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("cache").join("a"));
        write(&dir.path().join("cache").join("sub").join("b"));
        write(&dir.path().join("dump.dmp"));
        let mut inside = entries(&dir.path().join("cache"));
        inside.sort();
        assert_eq!(inside, vec![dir.path().join("cache").join("a"), dir.path().join("cache").join("sub")]);
        assert_eq!(entries(&dir.path().join("dump.dmp")), vec![dir.path().join("dump.dmp")]);
        assert!(entries(&dir.path().join("missing")).is_empty());
    }
}
