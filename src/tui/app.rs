use std::io;
use std::collections::HashMap;
use std::{thread::sleep, time::Duration};

use crate::system::{
    collector::collect_processes,
    state::{build_state, compute_cpu, ProcessSnapshot},
};
use crate::system::state::ProcessSnapshot;
use crate::tui::renderer::render;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, Terminal};

loop {
    terminal.draw(|f| {
        render(f, &AppState::new());
    })?;

    break;
}

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

    terminal.clear()?;

    let mut app = AppState::new();
    let mut prev: HashMap<u32, ProcessSnapshot> = HashMap::new();

    loop {
        if !app.paused {
            let raw = collect_processes();
            let state = build_state(prev, raw);

            let cpu_map = compute_cpu(&state);

            app.processes = state.curr.clone();
            app.cpu_map = cpu_map;

            prev = state.curr;
        }

        terminal.draw(|f| {
            render(f, &app);
        })?;

        sleep(Duration::from_millis(1));
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
