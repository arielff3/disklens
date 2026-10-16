//! Starting a program's own uninstaller.

use super::registry::Program;
use crate::win::to_wide;
use std::mem::size_of;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

const SW_SHOWNORMAL: i32 = 1;

/// The program and its arguments, as ShellExecute wants them.
#[derive(Debug, PartialEq)]
pub struct Launch {
    pub file: String,
    pub args: String,
}

/// Splits an `UninstallString` into program and arguments. Handles quoted
/// paths, unquoted paths with spaces (up to the first `.exe`), and bare names.
pub fn split_command(command: &str) -> Launch {
    let command = command.trim();
    if let Some(rest) = command.strip_prefix('"') {
        if let Some((file, args)) = rest.split_once('"') {
            return Launch { file: file.to_string(), args: args.trim().to_string() };
        }
    }
    let lower = command.to_ascii_lowercase();
    if let Some(i) = lower.find(".exe") {
        let end = i + 4;
        if command[end..].is_empty() || command[end..].starts_with(char::is_whitespace) {
            return Launch { file: command[..end].to_string(), args: command[end..].trim().to_string() };
        }
    }
    match command.split_once(char::is_whitespace) {
        Some((file, args)) => Launch { file: file.to_string(), args: args.trim().to_string() },
        None => Launch { file: command.to_string(), args: String::new() },
    }
}

/// How to uninstall `program`. MSI entries often store `/I{GUID}` (which
/// opens the repair dialog), so they always get `msiexec /x {GUID}`.
pub fn command_for(program: &Program) -> Launch {
    let is_guid = program.key.starts_with('{') && program.key.ends_with('}');
    if program.msi && is_guid {
        return Launch { file: "msiexec.exe".into(), args: format!("/x {}", program.key) };
    }
    split_command(&program.uninstall_string)
}

/// Starts the uninstaller. Uninstallers that need admin ask through UAC themselves.
pub fn launch(launch: &Launch) -> Result<(), String> {
    let (verb, file, args) = (to_wide("open"), to_wide(&launch.file), to_wide(&launch.args));
    unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS;
        info.hwnd = null_mut();
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = args.as_ptr();
        info.nShow = SW_SHOWNORMAL;
        if ShellExecuteExW(&mut info) == 0 {
            return Err(format!("Windows couldn't start {}", launch.file));
        }
        if !info.hProcess.is_null() {
            CloseHandle(info.hProcess);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(file: &str, args: &str) -> Launch {
        Launch { file: file.into(), args: args.into() }
    }

    #[test]
    fn splits_uninstall_strings() {
        assert_eq!(
            split_command(r#""C:\Program Files\App\unins000.exe""#),
            launch(r"C:\Program Files\App\unins000.exe", "")
        );
        assert_eq!(
            split_command(r#""C:\Users\a\AppData\Local\Discord\Update.exe" --uninstall -s"#),
            launch(r"C:\Users\a\AppData\Local\Discord\Update.exe", "--uninstall -s")
        );
        assert_eq!(
            split_command(r"C:\Program Files\Some App\uninst.exe /allusers"),
            launch(r"C:\Program Files\Some App\uninst.exe", "/allusers")
        );
        assert_eq!(split_command("MsiExec.exe /I{ABC}"), launch("MsiExec.exe", "/I{ABC}"));
        assert_eq!(
            split_command("rundll32.exe dfshim.dll,ShArpMaintain app.application"),
            launch("rundll32.exe", "dfshim.dll,ShArpMaintain app.application")
        );
    }

    #[test]
    fn msi_entries_uninstall_by_product_code() {
        let program = Program {
            id: r"HKLM\{1234}".into(),
            key: "{1234}".into(),
            name: "Tool".into(),
            version: None,
            publisher: None,
            install_location: None,
            uninstall_string: "MsiExec.exe /I{1234}".into(),
            estimated_bytes: None,
            msi: true,
        };
        assert_eq!(command_for(&program), launch("msiexec.exe", "/x {1234}"));
        let not_msi = Program { msi: false, ..program };
        assert_eq!(command_for(&not_msi), launch("MsiExec.exe", "/I{1234}"));
    }
}
