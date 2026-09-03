//! Blockchain models

use alloy::primitives::Address;
use serde::{Serialize, Deserialize};

/// Blockchain transaction information
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockchainTransaction {
    /// Transaction hash
    pub tx_hash: String,
    /// Block number
    pub block_number: u64,
    /// Contract address
    pub contract_address: Address,
}
