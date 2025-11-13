# Queue Handling Mechanism Analysis

**Date:** 2025-11-06
**Purpose:** Deep dive analysis of queue handling mechanism based on code review and log failure analysis
**Related:** SPEC028 (Playback Orchestration), SPEC016 (Decoder Buffer Design)

---

## Executive Summary

WKMP's queue handling uses an **event-driven architecture** where passage completion triggers automatic queue advancement and resource cleanup. The system maintains a three-tier queue structure (Current → Next → Queued) with marker-driven events coordinating passage transitions.

**Key Finding from Log Analysis:** The observed errors indicate a **double-removal race condition** where PassageComplete events are being fired multiple times for the same queue entry, causing failed attempts to remove an already-deleted entry from the database.

---

## 1. Queue Structure

### 1.1 Three-Tier Architecture

The queue is managed by `QueueManager` ([wkmp-ap/src/playback/queue_manager.rs:98-111](../wkmp-ap/src/playback/queue_manager.rs#L98-L111)):

```rust
pub struct QueueManager {
    /// Currently playing passage
    current: Option<QueueEntry>,

    /// Next to play (gets full buffer immediately)
    next: Option<QueueEntry>,

    /// After next (get partial buffers)
    queued: Vec<QueueEntry>,

    /// Cached total count (current + next + queued.len())
    total_count: usize,
}
```

**Positions:**
- **Current:** Actively playing passage (mixer is consuming from this buffer)
- **Next:** Queued for crossfade (buffer pre-decoded, ready for seamless transition)
- **Queued:** Awaiting promotion (may or may not have decoder chains assigned)

### 1.2 Queue Entry Lifecycle

**States:** Enqueued → Assigned Chain → Decoding → Ready → Playing → Completed → Removed

**Key IDs:**
- `queue_entry_id`: Unique UUID for this specific queue instance (persists through lifecycle)
- `passage_id`: Optional UUID linking to passages table (None for ephemeral passages)

---

## 2. Intended Queue Handling Workflow

### 2.1 Passage Completion Flow

**Trigger:** Mixer reaches PassageComplete marker ([wkmp-ap/src/playback/engine/playback.rs:1186-1197](../wkmp-ap/src/playback/engine/playback.rs#L1186-L1197))

```
Marker Added (at playback start):
├─ Position: fade_out_point_ticks or passage_duration_ticks
├─ Event: MarkerEvent::PassageComplete
└─ Tied to: queue_entry_id + passage_id
```

**Event Processing Chain:**

1. **Mixer Thread** ([wkmp-ap/src/playback/engine/core.rs:522-528](../wkmp-ap/src/playback/engine/core.rs#L522-L528))
   ```rust
   MarkerEvent::PassageComplete => {
       *is_crossfading = false;
       if let Some(queue_entry_id) = *current_queue_entry_id {
           event_tx.send(PlaybackEvent::PassageComplete { queue_entry_id }).ok();
       }
   }
   ```
   - Mixer detects marker hit
   - Sends `PlaybackEvent::PassageComplete` to event channel
   - Continues processing audio (non-blocking)

2. **Diagnostics Event Handler** ([wkmp-ap/src/playback/engine/diagnostics.rs:575-598](../wkmp-ap/src/playback/engine/diagnostics.rs#L575-L598))
   ```rust
   Some(PlaybackEvent::PassageComplete { queue_entry_id }) => {
       info!("PassageComplete event received for queue_entry_id: {}", queue_entry_id);

       // Stop mixer and clear passage
       {
           let mut mixer = self.mixer.write().await;
           mixer.clear_passage();
           mixer.clear_all_markers();
       }

       // Remove completed entry from database + memory + emit events
       if let Err(e) = self.complete_passage_removal(
           queue_entry_id,
           QueueChangeTrigger::PassageCompletion
       ).await {
           error!("Failed to complete passage removal: {}", e);
       }

       // Trigger watchdog check to ensure next passage starts
       if let Err(e) = self.watchdog_check().await {
           error!("Failed to run watchdog check after passage complete: {}", e);
       }
   }
   ```
   - Receives event from mixer thread
   - Clears mixer state (stops audio output for this passage)
   - Calls `complete_passage_removal()` (see 2.2)
   - Runs watchdog check to start next passage

3. **Complete Passage Removal** ([wkmp-ap/src/playback/engine/queue.rs:556-637](../wkmp-ap/src/playback/engine/queue.rs#L556-L637))
   ```rust
   pub(super) async fn complete_passage_removal(
       &self,
       queue_entry_id: Uuid,
       trigger: QueueChangeTrigger,
   ) -> Result<bool> {
       // Step 1: Capture state BEFORE removal (detect promotions)
       let (next_before, queued_first_before) = {
           let queue = self.queue.read().await;
           (
               queue.next().map(|e| e.queue_entry_id),
               queue.queued().first().map(|e| e.queue_entry_id),
           )
       };

       // Step 2: Remove from database FIRST (persistence before memory)
       if let Err(e) = crate::db::queue::remove_from_queue(&self.db_pool, queue_entry_id).await {
           error!("Failed to remove entry from database: {}", e);
       }

       // Step 3: Remove from in-memory queue
       let removed = {
           let mut queue_write = self.queue.write().await;
           queue_write.remove(queue_entry_id)
       };

       if !removed {
           warn!("Failed to remove queue entry from memory: {}", queue_entry_id);
           return Ok(false);
       }

       // Step 4: Emit events
       self.state.broadcast_event(WkmpEvent::QueueChanged { ... });

       // Step 5: Trigger decode for promoted passages
       // (next → current or queued_first → next)
       // ...event-driven decode logic...

       Ok(true)
   }
   ```
   - **Captures queue state** before modification (for promotion detection)
   - **Removes from database** first (persistence layer)
   - **Removes from memory** via `QueueManager::remove()`
   - **Emits SSE events** (QueueChanged, QueueIndexChanged)
   - **Triggers decode** for newly promoted passages

### 2.2 Queue Advancement (QueueManager::remove)

**Location:** [wkmp-ap/src/playback/queue_manager.rs:307-339](../wkmp-ap/src/playback/queue_manager.rs#L307-L339)

**Behavior depends on which position is being removed:**

**Case 1: Remove Current**
```rust
if current.queue_entry_id == queue_entry_id {
    self.advance();  // Promotes next → current, queued[0] → next
    return true;
}
```

**Case 2: Remove Next**
```rust
if next.queue_entry_id == queue_entry_id {
    if !self.queued.is_empty() {
        self.next = Some(self.queued.remove(0));  // Promote queued[0] → next
    } else {
        self.next = None;  // No more passages
    }
    return true;
}
```

**Case 3: Remove from Queued**
```rust
if let Some(index) = self.queued.iter().position(|e| e.queue_entry_id == queue_entry_id) {
    self.queued.remove(index);  // Simple array removal
    return true;
}
```

**Automatic Promotion:** The `advance()` method ([queue_manager.rs:271-286](../wkmp-ap/src/playback/queue_manager.rs#L271-L286)) handles cascading:
```rust
pub fn advance(&mut self) -> Option<QueueEntry> {
    let old_current = self.current.take();

    // Move next to current
    self.current = self.next.take();

    // Move first queued to next
    if !self.queued.is_empty() {
        self.next = Some(self.queued.remove(0));
    }

    old_current
}
```

---

## 3. Log Failure Analysis

### 3.1 Observed Symptom

```
2025-11-06T16:35:32.593632Z  INFO PassageComplete event received for queue_entry_id: 66364e56-8188-4ce2-ae1d-10b1c62d8e66
2025-11-06T16:35:32.594494Z ERROR Failed to remove entry from database: Queue error: Queue entry not found: 66364e56-8188-4ce2-ae1d-10b1c62d8e66
2025-11-06T16:35:32.594609Z  WARN Failed to remove queue entry from memory: 66364e56-8188-4ce2-ae1d-10b1c62d8e66

[7ms later - duplicate event]

2025-11-06T16:35:32.601181Z  INFO PassageComplete event received for queue_entry_id: 66364e56-8188-4ce2-ae1d-10b1c62d8e66
2025-11-06T16:35:32.602043Z ERROR Failed to remove entry from database: Queue error: Queue entry not found: 66364e56-8188-4ce2-ae1d-10b1c62d8e66
2025-11-06T16:35:32.602223Z  WARN Failed to remove queue entry from memory: 66364e56-8188-4ce2-ae1d-10b1c62d8e66
```

**Analysis:**

1. **First PassageComplete** (16:35:32.593632Z)
   - Event received for `66364e56-8188-4ce2-ae1d-10b1c62d8e66`
   - Database removal **fails** → Entry already gone
   - Memory removal **fails** → Entry not in queue

2. **Second PassageComplete** (16:35:32.601181Z) - **7ms later**
   - **Duplicate event** for same `queue_entry_id`
   - Predictably fails (entry already processed)

3. **Subsequent Events**
   - Starting playback for `76097a9b-b23c-4d31-9b39-f5f60d3c246e` (likely the promoted "next" passage)
   - Watchdog intervention warnings (mixer state inconsistency)

### 3.2 Root Cause Hypothesis

**Double-Event Race Condition:** PassageComplete event fired multiple times for single passage.

**Possible Triggers:**

1. **Multiple Marker Hits**
   - EOF marker + PassageComplete marker both firing ([core.rs:534-542](../wkmp-ap/src/playback/engine/core.rs#L534-L542))
   - Both send `PlaybackEvent::PassageComplete` for same `queue_entry_id`
   - Race: First completes removal, second finds entry missing

2. **Watchdog Intervention Interference**
   - Watchdog detects stalled playback, attempts recovery
   - May trigger duplicate state changes or event emissions
   - Observed: "Watchdog intervention" log immediately after (lines 655, 655 again)

3. **Marker Clear Timing**
   - `mixer.clear_all_markers()` called in diagnostics handler
   - If marker already triggered but not cleared, may re-trigger
   - **However:** Code shows markers cleared BEFORE removal, should prevent this

### 3.3 Evidence from Code

**EOF Handling** ([core.rs:534-551](../wkmp-ap/src/playback/engine/core.rs#L534-L551)):
```rust
MarkerEvent::EndOfFile { unreachable_markers } => {
    warn!("EOF reached with {} unreachable markers", unreachable_markers.len());
    // Treat EOF as passage complete - emit PassageComplete event
    *is_crossfading = false;
    if let Some(queue_entry_id) = *current_queue_entry_id {
        event_tx.send(PlaybackEvent::PassageComplete { queue_entry_id }).ok();
        debug!("Sent PassageComplete event (EOF) for queue_entry_id: {}", queue_entry_id);
    }
}

MarkerEvent::EndOfFileBeforeLeadOut { planned_crossfade_tick, .. } => {
    warn!("EOF before crossfade at tick {}", planned_crossfade_tick);
    // Treat early EOF as passage complete - emit PassageComplete event
    *is_crossfading = false;
    if let Some(queue_entry_id) = *current_queue_entry_id {
        event_tx.send(PlaybackEvent::PassageComplete { queue_entry_id }).ok();
        debug!("Sent PassageComplete event (early EOF) for queue_entry_id: {}", queue_entry_id);
    }
}
```

**Risk:** If file has both PassageComplete marker AND hits EOF, **two events** will be sent.

### 3.4 Missing Idempotency Protection

**Current Implementation** ([queue.rs:571-586](../wkmp-ap/src/playback/engine/queue.rs#L571-L586)):
```rust
// Remove from database FIRST (persistence before memory)
if let Err(e) = crate::db::queue::remove_from_queue(&self.db_pool, queue_entry_id).await {
    error!("Failed to remove entry from database: {}", e);
    // ⚠️ ERROR LOGGED BUT CONTINUES - no early return
}

// Remove from in-memory queue
let removed = {
    let mut queue_write = self.queue.write().await;
    queue_write.remove(queue_entry_id)
};

if !removed {
    warn!("Failed to remove queue entry from memory: {}", queue_entry_id);
    return Ok(false);  // ⚠️ Returns false but operation "succeeds"
}
```

**Issue:** No check if entry already removed by previous event.

**Database Operation** ([db/queue.rs:162-176](../wkmp-ap/src/db/queue.rs#L162-L176)):
```rust
pub async fn remove_from_queue(db: &Pool<Sqlite>, queue_entry_id: Uuid) -> Result<()> {
    let result = sqlx::query("DELETE FROM queue WHERE guid = ?")
        .bind(queue_entry_id.to_string())
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(Error::Queue(format!(
            "Queue entry not found: {}",
            queue_entry_id
        )));  // ⚠️ Errors on idempotent call (already deleted)
    }

    Ok(())
}
```

**Issue:** Not idempotent - second removal attempt fails with error (should succeed with no-op).

---

## 4. Design Intent vs. Current Behavior

### 4.1 Design Intent (from SPEC028)

**Event-Driven Architecture** ([SPEC028:30-45](../docs/SPEC028-playback_orchestration.md#L30-L45)):
- Primary path: Event-triggered operations (<1ms latency)
- Watchdog path: Detection-only fallback (100ms interval)
- **Goal:** Zero watchdog interventions during normal operation

**Idempotency Principle:**
- Events may be delivered multiple times (distributed systems principle)
- Operations should be idempotent (safe to retry)
- **Not explicitly documented** but implied by event-driven architecture

### 4.2 Current Behavior Gaps

**Gap 1: Non-Idempotent Removal**
- Database removal errors on duplicate call
- Memory removal returns false (but logs WARN, not ERROR)
- Downstream logic continues despite failure

**Gap 2: Multiple Event Sources**
- PassageComplete marker
- EndOfFile marker
- EndOfFileBeforeLeadOut marker
- All emit same `PlaybackEvent::PassageComplete`
- No deduplication mechanism

**Gap 3: Mixer State Inconsistency**
- Watchdog interventions immediately after PassageComplete
- "Buffer ready but mixer not started" warning
- Suggests state machine desync between queue removal and mixer startup

---

## 5. Intended Workflow Summary

### 5.1 Normal Passage Completion (No Errors)

```
┌─────────────────┐
│ Mixer Thread    │
│ (audio output)  │
└────────┬────────┘
         │ Reaches PassageComplete marker at tick N
         │
         ▼
┌─────────────────────────────────────────────────┐
│ Send PlaybackEvent::PassageComplete             │
│   queue_entry_id: ABC                           │
└────────┬────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────┐
│ Diagnostics Event Handler (async task)         │
│                                                 │
│ 1. Receive PassageComplete for ABC             │
│ 2. Clear mixer (clear_passage, clear_markers)  │
│ 3. Call complete_passage_removal(ABC)          │
│    ├─ Capture queue state (next=DEF, queued[0]=GHI)
│    ├─ Remove ABC from database                 │
│    ├─ Remove ABC from memory (triggers advance)│
│    │  └─ QueueManager.remove(ABC)              │
│    │     └─ current.remove() → advance()       │
│    │        ├─ DEF promoted to current         │
│    │        └─ GHI promoted to next            │
│    ├─ Emit QueueChanged event                  │
│    └─ Trigger decode for DEF (now current)     │
│ 4. Call watchdog_check()                       │
│    └─ Starts mixer for DEF if buffer ready     │
└─────────────────────────────────────────────────┘
```

**Result:** ABC removed, DEF playing, GHI decoding for crossfade.

### 5.2 Failure Scenario (Double Event)

```
┌─────────────────┐
│ Mixer Thread    │
└────────┬────────┘
         │ BOTH PassageComplete marker + EOF hit
         │
         ├────────────────┬─────────────────┐
         ▼                ▼                 ▼
    Event 1          Event 2          Watchdog Check
    (ABC)            (ABC duplicate)   (detects desync)
         │                │                 │
         ▼                │                 │
  Remove ABC (success)    │                 │
    ├─ DB: DELETE OK      │                 │
    ├─ Memory: OK         │                 │
    └─ Advance queue      │                 │
       (DEF → current)    │                 │
         │                ▼                 │
         │        Remove ABC (FAIL)         │
         │          ├─ DB: NOT FOUND ❌     │
         │          └─ Memory: NOT FOUND ❌ │
         │                │                 │
         └────────────────┴─────────────────▼
                   Mixer state mismatch
                   (watchdog intervention logged)
```

**Result:** Errors logged, but system recovers (ABC removed once, DEF playing).

### 5.3 Resource Cleanup Chain

**When passage removed from queue:**

1. **Database Persistence**
   - `DELETE FROM queue WHERE guid = ?`
   - Persists removal before memory modification

2. **Queue Memory Update**
   - `QueueManager.remove(queue_entry_id)`
   - Automatic promotion (next → current)
   - O(1) for current/next, O(n) for queued

3. **Chain Release** ([chains.rs via lifecycle](../wkmp-ap/src/playback/engine/chains.rs))
   - Decoder-buffer chain freed
   - Chain index returned to available pool
   - Next unassigned passage may receive chain

4. **Buffer Cleanup**
   - `BufferManager.remove(passage_id)`
   - Pre-decoded audio data freed from memory

5. **Event Emission**
   - `WkmpEvent::QueueChanged` (queue contents changed)
   - `WkmpEvent::QueueIndexChanged` (positions shifted)
   - SSE broadcast to all connected UIs

6. **Decode Triggering** (Promoted Passages)
   - If next promoted to current → `DecodePriority::Immediate`
   - If queued[0] promoted to next → `DecodePriority::Next`
   - Event-driven decode request (<1ms after removal)

---

## 6. Key Mechanisms

### 6.1 Marker-Driven Events

**Markers Added at Playback Start** ([playback.rs:1137-1202](../wkmp-ap/src/playback/engine/playback.rs#L1137-L1202)):

1. **Position Update Markers**
   - Every N ticks (default: 1000ms intervals)
   - Emit `PlaybackEvent::PositionUpdate` for UI progress bars

2. **Crossfade Start Marker** (if next passage exists)
   - At `fade_out_point_ticks` or `lead_out_point_ticks - 5s`
   - Triggers `MarkerEvent::StartCrossfade`

3. **Passage Complete Marker**
   - At `fade_out_point_ticks` or `passage_duration_ticks`
   - Triggers `MarkerEvent::PassageComplete`

**Processing:** Mixer checks markers on every audio output callback, emits events when tick threshold crossed.

### 6.2 Event-Driven Decode

**Triggering Points** ([SPEC028:50-91](../docs/SPEC028-playback_orchestration.md#L50-L91)):

1. **Enqueue Event**
   - User adds passage to queue
   - Position determines priority:
     - Empty queue → `DecodePriority::Immediate` (start playing now)
     - Next slot → `DecodePriority::Next` (prepare for crossfade)
     - Queued slot → `DecodePriority::Prefetch` (background decode)

2. **Queue Advance Event**
   - Passage completes, queue promotions occur
   - Detect promotions by comparing queue state before/after removal
   - Trigger decode for newly promoted passages

**Design Goal:** <1ms latency from trigger to decode start (vs. 100ms polling interval in old architecture).

### 6.3 Watchdog Safety Net

**Purpose:** Detect event system failures and intervene.

**Operation** ([SPEC028:156-184](../docs/SPEC028-playback_orchestration.md#L156-L184)):
- Runs every 100ms (polling loop)
- **Detection-only:** Checks if operations should have happened
- **Intervention:** Performs operation + logs WARN + increments counter
- **Goal:** Zero interventions (non-zero indicates bug)

**Checks:**
1. Buffer ready but mixer not started
2. Queue has entries but no playback active
3. Decode not triggered for next passage

---

## 7. Recommendations

### 7.1 Idempotency Enhancement

**Make removal operations idempotent:**

```rust
pub async fn remove_from_queue(db: &Pool<Sqlite>, queue_entry_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM queue WHERE guid = ?")
        .bind(queue_entry_id.to_string())
        .execute(db)
        .await?;

    // Return true if deleted, false if already missing (both OK)
    Ok(result.rows_affected() > 0)
}
```

**Update complete_passage_removal to handle gracefully:**

```rust
// Check if already removed (idempotency)
let db_removed = crate::db::queue::remove_from_queue(&self.db_pool, queue_entry_id).await?;
let mem_removed = self.queue.write().await.remove(queue_entry_id);

if !db_removed && !mem_removed {
    // Already removed by previous call - this is OK
    debug!("Queue entry {} already removed (duplicate event)", queue_entry_id);
    return Ok(false);
}
```

### 7.2 Event Deduplication

**Add deduplication for PassageComplete events:**

```rust
// In PlaybackEngine
completed_passages: Arc<RwLock<HashSet<Uuid>>>,  // Recent completions (last 1 second)

// In diagnostics handler
Some(PlaybackEvent::PassageComplete { queue_entry_id }) => {
    // Check if already processed
    if self.completed_passages.read().await.contains(&queue_entry_id) {
        debug!("Ignoring duplicate PassageComplete for {}", queue_entry_id);
        return;
    }

    // Mark as processed
    self.completed_passages.write().await.insert(queue_entry_id);

    // ... existing removal logic ...

    // Cleanup after 1 second (prevent unbounded growth)
    let completed_clone = self.completed_passages.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        completed_clone.write().await.remove(&queue_entry_id);
    });
}
```

### 7.3 Marker Consolidation

**Prevent multiple PassageComplete sources:**

```rust
MarkerEvent::EndOfFile { unreachable_markers } => {
    // Check if PassageComplete marker was in unreachable set
    let had_complete_marker = unreachable_markers.iter()
        .any(|m| matches!(m.event_type, MarkerEvent::PassageComplete));

    if had_complete_marker {
        warn!("EOF reached before PassageComplete marker - already processed");
        // Don't send duplicate event
    } else {
        // Send PassageComplete event (file ended without marker)
        event_tx.send(PlaybackEvent::PassageComplete { queue_entry_id }).ok();
    }
}
```

### 7.4 Monitoring

**Add telemetry for duplicate detection:**

```rust
// New SSE event
WkmpEvent::DuplicateEventDetected {
    event_type: "PassageComplete",
    queue_entry_id: Uuid,
    timestamp: DateTime<Utc>,
}
```

---

## 8. Conclusion

The queue handling mechanism is **well-designed** with clear separation of concerns:
- Marker-driven events for timing-critical operations
- Event-driven decode for low-latency buffer preparation
- Automatic queue advancement with promotion logic
- Watchdog safety net for fault tolerance

**Current Issue:** Double-event race condition due to:
1. Multiple PassageComplete sources (marker + EOF)
2. Non-idempotent database removal
3. No event deduplication

**System Resilience:** Despite errors, system recovers gracefully:
- First removal succeeds (passage actually removed)
- Second removal fails gracefully (logs error but continues)
- Playback advances to next passage (watchdog ensures startup)

**Severity:** Low - cosmetic errors in logs, no functional impact on playback.

**Priority:** Medium - should be fixed to reduce log noise and improve telemetry clarity.

---

## Appendix A: Call Graph

```
User Action (Skip/Complete)
    │
    ├─→ Mixer Thread (audio callback)
    │   └─→ Check markers → PassageComplete hit
    │       └─→ Send PlaybackEvent::PassageComplete
    │
    └─→ Diagnostics Event Handler (async task)
        ├─→ Receive PlaybackEvent::PassageComplete
        ├─→ Clear mixer (clear_passage, clear_markers)
        ├─→ complete_passage_removal()
        │   ├─→ db::queue::remove_from_queue() ─→ DELETE FROM queue
        │   ├─→ QueueManager::remove() ─→ Advance queue (next→current)
        │   ├─→ broadcast_event(QueueChanged)
        │   └─→ request_decode() ─→ Trigger decode for promoted passages
        └─→ watchdog_check()
            └─→ start_mixer_for_current() ─→ Begin playback of next passage
```

---

## Appendix B: Key Files

| File | Lines | Purpose |
|------|-------|---------|
| [wkmp-ap/src/playback/queue_manager.rs](../wkmp-ap/src/playback/queue_manager.rs) | 98-429 | Queue structure (Current/Next/Queued) and advancement logic |
| [wkmp-ap/src/playback/engine/queue.rs](../wkmp-ap/src/playback/engine/queue.rs) | 556-637 | `complete_passage_removal()` - coordinated cleanup |
| [wkmp-ap/src/playback/engine/diagnostics.rs](../wkmp-ap/src/playback/engine/diagnostics.rs) | 575-598 | PassageComplete event handler |
| [wkmp-ap/src/playback/engine/core.rs](../wkmp-ap/src/playback/engine/core.rs) | 522-551 | Mixer thread marker processing (sends events) |
| [wkmp-ap/src/playback/engine/playback.rs](../wkmp-ap/src/playback/engine/playback.rs) | 1186-1202 | Marker creation at playback start |
| [wkmp-ap/src/db/queue.rs](../wkmp-ap/src/db/queue.rs) | 162-176 | Database removal operation |
| [docs/SPEC028-playback_orchestration.md](../docs/SPEC028-playback_orchestration.md) | Full | Event-driven architecture specification |
| [docs/SPEC016-decoder_buffer_design.md](../docs/SPEC016-decoder_buffer_design.md) | 72-100 | Chain lifecycle and queue integration |

---

## Appendix C: Related Requirements

| Requirement | Description | Satisfied By |
|-------------|-------------|--------------|
| [REQ-AP-QUEUE-010] | Queue CRUD operations | `enqueue_file()`, `remove_queue_entry()`, `clear_queue()` |
| [REQ-AP-QUEUE-020] | Automatic queue advancement | `QueueManager::advance()` called in `remove()` |
| [SSD-ENG-020] | Queue processing | Event-driven architecture (SPEC028) |
| [DBD-LIFECYCLE-020] | Chain release on completion | `release_chain()` called in skip/complete flows |
| [PLAN020 FR-001] | Event-driven decode | Triggered in `complete_passage_removal()` |

---

**End of Analysis**
