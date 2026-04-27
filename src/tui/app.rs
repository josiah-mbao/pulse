use std::{
    collections::HashMap,
    io,
    thread::sleep,
    time::Duration,
};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode},
};

use ratatui::{backend::CrosstermBackend, Terminal};

use pulse::system::{
    collector::collect_processes,
    state::{build_state, compute_cpu, ProcessSnapshot},
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
    let mut prev: HashMap<u32, ProcessSnapshot> = HashMap::new();

    loop {
        match read_input() {
            InputEvent::Quit => break,
            InputEvent::TogglePause => app.paused = !app.paused,
            InputEvent::SortCpu => app.sort_mode = SortMode::Cpu,
            InputEvent::SortMemory => app.sort_mode = SortMode::Memory,
            InputEvent::None => {}
        }

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

        sleep(Duration::from_secs(1));
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
