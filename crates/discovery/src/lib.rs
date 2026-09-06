//! TEKMERION candidate discovery abstraction.
//!
//! Provides the discovery engine, provider interface, URL validation,
//! result normalization, deduplication, deterministic ordering, retry policies,
//! timeout enforcement, and caching for reverse-image search and candidate discovery.

pub mod cache;
pub mod engine;
pub mod error;
pub mod external;
pub mod normalizer;
pub mod provider;
pub mod retry;

#[cfg(test)]
pub mod test_provider;

pub use cache::{DiscoveryCache, MemoryCache, NoopCache};
pub use engine::{DiscoveryEngine, DiscoveryEngineConfig};
pub use error::DiscoveryError;
pub use external::{
    extract_candidates_from_response, redact_secrets, ExternalReverseImageConfig,
    ExternalReverseImageProvider, DEFAULT_ENDPOINT, DEFAULT_IMAGE_FIELD, DEFAULT_PROVIDER_NAME,
    DEFAULT_TIMEOUT_SECONDS,
};
pub use normalizer::{
    normalize_candidate, normalize_domain, process_candidates, validate_and_normalize_url,
};
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
            image_path: None,
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

        let mock_provider = Arc::new(MockDiscoveryProvider::new("mock_search", vec![raw1, raw2]));

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
        assert_eq!(
            candidates[0].url.as_str(),
            "https://example.com/profile/123"
        );

        // Stripped port 80 for http
        assert_eq!(candidates[1].domain, "photos.example.org");
        assert_eq!(
            candidates[1].url.as_str(),
            "http://photos.example.org/sample"
        );
    }

    #[tokio::test]
    async fn engine_deduplicates_across_providers() {
        let raw_a = RawCandidate::new("https://example.com/person/1").with_title("First Title");
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
        assert_eq!(
            candidates[0].snippet.as_deref(),
            Some("Complementary snippet")
        );
        assert!(candidates[0].image_url.is_some());
    }

    #[tokio::test]
    async fn engine_enforces_candidate_limit() {
        let mut raw_list = Vec::new();
        for i in 0..20 {
            raw_list.push(RawCandidate::new(format!(
                "https://example.com/entry/{:02}",
                i
            )));
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

        let provider = Arc::new(MockDiscoveryProvider::new(
            "order_test",
            vec![raw_c, raw_a, raw_b],
        ));
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

        assert!(
            matches!(err, DiscoveryError::Timeout { provider, .. } if provider == "slow_provider")
        );
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

        let engine =
            DiscoveryEngine::new(vec![provider.clone()], Arc::new(NoopCache), config).unwrap();
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

        let engine =
            DiscoveryEngine::new(vec![provider.clone()], Arc::new(NoopCache), config).unwrap();
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

    async fn run_one_shot_server(
        status_code: u16,
        status_text: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (String, tokio::sync::oneshot::Receiver<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://127.0.0.1:{}/search", addr.port());
        let (tx, rx) = tokio::sync::oneshot::channel();
        let body_owned = body.to_string();
        let status_text_owned = status_text.to_string();
        let headers_owned: Vec<(String, String)> = headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = vec![0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let _ = tx.send(req);

                let mut resp = format!(
                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n",
                    status_code,
                    status_text_owned,
                    body_owned.len()
                );
                for (k, v) in headers_owned {
                    resp.push_str(&format!("{}: {}\r\n", k, v));
                }
                resp.push_str("\r\n");
                resp.push_str(&body_owned);
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.flush().await;
            }
        });

        (url, rx)
    }

    #[tokio::test]
    async fn external_provider_uploads_real_image_and_parses_candidates() {
        use crate::external::{ExternalReverseImageConfig, ExternalReverseImageProvider};
        use crate::provider::DiscoveryProvider;
        use url::Url;

        let response_body = serde_json::json!({
            "visual_matches": [
                {
                    "link": "https://example.com/target-profile",
                    "title": "Jane Doe Public Page",
                    "source": "example.com",
                    "thumbnail": "https://example.com/thumb.jpg",
                    "image": "https://example.com/full.jpg",
                    "snippet": "Verified portrait photo"
                }
            ]
        })
        .to_string();

        let (endpoint_str, req_rx) = run_one_shot_server(200, "OK", &[], &response_body).await;

        // Write a synthetic JPEG file for testing actual upload
        let temp_dir = std::env::temp_dir();
        let image_file = temp_dir.join(format!(
            "test_face_{}.jpg",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let dummy_jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
        tokio::fs::write(&image_file, &dummy_jpeg).await.unwrap();

        let mut analysis = sample_analysis();
        analysis.image_path = Some(image_file.to_str().unwrap().to_string());

        let config = ExternalReverseImageConfig::new(
            "super_secret_test_key",
            Url::parse(&endpoint_str).unwrap(),
            "real_external_provider",
            Duration::from_secs(5),
            "image",
        );

        let provider = ExternalReverseImageProvider::new(config).unwrap();
        assert_eq!(provider.id(), "real_external_provider");

        let candidates = provider.search(&analysis).await.unwrap();

        // Check server received the auth headers and multipart payload
        let req_text = req_rx.await.unwrap();
        assert!(req_text.contains("authorization: Bearer super_secret_test_key"));
        assert!(req_text.contains("x-api-key: super_secret_test_key"));
        assert!(req_text.contains("multipart/form-data"));
        assert!(req_text.contains("name=\"image\""));

        // Verify candidates were dynamically extracted
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url, "https://example.com/target-profile");
        assert_eq!(candidates[0].title.as_deref(), Some("Jane Doe Public Page"));
        assert_eq!(candidates[0].domain.as_deref(), Some("example.com"));
        assert_eq!(
            candidates[0].image_url.as_deref(),
            Some("https://example.com/full.jpg")
        );
        assert_eq!(
            candidates[0].thumbnail_url.as_deref(),
            Some("https://example.com/thumb.jpg")
        );
        assert_eq!(
            candidates[0].snippet.as_deref(),
            Some("Verified portrait photo")
        );

        // Clean up temp file
        let _ = tokio::fs::remove_file(&image_file).await;
    }

    #[tokio::test]
    async fn external_provider_handles_rate_limit_429() {
        use crate::external::{ExternalReverseImageConfig, ExternalReverseImageProvider};
        use crate::provider::DiscoveryProvider;
        use url::Url;

        let (endpoint_str, _req_rx) = run_one_shot_server(
            429,
            "Too Many Requests",
            &[("Retry-After", "45")],
            r#"{"error":"rate limit exceeded"}"#,
        )
        .await;

        let temp_dir = std::env::temp_dir();
        let image_file = temp_dir.join(format!(
            "test_face_{}.jpg",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        tokio::fs::write(&image_file, b"test").await.unwrap();

        let mut analysis = sample_analysis();
        analysis.image_path = Some(image_file.to_str().unwrap().to_string());

        let config = ExternalReverseImageConfig::new(
            "key123",
            Url::parse(&endpoint_str).unwrap(),
            "rate_limited_provider",
            Duration::from_secs(5),
            "image",
        );

        let provider = ExternalReverseImageProvider::new(config).unwrap();
        let err = provider.search(&analysis).await.unwrap_err();

        match err {
            DiscoveryError::RateLimited {
                provider,
                retry_after_secs,
            } => {
                assert_eq!(provider, "rate_limited_provider");
                assert_eq!(retry_after_secs, Some(45));
            }
            other => panic!("expected RateLimited error, got: {:?}", other),
        }

        let _ = tokio::fs::remove_file(&image_file).await;
    }

    #[tokio::test]
    async fn external_provider_handles_auth_failure_without_leaking_secrets() {
        use crate::external::{ExternalReverseImageConfig, ExternalReverseImageProvider};
        use crate::provider::DiscoveryProvider;
        use url::Url;

        let secret = "classified_api_token_xyz987";
        let (endpoint_str, _req_rx) = run_one_shot_server(
            401,
            "Unauthorized",
            &[],
            r#"{"error":"invalid credentials"}"#,
        )
        .await;

        let temp_dir = std::env::temp_dir();
        let image_file = temp_dir.join(format!(
            "test_face_{}.jpg",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        tokio::fs::write(&image_file, b"test").await.unwrap();

        let mut analysis = sample_analysis();
        analysis.image_path = Some(image_file.to_str().unwrap().to_string());

        let config = ExternalReverseImageConfig::new(
            secret,
            Url::parse(&endpoint_str).unwrap(),
            "auth_prov",
            Duration::from_secs(5),
            "image",
        );

        let provider = ExternalReverseImageProvider::new(config).unwrap();
        let err = provider.search(&analysis).await.unwrap_err();

        let err_msg = err.to_string();
        assert!(
            !err_msg.contains(secret),
            "Secret must never leak in error message"
        );
        assert!(err_msg.contains("Authentication failed"));

        let _ = tokio::fs::remove_file(&image_file).await;
    }

    #[tokio::test]
    async fn external_provider_rejects_missing_image_file() {
        use crate::external::{ExternalReverseImageConfig, ExternalReverseImageProvider};
        use crate::provider::DiscoveryProvider;
        use url::Url;

        let config = ExternalReverseImageConfig::new(
            "dummy_key",
            Url::parse("http://127.0.0.1:9999/search").unwrap(),
            "file_check_prov",
            Duration::from_secs(5),
            "image",
        );

        let provider = ExternalReverseImageProvider::new(config).unwrap();

        let mut analysis = sample_analysis();
        analysis.image_path = Some("nonexistent_path_to_face_image_123456.jpg".to_string());

        let err = provider.search(&analysis).await.unwrap_err();
        assert!(matches!(err, DiscoveryError::Provider { .. }));
        assert!(err.to_string().contains("does not exist"));
    }
}
