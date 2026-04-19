use crate::system::cpu::read_cpu_usage;
use crate::system::memory::{read_memory, memory_usage_percent};
use crate::system::uptime::read_uptime;

pub fn run_status() {
    let cpu = read_cpu_usage();

    let (total, available) = read_memory();
    let mem = memory_usage_percent(total, available);

    let uptime = read_uptime();

    println!("=== Pulse System Status ===");
    println!("CPU:      {:.2}%", cpu);
    println!("Memory:   {:.2}%", mem);
    println!("Uptime:   {:.2} seconds", uptime);
}
