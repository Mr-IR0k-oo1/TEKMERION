//! Pipeline stage definitions, structured errors, and dependency-injection
//! boundaries.
//!
//! The pipeline is modelled as an ordered sequence of [`PipelineStage`]s. Each
//! stage that requires external work is backed by a trait (engine) that must be
//! supplied by the caller. No default or fake implementation is provided here:
//! when an engine is absent the runner reports an explicit
//! [`PipelineError::NotConfigured`] error for the offending stage.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::{BlockchainRecord, EvidenceBundle, FaceAnalysis, SearchCandidate, VerificationResult};

/// The ordered stages of the TEKMERION evidence pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineStage {
    Input,
    FaceAnalysis,
    Discovery,
    CandidateVerification,
    MatchSelection,
    Evidence,
    Blockchain,
    OnchainVerification,
}

impl PipelineStage {
    /// All stages in execution order.
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

    /// Returns the next stage in the pipeline, or [`None`] for the final stage.
    pub fn next(self) -> Option<PipelineStage> {
        match self {
            PipelineStage::Input => Some(PipelineStage::FaceAnalysis),
            PipelineStage::FaceAnalysis => Some(PipelineStage::Discovery),
            PipelineStage::Discovery => Some(PipelineStage::CandidateVerification),
            PipelineStage::CandidateVerification => Some(PipelineStage::MatchSelection),
            PipelineStage::MatchSelection => Some(PipelineStage::Evidence),
            PipelineStage::Evidence => Some(PipelineStage::Blockchain),
            PipelineStage::Blockchain => Some(PipelineStage::OnchainVerification),
            PipelineStage::OnchainVerification => None,
        }
    }

    /// Zero-based index of the stage within [`PipelineStage::ALL`].
    pub fn index(self) -> usize {
        PipelineStage::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// Human-readable label for the stage.
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

/// Structured error type produced by the pipeline and its engines.
///
/// The [`Stage`] and [`NotConfigured`] variants carry the stage responsible so
/// failures can be reported per-stage without losing context.
///
/// [`Stage`]: PipelineError::Stage
/// [`NotConfigured`]: PipelineError::NotConfigured
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum PipelineError {
    /// A stage depends on an engine that has not been supplied.
    #[error("engine not configured for stage {0:?}")]
    NotConfigured(PipelineStage),

    /// A specific stage failed. The message does not leak internal error
    /// types but preserves the failing stage.
    #[error("stage {stage:?} failed: {message}")]
    Stage { stage: PipelineStage, message: String },

    /// The pipeline was cancelled by the caller.
    #[error("pipeline cancelled")]
    Cancelled,

    /// A control operation was not valid for the current runner state.
    #[error("invalid pipeline transition: {0}")]
    InvalidTransition(String),

    /// No candidate passed the verification threshold.
    #[error("no matching candidate found")]
    NoMatch,

    /// The input supplied to the pipeline was invalid.
    #[error("invalid pipeline input: {0}")]
    Input(String),

    /// An unexpected internal error.
    #[error("internal pipeline error: {0}")]
    Internal(String),
}

impl PipelineError {
    /// True if the error is a stage-specific failure (as opposed to a control
    /// error such as cancellation or an invalid transition).
    pub fn is_stage_failure(&self) -> bool {
        matches!(
            self,
            PipelineError::Stage { .. }
                | PipelineError::NotConfigured(_)
                | PipelineError::NoMatch
                | PipelineError::Input(_)
        )
    }
}

/// Output of the built-in INPUT stage.
#[derive(Debug, Clone)]
pub struct InputPayload {
    /// Free-form source identifier of the media being analysed.
    pub source: String,
}

impl InputPayload {
    /// Creates a new input payload, rejecting empty sources.
    pub fn new(source: impl Into<String>) -> Result<Self, PipelineError> {
        let source = source.into();
        if source.trim().is_empty() {
            return Err(PipelineError::Input(
                "source must not be empty".to_string(),
            ));
        }
        Ok(Self { source })
    }
}

/// Engine that analyses face data from the input media.
#[async_trait]
pub trait FaceEngine: Send + Sync {
    async fn analyze(&self, input: &InputPayload) -> Result<FaceAnalysis, PipelineError>;
}

/// Engine that performs reverse-image / facial discovery for a face analysis.
#[async_trait]
pub trait DiscoveryEngine: Send + Sync {
    async fn discover(
        &self,
        analysis: &FaceAnalysis,
    ) -> Result<Vec<SearchCandidate>, PipelineError>;
}

/// Engine that verifies discovered candidates against the source face.
#[async_trait]
pub trait CandidateVerifier: Send + Sync {
    async fn verify(
        &self,
        candidates: Vec<SearchCandidate>,
    ) -> Result<Vec<VerificationResult>, PipelineError>;
}

/// Engine that bundles the selected verified match into an evidence bundle.
#[async_trait]
pub trait EvidenceEngine: Send + Sync {
    async fn build_evidence(
        &self,
        matched: VerificationResult,
    ) -> Result<EvidenceBundle, PipelineError>;
}

/// Engine that anchors evidence on-chain and verifies an existing anchor.
#[async_trait]
pub trait EvidenceRegistry: Send + Sync {
    async fn register(&self, bundle: EvidenceBundle) -> Result<BlockchainRecord, PipelineError>;

    async fn verify_anchor(
        &self,
        tx_hash: &str,
    ) -> Result<BlockchainRecord, PipelineError>;
}

/// Container for the engines that back the pipeline.
///
/// Any slot left [`None`] causes the corresponding stage to fail with
/// [`PipelineError::NotConfigured`]. This is the explicit "not configured"
/// boundary for unavailable implementations. Engines are stored as [`Arc`] so
/// they can be cheaply shared into per-stage spawned tasks.
///
/// [`Arc`]: std::sync::Arc
#[derive(Debug, Default, Clone)]
pub struct EngineSet {
    pub face: Option<Arc<dyn FaceEngine>>,
    pub discovery: Option<Arc<dyn DiscoveryEngine>>,
    pub verification: Option<Arc<dyn CandidateVerifier>>,
    pub evidence: Option<Arc<dyn EvidenceEngine>>,
    pub registry: Option<Arc<dyn EvidenceRegistry>>,
}

impl EngineSet {
    /// An engine set with no engines configured.
    pub fn none() -> Self {
        Self::default()
    }
}
