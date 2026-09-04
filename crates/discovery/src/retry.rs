//! Retry policy and backoff calculation for discovery queries.

use std::time::Duration;

use crate::error::DiscoveryError;

/// Configuration for query retry attempts on transient failures.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 means no retries, only the initial attempt).
    pub max_retries: usize,
    /// Delay before the first retry attempt.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Multiplicative factor applied to delay on successive attempts.
    pub backoff_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(2),
            backoff_factor: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Create a policy with zero retries (fail fast).
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            backoff_factor: 1.0,
        }
    }

    /// Calculate backoff duration for a 1-indexed retry attempt.
    pub fn delay_for_attempt(&self, attempt: usize) -> Duration {
        if attempt == 0 || self.max_retries == 0 {
            return Duration::ZERO;
        }
        let factor = self.backoff_factor.powi((attempt - 1) as i32);
        let millis = (self.initial_delay.as_millis() as f64 * factor).round() as u64;
        Duration::from_millis(millis).min(self.max_delay)
    }

    /// Determine if an error is transient and eligible for retry.
    pub fn is_transient(&self, error: &DiscoveryError) -> bool {
        match error {
            DiscoveryError::Timeout { .. } => true,
            DiscoveryError::RateLimited { .. } => true,
            DiscoveryError::Provider { .. } => true,
            DiscoveryError::InvalidUrl { .. } => false,
            DiscoveryError::Config(_) => false,
            DiscoveryError::Internal(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_delay_scales_exponentially_and_clamps() {
        let policy = RetryPolicy {
            max_retries: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(500),
            backoff_factor: 2.0,
        };

        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(400));
        // Clamped to max_delay
        assert_eq!(policy.delay_for_attempt(4), Duration::from_millis(500));
    }

    #[test]
    fn classifies_transient_vs_fatal_errors() {
        let policy = RetryPolicy::default();

        let timeout_err = DiscoveryError::Timeout {
            provider: "test".to_string(),
            timeout_ms: 500,
        };
        assert!(policy.is_transient(&timeout_err));

        let invalid_url = DiscoveryError::InvalidUrl {
            url: "bad".to_string(),
            reason: "malformed".to_string(),
        };
        assert!(!policy.is_transient(&invalid_url));
    }
}
