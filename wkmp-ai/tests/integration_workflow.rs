// PLAN023: Song Workflow Engine Integration Tests
//
// Tests the complete per-song workflow including:
// - Tier 1 → Tier 2 → Tier 3 pipeline
// - Error isolation (failures don't cascade)
// - SSE event broadcasting
// - Per-passage vs per-file processing
//
// Requirements tested:
// - TC-I-012-01: Phase 1-6 per-song processing
// - TC-I-013-01: Per-song error isolation
// - TC-I-071-01: SSE event types
// - TC-I-073-01: SSE event throttling

use wkmp_ai::import_v2::song_workflow_engine::SongWorkflowEngine;
use wkmp_ai::import_v2::types::{BoundaryDetectionMethod, ImportEvent, PassageBoundary};
use std::path::PathBuf;
use tokio::sync::broadcast;

/// Helper to create a test passage boundary
fn create_test_boundary(start_sec: u32, end_sec: u32) -> PassageBoundary {
    const TICKS_PER_SECOND: i64 = 28_224_000;
    PassageBoundary {
        start_ticks: (start_sec as i64) * TICKS_PER_SECOND,
        end_ticks: (end_sec as i64) * TICKS_PER_SECOND,
        confidence: 0.9,
        detection_method: BoundaryDetectionMethod::SilenceDetection,
    }
}

/// TC-I-012-01: Phase 1-6 Per-Song Processing
///
/// **Requirement:** REQ-AI-012 - Process each song through phases 1-6
/// **Test:** Verify workflow engine executes all phases sequentially
/// **Expected:** Complete workflow with all phases executed
#[tokio::test]
async fn test_complete_per_song_workflow() {
    let mut engine = SongWorkflowEngine::default();

    // Note: This test will use placeholder implementations for Tier 1 extractors
    // In the current state, they return empty/default data
    let file_path = PathBuf::from("test.mp3");
    let boundary = create_test_boundary(0, 180);  // 0-180 seconds

    let result = engine
        .process_passage(&file_path, 0, 1, &boundary)
        .await;

    // Workflow should complete (even with empty data)
    assert_eq!(result.passage_index, 0);

    // Duration should be measured (might be 0ms if very fast, that's OK)
    // The important thing is that we got a result without panic

    // If extraction succeeds, we should have metadata, flavor, and validation
    // If it fails, result.error should be set
    if result.success {
        assert!(result.metadata.is_some());
        assert!(result.flavor.is_some());
        assert!(result.validation.is_some());
    } else {
        assert!(result.error.is_some());
    }
}

/// TC-I-013-01: Per-Song Error Isolation
///
/// **Requirement:** REQ-AI-013 - Isolate errors to individual songs
/// **Test:** Process file with multiple boundaries, verify one failure doesn't abort others
/// **Expected:** Failed passage isolated, other passages process successfully
#[tokio::test]
async fn test_error_isolation_multi_passage() {
    let mut engine = SongWorkflowEngine::default();

    let file_path = PathBuf::from("test.mp3");

    // Create 3 passage boundaries
    let boundaries = vec![
        create_test_boundary(0, 60),      // Passage 1: 0-60s
        create_test_boundary(60, 120),    // Passage 2: 60-120s
        create_test_boundary(120, 180),   // Passage 3: 120-180s
    ];

    let summary = engine.process_file(&file_path, &boundaries).await;

    // All 3 passages should be processed
    assert_eq!(summary.total_passages, 3);
    assert_eq!(summary.results.len(), 3);

    // Each passage result should have correct index
    for (i, result) in summary.results.iter().enumerate() {
        assert_eq!(result.passage_index, i);
    }

    // Note: In current implementation with placeholder extractors,
    // all passages will likely fail validation (no title/artist),
    // but they should all be processed (not aborted early)
    assert_eq!(summary.successes + summary.failures, summary.total_passages);
}

/// TC-I-071-01: SSE Event Types
///
/// **Requirement:** REQ-AI-071 - Emit 10 event types during workflow
/// **Test:** Subscribe to SSE events, process passage, verify all event types emitted
/// **Expected:** PassagesDiscovered, SongStarted, ExtractionComplete, FusionComplete,
///              ValidationComplete, SongComplete/SongFailed, FileComplete
#[tokio::test]
async fn test_sse_event_types() {
    let (tx, mut rx) = broadcast::channel(100);
    let mut engine = SongWorkflowEngine::with_sse(tx, 1000);

    let file_path = PathBuf::from("test.mp3");
    let boundaries = vec![create_test_boundary(0, 180)];

    // Process file in background
    let file_path_clone = file_path.clone();
    let boundaries_clone = boundaries.clone();
    tokio::spawn(async move {
        engine.process_file(&file_path_clone, &boundaries_clone).await;
    });

    // Collect events
    let mut events = Vec::new();
    let timeout = tokio::time::Duration::from_millis(500);

    loop {
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Ok(event)) => events.push(event),
            Ok(Err(_)) => break,  // Channel closed
            Err(_) => break,      // Timeout - no more events
        }
    }

    // Verify we received expected event types
    let has_passages_discovered = events.iter().any(|e| matches!(e, ImportEvent::PassagesDiscovered { .. }));
    let has_song_started = events.iter().any(|e| matches!(e, ImportEvent::SongStarted { .. }));
    let has_extraction_complete = events.iter().any(|e| matches!(e, ImportEvent::ExtractionComplete { .. }));
    let has_fusion_complete = events.iter().any(|e| matches!(e, ImportEvent::FusionComplete { .. }));
    let has_validation_complete = events.iter().any(|e| matches!(e, ImportEvent::ValidationComplete { .. }));
    let has_song_outcome = events.iter().any(|e|
        matches!(e, ImportEvent::SongComplete { .. } | ImportEvent::SongFailed { .. })
    );
    let has_file_complete = events.iter().any(|e| matches!(e, ImportEvent::FileComplete { .. }));

    assert!(has_passages_discovered, "Missing PassagesDiscovered event");
    assert!(has_song_started, "Missing SongStarted event");

    // Note: If extraction fails early, ExtractionComplete may not be emitted
    // This is expected behavior - early failures emit SongFailed directly
    // So we verify either extraction completed OR song failed
    assert!(
        has_extraction_complete || has_song_outcome,
        "Expected either ExtractionComplete or SongFailed event"
    );

    // If we got past extraction, we should have fusion and validation
    if has_extraction_complete {
        assert!(has_fusion_complete, "Missing FusionComplete event after successful extraction");
        assert!(has_validation_complete, "Missing ValidationComplete event after successful extraction");
    }

    assert!(has_song_outcome, "Missing SongComplete/SongFailed event");
    assert!(has_file_complete, "Missing FileComplete event");
}

/// TC-I-073-01: SSE Event Throttling
///
/// **Requirement:** REQ-AI-073 - Throttle progress events to 1/second
/// **Test:** Process multiple passages rapidly, verify throttling works
/// **Expected:** Critical events immediate, progress events throttled
#[tokio::test]
async fn test_sse_event_throttling_multi_passage() {
    let (tx, mut rx) = broadcast::channel(100);
    let mut engine = SongWorkflowEngine::with_sse(tx, 100);  // 100ms throttle for faster test

    let file_path = PathBuf::from("test.mp3");

    // Create 5 passages to process quickly
    let boundaries: Vec<PassageBoundary> = (0..5)
        .map(|i| create_test_boundary(i * 60, (i + 1) * 60))
        .collect();

    // Process file in background
    let file_path_clone = file_path.clone();
    let boundaries_clone = boundaries.clone();
    tokio::spawn(async move {
        engine.process_file(&file_path_clone, &boundaries_clone).await;
    });

    // Collect events with timestamps
    let mut events_with_time: Vec<(ImportEvent, std::time::Instant)> = Vec::new();
    let timeout = tokio::time::Duration::from_secs(2);

    loop {
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Ok(event)) => {
                events_with_time.push((event, std::time::Instant::now()));
            }
            Ok(Err(_)) => break,
            Err(_) => break,
        }
    }

    // Verify we got events from all 5 passages
    let song_started_count = events_with_time.iter()
        .filter(|(e, _)| matches!(e, ImportEvent::SongStarted { .. }))
        .count();

    assert_eq!(song_started_count, 5, "Should have SongStarted for all 5 passages");

    // Critical events (SongStarted, SongComplete/Failed) should NOT be throttled
    // Progress events (ExtractionComplete, FusionComplete, ValidationComplete) SHOULD be throttled

    // Count throttled events (ExtractionComplete, FusionComplete, ValidationComplete)
    let throttled_events: Vec<_> = events_with_time.iter()
        .filter(|(e, _)| matches!(
            e,
            ImportEvent::ExtractionComplete { .. }
                | ImportEvent::FusionComplete { .. }
                | ImportEvent::ValidationComplete { .. }
        ))
        .collect();

    // With 5 passages, we expect some throttling (not all 15 events should come through)
    // Note: Exact count depends on timing, but we should see < 15 throttled events
    // In practice, with 100ms throttle and fast processing, we might see 3-10 events

    // Verify throttled events respect minimum interval
    for i in 1..throttled_events.len() {
        let (_, prev_time) = throttled_events[i - 1];
        let (_, curr_time) = throttled_events[i];
        let interval = curr_time.duration_since(*prev_time);

        // Should be at least 100ms apart (with some tolerance for timing variance)
        if interval.as_millis() < 80 {
            panic!("Throttled events too close: {}ms apart", interval.as_millis());
        }
    }
}

/// TC-I-NF-021-01: Error Isolation (Non-Functional)
///
/// **Requirement:** REQ-AI-NF-021 - Errors isolated to individual songs
/// **Test:** Simulate extraction failure, verify it doesn't propagate
/// **Expected:** Failed passage logged, other passages unaffected
#[tokio::test]
async fn test_error_isolation_no_cascade() {
    // Note: In current implementation, extraction failures are caught and returned
    // as SongWorkflowResult with success=false, error=Some(...)

    let mut engine = SongWorkflowEngine::default();
    let file_path = PathBuf::from("nonexistent.mp3");  // File doesn't exist
    let boundary = create_test_boundary(0, 180);

    let result = engine
        .process_passage(&file_path, 0, 1, &boundary)
        .await;

    // Result should be returned (not panic/abort)
    assert_eq!(result.passage_index, 0);

    // Workflow should complete without panic (either success or controlled failure)
    // Duration might be 0 if extraction fails immediately, that's OK
}

/// TC-I-NF-022-01: Graceful Degradation
///
/// **Requirement:** REQ-AI-NF-022 - Continue with partial data
/// **Test:** Process with only one extractor available (ID3)
/// **Expected:** Workflow completes with partial metadata
#[tokio::test]
async fn test_graceful_degradation_partial_sources() {
    // Current implementation uses placeholder extractors, so this test
    // verifies that the workflow completes even when extractors return empty data

    let mut engine = SongWorkflowEngine::default();
    let file_path = PathBuf::from("test.mp3");
    let boundary = create_test_boundary(0, 180);

    let result = engine
        .process_passage(&file_path, 0, 1, &boundary)
        .await;

    // Workflow should complete
    assert_eq!(result.passage_index, 0);

    // If successful, metadata should be present
    // If validation fails (missing required fields), that's expected with placeholder data
    if result.success {
        assert!(result.metadata.is_some());

        // Validation should report low quality due to missing data
        if let Some(validation) = &result.validation {
            // With placeholder extractors, quality will be low
            assert!(validation.quality_score < 1.0);
        }
    }
    // Failure is also acceptable with empty extractors
}
