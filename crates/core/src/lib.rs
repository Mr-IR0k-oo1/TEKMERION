pub mod errors;
pub mod events;
pub mod models;
pub mod pipeline;
pub mod state;
pub mod config;

pub use errors::{CoreError, Result};
pub use state::PipelineState;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PipelineState;
    use crate::events::PipelineEvent;
    use crate::models::*;
    use chrono::Utc;
    use url::Url;

    #[test]
    fn test_state_transitions() {
        let mut state = PipelineState::Idle;

        // Valid progression
        state = PipelineState::InputReady;
        assert_eq!(state, PipelineState::InputReady);

        state = PipelineState::FaceAnalysis;
        assert_eq!(state, PipelineState::FaceAnalysis);

        state = PipelineState::Verified;
        assert!(state.is_terminal());
    }

    #[test]
    fn test_model_serialization() {
        let candidate = SearchCandidate {
            url: Url::parse("https://example.com").unwrap(),
            title: Some("Test".to_string()),
            provider: "Google".to_string(),
            image_url: None,
            snippet: None,
            discovered_at: Utc::now(),
        };

        let json = serde_json::to_string(&candidate).unwrap();
        let deserialized: SearchCandidate = serde_json::from_str(&json).unwrap();

        assert_eq!(candidate.url, deserialized.url);
        assert_eq!(candidate.provider, deserialized.provider);
    }

    #[test]
    fn test_event_serialization() {
        let event = PipelineEvent::new_transition(PipelineState::Idle, PipelineState::InputReady);
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("\"type\":\"StateTransition\""));

        let deserialized: PipelineEvent = serde_json::from_str(&json).unwrap();
        if let PipelineEvent::StateTransition { from, to, .. } = deserialized {
            assert_eq!(from, PipelineState::Idle);
            assert_eq!(to, PipelineState::InputReady);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_invalid_state_transition_logic() {
        // This tests the domain model's ability to represent a transition,
        // not the runner's validation (which comes in Phase 02).
        let from = PipelineState::Idle;
        let to = PipelineState::Verified;

        let event = PipelineEvent::new_transition(from, to);
        if let PipelineEvent::StateTransition { from: f, to: t, .. } = event {
            assert_eq!(f, PipelineState::Idle);
            assert_eq!(t, PipelineState::Verified);
        }
    }
}
