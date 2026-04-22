use std::fs;

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

pub fn read_total_cpu_time() -> u64 {
    let contents = fs::read_to_string("/proc/stat").unwrap();
    let line = contents.lines().next().unwrap();

    line.split_whitespace()
        .skip(1)
        .map(|v| v.parse::<u64>().unwrap())
        .sum()
}
