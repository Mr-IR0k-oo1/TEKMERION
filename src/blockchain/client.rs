//! Blockchain client implementation

use alloy::{
    providers::{Provider, ProviderBuilder},
    signers::{local::PrivateKeySigner, Signer},
    transports::http::Client,
};
use std::sync::Arc;
use tracing::info;

/// Blockchain client
pub struct BlockchainClient {
    provider: Provider<Client>,
    signer: PrivateKeySigner,
}

impl BlockchainClient {
    /// Create a new blockchain client
    pub fn new(rpc_url: &str, private_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Creating blockchain client");

        // Create HTTP client
        let client = Client::new();

        // Create provider
        let provider = ProviderBuilder::new().on_client(client).with_recommended_fillers().await?;

        // Create signer
        let signer = PrivateKeySigner::from_str(private_key)?;

        Ok(Self {
            provider,
            signer,
        })
    }
}
