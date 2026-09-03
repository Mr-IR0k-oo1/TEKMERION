use chrono::{DateTime, Utc};
use std::time::Instant;

pub const PIPELINE_PHASES: &[&str] = &[
    "INPUT",
    "FACE",
    "DISCOVERY",
    "VERIFY",
    "EVIDENCE",
    "BLOCKCHAIN",
    "FINAL VERIFY",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelinePhase {
    Idle,
    Input,
    Face,
    Discovery,
    Verify,
    Evidence,
    Blockchain,
    FinalVerify,
}

impl PipelinePhase {
    pub fn index(self) -> usize {
        match self {
            PipelinePhase::Idle => 0,
            PipelinePhase::Input => 0,
            PipelinePhase::Face => 1,
            PipelinePhase::Discovery => 2,
            PipelinePhase::Verify => 3,
            PipelinePhase::Evidence => 4,
            PipelinePhase::Blockchain => 5,
            PipelinePhase::FinalVerify => 6,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, PipelinePhase::FinalVerify)
    }

    pub fn status_text(self) -> &'static str {
        match self {
            PipelinePhase::Idle => "READY",
            PipelinePhase::Input => "PROCESSING",
            PipelinePhase::Face => "ANALYZING",
            PipelinePhase::Discovery => "SEARCHING",
            PipelinePhase::Verify => "MATCHING",
            PipelinePhase::Evidence => "BUNDLING",
            PipelinePhase::Blockchain => "SUBMITTING",
            PipelinePhase::FinalVerify => "CONFIRMING",
        }
    }

    pub fn step_duration_ms(self) -> u64 {
        match self {
            PipelinePhase::Idle => 0,
            PipelinePhase::Input => 400,
            PipelinePhase::Face => 600,
            PipelinePhase::Discovery => 800,
            PipelinePhase::Verify => 500,
            PipelinePhase::Evidence => 400,
            PipelinePhase::Blockchain => 700,
            PipelinePhase::FinalVerify => 500,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub title: String,
    pub provider: String,
    pub url: String,
    pub snippet: String,
    pub similarity: f32,
}

#[derive(Debug, Clone)]
pub struct PipelineEvent {
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub level: EventLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLevel {
    Info,
    Success,
    #[allow(dead_code)]
    Warning,
    Error,
}

pub struct App {
    pub should_quit: bool,
    pub phase: PipelinePhase,
    pub candidates: Vec<Candidate>,
    pub selected_candidate: usize,
    pub evidence_root: String,
    pub tx_hash: String,
    pub verification_result: String,
    pub events: Vec<PipelineEvent>,
    pub progress: f32,
    tick_start: Option<Instant>,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            phase: PipelinePhase::Idle,
            candidates: Vec::new(),
            selected_candidate: 0,
            evidence_root: String::new(),
            tx_hash: String::new(),
            verification_result: String::new(),
            events: Vec::new(),
            progress: 0.0,
            tick_start: None,
        }
    }

    pub fn tick(&mut self) {
        if self.phase.is_terminal() || self.phase == PipelinePhase::Idle {
            return;
        }
        let Some(start) = self.tick_start else {
            return;
        };
        let elapsed = start.elapsed().as_millis() as u64;
        let total = self.phase.step_duration_ms();
        if total == 0 {
            return;
        }
        self.progress = (elapsed as f32 / total as f32).min(1.0);
        if elapsed >= total {
            self.advance_phase();
        }
    }

    pub fn start_pipeline(&mut self) {
        if self.phase != PipelinePhase::Idle && !self.phase.is_terminal() {
            return;
        }
        self.reset_internal();
        self.phase = PipelinePhase::Input;
        self.tick_start = Some(Instant::now());
        self.progress = 0.0;
        self.push_event("Pipeline started".into(), EventLevel::Info);
    }

    pub fn verify(&mut self) {
        if !self.phase.is_terminal() {
            return;
        }
        if self.verification_result == "CONFIRMED" {
            return;
        }
        self.verification_result = "CONFIRMED".into();
        self.push_event("Verification result: CONFIRMED".into(), EventLevel::Success);
    }

    pub fn tamper(&mut self) {
        if self.phase.is_terminal() && !self.evidence_root.is_empty() {
            self.evidence_root = format!("TAMPERED_{}", &self.evidence_root[..12]);
            self.verification_result = "TAMPER DETECTED".into();
            self.push_event(
                "Evidence tampered - integrity compromised".into(),
                EventLevel::Error,
            );
        }
    }

    pub fn reset(&mut self) {
        self.reset_internal();
        self.push_event("Pipeline reset".into(), EventLevel::Info);
    }

    pub fn select_up(&mut self) {
        if self.selected_candidate > 0 {
            self.selected_candidate -= 1;
        }
    }

    pub fn select_down(&mut self) {
        if !self.candidates.is_empty() && self.selected_candidate < self.candidates.len() - 1 {
            self.selected_candidate += 1;
        }
    }

    pub fn push_event(&mut self, message: String, level: EventLevel) {
        self.events.push(PipelineEvent {
            message,
            timestamp: Utc::now(),
            level,
        });
        let max_events = 200;
        if self.events.len() > max_events {
            self.events.drain(0..self.events.len() - max_events);
        }
    }

    fn advance_phase(&mut self) {
        let next = match self.phase {
            PipelinePhase::Input => PipelinePhase::Face,
            PipelinePhase::Face => PipelinePhase::Discovery,
            PipelinePhase::Discovery => {
                self.populate_candidates();
                self.push_event(
                    format!("Discovered {} candidates", self.candidates.len()),
                    EventLevel::Info,
                );
                PipelinePhase::Verify
            }
            PipelinePhase::Verify => {
                self.push_event("Verification complete".into(), EventLevel::Info);
                PipelinePhase::Evidence
            }
            PipelinePhase::Evidence => {
                self.evidence_root =
                    "0x7a3b".to_string() + &hex_hash(format!("evidence_{}", Utc::now()));
                self.push_event(
                    format!("Evidence root: {}", &self.evidence_root[..16]),
                    EventLevel::Success,
                );
                PipelinePhase::Blockchain
            }
            PipelinePhase::Blockchain => {
                self.tx_hash = "0x".to_string() + &hex_hash(format!("tx_{}", Utc::now()));
                self.push_event(
                    format!("TX submitted: {}", &self.tx_hash[..12]),
                    EventLevel::Info,
                );
                PipelinePhase::FinalVerify
            }
            PipelinePhase::FinalVerify => {
                self.verification_result = "PENDING".into();
                self.progress = 1.0;
                self.push_event(
                    "Pipeline complete - press V to verify".into(),
                    EventLevel::Success,
                );
                self.phase = PipelinePhase::FinalVerify;
                return;
            }
            _ => return,
        };
        self.phase = next;
        self.tick_start = Some(Instant::now());
        self.progress = 0.0;
        self.push_event(format!("Phase: {}", next.status_text()), EventLevel::Info);
    }

    fn populate_candidates(&mut self) {
        self.candidates = vec![
            Candidate {
                title: "Social Media Profile #1".into(),
                provider: "Google Vision".into(),
                url: "https://example.com/profile/1".into(),
                snippet: "Professional headshot, studio background".into(),
                similarity: 0.94,
            },
            Candidate {
                title: "News Article Photo".into(),
                provider: "Bing Visual".into(),
                url: "https://news.example.com/article/42".into(),
                snippet: "Event photography, natural lighting".into(),
                similarity: 0.87,
            },
            Candidate {
                title: "Public Records Image".into(),
                provider: "Yandex".into(),
                url: "https://records.example.com/doc/99".into(),
                snippet: "Official document photo".into(),
                similarity: 0.91,
            },
            Candidate {
                title: "Conference Badge".into(),
                provider: "TinEye".into(),
                url: "https://events.example.com/speaker/7".into(),
                snippet: "Conference badge photo, cropped".into(),
                similarity: 0.82,
            },
        ];
        self.selected_candidate = 0;
    }

    fn reset_internal(&mut self) {
        self.phase = PipelinePhase::Idle;
        self.candidates.clear();
        self.selected_candidate = 0;
        self.evidence_root.clear();
        self.tx_hash.clear();
        self.verification_result.clear();
        self.events.clear();
        self.progress = 0.0;
        self.tick_start = None;
    }
}

fn hex_hash(input: String) -> String {
    let bytes = input.as_bytes();
    let mut result = String::with_capacity(12);
    for i in 0..6 {
        let b = bytes[i % bytes.len()].wrapping_add(i as u8);
        result.push_str(&format!("{:02x}", b));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_app() {
        let app = App::new();
        assert_eq!(app.phase, PipelinePhase::Idle);
        assert!(app.candidates.is_empty());
        assert!(app.evidence_root.is_empty());
        assert!(app.tx_hash.is_empty());
        assert!(app.verification_result.is_empty());
        assert!((app.progress - 0.0).abs() < f32::EPSILON);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_start_pipeline() {
        let mut app = App::new();
        app.start_pipeline();
        assert_eq!(app.phase, PipelinePhase::Input);
        assert!(!app.events.is_empty());
    }

    #[test]
    fn test_reset() {
        let mut app = App::new();
        app.start_pipeline();
        app.reset();
        assert_eq!(app.phase, PipelinePhase::Idle);
        assert!(app.candidates.is_empty());
    }

    #[test]
    fn test_select_up_down() {
        let mut app = App::new();
        app.populate_candidates();
        assert_eq!(app.selected_candidate, 0);
        app.select_down();
        assert_eq!(app.selected_candidate, 1);
        app.select_down();
        assert_eq!(app.selected_candidate, 2);
        app.select_up();
        assert_eq!(app.selected_candidate, 1);
    }

    #[test]
    fn test_select_bounds() {
        let mut app = App::new();
        app.select_up();
        assert_eq!(app.selected_candidate, 0);
        app.select_down();
        assert_eq!(app.selected_candidate, 0);
    }

    #[test]
    fn test_verify_terminal() {
        let mut app = App::new();
        app.verify();
        assert_eq!(app.verification_result, "");
        app.phase = PipelinePhase::FinalVerify;
        app.evidence_root = "0x1234".into();
        app.verify();
        assert_eq!(app.verification_result, "CONFIRMED");
    }

    #[test]
    fn test_tamper() {
        let mut app = App::new();
        app.phase = PipelinePhase::FinalVerify;
        app.evidence_root = "0x7a3b12345678".into();
        app.tamper();
        assert!(app.evidence_root.starts_with("TAMPERED_"));
        assert_eq!(app.verification_result, "TAMPER DETECTED");
    }

    #[test]
    fn test_tamper_no_evidence() {
        let mut app = App::new();
        app.phase = PipelinePhase::FinalVerify;
        app.tamper();
        assert!(app.evidence_root.is_empty());
    }

    #[test]
    fn test_push_event() {
        let mut app = App::new();
        app.push_event("test".into(), EventLevel::Info);
        assert_eq!(app.events.len(), 1);
        assert_eq!(app.events[0].message, "test");
    }

    #[test]
    fn test_event_limit() {
        let mut app = App::new();
        for i in 0..300 {
            app.push_event(format!("event {i}"), EventLevel::Info);
        }
        assert!(app.events.len() <= 200);
    }

    #[test]
    fn test_advance_populates_candidates() {
        let mut app = App::new();
        app.phase = PipelinePhase::Discovery;
        app.tick_start = Some(Instant::now() - std::time::Duration::from_secs(5));
        app.tick();
        assert!(!app.candidates.is_empty());
    }

    #[test]
    fn test_pipeline_phase_index() {
        assert_eq!(PipelinePhase::Input.index(), 0);
        assert_eq!(PipelinePhase::Face.index(), 1);
        assert_eq!(PipelinePhase::Discovery.index(), 2);
        assert_eq!(PipelinePhase::Verify.index(), 3);
        assert_eq!(PipelinePhase::Evidence.index(), 4);
        assert_eq!(PipelinePhase::Blockchain.index(), 5);
        assert_eq!(PipelinePhase::FinalVerify.index(), 6);
    }

    #[test]
    fn test_pipeline_phase_terminal() {
        assert!(!PipelinePhase::Idle.is_terminal());
        assert!(!PipelinePhase::Input.is_terminal());
        assert!(PipelinePhase::FinalVerify.is_terminal());
    }

    #[test]
    fn test_pipeline_phase_status_text() {
        assert_eq!(PipelinePhase::Idle.status_text(), "READY");
        assert_eq!(PipelinePhase::Face.status_text(), "ANALYZING");
    }

    #[test]
    fn test_hex_hash_deterministic() {
        let h1 = hex_hash("test".into());
        let h2 = hex_hash("test".into());
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 12);
    }

    #[test]
    fn test_hex_hash_different_inputs() {
        let h1 = hex_hash("a".into());
        let h2 = hex_hash("b".into());
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_tick_no_advance_if_idle() {
        let mut app = App::new();
        app.tick();
        assert_eq!(app.phase, PipelinePhase::Idle);
    }

    #[test]
    fn test_tick_no_advance_if_terminal() {
        let mut app = App::new();
        app.phase = PipelinePhase::FinalVerify;
        app.tick();
        assert_eq!(app.phase, PipelinePhase::FinalVerify);
    }

    #[test]
    fn test_verify_already_confirmed() {
        let mut app = App::new();
        app.phase = PipelinePhase::FinalVerify;
        app.evidence_root = "0x1234".into();
        app.verify();
        assert_eq!(app.verification_result, "CONFIRMED");
        let events_before = app.events.len();
        app.verify();
        assert_eq!(app.verification_result, "CONFIRMED");
        assert_eq!(app.events.len(), events_before);
    }
}
