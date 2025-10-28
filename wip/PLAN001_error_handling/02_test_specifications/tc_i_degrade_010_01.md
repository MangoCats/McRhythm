# TC-I-DEGRADE-010-01: Queue Integrity After Decode Error

**Test ID:** TC-I-DEGRADE-010-01
**Test Type:** Integration Test
**Requirement:** REQ-AP-DEGRADE-010 (Preserve queue integrity under all error conditions)
**Specification:** SPEC021 ERH-DEGRADE-010, ERH-DEGRADE-020
**Priority:** High
**Estimated Effort:** 25 minutes

---

## Test Objective

Verify that when decode errors occur, queue integrity is preserved:
1. Queue order maintained (remaining passages not reordered)
2. Failed passage removed from queue
3. Next passage loaded automatically
4. No queue corruption or data loss
5. Queue database reflects correct state

---

## Scope

**Components Under Test:**
- Queue Manager (queue integrity)
- Engine (error recovery)
- Database (queue persistence)

**Integration Points:**
- Engine → Queue Manager (passage removal)
- Queue Manager → Database (state persistence)
- Engine → Decoder (next passage load)

---

## Test Setup

**Given:**
```rust
// Set up test database and engine
let db_pool = setup_test_database().await;
let engine = Arc::new(Engine::new(db_pool.clone()));

// Enqueue 5 passages: 1 valid, 1 invalid, 3 valid
let passage1_id = enqueue_valid_passage(&engine, "track1.mp3").await;  // Valid
let passage2_id = enqueue_invalid_passage(&engine, "/nonexistent/track2.mp3").await;  // INVALID
let passage3_id = enqueue_valid_passage(&engine, "track3.mp3").await;  // Valid
let passage4_id = enqueue_valid_passage(&engine, "track4.mp3").await;  // Valid
let passage5_id = enqueue_valid_passage(&engine, "track5.mp3").await;  // Valid

// Record original queue order
let original_queue = engine.get_queue().await;
assert_eq!(original_queue.len(), 5, "Queue should have 5 passages");
assert_eq!(original_queue[0].passage_id, Some(passage1_id));
assert_eq!(original_queue[1].passage_id, Some(passage2_id));
assert_eq!(original_queue[2].passage_id, Some(passage3_id));
assert_eq!(original_queue[3].passage_id, Some(passage4_id));
assert_eq!(original_queue[4].passage_id, Some(passage5_id));

// Start playback
engine.play().await.unwrap();
```

---

## Test Execution

**When:**
```rust
// Wait for passage1 to start playing
wait_for_passage_start(&engine, passage1_id).await;

// Skip to passage2 (which will fail to decode)
engine.skip_forward().await.unwrap();

// Wait for decode error and automatic skip to passage3
tokio::time::sleep(Duration::from_secs(2)).await;
```

---

## Test Verification

**Then:**

### 1. Failed Passage Removed from Queue
```rust
let queue_after_error = engine.get_queue().await;

// passage2 should be gone
assert!(
    !queue_after_error.iter().any(|e| e.passage_id == Some(passage2_id)),
    "Failed passage should be removed from queue"
);
```

### 2. Queue Order Preserved
```rust
// Remaining passages should still be in original order
assert_eq!(queue_after_error.len(), 4, "Queue should have 4 passages (1 removed)");

// passage3, passage4, passage5 should still be in order
let remaining_ids: Vec<Uuid> = queue_after_error.iter()
    .filter_map(|e| e.passage_id)
    .collect();

assert_eq!(remaining_ids[0], passage3_id, "passage3 should be first");
assert_eq!(remaining_ids[1], passage4_id, "passage4 should be second");
assert_eq!(remaining_ids[2], passage5_id, "passage5 should be third");
```

### 3. Next Passage Loaded Automatically
```rust
// passage3 should now be playing
let current_passage = engine.get_current_passage().await;
assert_eq!(
    current_passage.unwrap().passage_id,
    Some(passage3_id),
    "passage3 should be playing after passage2 failed"
);
```

### 4. Database Queue State Correct
```rust
// Verify database matches in-memory queue
let db_queue = sqlx::query_as::<_, QueueEntry>(
    "SELECT * FROM queue ORDER BY play_order"
)
.fetch_all(&db_pool)
.await
.unwrap();

assert_eq!(db_queue.len(), 4, "Database should have 4 queue entries");

// Verify no orphaned passages
let db_passage_ids: Vec<Uuid> = db_queue.iter()
    .filter_map(|e| e.passage_id)
    .collect();

assert!(!db_passage_ids.contains(&passage2_id), "Failed passage should be removed from DB");
assert!(db_passage_ids.contains(&passage3_id), "passage3 should be in DB");
assert!(db_passage_ids.contains(&passage4_id), "passage4 should be in DB");
assert!(db_passage_ids.contains(&passage5_id), "passage5 should be in DB");
```

### 5. No Queue Corruption
```rust
// Verify all queue entries are valid
for entry in &queue_after_error {
    assert!(entry.queue_entry_id != Uuid::nil(), "Queue entry ID should be valid");
    assert!(entry.passage_id.is_some(), "Passage ID should be present");
    assert!(entry.play_order >= 0, "Play order should be non-negative");
}

// Verify no duplicate play_order values
let play_orders: Vec<i64> = queue_after_error.iter()
    .map(|e| e.play_order)
    .collect();
let unique_orders: std::collections::HashSet<_> = play_orders.iter().collect();
assert_eq!(
    play_orders.len(),
    unique_orders.len(),
    "Play orders should be unique (no duplicates)"
);
```

---

## Pass Criteria

**Test passes if ALL of the following are true:**
- ✓ Failed passage removed from queue
- ✓ Remaining passages maintain original order
- ✓ Next passage (passage3) loaded and playing automatically
- ✓ Database queue matches in-memory queue
- ✓ No orphaned or duplicate queue entries
- ✓ All queue entries have valid IDs and play orders
- ✓ No queue corruption detected

---

## Fail Criteria

**Test fails if ANY of the following occur:**
- ✗ Failed passage remains in queue
- ✗ Queue order changed (passages reordered)
- ✗ Next passage did not load
- ✗ Wrong passage is playing
- ✗ Database queue does not match in-memory queue
- ✗ Orphaned passages in database
- ✗ Duplicate play orders
- ✗ Invalid queue entry IDs
- ✗ Queue corruption (nil UUIDs, negative play orders)

---

## Edge Cases

**Also test:**
- First passage in queue fails
- Last passage in queue fails
- All passages fail (empty queue result)
- Multiple consecutive failures

---

**Test Status:** Defined
**Implementation Status:** Pending
