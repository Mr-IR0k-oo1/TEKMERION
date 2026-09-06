//! Structured discovery errors.

use thiserror::Error;

/// Structured, domain-aware error produced during candidate discovery.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DiscoveryError {
    /// An upstream search provider failed with an error message.
    #[error("provider '{provider}' error: {message}")]
    Provider { provider: String, message: String },

    /// A discovery provider query timed out.
    #[error("timeout waiting for provider '{provider}' after {timeout_ms}ms")]
    Timeout { provider: String, timeout_ms: u64 },

    /// Upstream rate limit or quota exceeded.
    #[error("rate limit exceeded for provider '{provider}'{}", .retry_after_secs.map(|s| format!(" (retry after {s}s)")).unwrap_or_default())]
    RateLimited {
        provider: String,
        retry_after_secs: Option<u64>,
    },

    /// A URL candidate failed validation.
    #[error("invalid URL '{url}': {reason}")]
    InvalidUrl { url: String, reason: String },

    /// Discovery engine or provider configuration error.
    #[error("invalid discovery configuration: {0}")]
    Config(String),

    /// Internal error during discovery operations.
    #[error("internal discovery error: {0}")]
    Internal(String),
}

impl From<DiscoveryError> for tekmerion_core::PipelineError {
    fn from(err: DiscoveryError) -> Self {
        tekmerion_core::PipelineError::Stage {
            stage: tekmerion_core::PipelineStage::Discovery,
            message: err.to_string(),
        }
    }
}
