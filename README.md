# Pulse

A lightweight Linux system observability CLI written in Rust.

Pulse provides real-time system metrics such as CPU usage, memory usage, and uptime by reading directly from the Linux `/proc` filesystem.

---

## ⚙️ Current Features (v0.2)

- `pulse status` — displays system metrics:
  - CPU usage (delta-based calculation)
  - Memory usage (% used)
  - System uptime

---

## 🧠 How It Works

Pulse reads raw system data directly from Linux:

- `/proc/stat` → CPU usage calculation
- `/proc/meminfo` → memory usage
- `/proc/uptime` → system uptime

CPU usage is computed using a **delta-based sampling method**, similar to traditional system monitors.

---

## 🧱 Architecture

Pulse is split into two layers:
CLI Layer (presentation) --> System Layer (metrics engine) --> /proc filesystem (Linux kernel interface)

### Structure


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
    └── mod.rs
```

## 📌 Design Goals
- Keep system logic separate from CLI
- Avoid external system monitoring dependencies
- Use Linux-native data sources (/proc)
- Maintain minimal and readable codebase

##⚠️ Limitations (Current Phase)
- Linux only (relies on /proc)
- No historical metrics tracking
- No process-level monitoring yet
- CPU sampling uses short blocking delay
