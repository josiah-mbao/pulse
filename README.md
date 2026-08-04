# Pulse

An open-source, high-performance Linux system observability TUI written in Rust, combining low-overhead `/proc` parsing with real-time eBPF kernel lifecycle tracing.

### 🎭 Visual Evolution

**Latest: v0.7 ("Zinc & Rust" with eBPF Trace Lens)**
![Latest Demo](docs/pulse-demo.gif)

**Legacy: v0.1 (Original)**
![Original Demo](docs/demo.gif)

*Live terminal recordings generated with [asciinema](https://asciinema.org).*

---

## 🌱 The Origin Story

Pulse started on a whim and a hand-me-down laptop. 

After reviving the old machine by installing Arch Linux and setting up a customized Wayland environment with Hyprland, I found myself constantly running `top` and `htop` to diagnose performance bottlenecks and slow-downs. Seeing the screen fill with rapid updates got me thinking: *How do these tools actually capture and display so much moving system telemetry in real time without lagging the system?*

Arch Linux is all about building and customizing your system from the ground up, so I decided to take that philosophy to the application level. I set out to build my own system monitor from scratch to see how system resource observation works at the deepest kernel levels. 

Pulse is the result of that exploration.

---

## ⚙️ Features

### 🔍 Trace (eBPF Process Lifecycle Lens)
*   **Kernel Event Streaming**: Captures `sched_process_exec` and `sched_process_exit` events directly from the Linux kernel using safety-guaranteed eBPF probes.
*   **Zero-Allocation Reducer**: Real-time event propagation utilizing a bounded, lock-free ring buffer directly to the TUI event loop.
*   **Bounded Ring Buffer**: Limits in-memory userspace logs to 500 events using an eviction policy to prevent memory leaks under system load.
*   **Trace Lens View**: A dedicated dashboard screen (key **`4`**) color-coding startup events (`EXEC` in green) and exit events (`EXIT` in crimson) with active process PIDs and names.

### 🛰️ Sentinel (Network Lens)
*   **High-Density Panels**: Visualizes interface status, operational states, and receive/transmit errors.
*   **Throughput Intensity**: Tracks live RX/TX throughput rates with visual intensity bars and cumulative stats.
*   **Stable Sorting**: Deterministic interface indexing to prevent jumping lists.

### 📈 EKG (Telemetry Lens)
*   **Heartbeat Sparklines**: Rolling time-series graphs tracking global CPU utilization.
*   **Disk I/O Velocity**: Real-time read/write rates aggregated across physical block devices.

### 🚢 Fleet (Process Lens)
*   **Stateless Projection**: Functional core for filtering, searching, and structuring process trees.
*   **Flexible Grouping**: Toggle between flat process lists and groupings by container namespaces or virtual "hosts".
*   **Signals & Control**: Send process signals (`SIGTERM` or `SIGKILL`) directly from the UI.

---

## 🧱 Architecture

Pulse uses a multi-threaded producer-consumer pipeline ensuring filesystem and kernel I/O never block TUI drawing loops.

```text
       KERNEL SPACE            │                 USERSPACE (TUI)
                               │
 ┌──────────────────────────┐  │  ┌───────────────┐      ┌────────────────┐
 │   sched_process_exec/    │  │  │  TUI Renderer │      │    Collector   │
 │   sched_process_exit     │  │  │ (Stateless v4)│      │ (/proc parser) │
 └────────────┬─────────────┘  │  └───────┬───────┘      └───────┬────────┘
              │ (eBPF RingBuf) │          ▲                      │
              ▼                │          │                      │
 ┌──────────────────────────┐  │   View DTO Frame         SystemEvent
 │   ebpf_collector Thread  ├ ─┼──────────┼──────────────────────┘
 │   (Aya-driven consumer)  │  │   ┌──────┴──────────┐
 └──────────────────────────┘  │   │Projection Engine│◄── AppState Reducer
                               │   └─────────────────┘
```

---

## 📁 Workspace Structure

Pulse is structured as a multi-crate Rust workspace:

*   **`pulse`** (Core): The userspace entry point, engine, state reducers, and Ratatui-based rendering layers.
*   **`pulse-common`**: Shared POD (Plain Old Data) structs compiled under `#![no_std]` for binary-level compatibility between userspace and the kernel.
*   **`pulse-ebpf`**: The kernel-space eBPF program utilizing `aya-ebpf` to hook into kernel tracepoints.
*   **`xtask`**: Development build script pipeline (automates eBPF compilation, local CLI tracing, and workspace check pipelines).

---

## ⚡ Getting Started

```bash
# Download and run the latest release (Linux x86_64)
curl -sSL https://github.com/josiah-mbao/pulse/releases/latest/download/pulse -o pulse
chmod +x pulse
sudo ./pulse
```

### Prerequisites
*   **Rust (Nightly)**: Required for building the `#![no_std]` eBPF program (`bpfel-unknown-none` target).
*   **LLVM / Clang**: For compiled eBPF assets.
*   **bpf-linker**: Installed via cargo:
    ```bash
    cargo install bpf-linker
    ```

### Compilation & Running

1.  **Clone the Repository**:
    ```bash
    git clone https://github.com/josiah-mbao/pulse.git
    cd pulse
    ```
2.  **Build the eBPF Program**:
    Use the `xtask` workspace script to compile the eBPF probes in release mode (optimizations are required for BPF verifier loading):
    ```bash
    cargo run --package xtask -- build-ebpf
    ```
3.  **Run the Observability TUI**:
    Since loading eBPF maps and attaching to tracepoints requires superuser permissions, launch the compiled binary with `sudo`:
    ```bash
    sudo ./target/debug/pulse
    ```
    *   *Press keys **`1`**, **`2`**, **`3`**, or **`4`** to switch between Fleet, EKG, Sentinel, and Trace lenses respectively.*

---

## 🛠️ Helper Diagnostics & Development

*   **Standalone Trace Console**:
    Run a terminal-only version of the eBPF tracer directly dump process events to stdout:
    ```bash
    sudo cargo run --package xtask -- trace
    ```
*   **Workspace Checks (Format, Clippy, Tests)**:
    Run the workspace sanity checker script before staging changes:
    ```bash
    cargo run --package xtask -- ci
    ```

---

## 📄 License

This project is open-source and licensed under the [MIT License](LICENSE).
