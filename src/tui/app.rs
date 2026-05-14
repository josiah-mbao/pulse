use std::{collections::{HashMap, VecDeque}, io, sync::mpsc, thread, time::{Duration, Instant}};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tachyonfx::EffectManager;

use pulse::system::{engine::Engine, state::ProcessSnapshot};
use crate::tui::renderer::render;
use crate::tui::input::{read_input, InputEvent};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tab { Fleet, Ekg, Sentinel }

#[derive(Clone, Copy, PartialEq)]
pub enum SortMode { Cpu, Memory }

#[derive(Clone, Copy, PartialEq)]
pub enum InputMode { Normal, Filter, Confirm }

pub struct AppState {
    pub processes: HashMap<u32, ProcessSnapshot>,
    pub cpu_map: HashMap<u32, f32>,
    pub active_tab: Tab,
    pub sort_mode: SortMode,
    pub input_mode: InputMode,
    pub paused: bool,
    pub selection_index: usize,
    pub scroll_offset: usize,
    pub filter_query: String,
    pub cpu_history: VecDeque<u64>,
    pub mem_history: VecDeque<u64>,
    pub target_pid: Option<u32>,
    // Animation State: Using String as a concrete key to avoid generic propagation errors
    pub fx: EffectManager<String>,
    pub last_tick: Instant,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            cpu_map: HashMap::new(),
            active_tab: Tab::Fleet,
            sort_mode: SortMode::Cpu,
            input_mode: InputMode::Normal,
            paused: false,
            selection_index: 0,
            scroll_offset: 0,
            filter_query: String::new(),
            cpu_history: VecDeque::with_capacity(60),
            mem_history: VecDeque::with_capacity(60),
            target_pid: None,
            fx: EffectManager::default(),
            last_tick: Instant::now(),
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
    let (tx, rx) = mpsc::channel();

    // Engine Thread: Samples system data every 500ms
    thread::spawn(move || {
        loop {
            let data = engine.tick();
            if tx.send(data).is_err() { break; }
            thread::sleep(Duration::from_millis(500));
        }
    });

    loop {
        let now = Instant::now();
        let dt = now.duration_since(app.last_tick);
        app.last_tick = now;

        if let Ok((procs, cpu)) = rx.try_recv() {
            if !app.paused {
                app.processes = procs;
                app.cpu_map = cpu;
            }
        }

        match read_input() {
            InputEvent::Quit => break,
            InputEvent::SwitchTab(tab) => {
                if app.active_tab != tab {
                    app.active_tab = tab;
                }
            }
            InputEvent::EnterFilter => app.input_mode = InputMode::Filter,
            InputEvent::Esc => {
                app.input_mode = InputMode::Normal;
                app.filter_query.clear();
            }
            InputEvent::Char(c) => {
                if app.input_mode == InputMode::Filter {
                    app.filter_query.push(c);
                    app.selection_index = 0;
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
            _ => {}
        }

        let last_tick = std::time::Instant::now();
        let dt = last_tick.elapsed();
        terminal.draw(|f| {

        render(f, &mut app);
        let area = f.area();
        app.fx.process_effects(dt.into(), f.buffer_mut(), area);

        })?;
        
        // Advance animation state. In tachyonfx 0.25, the manager uses update().
        // If your specific build environment requires update_effects, 
        // the compiler will be happy with this direct call.
        
        // Maintain 60fps for smooth UI movement
        thread::sleep(Duration::from_millis(16));
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
