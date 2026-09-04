//! Live integration test mode for TEKMERION reverse-image discovery provider.
//!
//! This test only runs when `TEKMERION_SEARCH_API_KEY` is explicitly configured
//! in the environment. If the API key is not present, the test skips cleanly.

use std::time::Duration;
use chrono::Utc;
use tekmerion_core::{FaceAnalysis, FaceDetection, FaceEmbedding};
use tekmerion_discovery::external::{ExternalReverseImageConfig, ExternalReverseImageProvider};
use tekmerion_discovery::provider::DiscoveryProvider;

#[tokio::test]
async fn test_live_external_reverse_image_search_integration() {
    let api_key = match std::env::var("TEKMERION_SEARCH_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => {
            eprintln!(
                "[SKIP] TEKMERION_SEARCH_API_KEY not configured in environment; skipping live integration search test."
            );
            return;
        }
    };

    println!("[INTEGRATION] TEKMERION_SEARCH_API_KEY detected. Running live reverse-image provider test...");

    let config = match ExternalReverseImageConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            panic!("Failed to load ExternalReverseImageConfig from environment: {}", e);
        }
    };

    // Ensure debug display never leaks the api key
    let debug_repr = format!("{:?}", config);
    assert!(
        !debug_repr.contains(&api_key),
        "API key leaked in config debug representation"
    );

    let provider = ExternalReverseImageProvider::new(config)
        .expect("Failed to construct ExternalReverseImageProvider");

    // Write a temporary valid test image
    let temp_dir = std::env::temp_dir();
    let image_file = temp_dir.join(format!(
        "tekmerion_integration_test_{}.jpg",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    // 1x1 JPEG minimal bytes
    let minimal_jpeg = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01,
        0x00, 0x48, 0x00, 0x48, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06,
        0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D,
        0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D,
        0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28,
        0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
        0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01,
        0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02,
        0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x08, 0x01,
        0x01, 0x00, 0x00, 0x3F, 0x00, 0xBF, 0x80, 0xFF, 0xD9,
    ];
    tokio::fs::write(&image_file, &minimal_jpeg)
        .await
        .expect("Failed to write integration test image");

    let analysis = FaceAnalysis {
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
        image_path: Some(image_file.to_str().unwrap().to_string()),
    };

    let result = tokio::time::timeout(Duration::from_secs(30), provider.search(&analysis)).await;

    // Clean up temporary image
    let _ = tokio::fs::remove_file(&image_file).await;

    match result {
        Ok(Ok(candidates)) => {
            println!(
                "[INTEGRATION] Discovery search succeeded with {} genuine candidates.",
                candidates.len()
            );
            for (idx, cand) in candidates.iter().enumerate() {
                assert!(
                    cand.url.starts_with("http://") || cand.url.starts_with("https://"),
                    "Candidate {} URL must be valid HTTP(S): {}",
                    idx,
                    cand.url
                );
            }
        }
        Ok(Err(err)) => {
            let err_msg = err.to_string();
            assert!(
                !err_msg.contains(&api_key),
                "API key leaked in integration test error: {}",
                err_msg
            );
            println!(
                "[INTEGRATION] Provider returned structured error: {}",
                err_msg
            );
        }
        Err(_) => {
            panic!("Live integration test timed out after 30 seconds");
        }
    }
}
