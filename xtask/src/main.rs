use anyhow::Context;
use aya::Ebpf;
use aya::maps::RingBuf;
use aya::programs::TracePoint;
use clap::Parser;
use pulse_common::{EVENT_EXEC, TraceEvent};
use std::process::Command;
use tokio::signal;

#[derive(Parser)]
enum Cli {
    /// Build the eBPF program
    BuildEbpf,
    /// Run all CI checks (fmt, clippy, test)
    Ci,
    /// Live process lifecycle tracing
    Trace,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli {
        Cli::BuildEbpf => build_ebpf()?,
        Cli::Ci => run_ci()?,
        Cli::Trace => run_trace().await?,
    }
    Ok(())
}

async fn run_trace() -> anyhow::Result<()> {
    let mut bpf = Ebpf::load_file("target/bpfel-unknown-none/release/pulse-ebpf")
        .context("Failed to load eBPF object")?;

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

    println!("Tracing started. Press Ctrl+C to stop.");

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("Stopping...");
                break;
            }
            Some(event) = async {
                let entry = events.next()?;
                let event = unsafe { core::ptr::read_unaligned(entry.as_ptr() as *const TraceEvent) };
                Some(event)
            } => {
                let type_str = if event.event_type == EVENT_EXEC { "EXEC" } else { "EXIT" };
                let comm = core::str::from_utf8(&event.comm)
                    .unwrap_or("unknown")
                    .trim_end_matches('\0');
                println!("[{}] pid={} comm={}", type_str, event.pid, comm);
            }
        }
    }

    Ok(())
}

fn run_ci() -> anyhow::Result<()> {
    println!("Running cargo fmt...");
    run("cargo", &["fmt", "--all", "--", "--check"])?;

    println!("Running cargo clippy...");
    run(
        "cargo",
        &["clippy", "--all-targets", "--", "-D", "warnings"],
    )?;

    println!("Running cargo test...");
    run("cargo", &["test", "--workspace", "--exclude", "pulse-ebpf"])?;

    println!("CI checks passed!");
    Ok(())
}

fn run(command: &str, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new(command)
        .args(args)
        .status()
        .with_context(|| format!("Failed to run {} {:?}", command, args))?;

    if !status.success() {
        anyhow::bail!("Command failed: {} {:?}", command, args);
    }
    Ok(())
}

fn build_ebpf() -> anyhow::Result<()> {
    println!("Building pulse-ebpf...");
    let workspace_root = std::env::current_dir()?;
    let ebpf_dir = workspace_root.join("pulse-ebpf");

    let status = Command::new("cargo")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTC")
        .env_remove("RUSTDOC")
        .current_dir(ebpf_dir)
        .args(["build", "--release", "--target", "bpfel-unknown-none"])
        .status()
        .context("Failed to run cargo build for eBPF")?;

    if !status.success() {
        anyhow::bail!("eBPF build failed");
    }
    Ok(())
}
