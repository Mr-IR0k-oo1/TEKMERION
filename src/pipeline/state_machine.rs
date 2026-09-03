//! Core pipeline state machine

use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents the current state of the pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineState {
    Idle,
    FaceDetection { image_path: PathBuf },
    FaceEmbedding { embedding: Vec<f32> },
    Searching { query: String },
    CandidateDiscovery { candidates: Vec<String> },
    Verification { similarity_score: f32 },
    Canonicalization { evidence: String },
    Hashing { hash: String },
    BlockchainRegistration { tx_hash: String },
    VerificationComplete { result: bool },
}

/// Pipeline state machine
#[derive(Debug, Clone)]
pub struct PipelineStateMachine {
    pub current_state: PipelineState,
}

impl PipelineStateMachine {
    /// Create a new pipeline state machine
    pub fn new() -> Self {
        Self {
            current_state: PipelineState::Idle,
        }
    }

    /// Transition to the next state in the pipeline
    pub fn transition(&mut self, new_state: PipelineState) -> Result<(), AppError> {
        // Add validation logic for state transitions here
        self.current_state = new_state;
        Ok(())
    }
}
