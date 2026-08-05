use std::fs;

pub fn read_memory() -> (u64, u64) {
    // Replace unwrap with a safe fall-back to avoid crashes
    let contents = fs::read_to_string("/proc/meminfo").unwrap_or_else(|_| String::new());

    let mut total = 0;
    let mut available = 0;

    for line in contents.lines() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_usage_percent_normal() {
        let percent = memory_usage_percent(1000, 400);
        assert!((percent - 60.0).abs() < 1e-4);
    }

    #[test]
    fn test_memory_usage_percent_zero_total() {
        let percent = memory_usage_percent(0, 500);
        assert_eq!(percent, 0.0);
    }

    #[test]
    fn test_memory_usage_percent_available_equals_total() {
        let percent = memory_usage_percent(1000, 1000);
        assert_eq!(percent, 0.0);
    }

    #[test]
    fn test_memory_usage_percent_available_zero() {
        let percent = memory_usage_percent(1000, 0);
        assert!((percent - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_memory_usage_percent_available_greater_than_total() {
        let percent = memory_usage_percent(1000, 1200);
        assert_eq!(percent, 0.0);
    }

    #[test]
    fn test_memory_usage_percent_large_values() {
        let total = 16_000_000_000u64; // ~16 GB in KB
        let available = 4_000_000_000u64; // ~4 GB in KB
        let percent = memory_usage_percent(total, available);
        assert!((percent - 75.0).abs() < 1e-4);
    }
}
