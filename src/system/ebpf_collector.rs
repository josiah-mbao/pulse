use crate::system::model::EventSender;
use crate::system::model::SystemEvent;
use anyhow::Context;
use aya::Ebpf;
use aya::maps::RingBuf;
use aya::programs::TracePoint;
use pulse_common::TraceEvent;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub fn load_ebpf() -> anyhow::Result<Ebpf> {
    Ebpf::load_file("target/bpfel-unknown-none/release/pulse-ebpf")
        .context("Failed to load eBPF object")
}

pub fn run_trace_task(
    mut bpf: Ebpf,
    tx: EventSender,
    shutdown: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let prog_exec: &mut TracePoint = bpf
        .program_mut("sched_process_exec")
        .context("Program sched_process_exec not found")?
        .try_into()?;
    prog_exec.load()?;
    prog_exec.attach("sched", "sched_process_exec")?;

    let prog_exit: &mut TracePoint = bpf
        .program_mut("sched_process_exit")
        .context("Program sched_process_exit not found")?
        .try_into()?;
    prog_exit.load()?;
    prog_exit.attach("sched", "sched_process_exit")?;

    let mut events: RingBuf<_> = bpf
        .map_mut("EVENTS")
        .context("Map EVENTS not found")?
        .try_into()?;

    while !shutdown.load(Ordering::Relaxed) {
        match events.next() {
            Some(entry) => {
                let event =
                    unsafe { core::ptr::read_unaligned(entry.as_ptr() as *const TraceEvent) };
                tx.send(SystemEvent::Trace(event));
            }
            None => {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    Ok(())
}
