# Playback Completion & Duration Tracking Fixes

**🗂️ TIER 2 - DESIGN SPECIFICATION**

Defines HOW playback completion detection and duration tracking are designed to prevent buffer growth race conditions. Derived from manual testing bug reports. See [Document Hierarchy](GOV001-document_hierarchy.md) and [Requirements Enumeration](GOV002-requirements_enumeration.md).

> **Related Documentation:** [Architecture](SPEC001-architecture.md) | [Single Stream Design](SPEC014-single_stream_design.md) | [Crossfade Design](SPEC002-crossfade.md)

**Document Type:** Design Specification (Tier 2)
**Status:** Implemented, archived, superceded by SPEC016-decoder_buffer_design.md
**Date:** 2025-10-19
**Related Requirements:** [REQ-CF-030], [SSD-MIX-060], [REV002]

---

## Category Codes

This document introduces the following requirement categories with prefix **PCF** (Playback Completion Fixes):

- **PCF-DUR** - Duration caching and tracking
- **PCF-COMP** - Completion detection mechanisms
- **PCF-EVENT** - Event-driven completion signaling
- **PCF-TEST** - Testing requirements

---

## Executive Summary

Two critical bugs in passage playback have been identified through manual testing:

1. **Total duration climbs unreasonably** - UI displays "2:30 / 118:13" where total duration grows indefinitely
2. **Passage repeats instead of advancing** - Song loops/restarts instead of moving to next queue entry

Both bugs stem from **incremental buffer decoding** interacting with **live duration calculations** and **racy completion detection**.

---

## Problem Analysis

### Bug #1: Duration Climbing

See [SPEC016 Buffers](SPEC016-decoder_buffer_design.md#buffers) for authoritative buffer behavior specification ([DBD-BUF-010] through [DBD-BUF-060]).

**Observed Behavior:**
- UI shows position like "2:30 / 10s", then "2:30 / 30s", then "2:30 / 118:13"
- Total duration continuously increases during playback
- Position time is accurate, but total time is wrong

**Root Cause:**

Duration is calculated **live from buffer sample count** rather than being cached:

```rust
// audio/types.rs:56-57
pub fn duration_ms(&self) -> u64 {
    (self.sample_count as u64 * 1000) / (self.sample_rate as u64 * self.channels as u64)
}
```

**Data Flow:**

1. Decoder appends chunk → `buffer.append_samples()` → `sample_count += new_samples.len()`
2. Position event handler → `buffer.duration_ms()` → recalculates from **current** sample_count
3. UI receives fresh duration → displays growing total

**Evidence Locations:**
- `/wkmp-ap/src/audio/types.rs:56-57` - Live duration calculation
- `/wkmp-ap/src/audio/types.rs:88-92` - Incremental append updates sample_count
- `/wkmp-ap/src/playback/engine.rs:1520-1543` - Position event handler emits duration
- `/wkmp-ap/src/playback/engine.rs:1547-1562` - State update emits duration

---

### Bug #2: Passage Repeats Instead of Advancing

Related to buffer exhaustion detection ([SPEC016 DBD-BUF-040](SPEC016-decoder_buffer_design.md#buffers)).

**Observed Behavior:**
- First passage plays for ~5 minutes (actual file length)
- Instead of advancing to next queue entry, same song starts over
- Queue appears stuck on first entry

**Root Cause:**

**Race condition** between position tracking and buffer growth:

```rust
// mixer.rs:558-569
pub async fn is_current_finished(&self) -> bool {
    match &self.state {
        MixerState::SinglePassage { buffer, position, .. } => {
            if let Ok(buf) = buffer.try_read() {
                *position >= buf.sample_count  // <-- RACY!
            } else {
                false
            }
        }
        _ => false,
    }
}
```

**Race Window:**

```
Time  Position  Sample_Count  Check Result  Action
----  --------  ------------  ------------  ------
T0    220000    221000        false         Continue playing
T1    221000    221000        should be TRUE but...
T1+ε  221000    265000        false         Decoder appended more samples!
T2    221500    265000        false         Position never catches up
T3    265000    265000        should be TRUE but...
T3+ε  265000    310000        false         Race repeats forever
```

**Evidence Locations:**
- `/wkmp-ap/src/playback/pipeline/mixer.rs:558-569` - Racy completion check
- `/wkmp-ap/src/playback/pipeline/mixer.rs:360-391` - Position increment
- `/wkmp-ap/src/playback/engine.rs:1346-1420` - Playback loop checks `is_current_finished()`

**Secondary Masking:**
- Underrun auto-resume (`mixer.rs:337-354`) outputs flatline frames when position freezes
- Makes it appear song is "looping" instead of stuck

---

## Design Solution

### Principle: Cache Immutable Metadata at Decode Completion

**Key Insight:** Total duration should be **fixed** once decode completes, not recalculated from growing buffer.

### Solution #1: Cache Total Duration in PassageBuffer

**Approach:** Add `total_duration_ms` field that's set once when decode completes or total frames are known.

**Changes Required:**

#### 1.1 Update PassageBuffer Structure

```rust
// audio/types.rs
pub struct PassageBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_count: usize,

    // NEW: Cached total duration (set once decode completes)
    pub total_duration_ms: Option<u64>,  // None = still decoding

    // NEW: Flag indicating all samples loaded
    pub decode_complete: bool,
}

impl PassageBuffer {
    pub fn duration_ms(&self) -> u64 {
        // Use cached value if available, otherwise calculate from current buffer
        self.total_duration_ms.unwrap_or_else(|| {
            (self.sample_count as u64 * 1000) / (self.sample_rate as u64 * self.channels as u64)
        })
    }

    // NEW: Finalize buffer when decode completes
    pub fn finalize(&mut self) {
        self.decode_complete = true;
        self.total_duration_ms = Some(
            (self.sample_count as u64 * 1000) / (self.sample_rate as u64 * self.channels as u64)
        );
    }
}
```

#### 1.2 Call finalize() When Decode Completes

```rust
// playback/decoder_pool.rs (in worker decode loop)
// After last chunk appended:
buffer_handle.finalize_decode().await;  // Sets total_duration_ms
```

**Traceability:** **[PCF-DUR-010]** Cache total duration on decode completion

---

### Solution #2: Detect Completion via Decode Sentinel

**Approach:** Use explicit "decode complete" signal instead of comparing position to growing buffer.

**Changes Required:**

#### 2.1 Add Completion Tracking to PassageBuffer

```rust
// audio/types.rs
pub struct PassageBuffer {
    // ... existing fields ...

    // NEW: Set to true when decoder finishes
    pub decode_complete: bool,

    // NEW: Total frames in passage (set when decode complete)
    pub total_frames: Option<usize>,
}

impl PassageBuffer {
    // NEW: Check if playback has consumed all frames
    pub fn is_exhausted(&self, current_position: usize) -> bool {
        if let Some(total) = self.total_frames {
            current_position >= total
        } else {
            // Decode not complete, cannot be exhausted yet
            false
        }
    }
}
```

#### 2.2 Update Mixer Completion Detection

```rust
// playback/pipeline/mixer.rs
pub async fn is_current_finished(&self) -> bool {
    match &self.state {
        MixerState::SinglePassage { buffer, position, .. } => {
            if let Ok(buf) = buffer.try_read() {
                // Use explicit exhaustion check instead of racy comparison
                buf.is_exhausted(*position)
            } else {
                false
            }
        }
        _ => false,
    }
}
```

#### 2.3 Decoder Signals Completion

```rust
// playback/decoder_pool.rs (after decode loop)
// When all frames decoded:
buffer_handle.mark_decode_complete(total_frames).await;
```

**Traceability:** **[PCF-COMP-010]** Explicit completion signaling

---

### Solution #3: Alternative - Use Decoder Completion Event

**Approach:** Emit explicit event when decoder finishes, queue manager listens for it.

**Pros:**
- Clean separation of concerns
- Aligns with event-driven architecture
- No polling needed

**Cons:**
- Requires new event type
- More complex to implement

**Event Flow:**

```
Decoder Worker
    ↓ (finishes decoding passage)
Emit DecodeComplete { queue_entry_id, total_frames, total_duration_ms }
    ↓
Buffer Manager receives event
    ↓ (updates PassageBuffer)
Set total_frames, total_duration_ms, decode_complete = true
    ↓
Mixer polls is_current_finished()
    ↓ (checks position >= total_frames)
Returns true when playback reaches end
    ↓
Playback Engine advances queue
```

**Traceability:** **[PCF-EVENT-010]** Event-driven decode completion

---

## Recommended Solution

**Use Solution #1 + Solution #2 together:**

1. **Cache total duration** when decode completes (fixes climbing duration bug)
2. **Use explicit `total_frames` sentinel** for completion detection (fixes repeat bug)
3. **Decoder calls `finalize()`** when done to set both values atomically

**Why This Combination:**
- ✅ Fixes both bugs with minimal code change
- ✅ No new event types needed
- ✅ Thread-safe (values set once, never mutated)
- ✅ Aligns with existing architecture
- ✅ Simple to test

---

## Implementation Plan

### Phase 1: Add Fields to PassageBuffer

**File:** `wkmp-ap/src/audio/types.rs`

```rust
pub struct PassageBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_count: usize,

    // NEW FIELDS
    pub decode_complete: bool,
    pub total_frames: Option<usize>,
    pub total_duration_ms: Option<u64>,
}

impl PassageBuffer {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            samples: Vec::new(),
            sample_rate,
            channels,
            sample_count: 0,
            decode_complete: false,       // NEW
            total_frames: None,            // NEW
            total_duration_ms: None,       // NEW
        }
    }

    pub fn duration_ms(&self) -> u64 {
        // Use cached value if decode complete, else calculate from current buffer
        self.total_duration_ms.unwrap_or_else(|| {
            (self.sample_count as u64 * 1000) / (self.sample_rate as u64 * self.channels as u64)
        })
    }

    // NEW: Mark decode complete and cache metadata
    pub fn finalize(&mut self) {
        self.decode_complete = true;
        self.total_frames = Some(self.sample_count);
        self.total_duration_ms = Some(
            (self.sample_count as u64 * 1000) / (self.sample_rate as u64 * self.channels as u64)
        );
    }

    // NEW: Check if position has reached end
    pub fn is_exhausted(&self, position: usize) -> bool {
        if let Some(total) = self.total_frames {
            position >= total
        } else {
            false  // Still decoding, cannot be exhausted
        }
    }
}
```

**Traceability:** **[PCF-DUR-010]**, **[PCF-COMP-010]**

---

### Phase 2: Update BufferManager

**File:** `wkmp-ap/src/playback/buffer_manager.rs`

```rust
impl BufferManager {
    // NEW: Finalize buffer when decode completes
    pub async fn finalize_buffer(&self, queue_entry_id: Uuid) {
        let buffers = self.buffers.read().await;
        if let Some(managed) = buffers.get(&queue_entry_id) {
            let mut buffer = managed.buffer.write().await;
            buffer.finalize();

            tracing::info!(
                "Finalized buffer for {}: {} frames, {}ms duration",
                queue_entry_id,
                buffer.total_frames.unwrap_or(0),
                buffer.total_duration_ms.unwrap_or(0)
            );
        }
    }
}
```

**Traceability:** **[PCF-DUR-020]**

---

### Phase 3: Call finalize() from Decoder

**File:** `wkmp-ap/src/playback/decoder_pool.rs`

Find the decode worker loop where it finishes processing a file, then add:

```rust
// After all chunks decoded (end of decode loop)
info!(
    "Decode complete for queue_entry_id={}: {} total samples",
    queue_entry_id, total_samples_appended
);

// NEW: Signal decode completion
buffer_manager.finalize_buffer(queue_entry_id).await;
```

**Traceability:** **[PCF-DUR-030]**

---

### Phase 4: Update Mixer Completion Check

**File:** `wkmp-ap/src/playback/pipeline/mixer.rs`

```rust
pub async fn is_current_finished(&self) -> bool {
    match &self.state {
        MixerState::SinglePassage { buffer, position, queue_entry_id } => {
            if let Ok(buf) = buffer.try_read() {
                let exhausted = buf.is_exhausted(*position);

                if exhausted {
                    tracing::info!(
                        "Passage {} finished: position={}, total_frames={:?}",
                        queue_entry_id,
                        position,
                        buf.total_frames
                    );
                }

                exhausted
            } else {
                false
            }
        }
        MixerState::Crossfading { .. } => {
            // Crossfade in progress, not finished
            false
        }
        MixerState::Idle | MixerState::Paused { .. } | MixerState::Underrun { .. } => {
            false
        }
    }
}
```

**Traceability:** **[PCF-COMP-020]**

---

### Phase 5: Update Queue Advancement Logic

**File:** `wkmp-ap/src/playback/engine.rs`

Verify that `is_current_finished()` is checked and queue advancement happens:

```rust
// In playback loop (around line 1346-1420)
if mixer.is_current_finished().await {
    info!("Current passage finished, advancing queue");

    // Mark current buffer as exhausted
    if let Some(current_id) = *self.position.queue_entry_id.read().await {
        self.buffer_manager.mark_exhausted(current_id).await;
    }

    // Advance to next queue entry
    let next_entry = self.queue_manager.get_next_entry().await;

    if let Some(next) = next_entry {
        // Start next passage (with or without crossfade)
        self.start_next_passage(next).await?;
    } else {
        // Queue empty, stop playback
        info!("Queue exhausted, stopping playback");
        self.stop().await?;
    }
}
```

**Traceability:** **[PCF-COMP-030]**

---

## Testing Strategy

### Unit Tests

**Test 1: Duration Remains Fixed**
```rust
#[tokio::test]
async fn test_duration_fixed_after_finalize() {
    let mut buffer = PassageBuffer::new(44100, 2);

    // Append first chunk
    buffer.append_samples(vec![0.0; 88200]); // 500ms
    assert_eq!(buffer.duration_ms(), 500);

    // Append second chunk
    buffer.append_samples(vec![0.0; 88200]); // +500ms = 1000ms total
    assert_eq!(buffer.duration_ms(), 1000);  // Still growing

    // Finalize decode
    buffer.finalize();
    assert_eq!(buffer.duration_ms(), 1000);  // Locked

    // Append more (shouldn't happen, but test defensively)
    buffer.append_samples(vec![0.0; 88200]); // Buffer grows to 1500ms
    assert_eq!(buffer.duration_ms(), 1000);  // Duration STILL 1000ms (cached)
}
```

**Test 2: Completion Detection**
```rust
#[tokio::test]
async fn test_exhaustion_detection() {
    let mut buffer = PassageBuffer::new(44100, 2);
    buffer.append_samples(vec![0.0; 88200]); // 500ms = 44100 frames

    // Not finalized yet
    assert!(!buffer.is_exhausted(44100));  // Can't be exhausted

    // Finalize
    buffer.finalize();
    assert_eq!(buffer.total_frames, Some(44100));

    // Check exhaustion
    assert!(!buffer.is_exhausted(10000));   // Position < total
    assert!(!buffer.is_exhausted(44099));   // Position < total
    assert!(buffer.is_exhausted(44100));    // Position == total
    assert!(buffer.is_exhausted(50000));    // Position > total
}
```

**Traceability:** **[PCF-TEST-010]**

---

### Integration Tests

**Test 3: Queue Advances After Passage Completes**
```rust
#[tokio::test]
async fn test_queue_advancement_on_completion() {
    // Setup: Enqueue 3 files
    enqueue("/test1.mp3").await;
    enqueue("/test2.mp3").await;
    enqueue("/test3.mp3").await;

    // Start playback
    play().await;

    // Wait for first passage to complete
    wait_for_event(PassageCompleted).await;

    // Verify second passage started
    let status = get_playback_status().await;
    assert_eq!(status.current_queue_position, 1);  // Moved to second entry

    // Verify duration is fixed
    let duration1 = status.total_duration_ms;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let duration2 = get_playback_status().await.total_duration_ms;
    assert_eq!(duration1, duration2);  // Duration didn't change
}
```

**Traceability:** **[PCF-TEST-020]**

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Decoder never calls `finalize()` | Low | High | Add timeout check in playback loop |
| Race between finalize and position check | Low | Medium | Use atomic bool for decode_complete |
| Old buffers not cleaned up | Low | Low | BufferManager already handles cleanup |
| Crossfade breaks completion detection | Medium | High | Test crossfade + completion scenarios |

---

## Success Criteria

✅ **Bug #1 Fixed:** Total duration remains constant throughout playback
✅ **Bug #2 Fixed:** Queue advances to next entry when passage completes
✅ **No Regressions:** Existing crossfade, pause, seek functionality still works
✅ **Performance:** No measurable impact on playback performance

---

## Future Enhancements

1. **Streaming Mode:** For very large files, don't require full decode before playback
2. **Partial Decode:** Start playback after N seconds buffered, finalize when decode completes
3. **Progress Indication:** Show decode progress separately from playback progress

---

## References

- [SSD-MIX-060] Passage completion detection (SPEC014)
- [REV002] Event-driven position tracking (REV002)
- [REQ-CF-030] Crossfade requirements (REQ001)
- PassageBuffer: `wkmp-ap/src/audio/types.rs`
- CrossfadeMixer: `wkmp-ap/src/playback/pipeline/mixer.rs`
- PlaybackEngine: `wkmp-ap/src/playback/engine.rs`
