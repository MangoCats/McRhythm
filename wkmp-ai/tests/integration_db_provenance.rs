// PLAN023: Database Integration Tests - Provenance Storage/Retrieval
//
// Tests verify that ProcessedPassage data is correctly stored in the database
// with all PLAN023 columns populated and queryable.
//
// Test Coverage:
// - TC-I-081-01: Flavor source provenance storage/retrieval
// - TC-I-082-01: Metadata source provenance storage/retrieval
// - TC-I-083-01: Identity resolution tracking
// - TC-I-084-01: Quality scores storage
// - TC-I-085-01: Validation flags storage
// - TC-I-086-01: Import metadata storage
// - TC-I-087-01: Import provenance log queries

use chrono;
use sqlx::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;
use wkmp_ai::import_v2::db_repository::ImportRepository;
use wkmp_ai::import_v2::types::*;
use wkmp_common::db::migrations::run_migrations;

/// Create in-memory test database with full schema
async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    // Create minimal base schema (schema_version + passages table)
    sqlx::query(
        r#"
        CREATE TABLE schema_version (
            version INTEGER NOT NULL PRIMARY KEY
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO schema_version (version) VALUES (0)")
        .execute(&pool)
        .await
        .unwrap();

    // Create files table (required for passages foreign key)
    sqlx::query(
        r#"
        CREATE TABLE files (
            guid TEXT PRIMARY KEY,
            path TEXT NOT NULL UNIQUE
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create base passages table with original columns
    sqlx::query(
        r#"
        CREATE TABLE passages (
            guid TEXT PRIMARY KEY,
            file_id TEXT NOT NULL REFERENCES files(guid) ON DELETE CASCADE,
            start_time_ticks INTEGER NOT NULL,
            end_time_ticks INTEGER NOT NULL,
            title TEXT,
            artist TEXT,
            album TEXT,
            musical_flavor_vector TEXT
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Run migrations to add all PLAN023 columns
    run_migrations(&pool).await.unwrap();

    pool
}

/// Helper to create file record in database
async fn create_test_file(pool: &SqlitePool, file_id: &Uuid) {
    sqlx::query("INSERT INTO files (guid, path) VALUES (?, ?)")
        .bind(file_id.to_string())
        .bind(format!("/test/file_{}.flac", file_id))
        .execute(pool)
        .await
        .unwrap();
}

/// Create test ProcessedPassage with all fields populated
fn create_test_processed_passage() -> ProcessedPassage {
    let mbid1 = Uuid::new_v4();
    let mbid2 = Uuid::new_v4();

    ProcessedPassage {
        boundary: PassageBoundary {
            start_ticks: 0,
            end_ticks: 28_224_000 * 180, // 3 minutes in ticks
            confidence: 0.95,
            detection_method: BoundaryDetectionMethod::SilenceDetection,
        },
        identity: ResolvedIdentity {
            mbid: Some(mbid1),
            confidence: 0.85,
            candidates: vec![
                MBIDCandidate {
                    mbid: mbid1,
                    confidence: 0.85,
                    sources: vec![
                        ExtractionSource::AcoustID,
                        ExtractionSource::MusicBrainz,
                    ],
                },
                MBIDCandidate {
                    mbid: mbid2,
                    confidence: 0.42,
                    sources: vec![ExtractionSource::ID3Metadata],
                },
            ],
            has_conflict: false,
        },
        metadata: FusedMetadata {
            title: Some(MetadataField {
                value: "Test Song".to_string(),
                confidence: 0.9,
                source: ExtractionSource::MusicBrainz,
            }),
            artist: Some(MetadataField {
                value: "Test Artist".to_string(),
                confidence: 0.85,
                source: ExtractionSource::AcoustID,
            }),
            album: Some(MetadataField {
                value: "Test Album".to_string(),
                confidence: 0.8,
                source: ExtractionSource::ID3Metadata,
            }),
            release_date: Some(MetadataField {
                value: "2024-01-15".to_string(),
                confidence: 0.7,
                source: ExtractionSource::MusicBrainz,
            }),
            track_number: Some(MetadataField {
                value: 5,
                confidence: 0.6,
                source: ExtractionSource::ID3Metadata,
            }),
            duration_ms: Some(MetadataField {
                value: 180000, // 3 minutes
                confidence: 0.95,
                source: ExtractionSource::AudioDerived,
            }),
            metadata_confidence: 0.82,
        },
        flavor: SynthesizedFlavor {
            flavor: MusicalFlavor {
                characteristics: vec![
                    Characteristic {
                        name: "danceability".to_string(),
                        values: {
                            let mut map = HashMap::new();
                            map.insert("value".to_string(), 0.7);
                            map
                        },
                    },
                    Characteristic {
                        name: "energy".to_string(),
                        values: {
                            let mut map = HashMap::new();
                            map.insert("value".to_string(), 0.8);
                            map
                        },
                    },
                    Characteristic {
                        name: "valence".to_string(),
                        values: {
                            let mut map = HashMap::new();
                            map.insert("value".to_string(), 0.6);
                            map
                        },
                    },
                ],
            },
            flavor_confidence: 0.75,
            flavor_completeness: 0.9,
            sources_used: vec![
                ExtractionSource::Essentia,
                ExtractionSource::AudioDerived,
            ],
        },
        validation: ValidationReport {
            quality_score: 0.88,
            has_conflicts: false,
            warnings: vec!["Missing track number confidence".to_string()],
            conflicts: vec![],
        },
        import_duration_ms: 1250,
        import_timestamp: chrono::Utc::now().to_rfc3339(),
        import_version: "PLAN023-v1.0.0".to_string(),
    }
}

/// TC-I-081-01: Flavor source provenance storage/retrieval
#[tokio::test]
async fn test_flavor_source_provenance_storage() {
    let pool = setup_test_db().await;
    let repo = ImportRepository::new(pool.clone());

    let file_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let processed = create_test_processed_passage();

    // Create file record first (foreign key requirement)
    create_test_file(&pool, &file_id).await;

    // Save passage
    let passage_id = repo
        .save_processed_passage(&file_id, &processed, &session_id)
        .await
        .unwrap();

    // Query flavor source blend
    let flavor_sources: Option<String> = sqlx::query_scalar(
        "SELECT flavor_source_blend FROM passages WHERE guid = ?",
    )
    .bind(passage_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(flavor_sources.is_some());
    let sources_json = flavor_sources.unwrap();
    let sources: Vec<String> = serde_json::from_str(&sources_json).unwrap();

    assert_eq!(sources.len(), 2);
    assert!(sources.contains(&"Essentia".to_string()));
    assert!(sources.contains(&"AudioDerived".to_string()));

    // Verify flavor confidence map
    let flavor_confidence: Option<String> = sqlx::query_scalar(
        "SELECT flavor_confidence_map FROM passages WHERE guid = ?",
    )
    .bind(passage_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(flavor_confidence.is_some());
    let conf_json = flavor_confidence.unwrap();
    assert!(conf_json.contains("0.75")); // overall confidence
}

/// TC-I-082-01: Metadata source provenance storage/retrieval
#[tokio::test]
async fn test_metadata_source_provenance_storage() {
    let pool = setup_test_db().await;
    let repo = ImportRepository::new(pool.clone());

    let file_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let processed = create_test_processed_passage();

    // Create file record first (foreign key requirement)
    create_test_file(&pool, &file_id).await;

    // Save passage
    let passage_id = repo
        .save_processed_passage(&file_id, &processed, &session_id)
        .await
        .unwrap();

    // Query metadata provenance columns
    let (title_source, title_conf, artist_source, artist_conf, album_source, album_conf): (
        Option<String>,
        Option<f64>,
        Option<String>,
        Option<f64>,
        Option<String>,
        Option<f64>,
    ) = sqlx::query_as(
        r#"
        SELECT title_source, title_confidence,
               artist_source, artist_confidence,
               album_source, album_confidence
        FROM passages WHERE guid = ?
        "#,
    )
    .bind(passage_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    // Verify title provenance
    assert_eq!(title_source, Some("MusicBrainz".to_string()));
    assert_eq!(title_conf, Some(0.9));

    // Verify artist provenance
    assert_eq!(artist_source, Some("AcoustID".to_string()));
    assert_eq!(artist_conf, Some(0.85));

    // Verify album provenance
    assert_eq!(album_source, Some("ID3Metadata".to_string()));
    assert_eq!(album_conf, Some(0.8));
}

/// TC-I-083-01: Identity resolution tracking
#[tokio::test]
async fn test_identity_resolution_tracking() {
    let pool = setup_test_db().await;
    let repo = ImportRepository::new(pool.clone());

    let file_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let processed = create_test_processed_passage();

    // Create file record first (foreign key requirement)
    create_test_file(&pool, &file_id).await;

    // Save passage
    let passage_id = repo
        .save_processed_passage(&file_id, &processed, &session_id)
        .await
        .unwrap();

    // Query identity resolution data
    let (mbid, confidence, conflicts_json): (Option<String>, Option<f64>, Option<String>) =
        sqlx::query_as(
            r#"
        SELECT recording_mbid, identity_confidence, identity_conflicts
        FROM passages WHERE guid = ?
        "#,
        )
        .bind(passage_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

    // Verify resolved MBID
    assert!(mbid.is_some());
    assert_eq!(confidence, Some(0.85));

    // Verify identity candidates stored
    assert!(conflicts_json.is_some());
    let candidates: Vec<serde_json::Value> = serde_json::from_str(&conflicts_json.unwrap()).unwrap();
    assert_eq!(candidates.len(), 2);

    // Verify first candidate (high confidence)
    assert_eq!(candidates[0]["confidence"], 0.85);
    let sources: Vec<String> = serde_json::from_value(candidates[0]["sources"].clone()).unwrap();
    assert_eq!(sources.len(), 2);
    assert!(sources.contains(&"AcoustID".to_string()));
    assert!(sources.contains(&"MusicBrainz".to_string()));

    // Verify second candidate (low confidence)
    assert_eq!(candidates[1]["confidence"], 0.42);
}

/// TC-I-084-01: Quality scores storage
#[tokio::test]
async fn test_quality_scores_storage() {
    let pool = setup_test_db().await;
    let repo = ImportRepository::new(pool.clone());

    let file_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let processed = create_test_processed_passage();

    // Create file record first (foreign key requirement)
    create_test_file(&pool, &file_id).await;

    // Save passage
    let passage_id = repo
        .save_processed_passage(&file_id, &processed, &session_id)
        .await
        .unwrap();

    // Query quality scores
    let (overall_quality, metadata_complete, flavor_complete): (
        Option<f64>,
        Option<f64>,
        Option<f64>,
    ) = sqlx::query_as(
        r#"
        SELECT overall_quality_score, metadata_completeness, flavor_completeness
        FROM passages WHERE guid = ?
        "#,
    )
    .bind(passage_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    // Verify quality scores
    assert_eq!(overall_quality, Some(0.88));
    assert!(metadata_complete.is_some());
    assert_eq!(flavor_complete, Some(0.9));

    // Metadata completeness should be 1.0 (all 6 fields present)
    let metadata_comp = metadata_complete.unwrap();
    assert!((metadata_comp - 1.0).abs() < 0.01);
}

/// TC-I-085-01: Validation flags storage
#[tokio::test]
async fn test_validation_flags_storage() {
    let pool = setup_test_db().await;
    let repo = ImportRepository::new(pool.clone());

    let file_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let processed = create_test_processed_passage();

    // Create file record first (foreign key requirement)
    create_test_file(&pool, &file_id).await;

    // Save passage
    let passage_id = repo
        .save_processed_passage(&file_id, &processed, &session_id)
        .await
        .unwrap();

    // Query validation status and report
    let (status, report_json): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT validation_status, validation_report FROM passages WHERE guid = ?",
    )
    .bind(passage_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    // Verify validation status
    // Status is "Warning" because there are warnings present, even though quality_score >= 0.8
    assert_eq!(status, Some("Warning".to_string()));

    // Verify validation report
    assert!(report_json.is_some());
    let report: serde_json::Value = serde_json::from_str(&report_json.unwrap()).unwrap();

    assert_eq!(report["quality_score"], 0.88);
    assert_eq!(report["has_conflicts"], false);

    let warnings: Vec<String> = serde_json::from_value(report["warnings"].clone()).unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0], "Missing track number confidence");

    let conflicts: Vec<serde_json::Value> =
        serde_json::from_value(report["conflicts"].clone()).unwrap();
    assert_eq!(conflicts.len(), 0);
}

/// TC-I-086-01: Import metadata storage
#[tokio::test]
async fn test_import_metadata_storage() {
    let pool = setup_test_db().await;
    let repo = ImportRepository::new(pool.clone());

    let file_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let processed = create_test_processed_passage();

    // Create file record first (foreign key requirement)
    create_test_file(&pool, &file_id).await;

    // Save passage
    let passage_id = repo
        .save_processed_passage(&file_id, &processed, &session_id)
        .await
        .unwrap();

    // Query import metadata
    let (sess_id, timestamp, strategy, duration, version): (
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<String>,
    ) = sqlx::query_as(
        r#"
        SELECT import_session_id, import_timestamp, import_strategy,
               import_duration_ms, import_version
        FROM passages WHERE guid = ?
        "#,
    )
    .bind(passage_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    // Verify import metadata
    assert_eq!(sess_id, Some(session_id.to_string()));
    assert!(timestamp.is_some());
    assert!(timestamp.unwrap() > 0); // Unix timestamp
    assert_eq!(strategy, Some("HybridFusion".to_string()));
    assert_eq!(duration, Some(1250));
    assert_eq!(version, Some("PLAN023-v1.0.0".to_string()));
}

/// TC-I-087-01: Import provenance log queries
#[tokio::test]
async fn test_import_provenance_log_queries() {
    let pool = setup_test_db().await;
    let repo = ImportRepository::new(pool.clone());

    let file_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let processed = create_test_processed_passage();

    // Create file record first (foreign key requirement)
    create_test_file(&pool, &file_id).await;

    // Save passage (creates provenance log entries)
    let passage_id = repo
        .save_processed_passage(&file_id, &processed, &session_id)
        .await
        .unwrap();

    // Query provenance log entries
    let entries: Vec<(String, String, f64)> = sqlx::query_as(
        r#"
        SELECT source_type, data_extracted, confidence
        FROM import_provenance
        WHERE passage_id = ?
        ORDER BY source_type
        "#,
    )
    .bind(passage_id.to_string())
    .fetch_all(&pool)
    .await
    .unwrap();

    // Verify provenance entries exist
    assert!(entries.len() > 0);

    // Should have entries for MBID candidates (2) and flavor sources (2)
    let mbid_entries: Vec<_> = entries
        .iter()
        .filter(|(source_type, _, _)| source_type == "MBIDCandidate")
        .collect();
    assert_eq!(mbid_entries.len(), 2);

    // Verify MBID candidate provenance
    for (_, data, conf) in mbid_entries {
        let data_json: serde_json::Value = serde_json::from_str(data).unwrap();
        assert!(data_json["mbid"].is_string());
        assert!(data_json["sources"].is_string());
        assert!(*conf >= 0.0 && *conf <= 1.0);
    }

    // Verify flavor source provenance
    let flavor_entries: Vec<_> = entries
        .iter()
        .filter(|(source_type, _, _)| {
            source_type == "Essentia" || source_type == "AudioDerived"
        })
        .collect();
    assert_eq!(flavor_entries.len(), 2);

    // Query by source type index
    let essentia_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM import_provenance WHERE passage_id = ? AND source_type = 'Essentia'",
    )
    .bind(passage_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(essentia_count, 1);
}
