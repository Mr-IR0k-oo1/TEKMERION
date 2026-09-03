use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct FaceDetection {
    pub bounding_box: [f32; 4], // [x, y, w, h]
    pub confidence: f32,
    pub quality: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FaceEmbedding {
    pub vector: Vec<f32>,
    pub normalized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceAnalysis {
    pub detections: Vec<FaceDetection>,
    pub embeddings: Vec<FaceEmbedding>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCandidate {
    pub url: Url,
    pub title: Option<String>,
    pub provider: String,
    pub image_url: Option<Url>,
    pub snippet: Option<String>,
    pub discovered_at: DateTime<Utc>,
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineResult {
    pub final_state: super::state::PipelineState,
    pub evidence: Option<EvidenceBundle>,
    pub blockchain: Option<BlockchainRecord>,
    pub error: Option<String>,
}
