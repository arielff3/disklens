//! GPU use per process, from the same performance counters Task Manager reads.

use std::collections::HashMap;
use windows_sys::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    PDH_CSTATUS_VALID_DATA, PDH_FMT, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_FMT_LARGE, PDH_HCOUNTER,
    PDH_HQUERY, PDH_MORE_DATA,
};
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

/// PID of a counter instance named `pid_<n>_luid_...`.
pub fn parse_pid(instance: &str) -> Option<u32> {
    instance.strip_prefix("pid_")?.split('_').next()?.parse().ok()
}

/// PID and engine type of a `\GPU Engine` instance (`pid_1234_..._engtype_3D`).
pub fn parse_engine(instance: &str) -> Option<(u32, String)> {
    let pid = parse_pid(instance)?;
    let kind = instance.rsplit_once("_engtype_")?.1;
    Some((pid, kind.to_string()))
}

/// Task Manager's numbers from per-engine utilization: a process uses as much GPU as its busiest
/// engine type (its engines of one type added up); the GPU as a whole is as busy as its busiest
/// engine type across every process, capped at 100.
pub fn fold_engines(engines: &[(u32, String, f64)]) -> (HashMap<u32, f64>, f64) {
    let mut by_pid_type: HashMap<(u32, &str), f64> = HashMap::new();
    let mut by_type: HashMap<&str, f64> = HashMap::new();
    for (pid, kind, value) in engines {
        *by_pid_type.entry((*pid, kind.as_str())).or_default() += value;
        *by_type.entry(kind.as_str()).or_default() += value;
    }
    let mut per_pid: HashMap<u32, f64> = HashMap::new();
    for ((pid, _), value) in by_pid_type {
        let entry = per_pid.entry(pid).or_default();
        *entry = entry.max(value);
    }
    let system = by_type.values().copied().fold(0.0, f64::max).min(100.0);
    (per_pid, system)
}

pub struct GpuSample {
    /// Percent and dedicated memory in bytes, per PID.
    pub per_pid: HashMap<u32, (f32, u64)>,
    pub total: f32,
    pub mem_used: u64,
}

/// An open PDH query on the GPU counters. Lives on the sampler thread (PDH handles aren't `Send`).
pub struct GpuQuery {
    query: PDH_HQUERY,
    engines: PDH_HCOUNTER,
    process_memory: PDH_HCOUNTER,
    adapter_memory: PDH_HCOUNTER,
}

impl GpuQuery {
    /// `None` when this machine has no GPU counters (some VMs, old drivers).
    pub fn open() -> Option<GpuQuery> {
        let mut query: PDH_HQUERY = std::ptr::null_mut();
        if unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut query) } != 0 {
            return None;
        }
        let add = |path: &str| -> Option<PDH_HCOUNTER> {
            let wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
            let mut counter: PDH_HCOUNTER = std::ptr::null_mut();
            (unsafe { PdhAddEnglishCounterW(query, wide.as_ptr(), 0, &mut counter) } == 0).then_some(counter)
        };
        let counters = (
            add(r"\GPU Engine(*)\Utilization Percentage"),
            add(r"\GPU Process Memory(*)\Dedicated Usage"),
            add(r"\GPU Adapter Memory(*)\Dedicated Usage"),
        );
        let (Some(engines), Some(process_memory), Some(adapter_memory)) = counters else {
            unsafe { PdhCloseQuery(query) };
            return None;
        };
        // Utilization is a rate: the first collection only sets the starting point.
        unsafe { PdhCollectQueryData(query) };
        Some(GpuQuery { query, engines, process_memory, adapter_memory })
    }

    pub fn sample(&mut self) -> GpuSample {
        unsafe { PdhCollectQueryData(self.query) };
        let engines: Vec<(u32, String, f64)> = read(self.engines, PDH_FMT_DOUBLE)
            .into_iter()
            .filter_map(|(name, v)| parse_engine(&name).map(|(pid, kind)| (pid, kind, v.as_f64())))
            .collect();
        let (percent, total) = fold_engines(&engines);
        let mut per_pid: HashMap<u32, (f32, u64)> = percent.into_iter().map(|(pid, p)| (pid, (p as f32, 0))).collect();
        for (name, v) in read(self.process_memory, PDH_FMT_LARGE) {
            if let Some(pid) = parse_pid(&name) {
                per_pid.entry(pid).or_default().1 += v.as_u64();
            }
        }
        let mem_used = read(self.adapter_memory, PDH_FMT_LARGE).into_iter().map(|(_, v)| v.as_u64()).sum();
        GpuSample { per_pid, total: total as f32, mem_used }
    }
}

impl Drop for GpuQuery {
    fn drop(&mut self) {
        unsafe { PdhCloseQuery(self.query) };
    }
}

#[derive(Clone, Copy)]
enum Value {
    Double(f64),
    Large(i64),
}

impl Value {
    fn as_f64(self) -> f64 {
        match self {
            Value::Double(v) => v,
            Value::Large(v) => v as f64,
        }
    }
    fn as_u64(self) -> u64 {
        match self {
            Value::Double(v) => v.max(0.0) as u64,
            Value::Large(v) => v.max(0) as u64,
        }
    }
}

/// Every instance of a wildcard counter with valid data, by instance name.
fn read(counter: PDH_HCOUNTER, format: PDH_FMT) -> Vec<(String, Value)> {
    let (mut size, mut count) = (0u32, 0u32);
    let status = unsafe { PdhGetFormattedCounterArrayW(counter, format, &mut size, &mut count, std::ptr::null_mut()) };
    if status != PDH_MORE_DATA || size == 0 {
        return Vec::new();
    }
    // The buffer holds the items followed by their names; u64 keeps it aligned for the items.
    let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
    let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
    if unsafe { PdhGetFormattedCounterArrayW(counter, format, &mut size, &mut count, items) } != 0 {
        return Vec::new();
    }
    let items = unsafe { std::slice::from_raw_parts(items, count as usize) };
    items
        .iter()
        .filter(|item| item.FmtValue.CStatus == PDH_CSTATUS_VALID_DATA)
        .map(|item| {
            let name = unsafe { wide_str(item.szName) };
            let value = unsafe {
                if format == PDH_FMT_DOUBLE {
                    Value::Double(item.FmtValue.Anonymous.doubleValue)
                } else {
                    Value::Large(item.FmtValue.Anonymous.largeValue)
                }
            };
            (name, value)
        })
        .collect()
}

unsafe fn wide_str(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut len = 0;
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) })
}

/// Dedicated memory of the biggest display adapter, as the driver reports it in the registry.
pub fn dedicated_total() -> Option<u64> {
    let class = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}")
        .ok()?;
    class
        .enum_keys()
        .flatten()
        .filter(|k| k.len() == 4 && k.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|k| class.open_subkey(k).ok()?.get_value::<u64, _>("HardwareInformation.qwMemorySize").ok())
        .max()
        .filter(|&bytes| bytes > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_engine_instances() {
        assert_eq!(parse_engine("pid_1234_luid_0x00000000_0x0000D1A0_phys_0_eng_0_engtype_3D"), Some((1234, "3D".into())));
        assert_eq!(parse_engine("pid_8_luid_0x0_0x1_phys_0_eng_3_engtype_VideoDecode"), Some((8, "VideoDecode".into())));
        assert_eq!(parse_engine("luid_0x0_phys_0"), None);
    }

    #[test]
    fn parses_memory_instances() {
        assert_eq!(parse_pid("pid_77_luid_0x0_0x1_phys_0"), Some(77));
        assert_eq!(parse_pid("luid_0x0_0x1_phys_0"), None);
    }

    #[test]
    fn per_process_is_busiest_engine_type() {
        let (per, system) = fold_engines(&[
            (1, "3D".into(), 20.0),
            (1, "3D".into(), 10.0),
            (1, "Copy".into(), 5.0),
            (2, "VideoDecode".into(), 40.0),
        ]);
        assert_eq!(per[&1], 30.0);
        assert_eq!(per[&2], 40.0);
        assert_eq!(system, 40.0);
    }

    #[test]
    fn system_total_caps_at_100() {
        let (_, system) = fold_engines(&[(1, "3D".into(), 80.0), (2, "3D".into(), 70.0)]);
        assert_eq!(system, 100.0);
    }
}
