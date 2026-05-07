use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Row, Table, Gauge, Paragraph},
    Frame,
};
use crate::tui::app::{AppState, SortMode, InputMode};
use pulse::system::memory::{read_memory, memory_usage_percent};
use pulse::system::uptime::read_uptime;

pub fn render(frame: &mut Frame, app: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Stats
            Constraint::Min(10),   // Table
            Constraint::Length(1), // Footer
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_stats(frame, app, chunks[1]);
    render_table(frame, app, chunks[2]);
    render_footer(frame, app, chunks[3]);
}

fn render_header(frame: &mut Frame, _app: &AppState, area: Rect) {
    let uptime = read_uptime();
    let header_text = format!(" Pulse | Uptime: {:.2}s | System Active", uptime);
    
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
    frame.render_widget(header, area);
}

fn render_stats(frame: &mut Frame, app: &AppState, area: Rect) {
    let stats_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // CPU Gauge
    let total_cpu_load: f32 = app.cpu_map.values().sum::<f32>().min(100.0);
    let cpu_gauge = Gauge::default()
        .block(Block::default().title(" CPU ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(total_cpu_load as u16);
    frame.render_widget(cpu_gauge, stats_chunks[0]);

    // Memory Gauge
    let (total_mem, avail_mem) = read_memory();
    let mem_percent = memory_usage_percent(total_mem, avail_mem);
    let mem_gauge = Gauge::default()
        .block(Block::default().title(" MEM ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Magenta))
        .percent(mem_percent as u16);
    frame.render_widget(mem_gauge, stats_chunks[1]);
}

fn render_table(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let mut processes: Vec<_> = app.processes.iter().collect();
    
    match app.sort_mode {
        SortMode::Cpu => processes.sort_by(|(a,_),(b,_)| {
            app.cpu_map.get(b).partial_cmp(&app.cpu_map.get(a)).unwrap_or(std::cmp::Ordering::Equal)
        }),
        SortMode::Memory => processes.sort_by(|(_,a),(_,b)| b.memory_kb.cmp(&a.memory_kb)),
    }

    let visible_rows = (area.height as usize).saturating_sub(3); 
    if app.selection_index >= app.scroll_offset + visible_rows {
        app.scroll_offset = app.selection_index - visible_rows + 1;
    }

    let rows: Vec<Row> = processes
        .iter()
        .enumerate()
        .skip(app.scroll_offset)
        .take(visible_rows)
        .map(|(i, (pid, proc))| {
            let cpu = app.cpu_map.get(pid).unwrap_or(&0.0);
            let style = if i == app.selection_index {
                Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                pid.to_string(),
                proc.name.clone(),
                format!("{:.1}%", cpu),
                format!("{} KB", proc.memory_kb),
            ]).style(style)
        }).collect();

    let table = Table::new(rows, [
        Constraint::Length(8),
        Constraint::Percentage(40),
        Constraint::Length(10),
        Constraint::Length(15),
    ])
    .header(Row::new(vec!["PID", "NAME", "CPU", "MEM"]).style(Style::default().fg(Color::Yellow)))
    .block(Block::default().borders(Borders::ALL).title(" Processes "));

    frame.render_widget(table, area);
}

fn render_footer(frame: &mut Frame, app: &AppState, area: Rect) {
    let text = match app.input_mode {
        InputMode::Normal => " ↑↓ Select | s/m: Sort | p: Pause | q: Quit ",
        InputMode::Filter => " TYPE TO FILTER... | <Enter> Accept | <Esc> Cancel ",
    };
    let footer = Paragraph::new(text).style(Style::default().bg(Color::Cyan).fg(Color::Black));
    frame.render_widget(footer, area);
}
