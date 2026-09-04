//! Face-input quality assessment.
//!
//! # Purpose and Invariant
//!
//! The goal of quality assessment is to prevent poor input (e.g. severe blur,
//! extreme head angles, tiny face crops, extreme exposure, or multiple subjects)
//! from producing misleading ArcFace embeddings and false match results.
//!
//! **CRITICAL BIOMETRIC BOUNDARY**:
//! Quality metrics and overall scores evaluate **input image capture suitability
//! and feature extractability only**. They are **NOT** identity probabilities,
//! subject verification confidences, or authenticity claims. A quality score of
//! `0.91` with status `GOOD` indicates that an input image is sufficiently sharp,
//! well-illuminated, and properly posed for reliable feature extraction; it
//! makes no assertion about who the subject is.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::protocol::{WorkerFace, WorkerPose, WorkerResponse};

/// Quality rating classification for face inputs.
///
/// Distinguishes clearly between acceptable input, suboptimal input, and
/// rejected input unsuitable for face analysis and matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QualityStatus {
    /// Input meets or exceeds quality requirements for reliable analysis.
    Good,
    /// Input has flaws (e.g. moderate pose, suboptimal resolution) that may
    /// impair matching accuracy but still allow embedding generation.
    Warning,
    /// Input is unacceptable (e.g. zero faces, severe blur, extreme profile)
    /// and should not be used for verification or enrollment.
    Reject,
}

impl QualityStatus {
    pub fn label(self) -> &'static str {
        match self {
            QualityStatus::Good => "GOOD",
            QualityStatus::Warning => "WARNING",
            QualityStatus::Reject => "REJECT",
        }
    }
}

impl fmt::Display for QualityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Categorical blur assessment level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlurLevel {
    /// Sharp, clear image with high edge definition (low blur).
    Low,
    /// Moderate softness; usable but may lack fine facial details.
    Medium,
    /// Heavy motion or optical blur; unacceptable for reliable embedding.
    High,
}

impl BlurLevel {
    pub fn label(self) -> &'static str {
        match self {
            BlurLevel::Low => "LOW",
            BlurLevel::Medium => "MEDIUM",
            BlurLevel::High => "HIGH",
        }
    }
}

impl fmt::Display for BlurLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Blur estimate including continuous numerical variance and category.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlurEstimate {
    /// Laplacian variance or focus measure (higher is sharper).
    pub variance: f32,
    /// Categorical classification: LOW (sharp), MEDIUM, HIGH (blurry).
    pub level: BlurLevel,
}

/// Categorical illumination / exposure level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExposureLevel {
    /// Severe underexposure; image is excessively dark.
    Underexposed,
    /// Balanced illumination with good dynamic range.
    Normal,
    /// Severe overexposure; facial highlights are blown out.
    Overexposed,
}

impl ExposureLevel {
    pub fn label(self) -> &'static str {
        match self {
            ExposureLevel::Underexposed => "UNDEREXPOSED",
            ExposureLevel::Normal => "NORMAL",
            ExposureLevel::Overexposed => "OVEREXPOSED",
        }
    }
}

impl fmt::Display for ExposureLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Brightness and exposure assessment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExposureEstimate {
    /// Mean luminance in `0.0..=255.0`.
    pub brightness: f32,
    /// Categorical exposure rating.
    pub level: ExposureLevel,
}

/// Head pose orientation angles in degrees.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PoseEstimate {
    /// Pitch angle in degrees (nodding up/down; positive is up).
    pub pitch: f32,
    /// Yaw angle in degrees (turning left/right; positive is right).
    pub yaw: f32,
    /// Roll angle in degrees (tilt shoulder-to-shoulder; positive is clockwise).
    pub roll: f32,
    /// Whether the face is within frontal tolerances.
    pub is_frontal: bool,
}

impl PoseEstimate {
    /// Format pose in a human-readable string suitable for display.
    pub fn display_summary(&self) -> String {
        format!(
            "Pitch: {:.1}°, Yaw: {:.1}°, Roll: {:.1}°",
            self.pitch, self.yaw, self.roll
        )
    }
}

/// Occlusion and boundary truncation indicators.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OcclusionIndicators {
    /// Whether the face bounding box touches or exceeds the image boundary.
    pub border_truncated: bool,
    /// Whether expected facial landmarks (eyes, nose, mouth) are missing or occluded.
    pub landmarks_missing: bool,
    /// Estimated occlusion factor in `0.0..=1.0` (0.0 = clear, 1.0 = fully occluded).
    pub occlusion_score: f32,
}

impl Default for OcclusionIndicators {
    fn default() -> Self {
        Self {
            border_truncated: false,
            landmarks_missing: false,
            occlusion_score: 0.0,
        }
    }
}

/// Face bounding box in image pixel coordinates: `[x1, y1, x2, y2]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FaceBoundingBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl FaceBoundingBox {
    pub fn from_array(b: [f32; 4]) -> Self {
        Self {
            x1: b[0].min(b[2]),
            y1: b[1].min(b[3]),
            x2: b[0].max(b[2]),
            y2: b[1].max(b[3]),
        }
    }

    pub fn width(&self) -> f32 {
        (self.x2 - self.x1).max(0.0)
    }

    pub fn height(&self) -> f32 {
        (self.y2 - self.y1).max(0.0)
    }

    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    pub fn min_dimension(&self) -> f32 {
        self.width().min(self.height())
    }
}

/// Explicit quality assessment thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityThresholds {
    /// Minimum face bounding-box dimension (width/height) below which input is REJECTED.
    pub min_face_dim_reject: f32,
    /// Minimum face bounding-box dimension below which input receives a WARNING.
    pub min_face_dim_warning: f32,

    /// Minimum image resolution width below which input is REJECTED.
    pub min_res_width_reject: u32,
    /// Minimum image resolution height below which input is REJECTED.
    pub min_res_height_reject: u32,
    /// Minimum image resolution width below which input receives a WARNING.
    pub min_res_width_warning: u32,
    /// Minimum image resolution height below which input receives a WARNING.
    pub min_res_height_warning: u32,

    /// Laplacian blur variance below which image is considered HIGH blur (REJECT).
    pub blur_variance_reject: f32,
    /// Laplacian blur variance below which image is considered MEDIUM blur (WARNING).
    pub blur_variance_warning: f32,

    /// Minimum mean luminance for exposure (below is REJECT / severe underexposure).
    pub brightness_min_reject: f32,
    /// Minimum mean luminance for exposure (below is WARNING / underexposed).
    pub brightness_min_warning: f32,
    /// Maximum mean luminance for exposure (above is WARNING / overexposed).
    pub brightness_max_warning: f32,
    /// Maximum mean luminance for exposure (above is REJECT / severe overexposure).
    pub brightness_max_reject: f32,

    /// Maximum absolute yaw angle in degrees (above is REJECT).
    pub max_yaw_reject_deg: f32,
    /// Maximum absolute yaw angle in degrees (above is WARNING).
    pub max_yaw_warning_deg: f32,

    /// Maximum absolute pitch angle in degrees (above is REJECT).
    pub max_pitch_reject_deg: f32,
    /// Maximum absolute pitch angle in degrees (above is WARNING).
    pub max_pitch_warning_deg: f32,

    /// Maximum absolute roll angle in degrees (above is REJECT).
    pub max_roll_reject_deg: f32,
    /// Maximum absolute roll angle in degrees (above is WARNING).
    pub max_roll_warning_deg: f32,

    /// Overall composite quality score threshold for GOOD status.
    pub score_good_threshold: f32,
    /// Overall composite quality score threshold for WARNING status.
    pub score_warning_threshold: f32,
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            min_face_dim_reject: 60.0,
            min_face_dim_warning: 100.0,

            min_res_width_reject: 160,
            min_res_height_reject: 160,
            min_res_width_warning: 320,
            min_res_height_warning: 320,

            blur_variance_reject: 30.0,
            blur_variance_warning: 80.0,

            brightness_min_reject: 30.0,
            brightness_min_warning: 50.0,
            brightness_max_warning: 210.0,
            brightness_max_reject: 235.0,

            max_yaw_reject_deg: 45.0,
            max_yaw_warning_deg: 25.0,

            max_pitch_reject_deg: 35.0,
            max_pitch_warning_deg: 20.0,

            max_roll_reject_deg: 35.0,
            max_roll_warning_deg: 20.0,

            score_good_threshold: 0.75,
            score_warning_threshold: 0.50,
        }
    }
}

/// Comprehensive face-input quality assessment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceQualityAssessment {
    /// Number of detected faces in the input image.
    pub face_count: usize,
    /// Bounding box size `(width, height)` of the primary face in pixels.
    pub bounding_box_size: Option<(f32, f32)>,
    /// Image resolution `(width, height)` in pixels.
    pub image_resolution: Option<(u32, u32)>,
    /// Blur estimate (numerical variance and category).
    pub blur: BlurEstimate,
    /// Brightness/exposure estimate (mean luminance and category).
    pub exposure: ExposureEstimate,
    /// Head pose estimate (pitch, yaw, roll where supported).
    pub pose: Option<PoseEstimate>,
    /// Occlusion indicators (boundary truncation and landmark completeness).
    pub occlusion: OcclusionIndicators,
    /// Overall composite quality score in `0.0..=1.0`.
    pub overall_quality: f32,
    /// Categorical assessment status: GOOD, WARNING, or REJECT.
    pub status: QualityStatus,
    /// Human-readable explanations for any degradation or rejection.
    pub reasons: Vec<String>,
}

impl FaceQualityAssessment {
    /// Default realistic high-quality assessment for testing and UI display.
    pub fn sample_good() -> Self {
        Self {
            face_count: 1,
            bounding_box_size: Some((240.0, 280.0)),
            image_resolution: Some((1920, 1080)),
            blur: BlurEstimate {
                variance: 145.0,
                level: BlurLevel::Low,
            },
            exposure: ExposureEstimate {
                brightness: 128.0,
                level: ExposureLevel::Normal,
            },
            pose: Some(PoseEstimate {
                pitch: 0.1,
                yaw: 0.2,
                roll: 0.3,
                is_frontal: true,
            }),
            occlusion: OcclusionIndicators::default(),
            overall_quality: 0.91,
            status: QualityStatus::Good,
            reasons: Vec::new(),
        }
    }

    /// Formatted face count string for UI display.
    pub fn faces_display(&self) -> String {
        self.face_count.to_string()
    }

    /// Formatted resolution string for UI display.
    pub fn resolution_display(&self) -> String {
        match self.image_resolution {
            Some((w, h)) => format!("{}x{}", w, h),
            None => "--".to_string(),
        }
    }

    /// Formatted blur string for UI display.
    pub fn blur_display(&self) -> &'static str {
        self.blur.level.label()
    }

    /// Formatted pose string for UI display.
    pub fn pose_display(&self) -> String {
        match &self.pose {
            Some(p) => p.display_summary(),
            None => "Not supported".to_string(),
        }
    }

    /// Formatted overall quality score string for UI display.
    pub fn quality_display(&self) -> String {
        format!("{:.2}", self.overall_quality)
    }

    /// Formatted status label for UI display.
    pub fn status_display(&self) -> &'static str {
        self.status.label()
    }
}

/// Numerical input payload used to evaluate face-input quality.
#[derive(Debug, Clone)]
pub struct QualityInput {
    pub face_count: usize,
    pub bounding_boxes: Vec<[f32; 4]>,
    pub detector_confidences: Vec<f32>,
    pub poses: Vec<Option<WorkerPose>>,
    pub landmarks: Vec<Option<Vec<[f32; 2]>>>,
    pub image_resolution: Option<(u32, u32)>,
    pub blur_variance: Option<f32>,
    pub brightness: Option<f32>,
}

impl QualityInput {
    pub fn from_worker_response(
        response: &WorkerResponse,
        image_resolution: Option<(u32, u32)>,
        blur_variance: Option<f32>,
        brightness: Option<f32>,
    ) -> Self {
        let face_count = response.faces.len();
        let mut bounding_boxes = Vec::with_capacity(face_count);
        let mut detector_confidences = Vec::with_capacity(face_count);
        let mut poses = Vec::with_capacity(face_count);
        let mut landmarks = Vec::with_capacity(face_count);

        for face in &response.faces {
            bounding_boxes.push(face.bounding_box);
            detector_confidences.push(face.quality);
            poses.push(face.pose.clone());
            landmarks.push(face.landmarks.clone());
        }

        Self {
            face_count,
            bounding_boxes,
            detector_confidences,
            poses,
            landmarks,
            image_resolution,
            blur_variance,
            brightness,
        }
    }
}

/// Calculate the Laplacian variance on a 2D grayscale pixel buffer.
///
/// This provides a fast, deterministic blur metric:
/// - Higher values correspond to sharp edges (in-focus).
/// - Lower values correspond to smooth/blurred edges (out-of-focus).
pub fn calculate_blur_variance(pixels: &[u8], width: usize, height: usize) -> f32 {
    if width < 3 || height < 3 || pixels.len() < width * height {
        return 0.0;
    }

    let mut sum_laplacian = 0.0f64;
    let mut sum_sq_laplacian = 0.0f64;
    let mut count = 0usize;

    for y in 1..(height - 1) {
        let row_curr = y * width;
        let row_prev = (y - 1) * width;
        let row_next = (y + 1) * width;

        for x in 1..(width - 1) {
            let center = pixels[row_curr + x] as f64;
            let top = pixels[row_prev + x] as f64;
            let bottom = pixels[row_next + x] as f64;
            let left = pixels[row_curr + x - 1] as f64;
            let right = pixels[row_curr + x + 1] as f64;

            // Discrete 4-neighborhood Laplacian: L = top + bottom + left + right - 4 * center
            let lap = top + bottom + left + right - 4.0 * center;
            sum_laplacian += lap;
            sum_sq_laplacian += lap * lap;
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }

    let mean = sum_laplacian / (count as f64);
    let variance = (sum_sq_laplacian / (count as f64)) - (mean * mean);
    variance.max(0.0) as f32
}

/// Calculate the mean luminance across a grayscale pixel buffer in `0.0..=255.0`.
pub fn calculate_brightness(pixels: &[u8]) -> f32 {
    if pixels.is_empty() {
        return 0.0;
    }
    let sum: u64 = pixels.iter().map(|&p| p as u64).sum();
    (sum as f64 / pixels.len() as f64) as f32
}

/// Assess face input quality against clear thresholds.
///
/// Evaluates:
/// - Face count (reject if 0, warn if > 1)
/// - Face bounding-box size
/// - Image resolution
/// - Blur estimate
/// - Brightness / exposure estimate
/// - Pose (yaw, pitch, roll angles)
/// - Occlusion & image border truncation
/// - Overall composite quality score
pub fn assess_face_quality(
    input: &QualityInput,
    thresholds: &QualityThresholds,
) -> FaceQualityAssessment {
    let mut reasons = Vec::new();
    let mut has_reject = false;
    let mut has_warning = false;

    // 1. Face count check
    let face_count = input.face_count;
    if face_count == 0 {
        has_reject = true;
        reasons.push("No face detected in input image".to_string());
    } else if face_count > 1 {
        has_warning = true;
        reasons.push(format!(
            "Multiple faces ({}) detected; single face required for unambiguous verification",
            face_count
        ));
    }

    // 2. Primary face bounding box
    let primary_bbox = input.bounding_boxes.first().map(|b| FaceBoundingBox::from_array(*b));
    let bbox_size = primary_bbox.map(|b| (b.width(), b.height()));

    let mut size_score = 0.0f32;
    if let Some(bbox) = primary_bbox {
        let min_dim = bbox.min_dimension();
        if min_dim < thresholds.min_face_dim_reject {
            has_reject = true;
            reasons.push(format!(
                "Face bounding box too small ({:.0}px < {:.0}px reject threshold)",
                min_dim, thresholds.min_face_dim_reject
            ));
            size_score = (min_dim / thresholds.min_face_dim_reject) * 0.3;
        } else if min_dim < thresholds.min_face_dim_warning {
            has_warning = true;
            reasons.push(format!(
                "Face bounding box is small ({:.0}px < {:.0}px warning threshold)",
                min_dim, thresholds.min_face_dim_warning
            ));
            size_score = 0.3 + ((min_dim - thresholds.min_face_dim_reject)
                / (thresholds.min_face_dim_warning - thresholds.min_face_dim_reject))
                * 0.4;
        } else {
            size_score = (0.7 + (min_dim - thresholds.min_face_dim_warning) / 100.0 * 0.3).min(1.0);
        }
    }

    // 3. Image resolution
    let mut res_score = 1.0f32;
    if let Some((w, h)) = input.image_resolution {
        if w < thresholds.min_res_width_reject || h < thresholds.min_res_height_reject {
            has_reject = true;
            reasons.push(format!(
                "Image resolution {}x{} is below minimum threshold {}x{}",
                w, h, thresholds.min_res_width_reject, thresholds.min_res_height_reject
            ));
            res_score = 0.2;
        } else if w < thresholds.min_res_width_warning || h < thresholds.min_res_height_warning {
            has_warning = true;
            reasons.push(format!(
                "Image resolution {}x{} is below recommended threshold {}x{}",
                w, h, thresholds.min_res_width_warning, thresholds.min_res_height_warning
            ));
            res_score = 0.6;
        } else {
            res_score = 1.0;
        }
    }

    // 4. Blur estimate
    let blur_variance = input.blur_variance.unwrap_or(120.0);
    let blur_level = if blur_variance < thresholds.blur_variance_reject {
        has_reject = true;
        reasons.push(format!(
            "Severe image blur detected (variance {:.1} < {:.1})",
            blur_variance, thresholds.blur_variance_reject
        ));
        BlurLevel::High
    } else if blur_variance < thresholds.blur_variance_warning {
        has_warning = true;
        reasons.push(format!(
            "Moderate image blur detected (variance {:.1} < {:.1})",
            blur_variance, thresholds.blur_variance_warning
        ));
        BlurLevel::Medium
    } else {
        BlurLevel::Low
    };
    let blur_estimate = BlurEstimate {
        variance: blur_variance,
        level: blur_level,
    };
    let blur_score = match blur_level {
        BlurLevel::Low => (0.8 + (blur_variance / 200.0) * 0.2).min(1.0),
        BlurLevel::Medium => 0.55,
        BlurLevel::High => 0.2,
    };

    // 5. Exposure / brightness
    let brightness = input.brightness.unwrap_or(128.0);
    let exposure_level = if brightness < thresholds.brightness_min_reject {
        has_reject = true;
        reasons.push(format!(
            "Severe underexposure; image is too dark (brightness {:.1} < {:.1})",
            brightness, thresholds.brightness_min_reject
        ));
        ExposureLevel::Underexposed
    } else if brightness > thresholds.brightness_max_reject {
        has_reject = true;
        reasons.push(format!(
            "Severe overexposure; image highlights blown out (brightness {:.1} > {:.1})",
            brightness, thresholds.brightness_max_reject
        ));
        ExposureLevel::Overexposed
    } else if brightness < thresholds.brightness_min_warning {
        has_warning = true;
        reasons.push(format!(
            "Suboptimal underexposure (brightness {:.1} < {:.1})",
            brightness, thresholds.brightness_min_warning
        ));
        ExposureLevel::Underexposed
    } else if brightness > thresholds.brightness_max_warning {
        has_warning = true;
        reasons.push(format!(
            "Suboptimal overexposure (brightness {:.1} > {:.1})",
            brightness, thresholds.brightness_max_warning
        ));
        ExposureLevel::Overexposed
    } else {
        ExposureLevel::Normal
    };
    let exposure_estimate = ExposureEstimate {
        brightness,
        level: exposure_level,
    };
    let exposure_score = match exposure_level {
        ExposureLevel::Normal => 1.0,
        ExposureLevel::Underexposed | ExposureLevel::Overexposed => {
            if has_reject {
                0.2
            } else {
                0.6
            }
        }
    };

    // 6. Pose assessment
    let primary_pose_worker = input.poses.first().and_then(|p| p.clone());
    let mut pose_score = 1.0f32;
    let pose_estimate = primary_pose_worker.map(|wp| {
        let abs_yaw = wp.yaw.abs();
        let abs_pitch = wp.pitch.abs();
        let abs_roll = wp.roll.abs();

        let is_frontal = abs_yaw <= thresholds.max_yaw_warning_deg
            && abs_pitch <= thresholds.max_pitch_warning_deg
            && abs_roll <= thresholds.max_roll_warning_deg;

        if abs_yaw > thresholds.max_yaw_reject_deg {
            has_reject = true;
            reasons.push(format!(
                "Extreme yaw angle {:.1}° (profile face exceeds {:.1}° threshold)",
                abs_yaw, thresholds.max_yaw_reject_deg
            ));
            pose_score = 0.2;
        } else if abs_pitch > thresholds.max_pitch_reject_deg {
            has_reject = true;
            reasons.push(format!(
                "Extreme pitch angle {:.1}° exceeds {:.1}° threshold",
                abs_pitch, thresholds.max_pitch_reject_deg
            ));
            pose_score = 0.2;
        } else if abs_roll > thresholds.max_roll_reject_deg {
            has_reject = true;
            reasons.push(format!(
                "Extreme roll angle {:.1}° exceeds {:.1}° threshold",
                abs_roll, thresholds.max_roll_reject_deg
            ));
            pose_score = 0.2;
        } else if abs_yaw > thresholds.max_yaw_warning_deg
            || abs_pitch > thresholds.max_pitch_warning_deg
            || abs_roll > thresholds.max_roll_warning_deg
        {
            has_warning = true;
            reasons.push(format!(
                "Non-frontal head pose (yaw: {:.1}°, pitch: {:.1}°, roll: {:.1}°)",
                abs_yaw, abs_pitch, abs_roll
            ));
            pose_score = 0.65;
        } else {
            pose_score = 1.0;
        }

        PoseEstimate {
            pitch: wp.pitch,
            yaw: wp.yaw,
            roll: wp.roll,
            is_frontal,
        }
    });

    // 7. Occlusion and border truncation
    let mut border_truncated = false;
    let mut landmarks_missing = false;
    let mut occlusion_factor = 0.0f32;

    if let (Some(bbox), Some((img_w, img_h))) = (primary_bbox, input.image_resolution) {
        // Truncation check: within 2 pixels of image edge
        if bbox.x1 <= 2.0
            || bbox.y1 <= 2.0
            || bbox.x2 >= (img_w as f32 - 2.0)
            || bbox.y2 >= (img_h as f32 - 2.0)
        {
            border_truncated = true;
            occlusion_factor += 0.25;
            has_warning = true;
            reasons.push("Face bounding box is truncated at image border".to_string());
        }
    }

    if let Some(Some(lms)) = input.landmarks.first() {
        if lms.len() < 5 {
            landmarks_missing = true;
            occlusion_factor += 0.35;
            has_warning = true;
            reasons.push("Missing facial landmarks indicates occlusion".to_string());
        }
    }

    let occlusion = OcclusionIndicators {
        border_truncated,
        landmarks_missing,
        occlusion_score: occlusion_factor.min(1.0),
    };

    // 8. Overall composite quality score calculation
    let detector_quality = input.detector_confidences.first().copied().unwrap_or(0.9);

    let composite_score = if face_count == 0 {
        0.0f32
    } else {
        let weighted = (size_score * 0.20)
            + (res_score * 0.15)
            + (blur_score * 0.25)
            + (exposure_score * 0.15)
            + (pose_score * 0.15)
            + (detector_quality * 0.10);
        let penalized = (weighted - occlusion_factor * 0.2).clamp(0.0, 1.0);

        if has_reject {
            // Guarantee reject score falls below reject threshold
            penalized.min(thresholds.score_warning_threshold - 0.01)
        } else if has_warning {
            penalized
                .clamp(thresholds.score_warning_threshold, thresholds.score_good_threshold - 0.01)
        } else {
            penalized.max(thresholds.score_good_threshold)
        }
    };

    // Determine final status
    let status = if has_reject || composite_score < thresholds.score_warning_threshold {
        QualityStatus::Reject
    } else if has_warning || composite_score < thresholds.score_good_threshold {
        QualityStatus::Warning
    } else {
        QualityStatus::Good
    };

    FaceQualityAssessment {
        face_count,
        bounding_box_size: bbox_size,
        image_resolution: input.image_resolution,
        blur: blur_estimate,
        exposure: exposure_estimate,
        pose: pose_estimate,
        occlusion,
        overall_quality: composite_score,
        status,
        reasons,
    }
}

/// Convenience helper to assess a single `WorkerFace` with optional resolution and image statistics.
pub fn assess_face(
    face: &WorkerFace,
    image_resolution: Option<(u32, u32)>,
    blur_variance: Option<f32>,
    brightness: Option<f32>,
    thresholds: Option<&QualityThresholds>,
) -> FaceQualityAssessment {
    let input = QualityInput {
        face_count: 1,
        bounding_boxes: vec![face.bounding_box],
        detector_confidences: vec![face.quality],
        poses: vec![face.pose.clone()],
        landmarks: vec![face.landmarks.clone()],
        image_resolution,
        blur_variance,
        brightness,
    };
    let default_thresholds = QualityThresholds::default();
    assess_face_quality(&input, thresholds.unwrap_or(&default_thresholds))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn optimal_thresholds() -> QualityThresholds {
        QualityThresholds::default()
    }

    #[test]
    fn quality_status_labels_are_exact() {
        assert_eq!(QualityStatus::Good.label(), "GOOD");
        assert_eq!(QualityStatus::Warning.label(), "WARNING");
        assert_eq!(QualityStatus::Reject.label(), "REJECT");
    }

    #[test]
    fn blur_level_labels_are_exact() {
        assert_eq!(BlurLevel::Low.label(), "LOW");
        assert_eq!(BlurLevel::Medium.label(), "MEDIUM");
        assert_eq!(BlurLevel::High.label(), "HIGH");
    }

    #[test]
    fn zero_faces_is_rejected() {
        let input = QualityInput {
            face_count: 0,
            bounding_boxes: vec![],
            detector_confidences: vec![],
            poses: vec![],
            landmarks: vec![],
            image_resolution: Some((1920, 1080)),
            blur_variance: Some(150.0),
            brightness: Some(128.0),
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert_eq!(result.status, QualityStatus::Reject);
        assert_eq!(result.overall_quality, 0.0);
        assert!(result.reasons.iter().any(|r| r.contains("No face detected")));
    }

    #[test]
    fn multiple_faces_produces_warning() {
        let input = QualityInput {
            face_count: 2,
            bounding_boxes: vec![
                [50.0, 50.0, 250.0, 300.0],
                [350.0, 50.0, 550.0, 300.0],
            ],
            detector_confidences: vec![0.95, 0.92],
            poses: vec![
                Some(WorkerPose { pitch: 0.0, yaw: 0.0, roll: 0.0 }),
                Some(WorkerPose { pitch: 0.0, yaw: 0.0, roll: 0.0 }),
            ],
            landmarks: vec![None, None],
            image_resolution: Some((1920, 1080)),
            blur_variance: Some(150.0),
            brightness: Some(128.0),
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert_eq!(result.status, QualityStatus::Warning);
        assert!(result.overall_quality >= 0.50 && result.overall_quality < 0.75);
        assert!(result.reasons.iter().any(|r| r.contains("Multiple faces")));
    }

    #[test]
    fn tiny_bounding_box_is_rejected() {
        let input = QualityInput {
            face_count: 1,
            bounding_boxes: vec![[10.0, 10.0, 40.0, 45.0]], // 30x35 px, < 60px reject threshold
            detector_confidences: vec![0.85],
            poses: vec![Some(WorkerPose { pitch: 0.0, yaw: 0.0, roll: 0.0 })],
            landmarks: vec![None],
            image_resolution: Some((1920, 1080)),
            blur_variance: Some(150.0),
            brightness: Some(128.0),
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert_eq!(result.status, QualityStatus::Reject);
        assert!(result.reasons.iter().any(|r| r.contains("bounding box too small")));
    }

    #[test]
    fn tiny_resolution_is_rejected() {
        let input = QualityInput {
            face_count: 1,
            bounding_boxes: vec![[10.0, 10.0, 110.0, 120.0]],
            detector_confidences: vec![0.9],
            poses: vec![Some(WorkerPose { pitch: 0.0, yaw: 0.0, roll: 0.0 })],
            landmarks: vec![None],
            image_resolution: Some((120, 120)), // < 160x160 reject threshold
            blur_variance: Some(150.0),
            brightness: Some(128.0),
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert_eq!(result.status, QualityStatus::Reject);
        assert!(result.reasons.iter().any(|r| r.contains("resolution")));
    }

    #[test]
    fn severe_blur_is_rejected() {
        let input = QualityInput {
            face_count: 1,
            bounding_boxes: vec![[50.0, 50.0, 250.0, 300.0]],
            detector_confidences: vec![0.9],
            poses: vec![Some(WorkerPose { pitch: 0.0, yaw: 0.0, roll: 0.0 })],
            landmarks: vec![None],
            image_resolution: Some((1920, 1080)),
            blur_variance: Some(15.0), // < 30.0 reject threshold
            brightness: Some(128.0),
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert_eq!(result.status, QualityStatus::Reject);
        assert_eq!(result.blur.level, BlurLevel::High);
        assert!(result.reasons.iter().any(|r| r.contains("blur")));
    }

    #[test]
    fn severe_darkness_underexposure_is_rejected() {
        let input = QualityInput {
            face_count: 1,
            bounding_boxes: vec![[50.0, 50.0, 250.0, 300.0]],
            detector_confidences: vec![0.9],
            poses: vec![Some(WorkerPose { pitch: 0.0, yaw: 0.0, roll: 0.0 })],
            landmarks: vec![None],
            image_resolution: Some((1920, 1080)),
            blur_variance: Some(150.0),
            brightness: Some(15.0), // < 30.0 reject threshold
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert_eq!(result.status, QualityStatus::Reject);
        assert_eq!(result.exposure.level, ExposureLevel::Underexposed);
        assert!(result.reasons.iter().any(|r| r.contains("underexposure")));
    }

    #[test]
    fn severe_overexposure_is_rejected() {
        let input = QualityInput {
            face_count: 1,
            bounding_boxes: vec![[50.0, 50.0, 250.0, 300.0]],
            detector_confidences: vec![0.9],
            poses: vec![Some(WorkerPose { pitch: 0.0, yaw: 0.0, roll: 0.0 })],
            landmarks: vec![None],
            image_resolution: Some((1920, 1080)),
            blur_variance: Some(150.0),
            brightness: Some(248.0), // > 235.0 reject threshold
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert_eq!(result.status, QualityStatus::Reject);
        assert_eq!(result.exposure.level, ExposureLevel::Overexposed);
        assert!(result.reasons.iter().any(|r| r.contains("overexposure")));
    }

    #[test]
    fn extreme_pose_yaw_is_rejected() {
        let input = QualityInput {
            face_count: 1,
            bounding_boxes: vec![[50.0, 50.0, 250.0, 300.0]],
            detector_confidences: vec![0.9],
            poses: vec![Some(WorkerPose { pitch: 2.0, yaw: 65.0, roll: 1.0 })], // > 45 deg yaw
            landmarks: vec![None],
            image_resolution: Some((1920, 1080)),
            blur_variance: Some(150.0),
            brightness: Some(128.0),
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert_eq!(result.status, QualityStatus::Reject);
        assert!(result.reasons.iter().any(|r| r.contains("yaw")));
    }

    #[test]
    fn moderate_pose_yaw_triggers_warning() {
        let input = QualityInput {
            face_count: 1,
            bounding_boxes: vec![[50.0, 50.0, 250.0, 300.0]],
            detector_confidences: vec![0.9],
            poses: vec![Some(WorkerPose { pitch: 2.0, yaw: 30.0, roll: 1.0 })], // > 25 deg warning
            landmarks: vec![None],
            image_resolution: Some((1920, 1080)),
            blur_variance: Some(150.0),
            brightness: Some(128.0),
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert_eq!(result.status, QualityStatus::Warning);
        assert!(result.reasons.iter().any(|r| r.contains("Non-frontal")));
    }

    #[test]
    fn border_truncation_triggers_occlusion_warning() {
        let input = QualityInput {
            face_count: 1,
            bounding_boxes: vec![[0.0, 50.0, 200.0, 300.0]], // x1 = 0 touches left border
            detector_confidences: vec![0.9],
            poses: vec![Some(WorkerPose { pitch: 0.0, yaw: 0.0, roll: 0.0 })],
            landmarks: vec![None],
            image_resolution: Some((1920, 1080)),
            blur_variance: Some(150.0),
            brightness: Some(128.0),
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert!(result.occlusion.border_truncated);
        assert_eq!(result.status, QualityStatus::Warning);
        assert!(result.reasons.iter().any(|r| r.contains("truncated")));
    }

    #[test]
    fn optimal_synthetic_face_produces_good_status() {
        let input = QualityInput {
            face_count: 1,
            bounding_boxes: vec![[100.0, 100.0, 340.0, 380.0]], // 240x280 px
            detector_confidences: vec![0.95],
            poses: vec![Some(WorkerPose { pitch: 0.1, yaw: 0.2, roll: 0.3 })],
            landmarks: vec![Some(vec![
                [160.0, 180.0],
                [280.0, 180.0],
                [220.0, 240.0],
                [180.0, 310.0],
                [260.0, 310.0],
            ])],
            image_resolution: Some((1920, 1080)),
            blur_variance: Some(160.0),
            brightness: Some(128.0),
        };

        let result = assess_face_quality(&input, &optimal_thresholds());
        assert_eq!(result.status, QualityStatus::Good);
        assert_eq!(result.blur.level, BlurLevel::Low);
        assert_eq!(result.exposure.level, ExposureLevel::Normal);
        assert!(result.pose.as_ref().unwrap().is_frontal);
        assert!(!result.occlusion.border_truncated);
        assert!(!result.occlusion.landmarks_missing);
        assert!(result.overall_quality >= 0.75);
    }

    #[test]
    fn synthetic_blur_variance_on_flat_vs_checkerboard_pixels() {
        // Uniform/flat 10x10 image -> zero variance (extreme blur)
        let flat_pixels = vec![128u8; 100];
        let flat_variance = calculate_blur_variance(&flat_pixels, 10, 10);
        assert_eq!(flat_variance, 0.0);

        // High contrast alternating pattern -> high variance (sharp edges)
        let mut sharp_pixels = vec![0u8; 100];
        for (i, p) in sharp_pixels.iter_mut().enumerate() {
            if i % 2 == 0 {
                *p = 255;
            }
        }
        let sharp_variance = calculate_blur_variance(&sharp_pixels, 10, 10);
        assert!(sharp_variance > 1000.0);
    }

    #[test]
    fn synthetic_brightness_calculation() {
        let dark = vec![20u8; 50];
        assert_eq!(calculate_brightness(&dark), 20.0);

        let mid = vec![128u8; 50];
        assert_eq!(calculate_brightness(&mid), 128.0);

        let bright = vec![250u8; 50];
        assert_eq!(calculate_brightness(&bright), 250.0);
    }

    #[test]
    fn sample_good_matches_tui_requirements() {
        let sample = FaceQualityAssessment::sample_good();
        assert_eq!(sample.faces_display(), "1");
        assert_eq!(sample.resolution_display(), "1920x1080");
        assert_eq!(sample.blur_display(), "LOW");
        assert_eq!(sample.quality_display(), "0.91");
        assert_eq!(sample.status_display(), "GOOD");
        assert!(sample.pose_display().contains("Pitch: 0.1°"));
    }
}
