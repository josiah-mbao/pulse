use std::{collections::HashMap, io, sync::mpsc, thread, time::Duration};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use pulse::system::{engine::Engine, state::ProcessSnapshot};
use crate::tui::renderer::render;
use crate::tui::input::{read_input, InputEvent};

#[derive(Clone, Copy, PartialEq)]
pub enum SortMode { Cpu, Memory }

#[derive(Clone, Copy, PartialEq)]
pub enum InputMode { Normal, Filter }

pub struct AppState {
    pub processes: HashMap<u32, ProcessSnapshot>,
    pub cpu_map: HashMap<u32, f32>,
    pub sort_mode: SortMode,
    pub input_mode: InputMode,
    pub paused: bool,
    // UX State
    pub selection_index: usize,
    pub scroll_offset: usize,
    pub filter_query: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            cpu_map: HashMap::new(),
            sort_mode: SortMode::Cpu,
            input_mode: InputMode::Normal,
            paused: false,
            selection_index: 0,
            scroll_offset: 0,
            filter_query: String::new(),
        }
    }
}

pub fn run_app() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut app = AppState::new();
    let (tx, rx) = mpsc::channel();
    
    // Background Engine Thread
    thread::spawn(move || {
        let mut engine = Engine::new();
        loop {
            let (proc, cpu) = engine.tick();
            if tx.send((proc, cpu)).is_err() { break; }
            thread::sleep(Duration::from_secs(1));
        }
    });

    loop {
        // Handle background data updates
        if let Ok((new_proc, new_cpu)) = rx.try_recv() {
            if !app.paused {
                app.processes = new_proc;
                app.cpu_map = new_cpu;
            }
        }

        // Handle Input Events
        match read_input() {
            InputEvent::Quit => if app.input_mode == InputMode::Normal { break },
            InputEvent::EnterFilter => {
                app.input_mode = InputMode::Filter;
            }
            InputEvent::Esc => {
                app.input_mode = InputMode::Normal;
                app.filter_query.clear();
            }
            InputEvent::Enter => {
                app.input_mode = InputMode::Normal;
            }
            InputEvent::Char(c) => {
                if app.input_mode == InputMode::Filter {
                    app.filter_query.push(c);
                    app.selection_index = 0; // Reset selection when filtering
                }
            }
            InputEvent::Backspace => {
                if app.input_mode == InputMode::Filter {
                    app.filter_query.pop();
                }
            }
            InputEvent::Up => {
                if app.input_mode == InputMode::Normal {
                    app.selection_index = app.selection_index.saturating_sub(1);
                }
            }
            InputEvent::Down => {
                if app.input_mode == InputMode::Normal {
                    app.selection_index += 1;
                }
            }
            InputEvent::SortCpu => app.sort_mode = SortMode::Cpu,
            InputEvent::SortMemory => app.sort_mode = SortMode::Memory,
            InputEvent::TogglePause => app.paused = !app.paused,
            InputEvent::None => {}
        }

        terminal.draw(|f| render(f, &mut app))?;
        thread::sleep(Duration::from_millis(16)); // ~60fps
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
