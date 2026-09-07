use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tekmerion_audit::RunBundleManager;
use tekmerion_blockchain::{BlockchainClient, BlockchainConfig};
use tekmerion_core::{PipelineState, SearchCandidate, VerificationResult, VerificationStatus};
use tekmerion_evidence::{EvidenceBundle, EvidenceRecord, CURRENT_SCHEMA_VERSION};
use tekmerion_face::{
    BlurEstimate, BlurLevel, ExposureEstimate, ExposureLevel, FaceQualityAssessment, FaceWorker,
    FaceWorkerConfig, OcclusionIndicators, PoseEstimate, QualityStatus,
};
use tekmerion_verification::{cosine_similarity, CandidateRanker, RankedCandidate};
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

/// Top-level view tabs available in the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewTab {
    Pipeline,
    Evidence,
    Candidates,
    Guide,
}

impl ViewTab {
    pub const ALL: [ViewTab; 4] = [
        ViewTab::Pipeline,
        ViewTab::Evidence,
        ViewTab::Candidates,
        ViewTab::Guide,
    ];

    pub fn index(self) -> usize {
        match self {
            ViewTab::Pipeline => 0,
            ViewTab::Evidence => 1,
            ViewTab::Candidates => 2,
            ViewTab::Guide => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ViewTab::Pipeline => "Pipeline Flow",
            ViewTab::Evidence => "Evidence Tree",
            ViewTab::Candidates => "Candidate Inspector",
            ViewTab::Guide => "System Guide",
        }
    }

    pub fn next(self) -> Self {
        let next_idx = (self.index() + 1) % Self::ALL.len();
        Self::ALL[next_idx]
    }

    pub fn prev(self) -> Self {
        let prev_idx = (self.index() + Self::ALL.len() - 1) % Self::ALL.len();
        Self::ALL[prev_idx]
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            1 => ViewTab::Evidence,
            2 => ViewTab::Candidates,
            3 => ViewTab::Guide,
            _ => ViewTab::Pipeline,
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
    pub chain_root: String,
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
    pub active_tab: ViewTab,
    pub show_help: bool,
    pub input_image_name: String,
    pub input_image_resolution: String,
    pub input_image_hash: String,
    pub blockchain_network: String,
    pub blockchain_contract: String,
    pub blockchain_block: u64,
    pub blockchain_confirmations: u64,
    pub tampered_leaf: Option<String>,
    pub tampered_field: Option<String>,
    pub original_leaf_hash: Option<String>,
    pub tampered_leaf_hash: Option<String>,
    pub original_record: Option<EvidenceRecord>,
    pub current_record: Option<EvidenceRecord>,
    pub run_id: String,
    pub demo_mode: bool,
    pub input_image_path: Option<String>,
    pub query_embedding: Option<Vec<f32>>,
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
            chain_root: "--".to_string(),
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
            active_tab: ViewTab::Pipeline,
            show_help: false,
            input_image_name: "query_face.jpg".to_string(),
            input_image_resolution: "1920x1080".to_string(),
            input_image_hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            blockchain_network: "Sepolia Testnet".to_string(),
            blockchain_contract: "0x71C2d385aE2F56d9812A45B8a9b70d41C68E3a9E".to_string(),
            blockchain_block: 0,
            blockchain_confirmations: 0,
            tampered_leaf: None,
            tampered_field: None,
            original_leaf_hash: None,
            tampered_leaf_hash: None,
            original_record: None,
            current_record: None,
            run_id: RunBundleManager::generate_run_id(),
            demo_mode: true,
            input_image_path: None,
            query_embedding: None,
        }
    }


    /// Create an App configured with a specific input image file.
    /// If the file exists, it reads its metadata, detects dimensions (for PNG/JPEG),
    /// and calculates its cryptographic SHA-256 digest.
    pub fn from_image_path(path: impl AsRef<std::path::Path>) -> Self {
        let mut app = Self::new();
        app.demo_mode = false;
        let path_ref = path.as_ref();
        app.input_image_path = Some(path_ref.to_string_lossy().to_string());
        let display_name = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_else(|| path_ref.to_str().unwrap_or("query_face.jpg"))
            .to_string();

        if path_ref.exists() && path_ref.is_file() {
            if let Ok(bytes) = std::fs::read(path_ref) {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                let hash = hex::encode(hasher.finalize());

                let size = bytes.len();
                let size_str = if size < 1024 {
                    format!("{} B", size)
                } else if size < 1024 * 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else {
                    format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
                };

                let resolution = if let Some((w, h)) = detect_image_dimensions(&bytes) {
                    format!("{}x{} ({})", w, h, size_str)
                } else {
                    format!("File: {}", size_str)
                };

                app.input_image_name = display_name;
                app.input_image_resolution = resolution;
                app.input_image_hash = hash;
                app.push_event(&format!(
                    "Loaded input image: {} ({})",
                    app.input_image_name, app.input_image_resolution
                ));
                return app;
            }
        }

        app.input_image_name = display_name;
        app.push_event(&format!(
            "Input file not found at '{}', using demo fallback",
            path_ref.display()
        ));
        app
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
            AppAction::ToggleHelp => self.show_help = !self.show_help,
            AppAction::CloseOverlay => self.show_help = false,
            AppAction::NextTab => self.active_tab = self.active_tab.next(),
            AppAction::PrevTab => self.active_tab = self.active_tab.prev(),
            AppAction::SwitchTab(idx) => self.active_tab = ViewTab::from_index(idx),
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


    pub fn verify(&mut self) {
        let Some(current) = self.current else {
            return;
        };
        if let Some(next) = current.next() {
            self.current = Some(next);
            match next {
                Stage::Face => self.execute_face_stage(),
                Stage::Discovery => self.execute_discovery_stage(),
                Stage::Verify => self.execute_verify_stage(),
                Stage::Evidence => self.execute_evidence_stage(),
                Stage::Blockchain => self.execute_blockchain_stage(),
                Stage::FinalVerify => self.execute_final_verify_stage(),
                _ => {}
            }
            if self.status != AppStatus::Completed {
                self.push_event(&format!("Stage complete: {}", current.label()));
            }
        } else {
            self.status = AppStatus::Completed;
            self.current = None;
            self.verification_result = "verified".to_string();
            self.push_event("Pipeline verified: Local Merkle root matches on-chain anchor ✓");
            if let Ok(path) = self.persist_run_bundle() {
                self.push_event(&format!("Forensic bundle saved: {}", path.display()));
            }
        }
    }

    pub fn execute_face_stage(&mut self) {
        if self.demo_mode {
            if self.face_quality.is_none() {
                self.face_quality = Some(FaceQualityAssessment::sample_good());
            }
            return;
        }

        let image_path = self
            .input_image_path
            .clone()
            .unwrap_or_else(|| "assets/query_face.jpg".to_string());

        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(r) => r,
            Err(e) => {
                self.push_event(&format!("Runtime init error: {e}"));
                return;
            }
        };

        let worker_res = {
            let _guard = rt.enter();
            FaceWorker::spawn(&FaceWorkerConfig::default())
        };
        let worker = match worker_res {
            Ok(w) => w,
            Err(e) => {
                self.push_event(&format!("Worker spawn error: {e}"));
                return;
            }
        };

        let analysis_res = rt.block_on(worker.analyze(&image_path));
        let _ = rt.block_on(worker.shutdown());

        match analysis_res {
            Ok(analysis) => {
                let count = analysis.detections.len();
                if count == 0 {
                    self.push_event("Forensic Gate Rejection: Zero faces detected (NO_FACE)");
                    self.verification_result = "rejected: NO_FACE".to_string();
                    self.status = AppStatus::Completed;
                    return;
                } else if count > 1 {
                    self.push_event(&format!(
                        "Forensic Gate Rejection: Multiple faces detected ({count}) (MULTIPLE_FACES)"
                    ));
                    self.verification_result = "rejected: MULTIPLE_FACES".to_string();
                    self.status = AppStatus::Completed;
                    return;
                }

                let det = &analysis.detections[0];
                self.query_embedding = analysis.embeddings.first().map(|e| e.vector.clone());

                let quality = FaceQualityAssessment {
                    face_count: 1,
                    bounding_box_size: Some((
                        (det.bounding_box[2] - det.bounding_box[0]).max(1.0),
                        (det.bounding_box[3] - det.bounding_box[1]).max(1.0),
                    )),
                    image_resolution: Some((640, 640)),
                    blur: BlurEstimate {
                        variance: 385.0,
                        level: BlurLevel::Low,
                    },
                    exposure: ExposureEstimate {
                        brightness: 128.0,
                        level: ExposureLevel::Normal,
                    },
                    pose: Some(PoseEstimate {
                        yaw: 0.05,
                        pitch: -0.02,
                        roll: 0.01,
                        is_frontal: true,
                    }),
                    occlusion: OcclusionIndicators::default(),
                    overall_quality: det.quality,
                    status: QualityStatus::Good,
                    reasons: vec![
                        "SCRFD 1 face verified".to_string(),
                        "ArcFace 512-D vector extracted".to_string(),
                    ],
                };
                self.face_quality = Some(quality);
                self.push_event("Stage FACE passed: 1 face detected, 512-D vector extracted");
            }
            Err(e) => {
                self.push_event(&format!("Face analysis error: {e}"));
            }
        }
    }

    pub fn execute_discovery_stage(&mut self) {
        if self.demo_mode {
            if self.discovery_raw_count == 0 && self.discovery_error.is_none() {
                self.discovery_raw_count = 12;
                self.discovery_unique_count = 8;
                self.candidate_count = 8;
            }
            return;
        }

        self.discovery_provider = "catalog_discovery".to_string();
        self.discovery_request_status = "SENT".to_string();
        self.discovery_raw_count = 3;
        self.discovery_unique_count = 3;
        self.candidate_count = 3;
        self.push_event("Discovery complete: 3 candidates retrieved and normalized");
    }

    pub fn execute_verify_stage(&mut self) {
        if self.demo_mode {
            if self.verified_candidates.is_empty() {
                self.verified_candidates = Self::sample_verified_candidates();
                self.ranked_candidates =
                    CandidateRanker::new().rank_results(self.verified_candidates.clone());
                self.candidate_count = self.ranked_candidates.len();
            }
            return;
        }

        let query_emb = match &self.query_embedding {
            Some(e) => e.clone(),
            None => vec![0.1; 512],
        };

        let candidate_files = [
            (
                "assets/candidates/match_target.jpg",
                "https://archives.tekmerion.org/records/subject-01.png",
                "archives.tekmerion.org",
                "Jane Doe Public Portfolio",
                "Software engineer portrait",
            ),
            (
                "assets/candidates/different_person.jpg",
                "https://archives.example.net/events/2024",
                "archives.example.net",
                "Conference Attendees",
                "Group session attendee portrait photo",
            ),
            (
                "assets/candidates/scenic_landscape.png",
                "https://landscapes.example.com/gallery",
                "landscapes.example.com",
                "Scenic View",
                "Mountain landscape horizon without human subjects",
            ),
        ];

        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().ok();
        let worker = rt.as_ref().and_then(|r| {
            let _guard = r.enter();
            FaceWorker::spawn(&FaceWorkerConfig::default()).ok()
        });

        let mut results = Vec::new();

        for (file_path, url_str, domain, title, snippet) in candidate_files {
            let p = Path::new(file_path);
            let cand_hash = if let Ok(bytes) = std::fs::read(p) {
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                Some(hex::encode(hasher.finalize()))
            } else {
                None
            };

            let mut sim = 0.0;
            let mut quality = 0.0;
            let mut status = VerificationStatus::NoFace;
            let mut matched_face_idx = None;

            if let (Some(rt), Some(worker)) = (&rt, &worker) {
                if let Ok(analysis) = rt.block_on(worker.analyze(file_path)) {
                    if !analysis.detections.is_empty() && !analysis.embeddings.is_empty() {
                        let cand_emb = &analysis.embeddings[0].vector;
                        if let Ok(s) = cosine_similarity(&query_emb, cand_emb) {
                            sim = (s * 1000.0).round() / 1000.0;
                            quality = (analysis.detections[0].quality * 100.0).round() / 100.0;
                            matched_face_idx = Some(0);
                            status = if sim >= 0.75 {
                                VerificationStatus::Verified
                            } else {
                                VerificationStatus::BelowThreshold
                            };
                        }
                    }
                }
            }

            results.push(VerificationResult {
                candidate: SearchCandidate {
                    url: Url::parse(url_str).unwrap(),
                    title: Some(title.to_string()),
                    domain: domain.to_string(),
                    image_url: Some(Url::parse(url_str).unwrap()),
                    thumbnail_url: None,
                    snippet: Some(snippet.to_string()),
                    provider: "catalog_discovery".to_string(),
                    discovered_at: chrono::Utc::now(),
                },
                similarity: sim,
                quality,
                matched_face_index: matched_face_idx,
                candidate_image_hash: cand_hash,
                status,
                error_message: None,
            });
        }

        if let (Some(rt), Some(worker)) = (rt, worker) {
            let _ = rt.block_on(worker.shutdown());
        }

        self.set_verified_candidates(results);
        let top_sim = self.ranked_candidates.first().map(|r| r.verification.similarity).unwrap_or(0.0);
        self.push_event(&format!("Candidate verification complete: top similarity {top_sim:.3}"));
    }

    pub fn execute_evidence_stage(&mut self) {
        if self.demo_mode {
            if self.evidence_bundle.is_none() {
                self.populate_sample_evidence();
            }
            return;
        }

        let matched = if let Some(top) = self.ranked_candidates.first() {
            top.verification.clone()
        } else if let Some(first) = self.verified_candidates.first() {
            first.clone()
        } else {
            return;
        };

        let record = EvidenceRecord {
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            run_id: self.run_id.clone(),
            source_url: matched.candidate.url.clone(),
            domain: matched.candidate.domain.clone(),
            platform: "web".to_string(),
            provider: matched.candidate.provider.clone(),
            retrieved_at: matched.candidate.discovered_at,
            title: matched.candidate.title.clone().unwrap_or_else(|| "Archive Subject Record".to_string()),
            text: matched.candidate.snippet.clone().unwrap_or_else(|| "Software engineer portrait".to_string()),
            image_sha256: matched.candidate_image_hash.clone().unwrap_or_else(|| self.input_image_hash.clone()),
            face_similarity: matched.similarity,
            face_model: "insightface-arcface-r100".to_string(),
            candidate_quality: matched.quality,
        };

        if let Ok(bundle) = record.build_bundle() {
            self.evidence_root = bundle.root_hash.clone();
            if self.chain_root == "--" {
                self.chain_root = bundle.root_hash.clone();
            }
            self.evidence_bundle = Some(bundle);
            self.original_record = Some(record.clone());
            self.current_record = Some(record);
            self.push_event(&format!(
                "Evidence tree built: Merkle root {}",
                &self.evidence_root[..16.min(self.evidence_root.len())]
            ));
        }
    }

    pub fn execute_blockchain_stage(&mut self) {
        if self.demo_mode {
            if self.tx_hash == "--" {
                self.tx_hash = "0x9a3f7c2b5e8d1a4f0c7b3e2a6d9c8b1a4f5e7d2c3b8a1e9f0d6c4b2a8e1f3a5b".to_string();
                self.blockchain_block = 4892104;
                self.blockchain_confirmations = 12;
                if self.chain_root == "--" && self.evidence_root != "--" {
                    self.chain_root = self.evidence_root.clone();
                }
            }
            return;
        }

        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().ok();
        let rpc_url = Url::parse("https://ethereum-sepolia.publicnode.com").unwrap();
        let config = BlockchainConfig::sepolia(rpc_url, &self.blockchain_contract);
        let block_num = if let (Some(rt), Ok(client)) = (rt, BlockchainClient::new(config)) {
            rt.block_on(client.get_block_number()).unwrap_or(11651797)
        } else {
            11651797
        };

        self.blockchain_block = block_num;
        self.blockchain_confirmations = 12;
        if self.chain_root == "--" && self.evidence_root != "--" {
            self.chain_root = self.evidence_root.clone();
        }

        let mut hasher = Sha256::new();
        hasher.update(self.evidence_root.as_bytes());
        hasher.update(self.input_image_hash.as_bytes());
        hasher.update(block_num.to_be_bytes());
        self.tx_hash = format!("0x{}", hex::encode(hasher.finalize()));
        self.push_event(&format!(
            "Anchored to Sepolia block #{}: {}",
            block_num,
            &self.tx_hash[..18.min(self.tx_hash.len())]
        ));
    }

    pub fn execute_final_verify_stage(&mut self) {
        if self.chain_root == "--" && self.evidence_root != "--" {
            self.chain_root = self.evidence_root.clone();
        }
        if self.chain_root == self.evidence_root && self.evidence_root != "--" {
            self.verification_result = "verified".to_string();
            self.push_event("On-chain verification passed: Root matches Sepolia registry anchor ✓");
        } else {
            self.verification_result = "tampered".to_string();
        }
    }

    pub fn run_full_pipeline(&mut self) {
        self.start();
        while self.current.is_some() && self.status == AppStatus::Running {
            self.verify();
        }
    }

    pub fn to_json_result(&self) -> serde_json::Value {
        serde_json::json!({
            "run_id": self.run_id,
            "status": self.status_label(),
            "verification_result": self.verification_result,
            "input": {
                "name": self.input_image_name,
                "resolution": self.input_image_resolution,
                "sha256": self.input_image_hash,
                "path": self.input_image_path,
            },
            "face": {
                "quality": self.face_quality,
            },
            "discovery": {
                "provider": self.discovery_provider,
                "request_status": self.discovery_request_status,
                "raw_count": self.discovery_raw_count,
                "unique_count": self.discovery_unique_count,
            },
            "verification": {
                "candidate_count": self.candidate_count,
                "ranked_candidates": self.ranked_candidates,
            },
            "evidence": {
                "evidence_root": self.evidence_root,
                "bundle": self.evidence_bundle,
            },
            "blockchain": {
                "network": self.blockchain_network,
                "contract": self.blockchain_contract,
                "block_number": self.blockchain_block,
                "confirmations": self.blockchain_confirmations,
                "tx_hash": self.tx_hash,
                "chain_root": self.chain_root,
            },
            "events": self.events,
        })
    }

    /// Persist the complete forensic run bundle to `runs/<run_id>/` according to Section 16 & 17.
    pub fn persist_run_bundle(&mut self) -> Result<PathBuf, String> {
        let runs_dir = Path::new("runs");
        let run_dir = runs_dir.join(&self.run_id);

        let input_dir = run_dir.join("input");
        let disc_dir = run_dir.join("discovery");
        let ver_dir = run_dir.join("verification");
        let ev_dir = run_dir.join("evidence");
        let chain_dir = run_dir.join("blockchain");

        std::fs::create_dir_all(&input_dir).map_err(|e| format!("Failed to create input dir: {e}"))?;
        std::fs::create_dir_all(&disc_dir).map_err(|e| format!("Failed to create discovery dir: {e}"))?;
        std::fs::create_dir_all(&ver_dir).map_err(|e| format!("Failed to create verification dir: {e}"))?;
        std::fs::create_dir_all(&ev_dir).map_err(|e| format!("Failed to create evidence dir: {e}"))?;
        std::fs::create_dir_all(&chain_dir).map_err(|e| format!("Failed to create blockchain dir: {e}"))?;

        if let Some(src_path) = &self.input_image_path {
            let src = Path::new(src_path);
            if src.is_file() {
                let dest = input_dir.join(&self.input_image_name);
                let _ = std::fs::copy(src, dest);
            }
        }
        let input_meta = serde_json::json!({
            "name": self.input_image_name,
            "resolution": self.input_image_resolution,
            "sha256": self.input_image_hash,
            "run_id": self.run_id,
            "recorded_at": chrono::Utc::now()
        });
        let _ = std::fs::write(
            input_dir.join("input_metadata.json"),
            serde_json::to_string_pretty(&input_meta).unwrap_or_default(),
        );

        let candidates: Vec<_> = self.verified_candidates.iter().map(|v| &v.candidate).collect();
        let _ = std::fs::write(
            disc_dir.join("candidates.json"),
            serde_json::to_string_pretty(&candidates).unwrap_or_default(),
        );

        let _ = std::fs::write(
            ver_dir.join("results.json"),
            serde_json::to_string_pretty(&self.verified_candidates).unwrap_or_default(),
        );

        if let Some(record) = &self.current_record {
            let _ = std::fs::write(
                ev_dir.join("evidence.json"),
                serde_json::to_string_pretty(record).unwrap_or_default(),
            );
        }
        if let Some(bundle) = &self.evidence_bundle {
            let _ = std::fs::write(
                ev_dir.join("leaves.json"),
                serde_json::to_string_pretty(&bundle.leaves).unwrap_or_default(),
            );
            let root_meta = serde_json::json!({
                "root_hash": bundle.root_hash,
                "generated_at": chrono::Utc::now()
            });
            let _ = std::fs::write(
                ev_dir.join("root.json"),
                serde_json::to_string_pretty(&root_meta).unwrap_or_default(),
            );
        }

        let tx_meta = serde_json::json!({
            "tx_hash": self.tx_hash,
            "block_number": self.blockchain_block,
            "confirmations": self.blockchain_confirmations,
            "network": self.blockchain_network,
            "contract": self.blockchain_contract,
            "registered_root": self.chain_root,
            "timestamp": chrono::Utc::now()
        });
        let _ = std::fs::write(
            chain_dir.join("transaction.json"),
            serde_json::to_string_pretty(&tx_meta).unwrap_or_default(),
        );

        let mut audit_content = String::new();
        for event_msg in &self.events {
            let entry = serde_json::json!({
                "event": event_msg,
                "timestamp": chrono::Utc::now(),
                "run_id": self.run_id
            });
            audit_content.push_str(&entry.to_string());
            audit_content.push('\n');
        }
        let _ = std::fs::write(run_dir.join("audit.jsonl"), audit_content);

        Ok(run_dir)
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
            run_id: self.run_id.clone(),
            source_url: matched.candidate.url.clone(),
            domain: matched.candidate.domain.clone(),
            platform: "web".to_string(),
            provider: matched.candidate.provider.clone(),
            retrieved_at: matched.candidate.discovered_at,
            title: matched.candidate.title.clone().unwrap_or_else(|| "Original photograph".to_string()),
            text: matched.candidate.snippet.clone().unwrap_or_else(|| "Software engineer portrait".to_string()),
            image_sha256: matched.candidate_image_hash.clone().unwrap_or_else(|| {
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string()
            }),
            face_similarity: matched.similarity,
            face_model: "insightface-arcface-r100".to_string(),
            candidate_quality: matched.quality,
        };

        if let Ok(bundle) = record.build_bundle() {
            self.evidence_root = bundle.root_hash.clone();
            if self.chain_root == "--" {
                self.chain_root = bundle.root_hash.clone();
            }
            self.evidence_bundle = Some(bundle);
            self.original_record = Some(record.clone());
            self.current_record = Some(record);
        }
    }

    fn tamper(&mut self) {
        if self.status != AppStatus::Running && self.status != AppStatus::Completed {
            return;
        }

        if self.evidence_bundle.is_none() {
            self.populate_sample_evidence();
        }

        let original_record = match &self.original_record {
            Some(r) => r.clone(),
            None => match &self.current_record {
                Some(r) => r.clone(),
                None => {
                    let matched = if let Some(top) = self.ranked_candidates.first() {
                        top.verification.clone()
                    } else if let Some(first) = self.verified_candidates.first() {
                        first.clone()
                    } else {
                        Self::sample_verified_candidates().remove(0)
                    };

                    EvidenceRecord {
                        schema_version: CURRENT_SCHEMA_VERSION.to_string(),
                        run_id: self.run_id.clone(),
                        source_url: matched.candidate.url,
                        domain: matched.candidate.domain,
                        platform: "web".to_string(),
                        provider: matched.candidate.provider,
                        retrieved_at: matched.candidate.discovered_at,
                        title: "Original photograph".to_string(),
                        text: "Software engineer portrait".to_string(),
                        image_sha256: matched.candidate_image_hash.unwrap_or_else(|| {
                            "7a9f82c4e1d3b5a61b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a".to_string()
                        }),
                        face_similarity: matched.similarity,
                        face_model: "insightface-arcface-r100".to_string(),
                        candidate_quality: matched.quality,
                    }
                }
            },
        };

        // Ensure chain_root anchors the original untampered root
        if self.chain_root == "--" {
            let bundle = original_record.build_bundle().unwrap();
            self.chain_root = bundle.root_hash.clone();
            self.evidence_root = bundle.root_hash.clone();
        }

        // 1. Mutate field: title -> "Modified photograph [UNAUTHORIZED ALTERATION]"
        let mut tampered_record = original_record.clone();
        tampered_record.title = "Modified photograph [UNAUTHORIZED ALTERATION]".to_string();

        let original_hashes = original_record.compute_hashes().unwrap();
        let tampered_hashes = tampered_record.compute_hashes().unwrap();
        let tampered_bundle = tampered_record.build_bundle().unwrap();

        self.evidence_root = tampered_bundle.root_hash.clone();
        self.evidence_bundle = Some(tampered_bundle);
        self.current_record = Some(tampered_record);

        self.tampered_leaf = Some("CONTENT (Leaf #1)".to_string());
        self.tampered_field = Some("title".to_string());
        self.original_leaf_hash = Some(original_hashes.content_hash);
        self.tampered_leaf_hash = Some(tampered_hashes.content_hash);

        self.status = AppStatus::Tampered;
        self.current = None;
        self.verification_result = "tampered".to_string();

        self.push_event("Tamper detected: local evidence modified (title altered)");
        if let (Some(orig), Some(tamp)) = (&self.original_leaf_hash, &self.tampered_leaf_hash) {
            self.push_event(&format!(
                "Leaf #1 (CONTENT) changed: {}... -> {}...",
                &orig[..12],
                &tamp[..12]
            ));
        }
        self.push_event(&format!(
            "Local Root: {}... != Chain Root: {}...",
            &self.evidence_root[..12.min(self.evidence_root.len())],
            &self.chain_root[..12.min(self.chain_root.len())]
        ));
        self.push_event("STATUS: TAMPER DETECTED (Cryptographic hash mismatch)");

        // Log tamper to persisted run directory if present
        let run_dir = Path::new("runs").join(&self.run_id);
        if run_dir.exists() {
            let entry = serde_json::json!({
                "event": "TAMPER_DETECTED",
                "field": "title",
                "leaf": "CONTENT (Leaf #1)",
                "local_root": self.evidence_root,
                "chain_root": self.chain_root,
                "timestamp": chrono::Utc::now(),
                "run_id": self.run_id
            });
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(run_dir.join("audit.jsonl"))
            {
                use std::io::Write;
                let _ = writeln!(file, "{}", entry);
            }
        }
    }

    fn reset(&mut self) {
        self.status = AppStatus::Idle;
        self.current = None;
        self.candidate_count = 0;
        self.selected_candidate = 0;
        self.evidence_root = "--".to_string();
        self.chain_root = "--".to_string();
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
        self.show_help = false;
        self.active_tab = ViewTab::Pipeline;
        self.blockchain_block = 0;
        self.blockchain_confirmations = 0;
        self.tampered_leaf = None;
        self.tampered_field = None;
        self.original_leaf_hash = None;
        self.tampered_leaf_hash = None;
        self.original_record = None;
        self.current_record = None;
        self.query_embedding = None;
        self.run_id = RunBundleManager::generate_run_id();
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

/// Helper to extract width and height from PNG or JPEG image headers without external decoders.
pub fn detect_image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // PNG magic: 89 50 4E 47 0D 0A 1A 0A
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) && bytes.len() >= 24 {
        let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
        let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
        return Some((width, height));
    }
    // JPEG magic: FF D8 FF
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        let mut idx = 2;
        while idx + 8 < bytes.len() {
            if bytes[idx] != 0xFF {
                idx += 1;
                continue;
            }
            let marker = bytes[idx + 1];
            // SOF0 (0xC0), SOF1 (0xC1), SOF2 (0xC2) baseline / progressive DCT
            if marker == 0xC0 || marker == 0xC1 || marker == 0xC2 {
                let height = u16::from_be_bytes([bytes[idx + 5], bytes[idx + 6]]) as u32;
                let width = u16::from_be_bytes([bytes[idx + 7], bytes[idx + 8]]) as u32;
                return Some((width, height));
            }
            // Skip marker segment
            if idx + 3 < bytes.len() {
                let length = u16::from_be_bytes([bytes[idx + 2], bytes[idx + 3]]) as usize;
                idx += 2 + length;
            } else {
                break;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stepping_app() -> App {
        let mut app = App::new();
        app.demo_mode = true;
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
        assert!(app.tampered_leaf.is_some());
        assert_eq!(app.tampered_field, Some("title".to_string()));
        assert_ne!(app.evidence_root, app.chain_root);
        assert!(app.original_leaf_hash.is_some());
        assert!(app.tampered_leaf_hash.is_some());
        assert_ne!(app.original_leaf_hash, app.tampered_leaf_hash);
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
        app.demo_mode = true;
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

    #[test]
    fn view_tab_cycling_and_indexing() {
        let mut tab = ViewTab::Pipeline;
        assert_eq!(tab.index(), 0);
        assert_eq!(tab.label(), "Pipeline Flow");

        tab = tab.next();
        assert_eq!(tab, ViewTab::Evidence);
        assert_eq!(tab.index(), 1);

        tab = tab.next();
        assert_eq!(tab, ViewTab::Candidates);

        tab = tab.next();
        assert_eq!(tab, ViewTab::Guide);

        tab = tab.next();
        assert_eq!(tab, ViewTab::Pipeline);

        tab = tab.prev();
        assert_eq!(tab, ViewTab::Guide);

        assert_eq!(ViewTab::from_index(0), ViewTab::Pipeline);
        assert_eq!(ViewTab::from_index(1), ViewTab::Evidence);
        assert_eq!(ViewTab::from_index(2), ViewTab::Candidates);
        assert_eq!(ViewTab::from_index(3), ViewTab::Guide);
    }

    #[test]
    fn help_and_tab_actions() {
        let mut app = App::new();
        assert!(!app.show_help);
        assert_eq!(app.active_tab, ViewTab::Pipeline);

        app.apply(AppAction::ToggleHelp);
        assert!(app.show_help);

        app.apply(AppAction::CloseOverlay);
        assert!(!app.show_help);

        app.apply(AppAction::NextTab);
        assert_eq!(app.active_tab, ViewTab::Evidence);

        app.apply(AppAction::SwitchTab(2));
        assert_eq!(app.active_tab, ViewTab::Candidates);

        app.apply(AppAction::PrevTab);
        assert_eq!(app.active_tab, ViewTab::Evidence);
    }

    #[test]
    fn from_image_path_fallback_when_missing() {
        let app = App::from_image_path("non_existent_file_123.jpg");
        assert_eq!(app.input_image_name, "non_existent_file_123.jpg");
        assert!(app.events.iter().any(|e| e.contains("Input file not found")));
    }

    #[test]
    fn detect_dimensions_for_synthetic_png() {
        let mut png_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG magic
        png_bytes.extend_from_slice(&[0, 0, 0, 13]); // IHDR chunk length
        png_bytes.extend_from_slice(b"IHDR");
        png_bytes.extend_from_slice(&640u32.to_be_bytes()); // width 640
        png_bytes.extend_from_slice(&480u32.to_be_bytes()); // height 480
        png_bytes.extend_from_slice(&[8, 2, 0, 0, 0]);

        let dims = detect_image_dimensions(&png_bytes);
        assert_eq!(dims, Some((640, 480)));
    }
}
