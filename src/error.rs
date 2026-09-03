//! Error types for the application

use thiserror::Error;
use std::io;
use ratatui::backend::Backend;
use crossterm::ErrorKind;

/// Application error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("TUI error: {0}")]
    Tui(#[from] ratatui::Error),

    #[error("Crossterm error: {0}")]
    Crossterm(#[from] crossterm::ErrorKind),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Pipeline error: {0}")]
    Pipeline(String),

    #[error("Blockchain error: {0}")]
    Blockchain(String),

    #[error("Face recognition error: {0}")]
    FaceRecognition(String),

    #[error("Search error: {0}")]
    Search(String),
}

impl From<ErrorKind> for AppError {
    fn from(error: ErrorKind) -> Self {
        AppError::Crossterm(error)
    }
}
