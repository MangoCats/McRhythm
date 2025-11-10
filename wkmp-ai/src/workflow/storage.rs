//! Database Storage for Processed Passages
//!
//! PLAN024: Storage for 3-tier hybrid fusion results
//! SPEC017 Compliance: Stores passage times as INTEGER ticks, not REAL seconds

use super::{ProcessedPassage, TICK_RATE};
use anyhow::{Context, Result};
use sqlx::SqlitePool;
use tracing::{debug, info};
use uuid::Uuid;

/// Store a processed passage in the database
///
/// # Arguments
/// * `db` - Database connection pool
/// * `file_path` - Source audio file path
/// * `passage` - Processed passage with fusion and validation results
/// * `import_session_id` - UUID for this import session
///
/// # Returns
/// * Passage UUID (GUID)
pub async fn store_passage(
    db: &SqlitePool,
    file_path: &str,
    passage: &ProcessedPassage,
    import_session_id: &str,
) -> Result<String> {
    let passage_id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp();

    // Extract fusion result data
    let fusion = &passage.fusion;
    let validation = &passage.validation;

    // Extract metadata fields (each is Option<ConfidenceValue<String>>)
    let (title, title_source, title_confidence) = fusion
        .metadata
        .title
        .as_ref()
        .map(|cv| (Some(cv.value.clone()), Some(cv.source.clone()), cv.confidence))
        .unwrap_or((None, None, 0.0));

    let (artist, artist_source, artist_confidence) = fusion
        .metadata
        .artist
        .as_ref()
        .map(|cv| (Some(cv.value.clone()), Some(cv.source.clone()), cv.confidence))
        .unwrap_or((None, None, 0.0));

    let (album, album_source, album_confidence) = fusion
        .metadata
        .album
        .as_ref()
        .map(|cv| (Some(cv.value.clone()), Some(cv.source.clone()), cv.confidence))
        .unwrap_or((None, None, 0.0));

    let (recording_mbid, mbid_source, mbid_confidence) = fusion
        .metadata
        .recording_mbid
        .as_ref()
        .map(|cv| (Some(cv.value.clone()), Some(cv.source.clone()), cv.confidence))
        .unwrap_or((None, None, 0.0));

    // Serialize JSON fields
    let flavor_json = serde_json::to_string(&fusion.flavor.characteristics)
        .context("Failed to serialize flavor characteristics")?;

    let flavor_source_blend = serde_json::to_string(&fusion.flavor.source_blend)
        .context("Failed to serialize flavor source blend")?;

    let flavor_confidence_map = serde_json::to_string(&fusion.flavor.confidence_map)
        .context("Failed to serialize flavor confidence map")?;

    let identity_conflicts = if !fusion.identity.conflicts.is_empty() {
        Some(serde_json::to_string(&fusion.identity.conflicts)?)
    } else {
        None
    };

    let validation_report = serde_json::to_string(&validation.report)
        .context("Failed to serialize validation report")?;

    let validation_issues = if !validation.issues.is_empty() {
        Some(serde_json::to_string(&validation.issues)?)
    } else {
        None
    };

    // Insert passage record (SPEC017: times as INTEGER ticks)
    let query = r#"
        INSERT INTO passages (
            guid,
            file_path,
            start_time,
            end_time,
            title,
            artist,
            album,
            recording_mbid,
            musical_flavor,
            flavor_source_blend,
            flavor_confidence_map,
            flavor_completeness,
            title_source,
            title_confidence,
            artist_source,
            artist_confidence,
            album_source,
            album_confidence,
            mbid_source,
            mbid_confidence,
            identity_confidence,
            identity_posterior_probability,
            identity_conflicts,
            overall_quality_score,
            metadata_completeness,
            validation_status,
            validation_report,
            validation_issues,
            import_session_id,
            import_timestamp,
            import_strategy
        ) VALUES (
            ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?
        )
    "#;

    let validation_status_str = format!("{:?}", validation.status);

    sqlx::query(query)
        .bind(&passage_id)
        .bind(file_path)
        .bind(passage.boundary.start_time)
        .bind(passage.boundary.end_time)
        .bind(&title)
        .bind(&artist)
        .bind(&album)
        .bind(&recording_mbid)
        .bind(&flavor_json)
        .bind(&flavor_source_blend)
        .bind(&flavor_confidence_map)
        .bind(fusion.flavor.completeness)
        .bind(&title_source)
        .bind(title_confidence)
        .bind(&artist_source)
        .bind(artist_confidence)
        .bind(&album_source)
        .bind(album_confidence)
        .bind(&mbid_source)
        .bind(mbid_confidence)
        .bind(fusion.identity.confidence)
        .bind(fusion.identity.posterior_probability)
        .bind(&identity_conflicts)
        .bind(validation.score)
        .bind(fusion.metadata.metadata_completeness)
        .bind(&validation_status_str)
        .bind(&validation_report)
        .bind(&validation_issues)
        .bind(import_session_id)
        .bind(timestamp)
        .bind("hybrid_fusion_v2") // import_strategy (PLAN024)
        .execute(db)
        .await
        .context("Failed to insert passage")?;

    // Convert ticks to seconds for logging display
    let start_seconds = passage.boundary.start_time as f64 / TICK_RATE as f64;
    let end_seconds = passage.boundary.end_time as f64 / TICK_RATE as f64;

    info!(
        "Stored passage {} ({:.1}s-{:.1}s) with quality {:.1}%",
        passage_id,
        start_seconds,
        end_seconds,
        validation.score * 100.0
    );

    // Store provenance log entries
    store_provenance_logs(db, &passage_id, passage).await?;

    Ok(passage_id)
}

/// Store import provenance log entries
///
/// Records which extractors contributed data for this passage
async fn store_provenance_logs(
    db: &SqlitePool,
    passage_id: &str,
    passage: &ProcessedPassage,
) -> Result<()> {
    let timestamp = chrono::Utc::now().timestamp();

    // Insert provenance entry for each extraction source
    for extraction in &passage.extractions {
        // Determine extractor name from extraction result
        let extractor_name = if extraction.metadata.is_some() {
            "metadata_extractor"
        } else if extraction.identity.is_some() {
            "identity_extractor"
        } else if extraction.musical_flavor.is_some() {
            "flavor_extractor"
        } else {
            "unknown_extractor"
        };

        let data_json = serde_json::json!({
            "has_metadata": extraction.metadata.is_some(),
            "has_identity": extraction.identity.is_some(),
            "has_flavor": extraction.musical_flavor.is_some(),
        });

        let data_str = serde_json::to_string(&data_json)?;

        // Extract confidence from the extraction result
        // MetadataExtraction: average confidence of non-None fields
        // IdentityExtraction: direct confidence field
        // FlavorExtraction: direct confidence field
        let confidence = if let Some(ref meta) = extraction.metadata {
            // Average confidence across all non-None metadata fields
            let mut confidences = Vec::new();
            if let Some(ref title) = meta.title {
                confidences.push(title.confidence);
            }
            if let Some(ref artist) = meta.artist {
                confidences.push(artist.confidence);
            }
            if let Some(ref album) = meta.album {
                confidences.push(album.confidence);
            }
            if let Some(ref mbid) = meta.recording_mbid {
                confidences.push(mbid.confidence);
            }
            if confidences.is_empty() {
                0.0
            } else {
                confidences.iter().sum::<f32>() / confidences.len() as f32
            }
        } else if let Some(ref identity) = extraction.identity {
            identity.confidence
        } else if let Some(ref flavor) = extraction.musical_flavor {
            flavor.confidence
        } else {
            0.0
        };

        sqlx::query(
            r#"
            INSERT INTO import_provenance (
                id,
                passage_id,
                source_type,
                data_extracted,
                confidence,
                timestamp
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(passage_id)
        .bind(extractor_name)
        .bind(&data_str)
        .bind(confidence)
        .bind(timestamp)
        .execute(db)
        .await
        .context("Failed to insert provenance log")?;
    }

    debug!(
        "Stored {} provenance log entries for passage {}",
        passage.extractions.len(),
        passage_id
    );

    Ok(())
}

/// Store multiple passages in a transaction
///
/// # Arguments
/// * `db` - Database connection pool
/// * `file_path` - Source audio file path
/// * `passages` - All processed passages from the file
/// * `import_session_id` - UUID for this import session
///
/// # Returns
/// * Vec of passage UUIDs
pub async fn store_passages_batch(
    db: &SqlitePool,
    file_path: &str,
    passages: &[ProcessedPassage],
    import_session_id: &str,
) -> Result<Vec<String>> {
    let mut tx = db.begin().await.context("Failed to begin transaction")?;
    let mut passage_ids = Vec::new();

    for passage in passages {
        let passage_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp();

        // Extract fusion result data
        let fusion = &passage.fusion;
        let validation = &passage.validation;

        // Extract metadata fields
        let (title, title_source, title_confidence) = fusion
            .metadata
            .title
            .as_ref()
            .map(|cv| (Some(cv.value.clone()), Some(cv.source.clone()), cv.confidence))
            .unwrap_or((None, None, 0.0));

        let (artist, artist_source, artist_confidence) = fusion
            .metadata
            .artist
            .as_ref()
            .map(|cv| (Some(cv.value.clone()), Some(cv.source.clone()), cv.confidence))
            .unwrap_or((None, None, 0.0));

        let (album, album_source, album_confidence) = fusion
            .metadata
            .album
            .as_ref()
            .map(|cv| (Some(cv.value.clone()), Some(cv.source.clone()), cv.confidence))
            .unwrap_or((None, None, 0.0));

        let (recording_mbid, mbid_source, mbid_confidence) = fusion
            .metadata
            .recording_mbid
            .as_ref()
            .map(|cv| (Some(cv.value.clone()), Some(cv.source.clone()), cv.confidence))
            .unwrap_or((None, None, 0.0));

        // Serialize JSON fields
        let flavor_json = serde_json::to_string(&fusion.flavor.characteristics)?;
        let flavor_source_blend = serde_json::to_string(&fusion.flavor.source_blend)?;
        let flavor_confidence_map = serde_json::to_string(&fusion.flavor.confidence_map)?;
        let identity_conflicts = if !fusion.identity.conflicts.is_empty() {
            Some(serde_json::to_string(&fusion.identity.conflicts)?)
        } else {
            None
        };
        let validation_report = serde_json::to_string(&validation.report)?;
        let validation_issues = if !validation.issues.is_empty() {
            Some(serde_json::to_string(&validation.issues)?)
        } else {
            None
        };

        // Insert passage (SPEC017: times as INTEGER ticks)
        let query = r#"
            INSERT INTO passages (
                guid, file_path, start_time, end_time,
                title, artist, album, recording_mbid, musical_flavor,
                flavor_source_blend, flavor_confidence_map, flavor_completeness,
                title_source, title_confidence, artist_source, artist_confidence,
                album_source, album_confidence, mbid_source, mbid_confidence,
                identity_confidence, identity_posterior_probability, identity_conflicts,
                overall_quality_score, metadata_completeness,
                validation_status, validation_report, validation_issues,
                import_session_id, import_timestamp, import_strategy
            ) VALUES (
                ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?
            )
        "#;

        let validation_status_str = format!("{:?}", validation.status);

        sqlx::query(query)
            .bind(&passage_id)
            .bind(file_path)
            .bind(passage.boundary.start_time)
            .bind(passage.boundary.end_time)
            .bind(&title)
            .bind(&artist)
            .bind(&album)
            .bind(&recording_mbid)
            .bind(&flavor_json)
            .bind(&flavor_source_blend)
            .bind(&flavor_confidence_map)
            .bind(fusion.flavor.completeness)
            .bind(&title_source)
            .bind(title_confidence)
            .bind(&artist_source)
            .bind(artist_confidence)
            .bind(&album_source)
            .bind(album_confidence)
            .bind(&mbid_source)
            .bind(mbid_confidence)
            .bind(fusion.identity.confidence)
            .bind(fusion.identity.posterior_probability)
            .bind(&identity_conflicts)
            .bind(validation.score)
            .bind(fusion.metadata.metadata_completeness)
            .bind(&validation_status_str)
            .bind(&validation_report)
            .bind(&validation_issues)
            .bind(import_session_id)
            .bind(timestamp)
            .bind("hybrid_fusion_v2")
            .execute(&mut *tx)
            .await?;

        // Store provenance logs
        for extraction in &passage.extractions {
            let extractor_name = if extraction.metadata.is_some() {
                "metadata_extractor"
            } else if extraction.identity.is_some() {
                "identity_extractor"
            } else if extraction.musical_flavor.is_some() {
                "flavor_extractor"
            } else {
                "unknown_extractor"
            };

            let data_json = serde_json::json!({
                "has_metadata": extraction.metadata.is_some(),
                "has_identity": extraction.identity.is_some(),
                "has_flavor": extraction.musical_flavor.is_some(),
            });

            let data_str = serde_json::to_string(&data_json)?;

            let confidence = if let Some(ref meta) = extraction.metadata {
                // Average confidence across all non-None metadata fields
                let mut confidences = Vec::new();
                if let Some(ref title) = meta.title {
                    confidences.push(title.confidence);
                }
                if let Some(ref artist) = meta.artist {
                    confidences.push(artist.confidence);
                }
                if let Some(ref album) = meta.album {
                    confidences.push(album.confidence);
                }
                if let Some(ref mbid) = meta.recording_mbid {
                    confidences.push(mbid.confidence);
                }
                if confidences.is_empty() {
                    0.0
                } else {
                    confidences.iter().sum::<f32>() / confidences.len() as f32
                }
            } else if let Some(ref identity) = extraction.identity {
                identity.confidence
            } else if let Some(ref flavor) = extraction.musical_flavor {
                flavor.confidence
            } else {
                0.0
            };

            sqlx::query(
                r#"
                INSERT INTO import_provenance (
                    id, passage_id, source_type, data_extracted, confidence, timestamp
                ) VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&passage_id)
            .bind(extractor_name)
            .bind(&data_str)
            .bind(confidence)
            .bind(timestamp)
            .execute(&mut *tx)
            .await?;
        }

        passage_ids.push(passage_id);
    }

    tx.commit().await.context("Failed to commit transaction")?;

    info!(
        "Stored {} passages from {} in transaction",
        passages.len(),
        file_path
    );

    Ok(passage_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        ConfidenceValue, ExtractionResult, FlavorExtraction, FusedFlavor, FusedIdentity,
        FusedMetadata, MetadataExtraction, ValidationStatus,
    };
    use crate::workflow::{FusedPassage, PassageBoundary, ProcessedPassage};
    use crate::types::ValidationResult;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::collections::HashMap;

    #[test]
    fn test_uuid_generation() {
        let uuid1 = Uuid::new_v4().to_string();
        let uuid2 = Uuid::new_v4().to_string();
        assert_ne!(uuid1, uuid2);
        assert_eq!(uuid1.len(), 36); // UUID format: 8-4-4-4-12
    }

    #[test]
    fn test_uuid_format() {
        let uuid = Uuid::new_v4().to_string();
        assert!(uuid.contains('-'));
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5);
    }

    /// Create test database with passages and import_provenance tables
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .expect("Failed to create test database");

        // Create passages table (SPEC017 compliant schema)
        sqlx::query(
            r#"
            CREATE TABLE passages (
                guid TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER NOT NULL,
                title TEXT,
                artist TEXT,
                album TEXT,
                recording_mbid TEXT,
                musical_flavor TEXT,
                flavor_source_blend TEXT,
                flavor_confidence_map TEXT,
                flavor_completeness REAL,
                title_source TEXT,
                title_confidence REAL,
                artist_source TEXT,
                artist_confidence REAL,
                album_source TEXT,
                album_confidence REAL,
                mbid_source TEXT,
                mbid_confidence REAL,
                identity_confidence REAL,
                identity_posterior_probability REAL,
                identity_conflicts TEXT,
                overall_quality_score REAL,
                metadata_completeness REAL,
                validation_status TEXT,
                validation_report TEXT,
                validation_issues TEXT,
                import_session_id TEXT,
                import_timestamp INTEGER,
                import_strategy TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create passages table");

        // Create import_provenance table
        sqlx::query(
            r#"
            CREATE TABLE import_provenance (
                id TEXT PRIMARY KEY,
                passage_id TEXT NOT NULL,
                source_type TEXT NOT NULL,
                data_extracted TEXT,
                confidence REAL,
                timestamp INTEGER,
                FOREIGN KEY (passage_id) REFERENCES passages(guid) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create import_provenance table");

        pool
    }

    /// Create mock ProcessedPassage for testing
    fn create_mock_passage(start_time: i64, end_time: i64) -> ProcessedPassage {
        // Create mock extraction results
        let metadata_extraction = MetadataExtraction {
            title: Some(ConfidenceValue::new("Test Song".to_string(), 0.95, "id3")),
            artist: Some(ConfidenceValue::new(
                "Test Artist".to_string(),
                0.95,
                "id3",
            )),
            album: Some(ConfidenceValue::new("Test Album".to_string(), 0.90, "id3")),
            recording_mbid: Some(ConfidenceValue::new(
                "12345678-1234-1234-1234-123456789abc".to_string(),
                0.98,
                "musicbrainz",
            )),
            additional: HashMap::new(),
        };

        let flavor_extraction = FlavorExtraction {
            characteristics: {
                let mut map = HashMap::new();
                map.insert("tempo".to_string(), 120.0);
                map.insert("energy".to_string(), 0.8);
                map
            },
            confidence: 0.9,
            source: "essentia".to_string(),
        };

        let extractions = vec![
            ExtractionResult {
                metadata: Some(metadata_extraction),
                identity: None,
                musical_flavor: None,
            },
            ExtractionResult {
                metadata: None,
                identity: None,
                musical_flavor: Some(flavor_extraction),
            },
        ];

        // Create mock fusion results
        let fusion = FusedPassage {
            metadata: FusedMetadata {
                title: Some(ConfidenceValue {
                    value: "Test Song".to_string(),
                    confidence: 0.95,
                    source: "id3".to_string(),
                }),
                artist: Some(ConfidenceValue {
                    value: "Test Artist".to_string(),
                    confidence: 0.95,
                    source: "id3".to_string(),
                }),
                album: Some(ConfidenceValue {
                    value: "Test Album".to_string(),
                    confidence: 0.90,
                    source: "id3".to_string(),
                }),
                recording_mbid: Some(ConfidenceValue {
                    value: "12345678-1234-1234-1234-123456789abc".to_string(),
                    confidence: 0.98,
                    source: "musicbrainz".to_string(),
                }),
                metadata_completeness: 1.0,
                additional: std::collections::HashMap::new(),
            },
            identity: FusedIdentity {
                recording_mbid: Some("12345678-1234-1234-1234-123456789abc".to_string()),
                confidence: 0.98,
                posterior_probability: 0.95,
                conflicts: vec![],
            },
            flavor: FusedFlavor {
                characteristics: {
                    let mut map = HashMap::new();
                    map.insert("tempo".to_string(), 120.0);
                    map.insert("energy".to_string(), 0.8);
                    map
                },
                source_blend: vec![
                    ("essentia".to_string(), 0.6),
                    ("audio_derived".to_string(), 0.4),
                ],
                confidence_map: {
                    let mut map = HashMap::new();
                    map.insert("tempo".to_string(), 0.9);
                    map.insert("energy".to_string(), 0.85);
                    map
                },
                completeness: 1.0,
            },
        };

        // Create mock validation result
        let validation = ValidationResult {
            status: ValidationStatus::Pass,
            score: 0.92,
            report: serde_json::json!({
                "consistency": 0.95,
                "completeness": 1.0,
                "quality": 0.90
            }),
            issues: vec![],
        };

        ProcessedPassage {
            boundary: PassageBoundary {
                start_time,
                end_time,
                confidence: 0.99,
            },
            extractions,
            fusion,
            validation,
        }
    }

    #[tokio::test]
    async fn test_store_passage() {
        let db = setup_test_db().await;
        let import_session_id = Uuid::new_v4().to_string();
        let file_path = "/test/audio/song.mp3";

        // Create mock passage (0.0s - 180.0s)
        let start_ticks = 0;
        let end_ticks = 180 * TICK_RATE; // 180 seconds
        let passage = create_mock_passage(start_ticks, end_ticks);

        // Store passage
        let passage_id = store_passage(&db, file_path, &passage, &import_session_id)
            .await
            .expect("Failed to store passage");

        // Verify passage was stored
        let row: (String, String, i64, i64) = sqlx::query_as(
            "SELECT guid, file_path, start_time, end_time FROM passages WHERE guid = ?",
        )
        .bind(&passage_id)
        .fetch_one(&db)
        .await
        .expect("Failed to fetch stored passage");

        assert_eq!(row.0, passage_id);
        assert_eq!(row.1, file_path);
        assert_eq!(row.2, start_ticks);
        assert_eq!(row.3, end_ticks);

        // Verify metadata fields
        let row: (Option<String>, Option<String>, Option<String>, Option<String>) =
            sqlx::query_as("SELECT title, artist, album, recording_mbid FROM passages WHERE guid = ?")
                .bind(&passage_id)
                .fetch_one(&db)
                .await
                .expect("Failed to fetch metadata");

        assert_eq!(row.0, Some("Test Song".to_string()));
        assert_eq!(row.1, Some("Test Artist".to_string()));
        assert_eq!(row.2, Some("Test Album".to_string()));
        assert_eq!(
            row.3,
            Some("12345678-1234-1234-1234-123456789abc".to_string())
        );

        // Verify confidence scores and sources
        let row: (Option<String>, f64, Option<String>, f64) = sqlx::query_as(
            "SELECT title_source, title_confidence, artist_source, artist_confidence FROM passages WHERE guid = ?",
        )
        .bind(&passage_id)
        .fetch_one(&db)
        .await
        .expect("Failed to fetch confidence data");

        assert_eq!(row.0, Some("id3".to_string()));
        assert!((row.1 - 0.95).abs() < 0.01);
        assert_eq!(row.2, Some("id3".to_string()));
        assert!((row.3 - 0.95).abs() < 0.01);

        // Verify validation data
        let row: (String, f64) =
            sqlx::query_as("SELECT validation_status, overall_quality_score FROM passages WHERE guid = ?")
                .bind(&passage_id)
                .fetch_one(&db)
                .await
                .expect("Failed to fetch validation data");

        assert_eq!(row.0, "Pass");
        assert!((row.1 - 0.92).abs() < 0.01);

        // Verify import_strategy
        let import_strategy: String =
            sqlx::query_scalar("SELECT import_strategy FROM passages WHERE guid = ?")
                .bind(&passage_id)
                .fetch_one(&db)
                .await
                .expect("Failed to fetch import_strategy");

        assert_eq!(import_strategy, "hybrid_fusion_v2");

        // Verify provenance logs were created
        let provenance_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM import_provenance WHERE passage_id = ?")
                .bind(&passage_id)
                .fetch_one(&db)
                .await
                .expect("Failed to count provenance logs");

        assert_eq!(provenance_count, 2); // 2 extraction results
    }

    #[tokio::test]
    async fn test_store_passages_batch() {
        let db = setup_test_db().await;
        let import_session_id = Uuid::new_v4().to_string();
        let file_path = "/test/audio/album.mp3";

        // Create 3 mock passages
        let passages = vec![
            create_mock_passage(0, 180 * TICK_RATE),
            create_mock_passage(180 * TICK_RATE, 360 * TICK_RATE),
            create_mock_passage(360 * TICK_RATE, 540 * TICK_RATE),
        ];

        // Store all passages in transaction
        let passage_ids =
            store_passages_batch(&db, file_path, &passages, &import_session_id)
                .await
                .expect("Failed to store passages batch");

        assert_eq!(passage_ids.len(), 3);

        // Verify all passages were stored
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passages")
            .fetch_one(&db)
            .await
            .expect("Failed to count passages");

        assert_eq!(count, 3);

        // Verify correct time boundaries
        let times: Vec<(i64, i64)> =
            sqlx::query_as("SELECT start_time, end_time FROM passages ORDER BY start_time")
                .fetch_all(&db)
                .await
                .expect("Failed to fetch times");

        assert_eq!(times[0], (0, 180 * TICK_RATE));
        assert_eq!(times[1], (180 * TICK_RATE, 360 * TICK_RATE));
        assert_eq!(times[2], (360 * TICK_RATE, 540 * TICK_RATE));

        // Verify provenance logs for all passages
        let provenance_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM import_provenance")
                .fetch_one(&db)
                .await
                .expect("Failed to count provenance logs");

        assert_eq!(provenance_count, 6); // 3 passages × 2 extractions each
    }

    #[tokio::test]
    async fn test_provenance_confidence_extraction() {
        let db = setup_test_db().await;
        let import_session_id = Uuid::new_v4().to_string();
        let file_path = "/test/audio/test.mp3";

        let passage = create_mock_passage(0, 180 * TICK_RATE);
        let passage_id = store_passage(&db, file_path, &passage, &import_session_id)
            .await
            .expect("Failed to store passage");

        // Verify provenance confidence values
        let confidences: Vec<f32> =
            sqlx::query_scalar("SELECT confidence FROM import_provenance WHERE passage_id = ? ORDER BY confidence DESC")
                .bind(&passage_id)
                .fetch_all(&db)
                .await
                .expect("Failed to fetch confidences");

        assert_eq!(confidences.len(), 2);
        assert!(confidences[0] >= 0.0 && confidences[0] <= 1.0);
        assert!(confidences[1] >= 0.0 && confidences[1] <= 1.0);
    }

    #[tokio::test]
    async fn test_spec017_tick_conversion() {
        // Test that TICK_RATE constant matches SPEC017
        assert_eq!(TICK_RATE, 28_224_000);

        // Test conversion: 180 seconds = 5,080,320,000 ticks
        let seconds = 180.0;
        let ticks = (seconds * TICK_RATE as f64) as i64;
        assert_eq!(ticks, 5_080_320_000);

        // Test reverse conversion
        let converted_seconds = ticks as f64 / TICK_RATE as f64;
        assert!((converted_seconds - seconds).abs() < 0.000001);
    }

    #[tokio::test]
    async fn test_empty_optional_fields() {
        let db = setup_test_db().await;
        let import_session_id = Uuid::new_v4().to_string();

        // Create passage with minimal data (no conflicts, no issues)
        let mut passage = create_mock_passage(0, 180 * TICK_RATE);
        passage.fusion.identity.conflicts = vec![]; // Empty conflicts
        passage.validation.issues = vec![]; // Empty issues

        let passage_id = store_passage(&db, "/test/minimal.mp3", &passage, &import_session_id)
            .await
            .expect("Failed to store minimal passage");

        // Verify NULL values for empty arrays
        let row: (Option<String>, Option<String>) = sqlx::query_as(
            "SELECT identity_conflicts, validation_issues FROM passages WHERE guid = ?",
        )
        .bind(&passage_id)
        .fetch_one(&db)
        .await
        .expect("Failed to fetch optional fields");

        assert_eq!(row.0, None); // identity_conflicts should be NULL
        assert_eq!(row.1, None); // validation_issues should be NULL
    }
}
