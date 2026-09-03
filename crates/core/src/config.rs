use std::env;
use std::path::PathBuf;
use crate::errors::{CoreError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub search_api_key: String,
    pub search_endpoint: String,
    pub eth_rpc_url: String,
    pub eth_private_key: String,
    pub evidence_contract_address: String,
    pub face_worker_path: PathBuf,
    pub face_similarity_threshold: f32,
    pub max_candidates: usize,
    pub max_download_bytes: u64,
    pub http_timeout_seconds: u64,
    pub cache_directory: PathBuf,
    pub run_directory: PathBuf,
}

impl Config {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            search_api_key: Self::get_env("TEKMERION_SEARCH_API_KEY")?,
            search_endpoint: Self::get_env("TEKMERION_SEARCH_ENDPOINT")?,
            eth_rpc_url: Self::get_env("ETH_RPC_URL")?,
            eth_private_key: Self::get_env("ETH_PRIVATE_KEY")?,
            evidence_contract_address: Self::validate_eth_address(
                &Self::get_env("EVIDENCE_CONTRACT_ADDRESS")?
            )?,
            face_worker_path: PathBuf::from(Self::get_env("FACE_WORKER_PATH")?),
            face_similarity_threshold: Self::parse_float(
                "FACE_SIMILARITY_THRESHOLD",
                &Self::get_env("FACE_SIMILARITY_THRESHOLD")?
            )?,
            max_candidates: Self::parse_usize(
                "MAX_CANDIDATES",
                &Self::get_env("MAX_CANDIDATES")?
            )?,
            max_download_bytes: Self::parse_u64(
                "MAX_DOWNLOAD_BYTES",
                &Self::get_env("MAX_DOWNLOAD_BYTES")?
            )?,
            http_timeout_seconds: Self::parse_u64(
                "HTTP_TIMEOUT_SECONDS",
                &Self::get_env("HTTP_TIMEOUT_SECONDS")?
            )?,
            cache_directory: PathBuf::from(Self::get_env("CACHE_DIRECTORY")?),
            run_directory: PathBuf::from(Self::get_env("RUN_DIRECTORY")?),
        })
    }

    fn get_env(key: &str) -> Result<String> {
        env::var(key).map_err(|_| CoreError::ValidationError(format!("Missing required environment variable: {}", key)))
    }

    fn parse_float(key: &str, value: &str) -> Result<f32> {
        let val = value.parse::<f32>().map_err(|_| {
            CoreError::ValidationError(format!("Invalid float for {}: {}", key, value))
        })?;
        if !(0.0..=1.0).contains(&val) {
            return Err(CoreError::ValidationError(format!(
                "{} must be between 0.0 and 1.0, got {}", key, val
            )));
        }
        Ok(val)
    }

    fn parse_usize(key: &str, value: &str) -> Result<usize> {
        value.parse::<usize>().map_err(|_| {
            CoreError::ValidationError(format!("Invalid positive integer for {}: {}", key, value))
        })
    }

    fn parse_u64(key: &str, value: &str) -> Result<u64> {
        value.parse::<u64>().map_err(|_| {
            CoreError::ValidationError(format!("Invalid positive integer for {}: {}", key, value))
        })
    }

    fn validate_eth_address(address: &str) -> Result<String> {
        if address.starts_with("0x") && address.len() == 42 {
            Ok(address.to_string())
        } else {
            Err(CoreError::ValidationError(format!(
                "Invalid Ethereum address format: {}. Must start with 0x and be 42 characters long.",
                address
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_env() {
        env::set_var("TEKMERION_SEARCH_API_KEY", "test_key");
        env::set_var("TEKMERION_SEARCH_ENDPOINT", "https://test.com");
        env::set_var("ETH_RPC_URL", "https://rpc.test");
        env::set_var("ETH_PRIVATE_KEY", "0x1234567890123456789012345678901234567890123456789012345678901234");
        env::set_var("EVIDENCE_CONTRACT_ADDRESS", "0x1234567890123456789012345678901234567890ab");
        env::set_var("FACE_WORKER_PATH", "/tmp/worker.py");
        env::set_var("FACE_SIMILARITY_THRESHOLD", "0.85");
        env::set_var("MAX_CANDIDATES", "10");
        env::set_var("MAX_DOWNLOAD_BYTES", "1000");
        env::set_var("HTTP_TIMEOUT_SECONDS", "30");
        env::set_var("CACHE_DIRECTORY", "/tmp/cache");
        env::set_var("RUN_DIRECTORY", "/tmp/runs");
    }

    #[test]
    fn test_config_load_success() {
        setup_env();
        let config = Config::from_env().expect("Should load config");
        assert_eq!(config.search_api_key, "test_key");
        assert_eq!(config.face_similarity_threshold, 0.85);
    }

    #[test]
    fn test_config_missing_var() {
        env::remove_var("TEKMERION_SEARCH_API_KEY");
        let result = Config::from_env();
        assert!(result.is_err());
        if let Err(CoreError::ValidationError(msg)) = result {
            assert!(msg.contains("TEKMERION_SEARCH_API_KEY"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_config_invalid_threshold() {
        setup_env();
        env::set_var("FACE_SIMILARITY_THRESHOLD", "1.5");
        let result = Config::from_env();
        assert!(result.is_err());
        if let Err(CoreError::ValidationError(msg)) = result {
            assert!(msg.contains("must be between 0.0 and 1.0"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_config_invalid_eth_address() {
        setup_env();
        env::set_var("EVIDENCE_CONTRACT_ADDRESS", "0xInvalidAddress");
        let result = Config::from_env();
        assert!(result.is_err());
        if let Err(CoreError::ValidationError(msg)) = result {
            assert!(msg.contains("Invalid Ethereum address format"));
        } else {
            panic!("Expected ValidationError");
        }
    }
}
