use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    StartPipeline,
    Verify,
    Tamper,
    Reset,
    SelectUp,
    SelectDown,
    Quit,
}

pub fn poll_event() -> Option<Event> {
    if event::poll(Duration::ZERO).ok()? {
        event::read().ok()
    } else {
        None
    }
}

pub fn handle_key_event(key: event::KeyEvent) -> Option<AppEvent> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(AppEvent::Quit);
    }
    match key.code {
        KeyCode::Char('q') => Some(AppEvent::Quit),
        KeyCode::Char('Q') => Some(AppEvent::Quit),
        KeyCode::Enter => Some(AppEvent::StartPipeline),
        KeyCode::Char('v') => Some(AppEvent::Verify),
        KeyCode::Char('V') => Some(AppEvent::Verify),
        KeyCode::Char('t') => Some(AppEvent::Tamper),
        KeyCode::Char('T') => Some(AppEvent::Tamper),
        KeyCode::Char('r') => Some(AppEvent::Reset),
        KeyCode::Char('R') => Some(AppEvent::Reset),
        KeyCode::Up => Some(AppEvent::SelectUp),
        KeyCode::Down => Some(AppEvent::SelectDown),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    #[test]
    fn test_quit_key() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(AppEvent::Quit)
        );
    }

    #[test]
    fn test_quit_uppercase() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Char('Q'), KeyModifiers::NONE)),
            Some(AppEvent::Quit)
        );
    }

    #[test]
    fn test_ctrl_c() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(AppEvent::Quit)
        );
    }

    #[test]
    fn test_enter() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Enter, KeyModifiers::NONE)),
            Some(AppEvent::StartPipeline)
        );
    }

    #[test]
    fn test_verify() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Char('v'), KeyModifiers::NONE)),
            Some(AppEvent::Verify)
        );
    }

    #[test]
    fn test_verify_uppercase() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Char('V'), KeyModifiers::NONE)),
            Some(AppEvent::Verify)
        );
    }

    #[test]
    fn test_tamper() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Char('t'), KeyModifiers::NONE)),
            Some(AppEvent::Tamper)
        );
    }

    #[test]
    fn test_tamper_uppercase() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Char('T'), KeyModifiers::NONE)),
            Some(AppEvent::Tamper)
        );
    }

    #[test]
    fn test_reset() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(AppEvent::Reset)
        );
    }

    #[test]
    fn test_reset_uppercase() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Char('R'), KeyModifiers::NONE)),
            Some(AppEvent::Reset)
        );
    }

    #[test]
    fn test_select_up() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Up, KeyModifiers::NONE)),
            Some(AppEvent::SelectUp)
        );
    }

    #[test]
    fn test_select_down() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Down, KeyModifiers::NONE)),
            Some(AppEvent::SelectDown)
        );
    }

    #[test]
    fn test_unrecognized_key() {
        assert_eq!(
            handle_key_event(make_key(KeyCode::Char('x'), KeyModifiers::NONE)),
            None
        );
    }

    #[test]
    fn test_release_event_ignored() {
        let key = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Release,
            state: crossterm::event::KeyEventState::NONE,
        };
        assert_eq!(handle_key_event(key), None);
    }
}
