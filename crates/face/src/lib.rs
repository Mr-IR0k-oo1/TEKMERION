//! TEKMERION local face-analysis worker client.
//!
//! This crate spawns the bundled Python worker (`workers/face/worker.py`) and
//! communicates with it over stdin/stdout JSON Lines. It implements the
//! [`tekmerion_core::FaceEngine`] dependency-injection boundary so the pipeline
//! can consume a [`FaceAnalysis`] without blocking on inference.

pub mod client;
pub mod error;
pub mod protocol;
pub mod quality;

pub use client::{FaceWorker, FaceWorkerConfig};
pub use error::FaceWorkerError;
pub use protocol::{WorkerFace, WorkerPose, WorkerResponse};
pub use quality::{
    assess_face, assess_face_quality, calculate_blur_variance, calculate_brightness, BlurEstimate,
    BlurLevel, ExposureEstimate, ExposureLevel, FaceBoundingBox, FaceQualityAssessment,
    OcclusionIndicators, PoseEstimate, QualityInput, QualityStatus, QualityThresholds,
};
