use anyhow::Context;
use clap::Parser;
use std::process::Command;

#[derive(Parser)]
enum Cli {
    BuildEbpf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli {
        Cli::BuildEbpf => build_ebpf()?,
    }
    Ok(())
}

fn build_ebpf() -> anyhow::Result<()> {
    println!("Building pulse-ebpf...");
    let status = Command::new("cargo")
        .args([
            "build",
            "--package",
            "pulse-ebpf",
            "--target",
            "bpfel-unknown-none",
            "-Z",
            "build-std=core",
        ])
        .status()
        .context("Failed to run cargo build for eBPF")?;

    if !status.success() {
        anyhow::bail!("eBPF build failed");
    }
    Ok(())
}
