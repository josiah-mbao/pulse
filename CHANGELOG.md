# Changelog

All notable changes to **Pulse** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [v0.7.0] - 2026-08-22

### Added
- **Embedded eBPF Bytecode**: Standalone compilation with compile-time embedded eBPF object (`include_bytes!`) and fallback file loading.
- **TUI Observability Lenses**:
  - **Fleet Lens** (`1`): Process navigation, sorting, namespace-aware grouping, filtering, SIGTERM/SIGKILL signal dispatch.
  - **EKG Telemetry Lens** (`2`): System-wide CPU sparklines, memory utilization, disk read/write velocity.
  - **Sentinel Network Lens** (`3`): Interface status, RX/TX throughput, packet/error metrics.
  - **Trace Lens** (`4`): eBPF process execution (`sched_process_exec`) and termination (`sched_process_exit`) event streaming.
- **Testing Infrastructure**:
  - 53 core library unit tests covering CPU/memory arithmetic, `/proc` parsing, view sorting, and event sender backpressure.
  - `/proc` fixture-based test suite (`tests/proc_fixture_tests.rs`).
  - Cross-module system integration test suite (`tests/system_integration_tests.rs`).
  - eBPF binary payload decoding & UTF-8 conversion tests (`tests/ebpf_decoding_tests.rs`).
  - Trace event ingestion pipeline tests (`tests/trace_pipeline_tests.rs`).
  - 10,000+ synthetic event flood stress & stability suite (`tests/stress_stability_tests.rs`).
  - Headless TUI rendering verification suite (`tests/tui_rendering_tests.rs`) using `Ratatui TestBackend`.
- **Criterion Benchmark Suite**: `benches/telemetry_benchmarks.rs` benchmarking `/proc` parsing, state construction, CPU calculation, and `project_view` sorting/filtering.
- **CI/CD & Shipping**:
  - GitHub Actions CI workflow enforcing `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, workspace unit/integration testing, and benchmark compilation.
  - Release workflow building eBPF bytecode, packaging `.tar.gz` release archives, and generating `SHA256SUMS`.
  - Automated shell installer script (`scripts/install.sh`).
