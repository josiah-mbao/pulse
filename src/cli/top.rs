use std::{thread::sleep, time::Duration};

use pulse::system::process::get_processes;
use pulse::system::snapshot::{sample_system, compute_cpu_usage};

pub fn run_top() {
    let prev = sample_system();
    sleep(Duration::from_millis(500));
    let curr = sample_system();

    let cpu_map = compute_cpu_usage(&prev, &curr);

    let mut processes = get_processes();

    for p in processes.iter_mut() {
        if let Some(cpu) = cpu_map.get(&p.pid) {
            p.cpu_percent = *cpu;
        }
    }

    processes.sort_by(|a, b| {
        b.cpu_percent
            .partial_cmp(&a.cpu_percent)
            .unwrap()
    });

    println!("{:<6} {:<20} {:<10} {:<10}", "PID", "NAME", "MEM(KB)", "CPU(%)");

    for p in processes.iter().take(15) {
        println!(
            "{:<6} {:<20} {:<10} {:.2}",
            p.pid,
            p.name,
            p.memory_kb,
            p.cpu_percent
        );
    }
}
