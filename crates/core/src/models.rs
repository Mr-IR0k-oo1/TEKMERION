use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::state::PipelineState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetection {
    pub bounding_box: [f32; 4],
    pub confidence: f32,
    pub quality: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceEmbedding {
    pub vector: Vec<f32>,
    pub normalized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceAnalysis {
    pub detections: Vec<FaceDetection>,
    pub embeddings: Vec<FaceEmbedding>,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub image_path: Option<String>,
}

impl FaceAnalysis {
    pub fn new(detections: Vec<FaceDetection>, embeddings: Vec<FaceEmbedding>) -> Self {
        Self {
            detections,
            embeddings,
            timestamp: Utc::now(),
            image_path: None,
        }
    }

    pub fn with_image_path(mut self, path: impl Into<String>) -> Self {
        self.image_path = Some(path.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchCandidate {
    pub url: Url,
    pub title: Option<String>,
    pub domain: String,
    pub image_url: Option<Url>,
    pub thumbnail_url: Option<Url>,
    pub snippet: Option<String>,
    pub provider: String,
    pub discovered_at: DateTime<Utc>,
}

impl SearchCandidate {
    /// Create a candidate with automatically extracted domain.
    pub fn new(url: Url, provider: impl Into<String>) -> Self {
        let domain = url
            .host_str()
            .unwrap_or_default()
            .trim_start_matches("www.")
            .to_lowercase();
        Self {
            url,
            title: None,
            domain,
            image_url: None,
            thumbnail_url: None,
            snippet: None,
            provider: provider.into(),
            discovered_at: Utc::now(),
        }
    }

    /// Accessor for the candidate's source/domain.
    pub fn source(&self) -> &str {
        &self.domain
    }

    /// Accessor for the candidate's source/domain.
    pub fn source_domain(&self) -> &str {
        &self.domain
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub candidate: SearchCandidate,
    pub similarity: f32,
    pub status: VerificationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    Match,
    Review,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub source_url: Url,
    pub provider: String,
    pub timestamp: DateTime<Utc>,
    pub content_hash: String,
    pub face_similarity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub record: EvidenceRecord,
    pub root_hash: String,
    pub leaf_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainRecord {
    pub tx_hash: String,
    pub block_number: u64,
    pub registered_root: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub final_state: PipelineState,
    pub evidence: Option<EvidenceBundle>,
    pub blockchain: Option<BlockchainRecord>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_candidate_round_trip() {
        let candidate = SearchCandidate {
            url: Url::parse("https://example.com").unwrap(),
            title: Some("Test".to_string()),
            domain: "example.com".to_string(),
            provider: "Google".to_string(),
            image_url: None,
            thumbnail_url: None,
            snippet: None,
            discovered_at: Utc::now(),
        };

        let json = serde_json::to_string(&candidate).unwrap();
        let deserialized: SearchCandidate = serde_json::from_str(&json).unwrap();

        assert_eq!(candidate.url, deserialized.url);
        assert_eq!(candidate.domain, deserialized.domain);
        assert_eq!(candidate.provider, deserialized.provider);
    }

    #[test]
    fn verification_status_variants() {
        assert_ne!(VerificationStatus::Match, VerificationStatus::Reject);
        assert_eq!(VerificationStatus::Review, VerificationStatus::Review);
    }
}
