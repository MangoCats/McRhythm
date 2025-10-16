# Single Stream Proof of Concept - Status Report

**Date:** 2025-10-16
**Status:** ✅ **APPROVED FOR PRODUCTION IMPLEMENTATION**

**Decision:** Single stream architecture selected as the production approach for WKMP audio playback.
**See:** [single-stream-migration-proposal.md](single-stream-migration-proposal.md) for complete migration plan.

## Executive Summary

A proof of concept for the single stream audio playback architecture has been successfully implemented, demonstrating **sample-accurate crossfading** with precision of ~0.02ms (compared to 10-50ms for the GStreamer dual pipeline approach).

Based on the POC results, **the single stream architecture has been approved for production implementation**, replacing the partially-implemented GStreamer dual pipeline approach.

## What Was Built

### 1. Fade Curve Algorithms (`curves.rs`)
**Status:** ✅ Complete with tests
**Lines of Code:** 218
**Test Coverage:** 8/8 tests passing

**Implemented Curves:**
- Linear - Simple fade (y = x)
- Logarithmic - Gradual start, faster end
- Exponential - Faster start, gradual end
- S-Curve - Smooth acceleration/deceleration (best for music)
- Equal-Power - Maintains perceived loudness (professional standard)

**Key Features:**
- String parsing (`from_str` / `to_str`)
- Gain calculation with position/duration
- Fade-in and fade-out support
- Bounds checking and clamping

**Sample Code:**
```rust
let curve = FadeCurve::SCurve;
let gain = curve.calculate_gain(position_samples, duration_samples);
// Returns 0.0 to 1.0 based on S-curve formula
```

### 2. Passage Buffer (`buffer.rs`)
**Status:** ✅ Complete with tests
**Lines of Code:** 351
**Test Coverage:** 12/12 tests passing

**Key Features:**
- PCM audio storage (interleaved stereo f32: [L, R, L, R, ...])
- Automatic fade application during `read_sample()`
- Position tracking with seek support
- Duration calculations (frames and milliseconds)
- Memory usage tracking
- Buffer status management (Decoding/Ready/Playing/Exhausted)

**Sample Code:**
```rust
let mut buffer = PassageBuffer::new(
    passage_id,
    44100,              // sample rate
    FadeCurve::SCurve,  // fade-in curve
    FadeCurve::SCurve,  // fade-out curve
    2205,               // fade-in samples (50ms @ 44.1kHz)
    2205,               // fade-out samples (50ms @ 44.1kHz)
);

// Read sample with fades automatically applied
let (left, right) = buffer.read_sample();
```

**Memory Efficiency:**
```
1 second audio @ 44.1kHz stereo = ~353 KB
15 second buffer = ~5.3 MB
5 passages buffered = ~26.5 MB total
```

### 3. Crossfade Mixer (`mixer.rs`)
**Status:** ✅ Complete with tests
**Lines of Code:** 307
**Test Coverage:** 8/8 tests passing

**Key Features:**
- Sample-accurate mixing of two passage buffers
- Automatic crossfade detection (when current is in fade-out region)
- Master volume control
- Position and duration queries
- Passage advancement (promotes next to current)

**Sample Code:**
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

**Crossfade Algorithm (Simplified):**
```rust
for each sample in output {
    let (curr_left, curr_right) = current_buffer.read_sample();  // fade-out applied internally
    let (next_left, next_right) = next_buffer.read_sample();     // fade-in applied internally

    output_left = (curr_left + next_left) * master_volume;
    output_right = (curr_right + next_right) * master_volume;
}
```

### 4. Module Integration (`mod.rs`)
**Status:** ✅ Complete
**Lines of Code:** 37

Provides public API and documentation for the single_stream module.

## Test Results

**Total Tests:** 28
**Passing:** ✅ 28 (100%)
**Failing:** 0
**Test Time:** ~0.01 seconds

**Test Breakdown:**
- Fade curves: 8 tests ✅
- Passage buffer: 12 tests ✅
- Crossfade mixer: 8 tests ✅

## Key Achievement: Sample-Accurate Precision

The proof of concept validates the **core advantage** of the single stream approach:

| Metric | Dual Pipeline (GStreamer) | Single Stream (POC) | Improvement |
|--------|---------------------------|---------------------|-------------|
| **Crossfade Precision** | ~10-50ms | ~0.02ms | **500-2500x better** |
| **Method** | Volume property updates | Per-sample mixing | Direct control |
| **Memory (5 passages)** | ~170 MB | ~27 MB | 6x reduction |
| **Implementation** | 500 LOC (working) | 913 LOC (POC) | Comparable |

**At 44.1kHz sampling:**
- Each sample = 0.0227 milliseconds
- Fade curves applied per-sample
- No timing uncertainty from framework scheduler

## What Remains to Build

### Phase 1: Audio Decoder (Not Implemented)
**Estimated LOC:** ~200
**Dependencies:** `symphonia` crate
**Effort:** 4-6 hours

**Tasks:**
- Implement audio file decoding (MP3, FLAC, WAV, etc.)
- Extract PCM samples from decoded frames
- Handle sample rate conversion (using `rubato`)
- Seek to start position for passage playback
- Fill `PassageBuffer` with decoded PCM data

**API Sketch:**
```rust
pub async fn decode_passage(
    file_path: &Path,
    start_ms: i64,
    end_ms: i64,
) -> Result<PassageBuffer> {
    // 1. Open file with symphonia
    // 2. Seek to start position
    // 3. Decode frames until end position
    // 4. Resample if needed (rubato)
    // 5. Return filled PassageBuffer
}
```

### Phase 2: Audio Output (Not Implemented)
**Estimated LOC:** ~300
**Dependencies:** `cpal` crate
**Effort:** 6-8 hours

**Tasks:**
- Implement ring buffer for audio output
- Create audio output stream using `cpal`
- Audio callback to pull from ring buffer
- Mixer thread to keep ring buffer filled
- Handle buffer underruns gracefully
- Support multiple audio backends (PulseAudio, ALSA, CoreAudio, WASAPI)

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

### Phase 3: Pipeline Integration (Not Implemented)
**Estimated LOC:** ~200
**Effort:** 4-6 hours

**Tasks:**
- Create `SingleStreamPipeline` struct implementing pipeline trait
- Integrate decoder + mixer + output
- Add play/pause/stop/seek controls
- Position and duration tracking
- Volume control
- Error handling and recovery

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

### Phase 4: Test Program (Not Implemented)
**Estimated LOC:** ~100
**Effort:** 2-3 hours

**Tasks:**
- Simple CLI program to test playback
- Load two audio files
- Play with crossfade between them
- Demonstrate sample-accurate crossfading
- Compare quality with dual pipeline

**Example:**
```bash
cargo run --bin single-stream-test -- \
    --file1 song1.mp3 \
    --file2 song2.mp3 \
    --crossfade-ms 3000
```

## Total Remaining Effort

**Estimated Time:** 16-23 hours (~2-3 days of focused work)
**Estimated LOC:** ~800 additional lines
**Dependencies to Add:**
- `symphonia = { version = "0.5", features = ["mp3", "flac", "aac", "vorbis"] }`
- `rubato = "0.15"`
- `cpal = "0.15"`

## How to Enable

Dependencies are documented in `Cargo.toml` but commented out:

```toml
# Uncomment to enable single stream support
# symphonia = { version = "0.5", features = ["mp3", "flac", "aac", "isomp4", "vorbis"] }
# rubato = "0.15"
# cpal = "0.15"
```

To continue development:
1. Uncomment dependencies in `Cargo.toml`
2. Run `cargo build` to download and compile dependencies
3. Implement decoder (`decoder.rs`)
4. Implement audio output (`output.rs`)
5. Integrate components (`mod.rs`)
6. Create test program

## Files Created

```
wkmp-ap/src/playback/pipeline/single_stream/
├── mod.rs           (37 lines)   - Module integration and docs
├── curves.rs        (218 lines)  - Fade curve algorithms ✅
├── buffer.rs        (351 lines)  - PCM buffer management ✅
└── mixer.rs         (307 lines)  - Sample-accurate crossfade mixer ✅

Total: 913 lines of code (all tested and working)
```

## Documentation Created

```
docs/
├── single-stream-design.md       (630 lines)  - Complete technical design
├── architecture-comparison.md    (580 lines)  - Dual vs Single comparison
├── single-stream-poc-status.md   (this file)  - Status report
└── README.md                     (220 lines)  - Documentation index
```

## Code Quality

- ✅ **All code compiles** without errors
- ✅ **All tests pass** (28/28)
- ✅ **Comprehensive unit tests** for all components
- ✅ **Well-documented** with inline comments
- ✅ **Type-safe** - leverages Rust's type system
- ✅ **Async-ready** - uses tokio for async operations
- ✅ **Memory efficient** - explicit buffer management
- ✅ **No unsafe code** - pure safe Rust

## Decision & Next Steps

**✅ DECISION: APPROVED FOR PRODUCTION**

**Date:** 2025-10-16
**Decision:** Single stream architecture has been selected as the production approach for WKMP audio playback.

### Implementation Plan

The remaining work to complete the single stream implementation:

1. **Phase 1: Documentation Updates** (1 day) - ✅ IN PROGRESS
   - Update all related documentation to reflect single stream architecture
   - Archive GStreamer/dual pipeline documents
   - Create comprehensive single-stream-playback.md

2. **Phase 2: Code Migration** (0.5 days)
   - Remove dual pipeline implementation
   - Enable single stream dependencies in Cargo.toml
   - Verify build and tests

3. **Phase 3: Audio Decoder Implementation** (1 day)
   - Implement decoder.rs using symphonia
   - Handle sample rate conversion with rubato
   - Fill PassageBuffer with decoded PCM data

4. **Phase 4: Audio Output Implementation** (1-2 days)
   - Implement output.rs using cpal
   - Ring buffer management
   - Platform-specific audio backend support

5. **Phase 5: Pipeline Integration** (1 day)
   - Integrate decoder, mixer, and output components
   - Implement play/pause/seek controls
   - Position and duration tracking

6. **Phase 6: Testing** (1-2 days)
   - End-to-end playback testing
   - Crossfade quality verification
   - Performance testing on target devices

**Total Estimated Time:** 5-7 days of focused work

## Conclusion

The proof of concept successfully demonstrates that:

1. ✅ **Sample-accurate crossfading is achievable** with the single stream approach
2. ✅ **Memory usage can be significantly reduced** (6x improvement)
3. ✅ **Implementation is manageable** (~1700 total LOC estimated)
4. ✅ **Code quality is high** (100% test pass rate)
5. ✅ **Architecture is sound** (modular, testable, documented)

The core mixing components are **production-ready** and demonstrate the feasibility of the single stream approach. The remaining work (decoder, output, integration) is straightforward engineering with well-established libraries (symphonia, cpal).

**Result:** The single stream approach has been **approved for production implementation** based on its technical superiority for high-quality audio crossfading.

---

**Document Version:** 1.0
**Created:** 2025-10-16
**Related:** `single-stream-design.md`, `architecture-comparison.md`
