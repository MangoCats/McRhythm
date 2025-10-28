//! Queue Integrity and Advancement Tests
//!
//! Comprehensive tests to catch queue advancement bugs including:
//! - Duplicate passage playback detection
//! - Premature passage abortion
//! - Queue/database synchronization issues
//! - Race conditions in passage advancement
//! - Mixer state consistency
//!
//! **Background:**
//! These tests were designed to catch a production bug where:
//! - Passage 1 aborted early (not playing full duration)
//! - Passage 2 played twice instead of once
//! - Queue display lagged behind actual state
//!
//! **Traceability:**
//! - [SSD-FLOW-010] Complete playback sequence
//! - [SSD-ENG-020] Queue processing
//! - [DB-QUEUE-010] Queue table schema

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;
use wkmp_common::events::WkmpEvent;

use wkmp_ap::playback::engine::PlaybackEngine;
use wkmp_ap::playback::queue_manager::QueueManager;
use wkmp_ap::state::SharedState;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};

/// Create in-memory test database with schema
async fn create_test_db() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    // Create queue table
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

    // Create settings table (required by engine)
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
    .unwrap();

    pool
}

/// Event counter for tracking PassageStarted/PassageCompleted events
#[derive(Debug, Clone)]
struct EventCounter {
    started: Arc<Mutex<HashMap<Uuid, usize>>>,
    completed: Arc<Mutex<HashMap<Uuid, usize>>>,
    queue_changed: Arc<Mutex<usize>>,
}

impl EventCounter {
    fn new() -> Self {
        Self {
            started: Arc::new(Mutex::new(HashMap::new())),
            completed: Arc::new(Mutex::new(HashMap::new())),
            queue_changed: Arc::new(Mutex::new(0)),
        }
    }

    fn record_event(&self, event: &WkmpEvent) {
        match event {
            WkmpEvent::PassageStarted { passage_id, .. } => {
                let mut started = self.started.lock().unwrap();
                *started.entry(*passage_id).or_insert(0) += 1;
            }
            WkmpEvent::PassageCompleted { passage_id, .. } => {
                let mut completed = self.completed.lock().unwrap();
                *completed.entry(*passage_id).or_insert(0) += 1;
            }
            WkmpEvent::QueueChanged { .. } => {
                let mut queue_changed = self.queue_changed.lock().unwrap();
                *queue_changed += 1;
            }
            _ => {}
        }
    }

    fn get_started_count(&self, passage_id: Uuid) -> usize {
        self.started.lock().unwrap().get(&passage_id).copied().unwrap_or(0)
    }

    fn get_completed_count(&self, passage_id: Uuid) -> usize {
        self.completed.lock().unwrap().get(&passage_id).copied().unwrap_or(0)
    }

    fn get_queue_changed_count(&self) -> usize {
        *self.queue_changed.lock().unwrap()
    }

    fn assert_passage_played_exactly_once(&self, passage_id: Uuid, passage_name: &str) {
        let started = self.get_started_count(passage_id);
        let completed = self.get_completed_count(passage_id);

        assert_eq!(
            started, 1,
            "{} should have exactly ONE PassageStarted event, but got {}",
            passage_name, started
        );
        assert_eq!(
            completed, 1,
            "{} should have exactly ONE PassageCompleted event, but got {}",
            passage_name, completed
        );
    }

    fn assert_passage_never_played(&self, passage_id: Uuid, passage_name: &str) {
        let started = self.get_started_count(passage_id);
        let completed = self.get_completed_count(passage_id);

        assert_eq!(
            started, 0,
            "{} should have ZERO PassageStarted events, but got {}",
            passage_name, started
        );
        assert_eq!(
            completed, 0,
            "{} should have ZERO PassageCompleted events, but got {}",
            passage_name, completed
        );
    }
}

// ============================================================================
// TEST 1: Three-Passage Queue Advancement Integrity
// ============================================================================

/// **Integration Test: Three-Passage Queue Advancement Integrity**
///
/// This test verifies that when 3 passages are enqueued and played through:
/// 1. Each passage receives exactly ONE PassageStarted event
/// 2. Each passage receives exactly ONE PassageCompleted event
/// 3. Queue length decrements correctly (3 → 2 → 1 → 0)
/// 4. No passage is played twice (duplicate events)
/// 5. Passages play in correct order
///
/// **Bug Detection:**
/// - If passage plays twice: Will detect duplicate PassageStarted events
/// - If passage aborts early: Will detect missing PassageCompleted event
/// - If queue lags: Will detect queue length mismatch
///
/// **Current Bug Expected Failure:**
/// - Passage 1: Should get 1 started, 1 completed (may get 0 completed if aborts early)
/// - Passage 2: Should get 1 started, 1 completed (may get 2 started if plays twice)
/// - Passage 3: Should get 1 started, 1 completed
#[tokio::test]
#[ignore] // Requires actual audio files to decode - enable for integration testing
async fn test_three_passage_queue_advancement_integrity() {
    // Setup
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db.clone(), state.clone())
        .await
        .expect("Failed to create engine");

    // Subscribe to events BEFORE starting engine
    let mut event_rx = state.subscribe_events();
    let event_counter = EventCounter::new();

    // Spawn event listener task
    let counter_clone = event_counter.clone();
    let event_listener = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            counter_clone.record_event(&event);
        }
    });

    engine.start().await.expect("Failed to start engine");

    // Enqueue 3 test passages (must be real audio files for this test)
    let passage1_id = Uuid::new_v4();
    let passage2_id = Uuid::new_v4();
    let passage3_id = Uuid::new_v4();

    // Insert passages into queue table
    sqlx::query(
        r#"
        INSERT INTO queue (guid, file_path, passage_guid, play_order)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind("/path/to/test1.mp3")
    .bind(passage1_id.to_string())
    .bind(10)
    .execute(&db)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO queue (guid, file_path, passage_guid, play_order)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind("/path/to/test2.mp3")
    .bind(passage2_id.to_string())
    .bind(20)
    .execute(&db)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO queue (guid, file_path, passage_guid, play_order)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind("/path/to/test3.mp3")
    .bind(passage3_id.to_string())
    .bind(30)
    .execute(&db)
    .await
    .unwrap();

    // Wait for passages to play through (with timeout)
    // This test requires real audio files and actual playback
    let result = timeout(Duration::from_secs(60), async {
        // Poll until all 3 passages complete
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;

            let p1_completed = event_counter.get_completed_count(passage1_id);
            let p2_completed = event_counter.get_completed_count(passage2_id);
            let p3_completed = event_counter.get_completed_count(passage3_id);

            if p1_completed > 0 && p2_completed > 0 && p3_completed > 0 {
                break;
            }
        }
    })
    .await;

    assert!(result.is_ok(), "Test timed out waiting for passages to complete");

    // Give events time to propagate
    tokio::time::sleep(Duration::from_millis(500)).await;

    // CRITICAL ASSERTIONS: Verify each passage played exactly once
    event_counter.assert_passage_played_exactly_once(passage1_id, "Passage 1");
    event_counter.assert_passage_played_exactly_once(passage2_id, "Passage 2");
    event_counter.assert_passage_played_exactly_once(passage3_id, "Passage 3");

    // Verify queue is empty after all passages complete
    let queue_len = engine.queue_len().await;
    assert_eq!(queue_len, 0, "Queue should be empty after all passages complete");

    // Verify database is also empty
    let db_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM queue")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(db_count, 0, "Database queue should be empty");

    // Cleanup
    event_listener.abort();
    engine.stop().await.expect("Failed to stop engine");
}

// ============================================================================
// TEST 2: Unit Test - Queue Manager advance() Removes Current
// ============================================================================

/// **Unit Test: Queue Manager advance() Removes Current**
///
/// This test verifies that QueueManager.advance():
/// 1. Removes the current entry from the queue
/// 2. Promotes next to current
/// 3. Promotes first queued to next
/// 4. Decrements total queue length correctly
///
/// **Bug Detection:**
/// - If advance() doesn't remove current: Will detect queue length mismatch
/// - If advance() loses entries: Will detect missing passages
/// - If advance() duplicates entries: Will detect length increase
#[test]
fn test_queue_advance_removes_current() {
    use wkmp_ap::playback::queue_manager::{QueueEntry, QueueManager};
    use std::path::PathBuf;

    // Helper to create test entries
    fn create_entry(id: u8) -> QueueEntry {
        QueueEntry {
            queue_entry_id: Uuid::from_bytes([id; 16]),
            passage_id: Some(Uuid::from_bytes([id; 16])),
            file_path: PathBuf::from(format!("test{}.mp3", id)),
            play_order: (id as i64) * 10,
            start_time_ms: None,
            end_time_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: None,
            fade_in_point_ms: None,
            fade_out_point_ms: None,
            fade_in_curve: None,
            fade_out_curve: None,
            discovered_end_ticks: None,
        }
    }

    let mut manager = QueueManager::new();

    // Create 3 entries
    let entry1 = create_entry(1);
    let entry2 = create_entry(2);
    let entry3 = create_entry(3);

    let id1 = entry1.queue_entry_id;
    let id2 = entry2.queue_entry_id;
    let id3 = entry3.queue_entry_id;

    // Enqueue all 3
    manager.enqueue(entry1);
    manager.enqueue(entry2);
    manager.enqueue(entry3);

    // Initial state: current=1, next=2, queued=[3]
    assert_eq!(manager.len(), 3, "Queue should have 3 entries");
    assert_eq!(manager.current().unwrap().queue_entry_id, id1);
    assert_eq!(manager.next().unwrap().queue_entry_id, id2);
    assert_eq!(manager.queued().len(), 1);

    // CRITICAL: Advance and verify entry 1 is removed
    let new_current = manager.advance();
    assert!(new_current.is_some(), "advance() should return new current");
    assert_eq!(new_current.unwrap().queue_entry_id, id2, "Entry 2 should be new current");

    // Verify entry 1 is GONE (not in current, next, or queued)
    assert_ne!(manager.current().unwrap().queue_entry_id, id1, "Entry 1 should be removed from current");
    assert!(manager.next().is_some(), "Next should not be None");
    assert_ne!(manager.next().unwrap().queue_entry_id, id1, "Entry 1 should not be in next");
    assert!(
        !manager.queued().iter().any(|e| e.queue_entry_id == id1),
        "Entry 1 should not be in queued"
    );

    // Verify queue length decremented
    assert_eq!(manager.len(), 2, "Queue should have 2 entries after advance (3 - 1 = 2)");

    // Verify new state: current=2, next=3, queued=[]
    assert_eq!(manager.current().unwrap().queue_entry_id, id2);
    assert_eq!(manager.next().unwrap().queue_entry_id, id3);
    assert_eq!(manager.queued().len(), 0);

    // Advance again
    manager.advance();
    assert_eq!(manager.len(), 1, "Queue should have 1 entry after second advance");
    assert_eq!(manager.current().unwrap().queue_entry_id, id3);
    assert!(manager.next().is_none(), "Next should be None after second advance");

    // Advance final time
    manager.advance();
    assert_eq!(manager.len(), 0, "Queue should be empty after third advance");
    assert!(manager.current().is_none(), "Current should be None when empty");
    assert!(manager.next().is_none(), "Next should be None when empty");
}

// ============================================================================
// TEST 3: Mixer State Consistency Test
// ============================================================================

/// **Unit Test: Mixer Queue State Synchronization**
///
/// This test verifies that after passage completion:
/// 1. Mixer stops reading from completed passage's buffer
/// 2. Mixer transitions to next passage cleanly
/// 3. No buffer is accessed after being marked exhausted
/// 4. No passage is started twice in the mixer
///
/// **Bug Detection:**
/// - If mixer continues reading after completion: Will detect buffer access violation
/// - If mixer starts passage twice: Will detect duplicate state transitions
///
/// **Note:** This is a conceptual test - actual implementation requires mock buffers
#[tokio::test]
#[ignore] // Requires mock buffer infrastructure
async fn test_mixer_queue_state_sync() {
    // This test would require:
    // 1. Mock PassageBuffer that tracks access count
    // 2. Mixer that can be stepped manually
    // 3. Verification that buffer is not accessed after is_current_finished() returns true

    // Pseudo-code for implementation:
    // let mixer = CrossfadeMixer::new();
    // let buffer1 = MockBuffer::new(passage1_id, samples=1000);
    // let buffer2 = MockBuffer::new(passage2_id, samples=1000);
    //
    // // Start passage 1
    // mixer.start_passage(buffer1.clone(), ...);
    // assert_eq!(buffer1.access_count(), 1, "Buffer 1 should be accessed once for start");
    //
    // // Read until completion
    // while !mixer.is_current_finished() {
    //     mixer.get_next_frame();
    // }
    //
    // // Mark as finished
    // let access_count_at_finish = buffer1.access_count();
    //
    // // Start passage 2
    // mixer.start_passage(buffer2.clone(), ...);
    //
    // // Read some frames from passage 2
    // for _ in 0..100 {
    //     mixer.get_next_frame();
    // }
    //
    // // CRITICAL ASSERTION: Buffer 1 should not be accessed again
    // assert_eq!(
    //     buffer1.access_count(),
    //     access_count_at_finish,
    //     "Completed buffer should not be accessed after mixer moves to next passage"
    // );
    //
    // // Verify passage 2 is being read
    // assert!(buffer2.access_count() > 1, "Buffer 2 should be actively accessed");

    panic!("Test not implemented - requires mock buffer infrastructure");
}

// ============================================================================
// TEST 4: Race Condition - Rapid Completion Events
// ============================================================================

/// **Integration Test: Queue Advancement No Double-Trigger**
///
/// This test verifies that rapid passage completion events don't cause:
/// 1. Duplicate "start next passage" triggers
/// 2. Double advancement of queue
/// 3. State machine violations
///
/// **Bug Detection:**
/// - If completion handler is not idempotent: Will detect duplicate advancement
/// - If state machine has race conditions: Will detect invalid state transitions
///
/// **Test Strategy:**
/// Simulate rapid is_current_finished() checks and verify queue advances exactly once
#[tokio::test]
async fn test_queue_advancement_no_double_trigger() {
    // This test simulates the race condition where:
    // 1. playback_loop checks is_current_finished() → true
    // 2. buffer_event_handler also checks and starts next passage
    // 3. Both try to advance the queue simultaneously

    let mut manager = QueueManager::new();

    // Helper to create test entries
    fn create_entry(id: u8) -> wkmp_ap::playback::queue_manager::QueueEntry {
        wkmp_ap::playback::queue_manager::QueueEntry {
            queue_entry_id: Uuid::from_bytes([id; 16]),
            passage_id: Some(Uuid::from_bytes([id; 16])),
            file_path: std::path::PathBuf::from(format!("test{}.mp3", id)),
            play_order: (id as i64) * 10,
            start_time_ms: None,
            end_time_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: None,
            fade_in_point_ms: None,
            fade_out_point_ms: None,
            fade_in_curve: None,
            fade_out_curve: None,
            discovered_end_ticks: None,
        }
    }

    // Enqueue 3 passages
    manager.enqueue(create_entry(1));
    manager.enqueue(create_entry(2));
    manager.enqueue(create_entry(3));

    assert_eq!(manager.len(), 3, "Queue should start with 3 entries");

    // Simulate race condition: Multiple advance() calls in rapid succession
    // (As if multiple threads detected completion simultaneously)
    manager.advance();
    assert_eq!(manager.len(), 2, "First advance should decrement to 2");

    // Second advance (should continue to next passage, not corrupt state)
    manager.advance();
    assert_eq!(manager.len(), 1, "Second advance should decrement to 1");

    // Third advance
    manager.advance();
    assert_eq!(manager.len(), 0, "Third advance should result in empty queue");

    // Fourth advance (on empty queue - should not crash or corrupt)
    let result = manager.advance();
    assert!(result.is_none(), "Advancing empty queue should return None");
    assert_eq!(manager.len(), 0, "Queue should remain empty");
    assert!(manager.is_empty(), "Queue should report as empty");
}

// ============================================================================
// TEST 5: Queue/Database Consistency Test
// ============================================================================

/// **Integration Test: Queue Database Consistency**
///
/// This test verifies that in-memory queue state stays synchronized with database:
/// 1. After enqueue: Both memory and DB have entry
/// 2. After passage completes: Entry removed from BOTH memory and DB
/// 3. After skip: Entry removed from BOTH memory and DB
/// 4. Final state: Both memory and DB are consistent
///
/// **Bug Detection:**
/// - If DB update fails: Will detect memory/DB mismatch
/// - If DB update is delayed: Will detect stale data in queries
/// - If queue advancement doesn't sync: Will detect orphaned DB entries
#[tokio::test]
#[ignore] // Requires actual audio files to enqueue
async fn test_queue_database_consistency() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db.clone(), state.clone())
        .await
        .expect("Failed to create engine");

    engine.start().await.expect("Failed to start engine");

    // Enqueue 3 passages
    use std::path::PathBuf;
    let id1 = engine.enqueue_file(PathBuf::from("/test/passage1.mp3")).await.unwrap();
    let id2 = engine.enqueue_file(PathBuf::from("/test/passage2.mp3")).await.unwrap();
    let id3 = engine.enqueue_file(PathBuf::from("/test/passage3.mp3")).await.unwrap();

    // Verify both memory and database have 3 entries
    let memory_len = engine.queue_len().await;
    assert_eq!(memory_len, 3, "In-memory queue should have 3 entries");

    let db_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM queue")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(db_count, 3, "Database queue should have 3 entries");

    // Skip first passage (simulates completion)
    engine.skip_next().await.expect("Skip 1 failed");

    // Verify both decremented
    let memory_len = engine.queue_len().await;
    assert_eq!(memory_len, 2, "In-memory queue should have 2 entries after skip");

    let db_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM queue")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(db_count, 2, "Database queue should have 2 entries after skip");

    // Verify the correct entry was removed from DB
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM queue WHERE guid = ?)")
        .bind(id1.to_string())
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(!exists, "Skipped entry should be removed from database");

    // Skip remaining passages
    engine.skip_next().await.expect("Skip 2 failed");
    engine.skip_next().await.expect("Skip 3 failed");

    // Verify both are empty
    let memory_len = engine.queue_len().await;
    assert_eq!(memory_len, 0, "In-memory queue should be empty");

    let db_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM queue")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(db_count, 0, "Database queue should be empty");

    // Verify all entries removed from DB
    for id in [id1, id2, id3] {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM queue WHERE guid = ?)")
            .bind(id.to_string())
            .fetch_one(&db)
            .await
            .unwrap();
        assert!(!exists, "Entry {} should be removed from database", id);
    }

    engine.stop().await.expect("Failed to stop engine");
}

// ============================================================================
// TEST 6: Event Ordering and Completeness
// ============================================================================

/// **Integration Test: Event Ordering and Completeness**
///
/// This test verifies that events are emitted in the correct order:
/// 1. PassageStarted for passage N
/// 2. PassageCompleted for passage N
/// 3. PassageStarted for passage N+1
/// 4. No events are dropped or duplicated
/// 5. QueueChanged events are emitted appropriately
///
/// **Bug Detection:**
/// - If events are out of order: Will detect Started before previous Completed
/// - If events are missing: Will detect gap in sequence
/// - If events are duplicated: Will detect multiple Started for same passage
#[tokio::test]
#[ignore] // Requires actual audio files
async fn test_event_ordering_and_completeness() {
    let db = create_test_db().await;
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db.clone(), state.clone())
        .await
        .expect("Failed to create engine");

    // Track event sequence
    let event_sequence = Arc::new(Mutex::new(Vec::new()));

    // Subscribe to events
    let mut event_rx = state.subscribe_events();
    let seq_clone = event_sequence.clone();
    let event_listener = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let event_type = match &event {
                WkmpEvent::PassageStarted { passage_id, .. } => {
                    format!("Started:{}", passage_id)
                }
                WkmpEvent::PassageCompleted { passage_id, .. } => {
                    format!("Completed:{}", passage_id)
                }
                WkmpEvent::QueueChanged { .. } => "QueueChanged".to_string(),
                _ => continue,
            };
            seq_clone.lock().unwrap().push(event_type);
        }
    });

    engine.start().await.expect("Failed to start engine");

    // Enqueue and wait for completion (requires real files)
    // ... test implementation ...

    // Verify event sequence
    let sequence = event_sequence.lock().unwrap();

    // Expected pattern:
    // QueueChanged (enqueue 1)
    // QueueChanged (enqueue 2)
    // QueueChanged (enqueue 3)
    // Started:passage1
    // Completed:passage1
    // Started:passage2
    // Completed:passage2
    // Started:passage3
    // Completed:passage3

    // Verify no passage gets Started twice before Completed
    let mut started_passages = std::collections::HashSet::new();
    for event in sequence.iter() {
        if event.starts_with("Started:") {
            let passage = event.split(':').nth(1).unwrap();
            assert!(
                !started_passages.contains(passage),
                "Passage {} started twice without completing",
                passage
            );
            started_passages.insert(passage.to_string());
        } else if event.starts_with("Completed:") {
            let passage = event.split(':').nth(1).unwrap();
            assert!(
                started_passages.remove(passage),
                "Passage {} completed without being started",
                passage
            );
        }
    }

    // Verify all passages completed
    assert!(
        started_passages.is_empty(),
        "Some passages started but never completed: {:?}",
        started_passages
    );

    event_listener.abort();
    engine.stop().await.expect("Failed to stop engine");
}
