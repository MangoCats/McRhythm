# Single Stream Migration Proposal

**✅ DECISION MADE: Single Stream Architecture Selected**

**Decision Date:** 2025-10-16
**Status:** Single stream architecture approved for production implementation.
**Rationale:** Sample-accurate crossfading, 6x lower memory usage, simpler architecture.
**See:** [single-stream-migration-proposal.md](../archive/ARCH004-single_stream_migration_proposal.md) for complete migration plan.

This document is preserved for historical reference to document the decision-making process.

## Executive Summary

This document proposes abandoning the GStreamer dual pipeline development path in favor of the **single stream crossfading solution** to meet all WKMP requirements. The single stream approach provides:

- **500-2500x better crossfade precision** (0.02ms vs 10-50ms)
- **6x lower memory usage** (27 MB vs 170 MB for 5 passages)
- **Simpler architecture** with direct sample-level control
- **Pure Rust implementation** removing GStreamer framework dependency
- **Cross-platform static binaries** without external runtime dependencies

## Rationale for Change

### Technical Superiority

The proof of concept successfully demonstrated that single stream provides:

1. **Sample-accurate crossfading**: Direct per-sample mixing eliminates GStreamer's property update latency
2. **Lower memory footprint**: PCM buffer management is 6x more efficient than dual pipeline overhead
3. **Simpler implementation**: 913 LOC proof of concept vs 500+ LOC dual pipeline (partial implementation)
4. **Better control**: Direct access to audio samples vs indirect through GStreamer properties

### Current State Analysis

**Dual Pipeline Status:**
- Partially implemented (~500 LOC)
- Working but imprecise crossfading (10-50ms timing uncertainty)
- Complex state management with GStreamer bins
- Platform-specific GStreamer bundling required

**Single Stream Status:**
- Core mixing components complete (913 LOC, 28/28 tests passing)
- Remaining work: decoder, audio output, integration (~800 LOC estimated)
- All tests passing, production-ready mixing code

### Decision Point

We are at an ideal point to make this transition:
- Dual pipeline is not deeply integrated into higher-level systems
- Single stream core is proven and tested
- Remaining work is straightforward (decoder/output wrappers)
- Earlier transition = less refactoring

---

## Proposed Changes to Documentation

### 1. implementation_order.md

**Changes Required:**

#### Phase 2.6: Replace Dual-Pipeline Section

**Current:** "Phase 2.6. Dual-Pipeline Crossfade Engine"

**Proposed:** "Phase 2.6. Single-Stream Crossfade Engine"

Replace content:

```markdown
- **2.6. Single-Stream Crossfade Engine:**
  - **Moderate task** - Implement single stream audio architecture with sample-accurate crossfading
  - Integrate audio decoder using `symphonia` crate
    - Support MP3, FLAC, AAC, Vorbis, and other common formats
    - Handle sample rate conversion with `rubato`
    - Decode passages to PCM buffers in memory
  - Integrate audio output using `cpal` crate
    - Ring buffer for smooth audio delivery
    - Support PulseAudio, ALSA, CoreAudio, WASAPI backends
    - Handle buffer underruns gracefully
  - Leverage existing single_stream components (already implemented):
    - Fade curve algorithms (`curves.rs`) - 5 curve types ✅
    - PCM buffer management (`buffer.rs`) - automatic fade application ✅
    - Sample-accurate crossfade mixer (`mixer.rs`) - per-sample mixing ✅
  - Implement SingleStreamPipeline integrating decoder + mixer + output
  - Calculate crossfade timing from passage lead-in/lead-out points
  - Support three fade profiles (exponential, cosine, linear) per passage
  - Handle pause (immediate stop) and resume (configurable fade-in)
  - Emit SSE events for passage transitions and crossfades

  **Advantages over GStreamer dual pipeline:**
  - Sample-accurate crossfading (~0.02ms precision vs 10-50ms for GStreamer)
  - 6x lower memory usage (~27 MB vs ~170 MB for 5 passages)
  - Direct sample-level control without framework indirection
  - Pure Rust implementation (no GStreamer bundling required)
  - Simpler architecture with fewer moving parts
```

#### Dependencies Section Update

**Replace:**
```toml
# GStreamer bindings
gstreamer = "0.22"
gstreamer-audio = "0.22"
gstreamer-app = "0.22"
```

**With:**
```toml
# Single stream audio playback
symphonia = { version = "0.5", features = ["mp3", "flac", "aac", "isomp4", "vorbis"] }
rubato = "0.15"  # Sample rate conversion
cpal = "0.15"     # Cross-platform audio output
```

#### Phase 9.2: Update Build Scripts Section

**Remove:** GStreamer bundling requirements

**Add:** Note that single stream requires no external framework dependencies

---

### 2. gstreamer_design.md

**Action:** **Archive and replace** with new document

**New Document:** `docs/single-stream-playback.md`

Create comprehensive playback architecture document covering:

1. **Overview**
   - Single stream architecture principles
   - Component interaction diagram
   - State management model

2. **Audio Decoder (Symphonia Integration)**
   - File format support (MP3, FLAC, AAC, Vorbis, etc.)
   - Sample rate conversion with rubato
   - PCM extraction and buffering
   - Seek support for passage start/end times

3. **PCM Buffer Management**
   - PassageBuffer API (already implemented)
   - Automatic fade application during read
   - Memory management and efficiency
   - Position tracking and seeking

4. **Crossfade Mixer**
   - CrossfadeMixer API (already implemented)
   - Sample-accurate mixing algorithm
   - Fade curve application
   - Volume control (per-passage and master)

5. **Audio Output (cpal Integration)**
   - Ring buffer architecture
   - Audio callback implementation
   - Platform-specific backends (PulseAudio, ALSA, CoreAudio, WASAPI)
   - Buffer underrun handling
   - Device enumeration and selection

6. **Pipeline Integration**
   - SingleStreamPipeline struct
   - Play/pause/stop/seek control
   - Position and duration queries
   - Event emission (PassageStarted, PassageCompleted, etc.)

7. **State Management**
   - Pipeline states (Idle, Loading, Playing, Paused, Crossfading)
   - State transition diagram
   - Empty queue behavior

8. **Error Handling**
   - Decode errors
   - Audio device errors
   - File access errors
   - Buffer underrun recovery

9. **Performance Optimization**
   - Pre-loading strategy
   - Memory usage targets
   - CPU usage optimization
   - Buffer sizing recommendations

**Archive:** Rename `gstreamer_design.md` to `gstreamer_design_archived.md` with note at top explaining it represents abandoned approach

---

### 3. crossfade.md

**Changes Required:** Minimal - this document is largely implementation-agnostic

**Updates:**

#### Section: Implementation Algorithm (XFD-IMPL-090 onwards)

**Add note referencing new playback document:**

```markdown
### Volume Fade Curve Formulas

**[XFD-IMPL-090]** During crossfade, each passage's volume is controlled by applying a fade curve. The fade curve maps normalized time `t` (where 0.0 = fade start, 1.0 = fade end) to volume multiplier `v` (where 0.0 = silence, 1.0 = full volume).

> **Implementation:** See [Single Stream Playback Architecture](../SPEC013-single_stream_playback.md#fade-curve-implementation) for complete details on how these formulas are applied in the single stream mixing engine.
```

#### Section: Implementation Notes (XFD-IMPL-140)

**Update reference:**

Replace:
```markdown
> Complete GStreamer implementation details in [gstreamer_design.md - Section 5: Crossfade Implementation](ARCH002-gstreamer_design.md#5-crossfade-implementation)
```

With:
```markdown
> Complete implementation details in [Single Stream Playback Architecture](../SPEC013-single_stream_playback.md#crossfade-execution)
```

---

### 4. architecture.md

**Changes Required:** Update references and technology stack

#### Section: Audio Player Module

**Update responsibilities** (lines 83-89):

Replace:
```markdown
**Responsibilities:**
- Manages dual GStreamer pipelines for seamless crossfading
- Coordinates passage transitions based on lead-in/lead-out timing
```

With:
```markdown
**Responsibilities:**
- Manages single stream audio playback with sample-accurate crossfading
- Coordinates passage transitions based on lead-in/lead-out timing
- Decodes audio using Symphonia (MP3, FLAC, AAC, Vorbis, etc.)
- Outputs audio using cpal (PulseAudio, ALSA, CoreAudio, WASAPI)
```

#### Section: Audio Player Internal Components (lines 761-767)

**Update:**

Replace:
```markdown
**Audio Player Internal Components:**
- **Queue Manager**: Maintains playback queue, handles manual additions/removals, monitors queue levels, requests refills from Program Director
- **Queue Monitor**: Calculates remaining queue time, sends `POST /selection/request` to Program Director when below thresholds (< 2 passages or < 15 minutes), throttles requests to once per 10 seconds
- **Playback Controller**: Manages dual GStreamer pipelines for crossfading, coordinates passage transitions
- **Audio Engine**: GStreamer pipeline manager with dual pipelines, audio mixer, volume control
- **Historian**: Records passage plays with timestamps, updates last-play times for cooldown calculations
```

With:
```markdown
**Audio Player Internal Components:**
- **Queue Manager**: Maintains playback queue, handles manual additions/removals, monitors queue levels, requests refills from Program Director
- **Queue Monitor**: Calculates remaining queue time, sends `POST /selection/request` to Program Director when below thresholds (< 2 passages or < 15 minutes), throttles requests to once per 10 seconds
- **Playback Controller**: Manages single stream playback with sample-accurate crossfading, coordinates passage transitions
- **Audio Engine**: Integrates audio decoder (Symphonia), crossfade mixer, and audio output (cpal) for sample-accurate mixing
- **Historian**: Records passage plays with timestamps, updates last-play times for cooldown calculations
```

#### Section: Technology Stack (lines 1186-1229)

**Update Audio Processing section:**

Replace:
```markdown
**Audio Processing (Audio Player only):**
- GStreamer 1.x
- Rust bindings: gstreamer-rs
```

With:
```markdown
**Audio Processing (Audio Player only):**
- Audio decoding: Symphonia 0.5.x (pure Rust, supports MP3, FLAC, AAC, Vorbis, etc.)
- Sample rate conversion: rubato 0.15.x (high-quality resampling)
- Audio output: cpal 0.15.x (cross-platform, supports PulseAudio, ALSA, CoreAudio, WASAPI)
- Crossfading: Custom single-stream implementation with sample-accurate mixing
```

#### Section: Platform Abstraction - Audio Output (lines 1246-1270)

**Update diagram and text:**

Replace GStreamer-specific references with:

```markdown
### Audio Output
```
┌──────────────────────┐
│  Platform Detector   │
│  (cpal runtime)      │
└──────────┬───────────┘
           │
    ┌──────┴──────┬──────────┬──────────┐
    │             │          │          │
┌───▼────┐  ┌────▼────┐ ┌───▼────┐ ┌───▼────┐
│ ALSA   │  │PulseAudio│ │CoreAudio│ │WASAPI  │
│(Linux) │  │ (Linux)  │ │ (macOS) │ │(Windows)│
└────────┘  └─────────┘ └────────┘ └────────┘
```

**Auto-detection Priority:**
- Linux: PulseAudio → ALSA (cpal automatic selection)
- Windows: WASAPI (cpal automatic selection)
- macOS: CoreAudio (cpal automatic selection)

**Manual Override:**
- User can select specific output device via `POST /audio/device`
- Device enumeration via cpal's device discovery API
- Settings persisted in database
```

---

### 5. deployment.md

**Changes Required:** (Assuming deployment.md exists)

#### GStreamer Bundling Section

**Remove entire section** about bundling GStreamer with wkmp-ap

**Replace with:**

```markdown
### Audio Library Dependencies

**wkmp-ap (Audio Player) Dependencies:**

The single stream architecture uses pure Rust libraries with no external framework dependencies:

- **Symphonia**: Audio decoding (statically linked)
- **rubato**: Sample rate conversion (statically linked)
- **cpal**: Audio output (dynamically links to system audio libraries)

**System Audio Libraries Required:**

| Platform | Required Libraries | Notes |
|----------|-------------------|-------|
| Linux | libasound2 (ALSA) OR libpulse (PulseAudio) | cpal auto-detects and uses available backend |
| macOS | AudioToolbox.framework (built-in) | No additional dependencies |
| Windows | Windows Audio Session API (built-in) | No additional dependencies |

**Installation:**

- **Linux (Debian/Ubuntu)**: `sudo apt-get install libasound2-dev` OR `sudo apt-get install libpulse-dev`
- **Linux (Fedora/RHEL)**: `sudo dnf install alsa-lib-devel` OR `sudo dnf install pulseaudio-libs-devel`
- **macOS**: No installation required
- **Windows**: No installation required

**Static Binary:** All Rust crates (symphonia, rubato) compile directly into the wkmp-ap binary. Only system audio libraries are dynamically linked.
```

---

### 6. Create New Document: single-stream-playback.md

**Action:** Create comprehensive new document

**Location:** `docs/single-stream-playback.md`

**Content:** ~1500-2000 lines covering complete single stream playback architecture

**Sections:**

1. Overview and Architecture
2. Audio Decoder (Symphonia)
3. PCM Buffer Management (existing code)
4. Crossfade Mixer (existing code)
5. Audio Output (cpal)
6. Pipeline Integration
7. State Management
8. Position and Duration Queries
9. Volume Control
10. Audio Device Enumeration
11. Error Handling
12. Performance Optimization
13. Threading Model
14. Platform-Specific Considerations

(Detailed content to be written during implementation)

---

### 7. Update dual-pipeline-design.md

**Action:** Archive this document

**Steps:**

1. Rename to `dual-pipeline-design_archived.md`
2. Add header:

```markdown
# Dual Pipeline Design (ARCHIVED)

**⚠️ ARCHIVED DOCUMENT ⚠️**

This document describes the GStreamer dual pipeline approach which was **abandoned on 2025-10-16** in favor of the single stream crossfading solution.

**Reason for change:** Single stream provides 500-2500x better crossfade precision (0.02ms vs 10-50ms), 6x lower memory usage, and simpler architecture without GStreamer framework dependency.

**Current approach:** See [Single Stream Playback Architecture](../SPEC013-single_stream_playback.md) for the implemented solution.

**Historical value:** This document is preserved for reference to understand the design alternatives that were evaluated.

---

# Original Document Content (2025-XX-XX)

[rest of original content]
```

---

### 8. Update architecture-comparison.md

**Action:** Update status to reflect decision

**Changes:**

#### Add Decision Notice at Top

```markdown
# Architecture Comparison: Dual Pipeline vs Single Stream

**📋 DECISION MADE: 2025-10-16**

**Selected Architecture:** **Single Stream**

**Rationale:** Single stream provides objectively superior crossfade quality (500-2500x better precision), significantly lower memory usage (6x reduction), and simpler implementation without external framework dependencies. The proof of concept successfully demonstrated production-ready core components with 100% test pass rate.

**Implementation Status:** Core mixing components complete and tested. Remaining work: audio decoder, audio output, and pipeline integration (~800 LOC, 2-3 days estimated).

---

# Original Comparison (For Historical Reference)

[rest of original content]
```

---

### 9. Update single-stream-poc-status.md

**Action:** Update status to reflect production decision

**Changes:**

#### Executive Summary

**Replace:**
```markdown
**Date:** 2025-10-16
**Status:** ✅ **Core Components Complete and Tested**
```

**With:**
```markdown
**Date:** 2025-10-16
**Status:** ✅ **APPROVED FOR PRODUCTION** - Core components complete, remaining work scheduled

**Decision Date:** 2025-10-16
**Migration Status:** Dual pipeline development abandoned, single stream selected as production architecture
```

#### Add New Section: Next Steps (Production Implementation)

```markdown
## Next Steps (Production Implementation)

### Immediate Actions

1. **Update Documentation** (see `single-stream-migration-proposal.md`)
   - Archive GStreamer-related documents
   - Create `single-stream-playback.md` comprehensive guide
   - Update `implementation_order.md` Phase 2.6
   - Update `architecture.md` technology stack

2. **Remove Dual Pipeline Code**
   - Delete `wkmp-ap/src/playback/pipeline/dual.rs` (500 LOC)
   - Remove GStreamer dependencies from `Cargo.toml`
   - Update `mod.rs` to use single stream as default

3. **Enable Single Stream Dependencies**
   - Uncomment symphonia, rubato, cpal in `Cargo.toml`
   - Verify build with new dependencies
   - Run existing tests (28/28 should pass)

4. **Implement Remaining Components** (2-3 days)
   - Audio decoder (`decoder.rs` ~200 LOC)
   - Audio output (`output.rs` ~300 LOC)
   - Pipeline integration (~200 LOC)
   - Test program (~100 LOC)

### Implementation Schedule

**Week 1:**
- Documentation updates (1 day)
- Audio decoder implementation (1 day)
- Audio output implementation (1-2 days)

**Week 2:**
- Pipeline integration (1 day)
- Testing and debugging (1-2 days)
- Complete single stream implementation

**Total Estimated Effort:** 5-7 working days
```

---

## Implementation Roadmap

### Phase 1: Documentation Updates (1 day)

1. Create `docs/single-stream-migration-proposal.md` (this document)
2. Update `docs/implementation_order.md` (Phase 2.6 rewrite)
3. Archive `docs/gstreamer_design.md` → `gstreamer_design_archived.md`
4. Archive `docs/dual-pipeline-design.md` → `dual-pipeline-design_archived.md`
5. Update `docs/architecture.md` (technology stack, component descriptions)
6. Update `docs/architecture-comparison.md` (add decision notice)
7. Update `docs/single-stream-poc-status.md` (approved status, next steps)
8. Update `docs/deployment.md` (remove GStreamer bundling, add Rust library info)
9. Update `docs/crossfade.md` (minimal reference updates)

### Phase 2: Code Migration (0.5 days)

1. Remove dual pipeline implementation:
   - Delete `wkmp-ap/src/playback/pipeline/dual.rs`
   - Update `wkmp-ap/src/playback/pipeline/mod.rs`
2. Enable single stream dependencies in `Cargo.toml`:
   - Uncomment symphonia, rubato, cpal
   - Remove gstreamer dependencies
3. Verify build and run existing tests (28/28 should pass)

### Phase 3: Audio Decoder Implementation (1 day)

**File:** `wkmp-ap/src/playback/pipeline/single_stream/decoder.rs` (~200 LOC)

**Responsibilities:**
- Open audio files with symphonia
- Decode to PCM samples (f32 stereo interleaved)
- Handle sample rate conversion with rubato
- Support seek to passage start_time_ms
- Fill PassageBuffer with decoded samples

**API:**
```rust
pub async fn decode_passage(
    file_path: &Path,
    start_ms: i64,
    end_ms: i64,
    sample_rate: u32,
) -> Result<PassageBuffer>;
```

### Phase 4: Audio Output Implementation (1-2 days)

**File:** `wkmp-ap/src/playback/pipeline/single_stream/output.rs` (~300 LOC)

**Responsibilities:**
- Create audio output stream with cpal
- Implement ring buffer for smooth playback
- Audio callback to pull from CrossfadeMixer
- Handle buffer underruns gracefully
- Support multiple backends (PulseAudio, ALSA, CoreAudio, WASAPI)

**API:**
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

### Phase 5: Pipeline Integration (1 day)

**File:** `wkmp-ap/src/playback/pipeline/single_stream/pipeline.rs` (~200 LOC)

**Responsibilities:**
- Integrate decoder + mixer + output
- Implement play/pause/stop/seek controls
- Position and duration tracking
- Volume control
- Event emission (PassageStarted, PassageCompleted, etc.)
- Error handling and recovery

**API:**
```rust
pub struct SingleStreamPipeline {
    decoder: AudioDecoder,
    mixer: Arc<RwLock<CrossfadeMixer>>,
    output: AudioOutput,
    state: PipelineState,
}

impl SingleStreamPipeline {
    pub async fn load_passage(&mut self, passage: &Passage) -> Result<()>;
    pub fn play(&mut self) -> Result<()>;
    pub fn pause(&mut self) -> Result<()>;
    pub fn seek(&mut self, position_ms: i64) -> Result<()>;
    pub fn position_ms(&self) -> Option<i64>;
    pub fn duration_ms(&self) -> Option<i64>;
}
```

### Phase 6: Testing and Documentation (1-2 days)

1. Create test program (`examples/single_stream_test.rs` ~100 LOC)
2. Test with various audio formats (MP3, FLAC, AAC, Vorbis)
3. Test crossfading behavior with different fade curves
4. Verify position queries, seeking, volume control
5. Create `docs/single-stream-playback.md` (comprehensive architecture doc)
6. Update API documentation with examples

---

## Benefits Summary

### Technical Benefits

1. **Sample-Accurate Crossfading**: 0.02ms precision vs 10-50ms (GStreamer)
2. **Lower Memory Usage**: 27 MB vs 170 MB for 5 buffered passages (6x reduction)
3. **Simpler Architecture**: Direct sample access vs GStreamer framework indirection
4. **Pure Rust**: No C library dependencies (except system audio backends)
5. **Better Testability**: Unit tests for all components (28/28 passing)

### Development Benefits

1. **Less Code**: ~1700 total LOC (single stream) vs ~2000+ LOC (dual pipeline with integration)
2. **Easier Debugging**: Direct access to audio samples, no GStreamer black boxes
3. **Better Performance**: Lower memory and CPU usage
4. **Cleaner Abstractions**: PassageBuffer, CrossfadeMixer APIs are intuitive

### Deployment Benefits

1. **Static Binaries**: symphonia/rubato compile into binary (no GStreamer bundling)
2. **Smaller Packages**: No need to bundle 50+ MB GStreamer framework
3. **Simpler Installation**: Only system audio libraries required (already present on most systems)
4. **Cross-Platform**: cpal handles platform differences transparently

---

## Risks and Mitigation

### Risk 1: Audio Format Support

**Risk:** symphonia may not support all formats users need

**Mitigation:**
- symphonia supports: MP3, FLAC, AAC, Vorbis, Opus, WavPack, WAV, AIFF
- This covers >95% of common audio formats
- Can add additional format support via symphonia plugins if needed
- Document supported formats clearly in user documentation

### Risk 2: Platform Audio Backend Issues

**Risk:** cpal may have bugs or limitations on specific platforms

**Mitigation:**
- cpal is mature and widely used in Rust audio projects
- Supports all major platforms (Linux/PulseAudio/ALSA, Windows/WASAPI, macOS/CoreAudio)
- Active maintenance and community support
- Fallback options: can add alternative backends if issues arise

### Risk 3: Performance on Low-End Hardware

**Risk:** PCM buffer decoding may be too slow on Raspberry Pi Zero2W

**Mitigation:**
- Proof of concept demonstrates efficient memory usage (27 MB for 5 passages)
- Pre-loading strategy decodes passages before playback (5 seconds advance)
- CPU requirements for PCM mixing are minimal (simple arithmetic)
- Can optimize buffer sizes and pre-load timing if needed
- Target platform testing should be done early to validate performance

### Risk 4: Development Timeline

**Risk:** Estimated 2-3 days may be insufficient

**Mitigation:**
- Core components already implemented and tested (913 LOC, 28/28 tests passing)
- Remaining work is straightforward integration (decoder, output wrappers)
- Similar patterns exist in Rust ecosystem (many examples to reference)
- If timeline extends, dual pipeline can be kept as fallback temporarily
- Progressive migration: implement single stream alongside dual pipeline initially

---

## Recommendation

**APPROVE this migration** for the following reasons:

1. **Technical superiority is proven**: Proof of concept demonstrates 500-2500x better precision
2. **Core components are production-ready**: 100% test pass rate, well-architected APIs
3. **Remaining work is manageable**: 2-3 days for straightforward integration tasks
4. **Early migration is optimal**: Dual pipeline not yet deeply integrated
5. **Long-term benefits are substantial**: Lower memory, simpler code, better maintainability

**Next Step:** Obtain approval to proceed with Phase 1 (documentation updates) and Phase 2 (code migration)

---

**Document Version:** 1.0
**Created:** 2025-10-16
**Related Documents:**
- `single-stream-design.md` - Complete technical design
- `single-stream-poc-status.md` - Proof of concept status
- `architecture-comparison.md` - Dual vs Single comparison
- `implementation_order.md` - Implementation schedule

---
End of document - Single Stream Migration Proposal
