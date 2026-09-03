pub mod config;
pub mod errors;
pub mod events;
pub mod models;
pub mod pipeline;
pub mod state;

pub use config::Config;
pub use errors::{CoreError, Result};
pub use events::{EventLog, PipelineEvent};
pub use models::{
    BlockchainRecord, EvidenceBundle, EvidenceRecord, FaceAnalysis, FaceDetection, FaceEmbedding,
    PipelineResult, SearchCandidate, VerificationResult, VerificationStatus,
};
pub use pipeline::{
    CandidateVerifier, DiscoveryEngine, EngineSet, EvidenceEngine, EvidenceRegistry, FaceEngine,
    InputPayload, PipelineError, PipelineRunner, PipelineStage, RunnerStatus,
};
pub use state::{PipelineState, StateTransition};
