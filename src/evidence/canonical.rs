//! Evidence canonicalization

use crate::evidence::model::EvidenceRecord;
use serde_json::to_vec;
use sha2::{Sha256, Digest};

/// Canonicalize evidence for hashing
pub fn canonicalize_evidence(evidence: &EvidenceRecord) -> Vec<u8> {
    // Serialize to JSON with consistent formatting
    let mut json_bytes = to_vec(&evidence).expect("Failed to serialize evidence");
    json_bytes.sort();
    json_bytes
}

/// Hash canonicalized evidence
pub fn hash_evidence(canonical_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_bytes);
    format!("{:x}", hasher.finalize())
}
