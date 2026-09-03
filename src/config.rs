//! Configuration loading and management

use crate::error::AppError;
use serde::Deserialize;
use std::env;

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {

/// Load configuration from environment variables
pub fn load_config() -> Result<Config, AppError> {
    Ok(Config {
        blockchain_rpc_url: env::var("BLOCKCHAIN_RPC_URL")?,
        contract_address: env::var("CONTRACT_ADDRESS")?,
        face_model_path: env::var("FACE_MODEL_PATH")?,
        search_api_url: env::var("SEARCH_API_URL")?,
        search_api_key: env::var("SEARCH_API_KEY")?,
        max_search_candidates: env::var("MAX_SEARCH_CANDIDATES")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .map_err(|_| AppError::ConfigError("Invalid MAX_SEARCH_CANDIDATES".to_string()))?,
        search_timeout_seconds: env::var("SEARCH_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .map_err(|_| AppError::ConfigError("Invalid SEARCH_TIMEOUT_SECONDS".to_string()))?,
    })
}
