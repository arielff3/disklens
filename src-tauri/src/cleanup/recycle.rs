//! Moving files and folders to the Recycle Bin, through Windows' own shell.

use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use windows_sys::Win32::UI::Shell::{
    SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FOF_WANTNUKEWARNING, FO_DELETE,
    SHFILEOPSTRUCTW,
};

const CHUNK: usize = 64;

/// Sends `paths` to the Recycle Bin and returns the ones still on disk
/// afterwards (in use, or refused). Windows asks before permanently deleting
/// anything too big for the bin.
pub fn recycle(paths: &[PathBuf]) -> Vec<PathBuf> {
    for chunk in paths.chunks(CHUNK) {
        // One failure stops a whole shell operation, so retry a failed chunk entry by entry.
        if shell_recycle(chunk) != 0 {
            for path in chunk.iter().filter(|p| exists(p)) {
                shell_recycle(std::slice::from_ref(path));
            }
        }
    }
    paths.iter().filter(|p| exists(p)).cloned().collect()
}

fn exists(path: &PathBuf) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn shell_recycle(paths: &[PathBuf]) -> i32 {
    // A list of NUL-terminated paths, ended by an extra NUL.
    let mut from: Vec<u16> = Vec::new();
    for path in paths {
        from.extend(path.as_os_str().encode_wide());
        from.push(0);
    }
    from.push(0);
    let mut op = SHFILEOPSTRUCTW {
        hwnd: null_mut(),
        wFunc: FO_DELETE,
        pFrom: from.as_ptr(),
        pTo: null(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT | FOF_WANTNUKEWARNING) as u16,
        fAnyOperationsAborted: 0,
        hNameMappings: null_mut(),
        lpszProgressTitle: null(),
    };
    unsafe { SHFileOperationW(&mut op) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ignored by default: it puts a real (tiny) file in the user's Recycle Bin.
    #[test]
    #[ignore]
    fn moves_files_to_the_recycle_bin() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("disklens-recycle-test.txt");
        fs::write(&file, b"disklens test").unwrap();
        assert!(recycle(std::slice::from_ref(&file)).is_empty());
        assert!(!file.exists());
    }
}
