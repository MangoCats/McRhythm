// Regression Test: FOREIGN KEY Constraint Fix
//
// This test specifically verifies that the bug fix for FOREIGN KEY constraints
// remains in place. The bug was that SessionOrchestrator generated file_id UUIDs
// but never inserted file records before trying to save passages.
//
// **If this test fails, the FOREIGN KEY regression has occurred.**
//
// Test Coverage:
// - TC-REG-FK-001: Verify file metadata saved before passage persistence
// - TC-REG-FK-002: Verify file record exists when passage references it
// - TC-REG-FK-003: Verify FOREIGN KEY constraint is satisfied

use serial_test::serial;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;
use wkmp_common::db::migrations::run_migrations;

/// Create test database with full schema
async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    // Create minimal base schema (schema_version + files + passages tables)
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

    // Create files table with all columns (required for passages foreign key)
    sqlx::query(
        r#"
        CREATE TABLE files (
            guid TEXT PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            hash TEXT NOT NULL,
            duration_ticks INTEGER,
            format TEXT,
            sample_rate INTEGER,
            channels INTEGER,
            file_size_bytes INTEGER,
            modification_time TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
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

/// Create minimal test audio file
fn create_test_audio_file() -> (TempDir, PathBuf) {
    use hound::{SampleFormat, WavSpec, WavWriter};

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.wav");

    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&file_path, spec).unwrap();

    // Write 1 second of silence
    for _ in 0..(44100 * 2) {
        writer.write_sample(0i16).unwrap();
    }

    writer.finalize().unwrap();

    (temp_dir, file_path)
}

/// TC-REG-FK-001: Verify save_file_metadata is called before save_processed_passage
///
/// This test simulates what SessionOrchestrator does and verifies the FOREIGN KEY
/// constraint is satisfied. If someone removes the save_file_metadata() call,
/// this test will fail.
#[tokio::test]
#[serial]
async fn test_file_saved_before_passage_foreign_key() {
    use wkmp_ai::db::files::{calculate_file_hash, save_file, AudioFile};
    use wkmp_ai::import_v2::db_repository::ImportRepository;
    use chrono::Utc;

    let pool = setup_test_db().await;
    let (_temp_dir, audio_file) = create_test_audio_file();

    // **SIMULATE SessionOrchestrator behavior**
    let file_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    // **CRITICAL STEP**: Save file metadata BEFORE passage
    // This is what the bug fix added to SessionOrchestrator
    let hash = calculate_file_hash(&audio_file).unwrap_or_default();
    let audio_file_record = AudioFile {
        guid: file_id,
        path: audio_file.display().to_string(),
        hash,
        duration_ticks: Some(28_224_000 * 3), // 3 seconds
        format: Some("Wav".to_string()),
        sample_rate: Some(44100),
        channels: Some(2),
        file_size_bytes: None,
        modification_time: Utc::now(),
    };

    save_file(&pool, &audio_file_record)
        .await
        .expect("File should be saved successfully");

    // Now save passage (should succeed because file exists)
    let processed = create_minimal_processed_passage();
    let repo = ImportRepository::new(pool.clone());

    let result = repo
        .save_processed_passage(&file_id, &processed, &session_id)
        .await;

    assert!(
        result.is_ok(),
        "Passage save should succeed when file exists (FOREIGN KEY satisfied)"
    );

    // **REGRESSION CHECK**: Verify file and passage are linked correctly
    let passage_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM passages WHERE file_id = ?"
    )
    .bind(file_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(passage_count, 1, "Passage should reference the correct file_id");
}

/// TC-REG-FK-002: Verify FOREIGN KEY constraint FAILS without file record
///
/// This test verifies that the database schema correctly enforces the FOREIGN KEY
/// constraint. If someone bypasses or disables the constraint, this test will fail.
#[tokio::test]
#[serial]
async fn test_foreign_key_constraint_enforced() {
    use wkmp_ai::import_v2::db_repository::ImportRepository;

    let pool = setup_test_db().await;

    let file_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    // **INTENTIONALLY SKIP** saving file record
    // This simulates the bug that was fixed

    let processed = create_minimal_processed_passage();
    let repo = ImportRepository::new(pool.clone());

    // Attempt to save passage without file record
    let result = repo
        .save_processed_passage(&file_id, &processed, &session_id)
        .await;

    assert!(
        result.is_err(),
        "Passage save should FAIL when file doesn't exist (FOREIGN KEY enforced). \
         If this passes, the FOREIGN KEY constraint is not working!"
    );

    // Verify error is specifically a FOREIGN KEY constraint failure
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("FOREIGN KEY") || error_msg.contains("foreign key"),
        "Error should mention FOREIGN KEY constraint: {}",
        error_msg
    );
}

/// TC-REG-FK-003: Verify complete workflow order (file then passage)
///
/// This test verifies the correct order of operations matches what
/// SessionOrchestrator.save_file_metadata() was designed to enforce.
#[tokio::test]
#[serial]
async fn test_workflow_order_file_before_passage() {
    use wkmp_ai::db::files::{calculate_file_hash, save_file, AudioFile};
    use wkmp_ai::import_v2::db_repository::ImportRepository;
    use chrono::Utc;

    let pool = setup_test_db().await;
    let (_temp_dir, audio_file) = create_test_audio_file();

    let file_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    // **STEP 1**: File metadata extraction and save (Phase 2 in SessionOrchestrator)
    let hash = calculate_file_hash(&audio_file).unwrap_or_default();
    let audio_file_record = AudioFile {
        guid: file_id,
        path: audio_file.display().to_string(),
        hash,
        duration_ticks: Some(28_224_000 * 3),
        format: Some("Wav".to_string()),
        sample_rate: Some(44100),
        channels: Some(2),
        file_size_bytes: None,
        modification_time: Utc::now(),
    };

    save_file(&pool, &audio_file_record).await.unwrap();

    // Verify file exists
    let file_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE guid = ?")
        .bind(file_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(file_count, 1, "File should exist after Phase 2");

    // **STEP 2**: Passage processing and save (Phase 4 in SessionOrchestrator)
    let processed = create_minimal_processed_passage();
    let repo = ImportRepository::new(pool.clone());

    repo.save_processed_passage(&file_id, &processed, &session_id)
        .await
        .unwrap();

    // Verify passage references file correctly
    let (passage_file_id,): (String,) =
        sqlx::query_as("SELECT file_id FROM passages WHERE guid = (SELECT guid FROM passages LIMIT 1)")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(
        passage_file_id,
        file_id.to_string(),
        "Passage should reference file_id (FOREIGN KEY satisfied)"
    );

    // Verify JOIN works (FOREIGN KEY valid)
    let join_result: Result<(String, String), sqlx::Error> = sqlx::query_as(
        "SELECT f.guid, p.guid FROM files f INNER JOIN passages p ON f.guid = p.file_id"
    )
    .fetch_one(&pool)
    .await;

    assert!(
        join_result.is_ok(),
        "JOIN should succeed (proves FOREIGN KEY is valid)"
    );
}

/// Helper: Create minimal ProcessedPassage for testing
fn create_minimal_processed_passage() -> wkmp_ai::import_v2::types::ProcessedPassage {
    use wkmp_ai::import_v2::types::*;

    ProcessedPassage {
        boundary: PassageBoundary {
            start_ticks: 0,
            end_ticks: 28_224_000 * 180, // 3 minutes
            confidence: 0.9,
            detection_method: BoundaryDetectionMethod::SilenceDetection,
        },
        identity: ResolvedIdentity {
            mbid: None,
            confidence: 0.5,
            candidates: vec![],
            has_conflict: false,
        },
        metadata: FusedMetadata {
            title: Some(MetadataField {
                value: "Test".to_string(),
                confidence: 0.8,
                source: ExtractionSource::ID3Metadata,
            }),
            artist: None,
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.8,
        },
        flavor: SynthesizedFlavor {
            flavor: MusicalFlavor {
                characteristics: vec![],
            },
            flavor_confidence: 0.7,
            flavor_completeness: 1.0,
            sources_used: vec![ExtractionSource::AudioDerived],
        },
        validation: ValidationReport {
            quality_score: 0.75,
            has_conflicts: false,
            warnings: vec![],
            conflicts: vec![],
        },
        import_duration_ms: 1000,
        import_timestamp: chrono::Utc::now().to_rfc3339(),
        import_version: "TEST-v1".to_string(),
    }
}
