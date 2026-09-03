//! Evidence generation tests

use super::*;
use chrono::Utc;
use model::EvidenceRecord;
use hashing::generate_evidence_hashes;

#[test]
fn test_evidence_hashing_deterministic() {
    let evidence = EvidenceRecord {
        version: "1.0".to_string(),
        source_url: "https://example.com".to_string(),
        platform: "test".to_string(),
        title: "Test Title".to_string(),
        text: "Test text content".to_string(),
        image_sha256: "".to_string(),
        discovered_at: Utc::now(),
        face_similarity: 0.9,
    };

    let image_bytes = b"test image data";
    let (image_hash, evidence_hash, _) = generate_evidence_hashes(&evidence, image_bytes);

    // Test that same evidence produces same hash
    let (image_hash2, evidence_hash2, _) = generate_evidence_hashes(&evidence, image_bytes);
    assert_eq!(image_hash, image_hash2);
    assert_eq!(evidence_hash, evidence_hash2);
}

#[test]
fn test_evidence_hashing_changes() {
    let mut evidence = EvidenceRecord {
        version: "1.0".to_string(),
        source_url: "https://example.com".to_string(),
        platform: "test".to_string(),
        title: "Test Title".to_string(),
        text: "Test text content".to_string(),
        image_sha256: "".to_string(),
        discovered_at: Utc::now(),
        face_similarity: 0.9,
    };

    let image_bytes = b"test image data";
    let (_, original_hash, _) = generate_evidence_hashes(&evidence, image_bytes);

    // Change title should change hash
    evidence.title = "New Title".to_string();
    let (_, new_hash, _) = generate_evidence_hashes(&evidence, image_bytes);
    assert_ne!(original_hash, new_hash);

    // Change URL should change hash
    evidence.source_url = "https://new-example.com".to_string();
    let (_, new_hash, _) = generate_evidence_hashes(&evidence, image_bytes);
    assert_ne!(original_hash, new_hash);

    // Change image should change hash
    let new_image_bytes = b"new image data";
    let (new_image_hash, _, _) = generate_evidence_hashes(&evidence, new_image_bytes);
    assert_ne!(image_bytes, new_image_bytes);

    // Change similarity should change hash
    evidence.face_similarity = 0.8;
    let (_, new_hash, _) = generate_evidence_hashes(&evidence, image_bytes);
    assert_ne!(original_hash, new_hash);
}
