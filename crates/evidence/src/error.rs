//! Structured errors for TEKMERION evidence processing and serialization.

use thiserror::Error;

/// Errors that can occur during evidence processing, serialization, or verification.
#[derive(Debug, Error, PartialEq)]
pub enum EvidenceError {
    #[error("floating point field '{field}' must be finite (got {value})")]
    NonFiniteFloat { field: &'static str, value: f32 },

    #[error("schema version mismatch: expected '{expected}', found '{found}'")]
    InvalidSchemaVersion { found: String, expected: String },

    #[error("failed to serialize evidence canonically: {0}")]
    Serialization(String),

    #[error("invalid or malformed URL: {0}")]
    InvalidUrl(String),
}
