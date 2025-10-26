# Single Stream Design for Audio Playback with Crossfading

**🔔 IMPORTANT NOTICE:**
**SPEC016 Decoder Buffer Design is the authoritative specification for the audio pipeline architecture.**
This document (SPEC014) provides supplementary context and design rationale. For implementation details, decoder behavior, buffer management, mixer operation, and fade application, refer to [SPEC016-decoder_buffer_design.md](SPEC016-decoder_buffer_design.md).

> **Related Documentation:** [Architecture](SPEC001-architecture.md) | [Requirements](REQ001-requirements.md) | [Crossfade Design](SPEC002-crossfade.md) | [Single Stream Playback](SPEC013-single_stream_playback.md) | **[Decoder Buffer Design](SPEC016-decoder_buffer_design.md)** ← **Authoritative** | [Sample Rate Conversion](SPEC017-sample_rate_conversion.md)

---

## Overview

The Single Stream architecture uses serial decode execution and lock-free ring buffers to achieve continuous playback with seamless crossfading between passages. Audio is decoded, resampled, and faded before buffering. The mixer reads pre-faded samples and sums them during crossfade overlap.

**For complete architecture details, see [SPEC016 Decoder Buffer Design](SPEC016-decoder_buffer_design.md).**

## Motivation

This design addresses key limitations of the dual pipeline approach:
- **Sample-accurate crossfading** - Mix at buffer level rather than volume property updates
- **Lower memory footprint** - Single output stream with shorter decode-ahead buffers
- **Predictable behavior** - No framework state machine complexity
- **Cross-platform simplicity** - Pure Rust, single static binary
- **Precise timing control** - Direct control over fade curves and timing

## Architecture

### High-Level Design

Refer to [SPEC016 Decoder Buffer Design](SPEC016-decoder_buffer_design.md#overview) for current architecture.

### Component Structure

#### 1. Decoder Architecture

**[SSD-DEC-010]** Decoder architecture specified in [SPEC016 Decoder Buffer Design](SPEC016-decoder_buffer_design.md).

**Key Design Points:**
- **Serial decode execution:** One decoder processes at a time ([DBD-DEC-040])
- **Priority-based switching:** Higher priority passages interrupt lower priority ([DBD-DEC-110])
- **Decode-and-skip approach:** Sample-accurate positioning ([DBD-DEC-050] through [DBD-DEC-080])
- **Maximum decode streams:** 12 decoder-buffer chains ([DBD-PARAM-050])
- **DecoderWorker:** Single-threaded serial processing for cache coherency ([DBD-IMPL-040])

**Dependencies:**
- `symphonia` - Pure Rust audio decoding (MP3, FLAC, AAC, Vorbis, etc.)
- `rubato` - Pure Rust resampling ([DBD-RSMP-010]: resample when != working_sample_rate, bypass when == working_sample_rate)

#### 2. Buffer Management

**[SSD-BUF-005]** Buffer architecture specified in [SPEC016 Buffers](SPEC016-decoder_buffer_design.md#buffers).

**Key Design Points:**
- **Ring buffers:** Lock-free circular buffers per passage ([DBD-BUF-010])
- **Buffer size:** 661,941 samples (15.01 seconds @ 44.1kHz) ([DBD-PARAM-070])
- **Backpressure:** Decoder pauses when buffer nearly full ([DBD-BUF-050])
- **Completion detection:** Buffer signals when end sample consumed ([DBD-BUF-060])
- **Fade application:** Pre-buffer by Fader component ([DBD-FADE-030/050])

**Fade Curves:** Per [SPEC002 Fade Curves](SPEC002-crossfade.md#fade-curves):
- Linear, Exponential (fade-in), Logarithmic (fade-out), Cosine, Equal-power

**Chain Assignment Strategy:**

**[SSD-BUF-010]** Chain allocation per [SPEC016 Chain Assignment Lifecycle - DBD-LIFECYCLE-010]:
- Up to 12 passages assigned decoder-buffer chains ([DBD-PARAM-050])
- Chains assigned on enqueue when available
- Lowest-numbered chain allocated first
- Chain persists with passage until completion/removal

**Decoding Process:**
- **Decode-and-skip:** Start from file beginning, skip to passage start ([DBD-DEC-050/060])
- **Resampling:** Convert to working_sample_rate (44.1kHz) when needed ([DBD-RSMP-010])
- **Fade application:** Fader applies curves during decode, before buffering ([DBD-FADE-030/050])
- **End detection:** Stop when passage end_time reached ([DBD-FADE-060])

**Benefits:**
- Sample-accurate positioning at any point in the passage
- Repeatable, exact time points within the audio file
- No dependency on format-specific seeking capabilities
- Eliminates seeking latency during playback

**Buffer Underrun Handling:**

**[SSD-UND-010]** Per [SPEC016 Mixer - Pause Mode](SPEC016-decoder_buffer_design.md#mixer):
- **Detection:** Mixer reads from empty buffer
- **Response:** Auto-pause with exponential decay ([DBD-MIX-050/051/052])
- **Recovery:** Auto-resume when buffer refills to mixer_min_start_level

**Memory Usage:**

Per [SPEC016 Operating Parameters - DBD-PARAM-070]:
- Buffer size per passage: 661,941 samples (15.01s @ 44.1kHz) ≈ 5.3 MB
- Maximum 12 buffers: 60 MB total
- Typical usage (3-5 active passages): 16-27 MB

### Buffer State Event Emission

**[SSD-BUF-020]** Event Integration:

The Passage Buffer Manager emits `BufferStateChanged` events (event_system.md) at four key transition points:

**Transition 1: None → Decoding**
- **Trigger:** Decoder thread starts filling buffer from audio file
- **Event Data:** `old_state=None, new_state=Decoding, decode_progress_percent=0.0`
- **Use Case:** UI can show decode progress indicator

**Transition 2: Decoding → Ready**
- **Trigger:** Decoder completes buffer population (full or partial)
- **Event Data:** `old_state=Decoding, new_state=Ready, decode_progress_percent=100.0`
- **Use Case:** Confirms buffer available for playback

**Transition 3: Ready → Playing**
- **Trigger:** Mixer starts reading buffer for audio output
- **Event Data:** `old_state=Ready, new_state=Playing, decode_progress_percent=None`
- **Use Case:** Track which passage currently outputting audio

**Transition 4: Playing → Exhausted**
- **Trigger:** Mixer reaches end of buffer (or crossfade lead-out completes)
- **Event Data:** `old_state=Playing, new_state=Exhausted, decode_progress_percent=None`
- **Use Case:** Buffer lifecycle debugging

**[SSD-BUF-021]** Decode Progress Updates:
- While in Decoding state: Emit progress updates every 10% completion
- Throttling: Maximum one event per second
- Use Case: Large file decode monitoring (e.g., 30-minute recording)

**[SSD-BUF-022]** API Integration:
- `GET /playback/buffer_status` endpoint returns current state of all buffers
- State reflects most recent BufferStateChanged event for each passage
- See api_design.md for complete endpoint specification

#### 3. Crossfade Mixer

**Purpose:** Mix passage buffers with fade curves to produce single output stream.

**Rust Structure:**
```rust
pub struct CrossfadeMixer {
    buffer_manager: Arc<PassageBufferManager>,
    current_passage: Option<MixerPassage>,
    next_passage: Option<MixerPassage>,
    sample_rate: u32,
}

struct MixerPassage {
    passage_id: Uuid,
    position_samples: u64, // Current playback position
    volume: f32,
}

impl CrossfadeMixer {
    /// Fill output buffer with mixed audio
    pub fn fill_output_buffer(&mut self, output: &mut [f32]) -> Result<()> {
        // For each output sample pair (L, R):
        // 1. Read current passage sample
        // 2. Apply fade-out curve if in fade region
        // 3. Read next passage sample (if crossfading)
        // 4. Apply fade-in curve
        // 5. Sum samples: output = current * fade_out + next * fade_in
        // 6. Advance positions
    }

    /// Apply fade curve at given position
    fn apply_fade_curve(&self, sample: f32, curve: &FadeCurve,
                        position: u64, duration: u64) -> f32 {
        let t = position as f64 / duration as f64; // 0.0 to 1.0
        let gain = match curve {
            FadeCurve::Linear => t,
            FadeCurve::Logarithmic => (t * 100.0 + 1.0).ln() / (101.0_f64.ln()),
            FadeCurve::Exponential => t * t,
            FadeCurve::SCurve => (1.0 - (t * PI).cos()) / 2.0,
        };
        sample * gain as f32
    }
}
```

**Crossfade Algorithm:**

Crossfade mixing algorithm: See [SPEC016 Mixer - DBD-MIX-040] for complete mixer implementation including:
- Sample mixing with volume multiplication
- Crossfade handling during overlap
- Master volume application

For crossfade timing calculation, see [SPEC002 Crossfade Design](SPEC002-crossfade.md#implementation-algorithm).

Original pseudocode (superseded by SPEC016):
```rust
// Pseudocode for sample-accurate crossfade
for i in 0..output.len() step 2 {
    let mut left = 0.0;
    let mut right = 0.0;

    // Current passage contribution
    if let Some(current) = &mut self.current_passage {
        let (l, r) = self.read_sample(current);
        let fade_out_gain = self.calculate_fade_out_gain(current);
        left += l * fade_out_gain;
        right += r * fade_out_gain;
    }

    // Next passage contribution (during crossfade)
    if let Some(next) = &mut self.next_passage {
        let (l, r) = self.read_sample(next);
        let fade_in_gain = self.calculate_fade_in_gain(next);
        left += l * fade_in_gain;
        right += r * fade_in_gain;
    }

    output[i] = left;
    output[i + 1] = right;
}
```

**Crossfade Summation and Clipping Detection:**

See [SPEC002 Clipping Prevention](SPEC002-crossfade.md#clipping-prevention) for policy:
- [XFD-VOL-030]: No runtime clipping prevention
- [XFD-VOL-040]: UI warning during editing when amplitude >50%

Mixer implementation follows [SPEC016 Mixer - DBD-MIX-040].

**[SSD-CLIP-010]** Original specification (superseded by SPEC002):
- During crossover from one passage to the next (lead-out of the currently playing passage and lead-in of the next), data from the two passages' buffers is summed to get the audio data for output.

**[SSD-CLIP-020]** If summation results in an output level that will be clipped (absolute value > 1.0 for f32 samples):
- **[SSD-CLIP-021]** Log warning describing: The playback point in both passages, fade durations, fade curves
- **[SSD-CLIP-025]** Clipping prevention: Apply appropriate gain reduction or limiting
- **[SSD-CLIP-026]** Warning frequency: Only log a maximum of one warning per crossover (avoid log spam)

**Fade Curve Examples:**
```
Linear:       y = x
Logarithmic:  y = ln(100x + 1) / ln(101)  [gradual start, faster end]
Exponential:  y = x²                       [faster start, gradual end]
S-Curve:      y = (1 - cos(πx)) / 2       [smooth acceleration/deceleration]
```

### Crossfade Timing Calculation Ownership

See [SPEC002 Crossfade Design - Implementation Algorithm](SPEC002-crossfade.md#implementation-algorithm) for complete crossfade timing calculation specification ([XFD-IMPL-010] through [XFD-IMPL-050]).

Mixer implementation follows [SPEC016 Mixer - DBD-MIX-040].

**[SSD-MIX-040]** Original specification (superseded by SPEC002/SPEC016):
- Owner: CrossfadeMixer component
- Trigger: When `queue_next_passage()` called by PlaybackEngine
- Timing: Calculation happens BEFORE decode starts (enables pre-loading trigger)
- Algorithm: See crossfade.md [XFD-IMPL-020] for complete pseudocode

**[SSD-MIX-041]** Validation Integration:
- Executes Phase 3 validation (crossfade.md XFD-VAL-010)
- On validation failure: Return error to PlaybackEngine
- PlaybackEngine response: Emit PassageCompleted(reason="invalid_timing"), skip passage
- See crossfade.md "Validation Responsibility" for three-phase strategy

**[SSD-MIX-042]** Timing Output:
- Returns: (lead_out_start_sample, crossfade_duration_samples, next_passage_lead_in_samples)
- Used by: PlaybackEngine to calculate decoder queue submission timing
- Precision: Sample-accurate (not time-based)

#### 4. Audio Output Thread

**Purpose:** Send mixed audio to system audio device.

**Rust Structure:**
```rust
pub struct AudioOutput {
    device: cpal::Device,
    stream: cpal::Stream,
    mixer: Arc<RwLock<CrossfadeMixer>>,
    ring_buffer: Arc<RwLock<RingBuffer>>,
    sample_rate: u32,
}

struct RingBuffer {
    buffer: Vec<f32>,
    write_pos: usize,
    read_pos: usize,
    capacity: usize,
}

impl AudioOutput {
    /// Start audio playback
    pub fn start(&mut self) -> Result<()> {
        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Audio callback - runs on real-time thread
                // Read from ring buffer, fill output
                self.ring_buffer.write().unwrap().read_into(data);
            },
            move |err| {
                eprintln!("Audio stream error: {}", err);
            },
        )?;

        stream.play()?;
        self.stream = Some(stream);

        // Start mixer thread to keep ring buffer filled
        self.start_mixer_thread();

        Ok(())
    }

    fn start_mixer_thread(&self) {
        let mixer = self.mixer.clone();
        let ring_buffer = self.ring_buffer.clone();

        tokio::spawn(async move {
            let mut mix_buffer = vec![0.0f32; 2048];
            loop {
                // Fill mix buffer from passage buffers
                mixer.write().await.fill_output_buffer(&mut mix_buffer)?;

                // Write to ring buffer
                ring_buffer.write().await.write_from(&mix_buffer);

                // Sleep briefly to avoid busy loop
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
    }
}
```

**Dependencies:**
- `cpal` - Cross-platform audio I/O library (Pure Rust)

**Buffer Sizing:**
```
Ring buffer: [DBD-PARAM-030] output_ringbuffer_size (8192 samples = 185ms @ 44.1kHz)
Mix buffer: 2048 samples * 2 channels = 4096 samples (~46ms @ 44.1kHz)
```

## Audio Pipeline Architecture

### Overview

The audio pipeline processes audio from file to output using a chain of components. See [SPEC016 Decoder Buffer Design](SPEC016-decoder_buffer_design.md#overview) for complete architecture.

### Processing Chain

Per [SPEC016 DBD-OV-040](SPEC016-decoder_buffer_design.md#overview):
1. **Decoder:** Decode compressed audio to PCM
2. **Resampler:** Convert to working_sample_rate (44.1kHz)
3. **Fader:** Apply fade-in/fade-out curves to samples
4. **Buffer:** Store pre-faded PCM audio
5. **Mixer:** Sum samples from multiple buffers during crossfade
6. **Output:** Send to audio device via cpal

### Key Architectural Fact

**Fades Applied Pre-Buffer:** Per [SPEC002 XFD-ORTH-020](SPEC002-crossfade.md#fade-points-vs-lead-points-orthogonal-concepts) and [SPEC016 DBD-FADE-030/050](SPEC016-decoder_buffer_design.md#fade-inout-handlers):
- Fade curves applied by Fader component BEFORE buffering
- Buffered audio already has volume envelopes applied
- Mixer simply sums pre-faded samples during crossfade overlap
- No runtime fade calculations needed during playback

### Data Flow Summary

1. **Enqueue:** Passage added to queue, chain assigned if available ([DBD-LIFECYCLE-010])
2. **Decode:** Decoder processes file via decode-and-skip ([DBD-DEC-050/060/070])
3. **Resample:** Convert to 44.1kHz if needed ([DBD-RSMP-010])
4. **Fade:** Apply fade curves to samples ([DBD-FADE-030/050])
5. **Buffer:** Store pre-faded PCM in ring buffer ([DBD-BUF-010])
6. **Mix:** Read and sum pre-faded samples during crossfade ([DBD-MIX-040])
7. **Output:** Send to cpal audio device ([DBD-OUT-010])

### Crossfade Timing

**When Crossfade Occurs:** Per [SPEC002 Implementation Algorithm - XFD-IMPL-020](SPEC002-crossfade.md#implementation-algorithm):
- Crossfade duration = min(lead_out_duration of current, lead_in_duration of next)
- Next passage starts when current passage has crossfade_duration time remaining
- Both passages play simultaneously during overlap period

**How Mixing Works:** Per [SPEC016 Mixer - DBD-MIX-041](SPEC016-decoder_buffer_design.md#mixer):
```
// Both passages have fade curves already applied to buffered samples
sample_current = read_from_buffer(current_passage_buffer)  // Pre-faded
sample_next = read_from_buffer(next_passage_buffer)        // Pre-faded

// Simple addition - no fade curve calculations needed
mixed_sample = sample_current + sample_next
   - `output.clamp(-1.0, 1.0)`

6. **Advance positions:**
   - `current_passage_position += 1`
   - `next_passage_position += 1`

**Fade Curves:**
- Linear: Constant rate of change
- Exponential: Slow start, fast finish (natural for fade-in)
- Logarithmic: Fast start, slow finish (natural for fade-out)
- S-Curve: Smooth acceleration/deceleration using cosine

**Implementation:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/single_stream/mixer.rs:384-423`

#### Step 6: Output to Audio Device

Audio output continuously polls mixer for samples and sends to audio device.

**Polling Architecture:**
- Audio output requests buffer from mixer (e.g., 512 samples)
- Mixer returns pre-calculated playout buffer
- No real-time mixing in audio callback (all pre-calculated)
- Simple buffer copy operation in audio thread

**Implementation:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/single_stream/output_simple.rs:51-109`

### Advantages of This Architecture

**Sample Accuracy:**
- Wall-clock timing (tokio::sleep) has variable latency due to CPU scheduling
- Buffer position is deterministic and precise
- Crossfades start at exact sample positions, not approximate times
- At 44.1kHz: precision = 0.0227ms per sample

**Performance:**
- Pre-calculation means audio callback is simple buffer copy
- No complex fade calculations in real-time audio thread
- Reduced risk of audio underruns/glitches
- CPU-friendly for resource-constrained devices

**Correctness:**
- Independent passage position tracking prevents buffer read errors
- Per-passage positions reset when passages change
- Eliminates issue where passages tried to read from wrong buffer indices
- Deterministic behavior regardless of system load

### Testing Results

**Automated Integration Test:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/crossfade_test.rs`

Verified functionality:
- Enqueued 3 passages with 8-second crossfade overlap
- Decoder successfully extracted 20-second segments from middle of MP3 files
- Sample-accurate positioning (882,000+ samples per passage @ 44.1kHz)
- Crossfade transitions executed between all passages
- Complete playback from start to finish

**Timing Precision:**
- Trigger latency: ~10ms (467 samples @ 44.1kHz)
- Crossfade alignment: Sample-accurate
- No audio glitches or dropouts observed

## Data Flow

### Complete Playback Sequence

```
1. User enqueues passage
   └─> QueueManager adds entry to database
   └─> BufferManager triggers decode request
   └─> DecoderPool starts decoding passage

2. Decoder processes file
   └─> symphonia opens and decodes file
   └─> Uses decode-and-skip for sample-accurate positioning
       - Always decode from start of audio file
       - Skip samples until passage start time
       - Continue decoding until passage end time
       - If passage ends before file end, stop decoding at passage end
   └─> rubato resamples to 44.1kHz if needed
   └─> PCM data written to PassageBuffer
   └─> Fade curves applied during decode (or deferred to read-time)
       - Fade-in curve applied as soon as decode passes fade-in point
       - Fade-out curve applied as soon as decode reaches end time
   └─> BufferManager marks passage as Ready

3. Playback starts
   └─> CrossfadeMixer selects current passage
   └─> Resets current_passage_position to 0
   └─> MixerThread continuously fills output buffer
   └─> AudioOutput pulls from mixer

4. Next passage queued
   └─> Calculate crossfade trigger sample position
   └─> Store in mixer's crossfade_start_sample field
   └─> Reset next_passage_position to 0
   └─> Monitor current_passage_position

5. Crossfade auto-triggers
   └─> When current_passage_position >= crossfade_start_sample
   └─> Begin sample-accurate mixing
   └─> For each frame:
       - Read current passage sample at current_passage_position
       - Read next passage sample at next_passage_position
       - Calculate fade gains based on crossfade progress
       - Apply fade-out curve to current sample
       - Apply fade-in curve to next sample
       - Sum weighted samples
       - Clamp to prevent clipping
       - Advance both position counters

6. After crossfade complete
   └─> Current passage marked as Exhausted
   └─> Next passage becomes current
   └─> Transfer next_passage_position to current_passage_position
   └─> Reset next_passage_position to 0
   └─> BufferManager recycles exhausted buffer
   └─> Process continues for next passage
```

### Timing Precision

**Sample-Accurate Mixing:**
- At 44.1kHz, each sample = 0.0227 ms. Crossfade timing precise to ~0.02 ms
- Sample-accurate timing per [SPEC016 Decoders - DBD-DEC-080]. For tick-level precision (~35.4 nanoseconds), see [SPEC017 Tick Rate - SRC-TICK-030].
- Compare to property-based: ~10-50 ms precision

**Passage Timing Parameters:**
```rust
pub struct PassageTimingData {
    start_time_ms: i64,      // Start of passage in file
    end_time_ms: i64,        // End of passage in file
    lead_in_point_ms: i64,   // Where crossfade-in begins
    lead_out_point_ms: i64,  // Where crossfade-out begins
    fade_in_point_ms: i64,   // Full volume point after fade-in
    fade_out_point_ms: i64,  // Start fading out point
    fade_in_curve: String,
    fade_out_curve: String,
}
```

**Crossfade Overlap Calculation:**
```
Passage A: [========fade_out========]
Passage B:              [========fade_in========]
                        ^
                    overlap point

Overlap duration = (A.end - A.fade_out_point) = (B.fade_in_point - B.start)
```

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1)
- [ ] Implement `PassageBuffer` structure
- [ ] Implement `PassageBufferManager` with memory management
- [ ] Add `symphonia` integration for decoding
- [ ] Add `rubato` integration for resampling
- [ ] Write unit tests for decoding pipeline

### Phase 2: Crossfade Mixer (Week 1-2)
- [ ] Implement `CrossfadeMixer` with basic mixing
- [ ] Implement fade curve algorithms (Linear, Log, Exp, S-Curve)
- [ ] Add sample-accurate position tracking
- [ ] Write tests for crossfade algorithms
- [ ] Benchmark mixing performance

### Phase 3: Audio Output (Week 2)
- [ ] Implement `AudioOutput` with `cpal`
- [ ] Implement ring buffer with thread-safe access
- [ ] Add audio callback for real-time output
- [ ] Test on Linux, macOS, Windows
- [ ] Handle audio device errors and reconnection

### Phase 4: Integration (Week 3)
- [ ] Create `SingleStreamPipeline` interface matching `DualPipeline`
- [ ] Integrate with existing `PlaybackEngine`
- [ ] Update API endpoints to use new pipeline
- [ ] Test complete playback flow
- [ ] Add position/duration queries

### Phase 5: Optimization (Week 4)
- [ ] Profile and optimize hot paths
- [ ] Tune buffer sizes for latency vs. stability
- [ ] Add adaptive buffer management
- [ ] Test under load (rapid skipping, queue changes)
- [ ] Memory leak testing

### Phase 6: Feature Parity (Week 4)
- [ ] Implement pause/resume
- [ ] Implement seek within passage
- [ ] Implement volume control
- [ ] Implement EOS detection
- [ ] Add comprehensive error handling

## Performance Characteristics

### Memory Usage

See [SPEC016 Operating Parameters](SPEC016-decoder_buffer_design.md#operating-parameters) for authoritative buffer memory calculations:
- [DBD-PARAM-070]: playout_ringbuffer_size (661941 samples, 60MB total for 12 buffers)
- [DBD-PARAM-030]: output_ringbuffer_size (8192 samples)
- [DBD-PARAM-080]: playout_ringbuffer_headroom (441 samples)

Original estimate (superseded by SPEC016):
```
Base overhead:        ~5 MB (code + structures)
Per passage buffer:   ~5.3 MB (15 sec @ 44.1kHz stereo)
Ring buffer:          ~32 KB
Mix buffer:           ~16 KB
Total (5 passages):   ~31 MB
```

### CPU Usage
```
Decoding:    ~5-10% per decoder thread (depends on format)
Resampling:  ~2-5% per stream (if needed)
Mixing:      ~1-2% (highly optimized)
Total:       ~10-20% single core (multi-threaded decoding)
```

### Latency
```
Audio callback period:  ~10-20 ms (configurable)
Ring buffer latency:    ~46-185 ms (configurable)
Crossfade precision:    ~0.02 ms (sample-accurate)
Skip latency:           <100 ms (if buffer ready)
```

## Advantages Over Dual Pipeline

1. **Sample-Accurate Crossfading**
   - Direct sample mixing vs. volume property updates
   - Precision: 0.02 ms vs. 10-50 ms
   - Smoother transitions with custom curves

2. **Lower Memory Footprint**
   - ~31 MB vs. 100-200 MB
   - Efficient buffer recycling
   - Configurable buffer durations

3. **Simpler Deployment**
   - Pure Rust dependencies
   - Single static binary
   - No external framework (GStreamer not required)

4. **Predictable Behavior**
   - No state machine complexity
   - Full control over timing
   - Easier to debug (all code is yours)

5. **Cross-Platform**
   - Same code on Linux/macOS/Windows
   - No platform-specific codecs needed
   - Consistent behavior across platforms

## Challenges and Solutions

### Challenge 1: Real-Time Performance
**Problem:** Audio callbacks run on real-time threads - cannot block.
**Solution:**
- Use lock-free ring buffer for audio thread
- Pre-fill buffers before playback starts
- Handle buffer underruns gracefully (output silence)

### Challenge 2: Decoder Performance
**Problem:** Decoding might not keep up with playback.
**Solution:**
- Parallel decoding with thread pool
- Prioritize currently playing passage
- Pre-decode next 2-3 passages
- Monitor buffer fill levels, increase priority if low

### Challenge 3: Format Support
**Problem:** Need to support many audio formats.
**Solution:**
- Use `symphonia` (supports MP3, FLAC, AAC, Vorbis, WAV, etc.)
- Graceful fallback for unsupported formats
- Log warnings for problematic files

### Challenge 4: Sample Rate Conversion
**Problem:** Files may have different sample rates.
**Solution:**
- Use `rubato` for high-quality resampling
- Cache resampled data in passage buffer
- Standard rate: 44.1kHz (most common)

### Challenge 5: Seek Performance
**Problem:** Seeking in compressed formats is unreliable and format-dependent.
**Solution:**
- **Never use compressed file seeking** - always decode from file start
- Skip samples to reach desired position (decode-and-skip approach)
- For currently playing passage: Full decode ensures instant seeks within passage
- For queued passages: 15-second buffer provides time to complete full decode before playback
- Trade-off: Accepts longer initial decode time for guaranteed accuracy and reliability

### Challenge 6: Buffer Underruns During Skip-Ahead
**[SSD-UND-030] Problem:** User may skip ahead in queue faster than decoder can populate buffers.
**[SSD-UND-031] Solution:**
- **[SSD-UND-032]** Partial buffer playback: Start playing from partial buffer while decode continues
- **[SSD-UND-033]** Automatic pause/resume: If playback reaches unbuffered region, pause with flatline output until sufficient buffer available (1+ second)
- **[SSD-UND-034]** Logging and diagnostics: Log warnings with buffer status, skip activity, and decode speed
- **[SSD-UND-035]** Adaptive pre-buffering: Estimate required buffer time based on passage start time in file
- **[SSD-UND-036]** Seamless buffer switching: Switch from partial to complete buffer at same sample point when full decode completes

### Challenge 7: Fade Curve Application Timing
**[SSD-FADE-020] Problem:** Playback may reach fade-out point before curve has been applied to buffer data.
**[SSD-FADE-021] Solution:**
- **[SSD-FADE-022]** Early application: Apply fade-out curve as soon as decode reaches end time (during buffer population)
- **[SSD-FADE-023]** Automatic pause: If playback reaches fade-out point before curve applied, pause until application complete
- **[SSD-FADE-024]** Implementation flexibility: Allow fade application during decode OR defer to read-time based on implementation needs
- **[SSD-FADE-025]** Logging: Warn if timing issue occurs, including buffer status and skip activity

### Ring Buffer Underrun Logging Classification
**[SSD-RBUF-010] Implementation:** Audio ring buffer uses context-aware logging to distinguish expected underruns from concerning ones.

**[SSD-RBUF-011] Expected Underruns (TRACE level):**
- **Startup underruns:** First ~50-100ms after audio output initialization, before mixer fills buffer
- **Startup stabilization underruns:** Within 2-second grace period after buffer first filled to optimal level
- **Paused state underruns:** When PlaybackState is Paused, no audio output expected
- **Empty queue underruns:** When queue has no passages to play

**[SSD-RBUF-012] Concerning Underruns (WARN level):**
- **Active playback underruns:** When PlaybackState is Playing AND queue has passages AND past grace period
- Indicates CPU cannot keep up with decoding, or mixer thread is blocked

**[SSD-RBUF-013] Context Tracking:**
- `buffer_has_been_filled` flag: Set when buffer reaches 50-75% fill (startup complete)
- `buffer_filled_timestamp_ms` timestamp: Records Unix milliseconds when buffer first filled
- `audio_expected` flag: Updated by PlaybackEngine based on state + queue status
- All flags/timestamps use lock-free atomics for real-time audio thread safety

**[SSD-RBUF-014] Grace Period:**
- **Duration:** Configurable via `audio_ring_buffer_grace_period_ms` database setting (default: 2000ms)
- **Valid range:** 0-10000ms (0 = no grace period, 10000 = 10 seconds maximum)
- **Purpose:** Allow system stabilization before treating underruns as concerning
- **Implementation:** Check current time vs `buffer_filled_timestamp_ms + grace_period_ms`
- **Configuration:** Users can adjust grace period for slower/faster systems

**[SSD-RBUF-015] Log Frequency:**
- Underruns logged every 1000th occurrence to avoid log spam
- Each message includes cumulative total for trend tracking

## Testing Strategy

### Unit Tests
- Fade curve calculations
- Buffer management (allocation, recycling)
- Sample mixing algorithms
- Ring buffer operations

### Integration Tests
- Complete playback flow
- Crossfade transitions
- Skip and seek operations
- Queue manipulation during playback

### Performance Tests
- Memory usage profiling
- CPU usage under load
- Latency measurements
- Buffer underrun detection

### Platform Tests
- Linux (PulseAudio, ALSA, JACK)
- macOS (CoreAudio)
- Windows (WASAPI)
- Bluetooth audio devices
- HDMI audio output

## Migration Path

### Option 1: Feature Flag
```rust
#[cfg(feature = "single-stream")]
use crate::playback::pipeline::SingleStreamPipeline as Pipeline;

#[cfg(not(feature = "single-stream"))]
use crate::playback::pipeline::DualPipeline as Pipeline;
```

### Option 2: Runtime Selection
```rust
pub enum PipelineType {
    Dual,      // GStreamer-based
    SingleStream,  // Manual buffer management
}

impl PlaybackEngine {
    pub fn new(pipeline_type: PipelineType, ...) -> Self {
        let pipeline: Box<dyn PlaybackPipeline> = match pipeline_type {
            PipelineType::Dual => Box::new(DualPipeline::new()?),
            PipelineType::SingleStream => Box::new(SingleStreamPipeline::new()?),
        };
        ...
    }
}
```

### Option 3: Full Migration
1. Implement single stream pipeline
2. Test thoroughly in parallel with dual pipeline
3. Switch default to single stream
4. Deprecate dual pipeline
5. Remove GStreamer dependency

## File Structure

```
wkmp-ap/src/playback/
├── pipeline/
│   ├── mod.rs                 # Pipeline trait
│   ├── dual.rs                # GStreamer implementation (existing)
│   └── single_stream/
│       ├── mod.rs             # SingleStreamPipeline
│       ├── decoder.rs         # DecoderPool
│       ├── buffer.rs          # PassageBufferManager
│       ├── mixer.rs           # CrossfadeMixer
│       ├── output.rs          # AudioOutput
│       └── curves.rs          # Fade curve algorithms
├── engine.rs                  # PlaybackEngine (updated)
├── queue.rs                   # QueueManager (unchanged)
└── state.rs                   # SharedPlaybackState (unchanged)
```

## Dependencies to Add

```toml
[dependencies]
# Audio decoding (Pure Rust)
symphonia = { version = "0.5", features = ["mp3", "flac", "aac", "vorbis"] }

# Sample rate conversion (Pure Rust)
rubato = "0.15"

# Audio output (Pure Rust, cross-platform)
cpal = "0.15"

# Existing dependencies
tokio = { version = "1", features = ["full"] }
anyhow = "1"
tracing = "0.1"
```

## References

- **Symphonia**: https://github.com/pdm-sound/symphonia
- **Rubato**: https://github.com/HEnquist/rubato
- **CPAL**: https://github.com/RustAudio/cpal
- **Crossfading Algorithms**: https://signalsmith-audio.co.uk/writing/2021/cheap-energy-crossfade/
- **Audio Programming Guide**: https://www.musicdsp.org/en/latest/

---

## Requirement Traceability Index

This section provides a comprehensive index of all traceability IDs assigned to specifications in this document, organized by category for easy reference.

### DEC - Decoder (Decoding and Decode-and-Skip Strategy)

| ID | Description |
|----|-------------|
| SSD-DEC-010 | Decode-from-start-and-skip approach for sample-accurate positioning |
| SSD-DEC-011 | Receive decode request from buffer manager |
| SSD-DEC-012 | Open file using symphonia decoder |
| SSD-DEC-013 | Always decode from beginning of audio file (never use compressed seek) |
| SSD-DEC-014 | Skip samples until reaching passage start time |
| SSD-DEC-015 | Continue decoding until passage end time |
| SSD-DEC-016 | Resample to 44.1kHz if needed using rubato |
| SSD-DEC-017 | Write PCM data to passage buffer |
| SSD-DEC-018 | Notify buffer manager of completion |
| SSD-DEC-020 | Rationale for decode-and-skip approach |
| SSD-DEC-021 | Compressed file seeking is unreliable and format-dependent |
| SSD-DEC-022 | Variable bitrate files have unpredictable seek performance |
| SSD-DEC-023 | Decode-from-start ensures exact, repeatable time points |
| SSD-DEC-024 | Provides sample-accurate positioning for crossfading |
| SSD-DEC-025 | Trade-off: Longer decode time guarantees correctness |

### BUF - Buffer Management

| ID | Description |
|----|-------------|
| SSD-BUF-010 | Two distinct buffering strategies based on passage playback state |

### FBUF - Full Buffer (Full Decode Strategy)

| ID | Description |
|----|-------------|
| SSD-FBUF-010 | Full decode strategy for currently playing or next-to-play passages |
| SSD-FBUF-011 | Entire passage decoded to RAM when in current/next position |
| SSD-FBUF-012 | Purpose: Enable instant, sample-accurate seeking |
| SSD-FBUF-013 | Rationale: Compressed decoder seek-to-time is unreliable |
| SSD-FBUF-020 | Resampling to 44.1kHz when necessary |
| SSD-FBUF-021 | Decode-and-skip approach for accurate timing |
| SSD-FBUF-022 | Never use compressed seek |
| SSD-FBUF-023 | Skip decoded audio before passage start time |
| SSD-FBUF-024 | Implicit start time at audio file start if not specified |
| SSD-FBUF-025 | Continue decoding until passage end time |
| SSD-FBUF-026 | Implicit end time at audio file end if not specified |
| SSD-FBUF-027 | May stop decoding when passage end reached (before file end) |
| SSD-FBUF-030 | Benefit: Sample-accurate positioning at any point |
| SSD-FBUF-031 | Benefit: Repeatable, exact time points |
| SSD-FBUF-032 | Benefit: No dependency on format-specific seeking |
| SSD-FBUF-033 | Benefit: Eliminates seeking latency |

### FADE - Fade Curve Application

| ID | Description |
|----|-------------|
| SSD-FADE-010 | Fade-in curve may be applied after decode passes fade-in point |
| SSD-FADE-011 | Fade-out curve may be applied when decode reaches end time |
| SSD-FADE-012 | Fade application during decode or deferred to read-time |
| SSD-FADE-020 | Problem: Playback may reach fade-out before curve applied |
| SSD-FADE-021 | Solution for fade curve timing challenge |
| SSD-FADE-022 | Early application: Apply fade-out during buffer population |
| SSD-FADE-023 | Automatic pause if fade-out point reached before application |
| SSD-FADE-024 | Implementation flexibility: decode-time or read-time application |
| SSD-FADE-025 | Logging: Warn if timing issue occurs |

### PBUF - Partial Buffer (Partial Buffer Strategy)

| ID | Description |
|----|-------------|
| SSD-PBUF-010 | Partial buffer strategy for queued passages after next |
| SSD-PBUF-011 | Partial decode for passages after next-to-be-played position |
| SSD-PBUF-012 | Default: 15-second buffer (configurable) |
| SSD-PBUF-013 | Purpose: Instant skip without audio dropout |
| SSD-PBUF-014 | Sufficient time to fully decode before buffer starvation |
| SSD-PBUF-015 | Prevents audio glitches during transitions |
| SSD-PBUF-020 | Partial buffer playback handling |
| SSD-PBUF-021 | Case: Decoding in process when buffer starts playing |
| SSD-PBUF-022 | Continue decode to complete buffer |
| SSD-PBUF-023 | Playback proceeds while decode completes in background |
| SSD-PBUF-024 | Case: Decoding not in process when buffer starts playing |
| SSD-PBUF-025 | Create new buffer by restarting decode |
| SSD-PBUF-026 | Seamlessly switch to complete buffer at same sample point |

### UND - Underrun Handling (Buffer Underrun Detection and Recovery)

| ID | Description |
|----|-------------|
| SSD-UND-010 | Buffer underrun: Playback reaches unbuffered region |
| SSD-UND-011 | Log warning on buffer underrun |
| SSD-UND-012 | Log: Current buffer status |
| SSD-UND-013 | Log: Recent skip activity |
| SSD-UND-014 | Log: Decoding speed relative to playback speed |
| SSD-UND-015 | Log: Note about adaptive pre-buffering estimation |
| SSD-UND-016 | Pause playback until 1+ second buffer available |
| SSD-UND-017 | Pause implementation: Flatline output at pause point |
| SSD-UND-018 | Automatically resume when sufficient buffer available |
| SSD-UND-020 | Fade-out timing: Playback reaches fade-out before application |
| SSD-UND-021 | Log warning on fade-out timing issue |
| SSD-UND-022 | Log: Current buffer status |
| SSD-UND-023 | Log: Recent skip activity |
| SSD-UND-024 | Log: Decoding speed relative to playback speed |
| SSD-UND-025 | Pause until fade-out curve completely applied |
| SSD-UND-026 | Automatically resume once fade-out applied |
| SSD-UND-030 | Challenge: Skip-ahead faster than decoder can populate |
| SSD-UND-031 | Solution for buffer underrun challenge |
| SSD-UND-032 | Partial buffer playback while decode continues |
| SSD-UND-033 | Automatic pause/resume with flatline output |
| SSD-UND-034 | Logging and diagnostics for underrun events |
| SSD-UND-035 | Adaptive pre-buffering based on passage start time |
| SSD-UND-036 | Seamless buffer switching at same sample point |

### CLIP - Clipping Detection (Crossfade Summation)

| ID | Description |
|----|-------------|
| SSD-CLIP-010 | Crossfade summation of two passage buffers |
| SSD-CLIP-020 | Clipping detection when summation exceeds ±1.0 |
| SSD-CLIP-021 | Log warning on clipping detection |
| SSD-CLIP-022 | Log: Playback point in both passages |
| SSD-CLIP-023 | Log: Fade durations of both passages |
| SSD-CLIP-024 | Log: Fade curves of both passages |
| SSD-CLIP-025 | Clipping prevention: Apply gain reduction or limiting |
| SSD-CLIP-026 | Warning frequency: Maximum one warning per crossover |

### RBUF - Ring Buffer Underrun Classification

| ID | Description |
|----|-------------|
| SSD-RBUF-010 | Context-aware logging to distinguish expected vs concerning underruns |
| SSD-RBUF-011 | Expected underruns: Startup, stabilization, paused state, empty queue (TRACE level) |
| SSD-RBUF-012 | Concerning underruns: Active playback with queue past grace period (WARN level) |
| SSD-RBUF-013 | Context tracking using lock-free atomics (buffer_has_been_filled, buffer_filled_timestamp_ms, audio_expected) |
| SSD-RBUF-014 | Grace period: Configurable via database setting (default 2s, range 0-10s) |
| SSD-RBUF-015 | Log frequency: Every 1000th underrun with cumulative total |

---

---

**Document Version:** 2.0
**Created:** 2025-10-16
**Last Updated:** 2025-10-25
**Status:** Supplementary - See SPEC016 for Authoritative Design
**Note:** SPEC016 Decoder Buffer Design is the authoritative specification. This document provides design rationale and historical context.
**Related:** [SPEC016-decoder_buffer_design.md](SPEC016-decoder_buffer_design.md) (authoritative), [SPEC001-architecture.md](SPEC001-architecture.md), [SPEC013-single_stream_playback.md](SPEC013-single_stream_playback.md)

**Change Log:**
- v2.0 (2025-10-25): Harmonized with SPEC016 - Removed Contradictions and Applied DRY Principle
  - **BREAKING:** This document is now supplementary to SPEC016 (authoritative)
  - Removed obsolete 2-thread parallel decoder pool design (superseded by SPEC016 serial decode [DBD-DEC-040])
  - Removed redundant buffer management details (defer to SPEC016 [DBD-BUF-xxx])
  - Removed redundant fade application details (defer to SPEC016 [DBD-FADE-xxx] and SPEC002 [XFD-ORTH-xxx])
  - Removed redundant mixer operation details (defer to SPEC016 [DBD-MIX-xxx])
  - Replaced detailed specifications with cross-references to authoritative sources
  - Applied DRY principle throughout: removed content duplicated in SPEC016, SPEC002, SPEC017
  - Added prominent notice directing readers to SPEC016 for implementation details
  - Addresses specification gap: SPEC014 vs SPEC016 contradiction (identified in requirements review analysis)
  - Document now serves as design rationale and historical context, not implementation specification
- v1.4 (2025-10-18): Added Ring Buffer Underrun Logging Classification
- v1.3 (2025-10-17): Architectural decision specifications from wkmp-ap design review
- v1.2 (2025-10-17): Added requirement traceability IDs
- v1.1 (2025-10-17): Enhanced buffer management specifications
- v1.0 (2025-10-16): Initial version
