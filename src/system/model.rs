use std::collections::HashMap;

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

#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceSnapshot {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub operstate: String,
    pub rx_errors: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkStats {
    pub interfaces: HashMap<String, InterfaceSnapshot>,
}

/// The unified carrier payload bridging background I/O to the frontend loop
#[derive(Clone, Debug)]
pub struct TelemetryFrame {
    pub processes: HashMap<u32, crate::system::state::ProcessSnapshot>,
    pub cpu_map: HashMap<u32, f32>,
    pub global_cpu_utilization: f32,
    pub global_mem_utilization: f32,
    pub network: NetworkStats,
    pub disk_sectors_read: u64,
    pub disk_sectors_written: u64,
}

#[derive(Debug, Clone)]
pub enum SystemEvent {
    Trace(pulse_common::TraceEvent),
    Tick(TelemetryFrame),
}

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// A cloneable wrapper for the system-wide event bus sender.
/// Implements a centralized drop-on-full policy for backpressure management.
#[derive(Clone)]
pub struct EventSender {
    inner: std::sync::mpsc::SyncSender<SystemEvent>,
    dropped_traces: Arc<AtomicU64>,
    dropped_ticks: Arc<AtomicU64>,
}

impl EventSender {
    pub fn new(inner: std::sync::mpsc::SyncSender<SystemEvent>) -> Self {
        Self {
            inner,
            dropped_traces: Arc::new(AtomicU64::new(0)),
            dropped_ticks: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Attempts to send an event. If the buffer is full, the event is silently dropped
    /// to ensure producers never block the system.
    pub fn send(&self, event: SystemEvent) {
        if let Err(std::sync::mpsc::TrySendError::Full(e)) = self.inner.try_send(event) {
            match e {
                SystemEvent::Trace(_) => {
                    self.dropped_traces.fetch_add(1, Ordering::Relaxed);
                }
                SystemEvent::Tick(_) => {
                    self.dropped_ticks.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn dropped_traces(&self) -> u64 {
        self.dropped_traces.load(Ordering::Relaxed)
    }

    pub fn dropped_ticks(&self) -> u64 {
        self.dropped_ticks.load(Ordering::Relaxed)
    }
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
