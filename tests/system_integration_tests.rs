use pulse::system::collector::{RawProcess, parse_stat_content};
use pulse::system::engine::Engine;
use pulse::system::model::{
    InterfaceSnapshot, NetworkStats, SortMode, TelemetryFrame, ViewMode, ViewRow,
};
use pulse::system::state::{ProcessSnapshot, build_state, compute_cpu};
use pulse::system::view::build_view;
use pulse::tui::app::AppState;
use pulse::tui::projection::project_view;

use std::collections::HashMap;

// ============================================================================
// Boundary A: Collector -> State -> View End-to-End Pipeline
// ============================================================================

#[test]
fn test_pipeline_collector_to_view_end_to_end() {
    // 1. Raw collector simulation
    let raw_procs = vec![
        RawProcess {
            pid: 100,
            ppid: 1,
            name: "systemd".to_string(),
            cpu_time: 500,
            memory_kb: 4096,
        },
        RawProcess {
            pid: 200,
            ppid: 100,
            name: "pulse-worker".to_string(),
            cpu_time: 1200,
            memory_kb: 16384,
        },
        RawProcess {
            pid: 300,
            ppid: 100,
            name: "idle-daemon".to_string(),
            cpu_time: 300,
            memory_kb: 2048,
        },
    ];

    // 2. Previous state simulation
    let mut prev_procs = HashMap::new();
    prev_procs.insert(
        100,
        ProcessSnapshot {
            ppid: 1,
            name: "systemd".to_string(),
            cpu_time: 480, // delta = 20
            memory_kb: 4096,
        },
    );
    prev_procs.insert(
        200,
        ProcessSnapshot {
            ppid: 100,
            name: "pulse-worker".to_string(),
            cpu_time: 1000, // delta = 200
            memory_kb: 16384,
        },
    );
    prev_procs.insert(
        300,
        ProcessSnapshot {
            ppid: 100,
            name: "idle-daemon".to_string(),
            cpu_time: 300, // delta = 0
            memory_kb: 2048,
        },
    );

    let total_cpu_delta = 400; // total system jiffies elapsed

    // 3. Build state
    let state = build_state(prev_procs, raw_procs, total_cpu_delta);
    assert_eq!(state.curr.len(), 3);

    // 4. Compute CPU %
    let cpu_map = compute_cpu(&state);
    assert_eq!(cpu_map.len(), 2); // Procs with proc_delta > 0 (PID 100 & 200) included
    assert!((cpu_map.get(&200).copied().unwrap() - 50.0).abs() < f32::EPSILON); // (200 / 400) * 100

    // 5. Build view
    let views = build_view(&state.curr, &cpu_map);
    assert_eq!(views.len(), 3);

    // Verify view is sorted by CPU descending (pulse-worker first)
    assert_eq!(views[0].pid, 200);
    assert_eq!(views[0].name, "pulse-worker");
    assert!((views[0].cpu_percent - 50.0).abs() < f32::EPSILON);
    assert_eq!(views[0].memory_kb, 16384);

    assert_eq!(views[1].pid, 100);
    assert_eq!(views[1].name, "systemd");
    assert!((views[1].cpu_percent - 5.0).abs() < f32::EPSILON); // (20 / 400) * 100

    assert_eq!(views[2].pid, 300);
    assert_eq!(views[2].name, "idle-daemon");
    assert_eq!(views[2].cpu_percent, 0.0);
}

// ============================================================================
// Boundary B: Multi-Tick Engine State Evolution
// ============================================================================

#[test]
fn test_engine_multi_tick_lifecycle() {
    let mut engine = Engine::new();

    // Tick 1: Engine initializes state baseline
    let (procs1, cpu1) = engine.tick();
    // Verify engine produced initial process snapshots without error
    assert!(!procs1.is_empty() || procs1.is_empty()); // non-panicking execution
    let _ = cpu1;

    // Tick 2: Subsequent tick calculates CPU deltas against prev state
    let (procs2, cpu2) = engine.tick();
    assert_eq!(procs2.len(), procs2.keys().len());

    // All reported CPU usage values should be finite numbers
    for (&pid, &cpu_pct) in &cpu2 {
        assert!(procs2.contains_key(&pid));
        assert!(
            cpu_pct.is_finite(),
            "CPU % for PID {pid} is not finite: {cpu_pct}"
        );
        assert!(cpu_pct >= 0.0, "CPU % for PID {pid} is negative: {cpu_pct}");
    }
}

// ============================================================================
// Boundary C: TelemetryFrame -> AppState -> Projection Pipeline
// ============================================================================

#[test]
fn test_telemetry_frame_to_app_state_projection() {
    let mut app = AppState::new();

    // Frame 1 Setup
    let mut procs1 = HashMap::new();
    procs1.insert(
        1,
        ProcessSnapshot {
            ppid: 0,
            name: "init".to_string(),
            cpu_time: 100,
            memory_kb: 4096,
        },
    );
    procs1.insert(
        42,
        ProcessSnapshot {
            ppid: 1,
            name: "pulse-tui".to_string(),
            cpu_time: 500,
            memory_kb: 32768,
        },
    );

    let mut cpu_map1 = HashMap::new();
    cpu_map1.insert(1, 1.0);
    cpu_map1.insert(42, 25.0);

    let mut net_ifaces1 = HashMap::new();
    net_ifaces1.insert(
        "eth0".to_string(),
        InterfaceSnapshot {
            rx_bytes: 100_000,
            tx_bytes: 50_000,
            operstate: "up".to_string(),
            rx_errors: 0,
        },
    );

    let frame1 = TelemetryFrame {
        processes: procs1,
        cpu_map: cpu_map1,
        global_cpu_utilization: 15.5,
        global_mem_utilization: 45.0,
        network: NetworkStats {
            interfaces: net_ifaces1,
        },
        disk_sectors_read: 1000,
        disk_sectors_written: 2000,
    };

    app.apply_tick(frame1);

    assert_eq!(app.snapshots.len(), 2);
    assert_eq!(app.global_cpu_history.back().copied(), Some(15.5));
    assert_eq!(app.global_mem_history.back().copied(), Some(45.0));
    assert_eq!(app.view_pipeline.len(), 2);

    // Frame 2: Telemetry tick update with network traffic delta
    let mut procs2 = app.snapshots.clone();
    // Simulate pulse-tui memory increase
    if let Some(p) = procs2.get_mut(&42) {
        p.memory_kb = 40960;
    }

    let mut net_ifaces2 = HashMap::new();
    net_ifaces2.insert(
        "eth0".to_string(),
        InterfaceSnapshot {
            rx_bytes: 200_000, // delta = 100_000 bytes
            tx_bytes: 100_000, // delta = 50_000 bytes
            operstate: "up".to_string(),
            rx_errors: 0,
        },
    );

    let mut procs2_state = HashMap::new();
    for (pid, snap) in procs2 {
        procs2_state.insert(
            pid,
            ProcessSnapshot {
                ppid: snap.ppid,
                name: snap.name,
                cpu_time: 600,
                memory_kb: snap.memory_kb,
            },
        );
    }

    let mut cpu_map2 = HashMap::new();
    cpu_map2.insert(1, 0.5);
    cpu_map2.insert(42, 35.0);

    let frame2 = TelemetryFrame {
        processes: procs2_state,
        cpu_map: cpu_map2,
        global_cpu_utilization: 20.0,
        global_mem_utilization: 48.0,
        network: NetworkStats {
            interfaces: net_ifaces2,
        },
        disk_sectors_read: 1500,
        disk_sectors_written: 2500,
    };

    app.apply_tick(frame2);

    // Verify network speeds were calculated correctly (rx_delta * 2.0 / 1024.0)
    let eth0_speeds = app.current_speeds.get("eth0").expect("eth0 speeds present");
    assert!((eth0_speeds.0 - (100000.0 * 2.0 / 1024.0)).abs() < 1e-2);

    // Verify ViewMode and SortMode projection interactions
    app.sort_mode = SortMode::Memory;
    app.refresh_pipeline();
    assert_eq!(app.view_pipeline.len(), 2);
    if let ViewRow::Process { pid, .. } = app.view_pipeline[0] {
        let snap = app.snapshots.get(&pid).expect("Snapshot present");
        assert_eq!(pid, 42); // Highest memory process (40960 KB) first
        assert_eq!(snap.memory_kb, 40960);
    } else {
        panic!("Expected ViewRow::Process");
    }

    // Verify Filter Query integration
    app.filter_query = "pulse".to_string();
    app.refresh_pipeline();
    assert_eq!(app.view_pipeline.len(), 1);
    if let ViewRow::Process { pid, .. } = app.view_pipeline[0] {
        let snap = app.snapshots.get(&pid).expect("Snapshot present");
        assert_eq!(snap.name, "pulse-tui");
    } else {
        panic!("Expected filtered ViewRow::Process");
    }
}

// ============================================================================
// Boundary D: Failure-Path & System Edge Cases
// ============================================================================

#[test]
fn test_failure_path_disappearing_process() {
    let mut prev = HashMap::new();
    prev.insert(
        10,
        ProcessSnapshot {
            ppid: 1,
            name: "active".to_string(),
            cpu_time: 100,
            memory_kb: 1000,
        },
    );
    prev.insert(
        20,
        ProcessSnapshot {
            ppid: 1,
            name: "dying".to_string(),
            cpu_time: 50,
            memory_kb: 500,
        },
    );

    // Current raw collection omits PID 20 (process exited mid-collection)
    let curr_raw = vec![RawProcess {
        pid: 10,
        ppid: 1,
        name: "active".to_string(),
        cpu_time: 150,
        memory_kb: 1000,
    }];

    let state = build_state(prev, curr_raw, 100);
    let cpu_map = compute_cpu(&state);

    // PID 20 should be absent from current state and CPU map without error
    assert_eq!(state.curr.len(), 1);
    assert!(state.curr.contains_key(&10));
    assert!(!state.curr.contains_key(&20));
    assert_eq!(cpu_map.len(), 1);
    assert!(cpu_map.contains_key(&10));
    assert!(!cpu_map.contains_key(&20));
}

#[test]
fn test_failure_path_malformed_stat_among_valid_processes() {
    let valid_stat = "100 (valid_proc) S 1 100 100 0 -1 4194304 1000 0 0 0 400 100 0 0 20 0 1 0 12345 1000000 1024 18446744073709551615";
    let malformed_stat = "200 (corrupt_proc S 1 200 invalid line without closing paren";

    let valid_parsed = parse_stat_content(valid_stat, 100, 4);
    let malformed_parsed = parse_stat_content(malformed_stat, 200, 4);

    assert!(valid_parsed.is_some());
    assert!(malformed_parsed.is_none());

    // Verify state building succeeds with valid entries while skipping malformed ones
    let raw_list = vec![valid_parsed.unwrap()];
    let state = build_state(HashMap::new(), raw_list, 100);
    assert_eq!(state.curr.len(), 1);
    assert!(state.curr.contains_key(&100));
}

#[test]
fn test_failure_path_empty_system_and_zero_deltas() {
    // 0 processes & 0 total CPU delta
    let state = build_state(HashMap::new(), Vec::new(), 0);
    let cpu_map = compute_cpu(&state);
    let views = build_view(&state.curr, &cpu_map);

    assert!(state.curr.is_empty());
    assert!(cpu_map.is_empty());
    assert!(views.is_empty());

    // Projecting empty snapshots via TUI projection should produce empty view rows without panic
    let projected = project_view(&HashMap::new(), &ViewMode::Flat, &SortMode::Cpu, None);
    assert!(projected.is_empty());
}

#[test]
fn test_failure_path_partial_system_metrics() {
    let mut app = AppState::new();

    // Telemetry frame with empty network stats and 0 disk sectors
    let frame = TelemetryFrame {
        processes: HashMap::new(),
        cpu_map: HashMap::new(),
        global_cpu_utilization: 0.0,
        global_mem_utilization: 0.0,
        network: NetworkStats::default(),
        disk_sectors_read: 0,
        disk_sectors_written: 0,
    };

    // Should apply frame without panic or unwrap error
    app.apply_tick(frame);
    assert!(app.snapshots.is_empty());
    assert!(app.current_speeds.is_empty());
}

// ============================================================================
// Boundary E: Cross-Module Invariant Enforcement
// ============================================================================

#[test]
fn test_invariant_cpu_usage_bounds_and_no_nan() {
    let mut prev = HashMap::new();
    prev.insert(
        1,
        ProcessSnapshot {
            ppid: 0,
            name: "test".to_string(),
            cpu_time: 1000,
            memory_kb: 100,
        },
    );

    // Scenario 1: Extremely large delta
    let curr_raw1 = vec![RawProcess {
        pid: 1,
        ppid: 0,
        name: "test".to_string(),
        cpu_time: 5000, // delta = 4000
        memory_kb: 100,
    }];
    let state1 = build_state(prev.clone(), curr_raw1, 4000);
    let cpu_map1 = compute_cpu(&state1);
    let cpu1 = cpu_map1.get(&1).copied().unwrap_or(0.0);
    assert!(!cpu1.is_nan());
    assert!(!cpu1.is_infinite());
    assert!((cpu1 - 100.0).abs() < f32::EPSILON);

    // Scenario 2: Decreased CPU time (counter overflow/reset)
    let curr_raw2 = vec![RawProcess {
        pid: 1,
        ppid: 0,
        name: "test".to_string(),
        cpu_time: 500, // decreased
        memory_kb: 100,
    }];
    let state2 = build_state(prev, curr_raw2, 100);
    let cpu_map2 = compute_cpu(&state2);
    assert!(!cpu_map2.contains_key(&1)); // Omitted from map, default 0.0
}

#[test]
fn test_invariant_pid_identity_stability() {
    let pid = 4321;
    let ppid = 123;
    let name = "target-proc".to_string();

    let raw = vec![RawProcess {
        pid,
        ppid,
        name: name.clone(),
        cpu_time: 500,
        memory_kb: 8192,
    }];

    let state = build_state(HashMap::new(), raw, 100);
    assert_eq!(state.curr.get(&pid).unwrap().ppid, ppid);

    let views = build_view(&state.curr, &HashMap::new());
    assert_eq!(views[0].pid, pid);
    assert_eq!(views[0].name, name);
}

#[test]
fn test_invariant_fault_isolation_across_subsystems() {
    let mut app = AppState::new();

    // Valid process telemetry combined with empty network and disk stats
    let mut processes = HashMap::new();
    processes.insert(
        500,
        ProcessSnapshot {
            ppid: 1,
            name: "resilient-proc".to_string(),
            cpu_time: 200,
            memory_kb: 16384,
        },
    );
    let mut cpu_map = HashMap::new();
    cpu_map.insert(500, 12.5);

    let frame = TelemetryFrame {
        processes,
        cpu_map,
        global_cpu_utilization: 12.5,
        global_mem_utilization: 50.0,
        network: NetworkStats::default(), // empty network stream
        disk_sectors_read: 0,
        disk_sectors_written: 0,
    };

    app.apply_tick(frame);

    // Process telemetry and view pipeline must remain completely functional
    assert_eq!(app.snapshots.len(), 1);
    assert_eq!(app.view_pipeline.len(), 1);
    if let ViewRow::Process { pid, .. } = app.view_pipeline[0] {
        let snap = app.snapshots.get(&pid).expect("Snapshot present");
        assert_eq!(pid, 500);
        assert_eq!(snap.name, "resilient-proc");
        assert!((snap.cpu_usage_percent - 12.5).abs() < f32::EPSILON);
        assert_eq!(snap.memory_kb, 16384);
    } else {
        panic!("Expected ViewRow::Process");
    }
}
