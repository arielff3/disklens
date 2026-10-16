//! The last line of defence: folders Clean up must never empty.

use crate::system_data::expand::{expand, Lookup};
use std::path::{Component, Path, PathBuf};

/// Folders that are never emptied, nor any folder that contains them.
const PROTECTED: &[&str] = &[
    r"%SystemDrive%\",
    r"%SystemRoot%",
    r"%SystemRoot%\System32",
    r"%ProgramFiles%",
    r"%ProgramFiles(x86)%",
    r"%ProgramData%",
    r"%SystemDrive%\Users",
    r"%USERPROFILE%",
    r"%USERPROFILE%\AppData",
    r"%APPDATA%",
    r"%LOCALAPPDATA%",
    r"%USERPROFILE%\Documents",
    r"%USERPROFILE%\Desktop",
    r"%USERPROFILE%\Downloads",
    r"%USERPROFILE%\Pictures",
    r"%USERPROFILE%\Videos",
    r"%USERPROFILE%\Music",
];

/// Ok if Clean up may empty `path`; otherwise why not.
pub fn check(path: &Path, lookup: &Lookup) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(format!("{} is not an absolute path", path.display()));
    }
    let depth = path.components().filter(|c| matches!(c, Component::Normal(_))).count();
    if depth < 2 {
        return Err(format!("{} is too close to the drive root", path.display()));
    }
    let candidate = normalized(path);
    let known_folders = [
        dirs::document_dir(),
        dirs::desktop_dir(),
        dirs::download_dir(),
        dirs::picture_dir(),
        dirs::video_dir(),
        dirs::audio_dir(),
    ];
    let protected = PROTECTED
        .iter()
        .filter_map(|t| expand(t, lookup))
        .chain(known_folders.into_iter().flatten());
    for root in protected {
        if normalized(&root).starts_with(&candidate) {
            return Err(format!("{} is or contains a protected folder ({})", path.display(), root.display()));
        }
    }
    Ok(())
}

/// Lowercased, so `Path::starts_with` compares like Windows does.
fn normalized(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(name: &str) -> Option<String> {
        Some(
            match name {
                "SystemDrive" => "C:",
                "SystemRoot" => r"C:\Windows",
                "ProgramFiles" => r"C:\Program Files",
                "ProgramFiles(x86)" => r"C:\Program Files (x86)",
                "ProgramData" => r"C:\ProgramData",
                "USERPROFILE" => r"C:\Users\a",
                "APPDATA" => r"C:\Users\a\AppData\Roaming",
                "LOCALAPPDATA" => r"C:\Users\a\AppData\Local",
                _ => return None,
            }
            .to_string(),
        )
    }

    #[test]
    fn refuses_protected_folders_and_their_parents() {
        for path in [
            r"C:\",
            r"C:\x",
            r"C:\Windows",
            r"c:\windows\system32",
            r"C:\Users",
            r"C:\Users\a",
            r"C:\Users\A\AppData",
            r"C:\Users\a\AppData\Local",
            r"C:\Users\a\Documents",
            r"C:\Program Files (x86)",
            "relative\\path",
        ] {
            assert!(check(Path::new(path), &lookup).is_err(), "{path} should be refused");
        }
    }

    #[test]
    fn allows_caches_and_temp_folders() {
        for path in [
            r"C:\Users\a\AppData\Local\Temp",
            r"C:\Windows\Temp",
            r"C:\Windows\SoftwareDistribution\Download",
            r"C:\Users\a\AppData\Local\Google\Chrome\User Data\Default\Cache",
            r"C:\Program Files (x86)\Steam\steamapps\shadercache",
        ] {
            assert_eq!(check(Path::new(path), &lookup), Ok(()), "{path} should be allowed");
        }
    }
}
