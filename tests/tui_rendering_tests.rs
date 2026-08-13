use pulse::system::model::ProcessSnapshot as TuiProcessSnapshot;
use pulse::tui::app::{AppState, InputMode, Tab};
use pulse::tui::renderer::render;
use pulse_common::{EVENT_EXEC, TraceEvent};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::collections::HashMap;

#[test]
fn test_tui_render_normal_fleet_tab() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("Terminal creation");

    let mut app = AppState::new();

    let mut snapshots = HashMap::new();
    snapshots.insert(
        100,
        TuiProcessSnapshot {
            pid: 100,
            ppid: 1,
            name: "render-target".to_string(),
            cpu_usage_percent: 25.5,
            memory_kb: 16384,
            container_id: None,
        },
    );
    app.snapshots = snapshots;
    app.refresh_pipeline();

    terminal
        .draw(|f| render(f, &mut app))
        .expect("Terminal draw");

    let buffer = terminal.backend().buffer();
    let content = format!("{:?}", buffer);

    // Verify key UI text elements rendered without error
    assert!(content.contains("PID") || content.contains("NAME") || content.contains("Fleet"));
}

#[test]
fn test_tui_render_all_tabs_and_overlays() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("Terminal creation");

    let mut app = AppState::new();

    // Add trace event for Trace tab rendering
    let evt = TraceEvent {
        pid: 555,
        event_type: EVENT_EXEC,
        comm: *b"trace-render\0\0\0\0",
    };
    app.apply_trace(evt);

    // Test Tab::Fleet
    app.active_tab = Tab::Fleet;
    terminal
        .draw(|f| render(f, &mut app))
        .expect("Fleet tab draw");

    // Test Tab::Ekg
    app.active_tab = Tab::Ekg;
    terminal
        .draw(|f| render(f, &mut app))
        .expect("Ekg tab draw");

    // Test Tab::Sentinel
    app.active_tab = Tab::Sentinel;
    terminal
        .draw(|f| render(f, &mut app))
        .expect("Sentinel tab draw");

    // Test Tab::Trace
    app.active_tab = Tab::Trace;
    terminal
        .draw(|f| render(f, &mut app))
        .expect("Trace tab draw");

    // Test Help Overlay
    app.show_help = true;
    terminal.draw(|f| render(f, &mut app)).expect("Help draw");

    // Test Filter Mode Overlay
    app.show_help = false;
    app.input_mode = InputMode::Filter;
    app.filter_query = "render".to_string();
    terminal
        .draw(|f| render(f, &mut app))
        .expect("Filter mode draw");
}

#[test]
fn test_tui_render_terminal_resize_edge_cases() {
    // 1. Small terminal (10x5)
    let small_backend = TestBackend::new(10, 5);
    let mut small_term = Terminal::new(small_backend).expect("Small terminal creation");
    let mut app = AppState::new();

    small_term
        .draw(|f| render(f, &mut app))
        .expect("Small terminal draw");

    // 2. Empty terminal (0x0)
    let zero_backend = TestBackend::new(0, 0);
    let mut zero_term = Terminal::new(zero_backend).expect("Zero terminal creation");

    zero_term
        .draw(|f| render(f, &mut app))
        .expect("Zero terminal draw");
}
