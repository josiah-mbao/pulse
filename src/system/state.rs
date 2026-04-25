use std::collections::HashMap;
use crate::system::collector::RawProcess;

#[derive(Clone)]
pub struct ProcessSnapshot {
    pub cpu_time: u64,
}

#[derive(Clone)]
pub struct SystemState {
    pub prev: HashMap<u32, ProcessSnapshot>,
    pub curr: HashMap<u32, RawProcess>,
}

pub fn build_state(prev: HashMap<u32, ProcessSnapshot>, curr: Vec<RawProcess>) -> SystemState {
    let curr_map = curr
        .into_iter()
        .map(|p| (p.pid, p))
        .collect();

    SystemState {
        prev,
        curr: curr_map,
    }
}

pub fn compute_cpu(state: &SystemState) -> HashMap<u32, f32> {
    let mut out = HashMap::new();
    let mut deltas: HashMap<u32, u64> = HashMap::new();

    let mut total_delta: u64 = 0;

    for (pid, curr) in &state.curr {
        if let Some(prev) = state.prev.get(pid) {
            let delta = curr.cpu_time.saturating_sub(prev.cpu_time);
            total_delta += delta;
            deltas.insert(*pid, delta);
        }
    }

    if total_delta == 0 {
        return out;
    }

    for (pid, delta) in deltas {
        let percent = (delta as f32 / total_delta as f32) * 100.0;
        out.insert(pid, percent);
    }

    out
}
