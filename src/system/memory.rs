use std::fs;

pub fn read_memory() -> (u64, u64) {
    let contents = fs::read_to_string("/proc/meminfo").unwrap();

    let mut total = 0;
    let mut available = 0;

    for line in contents.lines() {
        if line.starts_with("MemTotal") {
            total = line.split_whitespace().nth(1).unwrap().parse().unwrap();
        }

        if line.starts_with("MemAvailable") {
            available = line.split_whitespace().nth(1).unwrap().parse().unwrap();
        }
    }

    (total, available)
}

pub fn memory_usage_percent(total: u64, available: u64) -> f32 {
    let used = total - available;
    (used as f32 / total as f32) * 100.0
}
