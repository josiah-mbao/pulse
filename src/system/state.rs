use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::system::collector::RawProcess;
use crate::system::memory::{read_memory, memory_usage_percent};

#[derive(Clone)]
pub struct ProcessSnapshot {
    pub name: String,
    pub cpu_time: u64,
    pub memory_kb: u64,
}

#[derive(Clone)]
pub struct SystemState {
    pub prev: HashMap<u32, ProcessSnapshot>,
    pub curr: HashMap<u32, ProcessSnapshot>,
    pub total_cpu_delta: u64, 
}

/// The unified carrier payload bridging background I/O to the frontend loop
#[derive(Clone)]
pub struct TelemetryFrame {
    pub processes: HashMap<u32, ProcessSnapshot>,
    pub cpu_map: HashMap<u32, f32>,
    pub global_cpu_utilization: f32,
    pub global_mem_utilization: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpuJiffies {
    pub total: u64,
    pub idle: u64,
}

pub fn build_state(
    prev: HashMap<u32, ProcessSnapshot>,
    curr: Vec<RawProcess>,
    total_cpu_delta: u64,
) -> SystemState {
    let mut curr_map: HashMap<u32, ProcessSnapshot> = HashMap::new();

    for p in curr {
        curr_map.insert(
            p.pid,
            ProcessSnapshot {
                name: p.name,
                cpu_time: p.cpu_time,
                memory_kb: p.memory_kb,
            },
        );
    }

    SystemState {
        prev,
        curr: curr_map,
        total_cpu_delta,
    }
}

pub fn compute_cpu(state: &SystemState) -> HashMap<u32, f32> {
    let mut usage = HashMap::new();

    if state.total_cpu_delta == 0 {
        return usage;
    }

    for (pid, curr) in &state.curr {
        if let Some(prev) = state.prev.get(pid) {
            let proc_delta = curr.cpu_time.saturating_sub(prev.cpu_time);

            if proc_delta > 0 {
                let percent = (proc_delta as f32 / state.total_cpu_delta as f32) * 100.0;
                usage.insert(*pid, percent);
            }
        }
    }

    usage
}

/// Parses aggregate CPU statistics from /proc/stat
pub fn read_global_jiffies() -> Option<CpuJiffies> {
    let file = File::open("/proc/stat").ok()?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;

    if line.starts_with("cpu ") {
        let parts: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|s| s.parse::<u64>().ok())
            .collect();

        if parts.len() >= 4 {
            // idle (index 3) + iowait (index 4)
            let idle = parts[3] + parts.get(4).unwrap_or(&0);
            let total: u64 = parts.iter().sum();
            return Some(CpuJiffies { total, idle });
        }
    }
    None
}

/// Calculates immediate global memory allocation percentage
pub fn read_global_mem_percent() -> f32 {
    let (total, avail) = read_memory();
    memory_usage_percent(total, avail)
}
