use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

pub enum InputEvent {
    Quit,
    TogglePause,
    SortCpu,
    SortMemory,
    Tick, 
    None,
}

pub fn read_input() -> InputEvent {
    // Poll for 10ms to allow the loop to stay responsive
    if event::poll(Duration::from_millis(10)).unwrap() {
        if let Event::Key(key) = event::read().unwrap() {
            match key.code {
                KeyCode::Char('q') => return InputEvent::Quit,
                KeyCode::Char('p') => return InputEvent::TogglePause,
                KeyCode::Char('c') => return InputEvent::SortCpu,
                KeyCode::Char('m') => return InputEvent::SortMemory,
                _ => return InputEvent::None,
            };
        }
    }

    // If no key was pressed within 10ms, return a Tick event
    InputEvent::Tick
}
