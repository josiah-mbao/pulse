use crate::system::memory::{memory_usage_percent, read_memory};
use crate::system::snapshot::{compute_cpu_usage, sample_system};
use crate::system::uptime::read_uptime;
use std::{thread::sleep, time::Duration};

pub fn run_status() {
    let prev = sample_system();
    sleep(Duration::from_millis(200));
    let curr = sample_system();

    let usage_map = compute_cpu_usage(&prev, &curr);
    let cpu: f32 = usage_map.values().sum();

    let (total, available) = read_memory();
    let mem = memory_usage_percent(total, available);

    let uptime = read_uptime();

    println!("=== Pulse System Status ===");
    println!("CPU:      {:.2}%", cpu);
    println!("Memory:   {:.2}%", mem);
    println!("Uptime:   {:.2} seconds", uptime);
}
