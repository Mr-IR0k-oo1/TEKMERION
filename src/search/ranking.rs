//! Search ranking module

use crate::search::models::SearchCandidate;
use std::collections::HashMap;
use tracing::info;

/// Rank search candidates
pub fn rank_candidates(candidates: Vec<SearchCandidate>) -> Vec<SearchCandidate> {
    info!("Ranking {} candidates", candidates.len());

    // Simple ranking based on domain authority and snippet length
    let mut scored_candidates: Vec<(f32, SearchCandidate)> = candidates
        .into_iter()
        .map(|candidate| {
            let mut score = 0.0;

            // Domain authority score
            let domain_score = match candidate.domain.as_str() {
                "wikipedia.org" => 1.0,
                "imdb.com" => 0.9,
                "twitter.com" => 0.8,
                "instagram.com" => 0.8,
                "facebook.com" => 0.7,
                _ => 0.5,
            };
            score += domain_score;

            // Snippet length score
            if let Some(snippet) = &candidate.snippet {
                score += snippet.len() as f32 / 100.0;
            }

            // Image presence score
            if candidate.image_url.is_some() {
                score += 0.2;
            }

            (score, candidate)
        })
        .collect();

    // Sort by score in descending order
    scored_candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    // Extract the ranked candidates
    scored_candidates.into_iter().map(|(_, candidate)| candidate).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ranking() {
        let candidates = vec![
            SearchCandidate {
                title: "Test 1".to_string(),
                url: "https://example.com/1".to_string(),
                domain: "example.com".to_string(),
                thumbnail_url: None,
                image_url: None,
                snippet: Some("Short snippet".to_string()),
            },
            SearchCandidate {
                title: "Test 2".to_string(),
                url: "https://wikipedia.org/2".to_string(),
                domain: "wikipedia.org".to_string(),
                thumbnail_url: None,
                image_url: Some("https://example.com/image.jpg".to_string()),
                snippet: Some("Longer snippet with more information".to_string()),
            },
        ];

        let ranked = rank_candidates(candidates);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].domain, "wikipedia.org");
    }
}
