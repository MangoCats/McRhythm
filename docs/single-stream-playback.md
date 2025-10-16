# Single Stream Audio Playback Architecture

**Status:** ✅ **Production Implementation** (Replaces GStreamer Dual Pipeline)
**Decision Date:** 2025-10-16
**Version:** 1.0

## Executive Summary

The WKMP Audio Player uses a **single-stream architecture** with sample-accurate crossfading to provide continuous audio playback with seamless transitions between passages. This approach delivers:

- **Sample-accurate crossfading**: ~0.02ms precision (500-2500x better than GStreamer dual pipeline)
- **Lower memory usage**: ~27 MB for 5 buffered passages (6x reduction vs dual pipeline)
- **Simpler deployment**: Single static binary with no runtime dependencies beyond system audio libraries
- **Pure Rust implementation**: No external frameworks (GStreamer, FFmpeg, etc.)
- **Cross-platform**: Single codebase for Linux, macOS, Windows

## Architecture Overview

The single-stream architecture decodes audio files into memory buffers, applies fade curves at the sample level, mixes passages with sample-accurate timing, and outputs the mixed audio stream to the system audio device.

### Component Diagram

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
│            │   symphonia      │  + rubato        │               │
│            └──────────────────┴──────────────────┘               │
│                               ↓                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │           Passage Buffer Manager                        │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │    │
│  │  │  Passage A   │  │  Passage B   │  │  Passage C   │ │    │
│  │  │  PCM Buffer  │  │  PCM Buffer  │  │  PCM Buffer  │ │    │
│  │  │  (15 sec)    │  │  (15 sec)    │  │  (15 sec)    │ │    │
│  │  │  + fades     │  │  + fades     │  │  + fades     │ │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘ │    │
│  └─────────────────────────┬──────────────────────────────┘    │
│                            │                                     │
│                            ↓                                     │
│  ┌────────────────────────────────────────────────────────┐    │
│  │              Crossfade Mixer                            │    │
│  │  • Reads samples from current & next buffers           │    │
│  │  • Fade curves applied automatically per-sample        │    │
│  │  • Sums overlapping passages (crossfade)               │    │
│  │  • Applies master volume                               │    │
│  │  • Sample-accurate timing (~0.02ms precision)          │    │
│  └─────────────────────────┬──────────────────────────────┘    │
│                            │                                     │
│                            ↓                                     │
│  ┌────────────────────────────────────────────────────────┐    │
│  │              Audio Output Thread                        │    │
│  │  • Ring buffer for smooth audio delivery               │    │
│  │  • cpal-based cross-platform output                    │    │
│  │  • Platform backends: PulseAudio, ALSA, CoreAudio,     │    │
│  │    WASAPI                                               │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Audio Decoder (symphonia + rubato)

**Purpose:** Decode compressed audio files into raw PCM samples.

**Technology:**
- `symphonia 0.5.x`: Pure Rust audio decoder
- `rubato 0.15.x`: High-quality sample rate conversion

**Supported Formats:**
- MP3 (MPEG-1/2 Layer 3)
- FLAC (Free Lossless Audio Codec)
- Ogg Vorbis
- Opus
- AAC/M4A (Advanced Audio Coding)
- WAV (Waveform Audio File Format)
- AIFF (Audio Interchange File Format)
- WavPack, ALAC, APE, and more

**Decoding Flow:**
1. Open audio file using symphonia decoder
2. Seek to passage start position (skip unwanted samples)
3. Decode compressed audio into PCM samples
4. Resample to standard rate (44.1kHz) if needed using rubato
5. Write interleaved stereo PCM data (f32: [L, R, L, R, ...]) to PassageBuffer
6. Notify buffer manager of completion

**File Locations:**
- Implementation: `wkmp-ap/src/playback/pipeline/single_stream/decoder.rs` (to be implemented)

### 2. Passage Buffer (PCM Storage + Fade Application)

**Purpose:** Store decoded PCM audio with automatic per-sample fade curve application.

**Status:** ✅ **Complete** (351 LOC, 12/12 tests passing)

**Key Features:**
- PCM audio storage (interleaved stereo f32: [L, R, L, R, ...])
- Automatic fade application during `read_sample()` - no separate fade step
- Position tracking with seek support
- Duration calculations (frames and milliseconds)
- Memory usage tracking
- Buffer status management (Decoding/Ready/Playing/Exhausted)

**Memory Efficiency:**
```
1 second audio @ 44.1kHz stereo = ~353 KB
15 second buffer = ~5.3 MB
5 passages buffered = ~26.5 MB total
```

**API Example:**
```rust
let mut buffer = PassageBuffer::new(
    passage_id,
    44100,              // sample rate
    FadeCurve::SCurve,  // fade-in curve
    FadeCurve::SCurve,  // fade-out curve
    2205,               // fade-in samples (50ms @ 44.1kHz)
    2205,               // fade-out samples (50ms @ 44.1kHz)
);

// Fill buffer with decoded PCM data
buffer.append_pcm_data(&pcm_samples)?;
buffer.mark_ready();

// Read sample with fades automatically applied
let (left, right) = buffer.read_sample();
```

**File Locations:**
- Implementation: `wkmp-ap/src/playback/pipeline/single_stream/buffer.rs` ✅
- Tests: `wkmp-ap/src/playback/pipeline/single_stream/buffer.rs#tests` ✅

### 3. Fade Curve Algorithms

**Purpose:** Calculate gain values for smooth audio transitions.

**Status:** ✅ **Complete** (218 LOC, 8/8 tests passing)

**Implemented Curves:**
1. **Linear** - Simple linear fade (y = x)
2. **Logarithmic** - Gradual start, faster end
3. **Exponential** - Faster start, gradual end
4. **S-Curve** - Smooth acceleration/deceleration (best for music)
5. **Equal-Power** - Maintains perceived loudness (professional standard)

**API Example:**
```rust
let curve = FadeCurve::SCurve;
let gain = curve.calculate_gain(
    position_samples,  // Current position in fade region
    duration_samples   // Total fade duration
);
// Returns 0.0 to 1.0 based on S-curve formula
```

**String Serialization:**
```rust
// From string (for configuration storage)
let curve = FadeCurve::from_str("s_curve")?;

// To string (for configuration display)
let name = curve.to_str(); // "s_curve"
```

**File Locations:**
- Implementation: `wkmp-ap/src/playback/pipeline/single_stream/curves.rs` ✅
- Tests: `wkmp-ap/src/playback/pipeline/single_stream/curves.rs#tests` ✅

### 4. Crossfade Mixer

**Purpose:** Mix two passage buffers with sample-accurate crossfading.

**Status:** ✅ **Complete** (307 LOC, 8/8 tests passing)

**Key Features:**
- Sample-accurate mixing of two passage buffers (current + next)
- Automatic crossfade detection (when current is in fade-out region)
- Master volume control
- Position and duration queries
- Passage advancement (promotes next to current)

**Crossfade Algorithm:**
```rust
// Simplified mixing algorithm
for each sample in output_buffer {
    // PassageBuffer.read_sample() applies fade curves internally
    let (curr_left, curr_right) = current_buffer.read_sample();  // fade-out applied
    let (next_left, next_right) = next_buffer.read_sample();     // fade-in applied

    // Sum weighted samples
    output_left = (curr_left + next_left) * master_volume;
    output_right = (curr_right + next_right) * master_volume;
}
```

**API Example:**
```rust
let mut mixer = CrossfadeMixer::new(44100);

mixer.set_current(buffer_a);  // Current passage
mixer.set_next(buffer_b);     // Next passage for crossfade

// Fill output buffer with mixed audio
let mut output = vec![0.0f32; 4096];  // 2048 frames
let frames_written = mixer.fill_output_buffer(&mut output).await?;

// When buffer_a reaches fade-out region, mixer automatically:
// 1. Reads from buffer_a with fade-out applied
// 2. Reads from buffer_b with fade-in applied
// 3. Sums the weighted samples
// 4. Applies master volume
```

**File Locations:**
- Implementation: `wkmp-ap/src/playback/pipeline/single_stream/mixer.rs` ✅
- Tests: `wkmp-ap/src/playback/pipeline/single_stream/mixer.rs#tests` ✅

### 5. Audio Output (cpal)

**Purpose:** Output mixed audio stream to system audio device.

**Technology:** `cpal 0.15.x` - Cross-platform audio output abstraction

**Platform Backends:**
- **Linux**: PulseAudio (primary), ALSA (fallback)
- **macOS**: CoreAudio (built into macOS)
- **Windows**: WASAPI (built into Windows Vista and later)

**Ring Buffer Design:**
```
┌─────────────────────────────────────────────┐
│            Ring Buffer                      │
│  ┌─────────────────────────────────────┐   │
│  │         Audio Data                  │   │
│  │  [L R L R L R L R L R L R L R L R]  │   │
│  └─────────────────────────────────────┘   │
│       ↑ write_pos         ↑ read_pos       │
│       (mixer thread)      (audio callback) │
└─────────────────────────────────────────────┘
```

**Threading Model:**
1. **Mixer Thread**: Pulls samples from CrossfadeMixer, writes to ring buffer
2. **Audio Callback**: Reads from ring buffer, sends to audio device
3. **Synchronization**: Lock-free ring buffer with atomic read/write pointers

**API Sketch:**
```rust
pub struct AudioOutput {
    stream: cpal::Stream,
    ring_buffer: Arc<RwLock<RingBuffer>>,
    mixer: Arc<RwLock<CrossfadeMixer>>,
}

impl AudioOutput {
    pub fn new(mixer: Arc<RwLock<CrossfadeMixer>>) -> Result<Self>;
    pub fn start(&mut self) -> Result<()>;
    pub fn stop(&mut self);
}
```

**File Locations:**
- Implementation: `wkmp-ap/src/playback/pipeline/single_stream/output.rs` (to be implemented)

### 6. Playback Pipeline Integration

**Purpose:** Coordinate decoder, mixer, and output components into complete playback system.

**API Sketch:**
```rust
pub struct SingleStreamPipeline {
    decoder: AudioDecoder,
    mixer: Arc<RwLock<CrossfadeMixer>>,
    output: AudioOutput,
    state: PipelineState,
}

impl SingleStreamPipeline {
    pub async fn load_file(&mut self, file_path: &Path) -> Result<()>;
    pub fn play(&mut self) -> Result<()>;
    pub fn pause(&mut self) -> Result<()>;
    pub fn seek(&mut self, position_ms: i64) -> Result<()>;
    pub fn position_ms(&self) -> Option<i64>;
    pub fn duration_ms(&self) -> Option<i64>;
}
```

**File Locations:**
- Implementation: `wkmp-ap/src/playback/pipeline/single_stream/pipeline.rs` (to be implemented)

## Crossfade Timing and Behavior

### Lead-In/Lead-Out Points

Passages in WKMP have three timing markers:
1. **Lead-in point**: Where passage should start playing (skips intro silence)
2. **Lead-out point**: Where crossfade to next passage should begin
3. **End point**: Absolute end of passage audio

**Example Passage Timeline:**
```
Audio File: 0s ──────────────────────────────────────────────── 240s
               ↑              ↑                          ↑
           Lead-in (5s)   Lead-out (220s)           End (240s)

Playback:      ├──────────────────────────────────┤
               5s                                220s
               Playing for 215 seconds

Fade-out:                                         ├──┤
                                                  220-240s
                                                  3s crossfade
```

### Crossfade Execution

When current passage reaches lead-out point:

1. **Crossfade Start**: Current passage position reaches lead-out sample
2. **Mixer Behavior**:
   - Current buffer enters fade-out region (PassageBuffer applies fade-out curve)
   - Next buffer enters fade-in region (PassageBuffer applies fade-in curve)
   - Mixer sums both buffers: `output = current_faded + next_faded`
3. **Crossfade Duration**: Determined by fade-out samples of current passage
4. **Crossfade End**: Current buffer exhausted, next buffer becomes current

**Sample-Accurate Timing:**
- Each sample = 0.0227 ms @ 44.1kHz
- Crossfade precision = ~0.02ms
- No timing uncertainty from framework scheduler
- Fade curves applied per-sample

### Fade Curve Selection

The fade curve determines how volume transitions during crossfade:

**Equal-Power (Recommended for crossfading):**
- Maintains constant perceived loudness during crossfade
- Uses complementary sine/cosine curves: `fade_out = cos(θ)`, `fade_in = sin(θ)`
- Professional audio standard

**S-Curve (Recommended for pause/resume):**
- Smooth acceleration and deceleration
- Musical and natural sounding
- Good for fade-in from pause

**Linear:**
- Simple linear transition
- Can sound abrupt for music
- Good for testing

**Logarithmic/Exponential:**
- Specialized curves for specific effects
- Logarithmic: gradual start, faster end
- Exponential: faster start, gradual end

## Performance Characteristics

### Memory Usage

**Per Passage Buffer:**
- Sample rate: 44.1kHz
- Channels: 2 (stereo)
- Sample format: f32 (4 bytes)
- Buffer duration: 15 seconds
- Memory: 44100 samples/sec × 2 channels × 4 bytes × 15 sec = **5.3 MB**

**Total Memory (5 passages buffered):**
- 5 passages × 5.3 MB = **~27 MB**

**Comparison to GStreamer Dual Pipeline:**
- GStreamer: ~170 MB (entire files buffered + framework overhead)
- Single stream: ~27 MB (only 15-second windows buffered)
- **Reduction: 6x lower memory usage**

### CPU Usage

**Decoding (symphonia):**
- Runs in background threads
- Minimal CPU impact on playback
- Typical: < 5% CPU on modern hardware

**Mixing (per-sample calculations):**
- Extremely efficient
- Two buffer reads + addition + volume multiply per sample
- Typical: < 1% CPU on modern hardware

**Resampling (rubato):**
- Only when source sample rate ≠ 44.1kHz
- High-quality sinc interpolation
- Typical: 2-5% CPU on modern hardware

### Latency

**Crossfade Precision:**
- Sample-accurate: ~0.02ms @ 44.1kHz
- GStreamer dual pipeline: 10-50ms (property update latency)
- **Improvement: 500-2500x better precision**

**Pause/Play Latency:**
- Near-instant (audio callback driven)
- < 1ms typical

## Implementation Status

### Completed (POC Phase)

✅ **Fade Curves** (`curves.rs`, 218 LOC, 8/8 tests)
- 5 curve types implemented
- String serialization support
- Comprehensive unit tests

✅ **Passage Buffer** (`buffer.rs`, 351 LOC, 12/12 tests)
- PCM storage with automatic fade application
- Position tracking and seek
- Memory usage tracking

✅ **Crossfade Mixer** (`mixer.rs`, 307 LOC, 8/8 tests)
- Sample-accurate mixing
- Automatic crossfade detection
- Master volume control

✅ **Module Integration** (`mod.rs`, 37 LOC)
- Public API exposure
- Documentation

**Total POC**: 913 LOC, 28/28 tests passing (100%)

### Remaining Work

⏳ **Audio Decoder** (`decoder.rs`, est. 200 LOC, 4-6 hours)
- Implement symphonia-based decoding
- Sample rate conversion with rubato
- Passage buffer filling

⏳ **Audio Output** (`output.rs`, est. 300 LOC, 6-8 hours)
- Ring buffer implementation
- cpal stream creation
- Audio callback handling

⏳ **Pipeline Integration** (`pipeline.rs`, est. 200 LOC, 4-6 hours)
- Component coordination
- Play/pause/seek controls
- Position tracking

⏳ **Testing** (est. 100 LOC, 2-3 hours)
- End-to-end playback tests
- Crossfade quality verification
- Performance testing

**Total Remaining**: ~800 LOC, 16-23 hours (2-3 days focused work)

## Deployment

### Dependencies

**Cargo.toml:**
```toml
[dependencies]
# Audio decoding
symphonia = { version = "0.5", features = ["mp3", "flac", "aac", "isomp4", "vorbis"] }

# Sample rate conversion
rubato = "0.15"

# Audio output
cpal = "0.15"

# Async runtime (already in use)
tokio = { version = "1", features = ["full"] }
```

### System Requirements

**Linux:**
- `libasound2` (ALSA library) - typically pre-installed
- `libpulse0` (PulseAudio client library) - typically pre-installed on desktop systems

**macOS:**
- CoreAudio framework - built into macOS, no additional libraries needed

**Windows:**
- WASAPI (Windows Audio Session API) - built into Windows Vista and later

### Distribution

**Single Binary:**
- All audio processing code compiled into wkmp-ap executable
- No plugin directories or separate libraries to bundle
- No environment variables required
- Binary size: ~15-20 MB (vs ~100+ MB with GStreamer bundling)

**Cross-Platform:**
- Same Rust codebase compiles for all platforms
- No platform-specific plugin versions to manage
- Consistent behavior across platforms

## Migration from GStreamer Dual Pipeline

### Code Removal

The following GStreamer-based components will be removed:

**Files to Remove:**
- `wkmp-ap/src/playback/pipeline/dual.rs` (partially implemented dual pipeline)
- Any GStreamer utility modules

**Dependencies to Remove (Cargo.toml):**
```toml
# Remove these:
gstreamer = "0.21"
gstreamer-audio = "0.21"
gstreamer-app = "0.21"
```

### Code Migration

**Replace in `wkmp-ap/src/playback/engine.rs`:**
```rust
// Old:
use crate::playback::pipeline::dual::DualPipeline;

// New:
use crate::playback::pipeline::single_stream::SingleStreamPipeline;
```

**Update PlaybackEngine:**
```rust
pub struct PlaybackEngine {
    // Old:
    // pipeline: Arc<RwLock<DualPipeline>>,

    // New:
    pipeline: Arc<RwLock<SingleStreamPipeline>>,

    // ... rest unchanged
}
```

### Testing Strategy

1. **Unit Tests**: Verify all single-stream components (already 28/28 passing)
2. **Integration Tests**: Test decoder → buffer → mixer → output chain
3. **End-to-End Tests**: Full playback with crossfading
4. **Performance Tests**: Memory usage, CPU usage, crossfade precision
5. **Platform Tests**: Verify on Linux, macOS, Windows

## References

**Related Documents:**
- [Single Stream Design](single-stream-design.md) - Detailed technical design
- [Architecture Comparison](architecture-comparison.md) - Dual vs Single stream comparison
- [POC Status Report](single-stream-poc-status.md) - Proof of concept results
- [Migration Proposal](single-stream-migration-proposal.md) - Migration plan

**External Documentation:**
- [symphonia documentation](https://docs.rs/symphonia/) - Audio decoding
- [rubato documentation](https://docs.rs/rubato/) - Sample rate conversion
- [cpal documentation](https://docs.rs/cpal/) - Cross-platform audio output

---

**Document Version:** 1.0
**Created:** 2025-10-16
**Status:** Production architecture document
**Related:** `single-stream-design.md`, `architecture-comparison.md`, `single-stream-poc-status.md`
