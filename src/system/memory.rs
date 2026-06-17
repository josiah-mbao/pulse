use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn read_memory() -> (u64, u64) {
    let file = File::open("/proc/meminfo").unwrap_or_else(|_| panic!("Failed to open /proc/meminfo"));
    read_memory_from_reader(BufReader::new(file))
}

pub fn read_memory_from_reader<R: BufRead>(reader: R) -> (u64, u64) {
    let mut total = 0;
    let mut available = 0;

    for line in reader.lines().flatten() {
        if line.starts_with("MemTotal") {
            total = line
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        }
        if line.starts_with("MemAvailable") {
            available = line
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        }
    }

    (total, available)
}

pub fn memory_usage_percent(total: u64, available: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    let used = total.saturating_sub(available);
    (used as f32 / total as f32) * 100.0
}
