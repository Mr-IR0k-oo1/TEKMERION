//! In-memory simulated Ethereum Sepolia blockchain client for demo and offline execution.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sha2::{Digest, Sha256};
use tekmerion_core::pipeline::{EvidenceRegistry, PipelineError, PipelineStage};
use tekmerion_core::{BlockchainRecord, EvidenceBundle};
use tokio::sync::RwLock;

use crate::error::BlockchainError;

/// An anchored on-chain record stored in the simulator.
#[derive(Debug, Clone)]
pub struct SimulatedOnChainEvidence {
    pub root_hash: String,
    pub image_hash: String,
    pub tx_hash: String,
    pub block_number: u64,
    pub confirmations: u64,
    pub registered_at: chrono::DateTime<Utc>,
    pub submitter: String,
}

/// Simulated Sepolia blockchain client.
///
/// Provides a zero-dependency, deterministic in-memory blockchain ledger for
/// local development, continuous integration, and demo presentations without
/// requiring an active Sepolia testnet connection or gas funds.
#[derive(Debug, Clone)]
pub struct SimulatedBlockchainClient {
    records: Arc<RwLock<HashMap<String, SimulatedOnChainEvidence>>>,
    by_tx: Arc<RwLock<HashMap<String, String>>>,
    block_counter: Arc<AtomicU64>,
    network_name: String,
    contract_address: String,
    submitter_address: String,
}

impl Default for SimulatedBlockchainClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedBlockchainClient {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            by_tx: Arc::new(RwLock::new(HashMap::new())),
            block_counter: Arc::new(AtomicU64::new(5_892_104)),
            network_name: "Ethereum Sepolia (Simulated)".to_string(),
            contract_address: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
            submitter_address: "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC".to_string(),
        }
    }

    pub fn with_contract(mut self, contract: impl Into<String>) -> Self {
        self.contract_address = contract.into();
        self
    }

    pub fn network_name(&self) -> &str {
        &self.network_name
    }

    pub fn contract_address(&self) -> &str {
        &self.contract_address
    }

    /// Register a root and image hash directly.
    pub async fn register_hash(
        &self,
        root_hash: &str,
        image_hash: &str,
    ) -> Result<BlockchainRecord, BlockchainError> {
        let block_num = self.block_counter.fetch_add(1, Ordering::SeqCst);
        let now = Utc::now();

        // Generate deterministic tx_hash from root + block
        let mut hasher = Sha256::new();
        hasher.update(root_hash.as_bytes());
        hasher.update(block_num.to_be_bytes());
        let tx_hash = format!("0x{}", hex::encode(hasher.finalize()));

        let entry = SimulatedOnChainEvidence {
            root_hash: root_hash.to_string(),
            image_hash: image_hash.to_string(),
            tx_hash: tx_hash.clone(),
            block_number: block_num,
            confirmations: 12,
            registered_at: now,
            submitter: self.submitter_address.clone(),
        };

        let mut map = self.records.write().await;
        map.insert(root_hash.to_string(), entry);

        let mut tx_map = self.by_tx.write().await;
        tx_map.insert(tx_hash.clone(), root_hash.to_string());

        Ok(BlockchainRecord {
            tx_hash,
            block_number: block_num,
            registered_root: root_hash.to_string(),
            timestamp: now,
        })
    }

    /// Retrieve an anchored record by its root hash.
    pub async fn get_by_root(
        &self,
        root_hash: &str,
    ) -> Result<Option<SimulatedOnChainEvidence>, BlockchainError> {
        let map = self.records.read().await;
        Ok(map.get(root_hash).cloned())
    }

    /// Verify an anchor against a candidate root hash.
    ///
    /// If the root hash is not found, or if another record exists for the tx,
    /// returns a TamperDetected error.
    pub async fn verify_evidence_root(
        &self,
        tx_hash: &str,
        candidate_root: &str,
    ) -> Result<bool, BlockchainError> {
        let tx_map = self.by_tx.read().await;
        let stored_root = tx_map
            .get(tx_hash)
            .ok_or_else(|| BlockchainError::NotFound(tx_hash.to_string()))?;

        if stored_root == candidate_root {
            Ok(true)
        } else {
            Err(BlockchainError::TamperDetected {
                local: candidate_root.to_string(),
                chain: stored_root.clone(),
            })
        }
    }
}

#[async_trait]
impl EvidenceRegistry for SimulatedBlockchainClient {
    async fn register(&self, bundle: EvidenceBundle) -> Result<BlockchainRecord, PipelineError> {
        let image_hash = bundle
            .leaves
            .first()
            .cloned()
            .unwrap_or_else(|| "0".repeat(64));

        self.register_hash(&bundle.root_hash, &image_hash)
            .await
            .map_err(|e| PipelineError::Stage {
                stage: PipelineStage::Blockchain,
                message: format!("Simulated blockchain registration failed: {}", e),
            })
    }

    async fn verify_anchor(&self, tx_hash: &str) -> Result<BlockchainRecord, PipelineError> {
        let tx_map = self.by_tx.read().await;
        let root = tx_map
            .get(tx_hash)
            .ok_or_else(|| PipelineError::Stage {
                stage: PipelineStage::OnchainVerification,
                message: format!("Transaction {} not found on simulated chain", tx_hash),
            })?;

        let map = self.records.read().await;
        let record = map.get(root).ok_or_else(|| PipelineError::Stage {
            stage: PipelineStage::OnchainVerification,
            message: format!("Evidence root {} not found on simulated chain", root),
        })?;

        Ok(BlockchainRecord {
            tx_hash: record.tx_hash.clone(),
            block_number: record.block_number,
            registered_root: record.root_hash.clone(),
            timestamp: record.registered_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn simulated_blockchain_register_and_verify() {
        let client = SimulatedBlockchainClient::new();
        let root = "8c4f91a2e3b5d6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1";
        let image = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let record = client.register_hash(root, image).await.unwrap();
        assert!(record.tx_hash.starts_with("0x"));
        assert!(record.block_number > 0);
        assert_eq!(record.registered_root, root);

        // Verify valid root
        let valid = client
            .verify_evidence_root(&record.tx_hash, root)
            .await
            .unwrap();
        assert!(valid);

        // Tampered root detection
        let tampered_root = "17bde902a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8";
        let err = client
            .verify_evidence_root(&record.tx_hash, tampered_root)
            .await
            .unwrap_err();
        match err {
            BlockchainError::TamperDetected { local, chain } => {
                assert_eq!(local, tampered_root);
                assert_eq!(chain, root);
            }
            _ => panic!("expected TamperDetected error"),
        }
    }
}
