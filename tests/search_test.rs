//! Tests for search functionality

use async_trait::async_trait;
use hh_face::error::AppError;
use hh_face::search::{SearchCandidate, SearchProvider};
use std::path::Path;
use std::sync::Arc;
use tempfile::NamedTempFile;
use validator::Validate;

/// Mock search provider for testing
struct MockSearchProvider;

#[async_trait]
impl SearchProvider for MockSearchProvider {
    async fn search(&self, _image_path: &Path) -> Result<Vec<SearchCandidate>, AppError> {
        Ok(vec![
            SearchCandidate {
                title: "Test Title".to_string(),
                url: "https://example.com".to_string(),
                domain: "example.com".to_string(),
                thumbnail_url: None,
                image_url: None,
                snippet: None,
            },
            SearchCandidate {
                title: "Duplicate Title".to_string(),
                url: "https://example.com".to_string(),
                domain: "example.com".to_string(),
                thumbnail_url: None,
                image_url: None,
                snippet: None,
            },
            SearchCandidate {
                title: "Invalid URL".to_string(),
                url: "invalid-url".to_string(),
                domain: "example.com".to_string(),
                thumbnail_url: None,
                image_url: None,
                snippet: None,
            },
        ])
    }
}

#[tokio::test]
async fn test_search_provider() {
    let provider = MockSearchProvider;
    let temp_file = NamedTempFile::new().unwrap();
    let image_path = temp_file.path();

    let candidates = provider.search(image_path).await.unwrap();

    // Test deduplication
    assert_eq!(candidates.len(), 2);

    // Test URL validation
    let valid_candidates: Vec<_> = candidates
        .into_iter()
        .filter(|c| c.validate().is_ok())
        .collect();
    assert_eq!(valid_candidates.len(), 1);
}

#[test]
fn test_search_candidate_validation() {
    let valid_candidate = SearchCandidate {
        title: "Test Title".to_string(),
        url: "https://example.com".to_string(),
        domain: "example.com".to_string(),
        thumbnail_url: Some("https://example.com/thumb.jpg".to_string()),
        image_url: Some("https://example.com/image.jpg".to_string()),
        snippet: Some("Test snippet".to_string()),
    };

    assert!(valid_candidate.validate().is_ok());

    let invalid_candidate = SearchCandidate {
        title: "".to_string(),
        url: "invalid-url".to_string(),
        domain: "".to_string(),
        thumbnail_url: Some("invalid-url".to_string()),
        image_url: Some("invalid-url".to_string()),
        snippet: None,
    };

    assert!(invalid_candidate.validate().is_err());
}
