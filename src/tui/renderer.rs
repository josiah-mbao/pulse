use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::tui::app::{AppState, SortMode};

fn format_memory(kb: u64) -> String {
    if kb > 1_000_000 {
        format!("{:.2} GB", kb as f64 / 1_000_000.0)
    } else if kb > 1_000 {
        format!("{:.2} MB", kb as f64 / 1_000.0)
    } else {
        format!("{} KB", kb)
    }
}

fn cpu_style(cpu: f32) -> Style {
    if cpu > 50.0 {
        Style::default().fg(Color::Red)
    } else if cpu > 20.0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    }
}

pub fn render(frame: &mut Frame, app: &AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // header
            Constraint::Percentage(100),
        ])
        .split(area);

    // 🔹 Header
    let header_text = match app.sort_mode {
        SortMode::Cpu => "Sorted by CPU (press 'm' for memory)",
        SortMode::Memory => "Sorted by Memory (press 'c' for CPU)",
    };

    let header = Block::default()
        .title(header_text)
        .borders(Borders::ALL);

    frame.render_widget(header, chunks[0]);

    // 🔹 Sorting
    let mut processes: Vec<_> = app.processes.iter().collect();

    match app.sort_mode {
        SortMode::Cpu => {
            processes.sort_by(|(pid_a, _), (pid_b, _)| {
                let cpu_a = app.cpu_map.get(pid_a).unwrap_or(&0.0);
                let cpu_b = app.cpu_map.get(pid_b).unwrap_or(&0.0);

                cpu_b.partial_cmp(cpu_a).unwrap()
            });
        }
        SortMode::Memory => {
            processes.sort_by(|(_, a), (_, b)| b.memory_kb.cmp(&a.memory_kb));
        }
    }

    // 🔹 Rows
    let rows: Vec<Row> = processes
        .iter()
        .take(20)
        .map(|(pid, proc)| {
            let cpu = app.cpu_map.get(pid).unwrap_or(&0.0);

            Row::new(vec![
                pid.to_string(),
                proc.name.clone(),
                format!("{:.2}", cpu),
                format_memory(proc.memory_kb),
            ])
            .style(cpu_style(*cpu))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Length(20),
            Constraint::Length(8),
            Constraint::Length(12),
        ],
    )
    .header(Row::new(vec!["PID", "NAME", "CPU%", "MEM"]))
    .block(Block::default().borders(Borders::ALL).title("Pulse"));

    frame.render_widget(table, chunks[1]);
}
