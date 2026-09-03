//! Blockchain integration tests

use super::*;
use alloy::primitives::Address;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::providers::{Provider, ProviderBuilder};
    use alloy::transports::http::Client;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_blockchain_client_creation() {
        // Mock RPC URL and private key
        let rpc_url = "http://localhost:8545";
        let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

        // Create client
        let client = BlockchainClient::new(rpc_url, private_key).unwrap();

        // Verify client was created
        assert!(client.provider.is_some());
        assert!(client.signer.is_some());
    }

    #[tokio::test]
    async fn test_evidence_registration() {
        // Mock configuration
        let rpc_url = "http://localhost:8545";
        let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let contract_address = "0x5FbDB2315678afecb367f032d93F642f64180aa3";

        // Create provider
        let client = Client::new();
        let provider = ProviderBuilder::new().on_client(client).with_recommended_fillers().await.unwrap();

        // Create evidence registry
        let evidence_registry = EvidenceRegistry::new(
            provider,
            contract_address,
            private_key,
        )
        .unwrap();

        // Mock evidence hash
        let evidence_hash = [0u8; 32];

        // Register evidence
        let result = evidence_registry.register_evidence(evidence_hash).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_evidence_verification() {
        // Mock configuration
        let rpc_url = "http://localhost:8545";
        let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let contract_address = "0x5FbDB2315678afecb367f032d93F642f64180aa3";

        // Create provider
        let client = Client::new();
        let provider = ProviderBuilder::new().on_client(client).with_recommended_fillers().await.unwrap();

        // Create evidence registry
        let evidence_registry = EvidenceRegistry::new(
            provider,
            contract_address,
            private_key,
        )
        .unwrap();

        // Mock evidence hash
        let evidence_hash = [0u8; 32];

        // Verify evidence
        let result = evidence_registry.verify_evidence(evidence_hash).await;
        assert!(result.is_ok());
    }
}
