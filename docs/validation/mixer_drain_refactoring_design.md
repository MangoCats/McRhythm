# Mixer Drain Refactoring Design

**Document Type:** Technical Design (Validation/Exploration)
**Created:** 2025-10-20
**Status:** DRAFT - Design Only (Implementation Pending)

---

## Executive Summary

This document proposes refactoring the `CrossfadeMixer` from index-based position tracking to ring buffer drain operations, aligning with [DBD-MIX-010] through [DBD-MIX-030] design requirements. The change eliminates manual position management while maintaining sample-accurate crossfading and completion detection.

**Key Design Decision:** Replace position increment (`position += 1`) with explicit drain operations on PassageBuffer, treating buffers as consumable streams rather than random-access arrays.

**Primary Risk:** Crossfading requires **simultaneous reads** from two buffers at the **same relative positions**. Drain operations are inherently sequential and destructive, making synchronized dual-buffer crossfading complex.

---

## 1. Current Mixer Architecture

### 1.1 Current Position Tracking

**File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/mixer.rs`

The mixer maintains **explicit position indices** for each buffer:

```rust
enum MixerState {
    None,

    SinglePassage {
        buffer: Arc<RwLock<PassageBuffer>>,
        passage_id: Uuid,
        position: usize,  // <-- Manual position tracking
        fade_in_curve: Option<FadeCurve>,
        fade_in_duration_samples: usize,
    },

    Crossfading {
        // Current passage (fading out)
        current_buffer: Arc<RwLock<PassageBuffer>>,
        current_passage_id: Uuid,
        current_position: usize,  // <-- Position 1
        fade_out_curve: FadeCurve,
        fade_out_duration_samples: usize,
        fade_out_progress: usize,

        // Next passage (fading in)
        next_buffer: Arc<RwLock<PassageBuffer>>,
        next_passage_id: Uuid,
        next_position: usize,  // <-- Position 2
        fade_in_curve: FadeCurve,
        fade_in_duration_samples: usize,
        fade_in_progress: usize,
    },
}
```

### 1.2 Current Crossfade Operation

**Location:** `mixer.rs:408-485`

```rust
// Crossfading state in get_next_frame()
MixerState::Crossfading {
    current_buffer, current_position,
    next_buffer, next_position, ...
} => {
    // Read from BOTH buffers at their respective positions
    let mut current_frame = read_frame(current_buffer, *current_position).await;
    let mut next_frame = read_frame(next_buffer, *next_position).await;

    // Apply fade curves
    let fade_out_mult = fade_out_curve.calculate_fade_out(fade_out_pos);
    let fade_in_mult = fade_in_curve.calculate_fade_in(fade_in_pos);
    current_frame.apply_volume(fade_out_mult);
    next_frame.apply_volume(fade_in_mult);

    // Mix frames
    mixed.add(&next_frame);

    // Advance BOTH positions independently
    *current_position += 1;
    *next_position += 1;
    *fade_out_progress += 1;
    *fade_in_progress += 1;
}
```

**Critical Observation:** Crossfading requires **non-destructive reads** from both buffers. Current implementation reads frame `N` from buffer A and frame `M` from buffer B **without consuming either**.

### 1.3 Current Helper Function

**Location:** `mixer.rs:943-952`

```rust
async fn read_frame(buffer: &Arc<RwLock<PassageBuffer>>, position: usize) -> AudioFrame {
    if let Ok(buf) = buffer.try_read() {
        buf.get_frame(position).unwrap_or_else(AudioFrame::zero)
    } else {
        AudioFrame::zero()
    }
}
```

This function:
1. Acquires read lock (non-blocking `try_read()`)
2. Calls `PassageBuffer::get_frame(position)` (random access, non-destructive)
3. Returns frame or silence on error

---

## 2. PassageBuffer Current Capabilities

### 2.1 Data Structure

**File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/types.rs:22-51`

```rust
pub struct PassageBuffer {
    pub passage_id: Uuid,
    pub samples: Vec<f32>,           // Interleaved stereo: [L, R, L, R, ...]
    pub sample_rate: u32,            // Always 44100 Hz
    pub channel_count: u16,          // Always 2 (stereo)
    pub sample_count: usize,         // samples.len() / 2
    pub decode_complete: bool,       // [PCF-DUR-010]
    pub total_frames: Option<usize>, // [PCF-COMP-010] Sentinel for completion
    pub total_duration_ms: Option<u64>,
}
```

### 2.2 Current API

**Random-Access Methods:**
- `get_frame(frame_index: usize) -> Option<AudioFrame>` - Non-destructive read at any position
- `append_samples(new_samples: Vec<f32>)` - Grow buffer (incremental decode support)
- `is_exhausted(position: usize) -> bool` - Check if position >= total_frames

**No Drain Operations:** PassageBuffer currently has **NO** drain/consume methods. It is designed as a **random-access buffer**, not a stream.

### 2.3 Ring Buffer Comparison

**File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/ring_buffer.rs`

The `AudioRingBuffer` (used for output) provides:
- `AudioConsumer::pop() -> Option<AudioFrame>` - **Destructive** read (drain operation)
- Lock-free single-producer/single-consumer
- Automatically removes consumed data

**Key Difference:** Ring buffer is a **FIFO stream** (consume and forget), while PassageBuffer is **random-access storage** (read many times from any position).

---

## 3. Drain-Based Refactoring Challenges

### 3.1 Challenge 1: Crossfading with Destructive Reads

**Problem:** During crossfade, mixer reads from **two buffers simultaneously**:
- Frame N from `current_buffer` (fading out)
- Frame M from `next_buffer` (fading in)

If we drain `current_buffer`, we **permanently consume** its data. But we still need to read from `next_buffer` at the **same instant**.

**Constraint:** Drain operations are **sequential and destructive**. You cannot:
1. Drain frame N from buffer A
2. Later re-read frame N from buffer A (it's gone)
3. Drain frame M from buffer B at a different rate

**Example Scenario:**
```
Crossfade duration: 1000 samples
current_buffer: playing from frame 50000 to 51000 (fading out)
next_buffer: playing from frame 0 to 1000 (fading in)

Without position tracking:
- How do we know we're at frame 50000 of current_buffer?
- How do we ensure we drain BOTH buffers at the same rate?
```

### 3.2 Challenge 2: Seeking During Playback

**Current Capability:** `mixer.rs:805-834` implements `set_position()`:
```rust
pub async fn set_position(&mut self, position_frames: usize) -> Result<()> {
    match &mut self.state {
        MixerState::SinglePassage { position, buffer, .. } => {
            let buf = buffer.read().await;
            let max_position = buf.sample_count.saturating_sub(1);
            *position = position_frames.min(max_position);
            Ok(())
        }
        // ... handle crossfade case
    }
}
```

**With Drain:** Seeking backward would require:
1. Resetting buffer drain position (requires new drain cursor API)
2. Potentially re-draining forward to target position (inefficient)
3. Or maintaining **parallel position state** (defeats purpose of drain refactoring)

### 3.3 Challenge 3: Pause/Resume and Underrun Detection

**Pause Behavior (mixer.rs:347-349):**
```rust
if self.pause_state.is_some() {
    return AudioFrame::zero();  // Output silence, DON'T consume buffer
}
```

**With Drain:** If we don't drain during pause, how do we know where we left off when resuming?

**Underrun Detection (mixer.rs:488-507):**
```rust
if let Some((passage_id, next_position, last_frame)) = underrun_check {
    if self.detect_underrun(passage_id, next_position).await {
        self.underrun_state = Some(UnderrunState {
            passage_id,
            flatline_frame: last_frame,
            position_frames: next_position - 1,  // <-- Need position for diagnostics
            ...
        });
    }
}
```

**With Drain:** How do we report current position for diagnostics if we don't track it explicitly?

---

## 4. Proposed Drain-Based Architecture

### 4.1 Design Option A: Add Drain Cursor to PassageBuffer

**Concept:** Extend PassageBuffer with internal drain cursor, keeping random-access capability.

```rust
pub struct PassageBuffer {
    pub passage_id: Uuid,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channel_count: u16,
    pub sample_count: usize,
    pub decode_complete: bool,
    pub total_frames: Option<usize>,
    pub total_duration_ms: Option<u64>,

    // NEW: Drain cursor
    drain_position: usize,  // Tracks consumed frames
}

impl PassageBuffer {
    /// Drain next frame (destructive read)
    pub fn drain_frame(&mut self) -> Option<AudioFrame> {
        if self.drain_position >= self.sample_count {
            return None;
        }

        let frame = self.get_frame(self.drain_position);
        self.drain_position += 1;
        frame
    }

    /// Get current drain position (for diagnostics/seeking)
    pub fn drain_position(&self) -> usize {
        self.drain_position
    }

    /// Reset drain position (for seeking)
    pub fn seek_drain(&mut self, position: usize) {
        self.drain_position = position.min(self.sample_count);
    }

    /// Check if drain is exhausted
    pub fn is_drain_exhausted(&self) -> bool {
        if let Some(total) = self.total_frames {
            self.drain_position >= total
        } else {
            false
        }
    }
}
```

**Mixer Changes:**
```rust
enum MixerState {
    None,

    SinglePassage {
        buffer: Arc<RwLock<PassageBuffer>>,
        passage_id: Uuid,
        // REMOVED: position (now internal to PassageBuffer)
        fade_in_curve: Option<FadeCurve>,
        fade_in_duration_samples: usize,
    },

    Crossfading {
        current_buffer: Arc<RwLock<PassageBuffer>>,
        current_passage_id: Uuid,
        // REMOVED: current_position
        fade_out_curve: FadeCurve,
        fade_out_duration_samples: usize,
        fade_out_progress: usize,

        next_buffer: Arc<RwLock<PassageBuffer>>,
        next_passage_id: Uuid,
        // REMOVED: next_position
        fade_in_curve: FadeCurve,
        fade_in_duration_samples: usize,
        fade_in_progress: usize,
    },
}
```

**Crossfade Implementation:**
```rust
MixerState::Crossfading { current_buffer, next_buffer, ... } => {
    // Drain from BOTH buffers
    let current_frame = {
        let mut buf = current_buffer.write().await;
        buf.drain_frame().unwrap_or_else(AudioFrame::zero)
    };

    let next_frame = {
        let mut buf = next_buffer.write().await;
        buf.drain_frame().unwrap_or_else(AudioFrame::zero)
    };

    // Apply fade curves (same as before)
    // Mix frames (same as before)

    // Check if crossfade complete
    if *fade_out_progress >= *fade_out_duration_samples
        && *fade_in_progress >= *fade_in_duration_samples
    {
        // Transition to SinglePassage (same as before)
    }
}
```

**Pros:**
- Eliminates position tracking in MixerState
- Maintains random-access capability (for seeking)
- Crossfading works (both buffers drain independently)
- Seeking supported via `seek_drain()`

**Cons:**
- PassageBuffer grows more complex (adds state)
- Requires mutable access (`write()` lock) instead of read-only access
- **Not truly "drain" semantics** - data is not freed (still consumes full memory)
- Position still tracked internally (just moved location)

### 4.2 Design Option B: True Ring Buffer with Destructive Drain

**Concept:** Replace PassageBuffer with true ring buffer that frees memory as consumed.

**Problem:** Crossfading requires reading from **two independent ring buffers simultaneously** at potentially **different starting positions**. This violates ring buffer FIFO semantics.

**Example:**
```
Crossfade scenario:
- current_buffer starts draining at sample 50,000 (lead-out point)
- next_buffer starts draining at sample 0 (beginning)
- Both drain for 5000 samples (crossfade duration)

Ring buffer challenge:
- To drain from sample 50,000, we must first drain samples 0-49,999
- But we want to KEEP samples 50,000+ (they're the lead-out region)
- Ring buffer can't "skip ahead" without losing data
```

**Verdict:** **Not feasible** for passage playback without major architecture changes.

### 4.3 Design Option C: Hybrid Approach - Drain with Position Query

**Concept:** Add drain operations while keeping position query methods.

```rust
impl PassageBuffer {
    /// Drain next frame (advances internal cursor)
    pub fn drain_frame(&mut self) -> Option<AudioFrame> {
        // Same as Option A
    }

    /// Get current position WITHOUT draining (for diagnostics)
    pub fn current_position(&self) -> usize {
        self.drain_position
    }

    /// Peek at current frame WITHOUT draining
    pub fn peek_frame(&self) -> Option<AudioFrame> {
        self.get_frame(self.drain_position)
    }
}
```

**Mixer Changes:** Same as Option A, plus:
```rust
// Underrun detection can query position
if let Some((passage_id, buffer)) = get_current_buffer() {
    let buf = buffer.read().await;
    let position = buf.current_position();  // <-- Non-destructive query

    if self.detect_underrun(passage_id, position).await {
        // Log position for diagnostics
    }
}
```

**Pros:**
- Provides drain semantics for normal playback
- Allows position queries for diagnostics/seeking
- Cleaner API than Option A (explicit drain vs peek)

**Cons:**
- Still tracks position internally (same as Option A)
- Doesn't free memory (same as Option A)

---

## 5. Recommended Design: Option A (Drain Cursor)

### 5.1 Rationale

**Why Option A:**
1. **Maintains crossfading capability** - Each buffer drains independently at its own rate
2. **Supports seeking** - `seek_drain()` allows repositioning without re-decoding
3. **Preserves pause/resume** - Cursor stays put when not draining
4. **Diagnostic-friendly** - Position still queryable for underrun detection
5. **Minimal risk** - Functionally equivalent to current system, just reorganized

**Why not Option B (True Ring Buffer):**
- Cannot support crossfading (requires simultaneous reads from two streams at different offsets)
- Cannot support seeking backward
- Would require pre-draining to lead-out point (wasteful)

**Why not Option C (Hybrid):**
- Adds complexity without clear benefit over Option A
- `peek_frame()` + `drain_frame()` is more confusing than just using `drain_position` field

### 5.2 Memory Considerations

**Current System:**
- PassageBuffer holds entire passage in RAM (per [SSD-FBUF-010])
- Maximum 12 buffers × 15 seconds × 8 bytes/sample = ~60 MB

**With Drain Cursor:**
- **No change** in memory usage
- Samples are NOT freed after draining (buffer still holds full passage)
- Drain cursor is metadata only (8 bytes per buffer)

**Why Not Free Memory:**
1. **Seeking requires full buffer** - User may seek backward
2. **Crossfading requires overlapping regions** - Can't free current passage until crossfade completes
3. **Simplicity** - Freeing memory during playback adds complexity and edge cases

**Future Optimization:** If memory becomes a concern, implement **segment-based draining**:
- Divide buffer into 1-second segments
- Free segments only after they're no longer needed (past end of crossfade region)
- Requires tracking segment boundaries and seek constraints

---

## 6. Implementation Plan

### 6.1 Code Change Locations

**File 1:** `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/types.rs`

**Changes to PassageBuffer (lines 22-162):**
```rust
// Line 22: Add drain_position field
pub struct PassageBuffer {
    // ... existing fields ...
    drain_position: usize,  // NEW: Tracks consumption cursor
}

// Line 53: Update constructor
pub fn new(...) -> Self {
    Self {
        // ... existing fields ...
        drain_position: 0,  // NEW
    }
}

// Line 162: Add new methods
impl PassageBuffer {
    /// Drain next frame (advances cursor)
    pub fn drain_frame(&mut self) -> Option<AudioFrame> { ... }

    /// Get current drain position
    pub fn drain_position(&self) -> usize { ... }

    /// Seek drain cursor
    pub fn seek_drain(&mut self, position: usize) { ... }

    /// Check if drain exhausted
    pub fn is_drain_exhausted(&self) -> bool { ... }

    /// Reset drain to beginning (for queue rewind)
    pub fn reset_drain(&mut self) { ... }
}
```

**File 2:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/mixer.rs`

**Changes to MixerState (lines 169-204):**
```diff
 enum MixerState {
     None,

     SinglePassage {
         buffer: Arc<RwLock<PassageBuffer>>,
         passage_id: Uuid,
-        position: usize,
         fade_in_curve: Option<FadeCurve>,
         fade_in_duration_samples: usize,
     },

     Crossfading {
         current_buffer: Arc<RwLock<PassageBuffer>>,
         current_passage_id: Uuid,
-        current_position: usize,
         fade_out_curve: FadeCurve,
         fade_out_duration_samples: usize,
         fade_out_progress: usize,

         next_buffer: Arc<RwLock<PassageBuffer>>,
         next_passage_id: Uuid,
-        next_position: usize,
         fade_in_curve: FadeCurve,
         fade_in_duration_samples: usize,
         fade_in_progress: usize,
     },
 }
```

**Changes to get_next_frame() (lines 345-564):**
```rust
// Line 377-406: SinglePassage case
MixerState::SinglePassage { buffer, passage_id, ... } => {
    // OLD: let mut frame = read_frame(buffer, *position).await;
    //      *position += 1;

    // NEW: Drain from buffer
    let mut frame = {
        let mut buf = buffer.write().await;
        buf.drain_frame().unwrap_or_else(AudioFrame::zero)
    };

    // Apply fade-in (same as before)
    // ... rest unchanged ...
}

// Line 408-485: Crossfading case
MixerState::Crossfading {
    current_buffer, next_buffer, ...
} => {
    // OLD:
    // let mut current_frame = read_frame(current_buffer, *current_position).await;
    // let mut next_frame = read_frame(next_buffer, *next_position).await;
    // *current_position += 1;
    // *next_position += 1;

    // NEW: Drain from both buffers
    let mut current_frame = {
        let mut buf = current_buffer.write().await;
        buf.drain_frame().unwrap_or_else(AudioFrame::zero)
    };

    let mut next_frame = {
        let mut buf = next_buffer.write().await;
        buf.drain_frame().unwrap_or_else(AudioFrame::zero)
    };

    // Apply fade curves and mix (same as before)
    // ... rest unchanged ...
}
```

**Changes to is_current_finished() (lines 584-598):**
```diff
 pub async fn is_current_finished(&self) -> bool {
     match &self.state {
         MixerState::SinglePassage { buffer, .. } => {
             if let Ok(buf) = buffer.try_read() {
-                buf.is_exhausted(*position)
+                buf.is_drain_exhausted()
             } else {
                 false
             }
         }
         _ => false,
     }
 }
```

**Changes to set_position() (lines 805-834):**
```rust
pub async fn set_position(&mut self, position_frames: usize) -> Result<()> {
    match &mut self.state {
        MixerState::SinglePassage { buffer, .. } => {
            // OLD: *position = position_frames.min(max_position);

            // NEW: Seek drain cursor
            let mut buf = buffer.write().await;
            let max_position = buf.sample_count.saturating_sub(1);
            buf.seek_drain(position_frames.min(max_position));
            Ok(())
        }
        MixerState::Crossfading { current_buffer, .. } => {
            // Seek current (fading out) buffer only
            let mut buf = current_buffer.write().await;
            let max_position = buf.sample_count.saturating_sub(1);
            buf.seek_drain(position_frames.min(max_position));
            Ok(())
        }
        // ... rest unchanged ...
    }
}
```

**Changes to get_position() (lines 749-757):**
```rust
pub fn get_position(&self) -> usize {
    match &self.state {
        MixerState::SinglePassage { buffer, .. } => {
            // OLD: *position

            // NEW: Query drain position
            if let Ok(buf) = buffer.try_read() {
                buf.drain_position()
            } else {
                0
            }
        }
        MixerState::Crossfading { current_buffer, .. } => {
            // Query current buffer's drain position
            if let Ok(buf) = current_buffer.try_read() {
                buf.drain_position()
            } else {
                0
            }
        }
        MixerState::None => 0,
    }
}
```

**Changes to underrun detection (lines 488-507):**
```rust
// Underrun check after generating frame
if let Some((passage_id, buffer, last_frame)) = underrun_check {
    // Query current position from buffer
    let next_position = {
        let buf = buffer.read().await;
        buf.drain_position()
    };

    if self.detect_underrun(passage_id, next_position).await {
        // ... rest unchanged ...
    }
}
```

**Remove helper function (lines 943-952):**
```diff
-async fn read_frame(buffer: &Arc<RwLock<PassageBuffer>>, position: usize) -> AudioFrame {
-    if let Ok(buf) = buffer.try_read() {
-        buf.get_frame(position).unwrap_or_else(AudioFrame::zero)
-    } else {
-        AudioFrame::zero()
-    }
-}
```

### 6.2 Testing Strategy

**Unit Tests (add to `wkmp-ap/src/audio/types.rs`):**
```rust
#[cfg(test)]
mod drain_tests {
    use super::*;

    #[test]
    fn test_drain_basic() {
        let mut buffer = PassageBuffer::new(
            Uuid::new_v4(),
            vec![0.1, 0.2, 0.3, 0.4],  // 2 stereo frames
            44100,
            2
        );

        assert_eq!(buffer.drain_position(), 0);

        let frame1 = buffer.drain_frame().unwrap();
        assert_eq!(frame1.left, 0.1);
        assert_eq!(frame1.right, 0.2);
        assert_eq!(buffer.drain_position(), 1);

        let frame2 = buffer.drain_frame().unwrap();
        assert_eq!(frame2.left, 0.3);
        assert_eq!(frame2.right, 0.4);
        assert_eq!(buffer.drain_position(), 2);

        // Exhausted
        assert!(buffer.drain_frame().is_none());
    }

    #[test]
    fn test_seek_drain() {
        let mut buffer = PassageBuffer::new(
            Uuid::new_v4(),
            vec![0.0; 100],  // 50 stereo frames
            44100,
            2
        );

        // Drain 10 frames
        for _ in 0..10 {
            buffer.drain_frame();
        }
        assert_eq!(buffer.drain_position(), 10);

        // Seek back to 5
        buffer.seek_drain(5);
        assert_eq!(buffer.drain_position(), 5);

        // Seek forward to 20
        buffer.seek_drain(20);
        assert_eq!(buffer.drain_position(), 20);

        // Seek beyond end (should clamp)
        buffer.seek_drain(1000);
        assert_eq!(buffer.drain_position(), 50);
    }

    #[test]
    fn test_is_drain_exhausted() {
        let mut buffer = PassageBuffer::new(
            Uuid::new_v4(),
            vec![0.0; 20],  // 10 stereo frames
            44100,
            2
        );

        buffer.finalize();
        assert_eq!(buffer.total_frames, Some(10));

        assert!(!buffer.is_drain_exhausted());

        // Drain to end
        for _ in 0..10 {
            buffer.drain_frame();
        }

        assert!(buffer.is_drain_exhausted());
    }

    #[test]
    fn test_drain_during_incremental_decode() {
        let mut buffer = PassageBuffer::new(
            Uuid::new_v4(),
            vec![],
            44100,
            2
        );

        // Append first chunk
        buffer.append_samples(vec![0.1, 0.2, 0.3, 0.4]);
        assert_eq!(buffer.sample_count, 2);

        // Drain one frame
        let frame = buffer.drain_frame().unwrap();
        assert_eq!(frame.left, 0.1);
        assert_eq!(buffer.drain_position(), 1);

        // Append second chunk
        buffer.append_samples(vec![0.5, 0.6, 0.7, 0.8]);
        assert_eq!(buffer.sample_count, 4);

        // Can continue draining
        let frame = buffer.drain_frame().unwrap();
        assert_eq!(frame.left, 0.3);  // Second frame
        assert_eq!(buffer.drain_position(), 2);
    }
}
```

**Integration Tests (add to `wkmp-ap/tests/`):**
```rust
// Test crossfading with drain operations
#[tokio::test]
async fn test_crossfade_with_drain() {
    let mut mixer = CrossfadeMixer::new();

    let passage1_id = Uuid::new_v4();
    let buffer1 = create_test_buffer(passage1_id, 1000, 0.5);

    let passage2_id = Uuid::new_v4();
    let buffer2 = create_test_buffer(passage2_id, 1000, 0.3);

    // Start first passage
    mixer.start_passage(buffer1.clone(), passage1_id, None, 0).await;

    // Play for a bit
    for _ in 0..100 {
        mixer.get_next_frame().await;
    }

    // Check position advanced via drain
    {
        let buf = buffer1.read().await;
        assert_eq!(buf.drain_position(), 100);
    }

    // Start crossfade
    mixer.start_crossfade(
        buffer2.clone(),
        passage2_id,
        FadeCurve::Linear,
        100,
        FadeCurve::Linear,
        100,
    ).await.unwrap();

    // During crossfade, both buffers should drain
    for _ in 0..100 {
        mixer.get_next_frame().await;
    }

    // Verify both drained
    {
        let buf1 = buffer1.read().await;
        assert_eq!(buf1.drain_position(), 200);  // 100 before + 100 during

        let buf2 = buffer2.read().await;
        assert_eq!(buf2.drain_position(), 100);  // Drained during crossfade
    }

    // Crossfade should be complete
    assert!(!mixer.is_crossfading());
    assert_eq!(mixer.get_current_passage_id(), Some(passage2_id));
}

// Test seeking with drain
#[tokio::test]
async fn test_seek_with_drain() {
    let mut mixer = CrossfadeMixer::new();

    let passage_id = Uuid::new_v4();
    let buffer = create_test_buffer(passage_id, 1000, 0.5);

    mixer.start_passage(buffer.clone(), passage_id, None, 0).await;

    // Play 50 frames
    for _ in 0..50 {
        mixer.get_next_frame().await;
    }

    assert_eq!(mixer.get_position(), 50);

    // Seek to position 100
    mixer.set_position(100).await.unwrap();
    assert_eq!(mixer.get_position(), 100);

    // Verify buffer drain cursor moved
    {
        let buf = buffer.read().await;
        assert_eq!(buf.drain_position(), 100);
    }

    // Continue playing from 100
    mixer.get_next_frame().await;
    assert_eq!(mixer.get_position(), 101);
}
```

### 6.3 Migration Checklist

**Pre-Implementation:**
- [ ] Review design with audio systems specialist
- [ ] Confirm memory constraints acceptable (no freeing during drain)
- [ ] Verify no other components depend on position being external to PassageBuffer

**Implementation Phase 1: PassageBuffer Changes**
- [ ] Add `drain_position` field to PassageBuffer struct
- [ ] Implement `drain_frame()`, `drain_position()`, `seek_drain()`, `is_drain_exhausted()`, `reset_drain()`
- [ ] Update PassageBuffer unit tests
- [ ] Verify incremental decode still works (drain cursor moves as buffer grows)

**Implementation Phase 2: Mixer Changes**
- [ ] Remove position fields from MixerState enum
- [ ] Update `get_next_frame()` SinglePassage case
- [ ] Update `get_next_frame()` Crossfading case
- [ ] Update `is_current_finished()`
- [ ] Update `set_position()`
- [ ] Update `get_position()`
- [ ] Update `get_state_info()`
- [ ] Update underrun detection logic
- [ ] Remove `read_frame()` helper function

**Implementation Phase 3: Integration Testing**
- [ ] Run existing mixer unit tests (should all pass with minimal changes)
- [ ] Add new drain-specific tests
- [ ] Test crossfading with drain operations
- [ ] Test seeking during playback
- [ ] Test pause/resume (verify cursor doesn't advance during pause)
- [ ] Test underrun detection (verify position still reported correctly)

**Implementation Phase 4: BufferManager Integration**
- [ ] Check if BufferManager needs drain cursor awareness
- [ ] Update any position queries to use `drain_position()`
- [ ] Verify buffer state transitions work with drain cursor

**Post-Implementation:**
- [ ] Performance testing (verify no regression from write locks)
- [ ] Memory profiling (confirm no leaks)
- [ ] Stress testing (long playlists, rapid seeking)

---

## 7. Error Handling for Underruns

### 7.1 Current Underrun Handling

**Location:** `mixer.rs:488-507`

Current system detects underrun by:
1. Attempting to read frame at `next_position`
2. Checking if `next_position >= buffer.sample_count` while buffer still decoding
3. Entering underrun state with flatline output

**Key Data:**
- `position_frames` - Where playback stopped
- `flatline_frame` - Last valid audio frame
- `passage_id` - Which passage underran

### 7.2 Drain-Based Underrun Handling

**Approach:** Query drain position after drain attempt.

```rust
// In get_next_frame()
MixerState::SinglePassage { buffer, passage_id, ... } => {
    let mut buf = buffer.write().await;

    // Attempt to drain
    let frame = match buf.drain_frame() {
        Some(f) => f,
        None => {
            // Underrun: drain returned None
            // Check if this is due to incomplete decode or actual completion
            let position = buf.drain_position();

            if !buf.decode_complete {
                // Buffer still decoding - this is an underrun
                let last_frame = buf.get_frame(position.saturating_sub(1))
                    .unwrap_or_else(AudioFrame::zero);

                self.underrun_state = Some(UnderrunState {
                    passage_id: *passage_id,
                    flatline_frame: last_frame,
                    position_frames: position,
                    started_at: Instant::now(),
                });

                return last_frame;  // Flatline output
            } else {
                // Decode complete - passage finished normally
                return AudioFrame::zero();
            }
        }
    };

    // Apply fade-in, etc.
    frame
}
```

**Benefits:**
- Underrun detected immediately when drain fails
- Position still available for diagnostics
- Flatline frame obtained from last successful position

### 7.3 BufferEmptyError Not Used

**Analysis:** Current system does **NOT** use a `BufferEmptyError` type. Instead:
- `PassageBuffer::get_frame()` returns `Option<AudioFrame>`
- `None` indicates position out of bounds
- Mixer interprets `None` as either underrun or completion

**With Drain:**
- `PassageBuffer::drain_frame()` returns `Option<AudioFrame>`
- `None` indicates drain exhausted (either underrun or completion)
- Mixer checks `decode_complete` flag to distinguish

**Recommendation:** Do **NOT** introduce `BufferEmptyError`. Keep using `Option<AudioFrame>` for consistency.

---

## 8. Crossfade Coordination Strategy

### 8.1 Current Coordination

Crossfading maintains **four independent counters**:
1. `current_position` - Frame index in outgoing buffer
2. `next_position` - Frame index in incoming buffer
3. `fade_out_progress` - Samples into fade-out curve
4. `fade_in_progress` - Samples into fade-in curve

**Invariant:** All four counters increment by 1 each frame during crossfade.

**Completion:** When `fade_out_progress >= fade_out_duration` AND `fade_in_progress >= fade_in_duration`

### 8.2 Drain-Based Coordination

**Key Insight:** Drain operations **implicitly** advance buffer positions. We only need to track **fade progress**.

```rust
Crossfading {
    current_buffer: Arc<RwLock<PassageBuffer>>,
    current_passage_id: Uuid,
    // REMOVED: current_position (tracked by current_buffer.drain_position)
    fade_out_curve: FadeCurve,
    fade_out_duration_samples: usize,
    fade_out_progress: usize,  // KEEP: Independent counter

    next_buffer: Arc<RwLock<PassageBuffer>>,
    next_passage_id: Uuid,
    // REMOVED: next_position (tracked by next_buffer.drain_position)
    fade_in_curve: FadeCurve,
    fade_in_duration_samples: usize,
    fade_in_progress: usize,  // KEEP: Independent counter
}
```

**Per-Frame Operations:**
```rust
// 1. Drain from both buffers
let current_frame = current_buffer.write().await.drain_frame().unwrap_or_zero();
let next_frame = next_buffer.write().await.drain_frame().unwrap_or_zero();

// 2. Calculate fade multipliers using PROGRESS counters
let fade_out_mult = fade_out_curve.calculate_fade_out(
    fade_out_progress as f32 / fade_out_duration_samples as f32
);
let fade_in_mult = fade_in_curve.calculate_fade_in(
    fade_in_progress as f32 / fade_in_duration_samples as f32
);

// 3. Apply fades and mix
current_frame.apply_volume(fade_out_mult);
next_frame.apply_volume(fade_in_mult);
mixed.add(&next_frame);

// 4. Advance progress counters (positions advance implicitly via drain)
fade_out_progress += 1;
fade_in_progress += 1;

// 5. Check completion
if fade_out_progress >= fade_out_duration_samples
    && fade_in_progress >= fade_in_duration_samples
{
    // Transition to SinglePassage with next_buffer
}
```

**Guarantees:**
- Both buffers drain exactly once per frame (synchronized)
- Fade curves calculated from explicit progress counters (unchanged)
- Completion detection unchanged
- Buffer positions queryable via `drain_position()` for diagnostics

### 8.3 Crossfade Initiation

**Current Logic (mixer.rs:300-333):**
```rust
pub async fn start_crossfade(...) -> Result<(), Error> {
    match &self.state {
        MixerState::SinglePassage { buffer, passage_id, position, .. } => {
            self.state = MixerState::Crossfading {
                current_buffer: buffer.clone(),
                current_passage_id: *passage_id,
                current_position: *position,  // <-- Capture current position
                // ... rest ...
            };
            Ok(())
        }
        _ => Err(...)
    }
}
```

**Drain-Based Logic:**
```rust
pub async fn start_crossfade(...) -> Result<(), Error> {
    match &self.state {
        MixerState::SinglePassage { buffer, passage_id, .. } => {
            // NO need to capture position - it's already in buffer.drain_position

            self.state = MixerState::Crossfading {
                current_buffer: buffer.clone(),
                current_passage_id: *passage_id,
                // REMOVED: current_position (implicitly at buffer.drain_position)
                fade_out_curve,
                fade_out_duration_samples,
                fade_out_progress: 0,

                next_buffer,
                next_passage_id,
                // REMOVED: next_position (implicitly at 0 if next_buffer just created)
                fade_in_curve,
                fade_in_duration_samples,
                fade_in_progress: 0,
            };
            Ok(())
        }
        _ => Err(...)
    }
}
```

**Assumption:** When crossfade starts, `next_buffer.drain_position` is **already at 0** (or wherever it should start). This is true if:
1. Buffer was just created (drain position defaults to 0)
2. Or buffer was explicitly seeked to start position before enqueueing

**Risk:** If `next_buffer` has non-zero drain position, crossfade will start from wrong position.

**Mitigation:** Ensure BufferManager resets drain position when assigning buffer to queue entry:
```rust
// In BufferManager::register_decoding() or similar
let mut buffer = PassageBuffer::new(...);
buffer.reset_drain();  // <-- Ensure starts at 0
```

---

## 9. MixerState Enum Changes

### 9.1 Before (Current)

```rust
enum MixerState {
    None,

    SinglePassage {
        buffer: Arc<RwLock<PassageBuffer>>,
        passage_id: Uuid,
        position: usize,  // <-- 8 bytes
        fade_in_curve: Option<FadeCurve>,
        fade_in_duration_samples: usize,
    },

    Crossfading {
        current_buffer: Arc<RwLock<PassageBuffer>>,
        current_passage_id: Uuid,
        current_position: usize,  // <-- 8 bytes
        fade_out_curve: FadeCurve,
        fade_out_duration_samples: usize,
        fade_out_progress: usize,

        next_buffer: Arc<RwLock<PassageBuffer>>,
        next_passage_id: Uuid,
        next_position: usize,  // <-- 8 bytes
        fade_in_curve: FadeCurve,
        fade_in_duration_samples: usize,
        fade_in_progress: usize,
    },
}
```

**Total overhead:** 24 bytes (3 × usize) per mixer instance.

### 9.2 After (Drain-Based)

```rust
enum MixerState {
    None,

    SinglePassage {
        buffer: Arc<RwLock<PassageBuffer>>,
        passage_id: Uuid,
        // REMOVED: position (now in buffer.drain_position)
        fade_in_curve: Option<FadeCurve>,
        fade_in_duration_samples: usize,
    },

    Crossfading {
        current_buffer: Arc<RwLock<PassageBuffer>>,
        current_passage_id: Uuid,
        // REMOVED: current_position (now in current_buffer.drain_position)
        fade_out_curve: FadeCurve,
        fade_out_duration_samples: usize,
        fade_out_progress: usize,

        next_buffer: Arc<RwLock<PassageBuffer>>,
        next_passage_id: Uuid,
        // REMOVED: next_position (now in next_buffer.drain_position)
        fade_in_curve: FadeCurve,
        fade_in_duration_samples: usize,
        fade_in_progress: usize,
    },
}
```

**Total overhead reduction:** 24 bytes per mixer instance (negligible).

**Added overhead in PassageBuffer:**
```rust
pub struct PassageBuffer {
    // ... existing fields ...
    drain_position: usize,  // +8 bytes per buffer
}
```

**Net change:** +8 bytes per buffer, -24 bytes per mixer = **+8 bytes × 12 buffers - 24 bytes = +72 bytes** total (negligible).

### 9.3 Semantic Changes

**Current:**
- Position is **local state** in mixer
- Buffer is **stateless storage** (read-only from mixer's perspective)

**Drain-Based:**
- Position is **buffer state** (owned by PassageBuffer)
- Buffer tracks **its own consumption** (mutable state)

**Implications:**
- Buffer becomes **stateful** (cannot be shared read-only across multiple consumers)
- Mixer must acquire **write lock** to drain (was read lock for `get_frame()`)
- Position is **durable** (survives mixer state transitions if buffer is retained)

**Benefit:**
- If we ever need to **pause/switch** buffers, position is preserved in buffer itself
- Cleaner separation: buffer knows where it is, mixer doesn't need to track

---

## 10. Identified Risks and Mitigations

### 10.1 Risk 1: Write Lock Contention

**Risk:** Draining requires `buffer.write().await` (mutable access). If other components also need buffer access during playback, could block.

**Current State:**
- Mixer uses `buffer.try_read()` (non-blocking read lock)
- Fails gracefully if buffer locked (returns silence)

**With Drain:**
- Mixer uses `buffer.write().await` (blocking write lock)
- If buffer locked elsewhere, mixer BLOCKS until released

**Impact:** Potential audio glitches if buffer held by slow operation (e.g., BufferManager diagnostics query).

**Mitigation:**
1. **Use try_write() instead:**
   ```rust
   let frame = match buffer.try_write() {
       Ok(mut buf) => buf.drain_frame().unwrap_or_zero(),
       Err(_) => AudioFrame::zero(),  // Buffer locked - output silence
   };
   ```
2. **Minimize write lock duration:** Drain operation is very fast (O(1) array access + increment).
3. **Audit other buffer access:** Ensure no long-running operations hold buffer locks during playback.

**Recommendation:** Implement Mitigation #1 (try_write) initially, monitor for lock failures.

### 10.2 Risk 2: Buffer Position Divergence

**Risk:** If buffer is accessed outside mixer, drain position could diverge from mixer's expectations.

**Scenario:**
```
1. Mixer drains buffer to position 100
2. External code (e.g., diagnostics) calls buffer.seek_drain(0)
3. Mixer's next drain reads frame 0 instead of 101
4. Playback jumps backward unexpectedly
```

**Mitigation:**
1. **Make drain_position private:** Only expose read-only query, prevent external mutation.
   ```rust
   pub struct PassageBuffer {
       // ... public fields ...
       drain_position: usize,  // <-- NOT pub
   }

   impl PassageBuffer {
       pub fn drain_position(&self) -> usize { self.drain_position }

       // seek_drain() only callable by mixer
       pub(crate) fn seek_drain(&mut self, position: usize) { ... }
   }
   ```
2. **Document ownership:** Buffer's drain cursor is **owned by mixer** - other code must not mutate.
3. **Use separate diagnostic cursor:** If diagnostics need to iterate buffer, use `get_frame(i)` (non-draining).

**Recommendation:** Implement Mitigation #1 (private drain_position, pub(crate) seek).

### 10.3 Risk 3: Crossfade Start Position Mismatch

**Risk:** When crossfade starts, `next_buffer` might have non-zero drain position if it was played before.

**Scenario:**
```
1. Buffer A plays from 0 to 100, then is removed from queue
2. User re-enqueues same file (reuses Buffer A)
3. Buffer A's drain_position is still 100
4. Crossfade starts, reads from position 100 instead of 0
5. Plays wrong section
```

**Mitigation:**
1. **Reset drain on enqueue:**
   ```rust
   // In BufferManager when assigning buffer to queue entry
   pub async fn register_decoding(&self, passage_id: Uuid) -> Arc<RwLock<PassageBuffer>> {
       let mut buffer = PassageBuffer::new(...);
       buffer.reset_drain();  // <-- Always start at 0
       // ...
   }
   ```
2. **Validate at crossfade start:**
   ```rust
   pub async fn start_crossfade(...) {
       // Ensure next_buffer starts at expected position
       {
           let mut buf = next_buffer.write().await;
           buf.reset_drain();  // <-- Force to 0
       }
       // ...
   }
   ```

**Recommendation:** Implement Mitigation #1 (reset on register). Consider #2 as defense-in-depth.

### 10.4 Risk 4: Seek Backward Beyond Drained Region

**Risk:** User seeks to position 50, but buffer already drained to position 100. Seeking backward works (cursor resets to 50), but this is unexpected for "drain" semantics.

**Observation:** This is **NOT a bug** with our design - we explicitly support seeking via `seek_drain()`. But it violates typical "drain" semantics (drain is usually unidirectional).

**Mitigation:** Accept this as intentional design:
- PassageBuffer is **NOT a true drain** (doesn't free memory)
- `drain_frame()` is **convenience API** for "consume next frame", but buffer remains random-access
- Seeking is a feature, not a bug

**Documentation:** Clearly document that PassageBuffer supports bidirectional seeking even after draining.

### 10.5 Risk 5: Pause Position Drift

**Risk:** During pause, mixer outputs silence but doesn't drain buffer. If drain cursor is accidentally advanced elsewhere, resume will play from wrong position.

**Mitigation:** Same as Risk 2 - make drain_position private, prevent external mutation.

**Additional Safety:** Verify in tests that pause/resume doesn't change position:
```rust
#[tokio::test]
async fn test_pause_preserves_position() {
    let mut mixer = CrossfadeMixer::new();
    let passage_id = Uuid::new_v4();
    let buffer = create_test_buffer(passage_id, 1000, 0.5);

    mixer.start_passage(buffer.clone(), passage_id, None, 0).await;

    // Play 50 frames
    for _ in 0..50 {
        mixer.get_next_frame().await;
    }
    assert_eq!(mixer.get_position(), 50);

    // Pause
    mixer.pause();

    // Generate 100 frames while paused (should output silence without draining)
    for _ in 0..100 {
        let frame = mixer.get_next_frame().await;
        assert_eq!(frame, AudioFrame::zero());
    }

    // Position should NOT change during pause
    assert_eq!(mixer.get_position(), 50);

    // Resume and verify playback continues from position 50
    mixer.resume(500, "exponential");
    mixer.get_next_frame().await;
    assert_eq!(mixer.get_position(), 51);
}
```

---

## 11. Summary of Key Design Decisions

### 11.1 Chosen Approach

**Option A: Drain Cursor in PassageBuffer**

Add internal `drain_position` field to PassageBuffer with methods:
- `drain_frame() -> Option<AudioFrame>` - Consume next frame
- `drain_position() -> usize` - Query current position
- `seek_drain(position)` - Reposition cursor
- `is_drain_exhausted() -> bool` - Check completion
- `reset_drain()` - Return to beginning

### 11.2 Rejected Alternatives

**Option B (True Ring Buffer):** Rejected because:
- Cannot support crossfading (needs dual simultaneous reads at different offsets)
- Cannot support seeking backward
- Would require buffering entire lead-in region even if not played

**Option C (Hybrid Peek/Drain):** Rejected because:
- More complex API than Option A
- No clear benefit over internal cursor
- Confusing distinction between peek and drain

### 11.3 Tradeoffs Accepted

**Memory:** No reduction in memory usage - buffers still hold full passage for seeking support.

**Lock Contention:** Write locks required for draining (was read locks). Mitigated with `try_write()` fallback.

**Semantic Purity:** "Drain" normally implies unidirectional consumption. We allow bidirectional seeking. Accepted as necessary for playback features.

**State Location:** Position moves from mixer (local) to buffer (durable). Accepted as cleaner separation of concerns.

### 11.4 Implementation Complexity

**Low Risk Changes:**
- PassageBuffer extension (add 4 methods, 1 field)
- MixerState simplification (remove 3 fields)
- Most mixer methods only need minor adjustments

**Medium Risk Changes:**
- Crossfading logic (requires write locks on both buffers simultaneously)
- Underrun detection (needs new position query pattern)

**High Risk Changes:**
- None identified

### 11.5 Testing Requirements

**Critical Test Cases:**
1. Basic drain operation (sequential consumption)
2. Drain during incremental decode (growing buffer)
3. Crossfading with drain (dual buffer synchronization)
4. Seeking backward and forward
5. Pause/resume position preservation
6. Underrun detection with drain cursor
7. Completion detection (drain exhausted vs decode complete)
8. Lock contention handling (try_write failure)

**Performance Tests:**
1. Drain throughput (vs current get_frame)
2. Write lock acquisition time
3. Memory usage (verify no leaks)

**Stress Tests:**
1. Long playlist (100+ passages)
2. Rapid seeking
3. Many crossfades in sequence

---

## 12. Conclusion and Recommendation

### 12.1 Design Feasibility

**Verdict: FEASIBLE with Option A (Drain Cursor)**

The proposed drain-based refactoring:
- ✅ Maintains all current functionality (crossfading, seeking, pause/resume)
- ✅ Simplifies mixer state (removes redundant position tracking)
- ✅ Minimal performance impact (write locks are fast)
- ✅ Low implementation risk (incremental changes, testable at each step)
- ⚠️ Does NOT reduce memory usage (buffers still hold full passages)
- ⚠️ Requires write locks instead of read locks (mitigated with try_write)

### 12.2 Is This Worth Implementing?

**Arguments FOR:**
- **Cleaner architecture:** Position is buffer property, not mixer state
- **Reduced state tracking:** Mixer doesn't maintain parallel position counters
- **Better encapsulation:** Buffer owns its consumption cursor
- **Aligns with [DBD-MIX-010]:** Mixer operates on streams, not indices

**Arguments AGAINST:**
- **No functional benefit:** Current system works correctly
- **Minimal code reduction:** Only saves 24 bytes and ~10 lines of code
- **Increased lock contention risk:** Write locks slower than read locks
- **Migration effort:** ~200 lines of code changes + testing
- **No memory savings:** Buffers still hold full passages

### 12.3 Recommendation

**PROCEED with caution IF:**
1. Alignment with [DBD-MIX-010] design philosophy is high priority
2. Team has bandwidth for low-priority refactoring
3. This enables future features (e.g., buffer streaming, memory reduction)

**DEFER IF:**
1. System is working well (don't fix what isn't broken)
2. Higher priority work exists
3. No clear user-facing benefit

**If proceeding:**
1. Start with PassageBuffer changes (Phase 1)
2. Write comprehensive tests before touching mixer
3. Use `try_write()` for lock-free fallback
4. Monitor performance in production before full rollout
5. Keep current implementation in version control for rollback

### 12.4 Future Enhancements (If Implemented)

Once drain cursor is in place, consider:
1. **Segment-based memory freeing:** Free drained segments if memory constrained
2. **Streaming decode:** Don't buffer entire passage, decode on-demand
3. **Circular buffer optimization:** Reuse freed regions for incremental decode
4. **Multi-reader support:** Allow multiple consumers with independent drain cursors

---

**END OF DESIGN DOCUMENT**

---

## Appendix A: Code Change Manifest

**Files Modified:**
1. `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/types.rs`
   - Lines 22-51: Add `drain_position` field to PassageBuffer
   - Lines 53-68: Update constructor
   - Lines 162+: Add drain methods (drain_frame, seek_drain, etc.)
   - Lines 596+: Add drain unit tests

2. `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/mixer.rs`
   - Lines 169-204: Remove position fields from MixerState enum
   - Lines 276-283: Update start_passage() signature (remove position)
   - Lines 300-333: Update start_crossfade() (remove position capture)
   - Lines 377-406: Refactor SinglePassage frame generation (use drain)
   - Lines 408-485: Refactor Crossfading frame generation (use drain)
   - Lines 584-598: Update is_current_finished() (use is_drain_exhausted)
   - Lines 749-757: Update get_position() (query drain_position)
   - Lines 760-790: Update get_state_info() (query drain_position)
   - Lines 805-834: Update set_position() (use seek_drain)
   - Lines 943-952: Remove read_frame() helper function
   - Lines 1000+: Update unit tests to work with drain semantics

**Files Not Modified:**
- `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/buffer_manager.rs` (may need drain_position queries)
- `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/engine.rs` (may need position query updates)

**Estimated Change Size:**
- ~150 lines modified
- ~50 lines removed
- ~100 lines added (mostly tests)
- Net: ~100 lines added

---

## Appendix B: Risk Matrix

| Risk | Probability | Impact | Severity | Mitigation |
|------|------------|--------|----------|------------|
| Write lock contention | Medium | Low | **Medium** | Use try_write() with silence fallback |
| Buffer position divergence | Low | High | **Medium** | Make drain_position private |
| Crossfade position mismatch | Low | Medium | **Low** | Reset drain on buffer registration |
| Seek backward confusion | Low | Low | **Low** | Document bidirectional seeking |
| Pause position drift | Very Low | Medium | **Low** | Private drain_position + tests |
| Performance regression | Low | Low | **Low** | Benchmark before/after |
| Memory leak | Very Low | High | **Low** | Memory profiling tests |

**Overall Risk Assessment: LOW**

Most risks are low probability and have effective mitigations. No showstoppers identified.

---

## Appendix C: Performance Considerations

### C.1 Lock Acquisition Cost

**Current (Read Lock):**
```rust
buffer.try_read()  // RwLock read acquisition (multiple concurrent readers allowed)
```

**Proposed (Write Lock):**
```rust
buffer.try_write()  // RwLock write acquisition (exclusive access required)
```

**Expected Impact:**
- Read locks: ~10-20ns (uncontended)
- Write locks: ~10-20ns (uncontended), blocks on contention
- With try_write(): Returns Err immediately if locked (no blocking)

**Contention Sources:**
- BufferManager status queries (rare, short duration)
- Diagnostic endpoints (rare)
- Other mixer threads (N/A - single mixer instance per playback engine)

**Verdict:** **Negligible impact** - locks are rarely contended during playback.

### C.2 Memory Access Pattern

**Current:**
```rust
buf.get_frame(position)  // Random access: samples[position * 2..position * 2 + 2]
*position += 1           // Increment separate variable
```

**Proposed:**
```rust
buf.drain_frame()  // Random access: samples[drain_position * 2..drain_position * 2 + 2]
                   // Increment internal field: drain_position += 1
```

**Cache Impact:** Identical (both access same memory regions sequentially).

**Verdict:** **No difference** in memory access pattern.

### C.3 Crossfade Lock Ordering

**Current:**
```rust
let current_frame = read_frame(&current_buffer, current_position).await;
let next_frame = read_frame(&next_buffer, next_position).await;
// Both locks released before proceeding
```

**Proposed:**
```rust
let current_frame = {
    let mut buf = current_buffer.write().await;
    buf.drain_frame().unwrap_or_zero()
};  // Lock released

let next_frame = {
    let mut buf = next_buffer.write().await;
    buf.drain_frame().unwrap_or_zero()
};  // Lock released
```

**Lock Hold Time:** ~50ns per buffer (array access + increment).

**Total Overhead:** ~100ns per frame (2 buffers × 50ns).

**At 44.1kHz:** Frame period = 22.7μs, overhead = 0.4% of frame time.

**Verdict:** **Negligible overhead** (less than 1% of frame budget).

---

**Document Version:** 1.0
**Author:** Rust Audio Systems Specialist
**Review Status:** Pending Technical Lead Approval
