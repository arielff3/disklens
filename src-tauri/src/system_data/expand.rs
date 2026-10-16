//! `%VAR%` path templates, expanded from the environment.

use std::path::PathBuf;

/// Looks up an environment variable by name.
pub type Lookup = dyn Fn(&str) -> Option<String> + Send + Sync;

pub fn env_lookup(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// Expands every `%NAME%` in `template`. `None` if a variable is unset or a `%` is unpaired.
pub fn expand(template: &str, lookup: &Lookup) -> Option<PathBuf> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after.find('%')?;
        out.push_str(&lookup(&after[..end])?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Some(PathBuf::from(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(name: &str) -> Option<String> {
        match name {
            "LOCALAPPDATA" => Some(r"C:\Users\a\AppData\Local".into()),
            "SystemDrive" => Some("C:".into()),
            _ => None,
        }
    }

    #[test]
    fn expands_variables() {
        assert_eq!(
            expand(r"%LOCALAPPDATA%\npm-cache", &lookup),
            Some(PathBuf::from(r"C:\Users\a\AppData\Local\npm-cache"))
        );
        assert_eq!(expand(r"%SystemDrive%\pagefile.sys", &lookup), Some(PathBuf::from(r"C:\pagefile.sys")));
        assert_eq!(expand(r"C:\plain", &lookup), Some(PathBuf::from(r"C:\plain")));
    }

    #[test]
    fn unknown_variable_or_unpaired_percent_is_none() {
        assert_eq!(expand(r"%NOPE%\x", &lookup), None);
        assert_eq!(expand(r"%SystemDrive\x", &lookup), None);
    }
}
