//! Ending processes, behind the guard.

use super::guard::refusal;
use serde::Serialize;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER};
use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Ended,
    /// It had already exited.
    Gone,
    /// It belongs to another user or runs elevated.
    Denied,
    /// The guard won't end it.
    Refused,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndResult {
    pub pid: u32,
    pub outcome: Outcome,
}

/// Ends each PID; `name_of` gives the process name the guard checks (None: already gone).
pub fn end(pids: &[u32], name_of: impl Fn(u32) -> Option<String>) -> Vec<EndResult> {
    let own = std::process::id();
    pids.iter()
        .map(|&pid| {
            let outcome = match name_of(pid) {
                None => Outcome::Gone,
                Some(name) if refusal(pid, &name, own).is_some() => Outcome::Refused,
                Some(_) => terminate(pid),
            };
            EndResult { pid, outcome }
        })
        .collect()
}

fn terminate(pid: u32) -> Outcome {
    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if handle.is_null() {
        return match unsafe { GetLastError() } {
            ERROR_ACCESS_DENIED => Outcome::Denied,
            ERROR_INVALID_PARAMETER => Outcome::Gone,
            _ => Outcome::Failed,
        };
    }
    let ok = unsafe { TerminateProcess(handle, 1) } != 0;
    let error = unsafe { GetLastError() };
    unsafe { CloseHandle(handle) };
    match (ok, error) {
        (true, _) => Outcome::Ended,
        (false, ERROR_ACCESS_DENIED) => Outcome::Denied,
        _ => Outcome::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guarded_and_missing_processes_are_never_opened() {
        let results = end(&[4, 600, 999_999], |pid| match pid {
            4 => Some("System".into()),
            600 => Some("csrss.exe".into()),
            _ => None,
        });
        let outcomes: Vec<Outcome> = results.iter().map(|r| r.outcome).collect();
        assert_eq!(outcomes, [Outcome::Refused, Outcome::Refused, Outcome::Gone]);
    }
}
