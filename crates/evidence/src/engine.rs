//! Deterministic evidence engine implementation for TEKMERION.

use async_trait::async_trait;
use tekmerion_core::{
    pipeline::{PipelineError, PipelineStage},
    EvidenceBundle, EvidenceEngine, VerificationResult,
};

use crate::record::{EvidenceRecord, CURRENT_SCHEMA_VERSION};

/// Deterministic evidence engine for bundling candidates and facial verification results
/// into tamper-evident cryptographic bundles.
#[derive(Debug, Clone)]
pub struct DeterministicEvidenceEngine {
    run_id: String,
    face_model: String,
    default_platform: String,
}

impl DeterministicEvidenceEngine {
    /// Create a new engine with a specific run ID and face model identifier.
    pub fn new(run_id: impl Into<String>, face_model: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            face_model: face_model.into(),
            default_platform: "web".to_string(),
        }
    }

    /// Set a custom default platform label (e.g. "web", "social", etc.).
    pub fn with_platform(mut self, platform: impl Into<String>) -> Self {
        self.default_platform = platform.into();
        self
    }
}

impl Default for DeterministicEvidenceEngine {
    fn default() -> Self {
        Self {
            run_id: "default-run".to_string(),
            face_model: "adaface-ir101".to_string(),
            default_platform: "web".to_string(),
        }
    }
}

#[async_trait]
impl EvidenceEngine for DeterministicEvidenceEngine {
    async fn build_evidence(
        &self,
        matched: VerificationResult,
    ) -> Result<EvidenceBundle, PipelineError> {
        let record = EvidenceRecord {
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            run_id: self.run_id.clone(),
            source_url: matched.candidate.url,
            domain: matched.candidate.domain,
            platform: self.default_platform.clone(),
            provider: matched.candidate.provider,
            retrieved_at: matched.candidate.discovered_at,
            title: matched.candidate.title.unwrap_or_default(),
            text: matched.candidate.snippet.unwrap_or_default(),
            image_sha256: matched.candidate_image_hash.unwrap_or_default(),
            face_similarity: matched.similarity,
            face_model: self.face_model.clone(),
            candidate_quality: matched.quality,
        };

        let bundle = record.build_bundle().map_err(|e| {
            PipelineError::Stage {
                stage: PipelineStage::Evidence,
                message: format!("failed to compute evidence tree: {}", e),
            }
        })?;

        Ok(bundle.into())
    }
}
