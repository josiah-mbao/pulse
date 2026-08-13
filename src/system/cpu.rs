use std::fs;

pub struct CpuSnapshot {
    pub total_time: u64,
}

pub fn parse_total_cpu_time(contents: &str) -> u64 {
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

/// Reads the total CPU time from /proc/stat
pub fn read_total_cpu_time() -> u64 {
    let contents = fs::read_to_string("/proc/stat").unwrap_or_default();
    parse_total_cpu_time(&contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_total_cpu_time_valid() {
        let sample = "cpu  1000 200 300 4000 500 600 700 0 0 0\ncpu0 500 100 150 2000...\n";
        let total = parse_total_cpu_time(sample);
        // 1000 + 200 + 300 + 4000 + 500 + 600 + 700 = 7300
        assert_eq!(total, 7300);
    }

    #[test]
    fn test_parse_total_cpu_time_short_or_malformed() {
        assert_eq!(parse_total_cpu_time("cpu 10 20"), 30);
        assert_eq!(parse_total_cpu_time("cpu invalid bad"), 0);
        assert_eq!(parse_total_cpu_time(""), 0);
    }
}
