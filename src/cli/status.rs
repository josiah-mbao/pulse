use pulse::system::snapshot::{sample_system, compute_cpu_usage};
use std::{thread::sleep, time::Duration};
use pulse::system::memory::{read_memory, memory_usage_percent};
use pulse::system::uptime::read_uptime;

pub fn run_status() {
    let prev = sample_system();
    sleep(Duration::from_millis(200));
    let curr = sample_system();

    let usage_map = compute_cpu_usage(&prev, &curr);

    // Aggregates CPU usage
    let total_cpu: f32 = usage_map.values().sum();

    let (total, available) = read_memory();
    let mem = memory_usage_percent(total, available);

    let uptime = read_uptime();

    // Take 2 snapshots
    let prev = sample_system();
    sleep(Duration::from_millis(200));
    let curr = sample_system();

    // Compute per process CPU usage
    let usage_map = compute_cpu_usage(&prev, &curr);

    // Aggregate into a single CPU value
    let cpu: f32 = usage_map.values().sum();


    println!("=== Pulse System Status ===");
    println!("CPU:      {:.2}%", cpu);
    println!("Memory:   {:.2}%", mem);
    println!("Uptime:   {:.2} seconds", uptime);
}
