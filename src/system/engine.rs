use std::collections::HashMap;
use crate::system::collector::collect_processes;
use crate::system::state::{build_state, compute_cpu, ProcessSnapshot};
use crate::system::cpu::read_total_cpu_time;

pub struct Engine {
    prev_processes: HashMap<u32, ProcessSnapshot>,
    prev_total_cpu: u64,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            prev_processes: HashMap::new(),
            prev_total_cpu: read_total_cpu_time(),
        }
    }

    pub fn tick(&mut self) -> (HashMap<u32, ProcessSnapshot>, HashMap<u32, f32>) {
        let curr_total_cpu = read_total_cpu_time();
        let total_delta = curr_total_cpu.saturating_sub(self.prev_total_cpu);
        
        let raw = collect_processes();
        
        // Build state with total_delta for normalized CPU usage[cite: 5]
        let state = build_state(self.prev_processes.clone(), raw, total_delta);
        let cpu_map = compute_cpu(&state);

        // Persist current state to use as "prev" in the next tick[cite: 5, 14]
        self.prev_processes = state.curr.clone();
        self.prev_total_cpu = curr_total_cpu;

        (state.curr, cpu_map)
    }
}
