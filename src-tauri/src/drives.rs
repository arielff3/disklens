//! Drives and well-known folders for the sidebar.

use crate::win::to_wide;
use serde::Serialize;
use std::path::PathBuf;
use std::ptr::null_mut;
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
};

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    /// Always with a trailing backslash, e.g. `C:\`.
    pub root: String,
    pub label: String,
    pub fs: String,
    pub total: u64,
    /// Free bytes available to the current user.
    pub free: u64,
    pub removable: bool,
}

#[derive(Debug, Serialize)]
pub struct Place {
    pub key: &'static str,
    pub path: String,
}

/// Fixed and removable drives (network and optical drives are skipped).
pub fn list_drives() -> Vec<Drive> {
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .filter_map(|i| drive_info(format!("{}:\\", (b'A' + i) as char)))
        .collect()
}

fn drive_info(root: String) -> Option<Drive> {
    let wide = to_wide(&root);
    let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
    if kind != DRIVE_FIXED && kind != DRIVE_REMOVABLE {
        return None;
    }
    let (mut free, mut total, mut total_free) = (0u64, 0u64, 0u64);
    if unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free) } == 0 {
        return None;
    }
    let mut label = [0u16; 261];
    let mut fs = [0u16; 261];
    unsafe {
        GetVolumeInformationW(
            wide.as_ptr(),
            label.as_mut_ptr(),
            label.len() as u32,
            null_mut(),
            null_mut(),
            null_mut(),
            fs.as_mut_ptr(),
            fs.len() as u32,
        )
    };
    let label = from_wide(&label);
    Some(Drive {
        label: if label.is_empty() { "Local Disk".into() } else { label },
        fs: from_wide(&fs),
        root,
        total,
        free,
        removable: kind == DRIVE_REMOVABLE,
    })
}

fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// Well-known folders that exist on this machine.
pub fn list_places() -> Vec<Place> {
    let candidates: [(&'static str, Option<PathBuf>); 8] = [
        ("home", dirs::home_dir()),
        ("downloads", dirs::download_dir()),
        ("documents", dirs::document_dir()),
        ("desktop", dirs::desktop_dir()),
        ("pictures", dirs::picture_dir()),
        ("videos", dirs::video_dir()),
        // %LOCALAPPDATA% is AppData\Local; its parent is AppData.
        ("appdata", dirs::data_local_dir().and_then(|p| p.parent().map(PathBuf::from))),
        ("programFiles", std::env::var_os("ProgramFiles").map(PathBuf::from)),
    ];
    candidates
        .into_iter()
        .filter_map(|(key, path)| {
            path.filter(|p| p.is_dir())
                .map(|p| Place { key, path: p.to_string_lossy().into_owned() })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_the_system_drive() {
        let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let drives = list_drives();
        let drive = drives.iter().find(|d| d.root.starts_with(&system)).expect("system drive listed");
        assert!(drive.total > 0 && drive.free <= drive.total);
        assert!(!drive.fs.is_empty());
        assert!(drive.root.ends_with('\\'));
    }

    #[test]
    fn lists_home_place() {
        assert!(list_places().iter().any(|p| p.key == "home"));
    }
}
