//! Forensic audit events for TEKMERION pipeline execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Strongly-typed event name representing forensic lifecycle moments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    InputReceived,
    FaceAnalysisStarted,
    FaceAnalysisCompleted,
    DiscoveryStarted,
    DiscoveryCompleted,
    CandidatesDeduplicated,
    VerificationStarted,
    CandidateVerified,
    MatchSelected,
    EvidenceCreated,
    RootHashComputed,
    BlockchainSubmitted,
    TransactionConfirmed,
    OnchainVerification,
    VerificationPassed,
    TamperDetected,
    SearchFailure,
    DownloadFailure,
    WorkerFailure,
    TransactionFailure,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::InputReceived => "INPUT_RECEIVED",
            EventType::FaceAnalysisStarted => "FACE_ANALYSIS_STARTED",
            EventType::FaceAnalysisCompleted => "FACE_ANALYSIS_COMPLETED",
            EventType::DiscoveryStarted => "DISCOVERY_STARTED",
            EventType::DiscoveryCompleted => "DISCOVERY_COMPLETED",
            EventType::CandidatesDeduplicated => "CANDIDATES_DEDUPLICATED",
            EventType::VerificationStarted => "VERIFICATION_STARTED",
            EventType::CandidateVerified => "CANDIDATE_VERIFIED",
            EventType::MatchSelected => "MATCH_SELECTED",
            EventType::EvidenceCreated => "EVIDENCE_CREATED",
            EventType::RootHashComputed => "ROOT_HASH_COMPUTED",
            EventType::BlockchainSubmitted => "BLOCKCHAIN_SUBMITTED",
            EventType::TransactionConfirmed => "TRANSACTION_CONFIRMED",
            EventType::OnchainVerification => "ONCHAIN_VERIFICATION",
            EventType::VerificationPassed => "VERIFICATION_PASSED",
            EventType::TamperDetected => "TAMPER_DETECTED",
            EventType::SearchFailure => "SEARCH_FAILURE",
            EventType::DownloadFailure => "DOWNLOAD_FAILURE",
            EventType::WorkerFailure => "WORKER_FAILURE",
            EventType::TransactionFailure => "TRANSACTION_FAILURE",
        }
    }
}

/// A structured immutable audit event line in the JSONL audit trail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event: String,
    pub timestamp: DateTime<Utc>,
    pub run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl AuditEvent {
    pub fn new(run_id: impl Into<String>, event_type: EventType) -> Self {
        Self {
            event: event_type.as_str().to_string(),
            timestamp: Utc::now(),
            run_id: run_id.into(),
            stage: None,
            details: None,
        }
    }

    pub fn with_stage(mut self, stage: impl Into<String>) -> Self {
        self.stage = Some(stage.into());
        self
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}
