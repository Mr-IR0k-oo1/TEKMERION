//! URL validation, result normalization, deduplication, and deterministic ordering.

use std::collections::HashMap;

use chrono::Utc;
use tekmerion_core::SearchCandidate;
use url::Url;

use crate::error::DiscoveryError;
use crate::provider::RawCandidate;

/// Validate and normalize a web URL.
///
/// Requires HTTP or HTTPS scheme, non-empty host, and normalizes standard ports,
/// empty paths, and removes fragments.
pub fn validate_and_normalize_url(raw: &str) -> Result<Url, DiscoveryError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DiscoveryError::InvalidUrl {
            url: raw.to_string(),
            reason: "URL must not be empty".to_string(),
        });
    }

    let mut parsed = Url::parse(trimmed).map_err(|e| DiscoveryError::InvalidUrl {
        url: raw.to_string(),
        reason: format!("failed to parse URL: {}", e),
    })?;

    // Scheme validation: only http and https are permitted
    let scheme = parsed.scheme().to_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(DiscoveryError::InvalidUrl {
            url: raw.to_string(),
            reason: format!("unsupported scheme '{}'; only http and https are allowed", scheme),
        });
    }

    // Host validation
    let host = parsed.host_str().unwrap_or("").trim();
    if host.is_empty() {
        return Err(DiscoveryError::InvalidUrl {
            url: raw.to_string(),
            reason: "URL must contain a valid, non-empty host".to_string(),
        });
    }

    // Normalization: strip fragment
    parsed.set_fragment(None);

    // Normalization: strip default ports
    if (parsed.scheme() == "http" && parsed.port() == Some(80))
        || (parsed.scheme() == "https" && parsed.port() == Some(443))
    {
        let _ = parsed.set_port(None);
    }

    // Normalization: strip leading "www." from host
    let stripped_host = parsed
        .host_str()
        .and_then(|h| h.strip_prefix("www.").map(|s| s.to_string()));
    if let Some(host) = stripped_host {
        let _ = parsed.set_host(Some(&host));
    }

    Ok(parsed)
}

/// Helper to optionally parse and validate an image or thumbnail URL.
fn validate_optional_url(raw: Option<String>) -> Option<Url> {
    raw.and_then(|s| validate_and_normalize_url(&s).ok())
}

/// Normalize an optional string: trim whitespace, and return `None` if empty.
fn clean_optional_str(raw: Option<String>) -> Option<String> {
    raw.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Extract and normalize a domain name (lowercase, strip leading "www.").
pub fn normalize_domain(url: &Url, explicit_domain: Option<String>) -> String {
    if let Some(d) = clean_optional_str(explicit_domain) {
        let lower = d.to_lowercase();
        return lower.trim_start_matches("www.").to_string();
    }

    url.host_str()
        .unwrap_or_default()
        .trim_start_matches("www.")
        .to_lowercase()
}

/// Normalize a raw candidate into a domain `SearchCandidate` with provider attribution.
pub fn normalize_candidate(
    raw: RawCandidate,
    provider_id: &str,
) -> Result<SearchCandidate, DiscoveryError> {
    let url = validate_and_normalize_url(&raw.url)?;
    let domain = normalize_domain(&url, raw.domain);
    let title = clean_optional_str(raw.title);
    let snippet = clean_optional_str(raw.snippet);
    let image_url = validate_optional_url(raw.image_url);
    let thumbnail_url = validate_optional_url(raw.thumbnail_url);
    let provider = provider_id.trim().to_string();

    Ok(SearchCandidate {
        url,
        title,
        domain,
        image_url,
        thumbnail_url,
        snippet,
        provider,
        discovered_at: Utc::now(),
    })
}

/// Deduplicate, deterministically sort, and limit search candidates.
///
/// - **Deduplication**: Deduplicates based on canonical URL. When duplicates occur,
///   merges richer metadata (title, snippet, image_url, thumbnail_url).
/// - **Deterministic Ordering**: Sorts candidates deterministically by `(domain, url, title)`.
/// - **Candidate Limit**: Truncates output to `max_candidates`.
pub fn process_candidates(
    candidates: impl IntoIterator<Item = SearchCandidate>,
    max_candidates: usize,
) -> Vec<SearchCandidate> {
    let mut dedup_map: HashMap<String, SearchCandidate> = HashMap::new();

    for candidate in candidates {
        let key = candidate.url.as_str().to_string();
        dedup_map
            .entry(key)
            .and_modify(|existing| {
                // Merge richer metadata if available
                if existing.title.is_none() && candidate.title.is_some() {
                    existing.title = candidate.title.clone();
                }
                if existing.snippet.is_none() && candidate.snippet.is_some() {
                    existing.snippet = candidate.snippet.clone();
                }
                if existing.image_url.is_none() && candidate.image_url.is_some() {
                    existing.image_url = candidate.image_url.clone();
                }
                if existing.thumbnail_url.is_none() && candidate.thumbnail_url.is_some() {
                    existing.thumbnail_url = candidate.thumbnail_url.clone();
                }
            })
            .or_insert(candidate);
    }

    let mut result: Vec<SearchCandidate> = dedup_map.into_values().collect();

    // Deterministic ordering: sort by domain, then URL string, then title
    result.sort_by(|a, b| {
        a.domain
            .cmp(&b.domain)
            .then_with(|| a.url.as_str().cmp(b.url.as_str()))
            .then_with(|| a.title.cmp(&b.title))
    });

    if max_candidates > 0 && result.len() > max_candidates {
        result.truncate(max_candidates);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_valid_http_and_https_urls() {
        let url1 = validate_and_normalize_url("https://example.com/photo").unwrap();
        assert_eq!(url1.as_str(), "https://example.com/photo");

        let url2 = validate_and_normalize_url("http://photos.example.org:80/face").unwrap();
        // Port 80 should be stripped for http
        assert_eq!(url2.as_str(), "http://photos.example.org/face");

        let url3 = validate_and_normalize_url("https://archive.example.net:443/doc#section").unwrap();
        // Port 443 and fragment should be stripped
        assert_eq!(url3.as_str(), "https://archive.example.net/doc");
    }

    #[test]
    fn rejects_invalid_schemes() {
        assert!(validate_and_normalize_url("ftp://example.com").is_err());
        assert!(validate_and_normalize_url("javascript:alert(1)").is_err());
        assert!(validate_and_normalize_url("file:///tmp/face.jpg").is_err());
        assert!(validate_and_normalize_url("data:image/png;base64,...").is_err());
        assert!(validate_and_normalize_url("/relative/path").is_err());
        assert!(validate_and_normalize_url("").is_err());
    }

    #[test]
    fn normalizes_domain_and_strips_www() {
        let url = Url::parse("https://www.example.com/path").unwrap();
        assert_eq!(normalize_domain(&url, None), "example.com");

        assert_eq!(
            normalize_domain(&url, Some("www.Sub.Example.Com".to_string())),
            "sub.example.com"
        );
    }

    #[test]
    fn deduplicates_and_merges_metadata() {
        let c1 = SearchCandidate {
            url: Url::parse("https://example.com/page").unwrap(),
            title: Some("Title 1".to_string()),
            domain: "example.com".to_string(),
            image_url: None,
            thumbnail_url: None,
            snippet: None,
            provider: "provider_a".to_string(),
            discovered_at: Utc::now(),
        };

        let c2 = SearchCandidate {
            url: Url::parse("https://example.com/page").unwrap(),
            title: None,
            domain: "example.com".to_string(),
            image_url: Some(Url::parse("https://example.com/img.jpg").unwrap()),
            thumbnail_url: Some(Url::parse("https://example.com/thumb.jpg").unwrap()),
            snippet: Some("Snippet text".to_string()),
            provider: "provider_b".to_string(),
            discovered_at: Utc::now(),
        };

        let result = process_candidates(vec![c1, c2], 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title.as_deref(), Some("Title 1"));
        assert_eq!(result[0].snippet.as_deref(), Some("Snippet text"));
        assert!(result[0].image_url.is_some());
        assert!(result[0].thumbnail_url.is_some());
    }

    #[test]
    fn enforces_candidate_limit() {
        let mut candidates = Vec::new();
        for i in 0..15 {
            candidates.push(SearchCandidate {
                url: Url::parse(&format!("https://example.com/page_{:02}", i)).unwrap(),
                title: Some(format!("Page {}", i)),
                domain: "example.com".to_string(),
                image_url: None,
                thumbnail_url: None,
                snippet: None,
                provider: "test".to_string(),
                discovered_at: Utc::now(),
            });
        }

        let limited = process_candidates(candidates, 5);
        assert_eq!(limited.len(), 5);
    }

    #[test]
    fn deterministic_ordering_is_reproducible() {
        let c_b = SearchCandidate {
            url: Url::parse("https://b.example.org/").unwrap(),
            title: Some("B".to_string()),
            domain: "b.example.org".to_string(),
            image_url: None,
            thumbnail_url: None,
            snippet: None,
            provider: "test".to_string(),
            discovered_at: Utc::now(),
        };
        let c_a = SearchCandidate {
            url: Url::parse("https://a.example.org/").unwrap(),
            title: Some("A".to_string()),
            domain: "a.example.org".to_string(),
            image_url: None,
            thumbnail_url: None,
            snippet: None,
            provider: "test".to_string(),
            discovered_at: Utc::now(),
        };

        let res1 = process_candidates(vec![c_b.clone(), c_a.clone()], 10);
        let res2 = process_candidates(vec![c_a, c_b], 10);

        assert_eq!(res1[0].domain, "a.example.org");
        assert_eq!(res1[1].domain, "b.example.org");
        assert_eq!(res1, res2);
    }
}
