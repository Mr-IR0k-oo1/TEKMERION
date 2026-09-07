use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Run not found: {0}")]
    RunNotFound(String),

    #[error("Corrupt run bundle: {0}")]
    CorruptBundle(String),
}

/// Atomically write bytes to a file by writing to a temporary sibling file and renaming.
pub fn atomic_write<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<(), StorageError> {
    let dest = path.as_ref();
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| StorageError::Io {
        path: parent.to_path_buf(),
        source: e,
    })?;

    let tmp_path = parent.join(format!(
        ".tmp_{}_{}",
        std::process::id(),
        hex::encode(&Sha256::digest(data)[..8])
    ));

    let mut f = File::create(&tmp_path).map_err(|e| StorageError::Io {
        path: tmp_path.clone(),
        source: e,
    })?;

    f.write_all(data).map_err(|e| StorageError::Io {
        path: tmp_path.clone(),
        source: e,
    })?;

    f.flush().map_err(|e| StorageError::Io {
        path: tmp_path.clone(),
        source: e,
    })?;

    fs::rename(&tmp_path, dest).map_err(|e| StorageError::Io {
        path: dest.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

/// Atomically write a JSON serializable value.
pub fn atomic_write_json<P: AsRef<Path>, T: serde::Serialize>(
    path: P,
    value: &T,
) -> Result<(), StorageError> {
    let json_bytes = serde_json::to_vec_pretty(value)?;
    atomic_write(path, &json_bytes)
}

/// Manages structured runs under `runs/<run_id>/` according to Stage 14 specifications.
pub struct RunStorageManager {
    base_dir: PathBuf,
}

impl Default for RunStorageManager {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from("runs"),
        }
    }
}

impl RunStorageManager {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    pub fn generate_run_id() -> String {
        let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let rand = hex::encode(&Sha256::digest(ts.to_string().as_bytes())[..2]);
        format!("{ts}-{rand}")
    }

    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.base_dir.join(run_id)
    }

    /// Persist the complete run bundle across subdirectories:
    /// `input/`, `discovery/`, `verification/`, `evidence/`, `blockchain/`, `audit.jsonl`.
    pub fn persist_bundle(
        &self,
        run_id: &str,
        input_data: (&str, &[u8], &serde_json::Value),
        discovery_candidates: &serde_json::Value,
        verification_results: &serde_json::Value,
        evidence_data: (&serde_json::Value, &serde_json::Value, &str), // (evidence_json, leaves_json, root_hash)
        blockchain_meta: &serde_json::Value,
        audit_events: &[String],
    ) -> Result<PathBuf, StorageError> {
        let run_dir = self.run_dir(run_id);
        let input_dir = run_dir.join("input");
        let disc_dir = run_dir.join("discovery");
        let ver_dir = run_dir.join("verification");
        let ev_dir = run_dir.join("evidence");
        let chain_dir = run_dir.join("blockchain");

        // 1. Input
        let (input_filename, input_bytes, input_meta) = input_data;
        atomic_write(input_dir.join(input_filename), input_bytes)?;
        atomic_write_json(input_dir.join("input_metadata.json"), input_meta)?;

        // 2. Discovery
        atomic_write_json(disc_dir.join("candidates.json"), discovery_candidates)?;

        // 3. Verification
        atomic_write_json(ver_dir.join("results.json"), verification_results)?;

        // 4. Evidence
        let (ev_record, leaves, root_hash) = evidence_data;
        atomic_write_json(ev_dir.join("evidence.json"), ev_record)?;
        atomic_write_json(ev_dir.join("leaves.json"), leaves)?;
        atomic_write_json(
            ev_dir.join("root.json"),
            &serde_json::json!({
                "root_hash": root_hash,
                "generated_at": chrono::Utc::now().to_rfc3339(),
            }),
        )?;

        // 5. Blockchain
        atomic_write_json(chain_dir.join("transaction.json"), blockchain_meta)?;

        // 6. Audit Trail
        let audit_content = audit_events
            .iter()
            .map(|e| {
                serde_json::to_string(&serde_json::json!({
                    "run_id": run_id,
                    "event": e,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
                .unwrap_or_else(|_| e.clone())
            })
            .collect::<Vec<_>>()
            .join("\n");
        atomic_write(run_dir.join("audit.jsonl"), audit_content.as_bytes())?;

        Ok(run_dir)
    }

    /// Read an existing evidence bundle for re-verification (Stage 16).
    pub fn load_evidence_record(&self, run_id: &str) -> Result<serde_json::Value, StorageError> {
        let ev_path = self.run_dir(run_id).join("evidence").join("evidence.json");
        if !ev_path.is_file() {
            return Err(StorageError::RunNotFound(format!(
                "evidence.json not found in run {}",
                run_id
            )));
        }
        let bytes = fs::read(&ev_path).map_err(|e| StorageError::Io {
            path: ev_path.clone(),
            source: e,
        })?;
        let val = serde_json::from_slice(&bytes)?;
        Ok(val)
    }

    /// Read stored root hash.
    pub fn load_root_hash(&self, run_id: &str) -> Result<String, StorageError> {
        let root_path = self.run_dir(run_id).join("evidence").join("root.json");
        if !root_path.is_file() {
            return Err(StorageError::RunNotFound(format!(
                "root.json not found in run {}",
                run_id
            )));
        }
        let bytes = fs::read(&root_path).map_err(|e| StorageError::Io {
            path: root_path.clone(),
            source: e,
        })?;
        let val: serde_json::Value = serde_json::from_slice(&bytes)?;
        let root = val
            .get("root_hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StorageError::CorruptBundle("root_hash missing in root.json".to_string()))?
            .to_string();
        Ok(root)
    }
}

/// Cache manager implementing deterministic cache keys (Stage 23).
pub struct CacheManager {
    base_dir: PathBuf,
}

impl Default for CacheManager {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from(".cache"),
        }
    }
}

impl CacheManager {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Compute deterministic cache key: `SHA256(provider + image_hash + provider_version)`.
    pub fn compute_key(provider: &str, image_hash: &str, provider_version: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(provider.as_bytes());
        hasher.update(b":");
        hasher.update(image_hash.as_bytes());
        hasher.update(b":");
        hasher.update(provider_version.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn get_discovery(&self, key: &str) -> Option<serde_json::Value> {
        let p = self.base_dir.join("discovery").join(format!("{key}.json"));
        if p.is_file() {
            let bytes = fs::read(&p).ok()?;
            serde_json::from_slice(&bytes).ok()
        } else {
            None
        }
    }

    pub fn put_discovery(&self, key: &str, val: &serde_json::Value) -> Result<(), StorageError> {
        let p = self.base_dir.join("discovery").join(format!("{key}.json"));
        atomic_write_json(p, val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_write_and_read() {
        let dir = std::env::temp_dir().join("tekmerion_storage_test");
        let file_path = dir.join("test.txt");

        atomic_write(&file_path, b"atomic verification data").unwrap();
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "atomic verification data");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_cache_key_determinism() {
        let k1 = CacheManager::compute_key("serpapi", "e76f2345d86de674", "v1");
        let k2 = CacheManager::compute_key("serpapi", "e76f2345d86de674", "v1");
        assert_eq!(k1, k2);

        let k3 = CacheManager::compute_key("serpapi", "different_hash", "v1");
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_run_storage_persistence() {
        let dir = std::env::temp_dir().join("tekmerion_runs_test");
        let manager = RunStorageManager::new(&dir);
        let run_id = "test-run-001";

        let input_meta = serde_json::json!({ "sha256": "abc" });
        let candidates = serde_json::json!([ { "id": "1" } ]);
        let results = serde_json::json!([ { "sim": 0.95 } ]);
        let ev_record = serde_json::json!({ "title": "Test Record" });
        let leaves = serde_json::json!([ "leaf1" ]);
        let chain_meta = serde_json::json!({ "block": 11651872 });
        let audit = vec!["Event 1".to_string(), "Event 2".to_string()];

        let run_path = manager
            .persist_bundle(
                run_id,
                ("input.jpg", b"fake image bytes", &input_meta),
                &candidates,
                &results,
                (&ev_record, &leaves, "root1234"),
                &chain_meta,
                &audit,
            )
            .unwrap();

        assert!(run_path.join("input").join("input.jpg").exists());
        assert!(run_path.join("evidence").join("root.json").exists());
        assert!(run_path.join("audit.jsonl").exists());

        let loaded_root = manager.load_root_hash(run_id).unwrap();
        assert_eq!(loaded_root, "root1234");

        let loaded_rec = manager.load_evidence_record(run_id).unwrap();
        assert_eq!(loaded_rec["title"], "Test Record");

        let _ = fs::remove_dir_all(dir);
    }
}
