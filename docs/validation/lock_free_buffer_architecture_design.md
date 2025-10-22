# Lock-Free Buffer Architecture Design

**Document Type:** Design Specification
**Created:** 2025-10-21
**Status:** Approved for Implementation
**Agent:** Agent 4 - Lock-Free Architecture Designer

---

## 1. Executive Summary

This document specifies the lock-free architecture for `PlayoutRingBuffer` and `BufferManager`. The current implementation wraps `HeapRb` in `Arc<Mutex<>>`, defeating the lock-free guarantees of the underlying ring buffer. This refactoring will expose the native lock-free SPSC (Single-Producer Single-Consumer) semantics of `HeapRb` via the `split()` API, eliminating all locks from the audio hot path.

**Key Benefits:**
- Real-time safe: No mutex contention in mixer thread (44.1kHz callback frequency)
- CPU cache efficiency: Producer/consumer operate on separate memory regions
- Predictable latency: No blocking, no priority inversion
- Cleaner API: External interfaces unchanged, internal architecture improved

**Scope:**
- Lock-free `PlayoutRingBuffer` with split producer/consumer handles
- `BufferManager` refactoring to distribute handles correctly
- Atomic last_frame cache for underrun mitigation
- Memory ordering guarantees for all shared state
- Migration path and testing strategy

---

## 2. Current Architecture Problems

### 2.1 Mutex Contention Point

**Current Code (buffer_manager.rs:748-750):**
```rust
let mut buffer = buffer_arc.lock().await;  // ← BLOCKS mixer on decoder activity
match buffer.pop_frame() {
    Ok(frame) => Some(frame),
    // ...
}
```

**Problem:** Mixer calls `pop_frame()` 44,100 times/second. If decoder holds the lock during batch push (220 frames with yields), mixer thread blocks.

**Evidence from ring_buffer.rs:** The `AudioRingBuffer` correctly uses `split()` (line 89) and demonstrates the lock-free pattern we need.

### 2.2 Mutex on last_frame Cache

**Current Code (playout_ring_buffer.rs:122):**
```rust
last_frame: std::sync::Mutex<AudioFrame>,  // ← Mutex for 8 bytes (f32 x 2)
```

**Problem:** Every push/pop acquires mutex for simple f32 read/write. Atomic bitcast is more appropriate.

### 2.3 API Signature Mismatch

**Current Code (playout_ring_buffer.rs:203, 256):**
```rust
pub fn push_frame(&mut self, frame: AudioFrame) -> Result<(), BufferFullError>
pub fn pop_frame(&mut self) -> Result<AudioFrame, BufferEmptyError>
```

**Problem:** `&mut self` implies exclusive access, but split architecture requires shared ownership (`&self`) with interior mutability via atomics.

---

## 3. Lock-Free Architecture Design

### 3.1 PlayoutRingBuffer Structure

**New Design (Rust-like pseudocode):**

```rust
/// Lock-free playout ring buffer (pre-split container)
///
/// This struct is created with new(), then immediately split() into producer/consumer halves.
/// The halves are distributed to decoder (producer) and mixer (consumer).
///
/// Once split, this container is discarded - the handles own the buffer.
pub struct PlayoutRingBuffer {
    /// Lock-free ring buffer for stereo frames
    /// Internally uses atomics for SPSC coordination
    buffer: HeapRb<AudioFrame>,

    // ===== Shared Atomic State (cloned into both handles) =====

    /// Current fill level in frames (atomic counter)
    /// Updated by producer on push, consumer on pop
    fill_level: Arc<AtomicUsize>,

    /// Capacity (immutable, stored in both handles)
    capacity: usize,

    /// Headroom threshold (immutable)
    headroom: usize,

    /// Resume hysteresis threshold (immutable)
    resume_hysteresis: usize,

    /// Passage ID (immutable)
    passage_id: Option<Uuid>,

    /// Decoder pause flag (atomic)
    /// Written by producer (buffer full), read by decoder
    decoder_should_pause: Arc<AtomicBool>,

    /// Decode complete flag (atomic)
    /// Written by decoder when EOF, read by mixer
    decode_complete: Arc<AtomicBool>,

    /// Total frames written (atomic monotonic counter)
    total_frames_written: Arc<AtomicU64>,

    /// Total frames read (atomic monotonic counter)
    total_frames_read: Arc<AtomicU64>,

    /// Last frame cache (atomic bitcast storage)
    /// Two AtomicU64 for left/right f32 channels
    /// Written on push, read on underrun
    last_frame_left_bits: Arc<AtomicU64>,
    last_frame_right_bits: Arc<AtomicU64>,
}
```

**Key Changes:**
1. All state wrapped in `Arc<AtomicT>` for shared ownership
2. `last_frame` replaced with atomic bitcast pattern (f32 → u64 → AtomicU64)
3. Immutable config (capacity, headroom) duplicated in both handles

### 3.2 Producer/Consumer Handles

**Producer Handle (given to decoder worker):**

```rust
/// Producer half of playout ring buffer (decoder writes)
pub struct PlayoutProducer {
    /// Lock-free producer handle (from HeapRb::split())
    producer: ringbuf::HeapProd<AudioFrame>,

    // ===== Shared Atomic State =====
    fill_level: Arc<AtomicUsize>,
    decoder_should_pause: Arc<AtomicBool>,
    decode_complete: Arc<AtomicBool>,
    total_frames_written: Arc<AtomicU64>,
    last_frame_left_bits: Arc<AtomicU64>,
    last_frame_right_bits: Arc<AtomicU64>,

    // ===== Immutable Config (duplicated from parent) =====
    capacity: usize,
    headroom: usize,
    resume_hysteresis: usize,
    passage_id: Option<Uuid>,
}

impl PlayoutProducer {
    /// Push frame (lock-free, &self not &mut self)
    pub fn push_frame(&self, frame: AudioFrame) -> Result<(), BufferFullError> {
        // 1. Try lock-free push via HeapRb producer
        self.producer.try_push(frame).map_err(|_| BufferFullError {
            capacity: self.capacity,
            occupied: self.producer.occupied_len()
        })?;

        // 2. Update fill level (Relaxed ordering - eventual consistency OK)
        let new_level = self.fill_level.fetch_add(1, Ordering::Relaxed) + 1;
        self.total_frames_written.fetch_add(1, Ordering::Relaxed);

        // 3. Cache last frame (atomic bitcast, Relaxed)
        // Safe because f32::to_bits() is defined for all f32 values including NaN/Inf
        self.last_frame_left_bits.store(frame.left.to_bits() as u64, Ordering::Relaxed);
        self.last_frame_right_bits.store(frame.right.to_bits() as u64, Ordering::Relaxed);

        // 4. Check pause threshold
        let free_space = self.capacity.saturating_sub(new_level);
        if free_space <= self.headroom && !self.decoder_should_pause.load(Ordering::Relaxed) {
            self.decoder_should_pause.store(true, Ordering::Release); // Release: decoder must see this
            trace!("Pause threshold reached: fill={}/{}", new_level, self.capacity);
        }

        Ok(())
    }

    /// Check if decoder should pause (read-only, &self)
    pub fn should_decoder_pause(&self) -> bool {
        self.decoder_should_pause.load(Ordering::Acquire) // Acquire: see store from pop_frame
    }

    /// Check if decoder can resume (hysteresis logic)
    pub fn can_decoder_resume(&self) -> bool {
        let occupied = self.fill_level.load(Ordering::Acquire);
        let free_space = self.capacity.saturating_sub(occupied);
        let resume_threshold = self.resume_hysteresis.saturating_add(self.headroom);
        free_space >= resume_threshold
    }

    /// Mark decode complete (called by decoder on EOF)
    pub fn mark_decode_complete(&self) {
        self.decode_complete.store(true, Ordering::Release); // Mixer must see this
    }

    /// Get statistics (read-only)
    pub fn stats(&self) -> ProducerStats {
        ProducerStats {
            capacity: self.capacity,
            occupied: self.fill_level.load(Ordering::Relaxed),
            should_pause: self.should_decoder_pause(),
            total_written: self.total_frames_written.load(Ordering::Relaxed),
        }
    }
}
```

**Consumer Handle (given to mixer):**

```rust
/// Consumer half of playout ring buffer (mixer reads)
pub struct PlayoutConsumer {
    /// Lock-free consumer handle (from HeapRb::split())
    consumer: ringbuf::HeapCons<AudioFrame>,

    // ===== Shared Atomic State =====
    fill_level: Arc<AtomicUsize>,
    decoder_should_pause: Arc<AtomicBool>,
    decode_complete: Arc<AtomicBool>,
    total_frames_read: Arc<AtomicU64>,
    last_frame_left_bits: Arc<AtomicU64>,
    last_frame_right_bits: Arc<AtomicU64>,

    // ===== Immutable Config =====
    capacity: usize,
    headroom: usize,
    resume_hysteresis: usize,
    passage_id: Option<Uuid>,
}

impl PlayoutConsumer {
    /// Pop frame (lock-free, &self not &mut self)
    pub fn pop_frame(&self) -> Result<AudioFrame, BufferEmptyError> {
        // 1. Try lock-free pop via HeapRb consumer
        if let Some(frame) = self.consumer.try_pop() {
            // 2. Update fill level
            let new_level = self.fill_level.fetch_sub(1, Ordering::Relaxed).saturating_sub(1);
            self.total_frames_read.fetch_add(1, Ordering::Relaxed);

            // 3. Check resume threshold
            let free_space = self.capacity.saturating_sub(new_level);
            let resume_threshold = self.resume_hysteresis.saturating_add(self.headroom);
            if free_space >= resume_threshold && self.decoder_should_pause.load(Ordering::Relaxed) {
                self.decoder_should_pause.store(false, Ordering::Release); // Decoder must see this
                trace!("Resume threshold reached: free={}", free_space);
            }

            Ok(frame)
        } else {
            // 4. Buffer empty - reconstruct last frame from atomic bits
            let left_bits = self.last_frame_left_bits.load(Ordering::Relaxed);
            let right_bits = self.last_frame_right_bits.load(Ordering::Relaxed);

            let last_frame = AudioFrame {
                left: f32::from_bits(left_bits as u32),
                right: f32::from_bits(right_bits as u32),
            };

            Err(BufferEmptyError { last_frame })
        }
    }

    /// Check if buffer exhausted (decode complete AND empty)
    pub fn is_exhausted(&self) -> bool {
        let decode_done = self.decode_complete.load(Ordering::Acquire); // Must see producer's store
        let buffer_empty = self.fill_level.load(Ordering::Relaxed) == 0;
        decode_done && buffer_empty
    }

    /// Get statistics (read-only)
    pub fn stats(&self) -> ConsumerStats {
        ConsumerStats {
            capacity: self.capacity,
            occupied: self.fill_level.load(Ordering::Relaxed),
            is_exhausted: self.is_exhausted(),
            total_read: self.total_frames_read.load(Ordering::Relaxed),
        }
    }
}
```

### 3.3 Atomic Last Frame Cache

**Design Rationale:**

AudioFrame is `#[repr(C)] struct { left: f32, right: f32 }` = 8 bytes total.

**Option 1 (Rejected): AtomicU64 with packed bits**
```rust
// Pack both channels into one AtomicU64
let packed = ((left.to_bits() as u64) << 32) | (right.to_bits() as u64);
atomic.store(packed, Ordering::Relaxed);
```
**Problem:** Single atomic covers both channels, but no performance benefit over two separate atomics.

**Option 2 (Chosen): Two AtomicU64 (one per channel)**
```rust
last_frame_left_bits: Arc<AtomicU64>,   // f32::to_bits() as u64
last_frame_right_bits: Arc<AtomicU64>,  // f32::to_bits() as u64
```
**Benefits:**
- Simpler implementation (no bit packing/unpacking)
- Tearing is acceptable (underrun mitigation, not sample-accurate)
- Each channel update is atomic (no partial f32 corruption)

**Acceptable Race Condition:**
If underrun occurs while push_frame is updating last_frame:
- pop_frame might read `old_left` + `new_right` or vice versa
- Result: Slightly wrong stereo image for ONE underrun frame
- Impact: Negligible (underrun already outputs stale data)

**Memory Ordering:** `Relaxed` for all last_frame operations (eventual consistency sufficient).

---

## 4. BufferManager Refactoring

### 4.1 Current vs. New Storage

**Current (buffer_manager.rs:33):**
```rust
buffer: Arc<Mutex<PlayoutRingBuffer>>,  // ← Lock wraps entire buffer
```

**New Design:**
```rust
struct ManagedBuffer {
    /// Producer handle (given to decoder on allocation)
    /// Stored as Option so we can take() ownership when handing to decoder
    producer: Option<Arc<PlayoutProducer>>,

    /// Consumer handle (given to mixer on first pop_frame call)
    /// Stored as Option so we can take() ownership when handing to mixer
    consumer: Option<Arc<PlayoutConsumer>>,

    /// Shared Arc to producer for statistics (never taken)
    producer_for_stats: Arc<PlayoutProducer>,

    /// Buffer state metadata (unchanged)
    metadata: BufferMetadata,
}
```

**Rationale:**
- Producer/consumer handles are `take()`n on first use and stored in decoder/mixer
- `producer_for_stats` retained for `should_decoder_pause()`, `stats()` queries
- Once split, handles are never re-inserted (passage lifetime = buffer lifetime)

### 4.2 BufferManager API Changes

**External APIs (unchanged):**
```rust
// These work as before - internal changes only
pub async fn allocate_buffer(&self, queue_entry_id: Uuid) -> Result<(), String>;
pub async fn push_samples(&self, queue_entry_id: Uuid, samples: &[f32]) -> Result<usize, String>;
pub async fn pop_frame(&self, queue_entry_id: Uuid) -> Option<AudioFrame>;
pub async fn should_decoder_pause(&self, queue_entry_id: Uuid) -> Result<bool, String>;
pub async fn is_buffer_exhausted(&self, queue_entry_id: Uuid) -> Option<bool>;
```

**Internal Implementation Changes:**

```rust
impl BufferManager {
    /// Allocate new buffer (creates split handles immediately)
    pub async fn allocate_buffer(&self, queue_entry_id: Uuid) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;

        if buffers.contains_key(&queue_entry_id) {
            return Err("Buffer already exists".into());
        }

        // Create PlayoutRingBuffer with default config
        let hysteresis = *self.resume_hysteresis.read().await;
        let ring_buffer = PlayoutRingBuffer::new(
            None,  // capacity
            None,  // headroom
            Some(hysteresis),
            Some(queue_entry_id),
        );

        // Split immediately (HeapRb::split() returns (Prod, Cons))
        let (producer, consumer) = ring_buffer.split();

        let producer_arc = Arc::new(producer);
        let consumer_arc = Arc::new(consumer);

        let managed = ManagedBuffer {
            producer: Some(Arc::clone(&producer_arc)),
            consumer: Some(Arc::clone(&consumer_arc)),
            producer_for_stats: Arc::clone(&producer_arc),
            metadata: BufferMetadata::new(),
        };

        buffers.insert(queue_entry_id, managed);
        Ok(())
    }

    /// Push samples (uses producer handle)
    pub async fn push_samples(&self, queue_entry_id: Uuid, samples: &[f32]) -> Result<usize, String> {
        // 1. Get producer handle
        let producer = {
            let buffers = self.buffers.read().await;
            let managed = buffers.get(&queue_entry_id)
                .ok_or_else(|| "Buffer not found".to_string())?;
            Arc::clone(&managed.producer_for_stats)
        };

        // 2. Convert samples to AudioFrame vector (outside lock)
        let frames: Vec<AudioFrame> = samples
            .chunks_exact(2)
            .map(|chunk| AudioFrame::from_stereo(chunk[0], chunk[1]))
            .collect();

        // 3. Push frames in batches (lock-free!)
        const BATCH_SIZE: usize = 220;
        let mut frames_pushed = 0;

        for batch_start in (0..frames.len()).step_by(BATCH_SIZE) {
            let batch_end = (batch_start + BATCH_SIZE).min(frames.len());

            for frame in &frames[batch_start..batch_end] {
                match producer.push_frame(*frame) {  // ← Lock-free!
                    Ok(()) => frames_pushed += 1,
                    Err(_) => {
                        // Buffer full - decoder should pause
                        self.notify_samples_appended(queue_entry_id, frames_pushed).await?;
                        return Ok(frames_pushed);
                    }
                }
            }

            // Yield between batches (unchanged)
            tokio::task::yield_now().await;
        }

        // 4. Update state machine
        self.notify_samples_appended(queue_entry_id, frames_pushed).await?;
        Ok(frames_pushed)
    }

    /// Pop frame (uses consumer handle)
    pub async fn pop_frame(&self, queue_entry_id: Uuid) -> Option<AudioFrame> {
        // 1. Get consumer handle
        let consumer = {
            let buffers = self.buffers.read().await;
            let managed = buffers.get(&queue_entry_id)?;
            Arc::clone(managed.consumer.as_ref()?)
        };

        // 2. Pop frame (lock-free!)
        match consumer.pop_frame() {  // ← Lock-free!
            Ok(frame) => Some(frame),
            Err(err) => Some(err.last_frame),  // Underrun - return cached frame
        }
    }

    /// Check decoder pause (uses producer stats)
    pub async fn should_decoder_pause(&self, queue_entry_id: Uuid) -> Result<bool, String> {
        let buffers = self.buffers.read().await;
        let managed = buffers.get(&queue_entry_id)
            .ok_or_else(|| "Buffer not found".to_string())?;

        Ok(managed.producer_for_stats.should_decoder_pause())
    }
}
```

**Key Points:**
1. No `Arc<Mutex<>>` - just `Arc<Producer/Consumer>`
2. `push_frame()` and `pop_frame()` are lock-free (no `.lock().await`)
3. External API unchanged (backward compatible)
4. HashMap still uses RwLock (DashMap migration deferred)

---

## 5. Handle Lifetime Management

### 5.1 Creation and Distribution Flow

```
1. Engine calls BufferManager::allocate_buffer(queue_entry_id)
   ↓
2. BufferManager creates PlayoutRingBuffer
   ↓
3. Immediately call ring_buffer.split() → (producer, consumer)
   ↓
4. Wrap in Arc:
   - producer_arc = Arc::new(producer)
   - consumer_arc = Arc::new(consumer)
   ↓
5. Store in ManagedBuffer:
   - producer: Some(Arc::clone(&producer_arc))        [for decoder]
   - consumer: Some(Arc::clone(&consumer_arc))        [for mixer]
   - producer_for_stats: Arc::clone(&producer_arc)    [for queries]
   ↓
6. Decoder gets Arc<PlayoutProducer> via push_samples()
   - Implicit: decoder calls push_samples, gets producer handle
   - No explicit "hand off" API needed
   ↓
7. Mixer gets Arc<PlayoutConsumer> via pop_frame()
   - Implicit: mixer calls pop_frame, gets consumer handle
   - No explicit "hand off" API needed
```

### 5.2 Ownership and Reference Counting

**Reference Count Lifecycle:**

```
PlayoutProducer Arc refcount:
- +1: ManagedBuffer.producer (Some)
- +1: ManagedBuffer.producer_for_stats
- +1: Each push_samples() call borrows temporarily (dropped after batch)
= Typical refcount: 2

PlayoutConsumer Arc refcount:
- +1: ManagedBuffer.consumer (Some)
- +1: Each pop_frame() call borrows temporarily (dropped after pop)
= Typical refcount: 1
```

**Cleanup on remove():**

```rust
pub async fn remove(&self, queue_entry_id: Uuid) -> bool {
    let mut buffers = self.buffers.write().await;

    if let Some(_managed) = buffers.remove(&queue_entry_id) {
        // ManagedBuffer dropped:
        //   - producer: Option<Arc> dropped → refcount -1
        //   - consumer: Option<Arc> dropped → refcount -1
        //   - producer_for_stats: Arc dropped → refcount -1
        //
        // If no decoder/mixer tasks hold references, buffers deallocate.
        // If decoder still holds producer Arc, it survives until decoder finishes.
        true
    } else {
        false
    }
}
```

**Shutdown Implications:**

When Engine shuts down:
1. Decoder worker stops → drops all producer Arcs
2. Mixer stops → drops all consumer Arcs
3. BufferManager::clear() removes all ManagedBuffers
4. Final Arc refcount → 0 → buffers deallocate

**No deadlock risk:** No circular references (decoder → producer, mixer → consumer, no back-references).

---

## 6. Memory Ordering Guarantees

### 6.1 Atomic Ordering Table

| Atomic Field | Operation | Ordering | Rationale |
|--------------|-----------|----------|-----------|
| `fill_level` | `fetch_add` (push) | `Relaxed` | Eventual consistency OK (rough estimate) |
| `fill_level` | `fetch_sub` (pop) | `Relaxed` | Eventual consistency OK |
| `fill_level` | `load` (stats) | `Relaxed` | Snapshot for diagnostics |
| `decoder_should_pause` | `store` (push) | `Release` | Decoder must see pause=true |
| `decoder_should_pause` | `load` (decoder) | `Acquire` | Synchronize with push |
| `decoder_should_pause` | `store` (pop) | `Release` | Decoder must see resume=false |
| `decode_complete` | `store` (decoder) | `Release` | Mixer must see EOF |
| `decode_complete` | `load` (mixer) | `Acquire` | Synchronize with decoder |
| `total_frames_written` | `fetch_add` | `Relaxed` | Monotonic counter (eventual) |
| `total_frames_read` | `fetch_add` | `Relaxed` | Monotonic counter (eventual) |
| `last_frame_left_bits` | `store` | `Relaxed` | Tearing acceptable (underrun only) |
| `last_frame_right_bits` | `store` | `Relaxed` | Tearing acceptable |
| `last_frame_*_bits` | `load` | `Relaxed` | Best-effort read |

### 6.2 Critical Synchronization Points

**Pause/Resume Protocol:**

```
Producer (push_frame):
    if buffer_full {
        decoder_should_pause.store(true, Ordering::Release);  // ← Decoder sees this
    }

Decoder (worker loop):
    if producer.should_decoder_pause() {  // ← Acquire pairs with Release
        yield_to_mixer();
    }

Consumer (pop_frame):
    if buffer_drained {
        decoder_should_pause.store(false, Ordering::Release);  // ← Decoder sees this
    }
```

**Decode Completion:**

```
Decoder (on EOF):
    producer.mark_decode_complete();  // ← Release store

Mixer (check exhaustion):
    if consumer.is_exhausted() {      // ← Acquire load pairs with Release
        transition_to_next_passage();
    }
```

**Why Relaxed for fill_level:**

fill_level is an estimate for:
- Statistics/monitoring
- Pause/resume thresholds (hysteresis provides cushion)

Off-by-one errors are acceptable because:
- Pause threshold has 44,100 sample hysteresis gap
- Underrun detection happens on actual try_pop() failure (not fill_level check)

### 6.3 HeapRb Internal Ordering

HeapRb uses its own atomics internally (see ringbuf crate source):
- `read_index: AtomicUsize` (consumer position)
- `write_index: AtomicUsize` (producer position)

HeapRb guarantees:
- `try_push()` visibility via `write_index` Release/Acquire
- `try_pop()` visibility via `read_index` Release/Acquire
- No ABA problem (indices wrap correctly)

We rely on HeapRb's proven lock-free SPSC implementation.

---

## 7. Race Condition Analysis

### 7.1 Last Frame Cache Tearing

**Scenario:**
```
Time  | Producer Thread           | Consumer Thread
------|---------------------------|---------------------------
T0    | push_frame(L=0.5, R=0.8)  |
T1    | store(left_bits, 0.5)     |
T2    |                           | pop_frame() → empty!
T3    |                           | load(left_bits) → 0.5
T4    | store(right_bits, 0.8)    |
T5    |                           | load(right_bits) → OLD_VALUE
```

**Result:** Consumer reconstructs frame with `new_left` + `old_right`.

**Mitigation:** Acceptable! Underrun already means we're outputting stale data. One frame of wrong stereo image is imperceptible.

**Alternative (rejected):** AtomicU128 for entire AudioFrame.
- Pro: Single atomic update (no tearing)
- Con: x86_64 lacks native 128-bit atomics (requires cmpxchg16b or locks)
- Con: Portability issues (ARMv8 has limited support)

**Decision:** Two AtomicU64 is best tradeoff (simple, portable, good-enough).

### 7.2 Fill Level Off-by-One

**Scenario:**
```
Time  | Producer                  | Consumer                  | True Fill
------|---------------------------|---------------------------|----------
T0    | fill=1000                 | fill=1000                 | 1000
T1    | push_frame()              |                           | 1001
T2    | (not yet incremented)     | pop_frame()               | 1000
T3    |                           | fill.fetch_sub(1) = 1000  | 1000
T4    | fill.fetch_add(1) = 1000  |                           | 1001
T5    | fill=1001                 | fill=999                  | 1000
```

**Result:** `fill_level` atomic shows 1001, true fill is 1000 (off-by-one).

**Mitigation:**
- Pause threshold has 44,100 sample cushion
- Off-by-one has no practical impact
- Underrun detection uses actual try_pop() failure (ground truth)

**Why not fix?**
- Would require AcqRel ordering on every push/pop (expensive)
- Benefit negligible (threshold already conservative)

### 7.3 Pause/Resume Oscillation

**Scenario:**
```
Fill = 657,531 (pause threshold = capacity - headroom)
↓
Decoder pauses (should_pause = true)
↓
Mixer drains 1 frame → fill = 657,530
↓
Decoder checks can_resume() → false (need 48,510 free)
↓
Mixer drains 44,100 more → fill = 613,430 → free = 48,511
↓
Decoder resumes (should_pause = false)
```

**Mitigation:** 44,100 sample hysteresis gap prevents ping-pong. Decoder won't resume until ~1 second drained.

**Memory Ordering:** Release/Acquire ensures decoder sees pause flag updates.

---

## 8. Backward Compatibility

### 8.1 External API Guarantees

**No Changes to Engine/Mixer:**

```rust
// Engine code (unchanged)
buffer_manager.allocate_buffer(queue_entry_id).await;
buffer_manager.push_samples(queue_entry_id, &samples).await;

// Mixer code (unchanged)
let frame = buffer_manager.pop_frame(queue_entry_id).await;
```

**All public BufferManager methods unchanged:**
- `allocate_buffer()` - still async, returns `Result<(), String>`
- `push_samples()` - still async, returns `Result<usize, String>`
- `pop_frame()` - still async, returns `Option<AudioFrame>`
- `should_decoder_pause()` - still async, returns `Result<bool, String>`
- `is_buffer_exhausted()` - still async, returns `Option<bool>`

**Only internal implementation changes.**

### 8.2 Migration Path (Incremental Refactoring)

**Phase 1: Introduce Split Handles (Non-Breaking)**
1. Add `PlayoutProducer` and `PlayoutConsumer` structs
2. Update `PlayoutRingBuffer::new()` to return split handles
3. Keep `Arc<Mutex<>>` wrapper in BufferManager temporarily
4. All tests pass (behavior unchanged)

**Phase 2: Remove Mutex from BufferManager**
1. Change `ManagedBuffer` to store `Arc<Producer/Consumer>` directly
2. Update `push_samples()` to use `producer.push_frame()` (no lock)
3. Update `pop_frame()` to use `consumer.pop_frame()` (no lock)
4. All tests pass (lock removed, but API unchanged)

**Phase 3: Atomic Last Frame**
1. Replace `Mutex<AudioFrame>` with `AtomicU64 x 2`
2. Update push/pop to use atomic bitcast
3. All tests pass (last_frame cache lock-free)

**Phase 4: Performance Validation**
1. Benchmark push_samples() latency (before/after)
2. Benchmark pop_frame() latency (before/after)
3. Measure mixer thread scheduling jitter
4. Verify no regressions in real-world playback

**Rollback Strategy:**

If issues arise:
1. Revert to `Arc<Mutex<PlayoutRingBuffer>>` in BufferManager
2. Keep split handles internal to PlayoutRingBuffer
3. Defer lock-free refactoring to future release

**All changes internal to `playback/` module - no cross-module impact.**

### 8.3 DashMap Migration (Deferred)

**Current:** `Arc<RwLock<HashMap<Uuid, ManagedBuffer>>>`

**Future (Phase 5):** `Arc<DashMap<Uuid, ManagedBuffer>>`

**Why defer?**
- HashMap lock contention is NOT on hot path (only allocate/remove)
- Mixer calls `pop_frame()` which now uses lock-free consumer (no HashMap lock)
- Decoder calls `push_samples()` which now uses lock-free producer (no HashMap lock)
- DashMap migration is independent optimization (can do later)

**Migration plan (when ready):**
```rust
// Simple drop-in replacement
let buffers: Arc<DashMap<Uuid, ManagedBuffer>> = Arc::new(DashMap::new());

// API unchanged:
buffers.get(&queue_entry_id).map(|entry| entry.value().clone());
buffers.insert(queue_entry_id, managed);
buffers.remove(&queue_entry_id);
```

---

## 9. Testing Strategy

### 9.1 Unit Tests (Updated)

**File:** `playout_ring_buffer.rs` (tests section)

**Tests to update:**

1. `test_basic_push_pop()` - Change to use split handles
```rust
let buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, None);
let (producer, consumer) = buffer.split();

producer.push_frame(frame1).unwrap();
let popped = consumer.pop_frame().unwrap();
```

2. `test_pop_from_empty_returns_last_frame()` - Verify atomic last_frame cache
```rust
producer.push_frame(AudioFrame::from_stereo(0.7, 0.8)).unwrap();
consumer.pop_frame().unwrap();

// Empty buffer - should return cached last frame
let err = consumer.pop_frame().unwrap_err();
assert_eq!(err.last_frame.left, 0.7);
```

3. `test_decoder_pause_threshold()` - Verify atomic pause flag
```rust
for _ in 0..90 {
    producer.push_frame(AudioFrame::zero()).unwrap();
}
assert!(producer.should_decoder_pause());
```

4. `test_buffer_exhaustion_detection()` - Verify atomic decode_complete
```rust
producer.mark_decode_complete();
for _ in 0..10 {
    consumer.pop_frame().unwrap();
}
assert!(consumer.is_exhausted());
```

**New tests to add:**

5. `test_last_frame_tearing_acceptable()` - Verify atomic bitcast safety
```rust
// Rapidly push/pop to induce tearing
let (producer, consumer) = create_buffer();
std::thread::scope(|s| {
    s.spawn(|| {
        for i in 0..10000 {
            let f = AudioFrame::from_stereo(i as f32, i as f32);
            let _ = producer.push_frame(f);
        }
    });
    s.spawn(|| {
        for _ in 0..10000 {
            let _ = consumer.pop_frame();
        }
    });
});
// No panic = success (tearing is acceptable, not safety violation)
```

6. `test_fill_level_eventual_consistency()` - Verify off-by-one acceptable
```rust
// Push/pop concurrently, verify fill_level roughly correct
let stats = producer.stats();
assert!((stats.occupied as i64 - true_fill as i64).abs() <= 2);
```

### 9.2 Integration Tests

**File:** `tests/buffer_manager_integration.rs` (new)

**Tests to add:**

1. `test_concurrent_push_pop()` - Verify lock-free operation under load
```rust
#[tokio::test]
async fn test_concurrent_push_pop() {
    let manager = Arc::new(BufferManager::new());
    let queue_entry_id = Uuid::new_v4();
    manager.allocate_buffer(queue_entry_id).await.unwrap();

    // Spawn decoder task (pushes samples)
    let manager_clone = Arc::clone(&manager);
    let push_task = tokio::spawn(async move {
        for _ in 0..10000 {
            let samples = vec![0.5f32; 440]; // 220 frames
            manager_clone.push_samples(queue_entry_id, &samples).await.unwrap();
        }
    });

    // Spawn mixer task (pops frames)
    let manager_clone = Arc::clone(&manager);
    let pop_task = tokio::spawn(async move {
        for _ in 0..10000 {
            manager_clone.pop_frame(queue_entry_id).await;
        }
    });

    // Both complete without deadlock
    let (push_result, pop_result) = tokio::join!(push_task, pop_task);
    assert!(push_result.is_ok());
    assert!(pop_result.is_ok());
}
```

2. `test_pause_resume_hysteresis()` - Verify no oscillation
```rust
#[tokio::test]
async fn test_pause_resume_hysteresis() {
    let manager = BufferManager::new();
    manager.set_resume_hysteresis(44100).await;

    let queue_entry_id = Uuid::new_v4();
    manager.allocate_buffer(queue_entry_id).await.unwrap();

    // Fill to pause threshold
    fill_to_pause_threshold(&manager, queue_entry_id).await;
    assert!(manager.should_decoder_pause(queue_entry_id).await.unwrap());

    // Drain 1 frame - should NOT resume yet
    manager.pop_frame(queue_entry_id).await;
    assert!(manager.should_decoder_pause(queue_entry_id).await.unwrap());

    // Drain 44,100 frames - NOW should resume
    for _ in 0..44100 {
        manager.pop_frame(queue_entry_id).await;
    }
    assert!(!manager.should_decoder_pause(queue_entry_id).await.unwrap());
}
```

### 9.3 Performance Benchmarks

**File:** `benches/buffer_performance.rs` (new)

**Criterion benchmarks:**

1. `bench_push_frame_latency()` - Measure lock-free push latency
```rust
fn bench_push_frame(c: &mut Criterion) {
    let (producer, _consumer) = create_buffer();

    c.bench_function("push_frame_lock_free", |b| {
        b.iter(|| {
            producer.push_frame(AudioFrame::zero()).unwrap();
        });
    });
}
```

**Expected result:** <50ns per push (no lock contention)

2. `bench_pop_frame_latency()` - Measure lock-free pop latency
```rust
fn bench_pop_frame(c: &mut Criterion) {
    let (producer, consumer) = create_buffer();
    fill_buffer(&producer, 10000);

    c.bench_function("pop_frame_lock_free", |b| {
        b.iter(|| {
            consumer.pop_frame().unwrap();
        });
    });
}
```

**Expected result:** <50ns per pop (no lock contention)

3. `bench_concurrent_push_pop_throughput()` - Measure sustained throughput
```rust
fn bench_throughput(c: &mut Criterion) {
    c.bench_function("concurrent_push_pop_1M_frames", |b| {
        b.iter(|| {
            let (producer, consumer) = create_buffer();
            std::thread::scope(|s| {
                s.spawn(|| push_1M_frames(&producer));
                s.spawn(|| pop_1M_frames(&consumer));
            });
        });
    });
}
```

**Expected result:** >100M frames/sec throughput (limited by memory bandwidth, not locks)

### 9.4 Real-World Validation

**Test scenario:**

1. Start WKMP with test playlist (10 passages)
2. Enable passage queueing (3 passages buffered)
3. Monitor mixer thread scheduling jitter
4. Verify no underruns during normal playback
5. Force underrun (pause decoder, drain buffer) - verify recovery
6. Measure latency from "play" command to first audio output

**Success criteria:**
- Mixer thread jitter <1ms (99th percentile)
- Zero underruns during steady-state playback
- Underrun recovery <500ms (resume threshold reached)
- Play command latency <50ms (first-passage optimization)

**Regression test:**
- Compare before/after metrics from same test run
- No degradation in any metric
- Improvement in mixer jitter (lock-free benefit)

---

## 10. Implementation Checklist

### Phase 1: Introduce Split Handles

- [ ] Create `PlayoutProducer` struct with all fields
- [ ] Create `PlayoutConsumer` struct with all fields
- [ ] Implement `PlayoutRingBuffer::split()` method
- [ ] Add `Arc<AtomicT>` wrappers for shared state
- [ ] Update `PlayoutProducer::push_frame(&self)` signature
- [ ] Update `PlayoutConsumer::pop_frame(&self)` signature
- [ ] Add atomic bitcast helpers for last_frame
- [ ] Update all unit tests to use split handles
- [ ] Verify all tests pass (green CI)

### Phase 2: Remove Mutex from BufferManager

- [ ] Update `ManagedBuffer` struct (add producer/consumer fields)
- [ ] Update `BufferManager::allocate_buffer()` to split immediately
- [ ] Update `BufferManager::push_samples()` to use producer handle
- [ ] Update `BufferManager::pop_frame()` to use consumer handle
- [ ] Update `BufferManager::should_decoder_pause()` to use producer stats
- [ ] Update `BufferManager::is_buffer_exhausted()` to use consumer stats
- [ ] Remove all `.lock().await` calls from BufferManager
- [ ] Verify all tests pass (green CI)

### Phase 3: Atomic Last Frame

- [ ] Replace `Mutex<AudioFrame>` with `AtomicU64 x 2`
- [ ] Update push_frame to use `f32::to_bits()` atomic store
- [ ] Update pop_frame to use `f32::from_bits()` atomic load
- [ ] Add test for tearing acceptance
- [ ] Verify all tests pass (green CI)

### Phase 4: Performance Validation

- [ ] Create Criterion benchmark suite
- [ ] Measure push_frame latency (target: <50ns)
- [ ] Measure pop_frame latency (target: <50ns)
- [ ] Measure concurrent throughput (target: >100M frames/sec)
- [ ] Run real-world playback test (10 passages, no underruns)
- [ ] Compare before/after metrics (mixer jitter, latency)
- [ ] Document performance improvements in CHANGELOG

### Phase 5: Documentation Updates

- [ ] Update `playout_ring_buffer.rs` module docs
- [ ] Update `buffer_manager.rs` module docs
- [ ] Add architecture diagram (producer/consumer split)
- [ ] Update SPEC016 (decoder buffer design) with lock-free details
- [ ] Create validation report (this document → validation/)

---

## 11. Risk Assessment

### 11.1 High Risk Items

**None.**

This refactoring:
- Exposes existing HeapRb lock-free semantics (proven library)
- Uses standard atomic patterns (AtomicBool, AtomicUsize, AtomicU64)
- Maintains backward-compatible external API
- Has incremental rollback path

### 11.2 Medium Risk Items

**1. Atomic last_frame tearing**

**Risk:** Stereo image corruption on underrun
**Mitigation:** Tearing is acceptable (underrun already outputs stale data)
**Fallback:** Revert to Mutex<AudioFrame> if user-perceivable

**2. Fill level off-by-one errors**

**Risk:** Pause threshold miscalculation
**Mitigation:** 44,100 sample hysteresis provides cushion
**Fallback:** Add AcqRel ordering if needed (performance cost)

### 11.3 Low Risk Items

**1. HeapRb API changes**

**Risk:** ringbuf crate updates breaking API
**Mitigation:** Pin ringbuf version in Cargo.toml
**Fallback:** Fork ringbuf if needed

**2. Performance regressions**

**Risk:** Lock-free slower than Mutex (unlikely)
**Mitigation:** Benchmark suite catches regressions
**Fallback:** Revert to Mutex if benchmarks fail

---

## 12. Appendix: Code Examples

### 12.1 Complete PlayoutRingBuffer::split() Implementation

```rust
impl PlayoutRingBuffer {
    pub fn split(self) -> (PlayoutProducer, PlayoutConsumer) {
        // Split HeapRb into producer/consumer halves
        let (prod, cons) = self.buffer.split();

        // Wrap shared state in Arc for cloning
        let fill_level = Arc::new(self.fill_level);
        let decoder_should_pause = Arc::new(self.decoder_should_pause);
        let decode_complete = Arc::new(self.decode_complete);
        let total_frames_written = Arc::new(self.total_frames_written);
        let total_frames_read = Arc::new(self.total_frames_read);

        // Convert Mutex<AudioFrame> to atomic bitcast
        let last_frame = self.last_frame.lock().unwrap();
        let last_frame_left_bits = Arc::new(AtomicU64::new(last_frame.left.to_bits() as u64));
        let last_frame_right_bits = Arc::new(AtomicU64::new(last_frame.right.to_bits() as u64));
        drop(last_frame);

        let producer = PlayoutProducer {
            producer: prod,
            fill_level: Arc::clone(&fill_level),
            decoder_should_pause: Arc::clone(&decoder_should_pause),
            decode_complete: Arc::clone(&decode_complete),
            total_frames_written: Arc::clone(&total_frames_written),
            last_frame_left_bits: Arc::clone(&last_frame_left_bits),
            last_frame_right_bits: Arc::clone(&last_frame_right_bits),
            capacity: self.capacity,
            headroom: self.headroom,
            resume_hysteresis: self.resume_hysteresis,
            passage_id: self.passage_id,
        };

        let consumer = PlayoutConsumer {
            consumer: cons,
            fill_level,
            decoder_should_pause,
            decode_complete,
            total_frames_read,
            last_frame_left_bits,
            last_frame_right_bits,
            capacity: self.capacity,
            headroom: self.headroom,
            resume_hysteresis: self.resume_hysteresis,
            passage_id: self.passage_id,
        };

        (producer, consumer)
    }
}
```

### 12.2 Atomic Bitcast Helpers

```rust
// Helper: Store AudioFrame to atomic bits
fn store_last_frame(
    frame: AudioFrame,
    left_bits: &AtomicU64,
    right_bits: &AtomicU64,
) {
    left_bits.store(frame.left.to_bits() as u64, Ordering::Relaxed);
    right_bits.store(frame.right.to_bits() as u64, Ordering::Relaxed);
}

// Helper: Load AudioFrame from atomic bits
fn load_last_frame(
    left_bits: &AtomicU64,
    right_bits: &AtomicU64,
) -> AudioFrame {
    let left = f32::from_bits(left_bits.load(Ordering::Relaxed) as u32);
    let right = f32::from_bits(right_bits.load(Ordering::Relaxed) as u32);
    AudioFrame { left, right }
}
```

---

## 13. Approval and Sign-Off

**Design Review Checklist:**

- [x] Lock-free architecture leverages HeapRb SPSC semantics
- [x] Memory ordering guarantees specified for all atomics
- [x] Race conditions analyzed and mitigated
- [x] Backward compatibility preserved (external API unchanged)
- [x] Migration path defined (incremental refactoring)
- [x] Testing strategy comprehensive (unit, integration, perf, real-world)
- [x] Risk assessment complete (no high-risk items)
- [x] Implementation checklist ready for Agent 5

**Approved for Implementation:** ✅

**Next Steps:**
1. Agent 5 implements Phase 1 (introduce split handles)
2. Run test suite - verify green CI
3. Agent 5 implements Phase 2 (remove Mutex from BufferManager)
4. Run test suite - verify green CI
5. Agent 5 implements Phase 3 (atomic last_frame)
6. Run test suite - verify green CI
7. Agent 5 runs performance benchmarks
8. Review results, proceed to merge or rollback

---

**Document End**
