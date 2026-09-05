//! TEKMERION deterministic evidence engine and canonicalization.
//!
//! Provides cryptographically sound, platform-independent, deterministic evidence
//! generation using SHA-256 digests over normalized candidate, facial verification,
//! and metadata structures.

pub mod engine;
pub mod error;
pub mod merkle;
pub mod record;

pub use engine::DeterministicEvidenceEngine;
pub use error::EvidenceError;
pub use merkle::{
    hash_leaf, hash_parent, recompute_root, recompute_root_from_proof, verify_proof,
    EvidenceBundle, EvidenceTree, EvidenceTreeLeaves, LeafType, MerkleProof, ProofDirection,
    LEAF_PREFIX, NODE_PREFIX,
};
pub use record::{
    format_float, normalize_url, normalize_utf8, EvidenceHashes, EvidenceRecord,
    CURRENT_SCHEMA_VERSION,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use url::Url;

    fn sample_record() -> EvidenceRecord {
        EvidenceRecord::new(
            "run-20260904-001",
            Url::parse("https://example.com/profiles/target?user=42&view=full#ignore").unwrap(),
            "example.com",
            "web",
            "reverse_image_search",
            Utc.with_ymd_and_hms(2026, 9, 4, 12, 0, 0).unwrap(),
            "Jane Doe - Public Profile",
            "High-resolution portrait found on public archive directory.",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            0.875231,
            "adaface-ir101",
            0.924105,
        )
    }

    #[test]
    fn test_same_record_produces_same_hash() {
        let rec1 = sample_record();
        let rec2 = sample_record();

        let hashes1 = rec1.compute_hashes().unwrap();
        let hashes2 = rec2.compute_hashes().unwrap();

        assert_eq!(hashes1.image_hash, hashes2.image_hash);
        assert_eq!(hashes1.content_hash, hashes2.content_hash);
        assert_eq!(hashes1.metadata_hash, hashes2.metadata_hash);
        assert_eq!(hashes1.face_result_hash, hashes2.face_result_hash);
        assert_eq!(hashes1.record_hash, hashes2.record_hash);

        // Byte serialization must also be byte-for-byte identical
        let bytes1 = rec1.canonical_bytes().unwrap();
        let bytes2 = rec2.canonical_bytes().unwrap();
        assert_eq!(bytes1, bytes2);

        // JSON serialization must be identical
        let json1 = rec1.canonical_json().unwrap();
        let json2 = rec2.canonical_json().unwrap();
        assert_eq!(json1, json2);
    }

    #[test]
    fn test_changed_title_produces_changed_hash() {
        let rec_orig = sample_record();
        let mut rec_mod = sample_record();
        rec_mod.title = "John Smith - Modified Profile".to_string();

        let orig_hashes = rec_orig.compute_hashes().unwrap();
        let mod_hashes = rec_mod.compute_hashes().unwrap();

        // Content hash and composite record hash must change
        assert_ne!(
            orig_hashes.content_hash, mod_hashes.content_hash,
            "changed title must produce changed content_hash"
        );
        assert_ne!(
            orig_hashes.record_hash, mod_hashes.record_hash,
            "changed title must produce changed record_hash"
        );

        // Other domain-separated hashes must remain unchanged
        assert_eq!(orig_hashes.image_hash, mod_hashes.image_hash);
        assert_eq!(orig_hashes.metadata_hash, mod_hashes.metadata_hash);
        assert_eq!(orig_hashes.face_result_hash, mod_hashes.face_result_hash);
    }

    #[test]
    fn test_changed_url_produces_changed_hash() {
        let rec_orig = sample_record();
        let mut rec_mod = sample_record();
        rec_mod.source_url = Url::parse("https://example.com/profiles/different_target").unwrap();

        let orig_hashes = rec_orig.compute_hashes().unwrap();
        let mod_hashes = rec_mod.compute_hashes().unwrap();

        // Metadata hash and composite record hash must change
        assert_ne!(
            orig_hashes.metadata_hash, mod_hashes.metadata_hash,
            "changed URL must produce changed metadata_hash"
        );
        assert_ne!(
            orig_hashes.record_hash, mod_hashes.record_hash,
            "changed URL must produce changed record_hash"
        );

        // Other domain-separated hashes must remain unchanged
        assert_eq!(orig_hashes.image_hash, mod_hashes.image_hash);
        assert_eq!(orig_hashes.content_hash, mod_hashes.content_hash);
        assert_eq!(orig_hashes.face_result_hash, mod_hashes.face_result_hash);
    }

    #[test]
    fn test_changed_image_produces_changed_hash() {
        let rec_orig = sample_record();
        let mut rec_mod = sample_record();
        rec_mod.image_sha256 = "1111111111111111111111111111111111111111111111111111111111111111".to_string();

        let orig_hashes = rec_orig.compute_hashes().unwrap();
        let mod_hashes = rec_mod.compute_hashes().unwrap();

        // Image hash and composite record hash must change
        assert_ne!(
            orig_hashes.image_hash, mod_hashes.image_hash,
            "changed image must produce changed image_hash"
        );
        assert_ne!(
            orig_hashes.record_hash, mod_hashes.record_hash,
            "changed image must produce changed record_hash"
        );

        // Other domain-separated hashes must remain unchanged
        assert_eq!(orig_hashes.content_hash, mod_hashes.content_hash);
        assert_eq!(orig_hashes.metadata_hash, mod_hashes.metadata_hash);
        assert_eq!(orig_hashes.face_result_hash, mod_hashes.face_result_hash);
    }

    #[test]
    fn test_changed_face_result_produces_changed_hash() {
        let rec_orig = sample_record();

        // Subtest 1: changed face similarity
        let mut rec_mod_sim = sample_record();
        rec_mod_sim.face_similarity = 0.950000;
        let hashes_sim = rec_mod_sim.compute_hashes().unwrap();
        let orig_hashes = rec_orig.compute_hashes().unwrap();

        assert_ne!(
            orig_hashes.face_result_hash, hashes_sim.face_result_hash,
            "changed face similarity must change face_result_hash"
        );
        assert_ne!(
            orig_hashes.record_hash, hashes_sim.record_hash,
            "changed face similarity must change record_hash"
        );
        assert_eq!(orig_hashes.image_hash, hashes_sim.image_hash);
        assert_eq!(orig_hashes.content_hash, hashes_sim.content_hash);
        assert_eq!(orig_hashes.metadata_hash, hashes_sim.metadata_hash);

        // Subtest 2: changed face model
        let mut rec_mod_model = sample_record();
        rec_mod_model.face_model = "facenet512".to_string();
        let hashes_model = rec_mod_model.compute_hashes().unwrap();

        assert_ne!(
            orig_hashes.face_result_hash, hashes_model.face_result_hash,
            "changed face model must change face_result_hash"
        );

        // Subtest 3: changed candidate quality
        let mut rec_mod_qual = sample_record();
        rec_mod_qual.candidate_quality = 0.500000;
        let hashes_qual = rec_mod_qual.compute_hashes().unwrap();

        assert_ne!(
            orig_hashes.face_result_hash, hashes_qual.face_result_hash,
            "changed candidate quality must change face_result_hash"
        );
    }

    #[test]
    fn test_utf8_normalization_precomposed_and_decomposed_produce_identical_hash() {
        let mut rec_precomposed = sample_record();
        // "café" with precomposed é (U+00E9)
        rec_precomposed.title = "caf\u{00E9}".to_string();

        let mut rec_decomposed = sample_record();
        // "café" with decomposed e + acute accent (U+0065 U+0301)
        rec_decomposed.title = "cafe\u{0301}".to_string();

        // Raw Rust strings have different byte lengths and contents
        assert_ne!(rec_precomposed.title.as_bytes(), rec_decomposed.title.as_bytes());

        let hashes1 = rec_precomposed.compute_hashes().unwrap();
        let hashes2 = rec_decomposed.compute_hashes().unwrap();

        // With UTF-8 NFC normalization, hashes must be identical
        assert_eq!(
            hashes1.content_hash, hashes2.content_hash,
            "Unicode normalization must produce identical content_hash"
        );
        assert_eq!(
            hashes1.record_hash, hashes2.record_hash,
            "Unicode normalization must produce identical record_hash"
        );
    }

    #[test]
    fn test_url_normalization_handles_default_ports_fragments_and_query_order() {
        let mut rec1 = sample_record();
        rec1.source_url = Url::parse("https://example.com:443/search?b=2&a=1#first_fragment").unwrap();

        let mut rec2 = sample_record();
        rec2.source_url = Url::parse("https://example.com/search?a=1&b=2#different_fragment").unwrap();

        let hashes1 = rec1.compute_hashes().unwrap();
        let hashes2 = rec2.compute_hashes().unwrap();

        assert_eq!(
            hashes1.metadata_hash, hashes2.metadata_hash,
            "URL normalization must produce identical metadata_hash for semantically identical URLs"
        );
        assert_eq!(hashes1.record_hash, hashes2.record_hash);
    }

    #[test]
    fn test_rejects_non_finite_float() {
        let mut rec_nan = sample_record();
        rec_nan.face_similarity = f32::NAN;
        assert!(matches!(
            rec_nan.compute_hashes(),
            Err(EvidenceError::NonFiniteFloat { field: "face_similarity", .. })
        ));

        let mut rec_inf = sample_record();
        rec_inf.candidate_quality = f32::INFINITY;
        assert!(matches!(
            rec_inf.compute_hashes(),
            Err(EvidenceError::NonFiniteFloat { field: "candidate_quality", .. })
        ));
    }

    #[test]
    fn test_schema_version_validation() {
        let mut rec = sample_record();
        assert!(rec.validate_schema().is_ok());

        rec.schema_version = "2.0.0".to_string();
        assert!(matches!(
            rec.validate_schema(),
            Err(EvidenceError::InvalidSchemaVersion { .. })
        ));
    }

    #[tokio::test]
    async fn test_deterministic_evidence_engine_builds_valid_bundle() {
        use tekmerion_core::{
            EvidenceEngine, SearchCandidate, VerificationResult, VerificationStatus,
        };

        let engine = DeterministicEvidenceEngine::new("run-xyz", "adaface-ir101")
            .with_platform("web");

        let candidate = SearchCandidate {
            url: Url::parse("https://example.com/profile").unwrap(),
            title: Some("Candidate Title".to_string()),
            domain: "example.com".to_string(),
            image_url: Some(Url::parse("https://example.com/img.jpg").unwrap()),
            thumbnail_url: None,
            snippet: Some("Sample snippet description".to_string()),
            provider: "serp_provider".to_string(),
            discovered_at: Utc.with_ymd_and_hms(2026, 9, 4, 10, 0, 0).unwrap(),
        };

        let verified = VerificationResult::new(
            candidate,
            0.881234,
            0.910000,
            Some(0),
            Some("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string()),
            VerificationStatus::Verified,
        );

        let bundle = engine.build_evidence(verified).await.unwrap();

        assert_eq!(bundle.leaves.len(), 5);
        assert!(!bundle.root_hash.is_empty());
        let rec = bundle.record.as_ref().unwrap();
        assert_eq!(rec.schema_version, "1.0.0");
        assert_eq!(rec.run_id, "run-xyz");
        assert_eq!(rec.face_model, "adaface-ir101");
        assert_eq!(rec.face_similarity, 0.881234);
    }

    #[test]
    fn test_same_leaves_same_root() {
        let leaves1 = vec![
            "1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "2222222222222222222222222222222222222222222222222222222222222222".to_string(),
            "3333333333333333333333333333333333333333333333333333333333333333".to_string(),
            "4444444444444444444444444444444444444444444444444444444444444444".to_string(),
            "5555555555555555555555555555555555555555555555555555555555555555".to_string(),
        ];
        let leaves2 = leaves1.clone();

        let root1 = recompute_root(&leaves1).unwrap();
        let root2 = recompute_root(&leaves2).unwrap();

        assert_eq!(root1, root2, "same leaves must produce identical root hash");

        let tree1 = EvidenceTree::from_leaves(leaves1).unwrap();
        let tree2 = EvidenceTree::from_leaves(leaves2).unwrap();
        assert_eq!(tree1.root_hash(), tree2.root_hash());
    }

    #[test]
    fn test_changing_one_leaf_different_root() {
        let base_leaves = vec![
            "1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "2222222222222222222222222222222222222222222222222222222222222222".to_string(),
            "3333333333333333333333333333333333333333333333333333333333333333".to_string(),
            "4444444444444444444444444444444444444444444444444444444444444444".to_string(),
            "5555555555555555555555555555555555555555555555555555555555555555".to_string(),
        ];
        let base_root = recompute_root(&base_leaves).unwrap();

        // Mutate each leaf independently and verify root changes
        for i in 0..5 {
            let mut modified = base_leaves.clone();
            modified[i] = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();
            let mod_root = recompute_root(&modified).unwrap();
            assert_ne!(
                base_root, mod_root,
                "modifying leaf {} must produce a different root hash",
                i
            );
        }
    }

    #[test]
    fn test_leaf_ordering_is_deterministic() {
        let leaf_a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
        let leaf_b = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();
        let leaf_c = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string();
        let leaf_d = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".to_string();
        let leaf_e = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string();

        let ordered = vec![
            leaf_a.clone(),
            leaf_b.clone(),
            leaf_c.clone(),
            leaf_d.clone(),
            leaf_e.clone(),
        ];
        let swapped = vec![
            leaf_b.clone(),
            leaf_a.clone(),
            leaf_c.clone(),
            leaf_d.clone(),
            leaf_e.clone(),
        ];

        let root_ordered = recompute_root(&ordered).unwrap();
        let root_swapped = recompute_root(&swapped).unwrap();

        assert_ne!(
            root_ordered, root_swapped,
            "permuting leaf positions must alter the Merkle root hash"
        );

        // Also test EvidenceTreeLeaves canonical order
        let struct_leaves = EvidenceTreeLeaves::new(&leaf_a, &leaf_b, &leaf_c, &leaf_d, &leaf_e);
        assert_eq!(struct_leaves.to_leaves_vec(), ordered);
    }

    #[test]
    fn test_proof_verification_succeeds() {
        let leaves = vec![
            "image_hash_001".to_string(),
            "content_hash_002".to_string(),
            "metadata_hash_003".to_string(),
            "face_result_hash_004".to_string(),
            "provenance_hash_005".to_string(),
        ];
        let tree = EvidenceTree::from_leaves(leaves.clone()).unwrap();

        for i in 0..leaves.len() {
            let proof = tree.generate_proof(i).unwrap();
            assert_eq!(proof.leaf_index, i);
            assert_eq!(proof.leaf_hash, leaves[i]);
            assert_eq!(proof.root_hash, tree.root_hash());

            assert!(
                verify_proof(&proof),
                "proof for leaf {} must verify successfully",
                i
            );
            assert_eq!(
                recompute_root_from_proof(&proof),
                tree.root_hash(),
                "recomputing root from valid proof must equal tree root"
            );
        }
    }

    #[test]
    fn test_proof_verification_fails_after_modification() {
        let leaves = vec![
            "image_hash_001".to_string(),
            "content_hash_002".to_string(),
            "metadata_hash_003".to_string(),
            "face_result_hash_004".to_string(),
            "provenance_hash_005".to_string(),
        ];
        let tree = EvidenceTree::from_leaves(leaves).unwrap();
        let original_proof = tree.generate_proof(0).unwrap();

        // 1. Modifying leaf hash
        let mut tampered_leaf = original_proof.clone();
        tampered_leaf.leaf_hash = "tampered_leaf_hash".to_string();
        assert!(
            !verify_proof(&tampered_leaf),
            "verification must fail when leaf hash is modified"
        );

        // 2. Modifying root hash
        let mut tampered_root = original_proof.clone();
        tampered_root.root_hash = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        assert!(
            !verify_proof(&tampered_root),
            "verification must fail when root hash is modified"
        );

        // 3. Modifying sibling in audit path
        let mut tampered_path = original_proof.clone();
        if let Some((sibling, _)) = tampered_path.audit_path.first_mut() {
            *sibling = "tampered_sibling_hash".to_string();
        }
        assert!(
            !verify_proof(&tampered_path),
            "verification must fail when audit path is modified"
        );
    }
}
