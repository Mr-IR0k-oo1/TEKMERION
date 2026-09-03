//! Main application module

use crate::config::Config;
use crate::error::AppError;
use crate::pipeline::state::PipelineState;
use crate::ui::layout::AppLayout;
use ratatui::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Main application state
#[derive(Debug, Clone)]
pub struct App {
    pub config: Arc<Config>,
    pub state: Arc<Mutex<AppState>>,
}

/// Application state
/// Main application state
#[derive(Debug, Clone)]
pub struct AppState {

impl App {
    /// Create a new application instance
    pub fn new(config: Config) -> Self {
        info!("Creating new application");
        Self {
            config: Arc::new(config),
            state: Arc::new(Mutex::new(AppState {
                pipeline_state: PipelineState::Idle,
                status_message: "Ready".to_string(),
                progress: 0.0,
                selected_candidate: 0,
                candidates: Vec::new(),
                evidence: None,
                blockchain_tx: None,
                error: None,
            })),
        }
    }

    /// Run the application
    pub async fn run(&mut self) -> Result<(), AppError> {
        // Initialize UI
        let mut terminal = init_terminal()?;

        // Initialize face model
        let mut face_model = FaceModel::new()?;

        // Initialize search client
        let search_client = SearchClient::new_http(
            self.config.search_api_url.clone(),
            self.config.search_api_key.clone(),
            self.config.max_search_candidates,
            self.config.search_timeout_seconds,
        );

        // Main application loop
        let res = run_app(&mut terminal, self, &mut face_model, &search_client).await;

        // Restore terminal
        restore_terminal()?;

        res
    }

    /// Process an image for face analysis
    pub async fn process_image(&mut self, image_path: &str, face_model: &mut FaceModel) -> Result<(), AppError> {
        let mut state = self.state.lock().await;
        state.pipeline_state = PipelineState::FaceProcessing {
            image_path: image_path.to_string(),
            face_count: None,
            embedding_dimensions: None,
        };
        state.status_message = "Processing face...".to_string();

        // Process the image
        let response = face_model.process_image(image_path).await?;

        if response.success {
            if let Some(embedding) = response.embedding {
                state.pipeline_state = PipelineState::FaceProcessing {
                    image_path: image_path.to_string(),
                    face_count: response.face_count,
                    embedding_dimensions: Some(embedding.len() as u32),
                };
                state.status_message = "Face processing complete".to_string();
            }
        } else {
            state.pipeline_state = PipelineState::Error;
            state.status_message = response.error.unwrap_or_else(|| "Face processing failed".to_string());
            state.error = Some(AppError::WorkerError(state.status_message.clone()));
        }

        Ok(())
    }

    /// Perform a search for the image
    pub async fn perform_search(&mut self, image_path: &str, search_client: &SearchClient) -> Result<(), AppError> {
        let mut state = self.state.lock().await;
        state.pipeline_state = PipelineState::Searching {
            image_path: image_path.to_string(),
            candidate_count: None,
        };
        state.status_message = "Searching for image...".to_string();

        // Perform the search
        let candidates = search_client.search(Path::new(image_path)).await?;

        // Rank the candidates
        let ranked_candidates = rank_candidates(candidates);

        // Convert to display format
        let display_candidates: Vec<String> = ranked_candidates
            .iter()
            .map(|candidate| {
                format!("{}\n{}\n{}",
                    candidate.title,
                    candidate.domain,
                    candidate.url
                )
            })
            .collect();

        state.pipeline_state = PipelineState::CandidatesFound {
            candidates: display_candidates,
        };
        state.status_message = "Search complete".to_string();

        Ok(())
    }
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
    app: &mut App,
    face_model: &mut FaceModel,
    search_client: &SearchClient,
) -> Result<(), AppError> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    info!("Exiting application");
                    break;
                }
                KeyCode::Char('p') => {
                    // Example: Process an image
                    let image_path = "path/to/image.jpg"; // Replace with actual image path
                    app.process_image(image_path, face_model).await?;
                }
                KeyCode::Char('s') => {
                    // Example: Perform a search
                    let image_path = "path/to/image.jpg"; // Replace with actual image path
                    app.perform_search(image_path, search_client).await?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// UI rendering function
fn ui(f: &mut Frame, app: &App) {
    let layout = AppLayout::new(f.size());
    layout.render(f, app);
}
