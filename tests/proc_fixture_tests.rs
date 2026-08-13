use pulse::system::collector::parse_stat_content;
use pulse::system::cpu::parse_total_cpu_time;
use pulse::system::memory::parse_meminfo;
use pulse::system::process::{
    parse_comm, parse_process_cpu_time, parse_status_extra_info, parse_vm_rss,
};
use pulse::system::state::{parse_diskstats, parse_global_jiffies};
use pulse::system::uptime::parse_uptime;

use std::fs;
use std::io::Cursor;

#[test]
fn fixture_test_proc_meminfo() {
    let content =
        fs::read_to_string("tests/fixtures/proc/proc_meminfo").expect("Read fixture proc_meminfo");
    let (total, avail) = parse_meminfo(&content);
    assert_eq!(total, 32_800_000);
    assert_eq!(avail, 16_400_000);
}

#[test]
fn fixture_test_proc_uptime() {
    let content =
        fs::read_to_string("tests/fixtures/proc/proc_uptime").expect("Read fixture proc_uptime");
    let uptime = parse_uptime(&content);
    assert!((uptime - 98765.43).abs() < 1e-2);
}

#[test]
fn fixture_test_proc_stat() {
    let content =
        fs::read_to_string("tests/fixtures/proc/proc_stat").expect("Read fixture proc_stat");
    let total_cpu = parse_total_cpu_time(&content);
    // 123456 + 7890 + 23456 + 987654 + 3456 + 123 + 456 = 1146491
    assert_eq!(total_cpu, 1_146_491);

    let first_line = content.lines().next().unwrap();
    let jiffies = parse_global_jiffies(first_line).expect("Parse global jiffies");
    assert_eq!(jiffies.total, 1_146_491);
    assert_eq!(jiffies.idle, 987654 + 3456);
}

#[test]
fn fixture_test_proc_diskstats() {
    let content = fs::read_to_string("tests/fixtures/proc/proc_diskstats")
        .expect("Read fixture proc_diskstats");
    let (read_sectors, write_sectors) =
        parse_diskstats(Cursor::new(content)).expect("Parse diskstats");
    // sda: 4000000 read, 8000000 write
    // loop0: skipped
    // nvme0n1: 9600000 read, 19200000 write
    // total read = 13600000, write = 27200000
    assert_eq!(read_sectors, 13_600_000);
    assert_eq!(write_sectors, 27_200_000);
}

#[test]
fn fixture_test_pid_stat_with_spaces() {
    let content =
        fs::read_to_string("tests/fixtures/proc/pid_stat").expect("Read fixture pid_stat");
    let proc = parse_stat_content(&content, 54321, 4).expect("Parse stat content");

    assert_eq!(proc.pid, 54321);
    assert_eq!(proc.ppid, 1);
    assert_eq!(proc.name, "pulse-worker (pool)");
    assert_eq!(proc.cpu_time, 1600); // utime 1200 + stime 400
    assert_eq!(proc.memory_kb, 8192); // rss 2048 * 4

    let cpu_time = parse_process_cpu_time(&content).expect("Parse process cpu time");
    assert_eq!(cpu_time, 1600);
}

#[test]
fn fixture_test_pid_status() {
    let content =
        fs::read_to_string("tests/fixtures/proc/pid_status").expect("Read fixture pid_status");
    let rss = parse_vm_rss(&content).expect("Parse VmRSS");
    assert_eq!(rss, 8192);

    let (ppid, threads, state) = parse_status_extra_info(&content);
    assert_eq!(ppid, 1);
    assert_eq!(threads, 8);
    assert_eq!(state, "R (running)");
}

#[test]
fn fixture_test_comm() {
    assert_eq!(parse_comm("pulse-worker\n"), "pulse-worker");
}
