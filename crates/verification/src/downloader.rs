//! Secure candidate-image downloader.
//!
//! Provides safe, streaming image downloading for reverse-image candidates
//! with strict security constraints:
//! - Never trusts Content-Length alone; streaming downloads abort immediately if limit exceeded.
//! - Content-Type validation and magic-byte inspection (detects and blocks executables).
//! - Automatic SHA-256 checksum generation during streaming.
//! - Safe temporary file storage and RAII cleanup.
//! - Enforces configurable timeouts and response size limits.
//! - Downloaded files are never executed.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

/// Default maximum allowed image download size (10 MB).
pub const DEFAULT_MAX_DOWNLOAD_BYTES: usize = 10 * 1024 * 1024;

/// Default download timeout in seconds (15s).
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 15;

static FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Structured errors returned by the image downloader.
#[derive(Debug, Error)]
pub enum DownloadError {
    /// Provided URL failed parsing or scheme validation.
    #[error("invalid URL '{url}': {reason}")]
    InvalidUrl { url: String, reason: String },

    /// Insecure HTTP scheme when HTTPS is strictly required.
    #[error("insecure scheme for URL '{url}': HTTPS is required")]
    InsecureScheme { url: String },

    /// Image size exceeded the configured maximum limit.
    #[error("download size exceeded limit of {limit_bytes} bytes (received: {actual_bytes:?} bytes)")]
    Oversized {
        limit_bytes: usize,
        actual_bytes: Option<usize>,
    },

    /// Upstream Content-Type is unsupported or dangerous.
    #[error("unsupported Content-Type '{content_type}'; allowed: {allowed:?}")]
    UnsupportedContentType {
        content_type: String,
        allowed: Vec<String>,
    },

    /// File payload contains an executable binary header signature.
    #[error("executable format detected in download payload: {signature}")]
    ExecutableDetected { signature: String },

    /// Payload bytes do not match expected image magic signatures.
    #[error("invalid image format: {reason}")]
    InvalidImageFormat { reason: String },

    /// Download timed out.
    #[error("download timed out after {timeout_ms}ms for URL '{url}'")]
    Timeout { url: String, timeout_ms: u64 },

    /// HTTP request failed with non-2xx status code.
    #[error("HTTP error {status} for URL '{url}': {message}")]
    Http {
        status: u16,
        url: String,
        message: String,
    },

    /// File system / I/O failure during temporary file writing.
    #[error("I/O error while writing '{path}': {message}")]
    Io { path: String, message: String },

    /// Internal or network transport error.
    #[error("internal transport error: {0}")]
    Internal(String),
}

/// Configuration options for the secure image downloader.
#[derive(Debug, Clone)]
pub struct DownloaderConfig {
    /// Maximum allowed download size in bytes.
    pub max_download_bytes: usize,
    /// Total download timeout.
    pub timeout: Duration,
    /// Strictly require HTTPS URLs (if false, HTTP is permitted but HTTPS is still preferred).
    pub require_https: bool,
    /// Whitelist of allowed image MIME types.
    pub allowed_content_types: Vec<String>,
    /// Safe directory where temporary downloaded images are placed.
    pub temp_dir: PathBuf,
}

impl Default for DownloaderConfig {
    fn default() -> Self {
        Self {
            max_download_bytes: DEFAULT_MAX_DOWNLOAD_BYTES,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECONDS),
            require_https: false,
            allowed_content_types: vec![
                "image/jpeg".to_string(),
                "image/jpg".to_string(),
                "image/png".to_string(),
                "image/webp".to_string(),
                "image/gif".to_string(),
                "image/bmp".to_string(),
                "image/x-ms-bmp".to_string(),
                "image/avif".to_string(),
            ],
            temp_dir: std::env::temp_dir().join("tekmerion_downloads"),
        }
    }
}

impl DownloaderConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_download_bytes = max_bytes;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_require_https(mut self, require_https: bool) -> Self {
        self.require_https = require_https;
        self
    }

    pub fn with_temp_dir(mut self, temp_dir: impl Into<PathBuf>) -> Self {
        self.temp_dir = temp_dir.into();
        self
    }
}

/// Represents a securely downloaded image on disk.
///
/// Features RAII cleanup: when dropped, if `delete_on_drop` is true (default: true),
/// the temporary file on disk is removed cleanly.
pub struct DownloadedImage {
    /// Safe local path to the downloaded image file.
    pub path: PathBuf,
    /// Hex-encoded SHA-256 checksum generated during streaming.
    pub sha256: String,
    /// Total byte count of the image.
    pub byte_count: usize,
    /// Validated MIME content type.
    pub content_type: String,
    /// Whether to delete the file when this struct is dropped.
    pub delete_on_drop: bool,
}

impl fmt::Debug for DownloadedImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DownloadedImage")
            .field("path", &self.path)
            .field("sha256", &self.sha256)
            .field("byte_count", &self.byte_count)
            .field("content_type", &self.content_type)
            .field("delete_on_drop", &self.delete_on_drop)
            .finish()
    }
}

impl DownloadedImage {
    /// Persist the image file, disabling automatic RAII deletion on drop.
    pub fn persist(&mut self) -> &Path {
        self.delete_on_drop = false;
        &self.path
    }

    /// Explicitly delete the downloaded file from disk.
    pub fn cleanup(&mut self) -> std::io::Result<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        self.delete_on_drop = false;
        Ok(())
    }
}

impl Drop for DownloadedImage {
    fn drop(&mut self) {
        if self.delete_on_drop && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Secure candidate image downloader.
pub struct ImageDownloader {
    config: DownloaderConfig,
    client: Client,
}

impl ImageDownloader {
    /// Create a new downloader with default configuration.
    pub fn new() -> Result<Self, DownloadError> {
        Self::with_config(DownloaderConfig::default())
    }

    /// Create a new downloader with explicit configuration.
    pub fn with_config(config: DownloaderConfig) -> Result<Self, DownloadError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| DownloadError::Internal(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// Access downloader configuration.
    pub fn config(&self) -> &DownloaderConfig {
        &self.config
    }

    /// Download an image securely from the provided URL.
    ///
    /// Validates scheme, streaming size limits, content types, and binary magic bytes.
    /// In case of any error during download, partial files are immediately deleted.
    pub async fn download(&self, raw_url: &str) -> Result<DownloadedImage, DownloadError> {
        let parsed_url = Url::parse(raw_url).map_err(|e| DownloadError::InvalidUrl {
            url: raw_url.to_string(),
            reason: e.to_string(),
        })?;

        let scheme = parsed_url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(DownloadError::InvalidUrl {
                url: raw_url.to_string(),
                reason: format!("unsupported scheme '{}'; only http/https allowed", scheme),
            });
        }

        if self.config.require_https && scheme != "https" {
            return Err(DownloadError::InsecureScheme {
                url: raw_url.to_string(),
            });
        }

        tokio::time::timeout(self.config.timeout, self.download_internal(&parsed_url))
            .await
            .map_err(|_| DownloadError::Timeout {
                url: raw_url.to_string(),
                timeout_ms: self.config.timeout.as_millis() as u64,
            })?
    }

    async fn download_internal(&self, url: &Url) -> Result<DownloadedImage, DownloadError> {
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DownloadError::Timeout {
                        url: url.to_string(),
                        timeout_ms: self.config.timeout.as_millis() as u64,
                    }
                } else {
                    DownloadError::Internal(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(DownloadError::Http {
                status: status.as_u16(),
                url: url.to_string(),
                message: format!("HTTP {}", status),
            });
        }

        // 1. Content-Type Header Validation
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        if !self.config.allowed_content_types.iter().any(|ct| ct == &content_type) {
            return Err(DownloadError::UnsupportedContentType {
                content_type,
                allowed: self.config.allowed_content_types.clone(),
            });
        }

        // 2. Early Content-Length Check (Note: NEVER trust Content-Length alone!)
        if let Some(content_length) = response.content_length() {
            if content_length > self.config.max_download_bytes as u64 {
                return Err(DownloadError::Oversized {
                    limit_bytes: self.config.max_download_bytes,
                    actual_bytes: Some(content_length as usize),
                });
            }
        }

        // 3. Prepare Safe Temporary Destination
        tokio::fs::create_dir_all(&self.config.temp_dir).await.map_err(|e| {
            DownloadError::Io {
                path: self.config.temp_dir.display().to_string(),
                message: format!("failed to create temp download dir: {}", e),
            }
        })?;

        let extension = match content_type.as_str() {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            "image/bmp" | "image/x-ms-bmp" => "bmp",
            "image/avif" => "avif",
            _ => "img",
        };

        let unique_id = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let filename = format!("candidate_{}_{}_{:06}.{}", timestamp, std::process::id(), unique_id, extension);
        let dest_path = self.config.temp_dir.join(filename);

        async fn delete_if_exists(path: &Path) {
            if path.exists() {
                let _ = tokio::fs::remove_file(path).await;
            }
        }

        // Open destination file
        use tokio::io::AsyncWriteExt;
        let mut file = match tokio::fs::File::create(&dest_path).await {
            Ok(f) => f,
            Err(e) => {
                return Err(DownloadError::Io {
                    path: dest_path.display().to_string(),
                    message: format!("failed to create destination file: {}", e),
                });
            }
        };

        let mut stream = response.bytes_stream();
        let mut downloaded_bytes = 0usize;
        let mut hasher = Sha256::new();
        let mut first_chunk = true;

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    delete_if_exists(&dest_path).await;
                    return Err(DownloadError::Internal(format!("stream read error: {}", e)));
                }
            };

            // 4. Inspect magic bytes on the first chunk
            if first_chunk {
                if let Err(magic_err) = validate_magic_bytes(&chunk, &content_type) {
                    delete_if_exists(&dest_path).await;
                    return Err(magic_err);
                }
                first_chunk = false;
            }

            downloaded_bytes += chunk.len();

            // 5. Abort downloads exceeding MAX_DOWNLOAD_BYTES immediately
            if downloaded_bytes > self.config.max_download_bytes {
                delete_if_exists(&dest_path).await;
                return Err(DownloadError::Oversized {
                    limit_bytes: self.config.max_download_bytes,
                    actual_bytes: Some(downloaded_bytes),
                });
            }

            hasher.update(&chunk);

            if let Err(e) = file.write_all(&chunk).await {
                delete_if_exists(&dest_path).await;
                return Err(DownloadError::Io {
                    path: dest_path.display().to_string(),
                    message: format!("failed to write chunk: {}", e),
                });
            }
        }

        if let Err(e) = file.flush().await {
            delete_if_exists(&dest_path).await;
            return Err(DownloadError::Io {
                path: dest_path.display().to_string(),
                message: format!("failed to flush file: {}", e),
            });
        }

        if downloaded_bytes == 0 {
            delete_if_exists(&dest_path).await;
            return Err(DownloadError::InvalidImageFormat {
                reason: "downloaded 0 bytes".to_string(),
            });
        }

        let sha256 = hex::encode(hasher.finalize());

        tracing::info!(
            path = %dest_path.display(),
            bytes = downloaded_bytes,
            sha256 = %sha256,
            "Successfully downloaded and verified candidate image"
        );

        Ok(DownloadedImage {
            path: dest_path,
            sha256,
            byte_count: downloaded_bytes,
            content_type,
            delete_on_drop: true,
        })
    }
}

/// Inspect binary payload signatures.
///
/// Strictly detects and rejects executable formats (Windows PE/MZ, ELF, Mach-O, scripts)
/// and verifies that initial bytes match the advertised image format.
pub fn validate_magic_bytes(bytes: &[u8], content_type: &str) -> Result<(), DownloadError> {
    if bytes.len() < 2 {
        return Err(DownloadError::InvalidImageFormat {
            reason: "payload too short for magic header validation".to_string(),
        });
    }

    // Check known executable binary signatures
    // 1. Windows PE / DOS MZ executable: "MZ"
    if bytes.starts_with(b"MZ") {
        return Err(DownloadError::ExecutableDetected {
            signature: "Windows PE/MZ executable (MZ header)".to_string(),
        });
    }

    // 2. Linux ELF: 0x7F 'E' 'L' 'F'
    if bytes.len() >= 4 && bytes.starts_with(b"\x7FELF") {
        return Err(DownloadError::ExecutableDetected {
            signature: "Linux ELF executable".to_string(),
        });
    }

    // 3. Mach-O executables
    if bytes.len() >= 4 {
        let head4 = &bytes[..4];
        if head4 == [0xFE, 0xED, 0xFA, 0xCE]
            || head4 == [0xCE, 0xFA, 0xED, 0xFE]
            || head4 == [0xFE, 0xED, 0xFA, 0xCF]
            || head4 == [0xCF, 0xFA, 0xED, 0xFE]
            || head4 == [0xCA, 0xFE, 0xBA, 0xBE]
        {
            return Err(DownloadError::ExecutableDetected {
                signature: "Mach-O executable binary".to_string(),
            });
        }
    }

    // 4. Shell script hashbang: "#!"
    if bytes.starts_with(b"#!") {
        return Err(DownloadError::ExecutableDetected {
            signature: "Shell script (hashbang #!)".to_string(),
        });
    }

    // 5. HTML / XML / JavaScript documents disguised as images
    let prefix_str = String::from_utf8_lossy(&bytes[..bytes.len().min(64)]).to_ascii_lowercase();
    if prefix_str.starts_with("<!doctype") || prefix_str.starts_with("<html") || prefix_str.starts_with("<script") {
        return Err(DownloadError::ExecutableDetected {
            signature: "HTML or Script document".to_string(),
        });
    }

    // Verify expected image format magic bytes
    match content_type {
        "image/jpeg" | "image/jpg" => {
            if bytes.len() < 3 || bytes[0] != 0xFF || bytes[1] != 0xD8 || bytes[2] != 0xFF {
                return Err(DownloadError::InvalidImageFormat {
                    reason: "JPEG magic bytes mismatch (expected 0xFF, 0xD8, 0xFF)".to_string(),
                });
            }
        }
        "image/png" => {
            const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
            if bytes.len() < 8 || !bytes.starts_with(&PNG_MAGIC) {
                return Err(DownloadError::InvalidImageFormat {
                    reason: "PNG magic bytes mismatch".to_string(),
                });
            }
        }
        "image/gif" => {
            if bytes.len() < 6 || (!bytes.starts_with(b"GIF87a") && !bytes.starts_with(b"GIF89a")) {
                return Err(DownloadError::InvalidImageFormat {
                    reason: "GIF magic bytes mismatch (expected GIF87a or GIF89a)".to_string(),
                });
            }
        }
        "image/webp" => {
            // RIFF....WEBP
            if bytes.len() < 12 || !bytes.starts_with(b"RIFF") || &bytes[8..12] != b"WEBP" {
                return Err(DownloadError::InvalidImageFormat {
                    reason: "WebP magic bytes mismatch (expected RIFF....WEBP)".to_string(),
                });
            }
        }
        "image/bmp" | "image/x-ms-bmp" => {
            if !bytes.starts_with(b"BM") {
                return Err(DownloadError::InvalidImageFormat {
                    reason: "BMP magic bytes mismatch (expected BM)".to_string(),
                });
            }
        }
        _ => {
            // Permitted types without explicit magic bytes check (e.g. avif) pass if not executable
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic valid JPEG header + data
    pub const VALID_JPEG: [u8; 14] = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0xFF, 0xD9,
    ];

    // Synthetic valid PNG header + data
    pub const VALID_PNG: [u8; 16] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    ];

    async fn start_mock_server(
        status_code: u16,
        status_text: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
        chunk_delay: Option<Duration>,
    ) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let status_text = status_text.to_string();
        let headers: Vec<(String, String)> = headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf).await;

                if let Some(delay) = chunk_delay {
                    tokio::time::sleep(delay).await;
                }

                let mut resp = format!("HTTP/1.1 {} {}\r\n", status_code, status_text);
                for (k, v) in headers {
                    resp.push_str(&format!("{}: {}\r\n", k, v));
                }
                resp.push_str("\r\n");
                let _ = stream.write_all(resp.as_bytes()).await;

                if !body.is_empty() {
                    let _ = stream.write_all(&body).await;
                }
                let _ = stream.flush().await;
            }
        });

        format!("http://127.0.0.1:{}/image", port)
    }

    #[tokio::test]
    async fn test_successful_image_download() {
        let url = start_mock_server(
            200,
            "OK",
            &[
                ("Content-Type", "image/jpeg"),
                ("Content-Length", &VALID_JPEG.len().to_string()),
            ],
            VALID_JPEG.to_vec(),
            None,
        )
        .await;

        let downloader = ImageDownloader::new().unwrap();
        let downloaded = downloader.download(&url).await.unwrap();

        assert!(downloaded.path.exists());
        assert_eq!(downloaded.byte_count, VALID_JPEG.len());
        assert_eq!(downloaded.content_type, "image/jpeg");

        // Verify SHA-256 matches actual byte hash
        let mut hasher = Sha256::new();
        hasher.update(&VALID_JPEG);
        let expected_sha256 = hex::encode(hasher.finalize());
        assert_eq!(downloaded.sha256, expected_sha256);

        // Verify cleanup on drop
        let path = downloaded.path.clone();
        drop(downloaded);
        assert!(!path.exists(), "Downloaded image must be removed on drop");
    }

    #[tokio::test]
    async fn test_successful_png_download() {
        let url = start_mock_server(
            200,
            "OK",
            &[
                ("Content-Type", "image/png"),
                ("Content-Length", &VALID_PNG.len().to_string()),
            ],
            VALID_PNG.to_vec(),
            None,
        )
        .await;

        let downloader = ImageDownloader::new().unwrap();
        let downloaded = downloader.download(&url).await.unwrap();

        assert!(downloaded.path.exists());
        assert_eq!(downloaded.byte_count, VALID_PNG.len());
        assert_eq!(downloaded.content_type, "image/png");

        let mut hasher = Sha256::new();
        hasher.update(&VALID_PNG);
        let expected_sha256 = hex::encode(hasher.finalize());
        assert_eq!(downloaded.sha256, expected_sha256);
    }

    #[tokio::test]
    async fn test_oversized_response() {
        // Stream 5000 bytes when max allowed is 1000
        let big_body = vec![0u8; 5000];
        let mut jpeg_payload = VALID_JPEG.to_vec();
        jpeg_payload.extend_from_slice(&big_body);

        let url = start_mock_server(
            200,
            "OK",
            &[("Content-Type", "image/jpeg")],
            jpeg_payload,
            None,
        )
        .await;

        let config = DownloaderConfig::default().with_max_bytes(1000);
        let downloader = ImageDownloader::with_config(config).unwrap();

        let err = downloader.download(&url).await.unwrap_err();
        match err {
            DownloadError::Oversized { limit_bytes, actual_bytes } => {
                assert_eq!(limit_bytes, 1000);
                assert!(actual_bytes.unwrap() > 1000);
            }
            other => panic!("expected Oversized error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_oversized_content_length_rejected_early() {
        let url = start_mock_server(
            200,
            "OK",
            &[
                ("Content-Type", "image/jpeg"),
                ("Content-Length", "999999999"),
            ],
            vec![],
            None,
        )
        .await;

        let config = DownloaderConfig::default().with_max_bytes(5000);
        let downloader = ImageDownloader::with_config(config).unwrap();

        let err = downloader.download(&url).await.unwrap_err();
        assert!(matches!(err, DownloadError::Oversized { .. }));
    }

    #[tokio::test]
    async fn test_unsupported_content_type() {
        let url = start_mock_server(
            200,
            "OK",
            &[("Content-Type", "text/html; charset=utf-8")],
            b"<html><body>Not an image</body></html>".to_vec(),
            None,
        )
        .await;

        let downloader = ImageDownloader::new().unwrap();
        let err = downloader.download(&url).await.unwrap_err();

        match err {
            DownloadError::UnsupportedContentType { content_type, .. } => {
                assert_eq!(content_type, "text/html");
            }
            other => panic!("expected UnsupportedContentType, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_executable_magic_bytes_rejected() {
        // Lies with Content-Type: image/jpeg, but payload starts with Windows PE 'MZ'
        let fake_jpeg = b"MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00".to_vec();
        let url = start_mock_server(
            200,
            "OK",
            &[("Content-Type", "image/jpeg")],
            fake_jpeg,
            None,
        )
        .await;

        let downloader = ImageDownloader::new().unwrap();
        let err = downloader.download(&url).await.unwrap_err();

        match err {
            DownloadError::ExecutableDetected { signature } => {
                assert!(signature.contains("MZ"));
            }
            other => panic!("expected ExecutableDetected error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_timeout() {
        let url = start_mock_server(
            200,
            "OK",
            &[("Content-Type", "image/jpeg")],
            VALID_JPEG.to_vec(),
            Some(Duration::from_millis(300)),
        )
        .await;

        let config = DownloaderConfig::default().with_timeout(Duration::from_millis(50));
        let downloader = ImageDownloader::with_config(config).unwrap();

        let err = downloader.download(&url).await.unwrap_err();
        assert!(matches!(err, DownloadError::Timeout { .. }));
    }

    #[tokio::test]
    async fn test_invalid_url() {
        let downloader = ImageDownloader::new().unwrap();

        let err1 = downloader.download("not a valid url").await.unwrap_err();
        assert!(matches!(err1, DownloadError::InvalidUrl { .. }));

        let err2 = downloader.download("ftp://example.com/pic.jpg").await.unwrap_err();
        assert!(matches!(err2, DownloadError::InvalidUrl { .. }));
    }

    #[tokio::test]
    async fn test_https_required_rejects_http() {
        let config = DownloaderConfig::default().with_require_https(true);
        let downloader = ImageDownloader::with_config(config).unwrap();

        let err = downloader.download("http://example.com/pic.jpg").await.unwrap_err();
        assert!(matches!(err, DownloadError::InsecureScheme { .. }));
    }
}
