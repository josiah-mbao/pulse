use std::fs;

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_kb: u64,
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
                });
            }
        }
    }

    processes
}
