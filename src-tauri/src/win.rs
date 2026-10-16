//! Small wrappers around Win32 calls the standard library doesn't expose.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Storage::FileSystem::{
    GetCompressedFileSizeW, GetDiskFreeSpaceW, GetVolumePathNameW,
};

const DEFAULT_CLUSTER: u64 = 4096;

/// NUL-terminated UTF-16, with the `\\?\` prefix for long absolute paths.
pub fn to_wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    let raw: Vec<u16> = s.as_ref().encode_wide().collect();
    let mut wide = Vec::with_capacity(raw.len() + 5);
    if raw.len() >= 248 && raw.get(1) == Some(&(b':' as u16)) {
        wide.extend(r"\\?\".encode_utf16());
    }
    wide.extend(raw);
    wide.push(0);
    wide
}

/// Cluster size of the volume holding `path`; 4 KiB if it can't be determined.
pub fn cluster_size(path: &Path) -> u64 {
    let wide = to_wide(path);
    let mut volume = [0u16; 512];
    if unsafe { GetVolumePathNameW(wide.as_ptr(), volume.as_mut_ptr(), volume.len() as u32) } == 0 {
        return DEFAULT_CLUSTER;
    }
    let (mut sectors_per_cluster, mut bytes_per_sector, mut free, mut total) = (0u32, 0u32, 0u32, 0u32);
    let ok = unsafe {
        GetDiskFreeSpaceW(
            volume.as_ptr(),
            &mut sectors_per_cluster,
            &mut bytes_per_sector,
            &mut free,
            &mut total,
        )
    };
    let cluster = sectors_per_cluster as u64 * bytes_per_sector as u64;
    if ok == 0 || cluster == 0 {
        DEFAULT_CLUSTER
    } else {
        cluster
    }
}

/// On-disk size of a compressed, sparse or cloud file.
pub fn compressed_size(path: &Path) -> Option<u64> {
    let wide = to_wide(path);
    let mut high = 0u32;
    let low = unsafe { GetCompressedFileSizeW(wide.as_ptr(), &mut high) };
    if low == u32::MAX && unsafe { GetLastError() } != 0 {
        return None;
    }
    Some(((high as u64) << 32) | low as u64)
}
