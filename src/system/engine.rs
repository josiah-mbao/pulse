use std::{thread::sleep, time::Duration};
use std::collections::HashMap;

use crate::system::{
    collector::collect_processes,
    state::{build_state, compute_cpu, ProcessSnapshot},
    view::build_view,
};

pub fn run_top_loop() {
    let mut prev: HashMap<u32, ProcessSnapshot> = HashMap::new();

    loop {
        let raw = collect_processes();

        let state = build_state(prev.clone(), raw.clone());

        let cpu_map = compute_cpu(&state);

        let view = build_view(&state.curr, &cpu_map);

        print!("\x1B[2J\x1B[1;1H");

        println!("{:<6} {:<20} {:<10} {:<10}", "PID", "NAME", "MEM", "CPU");

        for p in view.iter().take(15) {
            println!(
                "{:<6} {:<20} {:<10} {:.2}",
                p.pid, p.name, p.memory_kb, p.cpu_percent
            );
        }

        prev = state
            .curr
            .iter()
            .map(|(pid, p)| (*pid, ProcessSnapshot { cpu_time: p.cpu_time }))
            .collect();

        sleep(Duration::from_secs(1));
    }
}
