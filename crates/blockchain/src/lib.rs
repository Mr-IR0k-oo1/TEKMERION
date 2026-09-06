//! TEKMERION blockchain client and evidence registry.
//!
//! Provides anchoring and verification of evidence roots onto Ethereum Sepolia.

pub mod client;
pub mod error;
pub mod mock;

pub use client::{BlockchainClient, BlockchainConfig};
pub use error::BlockchainError;
pub use mock::{SimulatedBlockchainClient, SimulatedOnChainEvidence};
