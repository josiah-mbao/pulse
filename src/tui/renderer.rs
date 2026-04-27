use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::tui::app::AppState;

pub fn render(frame: &mut Frame, app: &AppState) {
    let size = frame.size()?;

    let chunks = layout::default()
        .direction(Direction::Vertical)
        .constraints([Constrain::Percentage(100)])
        .split(size);

    let mut rows = Vec::new();

    for (pid, proc) in &app.processes {
        let cpu = app.cpu_map.get(pid).unwrap_or(&0.0);

        rows.push(Row::new(vec![
                pid.to_string(),
                proc.name.clone(),
                format!("{:.2}", cpu),
                proc.memory_kb.to_string(),
        ]));
    }

    let table = Table::new(rows)
        .header(Row::new(vec!["PID", "NAME", "CPU%", "MEM"]))
        .block(Block::default().borders(Borders::ALL).title("Pulse"))
        .widths(&[
            Constraint::Length(6),
            Constraint::Length(20),
            Constraint::Length(8),
            Constraint::Length(10),
        ]);

    frame.render_widget(table, chunks[0]);
}
