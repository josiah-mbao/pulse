# Contributing to Pulse

Thank you for your interest in contributing to **Pulse**!

Pulse is a Linux observability TUI written in Rust, combining `/proc` telemetry with eBPF kernel lifecycle tracing.

---

## 🛠️ Development Setup

### 1. Prerequisites

- **Linux OS** (Kernel >= 5.8 recommended for eBPF tracepoints)
- **Rust Toolchain** (Stable for userspace, Nightly for eBPF target)
- **LLVM / Clang**
- **`bpf-linker`**:

```bash
rustup toolchain install nightly --component rust-src
rustup target add bpfel-unknown-none --toolchain nightly
cargo install bpf-linker
```

---

## 🏗️ Building & Running

### Build the eBPF Kernel Program

```bash
cargo run --package xtask -- build-ebpf
```

### Run Pulse in Development Mode

```bash
cargo run --package xtask -- dev
```

### Stream Live Kernel Traces (Development Console)

```bash
sudo cargo run --package xtask -- trace
```

---

## 🧪 Quality Gates & Testing

Before submitting a Pull Request, all standard quality gates must pass:

```bash
# 1. Format check
cargo fmt --all -- --check

# 2. Linting
cargo clippy --all-targets -- -D warnings

# 3. Unit & Integration Test Suite
cargo test --workspace

# 4. Benchmark Compilation Check
cargo bench -- --test
```

Alternatively, run the automated CI helper:

```bash
cargo run --package xtask -- ci
```

---

## 🏛️ Code Architecture Guidelines

1. **No Architectural Rewrites**: Maintain the existing multi-threaded pipeline:
   `Kernel/eBPF -> Collectors -> Engine -> AppState -> Projection -> Renderer`
2. **POD-Safe Shared Types**: Types in `pulse-common` must be `#[repr(C)]`, fixed-size, POD-safe, and `#![no_std]`.
3. **Userspace Responsibility**: All logic, formatting, filtering, and aggregation belong in userspace.
4. **Deterministic Testing**: Keep all tests non-flaky and runnable without root (use `#[ignore]` for privileged eBPF tests).
