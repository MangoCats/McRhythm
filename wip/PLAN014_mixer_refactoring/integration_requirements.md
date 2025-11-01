# PLAN014 Sub-Increment 4b: Integration Requirements

**Date:** 2025-01-30
**Status:** Architecture Complete - Integration Analysis Complete

---

## Current Status

✅ **Architecture Phase Complete:**
- Event-driven marker system implemented in correct mixer
- SPEC016, SPEC002, ADR-002 documentation updated
- Mixer ready for integration

⚠️ **Integration Phase Pending:**
- PlaybackEngine heavily coupled to legacy mixer API
- 20+ method calls need refactoring
- Significant playback loop changes required

---

## Build Errors Analysis

After updating `PlaybackEngine` to use `Mixer` instead of `CrossfadeMixer`, 20+ missing method errors occurred:

### Missing Methods (Legacy Mixer API)

**State Management:**
- `stop()` - Stop playback
- `pause()` - Enter pause mode
- `resume()` - Resume from pause with fade-in
- `set_position()` - Set playback position
- `passage_start_time()` - Get passage start time
- `get_state_info()` - Get mixer state information
- `is_crossfading()` - Check if crossfading
- `is_current_finished()` - Check if passage complete

**Passage Control:**
- `start_passage()` - Start playing a passage
- `start_crossfade()` - Begin crossfade to next passage
- `take_crossfade_completed()` - Poll crossfade completion

**Frame Processing:**
- `get_next_frame()` - Get next audio frame (called frequently in playback loop)
- `get_position()` - Get current position
- `get_total_frames_mixed()` - Get total frames output

### Correct Mixer API (What Exists)

**State Management:**
- `set_state(MixerState)` - Set Playing or Paused
- `state()` - Get current state
- `start_resume_fade()` - Start fade-in from pause
- `is_resume_fading()` - Check if fading from pause

**Position Tracking (Event-Driven):**
- `set_current_passage()` - Initialize passage tracking
- `get_current_tick()` - Get current tick position
- `get_frames_written()` - Get total frames output
- `add_marker()` - Set position marker
- `clear_markers_for_passage()` - Remove stale markers
- `clear_all_markers()` - Reset markers

**Mixing:**
- `mix_single()` → `Result<Vec<MarkerEvent>>` - Mix single passage
- `mix_crossfade()` → `Result<Vec<MarkerEvent>>` - Mix crossfade overlap

**Volume:**
- `set_master_volume()` - Set master volume
- `master_volume()` - Get master volume

---

## Architectural Differences

### Legacy Mixer (Stateful, Pull-Based)

**Pattern:**
```rust
// Engine tells mixer what to play
mixer.start_passage(queue_entry_id, passage_id, fade_in_duration, fade_in_curve);

// Engine repeatedly pulls frames
loop {
    let frame = mixer.get_next_frame()?;
    output.push(frame);

    // Mixer internally emits position events every 100ms
}

// Engine starts crossfade at calculated time
mixer.start_crossfade(next_passage_id, crossfade_duration, fade_out_curve, fade_in_curve);

// Engine polls for completion
if mixer.take_crossfade_completed() {
    // Advance queue
}
```

**Characteristics:**
- Mixer tracks state machine (Playing, Crossfading, Paused, etc.)
- Mixer knows passage IDs and fade parameters
- Frame-by-frame pull API (`get_next_frame()`)
- Timer-based position events (100ms polling)
- Crossfade completion polling

### Correct Mixer (Event-Driven, Push-Based)

**Pattern:**
```rust
// Engine sets markers for events
let crossfade_start_tick = end_tick - crossfade_duration_tick;
mixer.add_marker(PositionMarker {
    tick: crossfade_start_tick,
    passage_id: current_passage_id,
    event_type: MarkerEvent::StartCrossfade { next_passage_id },
});

// Engine pushes buffer data to mixer
let events = mixer.mix_single(&mut passage_buffer, &mut output)?;

// Engine processes returned events
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

**Characteristics:**
- Mixer simple state (Playing/Paused only)
- Engine tracks passage IDs and calculates timing
- Batch mixing API (`mix_single(buffer, output)`)
- Event-driven position updates (marker-based)
- Crossfade triggered by marker events

---

## Integration Tasks

### Phase 1: Playback Loop Refactoring (4-6 hours)

**Current Loop Structure:**
```rust
// Pseudo-code of current structure
loop {
    let mut mixer = self.mixer.write().await;
    let frame = mixer.get_next_frame()?;
    output_buffer.push(frame);
}
```

**Target Loop Structure:**
```rust
// Pseudo-code of target structure
loop {
    let mut mixer = self.mixer.write().await;
    let mut passage_buffer = get_current_passage_buffer();

    // Mix batch of frames
    let events = mixer.mix_single(&mut passage_buffer, &mut output)?;

    // Process marker events
    for event in events {
        self.handle_marker_event(event).await;
    }
}
```

**Sub-Tasks:**
1. Replace `get_next_frame()` with `mix_single()` / `mix_crossfade()`
2. Implement batch frame processing
3. Implement `handle_marker_event()` method
4. Update buffer access patterns

### Phase 2: State Management Refactoring (2-3 hours)

**Replace State Methods:**
- `mixer.start_passage()` → `mixer.set_current_passage()` + set markers
- `mixer.stop()` → `mixer.set_state(MixerState::Paused)` + clear markers
- `mixer.pause()` → `mixer.set_state(MixerState::Paused)`
- `mixer.resume()` → `mixer.set_state(MixerState::Playing)` + `start_resume_fade()`
- `mixer.is_crossfading()` → Engine tracks crossfade state
- `mixer.get_position()` → `mixer.get_current_tick()`

**Sub-Tasks:**
1. Move state tracking from mixer to engine
2. Update pause/resume handlers
3. Update position queries
4. Remove crossfade state polling

### Phase 3: Marker-Based Event System (3-4 hours)

**Implement Marker Management:**
- Calculate crossfade start ticks
- Set position update markers
- Set song boundary markers (multi-song passages)
- Set passage complete markers

**Implement Event Handling:**
- `handle_marker_event()` dispatcher
- `handle_crossfade_start()` - Switch to crossfade mixing
- `handle_position_update()` - Broadcast to UI
- `handle_song_boundary()` - Emit CurrentSongChanged
- `handle_passage_complete()` - Advance queue

**Sub-Tasks:**
1. Implement marker calculation methods
2. Implement event handler methods
3. Wire events to existing systems (queue advancement, UI updates)
4. Remove legacy position event channel

### Phase 4: Crossfade Refactoring (2-3 hours)

**Replace Crossfade API:**
- `mixer.start_crossfade()` → Set `StartCrossfade` marker
- `mixer.take_crossfade_completed()` → Handle `PassageComplete` event
- Crossfade timing → Calculate ticks, set markers

**Sub-Tasks:**
1. Calculate crossfade start tick from passage timing
2. Set `StartCrossfade` marker
3. Switch from `mix_single()` to `mix_crossfade()` on event
4. Handle crossfade completion event
5. Advance queue on completion

### Phase 5: Testing and Validation (2-3 hours)

**Test Cases:**
- Single passage playback
- Crossfade between passages
- Pause and resume with fade-in
- Position updates to UI
- Queue advancement
- Volume control

**Sub-Tasks:**
1. Manual playback testing
2. Verify position updates accurate
3. Verify crossfade timing sample-accurate
4. Verify pause/resume fade-in works
5. Check for memory leaks or performance issues

---

## Estimated Total Effort

**Sub-Increment 4b Total:** 13-19 hours

- Phase 1: Playback Loop - 4-6 hours
- Phase 2: State Management - 2-3 hours
- Phase 3: Marker Events - 3-4 hours
- Phase 4: Crossfade - 2-3 hours
- Phase 5: Testing - 2-3 hours

**Risk Level:** Medium-High
- Playback loop is critical path
- Many interconnected changes
- Requires careful testing

---

## Recommendation

**Option A: Full Integration (13-19 hours)**
- Complete Sub-Increment 4b as planned
- Highest risk but clean migration
- All benefits of event-driven architecture

**Option B: Phased Approach (Defer to Post-Testing)**
- Complete Increments 5-7 (testing) with legacy mixer first
- Validate correct mixer in isolation
- Then integrate with PlaybackEngine (Sub-Increment 4b)
- Lower risk, validates architecture before migration

**Option C: Hybrid Adapter (4-6 hours + future cleanup)**
- Create adapter layer that wraps correct mixer with legacy API
- Quick integration with existing engine
- Technical debt (adapter to remove later)
- NOT RECOMMENDED (violates clean architecture)

---

## Decision Point

**Recommended:** Option B (Defer Integration, Test First)

**Rationale:**
1. **Architecture validated:** Marker system complete and documented
2. **Risk mitigation:** Test correct mixer before full integration
3. **Effort optimization:** 7-10 hours testing vs. 13-19 hours integration
4. **Confidence building:** Validate approach before committing to large refactor

**Next Steps if Option B:**
1. Revert engine.rs to use `CrossfadeMixer` (temporary)
2. Proceed with Increments 5-7 (testing correct mixer in isolation)
3. After successful testing, proceed with Sub-Increment 4b (integration)

---

**Report Date:** 2025-01-30
**Status:** Integration Requirements Documented - Awaiting Decision
