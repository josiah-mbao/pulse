use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Row, Table, Paragraph, Tabs, Sparkline},
    text::{Span, Line},
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

// Semantic Alert Tokens
pub const COLOR_CRIMSON: Color = Color::Rgb(239, 68, 68);
pub const COLOR_AMBER: Color = Color::Rgb(245, 158, 11);

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

    // Top: Global CPU Heartbeat
    let cpu_data: Vec<u64> = app.global_cpu_history.iter().map(|&v| v as u64).collect();
    let cpu_spark = Sparkline::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_MUTED))
            .title(Span::styled(" Global CPU Heartbeat (0-100%) ", Style::default().fg(TEXT_PRIMARY))))
        .data(&cpu_data)
        .style(Style::default().fg(ACCENT_RUST));
    frame.render_widget(cpu_spark, chunks[0]);

    // Bottom: Disk I/O Velocity
    let io_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[1]);

    let disk_read_data: Vec<u64> = app.disk_read_history.iter().map(|&v| v as u64).collect();
    let disk_read_spark = Sparkline::default()
        .block(Block::default()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
            .border_style(Style::default().fg(BORDER_MUTED))
            .title(Span::styled(" Disk Read Velocity (KiB/s) ", Style::default().fg(TEXT_PRIMARY))))
        .data(&disk_read_data)
        .style(Style::default().fg(TEXT_PRIMARY));
    frame.render_widget(disk_read_spark, io_chunks[0]);

    let disk_write_data: Vec<u64> = app.disk_write_history.iter().map(|&v| v as u64).collect();
    let disk_write_spark = Sparkline::default()
        .block(Block::default()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_style(Style::default().fg(BORDER_MUTED))
            .title(Span::styled(" Disk Write Velocity (KiB/s) ", Style::default().fg(TEXT_PRIMARY))))
        .data(&disk_write_data)
        .style(Style::default().fg(TEXT_PRIMARY));
    frame.render_widget(disk_write_spark, io_chunks[1]);
}

fn render_sentinel_tab(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    render_sentinel_table(frame, app, chunks[0]);
    render_sentinel_stages(frame, app, chunks[1]);
}

fn render_sentinel_table(frame: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_MUTED))
        .title(Span::styled(" Sentinel Network Pipeline ", Style::default().fg(TEXT_PRIMARY)));

    if app.current_speeds.is_empty() {
        let placeholder = Paragraph::new("Sentinel Radar Scanning...")
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
        Row::new(vec!["INTERFACE", "RX RATE", "TX RATE"])
            .style(Style::default().fg(ACCENT_RUST).add_modifier(Modifier::BOLD))
    )
    .block(block);

    frame.render_widget(table, area);
}

fn render_sentinel_stages(frame: &mut Frame, app: &AppState, area: Rect) {
    let mut ifaces: Vec<_> = app.prev_network.interfaces.iter().collect();
    ifaces.sort_by(|a, b| a.0.cmp(b.0));

    if ifaces.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_MUTED))
            .title(Span::styled(" Network Health Matrix ", Style::default().fg(TEXT_PRIMARY)));
        let placeholder = Paragraph::new("Waiting for interface telemetry...")
            .style(Style::default().fg(TEXT_MUTED))
            .block(block);
        frame.render_widget(placeholder, area);
        return;
    }

    // Split area into equal stages for each interface
    let constraints: Vec<_> = ifaces.iter().map(|_| Constraint::Ratio(1, ifaces.len() as u32)).collect();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (i, (name, snap)) in ifaces.into_iter().enumerate() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_MUTED))
            .title(Span::styled(format!(" 󰛳 {} ", name), Style::default().fg(ACCENT_RUST).add_modifier(Modifier::BOLD)));
        
        let inner_area = block.inner(chunks[i]);
        frame.render_widget(block, chunks[i]);

        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Status & Errors
                Constraint::Min(2),    // Throughput Indicators
                Constraint::Length(1), // Cumulative Volume
            ])
            .margin(1)
            .split(inner_area);

        // 1. Status & Errors
        let is_up = snap.operstate == "up" || snap.operstate == "unknown";
        let status_color = if is_up { Color::Green } else { COLOR_CRIMSON };
        let status_line = Line::from(vec![
            Span::styled("● ", Style::default().fg(status_color)),
            Span::styled(snap.operstate.to_uppercase(), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            Span::styled(" | ", Style::default().fg(BORDER_MUTED)),
            Span::styled("RX_ERRORS: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(snap.rx_errors.to_string(), Style::default().fg(if snap.rx_errors > 0 { COLOR_CRIMSON } else { TEXT_PRIMARY })),
        ]);
        frame.render_widget(Paragraph::new(status_line), inner_chunks[0]);

        // 2. Throughput Indicators (Mock Bars)
        let (rx_rate, tx_rate) = app.current_speeds.get(name).cloned().unwrap_or((0.0, 0.0));
        
        let rx_bar_len = ((rx_rate / 1024.0).min(1.0) * (inner_chunks[1].width as f32 - 15.0)) as usize;
        let tx_bar_len = ((tx_rate / 1024.0).min(1.0) * (inner_chunks[1].width as f32 - 15.0)) as usize;

        let rx_line = Line::from(vec![
            Span::styled("RX ", Style::default().fg(TEXT_MUTED)),
            Span::styled(format!("{:>8.1} KiB/s ", rx_rate), Style::default().fg(TEXT_PRIMARY)),
            Span::styled("█".repeat(rx_bar_len), Style::default().fg(ACCENT_RUST)),
        ]);
        let tx_line = Line::from(vec![
            Span::styled("TX ", Style::default().fg(TEXT_MUTED)),
            Span::styled(format!("{:>8.1} KiB/s ", tx_rate), Style::default().fg(TEXT_PRIMARY)),
            Span::styled("█".repeat(tx_bar_len), Style::default().fg(TEXT_MUTED)),
        ]);
        frame.render_widget(Paragraph::new(vec![rx_line, tx_line]), inner_chunks[1]);

        // 3. Cumulative Volume
        let vol_line = Line::from(vec![
            Span::styled("TOTAL IN: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(format_bytes(snap.rx_bytes), Style::default().fg(TEXT_PRIMARY)),
            Span::styled("   TOTAL OUT: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(format_bytes(snap.tx_bytes), Style::default().fg(TEXT_PRIMARY)),
        ]);
        frame.render_widget(Paragraph::new(vol_line), inner_chunks[2]);
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes > 1024 * 1024 * 1024 {
        format!("{:.2} GiB", bytes as f32 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes > 1024 * 1024 {
        format!("{:.2} MiB", bytes as f32 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} KiB", bytes as f32 / 1024.0)
    }
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
        let cpu = *app.cpu_map.get(pid).unwrap_or(&0.0);
        let mem = p.memory_kb;

        let row_style = if cpu > 70.0 || mem > 1_000_000 {
            Style::default().fg(COLOR_CRIMSON)
        } else if cpu > 30.0 || mem > 500_000 {
            Style::default().fg(COLOR_AMBER)
        } else {
            Style::default().fg(TEXT_PRIMARY)
        };

        Some(Row::new(vec![
            pid.to_string(),
            p.name.clone(),
            format!("{:.1}%", cpu),
            format!("{} KB", mem),
        ]).style(row_style))
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
    .row_highlight_style(Style::default().bg(BORDER_MUTED).fg(ACCENT_RUST))
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
    // 1. Check for active transient error message (3s expiry)
    if let Some((msg, timestamp)) = &app.error_message {
        if timestamp.elapsed() < std::time::Duration::from_secs(3) {
            let style = Style::default().fg(COLOR_AMBER).add_modifier(Modifier::BOLD);
            frame.render_widget(Paragraph::new(format!(" {} ", msg)).style(style), area);
            return;
        }
    }

    // 2. Render normal or mode-specific footer
    let (text, style) = match app.input_mode {
        InputMode::Normal => {
            let base_text = if app.paused {
                " [PAUSED] | [1-3] Lenses | / Filter | s/m Sort | j/k Nav | q Quit "
            } else {
                " [1-3] Lenses | / Filter | s/m Sort | j/k Nav | q Quit "
            };
            let style = if app.paused {
                Style::default().bg(Color::Rgb(250, 204, 21)).fg(Color::Black)
            } else {
                Style::default().bg(ACCENT_RUST).fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD)
            };
            (base_text.to_string(), style)
        },
        InputMode::Filter => {
            (format!(" FILTERING: {} ", app.filter_query), Style::default().bg(ACCENT_RUST).fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD))
        },
        InputMode::Confirm => {
            (" ⚠️ CONFIRM: Press [t] SIGTERM (Graceful) | [k] SIGKILL (Force) | [Esc] Cancel ".to_string(), 
             Style::default().bg(COLOR_CRIMSON).fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD))
        },
    };
    
    frame.render_widget(Paragraph::new(text).style(style), area);
}
