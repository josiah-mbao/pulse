pub struct SystemSnapshot {
    pub total_cpu: u64,
    pub processes: std::collections::HashMap<u32, u64>,
}
