use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;
use crate::tui::app::Tab;

pub enum InputEvent {
    Quit,
    Up,
    Down,
    SortCpu,
    SortMemory,
    TogglePause,
    EnterFilter,
    SwitchTab(Tab), // New variant for navigation
    Char(char),
    Backspace,
    Esc,
    Enter,
    None,
}

pub fn read_input() -> InputEvent {
    // We keep the poll short to maintain a high-refresh animation loop
    if event::poll(Duration::from_millis(5)).unwrap_or(false) {
        if let Ok(Event::Key(key)) = event::read() {
            return match key.code {
                KeyCode::Char('q') => InputEvent::Quit,
                KeyCode::Up => InputEvent::Up,
                KeyCode::Down => InputEvent::Down,
                KeyCode::Char('s') => InputEvent::SortCpu,
                KeyCode::Char('m') => InputEvent::SortMemory,
                KeyCode::Char('p') => InputEvent::TogglePause,
                KeyCode::Char('/') => InputEvent::EnterFilter,
                // Tab Switching
                KeyCode::Char('1') => InputEvent::SwitchTab(Tab::Fleet),
                KeyCode::Char('2') => InputEvent::SwitchTab(Tab::Ekg),
                KeyCode::Char('3') => InputEvent::SwitchTab(Tab::Sentinel),
                
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
