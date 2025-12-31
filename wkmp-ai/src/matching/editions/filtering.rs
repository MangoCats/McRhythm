//! Edition Filtering and Sorting
//!
//! **[PLAN030]** Provides filtering and sorting of editions by name distance
//! to prioritize editions most likely to be the correct match.

use strsim::jaro_winkler;

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
}
