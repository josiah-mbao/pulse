use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Row, Table, Gauge, Paragraph},
    Frame,
};

use crate::tui::app::{AppState, SortMode};
use pulse::system::memory::{read_memory, memory_usage_percent};
use pulse::system::uptime::read_uptime;

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
            Constraint::Length(3), // Header
            Constraint::Length(3), // System Stats (The row for Gauges)
            Constraint::Min(10),   // Process List
        ])
        .split(area);

    // 1. Header Row
    let uptime = read_uptime();
    let header_text = format!(
        " Pulse | Uptime: {:.2}s | Mode: {} | 'q' to Quit, 'p' to Pause",
        uptime,
        match app.sort_mode {
            SortMode::Cpu => "Sorting by CPU",
            SortMode::Memory => "Sorting by Memory",
        }
    );
    
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
    frame.render_widget(header, chunks[0]);

    // 2. System Stats Row (Split into CPU and Memory)
    let stats_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[1]);

    // --- CPU Gauge ---
    // Calculate total system load by summing all process percentages
    let total_cpu_load: f32 = app.cpu_map.values().sum::<f32>().min(100.0);
    let cpu_gauge = Gauge::default()
        .block(Block::default().title(" System CPU ").borders(Borders::ALL))
        .gauge_style(cpu_style(total_cpu_load))
        .percent(total_cpu_load as u16)
        .label(format!("{:.1}%", total_cpu_load));
    frame.render_widget(cpu_gauge, stats_chunks[0]);

    // --- Memory Gauge ---
    let (total_mem, avail_mem) = read_memory();
    let mem_percent = memory_usage_percent(total_mem, avail_mem);
    let mem_gauge = Gauge::default()
        .block(Block::default().title(" System Memory ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(if mem_percent > 80.0 { Color::Red } else { Color::Magenta }))
        .percent(mem_percent as u16)
        .label(format!("{:.1}% ({}/{})", mem_percent, format_memory(total_mem - avail_mem), format_memory(total_mem)));
    frame.render_widget(mem_gauge, stats_chunks[1]);

    // 3. Process Table
    let mut processes: Vec<_> = app.processes.iter().collect();

    match app.sort_mode {
        SortMode::Cpu => {
            processes.sort_by(|(pid_a, _), (pid_b, _)| {
                let cpu_a = app.cpu_map.get(pid_a).unwrap_or(&0.0);
                let cpu_b = app.cpu_map.get(pid_a).unwrap_or(&0.0);
                cpu_b.partial_cmp(cpu_a).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortMode::Memory => {
            processes.sort_by(|(_, a), (_, b)| b.memory_kb.cmp(&a.memory_kb));
        }
    }

    let rows: Vec<Row> = processes
        .iter()
        .map(|(pid, proc)| {
            let cpu = app.cpu_map.get(pid).unwrap_or(&0.0);
            Row::new(vec![
                pid.to_string(),
                proc.name.clone(),
                format!("{:.2}%", cpu),
                format_memory(proc.memory_kb),
            ])
            .style(cpu_style(*cpu))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Percentage(40),
            Constraint::Length(10),
            Constraint::Length(15),
        ],
    )
    .header(Row::new(vec!["PID", "NAME", "CPU", "MEM"]).style(Style::default().add_modifier(Modifier::BOLD)))
    .block(Block::default().borders(Borders::ALL).title(" Processes "));

    frame.render_widget(table, chunks[2]);
}
