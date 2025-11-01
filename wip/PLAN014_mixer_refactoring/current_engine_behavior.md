# Current PlaybackEngine Behavior - Pre-Integration

**Date:** 2025-01-30
**Purpose:** Document legacy mixer behavior before Sub-Increment 4b integration

---

## Playback Loop Architecture

### Ring Buffer Filling Strategy

**Three-Tier Graduated Strategy** (lines 465-526):

1. **CRITICAL (< 25% fill)** - Underrun imminent
   - Batch size: `batch_size_low * 2` (aggressive)
   - No sleep between batches
   - Frame-by-frame push via `mixer.get_next_frame().await`

2. **LOW (25-50% fill)** - Below optimal
   - Batch size: `batch_size_low`
   - Minimal sleep (`check_interval.tick().await`)
   - Frame-by-frame push

3. **OPTIMAL (50-75% fill)** - Healthy range
   - Batch size: `batch_size_optimal`
   - Sleep before filling
   - Frame-by-frame push

4. **HIGH (>75% fill)** - Over-filled
   - No filling, just sleep and wait for consumption

### Frame-by-Frame Mixing

**Current Pattern:**
```rust
for _ in 0..batch_size {
    let frame = mixer.get_next_frame().await;  // Async call per frame
    if !producer.push(frame) {                 // Push to ring buffer
        break;
    }
}
```

**Characteristics:**
- Each frame requires async call to mixer
- Lock held during entire batch (write lock on mixer)
- ~44,100 mixer calls per second (44.1kHz sample rate)

---

## Legacy Mixer API Usage

### Initialization (lines 240-243)

```rust
mixer.set_event_channel(position_event_tx.clone());
mixer.set_position_event_interval_ms(interval_ms);
mixer.set_buffer_manager(Arc::clone(&buffer_manager));
mixer.set_mixer_min_start_level(mixer_min_start_level);
```

### Start Passage (line 1949)

```rust
self.mixer.write().await.start_passage(
    passage.passage_id,
    entry,
    buffer_arc,
    next_buffer_opt,
    timing,
    next_timing_opt,
);
```

### Pause (line 759)

```rust
self.mixer.write().await.pause();
```

### Resume (line 718)

```rust
self.mixer.write().await.resume(fade_duration_ms, &fade_curve);
```

### Stop (lines 871-872, 915-918)

```rust
let mut mixer = self.mixer.write().await;
mixer.stop();
```

### Seek (lines 1009-1010)

```rust
let mut mixer = self.mixer.write().await;
mixer.set_position(clamped_position).await?;
```

### State Queries

- `mixer.get_current_passage_id()` - Check if playing
- `mixer.get_state_info()` - Full state snapshot
- `mixer.is_crossfading()` - Crossfade detection
- `mixer.passage_start_time()` - Passage start timestamp
- `mixer.get_position()` - Current position in milliseconds

---

## Event Flow

### Position Events

**Current:** Timer-based polling (legacy mixer)
- Mixer owns event channel
- Sends `PlaybackEvent::PositionUpdate` every N milliseconds
- Interval configured via `set_position_event_interval_ms()`

**Event Handler:** Separate task (line 98-100)
```rust
position_event_tx: mpsc::UnboundedSender<PlaybackEvent>
```

Events sent to UI via SSE:
- `PositionUpdate { position_ms }`
- `CrossfadeStarted`
- `PassageComplete`

---

## Migration Impact Analysis

### High-Impact Changes

1. **Playback Loop Redesign** (lines 465-526)
   - Replace frame-by-frame with batch mixing
   - Maintain graduated filling strategy
   - Preserve underrun prevention logic

2. **Mixer Initialization** (lines 240-243)
   - Remove `set_event_channel`, `set_position_event_interval_ms`
   - Keep `set_buffer_manager` equivalent (pass as reference)
   - Remove `set_mixer_min_start_level` (not in SPEC016)

3. **Start Passage Logic** (line 1949)
   - Replace with `set_current_passage()` + marker setup
   - Calculate crossfade timing in engine
   - Add position/crossfade/complete markers

4. **Event Handling** (line 98-100)
   - Process events from `mix_single()`/`mix_crossfade()` return values
   - Convert `MarkerEvent` to `PlaybackEvent`
   - Send to existing channel (preserve downstream flow)

### Medium-Impact Changes

5. **Pause/Resume** (lines 718, 759)
   - Map to `MixerState::Paused` / `MixerState::Playing`
   - Use `start_resume_fade()` for fade-in

6. **Stop** (lines 871-872, 915-918)
   - Map to `MixerState::Idle`
   - Call `clear_all_markers()`

7. **Seek** (lines 1009-1010)
   - Use `set_current_passage(passage_id, seek_tick)`
   - Recalculate markers from seek point

### Low-Impact Changes

8. **State Queries** (lines 850-851, 1142-1143, 1994-1996)
   - `get_current_passage_id()` - Direct mapping
   - `get_state_info()` - Build from new state + tick
   - `is_crossfading()` - Track in engine (no mixer method)

---

## Batch Size Configuration

**Current Values** (from database settings):
- `batch_size_low`: Default 32 frames
- `batch_size_optimal`: Default 8 frames

**Proposed for SPEC016:**
- Use fixed batch: 512 frames (~11ms @ 44.1kHz)
- Simpler logic, better cache locality
- Matches integration test batch sizes

---

## Critical Invariants to Preserve

1. **No Audio Underruns** - Graduated filling strategy prevents gaps
2. **Position Update Frequency** - Every 100ms (configurable)
3. **Crossfade Timing** - Sample-accurate fade start/end
4. **Ring Buffer Fill** - Maintain 50-75% optimal range
5. **Event Ordering** - Events emitted in chronological order

---

**Next Step:** Begin Phase 2 implementation (batch mixing loop)

**Status:** Documentation complete
