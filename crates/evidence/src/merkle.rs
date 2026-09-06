//! Deterministic Merkle-style evidence tree implementation.
//!
//! Provides cryptographic anchoring for TEKMERION evidence records over five
//! canonical input leaves:
//! 1. `image_hash`
//! 2. `content_hash`
//! 3. `metadata_hash`
//! 4. `face_result_hash`
//! 5. `provenance_hash`
//!
//! # Odd-Node Handling Strategy (RFC 6962 Standard)
//!
//! When any level in the binary tree contains an odd count of nodes ($2k + 1$):
//! 1. The first $2k$ nodes are paired into $k$ parents: `hash_parent(nodes[2*i], nodes[2*i + 1])`.
//! 2. The trailing odd node `nodes[2k]` is promoted directly to the next level without
//!    duplicate hashing or synthetic padding.
//!
//! ## Defense Against CVE-2012-2459 (Bitcoin Duplicate-Leaf Vulnerability)
//! In Bitcoin, an odd node was duplicated (`hash_parent(node, node)`). This introduced
//! a known vulnerability where `[A, B, C]` produces the identical Merkle root as `[A, B, C, C]`.
//! TEKMERION's promotion approach completely eliminates duplicate-leaf vulnerabilities
//! while preserving strict deterministic ordering and succinct audit paths.
//!
//! # Domain Separation
//!
//! To prevent second-preimage attacks between leaf nodes and internal parent nodes:
//! - Leaf hashing prefix: `0x00` (`LEAF_PREFIX`)
//! - Internal node hashing prefix: `0x01` (`NODE_PREFIX`)

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::EvidenceError;
use crate::record::EvidenceRecord;

/// Domain separation prefix for hashing leaf nodes.
pub const LEAF_PREFIX: u8 = 0x00;

/// Domain separation prefix for hashing internal parent nodes.
pub const NODE_PREFIX: u8 = 0x01;

/// Canonical enumeration of the five evidence tree leaf positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LeafType {
    Image = 0,
    Content = 1,
    Metadata = 2,
    Face = 3,
    Provenance = 4,
}

impl LeafType {
    /// All leaf types in strict canonical ordering.
    pub const ALL: [LeafType; 5] = [
        LeafType::Image,
        LeafType::Content,
        LeafType::Metadata,
        LeafType::Face,
        LeafType::Provenance,
    ];

    /// Human-friendly display label.
    pub fn label(self) -> &'static str {
        match self {
            LeafType::Image => "IMAGE",
            LeafType::Content => "CONTENT",
            LeafType::Metadata => "METADATA",
            LeafType::Face => "FACE",
            LeafType::Provenance => "PROVENANCE",
        }
    }
}

/// Strongly typed container for the five input leaves of TEKMERION's evidence tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceTreeLeaves {
    pub image_hash: String,
    pub content_hash: String,
    pub metadata_hash: String,
    pub face_result_hash: String,
    pub provenance_hash: String,
}

impl EvidenceTreeLeaves {
    /// Create a new leaf set with explicit leaf values.
    pub fn new(
        image_hash: impl Into<String>,
        content_hash: impl Into<String>,
        metadata_hash: impl Into<String>,
        face_result_hash: impl Into<String>,
        provenance_hash: impl Into<String>,
    ) -> Self {
        Self {
            image_hash: image_hash.into(),
            content_hash: content_hash.into(),
            metadata_hash: metadata_hash.into(),
            face_result_hash: face_result_hash.into(),
            provenance_hash: provenance_hash.into(),
        }
    }

    /// Return the leaves as a deterministic vector in canonical order:
    /// `[image_hash, content_hash, metadata_hash, face_result_hash, provenance_hash]`
    pub fn to_leaves_vec(&self) -> Vec<String> {
        vec![
            self.image_hash.clone(),
            self.content_hash.clone(),
            self.metadata_hash.clone(),
            self.face_result_hash.clone(),
            self.provenance_hash.clone(),
        ]
    }
}

/// Sibling direction in a Merkle inclusion proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofDirection {
    Left,
    Right,
}

/// Cryptographic Merkle inclusion proof for a single leaf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProof {
    /// 0-indexed position of the leaf in the original canonical leaves vector.
    pub leaf_index: usize,
    /// The raw hash string of the leaf being proven.
    pub leaf_hash: String,
    /// Sequence of sibling hashes and relative directions from leaf to root.
    pub audit_path: Vec<(String, ProofDirection)>,
    /// Expected root hash against which the proof validates.
    pub root_hash: String,
}

/// Container bundling the canonical leaves and the computed Merkle root hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub leaves: Vec<String>,
    pub root_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<EvidenceRecord>,
}

impl EvidenceBundle {
    /// Construct a new `EvidenceBundle` with leaves and root hash.
    pub fn new(leaves: Vec<String>, root_hash: impl Into<String>) -> Self {
        Self {
            leaves,
            root_hash: root_hash.into(),
            record: None,
        }
    }

    /// Attach the original evidence record to the bundle.
    pub fn with_record(mut self, record: EvidenceRecord) -> Self {
        self.record = Some(record);
        self
    }
}

impl From<EvidenceBundle> for tekmerion_core::EvidenceBundle {
    fn from(b: EvidenceBundle) -> Self {
        tekmerion_core::EvidenceBundle {
            leaves: b.leaves,
            root_hash: b.root_hash,
            record: b.record.map(Into::into),
        }
    }
}

impl From<tekmerion_core::EvidenceBundle> for EvidenceBundle {
    fn from(b: tekmerion_core::EvidenceBundle) -> Self {
        Self {
            leaves: b.leaves,
            root_hash: b.root_hash,
            record: b.record.map(Into::into),
        }
    }
}

/// Compute a domain-separated SHA-256 digest for a leaf node.
///
/// Formula: `SHA-256(0x00 || leaf_bytes)`
pub fn hash_leaf(leaf: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update([LEAF_PREFIX]);
    hasher.update(leaf.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute a domain-separated SHA-256 digest for an internal parent node.
///
/// Formula: `SHA-256(0x01 || left_bytes || right_bytes)`
pub fn hash_parent(left: &str, right: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update([NODE_PREFIX]);
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    hex::encode(hasher.finalize())
}

/// Deterministic Merkle evidence tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceTree {
    leaves: Vec<String>,
    layers: Vec<Vec<String>>,
    root_hash: String,
}

impl EvidenceTree {
    /// Construct a tree from the five canonical `EvidenceTreeLeaves`.
    pub fn new(leaves_input: EvidenceTreeLeaves) -> Result<Self, EvidenceError> {
        Self::from_leaves(leaves_input.to_leaves_vec())
    }

    /// Construct a tree from an arbitrary vector of leaf hash strings.
    ///
    /// Preserves strict deterministic order of the provided leaves.
    pub fn from_leaves(leaves: Vec<String>) -> Result<Self, EvidenceError> {
        if leaves.is_empty() {
            return Err(EvidenceError::EmptyTree);
        }

        let mut current_layer: Vec<String> = leaves.iter().map(|l| hash_leaf(l)).collect();
        let mut layers: Vec<Vec<String>> = vec![current_layer.clone()];

        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();
            let mut i = 0;
            while i < current_layer.len() {
                if i + 1 < current_layer.len() {
                    let parent = hash_parent(&current_layer[i], &current_layer[i + 1]);
                    next_layer.push(parent);
                    i += 2;
                } else {
                    // Odd-node promotion: carry forward directly without duplicate hashing
                    next_layer.push(current_layer[i].clone());
                    i += 1;
                }
            }
            layers.push(next_layer.clone());
            current_layer = next_layer;
        }

        let root_hash = current_layer[0].clone();

        Ok(Self {
            leaves,
            layers,
            root_hash,
        })
    }

    /// Access the leaves of the tree.
    pub fn leaves(&self) -> &[String] {
        &self.leaves
    }

    /// Access the computed root hash of the tree.
    pub fn root_hash(&self) -> &str {
        &self.root_hash
    }

    /// Convert into an `EvidenceBundle`.
    pub fn bundle(&self) -> EvidenceBundle {
        EvidenceBundle::new(self.leaves.clone(), self.root_hash.clone())
    }

    /// Generate an inclusion proof for the leaf at `leaf_index`.
    pub fn generate_proof(&self, leaf_index: usize) -> Result<MerkleProof, EvidenceError> {
        if leaf_index >= self.leaves.len() {
            return Err(EvidenceError::LeafIndexOutOfBounds {
                index: leaf_index,
                count: self.leaves.len(),
            });
        }

        let mut audit_path = Vec::new();
        let mut idx = leaf_index;

        for layer_idx in 0..self.layers.len() - 1 {
            let layer = &self.layers[layer_idx];
            if idx.is_multiple_of(2) {
                if idx + 1 < layer.len() {
                    audit_path.push((layer[idx + 1].clone(), ProofDirection::Right));
                }
                // When idx is an odd promoted node at the end, it has no sibling at this level
            } else {
                audit_path.push((layer[idx - 1].clone(), ProofDirection::Left));
            }
            idx /= 2;
        }

        Ok(MerkleProof {
            leaf_index,
            leaf_hash: self.leaves[leaf_index].clone(),
            audit_path,
            root_hash: self.root_hash.clone(),
        })
    }
}

/// Recompute the root hash from a slice of leaf strings using the canonical Merkle algorithm.
pub fn recompute_root(leaves: &[String]) -> Result<String, EvidenceError> {
    if leaves.is_empty() {
        return Err(EvidenceError::EmptyTree);
    }
    let tree = EvidenceTree::from_leaves(leaves.to_vec())?;
    Ok(tree.root_hash().to_string())
}

/// Recompute the root hash from a Merkle inclusion proof.
pub fn recompute_root_from_proof(proof: &MerkleProof) -> String {
    let mut current = hash_leaf(&proof.leaf_hash);
    for (sibling, direction) in &proof.audit_path {
        current = match direction {
            ProofDirection::Left => hash_parent(sibling, &current),
            ProofDirection::Right => hash_parent(&current, sibling),
        };
    }
    current
}

/// Verify a cryptographic Merkle inclusion proof.
///
/// Returns `true` if and only if recomputing from the proof's leaf hash along
/// its audit path yields the exact root hash recorded in the proof.
pub fn verify_proof(proof: &MerkleProof) -> bool {
    if proof.root_hash.is_empty() {
        return false;
    }
    let computed = recompute_root_from_proof(proof);
    computed == proof.root_hash
}
