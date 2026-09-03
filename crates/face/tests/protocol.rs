//! Protocol tests for the face-analysis worker JSONL client.
//!
//! These tests run against a hermetic mock worker
//! (`tests/fixtures/mock_worker.py`) that speaks the same JSONL protocol as
//! the real `workers/face/worker.py` but requires no InsightFace/ONNX stack.

use std::path::PathBuf;
use std::time::Duration;

use tempfile::TempDir;

use tekmerion_face::{FaceWorker, FaceWorkerConfig, FaceWorkerError};

fn mock_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mock_worker.py")
}

fn config() -> FaceWorkerConfig {
    FaceWorkerConfig::default()
        .with_python("python")
        .with_script(mock_script())
        .with_request_timeout(Duration::from_secs(5))
}

/// Create a real (dummy) image file and return the directory held open for the
/// file's lifetime plus the file's path.
fn dummy_image() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("photo.jpg");
    std::fs::write(&path, b"dummy").expect("write dummy image");
    (dir, path)
}

#[tokio::test]
async fn spawns_worker_and_analyzes_single_face() {
    let (_dir, image) = dummy_image();
    let worker = FaceWorker::spawn(&config()).expect("spawn");
    let analysis = worker.analyze(&image).await.expect("analyze");

    assert_eq!(analysis.detections.len(), 1);
    assert_eq!(analysis.embeddings.len(), 1);
    assert!(analysis.embeddings[0].normalized);
    assert_eq!(analysis.embeddings[0].vector, vec![1.0, 0.0, 0.0]);
    assert_eq!(
        analysis.detections[0].bounding_box,
        [10.0, 20.0, 90.0, 120.0]
    );
    assert_eq!(analysis.detections[0].confidence, 0.9);

    worker.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn zero_faces_is_an_explicit_result() {
    let worker = FaceWorker::spawn(&config()).expect("spawn");
    let analysis = worker.analyze("__zero__").await.expect("analyze");

    assert!(analysis.detections.is_empty());
    assert!(analysis.embeddings.is_empty());

    worker.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn multiple_faces_are_all_represented() {
    let worker = FaceWorker::spawn(&config()).expect("spawn");
    let analysis = worker.analyze("__multi__").await.expect("analyze");

    assert_eq!(analysis.detections.len(), 2);
    assert_eq!(analysis.embeddings.len(), 2);

    worker.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn enforces_timeout() {
    let cfg = config().with_request_timeout(Duration::from_millis(400));
    let worker = FaceWorker::spawn(&cfg).expect("spawn");

    let err = worker
        .analyze("__timeout__")
        .await
        .expect_err("should time out");
    assert!(matches!(err, FaceWorkerError::Timeout(_)), "got {err:?}");

    worker.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn detects_worker_crash() {
    let worker = FaceWorker::spawn(&config()).expect("spawn");

    let err = worker
        .analyze("__crash__")
        .await
        .expect_err("expected worker crash");
    assert!(
        matches!(err, FaceWorkerError::WorkerCrashed(_)),
        "got {err:?}"
    );

    // Once crashed, further requests surface immediately as NotRunning.
    let after = worker.analyze("__zero__").await.expect_err("not running");
    assert!(
        matches!(after, FaceWorkerError::NotRunning),
        "got {after:?}"
    );
}

#[tokio::test]
async fn rejects_malformed_worker_output() {
    let worker = FaceWorker::spawn(&config()).expect("spawn");

    let err = worker.analyze("__badjson__").await.expect_err("bad json");
    assert!(
        matches!(err, FaceWorkerError::InvalidResponse(_)),
        "got {err:?}"
    );
}

#[tokio::test]
async fn surfaces_structured_worker_error() {
    let worker = FaceWorker::spawn(&config()).expect("spawn");

    let err = worker
        .analyze("__missing__")
        .await
        .expect_err("missing file");
    match err {
        FaceWorkerError::RequestFailed { errors } => {
            assert_eq!(errors.len(), 1);
            assert!(errors[0].contains("missing"));
        }
        other => panic!("got {other:?}"),
    }

    worker.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn correlates_requests_by_id_under_reordering() {
    let worker = FaceWorker::spawn(&config()).expect("spawn");

    // The mock answers B before A; the client must route by request_id, not by
    // arrival order. A's embedding is [0,0,1], B's is [0,1,0].
    let a = worker.analyze("__swap_a__");
    let b = worker.analyze("__swap_b__");
    let (fa, fb) = tokio::join!(a, b);

    let analysis_a = fa.expect("A should succeed");
    let analysis_b = fb.expect("B should succeed");

    assert_eq!(analysis_a.embeddings[0].vector, vec![0.0, 0.0, 1.0]);
    assert_eq!(analysis_b.embeddings[0].vector, vec![0.0, 1.0, 0.0]);

    worker.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn shutdown_cleans_up_worker_process() {
    let worker = FaceWorker::spawn(&config()).expect("spawn");
    let pid = worker.pid().await;
    assert!(pid.is_some(), "worker should have a pid");
    assert!(!worker.has_exited().await, "worker should start alive");

    worker.shutdown().await.expect("shutdown");

    assert!(
        worker.has_exited().await,
        "worker should be reaped after shutdown"
    );

    // A request after shutdown must not be accepted by a dead worker.
    let err = worker.analyze("__zero__").await.expect_err("shutdown");
    assert!(
        matches!(
            err,
            FaceWorkerError::NotRunning
                | FaceWorkerError::Shutdown
                | FaceWorkerError::WorkerCrashed(_)
        ),
        "got {err:?}"
    );
}

#[tokio::test]
async fn implements_core_face_engine_boundary() {
    use std::sync::Arc;
    use tekmerion_core::{FaceEngine, InputPayload, PipelineError};

    let worker: Arc<dyn FaceEngine> = Arc::new(FaceWorker::spawn(&config()).expect("spawn"));

    let (_dir, image) = dummy_image();
    let input = InputPayload::new(image.to_string_lossy()).expect("input");
    let analysis = worker.analyze(&input).await.expect("engine analyze");
    assert_eq!(analysis.embeddings.len(), 1);

    // Error path maps to a PipelineError::Stage for FaceAnalysis.
    let missing = InputPayload::new("__missing__").expect("input");
    let err = worker.analyze(&missing).await.expect_err("should fail");
    assert!(matches!(
        err,
        PipelineError::Stage {
            stage: tekmerion_core::PipelineStage::FaceAnalysis,
            ..
        }
    ));
}
