//! Startup Time Performance Benchmark (CRITICAL)
//!
//! Measures end-to-end startup latency from API enqueue to first audio sample.
//!
//! **Phase 5 Goal:** <100ms startup time
//! **Phase 1 Baseline:** ~1,500ms
//! **Required Improvement:** 15x faster (93% reduction)
//!
//! This is the PRIMARY performance requirement for Phase 5.
//!
//! ## Test Scenarios
//!
//! 1. **Best Case**: MP3 @ 44.1kHz starting at 0s (no resampling, no skip)
//!    - Target: <100ms
//!
//! 2. **Decode-and-Skip**: MP3 @ 44.1kHz starting at 60s (requires skip)
//!    - Target: <150ms
//!
//! 3. **Resample Case**: FLAC @ 48kHz (requires resampling to 44.1kHz)
//!    - Target: <200ms
//!
//! 4. **Worst Case**: FLAC @ 96kHz starting at 60s (resample + skip)
//!    - Target: <300ms
//!
//! ## Measured Timeline
//!
//! 1. API request received → request parsed (target: <5ms)
//! 2. Request parsed → database query complete (target: <10ms)
//! 3. Database query → decoder initialized (target: <20ms)
//! 4. Decoder initialized → decode-and-skip complete (target: <50ms)
//! 5. Skip complete → minimum buffer filled (target: <10ms)
//! 6. Buffer filled → mixer activated (target: <5ms)
//! 7. Mixer activated → first audio sample output (target: <5ms)
//!
//! **Total Target:** <100ms
//!
//! ## Requirements Traceability
//!
//! - [PERF-START-010] Instant playback start
//! - [PERF-FIRST-010] First-passage 500ms buffer optimization
//! - [PERF-POLL-010] Event-driven buffer readiness
//! - [SSD-PBUF-028] Enable instant play start with background decode

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use uuid::Uuid;

use wkmp_ap::playback::engine::PlaybackEngine;
use wkmp_ap::state::SharedState;
use sqlx::sqlite::SqlitePoolOptions;

/// Test helper: Setup in-memory database with minimal schema
async fn create_test_database() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create minimal schema for benchmarking
    sqlx::query(
        r#"
        CREATE TABLE settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create settings table");

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

    // Configure for instant startup: 500ms minimum buffer
    sqlx::query("INSERT INTO settings (key, value) VALUES ('minimum_buffer_threshold_ms', '500')")
        .execute(&pool)
        .await
        .expect("Failed to set minimum buffer threshold");

    pool
}

/// Test helper: Create playback engine for benchmarking
async fn create_test_engine() -> (Arc<SharedState>, Arc<PlaybackEngine>) {
    let db_pool = create_test_database().await;
    let state = Arc::new(SharedState::new());
    let engine = Arc::new(
        PlaybackEngine::new(db_pool, Arc::clone(&state))
            .await
            .expect("Failed to create engine"),
    );

    (state, engine)
}

/// Test helper: Wait for first audio sample to be output
///
/// In real implementation, this would monitor the audio callback.
/// For benchmarking, we wait for mixer to be active and buffer to have samples.
async fn wait_for_first_audio_sample(
    engine: &PlaybackEngine,
    queue_entry_id: Uuid,
) -> Duration {
    let start = Instant::now();
    let timeout = Duration::from_secs(5); // Fail benchmark if >5s

    loop {
        // Check if mixer is active and playing
        // This is a simplified check - real implementation would monitor audio thread
        if engine.is_playing().await {
            // Additional check: buffer has samples
            if engine.has_buffer_ready(queue_entry_id).await {
                return start.elapsed();
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;

        if start.elapsed() > timeout {
            panic!("Timeout waiting for first audio sample (>5s)");
        }
    }
}

/// Benchmark: Startup time - Best case (MP3 @ 44.1kHz, start at 0s)
///
/// **Target:** <100ms
/// **Phase 1 Baseline:** ~1,500ms
fn bench_startup_mp3_44100_start_0s(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let mut group = c.benchmark_group("startup_time");

    // Configure for statistical significance
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(60));

    group.bench_function("mp3_44100_start_0s", |b| {
        b.to_async(&runtime).iter(|| async {
            let (_state, engine) = create_test_engine().await;

            let start = Instant::now();

            // 1. Enqueue passage (ephemeral, no database passage entity)
            let test_file = PathBuf::from("/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-02-Superfly_.mp3");

            let queue_entry_id = engine
                .enqueue_file(
                    black_box(test_file),
                    black_box(0),          // start_time_ms
                    black_box(180_000),    // end_time_ms (3 minutes)
                    black_box(None),       // lead_in_point_ms
                    black_box(None),       // lead_out_point_ms
                    black_box(None),       // fade_in_point_ms
                    black_box(None),       // fade_out_point_ms
                    black_box(None),       // fade_in_curve
                    black_box(None),       // fade_out_curve
                )
                .await
                .expect("Failed to enqueue passage");

            // 2. Start playback
            engine
                .play()
                .await
                .expect("Failed to start playback");

            // 3. Wait for first audio sample
            let elapsed = wait_for_first_audio_sample(&engine, queue_entry_id).await;

            // Cleanup
            engine.stop().await.expect("Failed to stop playback");
            engine.clear_queue().await.expect("Failed to clear queue");

            // Assert target met
            if elapsed > Duration::from_millis(100) {
                eprintln!(
                    "WARNING: Startup time {}ms exceeds 100ms target",
                    elapsed.as_millis()
                );
            }

            elapsed
        })
    });

    group.finish();
}

/// Benchmark: Startup time - Decode-and-skip (MP3 @ 44.1kHz, start at 60s)
///
/// **Target:** <150ms
/// **Challenge:** Requires linear decode-and-skip to reach start point
fn bench_startup_mp3_44100_start_60s(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let mut group = c.benchmark_group("startup_time");

    group.sample_size(50); // Fewer samples due to decode overhead
    group.measurement_time(Duration::from_secs(60));

    group.bench_function("mp3_44100_start_60s", |b| {
        b.to_async(&runtime).iter(|| async {
            let (_state, engine) = create_test_engine().await;

            let start = Instant::now();

            let test_file = PathBuf::from("/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-02-Superfly_.mp3");

            let queue_entry_id = engine
                .enqueue_file(
                    black_box(test_file),
                    black_box(60_000),     // start_time_ms - SKIP TO 60s
                    black_box(180_000),    // end_time_ms
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                )
                .await
                .expect("Failed to enqueue passage");

            engine.play().await.expect("Failed to start playback");
            let elapsed = wait_for_first_audio_sample(&engine, queue_entry_id).await;

            engine.stop().await.expect("Failed to stop playback");
            engine.clear_queue().await.expect("Failed to clear queue");

            // Decode-and-skip should still be fast
            if elapsed > Duration::from_millis(150) {
                eprintln!(
                    "WARNING: Skip startup time {}ms exceeds 150ms target",
                    elapsed.as_millis()
                );
            }

            elapsed
        })
    });

    group.finish();
}

/// Benchmark: Startup time - Resample case (FLAC @ 48kHz)
///
/// **Target:** <200ms
/// **Challenge:** Requires rubato resampling 48kHz → 44.1kHz
fn bench_startup_flac_48000_resample(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let mut group = c.benchmark_group("startup_time");

    group.sample_size(50);
    group.measurement_time(Duration::from_secs(60));

    group.bench_function("flac_48000_resample", |b| {
        b.to_async(&runtime).iter(|| async {
            let (_state, engine) = create_test_engine().await;

            let start = Instant::now();

            // NOTE: This test file must exist and be 48kHz FLAC
            // If not available, this benchmark will fail
            let test_file = PathBuf::from("/test/audio/test_48000.flac");

            let queue_entry_id = engine
                .enqueue_file(
                    black_box(test_file),
                    black_box(0),
                    black_box(180_000),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                )
                .await
                .expect("Failed to enqueue passage");

            engine.play().await.expect("Failed to start playback");
            let elapsed = wait_for_first_audio_sample(&engine, queue_entry_id).await;

            engine.stop().await.expect("Failed to stop playback");
            engine.clear_queue().await.expect("Failed to clear queue");

            // Resampling adds overhead
            if elapsed > Duration::from_millis(200) {
                eprintln!(
                    "WARNING: Resample startup time {}ms exceeds 200ms target",
                    elapsed.as_millis()
                );
            }

            elapsed
        })
    });

    group.finish();
}

/// Benchmark: Startup time - Worst case (FLAC @ 96kHz, start at 60s)
///
/// **Target:** <300ms
/// **Challenge:** High sample rate resample + decode-and-skip
fn bench_startup_flac_96000_worst_case(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let mut group = c.benchmark_group("startup_time");

    group.sample_size(30); // Fewer samples due to high overhead
    group.measurement_time(Duration::from_secs(60));

    group.bench_function("flac_96000_worst_case", |b| {
        b.to_async(&runtime).iter(|| async {
            let (_state, engine) = create_test_engine().await;

            let start = Instant::now();

            let test_file = PathBuf::from("/test/audio/test_96000.flac");

            let queue_entry_id = engine
                .enqueue_file(
                    black_box(test_file),
                    black_box(60_000),     // start_time_ms - SKIP TO 60s
                    black_box(180_000),    // end_time_ms
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                )
                .await
                .expect("Failed to enqueue passage");

            engine.play().await.expect("Failed to start playback");
            let elapsed = wait_for_first_audio_sample(&engine, queue_entry_id).await;

            engine.stop().await.expect("Failed to stop playback");
            engine.clear_queue().await.expect("Failed to clear queue");

            // Worst case still has a target
            if elapsed > Duration::from_millis(300) {
                eprintln!(
                    "WARNING: Worst-case startup time {}ms exceeds 300ms target",
                    elapsed.as_millis()
                );
            }

            elapsed
        })
    });

    group.finish();
}

/// Benchmark: Component breakdown - API request parsing
///
/// Isolate API handler overhead
fn bench_component_api_parsing(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let mut group = c.benchmark_group("startup_components");

    group.bench_function("api_request_parsing", |b| {
        b.to_async(&runtime).iter(|| async {
            let start = Instant::now();

            // Simulate API request parsing (JSON deserialization)
            let json_payload = r#"{"file_path":"/test/audio/test.mp3","start_time_ms":0,"end_time_ms":180000}"#;
            let parsed: serde_json::Value = serde_json::from_str(black_box(json_payload))
                .expect("Failed to parse JSON");

            black_box(parsed);

            let elapsed = start.elapsed();

            // Target: <5ms
            if elapsed > Duration::from_millis(5) {
                eprintln!(
                    "WARNING: API parsing {}ms exceeds 5ms target",
                    elapsed.as_millis()
                );
            }

            elapsed
        })
    });

    group.finish();
}

/// Benchmark: Component breakdown - Database query
///
/// Isolate database INSERT overhead
fn bench_component_db_insert(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let mut group = c.benchmark_group("startup_components");

    group.bench_function("database_insert", |b| {
        b.to_async(&runtime).iter(|| async {
            let pool = create_test_database().await;
            let start = Instant::now();

            // Simulate queue entry INSERT
            sqlx::query(
                r#"
                INSERT INTO queue (
                    guid, file_path, play_order, start_time_ms, end_time_ms
                ) VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind("/test/audio/test.mp3")
            .bind(1)
            .bind(0)
            .bind(180000)
            .execute(&pool)
            .await
            .expect("Failed to insert queue entry");

            let elapsed = start.elapsed();

            // Target: <10ms
            if elapsed > Duration::from_millis(10) {
                eprintln!(
                    "WARNING: DB insert {}ms exceeds 10ms target",
                    elapsed.as_millis()
                );
            }

            elapsed
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_startup_mp3_44100_start_0s,
    bench_startup_mp3_44100_start_60s,
    bench_startup_flac_48000_resample,
    bench_startup_flac_96000_worst_case,
    bench_component_api_parsing,
    bench_component_db_insert,
);
criterion_main!(benches);
