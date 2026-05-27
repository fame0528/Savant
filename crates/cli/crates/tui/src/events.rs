use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

/// Terminal events handled by the TUI
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    /// Regular key press
    Key(KeyCode, KeyModifiers),
    /// Mouse event (reserved for future use)
    Mouse,
    /// Terminal resize
    Resize(u16, u16),
    /// Tick for rendering/animation
    Tick,
    /// Quit requested
    Quit,
}

/// Event handler that polls crossterm events
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate),
        }
    }

    /// Poll for the next event with timeout
    pub fn next(&self) -> Result<TerminalEvent> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                Event::Key(key) => Ok(TerminalEvent::Key(key.code, key.modifiers)),
                Event::Mouse(_) => Ok(TerminalEvent::Mouse),
                Event::Resize(w, h) => Ok(TerminalEvent::Resize(w, h)),
                _ => Ok(TerminalEvent::Tick),
            }
        } else {
            Ok(TerminalEvent::Tick)
        }
    }
}

/// Convert a crossterm key event to an action
pub fn handle_key(code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    match (code, modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => Some(Action::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Action::Quit),
        (KeyCode::Esc, _) => Some(Action::Escape),
        (KeyCode::Enter, _) => Some(Action::Submit),
        (KeyCode::Backspace, _) => Some(Action::Backspace),
        (KeyCode::Delete, _) => Some(Action::Delete),
        (KeyCode::Left, _) => Some(Action::CursorLeft),
        (KeyCode::Right, _) => Some(Action::CursorRight),
        (KeyCode::Up, _) => Some(Action::CursorUp),
        (KeyCode::Down, _) => Some(Action::CursorDown),
        (KeyCode::Tab, _) => Some(Action::TabNext),
        (KeyCode::BackTab, _) => Some(Action::TabPrev),
        (KeyCode::PageUp, _) => Some(Action::PageUp),
        (KeyCode::PageDown, _) => Some(Action::PageDown),
        (KeyCode::Home, _) => Some(Action::Home),
        (KeyCode::End, _) => Some(Action::End),
        (KeyCode::Char(':'), KeyModifiers::NONE) => Some(Action::CommandPalette),
        (KeyCode::Char('/'), KeyModifiers::NONE) => Some(Action::Search),
        (KeyCode::Char(c), KeyModifiers::NONE) => Some(Action::Char(c)),
        (KeyCode::Char(c), KeyModifiers::SHIFT) => Some(Action::Char(c)),
        _ => None,
    }
}

/// Actions that the TUI can perform
#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Escape,
    Submit,
    Backspace,
    Delete,
    CursorLeft,
    CursorRight,
    CursorUp,
    CursorDown,
    TabNext,
    TabPrev,
    PageUp,
    PageDown,
    Home,
    End,
    Char(char),
    CommandPalette,
    Search,
}
