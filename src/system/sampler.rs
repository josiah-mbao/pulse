use std::{thread::sleep, time::Duration};

use crate::system::process::get_processes;
use crate::system::snapshot::{compute_cpu_usage, sample_system};

pub fn sample_processes() -> Vec<crate::system::process::ProcessInfo> {
    let prev = sample_system();
    sleep(Duration::from_millis(500));
    let curr = sample_system();

    let cpu_map = compute_cpu_usage(&prev, &curr);

    let mut processes = get_processes();

    for p in processes.iter_mut() {
        if let Some(cpu) = cpu_map.get(&p.pid) {
            p.cpu_percent = *cpu;
        } else {
            p.cpu_percent = 0.0;
        }
    }

    processes
}
