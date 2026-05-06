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
        // 1. Get current system-wide CPU time
        let curr_total_cpu = read_total_cpu_time();
        let total_delta = curr_total_cpu.saturating_sub(self.prev_total_cpu);
        
        // 2. Collect current process data
        let raw = collect_processes();
        
        // 3. Build state using the new 3-argument signature
        // This was the source of the E0061 error
        let state = build_state(self.prev_processes.clone(), raw, total_delta);
        
        // 4. Compute normalized CPU percentages
        let cpu_map = compute_cpu(&state);

        // 5. Update persistence for the next tick
        self.prev_processes = state.curr.clone();
        self.prev_total_cpu = curr_total_cpu;

        (state.curr, cpu_map)
    }
}
