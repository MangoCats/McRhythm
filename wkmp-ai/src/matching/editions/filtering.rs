//! Edition Filtering and Sorting
//!
//! **[PLAN030]** Provides filtering and sorting of editions by name distance
//! to prioritize editions most likely to be the correct match.

use std::collections::HashSet;
use strsim::{jaro_winkler, normalized_levenshtein};

use crate::matching::constants::VARIOUS_ARTISTS;
use crate::matching::types::Edition;

/// Filter and sort editions by relevance to source metadata
///
/// Calculates name distance score for each edition and returns
/// top N editions sorted by relevance.
///
/// # Arguments
/// * `editions` - Vector of editions to filter
/// * `source_artist` - Artist name from source file metadata
/// * `source_album` - Album name from source file metadata
/// * `max_editions` - Maximum number of editions to return
///
/// # Returns
/// Filtered and sorted vector of editions
pub fn filter_and_sort_editions(
    editions: Vec<Edition>,
    source_artist: &str,
    source_album: &str,
    max_editions: usize,
) -> Vec<Edition> {
    let mut scored: Vec<(Edition, f64)> = editions
        .into_iter()
        .map(|mut edition| {
            let score = calculate_name_distance(
                &edition.artist,
                &edition.title,
                source_artist,
                source_album,
            );
            edition.name_distance_score = Some(score);
            (edition, score)
        })
        .collect();

    // Sort by name distance (higher = better match)
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Assign ranks
    scored
        .iter_mut()
        .enumerate()
        .for_each(|(idx, (edition, _))| {
            edition.name_distance_rank = Some(idx + 1);
        });

    // Take top N
    scored
        .into_iter()
        .take(max_editions)
        .map(|(edition, _)| edition)
        .collect()
}

/// Filter out editions with impossible total durations
///
/// **[BUG FIX]** Rejects editions whose total duration significantly exceeds
/// actual file duration. Prevents matching 59-minute files to 7-hour box sets.
///
/// Uses generous threshold (125%) to allow for:
/// - Hidden tracks, bonus content, silence padding
/// - Rounding errors in MusicBrainz durations
/// - Audio file metadata inaccuracies
///
/// # Threshold Rationale
/// - <85%: Edition too short (reject - missing tracks or wrong file)
/// - 85-125%: Acceptable range (hidden tracks, padding, metadata errors)
/// - >125%: Impossible (reject - wrong edition, e.g., box set vs standard album)
///
/// # Arguments
/// * `editions` - Candidate editions to filter
/// * `file_duration_ms` - Actual audio file duration in milliseconds
///
/// # Returns
/// Filtered editions within feasible duration range
pub fn filter_editions_by_file_duration(
    editions: Vec<Edition>,
    file_duration_ms: u64,
) -> Vec<Edition> {
    const MIN_DURATION_RATIO: f64 = 0.85; // 85% - allow for truncated files
    const MAX_DURATION_RATIO: f64 = 1.25; // 125% - allow for bonus content

    editions
        .into_iter()
        .filter(|edition| {
            let edition_total_ms: u64 = edition.durations.iter().map(|&x| x as u64).sum();

            // Skip editions with zero or missing durations
            if edition_total_ms == 0 {
                return false;
            }

            let ratio = edition_total_ms as f64 / file_duration_ms as f64;

            // Accept if within feasible range
            ratio >= MIN_DURATION_RATIO && ratio <= MAX_DURATION_RATIO
        })
        .collect()
}

/// Calculate artist similarity using hybrid token-based and character-based matching
///
/// **[ARTIST FILTERING]** Uses Jaccard similarity (token-based) combined with
/// normalized Levenshtein distance (character-based) to determine if two artist
/// names refer to the same artist. This approach is superior to pure Jaro-Winkler
/// for artist matching because:
///
/// - Token-based matching handles "The Beatles" vs "Beatles" correctly
/// - Levenshtein handles typos and minor variations
/// - Completely different artists get low scores (e.g., "The Cars" vs "Stephan Mathieu")
///
/// # Algorithm
/// 1. Tokenize both artist names (split on whitespace, lowercase)
/// 2. Calculate Jaccard similarity: |intersection| / |union|
/// 3. Calculate normalized Levenshtein distance
/// 4. Return maximum of the two (if EITHER metric is high, artists likely match)
///
/// # Arguments
/// * `mb_artist` - Artist name from MusicBrainz
/// * `source_artist` - Artist name from source file metadata
///
/// # Returns
/// Similarity score (0.0-1.0) where 1.0 = identical, 0.0 = completely different
///
/// # Examples
/// ```ignore
/// calculate_artist_similarity("The Cars", "Cars") → 0.67 (Jaccard: 0.50, Levenshtein: 0.67)
/// calculate_artist_similarity("The Cars", "Stephan Mathieu") → 0.16 (both metrics low)
/// calculate_artist_similarity("The Beatles", "Beatles") → 0.64 (Jaccard: 0.50, Levenshtein: 0.64)
/// calculate_artist_similarity("Led Zeppelin", "Led Zepelin") → 0.95 (typo handled)
/// ```
fn calculate_artist_similarity(mb_artist: &str, source_artist: &str) -> f64 {
    let mb_lower = mb_artist.to_lowercase();
    let src_lower = source_artist.to_lowercase();

    // Token-based similarity (Jaccard)
    let mb_tokens: HashSet<&str> = mb_lower.split_whitespace().collect();
    let src_tokens: HashSet<&str> = src_lower.split_whitespace().collect();

    let intersection = mb_tokens.intersection(&src_tokens).count();
    let union = mb_tokens.union(&src_tokens).count();

    let jaccard_sim = if union > 0 {
        intersection as f64 / union as f64
    } else {
        0.0
    };

    // Character-based similarity (normalized Levenshtein)
    let levenshtein_sim = normalized_levenshtein(&mb_lower, &src_lower);

    // Return maximum: if EITHER metric shows high similarity, artists likely match
    // This handles both "The X" vs "X" (Jaccard) and typos (Levenshtein)
    jaccard_sim.max(levenshtein_sim)
}

/// Filter out editions with clearly incorrect artists
///
/// **[ARTIST FILTERING]** Rejects editions whose artist name differs significantly
/// from source metadata based on hybrid token + character similarity. This prevents
/// wrong artists from being matched even if duration/quality scores are good.
///
/// Uses Jaccard + Levenshtein similarity (not Jaro-Winkler) for better semantic
/// matching at the artist name level.
///
/// # Special Cases
/// - **Various Artists:** Always passes filter for any source artist (compilations
///   can contain any artist's work)
/// - **Empty/Unknown:** Passes filter to allow manual review
///
/// # Arguments
/// * `editions` - Candidate editions to filter
/// * `source_artist` - Artist name from source file metadata
/// * `min_similarity` - Minimum similarity threshold (0.0-1.0), typically 0.60
///
/// # Returns
/// Filtered editions with artist similarity >= min_similarity OR special cases
///
/// # Example
/// ```ignore
/// // Reject editions where artist similarity < 0.60
/// let filtered = filter_editions_by_artist(editions, "The Cars", 0.60);
/// // "Stephan Mathieu" filtered out (similarity ~0.16 < 0.60)
/// // "The Cars" passes (similarity 1.00)
/// // "Cars" passes (similarity ~0.67 > 0.60)
/// // "Various Artists" passes (special case)
/// ```
pub fn filter_editions_by_artist(
    editions: Vec<Edition>,
    source_artist: &str,
    min_similarity: f64,
) -> Vec<Edition> {
    editions
        .into_iter()
        .filter(|edition| {
            let mb_artist = &edition.artist;

            // Special case: "Various Artists" always passes (compilations can contain any artist)
            if mb_artist.eq_ignore_ascii_case(VARIOUS_ARTISTS) {
                return true;
            }

            // Special case: Empty or unknown source artist passes (allow manual review)
            if source_artist.trim().is_empty() || source_artist.eq_ignore_ascii_case("unknown") {
                return true;
            }

            // Calculate hybrid similarity and check threshold
            let similarity = calculate_artist_similarity(mb_artist, source_artist);
            similarity >= min_similarity
        })
        .collect()
}

/// Calculate combined name distance score
///
/// Uses Jaro-Winkler similarity for both artist and album names,
/// with artist weighted slightly higher (60/40 split).
///
/// # Arguments
/// * `mb_artist` - Artist name from MusicBrainz
/// * `mb_album` - Album name from MusicBrainz
/// * `source_artist` - Artist name from source file
/// * `source_album` - Album name from source file
///
/// # Returns
/// Combined similarity score (0.0 to 1.0)
pub fn calculate_name_distance(
    mb_artist: &str,
    mb_album: &str,
    source_artist: &str,
    source_album: &str,
) -> f64 {
    let artist_sim = jaro_winkler(&mb_artist.to_lowercase(), &source_artist.to_lowercase());
    let album_sim = jaro_winkler(&mb_album.to_lowercase(), &source_album.to_lowercase());

    // Weight artist slightly higher
    artist_sim * 0.6 + album_sim * 0.4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_edition(artist: &str, title: &str, track_count: usize) -> Edition {
        Edition {
            release_mbid: "test-mbid".to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            artist_credit: None,
            country: None,
            status: None,
            track_count,
            track_durations: vec![180.0; track_count],
            recording_mbids: Vec::new(),
            name_distance_rank: None,
            name_distance_score: None,
            durations: vec![180000; track_count],
            track_titles: (1..=track_count).map(|i| format!("Track {}", i)).collect(),
        }
    }

    #[test]
    fn test_calculate_name_distance_exact() {
        let score =
            calculate_name_distance("The Beatles", "Abbey Road", "The Beatles", "Abbey Road");
        assert!(score > 0.99);
    }

    #[test]
    fn test_calculate_name_distance_case_insensitive() {
        let score =
            calculate_name_distance("THE BEATLES", "ABBEY ROAD", "the beatles", "abbey road");
        assert!(score > 0.99);
    }

    #[test]
    fn test_calculate_name_distance_partial() {
        let score = calculate_name_distance("The Beatles", "Abbey Road", "Beatles", "Abbey Road");
        // Should be high but not perfect
        assert!(score > 0.8 && score < 1.0);
    }

    #[test]
    fn test_calculate_name_distance_different() {
        let score = calculate_name_distance(
            "The Beatles",
            "Abbey Road",
            "Led Zeppelin",
            "Physical Graffiti",
        );
        // Should be low (but Jaro-Winkler may give higher scores for strings with common letters)
        // The key is it should be significantly lower than exact/partial matches
        assert!(score < 0.7);
    }

    #[test]
    fn test_filter_and_sort_ranking() {
        let editions = vec![
            create_test_edition("Led Zeppelin", "Physical Graffiti", 15),
            create_test_edition("The Beatles", "Abbey Road", 17),
            create_test_edition("Pink Floyd", "The Wall", 26),
        ];

        let filtered = filter_and_sort_editions(editions, "The Beatles", "Abbey Road", 10);

        // Beatles should be first (best match)
        assert_eq!(filtered[0].artist, "The Beatles");
        assert_eq!(filtered[0].name_distance_rank, Some(1));

        // All should have scores assigned
        for edition in &filtered {
            assert!(edition.name_distance_score.is_some());
            assert!(edition.name_distance_rank.is_some());
        }
    }

    #[test]
    fn test_filter_max_editions() {
        let editions = vec![
            create_test_edition("Artist 1", "Album 1", 10),
            create_test_edition("Artist 2", "Album 2", 10),
            create_test_edition("Artist 3", "Album 3", 10),
            create_test_edition("Artist 4", "Album 4", 10),
            create_test_edition("Artist 5", "Album 5", 10),
        ];

        let filtered = filter_and_sort_editions(editions, "Artist 1", "Album 1", 3);

        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].artist, "Artist 1"); // Best match first
    }

    #[test]
    fn test_filter_empty() {
        let editions: Vec<Edition> = vec![];
        let filtered = filter_and_sort_editions(editions, "Artist", "Album", 10);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_sorting_preserves_all_data() {
        let mut edition = create_test_edition("The Beatles", "Abbey Road", 17);
        edition.country = Some("UK".to_string());
        edition.status = Some("Official".to_string());

        let editions = vec![edition];
        let filtered = filter_and_sort_editions(editions, "The Beatles", "Abbey Road", 10);

        assert_eq!(filtered[0].country, Some("UK".to_string()));
        assert_eq!(filtered[0].status, Some("Official".to_string()));
        assert_eq!(filtered[0].track_count, 17);
    }

    // =============================================================================
    // Artist Similarity Tests (Jaccard + Levenshtein)
    // =============================================================================

    #[test]
    fn test_artist_similarity_exact_match() {
        let sim = calculate_artist_similarity("The Cars", "The Cars");
        assert!((sim - 1.0).abs() < 0.01, "Exact match should be 1.0, got {}", sim);
    }

    #[test]
    fn test_artist_similarity_with_the() {
        // "The Beatles" vs "Beatles" should have high similarity
        let sim = calculate_artist_similarity("The Beatles", "Beatles");
        // Jaccard: {the, beatles} ∩ {beatles} / {the, beatles} = 1/2 = 0.50
        // Levenshtein: normalized distance ~0.64
        // Max(0.50, 0.64) = 0.64
        assert!(sim >= 0.60, "Should handle 'The X' vs 'X', got {}", sim);
    }

    #[test]
    fn test_artist_similarity_completely_different() {
        // "The Cars" vs "Stephan Mathieu" should have LOW similarity
        let sim = calculate_artist_similarity("The Cars", "Stephan Mathieu");
        // Jaccard: no common tokens = 0.0
        // Levenshtein: very different strings ~0.16
        // Max(0.0, 0.16) = 0.16
        assert!(sim < 0.30, "Completely different artists should score low, got {}", sim);
    }

    #[test]
    fn test_artist_similarity_typo() {
        // "Led Zeppelin" vs "Led Zepelin" (missing 'p')
        let sim = calculate_artist_similarity("Led Zeppelin", "Led Zepelin");
        // Jaccard: {led, zeppelin} ∩ {led, zepelin} = {led}/union = ~0.33
        // Levenshtein: very similar (1 char difference) ~0.95
        // Max(0.33, 0.95) = 0.95
        assert!(sim >= 0.90, "Should handle typos, got {}", sim);
    }

    #[test]
    fn test_artist_similarity_case_insensitive() {
        let sim1 = calculate_artist_similarity("THE BEATLES", "the beatles");
        assert!((sim1 - 1.0).abs() < 0.01, "Case insensitive, got {}", sim1);

        let sim2 = calculate_artist_similarity("The Cars", "THE CARS");
        assert!((sim2 - 1.0).abs() < 0.01, "Case insensitive, got {}", sim2);
    }

    // =============================================================================
    // Artist Filtering Tests
    // =============================================================================

    #[test]
    fn test_filter_by_artist_exact_match() {
        let editions = vec![
            create_test_edition("The Cars", "Panorama", 10),
            create_test_edition("Stephan Mathieu", "Radioland", 1),
        ];
        let filtered = filter_editions_by_artist(editions, "The Cars", 0.60);

        assert_eq!(filtered.len(), 1, "Should filter out Stephan Mathieu");
        assert_eq!(filtered[0].artist, "The Cars");
    }

    #[test]
    fn test_filter_by_artist_with_the_prefix() {
        let editions = vec![
            create_test_edition("The Beatles", "Abbey Road", 17),
            create_test_edition("Beatles", "Abbey Road", 17),
            create_test_edition("Led Zeppelin", "Physical Graffiti", 15),
        ];
        let filtered = filter_editions_by_artist(editions, "The Beatles", 0.60);

        // Both "The Beatles" and "Beatles" should pass (similarity ~0.64 > 0.60)
        // "Led Zeppelin" should be filtered out
        assert_eq!(filtered.len(), 2, "Should keep both Beatles variants");
        assert!(filtered.iter().any(|e| e.artist == "The Beatles"));
        assert!(filtered.iter().any(|e| e.artist == "Beatles"));
    }

    #[test]
    fn test_filter_by_artist_completely_different() {
        let editions = vec![
            create_test_edition("The Cars", "Panorama", 10),
            create_test_edition("Pink Floyd", "The Wall", 26),
        ];
        let filtered = filter_editions_by_artist(editions, "The Cars", 0.60);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].artist, "The Cars");
    }

    #[test]
    fn test_filter_by_artist_various_artists() {
        let editions = vec![
            create_test_edition("Various Artists", "Greatest Hits of 1980", 20),
            create_test_edition("The Cars", "Panorama", 10),
        ];
        let filtered = filter_editions_by_artist(editions, "The Cars", 0.60);

        // "Various Artists" should ALWAYS pass (special case)
        assert_eq!(filtered.len(), 2, "Various Artists should always pass");
        assert!(filtered.iter().any(|e| e.artist == "Various Artists"));
        assert!(filtered.iter().any(|e| e.artist == "The Cars"));
    }

    #[test]
    fn test_filter_by_artist_empty_source() {
        let editions = vec![
            create_test_edition("The Cars", "Panorama", 10),
            create_test_edition("Pink Floyd", "The Wall", 26),
        ];
        let filtered = filter_editions_by_artist(editions, "", 0.60);

        // Empty source artist should pass all editions (allow manual review)
        assert_eq!(filtered.len(), 2, "Empty source should pass all");
    }

    #[test]
    fn test_filter_by_artist_unknown_source() {
        let editions = vec![
            create_test_edition("The Cars", "Panorama", 10),
            create_test_edition("Pink Floyd", "The Wall", 26),
        ];
        let filtered = filter_editions_by_artist(editions, "Unknown", 0.60);

        // "Unknown" source artist should pass all editions
        assert_eq!(filtered.len(), 2, "Unknown source should pass all");
    }

    #[test]
    fn test_filter_by_artist_case_insensitive() {
        let editions = vec![
            create_test_edition("THE CARS", "Panorama", 10),
            create_test_edition("the cars", "Panorama", 10),
            create_test_edition("The Cars", "Panorama", 10),
        ];
        let filtered = filter_editions_by_artist(editions, "The Cars", 0.60);

        // All three should pass (case-insensitive matching)
        assert_eq!(filtered.len(), 3, "Case insensitive matching");
    }

    #[test]
    fn test_filter_by_artist_threshold_sensitivity() {
        let editions = vec![
            create_test_edition("The Cars", "Panorama", 10),
            create_test_edition("Cars", "Candy-O", 11),
        ];

        // With threshold 0.50: both should pass
        let filtered_low = filter_editions_by_artist(editions.clone(), "The Cars", 0.50);
        assert_eq!(filtered_low.len(), 2, "Low threshold: both pass");

        // With threshold 1.0: only exact match passes
        let filtered_high = filter_editions_by_artist(editions, "The Cars", 1.0);
        assert_eq!(filtered_high.len(), 1, "High threshold: only exact");
        assert_eq!(filtered_high[0].artist, "The Cars");
    }

    #[test]
    fn test_filter_by_artist_regression_stephan_mathieu() {
        // Regression test: ensure "Stephan Mathieu" is filtered for "The Cars"
        let editions = vec![
            create_test_edition("The Cars", "Panorama", 10),
            create_test_edition("Stephan Mathieu", "Radioland", 1),
        ];

        // With recommended threshold 0.60
        let filtered = filter_editions_by_artist(editions, "The Cars", 0.60);

        assert_eq!(filtered.len(), 1, "Stephan Mathieu should be filtered out");
        assert_eq!(filtered[0].artist, "The Cars");
        assert!(
            filtered.iter().all(|e| e.artist != "Stephan Mathieu"),
            "Stephan Mathieu must not pass filter"
        );
    }
}
