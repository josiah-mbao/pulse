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
