# PLAN014 Phase Complete: Event-Driven Position Tracking

**Date:** 2025-01-30
**Status:** ✅ Architecture Phase Complete

---

## What Was Built

### Event-Driven Marker System

**Location:** [mixer.rs](../../wkmp-ap/src/playback/mixer.rs)

**Core Components:**
1. `PositionMarker` - Tick-based event markers
2. `MarkerEvent` - Event types (position updates, crossfade triggers, etc.)
3. Marker storage - BinaryHeap min-heap (O(log n) operations)
4. Position tracking - `current_tick`, `frames_written`, `current_passage_id`

**API Methods:**
- `add_marker()` - Engine sets markers at calculated ticks
- `set_current_passage()` - Initialize passage tracking
- `clear_markers_for_passage()` - Remove stale markers
- `mix_single()` → `Result<Vec<MarkerEvent>>` - Returns triggered events
- `mix_crossfade()` → `Result<Vec<MarkerEvent>>` - Returns triggered events

**Architecture:**
- **Calculation Layer (PlaybackEngine):** Determines WHAT and WHEN
- **Execution Layer (Mixer):** Signals when events occur

---

## What Was Documented

### Specifications Updated

**[SPEC016 v1.5](../../docs/SPEC016-decoder_buffer_design.md):**
- New section: "Position Tracking and Event-Driven Architecture"
- Requirements [DBD-MIX-070] through [DBD-MIX-078]
- Crossfade timing example
- Position update marker pattern

**[SPEC002 v1.3](../../docs/SPEC002-crossfade.md):**
- Requirement [XFD-IMPL-026]: Execution Architecture
- Clarified calculation vs. execution layer separation

**[ADR-002](../../docs/ADR-002-event_driven_position_tracking.md) (NEW):**
- Context: timer-based vs. event-driven
- Decision rationale and trade-offs
- Migration strategy

---

## Key Benefits

**Sample-Accurate Timing:**
- Events trigger at exact tick (not ~100ms intervals)
- Critical for crossfade precision

**No Polling Overhead:**
- Events only when markers reached
- No frame-by-frame timer checks

**Architectural Clarity:**
- Clean separation: engine calculates, mixer signals
- Mixer leverages unique position to know playback reality

---

## Next Steps

### Sub-Increment 4b: PlaybackEngine Integration (2-3 hours)

**Tasks:**
1. Replace `CrossfadeMixer` with `Mixer` in engine
2. Load master volume from settings
3. Set markers for crossfade timing
4. Set markers for position updates
5. Process marker events in playback loop
6. Update pause/resume calls

**Entry Point:** [engine.rs:238](../../wkmp-ap/src/playback/engine.rs#L238)

**Example Integration:**
```rust
// In PlaybackEngine::new()
let master_volume = load_master_volume(&db_pool).await?;
let mixer = Mixer::new(master_volume);

// When starting passage
mixer.write().await.set_current_passage(passage_id, 0);

// Set crossfade marker
let crossfade_start_tick = end_tick - crossfade_duration_tick;
mixer.write().await.add_marker(PositionMarker {
    tick: crossfade_start_tick,
    passage_id: current_passage_id,
    event_type: MarkerEvent::StartCrossfade { next_passage_id },
});

// In playback loop
let events = mixer.write().await.mix_single(&mut buffer, &mut output)?;
for event in events {
    match event {
        MarkerEvent::StartCrossfade { next_passage_id } => {
            // Switch to crossfade mode
        }
        MarkerEvent::PositionUpdate { position_ms } => {
            // Broadcast to UI
        }
        _ => {}
    }
}
```

### Sub-Increment 4c: Legacy Mixer Removal (30 min)

Delete `wkmp-ap/src/playback/pipeline/mixer.rs` (1,969 lines)

---

## Files Modified

**Implementation:**
- [wkmp-ap/src/playback/mixer.rs](../../wkmp-ap/src/playback/mixer.rs)
- [wkmp-ap/src/playback/mod.rs](../../wkmp-ap/src/playback/mod.rs)

**Documentation:**
- [docs/SPEC016-decoder_buffer_design.md](../../docs/SPEC016-decoder_buffer_design.md)
- [docs/SPEC002-crossfade.md](../../docs/SPEC002-crossfade.md)
- [docs/ADR-002-event_driven_position_tracking.md](../../docs/ADR-002-event_driven_position_tracking.md)

**Planning:**
- [wip/PLAN014_mixer_refactoring/implementation_status.md](implementation_status.md)
- [wip/PLAN014_mixer_refactoring/position_tracking_analysis.md](position_tracking_analysis.md)
- [wip/PLAN014_mixer_refactoring/playback_architecture_diagram.md](playback_architecture_diagram.md)
- [wip/PLAN014_mixer_refactoring/revised_mixer_architecture.md](revised_mixer_architecture.md)

---

## Decision Points

**User Feedback (Critical):**
> "Consider that recent edits to SPEC016 and related documents may have been overly aggressive in describing the statelessness of the mixer... the mixer is uniquely placed in the system to know what frames of both the currently playing and next playing passages have been handed to the output device... the system should be event driven."

**Resolution:**
- Mixer DOES track position (execution reality)
- Mixer does NOT calculate timing (engine's responsibility)
- Event-driven markers eliminate timer polling

**Trade-Off Accepted:**
- Mixer is stateful (position awareness)
- Gain: Sample-accurate events, no polling overhead

---

## Completion Checklist

- ✅ Marker system implemented
- ✅ SPEC016 updated with requirements
- ✅ SPEC002 clarified execution architecture
- ✅ ADR-002 created
- ✅ Implementation status documented
- ⏳ PlaybackEngine integration (pending)
- ⏳ Legacy mixer removal (pending)
- ⏳ Testing (pending)

---

**Report Date:** 2025-01-30
**Phase:** Architecture Complete - Ready for Integration
