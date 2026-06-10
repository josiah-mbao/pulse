use crate::tui::app::{AppState, InputMode, Tab};
use pulse::system::memory::{memory_usage_percent, read_memory};
use pulse::system::process::get_extra_info;
use pulse::system::uptime::read_uptime;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Sparkline, Table, Tabs},
};
use std::collections::HashMap;

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

    if app.show_help {
        render_help_modal(frame, area);
    }
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
        .highlight_style(
            Style::default()
                .fg(ACCENT_RUST)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled("|", Style::default().fg(BORDER_MUTED)));

    frame.render_widget(tabs, area);
}

fn render_fleet_tab(frame: &mut Frame, app: &mut AppState, area: Rect) {
    // Evaluation of tree_mode is delegated to render_table for structural column rendering
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    render_table(frame, app, main_chunks[0]);
    render_details(frame, app, main_chunks[1]);
}

fn render_ekg_tab(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Top: Global CPU Heartbeat
    let cpu_data: Vec<u64> = app.global_cpu_history.iter().map(|&v| v as u64).collect();
    let cpu_spark = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_MUTED))
                .title(Span::styled(
                    " Global CPU Heartbeat (0-100%) ",
                    Style::default().fg(TEXT_PRIMARY),
                )),
        )
        .data(&cpu_data)
        .style(Style::default().fg(ACCENT_RUST));
    frame.render_widget(cpu_spark, chunks[0]);

    // Bottom: Disk I/O Velocity
    let io_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let disk_read_data: Vec<u64> = app.disk_read_history.iter().map(|&v| v as u64).collect();
    let disk_read_spark = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
                .border_style(Style::default().fg(BORDER_MUTED))
                .title(Span::styled(
                    " Disk Read Velocity (KiB/s) ",
                    Style::default().fg(TEXT_PRIMARY),
                )),
        )
        .data(&disk_read_data)
        .style(Style::default().fg(TEXT_PRIMARY));
    frame.render_widget(disk_read_spark, io_chunks[0]);

    let disk_write_data: Vec<u64> = app.disk_write_history.iter().map(|&v| v as u64).collect();
    let disk_write_spark = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_style(Style::default().fg(BORDER_MUTED))
                .title(Span::styled(
                    " Disk Write Velocity (KiB/s) ",
                    Style::default().fg(TEXT_PRIMARY),
                )),
        )
        .data(&disk_write_data)
        .style(Style::default().fg(TEXT_PRIMARY));
    frame.render_widget(disk_write_spark, io_chunks[1]);
}

fn render_sentinel_tab(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_sentinel_table(frame, app, chunks[0]);
    render_sentinel_stages(frame, app, chunks[1]);
}

fn render_sentinel_table(frame: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_MUTED))
        .title(Span::styled(
            " Sentinel Network Pipeline ",
            Style::default().fg(TEXT_PRIMARY),
        ));

    if app.current_speeds.is_empty() {
        let placeholder = Paragraph::new("Sentinel Radar Scanning...")
            .style(Style::default().fg(TEXT_MUTED))
            .block(block);
        frame.render_widget(placeholder, area);
        return;
    }

    let mut speeds: Vec<_> = app.current_speeds.iter().collect();
    speeds.sort_by(|a, b| a.0.cmp(b.0));

    let rows: Vec<Row> = speeds
        .iter()
        .map(|(name, (rx, tx))| {
            Row::new(vec![
                format!("󰛳 {}", name),
                format!("{:.2} KiB/s", rx),
                format!("{:.2} KiB/s", tx),
            ])
            .style(Style::default().fg(TEXT_PRIMARY))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ],
    )
    .header(
        Row::new(vec!["INTERFACE", "RX RATE", "TX RATE"]).style(
            Style::default()
                .fg(ACCENT_RUST)
                .add_modifier(Modifier::BOLD),
        ),
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
            .title(Span::styled(
                " Network Health Matrix ",
                Style::default().fg(TEXT_PRIMARY),
            ));
        let placeholder = Paragraph::new("Waiting for interface telemetry...")
            .style(Style::default().fg(TEXT_MUTED))
            .block(block);
        frame.render_widget(placeholder, area);
        return;
    }

    // Split area into equal stages for each interface
    let constraints: Vec<_> = ifaces
        .iter()
        .map(|_| Constraint::Ratio(1, ifaces.len() as u32))
        .collect();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (i, (name, snap)) in ifaces.into_iter().enumerate() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_MUTED))
            .title(Span::styled(
                format!(" 󰛳 {} ", name),
                Style::default()
                    .fg(ACCENT_RUST)
                    .add_modifier(Modifier::BOLD),
            ));

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
            Span::styled(
                snap.operstate.to_uppercase(),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(BORDER_MUTED)),
            Span::styled("RX_ERRORS: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                snap.rx_errors.to_string(),
                Style::default().fg(if snap.rx_errors > 0 {
                    COLOR_CRIMSON
                } else {
                    TEXT_PRIMARY
                }),
            ),
        ]);
        frame.render_widget(Paragraph::new(status_line), inner_chunks[0]);

        // 2. Throughput Indicators (Mock Bars)
        let (rx_rate, tx_rate) = app.current_speeds.get(name).cloned().unwrap_or((0.0, 0.0));

        let rx_bar_len =
            ((rx_rate / 1024.0).min(1.0) * (inner_chunks[1].width as f32 - 15.0)) as usize;
        let tx_bar_len =
            ((tx_rate / 1024.0).min(1.0) * (inner_chunks[1].width as f32 - 15.0)) as usize;

        let rx_line = Line::from(vec![
            Span::styled("RX ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                format!("{:>8.1} KiB/s ", rx_rate),
                Style::default().fg(TEXT_PRIMARY),
            ),
            Span::styled("█".repeat(rx_bar_len), Style::default().fg(ACCENT_RUST)),
        ]);
        let tx_line = Line::from(vec![
            Span::styled("TX ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                format!("{:>8.1} KiB/s ", tx_rate),
                Style::default().fg(TEXT_PRIMARY),
            ),
            Span::styled("█".repeat(tx_bar_len), Style::default().fg(TEXT_MUTED)),
        ]);
        frame.render_widget(Paragraph::new(vec![rx_line, tx_line]), inner_chunks[1]);

        // 3. Cumulative Volume
        let vol_line = Line::from(vec![
            Span::styled("TOTAL IN: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                format_bytes(snap.rx_bytes),
                Style::default().fg(TEXT_PRIMARY),
            ),
            Span::styled("   TOTAL OUT: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                format_bytes(snap.tx_bytes),
                Style::default().fg(TEXT_PRIMARY),
            ),
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
        uptime,
        mem_p,
        total.saturating_sub(avail),
        total
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_MUTED))
        .style(Style::default().fg(TEXT_PRIMARY));
    frame.render_widget(Paragraph::new(stats_text).block(block), area);
}

fn render_table(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let selected_idx = app.table_state.selected();
    let sorted_pids = &app.sorted_pids;
    let process_depths = &app.process_depths;

    let rows: Vec<Row> = sorted_pids
        .iter()
        .enumerate()
        .filter_map(|(idx, pid)| {
            let p = app.processes.get(pid)?;
            let cpu = *app.cpu_map.get(pid).unwrap_or(&0.0);
            let mem = p.memory_kb;

            let is_selected = selected_idx == Some(idx);

            let mut row_style = if cpu > 70.0 || mem > 1_000_000 {
                Style::default().fg(COLOR_CRIMSON)
            } else if cpu > 30.0 || mem > 500_000 {
                Style::default().fg(COLOR_AMBER)
            } else {
                Style::default().fg(TEXT_PRIMARY)
            };

            if is_selected {
                row_style = row_style.fg(ACCENT_RUST);
            }

            let mut name_spans = Vec::new();
            if app.tree_mode {
                let depth = process_depths.get(pid).cloned().unwrap_or(0);
                if depth > 0 {
                    for d in 1..=depth {
                        let prefix_style = if is_selected {
                            Style::default().fg(ACCENT_RUST)
                        } else {
                            Style::default().fg(TEXT_MUTED)
                        };

                        if d == depth {
                            if has_more_siblings(idx, d, sorted_pids, process_depths) {
                                name_spans.push(Span::styled("├─ ", prefix_style));
                            } else {
                                name_spans.push(Span::styled("└─ ", prefix_style));
                            }
                        } else {
                            if has_more_siblings(idx, d, sorted_pids, process_depths) {
                                name_spans.push(Span::styled("│  ", prefix_style));
                            } else {
                                name_spans.push(Span::styled("   ", prefix_style));
                            }
                        }
                    }
                }
            }

            let name_style = if is_selected {
                Style::default().fg(ACCENT_RUST)
            } else {
                Style::default().fg(TEXT_PRIMARY)
            };
            name_spans.push(Span::styled(p.name.clone(), name_style));

            Some(
                Row::new(vec![
                    Cell::from(Span::styled(pid.to_string(), row_style)),
                    Cell::from(Line::from(name_spans)),
                    Cell::from(Span::styled(format!("{:.1}%", cpu), row_style)),
                    Cell::from(Span::styled(format!("{} KB", mem), row_style)),
                ])
                .style(row_style),
            )
        })
        .collect();

    let title = if app.tree_mode {
        " Process Tree "
    } else {
        " Processes "
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(15),
        ],
    )
    .header(Row::new(vec!["PID", "NAME", "CPU%", "MEM"]).style(Style::default().fg(ACCENT_RUST)))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_MUTED))
            .title(Span::styled(title, Style::default().fg(TEXT_PRIMARY))),
    )
    .row_highlight_style(Style::default().bg(BORDER_MUTED).fg(ACCENT_RUST))
    .highlight_symbol(">> ");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn has_more_siblings(
    idx: usize,
    depth: usize,
    sorted_pids: &[u32],
    process_depths: &HashMap<u32, usize>,
) -> bool {
    for pid in sorted_pids.iter().skip(idx + 1) {
        let next_depth = process_depths.get(pid).cloned().unwrap_or(0);
        if next_depth == depth {
            return true;
        }
        if next_depth < depth {
            return false;
        }
    }
    false
}

fn render_details(frame: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_MUTED))
        .title(Span::styled(" Inspect ", Style::default().fg(TEXT_PRIMARY)));

    let content = if let Some(pid) = app.target_pid {
        if let Some(proc) = app.processes.get(&pid) {
            let (ppid, threads, state) =
                get_extra_info(pid).unwrap_or((0, 0, "Unknown".to_string()));

            format!(
                " Name:    {}\n PID:     {}\n PPID:    {}\n State:   {}\n Threads: {}\n\n CPU:     {:.2}%\n Memory:  {} KB",
                proc.name,
                pid,
                ppid,
                state,
                threads,
                app.cpu_map.get(&pid).unwrap_or(&0.0),
                proc.memory_kb
            )
        } else {
            "Process terminated.".to_string()
        }
    } else {
        "Select a process to inspect internals.".to_string()
    };

    frame.render_widget(
        Paragraph::new(content)
            .style(Style::default().fg(TEXT_PRIMARY))
            .block(block),
        area,
    );
}

fn render_footer(frame: &mut Frame, app: &AppState, area: Rect) {
    // 1. Check for active transient error message (3s expiry)
    if let Some((msg, timestamp)) = &app.error_message
        && timestamp.elapsed() < std::time::Duration::from_secs(3)
    {
        let style = Style::default()
            .fg(COLOR_AMBER)
            .add_modifier(Modifier::BOLD);
        frame.render_widget(Paragraph::new(format!(" {} ", msg)).style(style), area);
        return;
    }

    // 2. Render normal or mode-specific footer
    let (text, style) = match app.input_mode {
        InputMode::Normal => {
            let base_text = if app.paused {
                " [PAUSED] | [1-3] Lenses | / Filter | s/m Sort | j/k Nav | t Tree | ? Help | q Quit "
            } else {
                " [1-3] Lenses | / Filter | s/m Sort | j/k Nav | t Tree | ? Help | q Quit "
            };
            let style = if app.paused {
                Style::default()
                    .bg(Color::Rgb(250, 204, 21))
                    .fg(Color::Black)
            } else {
                Style::default()
                    .bg(ACCENT_RUST)
                    .fg(TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            };
            (base_text.to_string(), style)
        }
        InputMode::Filter => (
            format!(" FILTERING: {} ", app.filter_query),
            Style::default()
                .bg(ACCENT_RUST)
                .fg(TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Confirm => (
            " ⚠️ CONFIRM: Press [t] SIGTERM (Graceful) | [k] SIGKILL (Force) | [Esc] Cancel "
                .to_string(),
            Style::default()
                .bg(COLOR_CRIMSON)
                .fg(TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    };

    frame.render_widget(Paragraph::new(text).style(style), area);
}

fn render_help_modal(frame: &mut Frame, area: Rect) {
    let popup_area = center_rect(60, 40, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_MUTED))
        .title(Span::styled(
            " 💡 Pulse Command Configuration Manual ",
            Style::default().fg(TEXT_PRIMARY),
        ));

    let help_rows = vec![
        Row::new(vec![
            Cell::from(Line::from(vec![
                Span::styled("[1, 2, 3]", Style::default().fg(ACCENT_RUST)),
                Span::styled(" Switch Views", Style::default().fg(TEXT_MUTED)),
            ])),
            Cell::from(Line::from(vec![
                Span::styled("[/]", Style::default().fg(ACCENT_RUST)),
                Span::styled(" Filter Processes", Style::default().fg(TEXT_MUTED)),
            ])),
            Cell::from(Line::from(vec![
                Span::styled("[t]", Style::default().fg(ACCENT_RUST)),
                Span::styled(" Toggle Tree/Flat Mode", Style::default().fg(TEXT_MUTED)),
            ])),
        ]),
        Row::new(vec![
            Cell::from(Line::from(vec![
                Span::styled("[s/m]", Style::default().fg(ACCENT_RUST)),
                Span::styled(" Sort CPU/Mem", Style::default().fg(TEXT_MUTED)),
            ])),
            Cell::from(Line::from(vec![
                Span::styled("[k]", Style::default().fg(ACCENT_RUST)),
                Span::styled(" Trigger Kill Dialog", Style::default().fg(TEXT_MUTED)),
            ])),
            Cell::from(Line::from(vec![
                Span::styled("[?]", Style::default().fg(ACCENT_RUST)),
                Span::styled(" Close Help Overlay", Style::default().fg(TEXT_MUTED)),
            ])),
        ]),
        Row::new(vec![
            Cell::from(Line::from(vec![
                Span::styled("[j/k, g/G]", Style::default().fg(ACCENT_RUST)),
                Span::styled(" Navigate", Style::default().fg(TEXT_MUTED)),
            ])),
            Cell::from(Line::from(vec![
                Span::styled("[p]", Style::default().fg(ACCENT_RUST)),
                Span::styled(" Pause Telemetry", Style::default().fg(TEXT_MUTED)),
            ])),
            Cell::from(Line::from(vec![
                Span::styled("[q]", Style::default().fg(ACCENT_RUST)),
                Span::styled(
                    " Terminate Pulse Application",
                    Style::default().fg(TEXT_MUTED),
                ),
            ])),
        ]),
    ];

    let table = Table::new(
        help_rows,
        [
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ],
    )
    .block(block);

    frame.render_widget(table, popup_area);
}

fn center_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
