//! Integration tests for decoder-buffer chain assignment and priority filling
//!
//! **Test Coverage:**
//! - [DBD-LIFECYCLE-010] Chain assignment on enqueue
//! - [DBD-LIFECYCLE-020] Chain release on removal/completion
//! - [DBD-LIFECYCLE-030] Chain reassignment after release
//! - [DBD-DEC-045] Buffer-fill-aware priority re-evaluation
//!
//! **Historical Issues Addressed:**
//! - Chain collision when passages removed (chains not properly cleaned up in decoder_worker)
//! - Unassigned passages not getting chains when available
//! - Buffer filling priority not respecting queue position
//! - Re-evaluation not triggered on chain assignment changes

mod test_engine;

use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;
use tempfile::TempDir;
use test_engine::TestEngine;

/// Test 1: Chain Assignment on Enqueue
///
/// **Scenario:** Enqueue passages up to maximum_decode_streams limit
/// **Expected:** Each passage gets assigned a unique chain index
/// **Verifies:** [DBD-LIFECYCLE-010]
#[tokio::test]
async fn test_chain_assignment_on_enqueue() {
    // Setup: Create engine with maximum_decode_streams = 12
    let engine = TestEngine::new(12).await.unwrap();

    // Create temp directory for test audio files
    let temp_dir = TempDir::new().unwrap();

    // Enqueue 12 passages
    let mut queue_entry_ids = Vec::new();
    for i in 0..12 {
        let file_path = test_engine::create_test_audio_file_in_dir(temp_dir.path(), i).unwrap();
        let queue_entry_id = engine.enqueue_file(file_path).await.unwrap();
        queue_entry_ids.push(queue_entry_id);
    }

    // Verify: All 12 passages have assigned chains
    let chains = engine.get_buffer_chains().await;
    assert_eq!(chains.len(), 12, "All 12 passages should have chains");

    // Verify: Chain indexes are unique (0-11)
    let mut chain_indexes: Vec<usize> = chains.iter().map(|c| c.slot_index).collect();
    chain_indexes.sort();
    assert_eq!(chain_indexes, (0..12).collect::<Vec<_>>(), "Chain indexes should be 0-11");
}

/// Test 2: 13th Passage Does Not Get Chain
///
/// **Scenario:** Enqueue 13 passages when maximum_decode_streams = 12
/// **Expected:** First 12 get chains, 13th does not
/// **Verifies:** Proper handling of chain exhaustion
#[tokio::test]
async fn test_chain_exhaustion() {
    // Setup: Create engine with maximum_decode_streams = 12
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Enqueue 13 passages
    let mut queue_entry_ids = Vec::new();
    for i in 0..13 {
        let file_path = test_engine::create_test_audio_file_in_dir(temp_dir.path(), i).unwrap();
        let queue_entry_id = engine.enqueue_file(file_path).await.unwrap();
        queue_entry_ids.push(queue_entry_id);
    }

    // Verify: Only 12 passages have chains
    let chains = engine.get_buffer_chains().await;
    assert_eq!(chains.len(), 12, "Only 12 passages should have chains");

    // Verify: 13th passage exists in queue but has no chain
    let queue = engine.get_queue().await;
    assert_eq!(queue.len(), 13, "All 13 passages should be in queue");

    // Verify: 13th passage specifically has no chain
    let chain_13th = engine.get_chain_index(queue_entry_ids[12]).await;
    assert!(chain_13th.is_none(), "13th passage should not have a chain");
}

/// Test 3: Chain Release and Cleanup
///
/// **Scenario:** Remove passage from queue
/// **Expected:**
/// - Chain removed from decoder_worker active/yielded sets
/// - Chain index returned to available pool
/// - Buffer removed
/// **Verifies:** [DBD-LIFECYCLE-020] and historical chain collision bug fix
#[tokio::test]
async fn test_chain_release_on_removal() -> anyhow::Result<()> {
    // Setup: Enqueue 3 passages
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    let file_path1 = test_engine::create_test_audio_file_in_dir(temp_dir.path(), 1).unwrap();
    let id1 = engine.enqueue_file(file_path1).await.unwrap();

    let file_path2 = test_engine::create_test_audio_file_in_dir(temp_dir.path(), 2).unwrap();
    let id2 = engine.enqueue_file(file_path2).await.unwrap();

    let file_path3 = test_engine::create_test_audio_file_in_dir(temp_dir.path(), 3).unwrap();
    let id3 = engine.enqueue_file(file_path3).await.unwrap();

    // Verify initial state
    let chains_before = engine.get_buffer_chains().await;
    assert_eq!(chains_before.len(), 3, "Should have 3 chains initially");

    // Capture the chain index of the passage we're about to remove
    let removed_chain_index = engine.get_chain_index(id2).await;
    assert!(removed_chain_index.is_some(), "Middle passage should have a chain");

    // Remove middle passage
    engine.remove_queue_entry(id2).await?;

    // Verify: Only 2 chains remain
    let chains_after = engine.get_buffer_chains().await;
    assert_eq!(chains_after.len(), 2, "Should have 2 chains after removal");

    // Verify: id2 no longer has a chain
    let chain_after_removal = engine.get_chain_index(id2).await;
    assert!(chain_after_removal.is_none(), "Removed passage should not have a chain");

    // Verify: Removed passage's chain index is back in available pool
    // Should be able to enqueue new passage and get a chain immediately
    let file_path4 = test_engine::create_test_audio_file_in_dir(temp_dir.path(), 4).unwrap();
    let id4 = engine.enqueue_file(file_path4).await.unwrap();

    let chains_final = engine.get_buffer_chains().await;
    assert_eq!(chains_final.len(), 3, "New passage should get the freed chain");

    // Verify: New passage got a chain (should reuse the freed chain index)
    let new_chain_index = engine.get_chain_index(id4).await;
    assert!(new_chain_index.is_some(), "New passage should have a chain");
    assert_eq!(new_chain_index, removed_chain_index, "Should reuse the same chain index");

    Ok(())
}

/// Test 4: Unassigned Passage Gets Chain When Available
///
/// **Scenario:**
/// 1. Enqueue 13 passages (12 get chains, 13th doesn't)
/// 2. Remove 10 passages (3 remain: positions 0, 1, 2)
/// 3. Verify 3rd passage (originally 13th) gets assigned a chain
/// **Expected:** Unassigned passage automatically gets chain when freed
/// **Verifies:** [DBD-LIFECYCLE-030] and historical regression fix
#[tokio::test]
async fn test_unassigned_passage_gets_chain_on_availability() -> anyhow::Result<()> {
    // Setup: Create engine with maximum_decode_streams = 12
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Enqueue 13 passages (12 get chains, 13th doesn't)
    let mut queue_entry_ids = Vec::new();
    for i in 0..13 {
        let file_path = test_engine::create_test_audio_file_in_dir(temp_dir.path(), i).unwrap();
        let queue_entry_id = engine.enqueue_file(file_path).await.unwrap();
        queue_entry_ids.push(queue_entry_id);
    }

    // Verify 13th has no chain initially
    let chains_initial = engine.get_buffer_chains().await;
    assert_eq!(chains_initial.len(), 12, "Only 12 passages should have chains initially");

    let chain_13th_initial = engine.get_chain_index(queue_entry_ids[12]).await;
    assert!(chain_13th_initial.is_none(), "13th passage should not have a chain initially");

    // Remove passages 2-11 (keep 0, 1, and 12)
    for i in 2..12 {
        engine.remove_queue_entry(queue_entry_ids[i]).await?;
    }

    // Wait for cleanup and reassignment
    sleep(Duration::from_millis(200)).await;

    // Verify: 3 passages remain in queue
    let queue_after = engine.get_queue().await;
    assert_eq!(queue_after.len(), 3, "Should have 3 passages in queue");

    // Verify: All 3 passages now have chains (including originally unassigned 13th)
    let chains_after = engine.get_buffer_chains().await;
    assert_eq!(chains_after.len(), 3, "All 3 remaining passages should have chains");

    // Verify: The 13th passage (now at position 2) has a chain
    let chain_13th_after = engine.get_chain_index(queue_entry_ids[12]).await;
    assert!(chain_13th_after.is_some(), "13th passage should now have a chain");

    Ok(())
}

/// Test 5: Chain Reassignment After Batch Removal
///
/// **Scenario:**
/// 1. Start with 13 passages (12 with chains)
/// 2. Remove last 10 passages
/// 3. Enqueue 10 new passages
/// **Expected:** All 10 new passages get chains immediately
/// **Verifies:** Full chain lifecycle including reassignment
#[tokio::test]
async fn test_chain_reassignment_after_batch_removal() -> anyhow::Result<()> {
    // Setup: Create engine with maximum_decode_streams = 12
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Enqueue 13 passages (12 get chains, 13th doesn't)
    let mut original_ids = Vec::new();
    for i in 0..13 {
        let file_path = test_engine::create_test_audio_file_in_dir(temp_dir.path(), i).unwrap();
        let id = engine.enqueue_file(file_path).await.unwrap();
        original_ids.push(id);
    }

    // Verify initial state: 12 with chains, 13th without
    let chains_initial = engine.get_buffer_chains().await;
    assert_eq!(chains_initial.len(), 12, "Should have 12 passages with chains initially");

    // Remove last 10 passages (indices 3-12, keeping 0, 1, 2)
    for i in 3..13 {
        engine.remove_queue_entry(original_ids[i]).await?;
    }

    // Wait for cleanup
    sleep(Duration::from_millis(200)).await;

    // Verify: Only 3 passages remain with chains
    let chains_after_removal = engine.get_buffer_chains().await;
    assert_eq!(chains_after_removal.len(), 3, "Should have 3 passages with chains after removal");

    // Enqueue 10 new passages
    let mut new_ids = Vec::new();
    for i in 100..110 {
        let file_path = test_engine::create_test_audio_file_in_dir(temp_dir.path(), i).unwrap();
        let id = engine.enqueue_file(file_path).await.unwrap();
        new_ids.push(id);
    }

    // Wait for assignment
    sleep(Duration::from_millis(200)).await;

    // Verify: Total of 13 passages in queue (3 original + 10 new)
    let queue_final = engine.get_queue().await;
    assert_eq!(queue_final.len(), 13, "Should have 13 passages in queue");

    // Verify: 12 passages have chains (maximum_decode_streams limit)
    let chains_final = engine.get_buffer_chains().await;
    assert_eq!(chains_final.len(), 12, "Should have 12 passages with chains");

    // Verify: First 9 new passages got chains (3 original + 9 new = 12 total)
    for i in 0..9 {
        let chain = engine.get_chain_index(new_ids[i]).await;
        assert!(chain.is_some(), "New passage {} should have a chain", i);
    }

    // Verify: 10th new passage does not have chain (would exceed limit)
    let chain_10th = engine.get_chain_index(new_ids[9]).await;
    assert!(chain_10th.is_none(), "10th new passage should not have a chain (limit reached)");

    Ok(())
}

/// Test 6: Buffer Priority Selection - Queue Position Order
///
/// **Scenario:** Multiple passages queued, verify decoder selects by play_order
/// **Expected:** Position 0 selected first, then 1, then 2+
/// **Verifies:** [DBD-DEC-045] priority selection algorithm
///
/// **NOTE:** This test requires active playback and buffer monitoring.
/// Current limitation: Without playback, buffers don't fill naturally.
/// This test verifies the infrastructure exists but cannot fully test priority
/// without decoder actively running.
#[tokio::test]
#[ignore = "Requires active playback - infrastructure test only"]
async fn test_buffer_priority_by_queue_position() -> anyhow::Result<()> {
    // Setup: Create engine with multiple decode streams
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Enqueue 3 passages
    let mut queue_entry_ids = Vec::new();
    for i in 0..3 {
        let file_path = test_engine::create_test_audio_file_in_dir(temp_dir.path(), i).unwrap();
        let queue_entry_id = engine.enqueue_file(file_path).await.unwrap();
        queue_entry_ids.push(queue_entry_id);
    }

    // Verify all have chains
    let chains = engine.get_buffer_chains().await;
    assert_eq!(chains.len(), 3, "All 3 passages should have chains");

    // Verify we can query buffer fill levels (infrastructure test)
    for queue_entry_id in &queue_entry_ids {
        let fill_percent = engine.engine.test_get_buffer_fill_percent(*queue_entry_id).await;
        // Buffer fill percent may be None if buffer not yet initialized, or Some(0.0) if initialized but empty
        // Both are valid states without active decoding
        assert!(
            fill_percent.is_none() || fill_percent == Some(0.0),
            "Without active decoding, buffers should be uninitialized or empty"
        );
    }

    // NOTE: To fully test priority, would need:
    // 1. Start playback with engine.play()
    // 2. Wait for decoder to start filling buffers
    // 3. Monitor fill levels over time
    // 4. Verify position 0 fills faster than position 1, etc.
    //
    // This requires the decoder worker to be actively running, which needs
    // audio output infrastructure that isn't available in test environment.

    Ok(())
}

/// Test 7: Re-evaluation Trigger on Chain Assignment Change
///
/// **Scenario:** Remove passage while decoder is filling another
/// **Expected:** Decoder re-evaluates priorities immediately
/// **Verifies:** [DBD-DEC-045] chain_assignments_generation trigger
///
/// **Infrastructure Test:** Verifies that chain assignment changes are detected
/// and tracked. Full test requires active decoder worker with telemetry.
#[tokio::test]
#[ignore = "Requires active playback - infrastructure test only"]
async fn test_reevaluation_on_chain_assignment_change() -> anyhow::Result<()> {
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Enqueue 3 passages
    let mut queue_entry_ids = Vec::new();
    for i in 0..3 {
        let file_path = test_engine::create_test_audio_file_in_dir(temp_dir.path(), i).unwrap();
        let queue_entry_id = engine.enqueue_file(file_path).await.unwrap();
        queue_entry_ids.push(queue_entry_id);
    }

    // Verify all have chains initially
    let chains_before = engine.get_buffer_chains().await;
    assert_eq!(chains_before.len(), 3, "All 3 passages should have chains");

    // Remove middle passage (position 1)
    engine.remove_queue_entry(queue_entry_ids[1]).await?;

    // Verify chain assignments updated (infrastructure verification)
    let chains_after = engine.get_buffer_chains().await;
    assert_eq!(chains_after.len(), 2, "Should have 2 passages with chains after removal");

    // Verify remaining passages still have chains
    assert!(
        engine.get_chain_index(queue_entry_ids[0]).await.is_some(),
        "First passage should still have chain"
    );
    assert!(
        engine.get_chain_index(queue_entry_ids[2]).await.is_some(),
        "Third passage should still have chain"
    );

    // NOTE: To fully test re-evaluation trigger, would need:
    // 1. Telemetry/events tracking when decoder re-evaluates priority
    // 2. Verification that re-evaluation happens immediately on chain change
    // 3. Monitoring which buffer decoder switches to after removal
    //
    // This requires decoder_worker instrumentation or event stream that
    // reports priority selection decisions.

    Ok(())
}

/// Test 8: Buffer Fill Level Check (can_decoder_resume)
///
/// **Scenario:** Verify decoder only fills buffers below resume threshold
/// **Expected:** Buffers above resume threshold not selected for filling
/// **Verifies:** [DBD-DEC-045] buffer-fill-aware selection
///
/// **Infrastructure Test:** Verifies buffer fill monitoring capability exists.
/// Full test requires ability to fill/drain buffers programmatically.
#[tokio::test]
#[ignore = "Requires active playback - infrastructure test only"]
async fn test_buffer_fill_level_selection() -> anyhow::Result<()> {
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Enqueue 3 passages
    let mut queue_entry_ids = Vec::new();
    for i in 0..3 {
        let file_path = test_engine::create_test_audio_file_in_dir(temp_dir.path(), i).unwrap();
        let queue_entry_id = engine.enqueue_file(file_path).await.unwrap();
        queue_entry_ids.push(queue_entry_id);
    }

    // Verify all have chains
    let chains = engine.get_buffer_chains().await;
    assert_eq!(chains.len(), 3, "All 3 passages should have chains");

    // Verify we can query buffer fill levels for each passage
    for (idx, queue_entry_id) in queue_entry_ids.iter().enumerate() {
        let fill_percent = engine.engine.test_get_buffer_fill_percent(*queue_entry_id).await;
        // Without active decoding, buffers should be uninitialized or empty
        assert!(
            fill_percent.is_none() || fill_percent == Some(0.0),
            "Passage {} buffer should be uninitialized or empty without active decoding",
            idx
        );
    }

    // NOTE: To fully test buffer fill level selection, would need:
    // 1. Start playback to activate decoder
    // 2. Programmatically fill buffer above resume threshold
    //    (e.g., write PCM data directly or let decoder run until threshold)
    // 3. Verify that select_highest_priority_chain() skips this buffer
    // 4. Drain buffer below threshold (consume PCM data)
    // 5. Verify that select_highest_priority_chain() now includes this buffer
    //
    // This requires either:
    // - Mock audio output that allows buffer manipulation, or
    // - Test helpers to artificially fill/drain buffers, or
    // - Integration test environment with real audio output
    //
    // Current infrastructure test validates that buffer monitoring exists,
    // which is prerequisite for full functional test.

    Ok(())
}

/// Test 9: Decode Work Period Re-evaluation
///
/// **Scenario:** Decoder filling one buffer for > decode_work_period
/// **Expected:** Decoder re-evaluates priorities after timeout
/// **Verifies:** [DBD-DEC-045] time-based re-evaluation trigger
///
/// **Infrastructure Test:** Verifies basic timing infrastructure exists.
/// Full test requires active decoder worker with telemetry.
#[tokio::test]
#[ignore = "Requires active playback - infrastructure test only"]
async fn test_decode_work_period_reevaluation() -> anyhow::Result<()> {
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Enqueue 2 passages
    let mut queue_entry_ids = Vec::new();
    for i in 0..2 {
        let file_path = test_engine::create_test_audio_file_in_dir(temp_dir.path(), i).unwrap();
        let queue_entry_id = engine.enqueue_file(file_path).await.unwrap();
        queue_entry_ids.push(queue_entry_id);
    }

    // Verify both have chains
    let chains = engine.get_buffer_chains().await;
    assert_eq!(chains.len(), 2, "Both passages should have chains");

    // Infrastructure test: Verify time can elapse and state remains consistent
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify chains still assigned after time passes
    let chains_after = engine.get_buffer_chains().await;
    assert_eq!(chains_after.len(), 2, "Both passages should still have chains after delay");

    // NOTE: To fully test decode work period re-evaluation, would need:
    // 1. Configure decode_work_period to small value (e.g., 500ms) in settings
    // 2. Start playback to activate decoder
    // 3. Monitor which buffer decoder is filling (via telemetry/events)
    // 4. Wait > decode_work_period duration
    // 5. Verify that decoder re-evaluated priority (via generation counter increment)
    // 6. Verify decoder potentially switched to different buffer based on priority
    //
    // This requires:
    // - Decoder worker instrumentation (generation counter access)
    // - Telemetry events for priority selection decisions
    // - Active playback environment
    //
    // Current infrastructure test validates that timing infrastructure works,
    // which is prerequisite for full functional test.

    Ok(())
}

/// Test 10: No Chain Collision After Remove/Enqueue
///
/// **Scenario:** Remove passage, enqueue new one with same chain index
/// **Expected:** No collision - old chain fully cleaned up before reuse
/// **Verifies:** [DBD-DEC-045] Chain assignment after removal
///
/// **Bug Fix:** Revealed timing issue where `assign_chains_to_unassigned_entries()`
/// was called from within `release_chain()` before queue state was consistent.
/// This caused freed chains to be reassigned to entries being removed.
#[tokio::test]
async fn test_no_chain_collision() -> anyhow::Result<()> {
    // Setup: Create engine with maximum_decode_streams = 12
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Enqueue single passage and get its chain index
    let file_path1 = test_engine::create_test_audio_file_in_dir(temp_dir.path(), 1).unwrap();
    let id1 = engine.enqueue_file(file_path1).await.unwrap();

    let chain_index_before = engine.get_chain_index(id1).await;
    assert!(chain_index_before.is_some(), "First passage should have a chain");

    // Remove passage
    engine.remove_queue_entry(id1).await?;

    // Wait for cleanup to complete before enqueueing new passage
    // (In production, the API enforces this sequencing naturally)
    sleep(Duration::from_millis(200)).await;

    // Enqueue new passage (should get a chain without collision)
    let file_path2 = test_engine::create_test_audio_file_in_dir(temp_dir.path(), 2).unwrap();
    let id2 = engine.enqueue_file(file_path2).await.unwrap();

    let chain_index_after = engine.get_chain_index(id2).await;
    assert!(chain_index_after.is_some(), "Second passage should have a chain");

    // Verify: No collision - only one chain exists for the new passage
    // (The specific chain index doesn't matter - what matters is no collision occurs)
    let chains_final = engine.get_buffer_chains().await;
    assert_eq!(chains_final.len(), 1, "Should have exactly 1 chain (no collision)");

    // Verify: The single chain belongs to the new passage (id2), not the old one
    assert_eq!(chains_final[0].queue_entry_id, Some(id2), "Chain should belong to new passage");

    // Verify: Old passage no longer has a chain (cleanup successful)
    let old_chain = engine.get_chain_index(id1).await;
    assert!(old_chain.is_none(), "Old passage should not have a chain anymore");

    Ok(())
}

/// Test 11: Play Order Synchronization on Enqueue
///
/// **Scenario:** Enqueue multiple passages and verify play_order is synchronized from database to memory
/// **Expected:** In-memory queue entries have correct play_order values (not hardcoded to 0)
/// **Verifies:** Bug #3 - play_order synchronization issue that caused priority selection failure
///
/// **Bug History:** This bug regressed 3+ times, causing decoder to fill buffers in haphazard order
/// instead of respecting queue position. Newly enqueued passages had play_order=0 in memory while
/// database had correct values, breaking decoder priority selection.
#[tokio::test]
async fn test_play_order_synchronization() -> anyhow::Result<()> {
    let engine = TestEngine::new(12).await.unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Enqueue first passage - should get play_order from database (typically 10)
    let file_path1 = test_engine::create_test_audio_file_in_dir(temp_dir.path(), 1).unwrap();
    let id1 = engine.enqueue_file(file_path1).await.unwrap();

    // Enqueue second passage - should get next play_order (typically 20)
    let file_path2 = test_engine::create_test_audio_file_in_dir(temp_dir.path(), 2).unwrap();
    let id2 = engine.enqueue_file(file_path2).await.unwrap();

    // Enqueue third passage - should get next play_order (typically 30)
    let file_path3 = test_engine::create_test_audio_file_in_dir(temp_dir.path(), 3).unwrap();
    let id3 = engine.enqueue_file(file_path3).await.unwrap();

    // Get IN-MEMORY queue entries to check play_order values
    // CRITICAL: Must use get_queue_entries_from_memory() not get_queue_entries()
    // Bug #3 was in the in-memory queue state (what decoder uses), not database
    let queue_entries = engine.get_queue_entries_from_memory().await;

    // Find our entries in the queue
    let entry1 = queue_entries.iter().find(|e| e.queue_entry_id == id1).expect("Entry 1 not found");
    let entry2 = queue_entries.iter().find(|e| e.queue_entry_id == id2).expect("Entry 2 not found");
    let entry3 = queue_entries.iter().find(|e| e.queue_entry_id == id3).expect("Entry 3 not found");

    // CRITICAL: Verify play_order values are NOT all zero
    // Bug #3 caused all newly enqueued entries to have play_order=0 in memory
    assert_ne!(entry1.play_order, 0, "Entry 1 play_order should not be 0 (Bug #3 regression check)");
    assert_ne!(entry2.play_order, 0, "Entry 2 play_order should not be 0 (Bug #3 regression check)");
    assert_ne!(entry3.play_order, 0, "Entry 3 play_order should not be 0 (Bug #3 regression check)");

    // Verify play_order values are sequential and increasing
    assert!(
        entry2.play_order > entry1.play_order,
        "Entry 2 play_order ({}) should be > Entry 1 play_order ({})",
        entry2.play_order, entry1.play_order
    );
    assert!(
        entry3.play_order > entry2.play_order,
        "Entry 3 play_order ({}) should be > Entry 2 play_order ({})",
        entry3.play_order, entry2.play_order
    );

    // Verify play_order values are properly spaced (database uses gaps of 10)
    let gap1 = entry2.play_order - entry1.play_order;
    let gap2 = entry3.play_order - entry2.play_order;
    assert_eq!(gap1, 10, "Gap between entry 1 and 2 should be 10");
    assert_eq!(gap2, 10, "Gap between entry 2 and 3 should be 10");

    Ok(())
}

// Helper functions (to be implemented)

// async fn create_test_engine(max_streams: usize) -> TestEngine {
//     // Create in-memory SQLite database
//     // Initialize engine with test configuration
//     // Set maximum_decode_streams to specified value
// }

// struct TestEngine {
//     // Wrapper around PlaybackEngine for testing
// }
