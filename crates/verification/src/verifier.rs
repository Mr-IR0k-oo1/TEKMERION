//! Candidate face verification engine.
//!
//! Orchestrates the full candidate verification pipeline:
//! SearchCandidate
//! → Image Download (safe temporary paths, checksum calculation)
//! → Face Worker (face detection and embedding extraction)
//! → Candidate Faces (multi-face comparison against query face)
//! → Embeddings & Cosine Similarity (validated vector math)
//! → VerificationResult (similarity, quality, matched face index, candidate image hash, status)

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tekmerion_core::pipeline::{CandidateVerifier, PipelineError};
use tekmerion_core::{FaceAnalysis, SearchCandidate, VerificationResult, VerificationStatus};

use crate::downloader::{DownloadError, DownloadedImage, ImageDownloader};
use crate::similarity::{cosine_similarity, SimilarityError};

/// Default similarity threshold for a verified match.
pub const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.75;

/// Abstraction for face analysis clients (allows mock injection in tests without Python worker).
#[async_trait]
pub trait FaceAnalysisClient: Send + Sync {
    /// Analyze an image file on disk and extract face detections and embeddings.
    async fn analyze(&self, path: &Path) -> Result<FaceAnalysis, String>;
}

#[async_trait]
impl FaceAnalysisClient for tekmerion_face::FaceWorker {
    async fn analyze(&self, path: &Path) -> Result<FaceAnalysis, String> {
        self.analyze(path).await.map_err(|e| e.to_string())
    }
}

/// Abstraction for candidate image downloading.
#[async_trait]
pub trait CandidateImageDownloader: Send + Sync {
    /// Safely stream-download an image from a URL.
    async fn download(&self, url: &str) -> Result<DownloadedImage, DownloadError>;
}

#[async_trait]
impl CandidateImageDownloader for ImageDownloader {
    async fn download(&self, url: &str) -> Result<DownloadedImage, DownloadError> {
        self.download(url).await
    }
}

/// Configuration parameters for candidate verification.
#[derive(Debug, Clone)]
pub struct CandidateVerifierConfig {
    /// Minimum cosine similarity required for a `VerificationStatus::Verified` match.
    pub similarity_threshold: f32,
}

impl Default for CandidateVerifierConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: DEFAULT_SIMILARITY_THRESHOLD,
        }
    }
}

/// Verification engine that compares candidate images against a reference face embedding.
pub struct CandidateFaceVerifier {
    query_embedding: Vec<f32>,
    downloader: Arc<dyn CandidateImageDownloader>,
    face_client: Arc<dyn FaceAnalysisClient>,
    config: CandidateVerifierConfig,
}

impl CandidateFaceVerifier {
    /// Create a new candidate verifier with validated reference face embedding.
    pub fn new(
        query_embedding: Vec<f32>,
        downloader: Arc<dyn CandidateImageDownloader>,
        face_client: Arc<dyn FaceAnalysisClient>,
    ) -> Result<Self, SimilarityError> {
        if query_embedding.is_empty() {
            return Err(SimilarityError::EmptyVector);
        }
        if query_embedding.iter().any(|x| !x.is_finite()) {
            return Err(SimilarityError::NonFiniteValue);
        }
        let norm_sq = query_embedding.iter().map(|x| x * x).sum::<f32>();
        if norm_sq.sqrt() <= f32::EPSILON {
            return Err(SimilarityError::ZeroNorm);
        }

        Ok(Self {
            query_embedding,
            downloader,
            face_client,
            config: CandidateVerifierConfig::default(),
        })
    }

    /// Update verification configuration.
    pub fn with_config(mut self, config: CandidateVerifierConfig) -> Self {
        self.config = config;
        self
    }

    /// Configure the similarity threshold.
    pub fn with_similarity_threshold(mut self, threshold: f32) -> Self {
        self.config.similarity_threshold = threshold;
        self
    }

    /// Access the current similarity threshold.
    pub fn similarity_threshold(&self) -> f32 {
        self.config.similarity_threshold
    }

    /// Verify a single search candidate through the complete pipeline.
    pub async fn verify_single(&self, candidate: &SearchCandidate) -> VerificationResult {
        let image_url = candidate
            .image_url
            .as_ref()
            .or(candidate.thumbnail_url.as_ref())
            .unwrap_or(&candidate.url);

        tracing::info!(
            url = %image_url,
            provider = %candidate.provider,
            "Downloading candidate image for face verification"
        );

        // 1. Download image
        let downloaded = match self.downloader.download(image_url.as_str()).await {
            Ok(img) => img,
            Err(e) => {
                tracing::warn!(
                    url = %image_url,
                    error = %e,
                    "Failed to download candidate image"
                );
                return VerificationResult {
                    candidate: candidate.clone(),
                    similarity: 0.0,
                    quality: 0.0,
                    matched_face_index: None,
                    candidate_image_hash: None,
                    status: VerificationStatus::Error,
                    error_message: Some(format!("image download failed: {e}")),
                };
            }
        };

        let candidate_image_hash = downloaded.sha256.clone();
        let temp_image_path = downloaded.path.clone();

        // 2. Face analysis
        tracing::info!(
            path = %temp_image_path.display(),
            hash = %candidate_image_hash,
            "Running face worker detection on candidate image"
        );

        let analysis = match self.face_client.analyze(&temp_image_path).await {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!(
                    hash = %candidate_image_hash,
                    error = %e,
                    "Face worker analysis failed on candidate image"
                );
                return VerificationResult {
                    candidate: candidate.clone(),
                    similarity: 0.0,
                    quality: 0.0,
                    matched_face_index: None,
                    candidate_image_hash: Some(candidate_image_hash),
                    status: VerificationStatus::Error,
                    error_message: Some(format!("face worker analysis failed: {e}")),
                };
            }
        };

        // Note: RAII cleanup will automatically remove `temp_image_path` when `downloaded` drops.

        // 3. Inspect detected faces
        let face_count = analysis.detections.len().min(analysis.embeddings.len());
        if face_count == 0 {
            tracing::info!(
                candidate = %candidate.url,
                "No faces detected in candidate image"
            );
            return VerificationResult {
                candidate: candidate.clone(),
                similarity: 0.0,
                quality: 0.0,
                matched_face_index: None,
                candidate_image_hash: Some(candidate_image_hash),
                status: VerificationStatus::NoFace,
                error_message: None,
            };
        }

        // 4. Compare every candidate face against query embedding and select the highest similarity
        let mut best_sim = -2.0f32;
        let mut best_index = 0usize;
        let mut best_quality = 0.0f32;
        let mut comparison_errors = 0usize;

        for i in 0..face_count {
            let candidate_vec = &analysis.embeddings[i].vector;
            let quality = analysis.detections[i].quality;

            match cosine_similarity(&self.query_embedding, candidate_vec) {
                Ok(sim) => {
                    if sim > best_sim {
                        best_sim = sim;
                        best_index = i;
                        best_quality = quality;
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        face_index = i,
                        error = %err,
                        "Failed to compute similarity for candidate face"
                    );
                    comparison_errors += 1;
                }
            }
        }

        if comparison_errors == face_count {
            return VerificationResult {
                candidate: candidate.clone(),
                similarity: 0.0,
                quality: 0.0,
                matched_face_index: None,
                candidate_image_hash: Some(candidate_image_hash),
                status: VerificationStatus::Error,
                error_message: Some("failed to compute similarity for all detected faces".to_string()),
            };
        }

        let status = if best_sim >= self.config.similarity_threshold {
            VerificationStatus::Verified
        } else {
            VerificationStatus::BelowThreshold
        };

        tracing::info!(
            candidate = %candidate.url,
            highest_similarity = best_sim,
            face_index = best_index,
            quality = best_quality,
            status = ?status,
            "Completed candidate face verification"
        );

        VerificationResult {
            candidate: candidate.clone(),
            similarity: best_sim,
            quality: best_quality,
            matched_face_index: Some(best_index),
            candidate_image_hash: Some(candidate_image_hash),
            status,
            error_message: None,
        }
    }

    /// Verify multiple candidates in sequence.
    pub async fn verify_candidates(&self, candidates: Vec<SearchCandidate>) -> Vec<VerificationResult> {
        let mut results = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            results.push(self.verify_single(&candidate).await);
        }
        results
    }
}

#[async_trait]
impl CandidateVerifier for CandidateFaceVerifier {
    async fn verify(
        &self,
        candidates: Vec<SearchCandidate>,
    ) -> Result<Vec<VerificationResult>, PipelineError> {
        Ok(self.verify_candidates(candidates).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tekmerion_core::{FaceDetection, FaceEmbedding};
    use url::Url;

    struct MockDownloader {
        should_fail: bool,
        image_hash: String,
    }

    #[async_trait]
    impl CandidateImageDownloader for MockDownloader {
        async fn download(&self, _url: &str) -> Result<DownloadedImage, DownloadError> {
            if self.should_fail {
                return Err(DownloadError::Http {
                    status: 404,
                    url: _url.to_string(),
                    message: "Not Found".to_string(),
                });
            }

            let temp_dir = std::env::temp_dir();
            let path = temp_dir.join(format!(
                "test_dl_{}.jpg",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::write(&path, b"test_img_data").unwrap();

            Ok(DownloadedImage {
                path,
                sha256: self.image_hash.clone(),
                byte_count: 13,
                content_type: "image/jpeg".to_string(),
                delete_on_drop: true,
            })
        }
    }

    struct MockFaceClient {
        analysis_result: Result<FaceAnalysis, String>,
    }

    #[async_trait]
    impl FaceAnalysisClient for MockFaceClient {
        async fn analyze(&self, _path: &Path) -> Result<FaceAnalysis, String> {
            self.analysis_result.clone()
        }
    }

    fn sample_candidate(url: &str) -> SearchCandidate {
        SearchCandidate {
            url: Url::parse(url).unwrap(),
            title: Some("Sample Profile".to_string()),
            domain: "example.com".to_string(),
            image_url: Some(Url::parse("https://example.com/photo.jpg").unwrap()),
            thumbnail_url: None,
            snippet: Some("Bio".to_string()),
            provider: "google_lens".to_string(),
            discovered_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_candidate_with_multiple_faces_selects_highest_similarity() {
        let query_vec = vec![1.0, 0.0, 0.0];

        // 3 detected faces:
        // Face 0: [0.0, 1.0, 0.0] -> sim 0.0, quality 0.70
        // Face 1: [0.96, 0.28, 0.0] -> sim 0.96, quality 0.91 (HIGHEST)
        // Face 2: [0.5, 0.5, 0.0] -> sim ~0.707, quality 0.85
        let detections = vec![
            FaceDetection { bounding_box: [0.0, 0.0, 1.0, 1.0], confidence: 0.9, quality: 0.70 },
            FaceDetection { bounding_box: [0.0, 0.0, 1.0, 1.0], confidence: 0.95, quality: 0.91 },
            FaceDetection { bounding_box: [0.0, 0.0, 1.0, 1.0], confidence: 0.88, quality: 0.85 },
        ];
        let embeddings = vec![
            FaceEmbedding { vector: vec![0.0, 1.0, 0.0], normalized: true },
            FaceEmbedding { vector: vec![0.96, 0.28, 0.0], normalized: true },
            FaceEmbedding { vector: vec![0.5, 0.5, 0.0], normalized: false },
        ];

        let analysis = FaceAnalysis {
            detections,
            embeddings,
            timestamp: Utc::now(),
            image_path: None,
        };

        let downloader = Arc::new(MockDownloader {
            should_fail: false,
            image_hash: "abcd1234deadbeef".to_string(),
        });
        let face_client = Arc::new(MockFaceClient {
            analysis_result: Ok(analysis),
        });

        let verifier = CandidateFaceVerifier::new(query_vec, downloader, face_client)
            .unwrap()
            .with_similarity_threshold(0.75);

        let cand = sample_candidate("https://example.com/user1");
        let result = verifier.verify_single(&cand).await;

        assert_eq!(result.status, VerificationStatus::Verified);
        assert_eq!(result.matched_face_index, Some(1));
        assert!((result.similarity - 0.96).abs() < 1e-3);
        assert!((result.quality - 0.91).abs() < 1e-3);
        assert_eq!(result.candidate_image_hash.as_deref(), Some("abcd1234deadbeef"));
        assert_eq!(result.status.label(), "Verified");
    }

    #[tokio::test]
    async fn test_candidate_below_threshold() {
        let query_vec = vec![1.0, 0.0, 0.0];

        // Similarity is 0.50 (< 0.75 threshold)
        let detections = vec![
            FaceDetection { bounding_box: [0.0, 0.0, 1.0, 1.0], confidence: 0.9, quality: 0.80 },
        ];
        let embeddings = vec![
            FaceEmbedding { vector: vec![0.5, 0.866, 0.0], normalized: true },
        ];

        let analysis = FaceAnalysis {
            detections,
            embeddings,
            timestamp: Utc::now(),
            image_path: None,
        };

        let downloader = Arc::new(MockDownloader {
            should_fail: false,
            image_hash: "hash_below".to_string(),
        });
        let face_client = Arc::new(MockFaceClient {
            analysis_result: Ok(analysis),
        });

        let verifier = CandidateFaceVerifier::new(query_vec, downloader, face_client)
            .unwrap()
            .with_similarity_threshold(0.75);

        let cand = sample_candidate("https://example.com/user2");
        let result = verifier.verify_single(&cand).await;

        assert_eq!(result.status, VerificationStatus::BelowThreshold);
        assert_eq!(result.matched_face_index, Some(0));
        assert!((result.similarity - 0.50).abs() < 1e-2);
        assert_eq!(result.status.label(), "Below Threshold");
    }

    #[tokio::test]
    async fn test_candidate_with_zero_faces_yields_no_face() {
        let query_vec = vec![1.0, 0.0, 0.0];

        let analysis = FaceAnalysis {
            detections: vec![],
            embeddings: vec![],
            timestamp: Utc::now(),
            image_path: None,
        };

        let downloader = Arc::new(MockDownloader {
            should_fail: false,
            image_hash: "hash_empty".to_string(),
        });
        let face_client = Arc::new(MockFaceClient {
            analysis_result: Ok(analysis),
        });

        let verifier = CandidateFaceVerifier::new(query_vec, downloader, face_client).unwrap();

        let cand = sample_candidate("https://example.com/no-face");
        let result = verifier.verify_single(&cand).await;

        assert_eq!(result.status, VerificationStatus::NoFace);
        assert_eq!(result.matched_face_index, None);
        assert_eq!(result.similarity, 0.0);
        assert_eq!(result.quality, 0.0);
        assert_eq!(result.candidate_image_hash.as_deref(), Some("hash_empty"));
        assert_eq!(result.status.label(), "No Face");
    }

    #[tokio::test]
    async fn test_download_failure_produces_error_status() {
        let query_vec = vec![1.0, 0.0, 0.0];

        let downloader = Arc::new(MockDownloader {
            should_fail: true,
            image_hash: "".to_string(),
        });
        let face_client = Arc::new(MockFaceClient {
            analysis_result: Err("should not be reached".to_string()),
        });

        let verifier = CandidateFaceVerifier::new(query_vec, downloader, face_client).unwrap();

        let cand = sample_candidate("https://example.com/error");
        let result = verifier.verify_single(&cand).await;

        assert_eq!(result.status, VerificationStatus::Error);
        assert!(result.error_message.is_some());
        assert!(result.error_message.as_ref().unwrap().contains("download failed"));
        assert_eq!(result.candidate_image_hash, None);
    }

    #[tokio::test]
    async fn test_face_worker_failure_produces_error_status() {
        let query_vec = vec![1.0, 0.0, 0.0];

        let downloader = Arc::new(MockDownloader {
            should_fail: false,
            image_hash: "downloaded_hash_123".to_string(),
        });
        let face_client = Arc::new(MockFaceClient {
            analysis_result: Err("worker crashed".to_string()),
        });

        let verifier = CandidateFaceVerifier::new(query_vec, downloader, face_client).unwrap();

        let cand = sample_candidate("https://example.com/worker-error");
        let result = verifier.verify_single(&cand).await;

        assert_eq!(result.status, VerificationStatus::Error);
        assert!(result.error_message.is_some());
        assert!(result.error_message.as_ref().unwrap().contains("face worker analysis failed"));
        assert_eq!(result.candidate_image_hash.as_deref(), Some("downloaded_hash_123"));
    }
}
