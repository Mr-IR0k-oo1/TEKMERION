//! Tests for face processing functionality

use hh_face::face::{FaceInfo, FaceWorkerRequest, FaceWorkerResponse};
use serde_json::{from_str, json};

#[test]
fn test_face_worker_request_serialization() {
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
    })
    .to_string();

    assert_eq!(serialized, expected);
}

#[test]
fn test_face_worker_response_deserialization() {
    let json_response = r#"{
        "request_id": "123",
        "success": true,
        "face_count": 1,
        "embedding": [0.1, 0.2, 0.3],
        "bbox": [10, 20, 30, 40]
    }"#;

    let response: FaceWorkerResponse = from_str(json_response).unwrap();

    assert_eq!(response.request_id, "123");
    assert!(response.success);
    assert_eq!(response.face_count, Some(1));
    assert_eq!(response.embedding, Some(vec![0.1, 0.2, 0.3]));
    assert_eq!(response.bbox, Some(vec![10, 20, 30, 40]));
}

#[test]
fn test_multiple_faces_response() {
    let json_response = r#"{
        "request_id": "123",
        "success": false,
        "error": "Multiple faces detected",
        "faces": [
            {
                "bbox": [10, 20, 30, 40],
                "embedding": [0.1, 0.2, 0.3]
            },
            {
                "bbox": [50, 60, 70, 80],
                "embedding": [0.4, 0.5, 0.6]
            }
        ]
    }"#;

    let response: FaceWorkerResponse = from_str(json_response).unwrap();

    assert_eq!(response.request_id, "123");
    assert!(!response.success);
    assert_eq!(response.error, Some("Multiple faces detected".to_string()));
    assert_eq!(response.faces.unwrap().len(), 2);
}
