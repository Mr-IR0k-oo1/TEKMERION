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
    pub quality: f32,
    pub matched_face_index: Option<usize>,
    pub candidate_image_hash: Option<String>,
    pub status: VerificationStatus,
    #[serde(default)]
    pub error_message: Option<String>,
}

impl VerificationResult {
    pub fn new(
        candidate: SearchCandidate,
        similarity: f32,
        quality: f32,
        matched_face_index: Option<usize>,
        candidate_image_hash: Option<String>,
        status: VerificationStatus,
    ) -> Self {
        Self {
            candidate,
            similarity,
            quality,
            matched_face_index,
            candidate_image_hash,
            status,
            error_message: None,
        }
    }

    pub fn with_error(candidate: SearchCandidate, error_message: impl Into<String>) -> Self {
        Self {
            candidate,
            similarity: 0.0,
            quality: 0.0,
            matched_face_index: None,
            candidate_image_hash: None,
            status: VerificationStatus::Error,
            error_message: Some(error_message.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    NoFace,
    Verified,
    BelowThreshold,
    Error,
}

impl VerificationStatus {
    pub fn label(self) -> &'static str {
        match self {
            VerificationStatus::NoFace => "No Face",
            VerificationStatus::Verified => "Verified",
            VerificationStatus::BelowThreshold => "Below Threshold",
            VerificationStatus::Error => "Error",
        }
    }

    pub fn is_verified(self) -> bool {
        matches!(self, VerificationStatus::Verified)
    }
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub schema_version: String,
    pub run_id: String,
    pub source_url: Url,
    pub domain: String,
    pub platform: String,
    pub provider: String,
    pub retrieved_at: DateTime<Utc>,
    pub title: String,
    pub text: String,
    pub image_sha256: String,
    pub face_similarity: f32,
    pub face_model: String,
    pub candidate_quality: f32,
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
        assert_ne!(VerificationStatus::Verified, VerificationStatus::BelowThreshold);
        assert_eq!(VerificationStatus::NoFace, VerificationStatus::NoFace);
        assert_eq!(VerificationStatus::Verified.label(), "Verified");
        assert!(VerificationStatus::Verified.is_verified());
        assert!(!VerificationStatus::BelowThreshold.is_verified());
    }
}

