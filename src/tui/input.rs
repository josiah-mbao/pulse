use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

pub enum InputEvent {
    Quit,
    Up,
    Down,
    SortCpu,
    SortMemory,
    TogglePause,
    EnterFilter,
    Char(char),
    Backspace,
    Esc,
    Enter,
    None,
}

pub fn read_input() -> InputEvent {
    if event::poll(Duration::from_millis(10)).unwrap_or(false) {
        if let Ok(Event::Key(key)) = event::read() {
            return match key.code {
                KeyCode::Char('q') => InputEvent::Quit,
                KeyCode::Up => InputEvent::Up,
                KeyCode::Down => InputEvent::Down,
                KeyCode::Char('s') => InputEvent::SortCpu,
                KeyCode::Char('m') => InputEvent::SortMemory,
                KeyCode::Char('p') => InputEvent::TogglePause,
                KeyCode::Char('/') => InputEvent::EnterFilter,
                KeyCode::Char(c) => InputEvent::Char(c),
                KeyCode::Backspace => InputEvent::Backspace,
                KeyCode::Esc => InputEvent::Esc,
                KeyCode::Enter => InputEvent::Enter,
                _ => InputEvent::None,
            };
        }
    }
    InputEvent::None
}
