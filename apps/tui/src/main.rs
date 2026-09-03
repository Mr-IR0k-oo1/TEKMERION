mod app;
mod input;
mod ui;

use std::time::Duration;

use crossterm::event::Event;
use ratatui::DefaultTerminal;

use crate::app::App;
use crate::input::{handle_key_event, poll_event, AppEvent};

fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
    let mut app = App::new();
    loop {
        terminal.draw(|f| ui::render(f, &app))?;
        app.tick();

        if let Some(Event::Key(key)) = poll_event() {
            if let Some(evt) = handle_key_event(key) {
                match evt {
                    AppEvent::StartPipeline => app.start_pipeline(),
                    AppEvent::Verify => app.verify(),
                    AppEvent::Tamper => app.tamper(),
                    AppEvent::Reset => app.reset(),
                    AppEvent::SelectUp => app.select_up(),
                    AppEvent::SelectDown => app.select_down(),
                    AppEvent::Quit => {
                        app.should_quit = true;
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }

        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}
