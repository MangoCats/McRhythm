//! Enhanced Import Logging
//!
//! Provides detailed logging for import operations showing:
//! - Single-track files: MB matched artist/recording, ID3 tags, durations
//! - Album files: Track-by-track details + album edition info

use anyhow::Result;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

/// Passage information for import logging
#[derive(Debug, FromRow)]
struct PassageInfo {
    #[sqlx(rename = "guid")]
    passage_id: String,
    start_seconds: f64,
    end_seconds: f64,
    song_id: Option<String>,
}

/// Log comprehensive import details for a processed file
///
/// Queries the database to show:
/// - For single-track files: MB artist/recording, ID3 info, durations
/// - For album files: All tracks + album edition info
pub async fn log_file_import_details(
    db: &SqlitePool,
    file_id: &Uuid,
    file_path: &std::path::Path,
) -> Result<()> {
    tracing::debug!(file_id = %file_id, file = ?file_path, "log_file_import_details called");

    // Get file record with ID3 metadata
    let file_record: Option<(
        Option<String>, // id3_artist
        Option<String>, // id3_title
        Option<String>, // id3_album
        i64,            // duration_ticks
    )> = sqlx::query_as(
        "SELECT id3_artist, id3_title, id3_album, duration_ticks
         FROM files WHERE guid = ?",
    )
    .bind(file_id.to_string())
    .fetch_optional(db)
    .await?;

    let (id3_artist, id3_title, id3_album, duration_ticks) = match file_record {
        Some(r) => r,
        None => {
            tracing::warn!(file_id = %file_id, "File record not found");
            return Ok(());
        }
    };

    // Convert ticks to seconds (28_224_000 ticks/sec)
    let file_duration_secs = duration_ticks as f64 / 28_224_000.0;

    // Get all passages for this file
    let passages: Vec<PassageInfo> = sqlx::query_as(
        "SELECT guid, start_seconds, end_seconds, song_id
         FROM passages WHERE file_id = ? ORDER BY start_seconds",
    )
    .bind(file_id.to_string())
    .fetch_all(db)
    .await?;

    if passages.is_empty() {
        tracing::info!(
            file = ?file_path,
            file_duration = format!("{:.2}s", file_duration_secs),
            id3_artist = ?id3_artist,
            id3_title = ?id3_title,
            id3_album = ?id3_album,
            "No passages created for file"
        );
        return Ok(());
    }

    // Check if this is a single-passage file or multi-passage
    if passages.len() == 1 {
        log_single_track_details(
            db,
            file_path,
            &passages[0],
            file_duration_secs,
            id3_artist.as_deref(),
            id3_title.as_deref(),
        )
        .await?;
    } else {
        log_album_details(
            db,
            file_path,
            &passages,
            file_duration_secs,
            id3_artist.as_deref(),
            id3_album.as_deref(),
        )
        .await?;
    }

    Ok(())
}

/// Log details for a single-track file
async fn log_single_track_details(
    db: &SqlitePool,
    file_path: &std::path::Path,
    passage: &PassageInfo,
    file_duration_secs: f64,
    id3_artist: Option<&str>,
    id3_title: Option<&str>,
) -> Result<()> {
    let passage_duration = passage.end_seconds - passage.start_seconds;

    // Get MusicBrainz matched song info if available
    if let Some(song_id) = &passage.song_id {
        let song_info: Option<(
            String, // artist
            String, // title
            Option<i64>, // mb_duration_ms (from MusicBrainz)
        )> = sqlx::query_as(
            "SELECT artist, title, mb_duration_ms FROM songs WHERE guid = ?",
        )
        .bind(song_id)
        .fetch_optional(db)
        .await?;

        if let Some((mb_artist, mb_title, mb_duration_ms)) = song_info {
            let mb_duration_secs = mb_duration_ms.map(|ms| ms as f64 / 1000.0);

            tracing::info!("");
            tracing::info!("┌─ SINGLE TRACK ─────────────────────────────────────");
            tracing::info!("│ File: {}", file_path.display());
            tracing::info!("│");
            tracing::info!("│ ID3 Tags:");
            tracing::info!("│   Artist: {}", id3_artist.unwrap_or("<none>"));
            tracing::info!("│   Title:  {}", id3_title.unwrap_or("<none>"));
            tracing::info!("│");
            tracing::info!("│ MusicBrainz Match:");
            tracing::info!("│   Artist:    {}", mb_artist);
            tracing::info!("│   Recording: {}", mb_title);
            if let Some(mb_dur) = mb_duration_secs {
                tracing::info!("│   MB Duration: {:.2}s", mb_dur);
            } else {
                tracing::info!("│   MB Duration: <unknown>");
            }
            tracing::info!("│");
            tracing::info!("│ Durations:");
            tracing::info!("│   File:    {:.2}s", file_duration_secs);
            tracing::info!("│   Passage: {:.2}s ({:.2}s - {:.2}s)",
                passage_duration, passage.start_seconds, passage.end_seconds);
            tracing::info!("└────────────────────────────────────────────────────");
            tracing::info!("");
        }
    } else {
        tracing::info!("");
        tracing::info!("┌─ SINGLE TRACK (No MB Match) ───────────────────────");
        tracing::info!("│ File: {}", file_path.display());
        tracing::info!("│");
        tracing::info!("│ ID3 Tags:");
        tracing::info!("│   Artist: {}", id3_artist.unwrap_or("<none>"));
        tracing::info!("│   Title:  {}", id3_title.unwrap_or("<none>"));
        tracing::info!("│");
        tracing::info!("│ MusicBrainz: No match found");
        tracing::info!("│");
        tracing::info!("│ Durations:");
        tracing::info!("│   File:    {:.2}s", file_duration_secs);
        tracing::info!("│   Passage: {:.2}s ({:.2}s - {:.2}s)",
            passage_duration, passage.start_seconds, passage.end_seconds);
        tracing::info!("└────────────────────────────────────────────────────");
        tracing::info!("");
    }

    Ok(())
}

/// Log details for an album file (multiple passages)
async fn log_album_details(
    db: &SqlitePool,
    file_path: &std::path::Path,
    passages: &[PassageInfo],
    file_duration_secs: f64,
    id3_artist: Option<&str>,
    id3_album: Option<&str>,
) -> Result<()> {
    // Try to get album edition info from the first passage's song
    let (mb_album_artist, mb_album_title) = if let Some(first_passage) = passages.first() {
        if let Some(song_id) = &first_passage.song_id {
            // Get release info via song -> recording_mbid -> release
            let album_info: Option<(String, String)> = sqlx::query_as(
                "SELECT DISTINCT r.artist, r.title
                 FROM songs s
                 JOIN release_recordings rr ON rr.recording_mbid = s.mbid
                 JOIN releases r ON r.mbid = rr.release_mbid
                 WHERE s.guid = ?
                 LIMIT 1",
            )
            .bind(song_id)
            .fetch_optional(db)
            .await?;

            album_info.unwrap_or(("<unknown>".to_string(), "<unknown>".to_string()))
        } else {
            ("<unknown>".to_string(), "<unknown>".to_string())
        }
    } else {
        ("<unknown>".to_string(), "<unknown>".to_string())
    };

    tracing::info!("");
    tracing::info!("┌─ ALBUM ────────────────────────────────────────────");
    tracing::info!("│ File: {}", file_path.display());
    tracing::info!("│ File Duration: {:.2}s", file_duration_secs);
    tracing::info!("│");
    tracing::info!("│ ID3 Tags:");
    tracing::info!("│   Artist: {}", id3_artist.unwrap_or("<none>"));
    tracing::info!("│   Album:  {}", id3_album.unwrap_or("<none>"));
    tracing::info!("│");
    tracing::info!("│ MusicBrainz Edition:");
    tracing::info!("│   Artist: {}", mb_album_artist);
    tracing::info!("│   Album:  {}", mb_album_title);
    tracing::info!("│");
    tracing::info!("│ Tracks ({}):", passages.len());
    tracing::info!("│");

    for (idx, passage) in passages.iter().enumerate() {
        let track_num = idx + 1;
        let passage_duration = passage.end_seconds - passage.start_seconds;

        // Get song info for this passage
        if let Some(song_id) = &passage.song_id {
            let song_info: Option<(String, String, Option<i64>)> = sqlx::query_as(
                "SELECT artist, title, mb_duration_ms FROM songs WHERE guid = ?",
            )
            .bind(song_id)
            .fetch_optional(db)
            .await?;

            if let Some((artist, title, mb_duration_ms)) = song_info {
                let mb_dur = mb_duration_ms.map(|ms| ms as f64 / 1000.0);
                tracing::info!("│ Track {}: {} - {}", track_num, artist, title);
                tracing::info!("│   Passage: {:.2}s ({:.2}s - {:.2}s)",
                    passage_duration, passage.start_seconds, passage.end_seconds);
                if let Some(mb_d) = mb_dur {
                    tracing::info!("│   MB Duration: {:.2}s", mb_d);
                } else {
                    tracing::info!("│   MB Duration: <unknown>");
                }
                tracing::info!("│");
            }
        } else {
            tracing::info!("│ Track {}: <No MB match>", track_num);
            tracing::info!("│   Passage: {:.2}s ({:.2}s - {:.2}s)",
                passage_duration, passage.start_seconds, passage.end_seconds);
            tracing::info!("│");
        }
    }

    tracing::info!("└────────────────────────────────────────────────────");
    tracing::info!("");

    Ok(())
}
