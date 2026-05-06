use std::fs;

pub fn read_memory() -> (u64, u64) {
    // Replace unwrap with a safe fall-back to avoid crashes
    let contents = fs::read_to_string("/proc/meminfo").unwrap_or_else(|_| String::new());

    let mut total = 0;
    let mut available = 0;

    for line in contents.lines() {
        if line.starts_with("MemTotal") {
            total = line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
        }
        if line.starts_with("MemAvailable") {
            available = line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
        }
    }

    (total, available)
}

pub fn memory_usage_percent(total: u64, available: u64) -> f32 {
    if total == 0 { return 0.0; }
    let used = total.saturating_sub(available);
    (used as f32 / total as f32) * 100.0
}
