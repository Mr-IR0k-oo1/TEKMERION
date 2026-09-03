//! Search client module

use crate::error::AppError;
use crate::search::models::{SearchCandidate, SearchProvider};
use crate::search::provider::HttpSearchProvider;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

/// Search client
pub struct SearchClient {
    provider: Arc<dyn SearchProvider + Send + Sync>,
}

impl SearchClient {
    /// Create a new search client with HTTP provider
    pub fn new_http(
        api_url: String,
        api_key: String,
        max_candidates: usize,
        timeout_seconds: u64,
    ) -> Self {
        let provider = HttpSearchProvider::new(api_url, api_key, max_candidates, timeout_seconds);
        Self {
            provider: Arc::new(provider),
        }
    }

    /// Search for candidates based on an image
    pub async fn search(&self, image_path: &Path) -> Result<Vec<SearchCandidate>, AppError> {
        info!("Searching for image: {:?}", image_path);
        self.provider.search(image_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, Server};
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_search_client() {
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

        // Create the client
        let client = SearchClient::new_http(
            server.url() + "/search",
            "test-api-key".to_string(),
            10,
            30,
        );

        // Perform the search
        let candidates = client.search(&image_path).await.unwrap();

        // Verify the results
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].title, "Test Title");
        assert_eq!(candidates[0].url, "https://example.com");
    }
}
