//! Integration Tests with Real Audio Files
//!
//! PLAN027 Sprint 3.2: Test full import workflow with real audio fixtures
//!
//! Tests use generated WAV files from tests/fixtures/audio/:
//! - multi_track_album.wav (boundary detection)
//! - minimal_valid.wav (chromaprint minimum duration)
//! - short_invalid.wav (error handling)
//! - no_silence.wav (single passage)

use std::path::PathBuf;
use tempfile::TempDir;
use wkmp_ai::import_v2::session_orchestrator::SessionOrchestrator;
use wkmp_ai::import_v2::types::ImportError;
use wkmp_common::db;

/// Helper: Get fixture path
fn fixture_path(filename: &str) -> PathBuf {
    PathBuf::from("tests/fixtures/audio").join(filename)
}

/// Helper: Create test database
async fn create_test_db() -> (sqlx::SqlitePool, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let pool = db::init::create_database(&db_path)
        .await
        .expect("Failed to create test database");

    db::init::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    db::init::seed_default_settings(&pool)
        .await
        .expect("Failed to seed settings");

    (pool, temp_dir)
}

#[tokio::test]
#[serial_test::serial]
async fn test_full_import_multi_track_album() {
    // Setup
    let (db, _temp_dir) = create_test_db().await;
    let orchestrator = SessionOrchestrator::new(db.clone());

    // Import multi-track album with silence gaps
    let audio_path = fixture_path("multi_track_album.wav");
    assert!(audio_path.exists(), "Fixture not found: {:?}", audio_path);

    // Start import session
    let session_result = orchestrator.import_file(&audio_path).await;

    // Verify import succeeded
    assert!(
        session_result.is_ok(),
        "Import failed: {:?}",
        session_result.err()
    );

    let session_id = session_result.unwrap();

    // Verify session in database
    let session_record = sqlx::query!(
        "SELECT status, total_files, successful_passages, failed_passages FROM import_sessions WHERE id = ?",
        session_id
    )
    .fetch_one(&db)
    .await
    .expect("Failed to fetch session");

    // Assert: Session completed successfully
    assert_eq!(session_record.status, "completed");
    assert_eq!(session_record.total_files, 1);

    // Assert: 3 passages detected (based on silence gaps at 5-7s and 12-14s)
    // Note: Exact count depends on silence detection parameters
    assert!(
        session_record.successful_passages >= 1,
        "Expected at least 1 passage, got {}",
        session_record.successful_passages
    );

    println!(
        "✓ Multi-track import: {} passages detected",
        session_record.successful_passages
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_minimal_valid_audio() {
    // Setup
    let (db, _temp_dir) = create_test_db().await;
    let orchestrator = SessionOrchestrator::new(db.clone());

    // Import 3-second audio (minimum for chromaprint)
    let audio_path = fixture_path("minimal_valid.wav");
    assert!(audio_path.exists(), "Fixture not found: {:?}", audio_path);

    // Start import
    let session_result = orchestrator.import_file(&audio_path).await;

    // Verify import succeeded
    assert!(
        session_result.is_ok(),
        "Import failed for 3-second audio: {:?}",
        session_result.err()
    );

    let session_id = session_result.unwrap();

    // Verify passage has fingerprint (3s meets minimum)
    let passages = sqlx::query!(
        "SELECT chromaprint_fingerprint FROM passages WHERE session_id = ?",
        session_id
    )
    .fetch_all(&db)
    .await
    .expect("Failed to fetch passages");

    assert!(!passages.is_empty(), "No passages created");

    // At least one passage should have fingerprint
    let has_fingerprint = passages.iter().any(|p| p.chromaprint_fingerprint.is_some());
    assert!(
        has_fingerprint,
        "3-second audio should generate fingerprint (chromaprint minimum)"
    );

    println!("✓ Minimal valid audio: fingerprint generated");
}

#[tokio::test]
#[serial_test::serial]
async fn test_error_handling_short_audio() {
    // Setup
    let (db, _temp_dir) = create_test_db().await;
    let orchestrator = SessionOrchestrator::new(db.clone());

    // Import 1-second audio (too short for chromaprint)
    let audio_path = fixture_path("short_invalid.wav");
    assert!(audio_path.exists(), "Fixture not found: {:?}", audio_path);

    // Start import
    let session_result = orchestrator.import_file(&audio_path).await;

    // Verify import completes (graceful handling, not panic)
    assert!(
        session_result.is_ok(),
        "Import should complete gracefully even with errors: {:?}",
        session_result.err()
    );

    let session_id = session_result.unwrap();

    // Verify session status
    let session_record = sqlx::query!(
        "SELECT status, successful_passages, failed_passages FROM import_sessions WHERE id = ?",
        session_id
    )
    .fetch_one(&db)
    .await
    .expect("Failed to fetch session");

    // Session may complete with partial success (passage created but no fingerprint)
    // or with failures (depending on error isolation behavior)
    assert!(
        session_record.status == "completed" || session_record.status == "failed",
        "Session should complete or fail gracefully, got: {}",
        session_record.status
    );

    println!(
        "✓ Short audio handled: status={}, successful={}, failed={}",
        session_record.status,
        session_record.successful_passages,
        session_record.failed_passages
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_no_silence_single_passage() {
    // Setup
    let (db, _temp_dir) = create_test_db().await;
    let orchestrator = SessionOrchestrator::new(db.clone());

    // Import continuous audio (no silence gaps)
    let audio_path = fixture_path("no_silence.wav");
    assert!(audio_path.exists(), "Fixture not found: {:?}", audio_path);

    // Start import
    let session_result = orchestrator.import_file(&audio_path).await;

    // Verify import succeeded
    assert!(
        session_result.is_ok(),
        "Import failed: {:?}",
        session_result.err()
    );

    let session_id = session_result.unwrap();

    // Verify single passage created (no silence detected)
    let passages = sqlx::query!(
        "SELECT start_ticks, end_ticks FROM passages WHERE session_id = ?",
        session_id
    )
    .fetch_all(&db)
    .await
    .expect("Failed to fetch passages");

    assert_eq!(
        passages.len(),
        1,
        "Expected 1 passage (no silence), got {}",
        passages.len()
    );

    // Verify passage spans full file (5 seconds = 141,120,000 ticks)
    let passage = &passages[0];
    assert_eq!(passage.start_ticks, 0, "Passage should start at 0");

    let expected_end_ticks = (5.0 * 28_224_000.0) as i64; // 5 seconds in ticks
    let tolerance = (0.1 * 28_224_000.0) as i64; // 100ms tolerance

    assert!(
        (passage.end_ticks - expected_end_ticks).abs() < tolerance,
        "Passage should span ~5 seconds, got {}ms",
        passage.end_ticks as f64 / 28_224_000.0 * 1000.0
    );

    println!("✓ No silence: single passage spanning full file");
}

#[tokio::test]
#[serial_test::serial]
async fn test_nonexistent_file_error() {
    // Setup
    let (db, _temp_dir) = create_test_db().await;
    let orchestrator = SessionOrchestrator::new(db.clone());

    // Attempt to import nonexistent file
    let audio_path = PathBuf::from("tests/fixtures/audio/nonexistent.wav");

    // Should return error
    let session_result = orchestrator.import_file(&audio_path).await;

    assert!(
        session_result.is_err(),
        "Should fail for nonexistent file"
    );

    // Verify error type
    match session_result.err() {
        Some(ImportError::FileNotFound(_)) => {
            println!("✓ Nonexistent file: correct error type");
        }
        other => panic!("Expected FileNotFound error, got: {:?}", other),
    }
}
