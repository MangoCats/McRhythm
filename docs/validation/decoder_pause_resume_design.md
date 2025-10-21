# Decoder Pause/Resume Mechanism Design

**Document Type:** Technical Design (Pre-Implementation)
**Created:** 2025-10-20
**Status:** Draft - Design Review
**Requirement Traceability:** [DBD-DEC-020] through [DBD-DEC-040], [DBD-BUF-050]

---

## 1. Executive Summary

This document designs a pause/resume mechanism for the DecoderPool to prevent buffer overflow during decoding. The mechanism allows the decoder to temporarily suspend work on a passage when its buffer is nearly full, switch to decoding a higher-priority or lower-filled passage, and resume the paused job when buffer space becomes available.

**Key Design Decisions:**
- **Event-driven architecture:** Leverage existing BufferManager event system for buffer fill monitoring
- **Priority preservation:** Paused jobs retain their original priority when resuming
- **Hysteresis:** Use dual thresholds (pause/resume) to prevent oscillation
- **Minimal state:** Reuse existing DecodeRequest structure with pause flag
- **Sample-level resume:** Resume exactly where decode paused (no re-decoding)

---

## 2. Current Decoder Architecture Analysis

### 2.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         PlaybackEngine                          │
│  Calls: decoder_pool.submit(passage_id, passage, priority, ..) │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                         DecoderPool                             │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  SharedPoolState                                          │  │
│  │  - queue: Mutex<BinaryHeap<DecodeRequest>>               │  │
│  │  - condvar: Condvar                                       │  │
│  │  - stop_flag: AtomicBool                                 │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────┐              ┌──────────────┐                │
│  │  Worker 0    │              │  Worker 1    │                │
│  │              │              │              │                │
│  │  Loop:       │              │  Loop:       │                │
│  │  1. Pop job  │              │  1. Pop job  │                │
│  │  2. Decode   │              │  2. Decode   │                │
│  │  3. Repeat   │              │  3. Repeat   │                │
│  └──────┬───────┘              └──────┬───────┘                │
└─────────┼──────────────────────────────┼────────────────────────┘
          │                              │
          ▼                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      BufferManager                              │
│  - register_decoding(passage_id) -> buffer_handle               │
│  - notify_samples_appended(passage_id, count)                  │
│  - finalize_buffer(passage_id, total_samples)                  │
│  - get_headroom(passage_id) -> usize                           │
│  - Emits: BufferEvent::StateChanged, ReadyForStart, etc.       │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Decode Job Priority System

**Location:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/types.rs:7-16`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecodePriority {
    Immediate = 0,  // Currently playing (underrun recovery)
    Next = 1,       // Next to play
    Prefetch = 2,   // Queued passages (prefetch)
}
```

**Priority Queue Behavior:**
- `BinaryHeap<DecodeRequest>` pops jobs in priority order (lower value = higher priority)
- Custom `Ord` implementation reverses ordering: `other.priority.cmp(&self.priority)`
- Result: Immediate jobs decode first, then Next, then Prefetch

### 2.3 Current Decode Flow

**Location:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/decoder_pool.rs:180-267`

```
Worker Thread Loop:
┌─────────────────────────────────────────────────────────────┐
│ 1. Lock queue mutex                                         │
│ 2. Wait on condvar while queue.is_empty() && !stop_flag    │
│ 3. Pop highest priority DecodeRequest                      │
│ 4. Release queue lock                                       │
│                                                             │
│ 5. Register buffer: buffer_manager.register_decoding()     │
│ 6. Decode passage:                                         │
│    a. SimpleDecoder::decode_passage() - decode full file   │
│    b. Resampler::resample() - convert to 44.1kHz          │
│    c. Convert to stereo (mono duplication / downmix)      │
│    d. Append samples in 1-second chunks:                  │
│       - buffer_handle.write().append_samples(chunk)       │
│       - buffer_manager.notify_samples_appended(...)       │
│       - buffer_manager.update_decode_progress(...)        │
│                                                             │
│ 7. Finalize: buffer_manager.finalize_buffer()             │
│ 8. Mark ready: buffer_manager.mark_ready()                │
│                                                             │
│ 9. On failure: buffer_manager.remove()                    │
└─────────────────────────────────────────────────────────────┘
```

**Key Observation:** Decode is currently atomic - once started, it runs to completion. There is no mechanism to pause mid-decode and switch to another job.

### 2.4 Buffer Fill Level Tracking

**Location:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/buffer_manager.rs:378-388`

```rust
pub async fn get_headroom(&self, queue_entry_id: Uuid) -> Result<usize, String> {
    let buffers = self.buffers.read().await;
    let managed = buffers.get(&queue_entry_id)
        .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;
    Ok(managed.metadata.headroom())
}

// BufferMetadata::headroom() calculates: write_position - read_position
```

**Buffer Capacity:**
- **[DBD-PARAM-070]** `playout_ringbuffer_size = 661,941 samples` (15.01s @ 44.1kHz stereo)
- **[DBD-PARAM-080]** `playout_ringbuffer_headroom = 441 samples` (0.01s @ 44.1kHz stereo)

**Current Exhaustion Detection:**
- `BUFFER_HEADROOM_THRESHOLD = 220,500 samples` (5s @ 44.1kHz stereo)
- BufferManager emits `BufferEvent::Exhausted` when headroom drops below threshold during playback

---

## 3. Pause/Resume Integration Points

### 3.1 Where to Check Buffer Fill Level

**Option A: Inside Worker Loop (Chosen)**
- **Location:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/decoder_pool.rs:385-420` (chunk append loop)
- **Frequency:** After each 1-second chunk is appended
- **Advantage:** Fine-grained control, minimal latency
- **Implementation:** Check `buffer_manager.get_headroom()` after each `notify_samples_appended()`

**Option B: Timer-Based Polling**
- **Frequency:** Every 5 seconds ([DBD-PARAM-060] decode_work_period)
- **Disadvantage:** Coarse-grained, may miss rapid buffer filling
- **Rejected:** Spec requires checking after append operations

### 3.2 Pause Trigger Logic

**[DBD-BUF-050] Pause Condition:**
```rust
let capacity = PLAYOUT_RINGBUFFER_SIZE; // 661,941 samples
let headroom_threshold = PLAYOUT_RINGBUFFER_HEADROOM; // 441 samples
let current_headroom = buffer_manager.get_headroom(passage_id).await?;

if current_headroom >= (capacity - headroom_threshold) {
    // Buffer nearly full, pause this decode job
    pause_current_job_and_switch();
}
```

**[DBD-BUF-050] Resume Condition (Hysteresis):**
```rust
// Resume when headroom drops below (capacity - headroom * 2)
// This prevents oscillation between pause/resume states
let resume_threshold = capacity - (headroom_threshold * 2); // 661,941 - 882 = 661,059

if current_headroom < resume_threshold {
    // Buffer has space, can resume paused jobs
}
```

### 3.3 State Management for Paused Jobs

**New Structure:**
```rust
/// Decode request with pause/resume state
#[derive(Debug, Clone)]
struct DecodeRequest {
    passage_id: Uuid,
    passage: PassageWithTiming,
    priority: DecodePriority,
    full_decode: bool,

    // NEW: Pause/resume state
    decode_state: DecodeState,
}

#[derive(Debug, Clone)]
enum DecodeState {
    /// Fresh job, not started
    NotStarted,

    /// Decode paused after N samples processed
    /// (samples_decoded, samples_appended)
    Paused {
        samples_decoded: usize,  // Total samples from decoder
        samples_appended: usize, // Total samples appended to buffer
    },
}
```

**Alternative (Simpler - Recommended):**
Reuse existing DecodeRequest, add paused job tracking in SharedPoolState:

```rust
struct SharedPoolState {
    queue: Mutex<BinaryHeap<DecodeRequest>>,
    condvar: Condvar,
    stop_flag: AtomicBool,

    // NEW: Track paused jobs separately
    // Key: passage_id, Value: (DecodeRequest, samples_processed)
    paused_jobs: Mutex<HashMap<Uuid, (DecodeRequest, usize)>>,
}
```

---

## 4. Pause/Resume State Machine

### 4.1 State Diagram

```
                    ┌──────────────┐
                    │   Queued     │ (DecodeRequest in BinaryHeap)
                    └──────┬───────┘
                           │ pop()
                           ▼
                    ┌──────────────┐
              ┌─────│   Decoding   │────┐
              │     └──────┬───────┘    │
              │            │            │
   Buffer     │            │ Complete   │ Error
   Nearly     │            │            │
   Full       │            ▼            ▼
              │     ┌──────────────┐   ┌──────────────┐
              └────▶│   Paused     │   │   Failed     │
                    └──────┬───────┘   └──────────────┘
                           │
                           │ Buffer has space
                           │ (Re-queue with priority)
                           ▼
                    ┌──────────────┐
                    │   Queued     │ (Back in BinaryHeap)
                    └──────────────┘
```

### 4.2 State Transitions

**Queued → Decoding:**
- **Trigger:** Worker pops DecodeRequest from queue
- **Action:** Call `decode_passage()`, start appending chunks

**Decoding → Paused:**
- **Trigger:** Buffer fill_level >= (capacity - headroom) after chunk append
- **Action:**
  1. Record current decode position (samples_processed)
  2. Move DecodeRequest to `paused_jobs` HashMap
  3. Worker returns to queue, pops next highest-priority job

**Paused → Queued:**
- **Trigger:** Buffer headroom < (capacity - headroom * 2)
- **Action:**
  1. Re-insert DecodeRequest into priority queue (preserves priority)
  2. Worker will pop it when it becomes highest priority again

**Decoding → Complete:**
- **Trigger:** All samples decoded and appended
- **Action:** Finalize buffer, mark ready, exit

**Decoding → Failed:**
- **Trigger:** Decode error (file read failure, corrupt data, etc.)
- **Action:** Remove buffer, log error, exit

---

## 5. Proposed Implementation Design

### 5.1 Code Changes by File

#### File: `wkmp-ap/src/playback/decoder_pool.rs`

**Change 1: Add paused job tracking to SharedPoolState (lines 67-77)**
```rust
struct SharedPoolState {
    queue: Mutex<BinaryHeap<DecodeRequest>>,
    condvar: Condvar,
    stop_flag: AtomicBool,

    // NEW: Track paused decode jobs
    // Key: passage_id, Value: (DecodeRequest, samples_decoded)
    paused_jobs: Mutex<HashMap<Uuid, (DecodeRequest, usize)>>,
}
```

**Change 2: Initialize paused_jobs in DecoderPool::new() (line 98)**
```rust
let state = Arc::new(SharedPoolState {
    queue: Mutex::new(BinaryHeap::new()),
    condvar: Condvar::new(),
    stop_flag: AtomicBool::new(false),
    paused_jobs: Mutex::new(HashMap::new()), // NEW
});
```

**Change 3: Check for paused job resume before popping queue (lines 190-207)**
```rust
// Pseudocode insertion point: BEFORE queue.pop()
loop {
    let request = {
        let mut queue = state.queue.lock().unwrap();

        // NEW: Check if any paused jobs can resume
        let resumable_job = check_paused_jobs_for_resume(
            &state.paused_jobs,
            &buffer_manager,
            &rt_handle
        );

        if let Some((request, resume_position)) = resumable_job {
            // Resume paused job instead of popping new one
            Some((request, resume_position))
        } else {
            // Wait for work or shutdown signal
            while queue.is_empty() && !state.stop_flag.load(Ordering::Relaxed) {
                queue = state.condvar.wait(queue).unwrap();
            }

            if state.stop_flag.load(Ordering::Relaxed) {
                break;
            }

            queue.pop().map(|req| (req, 0)) // Fresh job, start at 0
        }
    };

    if let Some((request, resume_from)) = request {
        // Decode (new or resumed)
        decode_passage_with_pause(&request, resume_from, ...);
    }
}
```

**Change 4: Modify decode_passage() to support pause/resume (lines 269-423)**

New signature:
```rust
fn decode_passage_with_pause(
    request: &DecodeRequest,
    resume_from_sample: usize, // 0 for fresh jobs, N for resumed
    buffer_handle: Arc<RwLock<PassageBuffer>>,
    buffer_manager: Arc<BufferManager>,
    rt_handle: &tokio::runtime::Handle,
    paused_jobs: Arc<Mutex<HashMap<Uuid, (DecodeRequest, usize)>>>,
) -> Result<DecodeStatus>

enum DecodeStatus {
    Completed,
    Paused { at_sample: usize },
    Failed(Error),
}
```

Pause check inside chunk append loop (after line 403):
```rust
for chunk_idx in 0..total_chunks {
    let start = chunk_idx * CHUNK_SIZE;
    let end = (start + CHUNK_SIZE).min(total_samples);
    let chunk = stereo_samples[start..end].to_vec();

    // Append chunk to buffer
    let chunk_len = chunk.len();
    rt_handle.block_on(async {
        let mut buffer = buffer_handle.write().await;
        buffer.append_samples(chunk);
    });

    rt_handle.block_on(async {
        if let Err(e) = buffer_manager.notify_samples_appended(passage_id, chunk_len).await {
            warn!("Failed to notify samples appended: {}", e);
        }
    });

    // NEW: Check if buffer is nearly full (pause condition)
    let should_pause = rt_handle.block_on(async {
        check_should_pause(&buffer_manager, passage_id).await
    });

    if should_pause {
        // Pause this job, record progress
        let samples_processed = end; // Current position in decode

        let mut paused = paused_jobs.lock().unwrap();
        paused.insert(passage_id, (request.clone(), samples_processed));

        debug!(
            "Pausing decode for {} at sample {} (buffer nearly full)",
            passage_id, samples_processed
        );

        return Ok(DecodeStatus::Paused { at_sample: samples_processed });
    }

    // Update progress
    let progress = ((chunk_idx + 1) * 100 / total_chunks).min(100) as u8;
    if progress % 10 == 0 || progress == 100 {
        rt_handle.block_on(async {
            buffer_manager.update_decode_progress(passage_id, progress).await;
        });
    }
}
```

**Change 5: Add helper functions (new, append after decode_passage)**

```rust
/// Check if buffer is nearly full (should pause decode)
/// [DBD-BUF-050] Pause when fill_level >= (capacity - headroom)
async fn check_should_pause(
    buffer_manager: &BufferManager,
    passage_id: Uuid,
) -> bool {
    const CAPACITY: usize = 661_941; // [DBD-PARAM-070]
    const HEADROOM: usize = 441;      // [DBD-PARAM-080]
    const PAUSE_THRESHOLD: usize = CAPACITY - HEADROOM;

    if let Ok(headroom) = buffer_manager.get_headroom(passage_id).await {
        headroom >= PAUSE_THRESHOLD
    } else {
        false
    }
}

/// Check if any paused jobs can resume (buffer has space)
/// [DBD-BUF-050] Resume when fill_level < (capacity - headroom * 2) [hysteresis]
fn check_paused_jobs_for_resume(
    paused_jobs: &Arc<Mutex<HashMap<Uuid, (DecodeRequest, usize)>>>,
    buffer_manager: &Arc<BufferManager>,
    rt_handle: &tokio::runtime::Handle,
) -> Option<(DecodeRequest, usize)> {
    const CAPACITY: usize = 661_941;
    const HEADROOM: usize = 441;
    const RESUME_THRESHOLD: usize = CAPACITY - (HEADROOM * 2);

    let paused = paused_jobs.lock().unwrap();

    // Find highest-priority paused job with buffer space
    let mut resumable: Option<(DecodeRequest, usize, DecodePriority)> = None;

    for (passage_id, (request, resume_pos)) in paused.iter() {
        let headroom = rt_handle.block_on(async {
            buffer_manager.get_headroom(*passage_id).await.unwrap_or(0)
        });

        if headroom < RESUME_THRESHOLD {
            // This job can resume - check if it's highest priority so far
            match resumable {
                None => {
                    resumable = Some((request.clone(), *resume_pos, request.priority));
                }
                Some((_, _, current_priority)) => {
                    if request.priority < current_priority {
                        resumable = Some((request.clone(), *resume_pos, request.priority));
                    }
                }
            }
        }
    }

    if let Some((request, resume_pos, _)) = resumable {
        // Remove from paused_jobs map
        drop(paused); // Release lock before removing
        let mut paused_mut = paused_jobs.lock().unwrap();
        paused_mut.remove(&request.passage_id);

        Some((request, resume_pos))
    } else {
        None
    }
}
```

### 5.2 Resume Implementation Detail

**Problem:** SimpleDecoder::decode_passage() decodes the entire passage in one call. How do we resume from sample N?

**Solution 1: Skip decoded samples (Recommended)**
```rust
// Inside decode_passage_with_pause(), after decoding:
let (samples, sample_rate, channels) =
    SimpleDecoder::decode_passage(&passage.file_path, start_ms, end_ms)?;

// If resuming, skip already-processed samples
let samples_to_process = if resume_from_sample > 0 {
    samples[resume_from_sample..].to_vec()
} else {
    samples
};

// Continue with resampling, conversion, chunking...
```

**Pros:** Simple, reuses existing decoder
**Cons:** Re-decodes samples we already processed (wasted CPU)

**Solution 2: Incremental decode (Future Enhancement)**
- Modify SimpleDecoder to support partial decode with resume
- More efficient but requires decoder refactor
- Out of scope for initial implementation

**Recommendation:** Use Solution 1 for MVP, document Solution 2 as future optimization.

---

## 6. Pseudocode for Pause/Resume Logic

### 6.1 Worker Loop with Pause/Resume

```rust
fn worker_loop(
    worker_id: usize,
    state: Arc<SharedPoolState>,
    buffer_manager: Arc<BufferManager>,
    rt_handle: tokio::runtime::Handle,
) {
    loop {
        // 1. Check for resumable paused jobs first
        let job = check_paused_jobs_for_resume(
            &state.paused_jobs,
            &buffer_manager,
            &rt_handle
        );

        let (request, resume_from) = if let Some((req, pos)) = job {
            // Resume paused job
            debug!("Worker {} resuming paused job {} from sample {}",
                   worker_id, req.passage_id, pos);
            (req, pos)
        } else {
            // Pop new job from queue
            let mut queue = state.queue.lock().unwrap();

            while queue.is_empty() && !state.stop_flag.load(Ordering::Relaxed) {
                queue = state.condvar.wait(queue).unwrap();
            }

            if state.stop_flag.load(Ordering::Relaxed) {
                break;
            }

            match queue.pop() {
                Some(req) => (req, 0), // Fresh job
                None => continue,
            }
        };

        // 2. Register/get buffer handle
        let buffer_handle = rt_handle.block_on(async {
            buffer_manager.register_decoding(request.passage_id).await
        });

        // 3. Decode with pause support
        match decode_passage_with_pause(
            &request,
            resume_from,
            buffer_handle,
            Arc::clone(&buffer_manager),
            &rt_handle,
            Arc::clone(&state.paused_jobs),
        ) {
            Ok(DecodeStatus::Completed) => {
                // Finalize buffer
                rt_handle.block_on(async {
                    buffer_manager.finalize_buffer(request.passage_id, total_samples).await;
                    buffer_manager.mark_ready(request.passage_id).await;
                });
            }

            Ok(DecodeStatus::Paused { at_sample }) => {
                // Job paused, already added to paused_jobs map
                debug!("Worker {} paused job {} at sample {}",
                       worker_id, request.passage_id, at_sample);
                // Continue to next job
            }

            Err(e) => {
                error!("Worker {} decode failed: {}", worker_id, e);
                rt_handle.block_on(async {
                    buffer_manager.remove(request.passage_id).await;
                });
            }
        }
    }
}
```

### 6.2 Decode with Pause Check

```rust
fn decode_passage_with_pause(
    request: &DecodeRequest,
    resume_from_sample: usize,
    buffer_handle: Arc<RwLock<PassageBuffer>>,
    buffer_manager: Arc<BufferManager>,
    rt_handle: &tokio::runtime::Handle,
    paused_jobs: Arc<Mutex<HashMap<Uuid, (DecodeRequest, usize)>>>,
) -> Result<DecodeStatus> {
    // 1. Decode entire passage (TODO: optimize to avoid re-decode on resume)
    let (samples, sample_rate, channels) =
        SimpleDecoder::decode_passage(&passage.file_path, start_ms, end_ms)?;

    // 2. Skip samples we already processed (if resuming)
    let samples_to_process = if resume_from_sample > 0 {
        samples[resume_from_sample..].to_vec()
    } else {
        samples
    };

    // 3. Resample to 44.1kHz
    let final_samples = if sample_rate != STANDARD_SAMPLE_RATE {
        Resampler::resample(&samples_to_process, sample_rate, channels)?
    } else {
        samples_to_process
    };

    // 4. Convert to stereo
    let stereo_samples = convert_to_stereo(final_samples, channels);

    // 5. Append samples in chunks WITH PAUSE CHECKS
    const CHUNK_SIZE: usize = 88200; // 1 second
    let total_samples = stereo_samples.len();
    let total_chunks = (total_samples + CHUNK_SIZE - 1) / CHUNK_SIZE;

    for chunk_idx in 0..total_chunks {
        let start = chunk_idx * CHUNK_SIZE;
        let end = (start + CHUNK_SIZE).min(total_samples);
        let chunk = stereo_samples[start..end].to_vec();

        // Append to buffer
        rt_handle.block_on(async {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(chunk);
        });

        rt_handle.block_on(async {
            buffer_manager.notify_samples_appended(passage_id, chunk.len()).await;
        });

        // CHECK FOR PAUSE
        let should_pause = rt_handle.block_on(async {
            check_should_pause(&buffer_manager, passage_id).await
        });

        if should_pause {
            let current_position = resume_from_sample + end;

            // Add to paused_jobs
            let mut paused = paused_jobs.lock().unwrap();
            paused.insert(passage_id, (request.clone(), current_position));

            return Ok(DecodeStatus::Paused { at_sample: current_position });
        }

        // Update progress
        update_progress(buffer_manager, passage_id, chunk_idx, total_chunks);
    }

    // All chunks appended successfully
    Ok(DecodeStatus::Completed)
}
```

---

## 7. Edge Cases to Handle

### 7.1 Job Completion During Pause

**Scenario:** Decoder pauses job A, switches to job B. Job A's buffer drains completely before job A resumes.

**Handling:**
```rust
// In check_paused_jobs_for_resume():
// Before returning paused job, verify buffer still exists
let buffer_exists = buffer_manager.is_managed(passage_id).await;

if !buffer_exists {
    // Buffer was removed (queue entry removed, user skipped, etc.)
    // Remove from paused_jobs and don't resume
    paused_jobs.lock().unwrap().remove(&passage_id);
    return None;
}
```

### 7.2 Multiple Paused Jobs with Same Priority

**Scenario:** Jobs A and B both have priority=Prefetch, both paused. Which resumes first?

**Handling:**
```rust
// In check_paused_jobs_for_resume():
// For same priority, use passage_id as tiebreaker (deterministic)
if request.priority == current_priority {
    if request.passage_id < current_passage_id {
        resumable = Some((request.clone(), *resume_pos, request.priority));
    }
}
```

### 7.3 Immediate Priority Job Arrives While Decoding

**Scenario:** Decoder working on Prefetch job, Immediate job (underrun recovery) arrives.

**Current Behavior:** Immediate job sits in queue until Prefetch completes.

**Proposed Enhancement (Out of Scope):**
- Add preemption: pause current job immediately when higher-priority job arrives
- Requires monitoring queue from within decode loop
- Document as "Future Enhancement: Preemptive Priority Switching"

### 7.4 Shutdown While Jobs Paused

**Scenario:** User stops playback, paused_jobs map has entries.

**Handling:**
```rust
// In DecoderPool::shutdown():
pub fn shutdown(self) -> Result<()> {
    // 1. Set stop flag
    self.state.stop_flag.store(true, Ordering::Relaxed);

    // 2. Notify all workers
    self.state.condvar.notify_all();

    // 3. Clear paused jobs (no longer needed)
    self.state.paused_jobs.lock().unwrap().clear();

    // 4. Join threads
    for handle in self.threads {
        handle.join()?;
    }

    Ok(())
}
```

### 7.5 Buffer Removed While Job Paused

**Scenario:** User removes queue entry, buffer_manager.remove() called, but job still in paused_jobs.

**Handling:**
- Already covered by 7.1 (verify buffer exists before resume)
- Alternative: Listen to buffer removal events and proactively clean paused_jobs

### 7.6 Rapid Pause/Resume Oscillation

**Scenario:** Buffer fill hovers at pause threshold, causing rapid pause/resume cycles.

**Mitigation:** Hysteresis thresholds
- **Pause:** fill_level >= (capacity - headroom) = 661,500 samples
- **Resume:** fill_level < (capacity - headroom * 2) = 661,059 samples
- **Gap:** 441 samples (0.01s) prevents oscillation

---

## 8. Testing Strategy

### 8.1 Unit Tests

**File:** `wkmp-ap/src/playback/decoder_pool.rs` (tests module)

1. **test_pause_resume_basic**
   - Enqueue 2 jobs (both Prefetch priority)
   - Mock buffer to trigger pause after 1 chunk
   - Verify first job pauses, second job starts
   - Mock buffer to allow resume
   - Verify first job resumes and completes

2. **test_pause_threshold_hysteresis**
   - Enqueue job, start decode
   - Set buffer fill to pause threshold - 1: verify no pause
   - Set buffer fill to pause threshold: verify pause
   - Set buffer fill to resume threshold + 1: verify no resume
   - Set buffer fill to resume threshold - 1: verify resume

3. **test_priority_preserved_on_resume**
   - Pause Immediate-priority job
   - Enqueue Next-priority job
   - Verify Immediate job resumes before Next job starts

4. **test_paused_job_cleanup_on_buffer_removal**
   - Pause job A
   - Remove buffer A (simulate queue entry removal)
   - Verify paused_jobs map clears entry
   - Verify no panic when checking for resume

5. **test_shutdown_with_paused_jobs**
   - Pause 3 jobs
   - Call shutdown()
   - Verify paused_jobs cleared
   - Verify threads join cleanly

### 8.2 Integration Tests

**File:** `wkmp-ap/tests/decoder_pool_pause_tests.rs` (new file)

1. **test_real_audio_pause_resume**
   - Use real MP3 test file
   - Configure small buffer capacity (mock override)
   - Enqueue file, verify decode pauses mid-stream
   - Drain buffer (simulate playback)
   - Verify decode resumes and completes

2. **test_multiple_paused_jobs_resume_order**
   - Enqueue 5 files (mixed priorities: Immediate, Next, Prefetch)
   - Pause all 5 by filling buffers
   - Drain buffers in random order
   - Verify resume order matches priority (Immediate first, then Next, then Prefetch)

3. **test_pause_during_crossfade**
   - Enqueue 2 passages (crossfade scenario)
   - Pause second passage during crossfade
   - Verify first passage completes
   - Verify second passage resumes and plays correctly

### 8.3 Manual Testing

1. **High-Load Scenario:**
   - Enqueue 20 passages (exceed maximum_decode_streams)
   - Enable slow playback (0.5x speed) to drain buffers slowly
   - Monitor logs for pause/resume events
   - Verify no decode failures or buffer underruns

2. **Priority Switching:**
   - Enqueue 5 Prefetch-priority passages
   - Start playback
   - Trigger immediate underrun recovery (manually skip to near-end of passage)
   - Verify Immediate-priority job preempts Prefetch jobs

3. **Memory Stability:**
   - Run overnight stress test (continuous playback, 1000+ passages)
   - Monitor memory usage (paused_jobs map should not leak)
   - Verify CPU usage stable (no decode thrashing)

---

## 9. Performance Considerations

### 9.1 Lock Contention

**Risk:** `paused_jobs` mutex locked frequently by workers checking for resume.

**Mitigation:**
- Resume check only happens when queue is empty
- Most of the time, paused_jobs will be empty (no contention)
- If contention becomes issue, switch to `DashMap` (concurrent hashmap)

### 9.2 Re-Decode Overhead

**Current Design:** Paused jobs re-decode entire file on resume, skip already-processed samples.

**Overhead Estimate:**
- Decode: ~10ms per second of audio (Raspberry Pi Zero 2W)
- Skip samples: ~1ms
- Total wasted: ~11ms per pause/resume cycle
- Impact: Negligible for typical 10-second pause duration

**Future Optimization:**
- Modify SimpleDecoder to support seek-and-resume
- Cache decoder state in paused_jobs map
- Reduces re-decode overhead to near-zero

### 9.3 Chunk Size Impact

**Current:** 1-second chunks (88,200 samples)

**Pause Latency:**
- Worst case: Job pauses immediately after chunk starts
- Time to detect: ~1 second (next chunk append triggers check)
- Impact: Acceptable for 15-second buffer

**Alternative:** Smaller chunks (0.5s or 0.25s)
- Reduces pause latency
- Increases buffer manager overhead (more notify_samples_appended calls)
- Recommendation: Keep 1s chunks for MVP, tune if needed

---

## 10. Implementation Phases

### Phase 1: Core Pause/Resume (MVP)
**Goal:** Basic pause when buffer full, resume when space available

**Changes:**
- Add `paused_jobs` HashMap to SharedPoolState
- Modify worker loop to check paused jobs before queue
- Add pause check in decode chunk loop
- Implement `check_should_pause()` and `check_paused_jobs_for_resume()`

**Acceptance Criteria:**
- Unit tests pass (8.1)
- Single paused job resumes correctly
- No memory leaks

**Effort:** 4-6 hours

### Phase 2: Priority Handling
**Goal:** Ensure priority preserved, handle edge cases

**Changes:**
- Priority-aware resume selection
- Buffer existence validation before resume
- Tiebreaker for same-priority jobs

**Acceptance Criteria:**
- Integration tests pass (8.2)
- Multiple paused jobs resume in correct order

**Effort:** 2-3 hours

### Phase 3: Robustness & Edge Cases
**Goal:** Handle shutdown, buffer removal, oscillation prevention

**Changes:**
- Shutdown cleanup of paused_jobs
- Hysteresis threshold tuning
- Event-based paused job cleanup (buffer removal listener)

**Acceptance Criteria:**
- All edge case tests pass (8.1.4, 8.1.5)
- Manual stress testing passes

**Effort:** 3-4 hours

### Phase 4: Performance Optimization (Future)
**Goal:** Reduce re-decode overhead, tune chunk size

**Changes:**
- Incremental decoder state preservation
- Configurable chunk size
- Concurrent hashmap for paused_jobs (if contention detected)

**Acceptance Criteria:**
- Re-decode overhead < 5ms
- Memory usage stable over 24hr test

**Effort:** 6-8 hours (deferred)

---

## 11. Open Questions for Review

1. **Hysteresis Gap:** Is 441-sample gap (0.01s) sufficient to prevent oscillation? Should we use larger gap (e.g., 5s = 220,500 samples)?

2. **Re-Decode Overhead:** Is re-decoding acceptable for MVP, or should we prioritize incremental decoder state preservation from the start?

3. **Preemption:** Should Phase 1 include immediate preemption (pause current job when higher-priority job arrives), or defer to Phase 4?

4. **Buffer Capacity:** Should pause threshold be based on absolute sample count or percentage of buffer fill? (Currently absolute: capacity - headroom)

5. **Event-Driven Cleanup:** Should paused_jobs listen to BufferEvent::Removed for proactive cleanup, or rely on lazy cleanup at resume-check time?

---

## 12. Glossary

- **Passage:** Continuous playable region within an audio file (start/end points, crossfade timing)
- **DecodeRequest:** Job structure containing passage metadata + priority
- **Priority Queue:** BinaryHeap that pops highest-priority jobs first
- **Headroom:** Available buffer space (write_position - read_position)
- **Hysteresis:** Dual thresholds (pause vs. resume) to prevent rapid oscillation
- **Incremental Decode:** Decoding audio in chunks, allowing pause/resume mid-stream

---

## 13. References

- **[DBD-DEC-020]**: Decoder assignment to passages within maximum_decode_streams
- **[DBD-DEC-030]**: Dedicated decoder per passage with pause on buffer full
- **[DBD-DEC-040]**: Serial decode with priority ordering
- **[DBD-BUF-050]**: Buffer pause when fill_level >= (capacity - headroom)
- **[DBD-PARAM-070]**: playout_ringbuffer_size = 661,941 samples (15.01s)
- **[DBD-PARAM-080]**: playout_ringbuffer_headroom = 441 samples (0.01s)
- **[SSD-DEC-030]**: Fixed 2-thread decoder pool
- **[SSD-DEC-032]**: Priority queue management

**Related Files:**
- `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/decoder_pool.rs`
- `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/buffer_manager.rs`
- `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/types.rs`
- `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/types.rs`
- `/home/sw/Dev/McRhythm/docs/SPEC016-decoder_buffer_design.md`

---

**Document Status:** Ready for technical review
**Next Steps:** Review with technical lead, approve design, begin Phase 1 implementation
**Estimated Implementation Time:** 9-13 hours (Phases 1-3), +6-8 hours (Phase 4 future optimization)
