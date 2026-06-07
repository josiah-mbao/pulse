# Pulse
A lightweight Linux system observability TUI written in Rust.

### 🎭 Visual Evolution

| Original (v0.1) | v0.7 ("Zinc & Rust") |
| :--- | :--- |
| ![Original Demo](docs/demo.gif) | ![Latest Demo](docs/pulse-demo.gif) |

*Live terminal recordings generated with [asciinema](https://asciinema.org).*

Pulse provides high-density system and process-level metrics by reading directly from Linux kernel virtual filesystems and computing normalized deltas using a decoupled, asynchronous pipeline.

---

## ⚙️ Current Features (v0.7 - "Zinc & Rust")

### 🛰️ Sentinel (Network Lens)
- **High-Density Stages:** Dynamic panels visualizing interface health, operational states, and rx_errors.
- **Throughput Intensity:** Real-time RX/TX rate tracking with visual intensity bars and cumulative volume stats.
- **Robust Parsing:** Specialized handling for kernel edge cases and virtual interfaces.

### 📈 EKG (Telemetry Lens)
- **Heartbeat Sparklines:** Rolling time-series history for global CPU utilization.
- **Disk I/O Velocity:** Real-time storage tracking aggregating sectors read/written across physical drives into KiB/s.

### 🚢 Fleet (Process Lens)
- **Semantic Alerting:** Modern color-coded rows (Amber/Crimson) highlighting high-load processes.
- **Instant Filtering:** Real-time process name search and filtering via `/` key.
- **Dynamic Sorting:** Toggle between CPU and Memory priority with zero-latency updates.

### 🎨 Modern UI Engine
- **Zinc & Rust Theme:** A bespoke, high-contrast palette designed for modern terminals.
- **Fluid Transitions:** 150ms alpha-blended fade transitions powered by `tachyonfx`.
- **Adaptive Polling:** Intelligent loop timing that drops to 16ms during animations for 60fps smoothness.

---

## 🏗️ Architecture & Design

Pulse uses a multi-threaded producer-consumer model to ensure that `/proc` filesystem I/O never blocks the terminal rendering thread.

### 🧱 System Flow

```text
  ┌──────────────────┐      ┌──────────────────────────────┐
  │  Renderer Thread │      │      Collector Thread        │
  │  (Ratatui @60fps)│      │  (Engine Logic @500ms sampling)│
  └────────┬─────────┘      └──────────────┬───────────────┘
           │                               │
           │        MPSC Channel           │
           └◄──────────────────────────────┘
                    (TelemetryFrame)
```

---

## 📁 Structure

```text
src/
├── tui/                 # Terminal UI Layer
│   ├── app.rs           # UI State & Effect Management
│   ├── renderer.rs      # Zinc & Rust Layouts & Styling
│   └── input.rs         # Vim-style navigation & event handling
└── system/              # Telemetry Engine
    ├── collector.rs     # Low-level /proc parser
    ├── state.rs         # Delta computation & snapshot logic
    ├── engine.rs        # Multi-threaded orchestration
    └── [cpu/mem/disk]   # Domain-specific virtual FS parsers
```

---

## 🚧 Roadmap

- **Tree View:** Group processes by parent PID hierarchy.
- **Sentinel Waveforms:** Replace static cards with real-time network sparklines.
- **Process Signals:** Ability to send SIGTERM/SIGKILL directly from the Fleet view.
- **Container Support:** Filter metrics by cgroup/namespace boundaries.

---

## 🧠 Philosophy

Pulse is built from first principles. It is not a wrapper; it is an exploration of the Linux kernel's internal telemetry. The goal is mechanical sympathy: understanding system behavior through direct, zero-allocation observation.
