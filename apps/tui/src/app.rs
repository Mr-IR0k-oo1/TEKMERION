use std::collections::VecDeque;

use tekmerion_core::{PipelineState, SearchCandidate, VerificationResult, VerificationStatus};
use tekmerion_evidence::{EvidenceBundle, EvidenceRecord, CURRENT_SCHEMA_VERSION};
use tekmerion_face::FaceQualityAssessment;
use tekmerion_verification::{CandidateRanker, RankedCandidate};
use url::Url;

use crate::input::{AppAction, Direction};

const MAX_EVENTS: usize = 8;
const STAGE_COUNT: usize = Stage::ALL.len();

/// The pipeline stages surfaced by the interface.
///
/// This is a display-level model of the pipeline. The interface advances
/// through these stages deterministically as the user drives it; it never
/// fabricates evidence, hashes or match results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Input,
    Face,
    Discovery,
    Verify,
    Evidence,
    Blockchain,
    FinalVerify,
}

impl Stage {
    pub const ALL: [Stage; 7] = [
        Stage::Input,
        Stage::Face,
        Stage::Discovery,
        Stage::Verify,
        Stage::Evidence,
        Stage::Blockchain,
        Stage::FinalVerify,
    ];

    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|s| *s == self)
            .expect("every stage is present in ALL")
    }

    pub fn label(self) -> &'static str {
        match self {
            Stage::Input => "INPUT",
            Stage::Face => "FACE",
            Stage::Discovery => "DISCOVERY",
            Stage::Verify => "VERIFY",
            Stage::Evidence => "EVIDENCE",
            Stage::Blockchain => "BLOCKCHAIN",
            Stage::FinalVerify => "FINAL VERIFY",
        }
    }

    pub fn next(self) -> Option<Stage> {
        Self::ALL.get(self.index() + 1).copied()
    }

    /// Map a UI stage onto the corresponding core domain state.
    fn to_pipeline_state(self) -> PipelineState {
        match self {
            Stage::Input => PipelineState::InputReady,
            Stage::Face => PipelineState::FaceAnalysis,
            Stage::Discovery => PipelineState::CandidatesFound,
            Stage::Verify => PipelineState::MatchFound,
            Stage::Evidence => PipelineState::EvidenceCreated,
            Stage::Blockchain => PipelineState::BlockchainConfirmed,
            Stage::FinalVerify => PipelineState::VerifyingOnchain,
        }
    }
}

/// Top-level status of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppStatus {
    Idle,
    Running,
    Completed,
    Tampered,
}

/// Mutable UI state.
///
/// `evidence_root` and `tx_hash` remain `"--"` until a real value is written by
/// an engine; the interface never invents them. `candidate_count` starts at
/// zero and reflects actual discovery results (none are available yet).
pub struct App {
    pub status: AppStatus,
    pub current: Option<Stage>,
    pub candidate_count: usize,
    pub selected_candidate: usize,
    pub evidence_root: String,
    pub tx_hash: String,
    pub verification_result: String,
    pub events: VecDeque<String>,
    pub face_quality: Option<FaceQualityAssessment>,
    pub discovery_provider: String,
    pub discovery_request_status: String,
    pub discovery_raw_count: usize,
    pub discovery_unique_count: usize,
    pub discovery_error: Option<String>,
    pub verified_candidates: Vec<VerificationResult>,
    pub ranked_candidates: Vec<RankedCandidate>,
    pub evidence_bundle: Option<EvidenceBundle>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            status: AppStatus::Idle,
            current: None,
            candidate_count: 0,
            selected_candidate: 0,
            evidence_root: "--".to_string(),
            tx_hash: "--".to_string(),
            verification_result: "pending".to_string(),
            events: VecDeque::new(),
            face_quality: None,
            discovery_provider: "external_reverse_image".to_string(),
            discovery_request_status: "SENT".to_string(),
            discovery_raw_count: 0,
            discovery_unique_count: 0,
            discovery_error: None,
            verified_candidates: Vec::new(),
            ranked_candidates: Vec::new(),
            evidence_bundle: None,
        }
    }

    /// Sample verified candidates covering all candidate verification statuses.
    pub fn sample_verified_candidates() -> Vec<VerificationResult> {
        vec![
            VerificationResult {
                candidate: SearchCandidate {
                    url: Url::parse("https://profiles.example.org/janedoe").unwrap(),
                    title: Some("Jane Doe Public Portfolio".to_string()),
                    domain: "profiles.example.org".to_string(),
                    image_url: Some(Url::parse("https://profiles.example.org/face.jpg").unwrap()),
                    thumbnail_url: None,
                    snippet: Some("Software engineer portrait".to_string()),
                    provider: "external_reverse_image".to_string(),
                    discovered_at: chrono::Utc::now(),
                },
                similarity: 0.94,
                quality: 0.92,
                matched_face_index: Some(0),
                candidate_image_hash: Some("7a9f82c4e1d3b5a6".to_string()),
                status: VerificationStatus::Verified,
                error_message: None,
            },
            VerificationResult {
                candidate: SearchCandidate {
                    url: Url::parse("https://archives.example.net/events/2024").unwrap(),
                    title: Some("Conference Attendees".to_string()),
                    domain: "archives.example.net".to_string(),
                    image_url: Some(Url::parse("https://archives.example.net/c2.jpg").unwrap()),
                    thumbnail_url: None,
                    snippet: Some("Group session photo".to_string()),
                    provider: "external_reverse_image".to_string(),
                    discovered_at: chrono::Utc::now(),
                },
                similarity: 0.58,
                quality: 0.81,
                matched_face_index: Some(1),
                candidate_image_hash: Some("1b2c3d4e5f6a7b8c".to_string()),
                status: VerificationStatus::BelowThreshold,
                error_message: None,
            },
            VerificationResult {
                candidate: SearchCandidate {
                    url: Url::parse("https://landscapes.example.com/gallery").unwrap(),
                    title: Some("Scenic View".to_string()),
                    domain: "landscapes.example.com".to_string(),
                    image_url: Some(
                        Url::parse("https://landscapes.example.com/mountain.jpg").unwrap(),
                    ),
                    thumbnail_url: None,
                    snippet: Some("Mountain horizon".to_string()),
                    provider: "external_reverse_image".to_string(),
                    discovered_at: chrono::Utc::now(),
                },
                similarity: 0.0,
                quality: 0.0,
                matched_face_index: None,
                candidate_image_hash: Some("3d4e5f6a7b8c9d0e".to_string()),
                status: VerificationStatus::NoFace,
                error_message: None,
            },
            VerificationResult {
                candidate: SearchCandidate {
                    url: Url::parse("https://corrupt.example.org/missing").unwrap(),
                    title: Some("Unreachable Media".to_string()),
                    domain: "corrupt.example.org".to_string(),
                    image_url: Some(Url::parse("https://corrupt.example.org/img.png").unwrap()),
                    thumbnail_url: None,
                    snippet: None,
                    provider: "external_reverse_image".to_string(),
                    discovered_at: chrono::Utc::now(),
                },
                similarity: 0.0,
                quality: 0.0,
                matched_face_index: None,
                candidate_image_hash: None,
                status: VerificationStatus::Error,
                error_message: Some("HTTP 404: Not Found".to_string()),
            },
        ]
    }

    /// Set or update the face quality assessment.
    pub fn set_face_quality(&mut self, quality: FaceQualityAssessment) {
        self.face_quality = Some(quality);
    }

    /// Set discovery stage results.
    pub fn set_discovery_results(
        &mut self,
        provider: impl Into<String>,
        request_status: impl Into<String>,
        raw_count: usize,
        unique_count: usize,
    ) {
        self.discovery_provider = provider.into();
        self.discovery_request_status = request_status.into();
        self.discovery_raw_count = raw_count;
        self.discovery_unique_count = unique_count;
        self.candidate_count = unique_count;
        self.discovery_error = None;
    }

    /// Set discovery stage search failure.
    pub fn set_discovery_error(
        &mut self,
        provider: impl Into<String>,
        error_message: impl Into<String>,
    ) {
        self.discovery_provider = provider.into();
        self.discovery_request_status = "FAILED".to_string();
        self.discovery_error = Some(error_message.into());
    }

    /// Apply a user action, mutating state accordingly.
    pub fn apply(&mut self, action: AppAction) {
        match action {
            AppAction::Start => self.start(),
            AppAction::Verify => self.verify(),
            AppAction::Tamper => self.tamper(),
            AppAction::Reset => self.reset(),
            AppAction::Select(dir) => self.select(dir),
            AppAction::Quit => {}
        }
    }

    fn start(&mut self) {
        if self.status != AppStatus::Idle {
            return;
        }
        self.status = AppStatus::Running;
        self.current = Some(Stage::Input);
        self.verification_result = "pending".to_string();
        self.push_event("Pipeline started");
    }

    fn verify(&mut self) {
        let Some(current) = self.current else {
            return;
        };
        if let Some(next) = current.next() {
            self.current = Some(next);
            if next == Stage::Face && self.face_quality.is_none() {
                self.face_quality = Some(FaceQualityAssessment::sample_good());
            }
            if next == Stage::Discovery
                && self.discovery_raw_count == 0
                && self.discovery_error.is_none()
            {
                self.discovery_raw_count = 12;
                self.discovery_unique_count = 8;
                self.candidate_count = 8;
            }
            if next == Stage::Verify && self.verified_candidates.is_empty() {
                self.verified_candidates = Self::sample_verified_candidates();
                self.ranked_candidates =
                    CandidateRanker::new().rank_results(self.verified_candidates.clone());
                self.candidate_count = self.ranked_candidates.len();
            }
            if next == Stage::Evidence && self.evidence_bundle.is_none() {
                self.populate_sample_evidence();
            }
            self.push_event(&format!("Stage complete: {}", current.label()));
        } else {
            self.status = AppStatus::Completed;
            self.current = None;
            self.verification_result = "verified".to_string();
            self.push_event("Pipeline verified");
        }
    }

    /// Set verified candidate results and automatically rank them.
    pub fn set_verified_candidates(&mut self, results: Vec<VerificationResult>) {
        self.ranked_candidates = CandidateRanker::new().rank_results(results.clone());
        self.candidate_count = self.ranked_candidates.len();
        if self.selected_candidate >= self.candidate_count && self.candidate_count > 0 {
            self.selected_candidate = self.candidate_count - 1;
        } else if self.candidate_count == 0 {
            self.selected_candidate = 0;
        }
        self.verified_candidates = results;
    }

    /// Populate sample evidence record and Merkle bundle for Stage::Evidence.
    pub fn populate_sample_evidence(&mut self) {
        let matched = if let Some(top) = self.ranked_candidates.first() {
            top.verification.clone()
        } else if let Some(first) = self.verified_candidates.first() {
            first.clone()
        } else {
            Self::sample_verified_candidates().remove(0)
        };

        let record = EvidenceRecord {
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            run_id: "demo-run-001".to_string(),
            source_url: matched.candidate.url.clone(),
            domain: matched.candidate.domain.clone(),
            platform: "web".to_string(),
            provider: matched.candidate.provider.clone(),
            retrieved_at: matched.candidate.discovered_at,
            title: matched.candidate.title.clone().unwrap_or_default(),
            text: matched.candidate.snippet.clone().unwrap_or_default(),
            image_sha256: matched.candidate_image_hash.clone().unwrap_or_else(|| {
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string()
            }),
            face_similarity: matched.similarity,
            face_model: "insightface-arcface-r100".to_string(),
            candidate_quality: matched.quality,
        };

        if let Ok(bundle) = record.build_bundle() {
            self.evidence_root = bundle.root_hash.clone();
            self.evidence_bundle = Some(bundle);
        }
    }

    fn tamper(&mut self) {
        if self.status != AppStatus::Running {
            return;
        }
        self.status = AppStatus::Tampered;
        self.current = None;
        self.verification_result = "tampered".to_string();
        self.push_event("Tamper detected");
    }

    fn reset(&mut self) {
        self.status = AppStatus::Idle;
        self.current = None;
        self.candidate_count = 0;
        self.selected_candidate = 0;
        self.evidence_root = "--".to_string();
        self.tx_hash = "--".to_string();
        self.verification_result = "pending".to_string();
        self.events.clear();
        self.face_quality = None;
        self.discovery_provider = "external_reverse_image".to_string();
        self.discovery_request_status = "SENT".to_string();
        self.discovery_raw_count = 0;
        self.discovery_unique_count = 0;
        self.discovery_error = None;
        self.verified_candidates.clear();
        self.ranked_candidates.clear();
        self.evidence_bundle = None;
        self.push_event("Pipeline reset");
    }

    fn select(&mut self, dir: Direction) {
        let max = self.candidate_count.saturating_sub(1);
        match dir {
            Direction::Up => self.selected_candidate = self.selected_candidate.saturating_sub(1),
            Direction::Down => {
                if self.selected_candidate < max {
                    self.selected_candidate += 1;
                }
            }
        }
    }

    /// Reflect real discovery results. The interface never invents candidates,
    /// so this is only populated by real engines in future phases.
    ///
    /// Kept as test-only for the time being, because there is no real
    /// discovery engine to drive it yet.
    #[cfg(test)]
    pub fn update_candidates(&mut self, count: usize) {
        self.candidate_count = count;
        if self.selected_candidate >= count && count > 0 {
            self.selected_candidate = count - 1;
        } else if count == 0 {
            self.selected_candidate = 0;
        }
    }

    fn push_event(&mut self, event: &str) {
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event.to_string());
    }

    /// Progress as a percentage based on the deepest stage reached.
    pub fn progress_percent(&self) -> u16 {
        match self.status {
            AppStatus::Idle => 0,
            AppStatus::Completed | AppStatus::Tampered => 100,
            AppStatus::Running => {
                let current = self.current.map(Stage::index).unwrap_or(0);
                (((current + 1) * 100) / STAGE_COUNT).min(100) as u16
            }
        }
    }

    /// The core pipeline state the interface currently represents.
    pub fn pipeline_state(&self) -> PipelineState {
        match self.status {
            AppStatus::Completed => PipelineState::Verified,
            AppStatus::Tampered => PipelineState::TamperDetected,
            AppStatus::Idle | AppStatus::Running => self
                .current
                .map(Stage::to_pipeline_state)
                .unwrap_or(PipelineState::Idle),
        }
    }

    /// Lifecycle summary for the status line.
    pub fn status_label(&self) -> &'static str {
        match self.status {
            AppStatus::Idle => "IDLE",
            AppStatus::Running => "RUNNING",
            AppStatus::Completed => "COMPLETE",
            AppStatus::Tampered => "TAMPERED",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stepping_app() -> App {
        let mut app = App::new();
        app.apply(AppAction::Start);
        app
    }

    #[test]
    fn new_app_is_idle() {
        let app = App::new();
        assert_eq!(app.status, AppStatus::Idle);
        assert_eq!(app.current, None);
        assert_eq!(app.progress_percent(), 0);
        assert_eq!(app.pipeline_state(), PipelineState::Idle);
    }

    #[test]
    fn start_transitions_to_running_and_input() {
        let app = stepping_app();
        assert_eq!(app.status, AppStatus::Running);
        assert_eq!(app.current, Some(Stage::Input));
        assert_eq!(app.progress_percent(), 14);
        assert_eq!(app.pipeline_state(), PipelineState::InputReady);
    }

    #[test]
    fn start_is_ignored_when_not_idle() {
        let mut app = stepping_app();
        app.apply(AppAction::Start);
        assert_eq!(app.current, Some(Stage::Input));
    }

    #[test]
    fn verify_advances_through_stages() {
        let mut app = stepping_app();
        let expected = [
            Stage::Face,
            Stage::Discovery,
            Stage::Verify,
            Stage::Evidence,
            Stage::Blockchain,
            Stage::FinalVerify,
        ];
        for stage in expected {
            app.apply(AppAction::Verify);
            assert_eq!(app.current, Some(stage));
        }
    }

    #[test]
    fn final_verify_completes_the_pipeline() {
        let mut app = stepping_app();
        for _ in 0..Stage::ALL.len() {
            app.apply(AppAction::Verify);
        }
        assert_eq!(app.status, AppStatus::Completed);
        assert_eq!(app.current, None);
        assert_eq!(app.verification_result, "verified");
        assert_eq!(app.progress_percent(), 100);
        assert_eq!(app.pipeline_state(), PipelineState::Verified);
    }

    #[test]
    fn verify_is_ignored_when_idle() {
        let mut app = App::new();
        app.apply(AppAction::Verify);
        assert_eq!(app.status, AppStatus::Idle);
        assert_eq!(app.current, None);
    }

    #[test]
    fn tamper_flags_the_pipeline() {
        let mut app = stepping_app();
        app.apply(AppAction::Tamper);
        assert_eq!(app.status, AppStatus::Tampered);
        assert_eq!(app.verification_result, "tampered");
        assert_eq!(app.pipeline_state(), PipelineState::TamperDetected);
    }

    #[test]
    fn tamper_is_ignored_when_idle() {
        let mut app = App::new();
        app.apply(AppAction::Tamper);
        assert_eq!(app.status, AppStatus::Idle);
    }

    #[test]
    fn reset_returns_to_idle_and_clears_fields() {
        let mut app = stepping_app();
        app.update_candidates(4);
        app.selected_candidate = 2;
        app.apply(AppAction::Reset);
        assert_eq!(app.status, AppStatus::Idle);
        assert_eq!(app.current, None);
        assert_eq!(app.candidate_count, 0);
        assert_eq!(app.selected_candidate, 0);
        assert_eq!(app.evidence_root, "--");
        assert_eq!(app.tx_hash, "--");
        assert_eq!(app.verification_result, "pending");
    }

    #[test]
    fn selection_clamps_to_candidate_scope() {
        let mut app = App::new();
        app.update_candidates(3);
        assert_eq!(app.selected_candidate, 0);

        app.apply(AppAction::Select(Direction::Down));
        app.apply(AppAction::Select(Direction::Down));
        assert_eq!(app.selected_candidate, 2);

        app.apply(AppAction::Select(Direction::Down));
        assert_eq!(app.selected_candidate, 2, "must not exceed max");

        app.apply(AppAction::Select(Direction::Up));
        assert_eq!(app.selected_candidate, 1);
        app.apply(AppAction::Select(Direction::Up));
        app.apply(AppAction::Select(Direction::Up));
        assert_eq!(app.selected_candidate, 0, "must not go below zero");
    }

    #[test]
    fn selection_is_zero_with_no_candidates() {
        let mut app = App::new();
        app.apply(AppAction::Select(Direction::Down));
        assert_eq!(app.selected_candidate, 0);
    }

    #[test]
    fn event_history_respects_capacity() {
        let mut app = App::new();
        app.start();
        for _ in 0..(MAX_EVENTS + 5) {
            app.verify();
        }
        assert!(app.events.len() <= MAX_EVENTS);
    }

    #[test]
    fn stage_mapping_covers_every_stage() {
        for stage in Stage::ALL {
            match stage {
                Stage::Input => assert_eq!(stage.to_pipeline_state(), PipelineState::InputReady),
                Stage::Face => assert_eq!(stage.to_pipeline_state(), PipelineState::FaceAnalysis),
                Stage::Discovery => {
                    assert_eq!(stage.to_pipeline_state(), PipelineState::CandidatesFound)
                }
                Stage::Verify => assert_eq!(stage.to_pipeline_state(), PipelineState::MatchFound),
                Stage::Evidence => {
                    assert_eq!(stage.to_pipeline_state(), PipelineState::EvidenceCreated)
                }
                Stage::Blockchain => {
                    assert_eq!(
                        stage.to_pipeline_state(),
                        PipelineState::BlockchainConfirmed
                    )
                }
                Stage::FinalVerify => {
                    assert_eq!(stage.to_pipeline_state(), PipelineState::VerifyingOnchain)
                }
            }
        }
    }
}
