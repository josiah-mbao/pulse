use anyhow::Context;
use clap::Parser;
use std::process::Command;

#[derive(Parser)]
enum Cli {
    /// Build the eBPF program
    BuildEbpf,
    /// Run all CI checks (fmt, clippy, test)
    Ci,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli {
        Cli::BuildEbpf => build_ebpf()?,
        Cli::Ci => run_ci()?,
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
    run(
        "cargo",
        &[
            "build",
            "--package",
            "pulse-ebpf",
            "--target",
            "bpfel-unknown-none",
            "-Z",
            "build-std=core",
        ],
    )
}
