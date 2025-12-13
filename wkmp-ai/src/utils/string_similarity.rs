//! String similarity utilities for metadata matching
//!
//! Provides Jaro-Winkler similarity and string normalization for
//! comparing artist/title metadata against MusicBrainz records.
//!
//! ## Ported from
//! `examples/am29/matching/validation.rs` as part of PLAN026
//!
//! ## Key Functions
//! - [`normalize_for_comparison`]: Normalize strings for fuzzy matching
//! - [`jaro_winkler_similarity`]: Compare two strings with normalization
//! - [`best_similarity`]: Find best match from multiple source variants

use strsim::jaro_winkler;

/// Normalize a string for comparison purposes
///
/// Removes common variations like article prefixes, punctuation, and normalizes case.
/// This allows fuzzy matching of names that differ only in formatting.
///
/// # Transformations
/// - Lowercase conversion
/// - Remove prefixes: "The ", "A ", "An "
/// - Remove conjunctions: " & ", " and "
/// - Remove punctuation: ' " ! . , -
/// - Unicode folding: ö→o, ü→u, é→e, ñ→n, etc.
/// - Collapse multiple spaces
///
/// # Examples
/// ```
/// use wkmp_ai::utils::string_similarity::normalize_for_comparison;
///
/// assert_eq!(normalize_for_comparison("The Beatles"), "beatles");
/// assert_eq!(normalize_for_comparison("Bob Marley & The Wailers"), "bob marley wailers");
/// assert_eq!(normalize_for_comparison("P!nk"), "pnk");
/// assert_eq!(normalize_for_comparison("Björk"), "bjork");
/// ```
pub fn normalize_for_comparison(s: &str) -> String {
    let mut normalized = s.to_lowercase();

    // Remove common article prefixes
    for prefix in &["the ", "a ", "an "] {
        if normalized.starts_with(prefix) {
            normalized = normalized[prefix.len()..].to_string();
        }
    }

    // Replace common conjunctions and punctuation
    normalized = normalized
        .replace(" & ", " ")
        .replace(" and ", " ")
        .replace('\'', "")
        .replace('"', "")
        .replace('!', "")
        .replace('.', "")
        .replace(',', "")
        .replace('-', " ");

    // Normalize unicode characters (basic ASCII folding)
    normalized = normalized
        .replace('ö', "o")
        .replace('ø', "o")
        .replace('ó', "o")
        .replace('ò', "o")
        .replace('ü', "u")
        .replace('ú', "u")
        .replace('ù', "u")
        .replace('ä', "a")
        .replace('á', "a")
        .replace('à', "a")
        .replace('é', "e")
        .replace('è', "e")
        .replace('ë', "e")
        .replace('í', "i")
        .replace('ì', "i")
        .replace('ï', "i")
        .replace('ñ', "n")
        .replace('ß', "ss");

    // Collapse multiple spaces
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    parts.join(" ")
}

/// Calculate Jaro-Winkler similarity between two strings
///
/// Normalizes both strings before comparison, then returns a value
/// from 0.0 (completely different) to 1.0 (identical).
///
/// Jaro-Winkler gives higher weight to prefix matches, making it
/// ideal for name variations where the beginning is usually preserved.
///
/// # Arguments
/// * `s1` - First string to compare
/// * `s2` - Second string to compare
///
/// # Returns
/// Similarity score from 0.0 to 1.0
///
/// # Examples
/// ```
/// use wkmp_ai::utils::string_similarity::jaro_winkler_similarity;
///
/// // Identical strings
/// let sim = jaro_winkler_similarity("Beatles", "Beatles");
/// assert!((sim - 1.0).abs() < 0.001);
///
/// // Similar with article prefix
/// let sim = jaro_winkler_similarity("The Beatles", "Beatles");
/// assert!((sim - 1.0).abs() < 0.001); // Normalized, so identical
///
/// // Different strings
/// let sim = jaro_winkler_similarity("Beatles", "Rolling Stones");
/// assert!(sim < 0.7);
/// ```
pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f64 {
    let norm1 = normalize_for_comparison(s1);
    let norm2 = normalize_for_comparison(s2);
    jaro_winkler(&norm1, &norm2)
}

/// Calculate Jaro-Winkler similarity without normalization
///
/// Useful when strings are already normalized or when you want
/// exact character-level comparison.
///
/// # Arguments
/// * `s1` - First string to compare (will be lowercased only)
/// * `s2` - Second string to compare (will be lowercased only)
///
/// # Returns
/// Similarity score from 0.0 to 1.0
pub fn jaro_winkler_raw(s1: &str, s2: &str) -> f64 {
    jaro_winkler(&s1.to_lowercase(), &s2.to_lowercase())
}

/// Find best similarity from candidate to any source variant
///
/// Returns the highest Jaro-Winkler similarity across all source variants.
/// Uses normalized comparison for consistent matching.
///
/// # Arguments
/// * `candidate` - The string to compare (e.g., MusicBrainz artist name)
/// * `sources` - List of source strings to compare against (e.g., ID3 tag variants)
///
/// # Returns
/// Best (highest) similarity score from 0.0 to 1.0
///
/// # Examples
/// ```
/// use wkmp_ai::utils::string_similarity::best_similarity;
///
/// let sim = best_similarity(
///     "The Beatles",
///     &["Beatles".to_string(), "The Beatles".to_string()]
/// );
/// assert!((sim - 1.0).abs() < 0.001);
///
/// // Returns 0.0 for empty sources
/// let sim = best_similarity("Beatles", &[]);
/// assert!((sim - 0.0).abs() < 0.001);
/// ```
pub fn best_similarity(candidate: &str, sources: &[String]) -> f64 {
    if sources.is_empty() {
        return 0.0;
    }
    sources
        .iter()
        .map(|s| jaro_winkler_similarity(candidate, s))
        .fold(0.0_f64, |a, b| a.max(b))
}

/// Find best raw Jaro-Winkler similarity without normalization
///
/// Returns the highest similarity across all source variants using
/// only lowercase comparison (no article removal, unicode folding, etc.).
///
/// This is useful for detecting completely different character sets
/// (e.g., Latin vs Japanese) which should return near-zero similarity.
///
/// # Arguments
/// * `candidate` - The string to compare
/// * `sources` - List of source strings to compare against
///
/// # Returns
/// Best (highest) raw similarity score from 0.0 to 1.0
pub fn best_similarity_raw(candidate: &str, sources: &[String]) -> f64 {
    if sources.is_empty() {
        return 0.0;
    }
    sources
        .iter()
        .map(|s| jaro_winkler_raw(candidate, s))
        .fold(0.0_f64, |a, b| a.max(b))
}

/// Check if two strings match with optional substring containment
///
/// Returns (similarity, is_acceptable) where is_acceptable is true if:
/// - Jaro-Winkler similarity >= threshold, OR
/// - One normalized string contains the other (substring match)
///
/// # Arguments
/// * `source` - Source string (e.g., from ID3 tags)
/// * `candidate` - Candidate string (e.g., from MusicBrainz)
/// * `threshold` - Minimum similarity for direct match (typically 0.85)
///
/// # Returns
/// Tuple of (similarity_score, is_acceptable_match)
pub fn verify_match(source: &str, candidate: &str, threshold: f64) -> (f64, bool) {
    let normalized_source = normalize_for_comparison(source);
    let normalized_candidate = normalize_for_comparison(candidate);

    let similarity = jaro_winkler(&normalized_source, &normalized_candidate);

    // Also check substring containment
    let is_substring = normalized_source.contains(&normalized_candidate)
        || normalized_candidate.contains(&normalized_source);

    let is_acceptable = similarity >= threshold || is_substring;

    (similarity, is_acceptable)
}

/// Default threshold for title similarity matching
pub const TITLE_SIMILARITY_THRESHOLD: f64 = 0.85;

/// Default threshold for artist similarity matching
pub const ARTIST_SIMILARITY_THRESHOLD: f64 = 0.80;

#[cfg(test)]
mod tests {
    use super::*;

    // TC-U-VAL-020-01: Test removal of "The " prefix
    #[test]
    fn test_normalize_removes_the_prefix() {
        assert_eq!(normalize_for_comparison("The Beatles"), "beatles");
        assert_eq!(normalize_for_comparison("the rolling stones"), "rolling stones");
        assert_eq!(normalize_for_comparison("A Hard Day's Night"), "hard days night");
        assert_eq!(normalize_for_comparison("An American Prayer"), "american prayer");
    }

    // TC-U-VAL-020-02: Test Unicode folding
    #[test]
    fn test_normalize_unicode_folding() {
        assert_eq!(normalize_for_comparison("Björk"), "bjork");
        assert_eq!(normalize_for_comparison("Mötley Crüe"), "motley crue");
        assert_eq!(normalize_for_comparison("Sigur Rós"), "sigur ros");
        assert_eq!(normalize_for_comparison("Héloïse"), "heloise");
        assert_eq!(normalize_for_comparison("Señorita"), "senorita");
        assert_eq!(normalize_for_comparison("Strauß"), "strauss");
    }

    #[test]
    fn test_normalize_conjunctions() {
        // Note: "The" is only removed at start, not in middle after "&" removal
        assert_eq!(
            normalize_for_comparison("Bob Marley & The Wailers"),
            "bob marley the wailers"
        );
        assert_eq!(
            normalize_for_comparison("Earth, Wind and Fire"),
            "earth wind fire"
        );
        // But "The" at start is removed
        assert_eq!(
            normalize_for_comparison("The Who"),
            "who"
        );
    }

    #[test]
    fn test_normalize_punctuation() {
        assert_eq!(normalize_for_comparison("P!nk"), "pnk");
        assert_eq!(normalize_for_comparison("Guns N' Roses"), "guns n roses");
        assert_eq!(normalize_for_comparison("AC/DC"), "ac/dc"); // Slash not removed
        assert_eq!(normalize_for_comparison("Dr. Dre"), "dr dre");
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(
            normalize_for_comparison("  Multiple   Spaces  "),
            "multiple spaces"
        );
        assert_eq!(normalize_for_comparison("Tab\tHere"), "tab here");
    }

    // TC-U-VAL-010-01: Test returns value in 0.0-1.0 range
    #[test]
    fn test_jaro_winkler_range() {
        let test_pairs = [
            ("identical", "identical"),
            ("Beatles", "Rolling Stones"),
            ("a", "z"),
            ("", "nonempty"),
            ("short", "very long string with many words"),
        ];

        for (s1, s2) in test_pairs {
            let sim = jaro_winkler_similarity(s1, s2);
            assert!(
                (0.0..=1.0).contains(&sim),
                "Similarity {} for ({}, {}) not in range 0.0-1.0",
                sim,
                s1,
                s2
            );
        }
    }

    // TC-U-VAL-010-02: Test similar strings return > 0.85
    #[test]
    fn test_jaro_winkler_similar_strings() {
        // Identical after normalization
        let sim = jaro_winkler_similarity("The Beatles", "Beatles");
        assert!(
            sim > 0.99,
            "Expected > 0.99 for normalized identical strings, got {}",
            sim
        );

        // Very similar
        let sim = jaro_winkler_similarity("Bohemian Rhapsody", "Bohemian Rhapsody - Remastered");
        assert!(
            sim > 0.85,
            "Expected > 0.85 for similar strings, got {}",
            sim
        );

        // Different character sets should have low similarity
        let sim = jaro_winkler_raw("Fluke", "ゆいかおり");
        assert!(
            sim < 0.5,
            "Expected < 0.5 for different character sets, got {}",
            sim
        );
    }

    #[test]
    fn test_jaro_winkler_identical() {
        let sim = jaro_winkler_similarity("Beatles", "Beatles");
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_jaro_winkler_with_normalization() {
        // "The Beatles" normalized to "beatles" should match "beatles"
        let sim = jaro_winkler_similarity("The Beatles", "beatles");
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_best_similarity_empty_sources() {
        let sim = best_similarity("Beatles", &[]);
        assert!((sim - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_best_similarity_finds_best() {
        let sources = vec![
            "Rolling Stones".to_string(),
            "The Beatles".to_string(),
            "Led Zeppelin".to_string(),
        ];
        let sim = best_similarity("Beatles", &sources);
        // Should match "The Beatles" after normalization
        assert!(sim > 0.99, "Expected > 0.99, got {}", sim);
    }

    #[test]
    fn test_verify_match_threshold() {
        let (sim, acceptable) = verify_match("Beatles", "The Beatles", 0.85);
        assert!(acceptable);
        assert!(sim > 0.99);

        let (sim, acceptable) = verify_match("Beatles", "Led Zeppelin", 0.85);
        assert!(!acceptable);
        assert!(sim < 0.85);
    }

    #[test]
    fn test_verify_match_substring() {
        // "Greatest Hits" is contained in normalized form
        let (_, acceptable) = verify_match("Greatest Hits", "Greatest Hits Vol. 1", 0.85);
        assert!(
            acceptable,
            "Substring containment should make match acceptable"
        );
    }

    #[test]
    fn test_thresholds_defined() {
        assert!((TITLE_SIMILARITY_THRESHOLD - 0.85).abs() < 0.001);
        assert!((ARTIST_SIMILARITY_THRESHOLD - 0.80).abs() < 0.001);
    }
}
