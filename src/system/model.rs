/// A point-in-time capture of process metadata and performance metrics.
#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    /// Unique process identifier.
    pub pid: u32,
    /// Parent process identifier.
    pub ppid: u32,
    /// Process name or command line short name.
    pub name: String,
    /// Delta-based CPU usage computed between two collector ticks. Never a raw cumulative value.
    pub cpu_usage_percent: f32,
    /// Resident set size (RSS) in kilobytes.
    pub memory_kb: u64,
    /// Optional container identifier if the process is running within a container namespace.
    pub container_id: Option<String>,
}

/// Defines the primary grouping and display strategy for the process list.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ViewMode {
    /// A flat, sortable list of all processes.
    #[default]
    Flat,
    /// Hierarchical grouping by container identifier.
    Container,
}

/// Criterion used for ordering processes in the UI.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SortMode {
    /// Order by CPU utilization.
    #[default]
    Cpu,
    /// Order by resident memory usage.
    Memory,
}

/// A pure presentation data transfer object representing a single line in the process view.
#[derive(Debug, Clone)]
pub enum ViewRow {
    /// Header row for a container group.
    ContainerHeader {
        /// Container identifier.
        id: String,
        /// Summed CPU usage of all processes in the container.
        aggregated_cpu: f32,
        /// Summed memory usage of all processes in the container.
        aggregated_mem_kb: u64,
    },
    /// Data row representing an individual process.
    Process {
        /// PID of the process.
        pid: u32,
        /// Visual indentation level for tree or group rendering.
        indent_level: u8,
    },
}
