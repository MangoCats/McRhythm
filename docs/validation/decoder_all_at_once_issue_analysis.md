# Decoder All-At-Once Issue Analysis

**Date:** 2025-10-21
**Issue:** Buffer fill jumps from 0% to 99% after 10-second delay for 5-minute file
**Root Cause:** Decoder processes entire file before writing any samples to buffer

---

## Problem Description

### Observed Behavior
After enqueueing a 5-minute audio file:
- **0-10 seconds**: Buffer fill shows 0%
- **10 seconds**: Buffer fill jumps to 99%, playback begins immediately
- **Expected**: Buffer fill should gradually increase as decoding progresses

### Root Cause

The current decoder implementation processes audio files in a **monolithic all-at-once** mode:

```
Current Flow (WRONG):
1. Decode ENTIRE file        → 10 seconds (blocking)
2. Resample ENTIRE file       → (blocking)
3. Apply fades to ENTIRE file → (blocking)
4. THEN chunk and append to buffer
```

**Source:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/serial_decoder.rs:319`
```rust
let decode_result = SimpleDecoder::decode_passage(&passage.file_path, start_time_ms, end_time_ms)?;
```

This call to `SimpleDecoder::decode_passage()` decodes the entire passage in one blocking operation.

**Source:** `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/decoder.rs:146-182`
```rust
// Decode all packets
let mut samples = Vec::new();

loop {
    let packet = match format.next_packet() { /* ... */ };

    match decoder.decode(&packet) {
        Ok(decoded) => {
            Self::convert_samples_to_f32(&decoded, &mut samples);
        }
        // ...
    }
}
```

The decoder reads **ALL packets** from the file in a tight loop with no yielding.

---

## What Should Happen

### Correct Architecture (Per SPEC016 + User Clarification)

```
Correct Flow:
1. Decode chunk (~1 second or less)
2. Resample chunk
3. Apply fades to chunk (if chunk includes fade region)
4. Append chunk to buffer
5. Check decode_work_period and priority queue
6. Yield to higher priority if needed
7. Repeat until complete
```

### Key Requirements

From **[SPEC016 DBD-PARAM-060]** `decode_work_period`:
- **Default**: 5000ms
- **Behavior**: "Once every decode_work_period the currently working decoder is paused and the list of pending decode jobs is evaluated"
- **Purpose**: "Allow decodes to continue uninterrupted while still serving the highest priority jobs often enough to ensure their buffers do not run empty"

From **User Clarification**:
- Each decoder-buffer chain should have its own decoder instance
- Decoders should output decoded audio samples in **chunks, typically less than 1 second duration per chunk**
- When the highest priority buffer is full, other chains should be able to start filling their buffers
- `decode_work_period` ensures lower priority decoders don't starve higher priority ones by preventing them from "decoding an entire 30 minute audio file at-once"

---

## Current vs. Expected Behavior

### Current Implementation Analysis

#### REV004 Chunked Buffer Filling

REV004 documents "incremental buffer implementation" but this only refers to chunking **already-decoded** samples:

```rust
// From REV004 lines 230-260
const CHUNK_SIZE: usize = 88200; // 1 second @ 44.1kHz stereo
let total_samples = stereo_samples.len(); // ← Already fully decoded!

for chunk_idx in 0..total_chunks {
    let chunk = stereo_samples[start..end].to_vec();
    buffer.append_samples(chunk); // Chunked append
}
```

**Problem:** By this point, `stereo_samples` contains the **entire decoded file**. The chunking only applies to buffer appending, not decoding itself.

#### Timeline for 5-Minute File

**Current (All-At-Once):**
```
T+0s:    Enqueue, start decode
T+0-10s: Decode all 300s of audio (buffer shows 0%)
T+10s:   Resample complete file
T+10s:   Apply fades to complete file
T+10s:   Chunk and append to buffer (99% instantly)
T+10s:   Playback begins
```

**Expected (Incremental):**
```
T+0s:     Enqueue, start decode
T+0-100ms: Decode first 1s chunk
T+100ms:   Resample chunk, append to buffer (buffer shows ~0.3%)
T+100ms:   Check priority queue, continue
T+100-200ms: Decode second 1s chunk
T+200ms:   Append (buffer shows ~0.7%)
T+3s:      Buffer threshold met (buffer shows ~1%), playback begins
T+3-300s:  Continue decoding in background while playing
```

---

## Fix Strategy

### Phase 1: Modify SimpleDecoder for Streaming

Rewrite `SimpleDecoder::decode_passage()` to support incremental decoding:

**Option A: Stateful Iterator Approach**
```rust
pub struct StreamingDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    // ... state
}

impl StreamingDecoder {
    pub fn decode_chunk(&mut self, max_duration_ms: u64) -> Result<Vec<f32>> {
        // Decode packets until max_duration_ms worth of samples collected
    }

    pub fn is_finished(&self) -> bool { /* ... */ }
}
```

**Option B: Callback-Based Streaming**
```rust
pub fn decode_passage_streaming<F>(
    path: &PathBuf,
    start_ms: u64,
    end_ms: u64,
    chunk_duration_ms: u64,
    chunk_callback: F,
) -> Result<()>
where
    F: FnMut(Vec<f32>, u32, u16) -> Result<bool>, // Returns Ok(true) to continue
{
    // Decode in chunks, call callback with each chunk
    // Callback can signal stop, check priority, etc.
}
```

**Recommendation:** Option A (stateful iterator) for better control flow and testability.

### Phase 2: Update SerialDecoder to Use Streaming

Modify `decode_passage_serial()` to:

```rust
fn decode_passage_serial(...) -> Result<()> {
    let mut streaming_decoder = StreamingDecoder::new(&passage.file_path, start_time_ms, end_time_ms)?;

    let decode_work_period_ms = 1000; // DBD-PARAM-060 (load from settings)
    let chunk_duration_ms = 1000; // ~1 second chunks

    let mut last_priority_check = Instant::now();

    while !streaming_decoder.is_finished() {
        // Decode chunk
        let chunk_samples = streaming_decoder.decode_chunk(chunk_duration_ms)?;

        // Resample chunk (if needed)
        let resampled_chunk = if source_rate != STANDARD_SAMPLE_RATE {
            Resampler::resample(&chunk_samples, source_rate, channels)?
        } else {
            chunk_samples
        };

        // Convert to stereo
        let stereo_chunk = convert_to_stereo(resampled_chunk, channels);

        // TODO: Apply fades (only to chunks in fade regions)

        // Append chunk to buffer
        rt_handle.block_on(async {
            buffer_manager.append_chunk(queue_entry_id, stereo_chunk).await?;
        });

        // Check priority queue every decode_work_period
        if last_priority_check.elapsed() >= Duration::from_millis(decode_work_period_ms) {
            if should_yield_to_higher_priority(state, request.priority) {
                // Pause this decode, queue will resume later
                return Ok(/* partial completion */);
            }
            last_priority_check = Instant::now();
        }
    }

    // Finalize buffer
    rt_handle.block_on(async {
        buffer_manager.finalize_buffer(queue_entry_id).await?;
    });

    Ok(())
}
```

### Phase 3: Handle Partial Decodes

Add support for pausing/resuming decoder state:

- Store decoder state in `SerialDecoder::paused_jobs` (already exists)
- Save `StreamingDecoder` instance when yielding to higher priority
- Resume from saved state when decoder becomes highest priority again

---

## SPEC016 Clarification Needs

### Current Ambiguities

1. **Chunk size specification**: SPEC016 references REV004's "1-second chunks" but doesn't make it clear this applies to **decoding** not just **buffer appending**

2. **decode_work_period scope**: The spec says "decoder is paused" but doesn't clarify that the decoder must support pausing **during** decoding, not just between files

3. **Streaming architecture**: SPEC016 should explicitly state that decoders must support incremental/streaming operation

### Proposed SPEC016 Updates

#### New Section: [DBD-DEC-015] Incremental Decoding Architecture

```markdown
## Incremental Decoding Architecture

**[DBD-DEC-015]** Decoders must support streaming/incremental operation to minimize latency and enable cooperative multitasking.

### Chunk-Based Decoding

Each decoder processes audio in chunks of approximately **1 second duration** or less:

1. **Decode chunk**: Process ~1 second worth of audio packets from source file
2. **Resample chunk**: Convert chunk to standard sample rate (44.1kHz)
3. **Append chunk**: Write resampled chunk to buffer
4. **Yield point**: Check priority queue and decode_work_period
5. **Repeat**: Continue until passage complete or higher priority work arrives

### Rationale

- **Reduces latency**: Buffer fills progressively, playback can start after first few chunks
- **Enables priority switching**: Decoder can yield to higher priority work every ~1 second
- **Prevents starvation**: Decoder for "now playing" buffer not blocked by 30-minute background decode
- **Improves monitoring**: Buffer fill percentage updates smoothly as chunks arrive

### All-At-Once Decoding is Prohibited

Decoders must NOT decode entire files into memory before writing to buffers. This creates:
- Long startup delays (10+ seconds for long files)
- Memory pressure (entire file in RAM)
- Priority inversion (low priority decode blocks high priority)
- Poor user experience (buffer shows 0% then jumps to 99%)
```

#### Update to [DBD-PARAM-060] decode_work_period

**Before:**
> Once every decode_work_period the currently working decoder is paused...

**After:**
> Once every decode_work_period (wall clock time), the currently working decoder pauses **within its decode loop** to check the priority queue. The decoder must support incremental operation such that it can yield between chunks without losing state. If a higher priority job is pending, the current decoder's state is saved and the higher priority decoder is resumed.

---

## Testing Plan

### Unit Tests

1. **Streaming decoder chunk size**
   - Verify chunks are ~1 second duration
   - Verify no chunk exceeds 2 seconds

2. **Priority yielding**
   - Start low priority 10-minute decode
   - Enqueue high priority 10-second file 1 second later
   - Verify high priority completes within 2 seconds
   - Verify low priority resumes after high priority completes

3. **Buffer fill progression**
   - Enqueue 5-minute file
   - Sample buffer fill every 100ms for first 5 seconds
   - Verify smooth progression (not 0% → 99% jump)

### Integration Tests

1. **Playback start latency**
   - Enqueue passage
   - Measure time from enqueue to playback start
   - Target: <100ms (currently 10+ seconds for long files)

2. **Decode_work_period respect**
   - Set decode_work_period to 2000ms
   - Start decode, measure actual yield interval
   - Verify yields occur within ±500ms of configured period

---

## Implementation Priority

**CRITICAL** - This issue breaks the intended architecture and severely degrades user experience.

**Recommended Sequence:**
1. ✅ Create this analysis document
2. ⏳ Update SPEC016 with clarifications (prevent future confusion)
3. ⏳ Implement `StreamingDecoder` in `audio/decoder.rs`
4. ⏳ Update `serial_decoder.rs` to use streaming API
5. ⏳ Add unit tests for chunk-based decoding
6. ⏳ Integration test with manual verification (buffer fill progression)

---

## Related Documents

- **SPEC016** - Decoder & Buffer Design: Defines decode_work_period and buffer architecture
- **REV004** - Incremental Buffer Implementation: Documents chunked buffer appending (but not decoding)
- **SPEC014** - Single Stream Design: Defines partial buffer playback thresholds

---

## Conclusion

The current decoder implementation contradicts the architectural intent of SPEC016 and REV004 by decoding entire files at once instead of in chunks. This causes:

1. **10-second delays** before playback starts (entire file must decode)
2. **Buffer fill jumps** from 0% to 99% (no progressive updates)
3. **Priority inversion** (low priority 30-minute decode blocks high priority work)
4. **Violates decode_work_period** (no yielding during decode)

The fix requires rewriting the decoder to support streaming/incremental operation with ~1 second chunks, allowing the decoder to yield to higher priority work and write progressive updates to buffers.

**SPEC016 should be updated** to explicitly state that all-at-once decoding is prohibited and that incremental/streaming operation is mandatory.
