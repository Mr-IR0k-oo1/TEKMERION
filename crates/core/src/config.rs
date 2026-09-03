use std::env;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use url::Url;

use crate::errors::{CoreError, Result};

const REDACTED: &str = "<redacted>";

/// Runtime configuration for TEKMERION, loaded from environment variables.
///
/// # Security
///
/// * [`Config`] intentionally does **not** implement `Serialize`, so it can
///   never be written to an audit log or any other file.
/// * [`fmt::Debug`] redacts both `search_api_key` and `eth_private_key`, so
///   `{:?}` / `tracing::debug!` output never leaks secrets.
///
/// See `.env.example` for the full variable reference. Each variable is also
/// documented on its field.
#[derive(Clone)]
pub struct Config {
    /// `TEKMERION_SEARCH_API_KEY` — credential for the search provider.
    pub search_api_key: String,
    /// `TEKMERION_SEARCH_ENDPOINT` — base URL of the search provider.
    pub search_endpoint: Url,
    /// `ETH_RPC_URL` — JSON-RPC endpoint for the blockchain.
    pub eth_rpc_url: Url,
    /// `ETH_PRIVATE_KEY` — wallet key used to sign anchoring transactions.
    pub eth_private_key: String,
    /// `EVIDENCE_CONTRACT_ADDRESS` — deployed evidence-anchoring contract (0x + 40 hex).
    pub evidence_contract_address: String,
    /// `FACE_WORKER_PATH` — path to the face-analysis worker executable.
    pub face_worker_path: PathBuf,
    /// `FACE_SIMILARITY_THRESHOLD` — match threshold in `[0.0, 1.0]`. Default `0.8`.
    pub face_similarity_threshold: f32,
    /// `MAX_CANDIDATES` — maximum candidates to retain. Default `10`.
    pub max_candidates: usize,
    /// `MAX_DOWNLOAD_BYTES` — maximum bytes for a candidate download. Default `5 MiB`.
    pub max_download_bytes: u64,
    /// `HTTP_TIMEOUT_SECONDS` — timeout for outbound HTTP requests. Default `30`.
    pub http_timeout_seconds: u64,
    /// `CACHE_DIRECTORY` — directory for cached artifacts.
    pub cache_directory: PathBuf,
    /// `RUN_DIRECTORY` — directory for run outputs and audit files.
    pub run_directory: PathBuf,
}

impl Config {
    /// Load configuration from the process environment.
    ///
    /// Required variables must be present and valid; tuning variables fall back
    /// to documented defaults when absent. Any failure is reported as a
    /// [`CoreError::Config`] naming the offending variable.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            search_api_key: required("TEKMERION_SEARCH_API_KEY")?,
            search_endpoint: parse_url(
                "TEKMERION_SEARCH_ENDPOINT",
                &required("TEKMERION_SEARCH_ENDPOINT")?,
            )?,
            eth_rpc_url: parse_url("ETH_RPC_URL", &required("ETH_RPC_URL")?)?,
            eth_private_key: validate_private_key(&required("ETH_PRIVATE_KEY")?)?,
            evidence_contract_address: validate_eth_address(&required(
                "EVIDENCE_CONTRACT_ADDRESS",
            )?)?,
            face_worker_path: PathBuf::from(required("FACE_WORKER_PATH")?),
            face_similarity_threshold: optional_parsed(
                "FACE_SIMILARITY_THRESHOLD",
                0.8,
                validate_threshold,
            )?,
            max_candidates: optional_parsed("MAX_CANDIDATES", 10, positive)?,
            max_download_bytes: optional_parsed("MAX_DOWNLOAD_BYTES", 5 * 1024 * 1024, positive)?,
            http_timeout_seconds: optional_parsed("HTTP_TIMEOUT_SECONDS", 30, positive)?,
            cache_directory: PathBuf::from(required("CACHE_DIRECTORY")?),
            run_directory: PathBuf::from(required("RUN_DIRECTORY")?),
        })
    }
}

/// A [`Debug`] formatting that redacts secret fields.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("search_api_key", &REDACTED)
            .field("search_endpoint", &self.search_endpoint)
            .field("eth_rpc_url", &self.eth_rpc_url)
            .field("eth_private_key", &REDACTED)
            .field("evidence_contract_address", &self.evidence_contract_address)
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

fn required(name: &str) -> Result<String> {
    env::var(name).map_err(|_| {
        CoreError::Config(format!("required environment variable '{name}' is not set"))
    })
}

fn parse_url(name: &str, raw: &str) -> Result<Url> {
    Url::parse(raw)
        .map_err(|e| CoreError::Config(format!("'{name}' is not a valid URL ('{raw}'): {e}")))
}

/// Reads an optional numeric variable, falling back to `default` when unset,
/// and applies a validator when present.
fn optional_parsed<T>(name: &str, default: T, validate: fn(&str, T) -> Result<T>) -> Result<T>
where
    T: FromStr,
{
    match env::var(name) {
        Ok(raw) => {
            let value = raw.parse::<T>().map_err(|_| {
                CoreError::Config(format!(
                    "'{name}' must be a valid numeric value, got '{raw}'"
                ))
            })?;
            validate(name, value)
        }
        Err(_) => Ok(default),
    }
}

fn positive<T>(name: &str, value: T) -> Result<T>
where
    T: PartialOrd + From<u8> + fmt::Debug,
{
    if value > T::from(0) {
        Ok(value)
    } else {
        Err(CoreError::Config(format!(
            "'{name}' must be strictly positive, got {value:?}"
        )))
    }
}

fn validate_threshold(name: &str, value: f32) -> Result<f32> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(value)
    } else {
        Err(CoreError::Config(format!(
            "'{name}' must be between 0.0 and 1.0, got {value}"
        )))
    }
}

fn validate_eth_address(raw: &str) -> Result<String> {
    let valid =
        raw.len() == 42 && raw.starts_with("0x") && raw[2..].chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        Ok(raw.to_string())
    } else {
        Err(CoreError::Config(format!(
            "'EVIDENCE_CONTRACT_ADDRESS' must be a 0x-prefixed 40-character hex address, got '{raw}'"
        )))
    }
}

fn validate_private_key(raw: &str) -> Result<String> {
    let body = raw.strip_prefix("0x").unwrap_or(raw);
    let valid = (body.len() == 64) && body.chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        Ok(raw.to_string())
    } else {
        Err(CoreError::Config(
            "'ETH_PRIVATE_KEY' must be a 0x-prefixed 64-character hex value".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn lock() -> MutexGuard<'static, ()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    const KEY: &str = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

    fn set_full_env() {
        env::set_var("TEKMERION_SEARCH_API_KEY", "test_key");
        env::set_var("TEKMERION_SEARCH_ENDPOINT", "https://api.test/v1");
        env::set_var("ETH_RPC_URL", "https://rpc.test");
        env::set_var("ETH_PRIVATE_KEY", KEY);
        env::set_var(
            "EVIDENCE_CONTRACT_ADDRESS",
            "0x1234567890abcdef1234567890abcdef12345678",
        );
        env::set_var("FACE_WORKER_PATH", "/tmp/worker.py");
        env::set_var("FACE_SIMILARITY_THRESHOLD", "0.85");
        env::set_var("MAX_CANDIDATES", "10");
        env::set_var("MAX_DOWNLOAD_BYTES", "1000");
        env::set_var("HTTP_TIMEOUT_SECONDS", "30");
        env::set_var("CACHE_DIRECTORY", "/tmp/cache");
        env::set_var("RUN_DIRECTORY", "/tmp/runs");
    }

    fn clear_env() {
        for var in [
            "TEKMERION_SEARCH_API_KEY",
            "TEKMERION_SEARCH_ENDPOINT",
            "ETH_RPC_URL",
            "ETH_PRIVATE_KEY",
            "EVIDENCE_CONTRACT_ADDRESS",
            "FACE_WORKER_PATH",
            "FACE_SIMILARITY_THRESHOLD",
            "MAX_CANDIDATES",
            "MAX_DOWNLOAD_BYTES",
            "HTTP_TIMEOUT_SECONDS",
            "CACHE_DIRECTORY",
            "RUN_DIRECTORY",
        ] {
            env::remove_var(var);
        }
    }

    #[test]
    fn loads_full_configuration() {
        let _guard = lock();
        set_full_env();
        let config = Config::from_env().expect("valid env should load");
        assert_eq!(config.search_api_key, "test_key");
        assert_eq!(config.search_endpoint.as_str(), "https://api.test/v1");
        assert_eq!(config.face_similarity_threshold, 0.85);
        assert_eq!(config.max_candidates, 10);
        assert_eq!(config.max_download_bytes, 1000);
        assert_eq!(config.http_timeout_seconds, 30);
    }

    #[test]
    fn missing_required_var_is_reported_by_name() {
        let _guard = lock();
        set_full_env();
        env::remove_var("TEKMERION_SEARCH_API_KEY");
        match Config::from_env() {
            Err(CoreError::Config(msg)) => {
                assert!(msg.contains("TEKMERION_SEARCH_API_KEY"), "{msg}")
            }
            other => panic!("expected Config error, got {other:?}"),
        }
    }

    #[test]
    fn optional_numeric_vars_fall_back_to_defaults() {
        let _guard = lock();
        set_full_env();
        env::remove_var("FACE_SIMILARITY_THRESHOLD");
        env::remove_var("MAX_CANDIDATES");
        env::remove_var("MAX_DOWNLOAD_BYTES");
        env::remove_var("HTTP_TIMEOUT_SECONDS");
        let config = Config::from_env().expect("defaults should apply");
        assert_eq!(config.face_similarity_threshold, 0.8);
        assert_eq!(config.max_candidates, 10);
        assert_eq!(config.max_download_bytes, 5 * 1024 * 1024);
        assert_eq!(config.http_timeout_seconds, 30);
    }

    #[test]
    fn invalid_similarity_threshold_is_rejected() {
        let _guard = lock();
        set_full_env();
        env::set_var("FACE_SIMILARITY_THRESHOLD", "1.5");
        assert!(matches!(Config::from_env(), Err(CoreError::Config(_))));
    }

    #[test]
    fn non_numeric_value_is_rejected_safely() {
        let _guard = lock();
        set_full_env();
        env::set_var("MAX_CANDIDATES", "lots");
        match Config::from_env() {
            Err(CoreError::Config(msg)) => assert!(msg.contains("MAX_CANDIDATES"), "{msg}"),
            other => panic!("expected Config error, got {other:?}"),
        }
    }

    #[test]
    fn zero_limits_are_rejected() {
        let _guard = lock();
        set_full_env();
        env::set_var("HTTP_TIMEOUT_SECONDS", "0");
        match Config::from_env() {
            Err(CoreError::Config(msg)) => assert!(msg.contains("strictly positive"), "{msg}"),
            other => panic!("expected Config error, got {other:?}"),
        }
    }

    #[test]
    fn invalid_eth_address_format_is_rejected() {
        let _guard = lock();
        set_full_env();
        env::set_var("EVIDENCE_CONTRACT_ADDRESS", "0xNotHex");
        match Config::from_env() {
            Err(CoreError::Config(msg)) => assert!(msg.contains("40-character"), "{msg}"),
            other => panic!("expected Config error, got {other:?}"),
        }
    }

    #[test]
    fn invalid_private_key_is_rejected() {
        let _guard = lock();
        set_full_env();
        env::set_var("ETH_PRIVATE_KEY", "short");
        assert!(matches!(Config::from_env(), Err(CoreError::Config(_))));
    }

    #[test]
    fn invalid_url_is_rejected() {
        let _guard = lock();
        set_full_env();
        env::set_var("TEKMERION_SEARCH_ENDPOINT", "not a url");
        match Config::from_env() {
            Err(CoreError::Config(msg)) => assert!(msg.contains("SEARCH_ENDPOINT"), "{msg}"),
            other => panic!("expected Config error, got {other:?}"),
        }
    }

    #[test]
    fn debug_redacts_secrets() {
        let _guard = lock();
        set_full_env();
        let config = Config::from_env().unwrap();
        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains(KEY),
            "private key leaked in debug output"
        );
        assert!(
            !rendered.contains("test_key"),
            "api key leaked in debug output"
        );
        assert!(rendered.contains(REDACTED));
    }

    #[test]
    fn empty_env_errors_on_first_required() {
        let _guard = lock();
        clear_env();
        assert!(matches!(Config::from_env(), Err(CoreError::Config(_))));
    }
}
