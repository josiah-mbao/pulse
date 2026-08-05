use crate::system::state::ProcessSnapshot;
use std::collections::HashMap;

pub struct ProcessView {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_kb: u64,
}

pub fn build_view(
    processes: &HashMap<u32, ProcessSnapshot>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_view_normal_and_sorting() {
        let mut processes = HashMap::new();
        processes.insert(
            1,
            ProcessSnapshot {
                ppid: 0,
                name: "proc1".to_string(),
                cpu_time: 100,
                memory_kb: 1000,
            },
        );
        processes.insert(
            2,
            ProcessSnapshot {
                ppid: 0,
                name: "proc2".to_string(),
                cpu_time: 200,
                memory_kb: 2000,
            },
        );
        processes.insert(
            3,
            ProcessSnapshot {
                ppid: 1,
                name: "proc3".to_string(),
                cpu_time: 300,
                memory_kb: 3000,
            },
        );

        let mut cpu_map = HashMap::new();
        cpu_map.insert(1, 15.5);
        cpu_map.insert(2, 45.0);
        cpu_map.insert(3, 5.0);

        let views = build_view(&processes, &cpu_map);
        assert_eq!(views.len(), 3);
        assert_eq!(views[0].pid, 2);
        assert_eq!(views[1].pid, 1);
        assert_eq!(views[2].pid, 3);
        assert!((views[0].cpu_percent - 45.0).abs() < f32::EPSILON);
        assert_eq!(views[0].memory_kb, 2000);
    }

    #[test]
    fn test_build_view_missing_cpu_entry() {
        let mut processes = HashMap::new();
        processes.insert(
            1,
            ProcessSnapshot {
                ppid: 0,
                name: "proc1".to_string(),
                cpu_time: 100,
                memory_kb: 1000,
            },
        );

        let views = build_view(&processes, &HashMap::new());
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].pid, 1);
        assert_eq!(views[0].cpu_percent, 0.0);
    }

    #[test]
    fn test_build_view_empty() {
        let views = build_view(&HashMap::new(), &HashMap::new());
        assert!(views.is_empty());
    }

    #[test]
    fn test_build_view_equal_cpu() {
        let mut processes = HashMap::new();
        processes.insert(
            10,
            ProcessSnapshot {
                ppid: 0,
                name: "procA".to_string(),
                cpu_time: 100,
                memory_kb: 500,
            },
        );
        processes.insert(
            20,
            ProcessSnapshot {
                ppid: 0,
                name: "procB".to_string(),
                cpu_time: 100,
                memory_kb: 800,
            },
        );

        let mut cpu_map = HashMap::new();
        cpu_map.insert(10, 10.0);
        cpu_map.insert(20, 10.0);

        let views = build_view(&processes, &cpu_map);
        assert_eq!(views.len(), 2);
    }
}
