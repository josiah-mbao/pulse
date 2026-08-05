use std::collections::HashMap;
use std::fs;

use crate::system::cpu::read_total_cpu_time;
use crate::system::process::read_cpu_time;

pub struct SystemSnapshot {
    pub total_cpu: u64,
    pub processes: std::collections::HashMap<u32, u64>,
}

pub fn sample_system() -> SystemSnapshot {
    let mut processes = HashMap::new();

    let entries = match fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => {
            return SystemSnapshot {
                total_cpu: 0,
                processes,
            };
        }
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        if let Ok(pid) = name.parse::<u32>()
            && let Some(cpu_time) = read_cpu_time(pid)
        {
            processes.insert(pid, cpu_time);
        }
    }

    let total_cpu = read_total_cpu_time();

    SystemSnapshot {
        total_cpu,
        processes,
    }
}

pub fn compute_cpu_usage(prev: &SystemSnapshot, curr: &SystemSnapshot) -> HashMap<u32, f32> {
    let mut usage: HashMap<u32, f32> = HashMap::new();

    let total_delta = curr.total_cpu.saturating_sub(prev.total_cpu);

    if total_delta == 0 {
        return usage;
    }

    for (pid, &curr_time) in &curr.processes {
        if let Some(&prev_time) = prev.processes.get(pid) {
            let delta = curr_time.saturating_sub(prev_time);

            let percent = (delta as f32 / total_delta as f32) * 100.0;
            usage.insert(*pid, percent);
        }
    }

    usage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_cpu_usage_normal() {
        let mut prev_procs = HashMap::new();
        prev_procs.insert(100, 1000);
        prev_procs.insert(200, 500);

        let mut curr_procs = HashMap::new();
        curr_procs.insert(100, 1050); // delta = 50
        curr_procs.insert(200, 520); // delta = 20

        let prev = SystemSnapshot {
            total_cpu: 10000,
            processes: prev_procs,
        };
        let curr = SystemSnapshot {
            total_cpu: 10100, // total_delta = 100
            processes: curr_procs,
        };

        let usage = compute_cpu_usage(&prev, &curr);
        assert_eq!(usage.len(), 2);
        assert!((usage.get(&100).unwrap() - 50.0).abs() < f32::EPSILON);
        assert!((usage.get(&200).unwrap() - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_cpu_usage_zero_total_delta() {
        let mut procs = HashMap::new();
        procs.insert(100, 1000);

        let prev = SystemSnapshot {
            total_cpu: 10000,
            processes: procs.clone(),
        };
        let curr = SystemSnapshot {
            total_cpu: 10000,
            processes: procs,
        };

        let usage = compute_cpu_usage(&prev, &curr);
        assert!(usage.is_empty());
    }

    #[test]
    fn test_compute_cpu_usage_total_cpu_wrapped() {
        let prev = SystemSnapshot {
            total_cpu: 10000,
            processes: HashMap::new(),
        };
        let curr = SystemSnapshot {
            total_cpu: 9000, // wrapped or smaller total_cpu
            processes: HashMap::new(),
        };

        let usage = compute_cpu_usage(&prev, &curr);
        assert!(usage.is_empty());
    }

    #[test]
    fn test_compute_cpu_usage_process_disappeared() {
        let mut prev_procs = HashMap::new();
        prev_procs.insert(100, 1000);
        prev_procs.insert(200, 500);

        let mut curr_procs = HashMap::new();
        curr_procs.insert(100, 1050);

        let prev = SystemSnapshot {
            total_cpu: 10000,
            processes: prev_procs,
        };
        let curr = SystemSnapshot {
            total_cpu: 10100,
            processes: curr_procs,
        };

        let usage = compute_cpu_usage(&prev, &curr);
        assert_eq!(usage.len(), 1);
        assert!(usage.contains_key(&100));
        assert!(!usage.contains_key(&200));
    }

    #[test]
    fn test_compute_cpu_usage_new_process() {
        let mut prev_procs = HashMap::new();
        prev_procs.insert(100, 1000);

        let mut curr_procs = HashMap::new();
        curr_procs.insert(100, 1050);
        curr_procs.insert(300, 200);

        let prev = SystemSnapshot {
            total_cpu: 10000,
            processes: prev_procs,
        };
        let curr = SystemSnapshot {
            total_cpu: 10100,
            processes: curr_procs,
        };

        let usage = compute_cpu_usage(&prev, &curr);
        assert_eq!(usage.len(), 1);
        assert!(!usage.contains_key(&300));
    }

    #[test]
    fn test_compute_cpu_usage_process_time_decreased() {
        let mut prev_procs = HashMap::new();
        prev_procs.insert(100, 1000);

        let mut curr_procs = HashMap::new();
        curr_procs.insert(100, 900); // decreased cpu_time

        let prev = SystemSnapshot {
            total_cpu: 10000,
            processes: prev_procs,
        };
        let curr = SystemSnapshot {
            total_cpu: 10100,
            processes: curr_procs,
        };

        let usage = compute_cpu_usage(&prev, &curr);
        assert!((usage.get(&100).unwrap() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_cpu_usage_empty_snapshots() {
        let prev = SystemSnapshot {
            total_cpu: 0,
            processes: HashMap::new(),
        };
        let curr = SystemSnapshot {
            total_cpu: 100,
            processes: HashMap::new(),
        };

        let usage = compute_cpu_usage(&prev, &curr);
        assert!(usage.is_empty());
    }
}
