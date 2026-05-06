use std::{
    collections::HashMap,
    io,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode},
};

use ratatui::{backend::CrosstermBackend, Terminal};

use pulse::system::{
    engine::Engine,
    state::ProcessSnapshot,
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

    // --- PHASE 3: Background Collection Setup ---
    let (tx, rx) = mpsc::channel();
    
    // Spawn the collector thread
    thread::spawn(move || {
        let mut engine = Engine::new();
        loop {
            // Collect and calculate data
            let (processes, cpu_map) = engine.tick();
            
            // Send to the UI thread
            if tx.send((processes, cpu_map)).is_err() {
                break; // UI thread has dropped, exit collector
            }
            
            // Fixed sampling rate of 1 second
            thread::sleep(Duration::from_secs(1));
        }
    });

    loop {
        // 1. Process Input (Instant Response)
        match read_input() {
            InputEvent::Quit => break,
            InputEvent::TogglePause => app.paused = !app.paused,
            InputEvent::SortCpu => app.sort_mode = SortMode::Cpu,
            InputEvent::SortMemory => app.sort_mode = SortMode::Memory,
            _ => {}
        }

        // 2. Non-blocking Check for New Data
        if !app.paused {
            // try_recv allows the UI to keep moving even if a new snapshot isn't ready
            if let Ok((new_processes, new_cpu_map)) = rx.try_recv() {
                app.processes = new_processes;
                app.cpu_map = new_cpu_map;
            }
        }

        // 3. Render at High Frame Rate
        terminal.draw(|f| {
            render(f, &app);
        })?;
        
        // Minor sleep to prevent 100% CPU usage by the UI loop itself
        thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
