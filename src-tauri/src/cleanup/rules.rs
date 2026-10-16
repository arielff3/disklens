//! Which System Data items Clean up may empty, and which apps own them.

/// An app whose cache must not be touched while it runs.
#[derive(Debug)]
pub struct App {
    pub name: &'static str,
    /// Executable names, matched case-insensitively.
    pub exes: &'static [&'static str],
}

#[derive(Debug)]
pub struct Rule {
    /// A System Data catalogue id.
    pub id: &'static str,
    /// Checked when the tab opens.
    pub default_on: bool,
    pub apps: &'static [App],
}

const CHROME: App = App { name: "Chrome", exes: &["chrome.exe"] };
const EDGE: App = App { name: "Edge", exes: &["msedge.exe"] };
const BRAVE: App = App { name: "Brave", exes: &["brave.exe"] };
const FIREFOX: App = App { name: "Firefox", exes: &["firefox.exe"] };
const TEAMS: App = App { name: "Teams", exes: &["ms-teams.exe", "teams.exe"] };
const DISCORD: App = App { name: "Discord", exes: &["discord.exe"] };
const SPOTIFY: App = App { name: "Spotify", exes: &["spotify.exe"] };
const STEAM: App = App { name: "Steam", exes: &["steam.exe"] };

const fn rule(id: &'static str, default_on: bool, apps: &'static [App]) -> Rule {
    Rule { id, default_on, apps }
}

pub static RULES: &[Rule] = &[
    rule("chromeCache", true, &[CHROME]),
    rule("edgeCache", true, &[EDGE]),
    rule("braveCache", true, &[BRAVE]),
    rule("firefoxCache", true, &[FIREFOX]),
    rule("teamsCache", true, &[TEAMS]),
    rule("discordCache", true, &[DISCORD]),
    rule("spotifyCache", true, &[SPOTIFY]),
    rule("steamShaderCache", true, &[STEAM]),
    rule("userTemp", true, &[]),
    rule("windowsTemp", true, &[]),
    rule("crashDumps", true, &[]),
    rule("errorReports", true, &[]),
    rule("updateDownloads", true, &[]),
    rule("deliveryOptimization", true, &[]),
    // Developer caches: safe, but clearing them means downloading packages again.
    rule("npmCache", false, &[]),
    rule("yarnCache", false, &[]),
    rule("pipCache", false, &[]),
    rule("cargoRegistry", false, &[]),
    rule("gradleCache", false, &[]),
    rule("mavenRepo", false, &[]),
    rule("nugetCache", false, &[]),
];

impl Rule {
    /// The first of this rule's apps that is running, if any.
    pub fn open_app(&self, running: &std::collections::HashSet<String>) -> Option<&'static str> {
        self.apps
            .iter()
            .find(|app| app.exes.iter().any(|exe| running.contains(&exe.to_ascii_lowercase())))
            .map(|app| app.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_data::catalog::{Probe, Verdict, CATALOG};
    use std::collections::HashSet;

    #[test]
    fn rules_cover_only_safe_folder_items() {
        let mut seen = HashSet::new();
        for rule in RULES {
            assert!(seen.insert(rule.id), "duplicate rule {}", rule.id);
            let def = CATALOG.iter().find(|d| d.id == rule.id).unwrap_or_else(|| panic!("{} not in catalogue", rule.id));
            assert_eq!(def.verdict, Verdict::SafeToClear, "{} is not safe to clear", rule.id);
            assert!(
                matches!(def.probe, Probe::Paths(_) | Probe::Each { .. }),
                "{} is not measured from folders",
                rule.id
            );
        }
    }

    #[test]
    fn finds_open_apps_case_insensitively() {
        let running: HashSet<String> = ["explorer.exe".to_string(), "spotify.exe".to_string()].into();
        let spotify = RULES.iter().find(|r| r.id == "spotifyCache").unwrap();
        let chrome = RULES.iter().find(|r| r.id == "chromeCache").unwrap();
        assert_eq!(spotify.open_app(&running), Some("Spotify"));
        assert_eq!(chrome.open_app(&running), None);
    }
}
