use pulse::system::get_processes();

pub fn run_top() {
    let mut processes = get_processes();

    processes.sort_by(|a, b| b.memory_kb.cmp(&a.memory_kb));

    println!("{:<6} {:<20} {:<20}", "PID", "NAME", "MEM(KB)");

    for p in processes.iter().take(15) {
        println!("{:<6", "{:<20)}", "{:<10}", p.pid, p.name. p.memory_kb);
    }
}
