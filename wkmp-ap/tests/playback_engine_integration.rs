//! End-to-end integration tests for PlaybackEngine
//!
//! Tests the complete playback flow including:
//! - Queue management and passage advancement
//! - Crossfade triggering
//! - Skip functionality
//! - Seek functionality
//! - Event emission
//!
//! **Traceability:**
//! - [SSD-FLOW-010] Complete playback sequence
//! - [SSD-ENG-020] Queue processing
//! - [SSD-ENG-025] Skip functionality
//! - [SSD-ENG-026] Seek functionality

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use wkmp_ap::playback::engine::PlaybackEngine;
use wkmp_ap::state::{PlaybackState, SharedState};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};

/// Create in-memory test database with schema
async fn create_test_db() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    // Create minimal queue table schema
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
    .unwrap();

    pool
}

#[tokio::test]
async fn test_playback_engine_basic_flow() {
    // Setup
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());

    let engine = PlaybackEngine::new(db, state.clone())
        .await
        .expect("Failed to create engine");

    // Verify initial state (defaults to Playing on startup)
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    // Start engine (begins processing loop)
    engine.start().await.expect("Failed to start engine");

    // Enqueue test passages (using mock paths)
    let passage1 = PathBuf::from("/test/passage1.mp3");
    let passage2 = PathBuf::from("/test/passage2.mp3");
    let passage3 = PathBuf::from("/test/passage3.mp3");

    let id1 = engine.enqueue_file(passage1).await.expect("Enqueue 1 failed");
    let id2 = engine.enqueue_file(passage2).await.expect("Enqueue 2 failed");
    let id3 = engine.enqueue_file(passage3).await.expect("Enqueue 3 failed");

    println!("Enqueued 3 passages: {}, {}, {}", id1, id2, id3);

    // Verify queue length
    let queue_len = engine.queue_len().await;
    assert_eq!(queue_len, 3, "Queue should have 3 entries");

    // Test play/pause state control
    engine.play().await.expect("Play failed");
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    engine.pause().await.expect("Pause failed");
    assert_eq!(state.get_playback_state().await, PlaybackState::Paused);

    engine.play().await.expect("Resume play failed");
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    println!("✅ Basic playback state control works");
}

#[tokio::test]
async fn test_skip_functionality() {
    // Setup
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());

    let engine = PlaybackEngine::new(db, state.clone())
        .await
        .expect("Failed to create engine");

    engine.start().await.expect("Failed to start engine");

    // Enqueue 3 test passages
    let passage1 = PathBuf::from("/test/skip1.mp3");
    let passage2 = PathBuf::from("/test/skip2.mp3");
    let passage3 = PathBuf::from("/test/skip3.mp3");

    engine.enqueue_file(passage1).await.expect("Enqueue 1 failed");
    engine.enqueue_file(passage2).await.expect("Enqueue 2 failed");
    engine.enqueue_file(passage3).await.expect("Enqueue 3 failed");

    assert_eq!(engine.queue_len().await, 3);

    // Skip first passage
    engine.skip_next().await.expect("Skip 1 failed");
    assert_eq!(engine.queue_len().await, 2, "Queue should have 2 after skip");

    // Skip second passage
    engine.skip_next().await.expect("Skip 2 failed");
    assert_eq!(engine.queue_len().await, 1, "Queue should have 1 after 2 skips");

    // Skip third passage
    engine.skip_next().await.expect("Skip 3 failed");
    assert_eq!(engine.queue_len().await, 0, "Queue should be empty after 3 skips");

    // Try to skip when queue is empty (should not error)
    engine.skip_next().await.expect("Skip on empty should not error");
    assert_eq!(engine.queue_len().await, 0, "Queue should still be empty");

    println!("✅ Skip functionality works correctly");
}

#[tokio::test]
async fn test_seek_no_passage_error() {
    // Setup
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());

    let engine = PlaybackEngine::new(db, state.clone())
        .await
        .expect("Failed to create engine");

    engine.start().await.expect("Failed to start engine");

    // Try to seek when no passage is playing
    let result = engine.seek(5000).await;

    assert!(result.is_err(), "Seek should fail when no passage playing");

    if let Err(e) = result {
        let err_msg = format!("{:?}", e);
        assert!(err_msg.contains("no passage playing"), "Error should mention no passage");
    }

    println!("✅ Seek correctly returns error when no passage playing");
}

#[tokio::test]
async fn test_queue_operations() {
    // Setup
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());

    let engine = PlaybackEngine::new(db, state.clone())
        .await
        .expect("Failed to create engine");

    engine.start().await.expect("Failed to start engine");

    // Test empty queue
    assert_eq!(engine.queue_len().await, 0);

    // Enqueue passages
    let passage1 = PathBuf::from("/test/queue1.mp3");
    let passage2 = PathBuf::from("/test/queue2.mp3");

    let id1 = engine.enqueue_file(passage1).await.expect("Enqueue 1 failed");
    let id2 = engine.enqueue_file(passage2).await.expect("Enqueue 2 failed");

    assert_eq!(engine.queue_len().await, 2);
    assert_ne!(id1, id2, "Queue entry IDs should be unique");

    // Clear queue via skips
    engine.skip_next().await.expect("Skip failed");
    engine.skip_next().await.expect("Skip failed");

    assert_eq!(engine.queue_len().await, 0);

    // Re-enqueue to test queue rebuilding
    engine.enqueue_file(PathBuf::from("/test/queue3.mp3")).await.expect("Re-enqueue failed");
    assert_eq!(engine.queue_len().await, 1);

    println!("✅ Queue operations work correctly");
}

#[tokio::test]
async fn test_multiple_engines_isolated() {
    // Test that multiple engine instances don't interfere with each other
    let db1 = create_test_db().await;
    let db2 = create_test_db().await;
    let state1 = Arc::new(SharedState::new());
    let state2 = Arc::new(SharedState::new());

    let engine1 = PlaybackEngine::new(db1, state1.clone())
        .await
        .expect("Failed to create engine1");
    let engine2 = PlaybackEngine::new(db2, state2.clone())
        .await
        .expect("Failed to create engine2");

    engine1.start().await.expect("Failed to start engine1");
    engine2.start().await.expect("Failed to start engine2");

    // Enqueue different passages to each engine
    engine1.enqueue_file(PathBuf::from("/test/engine1.mp3")).await.expect("Enqueue to engine1 failed");
    engine2.enqueue_file(PathBuf::from("/test/engine2_a.mp3")).await.expect("Enqueue to engine2 failed");
    engine2.enqueue_file(PathBuf::from("/test/engine2_b.mp3")).await.expect("Enqueue to engine2 failed");

    // Verify isolation
    assert_eq!(engine1.queue_len().await, 1, "Engine 1 should have 1 passage");
    assert_eq!(engine2.queue_len().await, 2, "Engine 2 should have 2 passages");

    // Pause engine2 to differentiate states
    engine2.pause().await.expect("Engine2 pause failed");
    assert_eq!(state2.get_playback_state().await, PlaybackState::Paused);

    // Modify engine1 state - engine2 should remain paused (independent)
    engine1.play().await.expect("Engine1 play failed");
    assert_eq!(state1.get_playback_state().await, PlaybackState::Playing);
    assert_eq!(state2.get_playback_state().await, PlaybackState::Paused, "Engine2 should remain paused");

    // Now modify engine2 state independently
    engine2.play().await.expect("Engine2 play failed");
    assert_eq!(state2.get_playback_state().await, PlaybackState::Playing);

    println!("✅ Multiple engine instances are properly isolated");
}

#[tokio::test]
async fn test_rapid_skip_operations() {
    // Test rapid consecutive skips to ensure queue state consistency
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());

    let engine = PlaybackEngine::new(db, state.clone())
        .await
        .expect("Failed to create engine");

    engine.start().await.expect("Failed to start engine");

    // Enqueue 10 passages
    for i in 0..10 {
        let path = PathBuf::from(format!("/test/rapid{}.mp3", i));
        engine.enqueue_file(path).await.expect("Enqueue failed");
    }

    assert_eq!(engine.queue_len().await, 10);

    // Rapidly skip all passages
    for expected_remaining in (0..10).rev() {
        engine.skip_next().await.expect("Rapid skip failed");
        assert_eq!(
            engine.queue_len().await,
            expected_remaining,
            "Queue length mismatch after skip"
        );
    }

    assert_eq!(engine.queue_len().await, 0, "Queue should be empty after rapid skips");

    println!("✅ Rapid skip operations maintain queue consistency");
}

#[tokio::test]
async fn test_play_pause_transitions() {
    // Test various play/pause state transitions
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());

    let engine = PlaybackEngine::new(db, state.clone())
        .await
        .expect("Failed to create engine");

    engine.start().await.expect("Failed to start engine");
    engine.enqueue_file(PathBuf::from("/test/transition.mp3")).await.expect("Enqueue failed");

    // Test: Initial state is Playing
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);
    engine.play().await.expect("Play failed");
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    // Test: Playing -> Paused
    engine.pause().await.expect("Pause failed");
    assert_eq!(state.get_playback_state().await, PlaybackState::Paused);

    // Test: Paused -> Playing
    engine.play().await.expect("Resume failed");
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    // Test: Playing -> Paused -> Playing (rapid toggle)
    engine.pause().await.expect("Pause failed");
    engine.play().await.expect("Resume failed");
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    // Test: Multiple play commands (idempotent)
    engine.play().await.expect("Redundant play 1 failed");
    engine.play().await.expect("Redundant play 2 failed");
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    // Test: Multiple pause commands (idempotent)
    engine.pause().await.expect("Pause failed");
    engine.pause().await.expect("Redundant pause 1 failed");
    engine.pause().await.expect("Redundant pause 2 failed");
    assert_eq!(state.get_playback_state().await, PlaybackState::Paused);

    println!("✅ Play/pause state transitions work correctly");
}

#[tokio::test]
async fn test_engine_lifecycle() {
    // Test engine creation, start, and operation
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());

    // Create engine
    let engine = PlaybackEngine::new(db, state.clone())
        .await
        .expect("Failed to create engine");

    // Verify initial state (defaults to Playing on startup)
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    // Start engine
    engine.start().await.expect("Failed to start engine");

    // Engine state remains Playing (start() just starts background loops)
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    // Enqueue and play
    engine.enqueue_file(PathBuf::from("/test/lifecycle.mp3")).await.expect("Enqueue failed");
    engine.play().await.expect("Play failed");
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    // Engine continues to operate
    sleep(Duration::from_millis(50)).await;
    assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

    println!("✅ Engine lifecycle operates correctly");
}
