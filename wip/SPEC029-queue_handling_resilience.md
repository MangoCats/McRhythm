# SPEC029: Queue Handling Resilience Improvements

**Status:** Draft for /plan workflow
**Version:** 1.0
**Date:** 2025-11-06
**Related Documents:**
- [Queue Handling Analysis](queue_handling_mechanism_analysis.md) - Root cause analysis
- [SPEC028: Playback Orchestration](../docs/SPEC028-playback_orchestration.md) - Event-driven architecture
- [SPEC016: Decoder Buffer Design](../docs/SPEC016-decoder_buffer_design.md) - Chain lifecycle

---

## 1. Executive Summary

This specification addresses three reliability gaps identified in the queue handling mechanism:
1. **Non-idempotent removal operations** causing duplicate event failures
2. **Multiple PassageComplete event sources** triggering race conditions
3. **Resource cleanup chain optimization** to ensure DRY principles

**Approach:** Implement idempotent operations with event deduplication using test-driven development (TDD) and DRY code organization to minimize redundancy.

**Success Criteria:**
- Zero ERROR logs from duplicate PassageComplete events
- All queue operations idempotent (safe to retry)
- Reduced code duplication in cleanup paths
- 100% test coverage for edge cases (duplicate events, missing entries)

---

## 2. Problem Statement

### 2.1 Gap Analysis Summary

**From:** [queue_handling_mechanism_analysis.md](queue_handling_mechanism_analysis.md)

**Gap 1: Non-Idempotent Database Removal**
```rust
// Current: db/queue.rs:162-176
pub async fn remove_from_queue(db: &Pool<Sqlite>, queue_entry_id: Uuid) -> Result<()> {
    let result = sqlx::query("DELETE FROM queue WHERE guid = ?")
        .bind(queue_entry_id.to_string())
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(Error::Queue(format!(
            "Queue entry not found: {}",
            queue_entry_id
        )));  // ❌ Errors on second call (already deleted)
    }

    Ok(())
}
```

**Issue:** Second removal attempt fails with ERROR log, not graceful no-op.

**Gap 2: Multiple Event Sources**

PassageComplete events emitted from:
1. `MarkerEvent::PassageComplete` (normal completion)
2. `MarkerEvent::EndOfFile` (unexpected EOF)
3. `MarkerEvent::EndOfFileBeforeLeadOut` (early EOF before crossfade)

**Issue:** All three send identical `PlaybackEvent::PassageComplete` with no deduplication.

**Gap 3: Code Duplication in Cleanup**

Cleanup logic duplicated across:
- `skip_next()` ([queue.rs:84-97](../wkmp-ap/src/playback/engine/queue.rs#L84-L97))
- `remove_queue_entry()` ([queue.rs:351-362](../wkmp-ap/src/playback/engine/queue.rs#L351-L362))
- `clear_queue()` ([queue.rs:142-170](../wkmp-ap/src/playback/engine/queue.rs#L142-L170))

**Issue:** 5-7 cleanup steps repeated verbatim across methods (violates DRY).

### 2.2 Impact Assessment

**Current Severity:** Low (cosmetic errors, system recovers)

**Risks if Unaddressed:**
- Log noise obscures real errors
- Telemetry pollution (false positives)
- Maintenance burden (duplicate logic divergence)
- Potential race conditions under high load

---

## 3. Solution Approach

### 3.1 Risk Assessment

**APPROACH 1: Idempotent Operations + Event Deduplication + DRY Refactoring**

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. Idempotency logic bugs (incorrect removal detection) - Probability: Low - Impact: Medium
     - Mitigation: Comprehensive unit tests for all edge cases (already removed, never existed, concurrent removal)
  2. Deduplication state management errors (memory leak, stale entries) - Probability: Low - Impact: Low
     - Mitigation: Time-bounded cache with automatic cleanup, integration tests for long-running scenarios
  3. Refactoring introduces behavioral regression - Probability: Low - Impact: High
     - Mitigation: Test-first approach, maintain existing test suite passing, add new edge case tests
- **Residual Risk After Mitigation:** Low

**Quality Characteristics:**
- **Maintainability:** High (DRY refactoring reduces duplication, clearer intent)
- **Test Coverage:** High (TDD approach ensures comprehensive test suite)
- **Architectural Alignment:** Strong (follows existing event-driven patterns, no new components)

**Implementation Considerations:**
- **Effort:** Medium (3-5 hours: design 0.5h, implement 1.5h, test 1.5h, review 0.5h)
- **Dependencies:** None (internal refactoring only)
- **Complexity:** Medium (careful state management for deduplication)

**APPROACH 2: Event Source Consolidation Only** (Alternative - Not Recommended)

**Risk Assessment:**
- **Failure Risk:** Medium
- **Failure Modes:**
  1. EOF detection logic error (miss completion events) - Probability: Medium - Impact: High
     - Mitigation: Extensive testing of EOF edge cases
  2. Marker unreachability detection bugs - Probability: Medium - Impact: Medium
     - Mitigation: Complex logic to track which events were already emitted
- **Residual Risk:** Low-Medium

**Quality Characteristics:**
- **Maintainability:** Medium (complex conditional logic for event emission)
- **Test Coverage:** Medium (difficult to test all EOF scenarios)
- **Architectural Alignment:** Moderate (changes event emission patterns)

**Implementation Considerations:**
- **Effort:** Medium (same as Approach 1)
- **Dependencies:** Deep mixer/marker changes
- **Complexity:** High (complex conditional event emission logic)

**RISK-BASED RANKING:**
1. **Approach 1 (Idempotent + Dedup + DRY)** - Lowest residual risk (Low)
2. Approach 2 (Event consolidation) - Higher risk (Low-Medium)

**RECOMMENDATION:**
Choose **Approach 1** due to lowest failure risk and highest maintainability. Idempotent operations are a standard distributed systems pattern, well-understood and testable. Event deduplication is a simple time-bounded cache. DRY refactoring reduces long-term maintenance risk.

---

## 4. Detailed Design

### 4.1 Idempotent Database Operations

#### 4.1.1 Specification

**[REQ-QUEUE-IDEMP-010] Idempotent Queue Removal**

Database removal operations MUST be idempotent:
- First call: Remove entry if exists, return `Ok(true)`
- Subsequent calls: Return `Ok(false)` (not error)
- Never error on missing entry (already removed is success case)

**[REQ-QUEUE-IDEMP-020] Return Value Semantics**

`remove_from_queue()` MUST return `Result<bool>`:
- `Ok(true)`: Entry was removed by this call
- `Ok(false)`: Entry not found (idempotent no-op)
- `Err(_)`: Database error (connection failure, etc.)

#### 4.1.2 Implementation Pattern

**File:** `wkmp-ap/src/db/queue.rs`

**Current signature:**
```rust
pub async fn remove_from_queue(db: &Pool<Sqlite>, queue_entry_id: Uuid) -> Result<()>
```

**New signature:**
```rust
pub async fn remove_from_queue(db: &Pool<Sqlite>, queue_entry_id: Uuid) -> Result<bool>
```

**Implementation logic:**
```rust
pub async fn remove_from_queue(db: &Pool<Sqlite>, queue_entry_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM queue WHERE guid = ?")
        .bind(queue_entry_id.to_string())
        .execute(db)
        .await?;

    // Idempotent: Return true if deleted, false if already missing
    Ok(result.rows_affected() > 0)
}
```

**Error handling change:**
```rust
// BEFORE: Error if not found
if result.rows_affected() == 0 {
    return Err(Error::Queue(format!("Queue entry not found: {}", queue_entry_id)));
}

// AFTER: No-op if not found (idempotent)
Ok(result.rows_affected() > 0)
```

#### 4.1.3 Caller Updates

**Update `complete_passage_removal()` to handle idempotent result:**

**File:** `wkmp-ap/src/playback/engine/queue.rs:556-637`

**Current:**
```rust
// Remove from database FIRST (persistence before memory)
if let Err(e) = crate::db::queue::remove_from_queue(&self.db_pool, queue_entry_id).await {
    error!("Failed to remove entry from database: {}", e);
}
```

**Updated:**
```rust
// Remove from database FIRST (persistence before memory)
// [REQ-QUEUE-IDEMP-010] Idempotent removal - false = already removed
let db_removed = match crate::db::queue::remove_from_queue(&self.db_pool, queue_entry_id).await {
    Ok(removed) => removed,
    Err(e) => {
        error!("Database error removing entry: {}", e);
        false  // Assume not removed on DB error, continue with memory cleanup
    }
};
```

**Add early return for duplicate removal:**
```rust
// Remove from in-memory queue
let mem_removed = {
    let mut queue_write = self.queue.write().await;
    queue_write.remove(queue_entry_id)
};

// [REQ-QUEUE-IDEMP-020] Detect duplicate removal (idempotent no-op)
if !db_removed && !mem_removed {
    debug!("Queue entry {} already removed (duplicate event or prior removal)", queue_entry_id);
    return Ok(false);  // Idempotent success
}
```

### 4.2 Event Deduplication

#### 4.2.1 Specification

**[REQ-QUEUE-DEDUP-010] PassageComplete Deduplication**

The system MUST deduplicate PassageComplete events:
- Only process first event for each `queue_entry_id`
- Track processed entries for 5 seconds (safety margin)
- Automatically cleanup stale entries (prevent memory leak)

**[REQ-QUEUE-DEDUP-020] Deduplication Scope**

Deduplication applies to:
- `PlaybackEvent::PassageComplete` only
- Per `queue_entry_id` (not passage_id)
- Time window: 5 seconds (longer than any passage transition)

**[REQ-QUEUE-DEDUP-030] Thread Safety**

Deduplication state MUST be thread-safe:
- Use `Arc<RwLock<HashMap<Uuid, Instant>>>` for shared state
- Read lock for checks (concurrent safe)
- Write lock for insertions/removals (exclusive)

#### 4.2.2 Implementation Pattern

**Add field to PlaybackEngine:**

**File:** `wkmp-ap/src/playback/engine/core.rs`

```rust
pub struct PlaybackEngine {
    // ... existing fields ...

    /// Recent PassageComplete events for deduplication
    /// [REQ-QUEUE-DEDUP-010] Tracks processed queue_entry_ids for 5 seconds
    completed_passages: Arc<RwLock<HashMap<Uuid, Instant>>>,
}
```

**Initialize in constructor:**
```rust
completed_passages: Arc::new(RwLock::new(HashMap::new())),
```

**Deduplication logic in diagnostics handler:**

**File:** `wkmp-ap/src/playback/engine/diagnostics.rs:575-598`

**Current:**
```rust
Some(PlaybackEvent::PassageComplete { queue_entry_id }) => {
    info!("PassageComplete event received for queue_entry_id: {}", queue_entry_id);

    // ... existing removal logic ...
}
```

**Updated:**
```rust
Some(PlaybackEvent::PassageComplete { queue_entry_id }) => {
    // [REQ-QUEUE-DEDUP-010] Check if already processed (deduplication)
    {
        let completed = self.completed_passages.read().await;
        if let Some(timestamp) = completed.get(&queue_entry_id) {
            let age_ms = timestamp.elapsed().as_millis();
            debug!(
                "Ignoring duplicate PassageComplete for {} (processed {}ms ago)",
                queue_entry_id, age_ms
            );
            continue;  // Skip duplicate event
        }
    }

    info!("PassageComplete event received for queue_entry_id: {}", queue_entry_id);

    // [REQ-QUEUE-DEDUP-010] Mark as processed
    {
        let mut completed = self.completed_passages.write().await;
        completed.insert(queue_entry_id, Instant::now());
    }

    // ... existing removal logic ...

    // [REQ-QUEUE-DEDUP-010] Schedule cleanup after 5 seconds
    let completed_clone = self.completed_passages.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let mut completed = completed_clone.write().await;
        completed.remove(&queue_entry_id);
    });
}
```

#### 4.2.3 Alternative: Cleanup Task

**Optional enhancement:** Background cleanup task instead of per-event spawn.

**Add periodic cleanup method:**
```rust
/// Cleanup stale deduplication entries (older than 5 seconds)
/// [REQ-QUEUE-DEDUP-010] Prevents memory leak from unbounded HashMap growth
async fn cleanup_completed_passages(&self) {
    let mut completed = self.completed_passages.write().await;
    let now = Instant::now();
    completed.retain(|_id, timestamp| {
        now.duration_since(*timestamp) < Duration::from_secs(5)
    });
}
```

**Spawn cleanup task in engine initialization:**
```rust
// Spawn background cleanup task (runs every 10 seconds)
let engine_clone = self.clone();
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        engine_clone.cleanup_completed_passages().await;
    }
});
```

**Trade-off:** Per-event spawn is simpler (chosen approach), background task is more efficient (alternative if profiling shows overhead).

### 4.3 DRY Cleanup Refactoring

#### 4.3.1 Specification

**[REQ-QUEUE-DRY-010] Single Cleanup Implementation**

Resource cleanup logic MUST be implemented once:
- Single helper method for all cleanup operations
- All callers (skip, remove, clear) use helper
- Helper handles all cleanup steps in correct order

**[REQ-QUEUE-DRY-020] Cleanup Operation Ordering**

Cleanup helper MUST execute steps in this order:
1. Release decoder-buffer chain (free resources first)
2. Stop mixer (clear audio output state)
3. Remove from queue (database + memory)
4. Release buffer (free decoded audio memory)
5. Emit events (notify subscribers)
6. Try chain reassignment (allocate freed chain to waiting passages)

**Rationale:** Order ensures resources freed before reassignment, persistence before memory updates.

#### 4.3.2 Implementation Pattern

**Create new helper method:**

**File:** `wkmp-ap/src/playback/engine/queue.rs`

**Add to PlaybackEngine impl:**
```rust
/// Perform complete resource cleanup for a queue entry
///
/// [REQ-QUEUE-DRY-010] Single implementation of cleanup logic
/// [REQ-QUEUE-DRY-020] Executes cleanup steps in correct order
///
/// # Steps (in order):
/// 1. Release decoder-buffer chain [DBD-LIFECYCLE-020]
/// 2. Stop mixer (if entry is current) [SSD-MIX-030]
/// 3. Remove from queue [REQ-QUEUE-IDEMP-010]
/// 4. Release buffer memory
/// 5. Emit events
/// 6. Try chain reassignment
///
/// # Arguments
/// * `queue_entry_id` - Entry to clean up
/// * `trigger` - Event trigger (for event emission)
/// * `stop_mixer` - Whether to stop mixer (true for current, false for non-current)
///
/// # Returns
/// * `Ok(true)` - Entry cleaned up successfully
/// * `Ok(false)` - Entry already cleaned up (idempotent)
/// * `Err(_)` - Cleanup failed (rare)
async fn cleanup_queue_entry(
    &self,
    queue_entry_id: Uuid,
    trigger: wkmp_common::events::QueueChangeTrigger,
    stop_mixer: bool,
) -> Result<bool> {
    // Capture queue state BEFORE cleanup (for promotion detection)
    let (next_before, queued_first_before) = {
        let queue = self.queue.read().await;
        (
            queue.next().map(|e| e.queue_entry_id),
            queue.queued().first().map(|e| e.queue_entry_id),
        )
    };

    // Step 1: Release decoder-buffer chain [DBD-LIFECYCLE-020]
    self.release_chain(queue_entry_id).await;

    // Step 2: Stop mixer if this is current passage
    if stop_mixer {
        let mut mixer = self.mixer.write().await;
        mixer.clear_all_markers();
        mixer.clear_passage();
        debug!("Mixer stopped for queue_entry_id: {}", queue_entry_id);
    }

    // Step 3: Remove from queue (database + memory) [REQ-QUEUE-IDEMP-010]
    let db_removed = match crate::db::queue::remove_from_queue(&self.db_pool, queue_entry_id).await {
        Ok(removed) => removed,
        Err(e) => {
            error!("Database error removing entry: {}", e);
            false
        }
    };

    let mem_removed = {
        let mut queue_write = self.queue.write().await;
        queue_write.remove(queue_entry_id)
    };

    // [REQ-QUEUE-IDEMP-020] Idempotent check
    if !db_removed && !mem_removed {
        debug!("Queue entry {} already cleaned up", queue_entry_id);
        return Ok(false);
    }

    info!("Cleaned up queue entry: {} (db={}, mem={})", queue_entry_id, db_removed, mem_removed);

    // Step 4: Release buffer memory
    // (Note: buffer_manager uses queue_entry_id as key, not passage_id)
    // This is a no-op if buffer already released, which is OK (idempotent)

    // Step 5: Emit events
    self.update_audio_expected_flag().await;

    let queue_snapshot = {
        let queue = self.queue.read().await;
        queue.get_all_entries()
    };

    self.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueChanged {
        entries: queue_snapshot,
        trigger,
        timestamp: chrono::Utc::now(),
    });

    // Step 6: Detect promotions and trigger decode [PLAN020 FR-001]
    {
        let queue = self.queue.read().await;
        let current_after = queue.current().map(|e| e.clone());
        let next_after = queue.next().map(|e| e.clone());

        // If next_before became current_after, it was promoted
        if let (Some(promoted_id), Some(promoted_entry)) = (next_before, current_after) {
            if promoted_entry.queue_entry_id == promoted_id {
                debug!("Detected promotion: next→current ({})", promoted_id);
                if let Err(e) = self.request_decode(&promoted_entry, DecodePriority::Immediate, true).await {
                    error!("Failed to trigger decode for promoted current: {}", e);
                }
            }
        }

        // If queued_first_before became next_after, it was promoted
        if let (Some(promoted_id), Some(promoted_entry)) = (queued_first_before, next_after) {
            if promoted_entry.queue_entry_id == promoted_id {
                debug!("Detected promotion: queued→next ({})", promoted_id);
                if let Err(e) = self.request_decode(&promoted_entry, DecodePriority::Next, true).await {
                    error!("Failed to trigger decode for promoted next: {}", e);
                }
            }
        }
    }

    // Step 7: Try to assign freed chain to unassigned entries
    self.assign_chains_to_unassigned_entries().await;

    Ok(true)
}
```

#### 4.3.3 Refactor Existing Methods

**Update `skip_next()` to use helper:**

**File:** `wkmp-ap/src/playback/engine/queue.rs:32-125`

**Current (lines 84-122):**
```rust
// Mark buffer as exhausted
self.buffer_manager.mark_exhausted(current.queue_entry_id).await;

// Stop mixer immediately
let mut mixer = self.mixer.write().await;
mixer.clear_all_markers();
mixer.clear_passage();
drop(mixer);

// Remove buffer from memory
if let Some(passage_id) = current.passage_id {
    self.buffer_manager.remove(passage_id).await;
}

// Release chain
self.release_chain(current.queue_entry_id).await;

// Remove from queue
if let Err(e) = self.complete_passage_removal(...).await {
    error!("Failed to complete passage removal: {}", e);
}

// Assign chains
self.assign_chains_to_unassigned_entries().await;

// Start next
if let Err(e) = self.watchdog_check().await {
    error!("Failed to run watchdog check after skip: {}", e);
}
```

**Refactored:**
```rust
// Mark buffer as exhausted
self.buffer_manager.mark_exhausted(current.queue_entry_id).await;

// [REQ-QUEUE-DRY-010] Use cleanup helper
if let Err(e) = self.cleanup_queue_entry(
    current.queue_entry_id,
    wkmp_common::events::QueueChangeTrigger::UserDequeue,
    true,  // stop_mixer=true (current passage)
).await {
    error!("Failed to cleanup queue entry: {}", e);
}

// Start next passage
if let Err(e) = self.watchdog_check().await {
    error!("Failed to run watchdog check after skip: {}", e);
}
```

**Lines reduced:** 38 → 12 (68% reduction)

**Update `remove_queue_entry()` to use helper:**

**File:** `wkmp-ap/src/playback/engine/queue.rs:281-427`

**Current (lines 351-395):**
```rust
// Release chain
self.release_chain(queue_entry_id).await;

// Stop mixer
{
    let mut mixer = self.mixer.write().await;
    mixer.clear_all_markers();
    mixer.clear_passage();
}

// Remove from queue
let removed = match self.complete_passage_removal(...).await {
    Ok(result) => result,
    Err(e) => {
        error!("Failed to complete passage removal: {}", e);
        false
    }
};

if removed {
    self.assign_chains_to_unassigned_entries().await;
    let has_current = self.queue.read().await.current().is_some();
    if has_current {
        if let Err(e) = self.watchdog_check().await {
            warn!("Failed to start next passage after removal: {}", e);
        }
    }
}
```

**Refactored:**
```rust
// [REQ-QUEUE-DRY-010] Use cleanup helper
let removed = match self.cleanup_queue_entry(
    queue_entry_id,
    wkmp_common::events::QueueChangeTrigger::UserDequeue,
    is_current,  // stop_mixer=true only if current
).await {
    Ok(result) => result,
    Err(e) => {
        error!("Failed to cleanup queue entry: {}", e);
        false
    }
};

if removed {
    // Start next passage if queue has one
    let has_current = self.queue.read().await.current().is_some();
    if has_current {
        if let Err(e) = self.watchdog_check().await {
            warn!("Failed to start next passage after removal: {}", e);
        }
    }
}
```

**Lines reduced:** 45 → 20 (56% reduction)

**Update `complete_passage_removal()` to use helper:**

**File:** `wkmp-ap/src/playback/engine/queue.rs:556-637`

**This method becomes the `cleanup_queue_entry()` helper (renamed and generalized).**

**Alternative:** Keep `complete_passage_removal()` as thin wrapper:
```rust
pub(super) async fn complete_passage_removal(
    &self,
    queue_entry_id: Uuid,
    trigger: wkmp_common::events::QueueChangeTrigger,
) -> Result<bool> {
    // [REQ-QUEUE-DRY-010] Delegate to cleanup helper
    self.cleanup_queue_entry(queue_entry_id, trigger, false).await
}
```

**Rationale:** Maintains backward compatibility with existing callers (diagnostics handler).

---

## 5. Test Specifications

### 5.1 Test Strategy

**Approach:** Test-Driven Development (TDD)
1. Write failing tests for each requirement
2. Implement minimum code to pass tests
3. Refactor while maintaining passing tests
4. Verify edge cases with additional tests

**Test Levels:**
- **Unit Tests:** Database operations, deduplication logic, helper methods
- **Integration Tests:** Complete removal flow, event emission, cleanup ordering
- **System Tests:** Duplicate event handling in real playback scenarios

### 5.2 Unit Tests

#### 5.2.1 Idempotent Database Operations

**Test File:** `wkmp-ap/src/db/queue.rs` (inline `#[cfg(test)]` module)

**TC-U-IDEMP-001: First removal succeeds**
```rust
#[tokio::test]
async fn test_remove_queue_entry_first_call_succeeds() {
    // Given: Entry exists in database
    let db = create_test_db().await;
    let entry_id = enqueue_test_entry(&db).await;

    // When: remove_from_queue called
    let result = remove_from_queue(&db, entry_id).await;

    // Then: Returns Ok(true)
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);

    // And: Entry no longer in database
    let remaining = get_queue(&db).await.unwrap();
    assert_eq!(remaining.len(), 0);
}
```

**TC-U-IDEMP-002: Second removal returns false (idempotent)**
```rust
#[tokio::test]
async fn test_remove_queue_entry_second_call_idempotent() {
    // Given: Entry already removed
    let db = create_test_db().await;
    let entry_id = enqueue_test_entry(&db).await;
    remove_from_queue(&db, entry_id).await.unwrap();

    // When: remove_from_queue called again
    let result = remove_from_queue(&db, entry_id).await;

    // Then: Returns Ok(false) (not error)
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}
```

**TC-U-IDEMP-003: Remove non-existent entry**
```rust
#[tokio::test]
async fn test_remove_queue_entry_never_existed() {
    // Given: Entry never existed
    let db = create_test_db().await;
    let fake_id = Uuid::new_v4();

    // When: remove_from_queue called
    let result = remove_from_queue(&db, fake_id).await;

    // Then: Returns Ok(false) (idempotent no-op)
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}
```

#### 5.2.2 Event Deduplication

**Test File:** `wkmp-ap/tests/queue_deduplication_tests.rs` (new file)

**TC-U-DEDUP-001: First event processed**
```rust
#[tokio::test]
async fn test_passage_complete_first_event_processed() {
    // Given: Engine with deduplication enabled
    let engine = create_test_engine().await;
    let entry_id = enqueue_test_passage(&engine).await;

    // When: PassageComplete event emitted
    send_passage_complete_event(&engine, entry_id).await;

    // Then: Event processed (entry removed from queue)
    tokio::time::sleep(Duration::from_millis(50)).await;
    let queue_len = engine.queue_len().await.unwrap();
    assert_eq!(queue_len, 0);
}
```

**TC-U-DEDUP-002: Duplicate event ignored**
```rust
#[tokio::test]
async fn test_passage_complete_duplicate_event_ignored() {
    // Given: First event already processed
    let engine = create_test_engine().await;
    let entry_id = enqueue_test_passage(&engine).await;
    send_passage_complete_event(&engine, entry_id).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // When: Duplicate event sent (within 5 seconds)
    send_passage_complete_event(&engine, entry_id).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Then: No ERROR logs (deduplication worked)
    // And: Entry already removed (queue still empty)
    let queue_len = engine.queue_len().await.unwrap();
    assert_eq!(queue_len, 0);

    // And: Debug log shows duplicate detected
    assert_log_contains("Ignoring duplicate PassageComplete");
}
```

**TC-U-DEDUP-003: Multiple distinct events processed**
```rust
#[tokio::test]
async fn test_passage_complete_multiple_distinct_events() {
    // Given: Two different queue entries
    let engine = create_test_engine().await;
    let entry1_id = enqueue_test_passage(&engine).await;
    let entry2_id = enqueue_test_passage(&engine).await;

    // When: PassageComplete for both entries
    send_passage_complete_event(&engine, entry1_id).await;
    send_passage_complete_event(&engine, entry2_id).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Then: Both processed (queue empty)
    let queue_len = engine.queue_len().await.unwrap();
    assert_eq!(queue_len, 0);
}
```

**TC-U-DEDUP-004: Stale entry cleanup**
```rust
#[tokio::test]
async fn test_deduplication_entry_cleanup_after_5_seconds() {
    // Given: Event processed
    let engine = create_test_engine().await;
    let entry_id = enqueue_test_passage(&engine).await;
    send_passage_complete_event(&engine, entry_id).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // When: 5 seconds elapse
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Then: Deduplication entry cleaned up (memory not leaked)
    let dedup_size = engine.completed_passages.read().await.len();
    assert_eq!(dedup_size, 0);
}
```

#### 5.2.3 DRY Cleanup Helper

**Test File:** `wkmp-ap/tests/cleanup_helper_tests.rs` (new file)

**TC-U-DRY-001: Cleanup helper order verification**
```rust
#[tokio::test]
async fn test_cleanup_helper_step_order() {
    // Given: Mock components tracking call order
    let (engine, spy) = create_test_engine_with_spy().await;
    let entry_id = enqueue_test_passage(&engine).await;

    // When: cleanup_queue_entry called
    engine.cleanup_queue_entry(entry_id, QueueChangeTrigger::UserDequeue, true).await.unwrap();

    // Then: Steps executed in correct order
    let call_order = spy.get_call_order().await;
    assert_eq!(call_order, vec![
        "release_chain",
        "clear_mixer",
        "db_remove",
        "mem_remove",
        "emit_event",
        "assign_chains",
    ]);
}
```

**TC-U-DRY-002: Cleanup helper idempotent**
```rust
#[tokio::test]
async fn test_cleanup_helper_idempotent() {
    // Given: Entry already cleaned up
    let engine = create_test_engine().await;
    let entry_id = enqueue_test_passage(&engine).await;
    engine.cleanup_queue_entry(entry_id, QueueChangeTrigger::UserDequeue, true).await.unwrap();

    // When: cleanup_queue_entry called again
    let result = engine.cleanup_queue_entry(entry_id, QueueChangeTrigger::UserDequeue, true).await;

    // Then: Returns Ok(false) (idempotent)
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}
```

**TC-U-DRY-003: Skip uses cleanup helper**
```rust
#[tokio::test]
async fn test_skip_next_uses_cleanup_helper() {
    // Given: Passage playing
    let engine = create_test_engine().await;
    let entry_id = enqueue_and_start_passage(&engine).await;

    // When: skip_next called
    engine.skip_next().await.unwrap();

    // Then: Cleanup helper invoked (verified via spy)
    let spy = engine.get_cleanup_spy().await;
    assert_eq!(spy.call_count(), 1);
    assert_eq!(spy.last_call_args().stop_mixer, true);
}
```

### 5.3 Integration Tests

#### 5.3.1 Complete Removal Flow

**Test File:** `wkmp-ap/tests/queue_removal_integration_tests.rs` (new file)

**TC-I-REMOVAL-001: Normal passage completion**
```rust
#[tokio::test]
async fn test_passage_completion_cleanup_chain() {
    // Given: Passage playing
    let engine = create_real_engine().await;
    let file_path = create_short_audio_file();  // 1 second duration
    let entry_id = engine.enqueue_file(&file_path).await.unwrap();
    engine.start().await.unwrap();

    // When: Passage completes naturally
    tokio::time::sleep(Duration::from_secs(2)).await;  // Wait for completion

    // Then: Complete cleanup chain executed
    assert!(queue_is_empty(&engine).await);
    assert!(mixer_is_stopped(&engine).await);
    assert!(chain_is_released(&engine, entry_id).await);
    assert!(buffer_is_freed(&engine, entry_id).await);
}
```

**TC-I-REMOVAL-002: Duplicate event during completion**
```rust
#[tokio::test]
async fn test_passage_completion_with_duplicate_event() {
    // Given: Passage approaching completion
    let engine = create_real_engine().await;
    let file_path = create_short_audio_file_with_early_eof();  // EOF before marker
    let entry_id = engine.enqueue_file(&file_path).await.unwrap();
    engine.start().await.unwrap();

    // When: Passage completes (both PassageComplete and EOF events fire)
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Then: No ERROR logs (deduplication worked)
    assert_no_error_logs();

    // And: Cleanup happened exactly once
    let queue_len = engine.queue_len().await.unwrap();
    assert_eq!(queue_len, 0);
}
```

#### 5.3.2 Queue Advancement

**TC-I-ADV-001: Promotion triggers decode**
```rust
#[tokio::test]
async fn test_removal_triggers_decode_for_promoted_passages() {
    // Given: Three passages in queue (current, next, queued)
    let engine = create_real_engine_with_spy().await;
    let entry1 = engine.enqueue_file(&create_test_audio()).await.unwrap();
    let entry2 = engine.enqueue_file(&create_test_audio()).await.unwrap();
    let entry3 = engine.enqueue_file(&create_test_audio()).await.unwrap();
    engine.start().await.unwrap();

    // When: Current passage completes
    send_passage_complete_event(&engine, entry1).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Then: Decode triggered for entry2 (next→current) with Immediate priority
    let spy = engine.get_decode_spy().await;
    spy.verify_decode_request(entry2, DecodePriority::Immediate, 50).await.unwrap();

    // And: Decode triggered for entry3 (queued→next) with Next priority
    spy.verify_decode_request(entry3, DecodePriority::Next, 50).await.unwrap();
}
```

### 5.4 System Tests

#### 5.4.1 Real-World Scenarios

**Test File:** `wkmp-ap/tests/system_queue_resilience_tests.rs` (new file)

**TC-S-RESIL-001: Rapid skip operations**
```rust
#[tokio::test]
async fn test_rapid_skip_operations_no_errors() {
    // Given: 10 passages in queue
    let engine = create_real_engine().await;
    for _ in 0..10 {
        engine.enqueue_file(&create_test_audio()).await.unwrap();
    }
    engine.start().await.unwrap();

    // When: Rapid skip operations (faster than dedup timeout)
    for _ in 0..9 {
        engine.skip_next().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;  // Faster than 5s dedup
    }

    // Then: No ERROR logs
    assert_no_error_logs();

    // And: Queue advanced correctly
    let queue_len = engine.queue_len().await.unwrap();
    assert_eq!(queue_len, 1);  // Last passage remains
}
```

**TC-S-RESIL-002: EOF before crossfade**
```rust
#[tokio::test]
async fn test_eof_before_crossfade_no_duplicate_errors() {
    // Given: Passage with unexpected early EOF
    let engine = create_real_engine().await;
    let file = create_audio_file_with_truncated_end();
    engine.enqueue_file(&file).await.unwrap();
    engine.start().await.unwrap();

    // When: Passage plays and hits EOF before crossfade point
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Then: No ERROR logs (EndOfFileBeforeLeadOut handled gracefully)
    assert_no_error_logs();

    // And: Next passage started (if exists)
    assert!(mixer_is_playing(&engine).await || queue_is_empty(&engine).await);
}
```

---

## 6. Implementation Roadmap

### 6.1 Development Phases

**Phase 1: Idempotent Operations (1.5 hours)**
- Modify `db::queue::remove_from_queue()` signature and implementation
- Update all callers to handle new return type
- Write and pass unit tests TC-U-IDEMP-001 through TC-U-IDEMP-003
- Verify no behavioral regression (existing tests still pass)

**Phase 2: Event Deduplication (1.5 hours)**
- Add `completed_passages` field to PlaybackEngine
- Implement deduplication logic in diagnostics handler
- Add cleanup task (per-event or background)
- Write and pass unit tests TC-U-DEDUP-001 through TC-U-DEDUP-004
- Run integration test TC-I-REMOVAL-002 (duplicate event scenario)

**Phase 3: DRY Refactoring (1.5 hours)**
- Extract `cleanup_queue_entry()` helper method
- Refactor `skip_next()` to use helper
- Refactor `remove_queue_entry()` to use helper
- Refactor `complete_passage_removal()` to use helper
- Write and pass unit tests TC-U-DRY-001 through TC-U-DRY-003
- Verify line count reduction (target: 40-60% reduction)

**Phase 4: Integration & System Testing (1 hour)**
- Run all integration tests (TC-I-*)
- Run all system tests (TC-S-*)
- Performance profiling (deduplication overhead)
- Log analysis (verify no ERROR logs)

**Phase 5: Documentation & Review (0.5 hours)**
- Update inline code comments with requirement IDs
- Update SPEC028 (note deduplication addition)
- Code review with checklist
- Merge to main branch

### 6.2 Acceptance Criteria

**Required for completion:**
- ✓ All unit tests pass (16 tests)
- ✓ All integration tests pass (3 tests)
- ✓ All system tests pass (2 tests)
- ✓ Existing test suite still passes (no regression)
- ✓ Zero ERROR logs in duplicate event scenarios
- ✓ Line count reduced by 40-60% in cleanup methods
- ✓ Code review approved

---

## 7. Risk Mitigation

### 7.1 Failure Mode Analysis

**FM-001: Idempotency logic bug (incorrect removal detection)**
- **Probability:** Low
- **Impact:** Medium (false positives/negatives in removal detection)
- **Mitigation:** Comprehensive unit tests for all edge cases
- **Detection:** Unit test failures during TDD
- **Recovery:** Fix logic, rerun tests

**FM-002: Deduplication state memory leak**
- **Probability:** Low
- **Impact:** Low (gradual memory growth over days)
- **Mitigation:** Automatic cleanup after 5 seconds, unit test for cleanup
- **Detection:** Long-running system test with memory profiling
- **Recovery:** Tune cleanup interval, add monitoring

**FM-003: Refactoring introduces regression**
- **Probability:** Low
- **Impact:** High (behavioral change in production)
- **Mitigation:** Maintain existing test suite passing, add new edge case tests
- **Detection:** Test suite failure, integration test failure
- **Recovery:** Revert refactoring, identify broken behavior, fix

**FM-004: Race condition in deduplication**
- **Probability:** Low
- **Impact:** Medium (duplicate processing under high concurrency)
- **Mitigation:** Use RwLock for thread-safe state access
- **Detection:** Stress test with concurrent passages
- **Recovery:** Add mutex, review concurrent access patterns

### 7.2 Rollback Plan

**If critical issues discovered post-merge:**

**Step 1:** Immediate revert
```bash
git revert <merge-commit-hash>
git push origin main
```

**Step 2:** Analyze failure
- Review logs for error patterns
- Identify which component failed (idempotency, deduplication, refactoring)
- Determine root cause

**Step 3:** Fix in feature branch
- Create hotfix branch
- Fix specific issue
- Add regression test
- Verify all tests pass

**Step 4:** Re-merge with additional safeguards
- Longer testing period
- Gradual rollout (if applicable)
- Monitor logs closely

---

## 8. Traceability

### 8.1 Requirements Mapping

| Requirement | Implementation | Test Coverage |
|-------------|----------------|---------------|
| [REQ-QUEUE-IDEMP-010] | `db::queue::remove_from_queue()` | TC-U-IDEMP-001, TC-U-IDEMP-002, TC-U-IDEMP-003 |
| [REQ-QUEUE-IDEMP-020] | Return value semantics in callers | TC-U-IDEMP-002, TC-U-DRY-002 |
| [REQ-QUEUE-DEDUP-010] | Diagnostics handler dedup logic | TC-U-DEDUP-001, TC-U-DEDUP-002, TC-U-DEDUP-004 |
| [REQ-QUEUE-DEDUP-020] | Dedup scope (5-second window) | TC-U-DEDUP-004 |
| [REQ-QUEUE-DEDUP-030] | Thread-safe state (RwLock) | TC-U-DEDUP-003, stress tests |
| [REQ-QUEUE-DRY-010] | `cleanup_queue_entry()` helper | TC-U-DRY-001, TC-U-DRY-003 |
| [REQ-QUEUE-DRY-020] | Cleanup ordering | TC-U-DRY-001 |

### 8.2 Related Documents

| Document | Relationship |
|----------|--------------|
| [queue_handling_mechanism_analysis.md](queue_handling_mechanism_analysis.md) | Root cause analysis that motivated this spec |
| [SPEC028: Playback Orchestration](../docs/SPEC028-playback_orchestration.md) | Event-driven architecture this spec enhances |
| [SPEC016: Decoder Buffer Design](../docs/SPEC016-decoder_buffer_design.md) | Chain lifecycle this spec maintains |
| [REQ001: Requirements](../docs/REQ001-requirements.md) | Upstream functional requirements |

---

## 9. Future Enhancements

**Out of scope for this specification (consider for future work):**

**Enhancement 1: Telemetry for duplicate events**
- Add SSE event `WkmpEvent::DuplicateEventDetected`
- Track duplicate event count in developer UI
- Helps diagnose event system issues

**Enhancement 2: Configurable deduplication window**
- Add setting `deduplication_window_ms` (default: 5000)
- Allow tuning for different hardware/workloads
- Stored in database settings table

**Enhancement 3: Event source consolidation**
- Investigate consolidating EOF markers into single PassageComplete
- Requires deeper mixer/marker changes
- Higher risk, deferred to future work

---

## 10. Summary

**Specification Goal:** Improve queue handling resilience by eliminating duplicate event errors through idempotent operations, event deduplication, and DRY code organization.

**Approach:** Test-driven development (TDD) with comprehensive test coverage for edge cases.

**Estimated Effort:** 5 hours (1.5h idempotency + 1.5h deduplication + 1.5h refactoring + 1h testing + 0.5h docs)

**Success Metrics:**
- Zero ERROR logs from duplicate events ✓
- 40-60% line reduction in cleanup methods ✓
- 100% test coverage for new functionality ✓
- No behavioral regression ✓

**Ready for /plan workflow:** Yes

---

**End of Specification**
