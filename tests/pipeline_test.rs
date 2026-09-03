//! Tests for pipeline state transitions

use hh_face::pipeline::state::PipelineState;

#[test]
fn test_state_transitions() {
    // Test initial state
    let initial_state = PipelineState::Idle;
    assert_eq!(initial_state.display_name(), "Idle");

    // Test state progression
    let states = [
        PipelineState::InputReady,
        PipelineState::FaceProcessing,
        PipelineState::Searching,
        PipelineState::CandidatesFound,
        PipelineState::Verifying,
        PipelineState::MatchFound,
        PipelineState::EvidenceCreated,
        PipelineState::BlockchainSubmitting,
        PipelineState::BlockchainConfirmed,
        PipelineState::Verified,
    ];

    for state in states {
        assert_ne!(state.display_name(), "Idle");
    }

    // Test error state
    let error_state = PipelineState::Error;
    assert_eq!(error_state.display_name(), "Error");
}
