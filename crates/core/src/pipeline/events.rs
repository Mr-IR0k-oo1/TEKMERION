//! Execution events emitted by the pipeline runner.
//!
//! These record the *runtime* lifecycle of a pipeline run (start, per-stage
//! progress, completion, cancellation, failure) and live alongside the
//! state-machine events in [`crate::events`].

use chrono::{DateTime, Utc};

use super::pipeline::{PipelineError, PipelineStage};

/// A structured event emitted while a pipeline run executes.
///
/// Every stage emits both a [`PipelineEvent::StageStarted`] and a
/// [`PipelineEvent::StageCompleted`] / [`PipelineEvent::StageFailed`], so a
/// consumer can follow progress stage-by-stage without making assumptions.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PipelineEvent {
    /// A run started.
    PipelineStarted { at: DateTime<Utc> },
    /// A stage began executing.
    StageStarted {
        stage: PipelineStage,
        sequence: usize,
    },
    /// A stage completed successfully.
    StageCompleted {
        stage: PipelineStage,
        sequence: usize,
    },
    /// A stage failed with a stage-specific error.
    StageFailed {
        stage: PipelineStage,
        sequence: usize,
        error: PipelineError,
    },
    /// The run completed successfully.
    PipelineCompleted,
    /// The run was cancelled.
    PipelineCancelled,
    /// The run failed.
    PipelineFailed { error: PipelineError },
    /// The runner was reset.
    PipelineReset,
}

impl PipelineEvent {
    /// Which stage a progress event refers to, if any.
    pub fn stage(&self) -> Option<PipelineStage> {
        match self {
            PipelineEvent::StageStarted { stage, .. }
            | PipelineEvent::StageCompleted { stage, .. }
            | PipelineEvent::StageFailed { stage, .. } => Some(*stage),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_failure_event_round_trip() {
        let event = PipelineEvent::StageFailed {
            stage: PipelineStage::Discovery,
            sequence: 3,
            error: PipelineError::NotConfigured(PipelineStage::Discovery),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: PipelineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
    }

    #[test]
    fn stage_event_exposes_its_stage() {
        assert_eq!(
            PipelineEvent::StageStarted {
                stage: PipelineStage::Evidence,
                sequence: 6
            }
            .stage(),
            Some(PipelineStage::Evidence)
        );
        assert_eq!(
            PipelineEvent::PipelineStarted { at: Utc::now() }.stage(),
            None
        );
    }
}
