//! # Metadata Extraction and Reconciliation
//!
//! ID3 tag extraction, path parsing, and metadata reconciliation with conflict resolution.
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
//!
//! ## Usage
//! ```ignore
//! let reconciled = extract_and_reconcile_metadata(file_path);
//! log_reconciliation_decision(&reconciled, album_idx);
//!
//! // Use reconciled.artist and reconciled.album for MusicBrainz search
//! // Fall back to reconciled.alternate_artist / alternate_album if needed
//! ```

use crate::constants::UNKNOWN_VALUE;
use crate::types::{
    ID3Metadata, MetadataConfidence, MetadataSource, ReconciledMetadata, ReconciliationStrategy,
};
use lofty::prelude::*;
use lofty::probe::Probe;
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

// =============================================================================
// Path-Based Metadata Extraction
// =============================================================================

/// Extract artist and album names from file path components
///
/// Assumes path format: .../Artist/Album.mp3
/// Returns (artist, album) tuple with "Unknown" fallback if path too short.
///
/// # Arguments
/// * `path` - Path to audio file
///
/// # Returns
/// (artist, album) tuple - both strings with ", " replaced by " "
///
/// # Algorithm
/// 1. Split path into components
/// 2. If >= 2 components: artist = second-to-last, album = last (minus .mp3)
/// 3. If < 2 components: return ("Unknown", "Unknown")
/// 4. Replace ", " with " " in both values (normalization)
///
/// # Example
/// ```ignore
/// // Path: /music/The Beatles/Abbey Road.mp3
/// let (artist, album) = extract_metadata_from_path(path);
/// // Returns: ("The Beatles", "Abbey Road")
///
/// // Path: /music/single_file.mp3
/// let (artist, album) = extract_metadata_from_path(path);
/// // Returns: ("Unknown", "Unknown")
/// ```
pub(crate) fn extract_metadata_from_path(path: &Path) -> (String, String) {
    // Use path components for cross-platform compatibility
    let components: Vec<_> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if components.len() >= 2 {
        let artist = components[components.len() - 2].replace(", ", " ");
        let album_with_ext = components[components.len() - 1];
        let album = album_with_ext.trim_end_matches(".mp3").replace(", ", " ");

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
///
/// # Arguments
/// * `a` - First string to compare
/// * `b` - Second string to compare
///
/// # Returns
/// true if normalized strings are equal
///
/// # Normalization
/// - Keep only alphanumeric characters and whitespace
/// - Convert to lowercase
///
/// # Example
/// ```ignore
/// assert!(strings_match("The Beatles", "the beatles"));
/// assert!(strings_match("Bob Marley & The Wailers", "Bob Marley  The Wailers"));
/// assert!(!strings_match("Beatles", "Rolling Stones"));
/// ```
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
/// Applies smart defaults to resolve conflicts between ID3 and path artist names.
///
/// # Arguments
/// * `id3_artist` - Artist from ID3 tags
/// * `path_artist` - Artist from filesystem path
/// * `album` - Album name (for compilation detection)
///
/// # Returns
/// (chosen_artist, alternate_artist, source) tuple
///
/// # Heuristics
/// 1. Avoid "Various Artists" if possible (prefer specific artist)
/// 2. If album suggests compilation (greatest, best of, etc.), prefer ID3
/// 3. Default: Prefer ID3 (typically more accurate)
///
/// # Example
/// ```ignore
/// let (chosen, alt, src) = choose_artist("Various Artists", "Bob Marley", "Legend");
/// // Returns: ("Bob Marley", "Various Artists", Path) - avoid "Various"
///
/// let (chosen, alt, src) = choose_artist("Queen", "Queen", "Greatest Hits");
/// // Returns: ("Queen", "Queen", ID3) - compilation, prefer ID3
/// ```
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
/// Applies smart defaults to resolve conflicts between ID3 and path album names.
///
/// # Arguments
/// * `id3_album` - Album from ID3 tags
/// * `path_album` - Album from filesystem path
///
/// # Returns
/// (chosen_album, alternate_album, source) tuple
///
/// # Heuristics
/// 1. Prefer ID3 if it contains more detail (parentheticals, "Deluxe", "Edition", etc.)
/// 2. Prefer ID3 if significantly longer than path (>10 char difference)
/// 3. Default: Prefer ID3
///
/// # Example
/// ```ignore
/// let (chosen, alt, src) = choose_album("Abbey Road (2009 Remaster)", "Abbey Road");
/// // Returns: ("Abbey Road (2009 Remaster)", "Abbey Road", ID3) - more detail
///
/// let (chosen, alt, src) = choose_album("Legend", "Legend");
/// // Returns: ("Legend", "Legend", ID3) - default
/// ```
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
/// # Arguments
/// * `file_path` - Path to audio file (typically .mp3)
///
/// # Returns
/// Result containing ID3Metadata or error
///
/// # Extracted Fields
/// - Standard: artist, album, date, genre, comment
/// - MusicBrainz: album ID, artist ID
/// - All tags: Complete HashMap for debugging
///
/// # Error Handling
/// Returns error if file cannot be opened or read. Caller should handle
/// gracefully (e.g., fall back to path-only metadata).
///
/// # Example
/// ```ignore
/// match extract_id3_tags(file_path) {
///     Ok(id3) => {
///         if let Some(artist) = id3.artist {
///             println!("Artist: {}", artist);
///         }
///     }
///     Err(e) => eprintln!("ID3 extraction failed: {}", e),
/// }
/// ```
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
/// # Arguments
/// * `id3` - ID3 metadata extracted from tags
/// * `path_artist` - Artist name from filesystem path (Some or None)
/// * `path_album` - Album name from filesystem path (Some or None)
///
/// # Returns
/// ReconciledMetadata with chosen values, strategy, confidence, and alternates
///
/// # Reconciliation Cases
/// 1. **Both sources have both fields**:
///    - DirectMatch (both agree) → High confidence
///    - PartialMatch (one agrees) → Medium confidence
///    - Conflict (both disagree) → Low confidence, apply heuristics
/// 2. **ID3 has both, path incomplete** → GapFill, Low confidence
/// 3. **Path has both, ID3 incomplete** → GapFill, Low confidence
/// 4. **Incomplete data** → Use what's available, fallback to "Unknown"
///
/// # Track Count Estimation
/// Parses ID3 comment field for patterns like "52 tracks", "28 tracks"
/// to estimate expected track count before MusicBrainz query.
///
/// # Example
/// ```ignore
/// let reconciled = reconcile_metadata(&id3, &Some("Beatles".into()), &Some("Abbey Road".into()));
/// // Returns chosen artist, chosen album, strategy, confidence, sources, alternates
/// ```
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
///
/// # Arguments
/// * `reconciled` - ReconciledMetadata from reconcile_metadata()
/// * `album_num` - Zero-based album index (displayed as 1-based in logs)
///
/// # Output Format
/// ```text
/// [A1]   Phase 0 Reconciliation:
/// [A1]     Strategy: DirectMatch
/// [A1]     Confidence: High
/// [A1]     Artist: The Beatles (source: Both)
/// [A1]     Album: Abbey Road (source: Both)
/// [A1]     Has MusicBrainz IDs in tags: Yes
/// [A1]     Estimated track count from ID3: 17
/// ```
pub(crate) fn log_reconciliation_decision(reconciled: &ReconciledMetadata, album_num: usize) {
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
/// # Arguments
/// * `file_path` - Path to audio file
///
/// # Returns
/// ReconciledMetadata with chosen artist/album and alternates for fallback
///
/// # Error Handling
/// If ID3 extraction fails, falls back to PathOnly strategy with medium confidence.
/// This ensures metadata is always available for MusicBrainz search.
///
/// # Workflow
/// 1. Extract ID3 tags (may fail if file corrupt or no tags)
/// 2. Extract artist/album from path (always succeeds)
/// 3. If ID3 failed → return PathOnly metadata
/// 4. Otherwise → reconcile ID3 and path using reconcile_metadata()
///
/// # Example
/// ```ignore
/// let reconciled = extract_and_reconcile_metadata(file_path);
/// println!("Artist: {}, Album: {}", reconciled.artist, reconciled.album);
/// println!("Confidence: {:?}", reconciled.confidence);
/// ```
pub(crate) fn extract_and_reconcile_metadata(file_path: &Path) -> ReconciledMetadata {
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
