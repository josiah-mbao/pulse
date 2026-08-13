use crate::system::model::{ProcessSnapshot, SortMode, ViewMode, ViewRow};
use std::collections::HashMap;

/// Projects process snapshots into a presentation-ready pipeline of view rows.
///
/// This function is pure and stateless, handling filtering, grouping, and sorting
/// based on the provided parameters.
pub fn project_view(
    snapshots: &HashMap<u32, ProcessSnapshot>,
    view_mode: &ViewMode,
    sort_mode: &SortMode,
    filter_query: Option<&str>,
) -> Vec<ViewRow> {
    // 1. Filter snapshots
    let mut filtered: Vec<&ProcessSnapshot> = snapshots
        .values()
        .filter(|p| {
            if let Some(query) = filter_query {
                p.name.contains(query)
            } else {
                true
            }
        })
        .collect();

    match view_mode {
        ViewMode::Flat => {
            // 2. Sort
            sort_snapshots(&mut filtered, sort_mode);

            // 3. Map to ViewRow
            filtered
                .into_iter()
                .map(|p| ViewRow::Process {
                    pid: p.pid,
                    indent_level: 0,
                })
                .collect()
        }
        ViewMode::Container => {
            // 2. Group by container_id
            let mut groups: HashMap<String, Vec<&ProcessSnapshot>> = HashMap::new();
            for p in filtered {
                let cid = p.container_id.clone().unwrap_or_else(|| "host".to_string());
                groups.entry(cid).or_default().push(p);
            }

            // 3. Aggregate and prepare group metadata
            let mut group_metadata: Vec<(String, f32, u64, Vec<&ProcessSnapshot>)> = groups
                .into_iter()
                .map(|(id, mut members)| {
                    let agg_cpu: f32 = members.iter().map(|m| m.cpu_usage_percent).sum::<f32>();
                    let agg_mem: u64 = members.iter().map(|m| m.memory_kb).sum::<u64>();
                    sort_snapshots(&mut members, sort_mode);
                    (id, agg_cpu, agg_mem, members)
                })
                .collect();

            // 4. Sort groups
            group_metadata.sort_by(|a, b| match sort_mode {
                SortMode::Cpu => b.1.total_cmp(&a.1),
                SortMode::Memory => b.2.cmp(&a.2),
            });

            // 5. Flatten
            let mut pipeline = Vec::new();
            for (id, agg_cpu, agg_mem, members) in group_metadata {
                pipeline.push(ViewRow::ContainerHeader {
                    id,
                    aggregated_cpu: agg_cpu,
                    aggregated_mem_kb: agg_mem,
                });
                for p in members {
                    pipeline.push(ViewRow::Process {
                        pid: p.pid,
                        indent_level: 1,
                    });
                }
            }
            pipeline
        }
    }
}

fn sort_snapshots(snapshots: &mut [&ProcessSnapshot], sort_mode: &SortMode) {
    match sort_mode {
        SortMode::Cpu => {
            snapshots.sort_by(|a, b| b.cpu_usage_percent.total_cmp(&a.cpu_usage_percent));
        }
        SortMode::Memory => {
            snapshots.sort_by_key(|b| std::cmp::Reverse(b.memory_kb));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_snapshot(pid: u32, cpu: f32, mem: u64, cid: Option<&str>) -> ProcessSnapshot {
        ProcessSnapshot {
            pid,
            ppid: 1,
            name: format!("proc-{}", pid),
            cpu_usage_percent: cpu,
            memory_kb: mem,
            container_id: cid.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_flat_sort_cpu() {
        let mut snapshots = HashMap::new();
        snapshots.insert(1, mock_snapshot(1, 10.0, 100, None));
        snapshots.insert(2, mock_snapshot(2, 50.0, 200, None));
        snapshots.insert(3, mock_snapshot(3, 25.0, 300, None));

        let pipeline = project_view(&snapshots, &ViewMode::Flat, &SortMode::Cpu, None);

        if let (
            ViewRow::Process { pid: p1, .. },
            ViewRow::Process { pid: p2, .. },
            ViewRow::Process { pid: p3, .. },
        ) = (&pipeline[0], &pipeline[1], &pipeline[2])
        {
            assert_eq!(*p1, 2);
            assert_eq!(*p2, 3);
            assert_eq!(*p3, 1);
        } else {
            panic!("Unexpected row types");
        }
    }

    #[test]
    fn test_container_grouping() {
        let mut snapshots = HashMap::new();
        snapshots.insert(1, mock_snapshot(1, 10.0, 100, Some("c1")));
        snapshots.insert(2, mock_snapshot(2, 50.0, 200, Some("c1")));
        snapshots.insert(3, mock_snapshot(3, 25.0, 300, Some("c2")));
        snapshots.insert(4, mock_snapshot(4, 5.0, 50, None));

        let pipeline = project_view(&snapshots, &ViewMode::Container, &SortMode::Cpu, None);

        // Group c1 total CPU: 60.0
        // Group c2 total CPU: 25.0
        // Group host total CPU: 5.0

        assert!(matches!(
            pipeline[0],
            ViewRow::ContainerHeader {
                ref id,
                aggregated_cpu,
                ..
            } if id == "c1" && (aggregated_cpu - 60.0).abs() < f32::EPSILON
        ));
        assert!(matches!(
            pipeline[1],
            ViewRow::Process {
                pid: 2,
                indent_level: 1
            }
        ));
        assert!(matches!(
            pipeline[2],
            ViewRow::Process {
                pid: 1,
                indent_level: 1
            }
        ));

        assert!(matches!(
            pipeline[3],
            ViewRow::ContainerHeader { ref id, .. } if id == "c2"
        ));
        assert!(matches!(
            pipeline[4],
            ViewRow::Process {
                pid: 3,
                indent_level: 1
            }
        ));

        assert!(matches!(
            pipeline[5],
            ViewRow::ContainerHeader { ref id, .. } if id == "host"
        ));
        assert!(matches!(
            pipeline[6],
            ViewRow::Process {
                pid: 4,
                indent_level: 1
            }
        ));
    }

    #[test]
    fn test_filtering() {
        let mut snapshots = HashMap::new();
        snapshots.insert(1, mock_snapshot(1, 10.0, 100, None));
        let mut p2 = mock_snapshot(2, 20.0, 200, None);
        p2.name = "match-me".to_string();
        snapshots.insert(2, p2);

        let pipeline = project_view(
            &snapshots,
            &ViewMode::Flat,
            &SortMode::Cpu,
            Some("match-me"),
        );

        assert_eq!(pipeline.len(), 1);
        assert!(matches!(pipeline[0], ViewRow::Process { pid: 2, .. }));
    }

    #[test]
    fn test_flat_sort_memory() {
        let mut snapshots = HashMap::new();
        snapshots.insert(1, mock_snapshot(1, 50.0, 100, None)); // high CPU, low RAM
        snapshots.insert(2, mock_snapshot(2, 10.0, 500, None)); // low CPU, high RAM
        snapshots.insert(3, mock_snapshot(3, 20.0, 300, None)); // mid RAM

        let pipeline = project_view(&snapshots, &ViewMode::Flat, &SortMode::Memory, None);

        if let (
            ViewRow::Process { pid: p1, .. },
            ViewRow::Process { pid: p2, .. },
            ViewRow::Process { pid: p3, .. },
        ) = (&pipeline[0], &pipeline[1], &pipeline[2])
        {
            assert_eq!(*p1, 2);
            assert_eq!(*p2, 3);
            assert_eq!(*p3, 1);
        } else {
            panic!("Unexpected row types");
        }
    }

    #[test]
    fn test_container_sort_memory() {
        let mut snapshots = HashMap::new();
        snapshots.insert(1, mock_snapshot(1, 10.0, 100, Some("c1"))); // c1 total mem: 300
        snapshots.insert(2, mock_snapshot(2, 50.0, 200, Some("c1")));
        snapshots.insert(3, mock_snapshot(3, 25.0, 1000, Some("c2"))); // c2 total mem: 1000

        let pipeline = project_view(&snapshots, &ViewMode::Container, &SortMode::Memory, None);

        assert!(matches!(
            pipeline[0],
            ViewRow::ContainerHeader {
                ref id,
                aggregated_mem_kb,
                ..
            } if id == "c2" && aggregated_mem_kb == 1000
        ));
        assert!(matches!(
            pipeline[1],
            ViewRow::Process {
                pid: 3,
                indent_level: 1
            }
        ));

        assert!(matches!(
            pipeline[2],
            ViewRow::ContainerHeader {
                ref id,
                aggregated_mem_kb,
                ..
            } if id == "c1" && aggregated_mem_kb == 300
        ));
        // Within c1, proc 2 (200 KB) comes before proc 1 (100 KB)
        assert!(matches!(
            pipeline[3],
            ViewRow::Process {
                pid: 2,
                indent_level: 1
            }
        ));
        assert!(matches!(
            pipeline[4],
            ViewRow::Process {
                pid: 1,
                indent_level: 1
            }
        ));
    }

    #[test]
    fn test_project_view_empty_snapshots() {
        let snapshots = HashMap::new();

        let flat_pipeline = project_view(&snapshots, &ViewMode::Flat, &SortMode::Cpu, None);
        assert!(flat_pipeline.is_empty());

        let container_pipeline =
            project_view(&snapshots, &ViewMode::Container, &SortMode::Cpu, None);
        assert!(container_pipeline.is_empty());
    }

    #[test]
    fn test_project_view_filter_no_match() {
        let mut snapshots = HashMap::new();
        snapshots.insert(1, mock_snapshot(1, 10.0, 100, None));

        let pipeline = project_view(
            &snapshots,
            &ViewMode::Flat,
            &SortMode::Cpu,
            Some("nonexistent_filter_query"),
        );
        assert!(pipeline.is_empty());
    }
}
