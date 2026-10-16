//! Finding the folders where an app keeps its data, strictly by name.

use super::names::{app_keys, is_generic, normalize, publisher_keys};
use crate::cleanup::safety;
use crate::system_data::expand::{expand, Lookup};
use std::fs;
use std::path::{Path, PathBuf};

/// Where apps keep data. Searched one level deep, or two for `Publisher\App`.
const ROOTS: &[&str] = &[r"%APPDATA%", r"%LOCALAPPDATA%", r"%USERPROFILE%\AppData\LocalLow", r"%ProgramData%"];

/// What an app is called and where it is installed.
pub struct Identity<'a> {
    pub name: &'a str,
    pub publisher: Option<&'a str>,
    pub install_location: Option<&'a Path>,
}

/// Folders that belong to this app and nothing else. `other_installs` are the
/// install folders of every other listed app: a match that is, or contains,
/// one of them is dropped.
pub fn data_folders(app: &Identity, lookup: &Lookup, other_installs: &[PathBuf]) -> Vec<PathBuf> {
    let mut keys = app_keys(app.name, app.publisher);
    if let Some(folder) = app.install_location.and_then(Path::file_name) {
        let key = normalize(&folder.to_string_lossy());
        if !is_generic(&key) && !keys.contains(&key) {
            keys.push(key);
        }
    }
    if keys.is_empty() {
        return Vec::new();
    }
    let publishers = app.publisher.map(publisher_keys).unwrap_or_default();

    let mut found = Vec::new();
    for root in ROOTS.iter().filter_map(|t| expand(t, lookup)) {
        for child in subdirs(&root) {
            let key = key_of(&child);
            if keys.contains(&key) {
                found.push(child);
            } else if publishers.contains(&key) {
                found.extend(subdirs(&child).into_iter().filter(|inner| keys.contains(&key_of(inner))));
            }
        }
    }
    found.retain(|path| {
        safety::check(path, lookup).is_ok()
            && app.install_location.is_none_or(|own| !same(own, path))
            && !other_installs.iter().any(|other| contains(path, other))
    });
    found.sort();
    found.dedup();
    found
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink()))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default()
}

fn key_of(path: &Path) -> String {
    path.file_name().map(|n| normalize(&n.to_string_lossy())).unwrap_or_default()
}

fn lower(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_lowercase())
}

fn same(a: &Path, b: &Path) -> bool {
    lower(a) == lower(b)
}

/// True if `outer` is `inner` or one of its parents.
fn contains(outer: &Path, inner: &Path) -> bool {
    lower(inner).starts_with(lower(outer))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _dir: tempfile::TempDir,
        root: PathBuf,
    }

    impl Fixture {
        fn new(folders: &[&str]) -> Fixture {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().to_path_buf();
            for folder in folders {
                fs::create_dir_all(root.join(folder)).unwrap();
            }
            Fixture { _dir: dir, root }
        }

        fn lookup(&self) -> impl Fn(&str) -> Option<String> + Send + Sync {
            let root = self.root.to_string_lossy().into_owned();
            move |name: &str| {
                Some(match name {
                    "APPDATA" => format!(r"{root}\Roaming"),
                    "LOCALAPPDATA" => format!(r"{root}\Local"),
                    "USERPROFILE" => root.clone(),
                    "ProgramData" => format!(r"{root}\ProgramData"),
                    _ => return None,
                })
            }
        }

        fn path(&self, rel: &str) -> PathBuf {
            self.root.join(rel)
        }
    }

    fn rel(f: &Fixture, found: Vec<PathBuf>) -> Vec<String> {
        found.iter().map(|p| p.strip_prefix(&f.root).unwrap().to_string_lossy().replace('\\', "/")).collect()
    }

    #[test]
    fn finds_folders_named_after_the_app() {
        let f = Fixture::new(&["Roaming/discord", "Local/Discord", "Local/DiscordPTB", "Roaming/Microsoft", "ProgramData/Other"]);
        let app = Identity { name: "Discord", publisher: Some("Discord Inc."), install_location: None };
        assert_eq!(rel(&f, data_folders(&app, &f.lookup(), &[])), ["Local/Discord", "Roaming/discord"]);
    }

    #[test]
    fn looks_inside_publisher_folders_but_never_takes_them_whole() {
        let f = Fixture::new(&["Local/Google/Chrome", "Local/Google/Drive", "Roaming/Microsoft/Teams", "Local/Microsoft/Edge"]);
        let chrome = Identity { name: "Google Chrome", publisher: Some("Google LLC"), install_location: None };
        assert_eq!(rel(&f, data_folders(&chrome, &f.lookup(), &[])), ["Local/Google/Chrome"]);
        let teams = Identity { name: "Microsoft Teams", publisher: Some("Microsoft Corporation"), install_location: None };
        assert_eq!(rel(&f, data_folders(&teams, &f.lookup(), &[])), ["Roaming/Microsoft/Teams"]);
    }

    #[test]
    fn matches_the_install_folders_name_and_skips_the_install_itself() {
        let f = Fixture::new(&["Local/Programs/Microsoft VS Code", "Roaming/Code", "Roaming/Microsoft VS Code"]);
        let install = f.path("Local/Programs/Microsoft VS Code");
        let app = Identity { name: "Microsoft Visual Studio Code (User)", publisher: Some("Microsoft Corporation"), install_location: Some(&install) };
        assert_eq!(rel(&f, data_folders(&app, &f.lookup(), &[])), ["Roaming/Microsoft VS Code"]);
    }

    #[test]
    fn never_takes_generic_folders_or_other_apps_installs() {
        let f = Fixture::new(&["Local/Temp", "Roaming/Packages", "Local/Slack"]);
        let generic = Identity { name: "Temp", publisher: None, install_location: None };
        assert!(data_folders(&generic, &f.lookup(), &[]).is_empty());
        let slack = Identity { name: "Slack", publisher: Some("Slack Technologies"), install_location: None };
        let other = vec![f.path("Local/Slack")];
        assert!(data_folders(&slack, &f.lookup(), &other).is_empty());
    }
}
