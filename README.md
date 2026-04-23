# Pulse

A lightweight Linux system observability CLI written in Rust.

Pulse reads directly from the Linux `/proc` filesystem to provide **system-level and process-level insights**, using time-based sampling to compute accurate CPU usage.

---

## ⚙️ Current Features (v0.3)

### 🖥️ `pulse status`
Displays system-wide metrics:

- CPU usage (delta-based calculation)
- Memory usage (% used)
- System uptime

---

### 📊 `pulse top`
Displays process-level metrics:

- Per-process CPU usage (time-based, delta computed)
- Memory usage (RSS)
- Sorted output (CPU or memory)
- Top processes view

---

## 🧠 How It Works

Pulse reads raw system data directly from Linux:

- `/proc/stat` → total CPU time
- `/proc/[pid]/stat` → per-process CPU time
- `/proc/[pid]/status` → memory usage (VmRSS)
- `/proc/meminfo` → system memory
- `/proc/uptime` → system uptime

---

### ⚡ CPU Calculation (Key Concept)

CPU usage is **not directly readable**.

Pulse computes it using a **delta-based sampling model**:


## 🧱 Architecture

Pulse is split into two layers:

CLI Layer (presentation) --> System Layer (metrics engine) --> /proc filesystem (Linux kernel interface)

### Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # system module exposure
├── cli/                 # CLI layer (commands + output)
│   ├── status.rs
│   ├── commands.rs
│   └── mod.rs
└── system/              # system metrics engine
    ├── cpu.rs
    ├── memory.rs
    ├── uptime.rs
    ├── snapshot.rs
    └── mod.rs

```


---

## 📌 Design Goals

- Separate system logic from presentation
- Build from first principles (no external monitoring libs)
- Use Linux-native interfaces (`/proc`)
- Model time-based metrics correctly
- Keep the codebase minimal and understandable

---

## ⚠️ Limitations

- Linux-only (depends on `/proc`)
- Snapshot-based (not yet real-time updating)
- CPU normalization across cores is basic
- No historical metrics storage
- Full `/proc` scan per sample (not optimized yet)

---

## 🚧 Roadmap

### Phase 5 — Live Monitoring
- Continuous refresh loop (`top`-like behavior)
- Real-time updating interface

### Phase 6 — Performance & Accuracy
- Multi-core CPU normalization
- Efficient `/proc` scanning
- Improved error handling

### Phase 7 — UX Improvements
- Better formatting and sorting options
- Optional colored output

### Phase 8 — Advanced Features
- JSON output
- Logging/export mode
- Lightweight daemon mode

---

## 🧠 Philosophy

Pulse is built to understand **how system monitoring actually works under the hood**, not just to replicate existing tools.

---
