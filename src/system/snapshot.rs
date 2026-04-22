use std::collections::HashMap;
use std::fs;

use pulse::system::cpu::read_total_cpu_time;
use pulse::system::process::read_cpu_time;

pub struct SystemSnapshot {
    pub total_cpu: u64,
    pub processes: std::collections::HashMap<u32, u64>,
}

pub fn sample_system() -> SystemSnapshot {
    let mut processes = HashMap::new();
    
    let entries = match fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => {
            return SystemSnapshot {
                total_cpu: 0,
                processes,
            }
        }
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        if let Ok(pid) = name.parse<u32>() {
            if let Some(cpu_time) = read_cpu_time(pid) {
                processes.insert(pid, cpu_time);
            }
        }
    }

    let total_cpu = read_total_cpu_time();

    SystemSnapshot {
        total_cpu,
        processes,
    }
}

pub fn compute_cpu_usage(
    prev: &SystemSnapshot,
    curr: &SystemSnapshot,
) -> HashMap<u32, u64> {
    let mut usage = HashMap::new();

    let total_delta = curr.total_cpu.saturating_sub(prev.total_cpu);

    if total_delta == 0 {
        return usage;
    }

    for (pid, &curr_time) in &curr.processes {
        if let Some(&prev_time) = prev.processes.get(pid) {
            let delta = curr_time.saturating_sub(prev_time);

            let percent = (delta as f32 / total_delta as f32) * 100.0;
            usage.insert(*pid, percent);
        }
    }

    usage
}
