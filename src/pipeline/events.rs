//! Pipeline event definitions

use crate::error::AppError;
use crate::pipeline::state::PipelineState;
use serde::{Serialize, Deserialize};

/// Pipeline event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineEvent {
    /// State changed
    StateChanged(PipelineState),
    /// Progress update
    Progress(f32),
    /// Status message
    Status(String),
    /// Error occurred
    Error(AppError),
    /// Pipeline completed
    Completed,
}
