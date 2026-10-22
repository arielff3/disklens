//! App names from executables' version info ("Google Chrome" for chrome.exe).

use crate::win::to_wide;
use std::path::Path;
use windows_sys::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};

/// The executable's FileDescription, or `None` if it has none.
pub fn describe(exe: &Path) -> Option<String> {
    let path = to_wide(exe);
    let size = unsafe { GetFileVersionInfoSizeW(path.as_ptr(), std::ptr::null_mut()) };
    if size == 0 {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    if unsafe { GetFileVersionInfoW(path.as_ptr(), 0, size, data.as_mut_ptr().cast()) } == 0 {
        return None;
    }
    // The first language/code page pair names the string table to read.
    let translation = query(&data, r"\VarFileInfo\Translation")?;
    if translation.len() < 4 {
        return None;
    }
    let lang = u16::from_le_bytes([translation[0], translation[1]]);
    let code_page = u16::from_le_bytes([translation[2], translation[3]]);
    let raw = query(&data, &format!(r"\StringFileInfo\{lang:04x}{code_page:04x}\FileDescription"))?;
    let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
    let text = String::from_utf16_lossy(&units).trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// The bytes of a version-info value.
fn query<'a>(data: &'a [u8], sub_block: &str) -> Option<&'a [u8]> {
    let block: Vec<u16> = sub_block.encode_utf16().chain([0]).collect();
    let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
    let mut len = 0u32;
    if unsafe { VerQueryValueW(data.as_ptr().cast(), block.as_ptr(), &mut ptr, &mut len) } == 0 || ptr.is_null() {
        return None;
    }
    // Strings report their length in characters, other values in bytes.
    let bytes = if sub_block.starts_with(r"\StringFileInfo") { len as usize * 2 } else { len as usize };
    let start = ptr as usize - data.as_ptr() as usize;
    data.get(start..start + bytes)
}
