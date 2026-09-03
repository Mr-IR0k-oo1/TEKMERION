//! Face worker client module

use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::process::{Child, Command, Stdio};
use std::io::{BufRead, BufReader, Write};
use uuid::Uuid;
use tokio::sync::mpsc;
use tracing::{info, error};

/// Face worker client
pub struct FaceWorkerClient {
    child: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

/// Request to the face worker
#[derive(Debug, Serialize)]
pub struct FaceWorkerRequest {
    pub request_id: String,
    pub operation: String,
    pub image_path: String,
}

/// Response from the face worker
#[derive(Debug, Deserialize)]
pub struct FaceWorkerResponse {
    pub request_id: String,
    pub success: bool,
    pub face_count: Option<u32>,
    pub embedding: Option<Vec<f32>>,
    pub bbox: Option<Vec<i32>>,
    pub error: Option<String>,
    pub faces: Option<Vec<FaceInfo>>,
}

/// Face information
#[derive(Debug, Deserialize)]
pub struct FaceInfo {
    pub bbox: Vec<i32>,
    pub embedding: Vec<f32>,
}

impl FaceWorkerClient {
    /// Create a new face worker client
    pub fn new() -> Result<Self, AppError> {
        info!("Starting face worker");

        // Spawn the Python worker process
        let mut child = Command::new("python")
            .arg("face-worker/worker.py")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Get handles to stdin and stdout
        let stdin = child.stdin.take().ok_or(AppError::WorkerError("Failed to get stdin".to_string()))?;
        let stdout = BufReader::new(child.stdout.take().ok_or(AppError::WorkerError("Failed to get stdout".to_string()))?);

        Ok(Self { child, stdin, stdout })
    }

    /// Send a request to the face worker
    pub async fn send_request(&mut self, image_path: &str) -> Result<FaceWorkerResponse, AppError> {
        let request_id = Uuid::new_v4().to_string();
        let request = FaceWorkerRequest {
            request_id: request_id.clone(),
            operation: "embed".to_string(),
            image_path: image_path.to_string(),
        };

        // Serialize the request
        let request_json = serde_json::to_string(&request)?;

        // Send the request to the worker
        writeln!(self.stdin, "{}", request_json)?;

        // Read the response
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line)?;

        // Deserialize the response
        let response: FaceWorkerResponse = serde_json::from_str(&response_line)?;

        // Verify the request_id matches
        if response.request_id != request_id {
            return Err(AppError::WorkerError("Request ID mismatch".to_string()));
        }

        Ok(response)
    }

    /// Check if the worker process is still running
    pub fn is_running(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization() {
        let request = FaceWorkerRequest {
            request_id: "123".to_string(),
            operation: "embed".to_string(),
            image_path: "/path/to/image.jpg".to_string(),
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let expected = json!({
            "request_id": "123",
            "operation": "embed",
            "image_path": "/path/to/image.jpg"
        }).to_string();

        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_response_deserialization() {
        let json_response = r#"{
            "request_id": "123",
            "success": true,
            "face_count": 1,
            "embedding": [0.1, 0.2, 0.3],
            "bbox": [10, 20, 30, 40]
        }"#;

        let response: FaceWorkerResponse = serde_json::from_str(json_response).unwrap();

        assert_eq!(response.request_id, "123");
        assert!(response.success);
        assert_eq!(response.face_count, Some(1));
        assert_eq!(response.embedding, Some(vec![0.1, 0.2, 0.3]));
        assert_eq!(response.bbox, Some(vec![10, 20, 30, 40]));
    }
}
