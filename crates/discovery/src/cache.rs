//! Discovery cache interface and implementations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tekmerion_core::SearchCandidate;
use tokio::sync::RwLock;

/// Cache interface for storing and retrieving discovered search candidates.
#[async_trait]
pub trait DiscoveryCache: Send + Sync {
    /// Retrieve cached candidates for a query key if present and not expired.
    async fn get(&self, key: &str) -> Option<Vec<SearchCandidate>>;

    /// Store discovered candidates for a query key with a time-to-live.
    async fn set(&self, key: &str, candidates: Vec<SearchCandidate>, ttl: Duration);
}

/// No-op cache when caching is disabled.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopCache;

#[async_trait]
impl DiscoveryCache for NoopCache {
    async fn get(&self, _key: &str) -> Option<Vec<SearchCandidate>> {
        None
    }

    async fn set(&self, _key: &str, _candidates: Vec<SearchCandidate>, _ttl: Duration) {}
}

struct CacheEntry {
    candidates: Vec<SearchCandidate>,
    expires_at: Instant,
}

/// Thread-safe in-memory cache with TTL expiration.
#[derive(Clone)]
pub struct MemoryCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Number of items currently stored in the cache (including potentially expired ones).
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }

    /// Purge expired entries from the cache.
    pub async fn purge_expired(&self) {
        let now = Instant::now();
        let mut map = self.entries.write().await;
        map.retain(|_, entry| entry.expires_at > now);
    }
}

#[async_trait]
impl DiscoveryCache for MemoryCache {
    async fn get(&self, key: &str) -> Option<Vec<SearchCandidate>> {
        let now = Instant::now();
        let map = self.entries.read().await;
        if let Some(entry) = map.get(key) {
            if entry.expires_at > now {
                return Some(entry.candidates.clone());
            }
        }
        None
    }

    async fn set(&self, key: &str, candidates: Vec<SearchCandidate>, ttl: Duration) {
        let expires_at = Instant::now() + ttl;
        let entry = CacheEntry {
            candidates,
            expires_at,
        };
        self.entries.write().await.insert(key.to_string(), entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use url::Url;

    fn sample_candidate() -> SearchCandidate {
        SearchCandidate {
            url: Url::parse("https://example.com/test").unwrap(),
            title: Some("Test".to_string()),
            domain: "example.com".to_string(),
            image_url: None,
            thumbnail_url: None,
            snippet: None,
            provider: "mock".to_string(),
            discovered_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn memory_cache_stores_and_expires() {
        let cache = MemoryCache::new();
        let candidates = vec![sample_candidate()];

        cache
            .set("key1", candidates.clone(), Duration::from_millis(50))
            .await;

        // Immediate read hit
        let hit = cache.get("key1").await;
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().len(), 1);

        // Wait for TTL expiration
        tokio::time::sleep(Duration::from_millis(70)).await;
        let expired = cache.get("key1").await;
        assert!(expired.is_none());
    }

    #[tokio::test]
    async fn noop_cache_always_returns_none() {
        let cache = NoopCache;
        cache
            .set("key1", vec![sample_candidate()], Duration::from_secs(60))
            .await;
        assert!(cache.get("key1").await.is_none());
    }
}
