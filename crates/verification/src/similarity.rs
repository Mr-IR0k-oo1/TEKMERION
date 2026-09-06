//! Vector similarity calculation.
//!
//! Provides deterministic cosine similarity computation with rigorous numerical validation:
//! - Validates vector dimensions
//! - Rejects NaN / infinite values
//! - Rejects zero-norm vectors
//! - Clamps results deterministically to `[-1.0, 1.0]`
//! - Yields structured, domain-aware numerical errors

use thiserror::Error;

/// Structured numerical errors returned during cosine similarity computation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SimilarityError {
    /// Dimension mismatch between query and candidate embedding vectors.
    #[error("vector dimension mismatch: query dimension {query_dim}, candidate dimension {candidate_dim}")]
    DimensionMismatch {
        query_dim: usize,
        candidate_dim: usize,
    },

    /// Vector is empty (0 dimensions).
    #[error("cannot compute similarity for empty vector")]
    EmptyVector,

    /// Vector contains a non-finite floating point value (NaN, positive infinity, or negative infinity).
    #[error("vector contains non-finite value (NaN or infinity)")]
    NonFiniteValue,

    /// Vector magnitude (L2 norm) is zero or below numerical epsilon.
    #[error("vector has zero or negligible magnitude (norm <= EPSILON)")]
    ZeroNorm,
}

/// Compute cosine similarity between two floating-point vectors:
///
/// $$\cos(\theta) = \frac{\mathbf{a} \cdot \mathbf{b}}{\|\mathbf{a}\|_2 \|\mathbf{b}\|_2}$$
///
/// # Preconditions & Guarantees
/// - Both vectors must have the same non-zero dimension.
/// - Every element must be finite (no NaNs or infinities).
/// - Both vectors must have non-zero Euclidean norm ($> \epsilon$).
/// - The return value is deterministically clamped to `[-1.0, 1.0]`.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32, SimilarityError> {
    if a.is_empty() || b.is_empty() {
        return Err(SimilarityError::EmptyVector);
    }

    if a.len() != b.len() {
        return Err(SimilarityError::DimensionMismatch {
            query_dim: a.len(),
            candidate_dim: b.len(),
        });
    }

    let mut dot = 0.0f32;
    let mut norm_a_sq = 0.0f32;
    let mut norm_b_sq = 0.0f32;

    for (&va, &vb) in a.iter().zip(b.iter()) {
        if !va.is_finite() || !vb.is_finite() {
            return Err(SimilarityError::NonFiniteValue);
        }
        dot += va * vb;
        norm_a_sq += va * va;
        norm_b_sq += vb * vb;
    }

    let norm_a = norm_a_sq.sqrt();
    let norm_b = norm_b_sq.sqrt();

    if norm_a <= f32::EPSILON || norm_b <= f32::EPSILON {
        return Err(SimilarityError::ZeroNorm);
    }

    let denom = norm_a * norm_b;
    let similarity = (dot / denom).clamp(-1.0, 1.0);

    // Guard against potential edge-case NaN produced by floating-point division
    if !similarity.is_finite() {
        return Err(SimilarityError::NonFiniteValue);
    }

    Ok(similarity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_vectors_yield_unit_similarity() {
        let a = [0.6, 0.8, 0.0];
        let sim = cosine_similarity(&a, &a).unwrap();
        assert!((sim - 1.0).abs() < 1e-6, "expected 1.0, got {sim}");
    }

    #[test]
    fn test_opposite_vectors_yield_negative_one() {
        let a = [1.0, 2.0, 3.0];
        let b = [-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!((sim - (-1.0)).abs() < 1e-6, "expected -1.0, got {sim}");
    }

    #[test]
    fn test_orthogonal_vectors_yield_zero() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!(sim.abs() < 1e-6, "expected 0.0, got {sim}");
    }

    #[test]
    fn test_scale_invariance() {
        let a = [1.0, 2.0, 3.0];
        let b = [10.0, 20.0, 30.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "expected 1.0 for scaled vector, got {sim}"
        );
    }

    #[test]
    fn test_known_angle_sixty_degrees() {
        // [1, 0] and [0.5, sqrt(3)/2] -> angle 60 deg -> cos = 0.5
        let a = [1.0, 0.0];
        let b = [0.5, (3.0f32).sqrt() / 2.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!(
            (sim - 0.5).abs() < 1e-6,
            "expected 0.5 for 60 deg angle, got {sim}"
        );
    }

    #[test]
    fn test_dimension_mismatch_is_rejected() {
        let a = [1.0, 2.0, 3.0];
        let b = [1.0, 2.0];
        let err = cosine_similarity(&a, &b).unwrap_err();
        match err {
            SimilarityError::DimensionMismatch {
                query_dim,
                candidate_dim,
            } => {
                assert_eq!(query_dim, 3);
                assert_eq!(candidate_dim, 2);
            }
            other => panic!("expected DimensionMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_empty_vector_is_rejected() {
        let a: [f32; 0] = [];
        let b: [f32; 0] = [];
        let err = cosine_similarity(&a, &b).unwrap_err();
        assert_eq!(err, SimilarityError::EmptyVector);
    }

    #[test]
    fn test_nan_values_are_rejected() {
        let a = [1.0, f32::NAN, 3.0];
        let b = [1.0, 2.0, 3.0];
        let err = cosine_similarity(&a, &b).unwrap_err();
        assert_eq!(err, SimilarityError::NonFiniteValue);

        let err2 = cosine_similarity(&b, &a).unwrap_err();
        assert_eq!(err2, SimilarityError::NonFiniteValue);
    }

    #[test]
    fn test_infinite_values_are_rejected() {
        let a = [1.0, f32::INFINITY, 3.0];
        let b = [1.0, 2.0, 3.0];
        let err = cosine_similarity(&a, &b).unwrap_err();
        assert_eq!(err, SimilarityError::NonFiniteValue);

        let c = [1.0, f32::NEG_INFINITY, 3.0];
        let err2 = cosine_similarity(&c, &b).unwrap_err();
        assert_eq!(err2, SimilarityError::NonFiniteValue);
    }

    #[test]
    fn test_zero_norm_vectors_are_rejected() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 2.0, 3.0];
        let err = cosine_similarity(&a, &b).unwrap_err();
        assert_eq!(err, SimilarityError::ZeroNorm);

        let err2 = cosine_similarity(&b, &a).unwrap_err();
        assert_eq!(err2, SimilarityError::ZeroNorm);
    }

    #[test]
    fn test_deterministic_clamping() {
        // High dimensional identical vectors with potential floating point rounding > 1.0
        let a = vec![1.0000001f32; 512];
        let b = vec![1.0000001f32; 512];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!((-1.0..=1.0).contains(&sim));
        assert!((sim - 1.0).abs() < 1e-6);
    }
}
