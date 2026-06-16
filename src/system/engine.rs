use crate::system::collector::collect_processes;
use crate::system::cpu::read_total_cpu_time;
use crate::system::ebpf_collector::{load_ebpf, run_trace_task};
use crate::system::model::{EventSender, SystemEvent, TelemetryFrame};
use crate::system::state::{
    CpuJiffies, ProcessSnapshot, build_state, compute_cpu, read_disk_io, read_global_jiffies,
    read_global_mem_percent, read_network_dev,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub struct Engine {
    prev_processes: HashMap<u32, ProcessSnapshot>,
    prev_total_cpu: u64,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            prev_processes: HashMap::new(),
            prev_total_cpu: read_total_cpu_time(),
        }
    }

    pub fn tick(&mut self) -> (HashMap<u32, ProcessSnapshot>, HashMap<u32, f32>) {
        let curr_total_cpu = read_total_cpu_time();
        let total_delta = curr_total_cpu.saturating_sub(self.prev_total_cpu);

        let raw = collect_processes();

        let state = build_state(self.prev_processes.clone(), raw, total_delta);
        let cpu_map = compute_cpu(&state);

        self.prev_processes = state.curr.clone();
        self.prev_total_cpu = curr_total_cpu;

        (state.curr, cpu_map)
    }

    pub fn spawn_collectors(mut self, shutdown: Arc<AtomicBool>) -> mpsc::Receiver<SystemEvent> {
        let (sync_tx, rx) = mpsc::sync_channel(1024);
        let event_tx = EventSender::new(sync_tx);

        // Spawn eBPF collector
        let ebpf_tx = event_tx.clone();
        let ebpf_shutdown = Arc::clone(&shutdown);
        thread::spawn(move || {
            if let Ok(bpf) = load_ebpf() {
                let _ = run_trace_task(bpf, ebpf_tx, ebpf_shutdown);
            }
        });

        // Spawn /proc collector
        let proc_tx = event_tx;
        let proc_shutdown = Arc::clone(&shutdown);
        thread::spawn(move || {
            let mut prev_jiffies =
                read_global_jiffies().unwrap_or(CpuJiffies { total: 0, idle: 0 });
            while !proc_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                let (procs, cpu) = self.tick();

                let mut global_cpu = 0.0;
                if let Some(curr_jiffies) = read_global_jiffies() {
                    let total_d = curr_jiffies.total.saturating_sub(prev_jiffies.total);
                    let idle_d = curr_jiffies.idle.saturating_sub(prev_jiffies.idle);
                    if total_d > 0 {
                        global_cpu = ((total_d - idle_d) as f32 / total_d as f32) * 100.0;
                    }
                    prev_jiffies = curr_jiffies;
                }

                let global_mem = read_global_mem_percent();
                let (disk_r, disk_w) = read_disk_io().unwrap_or((0, 0));

                let frame = TelemetryFrame {
                    processes: procs,
                    cpu_map: cpu,
                    global_cpu_utilization: global_cpu,
                    global_mem_utilization: global_mem,
                    network: read_network_dev().unwrap_or_default(),
                    disk_sectors_read: disk_r,
                    disk_sectors_written: disk_w,
                };

                proc_tx.send(SystemEvent::Tick(frame));
                thread::sleep(Duration::from_millis(500));
            }
        });

        rx
    }
}
