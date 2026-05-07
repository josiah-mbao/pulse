use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

pub enum InputEvent {
    Quit,
    Up,
    Down,
    SortCpu,
    SortMemory,
    TogglePause,
    None,
}

pub fn read_input() -> InputEvent {
    if event::poll(Duration::from_millis(10)).unwrap_or(false) {
        if let Ok(Event::Key(key)) = event::read() {
            return match key.code {
                KeyCode::Char('q') => InputEvent::Quit,
                KeyCode::Up => InputEvent::Up,
                KeyCode::Down => InputEvent::Down,
                KeyCode::Char('s') => InputEvent::SortCpu, // We'll use 's' to cycle later, for now maps to CPU
                KeyCode::Char('m') => InputEvent::SortMemory,
                KeyCode::Char('p') => InputEvent::TogglePause,
                _ => InputEvent::None,
            };
        }
    }
    InputEvent::None
}
