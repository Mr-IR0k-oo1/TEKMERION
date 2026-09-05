//! Deterministic evidence record definition, canonicalization, and SHA-256 hashing.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;
use url::Url;

use crate::error::EvidenceError;

/// Current supported evidence schema version.
pub const CURRENT_SCHEMA_VERSION: &str = "1.0.0";

/// Normalize a string into Unicode Normalization Form C (NFC).
///
/// Ensures that precomposed and decomposed forms (e.g. `é` vs `e` + `\u{0301}`)
/// produce byte-for-byte identical UTF-8 encodings.
pub fn normalize_utf8(input: &str) -> String {
    input.nfc().collect::<String>()
}

/// Normalize a URL into a deterministic canonical representation.
///
/// Steps:
/// 1. Strips fragment identifiers (`#...`).
/// 2. Normalizes default HTTP/HTTPS ports (removes `:80` and `:443`).
/// 3. Normalizes scheme and host to lowercase.
/// 4. Deterministically sorts query parameters lexicographically by key, then value.
/// 5. Ensures consistent path representation.
pub fn normalize_url(url: &Url) -> String {
    let mut normalized = url.clone();
    normalized.set_fragment(None);

    if (normalized.scheme() == "http" && normalized.port() == Some(80))
        || (normalized.scheme() == "https" && normalized.port() == Some(443))
    {
        let _ = normalized.set_port(None);
    }

    let mut pairs: Vec<(String, String)> = normalized
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if !pairs.is_empty() {
        pairs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        normalized.set_query(None);
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        for (k, v) in pairs {
            serializer.append_pair(&k, &v);
        }
        let query_str = serializer.finish();
        normalized.set_query(Some(&query_str));
    }

    normalized.to_string()
}

/// Format a floating-point number with deterministic fixed 6-decimal-place precision.
///
/// Rejects `NaN` and `Infinity` to prevent platform-dependent formatting quirks
/// and ensure deterministic cross-platform hashing without Debug formatting.
pub fn format_float(field: &'static str, value: f32) -> Result<String, EvidenceError> {
    if !value.is_finite() {
        return Err(EvidenceError::NonFiniteFloat { field, value });
    }
    Ok(format!("{:.6}", value))
}

/// Container for the five component SHA-256 hashes and composite Merkle root hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceHashes {
    /// SHA-256 hash of the normalized candidate image checksum.
    pub image_hash: String,
    /// SHA-256 hash of the normalized title and text content.
    pub content_hash: String,
    /// SHA-256 hash of the provenance metadata and normalized URL.
    pub metadata_hash: String,
    /// SHA-256 hash of the facial similarity score, model, and quality.
    pub face_result_hash: String,
    /// SHA-256 hash of the provenance chain (run ID, provider, platform, timestamp).
    pub provenance_hash: String,
    /// Composite Merkle tree root hash anchoring all five leaves.
    pub record_hash: String,
}

/// Tamper-evident record of discovered candidate evidence and facial verification results.
///
/// Designed with strict deterministic serialization rules:
/// - Explicit schema versioning
/// - Strict field ordering
/// - Unicode NFC normalization on all text
/// - Canonical URL representation
/// - No `HashMap`-dependent serialization
/// - No `Debug` formatting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub schema_version: String,
    pub run_id: String,
    pub source_url: Url,
    pub domain: String,
    pub platform: String,
    pub provider: String,
    pub retrieved_at: DateTime<Utc>,
    pub title: String,
    pub text: String,
    pub image_sha256: String,
    pub face_similarity: f32,
    pub face_model: String,
    pub candidate_quality: f32,
}

impl EvidenceRecord {
    /// Construct a new `EvidenceRecord` with explicit fields and default schema version `1.0.0`.
    pub fn new(
        run_id: impl Into<String>,
        source_url: Url,
        domain: impl Into<String>,
        platform: impl Into<String>,
        provider: impl Into<String>,
        retrieved_at: DateTime<Utc>,
        title: impl Into<String>,
        text: impl Into<String>,
        image_sha256: impl Into<String>,
        face_similarity: f32,
        face_model: impl Into<String>,
        candidate_quality: f32,
    ) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            run_id: run_id.into(),
            source_url,
            domain: domain.into(),
            platform: platform.into(),
            provider: provider.into(),
            retrieved_at,
            title: title.into(),
            text: text.into(),
            image_sha256: image_sha256.into(),
            face_similarity,
            face_model: face_model.into(),
            candidate_quality,
        }
    }

    /// Builder method for `schema_version`.
    pub fn with_schema_version(mut self, schema_version: impl Into<String>) -> Self {
        self.schema_version = schema_version.into();
        self
    }

    /// Builder method for `title`.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Builder method for `text`.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Builder method for `source_url`.
    pub fn with_source_url(mut self, url: Url) -> Self {
        self.source_url = url;
        self
    }

    /// Builder method for `image_sha256`.
    pub fn with_image_sha256(mut self, image_sha256: impl Into<String>) -> Self {
        self.image_sha256 = image_sha256.into();
        self
    }

    /// Builder method for `face_similarity`.
    pub fn with_face_similarity(mut self, face_similarity: f32) -> Self {
        self.face_similarity = face_similarity;
        self
    }

    /// Builder method for `face_model`.
    pub fn with_face_model(mut self, face_model: impl Into<String>) -> Self {
        self.face_model = face_model.into();
        self
    }

    /// Builder method for `candidate_quality`.
    pub fn with_candidate_quality(mut self, candidate_quality: f32) -> Self {
        self.candidate_quality = candidate_quality;
        self
    }

    /// Validate the schema version against the supported version.
    pub fn validate_schema(&self) -> Result<(), EvidenceError> {
        if self.schema_version.trim() != CURRENT_SCHEMA_VERSION {
            return Err(EvidenceError::InvalidSchemaVersion {
                found: self.schema_version.clone(),
                expected: CURRENT_SCHEMA_VERSION.to_string(),
            });
        }
        Ok(())
    }

    /// Compute the SHA-256 `image_hash`.
    ///
    /// Hashed input:
    /// `image:sha256:<len>:<lowercase_hex_image_sha256>`
    pub fn image_hash(&self) -> Result<String, EvidenceError> {
        let mut hasher = Sha256::new();
        hasher.update(b"image_component:v1\n");
        let clean = normalize_utf8(self.image_sha256.trim()).to_lowercase();
        hasher.update((clean.len() as u64).to_be_bytes());
        hasher.update(clean.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// Compute the SHA-256 `content_hash`.
    ///
    /// Hashed inputs:
    /// - Unicode NFC normalized `title`
    /// - Unicode NFC normalized `text`
    pub fn content_hash(&self) -> Result<String, EvidenceError> {
        let mut hasher = Sha256::new();
        hasher.update(b"content_component:v1\n");

        let nfc_title = normalize_utf8(self.title.trim());
        hasher.update((nfc_title.len() as u64).to_be_bytes());
        hasher.update(nfc_title.as_bytes());

        let nfc_text = normalize_utf8(self.text.trim());
        hasher.update((nfc_text.len() as u64).to_be_bytes());
        hasher.update(nfc_text.as_bytes());

        Ok(hex::encode(hasher.finalize()))
    }

    /// Compute the SHA-256 `metadata_hash`.
    ///
    /// Hashed inputs:
    /// - `schema_version`
    /// - `run_id`
    /// - canonical `source_url`
    /// - `domain`
    /// - `platform`
    /// - `provider`
    /// - `retrieved_at` in RFC 3339 UTC format
    pub fn metadata_hash(&self) -> Result<String, EvidenceError> {
        let mut hasher = Sha256::new();
        hasher.update(b"metadata_component:v1\n");

        let fields = [
            normalize_utf8(self.schema_version.trim()),
            normalize_utf8(self.run_id.trim()),
            normalize_url(&self.source_url),
            normalize_utf8(self.domain.trim()).to_lowercase(),
            normalize_utf8(self.platform.trim()).to_lowercase(),
            normalize_utf8(self.provider.trim()),
            self.retrieved_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        ];

        for field in &fields {
            hasher.update((field.len() as u64).to_be_bytes());
            hasher.update(field.as_bytes());
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// Compute the SHA-256 `face_result_hash`.
    ///
    /// Hashed inputs:
    /// - `face_similarity` formatted as fixed 6-decimal `{:.6}`
    /// - Unicode NFC normalized `face_model`
    /// - `candidate_quality` formatted as fixed 6-decimal `{:.6}`
    pub fn face_result_hash(&self) -> Result<String, EvidenceError> {
        let sim_str = format_float("face_similarity", self.face_similarity)?;
        let qual_str = format_float("candidate_quality", self.candidate_quality)?;
        let model_str = normalize_utf8(self.face_model.trim());

        let mut hasher = Sha256::new();
        hasher.update(b"face_result_component:v1\n");

        hasher.update((sim_str.len() as u64).to_be_bytes());
        hasher.update(sim_str.as_bytes());

        hasher.update((model_str.len() as u64).to_be_bytes());
        hasher.update(model_str.as_bytes());

        hasher.update((qual_str.len() as u64).to_be_bytes());
        hasher.update(qual_str.as_bytes());

        Ok(hex::encode(hasher.finalize()))
    }

    /// Compute the SHA-256 `provenance_hash`.
    ///
    /// Hashed inputs:
    /// - Unicode NFC normalized `run_id`
    /// - Unicode NFC normalized `provider`
    /// - Unicode NFC normalized lowercase `platform`
    /// - `retrieved_at` in RFC 3339 UTC format
    pub fn provenance_hash(&self) -> Result<String, EvidenceError> {
        let mut hasher = Sha256::new();
        hasher.update(b"provenance_component:v1\n");

        let fields = [
            normalize_utf8(self.run_id.trim()),
            normalize_utf8(self.provider.trim()),
            normalize_utf8(self.platform.trim()).to_lowercase(),
            self.retrieved_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        ];

        for field in &fields {
            hasher.update((field.len() as u64).to_be_bytes());
            hasher.update(field.as_bytes());
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// Compute the composite SHA-256 `record_hash` (Merkle tree root hash over all five leaves).
    pub fn record_hash(&self) -> Result<String, EvidenceError> {
        let hashes = self.compute_hashes()?;
        Ok(hashes.record_hash)
    }

    /// Compute all five component hashes and the Merkle tree root hash.
    pub fn compute_hashes(&self) -> Result<EvidenceHashes, EvidenceError> {
        let image_h = self.image_hash()?;
        let content_h = self.content_hash()?;
        let metadata_h = self.metadata_hash()?;
        let face_result_h = self.face_result_hash()?;
        let provenance_h = self.provenance_hash()?;

        let leaves = crate::merkle::EvidenceTreeLeaves {
            image_hash: image_h.clone(),
            content_hash: content_h.clone(),
            metadata_hash: metadata_h.clone(),
            face_result_hash: face_result_h.clone(),
            provenance_hash: provenance_h.clone(),
        };

        let tree = crate::merkle::EvidenceTree::new(leaves)?;
        let record_h = tree.root_hash().to_string();

        Ok(EvidenceHashes {
            image_hash: image_h,
            content_hash: content_h,
            metadata_hash: metadata_h,
            face_result_hash: face_result_h,
            provenance_hash: provenance_h,
            record_hash: record_h,
        })
    }

    /// Build the full deterministic Merkle evidence tree over all five canonical leaves.
    pub fn build_tree(&self) -> Result<crate::merkle::EvidenceTree, EvidenceError> {
        let hashes = self.compute_hashes()?;
        let leaves = crate::merkle::EvidenceTreeLeaves {
            image_hash: hashes.image_hash,
            content_hash: hashes.content_hash,
            metadata_hash: hashes.metadata_hash,
            face_result_hash: hashes.face_result_hash,
            provenance_hash: hashes.provenance_hash,
        };
        crate::merkle::EvidenceTree::new(leaves)
    }

    /// Build an `EvidenceBundle` containing the canonical leaves and Merkle root.
    pub fn build_bundle(&self) -> Result<crate::merkle::EvidenceBundle, EvidenceError> {
        let tree = self.build_tree()?;
        Ok(tree.bundle().with_record(self.clone()))
    }

    /// Deterministic canonical byte serialization of the record.
    ///
    /// Encodes all fields in fixed, stable sequence with big-endian length prefixes
    /// and normalized representations. No `HashMap` or `Debug` formatting is used.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EvidenceError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"TEKMERION_EVIDENCE_RECORD_V1\n");

        let sim_str = format_float("face_similarity", self.face_similarity)?;
        let qual_str = format_float("candidate_quality", self.candidate_quality)?;

        let fields: [(&'static str, String); 13] = [
            ("schema_version", normalize_utf8(self.schema_version.trim())),
            ("run_id", normalize_utf8(self.run_id.trim())),
            ("source_url", normalize_url(&self.source_url)),
            ("domain", normalize_utf8(self.domain.trim()).to_lowercase()),
            ("platform", normalize_utf8(self.platform.trim()).to_lowercase()),
            ("provider", normalize_utf8(self.provider.trim())),
            ("retrieved_at", self.retrieved_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            ("title", normalize_utf8(self.title.trim())),
            ("text", normalize_utf8(self.text.trim())),
            ("image_sha256", normalize_utf8(self.image_sha256.trim()).to_lowercase()),
            ("face_similarity", sim_str),
            ("face_model", normalize_utf8(self.face_model.trim())),
            ("candidate_quality", qual_str),
        ];

        for (name, val) in fields {
            bytes.extend_from_slice(name.as_bytes());
            bytes.push(b'=');
            bytes.extend_from_slice(&(val.len() as u64).to_be_bytes());
            bytes.extend_from_slice(val.as_bytes());
            bytes.push(b'\n');
        }

        Ok(bytes)
    }

    /// Deterministic canonical JSON serialization.
    ///
    /// Uses `BTreeMap` to enforce strict lexicographical key ordering with
    /// normalized values and no HashMap nondeterminism.
    pub fn canonical_json(&self) -> Result<String, EvidenceError> {
        let sim_str = format_float("face_similarity", self.face_similarity)?;
        let qual_str = format_float("candidate_quality", self.candidate_quality)?;

        let mut map: BTreeMap<&'static str, serde_json::Value> = BTreeMap::new();
        map.insert("candidate_quality", serde_json::Value::String(qual_str));
        map.insert("domain", serde_json::Value::String(normalize_utf8(self.domain.trim()).to_lowercase()));
        map.insert("face_model", serde_json::Value::String(normalize_utf8(self.face_model.trim())));
        map.insert("face_similarity", serde_json::Value::String(sim_str));
        map.insert("image_sha256", serde_json::Value::String(normalize_utf8(self.image_sha256.trim()).to_lowercase()));
        map.insert("platform", serde_json::Value::String(normalize_utf8(self.platform.trim()).to_lowercase()));
        map.insert("provider", serde_json::Value::String(normalize_utf8(self.provider.trim())));
        map.insert("retrieved_at", serde_json::Value::String(self.retrieved_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)));
        map.insert("run_id", serde_json::Value::String(normalize_utf8(self.run_id.trim())));
        map.insert("schema_version", serde_json::Value::String(normalize_utf8(self.schema_version.trim())));
        map.insert("source_url", serde_json::Value::String(normalize_url(&self.source_url)));
        map.insert("text", serde_json::Value::String(normalize_utf8(self.text.trim())));
        map.insert("title", serde_json::Value::String(normalize_utf8(self.title.trim())));

        serde_json::to_string(&map).map_err(|e| EvidenceError::Serialization(e.to_string()))
    }
}

impl From<EvidenceRecord> for tekmerion_core::EvidenceRecord {
    fn from(r: EvidenceRecord) -> Self {
        tekmerion_core::EvidenceRecord {
            schema_version: r.schema_version,
            run_id: r.run_id,
            source_url: r.source_url,
            domain: r.domain,
            platform: r.platform,
            provider: r.provider,
            retrieved_at: r.retrieved_at,
            title: r.title,
            text: r.text,
            image_sha256: r.image_sha256,
            face_similarity: r.face_similarity,
            face_model: r.face_model,
            candidate_quality: r.candidate_quality,
        }
    }
}

impl From<tekmerion_core::EvidenceRecord> for EvidenceRecord {
    fn from(r: tekmerion_core::EvidenceRecord) -> Self {
        EvidenceRecord {
            schema_version: r.schema_version,
            run_id: r.run_id,
            source_url: r.source_url,
            domain: r.domain,
            platform: r.platform,
            provider: r.provider,
            retrieved_at: r.retrieved_at,
            title: r.title,
            text: r.text,
            image_sha256: r.image_sha256,
            face_similarity: r.face_similarity,
            face_model: r.face_model,
            candidate_quality: r.candidate_quality,
        }
    }
}
