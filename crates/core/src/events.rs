use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::PipelineState;

/// Structured events emitted for every pipeline state transition.
///
/// Each variant corresponds to reaching a particular [`PipelineState`] and
/// carries the domain data that matters for that step. Instances are produced
/// via [`PipelineEvent::for_state`] (the canonical mapping) or the unit-style
/// constructors, and appended to an [`EventLog`] in transition order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PipelineEvent {
    /// Pipeline constructed and awaiting input.
    PipelineCreated { at: DateTime<Utc> },
    /// A source was accepted (`InputReady`).
    InputAccepted { source: String, at: DateTime<Utc> },
    /// Face analysis requested.
    AnalysisRequested { at: DateTime<Utc> },
    /// Face detection/embedding produced results.
    AnalysisCompleted {
        detections: usize,
        embeddings: usize,
        at: DateTime<Utc>,
    },
    /// Searching external sources for candidates.
    SearchStarted { at: DateTime<Utc> },
    /// Candidates discovered.
    CandidatesFound { count: usize, at: DateTime<Utc> },
    /// Candidates verified against the face embedding.
    CandidatesVerified { reviewed: usize, at: DateTime<Utc> },
    /// A candidate matched the similarity threshold.
    MatchConfirmed { similarity: f32, at: DateTime<Utc> },
    /// No candidate crossed the threshold (`Error` terminal).
    NoMatch { at: DateTime<Utc> },
    /// Evidence bundle assembled.
    EvidenceCommitted {
        root_hash: String,
        at: DateTime<Utc>,
    },
    /// Submitting the evidence root to a blockchain anchor.
    BlockchainSubmissionStarted { at: DateTime<Utc> },
    /// Blockchain anchor confirmed.
    BlockchainConfirmed {
        tx_hash: String,
        block: u64,
        at: DateTime<Utc>,
    },
    /// Performing on-chain verification of the registered root.
    OnchainVerificationStarted { at: DateTime<Utc> },
    /// Pipeline completed successfully (`Verified`).
    Verified { at: DateTime<Utc> },
    /// A tamper was detected during verification (`TamperDetected`).
    TamperDetected { reason: String, at: DateTime<Utc> },
    /// Pipeline failed (`Error` terminal).
    Failed { message: String, at: DateTime<Utc> },
}

impl PipelineEvent {
    /// Canonical event for entering a given state.
    ///
    /// Structured payloads that are not known at the point of the transition
    /// use neutral defaults; callers enrich them with the writer methods or by
    /// constructing the fully-populated variant directly.
    pub fn for_state(state: PipelineState) -> Self {
        let at = Utc::now();
        match state {
            PipelineState::Idle => Self::PipelineCreated { at },
            PipelineState::InputReady => Self::InputAccepted {
                source: String::new(),
                at,
            },
            PipelineState::FaceAnalysis => Self::AnalysisRequested { at },
            PipelineState::Searching => Self::SearchStarted { at },
            PipelineState::CandidatesFound => Self::CandidatesFound { count: 0, at },
            PipelineState::Verifying => Self::CandidatesVerified { reviewed: 0, at },
            PipelineState::MatchFound => Self::MatchConfirmed {
                similarity: 0.0,
                at,
            },
            PipelineState::EvidenceCreated => Self::EvidenceCommitted {
                root_hash: String::new(),
                at,
            },
            PipelineState::BlockchainSubmitting => Self::BlockchainSubmissionStarted { at },
            PipelineState::BlockchainConfirmed => Self::BlockchainConfirmed {
                tx_hash: String::new(),
                block: 0,
                at,
            },
            PipelineState::VerifyingOnchain => Self::OnchainVerificationStarted { at },
            PipelineState::Verified => Self::Verified { at },
            PipelineState::TamperDetected => Self::TamperDetected {
                reason: String::new(),
                at,
            },
            PipelineState::Error => Self::Failed {
                message: String::new(),
                at,
            },
        }
    }

    /// The state this event is associated with (inverse of [`Self::for_state`]).
    pub fn state(&self) -> PipelineState {
        match self {
            PipelineEvent::PipelineCreated { .. } => PipelineState::Idle,
            PipelineEvent::InputAccepted { .. } => PipelineState::InputReady,
            PipelineEvent::AnalysisRequested { .. } => PipelineState::FaceAnalysis,
            PipelineEvent::AnalysisCompleted { .. } => PipelineState::FaceAnalysis,
            PipelineEvent::SearchStarted { .. } => PipelineState::Searching,
            PipelineEvent::CandidatesFound { .. } => PipelineState::CandidatesFound,
            PipelineEvent::CandidatesVerified { .. } => PipelineState::Verifying,
            PipelineEvent::MatchConfirmed { .. } => PipelineState::MatchFound,
            PipelineEvent::NoMatch { .. } => PipelineState::Error,
            PipelineEvent::EvidenceCommitted { .. } => PipelineState::EvidenceCreated,
            PipelineEvent::BlockchainSubmissionStarted { .. } => {
                PipelineState::BlockchainSubmitting
            }
            PipelineEvent::BlockchainConfirmed { .. } => PipelineState::BlockchainConfirmed,
            PipelineEvent::OnchainVerificationStarted { .. } => PipelineState::VerifyingOnchain,
            PipelineEvent::Verified { .. } => PipelineState::Verified,
            PipelineEvent::TamperDetected { .. } => PipelineState::TamperDetected,
            PipelineEvent::Failed { .. } => PipelineState::Error,
        }
    }
}

/// An ordered history of [`PipelineEvent`]s.
///
/// Records are appended in the order transitions occur, so replay preserves
/// chronology without relying on any map ordering.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EventLog {
    events: Vec<PipelineEvent>,
}

impl EventLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: PipelineEvent) {
        self.events.push(event);
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &PipelineEvent> {
        self.events.iter()
    }

    pub fn all(&self) -> &[PipelineEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_state_maps_every_state() {
        for state in PipelineState::ALL {
            let event = PipelineEvent::for_state(state);
            assert_eq!(event.state(), state, "event must map back to {state:?}");
        }
    }

    #[test]
    fn structured_event_round_trip() {
        let event = PipelineEvent::BlockchainConfirmed {
            tx_hash: "0xabc".to_string(),
            block: 12,
            at: chrono::DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: PipelineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
    }

    #[test]
    fn event_log_preserves_order() {
        let mut log = EventLog::new();
        assert!(log.is_empty());

        let first = PipelineEvent::InputAccepted {
            source: "a.jpg".to_string(),
            at: Utc::now(),
        };
        let second = PipelineEvent::AnalysisCompleted {
            detections: 2,
            embeddings: 2,
            at: Utc::now(),
        };
        log.push(first.clone());
        log.push(second.clone());

        assert_eq!(log.len(), 2);
        let all = log.all();
        assert_eq!(&all[0], &first);
        assert_eq!(&all[1], &second);
    }

    #[test]
    fn event_serialization_round_trip_log() {
        let mut log = EventLog::new();
        log.push(PipelineEvent::PipelineCreated { at: Utc::now() });
        log.push(PipelineEvent::MatchConfirmed {
            similarity: 0.91,
            at: Utc::now(),
        });
        let json = serde_json::to_string(&log).unwrap();
        let back: EventLog = serde_json::from_str(&json).unwrap();
        assert_eq!(log, back);
    }

    #[test]
    fn no_match_maps_to_error_state() {
        let event = PipelineEvent::NoMatch { at: Utc::now() };
        assert_eq!(event.state(), PipelineState::Error);
    }
}
