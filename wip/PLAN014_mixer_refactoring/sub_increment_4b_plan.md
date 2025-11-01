# Sub-Increment 4b: PlaybackEngine Integration Plan

**Date:** 2025-01-30
**Status:** Planning
**Context:** PLAN014 Mixer Refactoring - Replace legacy CrossfadeMixer with SPEC016-compliant Mixer

---

## Executive Summary

**Objective:** Integrate SPEC016-compliant Mixer into PlaybackEngine, replacing legacy CrossfadeMixer

**Challenge:** Architectural mismatch between frame-by-frame mixing (legacy) and batch mixing (SPEC016)

**Estimated Duration:** 13-19 hours (original estimate), may be higher due to architectural changes

**Prerequisites:** ✅ Increments 5-6 complete (42 tests passing)

---

## API Comparison

### Legacy CrossfadeMixer API (Frame-by-Frame)

**Key Methods Used by Engine:**
```rust
// Initialization
mixer.set_event_channel(tx)
mixer.set_position_event_interval_ms(ms)
mixer.set_buffer_manager(manager)
mixer.set_mixer_min_start_level(level)

// Playback control
mixer.start_passage(passage_id, entry, ...)
mixer.pause()
mixer.resume(fade_ms, curve)
mixer.stop()
mixer.set_position(pos)

// Hot path (called every iteration ~44kHz)
frame = mixer.get_next_frame().await

// State queries
mixer.get_current_passage_id()
mixer.get_position()
mixer.get_state_info()
mixer.is_crossfading()
mixer.is_current_finished()
mixer.passage_start_time()
mixer.take_crossfade_completed()
```

### SPEC016 Mixer API (Batch Mixing)

**Available Methods:**
```rust
// Initialization
mixer = Mixer::new(master_volume)
mixer.set_master_volume(volume)
mixer.set_state(MixerState)

// Marker system (event-driven)
mixer.add_marker(PositionMarker)
mixer.clear_markers_for_passage(passage_id)
mixer.clear_all_markers()

// Passage control
mixer.set_current_passage(passage_id, start_tick)

// Pause/resume
mixer.start_resume_fade(samples, curve)

// Batch mixing (async)
events = mixer.mix_single(buffer_manager, passage_id, output).await
events = mixer.mix_crossfade(buffer_manager, current_id, next_id, output, ...).await

// State queries
mixer.get_current_tick()
mixer.get_current_passage_id()
mixer.get_frames_written()
mixer.state()
mixer.is_resume_fading()
```

### Key Architectural Differences

| Aspect | Legacy (CrossfadeMixer) | SPEC016 (Mixer) |
|--------|-------------------------|-----------------|
| **Mixing Model** | Frame-by-frame (`get_next_frame()`) | Batch (`mix_single()`, `mix_crossfade()`) |
| **Position Tracking** | Timer-based polling (100ms interval) | Event-driven markers |
| **Crossfade Control** | Mixer calculates timing internally | Engine calculates, mixer signals |
| **Event Channel** | Mixer owns channel, sends events | Engine handles events from return value |
| **State Management** | Complex internal state machine | Simple state enum + markers |
| **Buffer Access** | Mixer fetches from BufferManager | Engine passes BufferManager reference |

---

## Integration Challenges

### Challenge 1: Frame-by-Frame to Batch Conversion

**Problem:** Engine calls `get_next_frame()` in tight loop, SPEC016 mixer requires batch sizes

**Impact:** Core playback loop architecture change required

**Options:**

**Option A: Internal Buffer in Mixer Wrapper**
- Create `MixerAdapter` that wraps SPEC016 Mixer
- Adapter maintains internal buffer (e.g., 512 frames)
- Provides `get_next_frame()` interface for backward compatibility
- Fills buffer in batches, returns frames one at a time

**Pros:**
- Minimal engine changes
- Preserves frame-by-frame API
- Isolated complexity in adapter

**Cons:**
- Extra copy overhead (batch → internal buffer → frame)
- Latency increase (must fill buffer before first frame)
- Adapter complexity (state management, edge cases)

**Option B: Batch-Based Playback Loop**
- Redesign engine loop to mix in batches (512-2048 frames)
- Push batch directly to ring buffer
- Process events after each batch

**Pros:**
- More efficient (fewer async calls, better cache locality)
- Aligns with SPEC016 architecture
- Simplifies mixer implementation

**Cons:**
- Significant engine refactoring
- Higher implementation risk
- Potential timing changes

**Recommendation:** **Option B (Batch-Based Loop)** per Risk-First Framework
- Lower residual risk (fewer abstractions, direct architecture alignment)
- Performance benefits (efficiency gain justifies effort)
- Future-proof (SPEC016 is authoritative specification)

---

### Challenge 2: Event-Driven Position Tracking

**Problem:** Legacy mixer sends events via channel, SPEC016 mixer returns events from `mix_*()` calls

**Current Flow:**
```rust
// Engine sets up channel
let (tx, rx) = mpsc::unbounded_channel();
mixer.set_event_channel(tx);
mixer.set_position_event_interval_ms(100);

// Separate task processes events
tokio::spawn(async move {
    while let Some(event) = rx.recv().await {
        // Handle PlaybackEvent::PositionUpdate
    }
});

// Mixer sends events internally based on timer
```

**SPEC016 Flow:**
```rust
// Engine provides markers upfront
mixer.add_marker(PositionMarker {
    tick: calculate_position_update_tick(100ms),
    passage_id,
    event_type: MarkerEvent::PositionUpdate { position_ms: 100 },
});

// Mix call returns events
let events = mixer.mix_single(buffer_manager, passage_id, output).await?;

// Engine handles events immediately
for event in events {
    match event {
        MarkerEvent::PositionUpdate { position_ms } => {
            // Send to event channel
        }
        // ...
    }
}
```

**Implementation:**
- Engine calculates marker ticks (using `wkmp_common::timing::ms_to_ticks()`)
- Add markers before starting playback
- Process returned events in playback loop
- Convert `MarkerEvent` to `PlaybackEvent` and send to channel

---

### Challenge 3: Crossfade Timing Calculation

**Problem:** Legacy mixer calculates crossfade timing internally, SPEC016 requires engine to calculate

**Legacy Behavior:**
- Mixer knows passage duration, lead-out point
- Automatically starts crossfade at calculated tick
- Emits `CrossfadeStarted` event

**SPEC016 Behavior:**
- Engine calculates crossfade start tick from passage timing
- Engine adds `StartCrossfade` marker
- Mixer signals when marker reached
- Engine switches to `mix_crossfade()` calls

**Implementation:**
- Calculate crossfade start tick: `passage.lead_out_point_sample - crossfade_duration_samples`
- Add `StartCrossfade` marker at calculated tick
- On `MarkerEvent::StartCrossfade`, begin calling `mix_crossfade()` instead of `mix_single()`
- Continue until `PassageComplete` event

---

### Challenge 4: State Management

**Problem:** Legacy mixer has complex internal state, SPEC016 has simple state enum

**Legacy States:** (inferred from behavior)
- Idle
- Playing
- Crossfading
- Paused
- Resuming (fade-in after pause)

**SPEC016 States:**
```rust
pub enum MixerState {
    Idle,
    Playing,
    Paused,
}
```

**Mapping:**
- `Idle` → `MixerState::Idle`
- `Playing` → `MixerState::Playing`
- `Crossfading` → No explicit state (detected by `is_crossfading` flag in engine)
- `Paused` → `MixerState::Paused`
- `Resuming` → `MixerState::Playing` + `is_resume_fading()` returns true

**Implementation:**
- Engine tracks crossfade state (boolean flag: `is_crossfading`)
- Set when `StartCrossfade` marker received
- Clear when `PassageComplete` received
- Query `mixer.is_resume_fading()` for resume fade detection

---

## Implementation Plan

### Phase 1: Preparation (2-3 hours)

**Tasks:**
1. Create branch: `feature/plan014-sub-increment-4b`
2. Document current engine behavior (playback loop, event handling, state transitions)
3. Create test audio files (3-5 short passages for manual testing)
4. Create acceptance test criteria

**Deliverables:**
- Branch created
- Current behavior documented
- Test assets prepared
- Acceptance tests defined

---

### Phase 2: Batch Mixing Loop (5-7 hours)

**Tasks:**
1. Redesign playback loop to use batch mixing
2. Update imports (use `Mixer` instead of `CrossfadeMixer`)
3. Replace `get_next_frame()` with batch `mix_single()` calls
4. Adjust ring buffer push logic (batch instead of frame-by-frame)
5. Handle underrun scenarios (partial buffer fills)

**Changes Required:**

**engine.rs playback loop (approximate line 450-520):**
```rust
// OLD (frame-by-frame)
loop {
    let frame = mixer.get_next_frame().await;
    ring_buffer.push(frame);
}

// NEW (batch mixing)
const BATCH_SIZE: usize = 512; // frames
loop {
    let mut output = vec![0.0f32; BATCH_SIZE * 2]; // stereo

    if is_crossfading {
        let events = mixer.mix_crossfade(
            &buffer_manager,
            current_passage_id,
            next_passage_id,
            &mut output,
            crossfade_samples,
            fade_out_curve,
            fade_in_curve
        ).await?;
        handle_marker_events(events);
    } else {
        let events = mixer.mix_single(
            &buffer_manager,
            current_passage_id,
            &mut output
        ).await?;
        handle_marker_events(events);
    }

    // Push batch to ring buffer
    for i in (0..output.len()).step_by(2) {
        let frame = AudioFrame {
            left: output[i],
            right: output[i + 1],
        };
        ring_buffer.push(frame)?;
    }
}
```

**Deliverables:**
- Batch mixing loop implemented
- Compiles without errors
- Basic playback functional (no events yet)

---

### Phase 3: Event-Driven Markers (4-6 hours)

**Tasks:**
1. Implement marker calculation (position updates every 100ms)
2. Add markers before playback starts
3. Implement `handle_marker_events()` function
4. Convert `MarkerEvent` to `PlaybackEvent`
5. Update crossfade detection (use `StartCrossfade` marker)
6. Update passage completion detection (use `PassageComplete` marker)

**Marker Calculation Logic:**
```rust
// Calculate position update markers (every 100ms)
let interval_ms = 100; // From settings
let sample_rate = 44100;
let interval_samples = wkmp_common::timing::ms_to_ticks(interval_ms, sample_rate);

let passage_duration_samples = /* from passage timing */;
let num_markers = passage_duration_samples / interval_samples;

for i in 0..num_markers {
    let tick = i * interval_samples;
    let position_ms = wkmp_common::timing::ticks_to_ms(tick, sample_rate);

    mixer.add_marker(PositionMarker {
        tick,
        passage_id,
        event_type: MarkerEvent::PositionUpdate { position_ms },
    });
}

// Add crossfade marker
let crossfade_start_tick = lead_out_point_sample - crossfade_duration_samples;
mixer.add_marker(PositionMarker {
    tick: crossfade_start_tick,
    passage_id,
    event_type: MarkerEvent::StartCrossfade { next_passage_id },
});

// Add passage complete marker
mixer.add_marker(PositionMarker {
    tick: fade_out_point_sample,
    passage_id,
    event_type: MarkerEvent::PassageComplete,
});
```

**Event Handling:**
```rust
fn handle_marker_events(events: Vec<MarkerEvent>, event_tx: &mpsc::UnboundedSender<PlaybackEvent>) {
    for event in events {
        match event {
            MarkerEvent::PositionUpdate { position_ms } => {
                event_tx.send(PlaybackEvent::PositionUpdate { position_ms }).ok();
            }
            MarkerEvent::StartCrossfade { next_passage_id } => {
                // Set is_crossfading flag
                // Trigger next passage load if not already loaded
                event_tx.send(PlaybackEvent::CrossfadeStarted { next_passage_id }).ok();
            }
            MarkerEvent::PassageComplete => {
                // Clear is_crossfading flag
                // Advance queue
                event_tx.send(PlaybackEvent::PassageComplete).ok();
            }
            MarkerEvent::SongBoundary { new_song_id } => {
                event_tx.send(PlaybackEvent::SongChanged { song_id: new_song_id }).ok();
            }
            MarkerEvent::EndOfFile { unreachable_markers } => {
                // Handle early EOF (before crossfade point)
                warn!("EOF reached with {} unreachable markers", unreachable_markers.len());
                // Start next passage immediately
            }
            MarkerEvent::EndOfFileBeforeLeadOut { planned_crossfade_tick, unreachable_markers } => {
                warn!("EOF before crossfade at tick {}", planned_crossfade_tick);
                // Emergency passage switch
            }
        }
    }
}
```

**Deliverables:**
- Marker system integrated
- Position updates working
- Crossfade detection functional
- All events properly routed

---

### Phase 4: State Management & Control (2-3 hours)

**Tasks:**
1. Update `pause()` implementation
2. Update `resume()` implementation
3. Update `stop()` implementation
4. Update `seek()` implementation
5. Remove legacy mixer methods

**Pause Implementation:**
```rust
pub async fn pause(&self) -> Result<()> {
    let mut mixer = self.mixer.write().await;
    mixer.set_state(MixerState::Paused);
    // Pause also stops decoder worker
    self.decoder_worker.pause().await;
    Ok(())
}
```

**Resume Implementation:**
```rust
pub async fn resume(&self, fade_duration_ms: u64, fade_curve: &str) -> Result<()> {
    let mut mixer = self.mixer.write().await;

    // Calculate fade duration in samples
    let sample_rate = 44100; // From settings
    let fade_samples = (fade_duration_ms * sample_rate / 1000) as usize;

    // Parse fade curve
    let curve = FadeCurve::from_str(fade_curve)?;

    // Start resume fade
    mixer.start_resume_fade(fade_samples, curve);
    mixer.set_state(MixerState::Playing);

    // Resume decoder worker
    self.decoder_worker.resume().await;
    Ok(())
}
```

**Stop Implementation:**
```rust
pub async fn stop(&self) -> Result<()> {
    let mut mixer = self.mixer.write().await;

    // Clear all markers
    mixer.clear_all_markers();

    // Reset to idle
    mixer.set_state(MixerState::Idle);

    // No current passage
    // (set_current_passage not called - mixer tracks None internally)

    // Stop decoder
    self.decoder_worker.stop().await;

    Ok(())
}
```

**Deliverables:**
- Pause/resume working
- Stop working
- State transitions correct

---

### Phase 5: Testing & Validation (3-4 hours)

**Tasks:**
1. Manual testing with real audio files
2. Test single passage playback
3. Test crossfade between passages
4. Test pause/resume
5. Test seek
6. Test queue advancement
7. Test EOF scenarios
8. Fix bugs discovered during testing

**Test Scenarios:**
1. **Single Passage Playback**
   - Load one passage
   - Play to completion
   - Verify position updates every 100ms
   - Verify passage complete event

2. **Crossfade Playback**
   - Queue 2 passages
   - Play through crossfade
   - Verify `StartCrossfade` event
   - Verify smooth audio transition
   - Verify `PassageComplete` event

3. **Pause/Resume**
   - Play passage
   - Pause mid-playback
   - Resume with fade-in
   - Verify position tracking continues

4. **Seek**
   - Play passage
   - Seek to 50% position
   - Verify markers after seek point fire
   - Verify markers before seek point don't fire

5. **Queue Advancement**
   - Queue 3 passages
   - Play through all
   - Verify automatic queue progression

6. **EOF Scenarios**
   - Play passage shorter than expected (truncated file)
   - Verify `EndOfFile` event
   - Verify unreachable markers reported

**Deliverables:**
- All test scenarios passing
- No regressions in existing functionality
- Bug fixes applied

---

## Risk Assessment

### Risk 1: Batch Size Tuning

**Risk:** Incorrect batch size causes underruns or excessive latency

**Probability:** Medium
**Impact:** Medium (audio glitches, user-visible)

**Mitigation:**
- Start with 512 frames (~11ms @ 44.1kHz)
- Monitor ring buffer fill levels
- Adjust based on measurements

**Residual Risk:** Low

---

### Risk 2: Event Timing Drift

**Risk:** Marker-based timing drifts from actual playback position

**Probability:** Low (tested in Increment 5-6)
**Impact:** Medium (position display inaccuracy)

**Mitigation:**
- Use `frames_written` from mixer (authoritative)
- Validate against buffer statistics
- Add drift detection (warn if >100ms difference)

**Residual Risk:** Low

---

### Risk 3: Crossfade Calculation Errors

**Risk:** Engine miscalculates crossfade timing, causing audio gaps or overlaps

**Probability:** Medium (new calculation logic)
**Impact:** High (audio quality degradation)

**Mitigation:**
- Thorough manual testing with known-good passages
- Add validation (crossfade start < passage end)
- Log calculated ticks for debugging

**Residual Risk:** Low-Medium

---

### Risk 4: Incomplete State Migration

**Risk:** Legacy state not fully mapped to SPEC016, causing edge case bugs

**Probability:** Medium
**Impact:** Medium (functional bugs)

**Mitigation:**
- Comprehensive state transition testing
- Review all legacy mixer usages (grep analysis)
- Test pause/resume/stop thoroughly

**Residual Risk:** Low

---

## Success Criteria

**Functional:**
- ✅ Single passage plays to completion
- ✅ Crossfade between passages smooth and audible
- ✅ Position updates every 100ms (configurable)
- ✅ Pause/resume with fade-in works
- ✅ Seek works without audio glitches
- ✅ Queue auto-advances after passage complete
- ✅ EOF handling works (truncated files)

**Performance:**
- ✅ No audio underruns during normal playback
- ✅ Ring buffer maintains healthy fill level (30-70%)
- ✅ CPU usage comparable to legacy mixer (±10%)

**Code Quality:**
- ✅ All compiler warnings resolved
- ✅ No unsafe code added
- ✅ Clear comments for complex logic
- ✅ Traceability to requirements maintained

---

## Timeline Estimate

| Phase | Tasks | Estimated Hours | Risk Buffer |
|-------|-------|----------------|-------------|
| 1. Preparation | Documentation, test assets | 2-3 | +0.5 |
| 2. Batch Mixing Loop | Core playback refactor | 5-7 | +1.5 |
| 3. Event-Driven Markers | Marker calculation & handling | 4-6 | +1.0 |
| 4. State Management | Pause/resume/stop/seek | 2-3 | +0.5 |
| 5. Testing & Validation | Manual testing, bug fixes | 3-4 | +1.5 |
| **Total** | | **16-23 hours** | **+5 hours** |

**Recommended:** **20-28 hours** with risk buffer included

---

## Rollback Plan

**If integration fails or introduces critical bugs:**

1. Revert to previous commit (before Sub-Increment 4b)
2. Legacy mixer remains in use
3. SPEC016 mixer continues unit testing
4. Re-assess integration strategy

**Conditions for Rollback:**
- Audio quality degradation (glitches, dropouts)
- Performance regression (>20% CPU increase)
- Blocking bugs discovered after 4+ hours debugging
- Risk of missing project deadline

---

## Next Steps

1. Review this plan with user
2. Get approval to proceed
3. Create feature branch
4. Begin Phase 1 (Preparation)

---

**Document Created:** 2025-01-30
**Status:** Planning complete - awaiting approval to proceed
**Estimated Duration:** 20-28 hours (with risk buffer)
