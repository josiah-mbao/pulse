use std::collections::HashMap;
use crate::system::collector::RawProcess;

#[derive(Clone)]
pub struct ProcessSnapshot {
    pub cpu_time: u64,
    pub memory_kb: u64,
}

#[derive(Clone)]
pub struct SystemState {
    pub prev: HashMap<u32, ProcessSnapshot>,
    pub curr: HashMap<u32, ProcessSnapshot>,
}

pub fn build_state(
    prev: HashMap<u32, ProcessSnapshot>,
    curr: Vec<RawProcess>,
) -> SystemState {
    let mut curr_map: HashMap<u32, ProcessSnapshot> = HashMap::new();

    for p in curr {
        curr_map.insert(
            p.pid,
            ProcessSnapshot {
                cpu_time: p.cpu_time,
                memory_kb: p.memory_kb,
            },
        );
    }

    SystemState {
        prev,
        curr: curr_map,
    }
}
pub fn compute_cpu(state: &SystemState) -> HashMap<u32, f32> {
    let mut usage = HashMap::new();

    let mut total_delta: u64 = 0;
    let mut deltas: HashMap<u32, u64> = HashMap::new();

    for (pid, curr) in &state.curr {
        if let Some(prev) = state.prev.get(pid) {
            let delta = curr.cpu_time.saturating_sub(prev.cpu_time);

            if delta > 0 {
                deltas.insert(*pid, delta);
                total_delta += delta;
            }
        }
    }

    if total_delta == 0 {
        return usage;
    }

    for (pid, delta) in deltas {
        let percent = (delta as f32 / total_delta as f32) * 100.0;
        usage.insert(pid, percent);
    }

    usage
}
