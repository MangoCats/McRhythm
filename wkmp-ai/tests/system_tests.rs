// System Tests for PLAN023 - Multi-Song Import and Performance Validation
//
// Tests complete end-to-end scenarios with multiple files, database persistence,
// and performance requirements validation

use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;
use wkmp_ai::workflow::{event_bridge, song_processor::*};

/// Generate multiple test WAV files with different durations
fn generate_test_audio_library(num_files: usize, duration_secs: f64) -> (TempDir, Vec<PathBuf>) {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut files = Vec::new();

    for i in 0..num_files {
        let wav_path = temp_dir.path().join(format!("track_{:02}.wav", i + 1));

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(&wav_path, spec).unwrap();
        let total_samples = (duration_secs * 44100.0) as usize;

        // Generate audio with varying patterns to create different passages
        // Track 1: Single long passage (no silence)
        // Track 2+: Multiple passages with silence gaps
        let (silence_gap_start, silence_gap_end) = if i == 0 {
            (total_samples + 1, total_samples + 2) // No silence for first track
        } else {
            (total_samples / 2, total_samples / 2 + 88200) // 2 seconds silence
        };

        for sample_idx in 0..total_samples {
            let sample = if sample_idx >= silence_gap_start && sample_idx < silence_gap_end {
                // Silence gap
                0
            } else {
                // Non-silent audio (different frequency per track)
                let freq = 440.0 + (i as f32 * 50.0);
                let t = sample_idx as f32 / 44100.0;
                let amplitude = 0.3;
                (amplitude * (2.0 * std::f32::consts::PI * freq * t).sin() * i16::MAX as f32) as i16
            };

            writer.write_sample(sample).unwrap();
            writer.write_sample(sample).unwrap(); // Stereo
        }

        writer.finalize().unwrap();
        files.push(wav_path);
    }

    (temp_dir, files)
}

#[tokio::test]
async fn test_system_multi_file_import() {
    // TC-S-010-01: Multi-song import test

    // Generate 3 test files (35s each to ensure passages > 30s)
    let (_temp_dir, files) = generate_test_audio_library(3, 35.0);

    // Create processor with audio-derived extractor
    let (event_tx, _event_rx) = tokio::sync::mpsc::channel(100);
    let config = SongProcessorConfig {
        acoustid_api_key: String::new(),
        enable_musicbrainz: false,
        enable_audio_derived: true,
        enable_database_storage: false,
    };

    let processor = SongProcessor::new(config, event_tx);

    // Process all files
    let mut all_passages = Vec::new();
    for file in &files {
        let result = processor.process_file(file).await;
        assert!(result.is_ok(), "File processing should succeed: {:?}", file);

        let passages = result.unwrap();
        assert!(!passages.is_empty(), "Should extract passages from file: {:?}", file);
        all_passages.extend(passages);
    }

    // Verify we processed multiple files
    assert!(all_passages.len() >= 3, "Should have at least 3 passages (1 per file)");

    // Verify all passages have valid fusion results
    for passage in &all_passages {
        assert!(passage.fusion.metadata.completeness >= 0.0, "Metadata completeness should be valid");
        assert!(passage.fusion.flavor.completeness >= 0.0, "Flavor completeness should be valid");
        assert!(passage.validation.quality_score >= 0.0, "Quality score should be valid");
        assert!(passage.validation.quality_score <= 100.0, "Quality score should be <= 100%");
    }

    println!("✅ Processed {} files, extracted {} passages", files.len(), all_passages.len());
}

#[tokio::test]
async fn test_system_performance_validation() {
    // Performance requirement: ≤2 min/song processing time
    // Test with 45-second audio file (above min passage threshold)

    let (_temp_dir, files) = generate_test_audio_library(1, 45.0);
    let file = &files[0];

    let (event_tx, _event_rx) = tokio::sync::mpsc::channel(100);
    let config = SongProcessorConfig {
        acoustid_api_key: String::new(),
        enable_musicbrainz: false,
        enable_audio_derived: true,
        enable_database_storage: false,
    };

    let processor = SongProcessor::new(config, event_tx);

    // Measure processing time
    let start = Instant::now();
    let result = processor.process_file(file).await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Processing should succeed");

    let passages = result.unwrap();
    assert!(!passages.is_empty(), "Should extract passages");

    // Performance requirement: ≤120 seconds per song
    // For a 45-second song without network calls, should be much faster
    let max_allowed = std::time::Duration::from_secs(120);
    assert!(
        duration < max_allowed,
        "Processing took {:?}, should be < {:?}",
        duration,
        max_allowed
    );

    println!("✅ Processed in {:?} (< 2min requirement)", duration);
}

#[tokio::test]
#[ignore] // Requires full database schema - run separately with: cargo test -- --ignored
async fn test_system_database_persistence() {
    // Test complete workflow with database storage
    // NOTE: This test is ignored by default because it requires the full database schema
    // Run with: cargo test --test system_tests test_system_database_persistence -- --ignored

    use sqlx::sqlite::SqlitePoolOptions;

    // Create temporary database
    let db_path = tempfile::NamedTempFile::new().unwrap();
    let db_url = format!("sqlite://{}?mode=rwc", db_path.path().display());

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .expect("Failed to create test database");

    // Create minimal schema for test (SPEC017: times as INTEGER ticks)
    sqlx::query(r#"
        CREATE TABLE passages (
            guid TEXT PRIMARY KEY,
            file_path TEXT NOT NULL,
            start_time INTEGER NOT NULL,
            end_time INTEGER NOT NULL,
            title TEXT,
            artist TEXT,
            album TEXT,
            recording_mbid TEXT,
            musical_flavor TEXT NOT NULL,
            flavor_source_blend TEXT NOT NULL,
            flavor_confidence_map TEXT NOT NULL,
            flavor_completeness REAL NOT NULL,
            title_source TEXT,
            title_confidence REAL,
            artist_source TEXT,
            artist_confidence REAL,
            identity_confidence REAL NOT NULL,
            identity_conflicts TEXT,
            overall_quality_score REAL NOT NULL,
            metadata_completeness REAL NOT NULL,
            validation_status TEXT NOT NULL,
            validation_report TEXT NOT NULL,
            import_session_id TEXT NOT NULL,
            import_timestamp INTEGER NOT NULL,
            import_strategy TEXT NOT NULL
        )
    "#)
    .execute(&pool)
    .await
    .expect("Failed to create passages table");

    sqlx::query(r#"
        CREATE TABLE import_provenance (
            id TEXT PRIMARY KEY,
            passage_id TEXT NOT NULL,
            source_type TEXT NOT NULL,
            data_extracted TEXT NOT NULL,
            confidence REAL NOT NULL,
            timestamp INTEGER NOT NULL
        )
    "#)
    .execute(&pool)
    .await
    .expect("Failed to create import_provenance table");

    // Generate test file
    let (_temp_dir, files) = generate_test_audio_library(1, 40.0);
    let file = &files[0];

    // Create processor with database enabled
    let (event_tx, _event_rx) = tokio::sync::mpsc::channel(100);
    let config = SongProcessorConfig {
        acoustid_api_key: String::new(),
        enable_musicbrainz: false,
        enable_audio_derived: true,
        enable_database_storage: true,
    };

    let processor = SongProcessor::with_database(config, event_tx, pool.clone());

    // Process file
    let result = processor.process_file(file).await;
    assert!(result.is_ok(), "Processing with database should succeed");

    let passages = result.unwrap();
    assert!(!passages.is_empty(), "Should extract passages");

    // Verify passages were written to database
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passages")
        .fetch_one(&pool)
        .await
        .expect("Failed to query passages");

    assert_eq!(count, passages.len() as i64, "All passages should be in database");

    // Verify provenance logs were written
    let provenance_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM import_provenance")
        .fetch_one(&pool)
        .await
        .expect("Failed to query provenance");

    assert!(provenance_count > 0, "Should have provenance log entries");

    // Verify passage data integrity (SPEC017: query by tick range)
    const TICK_RATE: i64 = 28_224_000;
    for passage in &passages {
        let duration_ticks = passage.boundary.end_time - passage.boundary.start_time;
        let tolerance_ticks = TICK_RATE; // ±1 second tolerance

        let db_passage: Option<String> = sqlx::query_scalar(
            "SELECT title FROM passages WHERE (end_time - start_time) BETWEEN ? AND ?"
        )
        .bind(duration_ticks - tolerance_ticks)
        .bind(duration_ticks + tolerance_ticks)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query passage");

        // Note: title may be NULL if no extractors provided it
        assert!(db_passage.is_some(), "Passage should exist in database");
    }

    println!("✅ Verified {} passages in database with {} provenance entries",
             count, provenance_count);
}

#[tokio::test]
async fn test_system_stress_test_10_files() {
    // Stress test: Process 10 files to verify stability

    let (_temp_dir, files) = generate_test_audio_library(10, 35.0);

    let (event_tx, _event_rx) = tokio::sync::mpsc::channel(1000);
    let config = SongProcessorConfig {
        acoustid_api_key: String::new(),
        enable_musicbrainz: false,
        enable_audio_derived: true,
        enable_database_storage: false,
    };

    let processor = SongProcessor::new(config, event_tx);

    let start = Instant::now();
    let mut total_passages = 0;
    let mut failed_files = 0;

    for (idx, file) in files.iter().enumerate() {
        match processor.process_file(file).await {
            Ok(passages) => {
                assert!(!passages.is_empty(), "File {} should have passages", idx + 1);
                total_passages += passages.len();
            }
            Err(e) => {
                eprintln!("⚠️  File {} failed: {}", idx + 1, e);
                failed_files += 1;
            }
        }
    }

    let duration = start.elapsed();

    // All files should succeed
    assert_eq!(failed_files, 0, "All files should process successfully");
    assert!(total_passages >= 10, "Should extract at least 10 passages (1 per file)");

    println!("✅ Stress test: {} files → {} passages in {:?}",
             files.len(), total_passages, duration);
    println!("   Average: {:?} per file", duration / files.len() as u32);
}

#[tokio::test]
async fn test_system_event_flow_complete() {
    // Test complete event flow from file processing through SSE broadcasting

    let (_temp_dir, files) = generate_test_audio_library(1, 40.0);
    let file_path = files[0].clone();

    // Create event infrastructure
    let (workflow_tx, workflow_rx) = tokio::sync::mpsc::channel(100);
    let (event_bus_tx, _) = tokio::sync::broadcast::channel(100);
    let session_id = uuid::Uuid::new_v4();

    // Subscribe to broadcast events before spawning bridge
    let mut event_rx = event_bus_tx.subscribe();

    // Spawn event bridge
    let bridge_handle = tokio::spawn(event_bridge::bridge_workflow_events(
        workflow_rx,
        event_bus_tx,
        session_id,
    ));

    // Create processor
    let config = SongProcessorConfig {
        acoustid_api_key: String::new(),
        enable_musicbrainz: false,
        enable_audio_derived: true,
        enable_database_storage: false,
    };

    let processor = SongProcessor::new(config, workflow_tx);

    // Process file in background
    let process_handle = tokio::spawn(async move {
        processor.process_file(&file_path).await
    });

    // Collect broadcast events
    let mut wkmp_events = Vec::new();
    let collection_timeout = std::time::Duration::from_secs(10);
    let collection_start = Instant::now();

    while collection_start.elapsed() < collection_timeout {
        match tokio::time::timeout(
            std::time::Duration::from_millis(100),
            event_rx.recv()
        ).await {
            Ok(Ok(event)) => {
                use wkmp_common::events::WkmpEvent;
                if let WkmpEvent::ImportProgressUpdate { session_id: sid, state, current_operation, .. } = &event {
                    if sid == &session_id {
                        println!("Event: {} - {}", state, current_operation);
                        wkmp_events.push(event);
                    }
                }
            }
            Ok(Err(_)) => break, // Channel closed
            Err(_) => {
                // Timeout - check if processing is done
                if process_handle.is_finished() {
                    break;
                }
            }
        }
    }

    // Wait for processing to complete
    let result = process_handle.await.unwrap();
    assert!(result.is_ok(), "Processing should succeed");

    // Verify we received workflow events
    assert!(!wkmp_events.is_empty(), "Should receive SSE events");

    // Verify event sequence includes key states
    let states: Vec<String> = wkmp_events.iter().filter_map(|e| {
        use wkmp_common::events::WkmpEvent;
        if let WkmpEvent::ImportProgressUpdate { state, .. } = e {
            Some(state.clone())
        } else {
            None
        }
    }).collect();

    assert!(states.contains(&"PROCESSING".to_string()), "Should have PROCESSING state");

    // May have SEGMENTING, EXTRACTING, FUSING, VALIDATING depending on timing
    println!("✅ Received {} events with states: {:?}", wkmp_events.len(), states);

    // Clean up bridge
    drop(event_rx);
    bridge_handle.await.unwrap();
}

#[tokio::test]
async fn test_system_error_recovery() {
    // Test that workflow handles errors gracefully and continues processing

    let (event_tx, _event_rx) = tokio::sync::mpsc::channel(100);
    let config = SongProcessorConfig {
        acoustid_api_key: String::new(),
        enable_musicbrainz: false,
        enable_audio_derived: true,
        enable_database_storage: false,
    };

    let processor = SongProcessor::new(config, event_tx);

    // Try to process a non-existent file
    let fake_file = PathBuf::from("/nonexistent/file.wav");
    let result = processor.process_file(&fake_file).await;

    // Should return error (boundary detector fails)
    assert!(result.is_err(), "Should fail for non-existent file");

    // Verify error message contains file info
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.len() > 0, "Should have error message");

    // Now process a valid file to verify recovery (processor can be reused)
    let (_temp_dir, files) = generate_test_audio_library(1, 40.0);
    let result = processor.process_file(&files[0]).await;

    assert!(result.is_ok(), "Should recover and process valid file after error");

    println!("✅ Error recovery verified - processor reusable after errors");
}
