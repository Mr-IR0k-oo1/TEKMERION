//! Error types for audit logging and persistence operations.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("filesystem I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("tamper detected in run bundle {run_id}: field {field} modified (expected {expected}, found {actual})")]
    TamperDetected {
        run_id: String,
        field: String,
        expected: String,
        actual: String,
    },

    #[error("invalid run bundle at {0}: missing required file")]
    InvalidBundle(PathBuf),
}
