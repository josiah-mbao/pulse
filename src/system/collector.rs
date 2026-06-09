use std::fs;

#[derive(Debug, Clone)]
pub struct RawProcess {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cpu_time: u64,
    pub memory_kb: u64,
}

fn read_name(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{}/comm", pid))
        .ok()
        .map(|s| s.trim().to_string())
}

fn read_memory(pid: u32) -> Option<u64> {
    let content = fs::read_to_string(format!("/proc/{}/status", pid)).ok()?;

    for line in content.lines() {
        if line.starts_with("VmRSS:") {
            return line.split_whitespace().nth(1)?.parse().ok();
        }
    }

    None
}

/// Parses CPU time and PPID from /proc/[pid]/stat
/// PPID is the 4th field (index 3), utime is index 13, stime is index 14.
fn read_stat_info(pid: u32) -> Option<(u32, u64)> {
    let content = fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let parts: Vec<&str> = content.split_whitespace().collect();

    let ppid: u32 = parts.get(3)?.parse().ok()?;
    let utime: u64 = parts.get(13)?.parse().ok()?;
    let stime: u64 = parts.get(14)?.parse().ok()?;

    Some((ppid, utime + stime))
}

pub fn collect_processes() -> Vec<RawProcess> {
    let mut out = Vec::new();

    let entries = match fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => return out,
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let pid_str = file_name.to_string_lossy();

        let pid = match pid_str.parse::<u32>() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let name = match read_name(pid) {
            Some(n) => n,
            None => continue,
        };

        let memory_kb = match read_memory(pid) {
            Some(m) => m,
            None => continue,
        };

        let (ppid, cpu_time) = match read_stat_info(pid) {
            Some(info) => info,
            None => continue,
        };

        out.push(RawProcess {
            pid,
            ppid,
            name,
            memory_kb,
            cpu_time,
        });
    }

    out
}
