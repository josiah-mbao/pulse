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
        // 1. Collect raw process data
        let raw = collect_processes();

        // 2. Build system state (prev + current snapshot)
        let state = build_state(prev, raw);

        // 3. Compute CPU usage deltas
        let cpu_map = compute_cpu(&state);

        // 4. Build view model (sorted, display-ready)
        let view = build_view(&state.curr, &cpu_map);

        // 5. Render to terminal
        render(view);

        // 6. Update previous snapshot for next iteration
        prev = state.curr.clone();

        // 7. Sampling interval
        sleep(Duration::from_secs(1));
    }
}

/// Responsible ONLY for output formatting.
/// This is intentionally isolated for future TUI / JSON support.
fn render(view: Vec<crate::system::view::ProcessView>) {
    print!("\x1B[2J\x1B[1;1H");

    println!("{:<6} {:<20} {:<10} {:<10}", "PID", "NAME", "MEM", "CPU");

    for p in view.iter().take(15) {
        println!(
            "{:<6} {:<20} {:<10} {:.2}",
            p.pid,
            p.name,
            p.memory_kb,
            p.cpu_percent
        );
    }
}
