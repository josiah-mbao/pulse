use std::fs;

#[derive(Debug, Clone)]
pub struct RawProcess {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cpu_time: u64,
    pub memory_kb: u64,
}

pub fn parse_stat_file(pid: u32) -> Option<RawProcess> {
    let content = fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;

    // Find the first '(' and the last ')' to extract process name (comm)
    let open_paren = content.find('(')?;
    let close_paren = content.rfind(')')?;
    if open_paren >= close_paren {
        return None;
    }

    let name = content[open_paren + 1..close_paren].to_string();

    // The rest of the fields start after the last ')'
    let rest = &content[close_paren + 1..];
    let parts: Vec<&str> = rest.split_whitespace().collect();

    // Index mapping (0-based after the last ')')
    // index 0: state (Field 3)
    // index 1: ppid (Field 4)
    // index 11: utime (Field 14)
    // index 12: stime (Field 15)
    // index 21: rss (Field 24)

    let ppid: u32 = parts.get(1)?.parse().ok()?;
    let utime: u64 = parts.get(11)?.parse().ok()?;
    let stime: u64 = parts.get(12)?.parse().ok()?;
    let rss: u64 = parts.get(21)?.parse().ok()?;

    let page_size_kb = match unsafe { libc::sysconf(libc::_SC_PAGESIZE) } {
        val if val > 0 => (val as u64) / 1024,
        _ => 4,
    };

    Some(RawProcess {
        pid,
        ppid,
        name,
        cpu_time: utime + stime,
        memory_kb: rss * page_size_kb,
    })
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

        if let Some(proc) = parse_stat_file(pid) {
            out.push(proc);
        }
    }

    out
}
