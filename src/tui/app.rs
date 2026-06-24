use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use ratatui::{
    Terminal, backend::CrosstermBackend, layout::Rect, style::Color, widgets::TableState,
};
use std::{
    collections::HashMap,
    collections::VecDeque,
    io,
    sync::Arc,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
use tachyonfx::{EffectManager, Interpolation, fx};

use crate::tui::input::{InputEvent, read_input};
use crate::tui::projection::project_view;
use crate::tui::renderer::{BG_CANVAS, render};
use pulse::system::engine::Engine;
use pulse::system::model::{
    NetworkStats, ProcessSnapshot as TuiProcessSnapshot, SortMode, SystemEvent, TelemetryFrame,
    ViewMode, ViewRow,
};
use pulse_common::TraceEvent;

const MAX_HISTORY_POINTS: usize = 200;
const TRACE_BUFFER_CAPACITY: usize = 500;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tab {
    Fleet,
    Ekg,
    Sentinel,
    Trace,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TraceEventKind {
    Exec,
    Exit,
}

pub struct TraceEventView {
    pub pid: u32,
    pub kind: TraceEventKind,
    pub comm: String,
}

impl From<TraceEvent> for TraceEventView {
    fn from(event: TraceEvent) -> Self {
        let kind = if event.event_type == pulse_common::EVENT_EXEC {
            TraceEventKind::Exec
        } else {
            TraceEventKind::Exit
        };

        let comm = String::from_utf8_lossy(&event.comm)
            .trim_end_matches('\0')
            .trim()
            .to_string();

        Self {
            pid: event.pid,
            kind,
            comm,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Filter,
    Confirm,
}

pub struct AppState {
    pub snapshots: HashMap<u32, TuiProcessSnapshot>,
    pub view_pipeline: Vec<ViewRow>,
    pub view_mode: ViewMode,
    pub sort_mode: SortMode,
    pub table_state: TableState,
    pub active_tab: Tab,
    pub input_mode: InputMode,
    pub paused: bool,
    pub show_help: bool,
    pub filter_query: String,
    pub target_pid: Option<u32>,
    pub error_message: Option<(String, Instant)>,
    pub fx: EffectManager<String>,
    pub last_tick: Instant,

    // Time-series buffers for global telemetry
    pub global_cpu_history: VecDeque<f32>,
    pub global_mem_history: VecDeque<f32>,
    pub disk_read_history: VecDeque<f32>,
    pub disk_write_history: VecDeque<f32>,

    // Diff counters
    pub prev_network: NetworkStats,
    pub prev_disk_read: u64,
    pub prev_disk_write: u64,
    pub current_speeds: HashMap<String, (f32, f32)>,
    pub sorted_interfaces: Vec<String>,
    pub trace_log: VecDeque<TraceEventView>,
}

impl AppState {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            snapshots: HashMap::new(),
            view_pipeline: Vec::new(),
            view_mode: ViewMode::default(),
            sort_mode: SortMode::default(),
            table_state,
            active_tab: Tab::Fleet,
            input_mode: InputMode::Normal,
            paused: false,
            show_help: false,
            filter_query: String::new(),
            target_pid: None,
            error_message: None,
            fx: EffectManager::default(),
            last_tick: Instant::now(),
            global_cpu_history: VecDeque::with_capacity(MAX_HISTORY_POINTS),
            global_mem_history: VecDeque::with_capacity(MAX_HISTORY_POINTS),
            disk_read_history: VecDeque::with_capacity(MAX_HISTORY_POINTS),
            disk_write_history: VecDeque::with_capacity(MAX_HISTORY_POINTS),
            prev_network: NetworkStats::default(),
            prev_disk_read: 0,
            prev_disk_write: 0,
            current_speeds: HashMap::new(),
            sorted_interfaces: Vec::new(),
            trace_log: VecDeque::with_capacity(TRACE_BUFFER_CAPACITY),
        }
    }

    pub fn apply_tick(&mut self, frame: TelemetryFrame) {
        if self.paused {
            return;
        }

        // Map TelemetryFrame to model::ProcessSnapshot
        self.snapshots.clear();
        for (pid, proc) in frame.processes {
            let cpu = frame.cpu_map.get(&pid).copied().unwrap_or(0.0);
            self.snapshots.insert(
                pid,
                TuiProcessSnapshot {
                    pid,
                    ppid: proc.ppid,
                    name: proc.name,
                    cpu_usage_percent: cpu,
                    memory_kb: proc.memory_kb,
                    container_id: None,
                },
            );
        }

        // Shift time-series rolling history constraints
        if self.global_cpu_history.len() >= MAX_HISTORY_POINTS {
            self.global_cpu_history.pop_front();
        }
        self.global_cpu_history
            .push_back(frame.global_cpu_utilization);

        if self.global_mem_history.len() >= MAX_HISTORY_POINTS {
            self.global_mem_history.pop_front();
        }
        self.global_mem_history
            .push_back(frame.global_mem_utilization);

        if self.prev_disk_read > 0 || self.prev_disk_write > 0 {
            let r_delta = frame.disk_sectors_read.saturating_sub(self.prev_disk_read) as f32;
            let w_delta = frame
                .disk_sectors_written
                .saturating_sub(self.prev_disk_write) as f32;

            if self.disk_read_history.len() >= MAX_HISTORY_POINTS {
                self.disk_read_history.pop_front();
            }
            self.disk_read_history.push_back(r_delta);

            if self.disk_write_history.len() >= MAX_HISTORY_POINTS {
                self.disk_write_history.pop_front();
            }
            self.disk_write_history.push_back(w_delta);
        }
        self.prev_disk_read = frame.disk_sectors_read;
        self.prev_disk_write = frame.disk_sectors_written;

        self.current_speeds.clear();
        self.sorted_interfaces.clear();
        for (name, curr) in &frame.network.interfaces {
            if let Some(prev) = self.prev_network.interfaces.get(name) {
                let rx_delta = curr.rx_bytes.saturating_sub(prev.rx_bytes);
                let tx_delta = curr.tx_bytes.saturating_sub(prev.tx_bytes);

                let rx_kib = (rx_delta as f32 * 2.0) / 1024.0;
                let tx_kib = (tx_delta as f32 * 2.0) / 1024.0;

                self.current_speeds.insert(name.clone(), (rx_kib, tx_kib));
                self.sorted_interfaces.push(name.clone());
            }
        }
        self.sorted_interfaces.sort();
        self.prev_network = frame.network;

        self.refresh_pipeline();
    }

    pub fn apply_trace(&mut self, event: TraceEvent) {
        if self.paused {
            return;
        }

        if self.trace_log.len() >= TRACE_BUFFER_CAPACITY {
            self.trace_log.pop_front();
        }
        self.trace_log.push_back(TraceEventView::from(event));
    }

    pub fn trace_view(&self) -> impl Iterator<Item = &TraceEventView> {
        self.trace_log.iter()
    }

    pub fn refresh_pipeline(&mut self) {
        self.view_pipeline = project_view(
            &self.snapshots,
            &self.view_mode,
            &self.sort_mode,
            if self.filter_query.is_empty() {
                None
            } else {
                Some(&self.filter_query)
            },
        );

        if let Some(selected) = self.table_state.selected() {
            if self.view_pipeline.is_empty() {
                self.table_state.select(None);
            } else if selected >= self.view_pipeline.len() {
                self.table_state
                    .select(Some(self.view_pipeline.len().saturating_sub(1)));
            }
        } else if !self.view_pipeline.is_empty() {
            self.table_state.select(Some(0));
        }
    }
}

pub fn run_app() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();
    let engine = Engine::new();
    let shutdown = Arc::new(AtomicBool::new(false));
    let rx = engine.spawn_collectors(Arc::clone(&shutdown));

    loop {
        let now = Instant::now();
        let dt = now.duration_since(app.last_tick);
        app.last_tick = now;

        while let Ok(event) = rx.try_recv() {
            match event {
                SystemEvent::Tick(frame) => app.apply_tick(frame),
                SystemEvent::Trace(event) => app.apply_trace(event),
            }
        }

        let timeout = if app.fx.is_running() {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(100)
        };

        match read_input(timeout) {
            InputEvent::Quit => break,
            InputEvent::SwitchTab(tab) => {
                if app.active_tab != tab {
                    app.active_tab = tab;
                    if let Ok(size) = terminal.size() {
                        let main_area = Rect::new(0, 6, size.width, size.height.saturating_sub(7));
                        app.fx.add_effect(
                            fx::fade_from(BG_CANVAS, BG_CANVAS, (150, Interpolation::QuadOut))
                                .with_area(main_area),
                        );
                    }
                }
            }
            InputEvent::EnterFilter => {
                if !app.show_help {
                    app.input_mode = InputMode::Filter;
                }
            }
            InputEvent::InitiateKill => {
                if !app.show_help {
                    if app.input_mode == InputMode::Normal && app.target_pid.is_some() {
                        app.input_mode = InputMode::Confirm;
                    } else if app.input_mode == InputMode::Confirm
                        && let Some(target_pid) = app.target_pid
                    {
                        match kill(Pid::from_raw(target_pid as i32), Signal::SIGKILL) {
                            Ok(_) => {
                                app.input_mode = InputMode::Normal;
                                app.fx.add_effect(fx::fade_from(
                                    Color::White,
                                    Color::White,
                                    (200, Interpolation::Linear),
                                ));
                            }
                            Err(e) => {
                                let msg = match e {
                                    nix::errno::Errno::EPERM => {
                                        "Error: Permission Denied (Run as root)"
                                    }
                                    nix::errno::Errno::ESRCH => "Error: Process no longer exists",
                                    _ => "Error: Signal failed",
                                };
                                app.error_message = Some((msg.to_string(), Instant::now()));
                                app.input_mode = InputMode::Normal;
                            }
                        }
                    }
                }
            }
            InputEvent::Esc => {
                if app.show_help {
                    app.show_help = false;
                } else {
                    app.input_mode = InputMode::Normal;
                    app.filter_query.clear();
                    app.refresh_pipeline();
                }
            }
            InputEvent::Char(c) => match app.input_mode {
                InputMode::Normal => {
                    if c == '4' && app.active_tab != Tab::Trace {
                        app.active_tab = Tab::Trace;
                        if let Ok(size) = terminal.size() {
                            let main_area =
                                Rect::new(0, 6, size.width, size.height.saturating_sub(7));
                            app.fx.add_effect(
                                fx::fade_from(BG_CANVAS, BG_CANVAS, (150, Interpolation::QuadOut))
                                    .with_area(main_area),
                            );
                        }
                    }
                }
                InputMode::Filter => {
                    app.filter_query.push(c);
                    app.refresh_pipeline();
                    app.table_state.select(Some(0));
                }
                InputMode::Confirm => match c {
                    't' | 'T' => {
                        if let Some(target_pid) = app.target_pid {
                            match kill(Pid::from_raw(target_pid as i32), Signal::SIGTERM) {
                                Ok(_) => {
                                    app.input_mode = InputMode::Normal;
                                    app.fx.add_effect(fx::fade_from(
                                        Color::White,
                                        Color::White,
                                        (200, Interpolation::Linear),
                                    ));
                                }
                                Err(e) => {
                                    let msg = match e {
                                        nix::errno::Errno::EPERM => {
                                            "Error: Permission Denied (Run as root)"
                                        }
                                        nix::errno::Errno::ESRCH => {
                                            "Error: Process no longer exists"
                                        }
                                        _ => "Error: Signal failed",
                                    };
                                    app.error_message = Some((msg.to_string(), Instant::now()));
                                    app.input_mode = InputMode::Normal;
                                }
                            }
                        }
                    }
                    'k' | 'K' => {
                        if let Some(target_pid) = app.target_pid {
                            match kill(Pid::from_raw(target_pid as i32), Signal::SIGKILL) {
                                Ok(_) => {
                                    app.input_mode = InputMode::Normal;
                                    app.fx.add_effect(fx::fade_from(
                                        Color::White,
                                        Color::White,
                                        (200, Interpolation::Linear),
                                    ));
                                }
                                Err(e) => {
                                    let msg = match e {
                                        nix::errno::Errno::EPERM => {
                                            "Error: Permission Denied (Run as root)"
                                        }
                                        nix::errno::Errno::ESRCH => {
                                            "Error: Process no longer exists"
                                        }
                                        _ => "Error: Signal failed",
                                    };
                                    app.error_message = Some((msg.to_string(), Instant::now()));
                                    app.input_mode = InputMode::Normal;
                                }
                            }
                        }
                    }
                    'n' | 'N' => {
                        app.input_mode = InputMode::Normal;
                    }
                    _ => {}
                },
            },
            InputEvent::Backspace => {
                if app.input_mode == InputMode::Filter {
                    app.filter_query.pop();
                    app.refresh_pipeline();
                }
            }
            InputEvent::Up => {
                if app.input_mode == InputMode::Normal && !app.show_help {
                    let i = match app.table_state.selected() {
                        Some(i) => i.saturating_sub(1),
                        None => 0,
                    };
                    app.table_state.select(Some(i));
                }
            }
            InputEvent::Down => {
                if app.input_mode == InputMode::Normal && !app.show_help {
                    let i = match app.table_state.selected() {
                        Some(i) => {
                            if i >= app.view_pipeline.len().saturating_sub(1) {
                                i
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    app.table_state.select(Some(i));
                }
            }
            InputEvent::Top => {
                if app.input_mode == InputMode::Normal && !app.show_help {
                    app.table_state.select(Some(0));
                }
            }
            InputEvent::Bottom => {
                if app.input_mode == InputMode::Normal && !app.show_help {
                    let max = app.view_pipeline.len().saturating_sub(1);
                    app.table_state.select(Some(max));
                }
            }
            InputEvent::SortCpu => {
                if !app.show_help {
                    app.sort_mode = SortMode::Cpu;
                    app.refresh_pipeline();
                }
            }
            InputEvent::SortMemory => {
                if !app.show_help {
                    app.sort_mode = SortMode::Memory;
                    app.refresh_pipeline();
                }
            }
            InputEvent::TogglePause if !app.show_help => {
                app.paused = !app.paused;
            }
            InputEvent::ToggleTree if app.input_mode == InputMode::Normal && !app.show_help => {
                app.view_mode = match app.view_mode {
                    ViewMode::Flat => ViewMode::Container,
                    ViewMode::Container => ViewMode::Flat,
                };
                app.refresh_pipeline();
            }
            InputEvent::ToggleHelp if app.input_mode == InputMode::Normal => {
                app.show_help = !app.show_help;
            }
            _ => {}
        }

        if let Some(idx) = app.table_state.selected() {
            if let Some(ViewRow::Process { pid, .. }) = app.view_pipeline.get(idx) {
                app.target_pid = Some(*pid);
            } else {
                app.target_pid = None;
            }
        }

        terminal.draw(|f| {
            render(f, &mut app);
            let area = f.area();
            app.fx.process_effects(dt.into(), f.buffer_mut(), area);
        })?;
    }

    shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_trace_adds_events_and_respects_capacity() {
        let mut app = AppState::new();
        assert_eq!(app.trace_log.len(), 0);

        // Add 500 events (up to capacity)
        for i in 0..TRACE_BUFFER_CAPACITY {
            let event = TraceEvent {
                pid: i as u32,
                event_type: pulse_common::EVENT_EXEC,
                comm: [0; 16],
            };
            app.apply_trace(event);
        }
        assert_eq!(app.trace_log.len(), TRACE_BUFFER_CAPACITY);
        assert_eq!(app.trace_log.front().unwrap().pid, 0);

        // Add one more event to trigger eviction
        let extra_event = TraceEvent {
            pid: 9999,
            event_type: pulse_common::EVENT_EXIT,
            comm: [0; 16],
        };
        app.apply_trace(extra_event);

        assert_eq!(app.trace_log.len(), TRACE_BUFFER_CAPACITY);
        // The first event (pid 0) should be evicted, so the front is now pid 1
        assert_eq!(app.trace_log.front().unwrap().pid, 1);
        assert_eq!(app.trace_log.back().unwrap().pid, 9999);
        assert_eq!(app.trace_log.back().unwrap().kind, TraceEventKind::Exit);
    }

    #[test]
    fn test_apply_trace_ignores_events_when_paused() {
        let mut app = AppState::new();
        app.paused = true;

        let event = TraceEvent {
            pid: 123,
            event_type: pulse_common::EVENT_EXEC,
            comm: [0; 16],
        };
        app.apply_trace(event);
        assert_eq!(app.trace_log.len(), 0);
    }
}
