# Comprehensive Testing Framework Design for Decoder-Buffer-Mixer-Output Chain

**Document Type:** Testing Strategy and Framework Design
**Created:** 2025-10-22
**Status:** Design Proposal
**Related Specifications:**
- [SPEC016 Decoder Buffer Design](../SPEC016-decoder_buffer_design.md)
- [SPEC013 Single Stream Playback](../SPEC013-single_stream_playback.md)
- [SPEC002 Crossfade Design](../SPEC002-crossfade.md)

---

## Executive Summary

This document defines a comprehensive testing framework strategy for validating the decoder-buffer-mixer-output pipeline in the WKMP Audio Player. The framework is structured in 4 hierarchical levels, prioritizing the simplest case first (single chain, non-resampled, 44.1kHz) and progressively building to complex scenarios. The strategy emphasizes **automated validation** with **diagnostic instrumentation** to enable **in-situ performance optimization** on target hardware.

**Key Principles:**
1. **Start Simple**: Test single chain, non-resampled case first to establish baseline
2. **Measure Everything**: Instrument all critical points in the pipeline
3. **Automate Validation**: Define pass/fail criteria for deterministic testing
4. **Optimize In-Situ**: Use real-world scenarios on target hardware to tune parameters

---

## 1. Test Hierarchy Design

### Level 1: Unit Tests (Component Isolation)

**Purpose:** Validate individual components in isolation with mocked dependencies.

**Components to Test:**

#### 1.1 StreamingDecoder Unit Tests
- **File:** `wkmp-ap/src/audio/decoder.rs`
- **Tests:**
  - Decode single chunk from file (WAV, FLAC, MP3)
  - Verify chunk size matches requested duration (±tolerance)
  - Verify sample count matches expected (file_sample_rate * duration)
  - Verify format detection (sample_rate, channels)
  - Verify EOF detection on final chunk
  - Verify discovered endpoint tracking
  - State preservation across multiple decode_chunk() calls

**Traceability:**
- [DBD-DEC-090] Streaming/incremental operation
- [DBD-DEC-110] ~1 second chunk processing
- [DBD-DEC-130] State preservation for pause/resume

#### 1.2 StatefulResampler Unit Tests
- **File:** `wkmp-ap/src/audio/resampler.rs`
- **Tests:**
  - Pass-through mode when source_rate == target_rate (44.1kHz → 44.1kHz)
  - Upsampling (22.05kHz → 44.1kHz): verify 2x sample increase
  - Downsampling (88.2kHz → 44.1kHz): verify 2x sample reduction
  - Phase continuity across chunk boundaries (no clicks/pops)
  - Filter state preservation across chunks
  - Verify output sample count matches expected (input_count * target_rate / source_rate)

**Traceability:**
- [DBD-RSMP-010] Resampling to working_sample_rate
- [DBD-RSMP-020] Pass-through when at working_sample_rate
- [DBD-IMPL-100] Phase discontinuity elimination

#### 1.3 Fader Unit Tests
- **File:** `wkmp-ap/src/playback/pipeline/fader.rs`
- **Tests:**
  - Pass-through mode when fade durations are 0
  - Fade-in: verify samples multiply by fade curve (0.0 → 1.0)
  - Fade-out: verify samples multiply by fade curve (1.0 → 0.0)
  - Body region: verify no modification (multiplier == 1.0)
  - Sample-accurate frame position tracking
  - Verify all 5 fade curves produce correct multipliers (Linear, Exponential, Logarithmic, Cosine, EqualPower)

**Traceability:**
- [DBD-FADE-030] Pre-buffer fade-in application
- [DBD-FADE-050] Pre-buffer fade-out application
- [DBD-IMPL-090] Sample-accurate fade timing

#### 1.4 PlayoutRingBuffer Unit Tests
- **File:** `wkmp-ap/src/playback/playout_ring_buffer.rs`
- **Tests:**
  - Buffer starts empty ([DBD-BUF-020])
  - Push/pop FIFO ordering preserved
  - Fill percentage calculation (0% → 100%)
  - Decoder pause threshold detection ([DBD-BUF-050])
  - Decoder resume hysteresis ([DBD-PARAM-085])
  - Exhaustion detection (decode_complete + empty) ([DBD-BUF-060])
  - Last frame preservation on underrun ([DBD-BUF-030], [DBD-BUF-040])
  - Lock-free atomic operations correctness

**Traceability:**
- [DBD-BUF-010] through [DBD-BUF-060] Buffer lifecycle
- [DBD-PARAM-070] Capacity: 661,941 samples (15.01s @ 44.1kHz)
- [DBD-PARAM-080] Headroom: 4410 samples (0.1s @ 44.1kHz)

#### 1.5 BufferManager Unit Tests
- **File:** `wkmp-ap/src/playback/buffer_manager.rs`
- **Tests:**
  - State machine transitions (Empty → Filling → Ready → Playing → Finished)
  - ReadyForStart event emission at threshold
  - Event deduplication (only one ReadyForStart per buffer)
  - First-passage optimization (500ms threshold vs 3s)
  - Headroom calculation (write_position - read_position)
  - Buffer finalization (total_samples tracking)

**Traceability:**
- [DBD-BUF-020] through [DBD-BUF-060] State machine requirements
- [PERF-FIRST-010] First passage optimization
- [PERF-POLL-010] Event-driven buffer readiness

#### 1.6 CrossfadeMixer Unit Tests
- **File:** `wkmp-ap/src/playback/pipeline/mixer.rs`
- **Tests:**
  - SinglePassage state: sample retrieval from buffer
  - Crossfading state: dual buffer mixing with fade curves
  - Fade-in application (start at 0.0, end at 1.0)
  - Fade-out application (start at 1.0, end at 0.0)
  - Sample clipping detection and clamping
  - Passage completion detection via is_buffer_exhausted()
  - Pause/resume with fade-in
  - Underrun detection and flatline output
  - Mixer minimum start level enforcement ([DBD-MIX-060])

**Traceability:**
- [SSD-MIX-010] through [SSD-MIX-060] Mixer state machine
- [DBD-MIX-060] Minimum start level before drawing from chain
- [XFD-PAUS-010] Pause handling
- [XFD-PAUS-020] Resume fade-in

---

### Level 2: Integration Tests (Component Pairs)

**Purpose:** Test component interactions with real data flow between pairs.

#### 2.1 Decoder → Resampler Integration
- **Test:** Decode chunk from file, pass to resampler, verify output
- **Scenarios:**
  - Same rate (44.1kHz WAV → 44.1kHz output): verify pass-through
  - Upsample (22.05kHz WAV → 44.1kHz output): verify 2x sample increase
  - Downsample (96kHz FLAC → 44.1kHz output): verify sample reduction
- **Validation:**
  - Output sample count matches expected formula: `input_count * 44100 / source_rate`
  - No phase discontinuities between chunks (visual waveform inspection)
  - Frequency response preserved (spectral analysis)

**Traceability:**
- [DBD-DEC-110] Chunk-based decode → resample flow
- [DBD-RSMP-010] Resampling integration

#### 2.2 Resampler → Fader Integration
- **Test:** Resampled chunk → fader → verify fade application
- **Scenarios:**
  - No fade (pass-through): verify samples unchanged
  - Fade-in region: verify linear ramp (0.0 → 1.0)
  - Fade-out region: verify linear ramp (1.0 → 0.0)
  - Body region: verify no modification
- **Validation:**
  - Fade multipliers match expected curve values
  - Sample-accurate fade timing (frame position tracking)
  - No artifacts at fade boundaries

**Traceability:**
- [DBD-FADE-030] Fade-in before buffering
- [DBD-FADE-050] Fade-out before buffering
- [XFD-IMPL-091] Linear fade implementation

#### 2.3 Fader → Buffer Integration
- **Test:** Faded samples → buffer push → verify buffering
- **Scenarios:**
  - Push below threshold: verify state transitions (Empty → Filling)
  - Push to threshold: verify state transition (Filling → Ready) and event emission
  - Push to headroom: verify decoder pause flag
  - Push beyond capacity: verify BufferFullError
- **Validation:**
  - State transitions occur at correct sample counts
  - ReadyForStart event emitted exactly once
  - Pause threshold matches capacity - headroom
  - Samples retrievable in FIFO order

**Traceability:**
- [DBD-BUF-030] Filling state transition
- [DBD-BUF-050] Decoder pause on headroom
- [PERF-POLL-010] ReadyForStart event emission

#### 2.4 Buffer → Mixer Integration
- **Test:** Buffer → mixer pop_frame() → verify sample retrieval
- **Scenarios:**
  - Normal playback: verify samples retrieved in order
  - Buffer underrun: verify last frame returned ([DBD-BUF-040])
  - Buffer exhaustion: verify is_buffer_exhausted() detection
  - Crossfade: verify dual buffer mixing
- **Validation:**
  - Samples match expected values
  - Underrun returns last valid frame
  - Exhaustion detected when decode_complete + empty
  - Crossfade mixing applies correct fade curves

**Traceability:**
- [DBD-BUF-040] Last frame on underrun
- [DBD-BUF-060] Exhaustion detection
- [SSD-MIX-040] Crossfade mixing
- [SSD-MIX-060] Completion detection

#### 2.5 Mixer → Output Integration
- **Test:** Mixer → output ring buffer → verify audio output
- **Scenarios:**
  - Single passage playback: verify continuous audio
  - Crossfade: verify smooth transition
  - Pause: verify silence output
  - Resume: verify fade-in from silence
- **Validation:**
  - No clicks or pops in output
  - Volume level correct (master volume applied)
  - Crossfade smooth (no discontinuities)
  - Pause/resume timing accurate

**Traceability:**
- [DBD-MIX-010] through [DBD-MIX-060] Mixer output requirements
- [DBD-OUT-010] Single output stream
- [XFD-PAUS-010] Pause output
- [XFD-PAUS-020] Resume fade-in

---

### Level 3: End-to-End Tests (Full Pipeline)

**Purpose:** Test complete pipeline from file → audio output with real audio files.

#### 3.1 Single Chain Non-Resampled (PRIORITY 1)

**Test Case:** Complete pipeline for simplest scenario

**Setup:**
- Audio file: 44.1kHz stereo WAV (known sample count)
- Passage: Full file (start_time=0, end_time=file_duration)
- No fade-in, no fade-out
- No crossfade (single passage)

**Test Procedure:**
1. Enqueue passage
2. Start playback
3. Monitor pipeline metrics (see section 3 below)
4. Wait for completion
5. Validate results

**Integrity Tests:**

**Test 3.1.1: All Samples Reach Mixer**
- **Metric:** Total samples decoded == Total samples written to buffer == Total samples read by mixer
- **Pass Criteria:** Sample counts match ±1 frame (rounding tolerance)
- **Failure Modes:** Sample loss in buffer transitions, incomplete decode

**Test 3.1.2: No Samples Lost in Buffer Transitions**
- **Metric:** Buffer write_position - read_position == expected_buffered at any point
- **Pass Criteria:** Headroom never negative, no gaps in frame sequence
- **Failure Modes:** Buffer overflow, buffer underflow during decode

**Test 3.1.3: Sample Order Preserved**
- **Metric:** Visual waveform comparison (decoded vs played)
- **Pass Criteria:** Output waveform matches input waveform exactly
- **Failure Modes:** Sample reordering, buffer corruption

**Test 3.1.4: Exact Frame Count**
- **Metric:** Total frames output == file_duration_ms * 44100 / 1000
- **Pass Criteria:** Frame count matches expected ±1 frame
- **Failure Modes:** Truncation, extra samples appended

**Timing Tests:**

**Test 3.1.5: Samples Delivered at Correct Rate**
- **Metric:** Playback duration == expected duration (file_duration_ms ±10ms)
- **Pass Criteria:** Duration within 10ms of expected
- **Failure Modes:** Stalls (mixer waiting), rushing (mixer skipping)

**Test 3.1.6: Buffer Never Underruns**
- **Metric:** Buffer occupied > 0 during entire playback
- **Pass Criteria:** No underrun events logged
- **Failure Modes:** Decoder too slow, mixer too fast

**Test 3.1.7: Latency Within Bounds**
- **Metric:** Time from enqueue to first audio output
- **Pass Criteria:** Latency < min_buffer_threshold + decode_latency (typically ~3.5s)
- **Failure Modes:** Excessive buffering, slow decode

**State Machine Tests:**

**Test 3.1.8: Proper State Progression**
- **Metric:** State transitions match expected sequence
- **Pass Criteria:** Empty → Filling → Ready → Playing → Finished (no skips or reversals)
- **Failure Modes:** State machine bugs, race conditions

**Test 3.1.9: Buffer State Transitions**
- **Metric:** Buffer lifecycle states match expected
- **Pass Criteria:**
  - Empty (at allocation)
  - Filling (first chunk appended)
  - Ready (threshold reached, event emitted)
  - Playing (mixer starts reading)
  - Finished (decode complete, buffer drained)
- **Failure Modes:** Stuck in Filling, premature Finished

**Traceability:**
- [DBD-DEC-110] Chunk processing
- [DBD-BUF-020] through [DBD-BUF-060] Buffer lifecycle
- [SSD-MIX-030] Single passage playback
- [DBD-OUT-010] Output stream

#### 3.2 Single Chain Resampled

**Test Case:** Add sample rate conversion complexity

**Scenarios:**
- Upsample: 22.05kHz WAV → 44.1kHz output
- Downsample: 88.2kHz FLAC → 44.1kHz output
- High-res downsample: 192kHz FLAC → 44.1kHz output

**Additional Validation:**
- Output sample count == input_count * 44100 / source_rate (±1%)
- Frequency response preserved (spectral analysis)
- No aliasing artifacts (for downsample)
- No imaging artifacts (for upsample)

**Traceability:**
- [DBD-RSMP-010] Sample rate conversion
- [SPEC017] Sample rate conversion design

#### 3.3 Single Chain with Fades

**Test Case:** Add fade-in and fade-out

**Scenarios:**
- 5-second fade-in only (0s → 5s)
- 5-second fade-out only (duration-5s → duration)
- Both fade-in and fade-out

**Additional Validation:**
- First sample amplitude == 0.0 (fade-in)
- Last sample amplitude == 0.0 (fade-out)
- Body samples unmodified (amplitude matches source)
- Fade curve matches expected (Linear, Exponential, etc.)

**Traceability:**
- [DBD-FADE-030] Pre-buffer fade-in
- [DBD-FADE-050] Pre-buffer fade-out
- [XFD-IMPL-091] through [XFD-IMPL-095] Fade curves

#### 3.4 Dual Chain Crossfade

**Test Case:** Crossfade between two passages

**Setup:**
- Passage A: 44.1kHz WAV, 30s duration, 5s fade-out
- Passage B: 44.1kHz WAV, 30s duration, 5s fade-in
- Crossfade overlap: 5s

**Additional Validation:**
- Total samples == samples_A + samples_B (no gaps)
- Crossfade region: samples mixed correctly (A*fade_out + B*fade_in)
- No clipping in crossfade (values clamped to [-1.0, 1.0])
- Passage A completes correctly (completion event emitted)
- Mixer transitions to SinglePassage(B) after crossfade

**Traceability:**
- [SSD-MIX-040] Crossfade initiation
- [SSD-MIX-050] Sample-accurate mixing
- [XFD-COMP-010] Crossfade completion detection

#### 3.5 Multi-Chain Pre-buffering

**Test Case:** 3+ passages in queue with parallel decode

**Setup:**
- Enqueue 5 passages (all 44.1kHz WAV)
- Configure maximum_decode_streams = 3
- Start playback

**Additional Validation:**
- Chains 0, 1, 2 allocated to passages 1, 2, 3
- Passages 4, 5 wait without chains
- When passage 1 completes, chain 0 released and assigned to passage 4
- All passages play in correct order
- No buffer underruns during transitions

**Traceability:**
- [DBD-OV-050] maximum_decode_streams allocation
- [DBD-LIFECYCLE-010] through [DBD-LIFECYCLE-060] Chain lifecycle
- [DBD-DEC-040] Serial decode priority

---

### Level 4: In-Situ Performance Tests

**Purpose:** Test real-world scenarios on target hardware to measure performance and optimize parameters.

#### 4.1 Hardware Characterization

**Test:** Measure baseline decode/resample performance

**Procedure:**
1. Decode 30-minute FLAC file (96kHz → 44.1kHz downsample)
2. Measure decode time for each chunk
3. Calculate decode speed ratio (realtime vs wallclock)
4. Measure CPU usage during decode
5. Measure memory usage (peak and average)

**Metrics:**
- Decode speed: X.Xx realtime (e.g., 2.5x = decodes 2.5 seconds of audio per 1 second wallclock)
- CPU usage: X% average, Y% peak
- Memory usage: X MB average, Y MB peak
- Cache miss rate (if available via profiling)

**Use Case:**
- Determine if hardware can sustain 3+ parallel decodes
- Identify bottlenecks (CPU, I/O, memory bandwidth)
- Tune decode_chunk_size for optimal throughput

#### 4.2 Buffer Parameter Optimization

**Test:** Find optimal buffer sizes for target hardware

**Parameters to Tune:**
- `playout_ringbuffer_size` ([DBD-PARAM-070], default: 661,941 samples = 15.01s)
- `playout_ringbuffer_headroom` ([DBD-PARAM-080], default: 4410 samples = 0.1s)
- `decoder_resume_hysteresis_samples` ([DBD-PARAM-085], default: 44,100 samples = 1.0s)
- `mixer_min_start_level` ([DBD-PARAM-088], default: 44,100 samples = 1.0s)
- `decode_chunk_size` ([DBD-PARAM-065], default: 25,000 samples)

**Optimization Strategy:**

**Step 1: Profile Current Performance**
- Run Test 3.1 (single chain non-resampled) with default parameters
- Measure:
  - Time to first audio output (startup latency)
  - Buffer fill rate (samples/second)
  - Buffer underrun count
  - CPU usage during decode
  - Memory usage

**Step 2: Vary playout_ringbuffer_size**
- Test values: 220,500 (5s), 441,000 (10s), 661,941 (15s), 882,000 (20s)
- Measure impact on:
  - Startup latency (larger = slower start)
  - Underrun resilience (larger = fewer underruns)
  - Memory usage (larger = more RAM)

**Step 3: Vary mixer_min_start_level**
- Test values: 22,050 (0.5s), 44,100 (1.0s), 88,200 (2.0s), 132,300 (3.0s)
- Measure impact on:
  - Time to first audio (larger = longer wait)
  - Underrun count during crossfades (larger = fewer underruns)

**Step 4: Vary decode_chunk_size**
- Test values: 10,000, 25,000 (default), 50,000, 100,000
- Measure impact on:
  - Decode overhead (smaller = more overhead per chunk)
  - Buffer fill responsiveness (smaller = finer granularity)
  - CPU cache efficiency (larger = better cache locality)

**Step 5: Vary hysteresis**
- Test values: 22,050 (0.5s), 44,100 (1.0s), 88,200 (2.0s)
- Measure impact on:
  - Decoder pause/resume oscillation (smaller = more oscillation)
  - Buffer fill stability (larger = smoother operation)

**Automated Parameter Search:**

Use **gradient descent** or **binary search** to find optimal parameters:

```
Objective Function:
  minimize(startup_latency_ms)
  subject to:
    underrun_count == 0
    cpu_usage < 50%
    memory_usage < 200MB
```

**Algorithm:**
1. Start with default parameters
2. Measure objective function (run Test 3.1)
3. Perturb one parameter (±20%)
4. Re-measure objective function
5. If improvement: move in that direction
6. If degradation: revert and try different parameter
7. Repeat until convergence (no improvement in 10 iterations)

#### 4.3 Stress Testing

**Test:** Simulate worst-case scenarios

**Scenarios:**

**Scenario 1: Maximum Decode Streams**
- Enqueue 12 passages (maximum_decode_streams = 12)
- Mix of sample rates (22.05kHz, 44.1kHz, 88.2kHz, 192kHz)
- Mix of codecs (WAV, FLAC, MP3, AAC)
- Start playback and measure:
  - Time to decode all 12 passages
  - CPU usage during concurrent decodes
  - Buffer underrun count
  - Memory usage peak

**Scenario 2: Rapid Queue Changes**
- Enqueue 5 passages, start playback
- Every 10 seconds: skip current passage
- Measure:
  - Crossfade glitch count
  - Buffer cleanup time (chain release → reallocation)
  - Memory leaks (measure over 100 skip cycles)

**Scenario 3: High-Resolution Files**
- Queue 5x 30-minute FLAC files (192kHz → 44.1kHz downsample)
- Measure:
  - Decode time per file
  - Buffer underrun count
  - CPU temperature (thermal throttling indicator)
  - Fan noise (subjective measure of CPU load)

**Scenario 4: Low-Power Hardware**
- Run tests on Raspberry Pi 4 or equivalent low-power device
- Reduce maximum_decode_streams until underruns eliminated
  - Try: 12, 8, 6, 4, 3, 2, 1
  - Find minimum value that sustains zero underruns
- Document recommended settings for low-power hardware

---

## 2. Diagnostic Instrumentation Design

### 2.1 What to Measure

**Critical Metrics:**

#### Decoder Metrics
- **Total samples decoded** (counter, monotonic)
- **Decode chunk count** (counter)
- **Decode time per chunk** (histogram, ms)
- **Samples per second** (gauge, throughput)
- **File format** (metadata: WAV, FLAC, MP3, etc.)
- **Source sample rate** (metadata: 22050, 44100, 88200, etc.)
- **Discovered endpoint** (event, ticks)

#### Resampler Metrics
- **Pass-through mode** (boolean flag)
- **Resample ratio** (gauge, source_rate / target_rate)
- **Samples input** (counter)
- **Samples output** (counter)
- **Resample time per chunk** (histogram, ms)

#### Fader Metrics
- **Fade state** (enum: PassThrough, FadeIn, Body, FadeOut)
- **Frame position** (gauge, sample-accurate)
- **Current multiplier** (gauge, 0.0-1.0)
- **Fade-in duration** (metadata, samples)
- **Fade-out duration** (metadata, samples)

#### Buffer Metrics
- **Buffer state** (enum: Empty, Filling, Ready, Playing, Finished)
- **Fill percentage** (gauge, 0-100%)
- **Occupied frames** (gauge)
- **Free space** (gauge)
- **Write position** (counter, monotonic)
- **Read position** (counter, monotonic)
- **Decoder should pause** (boolean flag)
- **Decode complete** (boolean flag)
- **Is exhausted** (boolean flag)
- **Total written** (counter, lifetime)
- **Total read** (counter, lifetime)

#### Mixer Metrics
- **Mixer state** (enum: None, SinglePassage, Crossfading)
- **Current passage ID** (UUID)
- **Next passage ID** (UUID, if crossfading)
- **Frame position** (gauge)
- **Is crossfading** (boolean flag)
- **Crossfade frame count** (gauge, if crossfading)
- **Volume level** (gauge, 0.0-1.0)
- **Pause state** (boolean flag)
- **Resume fade-in progress** (gauge, 0.0-1.0)

#### Output Metrics
- **Output ring buffer fill** (gauge, %)
- **Output samples sent** (counter, monotonic)
- **Underrun count** (counter)
- **Clipping count** (counter)

### 2.2 How to Instrument

**Instrumentation Points:**

#### 2.2.1 Atomic Counters (Lock-Free)

Use `std::sync::atomic::AtomicU64` for high-frequency updates:

```rust
pub struct DecoderMetrics {
    total_samples_decoded: AtomicU64,
    chunk_count: AtomicU64,
}

impl DecoderMetrics {
    pub fn record_chunk(&self, samples: usize) {
        self.total_samples_decoded.fetch_add(samples as u64, Ordering::Relaxed);
        self.chunk_count.fetch_add(1, Ordering::Relaxed);
    }
}
```

**Usage:**
- Increment after each chunk decode
- Read periodically for monitoring (no locks required)

#### 2.2.2 Ring Buffer for Time-Series Metrics

Store last N measurements for trend analysis:

```rust
pub struct TimingRingBuffer {
    measurements: Vec<f64>, // circular buffer
    index: AtomicUsize,     // current write index
    capacity: usize,        // N measurements
}

impl TimingRingBuffer {
    pub fn record(&self, value: f64) {
        let idx = self.index.fetch_add(1, Ordering::Relaxed) % self.capacity;
        // SAFETY: Single writer (decoder thread) guarantees no race
        unsafe { *self.measurements.get_unchecked_mut(idx) = value; }
    }

    pub fn average(&self) -> f64 {
        self.measurements.iter().sum::<f64>() / self.capacity as f64
    }
}
```

**Usage:**
- Record decode time for each chunk
- Calculate rolling average (last 100 chunks)
- Detect decode slowdown trends

#### 2.2.3 Event Log for State Transitions

Log all state machine transitions with timestamps:

```rust
pub struct StateTransitionLog {
    events: Arc<RwLock<Vec<StateEvent>>>,
}

pub struct StateEvent {
    timestamp: Instant,
    queue_entry_id: Uuid,
    old_state: BufferState,
    new_state: BufferState,
    samples_buffered: usize,
}

impl StateTransitionLog {
    pub fn log_transition(&self, event: StateEvent) {
        self.events.write().unwrap().push(event);
    }

    pub fn validate_sequence(&self, queue_entry_id: Uuid) -> Result<(), String> {
        let events = self.events.read().unwrap();
        let sequence: Vec<_> = events.iter()
            .filter(|e| e.queue_entry_id == queue_entry_id)
            .map(|e| e.new_state)
            .collect();

        // Expected: Empty → Filling → Ready → Playing → Finished
        let expected = vec![
            BufferState::Empty,
            BufferState::Filling,
            BufferState::Ready,
            BufferState::Playing,
            BufferState::Finished,
        ];

        if sequence == expected {
            Ok(())
        } else {
            Err(format!("Invalid state sequence: {:?}", sequence))
        }
    }
}
```

**Usage:**
- Log every state transition in BufferManager
- Post-test validation of state machine correctness
- Replay event log for debugging

#### 2.2.4 Performance Profiling Hooks

Use `tracing::instrument` for automatic function timing:

```rust
use tracing::instrument;

#[instrument(skip(self, samples))]
pub fn push_samples(&self, queue_entry_id: Uuid, samples: &[f32]) -> Result<usize> {
    // Function body automatically timed by tracing
    // Logs: enter, exit, duration
}
```

**Configuration:**
```toml
# tracing_subscriber filters
RUST_LOG=wkmp_ap::playback=trace  # Enable all playback instrumentation
```

**Usage:**
- Automatically records function entry/exit/duration
- Aggregates timing statistics (min, max, avg, p50, p95, p99)
- Visualize with `tracing-chrome` or `tracing-flamegraph`

### 2.3 Metric Collection Strategy

**Polling vs Event-Driven:**

| Metric | Collection Method | Frequency | Overhead |
|--------|------------------|-----------|----------|
| Buffer fill % | Polling | 100ms | Low (atomic read) |
| Decode time | Event-driven | Per chunk | Low (timestamp diff) |
| State transitions | Event-driven | On change | Very low (log append) |
| Sample counts | Atomic counter | Per chunk | Negligible (fetch_add) |
| Mixer position | Atomic counter | Per frame | Negligible (load) |

**Monitoring Thread:**

Separate thread for periodic metric collection:

```rust
pub async fn monitoring_loop(metrics: Arc<PipelineMetrics>) {
    let mut interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        interval.tick().await;

        let snapshot = metrics.snapshot();

        // Emit SSE event to UI
        emit_metrics_update(snapshot);

        // Check for anomalies
        if snapshot.buffer_fill_percent < 10.0 && snapshot.is_playing {
            warn!("Buffer critically low: {}%", snapshot.buffer_fill_percent);
        }
    }
}
```

**SSE Integration:**

Metrics available via Server-Sent Events for real-time UI updates:

```
GET /playback/metrics/stream

data: {"buffer_fill": 75.3, "decode_speed": 2.1x, "mixer_state": "Playing"}
data: {"buffer_fill": 76.8, "decode_speed": 2.0x, "mixer_state": "Playing"}
```

---

## 3. Automated Validation Strategy

### 3.1 Pass/Fail Criteria

**Deterministic Tests (Must Pass):**

| Test | Pass Criteria | Failure Indication |
|------|--------------|-------------------|
| **Sample Integrity** | decoded == written == read (±1 frame) | Data loss or duplication |
| **Sample Order** | Output waveform matches input | Buffer corruption |
| **Frame Count** | output_frames == expected_frames (±1) | Truncation or padding |
| **State Sequence** | Empty → Filling → Ready → Playing → Finished | State machine bug |
| **No Gaps** | Continuous frame sequence numbers | Buffer transition error |
| **No Clipping** | All samples in [-1.0, 1.0] range | Overflow or mixing error |
| **Fade Accuracy** | Fade multipliers match curve formula (±0.01) | Fade calculation bug |

**Performance Tests (Target Thresholds):**

| Metric | Target | Warning | Failure |
|--------|--------|---------|---------|
| **Startup Latency** | < 3.5s | > 4.0s | > 5.0s |
| **Decode Speed** | > 1.5x realtime | < 1.2x | < 1.0x |
| **Buffer Underruns** | 0 | 1-3 | > 3 |
| **CPU Usage** | < 30% avg | 30-50% | > 50% |
| **Memory Usage** | < 150MB | 150-200MB | > 200MB |
| **Decode Time (per chunk)** | < 400ms | 400-600ms | > 600ms |

### 3.2 Test Automation

**Test Harness Structure:**

```rust
pub struct PipelineTestHarness {
    // Components
    buffer_manager: Arc<BufferManager>,
    mixer: Arc<RwLock<CrossfadeMixer>>,

    // Instrumentation
    metrics: Arc<PipelineMetrics>,
    event_log: Arc<StateTransitionLog>,

    // Test configuration
    config: TestConfig,
}

impl PipelineTestHarness {
    pub async fn run_test(&self, test_case: TestCase) -> TestResult {
        // 1. Setup
        self.setup(test_case.setup_config).await?;

        // 2. Execute
        let start = Instant::now();
        self.execute_pipeline(test_case.input_file).await?;
        let duration = start.elapsed();

        // 3. Collect metrics
        let metrics_snapshot = self.metrics.snapshot();
        let state_sequence = self.event_log.get_sequence(queue_entry_id);

        // 4. Validate
        let validation_result = self.validate(
            test_case.expected,
            metrics_snapshot,
            state_sequence,
        );

        // 5. Cleanup
        self.cleanup().await?;

        // 6. Report
        TestResult {
            test_name: test_case.name,
            duration,
            metrics: metrics_snapshot,
            validation: validation_result,
            pass: validation_result.is_ok(),
        }
    }

    fn validate(
        &self,
        expected: ExpectedMetrics,
        actual: MetricsSnapshot,
        state_sequence: Vec<BufferState>,
    ) -> Result<ValidationReport, ValidationError> {
        let mut report = ValidationReport::default();

        // Sample integrity check
        if (actual.total_decoded - actual.total_output).abs() > 1 {
            report.failures.push(format!(
                "Sample count mismatch: decoded={}, output={}",
                actual.total_decoded, actual.total_output
            ));
        }

        // State sequence check
        let expected_states = vec![
            BufferState::Empty,
            BufferState::Filling,
            BufferState::Ready,
            BufferState::Playing,
            BufferState::Finished,
        ];
        if state_sequence != expected_states {
            report.failures.push(format!(
                "Invalid state sequence: {:?}",
                state_sequence
            ));
        }

        // Timing check
        let expected_duration = expected.file_duration_ms;
        let actual_duration = actual.playback_duration_ms;
        if (expected_duration - actual_duration).abs() > 10 {
            report.warnings.push(format!(
                "Timing drift: expected={}ms, actual={}ms",
                expected_duration, actual_duration
            ));
        }

        // Underrun check
        if actual.underrun_count > 0 {
            report.failures.push(format!(
                "Buffer underruns detected: {}",
                actual.underrun_count
            ));
        }

        if report.failures.is_empty() {
            Ok(report)
        } else {
            Err(ValidationError::FailedCriteria(report))
        }
    }
}
```

**Test Suite Integration:**

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_single_chain_non_resampled() {
        let harness = PipelineTestHarness::new().await;

        let test_case = TestCase {
            name: "Single chain, 44.1kHz WAV, no resampling",
            input_file: "test_files/440hz_tone_44100hz_10s.wav",
            setup_config: TestConfig {
                sample_rate: 44100,
                fade_in_duration: 0,
                fade_out_duration: 0,
                crossfade: None,
            },
            expected: ExpectedMetrics {
                file_duration_ms: 10000,
                total_frames: 441000,
                decode_chunks: 10, // ~1s per chunk
            },
        };

        let result = harness.run_test(test_case).await;

        assert!(result.pass, "Test failed: {:?}", result.validation);
        assert_eq!(result.metrics.total_decoded, 441000);
        assert_eq!(result.metrics.underrun_count, 0);
    }

    #[tokio::test]
    async fn test_crossfade_dual_chain() {
        let harness = PipelineTestHarness::new().await;

        let test_case = TestCase {
            name: "Dual chain crossfade",
            input_file: vec![
                "test_files/passage_a.wav",
                "test_files/passage_b.wav",
            ],
            setup_config: TestConfig {
                crossfade: Some(CrossfadeConfig {
                    duration_ms: 5000,
                    fade_out_curve: FadeCurve::Linear,
                    fade_in_curve: FadeCurve::Linear,
                }),
            },
            expected: ExpectedMetrics {
                total_frames: 441000 + 441000 - (5 * 44100), // 10s + 10s - 5s overlap
                crossfade_duration_frames: 5 * 44100,
            },
        };

        let result = harness.run_test(test_case).await;

        assert!(result.pass);
        assert_eq!(result.metrics.crossfade_completed_count, 1);
    }
}
```

### 3.3 Performance Regression Detection

**Baseline Metrics Storage:**

Store baseline metrics for each test in JSON:

```json
{
  "test_single_chain_non_resampled": {
    "baseline_date": "2025-10-22",
    "hardware": "Intel i7-9750H, 16GB RAM",
    "metrics": {
      "startup_latency_ms": 3200,
      "decode_speed_ratio": 2.3,
      "cpu_usage_avg": 25.0,
      "memory_usage_mb": 120,
      "decode_time_p95_ms": 380
    }
  }
}
```

**Regression Check:**

Compare current run against baseline:

```rust
pub fn check_regression(
    current: MetricsSnapshot,
    baseline: BaselineMetrics,
) -> RegressionReport {
    let mut report = RegressionReport::default();

    // Startup latency (tolerance: +20%)
    let latency_increase = (current.startup_latency - baseline.startup_latency)
        / baseline.startup_latency * 100.0;

    if latency_increase > 20.0 {
        report.regressions.push(Regression {
            metric: "startup_latency_ms",
            baseline: baseline.startup_latency,
            current: current.startup_latency,
            change_percent: latency_increase,
            severity: Severity::Warning,
        });
    }

    // Decode speed (tolerance: -10%)
    let speed_decrease = (baseline.decode_speed - current.decode_speed)
        / baseline.decode_speed * 100.0;

    if speed_decrease > 10.0 {
        report.regressions.push(Regression {
            metric: "decode_speed_ratio",
            baseline: baseline.decode_speed,
            current: current.decode_speed,
            change_percent: -speed_decrease,
            severity: Severity::Error,
        });
    }

    report
}
```

**CI/CD Integration:**

```bash
#!/bin/bash
# test_runner.sh - Run tests and check for regressions

cargo test --release --test pipeline_integration_tests

# Extract metrics from test output
METRICS=$(cargo run --bin extract_metrics < test_output.log)

# Compare against baseline
REGRESSION_CHECK=$(cargo run --bin check_regression -- \
    --baseline baselines/current.json \
    --metrics "$METRICS")

if [ $? -ne 0 ]; then
    echo "REGRESSION DETECTED:"
    echo "$REGRESSION_CHECK"
    exit 1
fi

echo "All tests passed, no regressions detected"
```

---

## 4. In-Situ Optimization Strategy

### 4.1 Hardware Detection

**Capability Probing:**

Automatically detect hardware capabilities on startup:

```rust
pub struct HardwareCapabilities {
    cpu_cores: usize,
    cpu_speed_ghz: f64,
    memory_total_mb: usize,
    memory_available_mb: usize,
    decode_speed_estimate: f64, // realtime multiplier
}

impl HardwareCapabilities {
    pub async fn detect() -> Self {
        // Probe CPU
        let cpu_cores = num_cpus::get();
        let cpu_speed = Self::estimate_cpu_speed();

        // Probe memory
        let (total, available) = Self::get_memory_info();

        // Benchmark decode speed
        let decode_speed = Self::benchmark_decode().await;

        Self {
            cpu_cores,
            cpu_speed_ghz: cpu_speed,
            memory_total_mb: total,
            memory_available_mb: available,
            decode_speed_estimate: decode_speed,
        }
    }

    async fn benchmark_decode() -> f64 {
        // Decode 10 seconds of 96kHz FLAC
        let start = Instant::now();
        let decoder = StreamingDecoder::new("benchmark/test_96khz.flac", 0, 10000)?;

        let mut total_samples = 0;
        while let Some(chunk) = decoder.decode_chunk(1000)? {
            total_samples += chunk.len();
        }

        let elapsed = start.elapsed().as_secs_f64();
        let realtime_duration = total_samples as f64 / 96000.0 / 2.0; // stereo

        realtime_duration / elapsed // e.g., 2.5 = 2.5x realtime
    }
}
```

**Recommended Settings:**

Based on hardware capabilities, recommend optimal parameters:

```rust
pub fn recommend_settings(caps: &HardwareCapabilities) -> SettingsRecommendation {
    let mut settings = SettingsRecommendation::default();

    // maximum_decode_streams based on CPU and decode speed
    if caps.decode_speed_estimate > 3.0 && caps.cpu_cores >= 4 {
        settings.maximum_decode_streams = 12; // High-end hardware
    } else if caps.decode_speed_estimate > 2.0 && caps.cpu_cores >= 2 {
        settings.maximum_decode_streams = 6; // Mid-range hardware
    } else {
        settings.maximum_decode_streams = 3; // Low-end hardware
    }

    // playout_ringbuffer_size based on memory
    if caps.memory_available_mb > 500 {
        settings.playout_ringbuffer_size = 661_941; // 15s buffer (default)
    } else {
        settings.playout_ringbuffer_size = 441_000; // 10s buffer (reduced)
    }

    // mixer_min_start_level based on decode speed
    if caps.decode_speed_estimate > 4.0 {
        settings.mixer_min_start_level = 22_050; // 0.5s (fast decode)
    } else if caps.decode_speed_estimate > 2.0 {
        settings.mixer_min_start_level = 44_100; // 1.0s (normal)
    } else {
        settings.mixer_min_start_level = 88_200; // 2.0s (slow decode)
    }

    settings
}
```

### 4.2 Adaptive Parameter Tuning

**Runtime Adjustment:**

Monitor playback performance and adjust parameters dynamically:

```rust
pub struct AdaptiveTuner {
    metrics_history: RingBuffer<MetricsSnapshot>,
    current_settings: ParameterSet,
}

impl AdaptiveTuner {
    pub fn evaluate_and_adjust(&mut self) -> Option<ParameterAdjustment> {
        let recent_underruns = self.metrics_history.iter()
            .map(|m| m.underrun_count)
            .sum::<usize>();

        if recent_underruns > 3 {
            // Increase buffer to prevent underruns
            return Some(ParameterAdjustment {
                parameter: "mixer_min_start_level",
                old_value: self.current_settings.mixer_min_start_level,
                new_value: self.current_settings.mixer_min_start_level * 2,
                reason: "Frequent underruns detected",
            });
        }

        let avg_decode_speed = self.metrics_history.iter()
            .map(|m| m.decode_speed_ratio)
            .sum::<f64>() / self.metrics_history.len() as f64;

        if avg_decode_speed < 1.2 {
            // Decode too slow, reduce parallel streams
            return Some(ParameterAdjustment {
                parameter: "maximum_decode_streams",
                old_value: self.current_settings.maximum_decode_streams,
                new_value: (self.current_settings.maximum_decode_streams * 2 / 3).max(1),
                reason: "Decode speed insufficient for current load",
            });
        }

        None // No adjustment needed
    }
}
```

### 4.3 Test Infrastructure Recommendations

**Test File Repository:**

Create standardized test files for reproducible testing:

```
test_files/
├── 44100hz/
│   ├── 440hz_tone_10s.wav         # Pure 440Hz sine wave, 10s
│   ├── pink_noise_30s.wav         # Pink noise, 30s
│   ├── speech_sample_60s.wav      # Human speech, 60s
│   └── music_classical_180s.wav   # Classical music, 3min
├── 22050hz/
│   └── tone_440hz_10s.wav         # For upsample testing
├── 88200hz/
│   └── tone_440hz_10s.flac        # For downsample testing
├── 96000hz/
│   └── music_hires_300s.flac      # High-res, 5min
└── 192000hz/
    └── music_hires_300s.flac      # High-res, 5min
```

**Test Utilities:**

Provide helper functions for common test operations:

```rust
pub mod test_utils {
    /// Generate deterministic test file (440Hz sine wave)
    pub fn generate_test_file(
        path: &Path,
        sample_rate: u32,
        duration_secs: u64,
    ) -> Result<()> {
        let mut writer = WavWriter::create(path, WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        })?;

        let samples_per_channel = sample_rate as u64 * duration_secs;
        for i in 0..samples_per_channel {
            let t = i as f64 / sample_rate as f64;
            let sample = (t * 440.0 * 2.0 * PI).sin() * 0.5; // 440Hz, 50% amplitude

            let sample_i16 = (sample * i16::MAX as f64) as i16;
            writer.write_sample(sample_i16)?; // Left
            writer.write_sample(sample_i16)?; // Right
        }

        writer.finalize()?;
        Ok(())
    }

    /// Compare two waveforms for equality (with tolerance)
    pub fn compare_waveforms(
        actual: &[f32],
        expected: &[f32],
        tolerance: f32,
    ) -> Result<(), WaveformMismatch> {
        if actual.len() != expected.len() {
            return Err(WaveformMismatch::LengthMismatch {
                actual: actual.len(),
                expected: expected.len(),
            });
        }

        for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
            if (a - e).abs() > tolerance {
                return Err(WaveformMismatch::SampleMismatch {
                    index: i,
                    actual: *a,
                    expected: *e,
                    diff: (a - e).abs(),
                });
            }
        }

        Ok(())
    }

    /// Extract samples from audio file for comparison
    pub fn extract_samples(file_path: &Path) -> Result<Vec<f32>> {
        let decoder = StreamingDecoder::new(file_path, 0, u64::MAX)?;
        let mut samples = Vec::new();

        while let Some(chunk) = decoder.decode_chunk(1000)? {
            samples.extend_from_slice(&chunk);
        }

        Ok(samples)
    }
}
```

---

## 5. Implementation Roadmap

### Phase 1: Unit Tests (Week 1)
- Implement unit tests for all 6 components (Decoder, Resampler, Fader, Buffer, BufferManager, Mixer)
- Target: 80% code coverage
- Deliverable: `cargo test --lib` passes all unit tests

### Phase 2: Integration Tests (Week 2)
- Implement 5 integration test suites (Decoder→Resampler, Resampler→Fader, etc.)
- Create test file repository (10+ standardized audio files)
- Deliverable: `cargo test --test integration_tests` passes

### Phase 3: End-to-End Tests (Week 3)
- Implement Test 3.1 (single chain non-resampled) - PRIORITY 1
- Implement Test 3.2 (single chain resampled)
- Implement Test 3.3 (single chain with fades)
- Deliverable: Full pipeline validated for single passage playback

### Phase 4: Diagnostic Instrumentation (Week 4)
- Add atomic counters for sample counts
- Add timing ring buffers for performance tracking
- Add state transition event log
- Add SSE endpoint for real-time metrics
- Deliverable: Metrics dashboard showing live pipeline status

### Phase 5: Automated Validation (Week 5)
- Implement PipelineTestHarness
- Define pass/fail criteria for all tests
- Implement regression detection against baselines
- Integrate with CI/CD pipeline
- Deliverable: Automated test suite with regression alerts

### Phase 6: Performance Optimization (Week 6)
- Run hardware characterization tests
- Optimize buffer parameters using gradient descent
- Document recommended settings for 3 hardware tiers (high/mid/low)
- Deliverable: Tuned parameter recommendations

### Phase 7: Stress Testing (Week 7)
- Implement 4 stress test scenarios
- Test on low-power hardware (Raspberry Pi 4)
- Document failure modes and recovery strategies
- Deliverable: Stress test report with hardware recommendations

### Phase 8: Documentation (Week 8)
- Write testing guide for developers
- Document all test utilities and helpers
- Create troubleshooting guide for common failures
- Deliverable: Complete testing documentation

---

## 6. Success Criteria

**Milestone 1 (End of Phase 3):**
- ✓ Test 3.1 (single chain non-resampled) passes with zero failures
- ✓ All samples reach output (integrity check passes)
- ✓ No buffer underruns detected
- ✓ State machine progression correct (Empty → Filling → Ready → Playing → Finished)

**Milestone 2 (End of Phase 5):**
- ✓ 20+ automated tests pass consistently
- ✓ CI/CD pipeline runs tests on every commit
- ✓ Regression detection alerts on performance degradation (>10%)

**Milestone 3 (End of Phase 7):**
- ✓ Zero underruns on target hardware (normal load)
- ✓ Parameters optimized for 3 hardware tiers
- ✓ Stress tests pass on Raspberry Pi 4 (with reduced maximum_decode_streams)

**Final Success Criteria:**
- ✓ 100% of integrity tests pass (sample count, order, timing)
- ✓ 95% of performance tests meet targets (latency, CPU, memory)
- ✓ 100% test coverage for critical paths (decode → buffer → mixer → output)
- ✓ Zero known regressions vs baseline metrics
- ✓ Complete documentation for testing framework

---

## 7. Appendices

### Appendix A: Test File Specifications

| File | Format | Sample Rate | Duration | Purpose |
|------|--------|-------------|----------|---------|
| 440hz_tone_44100hz_10s.wav | WAV | 44100 | 10s | Baseline non-resampled test |
| 440hz_tone_22050hz_10s.wav | WAV | 22050 | 10s | Upsample 2x test |
| 440hz_tone_88200hz_10s.flac | FLAC | 88200 | 10s | Downsample 2x test |
| pink_noise_44100hz_30s.wav | WAV | 44100 | 30s | Crossfade test (passage A) |
| speech_sample_44100hz_60s.wav | WAV | 44100 | 60s | Crossfade test (passage B) |
| music_classical_96000hz_300s.flac | FLAC | 96000 | 300s | Stress test (high-res) |
| music_classical_192000hz_300s.flac | FLAC | 192000 | 300s | Stress test (ultra high-res) |

### Appendix B: Metric Definitions

| Metric | Unit | Formula | Interpretation |
|--------|------|---------|----------------|
| **Decode speed ratio** | ratio | realtime_duration / wallclock_duration | 2.5 = decodes 2.5x faster than realtime |
| **Buffer fill %** | % | (occupied_frames / capacity) * 100 | 75% = buffer 75% full |
| **Startup latency** | ms | time_to_first_audio - time_enqueued | Lower is better (target <3.5s) |
| **CPU usage** | % | (cpu_time / wall_time) * 100 | Lower is better (target <30%) |
| **Memory usage** | MB | process_rss | Lower is better (target <150MB) |
| **Underrun count** | count | Number of buffer empty events | Must be zero |

### Appendix C: Troubleshooting Guide

**Symptom: Buffer underruns during playback**
- **Cause 1:** Decode too slow (decode_speed_ratio < 1.2)
  - **Fix:** Reduce maximum_decode_streams to 6 or lower
  - **Fix:** Increase decode_chunk_size to reduce overhead
- **Cause 2:** Mixer starts before buffer filled
  - **Fix:** Increase mixer_min_start_level to 2.0s or higher
- **Cause 3:** Buffer too small for file type
  - **Fix:** Increase playout_ringbuffer_size to 20s or higher

**Symptom: High startup latency (>5 seconds)**
- **Cause 1:** min_buffer_threshold too high
  - **Fix:** Reduce to 1.0s for faster start (with more underrun risk)
- **Cause 2:** Decode slow to start (seeking overhead)
  - **Fix:** Use pre-decoded formats (WAV) instead of compressed (FLAC/MP3)
- **Cause 3:** Multiple passages decoding simultaneously
  - **Fix:** Prioritize "now playing" decoder (already implemented)

**Symptom: Samples lost or duplicated**
- **Cause:** Buffer corruption or state machine bug
  - **Fix:** Enable state transition logging
  - **Fix:** Validate buffer write/read positions with assertions
  - **Fix:** Check for race conditions in lock-free atomics

**Symptom: Crossfade glitches or pops**
- **Cause 1:** Fade curve miscalculation
  - **Fix:** Validate fade multipliers against expected curve
  - **Fix:** Use Linear fade as baseline (simplest)
- **Cause 2:** Sample clipping during crossfade
  - **Fix:** Enable clipping detection in mixer
  - **Fix:** Reduce passage volumes before crossfade
- **Cause 3:** Next buffer not ready when crossfade starts
  - **Fix:** Increase mixer_min_start_level for next passage
  - **Fix:** Pre-buffer next passage earlier (increase maximum_decode_streams)

---

## Document Metadata

**Version:** 1.0
**Created:** 2025-10-22
**Last Updated:** 2025-10-22
**Status:** Design Proposal
**Author:** Claude (AI Assistant)
**Reviewers:** TBD

**Related Requirements:**
- [DBD-DEC-110] Chunk-based decoding
- [DBD-BUF-010] through [DBD-BUF-060] Buffer lifecycle
- [SSD-MIX-010] through [SSD-MIX-060] Mixer state machine
- [DBD-PARAM-070] through [DBD-PARAM-088] Parameter specifications

**Change Log:**
- v1.0 (2025-10-22): Initial design document created

---

**END OF DOCUMENT**
