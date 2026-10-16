//! Turning app and folder names into comparable keys.

/// Folder names never matched on their own: they hold many apps' data.
const GENERIC: &[&str] = &[
    "adobe", "amd", "app", "appdata", "apple", "apps", "cache", "common", "commonfiles", "crashdumps", "data",
    "default", "google", "installer", "intel", "local", "locallow", "microsoft", "mozilla", "nvidia", "packages",
    "program", "programs", "roaming", "setup", "software", "system", "temp", "tools", "update", "updater", "user",
    "users", "windows",
];

/// Lowercase letters and digits only: "Visual Studio Code" → "visualstudiocode".
pub fn normalize(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// True for keys too vague to identify one app.
pub fn is_generic(key: &str) -> bool {
    key.chars().count() < 3 || GENERIC.contains(&key)
}

/// The display name without version numbers or architecture:
/// "Python 3.12.6 (64-bit)" → "Python", "7-Zip 23.01 (x64)" → "7-Zip".
pub fn base_name(display: &str) -> String {
    let mut plain = String::new();
    let mut depth = 0;
    for c in display.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    let mut tokens: Vec<&str> = plain.split_whitespace().collect();
    while tokens.last().is_some_and(|t| is_version_like(t)) {
        tokens.pop();
    }
    tokens.join(" ")
}

fn is_version_like(token: &str) -> bool {
    let t = token.trim_matches(|c: char| !c.is_alphanumeric()).to_ascii_lowercase();
    if t.is_empty() {
        return true;
    }
    if matches!(t.as_str(), "x64" | "x86" | "arm64" | "64bit" | "32bit" | "64-bit" | "32-bit" | "version") {
        return true;
    }
    let digits = t.strip_prefix('v').unwrap_or(&t);
    digits.starts_with(|c: char| c.is_ascii_digit()) && digits.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

/// Keys for a publisher's folder: "Google LLC" → ["googlellc", "google"].
pub fn publisher_keys(publisher: &str) -> Vec<String> {
    let mut keys = vec![normalize(publisher)];
    if let Some(first) = publisher.split_whitespace().next() {
        let first = normalize(first);
        if !keys.contains(&first) {
            keys.push(first);
        }
    }
    keys.retain(|k| k.chars().count() >= 3);
    keys
}

/// Keys an app's own folders may be named after: its base name, and the base
/// name without the publisher's first word ("Google Chrome" → "chrome").
pub fn app_keys(display: &str, publisher: Option<&str>) -> Vec<String> {
    let base = base_name(display);
    let mut keys = vec![normalize(&base)];
    if let Some(first) = publisher.and_then(|p| p.split_whitespace().next()) {
        let words: Vec<&str> = base.split_whitespace().collect();
        if words.len() > 1 && words[0].eq_ignore_ascii_case(first) {
            keys.push(normalize(&words[1..].join(" ")));
        }
    }
    keys.retain(|k| !is_generic(k));
    keys.dedup();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_versions_and_architecture() {
        assert_eq!(base_name("Python 3.12.6 (64-bit)"), "Python");
        assert_eq!(base_name("7-Zip 23.01 (x64)"), "7-Zip");
        assert_eq!(base_name("Notepad++ (64-bit x64)"), "Notepad++");
        assert_eq!(base_name("Microsoft Visual C++ 2015-2022 Redistributable (x64) - 14.38.33130"), "Microsoft Visual C++ 2015-2022 Redistributable");
        assert_eq!(base_name("Node.js"), "Node.js");
        assert_eq!(base_name("Git version 2.56.0"), "Git");
        assert_eq!(base_name("Steam"), "Steam");
    }

    #[test]
    fn normalizes_and_rejects_vague_keys() {
        assert_eq!(normalize("Visual Studio Code"), "visualstudiocode");
        assert_eq!(normalize("Node.js"), "nodejs");
        assert!(is_generic("microsoft"));
        assert!(is_generic("go"));
        assert!(!is_generic("discord"));
    }

    #[test]
    fn builds_app_and_publisher_keys() {
        assert_eq!(app_keys("Google Chrome", Some("Google LLC")), ["googlechrome", "chrome"]);
        assert_eq!(app_keys("Discord", Some("Discord Inc.")), ["discord"]);
        assert_eq!(app_keys("Microsoft Teams", Some("Microsoft Corporation")), ["microsoftteams", "teams"]);
        assert_eq!(publisher_keys("Google LLC"), ["googlellc", "google"]);
        assert_eq!(publisher_keys("Valve"), ["valve"]);
    }
}
