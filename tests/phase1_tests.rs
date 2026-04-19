use pulse::system::cpu::read_cpu_usage;
use pulse::system::memory::{read_memory, memory_usage_percent};
use pulse::system::uptime::read_uptime;

#[test]
fn cpu_is_valid_range() {
    let cpu = read_cpu_usage();

    assert!(
        cpu >= 0.0 && cpu <= 100.0,
        "CPU usage out of range: {}",
        cpu
    );
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
    // This ensures your core system calls are stable
    let _cpu = read_cpu_usage();
    let (_total, _available) = read_memory();
    let _uptime = read_uptime();

    assert!(true);
}
