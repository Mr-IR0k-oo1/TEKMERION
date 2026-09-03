//! Evidence hashing utilities

use crate::evidence::model::EvidenceRecord;
use crate::evidence::canonical::{canonicalize_evidence, hash_evidence};
use sha2::{Sha256, Digest};

/// Generate evidence hashes
pub fn generate_evidence_hashes(
    evidence: &EvidenceRecord,
    image_bytes: &[u8],
) -> (String, String, String) {
    // Hash the image
    let mut image_hasher = Sha256::new();
    image_hasher.update(image_bytes);
    let image_hash = format!("{:x}", image_hasher.finalize());

    // Canonicalize and hash the evidence
    let canonical_bytes = canonicalize_evidence(evidence);
    let evidence_hash = hash_evidence(&canonical_bytes);

    (image_hash, evidence_hash, String::from_utf8(canonical_bytes).unwrap())
}
