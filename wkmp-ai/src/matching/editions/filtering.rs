//! Edition Filtering and Sorting
//!
//! **[PLAN030]** Provides filtering and sorting of editions by name distance
//! to prioritize editions most likely to be the correct match.

use std::collections::HashSet;
use strsim::{jaro_winkler, normalized_levenshtein};

use crate::matching::constants::VARIOUS_ARTISTS;
use crate::matching::types::Edition;

// =============================================================================
// Live Album and Compilation Detection
// =============================================================================

/// Live album keywords for title detection
const LIVE_KEYWORDS: &[&str] = &[
    "live",
    "concert",
    "in concert",
    "live at",
    "live in",
    "live from",
    "unplugged",
    "mtv unplugged",
    "bbc live",
    "live on air",
    "live session",
];

/// Compilation keywords for title detection
const COMPILATION_KEYWORDS: &[&str] = &[
    "greatest hits",
    "best of",
    "collection",
    "anthology",
    "compilation",
    "hits",
    "essentials",
];

/// Detect if album title indicates a live recording
///
/// **[PHASE 1 EXTENSION 3]** Detects live albums by checking for common
/// keywords in album title. Live albums often have different track timing
/// than studio recordings due to extended performances, audience interaction,
/// and different arrangements.
///
/// # Arguments
/// * `title` - Album title to check
///
/// # Returns
/// `true` if title contains live album indicators, `false` otherwise
fn is_live_album(title: &str) -> bool {
    let title_lower = title.to_lowercase();
    LIVE_KEYWORDS.iter().any(|keyword| title_lower.contains(keyword))
}

/// Detect if album is a compilation or various artists release
///
/// **[PHASE 1 EXTENSION 3]** Detects compilation albums by checking:
/// 1. Artist is "Various Artists"
/// 2. Title contains compilation keywords
///
/// Compilations often have inconsistent timing and may include different
/// versions/edits than expected from standard releases.
///
/// # Arguments
/// * `artist` - Artist name
/// * `title` - Album title to check
///
/// # Returns
/// `true` if album appears to be a compilation, `false` otherwise
fn is_compilation(artist: &str, title: &str) -> bool {
    // Check if artist is "Various Artists"
    if artist.eq_ignore_ascii_case(VARIOUS_ARTISTS) {
        return true;
    }

    // Check if title contains compilation keywords
    let title_lower = title.to_lowercase();
    COMPILATION_KEYWORDS.iter().any(|keyword| title_lower.contains(keyword))
}

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
/// **[PHASE 1 EXTENSION 3]** Enhanced with debug logging showing:
/// - Edition total duration vs file duration
/// - Duration ratio percentage
/// - Whether edition passed or was filtered
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
    use tracing::debug;

    const MIN_DURATION_RATIO: f64 = 0.85; // 85% - allow for truncated files
    const MAX_DURATION_RATIO: f64 = 1.25; // 125% - allow for bonus content

    editions
        .into_iter()
        .filter(|edition| {
            let edition_total_ms: u64 = edition.durations.iter().map(|&x| x as u64).sum();

            // Skip editions with zero or missing durations
            if edition_total_ms == 0 {
                debug!(
                    release_mbid = %edition.release_mbid,
                    artist = %edition.artist,
                    title = %edition.title,
                    "Duration filter: REJECT (zero or missing track durations)"
                );
                return false;
            }

            let ratio = edition_total_ms as f64 / file_duration_ms as f64;
            let ratio_percent = ratio * 100.0;

            // Accept if within feasible range
            let passed = ratio >= MIN_DURATION_RATIO && ratio <= MAX_DURATION_RATIO;

            if passed {
                debug!(
                    release_mbid = %edition.release_mbid,
                    artist = %edition.artist,
                    title = %edition.title,
                    edition_duration_ms = edition_total_ms,
                    file_duration_ms = file_duration_ms,
                    ratio = %format!("{:.1}%", ratio_percent),
                    "Duration filter: PASS"
                );
            } else {
                let reason = if ratio < MIN_DURATION_RATIO {
                    "edition too short (< 85%)"
                } else {
                    "edition too long (> 125%)"
                };

                debug!(
                    release_mbid = %edition.release_mbid,
                    artist = %edition.artist,
                    title = %edition.title,
                    edition_duration_ms = edition_total_ms,
                    file_duration_ms = file_duration_ms,
                    ratio = %format!("{:.1}%", ratio_percent),
                    reason = reason,
                    "Duration filter: REJECT"
                );
            }

            passed
        })
        .collect()
}

/// Normalize artist name for improved matching
///
/// **[PHASE 1 EXTENSION 1]** Strips common prefixes, suffixes, and punctuation
/// to improve artist name matching. Handles variations like:
/// - "John Mayall & the Bluesbreakers" → "john mayall"
/// - "The Go-Go's" → "go gos"
/// - "Dave Brubeck Quartet" → "dave brubeck"
///
/// # Arguments
/// * `artist` - Artist name to normalize
///
/// # Returns
/// Normalized artist name (lowercase, stripped, cleaned)
fn normalize_artist_name(artist: &str) -> String {
    let mut normalized = artist.to_lowercase();

    // Remove common prefixes (greedy match - longest first)
    let prefixes = ["the ", "a ", "an "];
    for prefix in &prefixes {
        if normalized.starts_with(prefix) {
            normalized = normalized[prefix.len()..].to_string();
            break;
        }
    }

    // Remove common band suffixes (greedy match - longest first)
    let suffixes = [
        " & the bluesbreakers",
        " & his orchestra",
        " and the bluesbreakers",
        " & the gang",
        " and his orchestra",
        " and the gang",
        " orchestra",
        " ensemble",
        " quartet",
        " quintet",
        " trio",
        " band",
    ];
    for suffix in &suffixes {
        if normalized.ends_with(suffix) {
            normalized = normalized[..normalized.len() - suffix.len()].to_string();
            break;
        }
    }

    // Normalize punctuation
    normalized = normalized
        .replace("-", " ")   // Hyphens to spaces
        .replace("'", "")    // Remove apostrophes
        .replace(".", "")    // Remove periods
        .replace(",", "");   // Remove commas

    // Collapse multiple spaces
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
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
/// **[PHASE 1 EXTENSIONS 1-2]** Enhanced with:
/// - Artist name normalization (strips prefixes, suffixes, punctuation)
/// - Prefix/suffix matching bonus (+20% for substring matches)
/// - Token subset bonus (+15% when all tokens from shorter name in longer name)
///
/// # Algorithm
/// 1. Normalize both artist names (strip common affixes, clean punctuation)
/// 2. Tokenize normalized names (split on whitespace)
/// 3. Calculate Jaccard similarity: |intersection| / |union|
/// 4. Calculate normalized Levenshtein distance
/// 5. Calculate base similarity: max(Jaccard, Levenshtein)
/// 6. Apply bonuses for substring/subset matches
/// 7. Return capped score (max 1.0)
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
/// calculate_artist_similarity("The Cars", "Cars") → 1.0 (normalized exact match)
/// calculate_artist_similarity("John Mayall & the Bluesbreakers", "John Mayall") → 1.0 (suffix stripped)
/// calculate_artist_similarity("The Go-Go's", "Go-Go's") → 1.0 (prefix + punctuation)
/// calculate_artist_similarity("Carlos Santana", "Santana") → 0.70 (base 0.50 + capped bonus 0.20)
/// calculate_artist_similarity("The Cars", "Stephan Mathieu") → 0.16 (different artists)
/// ```
fn calculate_artist_similarity(mb_artist: &str, source_artist: &str) -> f64 {
    // **[EXTENSION 1]** Normalize both artist names
    let mb_norm = normalize_artist_name(mb_artist);
    let src_norm = normalize_artist_name(source_artist);

    // Token-based similarity (Jaccard)
    let mb_tokens: HashSet<&str> = mb_norm.split_whitespace().collect();
    let src_tokens: HashSet<&str> = src_norm.split_whitespace().collect();

    let intersection = mb_tokens.intersection(&src_tokens).count();
    let union = mb_tokens.union(&src_tokens).count();

    let jaccard_sim = if union > 0 {
        intersection as f64 / union as f64
    } else {
        0.0
    };

    // Character-based similarity (normalized Levenshtein)
    let levenshtein_sim = normalized_levenshtein(&mb_norm, &src_norm);

    // Base similarity: maximum of Jaccard and Levenshtein
    let base_similarity = jaccard_sim.max(levenshtein_sim);

    // **[EXTENSION 2]** Prefix/suffix matching bonus
    // If one name is a substring of the other, award bonus
    let prefix_bonus: f64 = if mb_norm.contains(&src_norm) || src_norm.contains(&mb_norm) {
        0.20  // +20% bonus for substring match
    } else {
        0.0
    };

    // **[EXTENSION 2]** Token subset bonus
    // If all tokens from shorter name appear in longer name
    let token_subset_bonus: f64 = if mb_tokens.is_subset(&src_tokens) || src_tokens.is_subset(&mb_tokens) {
        0.15  // +15% bonus for token subset
    } else {
        0.0
    };

    // **[BUG FIX]** Apply both bonuses when both conditions are true, but cap total bonus
    // They represent complementary aspects of similarity:
    // - Substring bonus: character-level match (+0.20)
    // - Token subset bonus: word-level match (+0.15)
    // However, cap total bonus at 0.20 to prevent partial matches from scoring too high
    // Example edge case: "Zeppelin" vs "Led Zeppelin" has base 0.69 (high Levenshtein)
    // + 0.20 bonus = 0.89, which correctly stays below 0.90 threshold for high-precision matching
    const MAX_BONUS: f64 = 0.20;
    let bonus = (prefix_bonus + token_subset_bonus).min(MAX_BONUS);

    // Cap final score at 1.0
    (base_similarity + bonus).min(1.0)
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
/// **[PHASE 1 EXTENSION 3]** Enhanced with:
/// - Debug logging showing similarity scores and filter decisions
/// - Relaxed matching for live albums (50% threshold vs 60%)
/// - Relaxed matching for compilations (50% threshold vs 60%)
/// - Special handling for Various Artists (always pass)
///
/// # Special Cases
/// - **Various Artists:** Always passes filter for any source artist (compilations
///   can contain any artist's work)
/// - **Empty/Unknown:** Passes filter to allow manual review
/// - **Live Albums:** Use relaxed threshold (0.50 vs 0.60) due to different artist
///   credits ("The Beatles" vs "The Beatles Live")
/// - **Compilations:** Use relaxed threshold (0.50 vs 0.60) due to various artists
///   or different artist credits on compilation releases
///
/// # Arguments
/// * `editions` - Candidate editions to filter
/// * `source_artist` - Artist name from source file metadata
/// * `source_album` - Album title from source file metadata
/// * `min_similarity` - Minimum similarity threshold (0.0-1.0), typically 0.60
///
/// # Returns
/// Filtered editions with artist similarity >= threshold OR special cases
///
/// # Example
/// ```ignore
/// // Reject editions where artist similarity < 0.60 (or 0.50 for live/compilation)
/// let filtered = filter_editions_by_artist(editions, "The Cars", "Panorama", 0.60);
/// // "Stephan Mathieu" filtered out (similarity ~0.16 < 0.60)
/// // "The Cars" passes (similarity 1.00)
/// // "Cars" passes (similarity ~0.67 > 0.60)
/// // "Various Artists" passes (special case)
/// // Live albums get 0.50 threshold instead of 0.60
/// ```
pub fn filter_editions_by_artist(
    editions: Vec<Edition>,
    source_artist: &str,
    source_album: &str,
    min_similarity: f64,
) -> Vec<Edition> {
    use tracing::debug;

    // Detect if source album is live or compilation for relaxed matching
    let is_source_live = is_live_album(source_album);
    let is_source_compilation = is_compilation(source_artist, source_album);

    // Use relaxed threshold for live albums and compilations
    const RELAXED_THRESHOLD: f64 = 0.50;
    let effective_min_similarity = if is_source_live || is_source_compilation {
        min_similarity.min(RELAXED_THRESHOLD)
    } else {
        min_similarity
    };

    if is_source_live {
        debug!(
            source_album = %source_album,
            threshold = %format!("{:.2}", effective_min_similarity),
            "Live album detected - using relaxed artist matching threshold"
        );
    }

    if is_source_compilation {
        debug!(
            source_artist = %source_artist,
            source_album = %source_album,
            threshold = %format!("{:.2}", effective_min_similarity),
            "Compilation detected - using relaxed artist matching threshold"
        );
    }

    editions
        .into_iter()
        .filter(|edition| {
            let mb_artist = &edition.artist;
            let mb_title = &edition.title;

            // Special case: "Various Artists" always passes (compilations can contain any artist)
            if mb_artist.eq_ignore_ascii_case(VARIOUS_ARTISTS) {
                debug!(
                    mb_artist = %mb_artist,
                    mb_title = %mb_title,
                    source_artist = %source_artist,
                    source_album = %source_album,
                    release_mbid = %edition.release_mbid,
                    "Artist filter: PASS (Various Artists special case)"
                );
                return true;
            }

            // Special case: Empty or unknown source artist passes (allow manual review)
            if source_artist.trim().is_empty() || source_artist.eq_ignore_ascii_case("unknown") {
                debug!(
                    mb_artist = %mb_artist,
                    mb_title = %mb_title,
                    source_artist = %source_artist,
                    source_album = %source_album,
                    release_mbid = %edition.release_mbid,
                    "Artist filter: PASS (empty/unknown source artist)"
                );
                return true;
            }

            // Check if this edition is also live or compilation for additional relaxation
            let is_edition_live = is_live_album(mb_title);
            let is_edition_compilation = is_compilation(mb_artist, mb_title);

            // Use minimum threshold if either source or edition is live/compilation
            let threshold = if is_edition_live || is_edition_compilation {
                effective_min_similarity.min(RELAXED_THRESHOLD)
            } else {
                effective_min_similarity
            };

            // Calculate hybrid similarity and check threshold
            let similarity = calculate_artist_similarity(mb_artist, source_artist);
            let passed = similarity >= threshold;

            if passed {
                let reason = if is_edition_live {
                    " (live album)"
                } else if is_edition_compilation {
                    " (compilation)"
                } else {
                    ""
                };

                debug!(
                    mb_artist = %mb_artist,
                    mb_title = %mb_title,
                    source_artist = %source_artist,
                    source_album = %source_album,
                    release_mbid = %edition.release_mbid,
                    similarity = %format!("{:.3}", similarity),
                    threshold = %format!("{:.2}", threshold),
                    reason = reason,
                    "Artist filter: PASS"
                );
            } else {
                debug!(
                    mb_artist = %mb_artist,
                    mb_title = %mb_title,
                    source_artist = %source_artist,
                    source_album = %source_album,
                    release_mbid = %edition.release_mbid,
                    similarity = %format!("{:.3}", similarity),
                    threshold = %format!("{:.2}", threshold),
                    "Artist filter: REJECT (similarity below threshold)"
                );
            }

            passed
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
    // Artist Normalization Tests (Phase 1 Extension 1)
    // =============================================================================

    #[test]
    fn test_normalize_artist_with_suffix() {
        assert_eq!(
            normalize_artist_name("John Mayall & the Bluesbreakers"),
            "john mayall"
        );
        assert_eq!(
            normalize_artist_name("Dave Brubeck Quartet"),
            "dave brubeck"
        );
        assert_eq!(
            normalize_artist_name("Count Basie Orchestra"),
            "count basie"
        );
    }

    #[test]
    fn test_normalize_artist_punctuation() {
        assert_eq!(
            normalize_artist_name("The Go-Go's"),
            "go gos"
        );
        assert_eq!(
            normalize_artist_name("Earth, Wind & Fire"),
            "earth wind & fire"
        );
    }

    #[test]
    fn test_normalize_artist_prefix() {
        assert_eq!(
            normalize_artist_name("The Beatles"),
            "beatles"
        );
        assert_eq!(
            normalize_artist_name("A Tribe Called Quest"),
            "tribe called quest"
        );
    }

    #[test]
    fn test_normalize_artist_combined() {
        assert_eq!(
            normalize_artist_name("The Dave Brubeck Quartet"),
            "dave brubeck"
        );
        assert_eq!(
            normalize_artist_name("The Go-Go's"),
            "go gos"
        );
    }

    // =============================================================================
    // Enhanced Similarity Tests (Phase 1 Extensions 1-2)
    // =============================================================================

    #[test]
    fn test_artist_similarity_with_suffix() {
        // John Mayall vs John Mayall & the Bluesbreakers
        let sim = calculate_artist_similarity("John Mayall & the Bluesbreakers", "John Mayall");
        assert!(sim >= 0.95, "Should match with suffix stripped, got {}", sim);
    }

    #[test]
    fn test_artist_similarity_substring_bonus() {
        // Santana vs Carlos Santana - trace through calculation
        let sim = calculate_artist_similarity("Carlos Santana", "Santana");

        // Expected calculation:
        // 1. Normalize: "carlos santana" vs "santana"
        // 2. Jaccard: {"carlos", "santana"} ∩ {"santana"} / union = 1/2 = 0.50
        // 3. Levenshtein: normalized_levenshtein("carlos santana", "santana")
        // 4. Base: max(Jaccard, Levenshtein) = 0.50
        // 5. Substring bonus: "santana" in "carlos santana" → +0.20
        // 6. Token subset: {"santana"} ⊆ {"carlos", "santana"} → +0.15
        // 7. Total bonus: min(0.20 + 0.15, 0.20) = 0.20 (capped at MAX_BONUS)
        // 8. Final: 0.50 + 0.20 = 0.70

        println!("Actual similarity: {}", sim);
        assert!(sim >= 0.65, "Substring match should score high, got {}", sim);
    }

    #[test]
    fn test_artist_similarity_token_subset() {
        // Dave Brubeck vs The Dave Brubeck Quartet
        let sim = calculate_artist_similarity("The Dave Brubeck Quartet", "Dave Brubeck");
        assert!(sim >= 0.90, "Token subset should score high, got {}", sim);
    }

    #[test]
    fn test_artist_similarity_go_gos() {
        // The Go-Go's vs Go-Go's
        let sim = calculate_artist_similarity("The Go-Go's", "Go-Go's");
        assert!(sim >= 0.95, "Punctuation + prefix should match, got {}", sim);
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
        let filtered = filter_editions_by_artist(editions, "The Cars", "Panorama", 0.60);

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
        let filtered = filter_editions_by_artist(editions, "The Beatles", "Abbey Road", 0.60);

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
        let filtered = filter_editions_by_artist(editions, "The Cars", "Panorama", 0.60);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].artist, "The Cars");
    }

    #[test]
    fn test_filter_by_artist_various_artists() {
        let editions = vec![
            create_test_edition("Various Artists", "Greatest Hits of 1980", 20),
            create_test_edition("The Cars", "Panorama", 10),
        ];
        let filtered = filter_editions_by_artist(editions, "The Cars", "Panorama", 0.60);

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
        let filtered = filter_editions_by_artist(editions, "", "Panorama", 0.60);

        // Empty source artist should pass all editions (allow manual review)
        assert_eq!(filtered.len(), 2, "Empty source should pass all");
    }

    #[test]
    fn test_filter_by_artist_unknown_source() {
        let editions = vec![
            create_test_edition("The Cars", "Panorama", 10),
            create_test_edition("Pink Floyd", "The Wall", 26),
        ];
        let filtered = filter_editions_by_artist(editions, "Unknown", "Album", 0.60);

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
        let filtered = filter_editions_by_artist(editions, "The Cars", "Panorama", 0.60);

        // All three should pass (case-insensitive matching)
        assert_eq!(filtered.len(), 3, "Case insensitive matching");
    }

    #[test]
    fn test_filter_by_artist_threshold_sensitivity() {
        let editions = vec![
            create_test_edition("Led Zeppelin", "Physical Graffiti", 10),
            create_test_edition("Led", "Some Album", 11),
            create_test_edition("Zeppelin", "Another Album", 12),
        ];

        // With threshold 0.50: all should pass (partial matches)
        let filtered_low = filter_editions_by_artist(editions.clone(), "Led Zeppelin", "Physical Graffiti", 0.50);
        assert_eq!(filtered_low.len(), 3, "Low threshold: all pass");

        // With threshold 0.90: only exact match passes (partial names won't reach 0.90 after normalization)
        let filtered_high = filter_editions_by_artist(editions, "Led Zeppelin", "Physical Graffiti", 0.90);
        assert_eq!(filtered_high.len(), 1, "High threshold: only exact match");
        assert_eq!(filtered_high[0].artist, "Led Zeppelin");
    }

    #[test]
    fn test_filter_by_artist_regression_stephan_mathieu() {
        // Regression test: ensure "Stephan Mathieu" is filtered for "The Cars"
        let editions = vec![
            create_test_edition("The Cars", "Panorama", 10),
            create_test_edition("Stephan Mathieu", "Radioland", 1),
        ];

        // With recommended threshold 0.60
        let filtered = filter_editions_by_artist(editions, "The Cars", "Panorama", 0.60);

        assert_eq!(filtered.len(), 1, "Stephan Mathieu should be filtered out");
        assert_eq!(filtered[0].artist, "The Cars");
        assert!(
            filtered.iter().all(|e| e.artist != "Stephan Mathieu"),
            "Stephan Mathieu must not pass filter"
        );
    }

    // =============================================================================
    // Live Album and Compilation Tests (Phase 1 Extension 3)
    // =============================================================================

    #[test]
    fn test_filter_by_artist_live_album_relaxed() {
        // Live album should use 0.50 threshold instead of 0.60
        let editions = vec![
            create_test_edition("The Beatles", "Live at the BBC", 20),
            create_test_edition("Beatles", "Live at the Hollywood Bowl", 15),
        ];

        // Even with 0.60 threshold, live albums should use 0.50
        let filtered = filter_editions_by_artist(editions, "The Beatles", "Live at the BBC", 0.60);

        // Both should pass with relaxed threshold
        assert_eq!(filtered.len(), 2, "Live albums should use relaxed threshold");
    }

    #[test]
    fn test_filter_by_artist_compilation_relaxed() {
        // Compilation should use 0.50 threshold instead of 0.60
        let editions = vec![
            create_test_edition("The Beatles", "Greatest Hits", 25),
            create_test_edition("Beatles", "The Best of the Beatles", 20),
        ];

        // Even with 0.60 threshold, compilations should use 0.50
        let filtered = filter_editions_by_artist(editions, "The Beatles", "Greatest Hits", 0.60);

        // Both should pass with relaxed threshold
        assert_eq!(filtered.len(), 2, "Compilations should use relaxed threshold");
    }

    #[test]
    fn test_is_live_album_detection() {
        assert!(is_live_album("Live at the BBC"));
        assert!(is_live_album("MTV Unplugged"));
        assert!(is_live_album("In Concert"));
        assert!(is_live_album("Live from Madison Square Garden"));
        assert!(!is_live_album("Abbey Road"));
        assert!(!is_live_album("Revolver"));
    }

    #[test]
    fn test_is_compilation_detection() {
        assert!(is_compilation("Various Artists", "Now That's What I Call Music"));
        assert!(is_compilation("The Beatles", "Greatest Hits"));
        assert!(is_compilation("Led Zeppelin", "The Best of Led Zeppelin"));
        assert!(is_compilation("Pink Floyd", "Echoes: The Best of Pink Floyd"));
        assert!(!is_compilation("The Beatles", "Abbey Road"));
        assert!(!is_compilation("Pink Floyd", "The Wall"));
    }
}
