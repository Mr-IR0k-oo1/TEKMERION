//! Append-only JSONL audit logger.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

use crate::error::AuditError;
use crate::events::AuditEvent;

/// Append-only audit logger writing to `audit.jsonl`.
#[derive(Debug, Clone)]
pub struct AuditLogger {
    file_path: PathBuf,
    history: Arc<RwLock<Vec<AuditEvent>>>,
}

impl AuditLogger {
    /// Create a new audit logger pointed at a specific JSONL file path.
    pub async fn new(file_path: impl Into<PathBuf>) -> Result<Self, AuditError> {
        let file_path = file_path.into();
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AuditError::Io {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
        }

        Ok(Self {
            file_path,
            history: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Create an in-memory logger (useful for tests and demos without disk persistence).
    pub fn in_memory() -> Self {
        Self {
            file_path: PathBuf::from("in-memory-audit.jsonl"),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Append an event to the audit trail and in-memory history.
    pub async fn log(&self, event: AuditEvent) -> Result<(), AuditError> {
        // Record in memory
        {
            let mut hist = self.history.write().await;
            hist.push(event.clone());
        }

        // Append line to JSONL file if parent dir or path is writable
        let line = serde_json::to_string(&event)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .await
            .map_err(|e| AuditError::Io {
                path: self.file_path.clone(),
                source: e,
            })?;

        file.write_all(format!("{}\n", line).as_bytes())
            .await
            .map_err(|e| AuditError::Io {
                path: self.file_path.clone(),
                source: e,
            })?;

        file.flush().await.map_err(|e| AuditError::Io {
            path: self.file_path.clone(),
            source: e,
        })?;

        Ok(())
    }

    /// Retrieve all recorded events in this session.
    pub async fn get_events(&self) -> Vec<AuditEvent> {
        let hist = self.history.read().await;
        hist.clone()
    }
}
