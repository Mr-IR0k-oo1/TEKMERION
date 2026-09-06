use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Direction of a candidate-selection movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

/// High-level actions the interface can perform, produced by the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    /// ENTER: start the pipeline.
    Start,
    /// V: advance/verify the current stage.
    Verify,
    /// T: flag a tamper.
    Tamper,
    /// R: reset the interface to idle.
    Reset,
    /// UP / DOWN: move the candidate selection.
    Select(Direction),
    /// Q or Ctrl+C: quit and restore the terminal.
    Quit,
    /// ? or H: toggle the help / architecture explainer modal.
    ToggleHelp,
    /// TAB: cycle to the next view tab.
    NextTab,
    /// BACKTAB (Shift+Tab): cycle to the previous view tab.
    PrevTab,
    /// 1..=4: switch directly to a view tab (0-indexed).
    SwitchTab(usize),
    /// ESC: close open modals / overlays.
    CloseOverlay,
}

/// Translate a key event into an [`AppAction`].
///
/// Ctrl+C is handled first so it always quits regardless of the focused
/// widget. Case-insensitive letters are accepted for convenience.
pub fn handle_key(key: KeyEvent) -> Option<AppAction> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(AppAction::Quit);
    }
    match key.code {
        KeyCode::Enter => Some(AppAction::Start),
        KeyCode::Char('v') | KeyCode::Char('V') => Some(AppAction::Verify),
        KeyCode::Char('t') | KeyCode::Char('T') => Some(AppAction::Tamper),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(AppAction::Reset),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppAction::Quit),
        KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') => Some(AppAction::ToggleHelp),
        KeyCode::Tab => Some(AppAction::NextTab),
        KeyCode::BackTab => Some(AppAction::PrevTab),
        KeyCode::Char('1') => Some(AppAction::SwitchTab(0)),
        KeyCode::Char('2') => Some(AppAction::SwitchTab(1)),
        KeyCode::Char('3') => Some(AppAction::SwitchTab(2)),
        KeyCode::Char('4') => Some(AppAction::SwitchTab(3)),
        KeyCode::Esc => Some(AppAction::CloseOverlay),
        KeyCode::Up => Some(AppAction::Select(Direction::Up)),
        KeyCode::Down => Some(AppAction::Select(Direction::Down)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn start_verify_tamper_reset_keys() {
        assert_eq!(
            handle_key(key(KeyCode::Enter, KeyModifiers::NONE)),
            Some(AppAction::Start)
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('v'), KeyModifiers::NONE)),
            Some(AppAction::Verify)
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('V'), KeyModifiers::NONE)),
            Some(AppAction::Verify)
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('t'), KeyModifiers::NONE)),
            Some(AppAction::Tamper)
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(AppAction::Reset)
        );
    }

    #[test]
    fn quit_keys() {
        assert_eq!(
            handle_key(key(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(AppAction::Quit)
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('Q'), KeyModifiers::NONE)),
            Some(AppAction::Quit)
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(AppAction::Quit)
        );
    }

    #[test]
    fn selection_keys() {
        assert_eq!(
            handle_key(key(KeyCode::Up, KeyModifiers::NONE)),
            Some(AppAction::Select(Direction::Up))
        );
        assert_eq!(
            handle_key(key(KeyCode::Down, KeyModifiers::NONE)),
            Some(AppAction::Select(Direction::Down))
        );
    }

    #[test]
    fn help_and_overlay_keys() {
        assert_eq!(
            handle_key(key(KeyCode::Char('?'), KeyModifiers::NONE)),
            Some(AppAction::ToggleHelp)
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('h'), KeyModifiers::NONE)),
            Some(AppAction::ToggleHelp)
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('H'), KeyModifiers::NONE)),
            Some(AppAction::ToggleHelp)
        );
        assert_eq!(
            handle_key(key(KeyCode::Esc, KeyModifiers::NONE)),
            Some(AppAction::CloseOverlay)
        );
    }

    #[test]
    fn tab_navigation_keys() {
        assert_eq!(
            handle_key(key(KeyCode::Tab, KeyModifiers::NONE)),
            Some(AppAction::NextTab)
        );
        assert_eq!(
            handle_key(key(KeyCode::BackTab, KeyModifiers::NONE)),
            Some(AppAction::PrevTab)
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('1'), KeyModifiers::NONE)),
            Some(AppAction::SwitchTab(0))
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('2'), KeyModifiers::NONE)),
            Some(AppAction::SwitchTab(1))
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('3'), KeyModifiers::NONE)),
            Some(AppAction::SwitchTab(2))
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('4'), KeyModifiers::NONE)),
            Some(AppAction::SwitchTab(3))
        );
    }

    #[test]
    fn unrecognized_keys() {
        assert_eq!(
            handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            handle_key(key(KeyCode::Char('c'), KeyModifiers::NONE)),
            None
        );
    }
}
