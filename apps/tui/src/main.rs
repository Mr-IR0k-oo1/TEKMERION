use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{Event, KeyEventKind};
use ratatui::{DefaultTerminal, TerminalOptions, Viewport};
use tekmerion_tui::app::App;
use tekmerion_tui::input::{self, AppAction};
use tekmerion_tui::ui;

fn main() -> Result<()> {
    install_panic_hook();
    let mut terminal = init_terminal()?;
    let result = run(&mut terminal);
    restore_terminal()?;
    result
}

/// Set up a panic hook that restores the terminal before letting the default
/// panic handler print. This prevents leaving the user's terminal in an unreadable
/// raw state with a hidden cursor if a panic occurs.
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));
}

fn init_terminal() -> Result<DefaultTerminal> {
    crossterm::terminal::enable_raw_mode().context("failed to enable raw mode")?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    let options = TerminalOptions {
        viewport: Viewport::Fullscreen,
    };
    let mut terminal = ratatui::try_init_with_options(options)?;
    terminal.hide_cursor()?;
    terminal.clear()?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    let _ = ratatui::try_restore();
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    );
    let _ = crossterm::terminal::disable_raw_mode();
    Ok(())
}

/// Main event loop. Redraws every tick so the interface stays responsive, and
/// polls input with a short timeout. Returns when the caller issues `Quit`.
fn run(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if crossterm::event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let Some(action) = input::handle_key(key) {
                        if action == AppAction::Quit {
                            break;
                        }
                        app.apply(action);
                    }
                }
            }
        }
    }
    Ok(())
}
