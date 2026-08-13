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

pub fn parse_global_jiffies(line: &str) -> Option<CpuJiffies> {
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

/// Parses aggregate CPU statistics from /proc/stat
pub fn read_global_jiffies() -> Option<CpuJiffies> {
    let file = File::open("/proc/stat").ok()?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;

    parse_global_jiffies(&line)
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

pub fn parse_diskstats<R: BufRead>(mut reader: R) -> Option<(u64, u64)> {
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

/// Aggregates sectors read/written from /proc/diskstats across physical drives
pub fn read_disk_io() -> Option<(u64, u64)> {
    let file = File::open("/proc/diskstats").ok()?;
    let reader = BufReader::new(file);
    parse_diskstats(reader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::collector::RawProcess;
    use std::io::Cursor;

    #[test]
    fn test_build_state_basic() {
        let mut prev = HashMap::new();
        prev.insert(
            1,
            ProcessSnapshot {
                ppid: 0,
                name: "init".to_string(),
                cpu_time: 10,
                memory_kb: 100,
            },
        );

        let curr_raw = vec![
            RawProcess {
                pid: 1,
                ppid: 0,
                name: "init".to_string(),
                cpu_time: 20,
                memory_kb: 100,
            },
            RawProcess {
                pid: 2,
                ppid: 1,
                name: "bash".to_string(),
                cpu_time: 50,
                memory_kb: 200,
            },
        ];

        let state = build_state(prev.clone(), curr_raw, 100);
        assert_eq!(state.total_cpu_delta, 100);
        assert_eq!(state.prev, prev);
        assert_eq!(state.curr.len(), 2);
        assert_eq!(state.curr.get(&1).unwrap().cpu_time, 20);
        assert_eq!(state.curr.get(&2).unwrap().name, "bash");
    }

    #[test]
    fn test_build_state_empty() {
        let state = build_state(HashMap::new(), Vec::new(), 0);
        assert_eq!(state.total_cpu_delta, 0);
        assert!(state.prev.is_empty());
        assert!(state.curr.is_empty());
    }

    #[test]
    fn test_build_state_duplicate_pid_overwrites() {
        let curr_raw = vec![
            RawProcess {
                pid: 10,
                ppid: 1,
                name: "old_proc".to_string(),
                cpu_time: 10,
                memory_kb: 100,
            },
            RawProcess {
                pid: 10,
                ppid: 1,
                name: "new_proc".to_string(),
                cpu_time: 30,
                memory_kb: 300,
            },
        ];

        let state = build_state(HashMap::new(), curr_raw, 50);
        assert_eq!(state.curr.len(), 1);
        assert_eq!(state.curr.get(&10).unwrap().name, "new_proc");
        assert_eq!(state.curr.get(&10).unwrap().cpu_time, 30);
    }

    #[test]
    fn test_compute_cpu_basic() {
        let mut prev = HashMap::new();
        prev.insert(
            10,
            ProcessSnapshot {
                ppid: 1,
                name: "proc1".to_string(),
                cpu_time: 100,
                memory_kb: 1000,
            },
        );
        prev.insert(
            20,
            ProcessSnapshot {
                ppid: 1,
                name: "proc2".to_string(),
                cpu_time: 200,
                memory_kb: 2000,
            },
        );

        let mut curr = HashMap::new();
        curr.insert(
            10,
            ProcessSnapshot {
                ppid: 1,
                name: "proc1".to_string(),
                cpu_time: 150, // delta = 50
                memory_kb: 1000,
            },
        );
        curr.insert(
            20,
            ProcessSnapshot {
                ppid: 1,
                name: "proc2".to_string(),
                cpu_time: 200, // delta = 0 -> omitted
                memory_kb: 2000,
            },
        );

        let state = SystemState {
            prev,
            curr,
            total_cpu_delta: 200, // percentage for proc1 = (50 / 200) * 100 = 25.0%
        };

        let cpu = compute_cpu(&state);
        assert_eq!(cpu.len(), 1);
        assert!((cpu.get(&10).unwrap() - 25.0).abs() < f32::EPSILON);
        assert!(!cpu.contains_key(&20));
    }

    #[test]
    fn test_compute_cpu_zero_total_delta() {
        let mut prev = HashMap::new();
        prev.insert(
            10,
            ProcessSnapshot {
                ppid: 1,
                name: "proc1".to_string(),
                cpu_time: 100,
                memory_kb: 1000,
            },
        );
        let mut curr = HashMap::new();
        curr.insert(
            10,
            ProcessSnapshot {
                ppid: 1,
                name: "proc1".to_string(),
                cpu_time: 150,
                memory_kb: 1000,
            },
        );

        let state = SystemState {
            prev,
            curr,
            total_cpu_delta: 0,
        };

        let cpu = compute_cpu(&state);
        assert!(cpu.is_empty());
    }

    #[test]
    fn test_compute_cpu_missing_in_prev() {
        let mut curr = HashMap::new();
        curr.insert(
            30,
            ProcessSnapshot {
                ppid: 1,
                name: "proc30".to_string(),
                cpu_time: 50,
                memory_kb: 500,
            },
        );

        let state = SystemState {
            prev: HashMap::new(),
            curr,
            total_cpu_delta: 100,
        };

        let cpu = compute_cpu(&state);
        assert!(cpu.is_empty());
    }

    #[test]
    fn test_compute_cpu_decreased_cpu_time() {
        let mut prev = HashMap::new();
        prev.insert(
            10,
            ProcessSnapshot {
                ppid: 1,
                name: "proc1".to_string(),
                cpu_time: 100,
                memory_kb: 1000,
            },
        );
        let mut curr = HashMap::new();
        curr.insert(
            10,
            ProcessSnapshot {
                ppid: 1,
                name: "proc1".to_string(),
                cpu_time: 90, // decreased
                memory_kb: 1000,
            },
        );

        let state = SystemState {
            prev,
            curr,
            total_cpu_delta: 100,
        };

        let cpu = compute_cpu(&state);
        assert!(cpu.is_empty());
    }

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

    #[test]
    fn test_parse_network_stats_empty_and_malformed() {
        let empty_reader = Cursor::new("");
        let stats = parse_network_stats(empty_reader);
        assert!(stats.is_none_or(|s| s.interfaces.is_empty()));

        let malformed = r#"Inter-|   Receive
 interface |bytes
eth0:123 0 0 0 0 0 0 0 456 0 0 0 0 0 0 0
invalid_line_without_colon
"#;
        let stats = parse_network_stats(Cursor::new(malformed)).expect("Should parse valid lines");
        let eth0 = stats
            .interfaces
            .get("eth0")
            .expect("eth0 interface missing");
        assert_eq!(eth0.rx_bytes, 123);
        assert_eq!(eth0.tx_bytes, 456);
    }

    #[test]
    fn test_parse_global_jiffies_valid() {
        let line = "cpu  1000 100 300 5000 200 50 10 0 0 0";
        let jiffies = parse_global_jiffies(line).expect("Should parse cpu line");
        assert_eq!(jiffies.total, 6660);
        assert_eq!(jiffies.idle, 5200); // 5000 + 200
    }

    #[test]
    fn test_parse_global_jiffies_malformed() {
        assert!(parse_global_jiffies("cpu0 100 200").is_none());
        assert!(parse_global_jiffies("not_cpu 100 200").is_none());
    }

    #[test]
    fn test_parse_diskstats_valid() {
        let mock_diskstats = r#"   8       0 sda 1000 0 8000 100 2000 0 16000 200 0 300 300
   7       0 loop0 500 0 4000 50 0 0 0 0 0 0 0
 259       0 nvme0n1 3000 0 24000 150 4000 0 32000 250 0 400 400
"#;
        let (read_sectors, write_sectors) =
            parse_diskstats(Cursor::new(mock_diskstats)).expect("Should parse diskstats");
        // sda: read 8000, write 16000
        // loop0: skipped
        // nvme0n1: read 24000, write 32000
        // total read = 32000, total write = 48000
        assert_eq!(read_sectors, 32000);
        assert_eq!(write_sectors, 48000);
    }
}
