//! Test provider implementation strictly for unit testing.
//!
//! # Safety & Isolation Invariant
//!
//! This module is compiled exclusively for unit tests (`#[cfg(test)]`).
//! The production discovery engine and production providers MUST NEVER
//! instantiate or use this mock test provider.

#[cfg(test)]
pub mod mock {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use tekmerion_core::FaceAnalysis;
    use tokio::sync::Mutex;

    use crate::error::DiscoveryError;
    use crate::provider::{DiscoveryProvider, RawCandidate};

    type MockResponses = Arc<Mutex<Vec<Result<Vec<RawCandidate>, DiscoveryError>>>>;

    /// Configurable mock provider for hermetic testing of engine behavior.
    pub struct MockDiscoveryProvider {
        id: String,
        responses: MockResponses,
        delay: Option<Duration>,
        call_count: Arc<AtomicUsize>,
    }

    impl MockDiscoveryProvider {
        /// Create a mock provider that returns a single successful response.
        pub fn new(id: impl Into<String>, candidates: Vec<RawCandidate>) -> Self {
            Self {
                id: id.into(),
                responses: Arc::new(Mutex::new(vec![Ok(candidates)])),
                delay: None,
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        /// Create a mock provider with an intentional delay (for testing timeouts).
        pub fn with_delay(
            id: impl Into<String>,
            delay: Duration,
            candidates: Vec<RawCandidate>,
        ) -> Self {
            Self {
                id: id.into(),
                responses: Arc::new(Mutex::new(vec![Ok(candidates)])),
                delay: Some(delay),
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        /// Create a mock provider that returns an error.
        pub fn with_error(id: impl Into<String>, error: DiscoveryError) -> Self {
            Self {
                id: id.into(),
                responses: Arc::new(Mutex::new(vec![Err(error)])),
                delay: None,
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        /// Create a mock provider with a sequence of responses (e.g. for testing retries).
        pub fn with_sequence(
            id: impl Into<String>,
            responses: Vec<Result<Vec<RawCandidate>, DiscoveryError>>,
        ) -> Self {
            Self {
                id: id.into(),
                responses: Arc::new(Mutex::new(responses)),
                delay: None,
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        /// Number of times this provider's `search` method was invoked.
        pub fn calls(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl DiscoveryProvider for MockDiscoveryProvider {
        fn id(&self) -> &str {
            &self.id
        }

        async fn search(
            &self,
            _analysis: &FaceAnalysis,
        ) -> Result<Vec<RawCandidate>, DiscoveryError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);

            if let Some(delay) = self.delay {
                tokio::time::sleep(delay).await;
            }

            let mut responses = self.responses.lock().await;
            if responses.is_empty() {
                Ok(Vec::new())
            } else if responses.len() == 1 {
                // If only one response remains, continue returning it
                responses[0].clone()
            } else {
                responses.remove(0)
            }
        }
    }
}
