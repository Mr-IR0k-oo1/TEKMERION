//! Terminal User Interface implementation using Ratatui and Crossterm

use crate::pipeline::state_machine::PipelineStateMachine;
use crate::config::Config;
use crate::error::AppError;
use ratatui::prelude::*;
use crossterm::event::{self, Event, KeyCode};
use std::io;
use log::{info, error};

/// Run the TUI application
pub async fn run_tui(
    state_machine: PipelineStateMachine,
    config: Config,
) -> Result<(), AppError> {
    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Main application loop
    let res = run_app(&mut terminal, state_machine, config).await;

    // Restore terminal
    restore_terminal()?;

    res
}

/// Initialize the terminal
fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, AppError> {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    Ok(terminal)
}

/// Restore the terminal to its original state
fn restore_terminal() -> Result<(), AppError> {
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

/// Main application loop
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut state_machine: PipelineStateMachine,
    config: Config,
) -> Result<(), AppError> {
    loop {
        terminal.draw(|f| ui(f, &state_machine, &config))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    info!("Exiting application");
                    break;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// UI rendering function
fn ui(f: &mut Frame, state_machine: &PipelineStateMachine, config: &Config) {
    let size = f.size();
    let block = Block::default()
        .title("Face Identification & Blockchain Verification")
        .borders(Borders::ALL);
    f.render_widget(block, size);

    // Add more UI elements as needed
}
