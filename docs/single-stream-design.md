# Single Stream Design for Audio Playback with Crossfading

## Overview

The Single Stream architecture uses manual buffer management and direct audio mixing to achieve continuous playback with seamless crossfading between passages. Unlike the dual pipeline approach, this design decodes audio into memory buffers and performs sample-accurate mixing in application code before sending to the audio device.

## Motivation

This design addresses key limitations of the dual pipeline approach:
- **Sample-accurate crossfading** - Mix at buffer level rather than volume property updates
- **Lower memory footprint** - Single output stream with shorter decode-ahead buffers
- **Predictable behavior** - No framework state machine complexity
- **Cross-platform simplicity** - Pure Rust, single static binary
- **Precise timing control** - Direct control over fade curves and timing

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────────┐
│                    Audio Playback System                         │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │              Decoder Thread Pool                        │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │    │
│  │  │  Decoder 1   │  │  Decoder 2   │  │  Decoder 3   │ │    │
│  │  │  (Passage A) │  │  (Passage B) │  │  (Passage C) │ │    │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘ │    │
│  └─────────┼──────────────────┼──────────────────┼─────────┘    │
│            │                  │                  │               │
│            └──────────────────┴──────────────────┘               │
│                               ↓                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │           Passage Buffer Manager                        │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │    │
│  │  │  Passage A   │  │  Passage B   │  │  Passage C   │ │    │
│  │  │  PCM Buffer  │  │  PCM Buffer  │  │  PCM Buffer  │ │    │
│  │  │  (15 sec)    │  │  (15 sec)    │  │  (15 sec)    │ │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘ │    │
│  └─────────────────────────┬──────────────────────────────┘    │
│                            │                                     │
│                            ↓                                     │
│  ┌────────────────────────────────────────────────────────┐    │
│  │              Crossfade Mixer                            │    │
│  │  • Applies fade-in/fade-out curves                      │    │
│  │  • Sums overlapping passages                            │    │
│  │  • Sample-accurate timing                               │    │
│  │  • Outputs single stereo stream                         │    │
│  └─────────────────────────┬──────────────────────────────┘    │
│                            │                                     │
│                            ↓                                     │
│  ┌────────────────────────────────────────────────────────┐    │
│  │              Audio Output Thread                        │    │
│  │  • Ring buffer for audio device                         │    │
│  │  • Clock-driven playback                                │    │
│  │  • Uses 'cpal' for cross-platform output               │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### Component Structure

#### 1. Decoder Thread Pool

**Purpose:** Decode audio files into raw PCM format in parallel.

**Rust Structure:**
```rust
pub struct DecoderPool {
    workers: Vec<DecoderWorker>,
    work_queue: Arc<RwLock<VecDeque<DecodeRequest>>>,
    buffer_manager: Arc<PassageBufferManager>,
}

struct DecodeRequest {
    passage_id: Uuid,
    file_path: PathBuf,
    start_sample: u64,
    end_sample: u64,
    priority: DecodePriority,
}

enum DecodePriority {
    Immediate, // Currently playing
    Next,      // Next in queue
    Prefetch,  // Future passages
}
```

**Decoding Flow:**

The decoder uses a **decode-from-start-and-skip** approach for reliable, sample-accurate positioning:

1. Receive decode request from buffer manager (with passage start/end times)
2. Open file using `symphonia` decoder
3. **Always decode from the beginning of the audio file** (never use compressed seek)
4. Skip samples until reaching passage start time
5. Continue decoding and buffering until passage end time
6. Resample to standard rate (44.1kHz) if needed using `rubato`
7. Write PCM data to passage buffer
8. Notify buffer manager of completion

**Rationale for decode-and-skip:**
- Compressed file seeking (e.g., MP3, AAC) is unreliable and format-dependent
- Variable bitrate files have unpredictable seek performance
- Decode-from-start ensures exact, repeatable time points
- Provides sample-accurate positioning required for crossfading
- Trade-off: Slightly longer decode time, but guarantees correctness

**Dependencies:**
- `symphonia` - Pure Rust audio decoding (MP3, FLAC, AAC, Vorbis, etc.)
- `rubato` - Pure Rust resampling library

#### 2. Passage Buffer Manager

**Purpose:** Manage PCM buffers for queued passages, coordinate decoding.

**Rust Structure:**
```rust
pub struct PassageBufferManager {
    passages: Arc<RwLock<HashMap<Uuid, PassageBuffer>>>,
    decoder_pool: Arc<DecoderPool>,
    buffer_duration: Duration, // Default: 15 seconds
}

pub struct PassageBuffer {
    passage_id: Uuid,
    pcm_data: Vec<f32>, // Interleaved stereo: [L, R, L, R, ...]
    sample_rate: u32,
    channels: u16,
    status: BufferStatus,
    fade_in_curve: FadeCurve,
    fade_out_curve: FadeCurve,
    fade_in_samples: u64,
    fade_out_samples: u64,
}

enum BufferStatus {
    Decoding,
    Ready,
    Playing,
    Exhausted,
}

pub enum FadeCurve {
    Linear,
    Logarithmic,
    Exponential,
    SCurve,
}
```

**Buffer Management Strategy:**

The system uses two distinct buffering strategies based on passage playback state:

**1. Currently Playing Passage - Full Decode Strategy:**
- When a passage enters "currently playing" status, the ENTIRE passage is decoded into RAM
- Purpose: Enable instant, sample-accurate seeking anywhere within the passage
- Rationale: Compressed file decoder seek-to-time performance is unreliable across formats
- Implementation: Always decode from file start, skip samples until passage start time, continue until passage end time
- Benefits:
  - Sample-accurate positioning at any point in the passage
  - Repeatable, exact time points within the audio file
  - No dependency on format-specific seeking capabilities
  - Eliminates seeking latency during playback

**2. Queued Passages - 15-Second Buffer Strategy (Configurable):**
- The 15-second buffer applies to LATER passages in the queue (not currently playing)
- Purpose: Facilitate instant skip to queued passages without audio dropout
- Gives sufficient time to fully decode the entire passage before playback buffer starvation
- Prevents audio glitches during passage transitions
- Configurable buffer size allows tuning for different use cases

**Additional Buffer Management:**
- **Background decoding** for next 2-3 passages in queue
- **On-demand decoding** for skip targets
- **Buffer recycling** - reuse memory after passage completes

**Memory Calculation:**
```
Queued passage (15-second buffer): 44100 Hz * 2 channels * 4 bytes/sample * 15 sec = ~5.3 MB
Currently playing passage (full): Varies by passage duration (e.g., 3 minutes = ~63 MB)
For 5 passages (1 playing, 4 queued): ~63 MB + (4 × 5.3 MB) = ~84 MB typical
```

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

**Fade Curve Examples:**
```
Linear:       y = x
Logarithmic:  y = ln(100x + 1) / ln(101)  [gradual start, faster end]
Exponential:  y = x²                       [faster start, gradual end]
S-Curve:      y = (1 - cos(πx)) / 2       [smooth acceleration/deceleration]
```

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
Ring buffer: 2048 samples * 2 channels * 2 buffers = 8192 samples (~185ms @ 44.1kHz)
Mix buffer: 2048 samples * 2 channels = 4096 samples (~46ms @ 44.1kHz)
```

## Sample-Accurate Mixing Architecture

### Overview

The system achieves sample-accurate crossfading by operating at the buffer level rather than using wall-clock timing. This ensures precise audio alignment regardless of CPU scheduling or system load.

### Key Principles

**1. Buffer Position-Based Triggering**
- Crossfades are triggered by buffer position (sample count), not time delays
- Eliminates variable latency from CPU scheduling
- Achieves ~10ms precision (467 samples at 44.1kHz) in testing
- No dependency on tokio::sleep() or wall-clock timing

**2. Pre-Calculated Mixing**
- All fade calculations happen before audio callback
- Audio output thread performs simple buffer copy
- Reduces risk of audio underruns and glitches
- Real-time audio thread remains deterministic

**3. Independent Position Tracking**
- Each passage maintains its own buffer position counter
- Current passage position: tracks playback in active passage buffer
- Next passage position: tracks crossfade progress in queued passage buffer
- Positions reset when passages transition
- Prevents buffer read errors during crossfades

### Complete Data Flow

The following six-step process describes the end-to-end buffer flow from enqueue to audio output:

#### Step 1: Decode Initiation

Audio file decode must start with sufficient lead time to ensure passage buffers are ready before needed.

**For first passage:** Decode on demand when playback starts
**For queued passages:** Prefetch during previous passage playback

**Implementation:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/engine.rs:237-303`

```rust
// When user enqueues a passage:
// 1. Create queue entry with UUID
// 2. Convert timing from ms to samples (44.1kHz)
// 3. Create DecodeRequest with start/end samples
// 4. Submit to decoder pool with priority
```

#### Step 2: Passage Buffer Population

Decoder reads compressed audio file and populates PCM buffer.

**Decode-and-Skip Approach:** For sample-accurate positioning
- Always decode from beginning of file (never use compressed seek)
- Skip samples until passage start time
- Continue decoding until passage end time
- Ensures exact, repeatable time points

**Buffering Strategy:**

1. **Currently Playing Passage - Full Decode:**
   - ENTIRE passage decoded into RAM when entering "currently playing" status
   - Enables instant, sample-accurate seeking anywhere within passage
   - Memory: ~63 MB for 3-minute passage @ 44.1kHz stereo
   - Eliminates dependency on unreliable compressed file seeking

2. **Queued Passages - 15-Second Buffer (Configurable):**
   - Only first 15 seconds buffered for queued (not currently playing) passages
   - Provides instant skip capability
   - Sufficient time to complete full decode before playback starts
   - Memory: ~5.3 MB per passage @ 44.1kHz stereo

**Buffer Contents:**
- PCM data (f32 stereo, interleaved: [L, R, L, R, ...])
- Fade parameters (curve type, duration in samples)
- Sample count and status flags

**Implementation:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/single_stream/decoder.rs`

#### Step 3: Crossfade Trigger Calculation

When next passage is queued, calculate the exact sample position to start crossfade.

**Trigger Calculation:**
```
trigger_sample = passage_duration_samples - overlap_samples
```

**Example:**
- Passage duration: 20 seconds = 882,765 samples @ 44.1kHz
- Overlap: 8 seconds = 352,800 samples
- Trigger: 882,765 - 352,800 = 529,965 samples

**Storage:** Stored in mixer's `crossfade_start_sample` field

**Implementation:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/single_stream/mixer.rs:223-254`

```rust
// In queue_next_passage():
let passage_duration_samples = buffer.sample_count();
let overlap_samples = (overlap_ms * STANDARD_SAMPLE_RATE as f64 / 1000.0) as u64;
let start_sample = passage_duration_samples.saturating_sub(overlap_samples);
*self.crossfade_start_sample.write().await = Some(start_sample);
```

#### Step 4: Sample-Accurate Crossfade Triggering

Mixer's `process_audio()` checks current buffer position on each call.

**Auto-Trigger Logic:**
```rust
if current_passage_position >= crossfade_start_sample {
    // Auto-start crossfade at exact sample position
    self.start_crossfade().await
}
```

**Performance:**
- Achieved ~10ms latency in testing (467 samples at 44.1kHz)
- No wall-clock timing or sleep() calls
- Deterministic and repeatable

**Implementation:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/single_stream/mixer.rs:273-292`

#### Step 5: Playout Buffer Creation

Mixer reads from both passage buffers simultaneously and creates mixed output.

**Per-Frame Processing:**

For each audio frame (stereo sample pair):

1. **Calculate fade gains** using configured curves:
   - Fade-out gain: `calculate_fade_gain(curve, progress, false)`
   - Fade-in gain: `calculate_fade_gain(curve, progress, true)`
   - Progress: `current_sample / total_crossfade_samples`

2. **Read samples** from both passages:
   - Current passage: Read at `current_passage_position`
   - Next passage: Read at `next_passage_position`
   - Independent position tracking prevents buffer errors

3. **Apply fade gains:**
   - `current_sample * fade_out_gain`
   - `next_sample * fade_in_gain`

4. **Sum overlapping values:**
   - `output = (current * fade_out_gain) + (next * fade_in_gain)`

5. **Clamp to prevent clipping:**
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
   └─> rubato resamples to 44.1kHz if needed
   └─> PCM data written to PassageBuffer
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
- At 44.1kHz, each sample = 0.0227 ms
- Crossfade timing precise to ~0.02 ms
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

**Document Version:** 1.0
**Created:** 2025-10-16
**Status:** Current Architecture (Selected for Implementation)
**Note:** This single-stream architecture has been selected as the current implementation approach. See [architecture.md](architecture.md) for integration details.
**Related:** `dual-pipeline-design.md` (archived), `architecture-comparison.md`, `single-stream-playback.md`
