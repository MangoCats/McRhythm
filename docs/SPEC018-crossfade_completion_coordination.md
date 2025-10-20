# SPEC018: Crossfade Completion Coordination

**Document Type:** Tier 2 - Design Specification
**Version:** 1.0
**Date:** 2025-10-20
**Status:** Draft → Implementation
**Author:** System Architecture Team

---

## Metadata

**Parent Documents (Tier 1):**
- [REQ001-requirements.md](REQ001-requirements.md) - XFD-COMP requirement family
- [REQ002-entity_definitions.md](REQ002-entity_definitions.md) - Passage entity definition

**Related Documents (Tier 2):**
- [SPEC002-crossfade.md](SPEC002-crossfade.md) - Crossfade timing and curves (primary dependency)
- [SPEC001-architecture.md](SPEC001-architecture.md) - Audio Player microservice architecture
- [SPEC016-decoder_buffer_design.md](SPEC016-decoder_buffer_design.md) - Buffer state machine
- [SPEC017-sample_rate_conversion.md](SPEC017-sample_rate_conversion.md) - Tick-based timing

**Child Documents (Tier 3):**
- [IMPL001-database_schema.md](IMPL001-database_schema.md) - Queue table structure
- [IMPL002-coding_conventions.md](IMPL002-coding_conventions.md) - Rust conventions

**Requirement Traceability:**
- **[XFD-COMP-010]** Crossfade completion must not interrupt incoming passage
- **[XFD-COMP-020]** Queue advancement must synchronize with crossfade transitions
- **[XFD-COMP-030]** Mixer state must remain consistent during crossfade-to-single transitions
- **[SSD-MIX-060]** Passage completion detection (extended for crossfades)

---

## Executive Summary

This specification addresses a critical gap in the crossfade state machine: **coordination between mixer-level crossfade completion and engine-level queue advancement**.

**Problem:** When a crossfade completes (fade-out and fade-in finish), the mixer correctly transitions the incoming passage to be the new current passage. However, the engine remains unaware of this transition, leading to incorrect queue advancement that stops and restarts the incoming passage.

**Solution:** Implement explicit crossfade completion signaling from mixer to engine, allowing queue advancement to occur **without interrupting** the incoming passage that is already playing.

**Impact:** Fixes BUG-003 (wrong passage playing) and ensures seamless crossfade-to-single playback transitions.

---

## Background

### Current Crossfade Implementation

**SPEC002-crossfade.md** defines the crossfade state machine with three mixer states:
1. **None** - No audio playing
2. **SinglePassage** - One passage playing (no crossfade)
3. **Crossfading** - Two passages overlapping with fade curves

**Crossfade Completion Logic (Current):**

```rust
// mixer.rs:437-453
if fade_out_progress >= fade_out_duration_samples
    && fade_in_progress >= fade_in_duration_samples
{
    self.state = MixerState::SinglePassage {
        buffer: next_buffer,
        passage_id: next_passage_id,  // Incoming passage becomes current
        position: next_position,
        ...
    };
}
```

This internal transition is **correct** but **not visible** to the engine.

### The Gap

**Missing Coordination:** The engine's `process_queue()` loop uses `mixer.is_current_finished()` to detect passage completion, but this method returns `false` during `Crossfading` state:

```rust
// mixer.rs:555-569
pub async fn is_current_finished(&self) -> bool {
    match &self.state {
        MixerState::SinglePassage { buffer, position, .. } => {
            buf.is_exhausted(*position)
        }
        _ => false,  // ❌ Returns false during Crossfading
    }
}
```

**Consequence:** The engine never knows when the outgoing passage finishes during a crossfade, leading to timing bugs.

---

## Requirements

### Functional Requirements

#### [XFD-COMP-010] Crossfade Completion Detection
**Priority:** Critical
**Description:** The engine MUST be notified when a crossfade transition completes, identifying which passage has finished fading out.

**Acceptance Criteria:**
- Engine receives signal when `Crossfading → SinglePassage` transition occurs
- Signal includes the outgoing passage ID (the one that faded out)
- Signal is delivered exactly once per crossfade completion
- Signal is delivered atomically with the mixer state transition

#### [XFD-COMP-020] Queue Advancement Without Mixer Restart
**Priority:** Critical
**Description:** When a crossfade completes, the engine MUST advance the queue WITHOUT stopping and restarting the mixer.

**Acceptance Criteria:**
- Queue advancement removes the outgoing passage
- Incoming passage continues playing seamlessly (no stop/restart)
- Buffer cleanup happens for outgoing passage only
- PassageCompleted event emitted for outgoing passage
- No duplicate PassageStarted events for incoming passage

#### [XFD-COMP-030] State Consistency During Transition
**Priority:** High
**Description:** Mixer state, queue state, and buffer state MUST remain consistent during crossfade completion.

**Acceptance Criteria:**
- Mixer's current passage ID matches queue's current entry after advancement
- Buffer manager holds buffer for incoming passage
- Outgoing passage's buffer is removed after cleanup
- No intermediate "idle" state where mixer.current_passage_id is None

### Non-Functional Requirements

#### [XFD-COMP-NFR-010] Performance
**Description:** Crossfade completion signaling MUST NOT add measurable latency to audio frame generation.

**Acceptance Criteria:**
- Completion flag check takes < 100ns
- No heap allocations during flag operations
- No mutex contention in audio callback path

#### [XFD-COMP-NFR-020] Thread Safety
**Description:** Crossfade completion signaling MUST be thread-safe between audio callback and engine loop.

**Acceptance Criteria:**
- Flag is atomic or behind appropriate synchronization
- No data races under concurrent access
- Miri passes for completion signaling code

---

## Design

### Architecture Overview

**Components:**

```
┌─────────────────────────────────────────────────────────────┐
│                      PlaybackEngine                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  process_queue() Loop (100ms interval)              │   │
│  │  ┌──────────────────────────────────────────────┐  │   │
│  │  │ 1. Check crossfade_completed_passage flag   │  │   │
│  │  │ 2. If set:                                   │  │   │
│  │  │    - Emit PassageCompleted(outgoing)        │  │   │
│  │  │    - Advance queue (outgoing → removed)     │  │   │
│  │  │    - Cleanup outgoing buffer                │  │   │
│  │  │    - DO NOT stop mixer                      │  │   │
│  │  └──────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ Checks flag via:
                            │ mixer.write().await.take_crossfade_completed()
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     CrossfadeMixer                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  get_next_frame()                                   │   │
│  │  ┌──────────────────────────────────────────────┐  │   │
│  │  │ If crossfade complete:                       │  │   │
│  │  │  - Transition Crossfading → SinglePassage   │  │   │
│  │  │  - Set crossfade_completed_passage flag     │  │   │
│  │  │  - Continue playing incoming passage        │  │   │
│  │  └──────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  crossfade_completed_passage: Option<Uuid>  ← State flag  │
└─────────────────────────────────────────────────────────────┘
```

### Data Structures

#### Mixer State Extension

**Location:** `wkmp-ap/src/playback/pipeline/mixer.rs`

**Modification:**

```rust
pub struct CrossfadeMixer {
    state: MixerState,
    sample_rate: u32,
    event_tx: Option<broadcast::Sender<PlaybackEvent>>,

    // NEW: Crossfade completion signaling
    /// Passage ID of outgoing passage when crossfade just completed
    /// Set by get_next_frame() when Crossfading → SinglePassage
    /// Consumed by engine via take_crossfade_completed()
    crossfade_completed_passage: Option<Uuid>,

    // ... existing fields ...
}
```

**Rationale:**
- `Option<Uuid>` represents three states:
  - `None` - No crossfade completion pending
  - `Some(passage_id)` - Outgoing passage completed, needs queue advancement
- Simple atomic operation via `take()` method
- No heap allocation, minimal memory overhead (16 bytes)

### API Changes

#### New Method: take_crossfade_completed()

**Signature:**
```rust
impl CrossfadeMixer {
    /// Check if a crossfade just completed and consume the signal
    ///
    /// **[XFD-COMP-010]** Crossfade completion detection
    ///
    /// Returns the passage ID of the outgoing passage that finished fading out.
    /// This should be called before is_current_finished() in the engine loop.
    ///
    /// # Returns
    /// - `Some(passage_id)` if a crossfade just completed
    /// - `None` if no crossfade completion pending
    ///
    /// # Side Effects
    /// Clears the internal flag, so subsequent calls return None until
    /// the next crossfade completes.
    ///
    /// # Thread Safety
    /// This method requires `&mut self`, so it's naturally serialized by
    /// Rust's borrow checker. Only one thread can call this at a time.
    pub fn take_crossfade_completed(&mut self) -> Option<Uuid> {
        self.crossfade_completed_passage.take()
    }
}
```

**Usage Example:**
```rust
// In engine.rs process_queue()
let mut mixer = self.mixer.write().await;
if let Some(completed_passage_id) = mixer.take_crossfade_completed() {
    drop(mixer); // Release lock before expensive operations

    // Handle crossfade completion
    self.handle_crossfade_completion(completed_passage_id).await?;
    return Ok(());
}
drop(mixer);

// Continue with normal completion check...
```

### Implementation Details

#### Step 1: Set Completion Flag on Transition

**Location:** `mixer.rs:437-453` (in `get_next_frame()` method)

**Modification:**

```rust
// Current code:
if *fade_out_progress >= *fade_out_duration_samples
    && *fade_in_progress >= *fade_in_duration_samples
{
    let new_passage_id = *next_passage_id;
    let new_position = *next_position;
    let new_buffer = next_buffer.clone();

    // NEW: Store outgoing passage ID before transition
    let outgoing_passage_id = *current_passage_id;

    self.state = MixerState::SinglePassage {
        buffer: new_buffer,
        passage_id: new_passage_id,
        position: new_position,
        fade_in_curve: None,
        fade_in_duration_samples: 0,
    };

    // NEW: Signal completion to engine
    self.crossfade_completed_passage = Some(outgoing_passage_id);

    debug!(
        "Crossfade completed: {} → {} (outgoing faded out)",
        outgoing_passage_id, new_passage_id
    );
}
```

**Traceability:**
- **[XFD-COMP-010]** - Sets completion flag atomically with transition
- **[SPEC002:XFD-TRAN-020]** - Maintains crossfade transition timing

#### Step 2: Check Completion Flag in Engine

**Location:** `engine.rs:1246-1370` (in `process_queue()` loop)

**New Code Block (insert BEFORE existing `is_current_finished()` check):**

```rust
// **[XFD-COMP-010]** Check for crossfade completion BEFORE normal completion
// This handles the case where outgoing passage finished during an active crossfade
let crossfade_completed_id = {
    let mut mixer = self.mixer.write().await;
    mixer.take_crossfade_completed()
};

if let Some(completed_id) = crossfade_completed_id {
    debug!("Processing crossfade completion for passage {}", completed_id);

    // Verify this is the current passage in queue
    let queue = self.queue.read().await;
    if let Some(current) = queue.current() {
        if current.queue_entry_id == completed_id {
            let passage_id_opt = current.passage_id;
            drop(queue);

            info!("Passage {} completed (via crossfade)", completed_id);

            // **[Event-PassageCompleted]** Emit completion event
            self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
                passage_id: passage_id_opt.unwrap_or_else(|| Uuid::nil()),
                completed: true,
                timestamp: chrono::Utc::now(),
            });

            // **[XFD-COMP-020]** Advance queue WITHOUT stopping mixer
            let mut queue_write = self.queue.write().await;
            queue_write.advance();
            drop(queue_write);

            // Remove from database
            if let Err(e) = crate::db::queue::remove_from_queue(&self.db_pool, completed_id).await {
                warn!("Failed to remove completed passage from database: {}", e);
            } else {
                info!("Queue advanced (crossfade) and synced to database (removed {})", completed_id);
            }

            // Update audio_expected flag
            self.update_audio_expected_flag().await;

            // **[XFD-COMP-020]** Clean up outgoing buffer (incoming continues playing)
            if let Some(p_id) = passage_id_opt {
                self.buffer_manager.remove(p_id).await;
            }

            // ✅ CRITICAL: DO NOT stop mixer - incoming passage is still playing!
            debug!("Crossfade completion handled - mixer continues playing incoming passage");

            return Ok(());
        } else {
            warn!(
                "Crossfade completed ID {} doesn't match queue current {}",
                completed_id, current.queue_entry_id
            );
        }
    }
}

// Continue with normal completion check for non-crossfade cases...
let mixer = self.mixer.read().await;
let is_finished = mixer.is_current_finished().await;
// ... rest of existing code ...
```

**Traceability:**
- **[XFD-COMP-010]** - Detects crossfade completion
- **[XFD-COMP-020]** - Advances queue without mixer restart
- **[XFD-COMP-030]** - Maintains state consistency

### Error Handling

#### Edge Case 1: Completion ID Mismatch

**Scenario:** Crossfade completion signal arrives but queue.current() doesn't match.

**Cause:** Race condition or out-of-order event processing.

**Handling:** Log warning and skip processing. Let normal completion path handle it later.

```rust
if current.queue_entry_id != completed_id {
    warn!(
        "Crossfade completion ID mismatch: expected {}, got {}",
        current.queue_entry_id, completed_id
    );
    // Don't advance queue - will be handled on next iteration
}
```

#### Edge Case 2: Multiple Crossfades in Quick Succession

**Scenario:** Second crossfade completes before engine processes first completion.

**Cause:** Very short passages with overlapping crossfades.

**Handling:** Only store most recent completion. Earlier completion will be detected when queue advances naturally.

**Note:** This is acceptable because crossfades must be at least 1-2 seconds, giving engine ample time (100ms polling) to process each completion.

#### Edge Case 3: Mixer Stopped During Crossfade

**Scenario:** User skips to next track while crossfade is active.

**Cause:** External control command.

**Handling:** `stop()` method clears the completion flag:

```rust
pub fn stop(&mut self) {
    self.state = MixerState::None;
    self.crossfade_completed_passage = None;  // Clear any pending signal
}
```

---

## Testing Strategy

### Unit Tests

#### Test 1: Completion Flag Set on Transition

**File:** `mixer.rs` (in `#[cfg(test)] mod tests`)

```rust
#[tokio::test]
async fn test_crossfade_sets_completion_flag() {
    let mut mixer = CrossfadeMixer::new();
    let passage1_id = Uuid::new_v4();
    let passage2_id = Uuid::new_v4();

    let buffer1 = create_test_buffer(passage1_id, 44100, 0.5); // 1 second
    let buffer2 = create_test_buffer(passage2_id, 44100, 0.5);

    // Start passage 1
    mixer.start_passage(buffer1, passage1_id, None, 0).await;

    // Start crossfade (5 seconds = 220,500 frames)
    mixer.start_crossfade(
        buffer2,
        passage2_id,
        FadeCurve::Logarithmic,
        FadeCurve::Logarithmic,
        5000, // 5 seconds
    ).await.unwrap();

    // Read frames until crossfade completes
    while matches!(mixer.get_state(), MixerState::Crossfading { .. }) {
        mixer.get_next_frame().await;
    }

    // Verify completion flag set
    let completed = mixer.take_crossfade_completed();
    assert_eq!(
        completed,
        Some(passage1_id),
        "Should signal passage 1 (outgoing) completed"
    );

    // Verify flag consumed (subsequent calls return None)
    assert_eq!(
        mixer.take_crossfade_completed(),
        None,
        "Flag should be cleared after take()"
    );
}
```

#### Test 2: Flag Cleared on stop()

```rust
#[tokio::test]
async fn test_stop_clears_completion_flag() {
    let mut mixer = CrossfadeMixer::new();
    let passage1_id = Uuid::new_v4();
    let passage2_id = Uuid::new_v4();

    // Setup crossfade
    // ... (similar to Test 1) ...

    // Stop mixer during crossfade
    mixer.stop();

    // Verify flag cleared
    assert_eq!(
        mixer.take_crossfade_completed(),
        None,
        "Completion flag should be cleared after stop()"
    );
}
```

### Integration Tests

#### Test 3: No Duplicate Playback After Crossfade

**File:** `tests/crossfade_completion_integration.rs`

```rust
#[tokio::test]
async fn test_three_passages_with_crossfades_no_duplicate() {
    let engine = create_test_engine().await;
    let mut event_counter = EventCounter::new();

    // Subscribe to events
    let mut event_rx = engine.subscribe_events();
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            event_counter.record_event(&event);
        }
    });

    // Enqueue 3 passages
    let p1 = engine.enqueue_file("test1.mp3").await.unwrap();
    let p2 = engine.enqueue_file("test2.mp3").await.unwrap();
    let p3 = engine.enqueue_file("test3.mp3").await.unwrap();

    // Wait for all to complete
    tokio::time::sleep(Duration::from_secs(600)).await;

    // Verify each passage played exactly once
    event_counter.assert_passage_played_exactly_once(p1, "Passage 1");
    event_counter.assert_passage_played_exactly_once(p2, "Passage 2");
    event_counter.assert_passage_played_exactly_once(p3, "Passage 3");
}
```

#### Test 4: Queue Advances Without Mixer Restart

```rust
#[tokio::test]
async fn test_queue_advances_seamlessly_on_crossfade() {
    let engine = create_test_engine().await;

    // Enqueue 2 passages
    let p1 = engine.enqueue_file("test1.mp3").await.unwrap();
    let p2 = engine.enqueue_file("test2.mp3").await.unwrap();

    // Wait for crossfade to complete
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Verify:
    // 1. Queue length = 1 (p1 removed)
    assert_eq!(engine.queue_len().await, 1);

    // 2. Mixer still playing (not idle)
    let mixer_idle = {
        let mixer = engine.mixer.read().await;
        mixer.get_current_passage_id().is_none()
    };
    assert!(!mixer_idle, "Mixer should still be playing after crossfade");

    // 3. Mixer playing p2
    let current_passage = {
        let mixer = engine.mixer.read().await;
        mixer.get_current_passage_id()
    };
    assert_eq!(current_passage, Some(p2));
}
```

---

## Performance Analysis

### Memory Overhead

**Per Mixer Instance:**
- 1 × `Option<Uuid>` = 24 bytes (16 bytes UUID + 8 bytes discriminant)

**System-Wide:**
- 1 mixer per audio player instance
- Total overhead: ~24 bytes

**Conclusion:** Negligible memory impact.

### CPU Overhead

**Flag Check Operation:**
```rust
self.crossfade_completed_passage.take()  // Cost: ~5ns (one compare + one write)
```

**Frequency:** Once per 100ms (engine loop interval)

**CPU Impact:** < 0.00001% CPU time

**Conclusion:** No measurable performance impact.

### Timing Analysis

**Crossfade Completion Latency:**

```
T0: Crossfade completes (get_next_frame)
T1: Flag set (same function call, ~0ns latency)
T2: Engine checks flag (next iteration, max 100ms later)
T3: Queue advanced (within same iteration, ~1-5ms)
```

**Total Latency:** 0-100ms (determined by engine polling interval)

**Acceptable:** User doesn't notice because:
- Incoming passage continues playing seamlessly
- No audio interruption occurs
- 100ms is well under perceptual threshold (~200ms)

---

## Backward Compatibility

### API Compatibility

**Additions Only:** This design adds new functionality without modifying existing APIs.

**Existing Code:**
- `is_current_finished()` behavior unchanged
- `stop()` behavior extended (clears new flag)
- No breaking changes to public mixer API

**Migration:** None required. Existing code continues to work.

### State Machine Compatibility

**No Changes to State Definitions:**
- `MixerState` enum unchanged
- Transition rules unchanged
- Audio processing unchanged

**Extended Behavior:**
- Crossfade completion now signals engine
- Engine has additional completion path (crossfade vs normal)

---

## Future Enhancements

### Enhancement 1: Crossfade Completion Events

**Description:** Emit dedicated SSE event when crossfade completes, distinct from PassageCompleted.

**Benefits:**
- UI can visualize crossfade transitions
- Analytics can track crossfade quality metrics

**Implementation:**
```rust
// In mixer.rs, when setting completion flag:
if let Some(tx) = &self.event_tx {
    let _ = tx.send(PlaybackEvent::CrossfadeCompleted {
        outgoing_passage_id,
        incoming_passage_id: new_passage_id,
        timestamp: Instant::now(),
    });
}
```

### Enhancement 2: Crossfade Queue Optimization

**Description:** Pre-advance queue when crossfade starts (not when it completes).

**Benefits:**
- Queue display updates earlier (more responsive UI)
- Simplifies completion logic

**Trade-offs:**
- More complex rollback if crossfade is interrupted
- Requires "tentative" queue state

**Defer:** Out of scope for current bug fix.

---

## References

### Specifications
- [SPEC002-crossfade.md](SPEC002-crossfade.md) - Primary crossfade specification
- [SPEC016-decoder_buffer_design.md](SPEC016-decoder_buffer_design.md) - Buffer lifecycle
- [SPEC001-architecture.md](SPEC001-architecture.md) - Overall system architecture

### Implementation
- `wkmp-ap/src/playback/pipeline/mixer.rs` - CrossfadeMixer implementation
- `wkmp-ap/src/playback/engine.rs` - Engine process_queue() loop
- `wkmp-ap/src/playback/queue_manager.rs` - Queue advancement logic

### Bug Reports
- `docs/validation/BUG-ANALYSIS-003-crossfade-completion.md` - Root cause analysis
- `docs/validation/BUG-FIX-003-crossfade-completion-fix.md` - Original fix proposal

---

## Change History

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2025-10-20 | System Architecture | Initial specification following GOV001 hierarchy |

---

## Approval

**Technical Review:** [Pending]
**Architecture Review:** [Pending]
**Implementation Authorization:** [Pending]

**Approval Criteria:**
- [ ] Design addresses all [XFD-COMP-*] requirements
- [ ] Test coverage plan is comprehensive
- [ ] Performance impact is acceptable
- [ ] No backward compatibility issues
- [ ] Documentation follows GOV001 standards
