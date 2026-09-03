//! Pipeline stage model, structured errors, engine traits and the dependency
//! injection container.
//!
//! This module defines *abstractions only*: the stages the pipeline walks
//! through, the errors it can produce, and the trait boundaries that concrete
//! implementations (face, discovery, verification, evidence, blockchain) will
//! satisfy in later phases. No implementation is provided here.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::models::{
    BlockchainRecord, EvidenceBundle, FaceAnalysis, SearchCandidate, VerificationResult,
};

/// A stage in the pipeline, executed in this exact order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineStage {
    /// Source acquisition.
    Input,
    /// Face detection and embedding.
    FaceAnalysis,
    /// Web / reverse-image search for candidate matches.
    Discovery,
    /// Verify discovered candidates against the face embedding.
    CandidateVerification,
    /// Select the best matching candidate.
    MatchSelection,
    /// Build the evidence bundle.
    Evidence,
    /// Register the evidence root on-chain.
    Blockchain,
    /// Verify the on-chain anchor.
    OnchainVerification,
}

impl PipelineStage {
    pub const ALL: [PipelineStage; 8] = [
        PipelineStage::Input,
        PipelineStage::FaceAnalysis,
        PipelineStage::Discovery,
        PipelineStage::CandidateVerification,
        PipelineStage::MatchSelection,
        PipelineStage::Evidence,
        PipelineStage::Blockchain,
        PipelineStage::OnchainVerification,
    ];

    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|s| *s == self)
            .expect("every stage is present in ALL")
    }

    pub fn next(self) -> Option<PipelineStage> {
        Self::ALL.get(self.index() + 1).copied()
    }

    pub fn label(self) -> &'static str {
        match self {
            PipelineStage::Input => "INPUT",
            PipelineStage::FaceAnalysis => "FACE_ANALYSIS",
            PipelineStage::Discovery => "DISCOVERY",
            PipelineStage::CandidateVerification => "CANDIDATE_VERIFICATION",
            PipelineStage::MatchSelection => "MATCH_SELECTION",
            PipelineStage::Evidence => "EVIDENCE",
            PipelineStage::Blockchain => "BLOCKCHAIN",
            PipelineStage::OnchainVerification => "ONCHAIN_VERIFICATION",
        }
    }
}

/// Structured, stage-aware error produced by the pipeline or its engines.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
pub enum PipelineError {
    /// The engine required by a stage is not wired in.
    #[error("engine not configured for stage {0:?}")]
    NotConfigured(PipelineStage),

    /// A stage's engine failed with a message.
    #[error("stage {stage:?} failed: {message}")]
    Stage {
        stage: PipelineStage,
        message: String,
    },

    /// The pipeline was cancelled while a stage was in flight.
    #[error("pipeline cancelled")]
    Cancelled,

    /// An illegal state transition was attempted.
    #[error("invalid pipeline transition: {0}")]
    InvalidTransition(String),

    /// No candidate matched the similarity threshold.
    #[error("no matching candidate found")]
    NoMatch,

    /// The input payload was invalid.
    #[error("invalid pipeline input: {0}")]
    Input(String),

    /// An internal, non-stage-specific failure.
    #[error("internal pipeline error: {0}")]
    Internal(String),
}

impl PipelineError {
    /// Whether the error is a stage-local failure vs. a control-flow error
    /// (cancellation / invalid transition).
    pub fn is_stage_failure(&self) -> bool {
        matches!(
            self,
            Self::Stage { .. } | Self::NotConfigured(_) | Self::NoMatch | Self::Input(_)
        )
    }
}

/// Payload that begins the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputPayload {
    pub source: String,
}

impl InputPayload {
    pub fn new(source: impl Into<String>) -> Result<Self, PipelineError> {
        let source = source.into();
        if source.trim().is_empty() {
            return Err(PipelineError::Input("source must not be empty".to_string()));
        }
        Ok(Self { source })
    }
}

/// Boundaries that concrete implementations satisfy. These are dependency
/// injection contracts only; no implementations exist in this phase.
#[async_trait]
pub trait FaceEngine: Send + Sync {
    async fn analyze(&self, input: &InputPayload) -> Result<FaceAnalysis, PipelineError>;
}

#[async_trait]
pub trait DiscoveryEngine: Send + Sync {
    async fn discover(
        &self,
        analysis: &FaceAnalysis,
    ) -> Result<Vec<SearchCandidate>, PipelineError>;
}

#[async_trait]
pub trait CandidateVerifier: Send + Sync {
    async fn verify(
        &self,
        candidates: Vec<SearchCandidate>,
    ) -> Result<Vec<VerificationResult>, PipelineError>;
}

#[async_trait]
pub trait EvidenceEngine: Send + Sync {
    async fn build_evidence(
        &self,
        matched: VerificationResult,
    ) -> Result<EvidenceBundle, PipelineError>;
}

#[async_trait]
pub trait EvidenceRegistry: Send + Sync {
    async fn register(&self, bundle: EvidenceBundle) -> Result<BlockchainRecord, PipelineError>;

    async fn verify_anchor(&self, tx_hash: &str) -> Result<BlockchainRecord, PipelineError>;
}

/// Dependency-injection container for the pipeline's engine boundaries.
///
/// A `None` entry means the corresponding stage is not available and will
/// report a [`PipelineError::NotConfigured`] error at run time.
#[derive(Default, Clone)]
pub struct EngineSet {
    pub face: Option<Arc<dyn FaceEngine>>,
    pub discovery: Option<Arc<dyn DiscoveryEngine>>,
    pub verification: Option<Arc<dyn CandidateVerifier>>,
    pub evidence: Option<Arc<dyn EvidenceEngine>>,
    pub registry: Option<Arc<dyn EvidenceRegistry>>,
}

impl EngineSet {
    /// An empty set: every engine boundary is `None`.
    pub fn none() -> Self {
        Self::default()
    }
}

impl fmt::Debug for EngineSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EngineSet")
            .field("face", &self.face.is_some())
            .field("discovery", &self.discovery.is_some())
            .field("verification", &self.verification.is_some())
            .field("evidence", &self.evidence.is_some())
            .field("registry", &self.registry.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_next_ordering() {
        assert_eq!(
            PipelineStage::Input.next(),
            Some(PipelineStage::FaceAnalysis)
        );
        assert_eq!(
            PipelineStage::Discovery.next(),
            Some(PipelineStage::CandidateVerification)
        );
        assert_eq!(PipelineStage::OnchainVerification.next(), None);
    }

    #[test]
    fn stage_index_and_label() {
        assert_eq!(PipelineStage::Input.index(), 0);
        assert_eq!(PipelineStage::Input.label(), "INPUT");
        assert_eq!(PipelineStage::OnchainVerification.index(), 7);
        assert_eq!(PipelineStage::CandidateVerification.index(), 3);
    }

    #[test]
    fn input_payload_rejects_empty_source() {
        assert!(matches!(
            InputPayload::new("   "),
            Err(PipelineError::Input(_))
        ));
        assert!(InputPayload::new("image.png").is_ok());
    }

    #[test]
    fn error_classification_is_stage_failure() {
        assert!(PipelineError::NotConfigured(PipelineStage::FaceAnalysis).is_stage_failure());
        assert!(PipelineError::NoMatch.is_stage_failure());
        assert!(!PipelineError::Cancelled.is_stage_failure());
        assert!(!PipelineError::InvalidTransition("x".to_string()).is_stage_failure());
    }

    #[test]
    fn engine_set_none_is_all_unconfigured() {
        let set = EngineSet::none();
        assert!(set.face.is_none());
        assert!(set.discovery.is_none());
        assert!(set.verification.is_none());
        assert!(set.evidence.is_none());
        assert!(set.registry.is_none());
    }
}
