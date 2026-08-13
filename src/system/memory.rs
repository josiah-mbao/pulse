use std::fs;

pub fn parse_meminfo(contents: &str) -> (u64, u64) {
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

pub fn read_memory() -> (u64, u64) {
    let contents = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    parse_meminfo(&contents)
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
    fn test_parse_meminfo_valid() {
        let sample = "MemTotal:       16384000 kB\nMemFree:         4000000 kB\nMemAvailable:    8000000 kB\nBuffers:          500000 kB\n";
        let (total, avail) = parse_meminfo(sample);
        assert_eq!(total, 16_384_000);
        assert_eq!(avail, 8_000_000);
    }

    #[test]
    fn test_parse_meminfo_malformed_or_missing() {
        let (total, avail) = parse_meminfo("MemTotal: invalid\nSomeOtherKey: 1234");
        assert_eq!(total, 0);
        assert_eq!(avail, 0);
    }

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
