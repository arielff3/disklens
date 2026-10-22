//! Processes Disklens refuses to end: ending them crashes or logs off Windows.

/// Matched case-insensitively against the process name.
pub const CRITICAL: [&str; 10] = [
    "csrss.exe",
    "wininit.exe",
    "smss.exe",
    "lsass.exe",
    "services.exe",
    "winlogon.exe",
    "svchost.exe",
    "fontdrvhost.exe",
    "dwm.exe",
    "system",
];

/// Why `pid` may not be ended, or `None` if it may.
pub fn refusal(pid: u32, name: &str, own_pid: u32) -> Option<&'static str> {
    if pid == 0 || pid == 4 {
        return Some("part of Windows itself");
    }
    if pid == own_pid {
        return Some("Disklens itself");
    }
    if CRITICAL.iter().any(|c| c.eq_ignore_ascii_case(name)) {
        return Some("Windows needs it to keep running");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_system_pids() {
        assert!(refusal(0, "Idle", 99).is_some());
        assert!(refusal(4, "System", 99).is_some());
    }

    #[test]
    fn refuses_critical_names_any_case() {
        assert!(refusal(700, "CSRSS.EXE", 99).is_some());
        assert!(refusal(800, "svchost.exe", 99).is_some());
    }

    #[test]
    fn refuses_itself() {
        assert!(refusal(99, "disklens.exe", 99).is_some());
    }

    #[test]
    fn allows_ordinary_apps() {
        assert!(refusal(1234, "chrome.exe", 99).is_none());
        assert!(refusal(1234, "explorer.exe", 99).is_none());
    }
}
