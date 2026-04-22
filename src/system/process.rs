use std::fs;

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_kb: u64,
    pub cpu_percent: f32,
}

fn read_cmdline(pid: u32) -> Option<String> {
    let path = format!("/proc/{}/comm", pid);
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_memory(pid: u32) -> Option<u64> {
    let path = format!("/proc/{}/status", pid);
    let content = fs::read_to_string(path).ok()?;

    for line in content.lines() {
        if line.starts_with("VmRSS:") {
            return line
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse::<u64>().ok());
        }
    }

    None
}

pub fn get_processes() -> Vec<ProcessInfo> {
    let mut processes = Vec::new();

    let entries = match fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => return processes,
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if let Ok(pid) = name.parse::<u32>() {
            if let (Some(name), Some(mem)) = (read_cmdline(pid), read_memory(pid)) {
                processes.push(ProcessInfo {
                    pid,
                    name,
                    memory_kb: mem,
                    cpu_percent: 0.0,
                });
            }
        }
    }

    processes
}

pub fn read_cpu_time(pid: u32) -> Option<u64> {
    let path = format!("/proc/{}/stat", pid);
    let content = fs::read_to_string(path).ok()?;

    let parts: Vec<&str> = content.split_whitespace().collect();

    let utime: u64 = parts.get(13)?.parse().ok()?;
    let stime: u64 = parts.get(14)?.parse().ok()?;

    Some(utime + stime)
}
