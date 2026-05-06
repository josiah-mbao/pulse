use std::fs;

pub fn read_uptime() -> f32 {
    // Handle potential read errors gracefully
    let contents = fs::read_to_string("/proc/uptime").unwrap_or_else(|_| "0.0 0.0".to_string());
    contents
        .split_whitespace()
        .next()
        .and_then(|val| val.parse::<f32>().ok())
        .unwrap_or(0.0)
}
