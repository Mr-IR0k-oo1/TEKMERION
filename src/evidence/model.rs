//! Evidence model definitions

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Represents evidence of a discovered web content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceRecord {
    /// Version of the evidence format
    pub version: String,
    /// URL where the content was discovered
    pub source_url: String,
    /// Platform where the content was discovered
    pub platform: String,
    /// Title of the content
    pub title: String,
    /// Text content
    pub text: String,
    /// SHA-256 hash of the downloaded image
    pub image_sha256: String,
    /// When the content was discovered
    pub discovered_at: DateTime<Utc>,
    /// Face similarity score
    pub face_similarity: f32,
}
