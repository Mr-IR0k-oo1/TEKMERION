use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{Event, KeyEventKind};
use ratatui::{DefaultTerminal, TerminalOptions, Viewport};
use tekmerion_tui::app::App;
use tekmerion_tui::input::{self, AppAction};
use tekmerion_tui::ui;

#[derive(Debug, Clone)]
struct CliOptions {
    image_path: Option<String>,
    demo_mode: bool,
}

fn parse_cli_args(args: &[String]) -> CliOptions {
    let mut image_path = None;
    let mut demo_mode = false;
    let mut iter = args.iter().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--demo" | "-d" => {
                demo_mode = true;
            }
            "-i" | "--image" => {
                image_path = iter.next().cloned();
            }
            "run" => {
                if let Some(next) = iter.next() {
                    image_path = Some(next.clone());
                }
            }
            other if !other.starts_with('-') => {
                image_path = Some(other.to_string());
            }
            _ => {}
        }
    }

    // Default to assets/query_face.png if no path was provided and the asset exists
    if image_path.is_none() && std::path::Path::new("assets/query_face.png").is_file() {
        image_path = Some("assets/query_face.png".to_string());
    }

    CliOptions {
        image_path,
        demo_mode,
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("TEKMERION Evidence Verification Pipeline\n");
        println!("USAGE:");
        println!("    cargo run -p tekmerion-tui -- [OPTIONS] [IMAGE_PATH]");
        println!("    cargo run -p tekmerion-tui -- run <IMAGE_PATH>\n");
        println!("ARGS:");
        println!("    <IMAGE_PATH>          Path to input image file (JPEG, PNG) to analyze\n");
        println!("OPTIONS:");
        println!("    -i, --image <PATH>    Path to input image file");
        println!("    -d, --demo            Run in deterministic demo mode with local simulation");
        println!("    -h, --help            Print help information\n");
        println!("KEYBINDINGS:");
        println!("    [ENTER] Run           Start forensic verification pipeline");
        println!("    [V]     Verify        Advance through verification stages");
        println!("    [T]     Tamper Test   Simulate unauthorized alteration & prove Merkle tamper detection");
        println!("    [R]     Reset         Reset to initial state with new unique run ID");
        println!("    [1..4]  Tabs          1: Flow, 2: Evidence Tree, 3: Candidates, 4: System Guide");
        println!("    [?]     Help          Toggle architecture and interactive guide");
        println!("    [Q]     Quit          Exit cleanly and restore terminal\n");
        println!("EXAMPLES:");
        println!("    cargo run -p tekmerion-tui -- assets/query_face.png");
        println!("    cargo run -p tekmerion-tui -- run assets/query_face.png");
        println!("    cargo run -p tekmerion-tui -- --demo\n");
        return Ok(());
    }

    let opts = parse_cli_args(&args);
    install_panic_hook();
    let mut terminal = init_terminal()?;
    let result = run(&mut terminal, opts);
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
fn run(terminal: &mut DefaultTerminal, opts: CliOptions) -> Result<()> {
    let mut app = match opts.image_path {
        Some(path) => App::from_image_path(path),
        None => App::new(),
    };
    app.demo_mode = opts.demo_mode;
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
