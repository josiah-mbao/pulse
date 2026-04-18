use std::fs;

pub fn read_uptime() -> f32 {
    let contents = fs::read_to_string("/proc/uptime").unwrap();
    let uptime = contents.split_whitespace().next().unwrap();
    uptime.parse::<f32>().unwrap()
}
