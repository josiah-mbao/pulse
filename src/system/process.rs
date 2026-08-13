use std::fs;

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_kb: u64,
    pub cpu_percent: f32,
}

pub fn parse_comm(content: &str) -> String {
    content.trim().to_string()
}

fn read_cmdline(pid: u32) -> Option<String> {
    let path = format!("/proc/{}/comm", pid);
    fs::read_to_string(path).ok().map(|s| parse_comm(&s))
}

pub fn parse_vm_rss(content: &str) -> Option<u64> {
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

fn read_memory(pid: u32) -> Option<u64> {
    let path = format!("/proc/{}/status", pid);
    let content = fs::read_to_string(path).ok()?;
    parse_vm_rss(&content)
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

        if let Ok(pid) = name.parse::<u32>()
            && let (Some(name), Some(mem)) = (read_cmdline(pid), read_memory(pid))
        {
            processes.push(ProcessInfo {
                pid,
                name,
                memory_kb: mem,
                cpu_percent: 0.0,
            });
        }
    }

    processes
}

pub fn parse_process_cpu_time(content: &str) -> Option<u64> {
    let open_paren = content.find('(')?;
    let close_paren = content.rfind(')')?;
    if open_paren >= close_paren {
        return None;
    }

    let rest = &content[close_paren + 1..];
    let parts: Vec<&str> = rest.split_whitespace().collect();

    let utime: u64 = parts.get(11)?.parse().ok()?;
    let stime: u64 = parts.get(12)?.parse().ok()?;

    Some(utime + stime)
}

pub fn read_cpu_time(pid: u32) -> Option<u64> {
    let path = format!("/proc/{}/stat", pid);
    let content = fs::read_to_string(path).ok()?;
    parse_process_cpu_time(&content)
}

pub fn parse_status_extra_info(content: &str) -> (u32, u32, String) {
    let mut ppid = 0;
    let mut threads = 0;
    let mut state = String::from("Unknown");

    for line in content.lines() {
        if line.starts_with("PPid:") {
            ppid = line
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("Threads:") {
            threads = line
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("State:") {
            state = line
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ");
        }
    }
    (ppid, threads, state)
}

pub fn get_extra_info(pid: u32) -> Option<(u32, u32, String)> {
    let status_path = format!("/proc/{}/status", pid);
    let content = fs::read_to_string(status_path).ok()?;
    Some(parse_status_extra_info(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_comm() {
        assert_eq!(parse_comm(" bash \n"), "bash");
    }

    #[test]
    fn test_parse_vm_rss() {
        let status = "Name:\tbash\nState:\tS (sleeping)\nVmRSS:\t   1024 kB\nThreads:\t1\n";
        assert_eq!(parse_vm_rss(status), Some(1024));
        assert_eq!(parse_vm_rss("Name:\tbash\n"), None);
    }

    #[test]
    fn test_parse_process_cpu_time_standard_and_with_spaces() {
        let standard = "100 (bash) S 1 100 100 0 -1 4194304 1000 0 0 0 400 100 0 0 20 0 1 0";
        assert_eq!(parse_process_cpu_time(standard), Some(500));

        let with_spaces =
            "200 (code helper (renderer)) S 1 200 200 0 -1 4194304 1000 0 0 0 800 200 0 0 20 0 1 0";
        assert_eq!(parse_process_cpu_time(with_spaces), Some(1000));
    }

    #[test]
    fn test_parse_status_extra_info() {
        let status = "Name:\tbash\nState:\tS (sleeping)\nPPid:\t42\nThreads:\t4\n";
        let (ppid, threads, state) = parse_status_extra_info(status);
        assert_eq!(ppid, 42);
        assert_eq!(threads, 4);
        assert_eq!(state, "S (sleeping)");
    }
}
