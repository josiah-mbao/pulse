use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Row, Table, Gauge, Paragraph, Sparkline},
    Frame,
};
use crate::tui::app::{AppState, SortMode, InputMode};
use pulse::system::memory::{read_memory, memory_usage_percent};
use pulse::system::uptime::read_uptime;
use pulse::system::process::get_extra_info;

pub fn render(frame: &mut Frame, app: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Sparklines
            Constraint::Min(10),   // Main View
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

    // CPU Sparkline
    let cpu_data: Vec<u64> = app.cpu_history.iter().cloned().collect();
    let cpu_spark = Sparkline::default()
        .block(Block::default().title(" CPU Trend ").borders(Borders::ALL))
        .data(&cpu_data)
        .style(Style::default().fg(Color::Green));
    frame.render_widget(cpu_spark, stats_chunks[0]);

    // Memory Sparkline
    let mem_data: Vec<u64> = app.mem_history.iter().cloned().collect();
    let mem_spark = Sparkline::default()
        .block(Block::default().title(" MEM Trend ").borders(Borders::ALL))
        .data(&mem_data)
        .style(Style::default().fg(Color::Magenta));
    frame.render_widget(mem_spark, stats_chunks[1]);
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
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Process Details ")
        .border_style(Style::default().fg(Color::Yellow));
    
    let mut filtered_procs: Vec<_> = app.processes.iter()
        .filter(|(_, p)| p.name.to_lowercase().contains(&app.filter_query.to_lowercase()))
        .collect();

    match app.sort_mode {
        SortMode::Cpu => filtered_procs.sort_by(|(a,_),(b,_)| {
            app.cpu_map.get(b).partial_cmp(&app.cpu_map.get(a)).unwrap_or(std::cmp::Ordering::Equal)
        }),
        SortMode::Memory => filtered_procs.sort_by(|(_,a),(_,b)| b.memory_kb.cmp(&a.memory_kb)),
    }
    
    let mut details_text = String::from("\n No selection");

    if let Some((pid, proc)) = filtered_procs.get(app.selection_index) {
        let cpu = app.cpu_map.get(pid).unwrap_or(&0.0);
        let extra = get_extra_info(**pid).unwrap_or((0, 0, "N/A".to_string()));

        details_text = format!(
            "\n NAME:    {}\n PID:     {}\n PPID:    {}\n STATE:   {}\n THREADS: {}\n\n MEMORY:  {} KB\n CPU:     {:.1}%",
            proc.name, pid, extra.0, extra.2, extra.1, proc.memory_kb, cpu
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
