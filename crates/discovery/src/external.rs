//! External reverse-image search discovery provider.
//!
//! Connects to configured external reverse-image / visual search APIs using
//! multipart/form-data image upload of the actual input image.
//!
//! Configuration is sourced exclusively from environment variables:
//! - `TEKMERION_SEARCH_API_KEY`: Authentication secret (never logged).
//! - `TEKMERION_SEARCH_ENDPOINT`: External visual search API endpoint.
//! - `TEKMERION_SEARCH_PROVIDER_NAME`: Optional provider identifier (default: "external_reverse_image").
//! - `HTTP_TIMEOUT_SECONDS`: Optional HTTP request timeout in seconds (default: 30).
//! - `TEKMERION_SEARCH_IMAGE_FIELD`: Optional form field name for image file upload (default: "image").

use std::fmt;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::RETRY_AFTER;
use reqwest::{multipart, Client, StatusCode};
use serde_json::Value;
use tekmerion_core::FaceAnalysis;
use url::Url;

use crate::error::DiscoveryError;
use crate::provider::{DiscoveryProvider, RawCandidate};

pub const DEFAULT_PROVIDER_NAME: &str = "external_reverse_image";
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
pub const DEFAULT_IMAGE_FIELD: &str = "image";
pub const DEFAULT_ENDPOINT: &str = "https://api.tekmerion.internal/v1/search/reverse-image";

/// Helper to redact secret keys from error strings and log messages.
pub fn redact_secrets(input: &str, secret: &str) -> String {
    if secret.is_empty() {
        input.to_string()
    } else {
        input.replace(secret, "<redacted>")
    }
}

/// Configuration for the external reverse-image search discovery provider.
///
/// Note: [`std::fmt::Debug`] is explicitly implemented to redact the API key
/// so secrets never appear in logs or diagnostics.
#[derive(Clone)]
pub struct ExternalReverseImageConfig {
    /// Authentication API key for the external visual search API.
    pub api_key: String,
    /// Upstream HTTP endpoint URL for reverse-image search requests.
    pub endpoint: Url,
    /// Provider identification tag for attribution.
    pub provider_name: String,
    /// Request timeout duration.
    pub timeout: Duration,
    /// Multipart form field name for the image file upload.
    pub image_field: String,
}

impl fmt::Debug for ExternalReverseImageConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalReverseImageConfig")
            .field("endpoint", &self.endpoint.as_str())
            .field("provider_name", &self.provider_name)
            .field("timeout", &self.timeout)
            .field("image_field", &self.image_field)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

impl ExternalReverseImageConfig {
    /// Create a new configuration with explicit parameters.
    pub fn new(
        api_key: impl Into<String>,
        endpoint: Url,
        provider_name: impl Into<String>,
        timeout: Duration,
        image_field: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            endpoint,
            provider_name: provider_name.into(),
            timeout,
            image_field: image_field.into(),
        }
    }

    /// Load provider configuration exclusively from environment variables:
    /// - `TEKMERION_SEARCH_API_KEY` (required)
    /// - `TEKMERION_SEARCH_ENDPOINT` (optional, defaults to internal/configured default)
    /// - `TEKMERION_SEARCH_PROVIDER_NAME` (optional, defaults to "external_reverse_image")
    /// - `HTTP_TIMEOUT_SECONDS` (optional, defaults to 30)
    /// - `TEKMERION_SEARCH_IMAGE_FIELD` (optional, defaults to "image")
    pub fn from_env() -> Result<Self, DiscoveryError> {
        let api_key = std::env::var("TEKMERION_SEARCH_API_KEY").map_err(|_| {
            DiscoveryError::Config(
                "missing required environment variable: TEKMERION_SEARCH_API_KEY".to_string(),
            )
        })?;

        let api_key = api_key.trim().to_string();
        if api_key.is_empty() {
            return Err(DiscoveryError::Config(
                "TEKMERION_SEARCH_API_KEY environment variable is empty".to_string(),
            ));
        }

        let endpoint_raw = std::env::var("TEKMERION_SEARCH_ENDPOINT")
            .unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string());
        let endpoint = Url::parse(&endpoint_raw).map_err(|e| {
            DiscoveryError::Config(format!(
                "invalid TEKMERION_SEARCH_ENDPOINT URL '{}': {}",
                endpoint_raw, e
            ))
        })?;

        let provider_name = std::env::var("TEKMERION_SEARCH_PROVIDER_NAME")
            .unwrap_or_else(|_| DEFAULT_PROVIDER_NAME.to_string());

        let timeout_secs = std::env::var("HTTP_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECONDS);

        let image_field = std::env::var("TEKMERION_SEARCH_IMAGE_FIELD")
            .unwrap_or_else(|_| DEFAULT_IMAGE_FIELD.to_string());

        Ok(Self {
            api_key,
            endpoint,
            provider_name,
            timeout: Duration::from_secs(timeout_secs),
            image_field,
        })
    }
}

/// Real reverse-image discovery provider.
///
/// Sends the actual input image from [`FaceAnalysis::image_path`] to the configured
/// external reverse-image-search API via multipart HTTP POST. Parses genuine returned
/// candidates dynamically without fabricating responses or hardcoding URLs.
pub struct ExternalReverseImageProvider {
    config: ExternalReverseImageConfig,
    client: Client,
}

impl fmt::Debug for ExternalReverseImageProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalReverseImageProvider")
            .field("config", &self.config)
            .finish()
    }
}

impl ExternalReverseImageProvider {
    /// Create a new external discovery provider instance from configuration.
    pub fn new(config: ExternalReverseImageConfig) -> Result<Self, DiscoveryError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| DiscoveryError::Internal(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// Access the provider configuration.
    pub fn config(&self) -> &ExternalReverseImageConfig {
        &self.config
    }
}

#[async_trait]
impl DiscoveryProvider for ExternalReverseImageProvider {
    fn id(&self) -> &str {
        &self.config.provider_name
    }

    async fn search(&self, analysis: &FaceAnalysis) -> Result<Vec<RawCandidate>, DiscoveryError> {
        let image_path = analysis.image_path.as_deref().ok_or_else(|| {
            DiscoveryError::Config(
                "FaceAnalysis does not contain an input image_path for reverse-image search"
                    .to_string(),
            )
        })?;

        let path = Path::new(image_path);
        if !path.is_file() {
            return Err(DiscoveryError::Provider {
                provider: self.id().to_string(),
                message: format!("image file does not exist at path: {}", image_path),
            });
        }

        let image_bytes = tokio::fs::read(path)
            .await
            .map_err(|e| DiscoveryError::Provider {
                provider: self.id().to_string(),
                message: format!("failed to read image file '{}': {}", image_path, e),
            })?;

        if image_bytes.is_empty() {
            return Err(DiscoveryError::Provider {
                provider: self.id().to_string(),
                message: format!("image file at '{}' is empty (0 bytes)", image_path),
            });
        }

        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("face.jpg")
            .to_string();

        let mime_type = match path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jpg")
            .to_ascii_lowercase()
            .as_str()
        {
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            _ => "image/jpeg",
        };

        let part = multipart::Part::bytes(image_bytes)
            .file_name(file_name)
            .mime_str(mime_type)
            .map_err(|e| DiscoveryError::Internal(e.to_string()))?;

        let form = multipart::Form::new().part(self.config.image_field.clone(), part);

        tracing::info!(
            provider = self.id(),
            endpoint = %self.config.endpoint,
            "Dispatching reverse-image discovery request with input image"
        );

        let response = self
            .client
            .post(self.config.endpoint.clone())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("X-API-Key", &self.config.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DiscoveryError::Timeout {
                        provider: self.id().to_string(),
                        timeout_ms: self.config.timeout.as_millis() as u64,
                    }
                } else {
                    DiscoveryError::Provider {
                        provider: self.id().to_string(),
                        message: redact_secrets(&e.to_string(), &self.config.api_key),
                    }
                }
            })?;

        let status = response.status();

        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after_secs = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            tracing::warn!(
                provider = self.id(),
                retry_after_secs = ?retry_after_secs,
                "Upstream provider reported rate limit"
            );

            return Err(DiscoveryError::RateLimited {
                provider: self.id().to_string(),
                retry_after_secs,
            });
        }

        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(DiscoveryError::Provider {
                provider: self.id().to_string(),
                message: "Authentication failed: invalid or unauthorized API key".to_string(),
            });
        }

        if status.is_server_error() {
            return Err(DiscoveryError::Provider {
                provider: self.id().to_string(),
                message: format!("Upstream server error: HTTP {}", status),
            });
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let sanitized = redact_secrets(&error_text, &self.config.api_key);
            let snippet = if sanitized.len() > 200 {
                format!("{}...", &sanitized[..200])
            } else {
                sanitized
            };
            return Err(DiscoveryError::Provider {
                provider: self.id().to_string(),
                message: format!(
                    "Upstream search request returned HTTP {}: {}",
                    status, snippet
                ),
            });
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| DiscoveryError::Provider {
                provider: self.id().to_string(),
                message: format!(
                    "Failed to parse upstream JSON response: {}",
                    redact_secrets(&e.to_string(), &self.config.api_key)
                ),
            })?;

        let raw_candidates = extract_candidates_from_response(&body);

        tracing::info!(
            provider = self.id(),
            extracted_count = raw_candidates.len(),
            "Successfully extracted discovery candidates from upstream response"
        );

        Ok(raw_candidates)
    }
}

/// Dynamically extract raw search candidates from an upstream JSON visual search response.
///
/// Inspects standard result containers (`visual_matches`, `results`, `candidates`, `organic_results`,
/// `matches`, `items`, `pages`) as well as top-level array responses. Extracts target URL, title,
/// domain, image URL, thumbnail URL, and snippet without fabricated fallback entries.
pub fn extract_candidates_from_response(json: &Value) -> Vec<RawCandidate> {
    let mut candidates = Vec::new();

    let items: Option<&Vec<Value>> = if let Some(arr) = json.as_array() {
        Some(arr)
    } else if let Some(obj) = json.as_object() {
        // Try common container keys in descending order of prevalence
        const KEYS: &[&str] = &[
            "visual_matches",
            "results",
            "candidates",
            "organic_results",
            "matches",
            "items",
            "pages",
        ];

        let mut found = None;
        for key in KEYS {
            if let Some(arr) = obj.get(*key).and_then(Value::as_array) {
                found = Some(arr);
                break;
            }
        }
        found
    } else {
        None
    };

    let Some(items) = items else {
        return candidates;
    };

    for item in items {
        if let Some(cand) = extract_single_candidate(item) {
            candidates.push(cand);
        }
    }

    candidates
}

fn extract_single_candidate(item: &Value) -> Option<RawCandidate> {
    let obj = item.as_object()?;

    // Dynamic URL extraction
    let url = get_str_by_keys(
        obj,
        &[
            "link",
            "url",
            "source_url",
            "page_url",
            "target_url",
            "link_url",
        ],
    )?;
    if url.trim().is_empty() {
        return None;
    }

    let mut candidate = RawCandidate::new(url);

    // Dynamic title extraction
    if let Some(title) = get_str_by_keys(obj, &["title", "name", "heading"]) {
        candidate = candidate.with_title(title);
    }

    // Dynamic domain extraction
    if let Some(domain) = get_str_by_keys(obj, &["domain", "source", "displayed_link", "site"]) {
        candidate = candidate.with_domain(domain);
    }

    // Dynamic full image URL extraction
    if let Some(image_url) =
        get_str_by_keys(obj, &["image", "original_image", "full_image", "image_url"])
    {
        candidate = candidate.with_image_url(image_url);
    }

    // Dynamic thumbnail URL extraction
    if let Some(thumb_url) = get_str_by_keys(obj, &["thumbnail", "thumbnail_url", "thumb"]) {
        candidate = candidate.with_thumbnail_url(thumb_url);
    }

    // Dynamic snippet / caption extraction
    if let Some(snippet) = get_str_by_keys(
        obj,
        &["snippet", "description", "text", "caption", "summary"],
    ) {
        candidate = candidate.with_snippet(snippet);
    }

    Some(candidate)
}

fn get_str_by_keys<'a>(obj: &'a serde_json::Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(val) = obj.get(*key).and_then(Value::as_str) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn secret_redaction_hides_api_keys() {
        let key = "secret_abc_123";
        let message = format!("Request failed with key: {} on host", key);
        let redacted = redact_secrets(&message, key);
        assert!(!redacted.contains(key));
        assert!(redacted.contains("<redacted>"));
    }

    #[test]
    fn config_debug_redacts_api_key() {
        let config = ExternalReverseImageConfig::new(
            "very_secret_token_12345",
            Url::parse("https://example.com/api").unwrap(),
            "test_provider",
            Duration::from_secs(15),
            "image",
        );

        let debug_str = format!("{:?}", config);
        assert!(!debug_str.contains("very_secret_token_12345"));
        assert!(debug_str.contains("<redacted>"));
        assert!(debug_str.contains("test_provider"));
    }

    #[test]
    fn provider_debug_redacts_api_key() {
        let config = ExternalReverseImageConfig::new(
            "super_secret_auth_token",
            Url::parse("https://example.com/api").unwrap(),
            "test_provider",
            Duration::from_secs(15),
            "image",
        );
        let provider = ExternalReverseImageProvider::new(config).unwrap();
        let debug_str = format!("{:?}", provider);
        assert!(!debug_str.contains("super_secret_auth_token"));
        assert!(debug_str.contains("<redacted>"));
    }

    #[test]
    fn config_from_env_fails_when_api_key_missing() {
        std::env::remove_var("TEKMERION_SEARCH_API_KEY");
        let res = ExternalReverseImageConfig::from_env();
        assert!(matches!(res, Err(DiscoveryError::Config(_))));
    }

    #[test]
    fn extracts_candidates_from_visual_matches() {
        let payload = json!({
            "visual_matches": [
                {
                    "link": "https://example.com/item/1",
                    "title": "Person Profile",
                    "source": "example.com",
                    "thumbnail": "https://images.example.com/thumb1.jpg",
                    "image": "https://images.example.com/full1.jpg",
                    "snippet": "Profile details and biography"
                },
                {
                    "link": "https://anothersite.org/photo",
                    "title": "Photo Gallery",
                    "source": "anothersite.org"
                }
            ]
        });

        let candidates = extract_candidates_from_response(&payload);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].url, "https://example.com/item/1");
        assert_eq!(candidates[0].title.as_deref(), Some("Person Profile"));
        assert_eq!(candidates[0].domain.as_deref(), Some("example.com"));
        assert_eq!(
            candidates[0].thumbnail_url.as_deref(),
            Some("https://images.example.com/thumb1.jpg")
        );
        assert_eq!(
            candidates[0].image_url.as_deref(),
            Some("https://images.example.com/full1.jpg")
        );
        assert_eq!(
            candidates[0].snippet.as_deref(),
            Some("Profile details and biography")
        );

        assert_eq!(candidates[1].url, "https://anothersite.org/photo");
        assert_eq!(candidates[1].title.as_deref(), Some("Photo Gallery"));
    }

    #[test]
    fn extracts_candidates_from_results_key_and_array() {
        let payload = json!({
            "results": [
                {
                    "url": "https://test.net/match",
                    "name": "Match Name",
                    "description": "Short summary text"
                }
            ]
        });

        let candidates = extract_candidates_from_response(&payload);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url, "https://test.net/match");
        assert_eq!(candidates[0].title.as_deref(), Some("Match Name"));
        assert_eq!(candidates[0].snippet.as_deref(), Some("Short summary text"));
    }

    #[test]
    fn skips_entries_without_urls() {
        let payload = json!({
            "visual_matches": [
                {
                    "title": "No URL provided",
                    "snippet": "This should be discarded"
                },
                {
                    "link": "https://valid.org/item"
                }
            ]
        });

        let candidates = extract_candidates_from_response(&payload);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url, "https://valid.org/item");
    }

    #[test]
    fn returns_empty_vec_for_empty_or_unknown_response() {
        let empty_payload = json!({"status": "ok", "count": 0});
        let candidates = extract_candidates_from_response(&empty_payload);
        assert!(candidates.is_empty());
    }
}
