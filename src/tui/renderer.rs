use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Row, Table, Paragraph, Tabs, Sparkline},
    text::Span,
    Frame,
};
use crate::tui::app::{AppState, InputMode, Tab};
use pulse::system::memory::{read_memory, memory_usage_percent};
use pulse::system::uptime::read_uptime;
use pulse::system::process::get_extra_info;

// Zinc & Rust UI Tokens
pub const BG_CANVAS: Color = Color::Rgb(9, 9, 11);
pub const BORDER_MUTED: Color = Color::Rgb(39, 39, 42);
pub const TEXT_PRIMARY: Color = Color::Rgb(250, 250, 250);
pub const TEXT_MUTED: Color = Color::Rgb(161, 161, 170);
pub const ACCENT_RUST: Color = Color::Rgb(244, 102, 35);

pub fn render(frame: &mut Frame, app: &mut AppState) {
    let area = frame.area();
    
    // Root background filling
    frame.render_widget(Block::default().style(Style::default().bg(BG_CANVAS)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab Bar
            Constraint::Length(3), // Top Stats
            Constraint::Min(10),   // Content Area
            Constraint::Length(1), // Footer
        ])
        .split(area);

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
    let titles = vec![" FLEET ", " EKG ", " SENTINEL "];
    let index = match app.active_tab {
        Tab::Fleet => 0,
        Tab::Ekg => 1,
        Tab::Sentinel => 2,
    };

    let tabs = Tabs::new(titles)
        .select(index)
        .style(Style::default().fg(TEXT_MUTED))
        .highlight_style(Style::default().fg(ACCENT_RUST).add_modifier(Modifier::BOLD))
        .divider(Span::styled("|", Style::default().fg(BORDER_MUTED)));

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

fn render_ekg_tab(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Dynamic data casting from f32 window points to graphable u64 slices
    let cpu_data: Vec<u64> = app.global_cpu_history.iter().map(|&v| v as u64).collect();
    let cpu_spark = Sparkline::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_MUTED))
            .title(Span::styled(" Global CPU Heartbeat (0-100%) ", Style::default().fg(TEXT_PRIMARY))))
        .data(&cpu_data)
        .style(Style::default().fg(ACCENT_RUST));
    frame.render_widget(cpu_spark, chunks[0]);

    let mem_data: Vec<u64> = app.global_mem_history.iter().map(|&v| v as u64).collect();
    let mem_spark = Sparkline::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_MUTED))
            .title(Span::styled(" Memory Load History (0-100%) ", Style::default().fg(TEXT_PRIMARY))))
        .data(&mem_data)
        .style(Style::default().fg(Color::Magenta));
    frame.render_widget(mem_spark, chunks[1]);
}

fn render_sentinel_tab(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_MUTED))
        .title(Span::styled(" Sentinel Interface Telemetry Pipeline ", Style::default().fg(TEXT_PRIMARY)));

    if app.current_speeds.is_empty() {
        let placeholder = Paragraph::new("Sentinel Radar Scanning... No active network interfaces detected.")
            .style(Style::default().fg(TEXT_MUTED))
            .block(block);
        frame.render_widget(placeholder, area);
        return;
    }

    let mut speeds: Vec<_> = app.current_speeds.iter().collect();
    speeds.sort_by(|a, b| a.0.cmp(b.0));

    let rows: Vec<Row> = speeds.iter().map(|(name, (rx, tx))| {
        Row::new(vec![
            format!("󰛳 {}", name),
            format!("{:.2} KiB/s", rx),
            format!("{:.2} KiB/s", tx),
        ]).style(Style::default().fg(TEXT_PRIMARY))
    }).collect();

    let table = Table::new(rows, [
        Constraint::Percentage(40),
        Constraint::Percentage(30),
        Constraint::Percentage(30),
    ])
    .header(
        Row::new(vec!["INTERFACE", "RX INCOMING RATE", "TX OUTGOING RATE"])
            .style(Style::default().fg(ACCENT_RUST).add_modifier(Modifier::BOLD))
    )
    .block(block);

    frame.render_widget(table, area);
}

fn render_stats(frame: &mut Frame, _app: &AppState, area: Rect) {
    let (total, avail) = read_memory();
    let mem_p = memory_usage_percent(total, avail);
    let uptime = read_uptime();

    let stats_text = format!(
        " UPTIME: {:<10.1}s | MEMORY: {:>3.1}% ({}/{} KB)",
        uptime, mem_p, total.saturating_sub(avail), total
    );
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_MUTED))
        .style(Style::default().fg(TEXT_PRIMARY));
    frame.render_widget(Paragraph::new(stats_text).block(block), area);
}

fn render_table(frame: &mut Frame, app: &mut AppState, area: Rect) {
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
    .header(Row::new(vec!["PID", "NAME", "CPU%", "MEM"]).style(Style::default().fg(ACCENT_RUST)))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_MUTED))
        .title(Span::styled(" Processes ", Style::default().fg(TEXT_PRIMARY))))
    .row_highlight_style(Style::default().bg(BORDER_MUTED).fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD))
    .highlight_symbol(">> ");

    frame.render_stateful_widget(table, area, &mut app.table_state); 
}

fn render_details(frame: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_MUTED))
        .title(Span::styled(" Inspect ", Style::default().fg(TEXT_PRIMARY)));
    
    let content = if let Some(pid) = app.target_pid {
        if let Some(proc) = app.processes.get(&pid) {
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

    frame.render_widget(Paragraph::new(content).style(Style::default().fg(TEXT_PRIMARY)).block(block), area);
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
        Style::default().bg(Color::Rgb(250, 204, 21)).fg(Color::Black)
    } else {
        Style::default().bg(ACCENT_RUST).fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD)
    };
    
    frame.render_widget(Paragraph::new(text).style(style), area);
}
