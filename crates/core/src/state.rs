use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::errors::{CoreError, Result};

/// The discrete states a TEKMERION pipeline can occupy.
///
/// The enum is deliberately fieldless: all per-step payloads live in the
/// domain structures (`models`) and are recorded on the transition events, so
/// the state itself remains a cheap, comparable tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineState {
    /// Pipeline has been constructed and is waiting for input.
    Idle,
    /// A source was accepted and is ready for analysis.
    InputReady,
    /// Face detection/embedding is being performed.
    FaceAnalysis,
    /// Searching external sources for candidate matches.
    Searching,
    /// One or more candidates were discovered.
    CandidatesFound,
    /// Candidates are being verified against the face embedding.
    Verifying,
    /// A candidate crossed the similarity threshold.
    MatchFound,
    /// An evidence bundle was assembled for the matched candidate.
    EvidenceCreated,
    /// The evidence root hash is being submitted to a blockchain anchor.
    BlockchainSubmitting,
    /// The blockchain anchor submission was confirmed.
    BlockchainConfirmed,
    /// Performing on-chain verification of the registered anchor.
    VerifyingOnchain,
    /// The pipeline completed successfully.
    Verified,
    /// A tamper was detected during verification.
    TamperDetected,
    /// The pipeline ended in a non-recoverable error.
    Error,
}

impl PipelineState {
    /// All states in canonical pipeline order.
    pub const ALL: [PipelineState; 14] = [
        PipelineState::Idle,
        PipelineState::InputReady,
        PipelineState::FaceAnalysis,
        PipelineState::Searching,
        PipelineState::CandidatesFound,
        PipelineState::Verifying,
        PipelineState::MatchFound,
        PipelineState::EvidenceCreated,
        PipelineState::BlockchainSubmitting,
        PipelineState::BlockchainConfirmed,
        PipelineState::VerifyingOnchain,
        PipelineState::Verified,
        PipelineState::TamperDetected,
        PipelineState::Error,
    ];

    /// Position within [`Self::ALL`].
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|s| *s == self)
            .expect("every PipelineState is present in ALL")
    }

    /// Stable, uppercase human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            PipelineState::Idle => "IDLE",
            PipelineState::InputReady => "INPUT_READY",
            PipelineState::FaceAnalysis => "FACE_ANALYSIS",
            PipelineState::Searching => "SEARCHING",
            PipelineState::CandidatesFound => "CANDIDATES_FOUND",
            PipelineState::Verifying => "VERIFYING",
            PipelineState::MatchFound => "MATCH_FOUND",
            PipelineState::EvidenceCreated => "EVIDENCE_CREATED",
            PipelineState::BlockchainSubmitting => "BLOCKCHAIN_SUBMITTING",
            PipelineState::BlockchainConfirmed => "BLOCKCHAIN_CONFIRMED",
            PipelineState::VerifyingOnchain => "VERIFYING_ONCHAIN",
            PipelineState::Verified => "VERIFIED",
            PipelineState::TamperDetected => "TAMPER_DETECTED",
            PipelineState::Error => "ERROR",
        }
    }

    /// Whether the state is a terminal endpoint of the pipeline.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            PipelineState::Verified | PipelineState::TamperDetected | PipelineState::Error
        )
    }

    /// The set of states reachable from `self`, in deterministic order.
    ///
    /// The forward (happy-path) flow is a straight line from `Idle` to
    /// `Verified`; the failure and tamper edges branch off as shown below.
    pub fn valid_transitions(self) -> &'static [PipelineState] {
        match self {
            PipelineState::Idle => &[PipelineState::InputReady],
            PipelineState::InputReady => &[PipelineState::FaceAnalysis],
            PipelineState::FaceAnalysis => &[PipelineState::Searching],
            PipelineState::Searching => &[PipelineState::CandidatesFound, PipelineState::Error],
            PipelineState::CandidatesFound => &[PipelineState::Verifying, PipelineState::Error],
            PipelineState::Verifying => &[
                PipelineState::MatchFound,
                PipelineState::TamperDetected,
                PipelineState::Error,
            ],
            PipelineState::MatchFound => &[PipelineState::EvidenceCreated],
            PipelineState::EvidenceCreated => &[PipelineState::BlockchainSubmitting],
            PipelineState::BlockchainSubmitting => {
                &[PipelineState::BlockchainConfirmed, PipelineState::Error]
            }
            PipelineState::BlockchainConfirmed => &[PipelineState::VerifyingOnchain],
            PipelineState::VerifyingOnchain => &[
                PipelineState::Verified,
                PipelineState::TamperDetected,
                PipelineState::Error,
            ],
            PipelineState::Verified | PipelineState::TamperDetected | PipelineState::Error => &[],
        }
    }

    /// True if `to` is a legal successor of `self`.
    pub fn can_transition_to(self, to: PipelineState) -> bool {
        self.valid_transitions().contains(&to)
    }

    /// Validate and record a transition from `self` to `to`.
    ///
    /// Returns a [`StateTransition`] on success, or a [`CoreError::State`]
    /// describing the illegal jump.
    pub fn transition(self, to: PipelineState) -> Result<StateTransition> {
        if !self.can_transition_to(to) {
            return Err(CoreError::State(format!(
                "illegal transition from {} to {}",
                self.label(),
                to.label()
            )));
        }
        Ok(StateTransition::new(self, to))
    }
}

/// A validated arc from one [`PipelineState`] to another, stamped with the
/// time the transition was recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: PipelineState,
    pub to: PipelineState,
    pub timestamp: DateTime<Utc>,
}

impl StateTransition {
    pub fn new(from: PipelineState, to: PipelineState) -> Self {
        Self {
            from,
            to,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_state_is_indexed_unique() {
        for (i, state) in PipelineState::ALL.iter().enumerate() {
            assert_eq!(state.index(), i);
        }
    }

    #[test]
    fn happy_path_is_a_straight_line() {
        let flow = [
            PipelineState::Idle,
            PipelineState::InputReady,
            PipelineState::FaceAnalysis,
            PipelineState::Searching,
            PipelineState::CandidatesFound,
            PipelineState::Verifying,
            PipelineState::MatchFound,
            PipelineState::EvidenceCreated,
            PipelineState::BlockchainSubmitting,
            PipelineState::BlockchainConfirmed,
            PipelineState::VerifyingOnchain,
            PipelineState::Verified,
        ];
        for pair in flow.windows(2) {
            assert!(
                pair[0].can_transition_to(pair[1]),
                "{} must transition to {}",
                pair[0].label(),
                pair[1].label()
            );
        }
    }

    #[test]
    fn valid_transitions_are_deterministic() {
        assert_eq!(
            PipelineState::Idle.valid_transitions(),
            &[PipelineState::InputReady]
        );
        assert_eq!(
            PipelineState::Searching.valid_transitions(),
            &[PipelineState::CandidatesFound, PipelineState::Error]
        );
        assert_eq!(
            PipelineState::VerifyingOnchain.valid_transitions(),
            &[
                PipelineState::Verified,
                PipelineState::TamperDetected,
                PipelineState::Error
            ]
        );
        assert!(PipelineState::Verified.valid_transitions().is_empty());
    }

    #[test]
    fn valid_transition_produces_record() {
        let t = PipelineState::Idle
            .transition(PipelineState::InputReady)
            .expect("Idle -> InputReady is legal");
        assert_eq!(t.from, PipelineState::Idle);
        assert_eq!(t.to, PipelineState::InputReady);
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        assert!(!PipelineState::Idle.can_transition_to(PipelineState::Verified));
        assert!(!PipelineState::MatchFound.can_transition_to(PipelineState::Searching));
        assert!(!PipelineState::Verified.can_transition_to(PipelineState::Idle));

        match PipelineState::Idle.transition(PipelineState::Verified) {
            Err(CoreError::State(msg)) => {
                assert!(msg.contains("IDLE"));
                assert!(msg.contains("VERIFIED"));
            }
            other => panic!("expected State error, got {other:?}"),
        }
    }

    #[test]
    fn terminal_states_are_flagged() {
        for terminal in [
            PipelineState::Verified,
            PipelineState::TamperDetected,
            PipelineState::Error,
        ] {
            assert!(terminal.is_terminal());
        }
        assert!(!PipelineState::Idle.is_terminal());
        assert!(!PipelineState::Verifying.is_terminal());
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(PipelineState::Idle.label(), "IDLE");
        assert_eq!(PipelineState::VerifyingOnchain.label(), "VERIFYING_ONCHAIN");
        assert_eq!(PipelineState::Error.label(), "ERROR");
    }

    #[test]
    fn state_serialization_round_trip() {
        let json = serde_json::to_string(&PipelineState::BlockchainConfirmed).unwrap();
        assert_eq!(json, "\"BlockchainConfirmed\"");
        let back: PipelineState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PipelineState::BlockchainConfirmed);
    }

    #[test]
    fn transition_serialization_round_trip() {
        let t = StateTransition {
            from: PipelineState::CandidatesFound,
            to: PipelineState::Verifying,
            timestamp: chrono::DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        };
        let json = serde_json::to_string(&t).unwrap();
        let back: StateTransition = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }
}
