//! Events emitted by the pipeline runner.
//!
//! Every stage reports its lifecycle through a [`PipelineEvent`]. Stage
//! transitions are never silent: the runner always emits a `Transition`,
//! `StageStarted` and (on success) `StageCompleted`, or a `StageFailed` and
//! `PipelineFailed` on error.

use super::pipeline::{PipelineError, PipelineStage};

/// Events emitted while running or controlling the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineEvent {
    /// The pipeline began running.
    PipelineStarted,
    /// The pipeline was reset to its idle state.
    PipelineReset,
    /// A new stage transitioned into.
    Transition {
        /// The stage just left.
        from: PipelineStage,
        /// The stage being entered.
        to: PipelineStage,
    },
    /// A stage began executing.
    StageStarted {
        stage: PipelineStage,
        sequence: usize,
    },
    /// A stage finished successfully.
    StageCompleted {
        stage: PipelineStage,
        sequence: usize,
    },
    /// A stage failed.
    StageFailed {
        stage: PipelineStage,
        sequence: usize,
        error: PipelineError,
    },
    /// The whole pipeline finished successfully.
    PipelineCompleted,
    /// The pipeline was cancelled.
    PipelineCancelled,
    /// The pipeline failed (final stage failure surfaced at the top level).
    PipelineFailed {
        error: PipelineError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_failure_event_round_trips() {
        let event = PipelineEvent::StageFailed {
            stage: PipelineStage::Discovery,
            sequence: 3,
            error: PipelineError::NotConfigured(PipelineStage::Discovery),
        };
        match &event {
            PipelineEvent::StageFailed {
                stage,
                sequence,
                error,
            } => {
                assert_eq!(*stage, PipelineStage::Discovery);
                assert_eq!(*sequence, 3);
                assert_eq!(
                    *error,
                    PipelineError::NotConfigured(PipelineStage::Discovery)
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn transition_event_holds_stages() {
        let event = PipelineEvent::Transition {
            from: PipelineStage::FaceAnalysis,
            to: PipelineStage::Discovery,
        };
        match event {
            PipelineEvent::Transition { from, to } => {
                assert_eq!(from, PipelineStage::FaceAnalysis);
                assert_eq!(to, PipelineStage::Discovery);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
