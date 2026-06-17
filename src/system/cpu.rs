use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct CpuSnapshot {
    pub total_time: u64,
}

/// Reads the total CPU time from /proc/stat
pub fn read_total_cpu_time() -> u64 {
    let file = File::open("/proc/stat").unwrap_or_else(|_| panic!("Failed to open /proc/stat"));
    read_total_cpu_time_from_reader(BufReader::new(file))
}

pub fn read_total_cpu_time_from_reader<R: BufRead>(reader: R) -> u64 {
    let line = reader.lines().next().and_then(|l| l.ok()).unwrap_or_default();
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
