//! Face model module

use crate::error::AppError;
use crate::face::client::{FaceWorkerClient, FaceWorkerResponse};
use std::path::Path;
use tracing::info;

/// Face model
pub struct FaceModel {
    worker_client: FaceWorkerClient,
}

impl FaceModel {
    /// Create a new face model
    pub fn new() -> Result<Self, AppError> {
        let worker_client = FaceWorkerClient::new()?;
        Ok(Self { worker_client })
    }

    /// Process an image to get face embeddings
    pub async fn process_image(&mut self, image_path: &str) -> Result<FaceWorkerResponse, AppError> {
        info!("Processing image: {}", image_path);

        // Verify the image exists
        if !Path::new(image_path).exists() {
            return Err(AppError::WorkerError("Image file not found".to_string()));
        }

        // Send the request to the worker
        let response = self.worker_client.send_request(image_path).await?;

        Ok(response)
    }

    /// Check if the worker is still running
    pub fn is_worker_running(&mut self) -> bool {
        self.worker_client.is_running()
    }
}
