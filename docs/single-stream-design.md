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
1. Receive decode request from buffer manager
2. Open file using `symphonia` decoder
3. Seek to start position (skip unwanted samples)
4. Decode compressed audio into PCM samples
5. Resample to standard rate (44.1kHz) if needed using `rubato`
6. Write PCM data to passage buffer
7. Notify buffer manager of completion

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
- **15-second buffers** for all queued passages (configurable)
- **Immediate decoding** for currently playing passage
- **Background decoding** for next 2-3 passages in queue
- **On-demand decoding** for skip targets
- **Buffer recycling** - reuse memory after passage completes

**Memory Calculation:**
```
Per passage: 44100 Hz * 2 channels * 4 bytes/sample * 15 sec = ~5.3 MB
For 5 passages: ~26.5 MB total
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

## Data Flow

### Complete Playback Sequence

```
1. User enqueues passage
   └─> QueueManager adds entry to database
   └─> BufferManager triggers decode request
   └─> DecoderPool starts decoding passage

2. Decoder processes file
   └─> symphonia opens and decodes file
   └─> rubato resamples to 44.1kHz if needed
   └─> PCM data written to PassageBuffer
   └─> BufferManager marks passage as Ready

3. Playback starts
   └─> CrossfadeMixer selects current passage
   └─> MixerThread continuously fills output buffer
   └─> AudioOutput pulls from ring buffer

4. Approaching crossfade point
   └─> CrossfadeMixer detects fade_out_point reached
   └─> Loads next passage from BufferManager
   └─> Begins sample-accurate mixing

5. During crossfade
   └─> For each output sample:
       - Read current passage sample, apply fade-out curve
       - Read next passage sample, apply fade-in curve
       - Sum weighted samples
       - Write to ring buffer

6. After crossfade complete
   └─> Current passage marked as Exhausted
   └─> Next passage becomes current
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
**Problem:** Seeking in compressed formats can be slow.
**Solution:**
- Buffer ahead to minimize seek operations
- Use format-specific seek optimization (MP3 frame seeking)
- Show loading indicator for slow seeks

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
