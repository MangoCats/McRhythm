# Batch Mixing Loop Implementation Guide

**Date:** 2025-01-30
**Context:** Sub-Increment 4b, Phase 2 - Converting frame-by-frame to batch mixing
**Estimated Time:** 5-7 hours
**File:** `wkmp-ap/src/playback/engine.rs` (lines 465-526)

---

## Current Architecture Analysis

### Scope Variables Available

From `tokio::spawn` closure (line 419):
- `mixer_clone: Arc<RwLock<Mixer>>`
- `running_clone: Arc<RwLock<bool>>`
- `audio_expected_clone: Arc<AtomicBool>`
- `check_interval_us: u64`
- `batch_size_low: usize` (default 32 frames)
- `batch_size_optimal: usize` (default 8 frames)
- `producer: AudioProducer` (ring buffer producer)

### Missing Variables Needed

1. **`buffer_manager`** - Need to clone and pass to spawn
2. **`position_event_tx`** - Need to clone and pass to spawn for event handling
3. **`is_crossfading`** - Need to add as local state
4. **`current_passage_id`** - Track for mix calls
5. **`next_passage_id`** - Track for crossfade calls

---

## Step 1: Add Required Variables to Spawn (BEFORE line 419)

**Location:** Insert after line 418

```rust
// Clone additional variables for batch mixing
let buffer_manager_clone = Arc::clone(&self.buffer_manager);
let position_event_tx_clone = self.position_event_tx.clone();
```

**Update spawn capture list (line 419):**
```rust
tokio::spawn(async move {
    info!("Mixer thread started");
    let mut check_interval = interval(Duration::from_micros(check_interval_us));

    // [SUB-INC-4B] Track crossfade state
    let mut is_crossfading = false;
    let mut current_passage_id: Option<Uuid> = None;
    let mut next_passage_id: Option<Uuid> = None;
```

---

## Step 2: Define Batch Size Constant

**Location:** After check_interval (line 421)

```rust
// [SUB-INC-4B] Fixed batch size for SPEC016 mixer (512 frames ~= 11ms @ 44.1kHz)
const BATCH_SIZE_FRAMES: usize = 512;
```

---

## Step 3: Replace Frame-by-Frame Loops

### Critical Branch (lines 482-495)

**BEFORE:**
```rust
if is_critical {
    let mut mixer = mixer_clone.write().await;
    let critical_batch_size = batch_size_low * 2;
    for _ in 0..critical_batch_size {
        let frame = mixer.get_next_frame().await;
        if !producer.push(frame) { break; }
    }
}
```

**AFTER:**
```rust
if is_critical {
    // Fill aggressively with larger batch
    let frames_to_mix = BATCH_SIZE_FRAMES * 2; // Double batch when critical
    mix_and_push_batch(
        &mixer_clone,
        &buffer_manager_clone,
        &position_event_tx_clone,
        &producer,
        &mut is_crossfading,
        &mut current_passage_id,
        &mut next_passage_id,
        frames_to_mix,
    ).await;
    // NO SLEEP - loop immediately
}
```

### Low Branch (lines 497-510)

**BEFORE:**
```rust
} else if needs_filling {
    let mut mixer = mixer_clone.write().await;
    for _ in 0..batch_size_low {
        let frame = mixer.get_next_frame().await;
        if !producer.push(frame) { break; }
    }
    check_interval.tick().await;
}
```

**AFTER:**
```rust
} else if needs_filling {
    // Fill with standard batch
    mix_and_push_batch(
        &mixer_clone,
        &buffer_manager_clone,
        &position_event_tx_clone,
        &producer,
        &mut is_crossfading,
        &mut current_passage_id,
        &mut next_passage_id,
        BATCH_SIZE_FRAMES,
    ).await;
    check_interval.tick().await;
}
```

### Optimal Branch (lines 512-522)

**BEFORE:**
```rust
} else if is_optimal {
    check_interval.tick().await;
    let mut mixer = mixer_clone.write().await;
    for _ in 0..batch_size_optimal {
        let frame = mixer.get_next_frame().await;
        if !producer.push(frame) { break; }
    }
}
```

**AFTER:**
```rust
} else if is_optimal {
    check_interval.tick().await;
    // Top up with smaller batch
    let frames_to_mix = BATCH_SIZE_FRAMES / 2; // Half batch when optimal
    mix_and_push_batch(
        &mixer_clone,
        &buffer_manager_clone,
        &position_event_tx_clone,
        &producer,
        &mut is_crossfading,
        &mut current_passage_id,
        &mut next_passage_id,
        frames_to_mix,
    ).await;
}
```

---

## Step 4: Implement `mix_and_push_batch()` Helper

**Location:** Inside the spawn closure, before the loop (after line 422)

```rust
// [SUB-INC-4B] Helper function for batch mixing and ring buffer push
async fn mix_and_push_batch(
    mixer: &Arc<RwLock<Mixer>>,
    buffer_manager: &Arc<BufferManager>,
    event_tx: &mpsc::UnboundedSender<PlaybackEvent>,
    producer: &AudioProducer,
    is_crossfading: &mut bool,
    current_passage_id: &mut Option<Uuid>,
    next_passage_id: &mut Option<Uuid>,
    frames_to_mix: usize,
) {
    // Allocate output buffer (stereo: 2 samples per frame)
    let mut output = vec![0.0f32; frames_to_mix * 2];

    let mut mixer_guard = mixer.write().await;

    // Update current passage ID if changed
    *current_passage_id = mixer_guard.get_current_passage_id();

    // If no passage playing, fill with silence
    let Some(passage_id) = *current_passage_id else {
        // No passage - push silence
        for _ in 0..frames_to_mix {
            if !producer.push(AudioFrame::zero()) {
                break;
            }
        }
        return;
    };

    // Mix batch
    let events = if *is_crossfading && next_passage_id.is_some() {
        // TODO: Crossfade mixing (Phase 3)
        // For now, fall back to single passage
        warn!("Crossfade not yet implemented in batch mixer");
        mixer_guard.mix_single(buffer_manager, passage_id, &mut output)
            .await
            .unwrap_or_else(|e| {
                error!("Mix error: {}", e);
                vec![]
            })
    } else {
        // Single passage mixing
        mixer_guard.mix_single(buffer_manager, passage_id, &mut output)
            .await
            .unwrap_or_else(|e| {
                error!("Mix error: {}", e);
                vec![]
            })
    };

    // Release mixer lock before pushing to ring buffer
    drop(mixer_guard);

    // Handle marker events
    handle_marker_events(events, event_tx, is_crossfading, next_passage_id);

    // Push frames to ring buffer
    for i in (0..output.len()).step_by(2) {
        let frame = AudioFrame {
            left: output[i],
            right: output[i + 1],
        };
        if !producer.push(frame) {
            // Ring buffer full, stop pushing
            break;
        }
    }
}

// [SUB-INC-4B] Convert MarkerEvents to PlaybackEvents
fn handle_marker_events(
    events: Vec<MarkerEvent>,
    event_tx: &mpsc::UnboundedSender<PlaybackEvent>,
    is_crossfading: &mut bool,
    next_passage_id: &mut Option<Uuid>,
) {
    for event in events {
        match event {
            MarkerEvent::PositionUpdate { position_ms } => {
                event_tx.send(PlaybackEvent::PositionUpdate { position_ms }).ok();
            }
            MarkerEvent::StartCrossfade { next_passage_id: next_id } => {
                *is_crossfading = true;
                *next_passage_id = Some(next_id);
                event_tx.send(PlaybackEvent::CrossfadeStarted { next_passage_id: next_id }).ok();
            }
            MarkerEvent::PassageComplete => {
                *is_crossfading = false;
                *next_passage_id = None;
                event_tx.send(PlaybackEvent::PassageComplete).ok();
            }
            MarkerEvent::SongBoundary { new_song_id } => {
                if let Some(song_id) = new_song_id {
                    event_tx.send(PlaybackEvent::SongChanged { song_id }).ok();
                }
            }
            MarkerEvent::EndOfFile { unreachable_markers } => {
                warn!("EOF reached with {} unreachable markers", unreachable_markers.len());
                // TODO: Handle early EOF (Phase 3)
            }
            MarkerEvent::EndOfFileBeforeLeadOut { planned_crossfade_tick, .. } => {
                warn!("EOF before crossfade at tick {}", planned_crossfade_tick);
                // TODO: Emergency passage switch (Phase 3)
            }
        }
    }
}
```

---

## Step 5: Update Mixer Playing Check (line 454-457)

**BEFORE:**
```rust
let mixer_playing = {
    let mixer = mixer_clone.read().await;
    mixer.get_current_passage_id().is_some()
};
```

**AFTER:**
```rust
// Check maintained by current_passage_id variable
let mixer_playing = current_passage_id.is_some();
```

---

## Step 6: Add Missing Imports

**Location:** Top of file (around line 19)

Add to existing imports:
```rust
use crate::playback::mixer::FadeCurve; // For crossfade (Phase 3)
```

---

## Testing After Implementation

### Compile Test

```bash
cargo check -p wkmp-ap
```

**Expected:**
- No errors in playback loop
- Remaining errors in control methods (pause/resume/stop) - addressed in Phase 4

### Build Test

```bash
cargo build -p wkmp-ap
```

### Runtime Test (if compiles)

```bash
RUST_LOG=info cargo run -p wkmp-ap
```

**Verify:**
- Playback starts
- No audio glitches
- Position updates appear in logs
- Ring buffer maintains healthy fill

---

## Known Limitations (TODO for Phase 3)

1. **Crossfade Not Implemented** - Falls back to single passage
   - Need to call `mixer.mix_crossfade()` when crossfading
   - Need crossfade timing parameters

2. **EOF Handling Incomplete** - Logs warning only
   - Need to trigger next passage load
   - Need emergency passage switch logic

3. **Marker Calculation Missing** - No markers added yet
   - Implemented in Phase 3 (`start_passage()` replacement)

---

## Rollback If Issues

If compilation errors are complex or runtime issues occur:

```bash
git checkout wkmp-ap/src/playback/engine.rs
```

This reverts to WIP commit with just import changes.

---

## Success Criteria

- ✅ Compiles without errors in playback loop (lines 419-527)
- ✅ `mix_and_push_batch()` helper compiles
- ✅ `handle_marker_events()` helper compiles
- ✅ Graduated filling strategy preserved (3 tiers)
- ✅ Ring buffer push logic functional
- ⏳ Audio playback (manual test in Phase 5)

---

## Next Phase

After Phase 2 complete:
- **Phase 3:** Implement marker calculation in `start_passage()` (4-6 hours)
- **Phase 4:** Implement control methods (pause/resume/stop/seek) (2-3 hours)
- **Phase 5:** Manual testing with real audio (3-4 hours)

---

**Document Created:** 2025-01-30
**Status:** Implementation guide complete - ready for 5-7 hour implementation session
**Estimated Completion:** Phase 2 complete after following this guide
