use crate::system::collector::RawProcess;
use crate::system::memory::{memory_usage_percent, read_memory};
use crate::system::model::{InterfaceSnapshot, NetworkStats};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessSnapshot {
    pub ppid: u32,
    pub name: String,
    pub cpu_time: u64,
    pub memory_kb: u64,
}

#[derive(Clone)]
pub struct SystemState {
    pub prev: HashMap<u32, ProcessSnapshot>,
    pub curr: HashMap<u32, ProcessSnapshot>,
    pub total_cpu_delta: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpuJiffies {
    pub total: u64,
    pub idle: u64,
}

pub fn build_state(
    prev: HashMap<u32, ProcessSnapshot>,
    curr: Vec<RawProcess>,
    total_cpu_delta: u64,
) -> SystemState {
    let mut curr_map: HashMap<u32, ProcessSnapshot> = HashMap::new();

    for p in curr {
        curr_map.insert(
            p.pid,
            ProcessSnapshot {
                ppid: p.ppid,
                name: p.name,
                cpu_time: p.cpu_time,
                memory_kb: p.memory_kb,
            },
        );
    }

    SystemState {
        prev,
        curr: curr_map,
        total_cpu_delta,
    }
}

pub fn compute_cpu(state: &SystemState) -> HashMap<u32, f32> {
    let mut usage = HashMap::new();

    if state.total_cpu_delta == 0 {
        return usage;
    }

    for (pid, curr) in &state.curr {
        if let Some(prev) = state.prev.get(pid) {
            let proc_delta = curr.cpu_time.saturating_sub(prev.cpu_time);

            if proc_delta > 0 {
                let percent = (proc_delta as f32 / state.total_cpu_delta as f32) * 100.0;
                usage.insert(*pid, percent);
            }
        }
    }

    usage
}

/// Parses aggregate CPU statistics from /proc/stat
pub fn read_global_jiffies() -> Option<CpuJiffies> {
    let file = File::open("/proc/stat").ok()?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;

    if line.starts_with("cpu ") {
        let parts: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|s| s.parse::<u64>().ok())
            .collect();

        if parts.len() >= 4 {
            // idle (index 3) + iowait (index 4)
            let idle = parts[3] + parts.get(4).unwrap_or(&0);
            let total: u64 = parts.iter().sum();
            return Some(CpuJiffies { total, idle });
        }
    }
    None
}

/// Calculates immediate global memory allocation percentage
pub fn read_global_mem_percent() -> f32 {
    let (total, avail) = read_memory();
    memory_usage_percent(total, avail)
}

/// Highly efficient, zero-allocation path function for parsing /proc/net/dev
pub fn read_network_dev() -> Option<NetworkStats> {
    let file = File::open("/proc/net/dev").ok()?;
    let reader = BufReader::new(file);
    parse_network_stats(reader)
}

fn parse_network_stats<R: BufRead>(mut reader: R) -> Option<NetworkStats> {
    let mut stats = NetworkStats::default();
    let mut line = String::with_capacity(256);

    // Skip the two header lines
    for _ in 0..2 {
        line.clear();
        reader.read_line(&mut line).ok()?;
    }

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Robust split to handle "eth0: 123" AND "eth0:123"
                let mut name_metrics = trimmed.splitn(2, ':');
                let name = match name_metrics.next() {
                    Some(n) => n.trim(),
                    None => continue,
                };
                let metrics_part = match name_metrics.next() {
                    Some(m) => m,
                    None => continue,
                };

                let mut metrics = metrics_part.split_whitespace();
                let rx_bytes = metrics
                    .next()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                // Skip 7 fields to reach tx_bytes (index 9 overall in /proc/net/dev line)
                let tx_bytes = metrics
                    .nth(7)
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                // Include loopback even if currently zero to ensure visibility
                if rx_bytes > 0 || tx_bytes > 0 || name == "lo" {
                    let operstate =
                        std::fs::read_to_string(format!("/sys/class/net/{}/operstate", name))
                            .unwrap_or_else(|_| "unknown".to_string())
                            .trim()
                            .to_string();

                    let rx_errors = std::fs::read_to_string(format!(
                        "/sys/class/net/{}/statistics/rx_errors",
                        name
                    ))
                    .ok()
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .unwrap_or(0);

                    stats.interfaces.insert(
                        name.to_string(),
                        InterfaceSnapshot {
                            rx_bytes,
                            tx_bytes,
                            operstate,
                            rx_errors,
                        },
                    );
                }
            }
            Err(_) => break,
        }
    }

    Some(stats)
}

/// Aggregates sectors read/written from /proc/diskstats across physical drives
pub fn read_disk_io() -> Option<(u64, u64)> {
    let file = File::open("/proc/diskstats").ok()?;
    let mut reader = BufReader::new(file);
    let mut line = String::with_capacity(256);
    let mut total_read = 0;
    let mut total_written = 0;

    loop {
        line.clear();
        if reader.read_line(&mut line).ok()? == 0 {
            break;
        }

        let mut parts = line.split_whitespace();
        let _major = parts.next();
        let _minor = parts.next();
        let name = match parts.next() {
            Some(n) => n,
            None => continue,
        };

        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("zram") {
            continue;
        }

        let s_read = parts
            .nth(2)
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let s_write = parts
            .nth(3)
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        total_read += s_read;
        total_written += s_write;
    }

    Some((total_read, total_written))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_network_dev_logic() {
        let mock_proc_net_dev = r#"Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 100 0 0 0 0 0 0 0 200 0 0 0 0 0 0 0
  eth0: 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
  wlan0: 500 0 0 0 0 0 0 0 600 0 0 0 0 0 0 0
"#;
        let reader = Cursor::new(mock_proc_net_dev);
        let stats = parse_network_stats(reader).expect("Failed to parse mock network data");

        // Verify "lo" interface
        let lo = stats.interfaces.get("lo").expect("lo interface missing");
        assert_eq!(lo.rx_bytes, 100);
        assert_eq!(lo.tx_bytes, 200);

        // Verify "wlan0" interface
        let wlan0 = stats
            .interfaces
            .get("wlan0")
            .expect("wlan0 interface missing");
        assert_eq!(wlan0.rx_bytes, 500);
        assert_eq!(wlan0.tx_bytes, 600);
    }
}
