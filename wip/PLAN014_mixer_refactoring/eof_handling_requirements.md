# End-of-File Handling Requirements

**Date:** 2025-01-30
**Status:** New Requirements - Clarification Received
**Context:** Increment 5 completion, preparing for Increment 6

---

## Requirements Summary

**Source:** User clarification during testing phase

### REQ-MIX-EOF-001: Unreachable Marker Signaling

**Requirement:**
When event markers are set for ticks beyond end-of-file (EOF), mixer shall:
1. Signal that EOF has been reached
2. Include designators of events that never occurred
3. Allow PlaybackEngine to handle unreachable events appropriately

**Example Scenario:**
```
Passage duration: 10,000 frames (end at tick 10,000)
Markers set: tick 8,000, tick 12,000, tick 15,000

Mixer behavior:
- Mix to tick 10,000 (EOF reached)
- Emit events: [EOF reached, unreachable: [tick 12,000, tick 15,000]]
```

### REQ-MIX-EOF-002: Early EOF Before Lead-Out

**Requirement:**
When mixer reaches EOF before calculated lead-out point AND next passage exists:
- Immediately start playing next passage from its start time
- Do not wait for calculated lead-out/crossfade point

**Example Scenario:**
```
Current passage: 8,000 frames actual
Crossfade marker: tick 9,000 (calculated lead-out)
Next passage: Available in queue

Mixer behavior:
- Mix to tick 8,000 (EOF reached)
- Signal: EOF before crossfade point
- Immediately switch to next passage at tick 0
```

### REQ-MIX-EOF-003: Automatic Queue Advancement

**Requirement:**
When passage plays to EOF without explicit lead-out/end markers:
- Signal EOF reached
- Automatically start next passage if available
- No gap in audio output

**Example Scenario:**
```
Current passage: No explicit end markers
Mixer reaches EOF
Next passage: Available

Mixer behavior:
- Signal EOF reached
- Immediately start next passage
- Seamless transition (no silence gap)
```

---

## Implementation Impact

### Current Implementation Gaps

**Marker System (mixer.rs):**
- ✅ Markers stored and checked
- ❌ No EOF detection
- ❌ No unreachable marker tracking
- ❌ No automatic passage advancement

**Mix Methods:**
- ✅ Buffer underrun handling (fills with silence)
- ❌ Buffer underrun != EOF (decoder may still be running)
- ❌ No EOF event emission
- ❌ No next passage switch logic

### Required Changes

**1. New Event Types:**
```rust
pub enum MarkerEvent {
    PositionUpdate { position_ms: u64 },
    StartCrossfade { next_passage_id: Uuid },
    SongBoundary { new_song_id: Option<Uuid> },
    PassageComplete,

    // NEW: EOF handling events
    EndOfFile {
        unreachable_markers: Vec<PositionMarker>,
    },
    EndOfFileBeforeLeadOut {
        planned_crossfade_tick: i64,
        unreachable_markers: Vec<PositionMarker>,
    },
}
```

**2. EOF Detection:**
- Check `buffer_arc.is_exhausted()` AND decode complete
- Distinguish from temporary underrun (decoder still running)

**3. Unreachable Marker Collection:**
```rust
// After EOF detected at tick X:
let unreachable = self.markers
    .iter()
    .filter(|m| m.0.tick > X && m.0.passage_id == self.current_passage_id)
    .cloned()
    .collect();
```

**4. Automatic Passage Advancement:**
- Mixer needs reference to next passage ID (set by engine)
- When EOF reached: automatically switch to `next_passage_id`
- Call `set_current_passage(next_passage_id, 0)`
- Continue mixing without silence gap

---

## Architectural Questions

### Q1: Should Mixer Know About "Next Passage"?

**Current Architecture:**
- Mixer is "execution layer" - signals events, doesn't make decisions
- PlaybackEngine is "calculation layer" - manages queue, decides transitions

**Option A: Mixer Tracks Next Passage (Simpler EOF Handling)**
```rust
pub struct Mixer {
    current_passage_id: Option<Uuid>,
    next_passage_id: Option<Uuid>, // NEW
    // ...
}

// Engine sets:
mixer.set_next_passage(next_id);

// Mixer automatically switches on EOF
```

**Option B: Mixer Signals, Engine Decides (Current Architecture)**
```rust
// Mixer emits:
MarkerEvent::EndOfFile { unreachable_markers }

// Engine handles:
match event {
    MarkerEvent::EndOfFile { .. } => {
        if let Some(next_id) = queue.peek() {
            mixer.set_current_passage(next_id, 0);
        }
    }
}
```

**Recommendation:** Option B (preserve architectural separation)
- Mixer signals EOF, engine switches passages
- Maintains "execution vs. calculation" boundary
- Engine retains full queue management control

### Q2: How to Avoid Silence Gap?

**Problem:** If mixer signals EOF and waits for engine response, audio gap occurs

**Solution: Lookahead Pattern**
```rust
// Engine proactively tells mixer about next passage BEFORE current ends
mixer.set_next_passage_for_lookahead(next_id);

// Mixer behavior:
// 1. EOF reached
// 2. Emit EndOfFile event
// 3. Immediately switch to next passage (no wait)
// 4. Continue mixing
```

**Alternative: Mix Both Passages**
```rust
// When EOF detected, mix remainder from next passage in same call
let events = mixer.mix_with_eof_handling(
    &buffer_manager,
    current_id,
    next_id, // Optional
    &mut output
)?;
```

---

## Proposed Implementation Strategy

### Phase 1: EOF Detection (30 min)

**Add to Mixer:**
```rust
fn detect_eof(&self, buffer: &Arc<PlayoutRingBuffer>) -> bool {
    buffer.is_exhausted() // Assumes PlayoutRingBuffer has this method
}
```

**Update mix_single():**
```rust
// After frame reading loop:
if frames_read < frames_requested {
    let is_eof = buffer_arc.is_exhausted();
    if is_eof {
        // Collect unreachable markers
        let unreachable = self.collect_unreachable_markers();
        events.push(MarkerEvent::EndOfFile { unreachable_markers: unreachable });
    }
}
```

### Phase 2: Unreachable Marker Collection (30 min)

**Add method:**
```rust
fn collect_unreachable_markers(&mut self) -> Vec<PositionMarker> {
    let current_tick = self.current_tick;
    let current_passage = self.current_passage_id;

    // Drain markers beyond current tick for current passage
    self.markers
        .iter()
        .filter(|m| m.0.tick > current_tick && Some(m.0.passage_id) == current_passage)
        .map(|m| m.0.clone())
        .collect()
}
```

### Phase 3: Next Passage Lookahead (1 hour)

**Add to Mixer:**
```rust
pub struct Mixer {
    // ...
    next_passage_id: Option<Uuid>, // For automatic EOF handling
}

pub fn set_next_passage_for_lookahead(&mut self, next_id: Option<Uuid>) {
    self.next_passage_id = next_id;
}
```

**Update mix_single() EOF handling:**
```rust
if is_eof {
    let unreachable = self.collect_unreachable_markers();
    events.push(MarkerEvent::EndOfFile { unreachable_markers: unreachable });

    // If next passage available, automatically switch
    if let Some(next_id) = self.next_passage_id {
        self.set_current_passage(next_id, 0);
        self.next_passage_id = None; // Clear lookahead

        // Continue mixing from next passage to fill output buffer
        // (requires refactoring to support mid-call passage switch)
    }
}
```

### Phase 4: Testing (1-2 hours)

**New Tests:**
- `test_eof_with_unreachable_markers()`
- `test_eof_before_crossfade_point()`
- `test_automatic_passage_advance_on_eof()`
- `test_eof_without_next_passage()`
- `test_mid_buffer_passage_switch()`

---

## Integration with Increment 6

**Impact on Integration Tests:**
- Single passage playback tests need EOF handling
- Crossfade tests need "EOF before crossfade" case
- New test suite needed for EOF scenarios

**Recommended Approach:**
1. Complete Increment 6 tests WITHOUT EOF handling first (validates basic flow)
2. Add EOF handling as Increment 6b (30 min implementation + 1 hour testing)
3. Update Increment 6 tests to handle EOF cases

---

## Questions for User

1. **PlayoutRingBuffer API:** Does `PlayoutRingBuffer` have an `is_exhausted()` method to distinguish EOF from temporary underrun?

2. **Mid-Call Passage Switch:** Should mixer switch passages mid-call to avoid gaps, or is it acceptable to:
   - Fill remainder with silence
   - Emit EOF event
   - Let next mix call start new passage

3. **Unreachable Marker Format:** Should `EndOfFile` event include:
   - Full `PositionMarker` objects (with tick, passage_id, event_type)?
   - Just the ticks?
   - Just the event types?

4. **Crossfade Implications:** When EOF reached before crossfade point, should mixer:
   - Emit `EndOfFileBeforeLeadOut` with planned tick?
   - Just emit regular `EndOfFile`?
   - Attempt abbreviated crossfade with remaining frames?

---

**Document Created:** 2025-01-30
**Status:** Requirements Documented - Awaiting Implementation Decision
**Estimated Implementation:** 2-3 hours (EOF detection + unreachable markers + lookahead + tests)
