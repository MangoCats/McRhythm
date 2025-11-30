//! Metadata Extraction and Reconciliation
//!
//! **[PLAN030]** ID3 tag extraction, path parsing, and metadata reconciliation.
//!
//! ## Features
//! - **ID3 extraction**: Uses lofty crate for pure Rust tag reading
//! - **Path parsing**: Extract artist/album from filesystem path
//! - **Reconciliation**: Resolve conflicts between ID3 and path metadata
//! - **Heuristics**: Smart defaults (avoid "Various Artists", prefer detailed album names)
//! - **Track count estimation**: Parse track count from ID3 comment field
//!
//! ## Reconciliation Strategies
//! - **DirectMatch**: ID3 and path agree (high confidence)
//! - **PartialMatch**: One field matches, choose best for other (medium confidence)
//! - **Conflict**: Both disagree, apply heuristics (low confidence)
//! - **GapFill**: One source missing, use other (low confidence)
//! - **PathOnly**: ID3 extraction failed (medium confidence)

use std::collections::HashMap;
use std::path::Path;

use lofty::prelude::*;
use lofty::probe::Probe;
use tracing::info;

use super::constants::UNKNOWN_VALUE;
use super::types::{
    ID3Metadata, MetadataConfidence, MetadataSource, ReconciledMetadata, ReconciliationStrategy,
};

// =============================================================================
// Path-Based Metadata Extraction
// =============================================================================

/// Extract artist and album names from file path components
///
/// Assumes path format: .../Artist/Album.mp3
/// Returns (artist, album) tuple with "Unknown" fallback if path too short.
///
/// # Algorithm
/// 1. Split path into components
/// 2. If >= 2 components: artist = second-to-last, album = last (minus extension)
/// 3. If < 2 components: return ("Unknown", "Unknown")
/// 4. Replace ", " with " " in both values (normalization)
pub(crate) fn extract_metadata_from_path(path: &Path) -> (String, String) {
    // Use path components for cross-platform compatibility
    let components: Vec<_> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if components.len() >= 2 {
        let artist = components[components.len() - 2].replace(", ", " ");
        let album_with_ext = components[components.len() - 1];
        // Remove common audio extensions
        let album = album_with_ext
            .trim_end_matches(".mp3")
            .trim_end_matches(".flac")
            .trim_end_matches(".m4a")
            .trim_end_matches(".ogg")
            .trim_end_matches(".wav")
            .replace(", ", " ");

        (artist, album)
    } else {
        (UNKNOWN_VALUE.to_string(), UNKNOWN_VALUE.to_string())
    }
}

// =============================================================================
// ID3 Tag Extraction
// =============================================================================

/// Normalized string comparison (case-insensitive, alphanumeric + whitespace only)
///
/// Used to determine if ID3 and path metadata are equivalent despite formatting differences.
fn strings_match(a: &str, b: &str) -> bool {
    let normalize = |s: &str| -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .to_lowercase()
    };

    normalize(a) == normalize(b)
}

/// Choose between ID3 and path artist using heuristics
///
/// # Heuristics
/// 1. Avoid "Various Artists" if possible (prefer specific artist)
/// 2. If album suggests compilation (greatest, best of, etc.), prefer ID3
/// 3. Default: Prefer ID3 (typically more accurate)
fn choose_artist(
    id3_artist: &str,
    path_artist: &str,
    album: &str,
) -> (String, String, MetadataSource) {
    // Heuristic 1: Avoid "Various Artists" if possible
    let id3_is_various = id3_artist.to_lowercase().contains("various");
    let path_is_various = path_artist.to_lowercase().contains("various");

    if id3_is_various && !path_is_various {
        return (
            path_artist.to_string(),
            id3_artist.to_string(),
            MetadataSource::Path,
        );
    }
    if path_is_various && !id3_is_various {
        return (
            id3_artist.to_string(),
            path_artist.to_string(),
            MetadataSource::ID3,
        );
    }

    // Heuristic 2: If album name suggests compilation, prefer ID3
    let album_lower = album.to_lowercase();
    if album_lower.contains("greatest")
        || album_lower.contains("best of")
        || album_lower.contains("collection")
        || album_lower.contains("anthology")
    {
        return (
            id3_artist.to_string(),
            path_artist.to_string(),
            MetadataSource::ID3,
        );
    }

    // Default: Prefer ID3
    (
        id3_artist.to_string(),
        path_artist.to_string(),
        MetadataSource::ID3,
    )
}

/// Choose between ID3 and path album using heuristics
///
/// # Heuristics
/// 1. Prefer ID3 if it contains more detail (parentheticals, "Deluxe", "Edition", etc.)
/// 2. Prefer ID3 if significantly longer than path (>10 char difference)
/// 3. Default: Prefer ID3
fn choose_album(id3_album: &str, path_album: &str) -> (String, String, MetadataSource) {
    // Heuristic: Prefer ID3 if it contains more detail (edition, year, etc.)
    let id3_has_detail = id3_album.contains('(')
        || id3_album.contains('[')
        || id3_album.contains("Deluxe")
        || id3_album.contains("Edition")
        || id3_album.len() > path_album.len() + 10;

    if id3_has_detail {
        return (
            id3_album.to_string(),
            path_album.to_string(),
            MetadataSource::ID3,
        );
    }

    // Default: Prefer ID3
    (
        id3_album.to_string(),
        path_album.to_string(),
        MetadataSource::ID3,
    )
}

/// Extract ID3 tags using lofty (pure Rust, no external dependencies)
///
/// Reads all relevant ID3v2 tag fields including MusicBrainz IDs.
/// Returns empty metadata (all None) if no tags present.
///
/// # Extracted Fields
/// - Standard: artist, album, date, genre, comment
/// - MusicBrainz: album ID, artist ID
/// - All tags: Complete HashMap for debugging
pub(crate) fn extract_id3_tags(
    file_path: &Path,
) -> Result<ID3Metadata, Box<dyn std::error::Error>> {
    let tagged_file = Probe::open(file_path)?
        .read()
        .map_err(|e| format!("Failed to read tags: {}", e))?;

    let tag = match tagged_file.primary_tag() {
        Some(t) => t,
        None => {
            // No primary tag, return empty metadata
            return Ok(ID3Metadata {
                artist: None,
                album: None,
                date: None,
                genre: None,
                musicbrainz_albumid: None,
                musicbrainz_artistid: None,
                comment: None,
                all_tags: HashMap::new(),
            });
        }
    };

    // Build all_tags map from all tag items
    let mut all_tags = HashMap::new();
    for item in tag.items() {
        let key = format!("{:?}", item.key()).to_lowercase();
        if let lofty::tag::ItemValue::Text(value) = item.value() {
            all_tags.insert(key, value.clone());
        }
    }

    // Extract standard fields using lofty's Accessor trait
    let artist = tag.artist().map(|s| s.to_string()).or_else(|| {
        tag.get_string(&lofty::tag::ItemKey::AlbumArtist)
            .map(String::from)
    });

    let album = tag.album().map(|s| s.to_string());

    let date = tag.year().map(|y| y.to_string()).or_else(|| {
        tag.get_string(&lofty::tag::ItemKey::RecordingDate)
            .map(String::from)
    });

    let genre = tag.genre().map(|s| s.to_string());

    let comment = tag.comment().map(|s| s.to_string());

    // MusicBrainz IDs
    let musicbrainz_albumid = tag
        .get_string(&lofty::tag::ItemKey::MusicBrainzReleaseId)
        .map(String::from);

    let musicbrainz_artistid = tag
        .get_string(&lofty::tag::ItemKey::MusicBrainzArtistId)
        .map(String::from);

    Ok(ID3Metadata {
        artist,
        album,
        date,
        genre,
        musicbrainz_albumid,
        musicbrainz_artistid,
        comment,
        all_tags,
    })
}

// =============================================================================
// Metadata Reconciliation
// =============================================================================

/// Reconcile ID3 tags with path/filename metadata
///
/// Resolves conflicts between ID3 tags and filesystem path using heuristics.
/// Provides both chosen values and alternates for fallback searches.
///
/// # Reconciliation Cases
/// 1. **Both sources have both fields**:
///    - DirectMatch (both agree) → High confidence
///    - PartialMatch (one agrees) → Medium confidence
///    - Conflict (both disagree) → Low confidence, apply heuristics
/// 2. **ID3 has both, path incomplete** → GapFill, Low confidence
/// 3. **Path has both, ID3 incomplete** → GapFill, Low confidence
/// 4. **Incomplete data** → Use what's available, fallback to "Unknown"
pub(crate) fn reconcile_metadata(
    id3: &ID3Metadata,
    path_artist: &Option<String>,
    path_album: &Option<String>,
) -> ReconciledMetadata {
    // Extract estimated track count from comment field
    let estimated_track_count = id3.comment.as_ref().and_then(|c| {
        // Look for patterns like "52 tracks", "28 tracks", etc.
        c.split_whitespace()
            .next()
            .and_then(|s| s.parse::<usize>().ok())
    });

    let has_musicbrainz_ids =
        id3.musicbrainz_albumid.is_some() || id3.musicbrainz_artistid.is_some();

    match (&id3.artist, path_artist, &id3.album, path_album) {
        // Case 1: Both sources have both fields
        (Some(id3_artist), Some(path_artist), Some(id3_album), Some(path_album)) => {
            let artist_match = strings_match(id3_artist, path_artist);
            let album_match = strings_match(id3_album, path_album);

            match (artist_match, album_match) {
                (true, true) => {
                    // DirectMatch - both sources agree
                    ReconciledMetadata {
                        artist: id3_artist.clone(),
                        album: id3_album.clone(),
                        strategy: ReconciliationStrategy::DirectMatch,
                        confidence: MetadataConfidence::High,
                        artist_source: MetadataSource::Both,
                        album_source: MetadataSource::Both,
                        alternate_artist: None,
                        alternate_album: None,
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
                (true, false) => {
                    // Artist matches, album conflicts
                    let (chosen_album, alt_album, album_src) = choose_album(id3_album, path_album);
                    ReconciledMetadata {
                        artist: id3_artist.clone(),
                        album: chosen_album,
                        strategy: ReconciliationStrategy::PartialMatch,
                        confidence: MetadataConfidence::Medium,
                        artist_source: MetadataSource::Both,
                        album_source: album_src,
                        alternate_artist: None,
                        alternate_album: Some(alt_album),
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
                (false, true) => {
                    // Album matches, artist conflicts
                    let (chosen_artist, alt_artist, artist_src) =
                        choose_artist(id3_artist, path_artist, id3_album);
                    ReconciledMetadata {
                        artist: chosen_artist,
                        album: id3_album.clone(),
                        strategy: ReconciliationStrategy::PartialMatch,
                        confidence: MetadataConfidence::Medium,
                        artist_source: artist_src,
                        album_source: MetadataSource::Both,
                        alternate_artist: Some(alt_artist),
                        alternate_album: None,
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
                (false, false) => {
                    // Both conflict
                    let (chosen_artist, alt_artist, artist_src) =
                        choose_artist(id3_artist, path_artist, id3_album);
                    let (chosen_album, alt_album, album_src) = choose_album(id3_album, path_album);
                    ReconciledMetadata {
                        artist: chosen_artist,
                        album: chosen_album,
                        strategy: ReconciliationStrategy::Conflict,
                        confidence: MetadataConfidence::Low,
                        artist_source: artist_src,
                        album_source: album_src,
                        alternate_artist: Some(alt_artist),
                        alternate_album: Some(alt_album),
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
            }
        }

        // Case 2: ID3 has both, path missing one or both
        (Some(id3_artist), _, Some(id3_album), _) => ReconciledMetadata {
            artist: id3_artist.clone(),
            album: id3_album.clone(),
            strategy: ReconciliationStrategy::GapFill,
            confidence: MetadataConfidence::Low,
            artist_source: MetadataSource::ID3,
            album_source: MetadataSource::ID3,
            alternate_artist: path_artist.clone(),
            alternate_album: path_album.clone(),
            has_musicbrainz_ids,
            estimated_track_count,
        },

        // Case 3: Path has both, ID3 missing one or both
        (_, Some(path_artist), _, Some(path_album)) => ReconciledMetadata {
            artist: path_artist.clone(),
            album: path_album.clone(),
            strategy: ReconciliationStrategy::GapFill,
            confidence: MetadataConfidence::Low,
            artist_source: MetadataSource::Path,
            album_source: MetadataSource::Path,
            alternate_artist: id3.artist.clone(),
            alternate_album: id3.album.clone(),
            has_musicbrainz_ids,
            estimated_track_count,
        },

        // Case 4: Incomplete data - use what we have
        _ => {
            let artist = id3
                .artist
                .clone()
                .or_else(|| path_artist.clone())
                .unwrap_or_else(|| UNKNOWN_VALUE.to_string());
            let album = id3
                .album
                .clone()
                .or_else(|| path_album.clone())
                .unwrap_or_else(|| UNKNOWN_VALUE.to_string());

            ReconciledMetadata {
                artist,
                album,
                strategy: ReconciliationStrategy::GapFill,
                confidence: MetadataConfidence::Low,
                artist_source: MetadataSource::Conflict,
                album_source: MetadataSource::Conflict,
                alternate_artist: None,
                alternate_album: None,
                has_musicbrainz_ids,
                estimated_track_count,
            }
        }
    }
}

/// Log reconciliation decision details
///
/// Outputs structured log showing reconciliation strategy, confidence, chosen values,
/// sources, and alternates. Used for debugging and transparency.
pub fn log_reconciliation_decision(reconciled: &ReconciledMetadata, album_num: usize) {
    info!("[A{}]   Phase 0 Reconciliation:", album_num);
    info!("[A{}]     Strategy: {:?}", album_num, reconciled.strategy);
    info!(
        "[A{}]     Confidence: {:?}",
        album_num, reconciled.confidence
    );
    info!(
        "[A{}]     Artist: {} (source: {:?})",
        album_num, reconciled.artist, reconciled.artist_source
    );
    if let Some(ref alt) = reconciled.alternate_artist {
        info!("[A{}]       Alternate: {}", album_num, alt);
    }
    info!(
        "[A{}]     Album: {} (source: {:?})",
        album_num, reconciled.album, reconciled.album_source
    );
    if let Some(ref alt) = reconciled.alternate_album {
        info!("[A{}]       Alternate: {}", album_num, alt);
    }
    if reconciled.has_musicbrainz_ids {
        info!("[A{}]     Has MusicBrainz IDs in tags: Yes", album_num);
    }
    if let Some(count) = reconciled.estimated_track_count {
        info!(
            "[A{}]     Estimated track count from ID3: {}",
            album_num, count
        );
    }
}

/// Extract and reconcile metadata from file (Phase 0 entry point)
///
/// High-level function that extracts metadata from both ID3 tags and filesystem path,
/// then reconciles conflicts using heuristics. Handles ID3 extraction failures gracefully.
///
/// # Error Handling
/// If ID3 extraction fails, falls back to PathOnly strategy with medium confidence.
pub fn extract_and_reconcile_metadata(file_path: &Path) -> ReconciledMetadata {
    // Extract from both sources
    let id3_result = extract_id3_tags(file_path);
    let (path_artist, path_album) = extract_metadata_from_path(file_path);

    // Handle ID3 extraction failure gracefully
    let id3 = match id3_result {
        Ok(tags) => tags,
        Err(_) => {
            // ID3 extraction failed, use path only
            return ReconciledMetadata {
                artist: path_artist,
                album: path_album,
                strategy: ReconciliationStrategy::PathOnly,
                confidence: MetadataConfidence::Medium,
                artist_source: MetadataSource::Path,
                album_source: MetadataSource::Path,
                alternate_artist: None,
                alternate_album: None,
                has_musicbrainz_ids: false,
                estimated_track_count: None,
            };
        }
    };

    // Reconcile
    let path_artist_opt = if path_artist != UNKNOWN_VALUE {
        Some(path_artist)
    } else {
        None
    };
    let path_album_opt = if path_album != UNKNOWN_VALUE {
        Some(path_album)
    } else {
        None
    };

    reconcile_metadata(&id3, &path_artist_opt, &path_album_opt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_metadata_from_path_standard() {
        let path = PathBuf::from("/music/The Beatles/Abbey Road.mp3");
        let (artist, album) = extract_metadata_from_path(&path);
        assert_eq!(artist, "The Beatles");
        assert_eq!(album, "Abbey Road");
    }

    #[test]
    fn test_extract_metadata_from_path_with_comma() {
        let path = PathBuf::from("/music/Crosby, Stills & Nash/CSN.mp3");
        let (artist, album) = extract_metadata_from_path(&path);
        assert_eq!(artist, "Crosby Stills & Nash"); // comma normalized
        assert_eq!(album, "CSN");
    }

    #[test]
    fn test_extract_metadata_from_path_short() {
        let path = PathBuf::from("file.mp3");
        let (artist, album) = extract_metadata_from_path(&path);
        assert_eq!(artist, UNKNOWN_VALUE);
        assert_eq!(album, UNKNOWN_VALUE);
    }

    #[test]
    fn test_strings_match_basic() {
        assert!(strings_match("The Beatles", "the beatles"));
        assert!(strings_match(
            "Bob Marley & The Wailers",
            "Bob Marley  The Wailers"
        ));
        assert!(!strings_match("Beatles", "Rolling Stones"));
    }

    #[test]
    fn test_choose_artist_avoids_various() {
        let (chosen, alt, src) = choose_artist("Various Artists", "Bob Marley", "Legend");
        assert_eq!(chosen, "Bob Marley");
        assert_eq!(alt, "Various Artists");
        assert_eq!(src, MetadataSource::Path);
    }

    #[test]
    fn test_choose_artist_compilation() {
        let (chosen, alt, src) = choose_artist("Queen", "Various", "Greatest Hits");
        assert_eq!(chosen, "Queen");
        assert_eq!(alt, "Various");
        assert_eq!(src, MetadataSource::ID3);
    }

    #[test]
    fn test_choose_album_detailed() {
        let (chosen, alt, src) = choose_album("Abbey Road (2009 Remaster)", "Abbey Road");
        assert_eq!(chosen, "Abbey Road (2009 Remaster)");
        assert_eq!(alt, "Abbey Road");
        assert_eq!(src, MetadataSource::ID3);
    }

    #[test]
    fn test_reconcile_direct_match() {
        let id3 = ID3Metadata {
            artist: Some("The Beatles".to_string()),
            album: Some("Abbey Road".to_string()),
            date: None,
            genre: None,
            musicbrainz_albumid: None,
            musicbrainz_artistid: None,
            comment: None,
            all_tags: HashMap::new(),
        };
        let path_artist = Some("The Beatles".to_string());
        let path_album = Some("Abbey Road".to_string());

        let result = reconcile_metadata(&id3, &path_artist, &path_album);

        assert_eq!(result.strategy, ReconciliationStrategy::DirectMatch);
        assert_eq!(result.confidence, MetadataConfidence::High);
        assert_eq!(result.artist, "The Beatles");
        assert_eq!(result.album, "Abbey Road");
    }

    #[test]
    fn test_reconcile_partial_match_album_conflict() {
        let id3 = ID3Metadata {
            artist: Some("The Beatles".to_string()),
            album: Some("Abbey Road (Remaster)".to_string()),
            date: None,
            genre: None,
            musicbrainz_albumid: None,
            musicbrainz_artistid: None,
            comment: None,
            all_tags: HashMap::new(),
        };
        let path_artist = Some("The Beatles".to_string());
        let path_album = Some("Abbey Road".to_string());

        let result = reconcile_metadata(&id3, &path_artist, &path_album);

        assert_eq!(result.strategy, ReconciliationStrategy::PartialMatch);
        assert_eq!(result.confidence, MetadataConfidence::Medium);
        assert_eq!(result.album, "Abbey Road (Remaster)"); // ID3 has more detail
        assert_eq!(result.alternate_album, Some("Abbey Road".to_string()));
    }

    #[test]
    fn test_reconcile_gap_fill_id3_only() {
        let id3 = ID3Metadata {
            artist: Some("The Beatles".to_string()),
            album: Some("Abbey Road".to_string()),
            date: None,
            genre: None,
            musicbrainz_albumid: None,
            musicbrainz_artistid: None,
            comment: None,
            all_tags: HashMap::new(),
        };

        let result = reconcile_metadata(&id3, &None, &None);

        assert_eq!(result.strategy, ReconciliationStrategy::GapFill);
        assert_eq!(result.confidence, MetadataConfidence::Low);
        assert_eq!(result.artist_source, MetadataSource::ID3);
    }

    #[test]
    fn test_estimated_track_count_from_comment() {
        let id3 = ID3Metadata {
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
            date: None,
            genre: None,
            musicbrainz_albumid: None,
            musicbrainz_artistid: None,
            comment: Some("52 tracks".to_string()),
            all_tags: HashMap::new(),
        };

        let result = reconcile_metadata(&id3, &None, &None);
        assert_eq!(result.estimated_track_count, Some(52));
    }
}
