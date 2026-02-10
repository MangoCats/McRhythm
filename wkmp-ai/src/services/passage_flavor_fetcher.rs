//! Passage Flavor Fetching for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-016] Flavoring (Phase 9)
//!
//! **[PLAN035]** Essentia-only flavoring. AcousticBrainz has been shut down.
//! Uses local Essentia analysis (native binary or Docker container) to compute
//! musical flavor vectors for songs.

use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::path::Path;
use uuid::Uuid;
use wkmp_common::{Error, Result};

use super::essentia_client::EssentiaClient;
use super::passage_recorder::PassageRecord;
use crate::utils::retry_on_lock;

/// Flavor fetch result for a song
#[derive(Debug, Clone)]
pub struct SongFlavorResult {
    /// Song GUID
    pub song_id: Uuid,
    /// Flavor source
    pub flavor_source: FlavorSource,
    /// Whether flavor was successfully fetched
    pub success: bool,
}

/// Flavor source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlavorSource {
    /// Computed via Essentia
    Essentia,
    /// Zero-song passage (no flavor)
    None,
    /// Failed to compute
    Failed,
}

impl FlavorSource {
    /// Convert to display string
    pub fn as_str(&self) -> &'static str {
        match self {
            FlavorSource::Essentia => "Essentia",
            FlavorSource::None => "None",
            FlavorSource::Failed => "Failed",
        }
    }
}

/// Flavor fetching result
#[derive(Debug, Clone)]
pub struct FlavorResult {
    /// Flavor results per song
    pub songs: Vec<SongFlavorResult>,
    /// Statistics
    pub stats: FlavorStats,
}

/// Flavor fetching statistics
#[derive(Debug, Clone)]
pub struct FlavorStats {
    /// Total songs processed
    pub songs_processed: usize,
    /// Songs computed via Essentia
    pub essentia_count: usize,
    /// Zero-song passages skipped
    pub zero_song_count: usize,
    /// Failed fetches
    pub failed_count: usize,
}

/// Passage Flavor Fetcher
///
/// **Traceability:** [REQ-SPEC032-016] (Phase 9: FLAVORING)
pub struct PassageFlavorFetcher {
    db: Pool<Sqlite>,
}

impl PassageFlavorFetcher {
    /// Create new passage flavor fetcher
    pub fn new(db: Pool<Sqlite>) -> Result<Self> {
        Ok(Self { db })
    }

    /// Fetch flavor vectors for songs via Essentia
    ///
    /// **[PLAN035]** Essentia-only. `essentia_client` is passed by reference from the
    /// orchestrator (shared across all files).
    ///
    /// **Traceability:** [REQ-SPEC032-016]
    pub async fn fetch_flavors(
        &self,
        file_path: &Path,
        passages: &[PassageRecord],
        essentia_client: Option<&EssentiaClient>,
    ) -> Result<FlavorResult> {
        tracing::debug!(
            path = %file_path.display(),
            passage_count = passages.len(),
            "Fetching flavor vectors via Essentia"
        );

        // Get max lock wait time from settings (default 5000ms)
        let max_wait_ms: i64 = sqlx::query_scalar(
            "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'ai_database_max_lock_wait_ms'"
        )
        .fetch_optional(&self.db)
        .await?
        .unwrap_or(5000);

        // Collect unique song IDs (skip None for zero-song passages)
        let unique_song_ids: HashSet<Uuid> = passages.iter().filter_map(|p| p.song_id).collect();

        tracing::debug!(
            unique_songs = unique_song_ids.len(),
            zero_song_passages = passages.iter().filter(|p| p.song_id.is_none()).count(),
            "Unique songs to process"
        );

        let mut results = Vec::new();
        let mut stats = FlavorStats {
            songs_processed: unique_song_ids.len(),
            essentia_count: 0,
            zero_song_count: passages.iter().filter(|p| p.song_id.is_none()).count(),
            failed_count: 0,
        };

        // Early return if no Essentia client available
        if essentia_client.is_none() && !unique_song_ids.is_empty() {
            tracing::warn!(
                songs = unique_song_ids.len(),
                "Essentia not available — all songs will be marked FLAVORING FAILED"
            );
        }

        for song_id in unique_song_ids {
            let song_id_str = song_id.to_string();

            tracing::debug!(song_id = %song_id, "Computing flavor via Essentia");

            let (flavor_source, success) = if let Some(essentia) = essentia_client {
                match essentia.analyze_file(file_path).await {
                    Ok(flavor_vector) => {
                        let flavor_json =
                            serde_json::to_string(&flavor_vector).map_err(|e| {
                                Error::Internal(format!("JSON serialization failed: {}", e))
                            })?;

                        let db_ref = &self.db;
                        let sid = song_id_str.clone();
                        retry_on_lock(
                            "song flavor update (Essentia)",
                            max_wait_ms as u64,
                            || async {
                                sqlx::query(
                                    r#"
                                    UPDATE songs
                                    SET flavor_vector = ?,
                                        flavor_source_blend = '["Essentia"]',
                                        status = 'FLAVOR READY',
                                        updated_at = CURRENT_TIMESTAMP
                                    WHERE guid = ?
                                    "#,
                                )
                                .bind(&flavor_json)
                                .bind(&sid)
                                .execute(db_ref)
                                .await
                                .map_err(|e| Error::Database(e))
                            },
                        )
                        .await?;

                        tracing::debug!(song_id = %song_id, "Flavor computed via Essentia");

                        stats.essentia_count += 1;
                        (FlavorSource::Essentia, true)
                    }
                    Err(essentia_error) => {
                        tracing::error!(
                            song_id = %song_id,
                            error = ?essentia_error,
                            "Essentia analysis failed"
                        );

                        if let Err(e) = sqlx::query(
                            "UPDATE songs SET status = 'FLAVORING FAILED', updated_at = CURRENT_TIMESTAMP WHERE guid = ?"
                        )
                        .bind(&song_id_str)
                        .execute(&self.db)
                        .await
                        {
                            tracing::error!(song_id = %song_id, error = ?e, "Failed to update song status to FLAVORING FAILED");
                        }

                        stats.failed_count += 1;
                        (FlavorSource::Failed, false)
                    }
                }
            } else {
                // Essentia not available
                if let Err(e) = sqlx::query(
                    "UPDATE songs SET status = 'FLAVORING FAILED', updated_at = CURRENT_TIMESTAMP WHERE guid = ?"
                )
                .bind(&song_id_str)
                .execute(&self.db)
                .await
                {
                    tracing::error!(song_id = %song_id, error = ?e, "Failed to update song status to FLAVORING FAILED");
                }

                stats.failed_count += 1;
                (FlavorSource::Failed, false)
            };

            results.push(SongFlavorResult {
                song_id,
                flavor_source,
                success,
            });
        }

        tracing::info!(
            path = %file_path.display(),
            songs_processed = stats.songs_processed,
            essentia = stats.essentia_count,
            failed = stats.failed_count,
            "Flavor fetching complete"
        );

        Ok(FlavorResult {
            songs: results,
            stats,
        })
    }
    /// Fetch flavor vectors using pre-decoded audio (per-passage analysis)
    ///
    /// For album files, this extracts each passage's audio to a temp WAV file
    /// and analyzes it individually, so each passage gets its own distinct
    /// flavor vector instead of all sharing the whole-album average.
    ///
    /// **Traceability:** [REQ-SPEC032-016] Phase 9: FLAVORING
    pub async fn fetch_flavors_with_audio(
        &self,
        decoded_samples: &[f32],
        sample_rate: u32,
        passages: &[PassageRecord],
        essentia_client: Option<&EssentiaClient>,
        music_root: &Path,
    ) -> Result<FlavorResult> {
        tracing::debug!(
            total_samples = decoded_samples.len(),
            sample_rate = sample_rate,
            passage_count = passages.len(),
            "Fetching flavor vectors from pre-decoded audio (per-passage)"
        );

        let max_wait_ms: i64 = sqlx::query_scalar(
            "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'ai_database_max_lock_wait_ms'"
        )
        .fetch_optional(&self.db)
        .await?
        .unwrap_or(5000);

        let unique_song_ids: HashSet<Uuid> = passages.iter().filter_map(|p| p.song_id).collect();

        tracing::debug!(
            unique_songs = unique_song_ids.len(),
            zero_song_passages = passages.iter().filter(|p| p.song_id.is_none()).count(),
            "Unique songs to process (per-passage)"
        );

        let mut results = Vec::new();
        let mut stats = FlavorStats {
            songs_processed: unique_song_ids.len(),
            essentia_count: 0,
            zero_song_count: passages.iter().filter(|p| p.song_id.is_none()).count(),
            failed_count: 0,
        };

        if essentia_client.is_none() && !unique_song_ids.is_empty() {
            tracing::warn!(
                songs = unique_song_ids.len(),
                "Essentia not available — all songs will be marked FLAVORING FAILED"
            );
        }

        // Create temp directory under music_root (accessible to Docker via volume mount)
        let temp_dir = music_root.join(".wkmp_temp");
        if essentia_client.is_some() {
            std::fs::create_dir_all(&temp_dir).map_err(|e| {
                Error::Internal(format!(
                    "Failed to create temp directory {}: {}",
                    temp_dir.display(),
                    e
                ))
            })?;
        }

        const TICKS_PER_SECOND: f64 = 28_224_000.0;

        for song_id in unique_song_ids {
            let song_id_str = song_id.to_string();

            // Find the passage for this song to get timing boundaries
            let passage = passages
                .iter()
                .find(|p| p.song_id == Some(song_id))
                .expect("Song ID must correspond to at least one passage");

            // Query passage boundaries from database
            let row: Option<(i64, i64)> = sqlx::query_as(
                "SELECT start_time_ticks, end_time_ticks FROM passages WHERE guid = ?",
            )
            .bind(passage.passage_id.to_string())
            .fetch_optional(&self.db)
            .await?;

            let (start_ticks, end_ticks) = match row {
                Some(r) => r,
                None => {
                    tracing::error!(
                        song_id = %song_id,
                        passage_id = %passage.passage_id,
                        "Passage not found in database"
                    );
                    stats.failed_count += 1;
                    results.push(SongFlavorResult {
                        song_id,
                        flavor_source: FlavorSource::Failed,
                        success: false,
                    });
                    continue;
                }
            };

            let start_seconds = start_ticks as f64 / TICKS_PER_SECOND;
            let end_seconds = end_ticks as f64 / TICKS_PER_SECOND;
            let start_sample = (start_seconds * sample_rate as f64) as usize;
            let end_sample = ((end_seconds * sample_rate as f64) as usize).min(decoded_samples.len());

            if start_sample >= decoded_samples.len() || start_sample >= end_sample {
                tracing::error!(
                    song_id = %song_id,
                    start_sample,
                    end_sample,
                    total_samples = decoded_samples.len(),
                    "Passage sample range out of bounds"
                );
                stats.failed_count += 1;
                results.push(SongFlavorResult {
                    song_id,
                    flavor_source: FlavorSource::Failed,
                    success: false,
                });
                continue;
            }

            let passage_samples = &decoded_samples[start_sample..end_sample];

            tracing::debug!(
                song_id = %song_id,
                start_seconds = format!("{:.2}", start_seconds),
                end_seconds = format!("{:.2}", end_seconds),
                sample_count = passage_samples.len(),
                "Extracting passage for Essentia analysis"
            );

            let (flavor_source, success) = if let Some(essentia) = essentia_client {
                // Write passage audio to temp WAV
                let temp_file = temp_dir.join(format!(
                    "passage_{}_{}.wav",
                    song_id,
                    uuid::Uuid::new_v4()
                ));

                let analysis_result = async {
                    crate::utils::write_samples_to_wav(passage_samples, sample_rate, &temp_file)
                        .map_err(|e| {
                            Error::Internal(format!("Failed to write temp WAV: {}", e))
                        })?;

                    essentia.analyze_file(&temp_file).await.map_err(|e| {
                        Error::Internal(format!("Essentia analysis failed: {}", e))
                    })
                }
                .await;

                // Always clean up temp file
                if temp_file.exists() {
                    let _ = std::fs::remove_file(&temp_file);
                }

                match analysis_result {
                    Ok(flavor_vector) => {
                        let flavor_json =
                            serde_json::to_string(&flavor_vector).map_err(|e| {
                                Error::Internal(format!("JSON serialization failed: {}", e))
                            })?;

                        let db_ref = &self.db;
                        let sid = song_id_str.clone();
                        retry_on_lock(
                            "song flavor update (Essentia per-passage)",
                            max_wait_ms as u64,
                            || async {
                                sqlx::query(
                                    r#"
                                    UPDATE songs
                                    SET flavor_vector = ?,
                                        flavor_source_blend = '["Essentia"]',
                                        status = 'FLAVOR READY',
                                        updated_at = CURRENT_TIMESTAMP
                                    WHERE guid = ?
                                    "#,
                                )
                                .bind(&flavor_json)
                                .bind(&sid)
                                .execute(db_ref)
                                .await
                                .map_err(|e| Error::Database(e))
                            },
                        )
                        .await?;

                        tracing::debug!(
                            song_id = %song_id,
                            duration_seconds = format!("{:.1}", end_seconds - start_seconds),
                            "Flavor computed via Essentia (per-passage)"
                        );

                        stats.essentia_count += 1;
                        (FlavorSource::Essentia, true)
                    }
                    Err(e) => {
                        tracing::error!(
                            song_id = %song_id,
                            error = %e,
                            "Essentia per-passage analysis failed"
                        );

                        if let Err(e) = sqlx::query(
                            "UPDATE songs SET status = 'FLAVORING FAILED', updated_at = CURRENT_TIMESTAMP WHERE guid = ?"
                        )
                        .bind(&song_id_str)
                        .execute(&self.db)
                        .await
                        {
                            tracing::error!(song_id = %song_id, error = ?e, "Failed to update song status");
                        }

                        stats.failed_count += 1;
                        (FlavorSource::Failed, false)
                    }
                }
            } else {
                // Essentia not available
                if let Err(e) = sqlx::query(
                    "UPDATE songs SET status = 'FLAVORING FAILED', updated_at = CURRENT_TIMESTAMP WHERE guid = ?"
                )
                .bind(&song_id_str)
                .execute(&self.db)
                .await
                {
                    tracing::error!(song_id = %song_id, error = ?e, "Failed to update song status");
                }

                stats.failed_count += 1;
                (FlavorSource::Failed, false)
            };

            results.push(SongFlavorResult {
                song_id,
                flavor_source,
                success,
            });
        }

        tracing::info!(
            songs_processed = stats.songs_processed,
            essentia = stats.essentia_count,
            failed = stats.failed_count,
            "Flavor fetching complete (per-passage from pre-decoded audio)"
        );

        Ok(FlavorResult {
            songs: results,
            stats,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        sqlx::query(
            r#"
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO settings (key, value) VALUES ('ai_database_max_lock_wait_ms', '5000')",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE songs (
                guid TEXT PRIMARY KEY,
                recording_mbid TEXT NOT NULL,
                base_probability REAL NOT NULL DEFAULT 1.0,
                min_cooldown INTEGER NOT NULL DEFAULT 604800,
                ramping_cooldown INTEGER NOT NULL DEFAULT 1209600,
                flavor_vector TEXT,
                flavor_source_blend TEXT,
                status TEXT DEFAULT 'PENDING',
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_fetcher_creation() {
        let pool = setup_test_db().await;
        let _fetcher = PassageFlavorFetcher::new(pool).unwrap();
    }

    #[test]
    fn test_flavor_source_as_str() {
        assert_eq!(FlavorSource::Essentia.as_str(), "Essentia");
        assert_eq!(FlavorSource::None.as_str(), "None");
        assert_eq!(FlavorSource::Failed.as_str(), "Failed");
    }

    #[test]
    fn test_flavor_stats_empty() {
        let stats = FlavorStats {
            songs_processed: 0,
            essentia_count: 0,
            zero_song_count: 0,
            failed_count: 0,
        };

        assert_eq!(stats.songs_processed, 0);
        assert_eq!(stats.essentia_count, 0);
    }

    #[tokio::test]
    async fn test_fetch_flavors_empty() {
        let pool = setup_test_db().await;
        let fetcher = PassageFlavorFetcher::new(pool).unwrap();

        let passages = vec![];
        let result = fetcher
            .fetch_flavors(Path::new("/test/file.mp3"), &passages, None)
            .await
            .unwrap();

        assert_eq!(result.stats.songs_processed, 0);
        assert_eq!(result.songs.len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_flavors_zero_song_only() {
        let pool = setup_test_db().await;
        let fetcher = PassageFlavorFetcher::new(pool).unwrap();

        let passages = vec![PassageRecord {
            passage_id: Uuid::new_v4(),
            song_id: None,
            song_created: false,
        }];

        let result = fetcher
            .fetch_flavors(Path::new("/test/file.mp3"), &passages, None)
            .await
            .unwrap();

        assert_eq!(result.stats.songs_processed, 0);
        assert_eq!(result.stats.zero_song_count, 1);
        assert_eq!(result.songs.len(), 0);
    }
}
