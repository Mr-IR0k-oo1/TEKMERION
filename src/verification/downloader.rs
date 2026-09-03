//! Image downloader module

use crate::error::AppError;
use reqwest::{Client, StatusCode};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::NamedTempFile;
use tracing::{info, error, warn};
use url::Url;

/// Image downloader
pub struct ImageDownloader {
    client: Client,
    max_size_bytes: usize,
    timeout: Duration,
}

impl ImageDownloader {
    /// Create a new image downloader
    pub fn new(max_size_bytes: usize, timeout_seconds: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            max_size_bytes,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    /// Download an image from a URL
    pub async fn download_image(&self, url: &str, temp_dir: &Path) -> Result<PathBuf, AppError> {
        info!("Downloading image from: {}
