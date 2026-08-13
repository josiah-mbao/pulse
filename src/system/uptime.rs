use std::fs;

pub fn parse_uptime(contents: &str) -> f32 {
    contents
        .split_whitespace()
        .next()
        .and_then(|val| val.parse::<f32>().ok())
        .unwrap_or(0.0)
}

pub fn read_uptime() -> f32 {
    // Handle potential read errors gracefully
    let contents = fs::read_to_string("/proc/uptime").unwrap_or_else(|_| "0.0 0.0".to_string());
    parse_uptime(&contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uptime_valid() {
        let sample = "12345.67 89012.34\n";
        let val = parse_uptime(sample);
        assert!((val - 12345.67).abs() < 1e-3);
    }

    #[test]
    fn test_parse_uptime_malformed() {
        assert_eq!(parse_uptime("invalid 123"), 0.0);
        assert_eq!(parse_uptime(""), 0.0);
    }
}
