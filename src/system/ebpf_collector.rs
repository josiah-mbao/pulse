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
    // eprintln!("DEBUG: load_ebpf() entered");
    let res = Ebpf::load_file("target/bpfel-unknown-none/release/pulse-ebpf")
        .context("Failed to load eBPF object");
    if res.is_ok() {
        // eprintln!("DEBUG: load_ebpf() returned successfully");
    }
    res
}

pub fn run_trace_task(
    mut bpf: Ebpf,
    tx: EventSender,
    shutdown: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    // eprintln!("DEBUG: run_trace_task() entered");
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

    // eprintln!("DEBUG: tracepoints successfully attached");

    let mut events: RingBuf<_> = bpf
        .map_mut("EVENTS")
        .context("Map EVENTS not found")?
        .try_into()?;

    // eprintln!("DEBUG: ring buffer successfully opened");

    let mut first_event_received = false;
    let mut first_event_sent = false;

    while !shutdown.load(Ordering::Relaxed) {
        match events.next() {
            Some(entry) => {
                if !first_event_received {
                    // eprintln!("DEBUG: first ring buffer event received");
                    first_event_received = true;
                }
                let event =
                    unsafe { core::ptr::read_unaligned(entry.as_ptr() as *const TraceEvent) };
                if !first_event_sent {
                    // eprintln!("DEBUG: first SystemEvent::Trace sent");
                    first_event_sent = true;
                }
                tx.send(SystemEvent::Trace(event));
            }
            None => {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    Ok(())
}
