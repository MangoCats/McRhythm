//! # Validation Logic
//!
//! Artist/album name validation using Jaro-Winkler and Levenshtein distance.
//!
//! ## Features
//! - **Artist verification**: Jaro-Winkler similarity with normalization
//! - **Album verification**: Removes parenthetical suffixes, substring matching
//! - **Name distance**: Combined metric for sorting editions by name similarity
//! - **Levenshtein utilities**: Average distance, best ratio across variants
//!
//! ## Name Normalization
//! Artist names: Remove "The", "A", punctuation, unicode folding, conjunctions
//! Album names: Lowercase, remove parenthetical suffixes like "(Deluxe Edition)"
//!
//! ## Thresholds
//! - ARTIST_MISMATCH_THRESHOLD: 0.5 (50% Jaro-Winkler similarity)
//! - ALBUM_MISMATCH_THRESHOLD: 0.5 (50% Jaro-Winkler similarity)
//! - Substring match: Auto-accept if one name contains the other
//!
//! ## Related Modules
//! - `constants`: NAME_DISTANCE_*, ARTIST_MISMATCH_THRESHOLD, ALBUM_MISMATCH_THRESHOLD
//! - `edition`: Uses verify_artist_match() and verify_album_match() for winner selection

use crate::constants::*;
use strsim::{jaro_winkler, levenshtein};

// =============================================================================
// Divergence Check (Single Source of Truth for Stage 3 and Stage 4)
// =============================================================================

/// Result of artist-album divergence check
///
/// The divergence check prevents wrong-artist matches where the album name
/// coincidentally matches but the artist doesn't (e.g., "Anthology", "Atlas").
///
/// Rule: `artist_similarity >= album_similarity × STAGE4_MIN_ARTIST_ALBUM_RATIO`
#[derive(Debug, Clone)]
pub(crate) struct DivergenceResult {
    /// Jaro-Winkler similarity of edition artist to source artist variants (0.0-1.0)
    pub artist_similarity: f64,
    /// Jaro-Winkler similarity of edition album to source album variants (0.0-1.0)
    pub album_similarity: f64,
    /// Minimum required artist similarity = album_similarity × ratio
    pub min_artist_required: f64,
    /// Whether the check passes (artist_similarity >= min_artist_required)
    pub passes: bool,
}

/// Check for artist-album name divergence (Stage 3/4 gating)
///
/// Rejects editions where album name matches well but artist doesn't - indicates
/// a wrong-artist match with coincidentally similar album name.
///
/// # Arguments
/// * `edition_artist` - Artist name from MusicBrainz edition
/// * `edition_album` - Album name from MusicBrainz edition
/// * `source_artists` - Artist name variants from ID3 tags/path
/// * `source_albums` - Album name variants from ID3 tags/path
///
/// # Returns
/// DivergenceResult with similarity scores and pass/fail status
///
/// # Example
/// ```ignore
/// let result = check_divergence(
///     "The Supremes",
///     "Anthology",
///     &["Steve Howe".to_string()],
///     &["Anthology".to_string()]
/// );
/// // result.passes = false (artist similarity < album similarity × 0.80)
/// ```
pub(crate) fn check_divergence(
    edition_artist: &str,
    edition_album: &str,
    source_artists: &[String],
    source_albums: &[String],
) -> DivergenceResult {
    let artist_similarity = best_jaro_winkler_ratio(edition_artist, source_artists);
    let album_similarity = best_jaro_winkler_ratio(edition_album, source_albums);
    let min_artist_required = album_similarity * STAGE4_MIN_ARTIST_ALBUM_RATIO;
    let passes = artist_similarity >= min_artist_required;

    DivergenceResult {
        artist_similarity,
        album_similarity,
        min_artist_required,
        passes,
    }
}

// =============================================================================
// Levenshtein Distance Utilities
// =============================================================================

/// Calculate average Levenshtein distance from candidate to all source variants
///
/// Returns the mean edit distance across all source strings. Lower values
/// indicate better matches.
///
/// # Arguments
/// * `candidate` - String to compare (e.g., MusicBrainz artist/album name)
/// * `sources` - List of source strings (e.g., ID3 tag variants)
///
/// # Returns
/// Average Levenshtein distance (edit operations required)
///
/// # Example
/// ```ignore
/// let distance = avg_levenshtein("The Beatles", &["Beatles".to_string()]);
/// // Returns 4.0 (need to remove "The " = 4 characters)
/// ```
pub(crate) fn avg_levenshtein(candidate: &str, sources: &[String]) -> f64 {
    if sources.is_empty() {
        return 0.0;
    }
    let total: usize = sources.iter().map(|s| levenshtein(candidate, s)).sum();
    total as f64 / sources.len() as f64
}

/// Calculate Levenshtein ratio (similarity) between two strings
///
/// The ratio is calculated as: 1 - (distance / max_length), giving a value
/// between 0.0 (completely different) and 1.0 (identical).
///
/// # Arguments
/// * `s1` - First string to compare
/// * `s2` - Second string to compare
///
/// # Returns
/// Similarity ratio from 0.0 to 1.0. Higher values indicate better matches.
///
/// # Example
/// ```ignore
/// let ratio = levenshtein_ratio("hello", "hallo");
/// // Returns 0.8 (4 matching chars out of 5)
/// ```
pub(crate) fn levenshtein_ratio(s1: &str, s2: &str) -> f64 {
    if s1.is_empty() && s2.is_empty() {
        return 1.0;
    }
    let max_len = s1.len().max(s2.len());
    if max_len == 0 {
        return 1.0;
    }
    let distance = levenshtein(s1, s2);
    1.0 - (distance as f64 / max_len as f64)
}

/// Calculate best Levenshtein ratio from candidate to any source variant
///
/// Returns the highest similarity ratio across all source variants (case-insensitive).
/// Used for Run 24 pre-NDR filtering.
///
/// # Arguments
/// * `candidate` - The string to compare (e.g., MusicBrainz artist/album name)
/// * `sources` - List of source strings to compare against (e.g., ID3 tag variants)
///
/// # Returns
/// Best (highest) similarity ratio from 0.0 to 1.0
///
/// # Example
/// ```ignore
/// let ratio = best_levenshtein_ratio(
///     "The Beatles",
///     &["Beatles".to_string(), "The Beatles".to_string()]
/// );
/// // Returns 1.0 (exact match with second variant)
/// ```
pub(crate) fn best_levenshtein_ratio(candidate: &str, sources: &[String]) -> f64 {
    if sources.is_empty() {
        return 0.0;
    }
    let candidate_lower = candidate.to_lowercase();
    sources
        .iter()
        .map(|s| levenshtein_ratio(&candidate_lower, &s.to_lowercase()))
        .fold(0.0_f64, |a, b| a.max(b))
}

/// Calculate best Jaro-Winkler similarity from candidate to any source variant
///
/// Returns the highest Jaro-Winkler similarity across all source variants.
/// Unlike Levenshtein ratio, Jaro-Winkler correctly identifies that strings
/// with completely different character sets (e.g., Latin vs Japanese) have
/// near-zero similarity.
///
/// **Run 29 fix**: Use this instead of best_levenshtein_ratio() for Stage 4
/// artist gating. Levenshtein ratio gave false 66.7% for "Fluke" vs "ゆいかおり"
/// (both 5 chars) when Jaro-Winkler correctly gives 0%.
///
/// # Arguments
/// * `candidate` - The string to compare (e.g., MusicBrainz artist/album name)
/// * `sources` - List of source strings to compare against (e.g., ID3 tag variants)
///
/// # Returns
/// Best (highest) Jaro-Winkler similarity from 0.0 to 1.0
///
/// # Example
/// ```ignore
/// // Different character sets correctly identified as dissimilar
/// let ratio = best_jaro_winkler_ratio("Fluke", &["ゆいかおり".to_string()]);
/// // Returns ~0.0 (completely different character sets)
///
/// // Similar names correctly matched
/// let ratio = best_jaro_winkler_ratio(
///     "The Beatles",
///     &["Beatles".to_string(), "The Beatles".to_string()]
/// );
/// // Returns 1.0 (exact match with second variant)
/// ```
pub(crate) fn best_jaro_winkler_ratio(candidate: &str, sources: &[String]) -> f64 {
    if sources.is_empty() {
        return 0.0;
    }
    let candidate_lower = candidate.to_lowercase();
    sources
        .iter()
        .map(|s| jaro_winkler(&candidate_lower, &s.to_lowercase()))
        .fold(0.0_f64, |a, b| a.max(b))
}

/// Calculate overall name distance score for a release against source variants
///
/// Combines artist and album distance with album weighted 2× (NAME_DISTANCE_ALBUM_WEIGHT = √2)
/// and artist weighted 1× (NAME_DISTANCE_ARTIST_WEIGHT = 1.0).
///
/// # Arguments
/// * `candidate_artist` - Artist name from MusicBrainz
/// * `candidate_album` - Album name from MusicBrainz
/// * `source_artists` - Artist variants from ID3 tags
/// * `source_albums` - Album variants from ID3 tags
///
/// # Returns
/// Combined name distance score. Lower scores indicate better matches.
///
/// # Formula
/// ```ignore
/// score = (1.414 × album_distance + 1.0 × artist_distance) / (1.414 + 1.0)
///       = (1.414 × album_distance + artist_distance) / 2.414
/// ```
///
/// # Example
/// ```ignore
/// let score = calculate_name_distance(
///     "Bob Marley",
///     "Legend",
///     &["Bob Marley & The Wailers".to_string()],
///     &["Legend - The Best Of".to_string()]
/// );
/// // Returns weighted average distance
/// ```
pub(crate) fn calculate_name_distance(
    candidate_artist: &str,
    candidate_album: &str,
    source_artists: &[String],
    source_albums: &[String],
) -> f64 {
    let avg_album_distance = avg_levenshtein(candidate_album, source_albums);
    let avg_artist_distance = avg_levenshtein(candidate_artist, source_artists);

    // Overall score: album name weighted 2x, artist name weighted 1x
    (NAME_DISTANCE_ALBUM_WEIGHT * avg_album_distance
        + NAME_DISTANCE_ARTIST_WEIGHT * avg_artist_distance)
        / (NAME_DISTANCE_ALBUM_WEIGHT + NAME_DISTANCE_ARTIST_WEIGHT)
}

// =============================================================================
// Artist Name Normalization and Verification
// =============================================================================

/// Normalize an artist name for comparison purposes
///
/// Removes common variations like "The", punctuation, and normalizes case.
/// This allows fuzzy matching of artist names that differ only in formatting.
///
/// # Transformations
/// - Lowercase conversion
/// - Remove prefixes: "The ", "a "
/// - Remove conjunctions: " & ", " and "
/// - Remove punctuation: ' " ! . , -
/// - Unicode folding: ö→o, ü→u, é→e, ñ→n, etc.
/// - Collapse multiple spaces
///
/// # Examples
/// ```ignore
/// assert_eq!(normalize_artist_name("The Beatles"), "beatles");
/// assert_eq!(normalize_artist_name("Bob Marley & The Wailers"), "bob marley wailers");
/// assert_eq!(normalize_artist_name("P!nk"), "pink");
/// assert_eq!(normalize_artist_name("Björk"), "bjork");
/// ```
pub(crate) fn normalize_artist_name(name: &str) -> String {
    let mut normalized = name.to_lowercase();

    // Remove common prefixes
    for prefix in &["the ", "a "] {
        if normalized.starts_with(prefix) {
            normalized = normalized[prefix.len()..].to_string();
        }
    }

    // Replace common conjunctions and punctuation
    normalized = normalized
        .replace(" & ", " ")
        .replace(" and ", " ")
        .replace("'", "")
        .replace("\"", "")
        .replace("!", "")
        .replace(".", "")
        .replace(",", "")
        .replace("-", " ");

    // Normalize unicode characters (basic ASCII folding)
    normalized = normalized
        .replace("ö", "o")
        .replace("ø", "o")
        .replace("ü", "u")
        .replace("ä", "a")
        .replace("é", "e")
        .replace("è", "e")
        .replace("ë", "e")
        .replace("í", "i")
        .replace("ì", "i")
        .replace("ñ", "n")
        .replace("ß", "ss");

    // Collapse multiple spaces
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    parts.join(" ")
}

/// Verify if a matched artist name is sufficiently similar to the source artist
///
/// Uses Jaro-Winkler similarity which gives higher weight to prefix matches,
/// making it ideal for artist name variations.
///
/// # Arguments
/// * `source_artist` - Artist name from ID3 tags/path
/// * `matched_artist` - Artist name from selected MusicBrainz edition
///
/// # Returns
/// Tuple of (similarity_score, is_acceptable_match)
/// - similarity_score: 0.0 (completely different) to 1.0 (identical)
/// - is_acceptable_match: true if similarity >= ARTIST_MISMATCH_THRESHOLD (0.5) OR substring match
///
/// # Algorithm
/// 1. Normalize both names (remove "The", punctuation, unicode folding)
/// 2. Calculate Jaro-Winkler similarity (prefix-weighted)
/// 3. Check substring containment (handles "Bob Marley" vs "Bob Marley & The Wailers")
/// 4. Accept if similarity >= 0.5 OR one contains the other
///
/// # Example
/// ```ignore
/// let (sim, ok) = verify_artist_match("The Beatles", "Beatles");
/// assert!(ok);  // True (substring match)
/// assert!(sim > 0.9);  // High similarity after normalization
/// ```
pub(crate) fn verify_artist_match(source_artist: &str, matched_artist: &str) -> (f64, bool) {
    let normalized_source = normalize_artist_name(source_artist);
    let normalized_matched = normalize_artist_name(matched_artist);

    // Use Jaro-Winkler for better handling of name variations
    // It gives higher scores when strings share a common prefix
    let similarity = jaro_winkler(&normalized_source, &normalized_matched);

    // Also check if one is a substring of the other (handles "Bob Marley" vs "Bob Marley & The Wailers")
    let is_substring = normalized_source.contains(&normalized_matched)
        || normalized_matched.contains(&normalized_source);

    let is_acceptable = similarity >= ARTIST_MISMATCH_THRESHOLD || is_substring;

    (similarity, is_acceptable)
}

// =============================================================================
// Album Name Verification
// =============================================================================

/// Verify if a matched album name is sufficiently similar to the source album
///
/// Uses Jaro-Winkler similarity which gives higher weight to prefix matches.
/// Removes parenthetical suffixes like "(Deluxe Edition)" before comparison.
///
/// # Arguments
/// * `source_album` - Album name from ID3 tags/path
/// * `matched_album` - Album name from selected MusicBrainz edition
///
/// # Returns
/// Tuple of (similarity_score, is_acceptable_match)
/// - similarity_score: 0.0 (completely different) to 1.0 (identical)
/// - is_acceptable_match: true if similarity >= ALBUM_MISMATCH_THRESHOLD (0.5) OR substring match
///
/// # Algorithm
/// 1. Normalize both names: lowercase, remove parenthetical suffixes
/// 2. Calculate Jaro-Winkler similarity
/// 3. Check substring containment (handles "Greatest Hits" vs "Greatest Hits Vol. 1")
/// 4. Accept if similarity >= 0.5 OR one contains the other
///
/// # Example
/// ```ignore
/// let (sim, ok) = verify_album_match("Abbey Road", "Abbey Road (2009 Remaster)");
/// assert!(ok);  // True (parentheticals removed, high similarity)
/// assert!(sim > 0.95);
/// ```
pub(crate) fn verify_album_match(source_album: &str, matched_album: &str) -> (f64, bool) {
    // Normalize album names: lowercase, remove parenthetical suffixes like "(Deluxe Edition)"
    let normalize = |s: &str| -> String {
        let lower = s.to_lowercase();
        // Remove common suffixes in parentheses
        let without_parens = lower.split('(').next().unwrap_or(&lower).trim().to_string();
        without_parens
    };

    let normalized_source = normalize(source_album);
    let normalized_matched = normalize(matched_album);

    // Use Jaro-Winkler for better handling of name variations
    let similarity = jaro_winkler(&normalized_source, &normalized_matched);

    // Also check if one is a substring of the other (handles "Greatest Hits" vs "Greatest Hits Vol. 1")
    let is_substring = normalized_source.contains(&normalized_matched)
        || normalized_matched.contains(&normalized_source);

    let is_acceptable = similarity >= ALBUM_MISMATCH_THRESHOLD || is_substring;

    (similarity, is_acceptable)
}
