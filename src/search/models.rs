//! Search models module

use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use url::Url;
use validator::Validate;

/// Search candidate
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SearchCandidate {
    #[validate(length(min = 1, message = "Title cannot be empty"))]
    pub title: String,
    #[validate(url(message = "Invalid URL format"))]
    pub url: String,
    #[validate(length(min = 1, message = "Domain cannot be empty"))]
    pub domain: String,
    #[validate(url(message = "Invalid thumbnail URL format"))]
    pub thumbnail_url: Option<String>,
    #[validate(url(message = "Invalid image URL format"))]
    pub image_url: Option<String>,
    pub snippet: Option<String>,
}

/// Search provider trait
pub trait SearchProvider {
    /// Search for candidates based on an image
    async fn search(&self, image_path: &Path) -> Result<Vec<SearchCandidate>, AppError>;
}

/// Validate a URL string
pub fn validate_url(url_str: &str) -> Result<(), AppError> {
    let _ = Url::parse(url_str).map_err(|e| AppError::SearchError(format!("Invalid URL: {}", e)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

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

    #[test]
    fn test_url_validation() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("invalid-url").is_err());
    }
}
