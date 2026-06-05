use std::{collections::HashMap, collections::VecDeque, io, sync::mpsc, thread, time::{Duration, Instant}};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, Terminal, widgets::TableState};
use tachyonfx::EffectManager;

use pulse::system::{
    engine::Engine, 
    state::{ProcessSnapshot, TelemetryFrame, CpuJiffies, NetworkStats, read_global_jiffies, read_global_mem_percent, read_network_dev}
};
use crate::tui::renderer::render;
use crate::tui::input::{read_input, InputEvent};

const MAX_HISTORY_POINTS: usize = 200;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tab { Fleet, Ekg, Sentinel }

#[derive(Clone, Copy, PartialEq)]
pub enum SortMode { Cpu, Memory }

#[derive(Clone, Copy, PartialEq)]
pub enum InputMode { Normal, Filter, Confirm }

pub struct AppState {
    pub processes: HashMap<u32, ProcessSnapshot>,
    pub cpu_map: HashMap<u32, f32>,
    pub sorted_pids: Vec<u32>,
    pub table_state: TableState, 
    pub active_tab: Tab,
    pub sort_mode: SortMode,
    pub input_mode: InputMode,
    pub paused: bool,
    pub filter_query: String,
    pub target_pid: Option<u32>,
    pub fx: EffectManager<String>,
    pub last_tick: Instant,
    
    // Time-series buffers for global telemetry
    pub global_cpu_history: VecDeque<f32>,
    pub global_mem_history: VecDeque<f32>,

    // Network monitoring
    pub prev_network: NetworkStats,
    pub current_speeds: HashMap<String, (f32, f32)>,
}

impl AppState {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            processes: HashMap::new(),
            cpu_map: HashMap::new(),
            sorted_pids: Vec::new(),
            table_state,
            active_tab: Tab::Fleet,
            sort_mode: SortMode::Cpu,
            input_mode: InputMode::Normal,
            paused: false,
            filter_query: String::new(),
            target_pid: None,
            fx: EffectManager::default(),
            last_tick: Instant::now(),
            global_cpu_history: VecDeque::with_capacity(MAX_HISTORY_POINTS),
            global_mem_history: VecDeque::with_capacity(MAX_HISTORY_POINTS),
            prev_network: NetworkStats::default(),
            current_speeds: HashMap::new(),
        }
    }

    pub fn update_sorted_pids(&mut self) {
        let mut procs: Vec<_> = self.processes.iter().collect();
        
        if !self.filter_query.is_empty() {
            procs.retain(|(_, p)| p.name.contains(&self.filter_query));
        }

        match self.sort_mode {
            SortMode::Cpu => procs.sort_by(|(a_id, _), (b_id, _)| {
                let a_cpu = self.cpu_map.get(a_id).unwrap_or(&0.0);
                let b_cpu = self.cpu_map.get(b_id).unwrap_or(&0.0);
                b_cpu.partial_cmp(a_cpu).unwrap()
            }),
            SortMode::Memory => procs.sort_by(|(_, a), (_, b)| b.memory_kb.cmp(&a.memory_kb)),
        }
        
        self.sorted_pids = procs.into_iter().map(|(pid, _)| *pid).collect();
        
        if let Some(selected) = self.table_state.selected() {
            if !self.sorted_pids.is_empty() && selected >= self.sorted_pids.len() {
                self.table_state.select(Some(self.sorted_pids.len() - 1));
            }
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
    let mut engine = Engine::new();
    let (tx, rx) = mpsc::channel::<TelemetryFrame>();

    // Background Thread Loop: Synthesizes system-wide metrics alongside processes
    thread::spawn(move || {
        let mut prev_jiffies = read_global_jiffies().unwrap_or(CpuJiffies { total: 0, idle: 0 });
        loop {
            let (procs, cpu) = engine.tick();
            
            let mut global_cpu = 0.0;
            if let Some(curr_jiffies) = read_global_jiffies() {
                let total_d = curr_jiffies.total.saturating_sub(prev_jiffies.total);
                let idle_d = curr_jiffies.idle.saturating_sub(prev_jiffies.idle);
                if total_d > 0 {
                    global_cpu = ((total_d - idle_d) as f32 / total_d as f32) * 100.0;
                }
                prev_jiffies = curr_jiffies;
            }

            let global_mem = read_global_mem_percent();

            let frame = TelemetryFrame {
                processes: procs,
                cpu_map: cpu,
                global_cpu_utilization: global_cpu,
                global_mem_utilization: global_mem,
                network: read_network_dev().unwrap_or_default(),
            };

            if tx.send(frame).is_err() { break; }
            thread::sleep(Duration::from_millis(500));
        }
    });

    loop {
        let now = Instant::now();
        let dt = now.duration_since(app.last_tick);
        app.last_tick = now;

        if let Ok(frame) = rx.try_recv() {
            if !app.paused {
                app.processes = frame.processes;
                app.cpu_map = frame.cpu_map;
                
                // Shift time-series rolling history constraints
                if app.global_cpu_history.len() >= MAX_HISTORY_POINTS {
                    app.global_cpu_history.pop_front();
                }
                app.global_cpu_history.push_back(frame.global_cpu_utilization);

                if app.global_mem_history.len() >= MAX_HISTORY_POINTS {
                    app.global_mem_history.pop_front();
                }
                app.global_mem_history.push_back(frame.global_mem_utilization);

                // Calculate network speeds (KiB/s) based on 500ms sampling window
                app.current_speeds.clear();
                for (name, curr) in &frame.network.interfaces {
                    if let Some(prev) = app.prev_network.interfaces.get(name) {
                        let rx_delta = curr.rx_bytes.saturating_sub(prev.rx_bytes);
                        let tx_delta = curr.tx_bytes.saturating_sub(prev.tx_bytes);
                        
                        // Multiply by 2.0 to scale 500ms -> 1s, divide by 1024.0 for KiB
                        let rx_kib = (rx_delta as f32 * 2.0) / 1024.0;
                        let tx_kib = (tx_delta as f32 * 2.0) / 1024.0;
                        
                        app.current_speeds.insert(name.clone(), (rx_kib, tx_kib));
                    }
                }
                app.prev_network = frame.network;

                app.update_sorted_pids();
            }
        }

        let timeout = if app.fx.is_running() {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(100) 
        };

        match read_input(timeout) {
            InputEvent::Quit => break,
            InputEvent::SwitchTab(tab) => app.active_tab = tab,
            InputEvent::EnterFilter => app.input_mode = InputMode::Filter,
            InputEvent::Esc => {
                app.input_mode = InputMode::Normal;
                app.filter_query.clear();
                app.update_sorted_pids();
            }
            InputEvent::Char(c) => {
                if app.input_mode == InputMode::Filter {
                    app.filter_query.push(c);
                    app.update_sorted_pids();
                    app.table_state.select(Some(0));
                }
            }
            InputEvent::Backspace => {
                if app.input_mode == InputMode::Filter {
                    app.filter_query.pop();
                    app.update_sorted_pids();
                }
            }
            InputEvent::Up => {
                if app.input_mode == InputMode::Normal {
                    let i = match app.table_state.selected() {
                        Some(i) => i.saturating_sub(1),
                        None => 0,
                    };
                    app.table_state.select(Some(i));
                }
            }
            InputEvent::Down => {
                if app.input_mode == InputMode::Normal {
                    let i = match app.table_state.selected() {
                        Some(i) => {
                            if i >= app.sorted_pids.len().saturating_sub(1) { i } else { i + 1 }
                        }
                        None => 0,
                    };
                    app.table_state.select(Some(i));
                }
            }
            InputEvent::Top => {
                if app.input_mode == InputMode::Normal {
                    app.table_state.select(Some(0));
                }
            }
            InputEvent::Bottom => {
                if app.input_mode == InputMode::Normal {
                    let max = app.sorted_pids.len().saturating_sub(1);
                    app.table_state.select(Some(max));
                }
            }
            InputEvent::SortCpu => {
                app.sort_mode = SortMode::Cpu;
                app.update_sorted_pids();
            }
            InputEvent::SortMemory => {
                app.sort_mode = SortMode::Memory;
                app.update_sorted_pids();
            }
            InputEvent::TogglePause => app.paused = !app.paused,
            _ => {}
        }

        if let Some(idx) = app.table_state.selected() {
            app.target_pid = app.sorted_pids.get(idx).copied();
        }

        terminal.draw(|f| {
            render(f, &mut app);
            let area = f.area();
            app.fx.process_effects(dt.into(), f.buffer_mut(), area);
        })?;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
