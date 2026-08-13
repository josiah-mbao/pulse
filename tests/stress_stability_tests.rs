use pulse::system::collector::RawProcess;
use pulse::system::model::{InterfaceSnapshot, NetworkStats, TelemetryFrame};
use pulse::system::state::{ProcessSnapshot, build_state, compute_cpu};
use pulse::tui::app::AppState;
use pulse_common::{EVENT_EXEC, TraceEvent};
use std::collections::HashMap;

#[test]
fn test_stress_synthetic_telemetry_event_flood() {
    let mut app = AppState::new();

    // Flood 10,000 telemetry frames and 10,000 trace events
    for i in 0..10_000 {
        let mut processes = HashMap::new();
        processes.insert(
            (i % 100) as u32 + 1,
            ProcessSnapshot {
                ppid: 1,
                name: format!("worker-{i}"),
                cpu_time: (i * 10) as u64,
                memory_kb: 1024 + (i as u64 % 4096),
            },
        );

        let mut cpu_map = HashMap::new();
        cpu_map.insert((i % 100) as u32 + 1, (i % 100) as f32);

        let mut ifaces = HashMap::new();
        ifaces.insert(
            "eth0".to_string(),
            InterfaceSnapshot {
                rx_bytes: (i * 1000) as u64,
                tx_bytes: (i * 500) as u64,
                operstate: "up".to_string(),
                rx_errors: 0,
            },
        );

        let frame = TelemetryFrame {
            processes,
            cpu_map,
            global_cpu_utilization: (i % 100) as f32,
            global_mem_utilization: (i % 100) as f32,
            network: NetworkStats { interfaces: ifaces },
            disk_sectors_read: (i * 10) as u64,
            disk_sectors_written: (i * 20) as u64,
        };

        app.apply_tick(frame);

        let trace = TraceEvent {
            pid: i as u32,
            event_type: EVENT_EXEC,
            comm: *b"flood-event\0\0\0\0\0",
        };
        app.apply_trace(trace);
    }

    // Verify bounded buffer limits are strictly enforced
    assert_eq!(app.global_cpu_history.len(), 200);
    assert_eq!(app.global_mem_history.len(), 200);
    assert_eq!(app.disk_read_history.len(), 200);
    assert_eq!(app.disk_write_history.len(), 200);
    assert_eq!(app.trace_log.len(), 500);
}

#[test]
fn test_stress_process_birth_death_cycle_stability() {
    let mut prev_state: HashMap<u32, ProcessSnapshot> = HashMap::new();
    let total_cpu_delta = 1000;

    // Run 1,000 sampling cycles continuously adding and removing PIDs
    for cycle in 0..1_000 {
        let mut raw_procs = Vec::new();
        let base_pid = (cycle * 10) as u32;

        // Spawn 20 new processes per cycle
        for p in 0..20 {
            let pid = base_pid + p;
            raw_procs.push(RawProcess {
                pid,
                ppid: 1,
                name: format!("task-{pid}"),
                cpu_time: (cycle as u64 + 1) * 100,
                memory_kb: 4096,
            });
        }

        let state = build_state(prev_state, raw_procs, total_cpu_delta);
        let cpu_map = compute_cpu(&state);

        assert_eq!(state.curr.len(), 20);
        assert!(cpu_map.len() <= 20);

        prev_state = state.curr;
    }
}

#[test]
fn test_stress_invariant_numeric_bounds_under_flood() {
    let mut prev = HashMap::new();
    prev.insert(
        1,
        ProcessSnapshot {
            ppid: 0,
            name: "stress-target".to_string(),
            cpu_time: 1_000_000,
            memory_kb: 16384,
        },
    );

    // Extreme case 1: Overflowing u64 max values
    let curr_raw1 = vec![RawProcess {
        pid: 1,
        ppid: 0,
        name: "stress-target".to_string(),
        cpu_time: u64::MAX,
        memory_kb: u64::MAX,
    }];
    let state1 = build_state(prev.clone(), curr_raw1, u64::MAX / 2);
    let cpu_map1 = compute_cpu(&state1);

    if let Some(&cpu) = cpu_map1.get(&1) {
        assert!(cpu.is_finite());
        assert!(cpu >= 0.0);
    }

    // Extreme case 2: Counter reset (decreased CPU time)
    let curr_raw2 = vec![RawProcess {
        pid: 1,
        ppid: 0,
        name: "stress-target".to_string(),
        cpu_time: 50, // decreased
        memory_kb: 100,
    }];
    let state2 = build_state(prev, curr_raw2, 100);
    let cpu_map2 = compute_cpu(&state2);
    assert!(!cpu_map2.contains_key(&1));
}
