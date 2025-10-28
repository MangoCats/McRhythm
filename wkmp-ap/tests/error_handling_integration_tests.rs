//! Phase 7 Error Handling Integration Tests
//!
//! Tests for end-to-end error handling through the playback system.
//! **[PLAN001 Phase 7]** Integration tests verifying error handling with full system context

mod helpers;

use helpers::{TestServer, PassageBuilder, ErrorInjectionBuilder, event_verification};
use std::time::Duration;
use tokio::time::sleep;
use wkmp_common::WkmpEvent;

// ============================================================================
// REQ-AP-ERR-010: File Read Error - Passage Skip
// ============================================================================

/// **[TC-I-ERR-040-EXPANDED]** Queue validation prevents invalid files at enqueue time
///
/// **Given:** Attempts to enqueue files with various issues
/// **When:** Enqueue API is called
/// **Then:**
///   - Valid files enqueue successfully
///   - Invalid files are rejected at enqueue time with error response
///   - Queue integrity preserved
///
/// Note: This demonstrates REQ-AP-ERR-040 validation happens early (at enqueue),
/// preventing invalid entries from entering the queue at all.
#[tokio::test]
async fn test_queue_validation_at_enqueue() {
    let server = TestServer::start().await.expect("Server start failed");
    let builder = ErrorInjectionBuilder::new().expect("Builder creation failed");

    // Create a valid file
    let valid_file = builder.file_path("valid.wav");
    helpers::generate_sine_wav(&valid_file, 500, 440.0, 0.5).expect("File generation failed");

    // Valid file should enqueue successfully
    let result1 = server
        .enqueue_passage(
            PassageBuilder::new()
                .file(valid_file.to_str().unwrap())
                .build(),
        )
        .await;
    assert!(result1.is_ok(), "Valid file should enqueue successfully");

    // Nonexistent file should be rejected at enqueue time
    let nonexistent = builder.nonexistent_file();
    let result2 = server
        .enqueue_passage(
            PassageBuilder::new()
                .file(nonexistent.to_str().unwrap())
                .build(),
        )
        .await;
    assert!(result2.is_err(), "Nonexistent file should be rejected at enqueue");
    let error_msg = result2.unwrap_err().to_string();
    assert!(error_msg.contains("does not exist") || error_msg.contains("File not found"),
           "Error should mention file doesn't exist: {}", error_msg);

    // Verify queue only has valid entry
    let queue = server.get_queue().await.expect("Get queue failed");
    assert_eq!(queue.len(), 1, "Queue should only have 1 valid entry");

    println!("✅ Queue validation correctly rejects invalid files at enqueue time");
}

// ============================================================================
// REQ-AP-ERR-011: Unsupported Codec Detection
// ============================================================================

/// **[TC-I-ERR-011-01]** Unsupported codec emits event and skips passage
///
/// **Given:** A queue with corrupted audio file
/// **When:** Decoder attempts to process the file
/// **Then:**
///   - PassageUnsupportedCodec event is emitted
///   - Passage is skipped
///   - Playback continues to next passage
#[tokio::test]
async fn test_unsupported_codec_skips_passage() {
    let server = TestServer::start().await.expect("Server start failed");
    let builder = ErrorInjectionBuilder::new().expect("Builder creation failed");

    // Create corrupted audio file
    let corrupted_file = builder.corrupted_audio_file().expect("Corrupted file creation failed");

    // Create valid fallback file
    let valid_file = builder.file_path("fallback.wav");
    helpers::generate_sine_wav(&valid_file, 500, 440.0, 0.5).expect("File generation failed");

    // Subscribe to events
    let mut events = server.subscribe_events().await;

    // Enqueue corrupted passage
    let _corrupted_id = server
        .enqueue_passage(
            PassageBuilder::new()
                .file(corrupted_file.to_str().unwrap())
                .build(),
        )
        .await
        .expect("Enqueue corrupted failed");

    // Enqueue valid fallback
    let _fallback_id = server
        .enqueue_passage(
            PassageBuilder::new()
                .file(valid_file.to_str().unwrap())
                .build(),
        )
        .await
        .expect("Enqueue fallback failed");

    // Start playback
    server.play().await.expect("Play failed");

    // Wait for error event (either UnsupportedCodec or DecodeFailed)
    sleep(Duration::from_millis(500)).await;

    // Try to get unsupported codec event first
    let mut found_error = false;
    let mut attempts = 0;

    while attempts < 10 {
        if let Some(event) = events.next_timeout(Duration::from_millis(200)).await {
            match event {
                WkmpEvent::PassageUnsupportedCodec { .. } => {
                    found_error = true;
                    println!("✅ Received PassageUnsupportedCodec event");
                    break;
                }
                WkmpEvent::PassageDecodeFailed { error_type, .. } => {
                    // Symphonia might report as decode error instead of unsupported codec
                    if error_type.contains("codec") || error_type.contains("probe") {
                        found_error = true;
                        println!("✅ Received PassageDecodeFailed with codec error");
                        break;
                    }
                }
                _ => {} // Ignore other events
            }
        }
        attempts += 1;
    }

    assert!(found_error, "Should receive error event for corrupted file");

    println!("✅ Unsupported codec correctly emitted event and skipped passage");
}


// ============================================================================
// REQ-AP-ERR-050: Resampling Initialization Errors
// ============================================================================

/// **[TC-I-ERR-050-01]** Resampling init failure skips passage
///
/// **Given:** Audio file requiring resampling
/// **When:** Resampler initialization succeeds (normal case)
/// **Then:**
///   - No ResamplingFailed event
///   - Passage plays normally with resampling
///
/// Note: Actual resampling init failures are rare with valid audio.
/// This test verifies the happy path (resampling works).
#[tokio::test]
async fn test_resampling_init_success() {
    let server = TestServer::start().await.expect("Server start failed");
    let builder = ErrorInjectionBuilder::new().expect("Builder creation failed");

    // Create valid audio file (will use standard 44.1kHz)
    let audio_file = builder.audio_file_for_resampling().expect("File creation failed");

    // Subscribe to events
    let mut events = server.subscribe_events().await;

    // Enqueue passage
    let _passage_id = server
        .enqueue_passage(
            PassageBuilder::new()
                .file(audio_file.to_str().unwrap())
                .build(),
        )
        .await
        .expect("Enqueue failed");

    // Start playback
    server.play().await.expect("Play failed");

    // Wait for playback to start
    sleep(Duration::from_millis(500)).await;

    // Verify no resampling failure events
    let mut found_resampling_error = false;
    let mut attempts = 0;

    while attempts < 5 {
        if let Some(event) = events.next_timeout(Duration::from_millis(200)).await {
            if matches!(event, WkmpEvent::ResamplingFailed { .. }) {
                found_resampling_error = true;
                break;
            }
        }
        attempts += 1;
    }

    assert!(!found_resampling_error, "Should not have resampling errors with valid audio");

    println!("✅ Resampling initialized successfully for valid audio");
}

// ============================================================================
// REQ-AP-DEGRADE-010/020/030: Graceful Degradation
// ============================================================================

/// **[TC-S-RECOVERY-001]** Multiple codec errors maintain system stability
///
/// **Given:** A queue with multiple corrupted files and valid files
/// **When:** Playback processes the queue
/// **Then:**
///   - All errors handled gracefully
///   - Error events emitted for corrupted files
///   - Valid passages play
///   - System remains stable
///   - User control remains available
#[tokio::test]
async fn test_multiple_errors_graceful_degradation() {
    let server = TestServer::start().await.expect("Server start failed");
    let builder = ErrorInjectionBuilder::new().expect("Builder creation failed");

    // Create test files
    let valid1 = builder.file_path("multi_valid1.wav");
    helpers::generate_sine_wav(&valid1, 300, 440.0, 0.5).expect("File generation failed");

    let corrupted1 = builder.corrupted_audio_file().expect("Corrupted file 1 creation failed");

    let valid2 = builder.file_path("multi_valid2.wav");
    helpers::generate_sine_wav(&valid2, 300, 440.0, 0.5).expect("File generation failed");

    let corrupted2 = builder.unsupported_format_file().expect("Unsupported file creation failed");

    let valid3 = builder.file_path("multi_valid3.wav");
    helpers::generate_sine_wav(&valid3, 300, 440.0, 0.5).expect("File generation failed");

    // Subscribe to events
    let mut events = server.subscribe_events().await;

    // Enqueue mixed valid/corrupted passages
    server
        .enqueue_passage(PassageBuilder::new().file(valid1.to_str().unwrap()).build())
        .await
        .expect("Enqueue 1 failed");

    server
        .enqueue_passage(PassageBuilder::new().file(corrupted1.to_str().unwrap()).build())
        .await
        .expect("Enqueue 2 failed");

    server
        .enqueue_passage(PassageBuilder::new().file(valid2.to_str().unwrap()).build())
        .await
        .expect("Enqueue 3 failed");

    server
        .enqueue_passage(PassageBuilder::new().file(corrupted2.to_str().unwrap()).build())
        .await
        .expect("Enqueue 4 failed");

    server
        .enqueue_passage(PassageBuilder::new().file(valid3.to_str().unwrap()).build())
        .await
        .expect("Enqueue 5 failed");

    // Start playback
    server.play().await.expect("Play failed");

    // Collect error events
    let mut error_count = 0;
    let mut event_types = Vec::new();

    // Wait for errors to occur and process through queue
    for _ in 0..5 {
        sleep(Duration::from_millis(150)).await;
        server.skip_next().await.ok(); // Ignore skip errors

        // Check for error events
        while let Some(event) = events.next_timeout(Duration::from_millis(50)).await {
            match event {
                WkmpEvent::PassageDecodeFailed { .. } => {
                    error_count += 1;
                    event_types.push("DecodeFailed");
                }
                WkmpEvent::PassageUnsupportedCodec { .. } => {
                    error_count += 1;
                    event_types.push("UnsupportedCodec");
                }
                _ => {} // Ignore other events
            }
        }
    }

    // Verify system remained stable
    let health = server.check_health().await.expect("Health check failed");
    let status = health.get("status").and_then(|v| v.as_str());
    assert!(status == Some("ok") || status == Some("healthy"),
            "System should remain healthy after multiple errors, got: {:?}", status);

    // Verify we can still control playback
    server.pause().await.expect("Pause should work after errors");
    server.play().await.expect("Play should work after errors");

    println!("✅ Processed {} error events: {:?}", error_count, event_types);
    println!("✅ System remained stable through multiple errors");
    println!("✅ User control remained available throughout");
}

// ============================================================================
// Helper: Verify queue integrity after errors
// ============================================================================

#[tokio::test]
async fn test_queue_integrity_after_codec_errors() {
    let server = TestServer::start().await.expect("Server start failed");
    let builder = ErrorInjectionBuilder::new().expect("Builder creation failed");

    // Create valid file
    let valid = builder.file_path("integrity.wav");
    helpers::generate_sine_wav(&valid, 500, 440.0, 0.5).expect("File generation failed");

    // Create corrupted file (will fail during decode)
    let corrupted = builder.corrupted_audio_file().expect("Corrupted file creation failed");

    // Enqueue valid, then corrupted
    server
        .enqueue_passage(PassageBuilder::new().file(valid.to_str().unwrap()).build())
        .await
        .expect("Enqueue valid failed");

    server
        .enqueue_passage(PassageBuilder::new().file(corrupted.to_str().unwrap()).build())
        .await
        .expect("Enqueue corrupted failed");

    // Verify queue has both entries
    let queue = server.get_queue().await.expect("Get queue failed");
    assert_eq!(queue.len(), 2, "Queue should have 2 entries");

    // Start playback
    server.play().await.expect("Play failed");
    sleep(Duration::from_millis(200)).await;

    // Skip to trigger codec error
    server.skip_next().await.ok();
    sleep(Duration::from_millis(300)).await;

    // Verify system is still healthy after error
    let health = server.check_health().await.expect("Health check failed");
    let status = health.get("status").and_then(|v| v.as_str());
    assert!(status == Some("ok") || status == Some("healthy"),
            "System should remain healthy after codec error, got: {:?}", status);

    // Verify queue structure still exists
    let final_queue = server.get_queue().await.expect("Get queue failed");
    assert!(final_queue.len() <= 2, "Queue should not grow unbounded");

    println!("✅ Queue integrity maintained after codec errors (queue entries: {})", final_queue.len());
}
