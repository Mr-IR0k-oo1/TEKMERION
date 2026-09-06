//! Error types for the blockchain client and registry operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlockchainError {
    #[error("network or RPC error: {0}")]
    Rpc(String),

    #[error("invalid contract response: {0}")]
    InvalidResponse(String),

    #[error("transaction failed or reverted: {0}")]
    TransactionFailed(String),

    #[error("evidence not found on chain for root: {0}")]
    NotFound(String),

    #[error("tamper detected: local root {local} does not match on-chain root {chain}")]
    TamperDetected { local: String, chain: String },

    #[error("configuration error: {0}")]
    Config(String),

    #[error("hex decoding error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
