use serde::{Deserialize, Serialize};
use thiserror::Error;

use tekmerion_core::{SearchCandidate, VerificationResult, VerificationStatus};

/// Structured errors that can occur during candidate ranking or weight configuration.
#[derive(Debug, Error, PartialEq)]
pub enum RankingError {
    #[error("weight '{name}' must be finite (got {value})")]
    NonFiniteWeight { name: &'static str, value: f32 },

    #[error("weight '{name}' must be non-negative (got {value})")]
    NegativeWeight { name: &'static str, value: f32 },

    #[error("total weight sum must be strictly positive (sum is {sum})")]
    ZeroTotalWeight { sum: f32 },
}

/// Configurable weights for candidate ranking scoring.
///
/// # Scoring Formula
///
/// Candidate ranking scores are computed using a normalized multi-factor linear weighting:
///
/// $$ \text{ranking\_score} = \frac{w_{\text{sim}} \cdot S_{\text{face}} + w_{\text{fq}} \cdot Q_{\text{face}} + w_{\text{rel}} \cdot R_{\text{source}} + w_{\text{iq}} \cdot Q_{\text{image}}}{W} $$
///
/// where $W = w_{\text{sim}} + w_{\text{fq}} + w_{\text{rel}} + w_{\text{iq}} > 0$, and:
/// - $S_{\text{face}}$: `face_similarity` $\in [0.0, 1.0]$, clamped from cosine similarity $[-1.0, 1.0]$.
/// - $Q_{\text{face}}$: `face_quality` $\in [0.0, 1.0]$, face detection and landmark clarity score.
/// - $R_{\text{source}}$: `source_relevance` $\in [0.0, 1.0]$, search provider confidence or ranking decay.
/// - $Q_{\text{image}}$: `image_quality` $\in [0.0, 1.0]$, candidate image resolution and fidelity metric.
///
/// The composite `quality_score` combines face quality and image quality:
/// $$ \text{quality\_score} = \begin{cases} \frac{w_{\text{fq}} \cdot Q_{\text{face}} + w_{\text{iq}} \cdot Q_{\text{image}}}{w_{\text{fq}} + w_{\text{iq}}} & \text{if } w_{\text{fq}} + w_{\text{iq}} > 0 \\ Q_{\text{face}} & \text{otherwise} \end{cases} $$
///
/// # Non-Probability Notice
///
/// The ranking score is a deterministic multi-factor heuristic relevance metric designed solely
/// for sorting, triaging, and prioritizing search candidates. It does **NOT** represent a
/// statistical or Bayesian probability of personal identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankingWeights {
    pub face_similarity: f32,
    pub face_quality: f32,
    pub source_relevance: f32,
    pub image_quality: f32,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            face_similarity: 0.50,
            face_quality: 0.25,
            source_relevance: 0.15,
            image_quality: 0.10,
        }
    }
}

impl RankingWeights {
    /// Create and validate a new weight configuration.
    pub fn new(
        face_similarity: f32,
        face_quality: f32,
        source_relevance: f32,
        image_quality: f32,
    ) -> Result<Self, RankingError> {
        let weights = Self {
            face_similarity,
            face_quality,
            source_relevance,
            image_quality,
        };
        weights.validate()?;
        Ok(weights)
    }

    /// Validate that all weights are finite, non-negative, and sum to > 0.
    pub fn validate(&self) -> Result<(), RankingError> {
        self.check_weight("face_similarity", self.face_similarity)?;
        self.check_weight("face_quality", self.face_quality)?;
        self.check_weight("source_relevance", self.source_relevance)?;
        self.check_weight("image_quality", self.image_quality)?;

        let sum = self.total_weight();
        if sum <= 0.0 || !sum.is_finite() {
            return Err(RankingError::ZeroTotalWeight { sum });
        }
        Ok(())
    }

    fn check_weight(&self, name: &'static str, val: f32) -> Result<(), RankingError> {
        if !val.is_finite() {
            return Err(RankingError::NonFiniteWeight { name, value: val });
        }
        if val < 0.0 {
            return Err(RankingError::NegativeWeight { name, value: val });
        }
        Ok(())
    }

    /// Sum of all weights.
    #[inline]
    pub fn total_weight(&self) -> f32 {
        self.face_similarity + self.face_quality + self.source_relevance + self.image_quality
    }

    /// Sum of quality-related weights ($w_{\text{fq}} + w_{\text{iq}}$).
    #[inline]
    pub fn total_quality_weight(&self) -> f32 {
        self.face_quality + self.image_quality
    }

    pub fn with_similarity(mut self, weight: f32) -> Result<Self, RankingError> {
        self.check_weight("face_similarity", weight)?;
        self.face_similarity = weight;
        self.validate()?;
        Ok(self)
    }

    pub fn with_face_quality(mut self, weight: f32) -> Result<Self, RankingError> {
        self.check_weight("face_quality", weight)?;
        self.face_quality = weight;
        self.validate()?;
        Ok(self)
    }

    pub fn with_source_relevance(mut self, weight: f32) -> Result<Self, RankingError> {
        self.check_weight("source_relevance", weight)?;
        self.source_relevance = weight;
        self.validate()?;
        Ok(self)
    }

    pub fn with_image_quality(mut self, weight: f32) -> Result<Self, RankingError> {
        self.check_weight("image_quality", weight)?;
        self.image_quality = weight;
        self.validate()?;
        Ok(self)
    }
}

/// Rich candidate ranking input providing optional explicit source relevance and image quality.
#[derive(Debug, Clone)]
pub struct CandidateRankingInput {
    pub verification: VerificationResult,
    pub source_relevance: Option<f32>,
    pub image_quality: Option<f32>,
}

impl From<VerificationResult> for CandidateRankingInput {
    fn from(verification: VerificationResult) -> Self {
        Self {
            verification,
            source_relevance: None,
            image_quality: None,
        }
    }
}

/// A fully ranked candidate with deterministic rank, scores, and source attribution.
///
/// # Field Semantics
/// - `face_similarity`: Cosine similarity of candidate face vs query face $\in [0.0, 1.0]$.
/// - `quality_score`: Composite quality combining face detection and image resolution $\in [0.0, 1.0]$.
/// - `ranking_score`: Overall multi-factor heuristic ranking score $\in [0.0, 1.0]$.
///
/// Neither `ranking_score` nor `face_similarity` represents a probability of identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedCandidate {
    pub rank: usize,
    pub verification: VerificationResult,
    pub face_similarity: f32,
    pub quality_score: f32,
    pub ranking_score: f32,
    pub face_quality: f32,
    pub source_relevance: f32,
    pub image_quality: f32,
}

impl RankedCandidate {
    /// 1-indexed deterministic rank (1 = highest rank).
    #[inline]
    pub fn rank(&self) -> usize {
        self.rank
    }

    /// Normalized face similarity score $\in [0.0, 1.0]$.
    #[inline]
    pub fn face_similarity(&self) -> f32 {
        self.face_similarity
    }

    /// Composite quality score $\in [0.0, 1.0]$.
    #[inline]
    pub fn quality_score(&self) -> f32 {
        self.quality_score
    }

    /// Multi-factor heuristic ranking score $\in [0.0, 1.0]$.
    #[inline]
    pub fn ranking_score(&self) -> f32 {
        self.ranking_score
    }

    /// Candidate source / domain string.
    #[inline]
    pub fn source(&self) -> &str {
        &self.verification.candidate.domain
    }

    /// Candidate verification status.
    #[inline]
    pub fn status(&self) -> VerificationStatus {
        self.verification.status
    }

    /// Underlying candidate discovery record.
    #[inline]
    pub fn candidate(&self) -> &SearchCandidate {
        &self.verification.candidate
    }
}

/// Candidate ranking engine implementing deterministic multi-factor scoring.
#[derive(Debug, Clone, Default)]
pub struct CandidateRanker {
    weights: RankingWeights,
}

impl CandidateRanker {
    /// Create a ranker with default weights.
    pub fn new() -> Self {
        Self {
            weights: RankingWeights::default(),
        }
    }

    /// Create a ranker with custom weights.
    pub fn with_weights(weights: RankingWeights) -> Self {
        Self { weights }
    }

    /// Access the active ranking weights.
    pub fn weights(&self) -> &RankingWeights {
        &self.weights
    }

    /// Rank a list of raw `VerificationResult` records.
    pub fn rank_results(&self, results: Vec<VerificationResult>) -> Vec<RankedCandidate> {
        let inputs: Vec<CandidateRankingInput> = results.into_iter().map(Into::into).collect();
        self.rank_inputs(inputs)
    }

    /// Rank rich `CandidateRankingInput` items with custom relevance and image quality.
    pub fn rank_inputs(&self, inputs: Vec<CandidateRankingInput>) -> Vec<RankedCandidate> {
        let total_w = self.weights.total_weight();
        let quality_w = self.weights.total_quality_weight();

        let mut ranked: Vec<RankedCandidate> = inputs
            .into_iter()
            .enumerate()
            .map(|(idx, input)| {
                let v = input.verification;

                // 1. Normalized Face Similarity: clamped to [0.0, 1.0].
                // If status is NoFace or Error, similarity is strictly 0.0.
                let face_sim = match v.status {
                    VerificationStatus::Verified | VerificationStatus::BelowThreshold => {
                        v.similarity.clamp(0.0, 1.0)
                    }
                    VerificationStatus::NoFace | VerificationStatus::Error => 0.0,
                };

                // 2. Face Quality: clamped to [0.0, 1.0].
                let face_qual = match v.status {
                    VerificationStatus::Verified | VerificationStatus::BelowThreshold => {
                        v.quality.clamp(0.0, 1.0)
                    }
                    VerificationStatus::NoFace | VerificationStatus::Error => 0.0,
                };

                // 3. Source Relevance: explicit, or fallback to discovery rank decay: 1 / (1 + 0.1 * idx).
                let source_rel = input
                    .source_relevance
                    .unwrap_or_else(|| (1.0 / (1.0 + 0.1 * idx as f32)).clamp(0.1, 1.0))
                    .clamp(0.0, 1.0);

                // 4. Image Quality: explicit, or derived from face quality / image status.
                let img_qual = input
                    .image_quality
                    .unwrap_or_else(|| match v.status {
                        VerificationStatus::Verified | VerificationStatus::BelowThreshold => {
                            face_qual.max(0.70)
                        }
                        VerificationStatus::NoFace => 0.50,
                        VerificationStatus::Error => 0.0,
                    })
                    .clamp(0.0, 1.0);

                // 5. Composite Quality Score:
                let comp_quality = if quality_w > 0.0 {
                    ((self.weights.face_quality * face_qual
                        + self.weights.image_quality * img_qual)
                        / quality_w)
                        .clamp(0.0, 1.0)
                } else {
                    face_qual
                };

                // 6. Overall Multi-Factor Ranking Score:
                let raw_ranking_score = if v.status == VerificationStatus::Error {
                    0.0
                } else {
                    let weighted_sum = self.weights.face_similarity * face_sim
                        + self.weights.face_quality * face_qual
                        + self.weights.source_relevance * source_rel
                        + self.weights.image_quality * img_qual;
                    (weighted_sum / total_w).clamp(0.0, 1.0)
                };

                RankedCandidate {
                    rank: 0, // Assigned after deterministic sort
                    verification: v,
                    face_similarity: face_sim,
                    quality_score: comp_quality,
                    ranking_score: raw_ranking_score,
                    face_quality: face_qual,
                    source_relevance: source_rel,
                    image_quality: img_qual,
                }
            })
            .collect();

        // Strict deterministic total sort
        ranked.sort_by(Self::compare_candidates);

        // Assign 1-indexed deterministic ranks
        for (idx, item) in ranked.iter_mut().enumerate() {
            item.rank = idx + 1;
        }

        ranked
    }

    /// Strict total ordering comparison for deterministic sorting.
    ///
    /// Tie-breaking hierarchy:
    /// 1. Primary: descending `ranking_score`
    /// 2. Secondary: descending `face_similarity`
    /// 3. Tertiary: descending `quality_score`
    /// 4. Quaternary: descending `source_relevance`
    /// 5. Quinary: status priority (`Verified` > `BelowThreshold` > `NoFace` > `Error`)
    /// 6. Senary: canonical URL string lexicographical ordering (absolute tie-breaker)
    pub fn compare_candidates(a: &RankedCandidate, b: &RankedCandidate) -> std::cmp::Ordering {
        // 1. Primary: ranking_score (descending)
        b.ranking_score
            .total_cmp(&a.ranking_score)
            // 2. Secondary: face_similarity (descending)
            .then_with(|| b.face_similarity.total_cmp(&a.face_similarity))
            // 3. Tertiary: quality_score (descending)
            .then_with(|| b.quality_score.total_cmp(&a.quality_score))
            // 4. Quaternary: source_relevance (descending)
            .then_with(|| b.source_relevance.total_cmp(&a.source_relevance))
            // 5. Quinary: status priority
            .then_with(|| {
                status_priority(&b.verification.status).cmp(&status_priority(&a.verification.status))
            })
            // 6. Senary: canonical URL lexicographical ordering
            .then_with(|| {
                a.verification
                    .candidate
                    .url
                    .as_str()
                    .cmp(b.verification.candidate.url.as_str())
            })
    }
}

fn status_priority(status: &VerificationStatus) -> u8 {
    match status {
        VerificationStatus::Verified => 3,
        VerificationStatus::BelowThreshold => 2,
        VerificationStatus::NoFace => 1,
        VerificationStatus::Error => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn sample_candidate(id: &str, domain: &str) -> SearchCandidate {
        SearchCandidate {
            url: Url::parse(&format!("https://{domain}/item/{id}")).unwrap(),
            title: Some(format!("Candidate {id}")),
            domain: domain.to_string(),
            image_url: Some(Url::parse(&format!("https://{domain}/img/{id}.jpg")).unwrap()),
            thumbnail_url: None,
            snippet: Some(format!("Snippet for {id}")),
            provider: "test_provider".to_string(),
            discovered_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_default_weights_scoring_formula() {
        let ranker = CandidateRanker::new();
        let cand = sample_candidate("1", "example.com");
        let v = VerificationResult::new(
            cand,
            0.80, // similarity
            0.90, // quality
            Some(0),
            Some("hash1".to_string()),
            VerificationStatus::Verified,
        );

        let input = CandidateRankingInput {
            verification: v,
            source_relevance: Some(1.0),
            image_quality: Some(1.0),
        };

        let ranked = ranker.rank_inputs(vec![input]);
        assert_eq!(ranked.len(), 1);
        let c = &ranked[0];

        // Formula: (0.50*0.80 + 0.25*0.90 + 0.15*1.0 + 0.10*1.0) / 1.00
        // = 0.40 + 0.225 + 0.15 + 0.10 = 0.875
        let expected_score = 0.875;
        assert!((c.ranking_score() - expected_score).abs() < 1e-4);
        assert_eq!(c.face_similarity(), 0.80);

        // Quality score: (0.25*0.90 + 0.10*1.0) / (0.25 + 0.10)
        // = (0.225 + 0.10) / 0.35 = 0.325 / 0.35 = 0.92857
        let expected_quality = (0.225 + 0.10) / 0.35;
        assert!((c.quality_score() - expected_quality).abs() < 1e-4);
    }

    #[test]
    fn test_configurable_weights() {
        // Similarity-dominant weights
        let sim_weights = RankingWeights::new(0.90, 0.05, 0.05, 0.0).unwrap();
        let sim_ranker = CandidateRanker::with_weights(sim_weights);

        // Quality-dominant weights
        let qual_weights = RankingWeights::new(0.10, 0.80, 0.05, 0.05).unwrap();
        let qual_ranker = CandidateRanker::with_weights(qual_weights);

        let cand1 = sample_candidate("1", "high-sim.org");
        let v1 = VerificationResult::new(
            cand1,
            0.95, // high sim
            0.20, // low quality
            Some(0),
            None,
            VerificationStatus::Verified,
        );

        let cand2 = sample_candidate("2", "high-qual.org");
        let v2 = VerificationResult::new(
            cand2,
            0.60, // lower sim
            0.98, // high quality
            Some(0),
            None,
            VerificationStatus::BelowThreshold,
        );

        // Under similarity weights, cand1 must win
        let ranked_sim = sim_ranker.rank_results(vec![v1.clone(), v2.clone()]);
        assert_eq!(ranked_sim[0].source(), "high-sim.org");
        assert_eq!(ranked_sim[0].rank(), 1);

        // Under quality weights, cand2 must win
        let ranked_qual = qual_ranker.rank_results(vec![v1, v2]);
        assert_eq!(ranked_qual[0].source(), "high-qual.org");
        assert_eq!(ranked_qual[0].rank(), 1);
    }

    #[test]
    fn test_weights_validation_rejects_invalid_values() {
        assert!(matches!(
            RankingWeights::new(-0.1, 0.5, 0.2, 0.2),
            Err(RankingError::NegativeWeight { name: "face_similarity", .. })
        ));

        assert!(matches!(
            RankingWeights::new(f32::NAN, 0.5, 0.2, 0.2),
            Err(RankingError::NonFiniteWeight { name: "face_similarity", .. })
        ));

        assert!(matches!(
            RankingWeights::new(0.0, 0.0, 0.0, 0.0),
            Err(RankingError::ZeroTotalWeight { .. })
        ));
    }

    #[test]
    fn test_error_status_gets_zero_ranking_score() {
        let ranker = CandidateRanker::new();
        let cand = sample_candidate("err", "error.net");
        let v = VerificationResult::with_error(cand, "Network error");

        let ranked = ranker.rank_results(vec![v]);
        assert_eq!(ranked[0].ranking_score(), 0.0);
        assert_eq!(ranked[0].face_similarity(), 0.0);
        assert_eq!(ranked[0].status(), VerificationStatus::Error);
    }

    #[test]
    fn test_no_face_status_penalizes_similarity_and_face_quality() {
        let ranker = CandidateRanker::new();
        let cand = sample_candidate("nf", "landscape.com");
        let v = VerificationResult::new(
            cand,
            0.0,
            0.0,
            None,
            None,
            VerificationStatus::NoFace,
        );

        let input = CandidateRankingInput {
            verification: v,
            source_relevance: Some(0.8),
            image_quality: Some(0.9),
        };

        let ranked = ranker.rank_inputs(vec![input]);
        let c = &ranked[0];
        assert_eq!(c.face_similarity(), 0.0);
        assert_eq!(c.face_quality, 0.0);
        assert_eq!(c.status(), VerificationStatus::NoFace);
        // Score only receives source_relevance (0.15*0.8) + image_quality (0.10*0.9) = 0.12 + 0.09 = 0.21
        assert!((c.ranking_score() - 0.21).abs() < 1e-4);
    }

    #[test]
    fn test_deterministic_ordering_under_all_permutations() {
        let ranker = CandidateRanker::new();

        let v1 = VerificationResult::new(
            sample_candidate("1", "a.org"),
            0.92,
            0.90,
            Some(0),
            None,
            VerificationStatus::Verified,
        );
        let v2 = VerificationResult::new(
            sample_candidate("2", "b.org"),
            0.75,
            0.80,
            Some(0),
            None,
            VerificationStatus::Verified,
        );
        let v3 = VerificationResult::new(
            sample_candidate("3", "c.org"),
            0.55,
            0.70,
            Some(0),
            None,
            VerificationStatus::BelowThreshold,
        );
        let v4 = VerificationResult::new(
            sample_candidate("4", "d.org"),
            0.0,
            0.0,
            None,
            None,
            VerificationStatus::NoFace,
        );

        let canonical = vec![v1.clone(), v2.clone(), v3.clone(), v4.clone()];
        let expected_ranks = ranker.rank_results(canonical);

        // Test all permutations of 4 items (24 permutations)
        let mut items = vec![v1, v2, v3, v4];
        use std::collections::BTreeSet;
        let mut seen_orders = BTreeSet::new();

        // Helper permutation
        fn permute(
            items: &mut Vec<VerificationResult>,
            k: usize,
            ranker: &CandidateRanker,
            seen: &mut BTreeSet<Vec<String>>,
        ) {
            if k == 1 {
                let ranked = ranker.rank_results(items.clone());
                let order: Vec<String> = ranked
                    .iter()
                    .map(|r| r.candidate().url.as_str().to_string())
                    .collect();
                seen.insert(order);
                return;
            }
            for i in 0..k {
                items.swap(i, k - 1);
                permute(items, k - 1, ranker, seen);
                items.swap(i, k - 1);
            }
        }

        permute(&mut items, 4, &ranker, &mut seen_orders);

        // Exactly one unique ordering must exist across all 24 permutations!
        assert_eq!(
            seen_orders.len(),
            1,
            "all permutations must yield the exact same deterministic ordering"
        );

        let expected_order: Vec<String> = expected_ranks
            .iter()
            .map(|r| r.candidate().url.as_str().to_string())
            .collect();
        assert_eq!(seen_orders.into_iter().next().unwrap(), expected_order);
    }

    #[test]
    fn test_deterministic_tie_breakers() {
        let ranker = CandidateRanker::new();

        // 1. Tie on ranking_score: higher face_similarity wins
        let weights = RankingWeights::new(0.50, 0.25, 0.25, 0.0).unwrap();
        let tie_ranker = CandidateRanker::with_weights(weights);

        let in_a = CandidateRankingInput {
            verification: VerificationResult::new(
                sample_candidate("a", "same-score-a.com"),
                0.80,
                0.40,
                Some(0),
                None,
                VerificationStatus::Verified,
            ),
            source_relevance: Some(0.0),
            image_quality: Some(0.0),
        };
        let in_b = CandidateRankingInput {
            verification: VerificationResult::new(
                sample_candidate("b", "same-score-b.com"),
                0.60,
                0.80,
                Some(0),
                None,
                VerificationStatus::Verified,
            ),
            source_relevance: Some(0.0),
            image_quality: Some(0.0),
        };

        let ranked = tie_ranker.rank_inputs(vec![in_b.clone(), in_a.clone()]);
        assert_eq!(ranked[0].source(), "same-score-a.com", "higher similarity must break tie");
        assert_eq!(ranked[0].rank(), 1);
        assert_eq!(ranked[1].rank(), 2);

        // 2. Tie on score AND similarity: higher quality breaks tie
        let in_c = CandidateRankingInput {
            verification: VerificationResult::new(
                sample_candidate("c", "tie-quality-c.com"),
                0.80,
                0.75,
                Some(0),
                None,
                VerificationStatus::Verified,
            ),
            source_relevance: Some(0.5),
            image_quality: Some(0.9),
        };
        let in_d = CandidateRankingInput {
            verification: VerificationResult::new(
                sample_candidate("d", "tie-quality-d.com"),
                0.80,
                0.60,
                Some(0),
                None,
                VerificationStatus::Verified,
            ),
            source_relevance: Some(0.5),
            image_quality: Some(0.5),
        };
        // Quality score of c > d
        let ranked_cd = ranker.rank_inputs(vec![in_d.clone(), in_c.clone()]);
        assert_eq!(ranked_cd[0].source(), "tie-quality-c.com");

        // 3. Absolute identical scores across all fields: URL lexicographical tie-breaker
        let in_e = CandidateRankingInput {
            verification: VerificationResult::new(
                sample_candidate("001", "alpha.org"),
                0.85,
                0.85,
                Some(0),
                None,
                VerificationStatus::Verified,
            ),
            source_relevance: Some(0.5),
            image_quality: Some(0.5),
        };
        let in_f = CandidateRankingInput {
            verification: VerificationResult::new(
                sample_candidate("002", "beta.org"),
                0.85,
                0.85,
                Some(0),
                None,
                VerificationStatus::Verified,
            ),
            source_relevance: Some(0.5),
            image_quality: Some(0.5),
        };

        let ranked_ef = ranker.rank_inputs(vec![in_f.clone(), in_e.clone()]);
        // alpha.org < beta.org lexicographically
        assert_eq!(ranked_ef[0].source(), "alpha.org");
        assert_eq!(ranked_ef[1].source(), "beta.org");
    }
}
