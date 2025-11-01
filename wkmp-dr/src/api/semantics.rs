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
        ("guid", "Unique identifier for this song (MusicBrainz recording)"),
        ("mbid", "MusicBrainz Recording ID"),
        ("title", "Song title from MusicBrainz"),
        ("duration_ms", "Total duration in milliseconds"),
        ("created_at", "When this record was created"),
        ("updated_at", "When this record was last updated"),
    ]);

    // passages table
    map.insert("passages", vec![
        ("guid", "Unique identifier for this passage"),
        ("file_guid", "References files.guid - which audio file contains this passage"),
        ("passage_number", "Ordinal position within file (1-based)"),
        ("title", "Optional descriptive title for this passage"),
        ("start_sample", "First playable audio sample (0-based, sample-accurate)"),
        ("end_sample", "Last playable audio sample (inclusive)"),
        ("crossfade_start_sample", "When outgoing crossfade may begin"),
        ("crossfade_end_sample", "When outgoing crossfade must complete"),
        ("fade_in_curve", "Incoming fade curve: linear, exponential, logarithmic, S-curve, cosine"),
        ("fade_out_curve", "Outgoing fade curve: linear, exponential, logarithmic, S-curve, cosine"),
        ("base_probability", "Selection weight 0.0-1.0, default 1.0 for normal selection"),
        ("musical_flavor", "AcousticBrainz feature vector (JSON) characterizing sonic properties"),
        ("last_played_at", "Timestamp of most recent playback (NULL if never played)"),
        ("created_at", "When this record was created"),
        ("updated_at", "When this record was last updated"),
    ]);

    // files table
    map.insert("files", vec![
        ("guid", "Unique identifier for this audio file"),
        ("path", "File path relative to root folder, forward-slash separated"),
        ("format", "Audio format: FLAC, MP3, AAC, etc."),
        ("sample_rate", "Sample rate in Hz (e.g., 44100, 48000)"),
        ("channels", "Number of audio channels (1=mono, 2=stereo)"),
        ("duration_samples", "Total file length in samples"),
        ("file_hash", "SHA-256 hash for duplicate detection"),
        ("file_size_bytes", "File size in bytes"),
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
