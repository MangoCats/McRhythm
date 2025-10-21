# Ring Buffer Architecture Refactoring Plan

**Date:** 2025-10-20
**Objective:** Implement SPEC016 ring buffer architecture to replace current PassageBuffer Vec<f32> implementation
**Related Requirements:** [DBD-BUF-010] through [DBD-BUF-060], [DBD-PARAM-070]

---

## Current Implementation vs. SPEC016

### Current Architecture (Incorrect)
```
Audio File → Decoder → Resampler → PassageBuffer (Vec<f32>)
                                    - Growing vector
                                    - sample_count increases, never decreases
                                    - Mixer reads by index (position)
                                    - Samples never removed
                                    - Buffer fill % frozen at decode pause point

                                    ↓ (index-based read)
                                    Mixer → Output Ring Buffer → CPAL
```

### SPEC016 Architecture (Target)
```
Audio File → Decoder → Resampler → Fade Unit → Playout Ring Buffer (per chain)
                                                - Fixed capacity: 661,941 samples
                                                - Starts empty (0%)
                                                - Fills to ~99% (decoder pauses)
                                                - Drains to 0% as mixer consumes
                                                - Dynamic fill percentage

                                                ↓ (pop/drain operation)
                                                Mixer → Output Ring Buffer → CPAL
```

---

## Key Architectural Changes

### 1. Replace PassageBuffer with PlayoutRingBuffer

**File:** `wkmp-ap/src/playback/playout_ring_buffer.rs` (NEW)

```rust
/// Per-chain playout ring buffer
/// **[DBD-BUF-010]** Holds playout_ringbuffer_size stereo samples
pub struct PlayoutRingBuffer {
    /// Lock-free ring buffer for stereo frames
    /// Capacity: playout_ringbuffer_size (default 661,941 samples)
    buffer: HeapRb<AudioFrame>,

    /// Current fill level in samples (frames)
    fill_level: Arc<AtomicUsize>,

    /// Total capacity in samples
    capacity: usize,

    /// Headroom threshold (default 441 samples)
    headroom: usize,

    /// Passage ID this buffer is assigned to
    passage_id: Option<Uuid>,

    /// Flag: decoder should pause (buffer nearly full)
    decoder_should_pause: Arc<AtomicBool>,

    /// Flag: passage end reached, no more samples to decode
    decode_complete: Arc<AtomicBool>,

    /// Total samples written (for tracking)
    total_samples_written: Arc<AtomicU64>,

    /// Total samples read (for tracking)
    total_samples_read: Arc<AtomicU64>,
}
```

**Key Methods:**
- `push_frame(&mut self, frame: AudioFrame) -> Result<(), BufferFullError>` - Decoder-fade chain writes
- `pop_frame(&mut self) -> Result<AudioFrame, BufferEmptyError>` - Mixer reads and removes
- `fill_percent(&self) -> f32` - Returns actual fill percentage (0.0-100.0)
- `should_decoder_pause(&self) -> bool` - Returns true when fill_level >= (capacity - headroom)
- `is_exhausted(&self) -> bool` - Returns true when decode_complete && fill_level == 0

### 2. Update BufferManager

**File:** `wkmp-ap/src/playback/buffer_manager.rs`

Replace `HashMap<Uuid, Arc<RwLock<PassageBuffer>>>` with `HashMap<Uuid, Arc<Mutex<PlayoutRingBuffer>>>`

**Key Changes:**
- `create_buffer()` - Creates PlayoutRingBuffer with capacity from settings
- `append_samples()` - Renamed to `push_samples()`, pushes to ring buffer
- `get_buffer_info()` - Now reads from `ring_buffer.fill_percent()`
- Add `should_decoder_pause(queue_entry_id)` - Checks ring buffer pause flag
- Add `is_buffer_exhausted(queue_entry_id)` - Checks if buffer drained + decode complete

### 3. Update Decoder Pool

**File:** `wkmp-ap/src/playback/decoder_pool.rs`

**Key Changes:**
- After decoding chunk, check `buffer_manager.should_decoder_pause(queue_entry_id)`
- If true, pause this decoder job and move to next priority job
- Resume when buffer falls below threshold (headroom > 441 samples free)

### 4. Update Mixer

**File:** `wkmp-ap/src/playback/pipeline/mixer.rs`

**Critical Change:** Mixer must **drain** samples from ring buffer instead of reading by index

**Current (Index-based Read):**
```rust
let frame = read_frame(buffer, *position).await;  // buffer.get_frame(position)
*position += 1;  // Just increment position
```

**New (Ring Buffer Drain):**
```rust
let frame = buffer_manager.pop_frame(passage_id).await?;  // Removes from ring buffer
// No position tracking needed - ring buffer handles read pointer internally
```

**Underrun Handling:**
- If `pop_frame()` returns `BufferEmptyError` but decode not complete → underrun (warn)
- If `pop_frame()` returns `BufferEmptyError` and decode complete → passage finished

### 5. Update Fade Unit (Decoder Pipeline)

**File:** `wkmp-ap/src/playback/decoder_pool.rs` (or new fade_unit.rs)

**Current:** Fade applied in mixer during playback
**SPEC016:** Fade applied BEFORE buffering

**Key Change:**
```rust
// After resampling, apply fades BEFORE pushing to ring buffer
fn apply_passage_fades(samples: &mut [f32], passage_timing: &PassageTiming, current_position: usize) {
    // [DBD-FADE-030] Apply fade-in if in fade-in region
    // [DBD-FADE-040] Pass-through samples in body region
    // [DBD-FADE-050] Apply fade-out if in fade-out region
}

// Then push faded samples to ring buffer
buffer_manager.push_samples(queue_entry_id, faded_samples).await?;
```

---

## Implementation Phases

### Phase 2: Implement PlayoutRingBuffer
- Create `playout_ring_buffer.rs`
- Implement ring buffer with capacity/headroom from settings
- Add unit tests for push/pop/fill_percent

### Phase 3: Refactor BufferManager
- Replace PassageBuffer with PlayoutRingBuffer
- Update all methods to use ring buffer operations
- Maintain backwards compatibility during transition

### Phase 4: Update Decoder to Pause/Resume
- Check `should_decoder_pause()` after each decode chunk
- Pause decoder when buffer nearly full (headroom threshold)
- Resume when buffer has space

### Phase 5: Move Fade to Pre-Buffer
- Apply fade curves in decoder pipeline before buffering
- Remove fade logic from mixer (mixer just passes through)

### Phase 6: Update Mixer to Drain Buffers
- Replace index-based reads with `pop_frame()` drains
- Handle underruns and buffer exhaustion
- Remove position tracking (ring buffer owns read pointer)

### Phase 7: Update Monitoring
- BufferChainInfo.buffer_fill_percent now shows actual ring buffer fill
- Should update every 1 second showing: 0% → 99% → draining → 0%

### Phase 8: Integration Testing
- Test buffer fill/drain cycle
- Verify decoder pause/resume works correctly
- Verify crossfading still works with drained buffers
- Verify buffer monitoring shows correct percentages

---

## Risk Assessment

### High Risk Changes
1. **Mixer refactoring** - Changing from index-based reads to drain operations
2. **Crossfade coordination** - Need to ensure two ring buffers can be drained simultaneously
3. **Underrun detection** - Must distinguish expected (decode slow) vs. unexpected underruns

### Breaking Changes
- PassageBuffer API completely replaced
- Mixer state machine needs significant refactoring
- All tests using PassageBuffer will need updates

### Mitigation Strategy
- Keep PassageBuffer code temporarily for reference
- Add feature flag to switch between old/new implementation during testing
- Extensive integration tests before removing old code

---

## Database Settings Required

```sql
-- [DBD-PARAM-070] Ring buffer capacity (default: 661,941 samples = 15.01s @ 44.1kHz)
INSERT INTO settings (key, value) VALUES ('playout_ringbuffer_size', '661941');

-- [DBD-PARAM-080] Ring buffer headroom (default: 441 samples)
INSERT INTO settings (key, value) VALUES ('playout_ringbuffer_headroom', '441');
```

---

## Success Criteria

1. ✅ Buffer fill percentage shows 0% when chain assigned to passage
2. ✅ Buffer fill percentage increases from 0% → ~99% as decoder fills
3. ✅ Decoder pauses when buffer reaches (capacity - headroom)
4. ✅ Buffer fill percentage decreases during playback as mixer drains
5. ✅ Buffer fill percentage reaches 0% when passage completes
6. ✅ Crossfading works correctly with drained buffers
7. ✅ All 177 existing tests pass with new architecture
8. ✅ No audio glitches or underruns during normal playback

---

**Status:** Design complete, ready for implementation
**Estimated Effort:** 8-12 hours of development + 4-6 hours testing
**Next Step:** Phase 2 - Implement PlayoutRingBuffer
