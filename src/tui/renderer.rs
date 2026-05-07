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
            Constraint::Min(10),   // Main (Table + Details)
            Constraint::Length(1), // Footer
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_stats(frame, app, chunks[1]);
    
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ])
        .split(chunks[2]);

    render_table(frame, app, main_chunks[0]);
    render_details(frame, app, main_chunks[1]);
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

    let total_cpu_load: f32 = app.cpu_map.values().sum::<f32>().min(100.0);
    let cpu_gauge = Gauge::default()
        .block(Block::default().title(" CPU ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(total_cpu_load as u16);
    frame.render_widget(cpu_gauge, stats_chunks[0]);

    let (total_mem, avail_mem) = read_memory();
    let mem_percent = memory_usage_percent(total_mem, avail_mem);
    let mem_gauge = Gauge::default()
        .block(Block::default().title(" MEM ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Magenta))
        .percent(mem_percent as u16);
    frame.render_widget(mem_gauge, stats_chunks[1]);
}

fn render_table(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let mut processes: Vec<_> = app.processes.iter()
        .filter(|(_, p)| p.name.to_lowercase().contains(&app.filter_query.to_lowercase()))
        .collect();
    
    match app.sort_mode {
        SortMode::Cpu => processes.sort_by(|(a,_),(b,_)| {
            app.cpu_map.get(b).partial_cmp(&app.cpu_map.get(a)).unwrap_or(std::cmp::Ordering::Equal)
        }),
        SortMode::Memory => processes.sort_by(|(_,a),(_,b)| b.memory_kb.cmp(&a.memory_kb)),
    }

    if processes.is_empty() {
        app.selection_index = 0;
    } else {
        app.selection_index = app.selection_index.min(processes.len().saturating_sub(1));
    }

    let visible_rows = (area.height as usize).saturating_sub(3); 
    if app.selection_index >= app.scroll_offset + visible_rows {
        app.scroll_offset = app.selection_index - visible_rows + 1;
    }
    if app.selection_index < app.scroll_offset {
        app.scroll_offset = app.selection_index;
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
            ]).style(style)
        }).collect();

    let table = Table::new(rows, [
        Constraint::Length(8),
        Constraint::Min(10),
        Constraint::Length(8),
    ])
    .header(Row::new(vec!["PID", "NAME", "CPU"]).style(Style::default().fg(Color::Yellow)))
    .block(Block::default().borders(Borders::ALL).title(format!(" Processes ({}) ", processes.len())));

    frame.render_widget(table, area);
}

fn render_details(frame: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Details ");
    
    let mut filtered_procs: Vec<_> = app.processes.iter()
        .filter(|(_, p)| p.name.to_lowercase().contains(&app.filter_query.to_lowercase()))
        .collect();

    // We must sort the details list the same way as the table so the selection index matches
    match app.sort_mode {
        SortMode::Cpu => filtered_procs.sort_by(|(a,_),(b,_)| {
            app.cpu_map.get(b).partial_cmp(&app.cpu_map.get(a)).unwrap_or(std::cmp::Ordering::Equal)
        }),
        SortMode::Memory => filtered_procs.sort_by(|(_,a),(_,b)| b.memory_kb.cmp(&a.memory_kb)),
    }
    
    let mut details_text = String::from("\n No process selected");

    if let Some((pid, proc)) = filtered_procs.get(app.selection_index) {
        details_text = format!(
            "\n Name: {}\n PID: {}\n Memory: {} KB\n CPU: {:.1}%",
            proc.name,
            pid, // Correctly using the key from the tuple
            proc.memory_kb,
            app.cpu_map.get(pid).unwrap_or(&0.0) // Correctly using the key from the tuple
        );
    }
    
    let p = Paragraph::new(details_text).block(block);
    frame.render_widget(p, area);
}

fn render_footer(frame: &mut Frame, app: &AppState, area: Rect) {
    let (text, style) = match app.input_mode {
        InputMode::Normal => (
            format!(" ↑↓ Select | / Filter | s/m Sort | q Quit "),
            Style::default().bg(Color::Cyan).fg(Color::Black)
        ),
        InputMode::Filter => (
            format!(" FILTERING: {}█ (Enter to apply, Esc to clear)", app.filter_query),
            Style::default().bg(Color::Yellow).fg(Color::Black)
        ),
    };
    
    let footer = Paragraph::new(text).style(style);
    frame.render_widget(footer, area);
}
