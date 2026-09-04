//! TEKMERION candidate discovery abstraction.
//!
//! Provides the discovery engine, provider interface, URL validation,
//! result normalization, deduplication, deterministic ordering, retry policies,
//! timeout enforcement, and caching for reverse-image search and candidate discovery.

pub mod cache;
pub mod engine;
pub mod error;
pub mod normalizer;
pub mod provider;
pub mod retry;

#[cfg(test)]
pub mod test_provider;

pub use cache::{DiscoveryCache, MemoryCache, NoopCache};
pub use engine::{DiscoveryEngine, DiscoveryEngineConfig};
pub use error::DiscoveryError;
pub use normalizer::{normalize_candidate, normalize_domain, process_candidates, validate_and_normalize_url};
pub use provider::{DiscoveryProvider, RawCandidate};
pub use retry::RetryPolicy;
pub use tekmerion_core::SearchCandidate;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use chrono::Utc;
    use tekmerion_core::{FaceAnalysis, FaceDetection, FaceEmbedding};

    use super::cache::{MemoryCache, NoopCache};
    use super::engine::{DiscoveryEngine, DiscoveryEngineConfig};
    use super::error::DiscoveryError;
    use super::provider::RawCandidate;
    use super::retry::RetryPolicy;
    use super::test_provider::mock::MockDiscoveryProvider;

    fn sample_analysis() -> FaceAnalysis {
        FaceAnalysis {
            detections: vec![FaceDetection {
                bounding_box: [10.0, 10.0, 100.0, 100.0],
                confidence: 0.95,
                quality: 0.90,
            }],
            embeddings: vec![FaceEmbedding {
                vector: vec![0.1, 0.2, 0.3],
                normalized: true,
            }],
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn engine_discovers_and_normalizes_candidates() {
        let raw1 = RawCandidate::new("https://www.example.com/profile/123#frag")
            .with_title("  Jane Doe  ")
            .with_snippet("  Bio info  ")
            .with_image_url("https://images.example.com/face.jpg");

        let raw2 = RawCandidate::new("http://photos.example.org:80/sample")
            .with_title("Sample Photo")
            .with_domain("photos.example.org");

        let mock_provider = Arc::new(MockDiscoveryProvider::new(
            "mock_search",
            vec![raw1, raw2],
        ));

        let engine = DiscoveryEngine::new(
            vec![mock_provider.clone()],
            Arc::new(NoopCache),
            DiscoveryEngineConfig::default(),
        )
        .unwrap();

        let candidates = engine.discover(&sample_analysis()).await.unwrap();

        assert_eq!(candidates.len(), 2);
        assert_eq!(mock_provider.calls(), 1);

        // Provider attribution
        assert_eq!(candidates[0].provider, "mock_search");
        assert_eq!(candidates[1].provider, "mock_search");

        // Normalization: domain, stripped www, trimmed whitespace
        assert_eq!(candidates[0].domain, "example.com");
        assert_eq!(candidates[0].title.as_deref(), Some("Jane Doe"));
        assert_eq!(candidates[0].snippet.as_deref(), Some("Bio info"));
        assert_eq!(
            candidates[0].image_url.as_ref().unwrap().as_str(),
            "https://images.example.com/face.jpg"
        );
        // Stripped fragment
        assert_eq!(candidates[0].url.as_str(), "https://example.com/profile/123");

        // Stripped port 80 for http
        assert_eq!(candidates[1].domain, "photos.example.org");
        assert_eq!(candidates[1].url.as_str(), "http://photos.example.org/sample");
    }

    #[tokio::test]
    async fn engine_deduplicates_across_providers() {
        let raw_a = RawCandidate::new("https://example.com/person/1")
            .with_title("First Title");
        let raw_b = RawCandidate::new("https://example.com/person/1")
            .with_snippet("Complementary snippet")
            .with_image_url("https://example.com/img.jpg");

        let provider_a = Arc::new(MockDiscoveryProvider::new("prov_a", vec![raw_a]));
        let provider_b = Arc::new(MockDiscoveryProvider::new("prov_b", vec![raw_b]));

        let engine = DiscoveryEngine::new(
            vec![provider_a, provider_b],
            Arc::new(NoopCache),
            DiscoveryEngineConfig::default(),
        )
        .unwrap();

        let candidates = engine.discover(&sample_analysis()).await.unwrap();

        // Must be deduplicated into 1 candidate with merged metadata
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].title.as_deref(), Some("First Title"));
        assert_eq!(candidates[0].snippet.as_deref(), Some("Complementary snippet"));
        assert!(candidates[0].image_url.is_some());
    }

    #[tokio::test]
    async fn engine_enforces_candidate_limit() {
        let mut raw_list = Vec::new();
        for i in 0..20 {
            raw_list.push(RawCandidate::new(format!("https://example.com/entry/{:02}", i)));
        }

        let provider = Arc::new(MockDiscoveryProvider::new("bulk", raw_list));
        let config = DiscoveryEngineConfig::default().with_max_candidates(7);

        let engine = DiscoveryEngine::new(vec![provider], Arc::new(NoopCache), config).unwrap();
        let candidates = engine.discover(&sample_analysis()).await.unwrap();

        assert_eq!(candidates.len(), 7);
    }

    #[tokio::test]
    async fn engine_deterministic_ordering_is_stable() {
        let raw_c = RawCandidate::new("https://c.example.net/page").with_domain("c.example.net");
        let raw_a = RawCandidate::new("https://a.example.net/page").with_domain("a.example.net");
        let raw_b = RawCandidate::new("https://b.example.net/page").with_domain("b.example.net");

        let provider = Arc::new(MockDiscoveryProvider::new("order_test", vec![raw_c, raw_a, raw_b]));
        let engine = DiscoveryEngine::new(
            vec![provider],
            Arc::new(NoopCache),
            DiscoveryEngineConfig::default(),
        )
        .unwrap();

        let candidates = engine.discover(&sample_analysis()).await.unwrap();
        assert_eq!(candidates[0].domain, "a.example.net");
        assert_eq!(candidates[1].domain, "b.example.net");
        assert_eq!(candidates[2].domain, "c.example.net");
    }

    #[tokio::test]
    async fn engine_enforces_timeout_and_returns_structured_error() {
        let provider = Arc::new(MockDiscoveryProvider::with_delay(
            "slow_provider",
            Duration::from_millis(250),
            vec![RawCandidate::new("https://example.com/late")],
        ));

        let config = DiscoveryEngineConfig::default()
            .with_timeout(Duration::from_millis(50))
            .with_retry_policy(RetryPolicy::none());

        let engine = DiscoveryEngine::new(vec![provider], Arc::new(NoopCache), config).unwrap();
        let err = engine.discover(&sample_analysis()).await.unwrap_err();

        assert!(matches!(err, DiscoveryError::Timeout { provider, .. } if provider == "slow_provider"));
    }

    #[tokio::test]
    async fn engine_retries_transient_error_and_succeeds() {
        let sequence = vec![
            Err(DiscoveryError::RateLimited {
                provider: "flaky".to_string(),
                retry_after_secs: Some(1),
            }),
            Ok(vec![RawCandidate::new("https://example.com/recovered")]),
        ];

        let provider = Arc::new(MockDiscoveryProvider::with_sequence("flaky", sequence));

        let config = DiscoveryEngineConfig::default().with_retry_policy(RetryPolicy {
            max_retries: 2,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(50),
            backoff_factor: 1.5,
        });

        let engine = DiscoveryEngine::new(vec![provider.clone()], Arc::new(NoopCache), config).unwrap();
        let candidates = engine.discover(&sample_analysis()).await.unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url.as_str(), "https://example.com/recovered");
        assert_eq!(provider.calls(), 2);
    }

    #[tokio::test]
    async fn engine_fails_fast_on_non_transient_error() {
        let provider = Arc::new(MockDiscoveryProvider::with_error(
            "fatal_prov",
            DiscoveryError::Config("missing api credential".to_string()),
        ));

        let config = DiscoveryEngineConfig::default().with_retry_policy(RetryPolicy {
            max_retries: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(50),
            backoff_factor: 2.0,
        });

        let engine = DiscoveryEngine::new(vec![provider.clone()], Arc::new(NoopCache), config).unwrap();
        let err = engine.discover(&sample_analysis()).await.unwrap_err();

        assert!(matches!(err, DiscoveryError::Config(_)));
        // Non-transient should fail immediately without retries (1 call only)
        assert_eq!(provider.calls(), 1);
    }

    #[tokio::test]
    async fn engine_uses_cache_on_subsequent_calls() {
        let raw = RawCandidate::new("https://example.com/cached_hit");
        let provider = Arc::new(MockDiscoveryProvider::new("cache_test", vec![raw]));

        let cache = Arc::new(MemoryCache::new());
        let engine = DiscoveryEngine::new(
            vec![provider.clone()],
            cache,
            DiscoveryEngineConfig::default(),
        )
        .unwrap();

        let analysis = sample_analysis();

        // First call: cache miss, provider invoked
        let res1 = engine.discover(&analysis).await.unwrap();
        assert_eq!(res1.len(), 1);
        assert_eq!(provider.calls(), 1);

        // Second call: cache hit, provider NOT invoked again
        let res2 = engine.discover(&analysis).await.unwrap();
        assert_eq!(res2.len(), 1);
        assert_eq!(provider.calls(), 1);
    }

    #[test]
    fn production_engine_does_not_default_to_test_provider() {
        let engine = DiscoveryEngine::empty();
        assert_eq!(engine.provider_count(), 0);
    }
}
