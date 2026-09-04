//! Discovery provider abstraction.
//!
//! Represents an upstream visual search or reverse-image search source.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tekmerion_core::FaceAnalysis;

use crate::error::DiscoveryError;

/// Raw search candidate emitted by a provider before validation, normalization, and deduplication.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawCandidate {
    /// Raw target webpage URL.
    pub url: String,
    /// Page title where available.
    #[serde(default)]
    pub title: Option<String>,
    /// Source domain if provided explicitly by upstream.
    #[serde(default)]
    pub domain: Option<String>,
    /// Full-resolution image URL where available.
    #[serde(default)]
    pub image_url: Option<String>,
    /// Image thumbnail URL where available.
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    /// Text snippet or caption associated with the match.
    #[serde(default)]
    pub snippet: Option<String>,
}

impl RawCandidate {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            title: None,
            domain: None,
            image_url: None,
            thumbnail_url: None,
            snippet: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    pub fn with_image_url(mut self, image_url: impl Into<String>) -> Self {
        self.image_url = Some(image_url.into());
        self
    }

    pub fn with_thumbnail_url(mut self, thumbnail_url: impl Into<String>) -> Self {
        self.thumbnail_url = Some(thumbnail_url.into());
        self
    }

    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }
}

/// Interface that discovery providers (e.g. reverse-image search services) must implement.
#[async_trait]
pub trait DiscoveryProvider: Send + Sync {
    /// Unique provider identifier (e.g. "google_lens", "bing_visual", "tineye").
    fn id(&self) -> &str;

    /// Execute candidate search for the given face analysis.
    async fn search(&self, analysis: &FaceAnalysis) -> Result<Vec<RawCandidate>, DiscoveryError>;
}
