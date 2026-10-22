//! Every process in one call, the way Task Manager reads them: `NtQuerySystemInformation`
//! returns CPU time and private working set for all processes without opening any of them,
//! so protected and other users' processes count too, and nothing needs admin.

use std::ffi::c_void;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::{CloseHandle, UNICODE_STRING};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::{
    GetActiveProcessorCount, OpenProcess, QueryFullProcessImageNameW, ALL_PROCESSOR_GROUPS,
    PROCESS_QUERY_LIMITED_INFORMATION,
};

const SYSTEM_PROCESS_INFORMATION_CLASS: u32 = 5;
const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004_u32 as i32;

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQuerySystemInformation(class: u32, info: *mut c_void, length: u32, returned: *mut u32) -> i32;
}

/// The start of `SYSTEM_PROCESS_INFORMATION`, as far as the fields read here.
#[repr(C)]
struct SystemProcessInformation {
    next_entry_offset: u32,
    number_of_threads: u32,
    working_set_private_size: i64,
    hard_fault_count: u32,
    number_of_threads_high_watermark: u32,
    cycle_time: u64,
    create_time: i64,
    user_time: i64,
    kernel_time: i64,
    image_name: UNICODE_STRING,
    base_priority: i32,
    unique_process_id: usize,
    inherited_from_unique_process_id: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawProcess {
    pub pid: u32,
    pub parent: u32,
    pub name: String,
    /// Identifies the process together with the PID (PIDs are reused).
    pub create_time: i64,
    /// User plus kernel time, in 100 ns units.
    pub cpu_time: i64,
    /// Private working set, bytes: Task Manager's Memory column.
    pub memory: u64,
}

/// Every process running now, PID 0 (Idle) included.
pub fn list() -> Vec<RawProcess> {
    let mut buffer: Vec<u64> = vec![0; 128 * 1024];
    loop {
        let bytes = (buffer.len() * 8) as u32;
        let mut needed = 0u32;
        let status = unsafe {
            NtQuerySystemInformation(SYSTEM_PROCESS_INFORMATION_CLASS, buffer.as_mut_ptr().cast(), bytes, &mut needed)
        };
        if status == STATUS_INFO_LENGTH_MISMATCH {
            // Processes may start between calls: leave some room.
            buffer = vec![0; (needed as usize + 64 * 1024).div_ceil(8)];
            continue;
        }
        if status < 0 {
            return Vec::new();
        }
        break;
    }
    let base = buffer.as_ptr().cast::<u8>();
    let mut out = Vec::new();
    let mut offset = 0usize;
    loop {
        let entry = unsafe { &*base.add(offset).cast::<SystemProcessInformation>() };
        let pid = entry.unique_process_id as u32;
        let name = image_name(&entry.image_name).unwrap_or_else(|| if pid == 0 { "Idle".into() } else { "System".into() });
        out.push(RawProcess {
            pid,
            parent: entry.inherited_from_unique_process_id as u32,
            name,
            create_time: entry.create_time,
            cpu_time: entry.user_time.saturating_add(entry.kernel_time),
            memory: entry.working_set_private_size.max(0) as u64,
        });
        if entry.next_entry_offset == 0 {
            break;
        }
        offset += entry.next_entry_offset as usize;
    }
    out
}

fn image_name(s: &UNICODE_STRING) -> Option<String> {
    if s.Buffer.is_null() || s.Length == 0 {
        return None;
    }
    let units = unsafe { std::slice::from_raw_parts(s.Buffer, s.Length as usize / 2) };
    Some(String::from_utf16_lossy(units))
}

/// Full path of a process's executable; `None` for protected processes.
pub fn exe_path(pid: u32) -> Option<PathBuf> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let mut buffer = [0u16; 1024];
    let mut len = buffer.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut len) } != 0;
    unsafe { CloseHandle(handle) };
    (ok && len > 0).then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize])))
}

/// Percent of the whole machine used between two readings of a process's CPU time.
pub fn cpu_percent(before: i64, after: i64, elapsed_100ns: i64, cores: usize) -> f32 {
    if elapsed_100ns <= 0 || after < before {
        return 0.0;
    }
    let percent = (after - before) as f64 / (elapsed_100ns as f64 * cores.max(1) as f64) * 100.0;
    percent.clamp(0.0, 100.0) as f32
}

/// Logical processors, across every processor group.
pub fn cores() -> usize {
    (unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) } as usize).max(1)
}

/// Physical memory in use and in total, bytes.
pub fn memory() -> (u64, u64) {
    let mut status = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return (0, 0);
    }
    (status.ullTotalPhys - status.ullAvailPhys, status.ullTotalPhys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_percent_is_of_the_whole_machine() {
        // One core busy for a whole second on a 4-core machine is 25%.
        assert_eq!(cpu_percent(0, 10_000_000, 10_000_000, 4), 25.0);
        assert_eq!(cpu_percent(5, 5, 10_000_000, 4), 0.0);
        // A process that restarted under the same PID reads lower: not negative.
        assert_eq!(cpu_percent(100, 50, 10_000_000, 4), 0.0);
        assert_eq!(cpu_percent(0, 10, 0, 4), 0.0);
    }

    #[test]
    fn lists_this_process() {
        let me = std::process::id();
        let all = list();
        let mine = all.iter().find(|p| p.pid == me).expect("this process is listed");
        assert!(mine.name.to_lowercase().ends_with(".exe"));
        assert!(mine.memory > 0);
        assert!(all.iter().any(|p| p.pid == 0));
        assert!(exe_path(me).is_some());
    }
}
