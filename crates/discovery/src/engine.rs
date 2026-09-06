//! Discovery engine orchestrating providers, normalization, caching, retries, and deduplication.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tekmerion_core::{FaceAnalysis, SearchCandidate};

use crate::cache::{DiscoveryCache, NoopCache};
use crate::error::DiscoveryError;
use crate::normalizer::{normalize_candidate, process_candidates};
use crate::provider::DiscoveryProvider;
use crate::retry::RetryPolicy;

/// Configuration for the discovery engine.
#[derive(Debug, Clone)]
pub struct DiscoveryEngineConfig {
    /// Maximum time allowed for a single provider attempt.
    pub timeout: Duration,
    /// Maximum number of candidates returned after deduplication.
    pub max_candidates: usize,
    /// Retry policy for transient failures.
    pub retry_policy: RetryPolicy,
    /// Time-to-live for cache entries.
    pub cache_ttl: Duration,
}

impl Default for DiscoveryEngineConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            max_candidates: 50,
            retry_policy: RetryPolicy::default(),
            cache_ttl: Duration::from_secs(3600),
        }
    }
}

impl DiscoveryEngineConfig {
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_candidates(mut self, max: usize) -> Self {
        self.max_candidates = max;
        self
    }

    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }
}

/// The core discovery engine.
///
/// Orchestrates discovery across configured providers with URL validation,
/// result normalization, deduplication, deterministic ordering, timeout enforcement,
/// retry policies, and caching.
#[derive(Clone)]
pub struct DiscoveryEngine {
    providers: Vec<Arc<dyn DiscoveryProvider>>,
    cache: Arc<dyn DiscoveryCache>,
    config: DiscoveryEngineConfig,
}

impl DiscoveryEngine {
    /// Construct a discovery engine with explicit providers and cache.
    ///
    /// Note: The production engine must be given production providers; it will
    /// never instantiate or default to a mock test provider.
    pub fn new(
        providers: Vec<Arc<dyn DiscoveryProvider>>,
        cache: Arc<dyn DiscoveryCache>,
        config: DiscoveryEngineConfig,
    ) -> Result<Self, DiscoveryError> {
        if config.max_candidates == 0 {
            return Err(DiscoveryError::Config(
                "max_candidates must be greater than zero".to_string(),
            ));
        }
        Ok(Self {
            providers,
            cache,
            config,
        })
    }

    /// Construct a default discovery engine with no active providers and noop cache.
    pub fn empty() -> Self {
        Self {
            providers: Vec::new(),
            cache: Arc::new(NoopCache),
            config: DiscoveryEngineConfig::default(),
        }
    }

    /// Number of configured providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Execute candidate search across all configured providers.
    pub async fn discover(
        &self,
        analysis: &FaceAnalysis,
    ) -> Result<Vec<SearchCandidate>, DiscoveryError> {
        if self.providers.is_empty() {
            return Ok(Vec::new());
        }

        let cache_key = cache_key_from_analysis(analysis);

        // 1. Check cache
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached);
        }

        // 2. Query each provider with timeout and retry
        let mut all_candidates = Vec::new();
        let mut last_error: Option<DiscoveryError> = None;
        let mut any_success = false;

        for provider in &self.providers {
            match self
                .query_provider_with_retry(provider.as_ref(), analysis)
                .await
            {
                Ok(raw_list) => {
                    any_success = true;
                    for raw in raw_list {
                        match normalize_candidate(raw, provider.id()) {
                            Ok(candidate) => all_candidates.push(candidate),
                            Err(e) => {
                                // Invalid candidate URL is skipped rather than aborting whole search
                                tracing::debug!("skipping invalid candidate: {}", e);
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!("provider '{}' failed: {}", provider.id(), err);
                    last_error = Some(err);
                }
            }
        }

        // If all providers failed and no candidates were found, propagate the error
        if !any_success && !self.providers.is_empty() {
            if let Some(err) = last_error {
                return Err(err);
            }
        }

        // 3. Deduplicate, deterministically sort, and limit candidates
        let final_candidates = process_candidates(all_candidates, self.config.max_candidates);

        // 4. Store in cache
        self.cache
            .set(&cache_key, final_candidates.clone(), self.config.cache_ttl)
            .await;

        Ok(final_candidates)
    }

    /// Query a single provider honoring the engine's timeout and retry policy.
    async fn query_provider_with_retry(
        &self,
        provider: &dyn DiscoveryProvider,
        analysis: &FaceAnalysis,
    ) -> Result<Vec<crate::provider::RawCandidate>, DiscoveryError> {
        let max_attempts = 1 + self.config.retry_policy.max_retries;
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            if attempt > 1 {
                let delay = self.config.retry_policy.delay_for_attempt(attempt - 1);
                tokio::time::sleep(delay).await;
            }

            let timeout_dur = self.config.timeout;
            let result = tokio::time::timeout(timeout_dur, provider.search(analysis)).await;

            match result {
                Ok(Ok(candidates)) => return Ok(candidates),
                Ok(Err(err)) => {
                    if !self.config.retry_policy.is_transient(&err) || attempt == max_attempts {
                        return Err(err);
                    }
                    last_error = Some(err);
                }
                Err(_) => {
                    let timeout_err = DiscoveryError::Timeout {
                        provider: provider.id().to_string(),
                        timeout_ms: timeout_dur.as_millis() as u64,
                    };
                    if attempt == max_attempts {
                        return Err(timeout_err);
                    }
                    last_error = Some(timeout_err);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| DiscoveryError::Internal("retries exhausted".to_string())))
    }
}

/// Derive a deterministic cache key from face embeddings.
fn cache_key_from_analysis(analysis: &FaceAnalysis) -> String {
    if let Some(emb) = analysis.embeddings.first() {
        let mut hasher = DefaultHasher::new();
        for &val in &emb.vector {
            val.to_bits().hash(&mut hasher);
        }
        format!("face-emb-{:016x}", hasher.finish())
    } else {
        "face-empty".to_string()
    }
}

#[async_trait]
impl tekmerion_core::DiscoveryEngine for DiscoveryEngine {
    async fn discover(
        &self,
        analysis: &FaceAnalysis,
    ) -> Result<Vec<SearchCandidate>, tekmerion_core::PipelineError> {
        self.discover(analysis).await.map_err(Into::into)
    }
}
