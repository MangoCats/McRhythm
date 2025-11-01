# ADR-002: Event-Driven Position Tracking via Mixer Markers

**Status:** Accepted
**Date:** 2025-01-30
**Context:** PLAN014 Mixer Refactoring (Sub-Increment 4a)
**Related Documents:** [SPEC016](SPEC016-decoder_buffer_design.md) | [SPEC002](SPEC002-crossfade.md) | [ADR-001](ADR-001-mixer_refactoring.md)

---

## Context

### Problem Statement

The legacy mixer (`pipeline/mixer.rs`) emits position events via timer-based polling (every 100ms), creating architectural issues:

**Timer-Based Approach (Legacy):**
- Mixer tracks `frame_counter` and emits events at fixed intervals
- Approximate timing (~100ms granularity)
- Duplicates state (mixer tracks passage_id and position)
- Polling overhead (checking timer every frame)

**Architectural Violation:**
- Position tracking is state management (not mixing responsibility)
- Mixer shouldn't know "what's playing" (that's PlaybackEngine's role)
- SPEC016 calls for mixer statelessness, but position awareness is needed

### Initial Misunderstanding

**Original Interpretation (Overly Aggressive):**
> "Mixer should be completely stateless with no position awareness"

**User Correction (2025-01-30):**
> "Recent edits to SPEC016 and related documents may have been overly aggressive in describing the statelessness of the mixer. The mixer should not be involved in fade calculations, but... the mixer is uniquely placed in the system to know what frames of both the currently playing and next playing passages have been handed to the output device and how full the output device's buffer is."

**Key Insight:** Mixer IS uniquely positioned to know playback reality (frames delivered to output device).

### Requirements

**REQ-MIX-008:** Position event emission must be:
- Sample-accurate (not approximate intervals)
- Event-driven (not timer-based polling)
- Architecturally clean (separation of concerns)
- Able to handle crossfade timing precisely

---

## Decision

Implement **event-driven position tracking via position markers** with clear separation between calculation and execution layers:

### Architecture

**Calculation Layer (PlaybackEngine):**
- Calculates WHAT events should occur and WHEN (specific tick counts)
- Knows queue state (current/next passages)
- Sets position markers at calculated ticks
- Receives marker events from mixer

**Execution Layer (Mixer):**
- Tracks playback reality (current_tick, frames_written)
- Checks markers during mixing operations
- Signals when ticks reached (sample-accurate)
- Returns MarkerEvent results to caller

### Components

#### PositionMarker
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarker {
    pub tick: i64,                    // Tick count in passage timeline
    pub passage_id: Uuid,             // Which passage this applies to
    pub event_type: MarkerEvent,      // What to signal when reached
}
```

#### MarkerEvent
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerEvent {
    PositionUpdate { position_ms: u64 },       // Periodic position updates
    StartCrossfade { next_passage_id: Uuid },  // Begin crossfade
    SongBoundary { new_song_id: Option<Uuid> }, // Multi-song passage
    PassageComplete,                           // Passage finished
}
```

#### Mixer State
```rust
pub struct Mixer {
    // ... existing fields ...
    markers: BinaryHeap<Reverse<PositionMarker>>,  // Min-heap by tick
    current_tick: i64,                              // Current tick count
    current_passage_id: Option<Uuid>,              // Currently playing
    frames_written: u64,                           // Total frames output
}
```

### Usage Pattern

**Setting Markers (PlaybackEngine):**
```rust
// Calculate crossfade start timing
let crossfade_start_tick = end_tick - crossfade_duration_tick;

// Set marker
mixer.add_marker(PositionMarker {
    tick: crossfade_start_tick,
    passage_id: current_passage_id,
    event_type: MarkerEvent::StartCrossfade { next_passage_id },
});
```

**Receiving Events (Mixer):**
```rust
// During mixing
let events = mixer.mix_single(&mut buffer, &mut output)?;

// Engine handles events
for event in events {
    match event {
        MarkerEvent::StartCrossfade { next_passage_id } => {
            // Switch to crossfade mode
        }
        MarkerEvent::PositionUpdate { position_ms } => {
            // Broadcast to UI via SSE
        }
        // ...
    }
}
```

### Implementation Details

**Marker Storage:**
- `BinaryHeap<Reverse<PositionMarker>>` (min-heap)
- Soonest marker always at top: O(1) peek, O(log n) insert/remove
- Checked during every mix operation

**Position Tracking:**
- `current_tick` advanced by frames mixed each operation
- `frames_written` tracks total output to device
- Markers checked after tick update

**Marker Lifecycle:**
1. **Add:** Engine adds marker at calculated tick
2. **Check:** Mixer checks after advancing tick
3. **Emit:** Return event when `current_tick >= marker.tick`
4. **Clear:** Remove marker after emission or when no longer relevant

---

## Rationale

### Why Event-Driven?

**Sample-Accurate Timing:**
- Events trigger exactly when tick reached (not "approximately 100ms")
- Critical for crossfade timing (must start at precise tick)
- Position updates accurate to single frame

**No Polling Overhead:**
- No timer checks every frame
- Events only when markers reached
- Reduced CPU usage in hot path

**Architectural Clarity:**
- Engine calculates timing (its responsibility)
- Mixer signals execution (its capability)
- Clean separation of concerns

### Why Mixer Tracks Position?

**Playback Reality:**
- Mixer uniquely knows frames delivered to output device
- Can account for output buffer latency
- Most accurate source of "what's actually playing"

**Sample-Accurate Events:**
- Tick incremented during mixing (exact frame count)
- Events tied to actual audio output
- No drift between calculation and reality

### Why Not Move Position Tracking to Engine?

**Considered Alternatives:**

**Option A: Engine tracks position independently**
- ❌ Requires engine to know mixer's frame output
- ❌ Potential drift between engine and mixer
- ❌ Engine doesn't know output device latency

**Option B: Mixer sends position updates periodically (timer)**
- ❌ Approximate timing (100ms granularity)
- ❌ Polling overhead
- ❌ Legacy approach we're trying to eliminate

**Option C: Event-driven via markers (CHOSEN)**
- ✅ Sample-accurate (tick-based)
- ✅ No polling overhead
- ✅ Clean separation (engine calculates, mixer signals)
- ✅ Mixer leverages unique position to know reality

---

## Consequences

### Positive

**Sample-Accurate Events:**
- Crossfade timing precise to single frame
- Position updates accurate (not approximate intervals)
- Eliminates timer-based polling overhead

**Architectural Clarity:**
- Clear separation: calculation (engine) vs. execution (mixer)
- Mixer responsibility focused on playback reality
- Engine responsibility focused on timing decisions

**Performance:**
- No timer checks every frame
- BinaryHeap O(log n) operations
- Events only when markers reached

**Maintainability:**
- Clear API (add_marker, check_markers)
- Event-driven pattern familiar to developers
- Testable (set markers, verify events)

### Negative

**Mixer Stateful:**
- Mixer no longer "completely stateless"
- Tracks current_tick, current_passage_id, frames_written
- Requires initialization when passage changes

**Complexity:**
- Marker system adds ~100 lines to mixer
- Engine must calculate timing and set markers
- Two-phase coordination (set marker, receive event)

### Trade-Offs Accepted

**Stateful Mixer vs. Sample Accuracy:**
- Accept mixer state tracking (position awareness)
- Gain sample-accurate event triggering
- Rationale: Mixer uniquely positioned to know playback reality

**Event-Driven Complexity vs. Timer Simplicity:**
- Accept marker management overhead
- Gain elimination of polling overhead
- Rationale: Sample accuracy critical for crossfade timing

---

## Migration Strategy

### Phase 1: Implement Marker System (COMPLETE)
- Add PositionMarker and MarkerEvent types to mixer
- Add marker storage (BinaryHeap) and tracking fields
- Implement add_marker(), check_markers() methods
- Update mix_single() and mix_crossfade() to return events

### Phase 2: Wire to PlaybackEngine (PENDING - Sub-Increment 4b)
- Engine calculates crossfade timing
- Engine sets markers at calculated ticks
- Engine receives events and switches mixer modes
- Remove legacy position event channel

### Phase 3: Remove Legacy Mixer (PENDING - Sub-Increment 4c)
- Delete `pipeline/mixer.rs` (1,969 lines)
- Remove timer-based position tracking
- Clean up codebase

---

## Specification Updates

**SPEC016 [DBD-MIX-070] through [DBD-MIX-078]:**
- Added Position Tracking and Event-Driven Architecture section
- Documented marker system design
- Provided crossfade timing example
- Clarified calculation vs. execution layer separation

**SPEC002 [XFD-IMPL-026]:**
- Added Execution Architecture clarification
- Documents PlaybackEngine sets markers
- Documents Mixer signals when ticks reached
- Cross-references SPEC016 position marker system

---

## Review and Approval

**Decision Date:** 2025-01-30
**Decided By:** Technical Lead (based on user feedback)
**Reviewed By:** Claude (PLAN014 Implementation)

**User Feedback (Trigger for Decision):**
> "Consider that recent edits to SPEC016 and related documents may have been overly aggressive in describing the statelessness of the mixer... the mixer is uniquely placed in the system to know what frames of both the currently playing and next playing passages have been handed to the output device... Would it make sense for the playback engine to inform the mixer of specific points of interest in the currently playing and next playing passages and then the mixer could signal when those points are being played? ...the system should be event driven."

**Status:** Accepted and implemented (marker system ready for integration)

---

**Document Version:** 1.0
**Created:** 2025-01-30
**Last Updated:** 2025-01-30
**Status:** Accepted
**Related ADRs:** [ADR-001 Mixer Refactoring](ADR-001-mixer_refactoring.md)
