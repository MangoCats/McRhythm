//! Table semantics API - provides column descriptions for database tables
//!
//! [REQ-DR-F-010]: Enhanced table viewing with column descriptions

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;

use crate::AppState;

/// Column description (max 25 words)
#[derive(Debug, Serialize)]
pub struct ColumnDescription {
    pub name: String,
    pub description: String,
}

/// Table semantics response
#[derive(Debug, Serialize)]
pub struct TableSemanticsResponse {
    pub table_name: String,
    pub columns: Vec<ColumnDescription>,
}

/// GET /api/semantics/:table_name
///
/// Returns concise descriptions for all columns in the specified table.
pub async fn get_table_semantics(
    State(_state): State<AppState>,
    Path(table_name): Path<String>,
) -> Result<Json<TableSemanticsResponse>, SemanticsError> {
    // Validate table name (same whitelist as table.rs)
    let semantics = get_semantics(&table_name)
        .ok_or_else(|| SemanticsError::InvalidTableName(table_name.clone()))?;

    Ok(Json(TableSemanticsResponse {
        table_name,
        columns: semantics,
    }))
}

/// Get column descriptions for a table
fn get_semantics(table_name: &str) -> Option<Vec<ColumnDescription>> {
    let mut map: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();

    // songs table
    map.insert("songs", vec![
        ("title", "Song title from MusicBrainz (UTF-8 text, NULL when unavailable)"),
        ("lyrics", "Lyrics text for this song (optional)"),
        ("related_songs", "Related songs metadata (JSON)"),
        ("base_probability", "Selection weight 0.0-1.0 for this song, default 1.0"),
        ("min_cooldown", "Minimum time before song can repeat (milliseconds)"),
        ("ramping_cooldown", "Progressive cooldown multiplier for repeated plays"),
        ("last_played_at", "Timestamp of most recent playback (NULL if never played)"),
        ("created_at", "When this record was created"),
        ("updated_at", "When this record was last updated"),
        ("guid", "Unique identifier for this song"),
        ("recording_mbid", "MusicBrainz Recording ID"),
        ("work_id", "Reference to works table (musical work/composition)"),
    ]);

    // passages table
    map.insert("passages", vec![
        ("path", "[DE-REFERENCED] File path from files table (light yellow background)"),
        ("guid", "Unique identifier for this passage"),
        ("file_id", "Foreign key to files.guid (path column shows actual file path)"),
        ("start_time_ticks", "Start position in ticks (100ns units)"),
        ("fade_in_start_ticks", "Fade-in begins at this tick position"),
        ("lead_in_start_ticks", "Lead-in begins at this tick position"),
        ("lead_out_start_ticks", "Lead-out begins at this tick position"),
        ("fade_out_start_ticks", "Fade-out begins at this tick position"),
        ("end_time_ticks", "End position in ticks (100ns units)"),
        ("fade_in_curve", "Fade curve: exponential, cosine, linear, logarithmic, equal_power"),
        ("fade_out_curve", "Fade curve: exponential, cosine, linear, logarithmic, equal_power"),
        ("title", "Passage title from metadata (optional)"),
        ("user_title", "User-provided override title (optional)"),
        ("artist", "Artist name from metadata (optional)"),
        ("album", "Album name from metadata (optional)"),
        ("musical_flavor_vector", "AcousticBrainz feature vector (JSON)"),
        ("import_metadata", "Import workflow metadata (JSON)"),
        ("additional_metadata", "Additional metadata (JSON)"),
        ("decode_status", "Decode status: pending, successful, unsupported_codec, failed"),
        ("created_at", "When this record was created"),
        ("updated_at", "When this record was last updated"),
    ]);

    // files table
    map.insert("files", vec![
        ("guid", "Unique identifier for this audio file"),
        ("path", "File path relative to root folder, forward-slash separated"),
        ("hash", "SHA-256 hash for duplicate detection"),
        ("duration_ticks", "File duration in ticks (28,224,000 Hz tick rate)"),
        ("format", "Audio format: FLAC, MP3, AAC, WAV, Opus, etc."),
        ("sample_rate", "Sample rate in Hz (e.g., 44100, 48000, 96000)"),
        ("channels", "Number of audio channels (1=mono, 2=stereo, 6=5.1)"),
        ("file_size_bytes", "File size in bytes"),
        ("modification_time", "File last modified timestamp (from filesystem)"),
        ("created_at", "When this record was created"),
        ("updated_at", "When this record was last updated"),
    ]);

    // artists table
    map.insert("artists", vec![
        ("guid", "Unique identifier for this artist"),
        ("mbid", "MusicBrainz Artist ID"),
        ("name", "Artist name from MusicBrainz"),
        ("sort_name", "Name formatted for alphabetical sorting"),
        ("artist_type", "MusicBrainz type: person, group, orchestra, choir, character, other"),
        ("last_played_at", "Timestamp when any song by this artist last played"),
        ("created_at", "When this record was created"),
        ("updated_at", "When this record was last updated"),
    ]);

    // albums table
    map.insert("albums", vec![
        ("guid", "Unique identifier for this album"),
        ("mbid", "MusicBrainz Release ID"),
        ("title", "Album title from MusicBrainz"),
        ("release_date", "Album release date (ISO 8601 format)"),
        ("album_type", "MusicBrainz type: album, single, EP, compilation, soundtrack, etc."),
        ("created_at", "When this record was created"),
        ("updated_at", "When this record was last updated"),
    ]);

    // works table
    map.insert("works", vec![
        ("guid", "Unique identifier for this musical work"),
        ("mbid", "MusicBrainz Work ID"),
        ("title", "Work title from MusicBrainz"),
        ("work_type", "MusicBrainz type: song, symphony, concerto, opera, etc."),
        ("iswc", "International Standard Musical Work Code (if available)"),
        ("last_played_at", "Timestamp when any song from this work last played"),
        ("created_at", "When this record was created"),
        ("updated_at", "When this record was last updated"),
    ]);

    // passage_songs junction table
    map.insert("passage_songs", vec![
        ("passage_guid", "References passages.guid"),
        ("song_guid", "References songs.guid"),
    ]);

    // album_songs junction table
    map.insert("album_songs", vec![
        ("album_guid", "References albums.guid"),
        ("song_guid", "References songs.guid"),
        ("track_number", "Position on album (NULL if unknown)"),
        ("disc_number", "Disc number for multi-disc albums (NULL if single disc)"),
    ]);

    // settings table
    map.insert("settings", vec![
        ("key", "Setting name (primary key)"),
        ("value", "Setting value stored as TEXT (may contain JSON)"),
        ("value_type", "Type hint: string, integer, float, boolean, json"),
        ("description", "Human-readable description of this setting"),
        ("created_at", "When this setting was created"),
        ("updated_at", "When this setting was last modified"),
    ]);

    // timeslots table
    map.insert("timeslots", vec![
        ("guid", "Unique identifier for this timeslot"),
        ("name", "Descriptive name (e.g., 'Morning Energy', 'Evening Chill')"),
        ("start_time", "Day time when timeslot begins (HH:MM:SS format)"),
        ("end_time", "Day time when timeslot ends (HH:MM:SS format)"),
        ("target_flavor", "Target musical flavor vector (JSON) for this time period"),
        ("enabled", "Whether this timeslot is active (1=enabled, 0=disabled)"),
        ("created_at", "When this timeslot was created"),
        ("updated_at", "When this timeslot was last modified"),
    ]);

    // Return the requested table's semantics
    map.get(table_name).map(|cols| {
        cols.iter()
            .map(|(name, desc)| ColumnDescription {
                name: name.to_string(),
                description: desc.to_string(),
            })
            .collect()
    })
}

/// Semantics API errors
#[derive(Debug)]
pub enum SemanticsError {
    InvalidTableName(String),
}

impl IntoResponse for SemanticsError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            SemanticsError::InvalidTableName(name) => {
                (StatusCode::BAD_REQUEST, format!("Invalid table name: {}", name))
            }
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
