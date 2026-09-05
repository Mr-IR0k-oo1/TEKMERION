//! Asynchronous pipeline runner.
//!
//! Executes the eight pipeline stages in order, each as a Tokio task so that
//! long-running engine work never blocks the caller (e.g. a TUI). Supports
//! cooperative cancellation via `CancellationToken`, reset, structured
//! stage-aware errors, and emits an execution event per stage.

use std::future::Future;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::models::{BlockchainRecord, EvidenceBundle, PipelineResult, VerificationResult};
use crate::state::PipelineState;

use super::events::PipelineEvent;
use super::pipeline::{EngineSet, InputPayload, PipelineError, PipelineStage};

/// Lifecycle status of the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerStatus {
    Idle,
    Running,
    Completed,
    Cancelled,
    Failed,
}

impl RunnerStatus {
    pub fn label(self) -> &'static str {
        match self {
            RunnerStatus::Idle => "IDLE",
            RunnerStatus::Running => "RUNNING",
            RunnerStatus::Completed => "COMPLETED",
            RunnerStatus::Cancelled => "CANCELLED",
            RunnerStatus::Failed => "FAILED",
        }
    }
}

/// Shared mutable state guarded by a mutex so [`PipelineRunner`] can be used
/// from `&self` while a run is in flight.
struct RunnerInner {
    status: RunnerStatus,
    state: PipelineState,
    events: Vec<PipelineEvent>,
    result: Option<PipelineResult>,
    error: Option<PipelineError>,
    active_token: Option<CancellationToken>,
}

impl Default for RunnerInner {
    fn default() -> Self {
        Self {
            status: RunnerStatus::Idle,
            state: PipelineState::Idle,
            events: Vec::new(),
            result: None,
            error: None,
            active_token: None,
        }
    }
}

/// Async orchestrator over the [`EngineSet`] dependency-injection container.
pub struct PipelineRunner {
    engines: EngineSet,
    inner: Arc<Mutex<RunnerInner>>,
}

impl PipelineRunner {
    pub fn new(engines: EngineSet) -> Self {
        Self {
            engines,
            inner: Arc::new(Mutex::new(RunnerInner::default())),
        }
    }

    /// Start a run with a fresh internal cancellation token.
    pub async fn run(&self, input: InputPayload) -> Result<RunnerStatus, PipelineError> {
        let token = CancellationToken::new();
        self.run_with_token(input, &token).await
    }

    /// Start a run honoring an externally-owned cancellation token.
    ///
    /// This is how callers (and tests) drive cooperative cancellation: cancel
    /// the token and the in-flight stage aborts.
    pub async fn run_with_token(
        &self,
        input: InputPayload,
        token: &CancellationToken,
    ) -> Result<RunnerStatus, PipelineError> {
        {
            let mut inner = self.inner.lock().await;
            if inner.status != RunnerStatus::Idle {
                return Err(PipelineError::InvalidTransition(format!(
                    "cannot start pipeline from {}",
                    inner.status.label()
                )));
            }
            inner.status = RunnerStatus::Running;
            inner.state = PipelineState::Idle;
            inner.events.clear();
            inner.result = None;
            inner.error = None;
            inner.active_token = Some(token.clone());
            inner.events.push(PipelineEvent::PipelineStarted {
                at: chrono::Utc::now(),
            });
        }

        match self.run_stages(input, token).await {
            Ok(result) => {
                let mut inner = self.inner.lock().await;
                inner.status = RunnerStatus::Completed;
                inner.state = PipelineState::Verified;
                inner.result = Some(result);
                inner.events.push(PipelineEvent::PipelineCompleted);
                Ok(RunnerStatus::Completed)
            }
            Err(PipelineError::Cancelled) => {
                let mut inner = self.inner.lock().await;
                inner.status = RunnerStatus::Cancelled;
                inner.events.push(PipelineEvent::PipelineCancelled);
                Ok(RunnerStatus::Cancelled)
            }
            Err(err) => {
                let mut inner = self.inner.lock().await;
                inner.status = RunnerStatus::Failed;
                inner.state = PipelineState::Error;
                inner.error = Some(err.clone());
                inner
                    .events
                    .push(PipelineEvent::PipelineFailed { error: err.clone() });
                Ok(RunnerStatus::Failed)
            }
        }
    }

    /// Cancel the currently-active run's token (no-op if none).
    pub async fn cancel(&self) {
        let token = { self.inner.lock().await.active_token.clone() };
        if let Some(token) = token {
            token.cancel();
        }
    }

    /// Reset to an idle, clean state, ready for a fresh run.
    pub async fn reset(&self) {
        let mut inner = self.inner.lock().await;
        inner.status = RunnerStatus::Idle;
        inner.state = PipelineState::Idle;
        inner.result = None;
        inner.error = None;
        inner.active_token = Some(CancellationToken::new());
        inner.events.clear();
        inner.events.push(PipelineEvent::PipelineReset);
    }

    pub async fn status(&self) -> RunnerStatus {
        self.inner.lock().await.status
    }

    pub async fn state(&self) -> PipelineState {
        self.inner.lock().await.state
    }

    pub async fn events(&self) -> Vec<PipelineEvent> {
        self.inner.lock().await.events.clone()
    }

    pub async fn last_error(&self) -> Option<PipelineError> {
        self.inner.lock().await.error.clone()
    }

    pub async fn result(&self) -> Option<PipelineResult> {
        self.inner.lock().await.result.clone()
    }

    /// Execute the eight stages in order.
    async fn run_stages(
        &self,
        input: InputPayload,
        token: &CancellationToken,
    ) -> Result<PipelineResult, PipelineError> {
        // INPUT
        let _source = InputPayload::new(input.source.clone())?;
        self.stage_bounds(PipelineStage::Input).await;
        self.transition_to(PipelineState::InputReady).await?;

        // FACE_ANALYSIS
        let face = self.engines.face.clone();
        self.transition_to(PipelineState::FaceAnalysis).await?;
        let analysis = match face {
            Some(engine) => {
                let input = input.clone();
                self.run_engine(PipelineStage::FaceAnalysis, token, async move {
                    engine.analyze(&input).await
                })
                .await?
            }
            None => return self.not_configured(PipelineStage::FaceAnalysis).await,
        };

        // DISCOVERY
        let discovery = self.engines.discovery.clone();
        self.transition_to(PipelineState::Searching).await?;
        let candidates = match discovery {
            Some(engine) => {
                let analysis = analysis.clone();
                self.run_engine(PipelineStage::Discovery, token, async move {
                    engine.discover(&analysis).await
                })
                .await?
            }
            None => return self.not_configured(PipelineStage::Discovery).await,
        };
        self.transition_to(PipelineState::CandidatesFound).await?;

        // CANDIDATE_VERIFICATION
        let verifier = self.engines.verification.clone();
        self.transition_to(PipelineState::Verifying).await?;
        let verified = match verifier {
            Some(engine) => {
                self.run_engine(PipelineStage::CandidateVerification, token, async move {
                    engine.verify(candidates).await
                })
                .await?
            }
            None => {
                return self
                    .not_configured(PipelineStage::CandidateVerification)
                    .await
            }
        };

        // MATCH_SELECTION (internal selection logic; no engine boundary)
        self.transition_to(PipelineState::MatchFound).await?;
        let matched = self.select_best(verified).await?;

        // EVIDENCE
        let evidence = self.engines.evidence.clone();
        self.transition_to(PipelineState::EvidenceCreated).await?;
        let bundle = match evidence {
            Some(engine) => {
                let matched = matched.clone();
                self.run_engine(PipelineStage::Evidence, token, async move {
                    engine.build_evidence(matched).await
                })
                .await?
            }
            None => return self.not_configured(PipelineStage::Evidence).await,
        };

        // BLOCKCHAIN
        let registry = self.engines.registry.clone();
        self.transition_to(PipelineState::BlockchainSubmitting)
            .await?;
        let registered = match registry.clone() {
            Some(engine) => {
                let bundle = bundle.clone();
                self.run_engine(PipelineStage::Blockchain, token, async move {
                    engine.register(bundle).await
                })
                .await?
            }
            None => return self.not_configured(PipelineStage::Blockchain).await,
        };
        self.transition_to(PipelineState::BlockchainConfirmed)
            .await?;

        // ONCHAIN_VERIFICATION
        self.transition_to(PipelineState::VerifyingOnchain).await?;
        let confirmed = match registry {
            Some(engine) => {
                let tx_hash = registered.tx_hash.clone();
                self.run_engine(PipelineStage::OnchainVerification, token, async move {
                    engine.verify_anchor(&tx_hash).await
                })
                .await?
            }
            None => {
                return self
                    .not_configured(PipelineStage::OnchainVerification)
                    .await
            }
        };

        let result = build_result(confirmed, bundle);
        Ok(result)
    }

    /// Emit `StageStarted`/`StageCompleted` for a synchronous (non-engine) stage.
    async fn stage_bounds(&self, stage: PipelineStage) {
        let sequence = stage.index();
        self.emit(PipelineEvent::StageStarted { stage, sequence })
            .await;
        self.emit(PipelineEvent::StageCompleted { stage, sequence })
            .await;
    }

    /// Emit the "not configured" failure for a stage and return the error.
    async fn not_configured<T>(&self, stage: PipelineStage) -> Result<T, PipelineError> {
        let sequence = stage.index();
        self.emit(PipelineEvent::StageStarted { stage, sequence })
            .await;
        self.emit(PipelineEvent::StageFailed {
            stage,
            sequence,
            error: PipelineError::NotConfigured(stage),
        })
        .await;
        Err(PipelineError::NotConfigured(stage))
    }

    /// Run a stage's engine future as a Tokio task, honoring cancellation.
    async fn run_engine<T: Send + 'static>(
        &self,
        stage: PipelineStage,
        token: &CancellationToken,
        fut: impl Future<Output = Result<T, PipelineError>> + Send + 'static,
    ) -> Result<T, PipelineError> {
        let sequence = stage.index();
        self.emit(PipelineEvent::StageStarted { stage, sequence })
            .await;

        let handle = tokio::spawn(fut);
        tokio::pin!(handle);

        tokio::select! {
            result = &mut handle => {
                let result = match result {
                    Ok(result) => result,
                    Err(join) => Err(PipelineError::Internal(format!(
                        "stage task ended abnormally: {join}"
                    ))),
                };
                match &result {
                    Ok(_) => {
                        self.emit(PipelineEvent::StageCompleted { stage, sequence }).await;
                    }
                    Err(error) => {
                        self.emit(PipelineEvent::StageFailed { stage, sequence, error: error.clone() }).await;
                    }
                }
                result
            }
            _ = token.cancelled() => {
                handle.abort();
                self.emit(PipelineEvent::PipelineCancelled).await;
                Err(PipelineError::Cancelled)
            }
        }
    }

    /// Pick the candidate with the highest similarity score.
    async fn select_best(
        &self,
        results: Vec<VerificationResult>,
    ) -> Result<VerificationResult, PipelineError> {
        let sequence = PipelineStage::MatchSelection.index();
        self.emit(PipelineEvent::StageStarted {
            stage: PipelineStage::MatchSelection,
            sequence,
        })
        .await;
        // Empty vector means no candidates survived; that is a genuine
        // stage-local failure, not a fabrication.
        let best = results
            .into_iter()
            .max_by(|a, b| a.similarity.total_cmp(&b.similarity));
        match best {
            Some(best) => {
                self.emit(PipelineEvent::StageCompleted {
                    stage: PipelineStage::MatchSelection,
                    sequence,
                })
                .await;
                Ok(best)
            }
            None => {
                self.emit(PipelineEvent::StageFailed {
                    stage: PipelineStage::MatchSelection,
                    sequence,
                    error: PipelineError::NoMatch,
                })
                .await;
                Err(PipelineError::NoMatch)
            }
        }
    }

    async fn transition_to(&self, to: PipelineState) -> Result<(), PipelineError> {
        let mut inner = self.inner.lock().await;
        let from = inner.state;
        inner.state = from
            .transition(to)
            .map_err(|e| PipelineError::InvalidTransition(e.to_string()))?
            .to;
        Ok(())
    }

    async fn emit(&self, event: PipelineEvent) {
        self.inner.lock().await.events.push(event);
    }
}

fn build_result(confirmed: BlockchainRecord, bundle: EvidenceBundle) -> PipelineResult {
    PipelineResult {
        final_state: PipelineState::Verified,
        evidence: Some(bundle),
        blockchain: Some(confirmed),
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tokio_util::sync::CancellationToken;

    use crate::models::{
        EvidenceRecord, FaceAnalysis, FaceDetection, FaceEmbedding, SearchCandidate,
        VerificationStatus,
    };
    use crate::pipeline::{
        CandidateVerifier, DiscoveryEngine, EvidenceEngine, EvidenceRegistry, FaceEngine,
    };

    fn candidate() -> SearchCandidate {
        SearchCandidate {
            url: url::Url::parse("https://example.com/face").unwrap(),
            title: Some("match".to_string()),
            domain: "example.com".to_string(),
            provider: "test".to_string(),
            image_url: None,
            thumbnail_url: None,
            snippet: None,
            discovered_at: chrono::Utc::now(),
        }
    }

    #[derive(Clone)]
    struct MockFaceEngine;
    #[async_trait]
    impl FaceEngine for MockFaceEngine {
        async fn analyze(&self, _input: &InputPayload) -> Result<FaceAnalysis, PipelineError> {
            Ok(FaceAnalysis {
                detections: vec![FaceDetection {
                    bounding_box: [0.0, 0.0, 1.0, 1.0],
                    confidence: 0.9,
                    quality: 0.8,
                }],
                embeddings: vec![FaceEmbedding {
                    vector: vec![0.1, 0.2],
                    normalized: true,
                }],
                timestamp: chrono::Utc::now(),
                image_path: None,
            })
        }
    }

    #[derive(Clone)]
    struct MockDiscoveryEngine;
    #[async_trait]
    impl DiscoveryEngine for MockDiscoveryEngine {
        async fn discover(
            &self,
            _analysis: &FaceAnalysis,
        ) -> Result<Vec<SearchCandidate>, PipelineError> {
            Ok(vec![candidate()])
        }
    }

    #[derive(Clone)]
    struct MockVerifier;
    #[async_trait]
    impl CandidateVerifier for MockVerifier {
        async fn verify(
            &self,
            candidates: Vec<SearchCandidate>,
        ) -> Result<Vec<VerificationResult>, PipelineError> {
            Ok(candidates
                .into_iter()
                .map(|candidate| VerificationResult {
                    candidate,
                    similarity: 0.9,
                    quality: 0.95,
                    matched_face_index: Some(0),
                    candidate_image_hash: Some("mock_hash".to_string()),
                    status: VerificationStatus::Verified,
                    error_message: None,
                })
                .collect())
        }
    }

    #[derive(Clone)]
    struct MockEvidenceEngine;
    #[async_trait]
    impl EvidenceEngine for MockEvidenceEngine {
        async fn build_evidence(
            &self,
            matched: VerificationResult,
        ) -> Result<EvidenceBundle, PipelineError> {
            Ok(EvidenceBundle {
                leaves: vec!["leaf".to_string()],
                root_hash: "root".to_string(),
                record: Some(EvidenceRecord {
                    schema_version: "1.0.0".to_string(),
                    run_id: "test-run".to_string(),
                    source_url: matched.candidate.url,
                    domain: matched.candidate.domain,
                    platform: "web".to_string(),
                    provider: matched.candidate.provider,
                    retrieved_at: chrono::Utc::now(),
                    title: matched.candidate.title.unwrap_or_default(),
                    text: matched.candidate.snippet.unwrap_or_default(),
                    image_sha256: matched.candidate_image_hash.unwrap_or_default(),
                    face_similarity: matched.similarity,
                    face_model: "test-model".to_string(),
                    candidate_quality: matched.quality,
                }),
            })
        }
    }

    #[derive(Clone)]
    struct MockRegistry;
    #[async_trait]
    impl EvidenceRegistry for MockRegistry {
        async fn register(
            &self,
            _bundle: EvidenceBundle,
        ) -> Result<BlockchainRecord, PipelineError> {
            Ok(BlockchainRecord {
                tx_hash: "0xabc".to_string(),
                block_number: 12,
                registered_root: "root".to_string(),
                timestamp: chrono::Utc::now(),
            })
        }
        async fn verify_anchor(&self, tx_hash: &str) -> Result<BlockchainRecord, PipelineError> {
            Ok(BlockchainRecord {
                tx_hash: tx_hash.to_string(),
                block_number: 12,
                registered_root: "root".to_string(),
                timestamp: chrono::Utc::now(),
            })
        }
    }

    fn configured_set() -> EngineSet {
        EngineSet {
            face: Some(Arc::new(MockFaceEngine)),
            discovery: Some(Arc::new(MockDiscoveryEngine)),
            verification: Some(Arc::new(MockVerifier)),
            evidence: Some(Arc::new(MockEvidenceEngine)),
            registry: Some(Arc::new(MockRegistry)),
        }
    }

    fn input() -> InputPayload {
        InputPayload::new("test-image.jpg").unwrap()
    }

    #[tokio::test]
    async fn successful_progression_runs_all_stages() {
        let runner = PipelineRunner::new(configured_set());
        let status = runner.run(input()).await.expect("run succeeds");
        assert_eq!(status, RunnerStatus::Completed);
        assert_eq!(runner.status().await, RunnerStatus::Completed);
        assert_eq!(runner.state().await, PipelineState::Verified);

        let events = runner.events().await;
        let started: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PipelineEvent::StageStarted { stage, .. } => Some(*stage),
                _ => None,
            })
            .collect();
        assert_eq!(started, PipelineStage::ALL.to_vec());
        assert!(events
            .iter()
            .any(|e| matches!(e, PipelineEvent::PipelineCompleted)));
    }

    #[tokio::test]
    async fn failure_propagates_and_is_reported_for_stage() {
        #[derive(Clone)]
        struct FailingDiscovery;
        #[async_trait]
        impl DiscoveryEngine for FailingDiscovery {
            async fn discover(
                &self,
                _analysis: &FaceAnalysis,
            ) -> Result<Vec<SearchCandidate>, PipelineError> {
                Err(PipelineError::Stage {
                    stage: PipelineStage::Discovery,
                    message: "upstream search failed".to_string(),
                })
            }
        }

        let mut set = configured_set();
        set.discovery = Some(Arc::new(FailingDiscovery));
        let runner = PipelineRunner::new(set);

        let status = runner.run(input()).await.expect("run returns status");
        assert_eq!(status, RunnerStatus::Failed);
        assert_eq!(runner.state().await, PipelineState::Error);

        let error = runner.last_error().await.expect("error recorded");
        assert!(
            matches!(error, PipelineError::Stage { stage, message } if stage == PipelineStage::Discovery && message == "upstream search failed")
        );

        let events = runner.events().await;
        assert!(events.iter().any(|e| matches!(
            e,
            PipelineEvent::StageFailed { stage, .. } if *stage == PipelineStage::Discovery
        )));
    }

    #[tokio::test]
    async fn not_configured_is_reported_per_stage() {
        let runner = PipelineRunner::new(EngineSet::none());
        let status = runner.run(input()).await.expect("run returns status");
        assert_eq!(status, RunnerStatus::Failed);
        assert_eq!(
            runner.last_error().await,
            Some(PipelineError::NotConfigured(PipelineStage::FaceAnalysis))
        );

        let events = runner.events().await;
        assert!(events.iter().any(|e| matches!(
            e,
            PipelineEvent::StageFailed { stage, error, .. }
                if *stage == PipelineStage::FaceAnalysis
                    && matches!(error, PipelineError::NotConfigured(PipelineStage::FaceAnalysis))
        )));
    }

    #[tokio::test]
    async fn cancellation_stops_inflight_stage() {
        #[derive(Clone)]
        struct SlowFaceEngine;
        #[async_trait]
        impl FaceEngine for SlowFaceEngine {
            async fn analyze(&self, _input: &InputPayload) -> Result<FaceAnalysis, PipelineError> {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                Ok(FaceAnalysis {
                    detections: vec![],
                    embeddings: vec![],
                    timestamp: chrono::Utc::now(),
                    image_path: None,
                })
            }
        }

        let mut set = EngineSet::none();
        set.face = Some(Arc::new(SlowFaceEngine));
        let runner = PipelineRunner::new(set);
        let token = CancellationToken::new();

        let runner_task = Arc::new(runner);
        let task_runner = runner_task.clone();
        let task_token = token.clone();
        let handle =
            tokio::spawn(async move { task_runner.run_with_token(input(), &task_token).await });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        token.cancel();

        let status = handle.await.expect("task completes");
        assert_eq!(status, Ok(RunnerStatus::Cancelled));
        assert_eq!(runner_task.status().await, RunnerStatus::Cancelled);

        let events = runner_task.events().await;
        assert!(events
            .iter()
            .any(|e| matches!(e, PipelineEvent::PipelineCancelled)));
    }

    #[tokio::test]
    async fn reset_allows_rerun() {
        let runner = PipelineRunner::new(configured_set());
        let first = runner.run(input()).await.expect("first run");
        assert_eq!(first, RunnerStatus::Completed);

        runner.reset().await;
        assert_eq!(runner.status().await, RunnerStatus::Idle);
        assert_eq!(runner.state().await, PipelineState::Idle);
        assert!(runner
            .events()
            .await
            .iter()
            .any(|e| matches!(e, PipelineEvent::PipelineReset)));

        let second = runner.run(input()).await.expect("rerun succeeds");
        assert_eq!(second, RunnerStatus::Completed);
    }

    #[tokio::test]
    async fn invalid_transition_running_twice() {
        let slow = {
            #[derive(Clone)]
            struct Slow;
            #[async_trait]
            impl FaceEngine for Slow {
                async fn analyze(
                    &self,
                    _input: &InputPayload,
                ) -> Result<FaceAnalysis, PipelineError> {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    Ok(FaceAnalysis {
                        detections: vec![],
                        embeddings: vec![],
                        timestamp: chrono::Utc::now(),
                        image_path: None,
                    })
                }
            }
            Slow
        };
        let mut set = EngineSet::none();
        set.face = Some(Arc::new(slow));
        let runner = Arc::new(PipelineRunner::new(set));
        let token = CancellationToken::new();

        let task_runner = runner.clone();
        let task_token = token.clone();
        let handle =
            tokio::spawn(async move { task_runner.run_with_token(input(), &task_token).await });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let second = runner.run(input()).await;
        assert!(matches!(second, Err(PipelineError::InvalidTransition(_))));

        token.cancel();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn select_best_picks_highest_similarity() {
        let runner = PipelineRunner::new(EngineSet::none());
        let results = vec![
            VerificationResult {
                candidate: candidate(),
                similarity: 0.4,
                quality: 0.85,
                matched_face_index: Some(0),
                candidate_image_hash: Some("h1".to_string()),
                status: VerificationStatus::BelowThreshold,
                error_message: None,
            },
            VerificationResult {
                candidate: candidate(),
                similarity: 0.95,
                quality: 0.90,
                matched_face_index: Some(0),
                candidate_image_hash: Some("h2".to_string()),
                status: VerificationStatus::Verified,
                error_message: None,
            },
        ];
        let best = runner.select_best(results).await.unwrap();
        assert_eq!(best.similarity, 0.95);
    }

    #[tokio::test]
    async fn select_best_rejects_empty() {
        let runner = PipelineRunner::new(EngineSet::none());
        let result = runner.select_best(vec![]).await;
        assert!(matches!(result, Err(PipelineError::NoMatch)));
    }
}
