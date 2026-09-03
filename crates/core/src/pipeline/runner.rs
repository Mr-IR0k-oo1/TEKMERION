//! The asynchronous pipeline runner.
//!
//! The runner executes the ordered [`PipelineStage`]s on a Tokio runtime. Each
//! stage is run as an independent [`tokio::spawn`]ed task so that long-running
//! engines never block a caller (such as the TUI event loop). The runner
//! supports:
//!
//! - cooperative release between stages and cancellation of an in-flight stage
//! - reset back to the idle state
//! - explicit, per-stage failure reporting through emitted events
//!
//! No stage transition is silent: every stage start, completion, transition and
//! failure is emitted as a [`PipelineEvent`].

use tokio::sync::watch;
use tokio::task::JoinHandle;

use super::events::PipelineEvent;
use super::pipeline::{
    EngineSet, InputPayload, PipelineError, PipelineStage,
};
use crate::models::{BlockchainRecord, EvidenceBundle, FaceAnalysis, SearchCandidate, VerificationResult};

/// Co-operative cancellation signal shared between the caller and the runner.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    tx: watch::Sender<bool>,
    rx: watch::Receiver<bool>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    /// Creates an un-cancelled token.
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self { tx, rx }
    }

    /// Requests cancellation of any pipeline using this token.
    pub fn cancel(&self) {
        let _ = self.tx.send(true);
    }

    /// Whether a cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        *self.rx.borrow()
    }

    /// Waits until the token is cancelled. Returns immediately if already
    /// cancelled.
    pub async fn cancelled(&mut self) {
        if self.is_cancelled() {
            return;
        }
        let _ = self.rx.changed().await;
    }
}

/// The lifecycle status of the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerStatus {
    Idle,
    Running,
    Completed,
    Cancelled,
    Failed,
}

/// Intermediate state threaded from one stage to the next.
#[derive(Debug, Default)]
struct StageState {
    analysis: Option<FaceAnalysis>,
    candidates: Option<Vec<SearchCandidate>>,
    verification: Option<Vec<VerificationResult>>,
    bundle: Option<EvidenceBundle>,
    record: Option<BlockchainRecord>,
}

/// Input to a stage, cloned from the runner state.
#[derive(Debug, Clone)]
enum StageInput {
    /// The payload passed to the INPUT stage, threaded to FACE_ANALYSIS.
    Payload(InputPayload),
    Analysis(FaceAnalysis),
    Candidates(Vec<SearchCandidate>),
    Verification(Vec<VerificationResult>),
    Selected(VerificationResult),
    Bundle(EvidenceBundle),
    Record(BlockchainRecord),
}

/// Output produced by a stage execution.
#[derive(Debug)]
enum StageOutput {
    /// The validated payload threaded from INPUT to FACE_ANALYSIS.
    Payload(InputPayload),
    Analysis(FaceAnalysis),
    Candidates(Vec<SearchCandidate>),
    Verification(Vec<VerificationResult>),
    Selected(VerificationResult),
    Bundle(EvidenceBundle),
    Record(BlockchainRecord),
    Verified(BlockchainRecord),
}

/// The engine-backed async pipeline runner.
pub struct PipelineRunner {
    engines: EngineSet,
    status: RunnerStatus,
    events: Vec<PipelineEvent>,
    sequence: usize,
    state: StageState,
    cancel_handle: Option<CancellationToken>,
}

impl Default for PipelineRunner {
    fn default() -> Self {
        Self::new(EngineSet::none())
    }
}

impl PipelineRunner {
    /// Creates a runner with the given engine set.
    pub fn new(engines: EngineSet) -> Self {
        Self {
            engines,
            status: RunnerStatus::Idle,
            events: Vec::new(),
            sequence: 0,
            state: StageState::default(),
            cancel_handle: None,
        }
    }

    /// The current lifecycle status of the runner.
    pub fn status(&self) -> RunnerStatus {
        self.status
    }

    /// Snapshot of all events emitted since construction (or the last reset).
    pub fn events(&self) -> &[PipelineEvent] {
        &self.events
    }

    /// Whether a cancellation has been requested for the current run.
    pub fn is_cancel_requested(&self) -> bool {
        self.cancel_handle
            .as_ref()
            .map(CancellationToken::is_cancelled)
            .unwrap_or(false)
    }

    /// Requests cancellation of the currently running pipeline.
    pub fn cancel(&self) {
        if let Some(token) = &self.cancel_handle {
            token.cancel();
        }
    }

    /// Resets the runner to its idle state, clearing intermediate data.
    pub fn reset(&mut self) {
        self.status = RunnerStatus::Idle;
        self.sequence = 0;
        self.state = StageState::default();
        self.cancel_handle = None;
        self.events.clear();
        self.emit(PipelineEvent::PipelineReset);
    }

    /// Runs the pipeline end-to-end for the given input.
    ///
    /// Returns an error when the pipeline cannot progress: a stage failure, a
    /// missing engine, a cancellation, or an invalid transition. The runner's
    /// [`RunnerStatus`] reflects the terminal outcome.
    pub async fn run(&mut self, input: InputPayload) -> Result<RunnerStatus, PipelineError> {
        if self.status != RunnerStatus::Idle {
            return Err(PipelineError::InvalidTransition(format!(
                "pipeline cannot start from state {:?}",
                self.status
            )));
        }

        let mut token = CancellationToken::new();
        self.cancel_handle = Some(token.clone());
        self.status = RunnerStatus::Running;
        self.emit(PipelineEvent::PipelineStarted);

        let mut from: Option<PipelineStage> = None;
        let mut stage_input = StageInput::Payload(input);

        for stage in PipelineStage::ALL {
            if token.is_cancelled() {
                self.status = RunnerStatus::Cancelled;
                self.emit(PipelineEvent::PipelineCancelled);
                return Err(PipelineError::Cancelled);
            }

            if let Some(prev) = from {
                self.emit(PipelineEvent::Transition { from: prev, to: stage });
            }

            let sequence = self.next_sequence();
            self.emit(PipelineEvent::StageStarted { stage, sequence });

            match self.execute_stage(stage, &stage_input, &mut token).await {
                Ok(output) => match self.apply_output(stage, output) {
                    Ok(next_input) => {
                        stage_input = next_input;
                        self.emit(PipelineEvent::StageCompleted { stage, sequence });
                    }
                    Err(error) => {
                        self.emit(PipelineEvent::StageFailed {
                            stage,
                            sequence,
                            error: error.clone(),
                        });
                        self.status = RunnerStatus::Failed;
                        self.emit(PipelineEvent::PipelineFailed { error });
                        return Err(error);
                    }
                },
                Err(PipelineError::Cancelled) => {
                    self.status = RunnerStatus::Cancelled;
                    self.emit(PipelineEvent::PipelineCancelled);
                    return Err(PipelineError::Cancelled);
                }
                Err(error) => {
                    self.emit(PipelineEvent::StageFailed {
                        stage,
                        sequence,
                        error: error.clone(),
                    });
                    self.status = RunnerStatus::Failed;
                    self.emit(PipelineEvent::PipelineFailed { error });
                    return Err(error);
                }
            }

            from = Some(stage);
        }

        self.status = RunnerStatus::Completed;
        self.emit(PipelineEvent::PipelineCompleted);
        Ok(RunnerStatus::Completed)
    }

    /// Executes a single stage on its own task, honouring cancellation.
    async fn execute_stage(
        &self,
        stage: PipelineStage,
        stage_input: &StageInput,
        token: &mut CancellationToken,
    ) -> Result<StageOutput, PipelineError> {
        let engines = self.engines.clone();
        let input = stage_input.clone();

        let mut task_token = token.clone();
        let handle: JoinHandle<Result<StageOutput, PipelineError>> =
            tokio::spawn(async move { run_stage(engines, stage, input).await });

        tokio::select! {
            result = handle => match result {
                Ok(inner) => inner,
                Err(join_err) => Err(PipelineError::Internal(format!(
                    "stage task failed to join: {}",
                    join_err
                ))),
            },
            _ = task_token.cancelled() => {
                handle.abort();
                Err(PipelineError::Cancelled)
            }
        }
    }

    /// Applies a stage output to the runner state, returning the input for the
    /// next stage.
    fn apply_output(
        &mut self,
        stage: PipelineStage,
        output: StageOutput,
    ) -> Result<StageInput, PipelineError> {
        match stage {
            PipelineStage::FaceAnalysis => match output {
                StageOutput::Analysis(a) => {
                    self.state.analysis = Some(a.clone());
                    Ok(StageInput::Analysis(a))
                }
                other => Err(PipelineError::Internal(format!(
                    "unexpected face output: {other:?}"
                ))),
            },
            PipelineStage::Discovery => match output {
                StageOutput::Candidates(c) => {
                    self.state.candidates = Some(c.clone());
                    Ok(StageInput::Candidates(c))
                }
                other => Err(PipelineError::Internal(format!(
                    "unexpected discovery output: {other:?}"
                ))),
            },
            PipelineStage::CandidateVerification => match output {
                StageOutput::Verification(v) => {
                    self.state.verification = Some(v.clone());
                    Ok(StageInput::Verification(v))
                }
                other => Err(PipelineError::Internal(format!(
                    "unexpected verification output: {other:?}"
                ))),
            },
            PipelineStage::MatchSelection => match output {
                StageOutput::Verification(v) => {
                    self.state.verification = Some(v.clone());
                    Ok(StageInput::Selected(v))
                }
                other => Err(PipelineError::Internal(format!(
                    "unexpected match output: {other:?}"
                ))),
            },
            PipelineStage::Evidence => match output {
                StageOutput::Bundle(b) => {
                    self.state.bundle = Some(b.clone());
                    Ok(StageInput::Bundle(b))
                }
                other => Err(PipelineError::Internal(format!(
                    "unexpected evidence output: {other:?}"
                ))),
            },
            PipelineStage::Blockchain => match output {
                StageOutput::Record(r) => {
                    self.state.record = Some(r.clone());
                    Ok(StageInput::Record(r))
                }
                other => Err(PipelineError::Internal(format!(
                    "unexpected blockchain output: {other:?}"
                ))),
            },
            PipelineStage::OnchainVerification => match output {
                StageOutput::Verified(r) => {
                    self.state.record = Some(r.clone());
                    Ok(StageInput::Record(r))
                }
                other => Err(PipelineError::Internal(format!(
                    "unexpected onchain output: {other:?}"
                ))),
            },
            PipelineStage::Input => Err(PipelineError::Internal(
                "INPUT stage produces no output".into(),
            )),
        }
    }

    fn next_sequence(&mut self) -> usize {
        let s = self.sequence;
        self.sequence += 1;
        s
    }

    fn emit(&mut self, event: PipelineEvent) {
        self.events.push(event);
    }
}

/// Runs a single stage against the supplied engines.
async fn run_stage(
    engines: EngineSet,
    stage: PipelineStage,
    input: StageInput,
) -> Result<StageOutput, PipelineError> {
    match stage {
        PipelineStage::Input => {
            let StageInput::Payload(payload) = input else {
                return Err(PipelineError::Internal("INPUT requires a payload".into()));
            };
            if payload.source.trim().is_empty() {
                return Err(PipelineError::Input("empty input source".into()));
            }
            Ok(StageOutput::Input)
        }
        PipelineStage::FaceAnalysis => {
            let face = engines
                .face
                .ok_or(PipelineError::NotConfigured(PipelineStage::FaceAnalysis))?;
            let payload = match input {
                StageInput::Payload(p) => p,
                _ => {
                    return Err(PipelineError::Internal(
                        "FACE_ANALYSIS requires the input payload".into(),
                    ))
                }
            };
            let a = face.analyze(&payload).await?;
            Ok(StageOutput::Analysis(a))
        }
        PipelineStage::Discovery => {
            let discovery = engines
                .discovery
                .ok_or(PipelineError::NotConfigured(PipelineStage::Discovery))?;
            let StageInput::Analysis(a) = input else {
                return Err(PipelineError::Internal("missing face analysis".into()));
            };
            let c = discovery.discover(&a).await?;
            Ok(StageOutput::Candidates(c))
        }
        PipelineStage::CandidateVerification => {
            let verifier = engines
                .verification
                .ok_or(PipelineError::NotConfigured(PipelineStage::CandidateVerification))?;
            let StageInput::Candidates(c) = input else {
                return Err(PipelineError::Internal("missing candidates".into()));
            };
            let v = verifier.verify(c).await?;
            Ok(StageOutput::Verification(v))
        }
        PipelineStage::MatchSelection => {
            let StageInput::Verification(v) = input else {
                return Err(PipelineError::Internal("missing verification results".into()));
            };
            let selected = select_best(v)?;
            Ok(StageOutput::Verification(selected))
        }
        PipelineStage::Evidence => {
            let evidence = engines
                .evidence
                .ok_or(PipelineError::NotConfigured(PipelineStage::Evidence))?;
            let StageInput::Selected(s) = input else {
                return Err(PipelineError::Internal("missing selected match".into()));
            };
            let b = evidence.build_evidence(s).await?;
            Ok(StageOutput::Bundle(b))
        }
        PipelineStage::Blockchain => {
            let registry = engines
                .registry
                .ok_or(PipelineError::NotConfigured(PipelineStage::Blockchain))?;
            let StageInput::Bundle(b) = input else {
                return Err(PipelineError::Internal("missing evidence bundle".into()));
            };
            let r = registry.register(b).await?;
            Ok(StageOutput::Record(r))
        }
        PipelineStage::OnchainVerification => {
            let registry = engines
                .registry
                .ok_or(PipelineError::NotConfigured(PipelineStage::OnchainVerification))?;
            let StageInput::Record(r) = input else {
                return Err(PipelineError::Internal("missing blockchain record".into()));
            };
            let verified = registry.verify_anchor(&r.tx_hash).await?;
            Ok(StageOutput::Verified(verified))
        }
    }
}

/// Picks the single strongest verified candidate.
fn select_best(results: Vec<VerificationResult>) -> Result<VerificationResult, PipelineError> {
    results
        .into_iter()
        .max_by(|a, b| {
            a.similarity
                .partial_cmp(&b.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or(PipelineError::NoMatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        BlockchainRecord, EvidenceBundle, EvidenceRecord, FaceAnalysis, FaceDetection,
        FaceEmbedding, SearchCandidate, VerificationStatus,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use url::Url;

    fn candidate(index: u32, similarity: f32) -> SearchCandidate {
        SearchCandidate {
            url: Url::parse(&format!("https://example.com/{index}")).unwrap(),
            title: Some(format!("Candidate {index}")),
            provider: "mock".to_string(),
            image_url: None,
            snippet: None,
            discovered_at: Utc::now(),
        }
    }

    fn verification(similarity: f32, status: VerificationStatus) -> VerificationResult {
        VerificationResult {
            candidate: candidate(1, similarity),
            similarity,
            status,
        }
    }

    fn analysis() -> FaceAnalysis {
        FaceAnalysis {
            detections: vec![FaceDetection {
                bounding_box: [0.0, 0.0, 1.0, 1.0],
                confidence: 0.99,
                quality: 0.95,
            }],
            embeddings: vec![FaceEmbedding {
                vector: vec![0.1, 0.2, 0.3],
                normalized: true,
            }],
            timestamp: Utc::now(),
        }
    }

    fn bundle() -> EvidenceBundle {
        EvidenceBundle {
            record: EvidenceRecord {
                source_url: Url::parse("https://example.com/src").unwrap(),
                provider: "mock".to_string(),
                timestamp: Utc::now(),
                content_hash: "0xabc".to_string(),
                face_similarity: 0.95,
            },
            root_hash: "0xroot".to_string(),
            leaf_hashes: vec!["0xleaf".to_string()],
        }
    }

    fn record() -> BlockchainRecord {
        BlockchainRecord {
            tx_hash: "0xtx".to_string(),
            block_number: 42,
            registered_root: "0xroot".to_string(),
            timestamp: Utc::now(),
        }
    }

    struct MockFaceEngine(Result<FaceAnalysis, PipelineError>);
    struct MockDiscoveryEngine(Result<Vec<SearchCandidate>, PipelineError>);
    struct MockVerifier(Result<Vec<VerificationResult>, PipelineError>);
    struct MockEvidenceEngine(Result<EvidenceBundle, PipelineError>);
    struct MockRegistry(
        Result<BlockchainRecord, PipelineError>,
        Result<BlockchainRecord, PipelineError>,
    );

    #[async_trait]
    impl FaceEngine for MockFaceEngine {
        async fn analyze(&self, _input: &InputPayload) -> Result<FaceAnalysis, PipelineError> {
            self.0.clone()
        }
    }

    #[async_trait]
    impl DiscoveryEngine for MockDiscoveryEngine {
        async fn discover(
            &self,
            _analysis: &FaceAnalysis,
        ) -> Result<Vec<SearchCandidate>, PipelineError> {
            self.0.clone()
        }
    }

    #[async_trait]
    impl CandidateVerifier for MockVerifier {
        async fn verify(
            &self,
            _candidates: Vec<SearchCandidate>,
        ) -> Result<Vec<VerificationResult>, PipelineError> {
            self.0.clone()
        }
    }

    #[async_trait]
    impl EvidenceEngine for MockEvidenceEngine {
        async fn build_evidence(
            &self,
            _result: VerificationResult,
        ) -> Result<EvidenceBundle, PipelineError> {
            self.0.clone()
        }
    }

    #[async_trait]
    impl EvidenceRegistry for MockRegistry {
        async fn register(
            &self,
            _bundle: EvidenceBundle,
        ) -> Result<BlockchainRecord, PipelineError> {
            self.0.clone()
        }

        async fn verify_anchor(
            &self,
            _tx_hash: &str,
        ) -> Result<BlockchainRecord, PipelineError> {
            self.1.clone()
        }
    }

    fn configured_set() -> EngineSet {
        let mut set = EngineSet::default();
        set.face = Some(std::sync::Arc::new(MockFaceEngine(Ok(analysis()))));
        set.discovery = Some(std::sync::Arc::new(MockDiscoveryEngine(Ok(vec![
            candidate(1, 0.9),
            candidate(2, 0.7),
        ]))));
        set.verification = Some(std::sync::Arc::new(MockVerifier(Ok(vec![
            verification(0.92, VerificationStatus::Match),
            verification(0.71, VerificationStatus::Review),
        ]))));
        set.evidence = Some(std::sync::Arc::new(MockEvidenceEngine(Ok(bundle()))));
        set.registry = Some(std::sync::Arc::new(MockRegistry(
            Ok(record()),
            Ok(record()),
        )));
        set
    }

    fn input() -> InputPayload {
        InputPayload::new("test-image.jpg").unwrap()
    }

    #[tokio::test]
    async fn successful_progression_runs_all_stages() {
        let mut runner = PipelineRunner::new(configured_set());
        let status = runner.run(input()).await.unwrap();
        assert_eq!(status, RunnerStatus::Completed);
        assert_eq!(runner.status(), RunnerStatus::Completed);

        let events = runner.events();
        assert!(events
            .iter()
            .any(|e| matches!(e, PipelineEvent::PipelineStarted)));
        assert!(events
            .iter()
            .any(|e| matches!(e, PipelineEvent::PipelineCompleted)));

        for stage in PipelineStage::ALL {
            assert!(
                events.iter().any(|e| matches!(
                    e,
                    PipelineEvent::StageCompleted { stage: s, .. } if *s == stage
                )),
                "missing StageCompleted for {stage:?}"
            );
            assert!(
                events.iter().any(|e| matches!(
                    e,
                    PipelineEvent::StageStarted { stage: s, .. } if *s == stage
                )),
                "missing StageStarted for {stage:?}"
            );
        }

        assert!(!events
            .iter()
            .any(|e| matches!(e, PipelineEvent::StageFailed { .. })));
    }

    #[tokio::test]
    async fn stage_execution_order_is_emitted() {
        let mut runner = PipelineRunner::new(configured_set());
        runner.run(input()).await.unwrap();

        let completed: Vec<PipelineStage> = runner
            .events()
            .iter()
            .filter_map(|e| match e {
                PipelineEvent::StageCompleted { stage, .. } => Some(*stage),
                _ => None,
            })
            .collect();

        assert_eq!(completed, PipelineStage::ALL.to_vec());
    }

    #[tokio::test]
    async fn not_configured_is_reported_per_stage() {
        let mut runner = PipelineRunner::new(EngineSet::none());
        let err = runner.run(input()).await.unwrap_err();

        assert_eq!(runner.status(), RunnerStatus::Failed);
        match &err {
            PipelineError::NotConfigured(stage) => {
                assert_eq!(*stage, PipelineStage::FaceAnalysis);
            }
            other => panic!("expected NotConfigured, got {other:?}"),
        }

        let events = runner.events();
        assert!(events.iter().any(|e| matches!(
            e,
            PipelineEvent::StageFailed {
                stage: PipelineStage::FaceAnalysis,
                ..
            }
        )));
        assert!(events
            .iter()
            .any(|e| matches!(e, PipelineEvent::PipelineFailed { .. })));
    }

    #[tokio::test]
    async fn failure_propagates_and_is_reported_for_stage() {
        let mut set = configured_set();
        set.discovery = Some(std::sync::Arc::new(MockDiscoveryEngine(Err(
            PipelineError::Stage {
                stage: PipelineStage::Discovery,
                message: "search backend unavailable".to_string(),
            },
        ))));
        let mut runner = PipelineRunner::new(set);

        let err = runner.run(input()).await.unwrap_err();
        assert_eq!(runner.status(), RunnerStatus::Failed);

        match &err {
            PipelineError::Stage { stage, message } => {
                assert_eq!(*stage, PipelineStage::Discovery);
                assert_eq!(message, "search backend unavailable");
            }
            other => panic!("expected Stage error, got {other:?}"),
        }

        assert!(runner.events().iter().any(|e| matches!(
            e,
            PipelineEvent::StageFailed {
                stage: PipelineStage::Discovery,
                ..
            }
        )));
    }

    #[tokio::test]
    async fn cancellation_stops_inflight_stage() {
        struct SlowFaceEngine;
        #[async_trait]
        impl FaceEngine for SlowFaceEngine {
            async fn analyze(&self, _input: &InputPayload) -> Result<FaceAnalysis, PipelineError> {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                Ok(analysis())
            }
        }

        let mut set = EngineSet::default();
        set.face = Some(std::sync::Arc::new(SlowFaceEngine));
        let mut runner = PipelineRunner::new(set);

        let handle = tokio::spawn({
            let runner = &mut runner;
            async move { runner.run(input()).await }
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        runner.cancel();

        let result = handle.await.unwrap();
        assert_eq!(result.unwrap_err(), PipelineError::Cancelled);
        assert_eq!(runner.status(), RunnerStatus::Cancelled);
        assert!(runner
            .events()
            .iter()
            .any(|e| matches!(e, PipelineEvent::PipelineCancelled)));
    }

    #[tokio::test]
    async fn reset_after_completion_returns_to_idle() {
        let mut runner = PipelineRunner::new(configured_set());
        runner.run(input()).await.unwrap();
        assert_eq!(runner.status(), RunnerStatus::Completed);

        runner.reset();
        assert_eq!(runner.status(), RunnerStatus::Idle);
        assert!(runner
            .events()
            .iter()
            .any(|e| matches!(e, PipelineEvent::PipelineReset)));

        // After reset the pipeline can run again.
        let status = runner.run(input()).await.unwrap();
        assert_eq!(status, RunnerStatus::Completed);
    }

    #[tokio::test]
    async fn invalid_transition_running_twice() {
        struct SlowFaceEngine;
        #[async_trait]
        impl FaceEngine for SlowFaceEngine {
            async fn analyze(&self, _input: &InputPayload) -> Result<FaceAnalysis, PipelineError> {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                Ok(analysis())
            }
        }

        let mut set = EngineSet::default();
        set.face = Some(std::sync::Arc::new(SlowFaceEngine));
        let mut runner = PipelineRunner::new(set);

        let handle = tokio::spawn({
            let runner = &mut runner;
            async move { runner.run(input()).await }
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let second = runner.run(input()).await;
        match second {
            Err(PipelineError::InvalidTransition(_)) => {}
            other => panic!("expected InvalidTransition, got {other:?}"),
        }
        runner.cancel();
        let _ = handle.await;
    }

    #[test]
    fn select_best_picks_highest_similarity() {
        let selected = select_best(vec![
            verification(0.5, VerificationStatus::Review),
            verification(0.95, VerificationStatus::Match),
            verification(0.7, VerificationStatus::Review),
        ])
        .unwrap();
        assert_eq!(selected.similarity, 0.95);
    }

    #[test]
    fn select_best_rejects_empty() {
        assert_eq!(select_best(vec![]), Err(PipelineError::NoMatch));
    }

    #[test]
    fn input_payload_rejects_empty_source() {
        assert!(matches!(
            InputPayload::new("   "),
            Err(PipelineError::Input(_))
        ));
        assert!(InputPayload::new("image.png").is_ok());
    }

    #[test]
    fn stage_next_ordering() {
        assert_eq!(
            PipelineStage::Input.next(),
            Some(PipelineStage::FaceAnalysis)
        );
        assert_eq!(PipelineStage::OnchainVerification.next(), None);
    }

    #[test]
    fn error_is_stage_failure_classification() {
        assert!(PipelineError::NotConfigured(PipelineStage::FaceAnalysis).is_stage_failure());
        assert!(!PipelineError::Cancelled.is_stage_failure());
        assert!(!PipelineError::InvalidTransition("x".into()).is_stage_failure());
    }
}
