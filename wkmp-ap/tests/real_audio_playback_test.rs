//! Real Audio Playback Integration Test
//!
//! Tests actual decoder and buffer filling with a real audio file,
//! capturing log output to detect warnings and errors.
//!
//! **Traceability:**
//! - [DBD-DEC-035] Decoder state persistence across yields
//! - [DBD-DEC-090] Streaming/incremental decoding
//! - [DBD-DEC-110] Chunk-based decoding
//! - [DBD-DEC-150] Decoder yielding priorities

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use wkmp_ap::playback::{BufferManager, DecoderWorker};
use wkmp_ap::playback::types::DecodePriority;
use wkmp_ap::state::SharedState;
use wkmp_ap::db::passages::PassageWithTiming;
use wkmp_common::FadeCurve;
use sqlx::sqlite::SqlitePoolOptions;

/// Test audio file path
const TEST_AUDIO_FILE: &str = "/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-03-What's_Up_.mp3";

/// Create passage for real audio file
fn create_real_passage(start_ms: u64, end_ms: u64) -> PassageWithTiming {
    PassageWithTiming {
        passage_id: Some(Uuid::new_v4()),
        file_path: PathBuf::from(TEST_AUDIO_FILE),
        start_time_ticks: wkmp_common::timing::ms_to_ticks(start_ms as i64),
        end_time_ticks: Some(wkmp_common::timing::ms_to_ticks(end_ms as i64)),
        lead_in_point_ticks: wkmp_common::timing::ms_to_ticks(start_ms as i64),
        lead_out_point_ticks: Some(wkmp_common::timing::ms_to_ticks(end_ms as i64)),
        fade_in_point_ticks: wkmp_common::timing::ms_to_ticks(start_ms as i64),
        fade_out_point_ticks: Some(wkmp_common::timing::ms_to_ticks(end_ms as i64)),
        fade_in_curve: FadeCurve::Linear,
        fade_out_curve: FadeCurve::Linear,
    }
}

/// Helper to create test dependencies for DecoderWorker
async fn create_test_deps_simple() -> (Arc<SharedState>, sqlx::Pool<sqlx::Sqlite>) {
    let shared_state = Arc::new(SharedState::new());
    let db_pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    (shared_state, db_pool)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_real_audio_decode_and_monitor_logs() {
    // Initialize tracing to capture log output
    let (log_tx, mut log_rx) = tokio::sync::mpsc::unbounded_channel();

    // Custom subscriber to capture logs
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    use tracing::Level;

    let filter = tracing_subscriber::filter::EnvFilter::from_default_env()
        .add_directive(Level::DEBUG.into());

    // Create a layer that sends logs to our channel
    struct LogCapture {
        tx: tokio::sync::mpsc::UnboundedSender<String>,
    }

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for LogCapture {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut visitor = LogVisitor::new();
            event.record(&mut visitor);
            if let Some(message) = visitor.message {
                let level = event.metadata().level();
                let log_line = format!("[{}] {}", level, message);
                let _ = self.tx.send(log_line);
            }
        }
    }

    struct LogVisitor {
        message: Option<String>,
    }

    impl LogVisitor {
        fn new() -> Self {
            Self { message: None }
        }
    }

    impl tracing::field::Visit for LogVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.message = Some(format!("{:?}", value));
            }
        }
    }

    let log_layer = LogCapture { tx: log_tx };

    tracing_subscriber::registry()
        .with(filter)
        .with(log_layer)
        .try_init()
        .ok(); // Ignore error if already initialized

    println!("=== Starting Real Audio Playback Test ===");
    println!("Test file: {}", TEST_AUDIO_FILE);

    // Check file exists
    assert!(
        std::path::Path::new(TEST_AUDIO_FILE).exists(),
        "Test audio file not found: {}",
        TEST_AUDIO_FILE
    );

    // Create buffer manager and decoder
    let buffer_manager = Arc::new(BufferManager::new());
    let (shared_state, db_pool) = create_test_deps_simple().await;
    let working_sample_rate = Arc::new(std::sync::RwLock::new(44100));
    let decoder = Arc::new(DecoderWorker::new(Arc::clone(&buffer_manager), shared_state, db_pool, working_sample_rate));

    // Submit first passage (0-30 seconds)
    let passage1_id = Uuid::new_v4();
    let passage1 = create_real_passage(0, 30000);

    println!("\n=== Enqueueing First Passage (0-30s) ===");
    decoder.submit(
        passage1_id,
        passage1,
        DecodePriority::Immediate,
        true,
    ).await.expect("First submission should succeed");

    // Wait for initial decode to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Submit second passage (30-60 seconds) to trigger yielding
    let passage2_id = Uuid::new_v4();
    let passage2 = create_real_passage(30000, 60000);

    println!("\n=== Enqueueing Second Passage (30-60s) ===");
    decoder.submit(
        passage2_id,
        passage2,
        DecodePriority::Next,
        true,
    ).await.expect("Second submission should succeed");

    // Let decoding run for a while
    println!("\n=== Allowing Decode to Run for 10 seconds ===");
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Check buffer states
    println!("\n=== Checking Buffer States ===");

    if let Some(buffer1) = buffer_manager.get_buffer(passage1_id).await {
        println!("Passage 1: {} samples occupied",
            buffer1.occupied()
        );
    }

    if let Some(buffer2) = buffer_manager.get_buffer(passage2_id).await {
        println!("Passage 2: {} samples occupied",
            buffer2.occupied()
        );
    }

    // Collect all log messages
    println!("\n=== Analyzing Log Output ===");
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut all_logs = Vec::new();

    while let Ok(log) = log_rx.try_recv() {
        all_logs.push(log.clone());
        if log.contains("[WARN]") {
            warnings.push(log.clone());
        } else if log.contains("[ERROR]") {
            errors.push(log.clone());
        }
    }

    // Print summary
    println!("\nTotal log messages: {}", all_logs.len());
    println!("Warnings: {}", warnings.len());
    println!("Errors: {}", errors.len());

    // Print all warnings
    if !warnings.is_empty() {
        println!("\n=== WARNINGS ===");
        for warn in &warnings {
            println!("{}", warn);
        }
    }

    // Print all errors
    if !errors.is_empty() {
        println!("\n=== ERRORS ===");
        for err in &errors {
            println!("{}", err);
        }
    }

    // Shutdown
    decoder.shutdown().await;

    println!("\n=== Test Complete ===");

    // Fail test if there were errors
    assert!(
        errors.is_empty(),
        "Found {} errors in log output (see above for details)",
        errors.len()
    );

    // Report warnings (don't fail, but make visible)
    if !warnings.is_empty() {
        println!("\n⚠️  WARNING: Found {} warnings (see above for details)", warnings.len());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_single_passage_decode() {
    println!("=== Testing Single Passage Decode ===");

    // Check file exists
    assert!(
        std::path::Path::new(TEST_AUDIO_FILE).exists(),
        "Test audio file not found: {}",
        TEST_AUDIO_FILE
    );

    let buffer_manager = Arc::new(BufferManager::new());
    let (shared_state, db_pool) = create_test_deps_simple().await;
    let working_sample_rate = Arc::new(std::sync::RwLock::new(44100));
    let decoder = Arc::new(DecoderWorker::new(Arc::clone(&buffer_manager), shared_state, db_pool, working_sample_rate));

    // Submit single passage (first 15 seconds)
    let passage_id = Uuid::new_v4();
    let passage = create_real_passage(0, 15000);

    decoder.submit(
        passage_id,
        passage,
        DecodePriority::Immediate,
        true,
    ).await.expect("Submission should succeed");

    // Wait for decode to complete
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Check buffer was filled
    if let Some(buffer) = buffer_manager.get_buffer(passage_id).await {
        let occupied = buffer.occupied();
        println!("Buffer filled with {} samples", occupied);

        assert!(
            occupied > 0,
            "Buffer should have been filled with audio data"
        );
    } else {
        panic!("Buffer not found for passage");
    }

    decoder.shutdown().await;
    println!("=== Single Passage Test Complete ===");
}
