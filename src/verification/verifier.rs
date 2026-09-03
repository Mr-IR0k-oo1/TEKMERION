//! Verification module

use crate::error::AppError;
use crate::face::FaceModel;
use crate::verification::models::{VerificationRequest, VerificationResponse, CandidateResult, VerificationStatus};
use crate::verification::similarity::cosine_similarity;
use std::path::Path;
use tracing::{info, error};

/// Verifier
pub struct Verifier {
    face_model: FaceModel,
    similarity_threshold: f32,
}

impl Verifier {
    /// Create a new verifier
    pub fn new(face_model: FaceModel, similarity_threshold: f32) -> Self {
        Self {
            face_model,
            similarity_threshold,
        }
    }

    /// Verify a candidate
    pub async fn verify(&mut self, request: VerificationRequest) -> Result<VerificationResponse, AppError> {
        info!("Verifying candidate: {}", request.search_candidate.url);

        // Check if the candidate has an image URL
        let image_url = match request.search_candidate.image_url {
            Some(url) => url,
            None => {
                error!("No image URL for candidate: {}", request.search_candidate.url);
                return Ok(VerificationResponse {
                    candidate_result: CandidateResult {
                        title: request.search_candidate.title,
                        url: request.search_candidate.url,
                        source: request.search_candidate.domain,
                        similarity: 0.0,
                        matched_face_index: None,
                        verification_status: VerificationStatus::Error,
                    },
                    candidate_image_path: None,
                });
            }
        };

        // Download the candidate image
        let image_path = match self.download_candidate_image(&image_url, &request.temp_dir).await {
            Ok(path) => path,
            Err(e) => {
                error!("Failed to download candidate image: {}", e);
                return Ok(VerificationResponse {
                    candidate_result: CandidateResult {
                        title: request.search_candidate.title,
                        url: request.search_candidate.url,
                        source: request.search_candidate.domain,
                        similarity: 0.0,
                        matched_face_index: None,
                        verification_status: VerificationStatus::Error,
                    },
                    candidate_image_path: None,
                });
            }
        };

        // Process the candidate image
        let face_response = match self.face_model.process_image(&image_path).await {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to process candidate image: {}", e);
                return Ok(VerificationResponse {
                    candidate_result: CandidateResult {
                        title: request.search_candidate.title,
                        url: request.search_candidate.url,
                        source: request.search_candidate.domain,
                        similarity: 0.0,
                        matched_face_index: None,
                        verification_status: VerificationStatus::Error,
                    },
                    candidate_image_path: Some(image_path),
                });
            }
        };

        // Check if any faces were detected
        if face_response.face_count == Some(0) {
            info!("No faces detected in candidate image");
            return Ok(VerificationResponse {
                candidate_result: CandidateResult {
                    title: request.search_candidate.title,
                    url: request.search_candidate.url,
                    source: request.search_candidate.domain,
                    similarity: 0.0,
                    matched_face_index: None,
                    verification_status: VerificationStatus::NoFace,
                },
                candidate_image_path: Some(image_path),
            });
        }

        // Compare embeddings with the input embedding
        let mut highest_similarity = 0.0;
        let mut matched_face_index = None;

        if let Some(embedding) = face_response.embedding {
            let similarity = cosine_similarity(&request.input_embedding, &embedding)?;
            highest_similarity = similarity;
            matched_face_index = Some(0); // Only one face in this case
        } else if let Some(faces) = face_response.faces {
            for (i, face) in faces.iter().enumerate() {
                let similarity = cosine_similarity(&request.input_embedding, &face.embedding)?;
                if similarity > highest_similarity {
                    highest_similarity = similarity;
                    matched_face_index = Some(i);
                }
            }
        }

        // Determine verification status
        let verification_status = if highest_similarity >= self.similarity_threshold {
            VerificationStatus::Match
        } else {
            VerificationStatus::BelowThreshold
        };

        // Create the candidate result
        let candidate_result = CandidateResult {
            title: request.search_candidate.title,
            url: request.search_candidate.url,
            source: request.search_candidate.domain,
            similarity: highest_similarity,
            matched_face_index,
            verification_status,
        };

        Ok(VerificationResponse {
            candidate_result,
            candidate_image_path: Some(image_path),
        })
    }

    /// Download a candidate image
    async fn download_candidate_image(&self, url: &str, temp_dir: &Path) -> Result<PathBuf, AppError> {
        // In a real implementation, this would use the ImageDownloader
        // For now, we'll just create a dummy file
        let temp_file = NamedTempFile::new_in(temp_dir)
            .map_err(|e| AppError::VerificationError(format!("Failed to create temp file: {}", e)))?;
        let path = temp_file.into_temp_path();
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::FaceWorkerResponse;
    use crate::search::models::SearchCandidate;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_verification() {
        // Create a mock face model
        let mut face_model = FaceModel::new().unwrap();

        // Create a verifier with a threshold of 0.8
        let mut verifier = Verifier::new(face_model, 0.8);

        // Create a temporary directory
        let temp_dir = tempdir().unwrap();

        // Create a mock search candidate
        let search_candidate = SearchCandidate {
            title: "Test Candidate".to_string(),
            url: "https://example.com".to_string(),
            domain: "example.com".to_string(),
            thumbnail_url: None,
            image_url: Some("https://example.com/image.jpg".to_string()),
            snippet: None,
        };

        // Create a mock input embedding
        let input_embedding = vec![0.1; 512];

        // Create a verification request
        let request = VerificationRequest {
            search_candidate,
            input_embedding,
            temp_dir: temp_dir.path().to_path_buf(),
        };

        // Mock the face model response
        let face_response = FaceWorkerResponse {
            request_id: "123".to_string(),
            success: true,
            face_count: Some(1),
            embedding: Some(vec![0.2; 512]),
            bbox: None,
            error: None,
            faces: None,
        };

        // Mock the face model's process_image method
        // In a real test, you would use a mocking framework
        // For simplicity, we'll just return the mock response
        // let _ = face_model.process_image(&Path::new("dummy_path")).await;

        // Verify the candidate
        let response = verifier.verify(request).await.unwrap();

        // Check the verification result
        assert_eq!(response.candidate_result.title, "Test Candidate");
        assert!(response.candidate_result.similarity > 0.0);
    }
}
