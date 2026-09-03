//! Pipeline state definitions

use crate::error::AppError;
use serde::{Serialize, Deserialize};

/// Pipeline state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PipelineState {
    /// Initial state
    Idle,
    /// Input image ready
    InputReady(String),
    /// Face processing
    FaceProcessing {
        image_path: String,
        face_count: Option<u32>,
        embedding_dimensions: Option<u32>,
    },
    /// Searching for image
    Searching {
        image_path: String,
        candidate_count: Option<u32>,
    },
    /// Candidates found
    CandidatesFound {
        candidates: Vec<String>,
    },
    /// Verifying candidates
    Verifying,
    /// Match found
    MatchFound,
    /// Evidence created
    EvidenceCreated,
    /// Submitting to blockchain
    BlockchainSubmitting,
    /// Blockchain confirmed
    BlockchainConfirmed,
    /// Pipeline completed successfully
    Verified,
    /// Error state
    Error(AppError),
}

impl PipelineState {
    /// Get display name for the state
    pub fn display_name(&self) -> &str {
        match self {
            PipelineState::Idle => "Idle",
            PipelineState::InputReady(_) => "Input Ready",
            PipelineState::FaceProcessing { .. } => "Face Processing",
            PipelineState::Searching { .. } => "Searching",
            PipelineState::CandidatesFound { .. } => "Candidates Found",
            PipelineState::Verifying => "Verifying",
            PipelineState::MatchFound => "Match Found",
            PipelineState::EvidenceCreated => "Evidence Created",
            PipelineState::BlockchainSubmitting => "Blockchain Submitting",
            PipelineState::BlockchainConfirmed => "Blockchain Confirmed",
            PipelineState::Verified => "Verified",
            PipelineState::Error(_) => "Error",
        }
    }
}
