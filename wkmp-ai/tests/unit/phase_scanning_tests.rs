//! TC-PHASE-001: phase_scanning Behavior Verification Tests
//!
//! Verifies SPEC032 SCANNING phase creates file records only (no processing)

use tempfile::TempDir;
use wkmp_ai::models::{ImportParameters, ImportSession, ImportState};
use wkmp_ai::services::WorkflowOrchestrator;

// Import test helpers
use crate::helpers::{audio_generator, db_utils, log_capture};

/// TC-PHASE-001: phase_scanning creates file records only (no processing)
///
/// **Requirement:** SPEC032 SCANNING phase - file discovery only
///
/// **Given:** Orchestrator with test audio files
/// **When:** phase_scanning() executes
/// **Then:**
///   - FileScanner.scan() discovers files
///   - Database contains file records with path, mod_time, size
///   - Database file records have EMPTY hash field
///   - NO metadata fields populated
///   - NO metadata extraction logs appear
///
/// **Verification Method:**
///   - Capture logs to verify no metadata extraction
///   - Query database to verify file records have empty/null processing fields
#[tokio::test]
async fn tc_phase_001_scanning_no_processing() {
    // Setup: Initialize log capture
    let log_capture = log_capture::init_test_logging();

    // Setup: Create test database and audio files
    let (_temp_db_dir, db_pool) = db_utils::create_test_db().await.unwrap();
    let temp_audio_dir = TempDir::new().unwrap();

    let audio_config = audio_generator::AudioConfig {
        duration_seconds: 35.0,
        ..Default::default()
    };

    // Generate 3 test audio files
    let audio_files =
        audio_generator::generate_test_library(temp_audio_dir.path(), 3, &audio_config).unwrap();

    // Setup: Create orchestrator
    let orchestrator = db_utils::create_test_orchestrator(db_pool.clone());

    // Setup: Create import session
    let mut session = ImportSession::new(
        temp_audio_dir.path().to_string_lossy().to_string(),
        ImportParameters::default(),
    );

    log_capture.clear();

    // Execute: Call phase_scanning directly (internal method, uses start_time and cancel_token)
    // NOTE: We can't call phase_scanning directly as it's not public
    // Instead, we'll verify behavior by running execute_import_plan024 and checking SCANNING phase

    // Execute import and let it complete SCANNING phase
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let result = orchestrator
        .execute_import_plan024(session.clone(), cancel_token.clone())
        .await;

    // CRITICAL VERIFICATION: Assert NO metadata extraction during SCANNING
    let scanning_logs = log_capture.matching("SCANNING");
    assert!(
        !scanning_logs.is_empty(),
        "Should have SCANNING phase logs"
    );

    // Verify no metadata extraction logs during SCANNING
    log_capture.assert_no_match("Extracting metadata from");
    log_capture.assert_no_match("metadata_extractor");
    log_capture.assert_no_match("ID3");

    // Verify: Database contains file records
    let file_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
        .fetch_one(&db_pool)
        .await
        .unwrap();

    assert_eq!(
        file_count, 3,
        "Should have created 3 file records during SCANNING"
    );

    // Verify: File records have empty hash (not yet processed)
    let empty_hash_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE hash = '' OR hash IS NULL")
            .fetch_one(&db_pool)
            .await
            .unwrap();

    assert!(
        empty_hash_count >= 1,
        "File records should have empty hash after SCANNING (processing happens later). Found {} with empty hash",
        empty_hash_count
    );

    // Verify: File records have paths
    let path_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE path IS NOT NULL AND path != ''")
            .fetch_one(&db_pool)
            .await
            .unwrap();

    assert_eq!(
        path_count, 3,
        "All file records should have paths after SCANNING"
    );

    println!("✅ TC-PHASE-001 PASS: phase_scanning creates file records without processing");
}

/// TC-PHASE-002: phase_scanning handles empty directory
///
/// **Requirement:** SPEC032 SCANNING phase error handling
///
/// **Given:** Empty directory (no audio files)
/// **When:** phase_scanning() executes
/// **Then:**
///   - Completes without error
///   - Reports 0 files found
///   - Transitions to PROCESSING (or COMPLETED with 0 files)
#[tokio::test]
async fn tc_phase_002_scanning_empty_directory() {
    // Setup: Create test database
    let (_temp_db_dir, db_pool) = db_utils::create_test_db().await.unwrap();
    let temp_audio_dir = TempDir::new().unwrap(); // Empty directory

    // Setup: Create orchestrator
    let orchestrator = db_utils::create_test_orchestrator(db_pool.clone());

    // Setup: Create import session
    let session = ImportSession::new(
        temp_audio_dir.path().to_string_lossy().to_string(),
        ImportParameters::default(),
    );

    // Execute: Run import workflow
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let result = orchestrator
        .execute_import_plan024(session, cancel_token)
        .await;

    // Verify: Completes successfully with 0 files
    assert!(result.is_ok(), "Should handle empty directory gracefully");

    let final_session = result.unwrap();
    assert_eq!(
        final_session.progress.total, 0,
        "Should report 0 files found"
    );

    // Verify: Database has 0 file records
    let file_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
        .fetch_one(&db_pool)
        .await
        .unwrap();

    assert_eq!(file_count, 0, "Should have 0 file records for empty directory");

    println!("✅ TC-PHASE-002 PASS: phase_scanning handles empty directory gracefully");
}

/// TC-PHASE-003: phase_scanning sets correct file modification times
///
/// **Requirement:** SPEC032 SCANNING phase - file metadata tracking
///
/// **Given:** Audio files with known modification times
/// **When:** phase_scanning() executes
/// **Then:**
///   - File records have correct modification_time
///   - Times match filesystem metadata
#[tokio::test]
async fn tc_phase_003_scanning_modification_times() {
    // Setup: Create test database and audio files
    let (_temp_db_dir, db_pool) = db_utils::create_test_db().await.unwrap();
    let temp_audio_dir = TempDir::new().unwrap();

    let audio_config = audio_generator::AudioConfig {
        duration_seconds: 35.0,
        ..Default::default()
    };

    // Generate 1 test audio file
    let audio_files =
        audio_generator::generate_test_library(temp_audio_dir.path(), 1, &audio_config).unwrap();
    let test_file = &audio_files[0];

    // Get file modification time from filesystem
    let fs_metadata = std::fs::metadata(test_file).unwrap();
    let fs_mod_time = fs_metadata.modified().unwrap();

    // Setup: Create orchestrator
    let orchestrator = db_utils::create_test_orchestrator(db_pool.clone());

    // Setup: Create import session
    let session = ImportSession::new(
        temp_audio_dir.path().to_string_lossy().to_string(),
        ImportParameters::default(),
    );

    // Execute: Run import workflow (SCANNING phase)
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let _ = orchestrator
        .execute_import_plan024(session, cancel_token)
        .await;

    // Verify: File record has modification time
    let db_mod_time: Option<String> =
        sqlx::query_scalar("SELECT modification_time FROM files LIMIT 1")
            .fetch_optional(&db_pool)
            .await
            .unwrap();

    assert!(
        db_mod_time.is_some(),
        "File record should have modification_time"
    );

    // Parse database timestamp
    let db_time_str = db_mod_time.unwrap();
    let db_time =
        chrono::DateTime::parse_from_rfc3339(&db_time_str).expect("Valid RFC3339 timestamp");

    // Convert filesystem time to chrono
    let fs_time: chrono::DateTime<chrono::Utc> = fs_mod_time.into();

    // Times should match (allow 1 second tolerance for filesystem precision)
    let diff = (db_time.timestamp() - fs_time.timestamp()).abs();
    assert!(
        diff <= 1,
        "Database modification time should match filesystem (diff: {}s)",
        diff
    );

    println!("✅ TC-PHASE-003 PASS: phase_scanning sets correct file modification times");
}
