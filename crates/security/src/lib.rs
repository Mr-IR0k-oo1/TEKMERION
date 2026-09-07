use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Input validation error codes strictly matching Stage 1 specifications.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum InputValidationError {
    #[error("INVALID_FILE: {0}")]
    InvalidFile(String),

    #[error("UNSUPPORTED_FORMAT: {0}")]
    UnsupportedFormat(String),

    #[error("FILE_TOO_LARGE: File size {size_bytes} exceeds limit of {max_bytes} bytes")]
    FileTooLarge { size_bytes: usize, max_bytes: usize },

    #[error("IMAGE_DECODE_FAILURE: {0}")]
    ImageDecodeFailure(String),

    #[error("IMAGE_TOO_SMALL: Image dimensions ({width}x{height}) are smaller than minimum allowed ({min_width}x{min_height})")]
    ImageTooSmall {
        width: u32,
        height: u32,
        min_width: u32,
        min_height: u32,
    },
}

impl InputValidationError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidFile(_) => "INVALID_FILE",
            Self::UnsupportedFormat(_) => "UNSUPPORTED_FORMAT",
            Self::FileTooLarge { .. } => "FILE_TOO_LARGE",
            Self::ImageDecodeFailure(_) => "IMAGE_DECODE_FAILURE",
            Self::ImageTooSmall { .. } => "IMAGE_TOO_SMALL",
        }
    }
}

/// A cryptographically verified and bounded image input.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ValidatedInput {
    pub path: PathBuf,
    pub filename: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: usize,
    pub mime_type: String,
}

impl ValidatedInput {
    pub fn resolution_label(&self) -> String {
        format!(
            "{}x{} ({:.1} KB)",
            self.width,
            self.height,
            (self.size_bytes as f64) / 1024.0
        )
    }
}

/// Security boundary validator for Stage 1.
pub struct InputValidator {
    pub max_file_bytes: usize,
    pub min_dimension: u32,
    pub max_dimension: u32,
}

impl Default for InputValidator {
    fn default() -> Self {
        Self {
            max_file_bytes: 10 * 1024 * 1024, // 10 MB limit
            min_dimension: 64,                // 64x64 minimum resolution
            max_dimension: 4096,              // 4096x4096 maximum resolution
        }
    }
}

impl InputValidator {
    pub fn new(max_file_bytes: usize) -> Self {
        Self {
            max_file_bytes,
            ..Default::default()
        }
    }

    /// Validate an input image path and compute its initial cryptographic evidence identity.
    pub fn validate<P: AsRef<Path>>(&self, path: P) -> Result<ValidatedInput, InputValidationError> {
        let p = path.as_ref();

        // 1. Exists and is readable file
        if !p.exists() {
            return Err(InputValidationError::InvalidFile(format!(
                "Path does not exist: {}",
                p.display()
            )));
        }
        if !p.is_file() {
            return Err(InputValidationError::InvalidFile(format!(
                "Path is not a regular file: {}",
                p.display()
            )));
        }

        // 2. Allowed extension
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        if ext != "jpg" && ext != "jpeg" && ext != "png" {
            return Err(InputValidationError::UnsupportedFormat(format!(
                "Extension '.{}' not supported. Only JPEG (.jpg, .jpeg) and PNG (.png) are allowed",
                ext
            )));
        }

        // 3. Read bytes & check size limit
        let bytes = std::fs::read(p).map_err(|e| {
            InputValidationError::InvalidFile(format!("Failed to read file: {e}"))
        })?;

        if bytes.is_empty() {
            return Err(InputValidationError::InvalidFile("File is empty (0 bytes)".to_string()));
        }

        if bytes.len() > self.max_file_bytes {
            return Err(InputValidationError::FileTooLarge {
                size_bytes: bytes.len(),
                max_bytes: self.max_file_bytes,
            });
        }

        // 4. Inspect magic bytes & MIME type
        let (mime, (width, height)) = self.inspect_and_decode(&bytes, &ext)?;

        // 5. Check resolution boundaries
        if width < self.min_dimension || height < self.min_dimension {
            return Err(InputValidationError::ImageTooSmall {
                width,
                height,
                min_width: self.min_dimension,
                min_height: self.min_dimension,
            });
        }

        if width > self.max_dimension || height > self.max_dimension {
            return Err(InputValidationError::UnsupportedFormat(format!(
                "Image dimensions ({width}x{height}) exceed maximum allowed dimension of {max}px",
                max = self.max_dimension
            )));
        }

        // 6. Compute SHA-256 digest
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha256_hex = hex::encode(hasher.finalize());

        let filename = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("input.jpg")
            .to_string();

        Ok(ValidatedInput {
            path: p.to_path_buf(),
            filename,
            sha256: sha256_hex,
            width,
            height,
            size_bytes: bytes.len(),
            mime_type: mime.to_string(),
        })
    }

    /// Inspect magic bytes and decode image dimensions without spawning external tools.
    fn inspect_and_decode(
        &self,
        bytes: &[u8],
        ext: &str,
    ) -> Result<(&'static str, (u32, u32)), InputValidationError> {
        // PNG magic: 89 50 4E 47 0D 0A 1A 0A
        if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
            if ext != "png" {
                return Err(InputValidationError::UnsupportedFormat(
                    "PNG image magic bytes detected but extension is not .png".to_string(),
                ));
            }
            let dims = Self::decode_png_dimensions(bytes).ok_or_else(|| {
                InputValidationError::ImageDecodeFailure(
                    "Corrupt or incomplete PNG IHDR chunk".to_string(),
                )
            })?;
            return Ok(("image/png", dims));
        }

        // JPEG magic: FF D8 FF
        if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
            if ext != "jpg" && ext != "jpeg" {
                return Err(InputValidationError::UnsupportedFormat(
                    "JPEG magic bytes detected but extension is not .jpg/.jpeg".to_string(),
                ));
            }
            let dims = Self::decode_jpeg_dimensions(bytes).ok_or_else(|| {
                InputValidationError::ImageDecodeFailure(
                    "Failed to decode JPEG frame header".to_string(),
                )
            })?;
            return Ok(("image/jpeg", dims));
        }

        Err(InputValidationError::ImageDecodeFailure(
            "File header does not match valid JPEG or PNG magic bytes".to_string(),
        ))
    }

    fn decode_png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
        if bytes.len() >= 24 && &bytes[12..16] == b"IHDR" {
            let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
            let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
            return Some((width, height));
        }
        None
    }

    fn decode_jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
        let mut idx = 2;
        while idx < bytes.len() {
            if bytes[idx] != 0xFF {
                idx += 1;
                continue;
            }
            while idx < bytes.len() && bytes[idx] == 0xFF {
                idx += 1;
            }
            if idx >= bytes.len() {
                break;
            }
            let marker = bytes[idx];
            idx += 1;

            // SOF markers: SOF0 (0xC0), SOF1 (0xC1), SOF2 (0xC2)
            if marker == 0xC0 || marker == 0xC1 || marker == 0xC2 {
                if idx + 7 < bytes.len() {
                    let height = u16::from_be_bytes([bytes[idx + 3], bytes[idx + 4]]) as u32;
                    let width = u16::from_be_bytes([bytes[idx + 5], bytes[idx + 6]]) as u32;
                    return Some((width, height));
                }
            } else if marker == 0xD9 || marker == 0xDA {
                // End of image or start of scan
                break;
            } else {
                if idx + 2 <= bytes.len() {
                    let length = u16::from_be_bytes([bytes[idx], bytes[idx + 1]]) as usize;
                    idx += length;
                } else {
                    break;
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_valid_synthetic_png() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_valid.png");

        let mut png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG signature
        png.extend_from_slice(&[0, 0, 0, 13]); // IHDR chunk length
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&256u32.to_be_bytes()); // width 256
        png.extend_from_slice(&256u32.to_be_bytes()); // height 256
        png.extend_from_slice(&[8, 2, 0, 0, 0]); // Bit depth, color type, etc.
        png.extend_from_slice(&[0, 0, 0, 0]); // CRC

        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(&png).unwrap();

        let validator = InputValidator::default();
        let validated = validator.validate(&file_path).unwrap();

        assert_eq!(validated.width, 256);
        assert_eq!(validated.height, 256);
        assert_eq!(validated.mime_type, "image/png");
        assert_eq!(validated.size_bytes, png.len());
        assert!(!validated.sha256.is_empty());

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_rejects_too_small_resolution() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_small.png");

        let mut png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[0, 0, 0, 13]);
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&32u32.to_be_bytes()); // width 32 (< 64)
        png.extend_from_slice(&32u32.to_be_bytes()); // height 32 (< 64)
        png.extend_from_slice(&[8, 2, 0, 0, 0]);
        png.extend_from_slice(&[0, 0, 0, 0]);

        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(&png).unwrap();

        let validator = InputValidator::default();
        let err = validator.validate(&file_path).unwrap_err();
        assert_eq!(err.error_code(), "IMAGE_TOO_SMALL");

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_rejects_unsupported_format() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_text.txt");
        std::fs::write(&file_path, "Hello text").unwrap();

        let validator = InputValidator::default();
        let err = validator.validate(&file_path).unwrap_err();
        assert_eq!(err.error_code(), "UNSUPPORTED_FORMAT");

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_rejects_file_too_large() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_large.jpg");

        let validator = InputValidator::new(100); // 100 bytes max
        let large_bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]; // oversized with padding
        let mut padded = large_bytes;
        padded.resize(200, 0);

        std::fs::write(&file_path, &padded).unwrap();
        let err = validator.validate(&file_path).unwrap_err();
        assert_eq!(err.error_code(), "FILE_TOO_LARGE");

        let _ = std::fs::remove_file(file_path);
    }
}
