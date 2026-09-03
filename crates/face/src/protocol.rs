//! JSON Lines wire protocol shared with the face-analysis worker.
//!
//! The worker speaks a single, stable JSONL protocol on stdin/stdout. These
//! types mirror that contract and convert parsed responses into the
//! `tekmerion-core` domain structures the pipeline consumes.

use serde::{Deserialize, Serialize};

use tekmerion_core::{FaceDetection, FaceEmbedding};

use crate::error::FaceWorkerError;

/// Three-axis head pose, expressed in the order given by InsightFace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPose {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

/// A single detected face as reported by the worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerFace {
    /// `[x1, y1, x2, y2]` corner box in image pixel coordinates.
    pub bounding_box: [f32; 4],
    /// L2-normalized ArcFace embedding vector.
    pub embedding: Option<Vec<f32>>,
    /// Detection quality / confidence score in `0.0..=1.0`.
    pub quality: f32,
    /// Facial landmark points where the backend provides them.
    #[serde(default)]
    pub landmarks: Option<Vec<[f32; 2]>>,
    /// Head pose where the backend provides it.
    #[serde(default)]
    pub pose: Option<WorkerPose>,
}

/// A complete worker response. Field names match the JSONL protocol exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResponse {
    /// Echoed request identifier, or `null` for unparseable input.
    pub request_id: Option<String>,
    /// Whether the analysis succeeded.
    pub success: bool,
    /// One entry per detected face; empty is an explicit (valid) result.
    #[serde(default)]
    pub faces: Vec<WorkerFace>,
    /// Convenience embedding; non-null only when exactly one face was found.
    #[serde(default)]
    pub embedding: Option<Vec<f32>>,
    /// Convenience quality; non-null only when exactly one face was found.
    #[serde(default)]
    pub quality: Option<f32>,
    /// Structured error summaries; empty on success.
    #[serde(default)]
    pub errors: Vec<String>,
}

impl WorkerResponse {
    /// Whether this response reports a failed analysis (as opposed to control
    /// transport errors like timeouts or crashes).
    pub fn failed(&self) -> bool {
        !self.success
    }

    /// Convert a successful response into domain detections and embeddings.
    ///
    /// Every face is represented explicitly; no face is silently dropped or
    /// chosen. Returns an error if the worker's payload is malformed or the
    /// analysis itself failed.
    pub fn into_semantics(
        self,
    ) -> Result<(Vec<FaceDetection>, Vec<FaceEmbedding>), FaceWorkerError> {
        if !self.success {
            return Err(FaceWorkerError::RequestFailed {
                errors: self.errors,
            });
        }
        let mut detections = Vec::with_capacity(self.faces.len());
        let mut embeddings = Vec::with_capacity(self.faces.len());
        for face in self.faces {
            let vector = face.embedding.ok_or_else(|| {
                FaceWorkerError::InvalidResponse("face entry missing embedding".to_string())
            })?;
            let quality = face.quality;
            detections.push(FaceDetection {
                bounding_box: face.bounding_box,
                confidence: quality,
                quality,
            });
            embeddings.push(FaceEmbedding {
                vector,
                normalized: true,
            });
        }
        Ok((detections, embeddings))
    }
}
