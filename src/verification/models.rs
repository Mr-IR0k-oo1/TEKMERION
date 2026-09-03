//! Verification models module

use crate::error::AppError;
use crate::search::models::SearchCandidate;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use validator::Validate;

/// Verification status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerificationStatus {
    NotChecked,
    NoFace,
    Checked,
    Match,
    BelowThreshold,
    Error,
}

/// Candidate verification result
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CandidateResult {
    #[validate(length(min = 1, message = "Title cannot be empty"))]
    pub title: String,
    #[validate(url(message = "Invalid URL format"))]
    pub url: String,
    #[validate(length(min = 1, message = "Source cannot be empty"))]
    pub source: String,
    pub similarity: f32,
    pub matched_face_index: Option<usize>,
    pub verification_status: VerificationStatus,
}

/// Verification request
pub struct VerificationRequest {
    pub search_candidate: SearchCandidate,
    pub input_embedding: Vec<f32>,
    pub temp_dir: PathBuf,
}

/// Verification response
pub struct VerificationResponse {
    pub candidate_result: CandidateResult,
    pub candidate_image_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_result_validation() {
        let valid_result = CandidateResult {
            title: "Test Title".to_string(),
            url: "https://example.com".to_string(),
            source: "Test Source".to_string(),
            similarity: 0.8,
            matched_face_index: Some(0),
            verification_status: VerificationStatus::Match,
        };

        assert!(valid_result.validate().is_ok());

        let invalid_result = CandidateResult {
            title: "".to_string(),
            url: "invalid-url".to_string(),
            source: "".to_string(),
            similarity: 0.8,
            matched_face_index: Some(0),
            verification_status: VerificationStatus::Match,
        };

        assert!(invalid_result.validate().is_err());
    }
}
