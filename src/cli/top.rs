use std::{thread::sleep, time::Duration};

use pulse::system::sampler::sample_processes;

pub fn run_loop() {
    loop {
        // Clear terminal
        print!("\x1B[2J\x1B[1;1H");

        let mut processes = sample_processes();

        processes.sort_by(|a, b| ){
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

        sleep(Duration::from_secs(1));
    }
}
