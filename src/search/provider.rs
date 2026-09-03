//! Search provider module

use crate::error::AppError;
use crate::search::models::{SearchCandidate, SearchProvider};
use async_trait::async_trait;
use reqwest::{Client, multipart, StatusCode};
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;
use tracing::{info, error, warn};
use validator::Validate;

/// HTTP-based search provider
pub struct HttpSearchProvider {
    client: Client,
    api_url: String,
    api_key: String,
    max_candidates: usize,
    timeout: Duration,
}

/// Provider-specific response structure
#[derive(Debug, Deserialize)]
struct ProviderResponse {
    results: Vec<ProviderResult>,
}

/// Provider-specific result structure
#[derive(Debug, Deserialize)]
struct ProviderResult {
    title: String,
    url: String,
    domain: String,
    thumbnail: Option<String>,
    image: Option<String>,
    snippet: Option<String>,
}

impl HttpSearchProvider {
    /// Create a new HTTP search provider
    pub fn new(
        api_url: String,
        api_key: String,
        max_candidates: usize,
        timeout_seconds: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_url,
            api_key,
            max_candidates,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    /// Convert provider result to search candidate
    fn convert_result(&self, result: ProviderResult) -> Result<SearchCandidate, AppError> {
        let mut candidate = SearchCandidate {
            title: result.title,
            url: result.url,
            domain: result.domain,
            thumbnail_url: result.thumbnail,
            image_url: result.image,
            snippet: result.snippet,
        };

        // Validate the candidate
        candidate.validate()?;

        Ok(candidate)
    }
}

#[async_trait]
impl SearchProvider for HttpSearchProvider {
    async fn search(&self, image_path: &Path) -> Result<Vec<SearchCandidate>, AppError> {
        info!("Searching for image: {:?}", image_path);

        // Read the image file
        let image_bytes = std::fs::read(image_path)
            .map_err(|e| AppError::SearchError(format!("Failed to read image: {}", e)))?;

        // Create multipart form
        let form = multipart::Form::new()
            .part(
                "image",
                multipart::Part::bytes(image_bytes)
                    .file_name("image.jpg")
                    .mime_str("image/jpeg")
                    .map_err(|e| AppError::SearchError(format!("Failed to create multipart: {}", e)))?,
            );

        // Send the request with retry logic
        let response = self.client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::SearchError(format!("Request failed: {}", e)))?;

        // Check the response status
        if response.status() != StatusCode::OK {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Provider returned error: {} - {}", status, error_text);
            return Err(AppError::SearchError(format!("Provider error: {}", status)));
        }

        // Parse the response
        let provider_response: ProviderResponse = response
            .json()
            .await
            .map_err(|e| AppError::SearchError(format!("Failed to parse response: {}", e)))?;

        // Convert and validate results
        let mut candidates = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        for result in provider_response.results {
            match self.convert_result(result) {
                Ok(candidate) => {
                    if !seen_urls.contains(&candidate.url) {
                        seen_urls.insert(candidate.url.clone());
                        candidates.push(candidate);

                        // Stop if we've reached the maximum number of candidates
                        if candidates.len() >= self.max_candidates {
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!("Skipping invalid candidate: {:?}", e);
                }
            }
        }

        info!("Found {} candidates", candidates.len());
        Ok(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, Server};
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_search_provider() {
        // Create a mock server
        let mut server = Server::new();

        // Mock response
        let mock_response = r#"{
            "results": [
                {
                    "title": "Test Title",
                    "url": "https://example.com",
                    "domain": "example.com",
                    "thumbnail": "https://example.com/thumb.jpg",
                    "image": "https://example.com/image.jpg",
                    "snippet": "Test snippet"
                }
            ]
        }"#;

        // Create a mock endpoint
        let _m = mock("POST", "/search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response)
            .create();

        // Create a temporary image file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test image data").unwrap();
        let image_path = temp_file.path().to_path_buf();

        // Create the provider
        let provider = HttpSearchProvider::new(
            server.url() + "/search",
            "test-api-key".to_string(),
            10,
            30,
        );

        // Perform the search
        let candidates = provider.search(&image_path).await.unwrap();

        // Verify the results
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].title, "Test Title");
        assert_eq!(candidates[0].url, "https://example.com");
    }
}
