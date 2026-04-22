use pulse::system::memory::{read_memory, memory_usage_percent};
use pulse::system::uptime::read_uptime;
use pulse::system::snapshot::{sample_system, compute_cpu_usage};

use std::{thread::sleep, time::Duration};

#[test]
fn cpu_usage_is_valid_range() {
    let prev = sample_system();
    sleep(Duration::from_millis(200));
    let curr = sample_system();

    let usage = compute_cpu_usage(&prev, &curr);

    for (_pid, cpu) in usage {
        assert!(
            cpu >= 0.0 && cpu <= 100.0,
            "CPU usage out of range: {}",
            cpu
        );
    }
}

#[test]
fn memory_values_are_valid() {
    let (total, available) = read_memory();

    assert!(total > 0, "Total memory should be > 0");
    assert!(
        available <= total,
        "Available memory should not exceed total",
    );
}

#[test]
fn memory_percent_is_valid_range() {
    let (total, available) = read_memory();
    let percent = memory_usage_percent(total, available);

    assert!(
        percent >= 0.0 && percent <= 100.0,
        "Memory usage percent out of range: {}",
        percent
    );
}

#[test]
fn uptime_is_non_negative() {
    let uptime = read_uptime();

    assert!(
        uptime >= 0.0,
        "Uptime should not be negative: {}",
        uptime
    );
}

#[test]
fn system_metrics_do_not_panic() {
    // CPU via snapshot system
    let prev = sample_system();
    sleep(Duration::from_millis(100));
    let curr = sample_system();
    let _usage = compute_cpu_usage(&prev, &curr);

    // Other metrics
    let (_total, _available) = read_memory();
    let _uptime = read_uptime();

    assert!(true);
}
