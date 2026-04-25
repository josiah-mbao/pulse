use crate::system::collector::RawProcess;
use std::collections::HashMap;

pub struct ProcessView {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_kb: u64,
}

pub fn build_view(
    processes: &HashMap<u32, RawProcess>,
    cpu_map: &HashMap<u32, f32>,
) -> Vec<ProcessView> {
    let mut out = Vec::new();

    for (pid, proc) in processes {
        out.push(ProcessView {
            pid: *pid,
            name: proc.name.clone(),
            cpu_percent: *cpu_map.get(pid).unwrap_or(&0.0),
            memory_kb: proc.memory_kb,
        });
    }

    out.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap());

    out
}
