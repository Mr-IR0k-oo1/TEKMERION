//! Errors produced by the local face-analysis worker client.

/// A structured error from spawning or talking to the face-analysis worker.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FaceWorkerError {
    /// The worker subprocess could not be started.
    #[error("failed to spawn face worker: {0}")]
    Spawn(String),

    /// An I/O failure writing to the worker's stdin or reading its stdout.
    #[error("face worker I/O error: {0}")]
    Io(String),

    /// A request did not receive a response within its timeout.
    #[error("face analysis timed out for request {0}")]
    Timeout(String),

    /// The worker process exited (or its pipe closed) before answering.
    #[error("face worker crashed: {0}")]
    WorkerCrashed(String),

    /// The worker died and has not been restarted; no further requests can run.
    #[error("face worker is not running")]
    NotRunning,

    /// The worker produced a response that could not be parsed or did not
    /// honor the protocol (wrong shape, missing fields, unexpected content).
    #[error("invalid face worker response: {0}")]
    InvalidResponse(String),

    /// The worker explicitly reported a failed analysis with structured errors.
    #[error("face analysis failed: {errors:?}")]
    RequestFailed {
        /// Structured error summaries returned by the worker.
        errors: Vec<String>,
    },

    /// A request was aborted because the worker was shut down.
    #[error("face worker shut down during request")]
    Shutdown,
}

impl FaceWorkerError {
    /// A human-friendly label used for logging / diagnostics.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Spawn(_) => "spawn",
            Self::Io(_) => "io",
            Self::Timeout(_) => "timeout",
            Self::WorkerCrashed(_) => "crashed",
            Self::NotRunning => "not_running",
            Self::InvalidResponse(_) => "invalid_response",
            Self::RequestFailed { .. } => "request_failed",
            Self::Shutdown => "shutdown",
        }
    }
}
