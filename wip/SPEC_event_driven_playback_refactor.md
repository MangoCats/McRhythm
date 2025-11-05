# SPEC: Event-Driven Playback Orchestration with Watchdog

**Status:** Draft - Ready for /plan
**Date Created:** 2025-11-04
**Author:** WKMP Development Team
**Related Documents:**
- [SPEC028: Playback Loop Orchestration](../docs/SPEC028-playback_orchestration.md) - Current implementation
- [SPEC001: Architecture](../docs/SPEC001-architecture.md) - System architecture
- [REQ001: Requirements](../docs/REQ001-requirements.md) - System requirements

---

## 1. Executive Summary

**Problem:** Current playback orchestration uses 100ms polling for operations that should be event-driven, resulting in unnecessary latency, CPU usage, and architectural inconsistency.

**Solution:** Refactor to fully event-driven architecture with 100ms watchdog loop that only intervenes when event system fails.

**Key Changes:**
1. Eliminate polling for decode initiation and mixer startup
2. Make all queue operations event-driven
3. Retain 100ms loop as watchdog-only (logs warnings, restores state)
4. Test suite treats watchdog intervention as event system failure

**Impact:** Improved responsiveness, reduced CPU usage, cleaner architecture, better testability.

---

## 2. Current State Analysis

### 2.1 What's Currently Polled (100ms process_queue loop)

**Per SPEC028 and core.rs:1324-1680:**

| Operation | Current | Should Be |
|-----------|---------|-----------|
| Trigger decode for current passage | Polled | Event-driven (on enqueue) |
| Start mixer when buffer ready | Polled (check 3s threshold) | Event-driven (buffer event) |
| Crossfade trigger | ~~Polled~~ | ✅ Already event-driven (markers) |
| Trigger decode for next/queued | Polled (on queue advance) | Event-driven (queue event) |
| Crossfade completion | ~~Polled~~ | ✅ Already event-driven (markers) |

### 2.2 What's Already Event-Driven ✅

- **Position tracking** - Via `PositionUpdate` markers
- **Crossfade initiation** - Via `StartCrossfade` marker
- **Crossfade completion** - Via `PassageComplete` event
- **Song boundaries** - Via `SongBoundary` markers
- **Skip commands** - Via API → immediate event
- **Enqueue/remove operations** - Via API → immediate event

### 2.3 Technical Debt

**Root Cause:** Incremental migration to marker-based system (SUB-INC-4B) completed crossfade/position operations but did not migrate decode/mixer startup logic.

**Evidence from SPEC028:**
- Line 1596: `let is_crossfading = false; // Stub: crossfading handled by markers now`
- Line 1674-1675: `// [SUB-INC-4B] Crossfade completion now handled via markers`

**Conclusion:** 100ms polling is historical artifact, not architectural necessity.

---

## 3. Requirements

### 3.1 Functional Requirements

**FR-001: Event-Driven Decode Initiation**
- **Description:** Decode requests triggered immediately by queue state changes, not polled
- **Events:**
  - `EnqueueEvent` → Trigger decode for newly enqueued passage
  - `QueueAdvanceEvent` → Trigger decode for newly promoted next/queued passages
- **Behavior:** Same as current (immediate priority, next priority, prefetch priority)
- **Latency:** <1ms (vs. current 0-100ms random latency)

**FR-002: Event-Driven Mixer Startup**
- **Description:** Mixer starts when buffer reaches threshold, not polled
- **Event:** `BufferThresholdReached(queue_entry_id, threshold_ms)`
  - Emitted by BufferManager when accumulated PCM ≥ threshold (3000ms)
- **Behavior:** Start mixer with markers, emit PassageStarted (same as current)
- **Latency:** <1ms (vs. current 0-100ms random latency)

**FR-003: Watchdog Loop (100ms)**
- **Description:** Safety mechanism detecting and recovering from event system failures
- **Behavior:**
  - Check for "stuck" states every 100ms
  - Log **WARN** when intervention required
  - Restore proper state
  - **CRITICAL:** Intervention indicates event system bug
- **Stuck States:**
  1. Current passage buffer missing (should decode)
  2. Mixer idle with ready buffer (should start)
  3. Next passage buffer missing with playing current (should prefetch)
  4. Queued passage buffer missing with available decode slots (should prefetch)
- **Logging Format:**
  ```rust
  warn!(
      "[WATCHDOG] Event system failure detected: {reason}. \
       Intervention: {action}. This indicates a bug in event-driven logic."
  );
  ```

**FR-004: Event System Test Coverage**
- **Description:** Test suite validates event-driven behavior without watchdog
- **Requirements:**
  1. Unit tests for each event trigger path
  2. Integration tests verify end-to-end event flows
  3. Test environment disables watchdog interventions OR fails test if watchdog intervenes
  4. Mock event system for failure testing

### 3.2 Non-Functional Requirements

**NFR-001: Responsiveness**
- Event-driven operations: <1ms latency (vs. current 0-100ms)
- Watchdog check interval: 100ms (unchanged)
- No perceptible delay for user operations

**NFR-002: CPU Efficiency**
- Watchdog loop: Minimal checks (5-10 condition evaluations)
- No buffer reads in watchdog (use cached state)
- No database queries in watchdog

**NFR-003: Testability**
- Event triggers testable in isolation
- Watchdog testable separately
- Test environment can disable watchdog
- Test failures distinguish event bugs from watchdog bugs

**NFR-004: Backward Compatibility**
- External behavior identical to current implementation
- API unchanged
- Database schema unchanged
- Configuration unchanged

---

## 4. Design

### 4.1 Event System Architecture

**New Events:**

```rust
/// Emitted when passage enqueued
pub enum PlaybackEvent {
    /// Passage added to queue
    Enqueued {
        queue_entry_id: Uuid,
        passage_id: Option<Uuid>,
        position: QueuePosition, // Current | Next | Queued(index)
    },

    /// Queue advanced (current completed, next → current)
    QueueAdvanced {
        previous_current: Uuid,
        new_current: Option<Uuid>,
        new_next: Option<Uuid>,
    },

    /// Buffer reached playback threshold
    BufferThresholdReached {
        queue_entry_id: Uuid,
        threshold_ms: u64,
        buffer_level_ms: u64,
    },

    /// Existing events (already implemented)
    PassageStarted { ... },
    PassageComplete { ... },
    PositionUpdate { ... },
    // ... others
}
```

**Event Routing:**

```
┌─────────────────┐
│  Queue Manager  │
├─────────────────┤
│ enqueue()       │──→ Enqueued event ──→ Decode trigger
│ advance()       │──→ QueueAdvanced   ──→ Decode trigger (next/queued)
└─────────────────┘

┌─────────────────┐
│ Buffer Manager  │
├─────────────────┤
│ push_samples()  │──→ Check threshold ──→ BufferThresholdReached
└─────────────────┘                     ──→ Mixer startup

┌─────────────────┐
│  Mixer Thread   │
├─────────────────┤
│ Marker reached  │──→ PassageComplete ──→ Queue advance
└─────────────────┘
```

### 4.2 Refactored process_queue() → watchdog_check()

**Before (core.rs:1324-1680, ~350 lines):**

```rust
async fn process_queue(&self) -> Result<()> {
    // Clone queue state
    let (current, next, queued) = { ... };

    // Trigger decode for current (REDUNDANT - should be event-driven)
    if !self.buffer_manager.is_managed(current.id).await {
        self.request_decode(current, Immediate).await?;
    }

    // Start mixer if ready (POLLING - should be event-driven)
    if buffer_has_minimum && mixer_idle {
        // 200+ lines of mixer startup logic
    }

    // Crossfade check (OBSOLETE - already marker-driven)
    if position >= crossfade_start { ... }

    // Trigger decode for next (REDUNDANT - should be event-driven)
    if !self.buffer_manager.is_managed(next.id).await {
        self.request_decode(next, Next).await?;
    }

    // Trigger decode for queued (REDUNDANT - should be event-driven)
    for queued in queued.iter() { ... }
}
```

**After (watchdog only, target: <100 lines):**

```rust
/// Watchdog loop - ONLY intervenes when event system fails
/// Normal operation: This loop detects nothing, does nothing
async fn watchdog_check(&self) -> Result<()> {
    // Clone queue state (fast, no database)
    let (current, next, queued) = {
        let queue = self.queue.read().await;
        (queue.current().cloned(), queue.next().cloned(),
         queue.queued().to_vec())
    };

    // Watchdog Check 1: Current passage missing buffer
    if let Some(current) = &current {
        if !self.buffer_manager.is_managed(current.queue_entry_id).await {
            warn!(
                "[WATCHDOG] Current passage {} has no buffer. \
                 Event system should have triggered decode on enqueue. \
                 Intervention: Requesting immediate decode.",
                current.queue_entry_id
            );
            self.request_decode(current, DecodePriority::Immediate, true).await?;
        }
    }

    // Watchdog Check 2: Mixer idle with ready buffer
    if let Some(current) = &current {
        let buffer_ready = self.buffer_manager
            .has_minimum_playback_buffer(current.queue_entry_id, 3000)
            .await;
        let mixer = self.mixer.read().await;
        let mixer_idle = mixer.get_current_passage_id().is_none();
        drop(mixer);

        if buffer_ready && mixer_idle {
            warn!(
                "[WATCHDOG] Mixer idle with ready buffer ({}). \
                 Event system should have started mixer on threshold. \
                 Intervention: Starting mixer now.",
                current.queue_entry_id
            );
            self.start_mixer_for_current(current).await?;
        }
    }

    // Watchdog Check 3: Next passage missing buffer (if current playing)
    let mixer_playing = {
        let mixer = self.mixer.read().await;
        mixer.get_current_passage_id().is_some()
    };

    if mixer_playing {
        if let Some(next) = &next {
            if !self.buffer_manager.is_managed(next.queue_entry_id).await {
                warn!(
                    "[WATCHDOG] Next passage {} has no buffer while current playing. \
                     Event system should have triggered prefetch. \
                     Intervention: Requesting next decode.",
                    next.queue_entry_id
                );
                self.request_decode(next, DecodePriority::Next, true).await?;
            }
        }
    }

    // Watchdog Check 4: Queued passages missing buffers (if decode slots available)
    if mixer_playing {
        let max_queued = self.maximum_decode_streams.saturating_sub(2);
        for queued_entry in queued.iter().take(max_queued) {
            if !self.buffer_manager.is_managed(queued_entry.queue_entry_id).await {
                warn!(
                    "[WATCHDOG] Queued passage {} has no buffer. \
                     Event system should have triggered prefetch. \
                     Intervention: Requesting prefetch decode.",
                    queued_entry.queue_entry_id
                );
                self.request_decode(queued_entry, DecodePriority::Prefetch, true).await?;
            }
        }
    }

    Ok(())
}
```

**Key Differences:**
- **Before:** Proactively initiates operations (polling)
- **After:** Reactively detects missing operations (watchdog)
- **Logging:** WARN every intervention (indicates event bug)
- **Size:** ~350 lines → ~100 lines (70% reduction)

### 4.3 Event-Driven Implementations

#### 4.3.1 Decode on Enqueue

**Location:** [queue.rs:enqueue_file()](wkmp-ap/src/playback/engine/queue.rs)

**Current (after Bug #3 fix):**
```rust
pub async fn enqueue_file(&self, file_path: PathBuf) -> Result<Uuid> {
    // ... database enqueue ...
    // ... create in-memory entry ...

    // Add to queue
    self.queue.write().await.enqueue(entry);

    // Update audio_expected flag
    self.update_audio_expected_flag().await;

    Ok(queue_entry_id)
}
```

**Refactored (event-driven):**
```rust
pub async fn enqueue_file(&self, file_path: PathBuf) -> Result<Uuid> {
    // ... database enqueue ...
    // ... create in-memory entry ...

    // Add to queue
    let position = {
        let mut queue = self.queue.write().await;
        let position = queue.get_position_for_new_entry();
        queue.enqueue(entry.clone());
        position
    };

    // Update audio_expected flag
    self.update_audio_expected_flag().await;

    // **EVENT-DRIVEN:** Trigger decode immediately
    let priority = match position {
        QueuePosition::Current => DecodePriority::Immediate,
        QueuePosition::Next => DecodePriority::Next,
        QueuePosition::Queued(_) => DecodePriority::Prefetch,
    };

    debug!(
        "Enqueued {} at position {:?}, triggering decode with priority {:?}",
        queue_entry_id, position, priority
    );

    self.request_decode(&entry, priority, true).await?;

    // Emit event (for monitoring/debugging)
    self.state.broadcast_event(WkmpEvent::Enqueued {
        queue_entry_id,
        passage_id: entry.passage_id,
        position,
    });

    Ok(queue_entry_id)
}
```

#### 4.3.2 Buffer Threshold Detection

**Location:** [buffer_manager.rs:push_samples()](wkmp-ap/src/playback/buffer_manager.rs)

**Concept:**

```rust
pub async fn push_samples(
    &self,
    queue_entry_id: Uuid,
    samples: Vec<f32>,
) -> Result<(), String> {
    // ... existing push logic ...

    // Check if threshold just crossed
    const THRESHOLD_MS: u64 = 3000;
    let buffer_ms_before = previous_sample_count * 1000 / sample_rate;
    let buffer_ms_after = new_sample_count * 1000 / sample_rate;

    if buffer_ms_before < THRESHOLD_MS && buffer_ms_after >= THRESHOLD_MS {
        // **EVENT-DRIVEN:** Threshold crossed, emit event
        debug!(
            "Buffer {} crossed threshold: {}ms → {}ms",
            queue_entry_id, buffer_ms_before, buffer_ms_after
        );

        // Notify engine (via event channel or callback)
        self.emit_threshold_event(queue_entry_id, THRESHOLD_MS, buffer_ms_after);
    }

    Ok(())
}
```

**Implementation Options:**

**Option A: Event Channel (tokio::broadcast)**
- BufferManager emits events
- PlaybackEngine subscribes to channel
- Loose coupling

**Option B: Callback (Arc<dyn Fn>)**
- PlaybackEngine registers callback with BufferManager
- Direct invocation
- Tighter coupling, simpler

**Recommendation:** Option A (event channel) for consistency with existing event system.

#### 4.3.3 Mixer Startup Handler

**Location:** [core.rs:start_mixer_for_current()](wkmp-ap/src/playback/engine/core.rs)

**Extract from process_queue() → separate method:**

```rust
/// Start mixer for current passage (extracted from process_queue)
/// Called by: event handler (buffer threshold) OR watchdog (if event missed)
async fn start_mixer_for_current(&self, current: &QueueEntry) -> Result<()> {
    // Check mixer is idle
    let mixer = self.mixer.read().await;
    let mixer_idle = mixer.get_current_passage_id().is_none();
    drop(mixer);

    if !mixer_idle {
        // Already playing, nothing to do
        return Ok(());
    }

    // Get buffer
    if let Some(_buffer) = self.buffer_manager.get_buffer(current.queue_entry_id).await {
        // ... existing mixer startup logic (lines 1378-1586) ...
        // (load timing, add markers, emit PassageStarted, etc.)
    }

    Ok(())
}
```

**Event Handler:**

```rust
// In playback_loop or separate event handler task
async fn handle_buffer_threshold_event(&self, event: BufferThresholdEvent) {
    // Check if this is for current passage
    let current_id = {
        let queue = self.queue.read().await;
        queue.current().map(|e| e.queue_entry_id)
    };

    if current_id == Some(event.queue_entry_id) {
        debug!(
            "Buffer threshold reached for current passage {}, starting mixer",
            event.queue_entry_id
        );

        if let Err(e) = self.start_mixer_for_current_by_id(event.queue_entry_id).await {
            error!("Failed to start mixer on buffer threshold event: {}", e);
        }
    }
}
```

#### 4.3.4 Queue Advance Event Handler

**Location:** [queue_manager.rs:advance()](wkmp-ap/src/playback/queue_manager.rs)

**Current:**
```rust
pub fn advance(&mut self) {
    self.current = self.next.take();
    if let Some(next_entry) = self.queued.first() {
        self.next = Some(next_entry.clone());
        self.queued.remove(0);
    } else {
        self.next = None;
    }
}
```

**Refactored (returns promotion info):**
```rust
pub fn advance(&mut self) -> QueueAdvanceInfo {
    let previous_current = self.current.as_ref().map(|e| e.queue_entry_id);

    self.current = self.next.take();

    let newly_promoted_to_next = if let Some(next_entry) = self.queued.first() {
        self.next = Some(next_entry.clone());
        self.queued.remove(0);
        Some(next_entry.queue_entry_id)
    } else {
        self.next = None;
        None
    };

    QueueAdvanceInfo {
        previous_current,
        new_current: self.current.as_ref().map(|e| e.queue_entry_id),
        new_next: self.next.as_ref().map(|e| e.queue_entry_id),
        newly_promoted_to_next,
    }
}
```

**Event Handler in PlaybackEngine:**
```rust
async fn handle_queue_advance(&self, info: QueueAdvanceInfo) {
    debug!(
        "Queue advanced: current={:?}, next={:?}",
        info.new_current, info.new_next
    );

    // Trigger decode for newly promoted next passage
    if let Some(next_id) = info.newly_promoted_to_next {
        if let Some(next_entry) = self.get_queue_entry_by_id(next_id).await {
            if !self.buffer_manager.is_managed(next_id).await {
                debug!("Triggering decode for newly promoted next passage {}", next_id);
                self.request_decode(&next_entry, DecodePriority::Next, true).await?;
            }
        }
    }

    // Trigger decode for newly promoted queued passages
    let queue = self.queue.read().await;
    let max_queued = self.maximum_decode_streams.saturating_sub(2);
    for (i, queued_entry) in queue.queued().iter().enumerate().take(max_queued) {
        if !self.buffer_manager.is_managed(queued_entry.queue_entry_id).await {
            debug!(
                "Triggering decode for queued passage {} (position {})",
                queued_entry.queue_entry_id, i
            );
            self.request_decode(queued_entry, DecodePriority::Prefetch, true).await?;
        }
    }
}
```

### 4.4 Main Playback Loop Refactoring

**Before:**
```rust
async fn playback_loop(&self) -> Result<()> {
    let mut tick = interval(Duration::from_millis(100));

    loop {
        tick.tick().await;

        if !*self.running.read().await { break; }

        let state = self.state.get_playback_state().await;
        if state != PlaybackState::Playing { continue; }

        // Process queue (350 lines of proactive operations)
        self.process_queue().await?;
    }

    Ok(())
}
```

**After:**
```rust
async fn playback_loop(&self) -> Result<()> {
    let mut tick = interval(Duration::from_millis(100));

    loop {
        tick.tick().await;

        if !*self.running.read().await { break; }

        let state = self.state.get_playback_state().await;
        if state != PlaybackState::Playing { continue; }

        // Watchdog only - reactive safety net (~100 lines)
        self.watchdog_check().await?;
    }

    Ok(())
}
```

**Note:** Event handlers run independently (either in separate task or inline with triggering operations).

---

## 5. Testing Strategy

### 5.1 Test Principles

**CRITICAL:** Watchdog intervention in tests = event system failure

**Test Categories:**
1. **Event-driven unit tests** - Verify each event trigger
2. **Event-driven integration tests** - Verify end-to-end event flows
3. **Watchdog unit tests** - Verify watchdog detects stuck states
4. **Watchdog intervention tests** - Verify recovery logic (but fail test)

### 5.2 Test Environment Configuration

**Option A: Disable Watchdog in Tests**

```rust
#[cfg(test)]
impl PlaybackEngine {
    pub fn new_with_config(
        db_pool: Pool<Sqlite>,
        state: Arc<SharedState>,
        config: TestConfig,
    ) -> Self {
        // ...
        Self {
            // ...
            watchdog_enabled: config.watchdog_enabled, // false in most tests
        }
    }
}
```

**Option B: Fail Test on Watchdog Intervention**

```rust
async fn watchdog_check(&self) -> Result<()> {
    // ... checks ...

    if intervention_required {
        warn!("[WATCHDOG] {}", message);

        #[cfg(test)]
        {
            // In test mode, watchdog intervention = test failure
            panic!(
                "WATCHDOG INTERVENTION IN TEST: {}. \
                 This indicates event system failure.",
                message
            );
        }

        #[cfg(not(test))]
        {
            // In production, watchdog recovers gracefully
            self.perform_intervention().await?;
        }
    }

    Ok(())
}
```

**Recommendation:** Option B (fail test on intervention) - catches event bugs immediately.

### 5.3 Test Cases

#### 5.3.1 Event-Driven Decode Tests

**TC-ED-001: Decode Triggered on Enqueue**
- **Given:** Empty queue, no decode activity
- **When:** `enqueue_file()` called
- **Then:**
  - Decode request issued immediately (verified via spy/mock)
  - No watchdog intervention (test would fail if it did)
  - Timing: <1ms from enqueue to decode request

**TC-ED-002: Decode Triggered on Queue Advance**
- **Given:** Queue with current, next, and 3 queued passages
- **When:** Current passage completes, queue advances
- **Then:**
  - Decode request issued for newly promoted next (was queued[0])
  - No watchdog intervention
  - Timing: <1ms from advance to decode request

**TC-ED-003: Decode Priority Correct by Position**
- **Given:** Enqueue to different positions
- **When:** Enqueue to current slot, next slot, queued slot
- **Then:**
  - Current → DecodePriority::Immediate
  - Next → DecodePriority::Next
  - Queued → DecodePriority::Prefetch

#### 5.3.2 Event-Driven Mixer Startup Tests

**TC-ED-004: Mixer Starts on Buffer Threshold**
- **Given:** Current passage, mixer idle, buffer <3s
- **When:** Buffer reaches 3000ms (via push_samples)
- **Then:**
  - BufferThresholdReached event emitted
  - Mixer startup initiated within 1ms
  - PassageStarted event emitted
  - No watchdog intervention

**TC-ED-005: Mixer Already Playing - No Duplicate Start**
- **Given:** Mixer already playing passage A
- **When:** Buffer threshold reached for passage B
- **Then:**
  - No mixer restart attempted
  - Current playback uninterrupted

#### 5.3.3 Watchdog Detection Tests

**TC-WD-001: Watchdog Detects Missing Current Buffer**
- **Given:** Current passage in queue, no buffer exists
- **When:** Watchdog check runs (100ms timer)
- **Then:**
  - WARN logged: "Current passage X has no buffer"
  - In production: Decode triggered
  - In test: Test fails (event system bug)

**TC-WD-002: Watchdog Detects Mixer Not Started**
- **Given:** Current passage, buffer ≥3s, mixer idle
- **When:** Watchdog check runs
- **Then:**
  - WARN logged: "Mixer idle with ready buffer"
  - In production: Mixer started
  - In test: Test fails (event system bug)

**TC-WD-003: Watchdog Detects Missing Next Buffer**
- **Given:** Current playing, next passage in queue, no buffer
- **When:** Watchdog check runs
- **Then:**
  - WARN logged: "Next passage X has no buffer"
  - In production: Decode triggered
  - In test: Test fails (event system bug)

#### 5.3.4 End-to-End Event Flow Tests

**TC-E2E-001: Complete Playback Flow (Event-Driven)**
- **Given:** Empty queue, audio output available
- **When:**
  1. Enqueue passage A
  2. Wait for buffer threshold
  3. Verify mixer starts
  4. Wait for passage completion
  5. Enqueue passage B during playback
- **Then:**
  - Each operation triggers next via events (no polling)
  - No watchdog interventions
  - Playback continuous and correct
- **Timing Verification:**
  - Enqueue → Decode start: <1ms
  - Buffer threshold → Mixer start: <1ms
  - PassageComplete → Queue advance: <1ms

**TC-E2E-002: Multi-Passage Queue Build (Event-Driven)**
- **Given:** Empty queue
- **When:** Enqueue 10 passages rapidly
- **Then:**
  - All decode requests triggered immediately
  - Priority correct (immediate, next, prefetch × 8)
  - No decode requests from watchdog
  - No watchdog interventions

#### 5.3.5 Watchdog Disabled Tests

**TC-WD-DISABLED-001: Event System Works Without Watchdog**
- **Given:** PlaybackEngine with watchdog disabled
- **When:** Complete playback flow (enqueue, play, advance)
- **Then:**
  - All operations complete successfully via events
  - No stuck states
  - Verifies event system is sufficient

### 5.4 Test Mocks and Spies

**DecoderWorkerSpy:**
```rust
struct DecoderWorkerSpy {
    decode_requests: Arc<Mutex<Vec<(Uuid, DecodePriority, Instant)>>>,
}

impl DecoderWorkerSpy {
    fn verify_decode_request(
        &self,
        queue_entry_id: Uuid,
        expected_priority: DecodePriority,
        max_latency_ms: u64,
    ) -> Result<(), String> {
        let requests = self.decode_requests.lock().unwrap();

        let request = requests.iter()
            .find(|(id, _, _)| *id == queue_entry_id)
            .ok_or_else(|| format!("No decode request for {}", queue_entry_id))?;

        let (_, priority, timestamp) = request;

        if *priority != expected_priority {
            return Err(format!(
                "Wrong priority: expected {:?}, got {:?}",
                expected_priority, priority
            ));
        }

        let latency_ms = timestamp.elapsed().as_millis() as u64;
        if latency_ms > max_latency_ms {
            return Err(format!(
                "Latency too high: {}ms (max: {}ms)",
                latency_ms, max_latency_ms
            ));
        }

        Ok(())
    }
}
```

**BufferManagerMock:**
```rust
struct BufferManagerMock {
    threshold_events: Arc<Mutex<Vec<(Uuid, u64)>>>,
}

impl BufferManagerMock {
    fn simulate_buffer_fill(&self, queue_entry_id: Uuid, target_ms: u64) {
        // Simulate gradual buffer fill
        for ms in (0..=target_ms).step_by(100) {
            self.push_samples_mock(queue_entry_id, ms);

            // Check threshold crossing (3000ms)
            if ms >= 3000 {
                self.emit_threshold_event(queue_entry_id, 3000);
            }
        }
    }
}
```

### 5.5 Regression Testing

**Add to existing test suite:**

**PROJ001 Chain Assignment Tests** (from Bug #3 work):
- Add event-driven decode verification to Test 11
- Verify no watchdog interventions during normal operations

**New Test Suite: Event-Driven Playback Tests**
- Location: `wkmp-ap/tests/event_driven_playback_tests.rs`
- Coverage: All TC-ED-* test cases above
- Execution time target: <3s

---

## 6. Implementation Plan (High-Level)

**Note:** Detailed implementation plan will be generated via `/plan` workflow.

**Phased Approach:**

### Phase 1: Event Infrastructure
- Add new events to WkmpEvent enum
- Implement event channel in BufferManager
- Add event handlers in PlaybackEngine
- Tests: Event emission and routing

### Phase 2: Event-Driven Decode
- Refactor enqueue_file() to trigger decode
- Refactor queue advance to trigger decode
- Extract event handlers from process_queue()
- Tests: TC-ED-001, TC-ED-002, TC-ED-003

### Phase 3: Event-Driven Mixer Startup
- Add threshold detection to BufferManager.push_samples()
- Extract start_mixer_for_current() method
- Implement buffer threshold event handler
- Tests: TC-ED-004, TC-ED-005

### Phase 4: Watchdog Refactoring
- Rename process_queue() → watchdog_check()
- Remove proactive operations (keep detection only)
- Add WARN logging for interventions
- Tests: TC-WD-001, TC-WD-002, TC-WD-003

### Phase 5: Test Infrastructure
- Add watchdog_enabled config flag
- Implement test-mode panic on intervention
- Create DecoderWorkerSpy and BufferManagerMock
- Tests: TC-WD-DISABLED-001

### Phase 6: Integration and Validation
- End-to-end testing with event-driven system
- Performance validation (latency measurements)
- Regression testing (existing test suite)
- Tests: TC-E2E-001, TC-E2E-002

### Phase 7: Documentation
- Update SPEC028 with event-driven architecture
- Document watchdog purpose and logging format
- Update test suite README
- Add architecture diagrams

---

## 7. Success Criteria

**Functional:**
- ✓ All queue operations trigger events (no polling)
- ✓ Decode requests issued <1ms after trigger
- ✓ Mixer starts <1ms after buffer threshold
- ✓ Watchdog detects stuck states (if they occur)
- ✓ Watchdog logs WARN on intervention
- ✓ Watchdog restores proper state

**Testing:**
- ✓ All event-driven paths covered by unit tests
- ✓ End-to-end event flows validated
- ✓ Watchdog intervention in test = test failure
- ✓ Existing test suite passes (no regressions)
- ✓ New test suite execution time <3s

**Quality:**
- ✓ CPU usage reduced (no unnecessary polling)
- ✓ Latency improved (event-driven vs. polling)
- ✓ Code complexity reduced (~250 lines eliminated)
- ✓ Architecture consistency (fully event-driven)

**Documentation:**
- ✓ SPEC028 updated with event-driven design
- ✓ Watchdog purpose and behavior documented
- ✓ Test strategy documented
- ✓ Migration guide for developers

---

## 8. Risk Assessment

### Risk 1: Event System Complexity

**Failure Modes:**
1. Event emitted but handler not registered → Watchdog detects, logs WARN
2. Event handler throws exception → Watchdog detects, logs WARN
3. Race condition between event and state check → Watchdog detects, logs WARN

**Mitigation:**
- Comprehensive unit tests for each event path
- Watchdog provides safety net
- Test mode fails fast on watchdog intervention

**Residual Risk:** Low (watchdog catches all failure modes)

### Risk 2: Watchdog False Positives

**Failure Modes:**
1. Watchdog intervenes unnecessarily (event system working but timing issue)
2. Watchdog WARN spam in logs

**Mitigation:**
- Careful state checking logic (avoid race conditions)
- Test event-driven system thoroughly before deployment
- Monitor production logs for unexpected WARN messages

**Residual Risk:** Low (can tune watchdog checks based on production data)

### Risk 3: Performance Regression

**Failure Modes:**
1. Event overhead higher than expected
2. Increased context switching

**Mitigation:**
- Performance benchmarking before/after
- Event emission is lightweight (tokio::broadcast)
- Fewer unnecessary operations overall

**Residual Risk:** Very Low (event-driven should be faster than polling)

### Risk 4: Test Suite Complexity

**Failure Modes:**
1. Tests too brittle (fail on timing variations)
2. Test mocks don't match production behavior

**Mitigation:**
- Use reasonable timing thresholds (1ms for event propagation)
- Test against real components where possible
- Watchdog disabled tests validate core functionality

**Residual Risk:** Low-Medium (typical for event-driven testing)

---

## 9. Alternatives Considered

### Alternative 1: Keep 100ms Polling as Primary Mechanism

**Analysis:** Continue using process_queue() for proactive operations, no event system.

**Pros:**
- No code changes required
- Well-understood behavior

**Cons:**
- 0-100ms random latency for all operations
- Unnecessary CPU usage (polling when nothing to do)
- Architectural inconsistency (markers are event-driven)

**Decision:** Rejected - polling is technical debt

### Alternative 2: Remove Watchdog Entirely (100% Event-Driven)

**Analysis:** Eliminate playback_loop, rely solely on events.

**Pros:**
- Cleaner architecture
- No watchdog overhead

**Cons:**
- No safety net if event system fails
- Difficult to debug stuck states
- Higher risk in production

**Decision:** Rejected - watchdog provides valuable safety and diagnostics

### Alternative 3: Longer Watchdog Interval (e.g., 1 second)

**Analysis:** Reduce watchdog frequency to minimize overhead.

**Pros:**
- Lower CPU usage

**Cons:**
- Slower recovery if event system fails
- Less responsive to stuck states

**Decision:** Rejected - 100ms is already very efficient, provides quick recovery

---

## 10. Open Questions

**Q1:** Should watchdog interventions be exposed via telemetry/metrics?

**A1:** Yes, add counter metric for production monitoring.

**Implementation Details:**
- **Metric type:** Counter (increments on each intervention)
- **Metric name:** `watchdog_interventions_total`
- **Labels/dimensions:**
  - `intervention_type`: `missing_current_buffer`, `mixer_not_started`, `missing_next_buffer`, `missing_queued_buffer`
  - `queue_entry_id`: UUID of affected entry (optional, for detailed debugging)
- **Exposure:**
  - Internal metrics endpoint (for monitoring dashboards)
  - Log correlation via request ID/timestamp
- **Alerting:**
  - Alert threshold: >0 interventions in production (indicates event system bug)
  - Severity: Warning (system recovers, but bug needs investigation)

**Usage:**
- Production: Monitor for event system failures
- Development: Validate event-driven refactoring (should be zero)
- Testing: Fail tests if counter increments (watchdog intervention = event bug)

**Rationale:**
- Quantifies event system reliability
- Enables proactive bug detection
- Supports A/B testing of event system changes
- Provides data for watchdog interval tuning

**Q2:** Should watchdog be configurable (interval, enabled/disabled)?

**A2:** Enabled/disabled useful for testing. Interval should be implemented as a parameter in the database settings table with default value of 100ms, range of 10ms to 2000ms, utilizing the same implementation as the DBD-PARAM-### parameters.

**Implementation Details:**
- Setting key: `watchdog_interval_ms`
- Default: 100ms
- Valid range: 10ms (fast recovery) to 2000ms (minimal overhead)
- Type: u64 (milliseconds)
- Validation: Clamp to range, log warning if out of bounds
- Hot-reload: Support runtime changes without restart

**Rationale:**
- 10ms minimum: Faster than this wastes CPU, event system should be <1ms
- 2000ms maximum: Longer delays reduce watchdog effectiveness
- Configurable: Allows tuning for different deployment scenarios (high-performance vs. low-power)
- Database setting: Consistent with other playback parameters (maximum_decode_streams, etc.)

**Q3:** Should buffer threshold be configurable per-passage or globally?

**A3:** Keep global for simplicity. Implemented as a parameter in the database settings table with default value of 3000ms, range of 100ms to 12000ms, utilizing the same implementation as the DBD-PARAM-### parameters.

**Implementation Details:**
- Setting key: `minimum_playback_buffer_ms`
- Default: 3000ms (3 seconds - current hardcoded value)
- Valid range: 100ms (minimal buffer) to 12000ms (12 seconds for very high quality)
- Type: u64 (milliseconds)
- Validation: Clamp to range, log warning if out of bounds
- Hot-reload: Support runtime changes without restart

**Rationale:**
- 100ms minimum: Below this risks underruns during decode/IO delays
- 12000ms maximum: Higher values waste memory, delay playback start unnecessarily
- Configurable: Allows tuning based on:
  - Storage speed (slower storage → higher buffer needed)
  - Network streaming scenarios (higher latency → higher buffer)
  - User preference (instant start vs. safer playback)
- Global setting: Simpler than per-passage, sufficient for most use cases
- Database setting: Consistent with other playback parameters (maximum_decode_streams, etc.)

**Q4:** How to handle edge case where buffer threshold crossed multiple times (rewind)?

**A4:** Event handler checks mixer state - only starts if idle. Multiple events safe.

---

## 11. Appendices

### Appendix A: Code References

**Current Implementation:**
- [core.rs:1292-1318](../wkmp-ap/src/playback/engine/core.rs#L1292-L1318) - playback_loop
- [core.rs:1324-1680](../wkmp-ap/src/playback/engine/core.rs#L1324-L1680) - process_queue
- [queue.rs:241-259](../wkmp-ap/src/playback/engine/queue.rs#L241-L259) - enqueue_file (Bug #3 fix)

**Related:**
- [SPEC028](../docs/SPEC028-playback_orchestration.md) - Current design
- [decoder_worker.rs](../wkmp-ap/src/playback/decoder_worker.rs) - Decoder priority selection
- [buffer_manager.rs](../wkmp-ap/src/playback/buffer_manager.rs) - Buffer lifecycle

### Appendix B: Performance Targets

| Metric | Current (Polling) | Target (Event-Driven) |
|--------|-------------------|----------------------|
| Enqueue → Decode start | 0-100ms | <1ms |
| Buffer ready → Mixer start | 0-100ms | <1ms |
| PassageComplete → Queue advance | <1ms (marker) | <1ms (unchanged) |
| Watchdog overhead | N/A (process_queue is primary) | <0.1ms per check |
| CPU usage (idle playback) | ~1% (100ms polling) | <0.1% (watchdog only) |

### Appendix C: Logging Format

**Event-Driven Operations (DEBUG level):**
```
[DEBUG] Enqueued abc123 at position Current, triggering decode with priority Immediate
[DEBUG] Buffer def456 crossed threshold: 2900ms → 3100ms
[DEBUG] Buffer threshold reached for current passage def456, starting mixer
[DEBUG] Queue advanced: current=Some(ghi789), next=Some(jkl012)
```

**Watchdog Interventions (WARN level):**
```
[WARN] [WATCHDOG] Event system failure detected: Current passage abc123 has no buffer. Intervention: Requesting immediate decode. This indicates a bug in event-driven logic.
[WARN] [WATCHDOG] Event system failure detected: Mixer idle with ready buffer (def456). Intervention: Starting mixer now. This indicates a bug in event-driven logic.
```

**Test Failures:**
```
thread 'test_event_driven_enqueue' panicked at 'WATCHDOG INTERVENTION IN TEST: Current passage has no buffer. This indicates event system failure.'
```

---

**END OF SPECIFICATION**

**Next Step:** Run `/plan SPEC_event_driven_playback_refactor.md` to generate detailed implementation plan with:
- Requirement traceability matrix
- Acceptance test specifications (Given/When/Then)
- Increment breakdown with tasks
- Test-first implementation order
