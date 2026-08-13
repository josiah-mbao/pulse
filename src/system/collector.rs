use std::fs;

#[derive(Debug, Clone)]
pub struct RawProcess {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cpu_time: u64,
    pub memory_kb: u64,
}

pub fn parse_stat_content(content: &str, pid: u32, page_size_kb: u64) -> Option<RawProcess> {
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

    Some(RawProcess {
        pid,
        ppid,
        name,
        cpu_time: utime + stime,
        memory_kb: rss * page_size_kb,
    })
}

pub fn parse_stat_file(pid: u32) -> Option<RawProcess> {
    let content = fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;

    let page_size_kb = match unsafe { libc::sysconf(libc::_SC_PAGESIZE) } {
        val if val > 0 => (val as u64) / 1024,
        _ => 4,
    };

    parse_stat_content(&content, pid, page_size_kb)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stat_content_normal() {
        let sample = "1234 (bash) S 100 1234 1234 0 -1 4194304 1000 0 0 0 300 150 0 0 20 0 1 0 12345 1000000 500 18446744073709551615";
        let proc = parse_stat_content(sample, 1234, 4).expect("Failed to parse stat content");

        assert_eq!(proc.pid, 1234);
        assert_eq!(proc.ppid, 100);
        assert_eq!(proc.name, "bash");
        assert_eq!(proc.cpu_time, 450); // utime(300) + stime(150)
        assert_eq!(proc.memory_kb, 2000); // rss(500) * page_size_kb(4)
    }

    #[test]
    fn test_parse_stat_content_with_spaces_and_parens_in_comm() {
        let sample = "9999 (code helper (renderer)) S 1 9999 9999 0 -1 4194304 1000 0 0 0 1000 500 0 0 20 0 1 0 12345 1000000 1024 18446744073709551615";
        let proc = parse_stat_content(sample, 9999, 4).expect("Failed to parse complex comm");

        assert_eq!(proc.pid, 9999);
        assert_eq!(proc.ppid, 1);
        assert_eq!(proc.name, "code helper (renderer)");
        assert_eq!(proc.cpu_time, 1500);
        assert_eq!(proc.memory_kb, 4096);
    }

    #[test]
    fn test_parse_stat_content_malformed() {
        // Missing closing paren
        assert!(parse_stat_content("1234 (bash S 100 1234", 1234, 4).is_none());
        // Inverted parens
        assert!(parse_stat_content("1234 )bash( S 100 1234", 1234, 4).is_none());
        // Missing numeric fields
        assert!(parse_stat_content("1234 (bash) S abc def", 1234, 4).is_none());
    }
}
