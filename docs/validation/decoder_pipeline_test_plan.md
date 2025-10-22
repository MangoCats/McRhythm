# Decoder Pipeline Unit Test Strategy

**Document Type:** Test Coverage Analysis
**Target Coverage:** >70% line coverage, >60% branch coverage, 100% critical path coverage
**Component:** Serial Decoder Pipeline (wkmp-ap)
**Date:** 2025-10-21

---

## Executive Summary

This document defines a comprehensive unit testing strategy for the new single-threaded decoder pipeline to achieve >70% code coverage and validate all critical functionality specified in SPEC016.

**Current Coverage Status:**
- **Existing Unit Tests:** 12 tests in `serial_decoder.rs` (inline module tests)
- **Existing Integration Tests:** 13 tests in `tests/serial_decoder_tests.rs`
- **Coverage Focus:** Priority queue ordering, yield logic, fade calculations
- **Gaps:** Pipeline flow, resampler state, buffer interaction, edge cases

---

## Components Under Test

### 1. Worker Thread Priority Queue Management
**Location:** `serial_decoder.rs` lines 208-297 (`worker_loop`)

**What It Does:**
- Waits for decode requests via condvar
- Pops highest priority request from BinaryHeap
- Processes requests serially (one at a time)
- Handles shutdown signal

**Coverage Status:**
- ✅ Priority ordering tested (unit tests)
- ✅ Queue submission tested (integration tests)
- ❌ **GAP:** Condvar wake-up behavior not tested
- ❌ **GAP:** Shutdown signal handling not tested
- ❌ **GAP:** Empty queue waiting not tested

---

### 2. Decoder → Resampler → Fader → Buffer Pipeline
**Location:** `serial_decoder.rs` lines 299-601 (`decode_passage_serial`)

**What It Does:**
- Creates/resumes StreamingDecoder
- Decodes chunks (~1 second duration)
- Resamples to 44.1kHz if needed
- Converts to stereo (mono duplication, multi-channel downmix)
- Appends chunks to buffer incrementally
- Updates progress every 5 chunks

**Coverage Status:**
- ✅ Fade application tested (unit tests with mock data)
- ❌ **GAP:** Decode → resample → append flow not tested
- ❌ **GAP:** Sample rate conversion (48kHz → 44.1kHz) not tested
- ❌ **GAP:** Mono → stereo conversion not tested
- ❌ **GAP:** Multi-channel downmix not tested
- ❌ **GAP:** Progress updates not tested
- ❌ **GAP:** Buffer push retry logic not tested

---

### 3. Yield Decision Logic
**Location:** `serial_decoder.rs` lines 396-435

**What It Does:**
- **Priority-based yield:** Check if higher priority work pending
- **Time-based yield:** Check decode_work_period elapsed
- **Buffer-full yield:** Check if buffer can't accept more samples

**Coverage Status:**
- ✅ Priority-based yield tested (`should_yield_to_higher_priority` tests)
- ✅ Time-based yield tested (`has_pending_requests` tests)
- ❌ **GAP:** Buffer-full yield not tested
- ❌ **GAP:** Yield timing (5000ms default) not tested
- ❌ **GAP:** Interaction between yield conditions not tested

---

### 4. Decoder State Preservation Across Yields
**Location:** `serial_decoder.rs` lines 673-697 (`pause_decoder`, `PausedDecoderState`)

**What It Does:**
- Saves StreamingDecoder instance (preserves decode position)
- Saves chunk_count (resume point)
- Saves total_frames_appended (progress tracking)
- Saves last_yield_check timestamp

**Coverage Status:**
- ❌ **GAP:** State saving not tested
- ❌ **GAP:** State restoration not tested
- ❌ **GAP:** Multiple pause/resume cycles not tested
- ❌ **GAP:** Decoder position continuity not tested

---

### 5. Resampler State Continuity Across Batches
**Location:** `resampler.rs` (stateless, but chunk boundaries matter)

**What It Does:**
- Converts input sample rate to 44.1kHz
- Uses FastFixedIn (rubato) for efficiency
- De-interleaves samples for processing
- Re-interleaves output

**Coverage Status:**
- ✅ Basic resampling tested (`resampler.rs` unit tests)
- ❌ **GAP:** Chunk boundary resampling not tested
- ❌ **GAP:** Resampler state across multiple decode chunks not tested
- ❌ **GAP:** Edge cases (exactly 44.1kHz input) tested but not for chunking

---

### 6. Fader Curve Application at Chunk Boundaries
**Location:** `serial_decoder.rs` lines 699-794 (`apply_fades_to_samples`)

**What It Does:**
- Applies fade-in from passage start to fade_in_point
- Applies fade-out from fade_out_point to passage end
- Uses discovered endpoint when passage end is undefined
- Sample-accurate positioning using tick-based timing

**Coverage Status:**
- ✅ Fade calculations tested (linear fade-in/out)
- ✅ Discovered endpoint tested
- ❌ **GAP:** Fade application across chunk boundaries not tested
- ❌ **GAP:** Chunk intersecting fade region not tested
- ❌ **GAP:** All 5 fade curves (Linear, Exponential, Logarithmic, SCurve, Cosine) not tested in chunks

---

### 7. Buffer Full/Empty Edge Cases
**Location:** `serial_decoder.rs` lines 480-540 (buffer push logic)

**What It Does:**
- Pushes chunks to buffer via `buffer_manager.push_samples()`
- Handles buffer full (frames_pushed == 0)
- Waits 50ms for mixer to drain when buffer full
- Yields to other decoders when buffer remains full

**Coverage Status:**
- ❌ **GAP:** Buffer full condition not tested
- ❌ **GAP:** Partial push (buffer almost full) not tested
- ❌ **GAP:** 50ms wait for mixer drain not tested
- ❌ **GAP:** Decoder resume after buffer space available not tested

---

## Test Plan

### Test File Organization

```
tests/
  unit/
    serial_decoder/
      priority_queue_tests.rs          # Priority queue management
      pipeline_flow_tests.rs            # Decode → Resample → Fade → Buffer
      yield_logic_tests.rs              # All 3 yield conditions
      state_preservation_tests.rs       # Pause/resume decoder state
      chunk_boundary_tests.rs           # Resampler/fader at chunk edges
      buffer_interaction_tests.rs       # Buffer full/empty handling
      edge_cases_tests.rs               # Special cases and error conditions

  integration/
    serial_decoder/
      real_audio_decode_tests.rs        # Full pipeline with real audio files
      multi_passage_tests.rs            # Multiple passages with priority switching
      underrun_recovery_tests.rs        # Immediate priority during underrun
```

---

## Unit Test Specifications

### 1. Priority Queue Tests (`priority_queue_tests.rs`)

**Objective:** Validate BinaryHeap ordering and request submission

#### Test Cases:

```rust
// ✅ EXISTING: test_decode_request_priority_ordering
// Verifies: Immediate > Next > Prefetch ordering

// ✅ EXISTING: test_priority_queue_ordering (integration)
// Verifies: Submit in reverse order, queue maintains priority

#[tokio::test]
async fn test_condvar_wakeup_on_submit() {
    // NEW TEST
    // Given: Empty queue, worker waiting on condvar
    // When: Submit decode request
    // Then: Worker wakes up within 100ms
}

#[tokio::test]
async fn test_shutdown_signal_interrupts_wait() {
    // NEW TEST
    // Given: Empty queue, worker waiting on condvar
    // When: Set stop_flag and notify condvar
    // Then: Worker exits loop within 500ms
}

#[test]
fn test_empty_queue_waiting_state() {
    // NEW TEST
    // Given: Empty queue
    // When: Worker calls queue.pop()
    // Then: Returns None (no panic)
}

#[tokio::test]
async fn test_multiple_requests_same_priority() {
    // NEW TEST
    // Given: 3 requests with same priority (Prefetch)
    // When: Pop all requests
    // Then: Order is arbitrary but deterministic (FIFO-like within priority)
}
```

---

### 2. Pipeline Flow Tests (`pipeline_flow_tests.rs`)

**Objective:** Validate Decode → Resample → Stereo-Convert → Append flow

#### Test Cases:

```rust
#[tokio::test]
async fn test_decode_single_chunk_no_resample() {
    // NEW TEST
    // Given: Mock audio file @ 44.1kHz mono
    // When: Decode one chunk
    // Then:
    //   - Chunk decoded (verify sample count)
    //   - No resampling (output == input * 2 for stereo)
    //   - Samples appended to buffer
    //   - Buffer occupied increases
}

#[tokio::test]
async fn test_decode_single_chunk_with_resample_48khz() {
    // NEW TEST
    // Given: Mock audio file @ 48kHz stereo
    // When: Decode one chunk (1 second = 48000 samples)
    // Then:
    //   - Resampler called with 48000 samples
    //   - Output ~44100 samples (44.1kHz)
    //   - Samples appended to buffer
    //   - Sample count ratio ~0.91875
}

#[test]
fn test_mono_to_stereo_conversion() {
    // NEW TEST (pure function test)
    // Given: Mono samples [0.1, 0.2, 0.3]
    // When: Convert to stereo
    // Then: [0.1, 0.1, 0.2, 0.2, 0.3, 0.3]
}

#[test]
fn test_downmix_5_1_to_stereo() {
    // NEW TEST (test downmix_to_stereo function)
    // Given: 6-channel audio (5.1 surround)
    // When: Downmix to stereo
    // Then: Verify averaging algorithm
    //   - Left = average of channels 0, 2, 4
    //   - Right = average of channels 1, 3, 5
}

#[tokio::test]
async fn test_progress_updates_every_5_chunks() {
    // NEW TEST
    // Given: Mock audio file (10 seconds)
    // When: Decode 15 chunks
    // Then:
    //   - Progress updated at chunks 5, 10, 15
    //   - Progress values: 50%, 100%, 100%
    //   - buffer_manager.update_decode_progress called 3 times
}

#[tokio::test]
async fn test_buffer_push_partial() {
    // NEW TEST
    // Given: Buffer with space for 1000 samples, chunk has 2000 samples
    // When: Push chunk
    // Then:
    //   - First push returns 1000
    //   - chunk_offset advances to 2000
    //   - Second push attempts remaining 1000
    //   - total_frames_appended == 1000 after first push
}
```

---

### 3. Yield Logic Tests (`yield_logic_tests.rs`)

**Objective:** Test all 3 yield conditions and re-queue behavior

#### Test Cases:

```rust
// ✅ EXISTING: test_should_yield_to_higher_priority_* (6 tests)
// Covers priority-based yielding

#[tokio::test]
async fn test_time_based_yield_after_decode_work_period() {
    // NEW TEST
    // Given: Decoder processing Prefetch request
    // When: 5 seconds elapse (decode_work_period)
    // Then:
    //   - Decoder yields (saves state)
    //   - Request re-queued with same priority
    //   - last_yield_check updated
}

#[tokio::test]
async fn test_buffer_full_yield() {
    // NEW TEST
    // Given: Buffer full (push_samples returns 0)
    // When: Decoder attempts to append chunk
    // Then:
    //   - Decoder waits 50ms for mixer to drain
    //   - Checks can_decoder_resume()
    //   - If still full, yields and re-queues
}

#[tokio::test]
async fn test_buffer_full_but_resume_threshold_met() {
    // NEW TEST
    // Given: Buffer temporarily full, mixer drains during 50ms wait
    // When: Decoder checks can_decoder_resume()
    // Then:
    //   - Returns true
    //   - Decoder continues without yielding
}

#[tokio::test]
async fn test_no_yield_when_no_pending_work() {
    // NEW TEST
    // Given: Decoder processing, decode_work_period elapsed
    // When: Check has_pending_requests()
    // Then:
    //   - Returns false (queue empty)
    //   - Decoder does NOT yield
    //   - last_yield_check updated
}

#[tokio::test]
async fn test_yield_priority_precedence() {
    // NEW TEST
    // Given: All 3 yield conditions met simultaneously
    // When: Decoder checks yield conditions
    // Then:
    //   - Priority check happens first (highest precedence)
    //   - Yields to higher priority work
    //   - Time-based/buffer-full checks skipped
}
```

---

### 4. State Preservation Tests (`state_preservation_tests.rs`)

**Objective:** Validate decoder pause/resume across yields

#### Test Cases:

```rust
#[tokio::test]
async fn test_pause_decoder_saves_state() {
    // NEW TEST
    // Given: Decoder mid-decode (chunk_count=5, frames=220500)
    // When: pause_decoder() called
    // Then:
    //   - PausedDecoderState inserted into HashMap
    //   - Contains: decoder, chunk_count, total_frames_appended, last_yield_check
}

#[tokio::test]
async fn test_resume_decoder_restores_state() {
    // NEW TEST
    // Given: Paused decoder state exists for queue_entry_id
    // When: decode_passage_serial() called with same queue_entry_id
    // Then:
    //   - Paused state retrieved from HashMap
    //   - Decoder instance reused (not recreated)
    //   - chunk_count continues from saved value
    //   - total_frames_appended continues from saved value
}

#[tokio::test]
async fn test_multiple_pause_resume_cycles() {
    // NEW TEST
    // Given: Decoder paused and resumed 3 times
    // When: Final decode completes
    // Then:
    //   - Total frames == expected for full passage
    //   - No duplicate samples
    //   - chunk_count increments correctly across resumes
}

#[tokio::test]
async fn test_decoder_position_continuity() {
    // NEW TEST (requires real audio file or mock StreamingDecoder)
    // Given: Decode 5 chunks, pause, resume, decode 5 more chunks
    // When: Compare samples before/after pause
    // Then:
    //   - No gap in samples (continuous audio)
    //   - No overlap (no duplicate samples)
    //   - Sample values match expected progression
}
```

---

### 5. Chunk Boundary Tests (`chunk_boundary_tests.rs`)

**Objective:** Test resampler/fader behavior at chunk edges

#### Test Cases:

```rust
#[test]
fn test_resampler_chunk_boundary_no_state() {
    // NEW TEST
    // Given: Two consecutive 1-second chunks @ 48kHz
    // When: Resample each chunk independently
    // Then:
    //   - Output sample counts match expected ratio
    //   - No samples lost at boundary
    //   - Total output == sum of chunk outputs
}

#[test]
fn test_fade_in_across_chunk_boundary() {
    // NEW TEST
    // Given: Passage with 8-second fade-in, chunk size = 1 second
    // When: Apply fade to chunks 0-7
    // Then:
    //   - Chunk 0: samples ~0.0 at start
    //   - Chunk 7: samples ~1.0 at end
    //   - No discontinuity at chunk boundaries
    //   - Fade multiplier progresses smoothly
}

#[test]
fn test_fade_out_across_chunk_boundary() {
    // NEW TEST
    // Given: Passage with fade-out starting at chunk 12
    // When: Apply fade to chunks 12-19 (8 seconds)
    // Then:
    //   - Chunk 12: samples ~1.0 at start
    //   - Chunk 19: samples ~0.0 at end
    //   - No discontinuity at boundaries
}

#[test]
fn test_chunk_partially_in_fade_region() {
    // NEW TEST
    // Given: Chunk intersects fade-in point (starts before, ends after)
    // When: Apply fade
    // Then:
    //   - Samples before fade-in point: multiplied by <1.0
    //   - Samples after fade-in point: multiplied by 1.0
    //   - Transition is sample-accurate
}

#[test]
fn test_all_5_fade_curves_in_chunks() {
    // NEW TEST
    // For each curve: Linear, Exponential, Logarithmic, SCurve, Cosine
    // Given: 4-second fade divided into 4 chunks
    // When: Apply fade curve
    // Then:
    //   - Verify curve characteristics:
    //     - Linear: steady progression
    //     - Exponential: slow start, fast end
    //     - Logarithmic: fast start, slow end
    //     - SCurve: slow-fast-slow
    //     - Cosine: smooth acceleration
}
```

---

### 6. Buffer Interaction Tests (`buffer_interaction_tests.rs`)

**Objective:** Test buffer manager integration and backpressure

#### Test Cases:

```rust
#[tokio::test]
async fn test_buffer_full_condition() {
    // NEW TEST
    // Given: Buffer capacity = 661941 samples, occupied = 661941
    // When: Decoder calls push_samples(chunk)
    // Then:
    //   - push_samples returns 0
    //   - Decoder enters buffer-full yield path
}

#[tokio::test]
async fn test_buffer_almost_full_partial_push() {
    // NEW TEST
    // Given: Buffer free space = 5000 samples, chunk = 44100 samples
    // When: push_samples(chunk)
    // Then:
    //   - Returns 5000 (partial push)
    //   - Decoder updates chunk_offset to 10000 (5000 frames * 2)
    //   - Next iteration attempts remaining 39100 samples
}

#[tokio::test]
async fn test_buffer_empty_accepts_full_chunk() {
    // NEW TEST
    // Given: Buffer empty (occupied = 0)
    // When: push_samples(44100 sample chunk)
    // Then:
    //   - Returns 44100
    //   - Buffer occupied == 88200 (stereo samples)
}

#[tokio::test]
async fn test_decoder_resume_after_mixer_drain() {
    // NEW TEST
    // Given: Buffer full, decoder yielded
    // When: Mixer consumes 100,000 samples
    // Then:
    //   - can_decoder_resume() returns true
    //   - Decoder resumes decoding
    //   - Continues from saved chunk_count
}

#[tokio::test]
async fn test_buffer_manager_register_prevents_duplicate() {
    // NEW TEST (validates existing fix)
    // Given: Empty decoder queue
    // When: submit() called twice with same queue_entry_id
    // Then:
    //   - First submit: registers buffer, queues request
    //   - Second submit: buffer already registered (is_managed == true)
    //   - Only one decode request in queue
}
```

---

### 7. Edge Cases Tests (`edge_cases_tests.rs`)

**Objective:** Cover special conditions and error handling

#### Test Cases:

```rust
#[tokio::test]
async fn test_last_chunk_less_than_full_size() {
    // NEW TEST
    // Given: Passage total = 95000 samples, chunk size = 44100
    // When: Decode chunks
    // Then:
    //   - Chunk 0: 44100 samples
    //   - Chunk 1: 44100 samples
    //   - Chunk 2: 6800 samples (remainder)
    //   - Total == 95000
}

#[tokio::test]
async fn test_passage_shorter_than_one_chunk() {
    // NEW TEST
    // Given: Passage duration = 0.5 seconds (22050 samples @ 44.1kHz)
    // When: Decode passage
    // Then:
    //   - One chunk decoded with 22050 samples
    //   - chunk_count == 1
    //   - Progress == 100%
}

#[tokio::test]
async fn test_buffer_exactly_full_no_partial() {
    // NEW TEST
    // Given: Buffer free space = 44100 samples, chunk = 44100 samples
    // When: push_samples(chunk)
    // Then:
    //   - Returns 44100 (exact fit)
    //   - No partial push, no yield
    //   - Buffer now full
}

#[test]
fn test_zero_length_fade_in_region() {
    // NEW TEST
    // Given: Passage with fade_in_point == start_time
    // When: Apply fades
    // Then:
    //   - No fade-in applied (fade_in_duration == 0)
    //   - All samples at 1.0 multiplier
}

#[test]
fn test_zero_length_fade_out_region() {
    // NEW TEST
    // Given: Passage with fade_out_point == end_time
    // When: Apply fades
    // Then:
    //   - No fade-out applied (fade_out_duration == 0)
    //   - All samples at 1.0 multiplier until last sample
}

#[tokio::test]
async fn test_decode_error_cleanup() {
    // NEW TEST
    // Given: Invalid audio file path
    // When: decode_passage_serial() fails
    // Then:
    //   - Error returned
    //   - Buffer removed from buffer_manager
    //   - Paused decoder state cleaned up
}

#[tokio::test]
async fn test_shutdown_with_paused_decoders() {
    // NEW TEST
    // Given: 3 paused decoders in HashMap
    // When: shutdown() called
    // Then:
    //   - Worker thread exits cleanly
    //   - Paused decoders not resumed
    //   - No resource leaks
}

#[test]
fn test_downmix_odd_channel_count() {
    // NEW TEST
    // Given: 5-channel audio (rare but possible)
    // When: downmix_to_stereo()
    // Then:
    //   - Left: average of channels 0, 2, 4 (3 channels)
    //   - Right: average of channels 1, 3 (2 channels)
    //   - No panic, no divide by zero
}

#[tokio::test]
async fn test_endpoint_discovery_for_undefined_passage() {
    // NEW TEST
    // Given: Passage with end_time_ticks == None
    // When: Decode to EOF
    // Then:
    //   - actual_end_ticks discovered
    //   - buffer_manager.set_discovered_endpoint() called
    //   - Fade-out uses discovered endpoint (not 10s fallback)
}

#[tokio::test]
async fn test_partial_decode_15_seconds() {
    // NEW TEST
    // Given: 60-second passage, full_decode = false
    // When: Decode
    // Then:
    //   - Decodes only 15 seconds (660 chunks @ 1s each)
    //   - end_time_ms = start_time_ms + 15000
    //   - Buffer contains ~661500 samples (15s @ 44.1kHz stereo)
}
```

---

## Integration Test Specifications

### 1. Real Audio Decode Tests (`real_audio_decode_tests.rs`)

**Objective:** Full pipeline with real audio files (requires test assets)

#### Test Cases:

```rust
#[tokio::test]
async fn test_decode_mp3_file_full_pipeline() {
    // Given: Real MP3 file (test_assets/test_44100_stereo.mp3)
    // When: Decode full file
    // Then:
    //   - All chunks processed
    //   - Buffer contains expected sample count
    //   - Audio data non-silent
}

#[tokio::test]
async fn test_decode_flac_file_with_resample() {
    // Given: Real FLAC file @ 48kHz
    // When: Decode full file
    // Then:
    //   - Resampler invoked
    //   - Output @ 44.1kHz
    //   - Sample count ratio ~0.91875
}

#[tokio::test]
async fn test_decode_with_fade_in_out() {
    // Given: Real audio file with 2s fade-in, 2s fade-out
    // When: Decode with fades enabled
    // Then:
    //   - First samples near silent
    //   - Last samples near silent
    //   - Middle samples at full volume
}
```

---

### 2. Multi-Passage Tests (`multi_passage_tests.rs`)

**Objective:** Priority switching with multiple passages

#### Test Cases:

```rust
#[tokio::test]
async fn test_prefetch_yields_to_next() {
    // Given: Prefetch passage decoding (30s file)
    // When: Next passage submitted
    // Then:
    //   - Prefetch yields within 5 seconds
    //   - Next passage starts decoding
    //   - Prefetch resumes after Next completes
}

#[tokio::test]
async fn test_immediate_preempts_all() {
    // Given: Next and Prefetch passages decoding
    // When: Immediate passage submitted (underrun recovery)
    // Then:
    //   - Current decoder yields immediately
    //   - Immediate passage decodes
    //   - Other passages resume after
}
```

---

## Mock/Fixture Requirements

### 1. Mock StreamingDecoder
**Purpose:** Test decoder state without real files

```rust
struct MockStreamingDecoder {
    chunk_data: Vec<Vec<f32>>,  // Pre-generated chunks
    current_chunk: usize,
    finished: bool,
}

impl MockStreamingDecoder {
    fn new(num_chunks: usize, chunk_size: usize) -> Self;
    fn decode_chunk(&mut self) -> Option<Vec<f32>>;
    fn is_finished(&self) -> bool;
}
```

### 2. Mock BufferManager
**Purpose:** Test buffer interaction without real buffer allocation

```rust
struct MockBufferManager {
    buffers: HashMap<Uuid, MockBuffer>,
    push_results: VecDeque<usize>,  // Pre-programmed push return values
}

impl MockBufferManager {
    fn set_push_result(&mut self, result: usize);
    async fn push_samples(&self, id: Uuid, samples: &[f32]) -> Result<usize>;
}
```

### 3. Test Audio Assets
**Location:** `tests/test_assets/`

**Required Files:**
- `test_44100_stereo.mp3` (10s, 44.1kHz stereo) - No resampling needed
- `test_48000_stereo.flac` (10s, 48kHz stereo) - Resampling test
- `test_44100_mono.wav` (5s, 44.1kHz mono) - Mono → stereo conversion
- `test_88200_stereo.flac` (5s, 88.2kHz stereo) - Downsample test
- `test_short.mp3` (0.5s, 44.1kHz stereo) - Shorter than one chunk

---

## Coverage Measurement Strategy

### 1. Tool: cargo-llvm-cov

```bash
# Install
cargo install cargo-llvm-cov

# Generate HTML report
cargo llvm-cov --html --lib --tests

# View report
open target/llvm-cov/html/index.html
```

### 2. Coverage Targets

**Critical Path Coverage: 100%**
- Priority queue pop and push
- Decode chunk loop
- Yield decision logic (all 3 conditions)
- Pause/resume decoder state

**Line Coverage: >70%**
- All major functions in serial_decoder.rs
- Pipeline flow (decode → resample → append)
- Fade application

**Branch Coverage: >60%**
- All if/else in yield logic
- State machine transitions (Empty → Filling → Ready)
- Error handling paths

### 3. Exclusions

**Explicitly exclude from coverage:**
- Debug logging statements
- Tracing macros
- Documentation examples in comments

---

## Test Execution Order

### Phase 1: Pure Function Tests (No I/O)
**Duration: ~1 second**
- `test_downmix_to_stereo`
- `test_mono_to_stereo_conversion`
- `test_fade_in_across_chunk_boundary`
- All fade curve tests

### Phase 2: Unit Tests (Mocked I/O)
**Duration: ~5 seconds**
- Priority queue tests
- Yield logic tests
- State preservation tests
- Chunk boundary tests

### Phase 3: Integration Tests (Real Buffer Manager)
**Duration: ~10 seconds**
- Buffer interaction tests
- Edge case tests

### Phase 4: Full Integration (Real Audio Files)
**Duration: ~30 seconds**
- Real audio decode tests
- Multi-passage tests

**Total Estimated Duration: ~46 seconds**

---

## Implementation Roadmap

### Week 1: Infrastructure
- [ ] Set up cargo-llvm-cov
- [ ] Create MockStreamingDecoder
- [ ] Create MockBufferManager
- [ ] Generate test audio assets
- [ ] Establish baseline coverage

### Week 2: Core Unit Tests
- [ ] Implement priority_queue_tests.rs
- [ ] Implement pipeline_flow_tests.rs
- [ ] Implement yield_logic_tests.rs
- [ ] Target: 50% line coverage

### Week 3: Advanced Unit Tests
- [ ] Implement state_preservation_tests.rs
- [ ] Implement chunk_boundary_tests.rs
- [ ] Implement buffer_interaction_tests.rs
- [ ] Target: 65% line coverage

### Week 4: Edge Cases + Integration
- [ ] Implement edge_cases_tests.rs
- [ ] Implement real_audio_decode_tests.rs
- [ ] Implement multi_passage_tests.rs
- [ ] Target: >70% line coverage

### Week 5: Coverage Refinement
- [ ] Identify remaining coverage gaps
- [ ] Add targeted tests for uncovered branches
- [ ] Review and refactor tests for clarity
- [ ] Final coverage report: >70% line, >60% branch

---

## Critical Path Test Matrix

| Critical Path | Test Coverage | Status |
|---------------|---------------|--------|
| Queue submission | ✅ Tested | Pass |
| Priority ordering | ✅ Tested | Pass |
| Condvar wake-up | ❌ Not tested | **TODO** |
| Decode chunk loop | ❌ Not tested | **TODO** |
| Resample chunk | ✅ Tested (unit) | Pass |
| Apply fades | ✅ Tested | Pass |
| Append to buffer | ❌ Not tested | **TODO** |
| Priority-based yield | ✅ Tested | Pass |
| Time-based yield | ✅ Tested | Pass |
| Buffer-full yield | ❌ Not tested | **TODO** |
| Pause decoder state | ❌ Not tested | **TODO** |
| Resume decoder state | ❌ Not tested | **TODO** |
| Shutdown | ❌ Not tested | **TODO** |

**Critical Path Coverage: 46% (6/13 tested)**
**Target: 100% (13/13 tested)**

---

## Success Criteria

### Minimum Acceptance Criteria
- ✅ Line coverage >70%
- ✅ Branch coverage >60%
- ✅ Critical path coverage 100%
- ✅ All tests pass
- ✅ No regressions in existing tests

### Stretch Goals
- Line coverage >80%
- Branch coverage >70%
- Integration tests with 5+ real audio formats
- Performance benchmarks for decode throughput
- Memory leak detection (valgrind/miri)

---

## References

- [SPEC016 Decoder Buffer Design](../SPEC016-decoder_buffer_design.md)
- [SPEC013 Single Stream Playback](../SPEC013-single_stream_playback.md)
- [SPEC014 Single Stream Design](../SPEC014-single_stream_design.md)
- [GOV002 Requirements Enumeration](../GOV002-requirements_enumeration.md)
- [Existing Serial Decoder Tests](../../wkmp-ap/tests/serial_decoder_tests.rs)

---

**Document Status:** DRAFT
**Next Review:** After Week 2 implementation
**Owner:** Test Coverage Analyst
