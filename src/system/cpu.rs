use std::{fs, thread::sleep, time::Duration};

#[derive(Clone, Copy)]
pub struct CpuSnapshot {
    pub total: u64,
    pub idle: u64,
}

fn read_snapshot() -> CpuSnapshot {
    let contents = fs::read_to_string("/proc/stat").unwrap();
    let line = contents.lines().next().unwrap();

    let values: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .map(|v| v.parse().unwrap())
        .collect();

    let idle = values[3];
    let total = values.iter().sum();

    CpuSnapshot { total, idle }
}

pub fn read_cpu_usage() -> f32 {
    let a = read_snapshot();
    sleep(Duration::from_millis(200));
    let b = read_snapshot();

    let total_delta = b.total - a.total;
    let idle_delta = b.idle - a.idle;

    if total_delta == 0 {
        return 0.0;
    }

    ((total_delta - idle_delta) as f32 / total_delta as f32) * 100.0
}
