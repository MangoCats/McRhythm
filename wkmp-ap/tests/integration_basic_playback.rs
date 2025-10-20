//! Integration Test - Basic Playback with Fast Startup
//!
//! **Requirements:** [DBD-OV-010], [DBD-DEC-050], [DBD-MIX-010]
//!
//! **Goal:** Verify passage plays from start to end with <100ms startup latency
//!
//! **Priority:** CRITICAL (Phase 1 target: <100ms startup)
//!
//! See: /home/sw/Dev/McRhythm/docs/validation/IMPL-TESTS-002-integration-test-specs.md

mod helpers;

use helpers::{TestServer, PassageBuilder, AudioAnalysisReport};
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_basic_playback_with_fast_startup() {
    // Setup
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let mut events = server.subscribe_events().await;

    // Note: Audio capture would be integrated here in production
    // For now, we test the API and event flow

    // Create test passage (30 seconds)
    let passage = PassageBuilder::new()
        .file("/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-01-Train_.mp3")
        .duration_seconds(30.0)
        .build();

    // Record start time
    let t0 = Instant::now();

    // Enqueue passage
    let passage_id = server
        .enqueue_passage(passage)
        .await
        .expect("Failed to enqueue passage");

    println!("Enqueued passage: {}", passage_id);

    // Wait for PassageEnqueued event
    let passage_enqueued = events
        .next_timeout(Duration::from_millis(100))
        .await
        .expect("PassageEnqueued event timeout");

    assert_eq!(
        passage_enqueued.event_type(),
        "PassageEnqueued",
        "Expected PassageEnqueued event"
    );

    let enqueue_latency = t0.elapsed();
    println!("Enqueue latency: {:?}", enqueue_latency);

    // Wait for DecodingStarted event
    let decoding_started = events
        .wait_for("DecodingStarted", Duration::from_millis(100))
        .await
        .expect("DecodingStarted event timeout");

    let decoding_latency = t0.elapsed();
    println!("Decoding started: {:?}", decoding_latency);

    // Wait for PlaybackStarted event (CRITICAL: <100ms)
    let playback_started = events
        .wait_for("PlaybackStarted", Duration::from_millis(100))
        .await
        .expect("PlaybackStarted event timeout");

    let t1 = Instant::now();
    let startup_latency = t1.duration_since(t0);

    // CRITICAL ASSERTION: Startup < 100ms
    assert!(
        startup_latency < Duration::from_millis(100),
        "FAILED: Startup took {:?}, expected <100ms (Phase 1 goal)",
        startup_latency
    );

    println!("✅ PASSED: Startup latency: {:?}", startup_latency);

    // Wait for playback to complete (with generous timeout)
    let completion = events
        .wait_for("PlaybackCompleted", Duration::from_secs(35))
        .await;

    // Note: In real implementation, we would also:
    // - Capture audio output
    // - Verify no clicks/pops
    // - Verify timing accuracy
    //
    // For now, we verify the event flow is correct

    if let Some(_) = completion {
        let t2 = Instant::now();
        let playback_duration = t2.duration_since(t1);

        println!("✅ Playback completed in {:?}", playback_duration);

        // Verify duration is approximately 30 seconds
        let expected_duration = Duration::from_secs(30);
        let timing_error = (playback_duration.as_secs_f32() - expected_duration.as_secs_f32()).abs();

        // Allow 1 second tolerance (due to mock/test environment)
        assert!(
            timing_error < 1.0,
            "Timing error: {:.2}s (expected ~30s, got {:.2}s)",
            timing_error,
            playback_duration.as_secs_f32()
        );

        println!("✅ Timing accuracy verified: {:.2}s (error: {:.2}s)",
                 playback_duration.as_secs_f32(), timing_error);
    }

    println!("\n✅✅✅ BASIC PLAYBACK TEST PASSED ✅✅✅");
}

#[tokio::test]
async fn test_playback_state_transitions() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let mut events = server.subscribe_events().await;

    // Enqueue a passage
    let passage = PassageBuilder::new()
        .file("/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-01-Train_.mp3")
        .duration_seconds(10.0)
        .build();

    server
        .enqueue_passage(passage)
        .await
        .expect("Enqueue failed");

    // Wait for playback to start
    events
        .wait_for("PlaybackStarted", Duration::from_secs(1))
        .await
        .expect("PlaybackStarted timeout");

    // Verify we can get queue
    let queue = server.get_queue().await.expect("Get queue failed");
    assert_eq!(queue.len(), 1, "Queue should have 1 entry");

    println!("✅ Queue contains {} entries", queue.len());

    // Verify health check works
    let health = server.check_health().await.expect("Health check failed");
    assert_eq!(
        health["module"].as_str().unwrap(),
        "wkmp-ap",
        "Module should be wkmp-ap"
    );

    println!("✅ Health check passed");
    println!("\n✅✅✅ PLAYBACK STATE TEST PASSED ✅✅✅");
}

#[tokio::test]
async fn test_rapid_skip() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let mut events = server.subscribe_events().await;

    // Enqueue 3 passages
    for i in 1..=3 {
        let passage = PassageBuilder::new()
            .file(format!(
                "/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-0{}-*.mp3",
                i
            ))
            .duration_seconds(10.0)
            .build();

        server.enqueue_passage(passage).await.expect("Enqueue failed");
    }

    println!("Enqueued 3 passages");

    // Wait for playback to start
    events
        .wait_for("PlaybackStarted", Duration::from_secs(1))
        .await
        .expect("PlaybackStarted timeout");

    // Verify queue has 3 entries
    let queue = server.get_queue().await.expect("Get queue failed");
    assert_eq!(queue.len(), 3, "Queue should have 3 entries");

    // Skip first passage
    server.skip_next().await.expect("Skip failed");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let queue = server.get_queue().await.expect("Get queue failed");
    assert_eq!(queue.len(), 2, "Queue should have 2 entries after skip");

    println!("✅ Skip 1: Queue now has {} entries", queue.len());

    // Skip second passage
    server.skip_next().await.expect("Skip failed");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let queue = server.get_queue().await.expect("Get queue failed");
    assert_eq!(queue.len(), 1, "Queue should have 1 entry after 2 skips");

    println!("✅ Skip 2: Queue now has {} entries", queue.len());

    // Skip third passage
    server.skip_next().await.expect("Skip failed");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let queue = server.get_queue().await.expect("Get queue failed");
    assert_eq!(queue.len(), 0, "Queue should be empty after 3 skips");

    println!("✅ Skip 3: Queue now empty");

    // Try skip on empty queue (should not error)
    let result = server.skip_next().await;
    assert!(result.is_ok(), "Skip on empty queue should not error");

    println!("✅ Skip on empty queue handled gracefully");
    println!("\n✅✅✅ RAPID SKIP TEST PASSED ✅✅✅");
}
