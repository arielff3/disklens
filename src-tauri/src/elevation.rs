//! Whether Disklens runs as administrator, and relaunching it elevated.

use crate::win::to_wide;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, WaitForSingleObject, INFINITE};
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, ShellExecuteW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

const SW_SHOWNORMAL: i32 = 1;
const SW_HIDE: i32 = 0;

pub fn is_elevated() -> bool {
    unsafe {
        let mut token: HANDLE = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut TOKEN_ELEVATION as *mut c_void,
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        );
        CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

/// Starts a new, elevated Disklens. Windows shows one UAC prompt.
pub fn relaunch_elevated() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let (verb, file) = (to_wide("runas"), to_wide(&exe));
    let instance = unsafe { ShellExecuteW(null_mut(), verb.as_ptr(), file.as_ptr(), null(), null(), SW_SHOWNORMAL) };
    // ShellExecute reports success with a value above 32; declining the UAC prompt is a failure.
    if instance as isize > 32 {
        Ok(())
    } else {
        Err("Windows didn't start Disklens as administrator.".into())
    }
}

/// Runs `file args` as administrator (one UAC prompt, or none if Disklens is
/// already elevated) in a hidden window, waits for it, and returns its exit code.
pub fn run_elevated(file: &str, args: &str) -> Result<u32, String> {
    let (verb, file_w, args_w) = (to_wide("runas"), to_wide(file), to_wide(args));
    unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file_w.as_ptr();
        info.lpParameters = args_w.as_ptr();
        info.nShow = SW_HIDE;
        if ShellExecuteExW(&mut info) == 0 {
            return Err(format!("{file} didn't start. The administrator prompt may have been declined."));
        }
        if info.hProcess.is_null() {
            return Ok(0);
        }
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 0u32;
        GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
        Ok(code)
    }
}

#[tauri::command]
pub fn is_admin() -> bool {
    is_elevated()
}

/// Relaunches elevated, then closes this instance.
#[tauri::command]
pub fn relaunch_as_admin(app: tauri::AppHandle) -> Result<(), String> {
    relaunch_elevated()?;
    app.exit(0);
    Ok(())
}
