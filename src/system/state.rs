use std::collections::HashMap;
use crate::system::collector::RawProcess;

#[derive(Clone)]
pub struct ProcessSnapshot {
    pub name: String,
    pub cpu_time: u64,
    pub memory_kb: u64,
}

#[derive(Clone)]
pub struct SystemState {
    pub prev: HashMap<u32, ProcessSnapshot>,
    pub curr: HashMap<u32, ProcessSnapshot>,
    pub total_cpu_delta: u64, 
}

pub fn build_state(
    prev: HashMap<u32, ProcessSnapshot>,
    curr: Vec<RawProcess>,
    total_cpu_delta: u64,
) -> SystemState {
    let mut curr_map: HashMap<u32, ProcessSnapshot> = HashMap::new();

    for p in curr {
        curr_map.insert(
            p.pid,
            ProcessSnapshot {
                name: p.name,
                cpu_time: p.cpu_time,
                memory_kb: p.memory_kb,
            },
        );
    }

    SystemState {
        prev,
        curr: curr_map,
        total_cpu_delta,
    }
}

pub fn compute_cpu(state: &SystemState) -> HashMap<u32, f32> {
    let mut usage = HashMap::new();

    // If no time has passed globally, avoid division by zero
    if state.total_cpu_delta == 0 {
        return usage;
    }

    for (pid, curr) in &state.curr {
        if let Some(prev) = state.prev.get(pid) {
            let proc_delta = curr.cpu_time.saturating_sub(prev.cpu_time);

            if proc_delta > 0 {
                // Calculation: (Process Jiffies / System Jiffies) * 100
                let percent = (proc_delta as f32 / state.total_cpu_delta as f32) * 100.0;
                usage.insert(*pid, percent);
            }
        }
    }

    usage
}
