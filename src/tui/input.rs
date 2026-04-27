use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

pub enum InputEvent {
    Quit,
    TogglePause,
    SortCpu,
    SortMemory,
    None,
}

pub fn read_input() -> InputEvent {
    if event::poll(Duration::from_millis(10)).unwrap() {
        if let Event::Key(key) = event::read().unwrap() {
            match key.code {
                KeyCode::Char('q') => return InputEvent::Quit,
                KeyCode::Char('p') => return InputEvent::TogglePause,
                KeyCode::Char('c') => return InputEvent::SortCpu,
                KeyCode::Char('m') => return InputEvent::SortMemory,
                _ => InputEvent::None,
            };
        }
    }

    InputEvent::None
}
