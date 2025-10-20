//! Startup Performance Integration Test
//!
//! Tests actual startup latency from enqueue to first audio sample.
//!
//! **Target:** <100ms for first passage (PERF-TARGET-010)
//!
//! **Traceability:**
//! - [PERF-TARGET-010] First audio sample within 100ms
//! - [PERF-TARGET-020] 95th percentile < 150ms
//! - [PERF-FIRST-010] First passage 500ms minimum buffer
//! - [PERF-INIT-010] Parallel initialization

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;
use wkmp_ap::playback::engine::PlaybackEngine;
use wkmp_ap::state::SharedState;
use wkmp_ap::audio::types::BufferStatus;
use sqlx::sqlite::SqlitePoolOptions;

/// Test fixture paths
const TEST_MP3: &str = "tests/fixtures/audio/test_audio_10s_mp3.mp3";
const TEST_FLAC: &str = "tests/fixtures/audio/test_audio_10s_flac.flac";
const TEST_OGG: &str = "tests/fixtures/audio/test_audio_10s_vorbis.ogg";

/// Helper: Create in-memory test database
async fn create_test_db() -> sqlx::Pool<sqlx::Sqlite> {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create queue table
    sqlx::query(
        r#"
        CREATE TABLE queue (
            guid TEXT PRIMARY KEY,
            file_path TEXT NOT NULL,
            passage_guid TEXT,
            play_order INTEGER NOT NULL,
            start_time_ms INTEGER,
            end_time_ms INTEGER,
            lead_in_point_ms INTEGER,
            lead_out_point_ms INTEGER,
            fade_in_point_ms INTEGER,
            fade_out_point_ms INTEGER,
            fade_in_curve TEXT,
            fade_out_curve TEXT
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create queue table");

    // Create settings table
    sqlx::query(
        r#"
        CREATE TABLE settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create settings table");

    // Configure for Phase 5 fast startup: 500ms buffer threshold
    sqlx::query("INSERT INTO settings (key, value) VALUES ('minimum_buffer_threshold_ms', '500')")
        .execute(&pool)
        .await
        .ok();

    sqlx::query("INSERT INTO settings (key, value) VALUES ('volume', '0.5')")
        .execute(&pool)
        .await
        .ok();

    pool
}

/// Measure startup time: enqueue to buffer ready
///
/// Returns (elapsed_time, queue_entry_id)
async fn measure_startup(file_path: &str) -> (Duration, Uuid) {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());

    // Create engine
    let engine = PlaybackEngine::new(db, state.clone())
        .await
        .expect("Failed to create engine");

    // Start background tasks
    engine.start().await.expect("Failed to start engine");

    // Give background tasks time to initialize
    sleep(Duration::from_millis(50)).await;

    // Start measurement
    let start = Instant::now();

    // Enqueue passage
    let path = PathBuf::from(file_path);
    let queue_entry_id = engine
        .enqueue_file(path)
        .await
        .expect("Failed to enqueue file");

    // Set to Playing state
    engine.play().await.expect("Failed to play");

    // Wait for buffer to become Ready or Playing
    let timeout = Instant::now() + Duration::from_secs(5);
    let mut ready_time = None;
    let mut last_status_log = Instant::now();

    while Instant::now() < timeout {
        let statuses = engine.get_buffer_statuses().await;

        // Debug: Log status every 500ms
        if last_status_log.elapsed() > Duration::from_millis(500) {
            eprintln!("  Status check: {} buffers tracked", statuses.len());
            for (id, status) in &statuses {
                if *id == queue_entry_id {
                    eprintln!("    Target buffer {}: {:?}", id, status);
                }
            }
            last_status_log = Instant::now();
        }

        // Check if buffer for our queue entry is Ready, Playing, or Exhausted (decode complete)
        if let Some(status) = statuses.get(&queue_entry_id) {
            if matches!(
                status,
                BufferStatus::Ready | BufferStatus::Playing | BufferStatus::Exhausted
            ) {
                eprintln!("  ✅ Buffer ready (status: {:?})", status);
                ready_time = Some(start.elapsed());
                break;
            } else {
                // Still decoding
                eprintln!("  ⏳ Buffer status: {:?}", status);
            }
        }

        sleep(Duration::from_millis(5)).await;
    }

    // Cleanup
    engine.stop().await.ok();

    let elapsed = ready_time.unwrap_or_else(|| {
        panic!("Buffer never reached ready state (timeout after 5s)");
    });

    (elapsed, queue_entry_id)
}

#[tokio::test]
async fn test_startup_mp3_baseline() {
    // **[PERF-TARGET-010]** Target: <100ms for first passage
    println!("\n=== Testing MP3 startup (44.1kHz native rate) ===");

    let (elapsed, _queue_id) = measure_startup(TEST_MP3).await;

    println!("✅ Startup time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);

    // Assert target met
    assert!(
        elapsed < Duration::from_millis(100),
        "Startup time {:.2}ms exceeds 100ms target",
        elapsed.as_secs_f64() * 1000.0
    );
}

#[tokio::test]
async fn test_startup_flac() {
    // FLAC at 44.1kHz (no resampling needed)
    println!("\n=== Testing FLAC startup (44.1kHz native rate) ===");

    let (elapsed, _queue_id) = measure_startup(TEST_FLAC).await;

    println!("✅ Startup time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);

    // FLAC might be slightly slower than MP3 due to decompression
    assert!(
        elapsed < Duration::from_millis(150),
        "FLAC startup time {:.2}ms exceeds 150ms threshold",
        elapsed.as_secs_f64() * 1000.0
    );
}

#[tokio::test]
async fn test_startup_ogg() {
    // OGG/Vorbis at 44.1kHz
    println!("\n=== Testing OGG/Vorbis startup (44.1kHz native rate) ===");

    let (elapsed, _queue_id) = measure_startup(TEST_OGG).await;

    println!("✅ Startup time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);

    assert!(
        elapsed < Duration::from_millis(150),
        "OGG startup time {:.2}ms exceeds 150ms threshold",
        elapsed.as_secs_f64() * 1000.0
    );
}

#[tokio::test]
async fn test_startup_percentiles() {
    // Measure multiple runs to calculate percentiles
    // **[PERF-TARGET-020]** p95 < 150ms
    // **[PERF-TARGET-030]** p99 < 200ms

    println!("\n=== Testing startup percentiles (100 runs, MP3) ===");

    let mut times: Vec<Duration> = Vec::new();

    for i in 0..100 {
        let (elapsed, _) = measure_startup(TEST_MP3).await;
        times.push(elapsed);

        if (i + 1) % 25 == 0 {
            println!("  ... completed {}/100 runs", i + 1);
        }
    }

    // Sort times
    times.sort();

    // Calculate percentiles
    let p50 = times[50];
    let p95 = times[95];
    let p99 = times[99];

    println!("\n📊 Startup Time Percentiles:");
    println!("  p50 (median): {:.2}ms", p50.as_secs_f64() * 1000.0);
    println!("  p95:          {:.2}ms", p95.as_secs_f64() * 1000.0);
    println!("  p99:          {:.2}ms", p99.as_secs_f64() * 1000.0);

    // Assert targets
    assert!(
        p50 < Duration::from_millis(100),
        "p50 startup time {:.2}ms exceeds 100ms target",
        p50.as_secs_f64() * 1000.0
    );

    assert!(
        p95 < Duration::from_millis(150),
        "p95 startup time {:.2}ms exceeds 150ms target",
        p95.as_secs_f64() * 1000.0
    );

    assert!(
        p99 < Duration::from_millis(200),
        "p99 startup time {:.2}ms exceeds 200ms target",
        p99.as_secs_f64() * 1000.0
    );

    println!("\n✅ All percentile targets met!");
}

#[tokio::test]
async fn test_phase4_baseline_confirmation() {
    // This test confirms we've improved from Phase 4 baseline (~500ms)
    println!("\n=== Confirming improvement from Phase 4 baseline ===");
    println!("  Phase 4 estimated: ~500ms");
    println!("  Phase 5 target:    <100ms (5x improvement)");

    let (elapsed, _) = measure_startup(TEST_MP3).await;

    println!("  Measured:          {:.2}ms", elapsed.as_secs_f64() * 1000.0);

    let improvement_factor = 500.0 / (elapsed.as_secs_f64() * 1000.0);
    println!("  Improvement:       {:.1}x faster", improvement_factor);

    // We should be at least 3x faster than Phase 4
    assert!(
        elapsed < Duration::from_millis(166),
        "Not enough improvement from Phase 4 (need 3x, got {:.1}x)",
        improvement_factor
    );

    println!("\n✅ Phase 5 improvement confirmed!");
}
