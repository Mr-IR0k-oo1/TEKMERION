//! Face Identification & Blockchain Verification
//! Main application entry point

use crate::app::App;
use crate::config::{Config, load_config};
use crate::error::AppError;
use crate::face::FaceModel;
use tracing::{info, Level};
use tracing_subscriber;

mod app;
mod config;
mod error;
mod pipeline;
mod ui;
mod face;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    info!("Starting Face Identification & Blockchain Verification");

    // Load configuration
    let config = load_config()?;

    // Create and run application
    let mut app = App::new(config);
    app.run().await?;

    Ok(())
}
