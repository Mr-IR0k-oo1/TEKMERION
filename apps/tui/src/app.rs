use std::collections::VecDeque;

use tekmerion_core::PipelineState;
use tekmerion_face::FaceQualityAssessment;

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
        }
    }

    /// Set or update the face quality assessment.
    pub fn set_face_quality(&mut self, quality: FaceQualityAssessment) {
        self.face_quality = Some(quality);
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
            self.push_event(&format!("Stage complete: {}", current.label()));
        } else {
            self.status = AppStatus::Completed;
            self.current = None;
            self.verification_result = "verified".to_string();
            self.push_event("Pipeline verified");
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
