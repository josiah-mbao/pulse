use std::{collections::HashMap, collections::VecDeque, io, sync::mpsc, thread, time::{Duration, Instant}};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, Terminal, widgets::TableState, layout::Rect, style::Color};
use tachyonfx::{EffectManager, fx, Interpolation};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

use pulse::system::{
    engine::Engine, 
    state::{ProcessSnapshot, TelemetryFrame, CpuJiffies, NetworkStats, read_global_jiffies, read_global_mem_percent, read_network_dev, read_disk_io}
};
use crate::tui::renderer::{render, BG_CANVAS};
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
    pub process_depths: HashMap<u32, usize>,
    pub table_state: TableState, 
    pub active_tab: Tab,
    pub sort_mode: SortMode,
    pub input_mode: InputMode,
    pub paused: bool,
    pub tree_mode: bool,
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
}

impl AppState {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            processes: HashMap::new(),
            cpu_map: HashMap::new(),
            sorted_pids: Vec::new(),
            process_depths: HashMap::new(),
            table_state,
            active_tab: Tab::Fleet,
            sort_mode: SortMode::Cpu,
            input_mode: InputMode::Normal,
            paused: false,
            tree_mode: false,
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
        }
    }

    pub fn update_sorted_pids(&mut self) {
        self.process_depths.clear();
        if !self.tree_mode {
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
        } else {
            self.sorted_pids.clear();
            let mut child_map: HashMap<u32, Vec<u32>> = HashMap::new();
            let mut roots = Vec::new();

            for (&pid, proc) in &self.processes {
                if !self.filter_query.is_empty() && !proc.name.contains(&self.filter_query) {
                    continue;
                }

                if proc.ppid == 0 || proc.ppid == 1 || !self.processes.contains_key(&proc.ppid) {
                    roots.push(pid);
                } else {
                    child_map.entry(proc.ppid).or_default().push(pid);
                }
            }

            // Deterministic sorting of roots and siblings
            let sort_fn = |a: &u32, b: &u32| {
                match self.sort_mode {
                    SortMode::Cpu => {
                        let a_cpu = self.cpu_map.get(a).unwrap_or(&0.0);
                        let b_cpu = self.cpu_map.get(b).unwrap_or(&0.0);
                        b_cpu.partial_cmp(a_cpu).unwrap()
                    }
                    SortMode::Memory => {
                        let a_mem = self.processes.get(a).map(|p| p.memory_kb).unwrap_or(0);
                        let b_mem = self.processes.get(b).map(|p| p.memory_kb).unwrap_or(0);
                        b_mem.cmp(&a_mem)
                    }
                }
            };

            roots.sort_by(sort_fn);

            let mut stack: Vec<(u32, usize)> = roots.into_iter().map(|r| (r, 0)).collect();
            stack.reverse(); // Process roots in sorted order using stack

            let mut visited = Vec::new();
            while let Some((pid, depth)) = stack.pop() {
                visited.push(pid);
                self.process_depths.insert(pid, depth);
                if let Some(mut children) = child_map.remove(&pid) {
                    children.sort_by(sort_fn);
                    children.reverse(); // Reverse so they are pushed onto stack in correct order
                    for child_pid in children {
                        stack.push((child_pid, depth + 1));
                    }
                }
            }
            self.sorted_pids = visited;
        }
        
        // Safety Clamping: Eliminate out-of-bounds rendering panic risks
        if let Some(selected) = self.table_state.selected() {
            if self.sorted_pids.is_empty() {
                self.table_state.select(None);
            } else if selected >= self.sorted_pids.len() {
                self.table_state.select(Some(self.sorted_pids.len().saturating_sub(1)));
            }
        } else if !self.sorted_pids.is_empty() {
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
            let (disk_r, disk_w) = read_disk_io().unwrap_or((0, 0));

            let frame = TelemetryFrame {
                processes: procs,
                cpu_map: cpu,
                global_cpu_utilization: global_cpu,
                global_mem_utilization: global_mem,
                network: read_network_dev().unwrap_or_default(),
                disk_sectors_read: disk_r,
                disk_sectors_written: disk_w,
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

                // Disk Velocity Calculation (KiB/s)
                // 1 sector = 512 bytes = 0.5 KiB. Sample every 500ms -> KiB/s = delta_sectors * 0.5 * 2 = delta_sectors
                if app.prev_disk_read > 0 || app.prev_disk_write > 0 {
                    let r_delta = frame.disk_sectors_read.saturating_sub(app.prev_disk_read) as f32;
                    let w_delta = frame.disk_sectors_written.saturating_sub(app.prev_disk_write) as f32;

                    if app.disk_read_history.len() >= MAX_HISTORY_POINTS { app.disk_read_history.pop_front(); }
                    app.disk_read_history.push_back(r_delta);

                    if app.disk_write_history.len() >= MAX_HISTORY_POINTS { app.disk_write_history.pop_front(); }
                    app.disk_write_history.push_back(w_delta);
                }
                app.prev_disk_read = frame.disk_sectors_read;
                app.prev_disk_write = frame.disk_sectors_written;

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
            InputEvent::SwitchTab(tab) => {
                if app.active_tab != tab {
                    app.active_tab = tab;
                    if let Ok(size) = terminal.size() {
                        // The main rendering area starts below tab bar (3) and stats (3).
                        let main_area = Rect::new(0, 6, size.width, size.height.saturating_sub(7));
                        app.fx.add_effect(
                            fx::fade_from(BG_CANVAS, BG_CANVAS, (150, Interpolation::QuadOut))
                                .with_area(main_area)
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
                    } else if app.input_mode == InputMode::Confirm {
                        // Treat 'k' in confirm mode as SIGKILL
                        if let Some(target_pid) = app.target_pid {
                            match kill(Pid::from_raw(target_pid as i32), Signal::SIGKILL) {
                                Ok(_) => {
                                    app.input_mode = InputMode::Normal;
                                    app.fx.add_effect(fx::fade_from(Color::White, Color::White, (200, Interpolation::Linear)));
                                }
                                Err(e) => {
                                    let msg = match e {
                                        nix::errno::Errno::EPERM => "Error: Permission Denied (Run as root)",
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
            }
            InputEvent::Esc => {
                if app.show_help {
                    app.show_help = false;
                } else {
                    app.input_mode = InputMode::Normal;
                    app.filter_query.clear();
                    app.update_sorted_pids();
                }
            }
            InputEvent::Char(c) => {
                match app.input_mode {
                    InputMode::Filter => {
                        app.filter_query.push(c);
                        app.update_sorted_pids();
                        app.table_state.select(Some(0));
                    }
                    InputMode::Confirm => {
                        match c {
                            't' | 'T' => {
                                if let Some(target_pid) = app.target_pid {
                                    match kill(Pid::from_raw(target_pid as i32), Signal::SIGTERM) {
                                        Ok(_) => {
                                            app.input_mode = InputMode::Normal;
                                            app.fx.add_effect(fx::fade_from(Color::White, Color::White, (200, Interpolation::Linear)));
                                        }
                                        Err(e) => {
                                            let msg = match e {
                                                nix::errno::Errno::EPERM => "Error: Permission Denied (Run as root)",
                                                nix::errno::Errno::ESRCH => "Error: Process no longer exists",
                                                _ => "Error: Signal failed",
                                            };
                                            app.error_message = Some((msg.to_string(), Instant::now()));
                                            app.input_mode = InputMode::Normal;
                                        }
                                    }
                                }
                            }
                            'k' | 'K' => {
                                // Handled by InitiateKill event too, but adding here for completeness if Char is fired
                                if let Some(target_pid) = app.target_pid {
                                    match kill(Pid::from_raw(target_pid as i32), Signal::SIGKILL) {
                                        Ok(_) => {
                                            app.input_mode = InputMode::Normal;
                                            app.fx.add_effect(fx::fade_from(Color::White, Color::White, (200, Interpolation::Linear)));
                                        }
                                        Err(e) => {
                                            let msg = match e {
                                                nix::errno::Errno::EPERM => "Error: Permission Denied (Run as root)",
                                                nix::errno::Errno::ESRCH => "Error: Process no longer exists",
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
                        }
                    }
                    _ => {}
                }
            }
            InputEvent::Backspace => {
                if app.input_mode == InputMode::Filter {
                    app.filter_query.pop();
                    app.update_sorted_pids();
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
                            if i >= app.sorted_pids.len().saturating_sub(1) { i } else { i + 1 }
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
                    let max = app.sorted_pids.len().saturating_sub(1);
                    app.table_state.select(Some(max));
                }
            }
            InputEvent::SortCpu => {
                if !app.show_help {
                    app.sort_mode = SortMode::Cpu;
                    app.update_sorted_pids();
                }
            }
            InputEvent::SortMemory => {
                if !app.show_help {
                    app.sort_mode = SortMode::Memory;
                    app.update_sorted_pids();
                }
            }
            InputEvent::TogglePause => {
                if !app.show_help {
                    app.paused = !app.paused;
                }
            }
            InputEvent::ToggleTree => {
                if app.input_mode == InputMode::Normal && !app.show_help {
                    app.tree_mode = !app.tree_mode;
                    app.update_sorted_pids();
                }
            }
            InputEvent::ToggleHelp => {
                if app.input_mode == InputMode::Normal {
                    app.show_help = !app.show_help;
                }
            }
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
