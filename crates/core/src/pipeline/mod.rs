//! The TEKMERION asynchronous evidence pipeline.
//!
//! This module provides the pipeline stage model ([`crate::pipeline::pipeline`]),
//! the events the runner emits ([`crate::pipeline::events`]), and the async
//! execution engine ([`crate::pipeline::runner`]).
//!
//! The pipeline depends on external work strictly through the engine traits in
//! [`crate::pipeline::pipeline`] (dependency-injection boundaries). No fake or
//! default implementation is provided; a stage whose engine is missing fails
//! with an explicit "not configured" error.

pub mod events;
pub mod pipeline;
pub mod runner;

pub use events::PipelineEvent;
pub use pipeline::{
    CandidateVerifier, DiscoveryEngine, EngineSet, EvidenceEngine, EvidenceRegistry, FaceEngine,
    InputPayload, PipelineError, PipelineStage,
};
pub use runner::{CancellationToken, PipelineRunner, RunnerStatus};
