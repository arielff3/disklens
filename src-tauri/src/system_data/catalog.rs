//! Known places where Windows and common apps keep data, and how to measure each.
//! Titles and explanations live in the UI (`src/strings.ts`, `systemItems`), keyed by id.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    Windows,
    Developer,
    AppCaches,
    TempLogs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    /// Recreated or downloaded again when needed.
    SafeToClear,
    /// Removable, but only after a decision (rollback, hibernation, VM data).
    LookFirst,
    /// Windows manages it; never remove by hand.
    ManagedByWindows,
}

#[derive(Debug)]
pub enum Probe {
    /// Files or folders, each measured if it exists.
    Paths(&'static [&'static str]),
    /// The given subfolders of every folder inside `base` (each browser profile's cache).
    Each { base: &'static str, subs: &'static [&'static str] },
    /// Every file with extension `ext` in the bases, at most `depth` folders down.
    Find { bases: &'static [&'static str], ext: &'static str, depth: usize },
    RecycleBin,
    ShadowStorage,
    ComponentStore,
}

impl Probe {
    /// The path templates this probe expands.
    pub fn templates(&self) -> Vec<&'static str> {
        match self {
            Probe::Paths(paths) => paths.to_vec(),
            Probe::Each { base, .. } => vec![*base],
            Probe::Find { bases, .. } => bases.to_vec(),
            Probe::RecycleBin | Probe::ShadowStorage | Probe::ComponentStore => Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ItemDef {
    pub id: &'static str,
    pub group: Group,
    pub verdict: Verdict,
    pub probe: Probe,
}

const fn item(id: &'static str, group: Group, verdict: Verdict, probe: Probe) -> ItemDef {
    ItemDef { id, group, verdict, probe }
}

const BROWSER_CACHES: &[&str] = &["Cache", "Code Cache", "GPUCache"];

use Group::{AppCaches, Developer, TempLogs, Windows};
use Verdict::{LookFirst, ManagedByWindows, SafeToClear};

pub static CATALOG: &[ItemDef] = &[
    // Windows
    item("pagefile", Windows, ManagedByWindows, Probe::Paths(&[r"%SystemDrive%\pagefile.sys"])),
    item("swapfile", Windows, ManagedByWindows, Probe::Paths(&[r"%SystemDrive%\swapfile.sys"])),
    item("hiberfil", Windows, LookFirst, Probe::Paths(&[r"%SystemDrive%\hiberfil.sys"])),
    item("componentStore", Windows, ManagedByWindows, Probe::ComponentStore),
    item("windowsOld", Windows, LookFirst, Probe::Paths(&[r"%SystemDrive%\Windows.old"])),
    item("updateDownloads", Windows, SafeToClear, Probe::Paths(&[r"%SystemRoot%\SoftwareDistribution\Download"])),
    item(
        "deliveryOptimization",
        Windows,
        SafeToClear,
        Probe::Paths(&[r"%SystemRoot%\ServiceProfiles\NetworkService\AppData\Local\Microsoft\Windows\DeliveryOptimization\Cache"]),
    ),
    item("installerCache", Windows, ManagedByWindows, Probe::Paths(&[r"%SystemRoot%\Installer"])),
    item("restorePoints", Windows, LookFirst, Probe::ShadowStorage),
    item("recycleBin", Windows, LookFirst, Probe::RecycleBin),
    // Developer data
    item("npmCache", Developer, SafeToClear, Probe::Paths(&[r"%LOCALAPPDATA%\npm-cache"])),
    item("pnpmStore", Developer, LookFirst, Probe::Paths(&[r"%LOCALAPPDATA%\pnpm\store"])),
    item("yarnCache", Developer, SafeToClear, Probe::Paths(&[r"%LOCALAPPDATA%\Yarn\Cache"])),
    item("pipCache", Developer, SafeToClear, Probe::Paths(&[r"%LOCALAPPDATA%\pip\cache"])),
    item(
        "cargoRegistry",
        Developer,
        SafeToClear,
        Probe::Paths(&[r"%USERPROFILE%\.cargo\registry", r"%USERPROFILE%\.cargo\git"]),
    ),
    item("gradleCache", Developer, SafeToClear, Probe::Paths(&[r"%USERPROFILE%\.gradle\caches"])),
    item("mavenRepo", Developer, SafeToClear, Probe::Paths(&[r"%USERPROFILE%\.m2\repository"])),
    item(
        "nugetCache",
        Developer,
        SafeToClear,
        Probe::Paths(&[r"%USERPROFILE%\.nuget\packages", r"%LOCALAPPDATA%\NuGet\v3-cache"]),
    ),
    item(
        "wslDisks",
        Developer,
        LookFirst,
        Probe::Find { bases: &[r"%LOCALAPPDATA%\Packages", r"%LOCALAPPDATA%\wsl"], ext: "vhdx", depth: 3 },
    ),
    item("dockerDisk", Developer, LookFirst, Probe::Find { bases: &[r"%LOCALAPPDATA%\Docker"], ext: "vhdx", depth: 4 }),
    // App caches
    item(
        "chromeCache",
        AppCaches,
        SafeToClear,
        Probe::Each { base: r"%LOCALAPPDATA%\Google\Chrome\User Data", subs: BROWSER_CACHES },
    ),
    item(
        "edgeCache",
        AppCaches,
        SafeToClear,
        Probe::Each { base: r"%LOCALAPPDATA%\Microsoft\Edge\User Data", subs: BROWSER_CACHES },
    ),
    item(
        "braveCache",
        AppCaches,
        SafeToClear,
        Probe::Each { base: r"%LOCALAPPDATA%\BraveSoftware\Brave-Browser\User Data", subs: BROWSER_CACHES },
    ),
    item(
        "firefoxCache",
        AppCaches,
        SafeToClear,
        Probe::Each { base: r"%LOCALAPPDATA%\Mozilla\Firefox\Profiles", subs: &["cache2"] },
    ),
    item(
        "teamsCache",
        AppCaches,
        SafeToClear,
        Probe::Paths(&[r"%APPDATA%\Microsoft\Teams\Cache", r"%LOCALAPPDATA%\Packages\MSTeams_8wekyb3d8bbwe\LocalCache"]),
    ),
    item(
        "discordCache",
        AppCaches,
        SafeToClear,
        Probe::Paths(&[r"%APPDATA%\discord\Cache", r"%APPDATA%\discord\Code Cache"]),
    ),
    item(
        "spotifyCache",
        AppCaches,
        SafeToClear,
        Probe::Paths(&[
            r"%LOCALAPPDATA%\Spotify\Storage",
            r"%LOCALAPPDATA%\Packages\SpotifyAB.SpotifyMusic_zpdnekdrzrea0\LocalCache\Spotify\Data",
        ]),
    ),
    item(
        "steamShaderCache",
        AppCaches,
        SafeToClear,
        Probe::Paths(&[r"%ProgramFiles(x86)%\Steam\steamapps\shadercache"]),
    ),
    // Temp & logs
    item("userTemp", TempLogs, SafeToClear, Probe::Paths(&[r"%TEMP%"])),
    item("windowsTemp", TempLogs, SafeToClear, Probe::Paths(&[r"%SystemRoot%\Temp"])),
    item(
        "crashDumps",
        TempLogs,
        SafeToClear,
        Probe::Paths(&[
            r"%LOCALAPPDATA%\CrashDumps",
            r"%SystemRoot%\Minidump",
            r"%SystemRoot%\MEMORY.DMP",
            r"%SystemRoot%\LiveKernelReports",
        ]),
    ),
    item(
        "errorReports",
        TempLogs,
        SafeToClear,
        Probe::Paths(&[r"%ProgramData%\Microsoft\Windows\WER", r"%LOCALAPPDATA%\Microsoft\Windows\WER"]),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_data::expand::expand;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique() {
        let mut seen = HashSet::new();
        for item in CATALOG {
            assert!(seen.insert(item.id), "duplicate id {}", item.id);
        }
    }

    #[test]
    fn every_template_is_well_formed() {
        let any = |_: &str| Some("X".to_string());
        for item in CATALOG {
            for template in item.probe.templates() {
                assert!(expand(template, &any).is_some(), "{}: {template}", item.id);
            }
        }
    }

    #[test]
    fn every_item_has_ui_strings() {
        let strings = include_str!("../../../src/strings.ts");
        for item in CATALOG {
            assert!(
                strings.contains(&format!("{}: {{", item.id)),
                "add systemItems.{} to src/strings.ts",
                item.id
            );
        }
    }
}
