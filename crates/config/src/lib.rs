use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;
use url::Url;

/// Errors arising from invalid or missing configuration parameters.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid URL for '{key}': {value} ({source})")]
    InvalidUrl {
        key: &'static str,
        value: String,
        source: url::ParseError,
    },

    #[error("Invalid numeric value for '{key}': {value}")]
    InvalidNumber { key: &'static str, value: String },

    #[error("Value out of valid range for '{key}': {msg}")]
    OutOfRange { key: &'static str, msg: String },

    #[error("Invalid Ethereum address for '{key}': {value} (must be 0x followed by 40 hex characters)")]
    InvalidAddress { key: &'static str, value: String },
}

/// Validated runtime configuration for TEKMERION.
///
/// Implements custom `Debug` and `Display` to guarantee that secrets
/// (`search_api_key`, `eth_private_key`) are NEVER exposed in logs,
/// console outputs, or error messages.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeConfig {
    pub search_api_key: Option<String>,
    pub search_endpoint: Url,

    pub eth_rpc_url: Url,
    pub eth_private_key: Option<String>,
    pub contract_address: String,

    pub face_worker_path: PathBuf,

    pub face_similarity_threshold: f64,
    pub max_candidates: usize,
    pub max_download_bytes: usize,
    pub http_timeout_seconds: u64,

    pub cache_directory: PathBuf,
    pub run_directory: PathBuf,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            search_api_key: None,
            search_endpoint: Url::parse("https://api.searchprovider.com/v1").unwrap(),
            eth_rpc_url: Url::parse("https://ethereum-sepolia.publicnode.com").unwrap(),
            eth_private_key: None,
            contract_address: "0x71C2d385aE2F56d9812A45B8a9b70d41C68E3a9E".to_string(),
            face_worker_path: PathBuf::from("workers/face/worker.py"),
            face_similarity_threshold: 0.75,
            max_candidates: 20,
            max_download_bytes: 10 * 1024 * 1024, // 10 MB
            http_timeout_seconds: 30,
            cache_directory: PathBuf::from(".cache"),
            run_directory: PathBuf::from("runs"),
        }
    }
}

impl fmt::Debug for RuntimeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeConfig")
            .field(
                "search_api_key",
                &self.search_api_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("search_endpoint", &self.search_endpoint.as_str())
            .field("eth_rpc_url", &self.eth_rpc_url.as_str())
            .field(
                "eth_private_key",
                &self.eth_private_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("contract_address", &self.contract_address)
            .field("face_worker_path", &self.face_worker_path)
            .field("face_similarity_threshold", &self.face_similarity_threshold)
            .field("max_candidates", &self.max_candidates)
            .field("max_download_bytes", &self.max_download_bytes)
            .field("http_timeout_seconds", &self.http_timeout_seconds)
            .field("cache_directory", &self.cache_directory)
            .field("run_directory", &self.run_directory)
            .finish()
    }
}

impl fmt::Display for RuntimeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TEKMERION Config [Contract: {}, Network RPC: {}, Threshold: {:.2}]",
            self.contract_address,
            self.eth_rpc_url,
            self.face_similarity_threshold
        )
    }
}

/// Loader and validator for runtime configuration.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration by parsing `.env` (if present) and merging with environment variables.
    pub fn load() -> Result<RuntimeConfig, ConfigError> {
        let mut map = HashMap::new();

        // Read .env if present
        if let Ok(content) = std::fs::read_to_string(".env") {
            Self::parse_env_file(&content, &mut map);
        }

        // Environment variables override .env
        for (k, v) in std::env::vars() {
            map.insert(k, v);
        }

        Self::from_map(&map)
    }

    /// Load from an explicit key-value map.
    pub fn from_map(map: &HashMap<String, String>) -> Result<RuntimeConfig, ConfigError> {
        let defaults = RuntimeConfig::default();

        let search_api_key = map
            .get("TEKMERION_SEARCH_API_KEY")
            .cloned()
            .filter(|s| !s.trim().is_empty() && s != "your_api_key_here");

        let search_endpoint = if let Some(val) = map.get("TEKMERION_SEARCH_ENDPOINT") {
            Url::parse(val).map_err(|e| ConfigError::InvalidUrl {
                key: "TEKMERION_SEARCH_ENDPOINT",
                value: val.clone(),
                source: e,
            })?
        } else {
            defaults.search_endpoint
        };

        let eth_rpc_url = if let Some(val) = map.get("ETH_RPC_URL") {
            Url::parse(val).map_err(|e| ConfigError::InvalidUrl {
                key: "ETH_RPC_URL",
                value: val.clone(),
                source: e,
            })?
        } else {
            defaults.eth_rpc_url
        };

        let eth_private_key = map
            .get("ETH_PRIVATE_KEY")
            .cloned()
            .filter(|s| !s.trim().is_empty() && !s.starts_with("0xyour_private_key"));

        let contract_address = map
            .get("EVIDENCE_CONTRACT_ADDRESS")
            .cloned()
            .unwrap_or(defaults.contract_address);

        // Validate contract address
        if !Self::is_valid_eth_address(&contract_address) {
            return Err(ConfigError::InvalidAddress {
                key: "EVIDENCE_CONTRACT_ADDRESS",
                value: contract_address,
            });
        }

        let face_worker_path = map
            .get("FACE_WORKER_PATH")
            .map(PathBuf::from)
            .unwrap_or(defaults.face_worker_path);

        let face_similarity_threshold = if let Some(val) = map.get("FACE_SIMILARITY_THRESHOLD") {
            let num = val.parse::<f64>().map_err(|_| ConfigError::InvalidNumber {
                key: "FACE_SIMILARITY_THRESHOLD",
                value: val.clone(),
            })?;
            if !(0.0..=1.0).contains(&num) {
                return Err(ConfigError::OutOfRange {
                    key: "FACE_SIMILARITY_THRESHOLD",
                    msg: "Must be between 0.0 and 1.0".to_string(),
                });
            }
            num
        } else {
            defaults.face_similarity_threshold
        };

        let max_candidates = if let Some(val) = map.get("MAX_CANDIDATES") {
            val.parse::<usize>().map_err(|_| ConfigError::InvalidNumber {
                key: "MAX_CANDIDATES",
                value: val.clone(),
            })?
        } else {
            defaults.max_candidates
        };

        let max_download_bytes = if let Some(val) = map.get("MAX_DOWNLOAD_BYTES") {
            val.parse::<usize>().map_err(|_| ConfigError::InvalidNumber {
                key: "MAX_DOWNLOAD_BYTES",
                value: val.clone(),
            })?
        } else {
            defaults.max_download_bytes
        };

        let http_timeout_seconds = if let Some(val) = map.get("HTTP_TIMEOUT_SECONDS") {
            val.parse::<u64>().map_err(|_| ConfigError::InvalidNumber {
                key: "HTTP_TIMEOUT_SECONDS",
                value: val.clone(),
            })?
        } else {
            defaults.http_timeout_seconds
        };

        let cache_directory = map
            .get("CACHE_DIRECTORY")
            .map(PathBuf::from)
            .unwrap_or(defaults.cache_directory);

        let run_directory = map
            .get("RUN_DIRECTORY")
            .map(PathBuf::from)
            .unwrap_or(defaults.run_directory);

        Ok(RuntimeConfig {
            search_api_key,
            search_endpoint,
            eth_rpc_url,
            eth_private_key,
            contract_address,
            face_worker_path,
            face_similarity_threshold,
            max_candidates,
            max_download_bytes,
            http_timeout_seconds,
            cache_directory,
            run_directory,
        })
    }

    fn parse_env_file(content: &str, map: &mut HashMap<String, String>) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim().to_string();
                let mut val = v.trim().to_string();
                if (val.starts_with('"') && val.ends_with('"'))
                    || (val.starts_with('\'') && val.ends_with('\''))
                {
                    val = val[1..val.len() - 1].to_string();
                }
                map.insert(key, val);
            }
        }
    }

    fn is_valid_eth_address(addr: &str) -> bool {
        if !addr.starts_with("0x") || addr.len() != 42 {
            return false;
        }
        addr[2..].chars().all(|c| c.is_ascii_hexdigit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = RuntimeConfig::default();
        assert_eq!(
            config.contract_address,
            "0x71C2d385aE2F56d9812A45B8a9b70d41C68E3a9E"
        );
        assert_eq!(config.face_similarity_threshold, 0.75);
    }

    #[test]
    fn test_secret_redaction_in_debug() {
        let mut config = RuntimeConfig::default();
        config.search_api_key = Some("super_secret_serp_key_999".to_string());
        config.eth_private_key =
            Some("0x1122334455667788990011223344556677889900112233445566778899001122".to_string());

        let debug_str = format!("{:?}", config);
        assert!(!debug_str.contains("super_secret_serp_key_999"));
        assert!(!debug_str.contains("112233445566778899001122"));
        assert!(debug_str.contains("[REDACTED]"));
    }

    #[test]
    fn test_parse_valid_custom_map() {
        let mut map = HashMap::new();
        map.insert(
            "ETH_RPC_URL".to_string(),
            "https://rpc.sepolia.org".to_string(),
        );
        map.insert("FACE_SIMILARITY_THRESHOLD".to_string(), "0.85".to_string());
        map.insert("MAX_CANDIDATES".to_string(), "30".to_string());

        let config = ConfigLoader::from_map(&map).unwrap();
        assert_eq!(config.eth_rpc_url.as_str(), "https://rpc.sepolia.org/");
        assert_eq!(config.face_similarity_threshold, 0.85);
        assert_eq!(config.max_candidates, 30);
    }

    #[test]
    fn test_rejects_invalid_contract_address() {
        let mut map = HashMap::new();
        map.insert(
            "EVIDENCE_CONTRACT_ADDRESS".to_string(),
            "0xInvalidAddress".to_string(),
        );

        let err = ConfigLoader::from_map(&map).unwrap_err();
        match err {
            ConfigError::InvalidAddress { .. } => {}
            _ => panic!("Expected InvalidAddress error"),
        }
    }

    #[test]
    fn test_rejects_out_of_range_threshold() {
        let mut map = HashMap::new();
        map.insert("FACE_SIMILARITY_THRESHOLD".to_string(), "1.5".to_string());

        let err = ConfigLoader::from_map(&map).unwrap_err();
        match err {
            ConfigError::OutOfRange { .. } => {}
            _ => panic!("Expected OutOfRange error"),
        }
    }
}
