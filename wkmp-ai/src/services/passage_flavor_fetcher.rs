//! Passage Flavor Fetching for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-016] Flavoring (Phase 9)
//!
//! Fetches musical flavor vectors for songs using AcousticBrainz + Essentia fallback.
//! Updates songs table with flavor_vector JSON and sets status to 'FLAVOR READY'.

use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::path::Path;
use uuid::Uuid;
use wkmp_common::{Error, Result};

use super::acousticbrainz_client::AcousticBrainzClient;
use super::essentia_client::EssentiaClient;
use super::passage_recorder::PassageRecord;

/// Flavor fetch result for a song
#[derive(Debug, Clone)]
pub struct SongFlavorResult {
    /// Song GUID
    pub song_id: Uuid,
    /// Flavor source (AcousticBrainz or Essentia)
    pub flavor_source: FlavorSource,
    /// Whether flavor was successfully fetched
    pub success: bool,
}

/// Flavor source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlavorSource {
    /// Fetched from AcousticBrainz
    AcousticBrainz,
    /// Computed via Essentia
    Essentia,
    /// Zero-song passage (no flavor)
    None,
    /// Failed to fetch/compute
    Failed,
}

impl FlavorSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlavorSource::AcousticBrainz => "AcousticBrainz",
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
    /// Songs fetched from AcousticBrainz
    pub acousticbrainz_count: usize,
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
    acousticbrainz_client: AcousticBrainzClient,
    essentia_client: Option<EssentiaClient>,
}

impl PassageFlavorFetcher {
    /// Create new passage flavor fetcher
    ///
    /// Note: Essentia client is optional. If Essentia binary is not available,
    /// flavor fetching will still work using AcousticBrainz only.
    pub fn new(db: Pool<Sqlite>) -> Result<Self> {
        // Try to create Essentia client, but don't fail if unavailable
        let essentia_client = match EssentiaClient::new() {
            Ok(client) => {
                tracing::info!("Essentia client available for fallback flavor computation");
                Some(client)
            }
            Err(e) => {
                tracing::warn!(error = %e, "Essentia client unavailable, will use AcousticBrainz only");
                None
            }
        };

        Ok(Self {
            db,
            acousticbrainz_client: AcousticBrainzClient::new()
                .map_err(|e| Error::Internal(format!("AcousticBrainz client creation failed: {}", e)))?,
            essentia_client,
        })
    }

    /// Fetch flavor vectors for songs
    ///
    /// **Algorithm:**
    /// 1. Collect unique song IDs from passage records (skip zero-song passages)
    /// 2. For each unique song:
    ///    a. Get MBID from songs table
    ///    b. Query AcousticBrainz API for flavor vector
    ///    c. If AcousticBrainz fails: Fallback to Essentia (local computation)
    ///    d. Update songs.flavor_vector (JSON), songs.flavor_source_blend (JSON array)
    ///    e. Set songs.status = 'FLAVOR READY'
    /// 3. Return flavor result with statistics
    ///
    /// **Traceability:** [REQ-SPEC032-016]
    pub async fn fetch_flavors(
        &self,
        file_path: &Path,
        passages: &[PassageRecord],
    ) -> Result<FlavorResult> {
        tracing::debug!(
            path = %file_path.display(),
            passage_count = passages.len(),
            "Fetching flavor vectors"
        );

        // Collect unique song IDs (skip None for zero-song passages)
        let unique_song_ids: HashSet<Uuid> = passages
            .iter()
            .filter_map(|p| p.song_id)
            .collect();

        tracing::debug!(
            unique_songs = unique_song_ids.len(),
            zero_song_passages = passages.iter().filter(|p| p.song_id.is_none()).count(),
            "Unique songs to process"
        );

        let mut results = Vec::new();
        let mut stats = FlavorStats {
            songs_processed: unique_song_ids.len(),
            acousticbrainz_count: 0,
            essentia_count: 0,
            zero_song_count: passages.iter().filter(|p| p.song_id.is_none()).count(),
            failed_count: 0,
        };

        for song_id in unique_song_ids {
            // Get MBID from songs table
            let mbid: String = sqlx::query_scalar(
                "SELECT recording_mbid FROM songs WHERE guid = ?"
            )
            .bind(song_id.to_string())
            .fetch_one(&self.db)
            .await?;

            tracing::debug!(
                song_id = %song_id,
                mbid,
                "Fetching flavor for song"
            );

            // Try AcousticBrainz first
            let (flavor_source, success) = match self.acousticbrainz_client.get_flavor_vector(&mbid).await {
                Ok(flavor_vector) => {
                    // Success: Update songs table
                    let flavor_json = serde_json::to_string(&flavor_vector)
                        .map_err(|e| Error::Internal(format!("JSON serialization failed: {}", e)))?;

                    sqlx::query(
                        r#"
                        UPDATE songs
                        SET flavor_vector = ?,
                            flavor_source_blend = '["AcousticBrainz"]',
                            status = 'FLAVOR READY',
                            updated_at = CURRENT_TIMESTAMP
                        WHERE guid = ?
                        "#
                    )
                    .bind(flavor_json)
                    .bind(song_id.to_string())
                    .execute(&self.db)
                    .await?;

                    tracing::debug!(
                        song_id = %song_id,
                        mbid,
                        "Flavor fetched from AcousticBrainz"
                    );

                    stats.acousticbrainz_count += 1;
                    (FlavorSource::AcousticBrainz, true)
                }
                Err(ab_error) => {
                    // AcousticBrainz failed: Try Essentia fallback if available
                    if let Some(ref essentia) = self.essentia_client {
                        tracing::warn!(
                            song_id = %song_id,
                            mbid,
                            error = %ab_error,
                            "AcousticBrainz failed, trying Essentia fallback"
                        );

                        match essentia.analyze_file(file_path).await {
                        Ok(flavor_vector) => {
                            // Success: Update songs table
                            let flavor_json = serde_json::to_string(&flavor_vector)
                                .map_err(|e| Error::Internal(format!("JSON serialization failed: {}", e)))?;

                            sqlx::query(
                                r#"
                                UPDATE songs
                                SET flavor_vector = ?,
                                    flavor_source_blend = '["Essentia"]',
                                    status = 'FLAVOR READY',
                                    updated_at = CURRENT_TIMESTAMP
                                WHERE guid = ?
                                "#
                            )
                            .bind(flavor_json)
                            .bind(song_id.to_string())
                            .execute(&self.db)
                            .await?;

                            tracing::debug!(
                                song_id = %song_id,
                                mbid,
                                "Flavor computed via Essentia"
                            );

                            stats.essentia_count += 1;
                            (FlavorSource::Essentia, true)
                        }
                        Err(essentia_error) => {
                            // Both failed
                            tracing::error!(
                                song_id = %song_id,
                                mbid,
                                ab_error = %ab_error,
                                essentia_error = %essentia_error,
                                "Failed to fetch flavor from both AcousticBrainz and Essentia"
                            );

                            stats.failed_count += 1;
                            (FlavorSource::Failed, false)
                        }
                        }
                    } else {
                        // Essentia not available
                        tracing::error!(
                            song_id = %song_id,
                            mbid,
                            ab_error = %ab_error,
                            "AcousticBrainz failed and Essentia not available"
                        );

                        stats.failed_count += 1;
                        (FlavorSource::Failed, false)
                    }
                }
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
            acousticbrainz = stats.acousticbrainz_count,
            essentia = stats.essentia_count,
            failed = stats.failed_count,
            "Flavor fetching complete"
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

        // Create songs table
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
            "#
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
        // Just verify it can be created
    }

    #[test]
    fn test_flavor_source_as_str() {
        assert_eq!(FlavorSource::AcousticBrainz.as_str(), "AcousticBrainz");
        assert_eq!(FlavorSource::Essentia.as_str(), "Essentia");
        assert_eq!(FlavorSource::None.as_str(), "None");
        assert_eq!(FlavorSource::Failed.as_str(), "Failed");
    }

    #[test]
    fn test_flavor_stats_empty() {
        let stats = FlavorStats {
            songs_processed: 0,
            acousticbrainz_count: 0,
            essentia_count: 0,
            zero_song_count: 0,
            failed_count: 0,
        };

        assert_eq!(stats.songs_processed, 0);
        assert_eq!(stats.acousticbrainz_count, 0);
    }

    #[tokio::test]
    async fn test_fetch_flavors_empty() {
        let pool = setup_test_db().await;
        let fetcher = PassageFlavorFetcher::new(pool).unwrap();

        let passages = vec![];
        let result = fetcher.fetch_flavors(Path::new("/test/file.mp3"), &passages).await.unwrap();

        assert_eq!(result.stats.songs_processed, 0);
        assert_eq!(result.songs.len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_flavors_zero_song_only() {
        let pool = setup_test_db().await;
        let fetcher = PassageFlavorFetcher::new(pool).unwrap();

        // Create passage with no song_id (zero-song passage)
        let passages = vec![PassageRecord {
            passage_id: Uuid::new_v4(),
            song_id: None,
            song_created: false,
        }];

        let result = fetcher.fetch_flavors(Path::new("/test/file.mp3"), &passages).await.unwrap();

        assert_eq!(result.stats.songs_processed, 0); // No songs to process
        assert_eq!(result.stats.zero_song_count, 1);
        assert_eq!(result.songs.len(), 0);
    }
}
