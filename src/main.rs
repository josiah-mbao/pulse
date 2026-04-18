mod system;

use system::cpu::read_cpu_usage;
use system::memory::{read_memory, memory_usage_percent};
use system::uptime::read_uptime;

fn main() {
    let cpu = read_cpu_usage();
    
    let (total, available) = read_memory();
    let mem = memory_usage_percent(total, available);

    let uptime = read_uptime();

    println!("=== Pulse System Status (Phase 1) ===");
    println!("CPU: {:.2}%", cpu);
    println!("Memory: {:.2}%", mem);
    println!("Uptime: {:.2} seconds", uptime);
}
