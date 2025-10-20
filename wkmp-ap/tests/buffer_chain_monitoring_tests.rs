//! Integration tests for buffer chain monitoring
//!
//! **[DBD-OV-010]** through **[DBD-OV-080]** Buffer chain visibility
//! **[DBD-PARAM-050]** maximum_decode_streams parameter
//!
//! Tests verify:
//! - 12-chain response for varying queue sizes
//! - Passage-based chain association
//! - Queue position tracking
//! - Idle chain filling

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use wkmp_ap::playback::engine::PlaybackEngine;
use wkmp_ap::state::SharedState;

/// Create test database with required schema
async fn create_test_db() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    // Create minimal schema for integration tests
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

    // Create settings table with maximum_decode_streams
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
    .unwrap();

    // Set maximum_decode_streams to 12 (default)
    sqlx::query("INSERT INTO settings (key, value) VALUES ('maximum_decode_streams', '12')")
        .execute(&pool)
        .await
        .unwrap();

    pool
}

/// **[DBD-OV-080]** Test buffer chain monitoring with 1 passage queue
#[tokio::test]
async fn test_buffer_chains_single_passage() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Create temporary file
    let temp_dir = std::env::temp_dir();
    let passage = temp_dir.join("test_chain_single.mp3");
    std::fs::write(&passage, b"").unwrap();

    // Enqueue 1 passage
    engine.enqueue_file(passage.clone()).await.unwrap();

    // Wait briefly for queue processing
    sleep(Duration::from_millis(50)).await;

    // Get buffer chains
    let chains = engine.get_buffer_chains().await;

    // Should always return 12 chains
    assert_eq!(chains.len(), 12, "Should return exactly 12 chains");

    // Chain 0 (position 1) should be active
    assert!(
        chains[0].queue_entry_id.is_some(),
        "Chain 0 should have queue_entry_id"
    );
    assert_eq!(
        chains[0].queue_position,
        Some(1),
        "Chain 0 should have queue_position 1"
    );

    // Chains 1-11 should be idle
    for i in 1..12 {
        assert_eq!(
            chains[i].queue_entry_id,
            None,
            "Chain {} should be idle",
            i
        );
        assert_eq!(
            chains[i].queue_position,
            None,
            "Chain {} should have no queue_position",
            i
        );
        assert_eq!(
            chains[i].buffer_state,
            Some("Idle".to_string()),
            "Chain {} should have Idle state",
            i
        );
    }

    // Clean up
    std::fs::remove_file(&passage).unwrap();
}

/// **[DBD-OV-060]** **[DBD-OV-070]** Test buffer chain monitoring with 2 passage queue
#[tokio::test]
async fn test_buffer_chains_two_passages() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let passage1 = temp_dir.join("test_chain_two_1.mp3");
    let passage2 = temp_dir.join("test_chain_two_2.mp3");
    std::fs::write(&passage1, b"").unwrap();
    std::fs::write(&passage2, b"").unwrap();

    // Enqueue 2 passages
    engine.enqueue_file(passage1.clone()).await.unwrap();
    engine.enqueue_file(passage2.clone()).await.unwrap();

    sleep(Duration::from_millis(50)).await;

    // Get buffer chains
    let chains = engine.get_buffer_chains().await;

    // Should always return 12 chains
    assert_eq!(chains.len(), 12);

    // Chains 0-1 (positions 1-2) should be active
    assert_eq!(
        chains[0].queue_position,
        Some(1),
        "Chain 0 should be position 1 (now playing)"
    );
    assert_eq!(
        chains[1].queue_position,
        Some(2),
        "Chain 1 should be position 2 (playing next)"
    );

    // Chains 2-11 should be idle
    for i in 2..12 {
        assert_eq!(
            chains[i].queue_entry_id,
            None,
            "Chain {} should be idle",
            i
        );
        assert_eq!(
            chains[i].buffer_state,
            Some("Idle".to_string()),
            "Chain {} should have Idle state",
            i
        );
    }

    // Clean up
    std::fs::remove_file(&passage1).unwrap();
    std::fs::remove_file(&passage2).unwrap();
}

/// **[DBD-PARAM-050]** Test buffer chain monitoring with full 12 passage queue
#[tokio::test]
async fn test_buffer_chains_full_queue() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let mut passages = Vec::new();
    for i in 0..12 {
        let passage = temp_dir.join(format!("test_chain_full_{}.mp3", i));
        std::fs::write(&passage, b"").unwrap();
        passages.push(passage);
    }

    // Enqueue 12 passages (fills all chains)
    for passage in &passages {
        engine.enqueue_file(passage.clone()).await.unwrap();
    }

    sleep(Duration::from_millis(50)).await;

    // Get buffer chains
    let chains = engine.get_buffer_chains().await;

    // Should return exactly 12 chains
    assert_eq!(chains.len(), 12);

    // All 12 chains should be active
    for i in 0..12 {
        assert!(
            chains[i].queue_entry_id.is_some(),
            "Chain {} should have queue_entry_id",
            i
        );
        assert_eq!(
            chains[i].queue_position,
            Some(i + 1), // 1-indexed positions
            "Chain {} should have queue_position {}",
            i,
            i + 1
        );
        // Buffer state should not be Idle
        assert_ne!(
            chains[i].buffer_state,
            Some("Idle".to_string()),
            "Chain {} should not be Idle",
            i
        );
    }

    // Clean up
    for passage in &passages {
        std::fs::remove_file(passage).unwrap();
    }
}

/// **[DBD-PARAM-050]** Test buffer chain monitoring with 15 passage queue (exceeds limit)
#[tokio::test]
async fn test_buffer_chains_exceeds_maximum() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let mut passages = Vec::new();
    for i in 0..15 {
        let passage = temp_dir.join(format!("test_chain_exceed_{}.mp3", i));
        std::fs::write(&passage, b"").unwrap();
        passages.push(passage);
    }

    // Enqueue 15 passages (exceeds maximum_decode_streams = 12)
    for passage in &passages {
        engine.enqueue_file(passage.clone()).await.unwrap();
    }

    sleep(Duration::from_millis(50)).await;

    // Get buffer chains
    let chains = engine.get_buffer_chains().await;

    // Should still return exactly 12 chains (limited by maximum_decode_streams)
    assert_eq!(
        chains.len(),
        12,
        "Should return exactly 12 chains (limited by maximum_decode_streams)"
    );

    // All 12 chains should be active
    for i in 0..12 {
        assert!(
            chains[i].queue_entry_id.is_some(),
            "Chain {} should have queue_entry_id",
            i
        );
        assert_eq!(
            chains[i].queue_position,
            Some(i + 1),
            "Chain {} should have queue_position {}",
            i,
            i + 1
        );
    }

    // Verify queue has all 15 entries (chains only show first 12)
    let queue_len = engine.queue_len().await;
    assert_eq!(
        queue_len, 15,
        "Queue should have all 15 entries (chains show first 12)"
    );

    // Clean up
    for passage in &passages {
        std::fs::remove_file(passage).unwrap();
    }
}

/// **[DBD-OV-080]** Test passage-based association across queue advances
#[tokio::test]
async fn test_buffer_chains_passage_tracking_on_skip() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let passage1 = temp_dir.join("test_skip_1.mp3");
    let passage2 = temp_dir.join("test_skip_2.mp3");
    let passage3 = temp_dir.join("test_skip_3.mp3");

    std::fs::write(&passage1, b"").unwrap();
    std::fs::write(&passage2, b"").unwrap();
    std::fs::write(&passage3, b"").unwrap();

    // Enqueue 3 passages
    engine.enqueue_file(passage1.clone()).await.unwrap();
    engine.enqueue_file(passage2.clone()).await.unwrap();
    engine.enqueue_file(passage3.clone()).await.unwrap();

    sleep(Duration::from_millis(50)).await;

    // Get initial buffer chains
    let chains_before = engine.get_buffer_chains().await;

    // Capture queue_entry_id of passage at position 2 (next)
    let next_qe_id = chains_before[1].queue_entry_id.expect("Position 2 should have ID");

    // Skip current passage
    engine.skip_next().await.unwrap();
    sleep(Duration::from_millis(50)).await;

    // Get buffer chains after skip
    let chains_after = engine.get_buffer_chains().await;

    // The passage that was at position 2 should now be at position 1
    // **[DBD-OV-080]** Chains follow passages via queue_entry_id
    assert_eq!(
        chains_after[0].queue_entry_id,
        Some(next_qe_id),
        "Passage should move from position 2 to position 1 (queue_entry_id follows passage)"
    );

    assert_eq!(
        chains_after[0].queue_position,
        Some(1),
        "queue_position should update to 1 (now playing)"
    );

    // Clean up
    std::fs::remove_file(&passage1).unwrap();
    std::fs::remove_file(&passage2).unwrap();
    std::fs::remove_file(&passage3).unwrap();
}

/// **[DBD-OV-080]** Test buffer chain updates during queue manipulation
#[tokio::test]
async fn test_buffer_chains_dynamic_queue_changes() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Start with empty queue - all chains should be idle
    let chains_empty = engine.get_buffer_chains().await;
    assert_eq!(chains_empty.len(), 12);
    for chain in &chains_empty {
        assert_eq!(chain.queue_entry_id, None, "Empty queue: all chains idle");
    }

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let passage1 = temp_dir.join("test_dynamic_1.mp3");
    let passage2 = temp_dir.join("test_dynamic_2.mp3");
    std::fs::write(&passage1, b"").unwrap();
    std::fs::write(&passage2, b"").unwrap();

    // Enqueue first passage - chain 0 should activate
    engine.enqueue_file(passage1.clone()).await.unwrap();
    sleep(Duration::from_millis(50)).await;

    let chains_one = engine.get_buffer_chains().await;
    assert!(
        chains_one[0].queue_entry_id.is_some(),
        "Chain 0 should activate"
    );
    assert_eq!(chains_one[1].queue_entry_id, None, "Chain 1 still idle");

    // Enqueue second passage - chain 1 should activate
    engine.enqueue_file(passage2.clone()).await.unwrap();
    sleep(Duration::from_millis(50)).await;

    let chains_two = engine.get_buffer_chains().await;
    assert!(
        chains_two[0].queue_entry_id.is_some(),
        "Chain 0 still active"
    );
    assert!(
        chains_two[1].queue_entry_id.is_some(),
        "Chain 1 now active"
    );

    // Skip to empty queue again
    engine.skip_next().await.unwrap();
    engine.skip_next().await.unwrap();
    sleep(Duration::from_millis(50)).await;

    let chains_final = engine.get_buffer_chains().await;
    for (i, chain) in chains_final.iter().enumerate() {
        assert_eq!(
            chain.queue_entry_id, None,
            "Chain {} should be idle after queue emptied",
            i
        );
    }

    // Clean up
    std::fs::remove_file(&passage1).unwrap();
    std::fs::remove_file(&passage2).unwrap();
}
