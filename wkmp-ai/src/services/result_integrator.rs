//! Result Integration Service
//!
//! **[PLAN026]** Increment 6: Result Integration & Entity Creation
//!
//! Persists ClassificationResult to database, creating:
//! - Updated Files record with content_type and match metadata
//! - Song/Artist/Album entities from MusicBrainz data
//! - Passage-to-Album relationships

use anyhow::Result;
use sqlx::SqlitePool;
use tracing::{debug, info};
use uuid::Uuid;

use super::content_type_classifier::{ClassificationResult, ContentType};
use crate::db::albums::Album;
use crate::db::songs::Song;

// Note: MatchConfidence available via ClassificationResult.confidence

/// Result integrator service
///
/// **[PLAN026]** Persists classification results to database
pub struct ResultIntegrator {
    /// Database connection pool
    pool: SqlitePool,
}

impl ResultIntegrator {
    /// Create new result integrator
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Update file with classification result
    ///
    /// **[REQ-DB-001]** Updates Files table with am28 classification columns:
    /// - content_type: Classification result (SINGLE_SONG, FULL_ALBUM, etc.)
    /// - matched_release_mbid: Best MusicBrainz Release MBID
    /// - match_confidence: Confidence level string
    /// - match_percentage: Percentage of tracks matched
    /// - artist_verified: Whether artist verification passed
    pub async fn update_file_classification(
        &self,
        file_id: Uuid,
        result: &ClassificationResult,
    ) -> Result<()> {
        info!(
            file_id = %file_id,
            content_type = %result.content_type.as_str(),
            confidence = %result.confidence.as_str(),
            "Updating file classification"
        );

        sqlx::query(
            r#"
            UPDATE files
            SET content_type = ?,
                matched_release_mbid = ?,
                match_confidence = ?,
                match_percentage = ?,
                artist_verified = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE guid = ?
            "#,
        )
        .bind(result.content_type.as_str())
        .bind(&result.release_mbid)
        .bind(result.confidence.as_str())
        .bind(result.match_percentage)
        .bind(if result.artist_verified { 1 } else { 0 })
        .bind(file_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create or update song from classification result
    ///
    /// **[REQ-DB-003]** Creates Song record from recording MBID
    ///
    /// # Returns
    /// Song UUID if created/found, None if no recording MBID
    pub async fn ensure_song(
        &self,
        recording_mbid: &str,
        title: Option<&str>,
        _artist_name: Option<&str>,  // Reserved for future Artist entity creation
    ) -> Result<Option<Uuid>> {
        // Check if song already exists
        if let Some(existing) = crate::db::songs::load_song_by_mbid(&self.pool, recording_mbid).await? {
            debug!(
                recording_mbid = %recording_mbid,
                song_id = %existing.guid,
                "Song already exists"
            );
            return Ok(Some(existing.guid));
        }

        // Create new song
        // Note: artist_name not stored in Song entity - only recording_mbid and title
        let song = Song::new(
            recording_mbid.to_string(),
            title.map(|s| s.to_string()),
        );

        crate::db::songs::save_song(&self.pool, &song).await?;

        info!(
            recording_mbid = %recording_mbid,
            song_id = %song.guid,
            title = ?title,
            "Created new song"
        );

        Ok(Some(song.guid))
    }

    /// Create or update album from classification result
    ///
    /// **[REQ-DB-005]** Creates Album record from release MBID
    ///
    /// # Returns
    /// Album UUID if created/found, None if no release MBID
    pub async fn ensure_album(
        &self,
        release_mbid: &str,
        title: &str,
        artist_credit: Option<&str>,
        country: Option<&str>,
        status: Option<&str>,
    ) -> Result<Uuid> {
        // Check if album already exists
        if let Some(existing) = crate::db::albums::load_album_by_mbid(&self.pool, release_mbid).await? {
            debug!(
                release_mbid = %release_mbid,
                album_id = %existing.guid,
                "Album already exists"
            );
            return Ok(existing.guid);
        }

        // Create new album
        let album = Album::new_with_details(
            release_mbid.to_string(),
            title.to_string(),
            None, // release_date - not available from ClassificationResult
            artist_credit.map(|s| s.to_string()),
            country.map(|s| s.to_string()),
            status.map(|s| s.to_string()),
        );

        crate::db::albums::save_album(&self.pool, &album).await?;

        info!(
            release_mbid = %release_mbid,
            album_id = %album.guid,
            title = %title,
            "Created new album"
        );

        Ok(album.guid)
    }

    /// Link passage to album
    ///
    /// **[REQ-DB-007]** Creates passage_albums relationship
    pub async fn link_passage_to_album(
        &self,
        passage_id: Uuid,
        album_id: Uuid,
    ) -> Result<()> {
        crate::db::albums::link_passage_to_album(&self.pool, passage_id, album_id).await?;

        debug!(
            passage_id = %passage_id,
            album_id = %album_id,
            "Linked passage to album"
        );

        Ok(())
    }

    /// Update passage with matching details
    ///
    /// **[REQ-DB-002]** Updates Passages table with am28 matching columns:
    /// - track_number, disc_number
    /// - matching_stage
    /// - mean_error_seconds
    /// - passage_match_percentage
    pub async fn update_passage_matching(
        &self,
        passage_id: Uuid,
        track_number: Option<i32>,
        disc_number: Option<i32>,
        matching_stage: Option<&str>,
        mean_error_seconds: Option<f64>,
        match_percentage: Option<f64>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE passages
            SET track_number = ?,
                disc_number = ?,
                matching_stage = ?,
                mean_error_seconds = ?,
                passage_match_percentage = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE guid = ?
            "#,
        )
        .bind(track_number)
        .bind(disc_number)
        .bind(matching_stage)
        .bind(mean_error_seconds)
        .bind(match_percentage)
        .bind(passage_id.to_string())
        .execute(&self.pool)
        .await?;

        debug!(
            passage_id = %passage_id,
            track_number = ?track_number,
            matching_stage = ?matching_stage,
            "Updated passage matching details"
        );

        Ok(())
    }

    /// Full integration of classification result
    ///
    /// **[REQ-STEP-007]** Complete result integration:
    /// 1. Update file with classification
    /// 2. Create song if single-song match
    /// 3. Create album if album match
    /// 4. Link passages to album
    ///
    /// # Arguments
    /// * `file_id` - File UUID
    /// * `result` - Classification result
    /// * `title` - Optional title for song creation
    /// * `artist_name` - Optional artist name
    ///
    /// # Returns
    /// Tuple of (song_id, album_id) if created
    pub async fn integrate_result(
        &self,
        file_id: Uuid,
        result: &ClassificationResult,
        title: Option<&str>,
        artist_name: Option<&str>,
    ) -> Result<(Option<Uuid>, Option<Uuid>)> {
        // 1. Update file classification
        self.update_file_classification(file_id, result).await?;

        let mut song_id = None;
        let mut album_id = None;

        // 2. Handle based on content type
        match result.content_type {
            ContentType::SingleSong => {
                // Create song from recording MBID
                if let Some(ref recording_mbid) = result.recording_mbid {
                    song_id = self.ensure_song(recording_mbid, title, artist_name).await?;
                }
            }
            ContentType::FullAlbum | ContentType::PartialAlbum => {
                // Create album from release MBID
                if let Some(ref release_mbid) = result.release_mbid {
                    let album_title = title.unwrap_or("Unknown Album");
                    let new_album_id = self.ensure_album(
                        release_mbid,
                        album_title,
                        None, // artist_credit - would come from AlbumMatchResult
                        None, // country
                        None, // status
                    ).await?;
                    album_id = Some(new_album_id);
                }
            }
            ContentType::MultipleSongs => {
                // Multiple songs in file - each passage will have its own song
                // Handled separately via integrate_passage_result
            }
            ContentType::NotInMusicbrainz | ContentType::IdentificationFailed => {
                // No entities to create
                debug!(
                    file_id = %file_id,
                    content_type = %result.content_type.as_str(),
                    "No entities to create for this content type"
                );
            }
        }

        info!(
            file_id = %file_id,
            content_type = %result.content_type.as_str(),
            song_id = ?song_id,
            album_id = ?album_id,
            "Result integration complete"
        );

        Ok((song_id, album_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_result_integrator_creation() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        let integrator = ResultIntegrator::new(pool);
        // Just verifies creation doesn't panic
        assert!(true);
    }
}
