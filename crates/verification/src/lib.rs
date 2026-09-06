//! TEKMERION similarity verification.
//!
//! This crate is the placeholder for candidate verification against a source
//! face via similarity scoring. No comparison logic is implemented yet; the
//! pipeline consumes the [`VerificationResult`] domain structure from
//! `tekmerion-core`.
//!
//! [`VerificationResult`]: tekmerion_core::VerificationResult

/// Human-friendly description of the crate's planned responsibility.
pub const DESCRIPTION: &str = "similarity verification of candidates";

pub mod downloader;
pub mod ranking;
pub mod similarity;
pub mod verifier;

pub use downloader::{
    validate_magic_bytes, DownloadError, DownloadedImage, DownloaderConfig, ImageDownloader,
    DEFAULT_MAX_DOWNLOAD_BYTES, DEFAULT_TIMEOUT_SECONDS,
};
pub use ranking::{
    CandidateRanker, CandidateRankingInput, RankedCandidate, RankingError, RankingWeights,
};
pub use similarity::{cosine_similarity, SimilarityError};
pub use verifier::{
    CandidateFaceVerifier, CandidateImageDownloader, CandidateVerifierConfig, FaceAnalysisClient,
    DEFAULT_SIMILARITY_THRESHOLD,
};
