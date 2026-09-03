use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineState {
    Idle,
    InputReady,
    FaceAnalysis,
    Searching,
    CandidatesFound,
    Verifying,
    MatchFound,
    EvidenceCreated,
    BlockchainSubmitting,
    BlockchainConfirmed,
    VerifyingOnchain,
    Verified,
    TamperDetected,
    Error,
}

impl PipelineState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineState::Verified | PipelineState::TamperDetected | PipelineState::Error
        )
    }
}
