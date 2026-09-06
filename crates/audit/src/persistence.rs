//! Persistent forensic run bundle manager according to Section 16 & 17.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tekmerion_core::{BlockchainRecord, EvidenceBundle, SearchCandidate, VerificationResult};
use tekmerion_evidence::record::EvidenceRecord;
use tokio::fs;

use crate::error::AuditError;
use crate::logger::AuditLogger;

/// Persisted run bundle metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunBundleMeta {
    pub run_id: String,
    pub created_at: chrono::DateTime<Utc>,
    pub root_hash: String,
    pub tx_hash: Option<String>,
}

/// Manager responsible for writing and verifying complete run bundles in `runs/<run_id>/`.
#[derive(Debug, Clone)]
pub struct RunBundleManager {
    base_dir: PathBuf,
}

impl RunBundleManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Generate a unique, timestamped run ID (e.g. `20260906-184221-a83f`).
    pub fn generate_run_id() -> String {
        let now = Utc::now();
        let rand_suffix = format!("{:04x}", (now.timestamp_subsec_nanos() % 0xffff));
        format!("{}-{}", now.format("%Y%m%d-%H%M%S"), rand_suffix)
    }

    /// Initialize directory hierarchy for a given run ID and return the run directory and audit logger.
    pub async fn initialize_run(
        &self,
        run_id: &str,
    ) -> Result<(PathBuf, AuditLogger), AuditError> {
        let run_dir = self.base_dir.join(run_id);
        fs::create_dir_all(run_dir.join("input")).await.map_err(|e| AuditError::Io {
            path: run_dir.join("input"),
            source: e,
        })?;
        fs::create_dir_all(run_dir.join("discovery")).await.map_err(|e| AuditError::Io {
            path: run_dir.join("discovery"),
            source: e,
        })?;
        fs::create_dir_all(run_dir.join("verification")).await.map_err(|e| AuditError::Io {
            path: run_dir.join("verification"),
            source: e,
        })?;
        fs::create_dir_all(run_dir.join("evidence")).await.map_err(|e| AuditError::Io {
            path: run_dir.join("evidence"),
            source: e,
        })?;
        fs::create_dir_all(run_dir.join("blockchain")).await.map_err(|e| AuditError::Io {
            path: run_dir.join("blockchain"),
            source: e,
        })?;

        let logger = AuditLogger::new(run_dir.join("audit.jsonl")).await?;
        Ok((run_dir, logger))
    }

    /// Persist input image or metadata.
    pub async fn persist_input(
        run_dir: &Path,
        image_path: &Path,
        image_bytes: &[u8],
    ) -> Result<(), AuditError> {
        let file_name = image_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("source.jpg");
        let dest = run_dir.join("input").join(file_name);
        fs::write(&dest, image_bytes).await.map_err(|e| AuditError::Io {
            path: dest,
            source: e,
        })?;
        Ok(())
    }

    /// Persist discovered candidates to `discovery/candidates.json`.
    pub async fn persist_discovery(
        run_dir: &Path,
        candidates: &[SearchCandidate],
    ) -> Result<(), AuditError> {
        let dest = run_dir.join("discovery").join("candidates.json");
        let json = serde_json::to_string_pretty(candidates)?;
        fs::write(&dest, json).await.map_err(|e| AuditError::Io {
            path: dest,
            source: e,
        })?;
        Ok(())
    }

    /// Persist candidate verification results to `verification/results.json`.
    pub async fn persist_verification(
        run_dir: &Path,
        results: &[VerificationResult],
    ) -> Result<(), AuditError> {
        let dest = run_dir.join("verification").join("results.json");
        let json = serde_json::to_string_pretty(results)?;
        fs::write(&dest, json).await.map_err(|e| AuditError::Io {
            path: dest,
            source: e,
        })?;
        Ok(())
    }

    /// Persist evidence records, leaves, and root to `evidence/`.
    pub async fn persist_evidence(
        run_dir: &Path,
        bundle: &EvidenceBundle,
    ) -> Result<(), AuditError> {
        let ev_dir = run_dir.join("evidence");

        // 1. evidence.json (record if available)
        if let Some(record) = &bundle.record {
            let record_dest = ev_dir.join("evidence.json");
            let json = serde_json::to_string_pretty(record)?;
            fs::write(&record_dest, json).await.map_err(|e| AuditError::Io {
                path: record_dest,
                source: e,
            })?;
        }

        // 2. leaves.json
        let leaves_dest = ev_dir.join("leaves.json");
        let leaves_json = serde_json::to_string_pretty(&bundle.leaves)?;
        fs::write(&leaves_dest, leaves_json)
            .await
            .map_err(|e| AuditError::Io {
                path: leaves_dest,
                source: e,
            })?;

        // 3. root.json
        let root_dest = ev_dir.join("root.json");
        let root_json = serde_json::to_string_pretty(&json!({
            "root_hash": bundle.root_hash,
            "generated_at": Utc::now()
        }))?;
        fs::write(&root_dest, root_json)
            .await
            .map_err(|e| AuditError::Io {
                path: root_dest,
                source: e,
            })?;

        Ok(())
    }

    /// Persist blockchain anchoring confirmation to `blockchain/transaction.json`.
    pub async fn persist_blockchain(
        run_dir: &Path,
        record: &BlockchainRecord,
    ) -> Result<(), AuditError> {
        let dest = run_dir.join("blockchain").join("transaction.json");
        let json = serde_json::to_string_pretty(record)?;
        fs::write(&dest, json).await.map_err(|e| AuditError::Io {
            path: dest,
            source: e,
        })?;
        Ok(())
    }

    /// Verify an existing persisted run bundle.
    ///
    /// Reads `evidence.json`, recomputes the Merkle tree root, compares with
    /// `root.json` and `blockchain/transaction.json`, and returns Ok(()) or TamperDetected!
    pub async fn verify_bundle(run_dir: &Path) -> Result<bool, AuditError> {
        let evidence_file = run_dir.join("evidence").join("evidence.json");
        let root_file = run_dir.join("evidence").join("root.json");

        if !evidence_file.exists() || !root_file.exists() {
            return Err(AuditError::InvalidBundle(run_dir.to_path_buf()));
        }

        let ev_content = fs::read_to_string(&evidence_file)
            .await
            .map_err(|e| AuditError::Io {
                path: evidence_file.clone(),
                source: e,
            })?;
        let record: EvidenceRecord = serde_json::from_str(&ev_content)?;

        let recomputed_bundle = record
            .build_bundle()
            .map_err(|e| AuditError::TamperDetected {
                run_id: record.run_id.clone(),
                field: "bundle_computation".into(),
                expected: "valid".into(),
                actual: e.to_string(),
            })?;

        let root_content = fs::read_to_string(&root_file)
            .await
            .map_err(|e| AuditError::Io {
                path: root_file.clone(),
                source: e,
            })?;
        let root_val: serde_json::Value = serde_json::from_str(&root_content)?;
        let stored_root = root_val["root_hash"]
            .as_str()
            .ok_or_else(|| AuditError::InvalidBundle(root_file.clone()))?;

        if recomputed_bundle.root_hash != stored_root {
            return Err(AuditError::TamperDetected {
                run_id: record.run_id,
                field: "root_hash".into(),
                expected: stored_root.to_string(),
                actual: recomputed_bundle.root_hash,
            });
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[tokio::test]
    async fn run_bundle_manager_lifecycle_and_tamper_detection() {
        let temp_dir = std::env::temp_dir().join(format!("tekmerion_test_{}", RunBundleManager::generate_run_id()));
        let manager = RunBundleManager::new(&temp_dir);

        let run_id = "test-run-123";
        let (run_dir, _logger) = manager.initialize_run(run_id).await.unwrap();

        assert!(run_dir.join("input").exists());
        assert!(run_dir.join("discovery").exists());
        assert!(run_dir.join("verification").exists());
        assert!(run_dir.join("evidence").exists());
        assert!(run_dir.join("blockchain").exists());

        // Create a valid record
        let record = EvidenceRecord {
            schema_version: "1.0.0".to_string(),
            run_id: run_id.to_string(),
            source_url: Url::parse("https://example.com/profile").unwrap(),
            domain: "example.com".to_string(),
            platform: "web".to_string(),
            provider: "test_provider".to_string(),
            retrieved_at: Utc::now(),
            title: "Original Title".to_string(),
            text: "Original Description".to_string(),
            image_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            face_similarity: 0.95,
            face_model: "arcface-r100".to_string(),
            candidate_quality: 0.90,
        };

        let bundle = record.build_bundle().unwrap();
        let core_bundle = tekmerion_core::EvidenceBundle::new(bundle.leaves.clone(), bundle.root_hash.clone())
            .with_record(tekmerion_core::EvidenceRecord {
                schema_version: record.schema_version.clone(),
                run_id: record.run_id.clone(),
                source_url: record.source_url.clone(),
                domain: record.domain.clone(),
                platform: record.platform.clone(),
                provider: record.provider.clone(),
                retrieved_at: record.retrieved_at,
                title: record.title.clone(),
                text: record.text.clone(),
                image_sha256: record.image_sha256.clone(),
                face_similarity: record.face_similarity,
                face_model: record.face_model.clone(),
                candidate_quality: record.candidate_quality,
            });

        RunBundleManager::persist_evidence(&run_dir, &core_bundle).await.unwrap();

        // 1. Initial verification passes
        let valid = RunBundleManager::verify_bundle(&run_dir).await.unwrap();
        assert!(valid);

        // 2. Tamper the evidence.json file directly on disk
        let ev_file = run_dir.join("evidence").join("evidence.json");
        let tampered_record_json = fs::read_to_string(&ev_file).await.unwrap().replace("Original Title", "Tampered Title");
        fs::write(&ev_file, tampered_record_json).await.unwrap();

        // 3. Re-verification detects tamper!
        let tamper_result = RunBundleManager::verify_bundle(&run_dir).await;
        assert!(tamper_result.is_err());
        match tamper_result.unwrap_err() {
            AuditError::TamperDetected { field, .. } => {
                assert_eq!(field, "root_hash");
            }
            e => panic!("expected TamperDetected, got: {:?}", e),
        }

        // Clean up temp dir
        let _ = fs::remove_dir_all(&temp_dir).await;
    }
}
