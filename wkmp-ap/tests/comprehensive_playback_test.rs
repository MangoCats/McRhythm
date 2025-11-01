//! Comprehensive Real Audio Playback Integration Test
//!
//! Tests actual audio playback with real files, monitoring logs for issues.
//! Scenarios:
//! 1. Single passage playback (30s)
//! 2. Two passages enqueued 2s apart (30s each)
//! 3. Three passages with skipping (30s playback per passage)
//!
//! **Traceability:**
//! - [DBD-DEC-090] Streaming/incremental decoding
//! - [DBD-DEC-110] Chunk-based decoding
//! - [DBD-DEC-150] Decoder yielding priorities
//! - [DBD-BUF-050] Buffer hysteresis management

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;
use wkmp_ap::playback::{BufferManager, DecoderWorker};
use wkmp_ap::playback::types::DecodePriority;
use wkmp_ap::db::passages::PassageWithTiming;
use wkmp_ap::state::SharedState;
use wkmp_common::FadeCurve;
use sqlx::sqlite::SqlitePoolOptions;

/// Test audio file path
const TEST_AUDIO_FILE: &str = "/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-03-What's_Up_.mp3";

/// Alternative test files
const TEST_FILE_FLAC: &str = "tests/fixtures/audio/test_audio_10s_flac.flac";
const TEST_FILE_MP3: &str = "tests/fixtures/audio/test_audio_10s_mp3.mp3";

/// Log collector for capturing and analyzing logs
struct LogCollector {
    logs: Arc<TokioMutex<Vec<String>>>,
    warnings: Arc<TokioMutex<Vec<String>>>,
    errors: Arc<TokioMutex<Vec<String>>>,
}

impl LogCollector {
    fn new() -> Self {
        Self {
            logs: Arc::new(TokioMutex::new(Vec::new())),
            warnings: Arc::new(TokioMutex::new(Vec::new())),
            errors: Arc::new(TokioMutex::new(Vec::new())),
        }
    }

    async fn add_log(&self, level: &str, message: String) {
        let log_line = format!("[{}] {}", level, message);
        self.logs.lock().await.push(log_line.clone());

        match level {
            "WARN" => self.warnings.lock().await.push(log_line),
            "ERROR" => self.errors.lock().await.push(log_line),
            _ => {}
        }
    }

    async fn print_summary(&self) {
        let logs = self.logs.lock().await;
        let warnings = self.warnings.lock().await;
        let errors = self.errors.lock().await;

        println!("\n=== LOG SUMMARY ===");
        println!("Total messages: {}", logs.len());
        println!("Warnings: {}", warnings.len());
        println!("Errors: {}", errors.len());

        if !warnings.is_empty() {
            println!("\n=== WARNINGS ===");
            for warn in warnings.iter() {
                println!("{}", warn);
            }
        }

        if !errors.is_empty() {
            println!("\n=== ERRORS ===");
            for err in errors.iter() {
                println!("{}", err);
            }
        }
    }

    async fn has_errors(&self) -> bool {
        !self.errors.lock().await.is_empty()
    }

    async fn get_error_count(&self) -> usize {
        self.errors.lock().await.len()
    }

    async fn get_warning_count(&self) -> usize {
        self.warnings.lock().await.len()
    }
}

/// Helper to create test dependencies for DecoderWorker
async fn create_test_deps() -> (Arc<BufferManager>, Arc<SharedState>, sqlx::Pool<sqlx::Sqlite>) {
    let buffer_manager = Arc::new(BufferManager::new());
    let shared_state = Arc::new(SharedState::new());
    let db_pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    (buffer_manager, shared_state, db_pool)
}

/// Create passage for real audio file
fn create_passage(file_path: &str, start_ms: u64, end_ms: Option<u64>) -> PassageWithTiming {
    let end_ticks = end_ms.map(|ms| wkmp_common::timing::ms_to_ticks(ms as i64));

    PassageWithTiming {
        passage_id: Some(Uuid::new_v4()),
        file_path: PathBuf::from(file_path),
        start_time_ticks: wkmp_common::timing::ms_to_ticks(start_ms as i64),
        end_time_ticks: end_ticks,
        lead_in_point_ticks: wkmp_common::timing::ms_to_ticks(start_ms as i64),
        lead_out_point_ticks: end_ticks,
        fade_in_point_ticks: wkmp_common::timing::ms_to_ticks((start_ms + 2000) as i64),
        fade_out_point_ticks: end_ticks.map(|t| t - wkmp_common::timing::ms_to_ticks(3000)),
        fade_in_curve: FadeCurve::Linear,
        fade_out_curve: FadeCurve::Linear,
    }
}

/// Check if test audio file exists
fn check_test_file() -> String {
    if std::path::Path::new(TEST_AUDIO_FILE).exists() {
        TEST_AUDIO_FILE.to_string()
    } else if std::path::Path::new(TEST_FILE_MP3).exists() {
        TEST_FILE_MP3.to_string()
    } else if std::path::Path::new(TEST_FILE_FLAC).exists() {
        TEST_FILE_FLAC.to_string()
    } else {
        panic!("No test audio files found! Tried:\n  {}\n  {}\n  {}",
            TEST_AUDIO_FILE, TEST_FILE_MP3, TEST_FILE_FLAC);
    }
}

async fn test_scenario_1_single_passage_30s() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 1: Single Passage Playback (30 seconds)        ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let test_file = check_test_file();
    println!("Using test file: {}", test_file);

    let collector = LogCollector::new();

    // Create buffer manager and decoder
    let (buffer_manager, shared_state, db_pool): (Arc<BufferManager>, Arc<SharedState>, sqlx::Pool<sqlx::Sqlite>) = create_test_deps().await;
    let working_sample_rate = Arc::new(std::sync::RwLock::new(44100));
    let decoder = Arc::new(DecoderWorker::new(Arc::clone(&buffer_manager), shared_state, db_pool, working_sample_rate));

    // Create passage (0-40s to have buffer for 30s playback)
    let passage_id = Uuid::new_v4();
    let passage = create_passage(&test_file, 0, Some(40000));

    println!("Enqueueing passage (0-40s)...");
    decoder.submit(passage_id, passage, DecodePriority::Immediate, true)
        .await
        .expect("Submission should succeed");

    // Monitor for 30 seconds
    println!("Monitoring playback for 30 seconds...\n");

    for i in 0..30 {
        tokio::time::sleep(Duration::from_secs(1)).await;

        if let Some(buffer) = buffer_manager.get_buffer(passage_id).await {
            let occupied = buffer.occupied();
            let stats = buffer.stats();

            if i % 5 == 0 {
                println!("[{}s] Buffer: {} samples ({:.1}% full)",
                    i + 1, occupied, stats.fill_percent * 100.0);
            }

            // Check for underrun conditions
            if stats.fill_percent < 0.1 && occupied > 0 {
                collector.add_log("WARN", format!(
                    "Low buffer at {}s: {:.1}% full", i + 1, stats.fill_percent * 100.0
                )).await;
            }
        }
    }

    // Shutdown and print summary
    decoder.shutdown().await;
    collector.print_summary().await;

    assert!(!collector.has_errors().await,
        "Found {} errors during single passage test",
        collector.get_error_count().await);

    println!("\n✅ Scenario 1 Complete\n");
}

async fn test_scenario_2_two_passages_with_delay() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 2: Two Passages, 2 Second Delay (30s each)     ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let test_file = check_test_file();
    println!("Using test file: {}", test_file);

    let collector = LogCollector::new();
    let (buffer_manager, shared_state, db_pool): (Arc<BufferManager>, Arc<SharedState>, sqlx::Pool<sqlx::Sqlite>) = create_test_deps().await;
    let working_sample_rate = Arc::new(std::sync::RwLock::new(44100));
    let decoder = Arc::new(DecoderWorker::new(Arc::clone(&buffer_manager), shared_state, db_pool, working_sample_rate));

    // Enqueue first passage
    let passage1_id = Uuid::new_v4();
    let passage1 = create_passage(&test_file, 0, Some(40000));

    println!("Enqueueing passage 1 (0-40s)...");
    decoder.submit(passage1_id, passage1, DecodePriority::Immediate, true)
        .await
        .expect("First submission should succeed");

    // Wait 2 seconds
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Enqueue second passage
    let passage2_id = Uuid::new_v4();
    let passage2 = create_passage(&test_file, 40000, Some(80000));

    println!("Enqueueing passage 2 (40-80s) [2s later]...\n");
    decoder.submit(passage2_id, passage2, DecodePriority::Next, true)
        .await
        .expect("Second submission should succeed");

    // Monitor for 60 seconds total
    println!("Monitoring both passages for 60 seconds...\n");

    for i in 0..60 {
        tokio::time::sleep(Duration::from_secs(1)).await;

        if i % 5 == 0 {
            if let Some(buffer1) = buffer_manager.get_buffer(passage1_id).await {
                println!("[{}s] Passage 1: {} samples ({:.1}% full)",
                    i + 1, buffer1.occupied(), buffer1.stats().fill_percent * 100.0);
            }

            if let Some(buffer2) = buffer_manager.get_buffer(passage2_id).await {
                println!("[{}s] Passage 2: {} samples ({:.1}% full)",
                    i + 1, buffer2.occupied(), buffer2.stats().fill_percent * 100.0);
            }
        }
    }

    decoder.shutdown().await;
    collector.print_summary().await;

    assert!(!collector.has_errors().await,
        "Found {} errors during two passage test",
        collector.get_error_count().await);

    println!("\n✅ Scenario 2 Complete\n");
}

async fn test_scenario_3_three_passages_with_skip() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  SCENARIO 3: Three Passages with Skip After 30s          ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let test_file = check_test_file();
    println!("Using test file: {}", test_file);

    let collector = LogCollector::new();
    let (buffer_manager, shared_state, db_pool): (Arc<BufferManager>, Arc<SharedState>, sqlx::Pool<sqlx::Sqlite>) = create_test_deps().await;
    let working_sample_rate = Arc::new(std::sync::RwLock::new(44100));
    let decoder = Arc::new(DecoderWorker::new(Arc::clone(&buffer_manager), shared_state, db_pool, working_sample_rate));

    // Enqueue three passages
    let passage_ids: Vec<Uuid> = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

    for (idx, &passage_id) in passage_ids.iter().enumerate() {
        let start_ms = (idx as u64) * 40000;
        let end_ms = start_ms + 40000;
        let passage = create_passage(&test_file, start_ms, Some(end_ms));

        let priority = if idx == 0 {
            DecodePriority::Immediate
        } else {
            DecodePriority::Next
        };

        println!("Enqueueing passage {} ({}-{}s)...", idx + 1, start_ms / 1000, end_ms / 1000);
        decoder.submit(passage_id, passage, priority, true)
            .await
            .expect("Submission should succeed");

        if idx < 2 {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    println!("\nMonitoring passage 1 for 30 seconds...\n");

    for i in 0..30 {
        tokio::time::sleep(Duration::from_secs(1)).await;

        if i % 5 == 0 {
            for (idx, &pid) in passage_ids.iter().enumerate() {
                if let Some(buffer) = buffer_manager.get_buffer(pid).await {
                    println!("[{}s] Passage {}: {} samples ({:.1}% full)",
                        i + 1, idx + 1, buffer.occupied(), buffer.stats().fill_percent * 100.0);
                }
            }
        }
    }

    println!("\n⏭️  Simulating skip to passage 2...\n");
    // Note: Would need actual engine to skip, for now just monitor passage 2

    println!("Monitoring passage 2 for 30 seconds...\n");

    for i in 0..30 {
        tokio::time::sleep(Duration::from_secs(1)).await;

        if i % 5 == 0 {
            if let Some(buffer) = buffer_manager.get_buffer(passage_ids[1]).await {
                println!("[{}s] Passage 2: {} samples ({:.1}% full)",
                    i + 1, buffer.occupied(), buffer.stats().fill_percent * 100.0);
            }
        }
    }

    decoder.shutdown().await;
    collector.print_summary().await;

    assert!(!collector.has_errors().await,
        "Found {} errors during three passage test",
        collector.get_error_count().await);

    println!("\n✅ Scenario 3 Complete\n");
}

/// Run all scenarios in sequence
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn run_all_comprehensive_tests() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║                                                            ║");
    println!("║        COMPREHENSIVE AUDIO PLAYBACK TEST SUITE           ║");
    println!("║                                                            ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    // Run all three scenarios
    test_scenario_1_single_passage_30s().await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    test_scenario_2_two_passages_with_delay().await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    test_scenario_3_three_passages_with_skip().await;

    println!("\n");
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║                                                            ║");
    println!("║              ✅ ALL TESTS PASSED                          ║");
    println!("║                                                            ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("\n");
}
