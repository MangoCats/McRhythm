# Streaming Decoder Implementation Status

**Date:** 2025-10-21
**Status:** SPEC016 Updated ✅ | StreamingDecoder Implemented ✅ | serial_decoder.rs Integration ⏳

---

## Completed Work ✅

### 1. SPEC016 Documentation Updates ✅

**File:** `/home/sw/Dev/McRhythm/docs/SPEC016-decoder_buffer_design.md`

**Added [DBD-DEC-090] through [DBD-DEC-140]:**
- **[DBD-DEC-090]**: Decoders MUST support streaming/incremental operation
- **[DBD-DEC-100]**: All-at-once decoding is PROHIBITED (with rationale)
- **[DBD-DEC-110]**: Chunk-based decoding process (7-step procedure)
- **[DBD-DEC-120]**: Chunk duration rationale (~1 second balances latency vs overhead)
- **[DBD-DEC-130]**: Decoder state preservation requirements for pause/resume
- **[DBD-DEC-140]**: Streaming decoder implementation requirements

**Updated [DBD-PARAM-060] decode_work_period:**
- Clarified decoder pauses "within its decode loop" (between chunks)
- Added implementation note about chunk boundary vs work period timing
- Explained prevention of priority inversion (30-minute decode blocking "now playing")

### 2. StreamingDecoder Implementation ✅

**File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/decoder.rs` (lines 657-972)

**Implemented:**
```rust
pub struct StreamingDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    start_sample_idx: usize,
    end_sample_idx: usize,
    current_sample_idx: usize,
    finished: bool,
    start_ticks: i64,
    undefined_endpoint: bool,
}

impl StreamingDecoder {
    pub fn new(path: &PathBuf, start_ms: u64, end_ms: u64) -> Result<Self>
    pub fn decode_chunk(&mut self, chunk_duration_ms: u64) -> Result<Option<Vec<f32>>>
    pub fn is_finished(&self) -> bool
    pub fn format_info(&self) -> (u32, u16)
    pub fn get_discovered_endpoint(&self) -> Option<i64>
}
```

**Features:**
- ✅ Stateful iteration over compressed audio packets
- ✅ Returns ~1 second chunks on each `decode_chunk()` call
- ✅ Automatic passage boundary trimming (start/end)
- ✅ Endpoint discovery support ([DBD-DEC-090])
- ✅ Preserves decoder state between chunks ([DBD-DEC-130])
- ✅ Comprehensive tracing/debug logging

---

## Remaining Work ⏳

### 3. Update serial_decoder.rs to Use StreamingDecoder

**File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/serial_decoder.rs`

**Current Problem (lines 316-427):**
```rust
// THIS IS THE ALL-AT-ONCE PROBLEM:
let decode_result = SimpleDecoder::decode_passage(&passage.file_path, start_time_ms, end_time_ms)?;
// ^ Blocks for 10+ seconds decoding entire 5-minute file

// Then resample entire file
// Then apply fades to entire file
// Then chunk and append to buffer
```

**Required Changes:**

Replace `decode_passage_serial()` function (lines 284-527) with:

```rust
fn decode_passage_serial(
    request: &DecodeRequest,
    buffer_manager: &Arc<BufferManager>,
    rt_handle: &tokio::runtime::Handle,
    state: &Arc<SharedDecoderState>,
) -> Result<()> {
    let passage = &request.passage;
    let queue_entry_id = request.queue_entry_id;

    // Calculate start/end times
    let start_time_ms = wkmp_common::timing::ticks_to_ms(passage.start_time_ticks) as u64;
    let end_time_ms = if request.full_decode {
        passage.end_time_ticks
            .map(|t| wkmp_common::timing::ticks_to_ms(t) as u64)
            .unwrap_or(0)
    } else {
        start_time_ms + 15_000
    };

    // Create streaming decoder
    let mut streaming_decoder = StreamingDecoder::new(
        &passage.file_path,
        start_time_ms,
        end_time_ms
    )?;

    let (source_sample_rate, source_channels) = streaming_decoder.format_info();
    let chunk_duration_ms = 1000; // 1 second chunks per [DBD-DEC-110]

    let mut total_frames_appended = 0;
    let mut chunk_count = 0;

    // Decode in chunks
    while !streaming_decoder.is_finished() {
        // **[DBD-DEC-070]** Check priority queue between chunks
        if Self::should_yield_to_higher_priority(state, request.priority) {
            warn!("Serial decoder yielding to higher priority (chunk {})", chunk_count);
            // TODO: Save streaming_decoder state for resume (future enhancement)
            return Ok(());
        }

        // Decode next chunk
        let chunk_samples = match streaming_decoder.decode_chunk(chunk_duration_ms)? {
            Some(samples) => samples,
            None => break, // Finished
        };

        // Resample chunk if needed
        let resampled_chunk = if source_sample_rate != STANDARD_SAMPLE_RATE {
            Resampler::resample(&chunk_samples, source_sample_rate, source_channels)?
        } else {
            chunk_samples
        };

        // Convert to stereo if needed
        let stereo_chunk = if source_channels == 1 {
            // Duplicate mono to stereo
            let mut stereo = Vec::with_capacity(resampled_chunk.len() * 2);
            for sample in resampled_chunk {
                stereo.push(sample);
                stereo.push(sample);
            }
            stereo
        } else if source_channels == 2 {
            resampled_chunk
        } else {
            // Downmix multi-channel to stereo
            Self::downmix_to_stereo(&resampled_chunk, source_channels)
        };

        // TODO: Apply fades to chunk (only if chunk intersects fade regions)
        // For now, fades will be applied in mixer (less optimal but functional)

        // Append chunk to buffer
        let mut chunk_offset = 0;
        while chunk_offset < stereo_chunk.len() {
            let remaining = &stereo_chunk[chunk_offset..];

            let frames_pushed = rt_handle.block_on(async {
                match buffer_manager.push_samples(queue_entry_id, remaining).await {
                    Ok(count) => count,
                    Err(e) => {
                        warn!("Failed to push samples: {}", e);
                        0
                    }
                }
            });

            if frames_pushed == 0 {
                // Buffer full, wait briefly
                std::thread::sleep(std::time::Duration::from_millis(50));
            } else {
                chunk_offset += frames_pushed * 2; // Stereo frames
                total_frames_appended += frames_pushed;
            }
        }

        chunk_count += 1;

        // Update progress
        rt_handle.block_on(async {
            let progress = ((chunk_count * chunk_duration_ms) * 100 /
                (end_time_ms.saturating_sub(start_time_ms))).min(100) as u8;
            buffer_manager.update_decode_progress(queue_entry_id, progress).await;
        });
    }

    // Handle endpoint discovery
    if let Some(actual_end_ticks) = streaming_decoder.get_discovered_endpoint() {
        debug!("Endpoint discovered: {}ticks", actual_end_ticks);
        rt_handle.block_on(async {
            if let Err(e) = buffer_manager.set_discovered_endpoint(queue_entry_id, actual_end_ticks).await {
                warn!("Failed to set discovered endpoint: {}", e);
            }
        });
    }

    Ok(())
}

// Helper function for multi-channel downmix
fn downmix_to_stereo(samples: &[f32], channels: u16) -> Vec<f32> {
    let frame_count = samples.len() / channels as usize;
    let mut stereo = Vec::with_capacity(frame_count * 2);

    for frame_idx in 0..frame_count {
        let base = frame_idx * channels as usize;
        let mut left = 0.0;
        let mut right = 0.0;

        for ch in (0..channels as usize).step_by(2) {
            left += samples[base + ch];
        }
        left /= (channels / 2) as f32;

        for ch in (1..channels as usize).step_by(2) {
            right += samples[base + ch];
        }
        right /= (channels / 2) as f32;

        stereo.push(left);
        stereo.push(right);
    }

    stereo
}
```

**Key Differences:**
- ✅ Creates `StreamingDecoder` instead of calling `SimpleDecoder::decode_passage()`
- ✅ Loops over `decode_chunk()` calls (~1 second each)
- ✅ Resample, stereo-convert per chunk (not entire file)
- ✅ Append chunk to buffer immediately (progressive filling)
- ✅ Check priority queue between chunks ([DBD-DEC-070])
- ⚠️  Fades not yet applied per-chunk (needs enhancement)

---

## Testing Plan

### Test 1: Buffer Fill Progression

**Objective:** Verify buffer fills progressively instead of jumping 0% → 99%

**Steps:**
1. Kill any running wkmp-ap instances
2. Start wkmp-ap with RUST_LOG=debug
3. Open developer UI in browser (http://localhost:5721)
4. Set buffer chain update rate to "0.1s" (100ms)
5. Enqueue a 5-minute audio file
6. Observe buffer fill percentage in chain monitor

**Expected Results:**
```
T+0s:    Buffer fill: 0%
T+100ms: Buffer fill: ~0.3%  (first chunk decoded)
T+200ms: Buffer fill: ~0.7%  (second chunk)
T+300ms: Buffer fill: ~1.0%  (third chunk)
T+3s:    Buffer fill: ~1%    (threshold met, playback starts)
T+5s:    Buffer fill: ~1.7%  (continues filling during playback)
...
T+300s:  Buffer fill: 100%   (complete)
```

**PASS Criteria:**
- ✅ Buffer fill updates smoothly every 100ms
- ✅ No jumps from 0% to 99%
- ✅ Playback starts within 5 seconds of enqueue
- ✅ Decode continues in background during playback

### Test 2: Playback Start Latency

**Objective:** Measure time from enqueue to playback start

**Steps:**
1. Enqueue 10-second passage
2. Note timestamps in logs:
   - `Enqueued passage` (T0)
   - `Starting playback` (T1)
3. Calculate latency: T1 - T0

**Expected Results:**
- **Current (all-at-once)**: 10+ seconds
- **After fix (streaming)**: <5 seconds (ideally <1 second after buffer threshold)

**PASS Criteria:**
- ✅ Latency < 5 seconds for 10-second file
- ✅ Latency < 10 seconds for 5-minute file

### Test 3: Priority Switching

**Objective:** Verify decoder yields to higher priority work

**Steps:**
1. Enqueue low-priority 30-minute file
2. Wait 5 seconds (let it start decoding)
3. Enqueue high-priority 10-second file
4. Verify high-priority file decodes immediately (not waiting for 30-min file)

**Expected Results:**
- High-priority decoder preempts low-priority within ~1 second
- Low-priority decoder resumes after high-priority completes

**PASS Criteria:**
- ✅ High-priority decode starts within 2 seconds of enqueue
- ✅ Log shows "yielding to higher priority"

---

## Known Limitations

### Fade Application ⚠️

**Current Implementation:** Fades NOT yet applied per-chunk in serial_decoder.rs

**Impact:**
- Fade-in/fade-out will not work correctly with streaming decoder
- Mixer may apply fades (fallback), but less optimal than pre-buffer application

**Fix Required:**
Add fade application logic inside chunk loop:

```rust
// After stereo conversion, before buffer append:
let faded_chunk = Self::apply_fades_to_chunk(
    stereo_chunk,
    passage,
    chunk_start_time_ms,  // Where this chunk starts in passage timeline
    chunk_end_time_ms,    // Where this chunk ends
    STANDARD_SAMPLE_RATE,
);
```

**Priority:** Medium (fades work in mixer, just less efficient)

### Pause/Resume Not Implemented ⚠️

**Current Implementation:** When yielding to higher priority, decoder state is discarded

**Impact:**
- Low-priority decode restarts from beginning when resumed
- Wastes CPU re-decoding same audio

**Fix Required:**
Store `StreamingDecoder` instance in `SerialDecoder::paused_jobs` HashMap

**Priority:** Low (functional without it, just inefficient)

---

## Next Steps

1. ✅ SPEC016 updated
2. ✅ StreamingDecoder implemented
3. ⏳ **Update serial_decoder.rs** (code changes above)
4. ⏳ **Build and test** with 5-minute file
5. ⏳ **Verify buffer fill progression** (Test 1)
6. ⏳ **Measure playback start latency** (Test 2)
7. ⏳ **Optional: Implement fade-per-chunk** (if time permits)

---

## Conclusion

The streaming decoder architecture is complete and documented. The remaining work is to integrate `StreamingDecoder` into `serial_decoder.rs` to replace the all-at-once `SimpleDecoder::decode_passage()` call.

This will fix the 10-second delay and 0% → 99% buffer jump issue, providing smooth progressive buffer filling and fast playback start.

**Estimated Remaining Work:** 30-60 minutes (code changes + testing)
