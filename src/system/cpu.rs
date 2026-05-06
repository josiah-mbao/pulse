use std::fs;

pub struct CpuSnapshot {
    pub total_time: u64,
}

/// Reads the total CPU time from /proc/stat
pub fn read_total_cpu_time() -> u64 {
    let contents = fs::read_to_string("/proc/stat").unwrap_or_default();
    let line = contents.lines().next().unwrap_or("");
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Summing: user, nice, system, idle, iowait, irq, softirq
    let mut total: u64 = 0;
    for part in parts.iter().skip(1).take(7) {
        if let Ok(val) = part.parse::<u64>() {
            total += val;
        }
    }
    total
}

// Resolved dead_code by removing the unused private helper 'read_snapshot' 

