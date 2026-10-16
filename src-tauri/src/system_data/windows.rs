//! System Data probes that ask Windows instead of walking folders.

use super::expand::expand;
use super::measure::Acc;
use super::probe::Ctx;
use super::{ItemState, Measured, Note};
use std::os::windows::process::CommandExt;
use std::process::Command;
use windows_sys::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Everything in the Recycle Bins of all drives.
pub fn recycle_bin() -> Measured {
    let mut info = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, i64Size: 0, i64NumItems: 0 };
    let hr = unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) };
    if hr < 0 {
        return Measured::with_state(ItemState::Missing);
    }
    Measured {
        alloc: info.i64Size.max(0) as u64,
        files: info.i64NumItems.max(0) as u64,
        ..Measured::with_state(ItemState::Ok)
    }
}

/// Space used by System Restore points (volume shadow copies). Admin only.
pub fn shadow_storage(elevated: bool) -> Measured {
    if !elevated {
        return Measured::with_state(ItemState::NeedsAdmin);
    }
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_ShadowStorage | ForEach-Object { $_.UsedSpace } | ConvertTo-Json -Compress",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let used = output
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| parse_used_space(&String::from_utf8_lossy(&o.stdout)));
    match used {
        Some(bytes) => Measured { alloc: bytes, ..Measured::with_state(ItemState::Ok) },
        None => Measured::with_state(ItemState::Missing),
    }
}

/// WinSxS: a quick walk first (overstated, because most files are hard links
/// shared with Windows), then DISM's real size when running as administrator.
pub fn component_store(ctx: &Ctx, report: &mut dyn FnMut(Measured)) {
    let mut acc = Acc::default();
    if let Some(path) = expand(r"%SystemRoot%\WinSxS", ctx.lookup) {
        acc.add_path(&path);
    }
    let walked = Measured { note: Some(Note::SharedWithWindows), ..acc.finish() };
    if !ctx.elevated || walked.state == ItemState::Missing {
        report(walked);
        return;
    }
    report(Measured { note: Some(Note::MeasuringWithDism), ..walked.clone() });
    let actual = Command::new("dism.exe")
        .args(["/Online", "/Cleanup-Image", "/AnalyzeComponentStore", "/English"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .and_then(|o| parse_dism_actual(&String::from_utf8_lossy(&o.stdout)));
    match actual {
        Some(bytes) => report(Measured {
            state: ItemState::Ok,
            alloc: bytes,
            note: Some(Note::MeasuredByDism),
            ..walked
        }),
        None => report(walked),
    }
}

/// `ConvertTo-Json` of one number, several numbers, or nothing at all.
pub fn parse_used_space(json: &str) -> Option<u64> {
    let json = json.trim();
    if json.is_empty() {
        return Some(0);
    }
    match serde_json::from_str::<serde_json::Value>(json).ok()? {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::Array(values) => values.iter().map(|v| v.as_u64()).sum(),
        _ => None,
    }
}

/// The "Actual Size of Component Store" line of DISM's English output.
pub fn parse_dism_actual(text: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.trim_start().starts_with("Actual Size of Component Store"))?;
    parse_size(line.split_once(':')?.1)
}

/// "9.58 GB" → bytes. DISM uses 1024-based units.
pub fn parse_size(s: &str) -> Option<u64> {
    let mut parts = s.split_whitespace();
    let value: f64 = parts.next()?.replace(',', "").parse().ok()?;
    let unit = match parts.next()?.to_ascii_uppercase().as_str() {
        "B" | "BYTES" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024f64.powi(2),
        "GB" => 1024f64.powi(3),
        "TB" => 1024f64.powi(4),
        _ => return None,
    };
    Some((value * unit).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISM: &str = "Deployment Image Servicing and Management tool\n\
        Component Store (WinSxS) information:\n\n\
        Windows Explorer Reported Size of Component Store : 9.85 GB\n\n\
        Actual Size of Component Store : 9.58 GB\n\n\
        \x20   Shared with Windows : 6.11 GB\n\
        \x20   Cache and Temporary Data :  347.33 MB\n";

    #[test]
    fn parses_dism_sizes() {
        assert_eq!(parse_size("9.58 GB"), Some((9.58f64 * 1073741824.0).round() as u64));
        assert_eq!(parse_size("  347.33 MB"), Some((347.33f64 * 1048576.0).round() as u64));
        assert_eq!(parse_size("12 parsecs"), None);
        assert_eq!(parse_dism_actual(DISM), parse_size("9.58 GB"));
        assert_eq!(parse_dism_actual("no such line"), None);
    }

    #[test]
    fn parses_shadow_storage_json() {
        assert_eq!(parse_used_space(""), Some(0));
        assert_eq!(parse_used_space("123"), Some(123));
        assert_eq!(parse_used_space("[100,200]"), Some(300));
        assert_eq!(parse_used_space("\"oops\""), None);
    }

    #[test]
    fn recycle_bin_query_succeeds() {
        assert_ne!(recycle_bin().state, ItemState::Pending);
    }
}
