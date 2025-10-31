# Sub-Increment 4b Implementation Checkpoint

**Date:** 2025-01-30
**Branch:** `feature/plan014-sub-increment-4b`
**Status:** Phase 2 In Progress - Imports Updated, Compilation Errors Identified

---

## Progress Summary

### ✅ Completed

1. **Feature Branch Created** - `feature/plan014-sub-increment-4b`
2. **Planning Documents Created:**
   - [sub_increment_4b_plan.md](sub_increment_4b_plan.md) - Complete integration plan (20-28 hours)
   - [current_engine_behavior.md](current_engine_behavior.md) - Legacy behavior documentation
3. **Import Updates** - Switched from `CrossfadeMixer` to `Mixer`
4. **Mixer Type Updated** - `Arc<RwLock<Mixer>>` in PlaybackEngine struct
5. **Mixer Initialization** - Basic `Mixer::new(master_volume)` (line 243)

### 🔄 In Progress

**Phase 2: Batch Mixing Loop** - Partially started

**Files Modified:**
- `wkmp-ap/src/playback/engine.rs` (lines 19, 90, 241-244)

---

## Compilation Errors Identified

### Critical Errors (Playback Loop)

**3 instances of `get_next_frame()` - Lines 490, 502, 518**
```
error[E0599]: no method named `get_next_frame` found
```

**Required:** Replace with batch mixing logic using `mix_single()` or `mix_crossfade()`

---

### Control Method Errors

1. **`stop()` - 3 instances (lines 872, 917, 1549)**
   - Map to: `mixer.set_state(MixerState::Idle)` + `clear_all_markers()`

2. **`pause()` - 1 instance (line 759)**
   - Map to: `mixer.set_state(MixerState::Paused)`

3. **`resume()` - 1 instance (line 718)**
   - Map to: `mixer.start_resume_fade(...)` + `set_state(MixerState::Playing)`

4. **`set_position()` - 1 instance (line 1010)**
   - Map to: `mixer.set_current_passage(passage_id, seek_tick)` + recalculate markers

---

### State Query Errors

5. **`passage_start_time()` - 1 instance (line 851)**
   - Solution: Track in engine, not mixer

6. **`get_state_info()` - 1 instance (line 1143)**
   - Solution: Build from `mixer.state()`, `mixer.get_current_tick()`, etc.

7. **`get_total_frames_mixed()` - 1 instance (line 3106)**
   - Map to: `mixer.get_frames_written()`

---

## Next Steps (Priority Order)

### 1. Implement Batch Mixing Loop (HIGHEST PRIORITY)

**Location:** Lines 465-526 (playback loop)

**Current Pattern:**
```rust
for _ in 0..batch_size {
    let frame = mixer.get_next_frame().await;
    if !producer.push(frame) { break; }
}
```

**New Pattern:**
```rust
const BATCH_SIZE: usize = 512; // frames
let mut output = vec![0.0f32; BATCH_SIZE * 2]; // stereo

// Determine if crossfading
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
    handle_marker_events(events, &position_event_tx);
} else {
    let events = mixer.mix_single(
        &buffer_manager,
        current_passage_id,
        &mut output
    ).await?;
    handle_marker_events(events, &position_event_tx);
}

// Push batch to ring buffer
for i in (0..output.len()).step_by(2) {
    let frame = AudioFrame {
        left: output[i],
        right: output[i + 1],
    };
    if !producer.push(frame) { break; }
}
```

**Required:**
- Add `is_crossfading` flag tracking
- Implement `handle_marker_events()` function
- Preserve graduated filling strategy (3-tier based on buffer fill)

---

### 2. Implement Control Methods

**`stop()` implementation:**
```rust
pub async fn stop(&self) -> Result<()> {
    let mut mixer = self.mixer.write().await;
    mixer.clear_all_markers();
    mixer.set_state(MixerState::Idle);
    // Clear current passage tracking
    Ok(())
}
```

**`pause()` implementation:**
```rust
pub async fn pause(&self) -> Result<()> {
    let mut mixer = self.mixer.write().await;
    mixer.set_state(MixerState::Paused);
    self.decoder_worker.pause().await;
    Ok(())
}
```

**`resume()` implementation:**
```rust
pub async fn resume(&self, fade_duration_ms: u64, fade_curve: &str) -> Result<()> {
    let mut mixer = self.mixer.write().await;

    let sample_rate = 44100; // From settings
    let fade_samples = (fade_duration_ms * sample_rate / 1000) as usize;
    let curve = FadeCurve::from_str(fade_curve)?;

    mixer.start_resume_fade(fade_samples, curve);
    mixer.set_state(MixerState::Playing);

    self.decoder_worker.resume().await;
    Ok(())
}
```

---

### 3. Implement `start_passage()` with Marker Calculation

**Location:** Line ~1949 (current `mixer.start_passage()` call)

**Required:**
```rust
// 1. Set current passage
mixer.set_current_passage(passage_id, 0);

// 2. Calculate and add markers
let sample_rate = 44100;

// Position update markers (every 100ms)
let interval_ms = 100; // From settings
let interval_samples = (interval_ms * sample_rate / 1000) as i64;
let passage_duration_samples = ...; // From passage timing

for i in 0..(passage_duration_samples / interval_samples) {
    let tick = i * interval_samples;
    let position_ms = (i * interval_ms) as u64;

    mixer.add_marker(PositionMarker {
        tick,
        passage_id,
        event_type: MarkerEvent::PositionUpdate { position_ms },
    });
}

// Crossfade marker
if let Some(next_passage_id) = next_passage {
    let crossfade_tick = lead_out_point_sample - crossfade_duration_samples;
    mixer.add_marker(PositionMarker {
        tick: crossfade_tick,
        passage_id,
        event_type: MarkerEvent::StartCrossfade { next_passage_id },
    });
}

// Passage complete marker
mixer.add_marker(PositionMarker {
    tick: fade_out_point_sample,
    passage_id,
    event_type: MarkerEvent::PassageComplete,
});
```

---

### 4. Implement `handle_marker_events()`

**New function needed:**
```rust
fn handle_marker_events(
    events: Vec<MarkerEvent>,
    event_tx: &mpsc::UnboundedSender<PlaybackEvent>,
    is_crossfading: &mut bool,
) {
    for event in events {
        match event {
            MarkerEvent::PositionUpdate { position_ms } => {
                event_tx.send(PlaybackEvent::PositionUpdate { position_ms }).ok();
            }
            MarkerEvent::StartCrossfade { next_passage_id } => {
                *is_crossfading = true;
                event_tx.send(PlaybackEvent::CrossfadeStarted { next_passage_id }).ok();
            }
            MarkerEvent::PassageComplete => {
                *is_crossfading = false;
                event_tx.send(PlaybackEvent::PassageComplete).ok();
            }
            MarkerEvent::SongBoundary { new_song_id } => {
                event_tx.send(PlaybackEvent::SongChanged { song_id: new_song_id }).ok();
            }
            MarkerEvent::EndOfFile { unreachable_markers } => {
                warn!("EOF reached with {} unreachable markers", unreachable_markers.len());
                // Handle early EOF
            }
            MarkerEvent::EndOfFileBeforeLeadOut { planned_crossfade_tick, .. } => {
                warn!("EOF before crossfade at tick {}", planned_crossfade_tick);
                // Emergency passage switch
            }
        }
    }
}
```

---

### 5. Implement State Query Methods

**`get_state_info()` replacement:**
```rust
// Build state info from mixer queries
let mixer = self.mixer.read().await;
let state = mixer.state();
let current_tick = mixer.get_current_tick();
let frames_written = mixer.get_frames_written();
// Build StateInfo struct from these
```

**`passage_start_time()` replacement:**
- Track in engine (new field: `passage_start_time: Option<Instant>`)
- Set when starting passage
- Query from engine, not mixer

---

## Rollback Plan

**If issues encountered:**

1. Revert to previous commit:
   ```bash
   git checkout dev
   git branch -D feature/plan014-sub-increment-4b
   ```

2. Legacy mixer remains in use
3. Re-assess integration strategy

---

## Testing Strategy

After each phase:

1. **Compile:** `cargo check -p wkmp-ap`
2. **Fix errors:** Address compilation issues
3. **Commit:** Save progress incrementally
4. **Build:** `cargo build -p wkmp-ap`
5. **Test:** Manual playback testing

---

## Time Estimates Remaining

- **Batch Mixing Loop:** 5-7 hours
- **Marker Calculation:** 4-6 hours
- **Control Methods:** 2-3 hours
- **State Queries:** 1-2 hours
- **Testing:** 3-4 hours
- **Total:** 15-22 hours remaining

---

## Files to Modify

### Primary

- ✅ `wkmp-ap/src/playback/engine.rs` (in progress)

### Supporting (if needed)

- `wkmp-ap/src/playback/events.rs` - May need new event types
- `wkmp-ap/src/state.rs` - State info struct updates

---

## Context for Next Session

**Current State:**
- Imports updated
- Mixer type changed
- Basic initialization done
- 13 compilation errors identified

**Next Action:**
Follow [batch_mixing_implementation_guide.md](batch_mixing_implementation_guide.md) step-by-step to implement batch mixing loop (5-7 hours).

**Key Decision:**
Use fixed 512-frame batches for simplicity, maintain 3-tier graduated filling strategy for underrun prevention.

---

**Document Created:** 2025-01-30
**Status:** Phase 2 partially complete - ready for batch mixing loop implementation
**Branch:** `feature/plan014-sub-increment-4b`
**Estimated Time Remaining:** 15-22 hours
