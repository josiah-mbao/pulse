# Pulse
A lightweight Linux system observability TUI written in Rust.

![Demo of Pulse](docs/demo.gif)

Pulse provides real-time system and process-level metrics by reading directly from the Linux `/proc` filesystem and computing normalized CPU usage using asynchronous time-delta sampling.

---

## ⚙️ Current Features (v0.4)

### 🖥️ System Dashboard
- **Multi-threaded I/O:** Background data collection ensures zero UI lag.
- **Real-time Metrics:** Normalized CPU usage and Memory utilization gauges.
- **Uptime Tracking:** Live system uptime display.

### 📊 Process Monitoring
- **Normalized CPU %:** Usage calculated against total system capacity, not just active processes.
- **Memory Tracking:** Resident Set Size (RSS) monitoring.
- **Interactive UI:** Instant sorting toggle (CPU/Mem) and pause functionality.

---

## 🏗️ Architecture & Design

Pulse uses a multi-threaded producer-consumer model to ensure that heavy `/proc` filesystem I/O never blocks the terminal rendering thread.

### 🧱 System Flow

```text
  ┌──────────────────┐      ┌──────────────────────────────┐
  │  Renderer Thread │      │      Collector Thread        │
  │  (Ratatui @60fps)│      │  (Engine Logic @1s sampling) │
  └────────┬─────────┘      └──────────────┬───────────────┘
           │                               │
           │        MPSC Channel           │
           └◄──────────────────────────────┘
                    (SystemState)
```

---

## ⚡ CPU Usage Model

Pulse calculates CPU utilization using a delta-based approach normalized against total system jiffies:

$$CPU\% = \frac{Process\Delta}{SystemTotal\Delta} \times 100$$

---

## 📁 Structure

```text
src/
├── main.rs              # App entry point
├── tui/                 # Terminal UI Layer
│   ├── app.rs           # UI Loop & Thread Management
│   ├── input.rs         # Non-blocking input handling
│   └── renderer.rs      # Layout & Widget definitions
└── system/              # System Engine
    ├── cpu.rs           # /proc/stat parser
    ├── memory.rs        # /proc/meminfo parser
    ├── engine.rs        # State orchestration
    ├── state.rs         # Delta computation logic
    └── process.rs       # /proc/[pid] parsing
```

---

## 🚧 Roadmap (Phase 5+)

- **Historical Metrics:** Implement a local time-series buffer for short-term history.
- **Filtering:** Add a search/filter bar to the process table.
- **Tree View:** Group processes by parent PID.
- **Export Formats:** Support for JSON or log-based snapshots.

---

## 🧠 Philosophy

Pulse is built from first principles to understand Linux observability. It is not a wrapper; it is an exploration of the `/proc` filesystem. The goal is understanding how the system behaves, not abstracting it away.
