use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::state::PipelineState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum PipelineEvent {
    StateTransition {
        from: PipelineState,
        to: PipelineState,
        timestamp: DateTime<Utc>,
    },
    FaceAnalyzed {
        face_count: usize,
        timestamp: DateTime<Utc>,
    },
    CandidatesDiscovered {
        count: usize,
        timestamp: DateTime<Utc>,
    },
    VerificationCompleted {
        match_found: bool,
        best_similarity: f32,
        timestamp: DateTime<Utc>,
    },
    EvidenceAnchored {
        root_hash: String,
        tx_hash: String,
        timestamp: DateTime<Utc>,
    },
    ErrorOccurred {
        message: String,
        timestamp: DateTime<Utc>,
    },
}

impl PipelineEvent {
    pub fn new_transition(from: PipelineState, to: PipelineState) -> Self {
        Self::StateTransition {
            from,
            to,
            timestamp: Utc::now(),
        }
    }
}
