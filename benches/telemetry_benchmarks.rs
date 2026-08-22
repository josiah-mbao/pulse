use criterion::{Criterion, criterion_group, criterion_main};
use pulse::system::collector::{RawProcess, parse_stat_content};
use pulse::system::model::{SortMode, ViewMode};
use pulse::system::state::{ProcessSnapshot, build_state, compute_cpu};
use pulse::tui::projection::project_view;
use std::collections::HashMap;

fn bench_proc_parsing(c: &mut Criterion) {
    let line = "1234 (pulse-worker) S 1 1234 1234 0 -1 4194304 1000 0 0 0 400 100 0 0 20 0 1 0 12345 16384000 2048 18446744073709551615";

    c.bench_function("parse_stat_content_standard", |b| {
        b.iter(|| parse_stat_content(line, 1234, 4))
    });
}

fn bench_state_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_state");

    for size in [10, 100, 1000] {
        let raw_procs: Vec<RawProcess> = (0..size)
            .map(|i| RawProcess {
                pid: i as u32 + 1,
                ppid: 1,
                name: format!("process-{i}"),
                cpu_time: 500 + i as u64,
                memory_kb: 4096 + i as u64,
            })
            .collect();

        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &raw_procs,
            |b, procs| b.iter(|| build_state(HashMap::new(), procs.clone(), 1000)),
        );
    }
    group.finish();
}

fn bench_compute_cpu(c: &mut Criterion) {
    let mut group = c.benchmark_group("compute_cpu");

    for size in [10, 100, 1000] {
        let mut prev = HashMap::new();
        let mut curr = HashMap::new();

        for i in 0..size {
            let pid = i as u32 + 1;
            prev.insert(
                pid,
                ProcessSnapshot {
                    ppid: 1,
                    name: format!("process-{i}"),
                    cpu_time: 100,
                    memory_kb: 4096,
                },
            );
            curr.insert(
                pid,
                ProcessSnapshot {
                    ppid: 1,
                    name: format!("process-{i}"),
                    cpu_time: 150 + i as u64,
                    memory_kb: 4096,
                },
            );
        }

        let state = pulse::system::state::SystemState {
            prev,
            curr,
            total_cpu_delta: 1000,
        };

        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &state,
            |b, s| b.iter(|| compute_cpu(s)),
        );
    }
    group.finish();
}

fn bench_projection_engine(c: &mut Criterion) {
    let mut group = c.benchmark_group("project_view");

    for size in [10, 100, 1000] {
        let mut snapshots = HashMap::new();

        for i in 0..size {
            let pid = i as u32 + 1;
            snapshots.insert(
                pid,
                pulse::system::model::ProcessSnapshot {
                    pid,
                    ppid: if pid > 5 { 1 } else { 0 },
                    name: format!("service-{i}"),
                    cpu_usage_percent: (i % 100) as f32,
                    memory_kb: (i as u64) * 1024,
                    container_id: None,
                },
            );
        }

        group.bench_with_input(
            criterion::BenchmarkId::new("Flat_SortCpu", size),
            &snapshots,
            |b, snaps| b.iter(|| project_view(snaps, &ViewMode::Flat, &SortMode::Cpu, None)),
        );

        group.bench_with_input(
            criterion::BenchmarkId::new("Container_SortMemory", size),
            &snapshots,
            |b, snaps| {
                b.iter(|| project_view(snaps, &ViewMode::Container, &SortMode::Memory, None))
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_proc_parsing,
    bench_state_construction,
    bench_compute_cpu,
    bench_projection_engine
);
criterion_main!(benches);
