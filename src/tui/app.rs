use std::{
    collections::HashMap,
    io,
    time::{Duration, Instant},
};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode},
};

use ratatui::{backend::CrosstermBackend, Terminal};

use pulse::system::{
    collector::collect_processes,
    state::{build_state, compute_cpu, ProcessSnapshot},
    cpu::read_total_cpu_time, 
};

use crate::tui::renderer::render;
use crate::tui::input::{read_input, InputEvent};

#[derive(Clone, Copy)]
pub enum SortMode {
    Cpu,
    Memory,
}

pub struct AppState {
    pub processes: HashMap<u32, ProcessSnapshot>,
    pub cpu_map: HashMap<u32, f32>,
    pub sort_mode: SortMode,
    pub paused: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            cpu_map: HashMap::new(),
            sort_mode: SortMode::Cpu,
            paused: false,
        }
    }
}

pub fn run_app() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();
    
    // Persistent state across loop iterations
    let mut prev_processes: HashMap<u32, ProcessSnapshot> = HashMap::new();
    let mut prev_total_cpu = read_total_cpu_time();
    
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_secs(1);

    loop {
        // 1. Process Input (Responsive Poll)
        match read_input() {
            InputEvent::Quit => break,
            InputEvent::TogglePause => app.paused = !app.paused,
            InputEvent::SortCpu => app.sort_mode = SortMode::Cpu,
            InputEvent::SortMemory => app.sort_mode = SortMode::Memory,
            _ => {}
        }

        // 2. Sample Data (Triggered on Tick)
        if last_tick.elapsed() >= tick_rate {
            if !app.paused {
                // Calculate system-wide delta
                let curr_total_cpu = read_total_cpu_time();
                let total_delta = curr_total_cpu.saturating_sub(prev_total_cpu);
                
                let raw = collect_processes();
                
                // FIXED: Now passing 3 arguments as required by state.rs
                let state = build_state(prev_processes, raw, total_delta);
                let cpu_map = compute_cpu(&state);

                app.processes = state.curr.clone();
                app.cpu_map = cpu_map;

                // Update previous state for the next calculation
                prev_processes = state.curr;
                prev_total_cpu = curr_total_cpu;
            }
            last_tick = Instant::now();
        }

        // 3. Render at High Frame Rate
        terminal.draw(|f| {
            render(f, &app);
        })?;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
