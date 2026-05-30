use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Row, Table, Paragraph, Tabs},
    Frame,
};
use crate::tui::app::{AppState, InputMode, Tab};
use pulse::system::memory::{read_memory, memory_usage_percent};
use pulse::system::uptime::read_uptime;
use pulse::system::process::get_extra_info; // Bring in the extra info

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
    // Rely on the single source of truth for sorting mapping
    let rows: Vec<Row> = app.sorted_pids.iter().filter_map(|pid| {
        let p = app.processes.get(pid)?;
        let cpu = app.cpu_map.get(pid).unwrap_or(&0.0);
        Some(Row::new(vec![
            pid.to_string(),
            p.name.clone(),
            format!("{:.1}%", cpu),
            format!("{} KB", p.memory_kb),
        ]))
    }).collect();

    let table = Table::new(rows, [
        Constraint::Length(8),
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(15),
    ])
    .header(Row::new(vec!["PID", "NAME", "CPU%", "MEM"]).style(Style::default().fg(Color::Yellow)))
    .block(Block::default().borders(Borders::ALL).title(" Processes "))
    .row_highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
    .highlight_symbol(">> "); // Makes selection obvious

    // This is the magic widget upgrade
    frame.render_stateful_widget(table, area, &mut app.table_state); 
}

fn render_details(frame: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Inspect ");
    
    let content = if let Some(pid) = app.target_pid {
        if let Some(proc) = app.processes.get(&pid) {
            // Attempt to grab live metadata from /proc
            let (ppid, threads, state) = get_extra_info(pid).unwrap_or((0, 0, "Unknown".to_string()));
            
            format!(
                " Name:    {}\n PID:     {}\n PPID:    {}\n State:   {}\n Threads: {}\n\n CPU:     {:.2}%\n Memory:  {} KB",
                proc.name, pid, ppid, state, threads, app.cpu_map.get(&pid).unwrap_or(&0.0), proc.memory_kb
            )
        } else {
            "Process terminated.".to_string()
        }
    } else {
        "Select a process to inspect internals.".to_string()
    };

    frame.render_widget(Paragraph::new(content).block(block), area);
}

fn render_footer(frame: &mut Frame, app: &AppState, area: Rect) {
    let text = match app.input_mode {
        InputMode::Normal => {
            if app.paused {
                " [PAUSED] | [1-3] Lenses | / Filter | s/m Sort | j/k Nav | q Quit ".to_string()
            } else {
                " [1-3] Lenses | / Filter | s/m Sort | j/k Nav | q Quit ".to_string()
            }
        },
        InputMode::Filter => format!(" FILTERING: {} ", app.filter_query),
        InputMode::Confirm => " KILL PROCESS? (y/n) ".to_string(),
    };
    
    let style = if app.paused {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default().bg(Color::Cyan).fg(Color::Black)
    };
    
    frame.render_widget(Paragraph::new(text).style(style), area);
}
