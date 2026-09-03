//! Smart contract integration

use alloy::{
    contract::{ContractInstance, ContractInstanceError},
    providers::{Provider, ProviderBuilder},
    signers::{local::PrivateKeySigner, Signer},
    sol,
    transports::http::Client,
};
use std::sync::Arc;
use tracing::{info, error};

/// Evidence registry contract
#[sol::interface]
interface IEvidenceRegistry {
    /// Register evidence
    #[sol::function]
    fn registerEvidence(bytes32 hash) external;

    /// Verify evidence
    #[sol::function]
    fn verifyEvidence(bytes32 hash) external view returns (bool);
}

/// Evidence registry contract instance
pub struct EvidenceRegistry {
    contract: ContractInstance<IEvidenceRegistry>,
    signer: PrivateKeySigner,
}

impl EvidenceRegistry {
    /// Create a new evidence registry instance
    pub fn new(
        provider: Provider<Client>,
        contract_address: &str,
        private_key: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Creating evidence registry instance");

        // Create signer
        let signer = PrivateKeySigner::from_str(private_key)?;

        // Create contract instance
        let contract = IEvidenceRegistry::new(contract_address.parse()?, provider.clone());

        Ok(Self {
            contract,
            signer,
        })
    }

    /// Register evidence
    pub async fn register_evidence(&self, hash: [u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
        info!("Registering evidence");

        // Call the contract function
        let tx = self.contract.registerEvidence(hash).call().await?;

        // Wait for transaction receipt
        let receipt = tx.get_receipt().await?;

        info!("Evidence registered in block {}", receipt.block_number.unwrap());
        Ok(())
    }

    /// Verify evidence
    pub async fn verify_evidence(&self, hash: [u8; 32]) -> Result<bool, Box<dyn std::error::Error>> {
        info!("Verifying evidence");

        // Call the contract function
        let exists = self.contract.verifyEvidence(hash).call().await?;

        Ok(exists)
    }
}
