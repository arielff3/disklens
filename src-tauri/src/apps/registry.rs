//! Classic programs from the registry's uninstall lists.

use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
use winreg::{RegKey, HKEY};

const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

/// Where an entry was found: (label used in ids, hive, access flags).
const SOURCES: [(&str, HKEY, u32); 3] = [
    ("HKLM", HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_64KEY),
    ("HKLM32", HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY),
    ("HKCU", HKEY_CURRENT_USER, KEY_READ),
];

/// The values of one uninstall entry that matter here.
#[derive(Debug, Default, Clone)]
pub struct RawEntry {
    pub display_name: Option<String>,
    pub display_version: Option<String>,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
    /// Kilobytes.
    pub estimated_size: Option<u32>,
    pub system_component: Option<u32>,
    pub windows_installer: Option<u32>,
    pub parent_key_name: Option<String>,
    pub release_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// `<source>\<key name>`, e.g. `HKLM\{GUID}`.
    pub id: String,
    pub key: String,
    pub name: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    pub uninstall_string: String,
    pub estimated_bytes: Option<u64>,
    pub msi: bool,
}

fn clean(value: &Option<String>) -> Option<String> {
    value.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.trim_matches('"').to_string())
}

/// An entry worth listing, or `None` for system components, updates and
/// entries without a name or uninstaller.
pub fn parse(source: &str, key: &str, raw: &RawEntry) -> Option<Program> {
    let name = clean(&raw.display_name)?;
    let uninstall_string = raw.uninstall_string.as_deref().map(str::trim).filter(|s| !s.is_empty())?.to_string();
    if raw.system_component == Some(1) || clean(&raw.parent_key_name).is_some() {
        return None;
    }
    if let Some(kind) = clean(&raw.release_type) {
        if ["update", "hotfix", "security update", "service pack"].contains(&kind.to_ascii_lowercase().as_str()) {
            return None;
        }
    }
    Some(Program {
        id: format!(r"{source}\{key}"),
        key: key.to_string(),
        name,
        version: clean(&raw.display_version),
        publisher: clean(&raw.publisher),
        install_location: clean(&raw.install_location).map(|p| p.trim_end_matches('\\').to_string()).filter(|p| !p.is_empty()),
        uninstall_string,
        estimated_bytes: raw.estimated_size.filter(|&kb| kb > 0).map(|kb| kb as u64 * 1024),
        msi: raw.windows_installer == Some(1),
    })
}

/// Every listed program, without duplicates (same name and version in two lists).
pub fn read_all() -> Vec<Program> {
    let mut programs: Vec<Program> = Vec::new();
    for (label, hive, flags) in SOURCES {
        let Ok(list) = RegKey::predef(hive).open_subkey_with_flags(UNINSTALL, flags) else { continue };
        for key in list.enum_keys().flatten() {
            let Ok(entry) = list.open_subkey_with_flags(&key, flags) else { continue };
            let raw = RawEntry {
                display_name: entry.get_value("DisplayName").ok(),
                display_version: entry.get_value("DisplayVersion").ok(),
                publisher: entry.get_value("Publisher").ok(),
                install_location: entry.get_value("InstallLocation").ok(),
                uninstall_string: entry.get_value("UninstallString").ok(),
                estimated_size: entry.get_value("EstimatedSize").ok(),
                system_component: entry.get_value("SystemComponent").ok(),
                windows_installer: entry.get_value("WindowsInstaller").ok(),
                parent_key_name: entry.get_value("ParentKeyName").ok(),
                release_type: entry.get_value("ReleaseType").ok(),
            };
            if let Some(program) = parse(label, &key, &raw) {
                if !programs.iter().any(|p| p.name == program.name && p.version == program.version) {
                    programs.push(program);
                }
            }
        }
    }
    programs
}

/// Whether the uninstall entry `id` (from [`Program::id`]) still exists.
pub fn exists(id: &str) -> bool {
    let Some((label, key)) = id.split_once('\\') else { return false };
    let Some((_, hive, flags)) = SOURCES.iter().find(|(l, _, _)| *l == label) else { return false };
    RegKey::predef(*hive).open_subkey_with_flags(format!(r"{UNINSTALL}\{key}"), *flags).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(name: &str, uninstall: &str) -> RawEntry {
        RawEntry {
            display_name: Some(name.into()),
            uninstall_string: Some(uninstall.into()),
            ..Default::default()
        }
    }

    #[test]
    fn parses_a_normal_entry() {
        let entry = RawEntry {
            display_version: Some("1.0".into()),
            publisher: Some("Discord Inc.".into()),
            install_location: Some(r"C:\Users\a\AppData\Local\Discord\".into()),
            estimated_size: Some(2048),
            windows_installer: Some(0),
            ..raw("Discord", r#""C:\Users\a\AppData\Local\Discord\Update.exe" --uninstall"#)
        };
        let p = parse("HKCU", "Discord", &entry).unwrap();
        assert_eq!(p.id, r"HKCU\Discord");
        assert_eq!(p.install_location.as_deref(), Some(r"C:\Users\a\AppData\Local\Discord"));
        assert_eq!(p.estimated_bytes, Some(2048 * 1024));
        assert!(!p.msi);
    }

    #[test]
    fn hides_entries_that_are_not_user_apps() {
        assert!(parse("HKLM", "k", &RawEntry { display_name: None, ..raw("", "x") }).is_none());
        assert!(parse("HKLM", "k", &RawEntry { uninstall_string: None, ..raw("App", "") }).is_none());
        assert!(parse("HKLM", "k", &RawEntry { system_component: Some(1), ..raw("Runtime", "x") }).is_none());
        assert!(parse("HKLM", "k", &RawEntry { parent_key_name: Some("Office".into()), ..raw("Patch", "x") }).is_none());
        assert!(parse("HKLM", "k", &RawEntry { release_type: Some("Security Update".into()), ..raw("KB1", "x") }).is_none());
        assert!(parse("HKLM", "k", &raw("Real App", "x")).is_some());
    }

    #[test]
    fn reads_this_machines_programs() {
        // Every Windows machine has at least one listed program; mostly a smoke test for the registry calls.
        let programs = read_all();
        assert!(programs.iter().all(|p| !p.name.is_empty()));
        assert!(programs.iter().all(|p| exists(&p.id)), "every listed entry should be found again");
    }
}
