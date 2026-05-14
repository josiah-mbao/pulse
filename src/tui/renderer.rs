use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Row, Table, Paragraph, Tabs},
    Frame,
};
use crate::tui::app::{AppState, SortMode, InputMode, Tab};
use pulse::system::memory::{read_memory, memory_usage_percent};
use pulse::system::uptime::read_uptime;

pub fn render(frame: &mut Frame, app: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab Bar
            Constraint::Length(3), // Top Stats
            Constraint::Min(10),   // Content Area
            Constraint::Length(1), // Footer
        ])
        .split(frame.area());

    render_tabs(frame, app, chunks[0]);
    render_stats(frame, app, chunks[1]);
    
    match app.active_tab {
        Tab::Fleet => render_fleet_tab(frame, app, chunks[2]),
        Tab::Ekg => render_ekg_tab(frame, app, chunks[2]),
        Tab::Sentinel => render_sentinel_tab(frame, app, chunks[2]),
    }

    render_footer(frame, app, chunks[3]);
}

fn render_tabs(frame: &mut Frame, app: &AppState, area: Rect) {
    let titles = vec![" [1] FLEET ", " [2] EKG ", " [3] SENTINEL "];
    let index = match app.active_tab {
        Tab::Fleet => 0,
        Tab::Ekg => 1,
        Tab::Sentinel => 2,
    };

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Pulse Lenses "))
        .select(index)
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .divider("|");

    frame.render_widget(tabs, area);
}

fn render_fleet_tab(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ])
        .split(area);

    render_table(frame, app, main_chunks[0]);
    render_details(frame, app, main_chunks[1]);
}

fn render_ekg_tab(frame: &mut Frame, _app: &mut AppState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" System Heartbeat ");
    let placeholder = Paragraph::new("EKG Monitoring Active... Waiting for biological signal.")
        .style(Style::default().fg(Color::DarkGray))
        .block(block);
    frame.render_widget(placeholder, area);
}

fn render_sentinel_tab(frame: &mut Frame, _app: &mut AppState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Sentinel Perimeter ");
    let placeholder = Paragraph::new("Sentinel Radar Scanning... Monitoring network telemetry.")
        .style(Style::default().fg(Color::DarkGray))
        .block(block);
    frame.render_widget(placeholder, area);
}

fn render_stats(frame: &mut Frame, _app: &AppState, area: Rect) {
    let (total, avail) = read_memory();
    let mem_p = memory_usage_percent(total, avail);
    let uptime = read_uptime();

    let stats_text = format!(
        " UPTIME: {:<10.1}s | MEMORY: {:>3.1}% ({}/{} KB)",
        uptime, mem_p, total.saturating_sub(avail), total
    );
    
    let block = Block::default().borders(Borders::ALL);
    frame.render_widget(Paragraph::new(stats_text).block(block), area);
}

fn render_table(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let mut procs: Vec<_> = app.processes.iter().collect();
    
    if !app.filter_query.is_empty() {
        procs.retain(|(_, p)| p.name.contains(&app.filter_query));
    }

    match app.sort_mode {
        SortMode::Cpu => procs.sort_by(|(a_id, _), (b_id, _)| {
            let a_cpu = app.cpu_map.get(a_id).unwrap_or(&0.0);
            let b_cpu = app.cpu_map.get(b_id).unwrap_or(&0.0);
            b_cpu.partial_cmp(a_cpu).unwrap()
        }),
        SortMode::Memory => procs.sort_by(|(_, a), (_, b)| b.memory_kb.cmp(&a.memory_kb)),
    }

    let rows: Vec<Row> = procs.iter().map(|(pid, p)| {
        let cpu = app.cpu_map.get(pid).unwrap_or(&0.0);
        Row::new(vec![
            pid.to_string(),
            p.name.clone(),
            format!("{:.1}%", cpu),
            format!("{} KB", p.memory_kb),
        ])
    }).collect();

    let table = Table::new(rows, [
        Constraint::Length(8),
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(15),
    ])
    .header(Row::new(vec!["PID", "NAME", "CPU%", "MEM"]).style(Style::default().fg(Color::Yellow)))
    .block(Block::default().borders(Borders::ALL).title(" Processes "))
    .row_highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    frame.render_widget(table, area);
}

fn render_details(frame: &mut Frame, _app: &AppState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Inspect ");
    frame.render_widget(Paragraph::new("Select a process to inspect internals.").block(block), area);
}

fn render_footer(frame: &mut Frame, app: &AppState, area: Rect) {
    // Both arms now return String to ensure type compatibility
    let text = match app.input_mode {
        InputMode::Normal => " [1-3] Lenses | / Filter | s/m Sort | q Quit ".to_string(),
        InputMode::Filter => format!(" FILTERING: {} ", app.filter_query),
        InputMode::Confirm => " KILL PROCESS? (y/n) ".to_string(),
    };
    frame.render_widget(Paragraph::new(text).style(Style::default().bg(Color::Cyan).fg(Color::Black)), area);
}
