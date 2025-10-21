//! Chain persistence integration tests
//!
//! **[DBD-OV-080]** Decoder-buffer chains remain associated with passages
//! **[DBD-LIFECYCLE-010]** through **[DBD-LIFECYCLE-040]** Chain lifecycle management
//!
//! Tests verify that:
//! - Chains are assigned on enqueue
//! - Chains persist with passages throughout lifecycle
//! - Chains are released on completion/skip
//! - Chain allocation follows lowest-first strategy
//! - System behavior when all chains allocated

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

/// **[DBD-LIFECYCLE-010]** Test chain assignment on enqueue
#[tokio::test]
async fn test_chain_assignment_on_enqueue() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let mut passages = Vec::new();
    for i in 0..3 {
        let passage = temp_dir.join(format!("test_assign_{}.mp3", i));
        std::fs::write(&passage, b"").unwrap();
        passages.push(passage);
    }

    // Enqueue 3 passages
    for passage in &passages {
        engine.enqueue_file(passage.clone()).await.unwrap();
    }

    sleep(Duration::from_millis(50)).await;

    // Get buffer chains
    let chains = engine.get_buffer_chains().await;

    // **[DBD-LIFECYCLE-010]** Verify chains 0, 1, 2 are assigned
    assert_eq!(chains.len(), 12, "Should return exactly 12 chains");

    for i in 0..3 {
        assert!(
            chains[i].queue_entry_id.is_some(),
            "Chain {} should be assigned to a passage",
            i
        );
        assert_eq!(
            chains[i].queue_position,
            Some(i),
            "Chain {} should track queue position {}",
            i,
            i
        );
    }

    // Chains 3-11 should be idle
    for i in 3..12 {
        assert_eq!(
            chains[i].queue_entry_id,
            None,
            "Chain {} should be idle",
            i
        );
    }

    // Clean up
    for passage in &passages {
        std::fs::remove_file(passage).unwrap();
    }
}

/// **[DBD-OV-080]** Test chain persistence across queue advance
#[tokio::test]
async fn test_chain_persistence_across_queue_advance() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let mut passages = Vec::new();
    for i in 0..3 {
        let passage = temp_dir.join(format!("test_persist_{}.mp3", i));
        std::fs::write(&passage, b"").unwrap();
        passages.push(passage);
    }

    // Enqueue 3 passages: A, B, C
    for passage in &passages {
        engine.enqueue_file(passage.clone()).await.unwrap();
    }

    sleep(Duration::from_millis(50)).await;

    // Get initial buffer chains
    // A→chain0(pos0), B→chain1(pos1), C→chain2(pos2)
    let chains_before = engine.get_buffer_chains().await;
    let passage_b_id = chains_before[1].queue_entry_id.expect("Chain 1 should have passage B");
    let passage_c_id = chains_before[2].queue_entry_id.expect("Chain 2 should have passage C");

    // Skip passage A
    engine.skip_next().await.unwrap();
    sleep(Duration::from_millis(50)).await;

    // Get buffer chains after skip
    let chains_after = engine.get_buffer_chains().await;

    // **[DBD-OV-080]** Verify B and C stayed in their assigned chains
    assert_eq!(
        chains_after[0].queue_entry_id,
        None,
        "Chain 0 should be freed after passage A was skipped"
    );

    assert_eq!(
        chains_after[1].queue_entry_id,
        Some(passage_b_id),
        "Passage B should STILL be in chain 1 (not moved to chain 0)"
    );
    assert_eq!(
        chains_after[1].queue_position,
        Some(0),
        "Passage B in chain 1 should now be at queue position 0"
    );

    assert_eq!(
        chains_after[2].queue_entry_id,
        Some(passage_c_id),
        "Passage C should STILL be in chain 2 (not moved to chain 1)"
    );
    assert_eq!(
        chains_after[2].queue_position,
        Some(1),
        "Passage C in chain 2 should now be at queue position 1"
    );

    // Clean up
    for passage in &passages {
        std::fs::remove_file(passage).unwrap();
    }
}

/// **[DBD-LIFECYCLE-020]** Test chain reuse after completion
#[tokio::test]
async fn test_chain_reuse_after_completion() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let mut passages = Vec::new();
    for i in 0..4 {
        let passage = temp_dir.join(format!("test_reuse_{}.mp3", i));
        std::fs::write(&passage, b"").unwrap();
        passages.push(passage);
    }

    // Enqueue passages A, B, C (fills chains 0, 1, 2)
    for i in 0..3 {
        engine.enqueue_file(passages[i].clone()).await.unwrap();
    }

    sleep(Duration::from_millis(50)).await;

    // Verify all 3 chains assigned
    let chains_initial = engine.get_buffer_chains().await;
    assert!(chains_initial[0].queue_entry_id.is_some(), "Chain 0 should be assigned");
    assert!(chains_initial[1].queue_entry_id.is_some(), "Chain 1 should be assigned");
    assert!(chains_initial[2].queue_entry_id.is_some(), "Chain 2 should be assigned");

    // Skip passage A (releases chain 0)
    engine.skip_next().await.unwrap();
    sleep(Duration::from_millis(50)).await;

    // Verify chain 0 is now idle
    let chains_after_skip = engine.get_buffer_chains().await;
    assert_eq!(
        chains_after_skip[0].queue_entry_id,
        None,
        "Chain 0 should be released after skip"
    );

    // Enqueue passage D
    engine.enqueue_file(passages[3].clone()).await.unwrap();
    sleep(Duration::from_millis(50)).await;

    // **[DBD-LIFECYCLE-020]** Verify passage D was assigned to freed chain 0
    let chains_after_enqueue = engine.get_buffer_chains().await;
    assert!(
        chains_after_enqueue[0].queue_entry_id.is_some(),
        "Chain 0 should be reassigned to passage D"
    );
    assert_eq!(
        chains_after_enqueue[0].queue_position,
        Some(2),
        "Passage D in chain 0 should be at queue position 2"
    );

    // Clean up
    for passage in &passages {
        std::fs::remove_file(passage).unwrap();
    }
}

/// **[DBD-LIFECYCLE-030]** Test chain allocation lowest-first strategy
#[tokio::test]
async fn test_chain_allocation_lowest_first() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let mut passages = Vec::new();
    for i in 0..15 {
        let passage = temp_dir.join(format!("test_lowest_{}.mp3", i));
        std::fs::write(&passage, b"").unwrap();
        passages.push(passage);
    }

    // Enqueue 12 passages to fill all chains
    for i in 0..12 {
        engine.enqueue_file(passages[i].clone()).await.unwrap();
    }

    sleep(Duration::from_millis(50)).await;

    // Verify all 12 chains assigned
    let chains_full = engine.get_buffer_chains().await;
    for i in 0..12 {
        assert!(
            chains_full[i].queue_entry_id.is_some(),
            "Chain {} should be assigned",
            i
        );
    }

    // Skip passages at chains 5, 2, 8 (in that order - releases in that order)
    // After 12 skips we'll have skipped passages 0-11, leaving chain assignments empty
    // Skip passages to free specific chains
    for _ in 0..6 {
        engine.skip_next().await.unwrap();
    }
    sleep(Duration::from_millis(50)).await;

    // Enqueue 3 more passages
    for i in 12..15 {
        engine.enqueue_file(passages[i].clone()).await.unwrap();
        sleep(Duration::from_millis(10)).await;
    }

    sleep(Duration::from_millis(50)).await;

    // **[DBD-LIFECYCLE-030]** Verify passages assigned to lowest available chains
    let chains_after = engine.get_buffer_chains().await;

    // After skipping 6 passages, chains 0-5 should be freed
    // New passages should be assigned to chains 0, 1, 2 (lowest first)
    assert!(
        chains_after[0].queue_entry_id.is_some(),
        "Chain 0 should be reassigned (lowest available)"
    );
    assert!(
        chains_after[1].queue_entry_id.is_some(),
        "Chain 1 should be reassigned (second lowest available)"
    );
    assert!(
        chains_after[2].queue_entry_id.is_some(),
        "Chain 2 should be reassigned (third lowest available)"
    );

    // Clean up
    for passage in &passages {
        std::fs::remove_file(passage).unwrap();
    }
}

/// **[DBD-LIFECYCLE-010]** Test behavior when all chains allocated
#[tokio::test]
async fn test_no_chain_when_all_allocated() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db, state).await.unwrap();

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let mut passages = Vec::new();
    for i in 0..14 {
        let passage = temp_dir.join(format!("test_full_{}.mp3", i));
        std::fs::write(&passage, b"").unwrap();
        passages.push(passage);
    }

    // Enqueue 12 passages to fill all chains
    for i in 0..12 {
        engine.enqueue_file(passages[i].clone()).await.unwrap();
    }

    sleep(Duration::from_millis(50)).await;

    // Verify all 12 chains assigned
    let chains_full = engine.get_buffer_chains().await;
    let mut assigned_count = 0;
    for chain in &chains_full {
        if chain.queue_entry_id.is_some() {
            assigned_count += 1;
        }
    }
    assert_eq!(assigned_count, 12, "All 12 chains should be assigned");

    // Enqueue 13th passage (no chains available)
    engine.enqueue_file(passages[12].clone()).await.unwrap();
    sleep(Duration::from_millis(50)).await;

    // Verify still only 12 chains shown (13th passage queued but no chain yet)
    let chains_overflow = engine.get_buffer_chains().await;
    let mut assigned_count_after = 0;
    for chain in &chains_overflow {
        if chain.queue_entry_id.is_some() {
            assigned_count_after += 1;
        }
    }
    assert_eq!(
        assigned_count_after, 12,
        "Still only 12 chains should be assigned (13th passage waiting)"
    );

    // Skip one passage to free a chain
    engine.skip_next().await.unwrap();
    sleep(Duration::from_millis(50)).await;

    // 14th passage should now get the freed chain
    engine.enqueue_file(passages[13].clone()).await.unwrap();
    sleep(Duration::from_millis(50)).await;

    let chains_final = engine.get_buffer_chains().await;
    let mut assigned_count_final = 0;
    for chain in &chains_final {
        if chain.queue_entry_id.is_some() {
            assigned_count_final += 1;
        }
    }

    // We should have: 11 remaining from original 12, plus 13th passage, plus 14th passage = 13 total queued
    // But only 12 chains, so only 12 should show as assigned
    assert_eq!(
        assigned_count_final, 12,
        "Should still have exactly 12 chains assigned"
    );

    // Clean up
    for passage in &passages {
        std::fs::remove_file(passage).unwrap();
    }
}
