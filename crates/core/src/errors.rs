use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("pipeline state transition error: {0}")]
    State(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
