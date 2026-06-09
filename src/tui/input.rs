use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;
use crate::tui::app::Tab;

pub enum InputEvent {
    Quit,
    Up,
    Down,
    Top,
    Bottom,
    SortCpu,
    SortMemory,
    TogglePause,
    EnterFilter,
    InitiateKill,
    SwitchTab(Tab),
    Char(char),
    Backspace,
    Esc,
    Enter,
    None,
}

pub fn read_input(timeout: Duration) -> InputEvent {
    // Blocks only for the adaptive timeout duration
    if event::poll(timeout).unwrap_or(false) {
        if let Ok(Event::Key(key)) = event::read() {
            return match key.code {
                KeyCode::Char('q') => InputEvent::Quit,
                // Arrow keys + Vim bindings
                KeyCode::Up => InputEvent::Up,
                KeyCode::Char('k') => InputEvent::InitiateKill,
                KeyCode::Down | KeyCode::Char('j') => InputEvent::Down,
                KeyCode::Char('g') => InputEvent::Top,
                KeyCode::Char('G') => {
                    // Check for shift modifier for capital G, or just map standard G
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        InputEvent::Bottom
                    } else {
                        InputEvent::Char('G')
                    }
                },
                KeyCode::Char('s') => InputEvent::SortCpu,
                KeyCode::Char('m') => InputEvent::SortMemory,
                KeyCode::Char('p') => InputEvent::TogglePause,
                KeyCode::Char('/') => InputEvent::EnterFilter,
                
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
