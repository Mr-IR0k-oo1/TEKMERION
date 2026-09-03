//! JSON Lines client for the local face-analysis worker.
//!
//! Spawns the Python worker as a child process and communicates over
//! stdin/stdout JSON Lines, correlating each response to its originating
//! request id so concurrent requests are routed correctly. Long-running
//! inference happens inside the worker process, so awaiting a reply never
//! blocks the host's executor (and therefore the TUI).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use std::process::Stdio;

use chrono::Utc;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;

use tekmerion_core::{FaceAnalysis, InputPayload};

use crate::error::FaceWorkerError;
use crate::protocol::WorkerResponse;

/// Friendly default interpreter name for the current platform.
fn default_python() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
}

/// Configuration for launching and talking to the face-analysis worker.
#[derive(Debug, Clone)]
pub struct FaceWorkerConfig {
    /// Python interpreter used to launch the worker script.
    pub python: String,
    /// Path to the worker Python script.
    pub script: PathBuf,
    /// Maximum time to wait for a single request's response.
    pub request_timeout: Duration,
}

impl Default for FaceWorkerConfig {
    fn default() -> Self {
        Self {
            python: default_python().to_string(),
            script: PathBuf::from("workers/face/worker.py"),
            request_timeout: Duration::from_secs(30),
        }
    }
}

impl FaceWorkerConfig {
    /// Set the Python interpreter (e.g. "python", "python3", a full path).
    pub fn with_python(mut self, python: impl Into<String>) -> Self {
        self.python = python.into();
        self
    }

    /// Set the worker script path.
    pub fn with_script(mut self, script: impl Into<PathBuf>) -> Self {
        self.script = script.into();
        self
    }

    /// Set the per-request timeout.
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
}

/// A JSONL request sent to the worker.
#[derive(Serialize)]
struct Request<'a> {
    request_id: String,
    operation: &'a str,
    image_path: &'a str,
}

type PendingMap = Mutex<HashMap<String, oneshot::Sender<Result<WorkerResponse, FaceWorkerError>>>>;

/// Shared state owned by both the client and its background reader task.
struct WorkerInner {
    child: Mutex<Child>,
    reader: Mutex<Option<JoinHandle<()>>>,
    pending: Arc<PendingMap>,
    next_id: AtomicU64,
    write: Mutex<tokio::process::ChildStdin>,
    request_timeout: Duration,
    crashed: Arc<AtomicBool>,
}

impl Drop for WorkerInner {
    fn drop(&mut self) {
        // Ensure the reader task stops if the client is dropped without an
        // explicit shutdown; the child is killed via `kill_on_drop`.
        if let Ok(mut reader) = self.reader.try_lock() {
            if let Some(handle) = reader.take() {
                handle.abort();
            }
        }
    }
}

/// Client over a spawned face-analysis worker process.
pub struct FaceWorker {
    inner: Arc<WorkerInner>,
}

impl FaceWorker {
    /// Spawn a worker using the given configuration.
    pub fn spawn(config: &FaceWorkerConfig) -> Result<Self, FaceWorkerError> {
        let mut child = Command::new(&config.python)
            .arg(&config.script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| FaceWorkerError::Spawn(e.to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| FaceWorkerError::Spawn("worker stdin unavailable".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| FaceWorkerError::Spawn("worker stdout unavailable".to_string()))?;

        let pending: Arc<PendingMap> = Arc::new(Mutex::new(HashMap::new()));
        let crashed = Arc::new(AtomicBool::new(false));

        let reader = spawn_reader(pending.clone(), stdout, crashed.clone());

        Ok(Self {
            inner: Arc::new(WorkerInner {
                child: Mutex::new(child),
                reader: Mutex::new(Some(reader)),
                pending,
                next_id: AtomicU64::new(0),
                write: Mutex::new(stdin),
                request_timeout: config.request_timeout,
                crashed,
            }),
        })
    }

    /// Run the `analyze` operation against `image_path`.
    ///
    /// Returns a [`FaceAnalysis`] with every detected face represented: an
    /// empty `detections` vector is an explicit, valid zero-face result.
    pub async fn analyze(
        &self,
        image_path: impl AsRef<Path>,
    ) -> Result<FaceAnalysis, FaceWorkerError> {
        let path = image_path.as_ref().to_string_lossy().into_owned();
        let response = self.dispatch("analyze", &path).await?;
        let (detections, embeddings) = response.into_semantics()?;
        Ok(FaceAnalysis {
            detections,
            embeddings,
            timestamp: Utc::now(),
        })
    }

    /// Send a request and await its correlated response.
    async fn dispatch(
        &self,
        operation: &str,
        image_path: &str,
    ) -> Result<WorkerResponse, FaceWorkerError> {
        if self.inner.crashed.load(Ordering::SeqCst) {
            return Err(FaceWorkerError::NotRunning);
        }

        let request_id = self.next_request_id();
        let (tx, rx) = oneshot::channel();
        self.inner
            .pending
            .lock()
            .await
            .insert(request_id.clone(), tx);

        let request = Request {
            request_id: request_id.clone(),
            operation,
            image_path,
        };
        let line = serde_json::to_string(&request)
            .map_err(|e| FaceWorkerError::InvalidResponse(e.to_string()))?;

        {
            let mut write = self.inner.write.lock().await;
            write
                .write_all(line.as_bytes())
                .await
                .map_err(|e| self.io_or_shutdown(e))?;
            write
                .write_all(b"\n")
                .await
                .map_err(|e| self.io_or_shutdown(e))?;
            write.flush().await.map_err(|e| self.io_or_shutdown(e))?;
        }

        match tokio::time::timeout(self.inner.request_timeout, rx).await {
            Err(_elapsed) => {
                // A very late response must not be mis-correlated.
                self.inner.pending.lock().await.remove(&request_id);
                Err(FaceWorkerError::Timeout(request_id))
            }
            Ok(Err(_recv)) => Err(FaceWorkerError::Shutdown),
            Ok(Ok(result)) => result,
        }
    }

    fn io_or_shutdown(&self, _e: std::io::Error) -> FaceWorkerError {
        if self.inner.crashed.load(Ordering::SeqCst) {
            FaceWorkerError::WorkerCrashed("pipe closed while writing".to_string())
        } else {
            FaceWorkerError::Io(_e.to_string())
        }
    }

    /// Generate a process-unique request id.
    fn next_request_id(&self) -> String {
        let n = self.inner.next_id.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{n}-{nanos}")
    }

    /// Whether the worker process has exited and been reaped by the OS.
    pub async fn has_exited(&self) -> bool {
        let mut child = self.inner.child.lock().await;
        match child.try_wait() {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(_) => true,
        }
    }

    /// The worker's OS process id, if it is still running.
    pub async fn pid(&self) -> Option<u32> {
        self.inner.child.lock().await.id()
    }

    /// Gracefully stop the worker: fail in-flight requests, stop the reader
    /// task and terminate + reap the child process so cleanup is complete.
    pub async fn shutdown(&self) -> Result<(), FaceWorkerError> {
        self.inner.crashed.store(true, Ordering::SeqCst);
        if let Some(reader) = self.inner.reader.lock().await.take() {
            reader.abort();
        }
        fail_all(&self.inner.pending, FaceWorkerError::Shutdown).await;
        self.kill_and_reap().await;
        Ok(())
    }

    /// Kill the child process (if still running) and wait for it to be reaped.
    async fn kill_and_reap(&self) {
        let mut child = self.inner.child.lock().await;
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        let _ = child.kill().await;
        drop(child);
        for _ in 0..100 {
            if self.has_exited().await {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
}

/// Background task that reads worker stdout, parses responses and routes them
/// to their waiting request by id. Exits (failing all pending requests) when
/// the worker's stdout closes or becomes unreadable.
fn spawn_reader(
    pending: Arc<PendingMap>,
    stdout: tokio::process::ChildStdout,
    crashed: Arc<AtomicBool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        loop {
            match reader.next_line().await {
                Ok(Some(line)) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<WorkerResponse>(line) {
                        Ok(response) => {
                            if let Some(id) = response.request_id.clone() {
                                let sender = pending.lock().await.remove(&id);
                                if let Some(tx) = sender {
                                    let _ = tx.send(Ok(response));
                                }
                            }
                        }
                        Err(_) => {
                            crashed.store(true, Ordering::SeqCst);
                            fail_all(
                                &pending,
                                FaceWorkerError::InvalidResponse(
                                    "worker emitted an unparseable line".to_string(),
                                ),
                            )
                            .await;
                            break;
                        }
                    }
                }
                Ok(None) => {
                    // EOF: the worker exited or closed stdout.
                    crashed.store(true, Ordering::SeqCst);
                    fail_all(
                        &pending,
                        FaceWorkerError::WorkerCrashed("worker pipe closed".to_string()),
                    )
                    .await;
                    break;
                }
                Err(e) => {
                    crashed.store(true, Ordering::SeqCst);
                    fail_all(&pending, FaceWorkerError::Io(e.to_string())).await;
                    break;
                }
            }
        }
    })
}

/// Send an error to every request currently awaiting a response.
async fn fail_all(pending: &Arc<PendingMap>, error: FaceWorkerError) {
    let senders: Vec<_> = {
        let mut map = pending.lock().await;
        map.drain().map(|(_, tx)| tx).collect()
    };
    for tx in senders {
        let _ = tx.send(Err(error.clone()));
    }
}

/// Adapter implementing the pipeline's [`tekmerion_core::FaceEngine`] boundary.
///
/// The worker JSONL client interprets `InputPayload::source` as the image
/// path to analyze.
#[async_trait::async_trait]
impl tekmerion_core::FaceEngine for FaceWorker {
    async fn analyze(
        &self,
        input: &InputPayload,
    ) -> Result<FaceAnalysis, tekmerion_core::PipelineError> {
        Self::analyze(self, &input.source)
            .await
            .map_err(|e| tekmerion_core::PipelineError::Stage {
                stage: tekmerion_core::PipelineStage::FaceAnalysis,
                message: e.to_string(),
            })
    }
}
