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

**[SSD-DEC-010]** The decoder uses a **decode-from-start-and-skip** approach for reliable, sample-accurate positioning:

1. **[SSD-DEC-011]** Receive decode request from buffer manager (with passage start/end times)
2. **[SSD-DEC-012]** Open file using `symphonia` decoder
3. **[SSD-DEC-013]** Always decode from the beginning of the audio file (never use compressed seek)
4. **[SSD-DEC-014]** Skip samples until reaching passage start time
5. **[SSD-DEC-015]** Continue decoding and buffering until passage end time
6. **[SSD-DEC-016]** Resample to standard rate (44.1kHz) if needed using `rubato`
7. **[SSD-DEC-017]** Write PCM data to passage buffer
8. **[SSD-DEC-018]** Notify buffer manager of completion

**[SSD-DEC-020]** Rationale for decode-and-skip:
- **[SSD-DEC-021]** Compressed file seeking (e.g., MP3, AAC) is unreliable and format-dependent
- **[SSD-DEC-022]** Variable bitrate files have unpredictable seek performance
- **[SSD-DEC-023]** Decode-from-start ensures exact, repeatable time points
- **[SSD-DEC-024]** Provides sample-accurate positioning required for crossfading
- **[SSD-DEC-025]** Trade-off: Slightly longer decode time, but guarantees correctness

### Decoder Pool Lifecycle Specification

**[SSD-DEC-030]** Pool Sizing:
- Fixed pool: 2 decoder threads
- Rationale: Sufficient for current + next passage full decode (SSD-FBUF-010)
- Hardware constraint: Raspberry Pi Zero2W resource limits (REQ-TECH-011)

**[SSD-DEC-031]** Thread Creation:
- Lazy initialization: Threads created on first decode request
- Persistent: Threads remain alive until module shutdown
- No dynamic scaling: Pool size fixed at 2 for entire session

**[SSD-DEC-032]** Priority Queue Management:
- Implementation: Ordered `VecDeque<DecodeRequest>`
- Insertion: Insert at position based on priority value
  - Priority 0 (Immediate): Currently playing passage (buffer underrun recovery)
  - Priority 1 (Next): Next-to-play passage
  - Priority 2 (Prefetch): Queued passages after next
- Dequeue: Always pop front (highest priority first)

**[SSD-DEC-033]** Shutdown Behavior:
- Signal: Set `AtomicBool` stop flag read by all threads
- Join: Wait for all threads with 5-second timeout
- Timeout handling: Log warning and proceed (non-critical at shutdown)
- In-flight decodes: Abandoned (buffers discarded)

**Memory Impact:** 2 threads × ~8KB stack = ~16KB overhead (negligible)

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

**[SSD-BUF-010]** The system uses two distinct buffering strategies based on passage playback state:

**[SSD-FBUF-010] 1. Currently Playing or Next-to-Play Passage - Full Decode Strategy:**
- **[SSD-FBUF-011]** When a passage is in the currently playing OR next-to-be-played position in the queue, the ENTIRE passage is completely decoded into raw waveforms buffered in RAM ready to be played
- **[SSD-FBUF-012]** Purpose: Enable instant, sample-accurate seeking anywhere within the passage
- **[SSD-FBUF-013]** Rationale: Compressed file decoder seek-to-time performance is unreliable across formats

**Decoding Process:**
- **[SSD-FBUF-020]** Resampling: When necessary, resample to the standard 44.1kHz sample rate using `rubato`
- **[SSD-FBUF-021]** Decode-and-skip approach: Always decode from the start of the audio file through to the end time specified in the passage to achieve accurate timing
  - **[SSD-FBUF-022]** Never use compressed seek (unreliable across formats and variable bitrate files)
  - **[SSD-FBUF-023]** Skip decoded audio before the start time specified in the passage
  - **[SSD-FBUF-024]** If no start time is specified, implicit start time is at the start of the audio file
  - **[SSD-FBUF-025]** Continue decoding until passage end time is reached
  - **[SSD-FBUF-026]** If no end time is specified, implicit end time is at the end of the audio file
  - **[SSD-FBUF-027]** When the end time of the passage is before the end of the audio file, decoding may stop as soon as the end time uncompressed audio has been buffered - no need to decode to the end of the audio file

**Fade Curve Application:**
- **[SSD-FADE-010]** Fade-in curve: May be applied to the buffered data as soon as the decoding process has passed the fade-in point
- **[SSD-FADE-011]** Fade-out curve: May be applied to the buffered data as soon as the decoding process has reached the end time
- **[SSD-FADE-012]** Fade application can occur during decode or be deferred to read-time (implementation choice)

**Benefits:**
- **[SSD-FBUF-030]** Sample-accurate positioning at any point in the passage
- **[SSD-FBUF-031]** Repeatable, exact time points within the audio file
- **[SSD-FBUF-032]** No dependency on format-specific seeking capabilities
- **[SSD-FBUF-033]** Eliminates seeking latency during playback

**[SSD-PBUF-010] 2. Queued Passages (After Next) - Partial Buffer Strategy (Configurable):**
- **[SSD-PBUF-011]** Passages in the queue AFTER the next-to-be-played position are partially decoded to obtain a short buffer of audio ready to be played immediately in case the user skips ahead in the queue
- **[SSD-PBUF-012]** Default: 15-second buffer (configurable)
- **[SSD-PBUF-013]** Purpose: Facilitate instant skip to queued passages without audio dropout
- **[SSD-PBUF-014]** Gives sufficient time to fully decode the entire passage before playback buffer starvation
- **[SSD-PBUF-015]** Prevents audio glitches during passage transitions

**Partial Buffer Playback Handling:**

**[SSD-PBUF-020]** When a buffer starts playing that does not contain the entire passage:

1. **[SSD-PBUF-021]** If decoding is currently in process when buffer starts playing:
   - **[SSD-PBUF-022]** Decoding continues to complete the buffer
   - **[SSD-PBUF-023]** Playback proceeds from partial buffer while decode completes in background

2. **[SSD-PBUF-024]** If decoding is not currently in process when buffer starts playing:
   - **[SSD-PBUF-025]** A new buffer is created by restarting the decoding process
   - **[SSD-PBUF-026]** When the currently playing buffer reaches the end of its cleanly decoded audio data, playback is seamlessly switched to the same sample point in the buffer that is being completely filled through decoding, whether decoding is complete yet or not

**Buffer Underrun Handling:**

**[SSD-UND-010]** If playback should progress to a point where no decoded audio is yet available to play:
- **[SSD-UND-011]** Log warning describing:
  - **[SSD-UND-012]** Current buffer status
  - **[SSD-UND-013]** Recent skip activity
  - **[SSD-UND-014]** Current decoding speed relative to playback speed
  - **[SSD-UND-015]** Note: Pre-buffering time to handle multiple skips may need to be extended, or estimated on a passage-by-passage basis depending on the passage's start time in its audio file
- **[SSD-UND-016]** Pause playback until at least one second of buffered data beyond the current playback point is available in buffer
  - **[SSD-UND-017]** Pause is implemented by re-feeding the same audio output level (flatline from the point of pausing) while pause is in effect
- **[SSD-UND-018]** Automatically resume once sufficient buffer is available

**[SSD-UND-020]** If playback should reach the fade-out point of a buffer before the fade-out curve has been applied to the data in the buffer:
- **[SSD-UND-021]** Log warning describing:
  - **[SSD-UND-022]** Current buffer status
  - **[SSD-UND-023]** Recent skip activity
  - **[SSD-UND-024]** Current decoding speed relative to playback speed
- **[SSD-UND-025]** Pause playback until the fade-out curve has been completely applied
- **[SSD-UND-026]** Automatically resume once fade-out curve is applied

**Additional Buffer Management:**
- **Background decoding** for next 2-3 passages in queue
- **On-demand decoding** for skip targets
- **Buffer recycling** - reuse memory after passage completes

**Memory Calculation:**
```
Partial buffer (15-second): 44100 Hz * 2 channels * 4 bytes/sample * 15 sec = ~5.3 MB
Full passage buffer: Varies by passage duration (e.g., 3 minutes = ~63 MB)
For 5 passages (1 playing + 1 next + 3 queued):
  - 2 fully decoded: 2 × ~63 MB = ~126 MB
  - 3 partially buffered: 3 × 5.3 MB = ~16 MB
  - Total: ~142 MB typical
```

### Partial to Complete Buffer Handoff Mechanism

**[SSD-PBUF-030]** Handoff Architecture:

When a partial buffer starts playing before complete decode finishes, the system seamlessly transitions to the complete buffer.

**Buffer References:**
- `partial_buffer_ref`: Initial 15-second buffer (available immediately for skip-ahead)
- `complete_buffer_ref`: Full passage buffer (populated during background decode)

**Handoff Procedure:**

1. **Initial State:**
   - Mixer starts reading from `partial_buffer_ref`
   - Decoder fills `complete_buffer_ref` in background
   - Both buffers share sample position tracking

2. **Decode Completion:**
   - Decoder signals completion
   - Atomically swap: `partial_buffer_ref = complete_buffer_ref`
   - Swap occurs at mixer cycle boundary (not mid-sample)

3. **Continued Playback:**
   - Sample position tracking continues uninterrupted
   - Mixer transparently reads from new buffer reference
   - No audible discontinuity (references point to same underlying data once decode completes)

**[SSD-PBUF-031]** Synchronization Mechanism:
- Buffer storage: `Arc<RwLock<PassageBuffer>>`
- Atomic reference updates: Write lock for swap, read lock for mixer access
- No sample-level synchronization needed (buffers contain identical data after decode)
- Lock contention: Minimal (swap occurs once per passage, mixer read locks are brief)

**[SSD-PBUF-032]** Edge Case - Playback Completes Before Full Decode:
- If partial buffer exhausts before complete buffer ready: Normal underrun handling (SSD-UND-015)
- Auto-pause applies if needed
- Complete buffer swap still occurs when decode finishes
- Subsequent seeks/replays use complete buffer

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

**[SSD-CLIP-010]** During crossover from one passage to the next (lead-out of the currently playing passage and lead-in of the next), data from the two passages' buffers is summed to get the audio data for output.

**[SSD-CLIP-020]** If summation results in an output level that will be clipped (absolute value > 1.0 for f32 samples):
- **[SSD-CLIP-021]** Log warning describing:
  - **[SSD-CLIP-022]** The playback point in both passages
  - **[SSD-CLIP-023]** Fade durations of both passages
  - **[SSD-CLIP-024]** Fade curves of both passages
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

**[SSD-MIX-040]** Calculation Responsibility:
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

1. **Currently Playing or Next-to-Play Passage - Full Decode:**
   - ENTIRE passage decoded into RAM when entering "currently playing" OR "next-to-be-played" status
   - Enables instant, sample-accurate seeking anywhere within passage
   - Memory: ~63 MB for 3-minute passage @ 44.1kHz stereo
   - Eliminates dependency on unreliable compressed file seeking

2. **Queued Passages (After Next) - Partial Buffer (Configurable):**
   - Only first 15 seconds buffered for passages AFTER the next-to-be-played position
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

---

**Document Version:** 1.3
**Created:** 2025-10-16
**Last Updated:** 2025-10-17
**Status:** Current Architecture (Selected for Implementation)
**Note:** This single-stream architecture has been selected as the current implementation approach. See [architecture.md](architecture.md) for integration details.
**Related:** `dual-pipeline-design.md` (archived), `architecture-comparison.md`, `single-stream-playback.md`

**Change Log:**
- v1.2 (2025-10-17): Added requirement traceability IDs
  - Added document code `SSD` (Single Stream Design) to requirements_enumeration.md
  - Added category codes: DEC, BUF, FBUF, PBUF, UND, FADE, CLIP, and others
  - Assigned 79 traceability IDs to specifications throughout document
  - Created comprehensive traceability index section organized by category
  - All v1.1 specifications now have unique, traceable requirement IDs
- v1.3 (2025-10-17): Architectural decision specifications from wkmp-ap design review
  - Added "Decoder Pool Lifecycle Specification" section with pool sizing, thread creation, priority queue management, and shutdown behavior
  - Added "Crossfade Timing Calculation Ownership" section specifying CrossfadeMixer responsibility
  - Added "Partial to Complete Buffer Handoff Mechanism" section with handoff procedure and synchronization details
  - Added "Buffer State Event Emission" section with four transition points and decode progress updates
  - Supports architectural decisions from wkmp-ap design review (ISSUE-5, ISSUE-7, ISSUE-8, ISSUE-1)
- v1.1 (2025-10-17): Enhanced buffer management specifications
  - Clarified that both "currently playing" AND "next-to-be-played" passages receive full decode
  - Added detailed decode-and-skip process with implicit start/end time handling
  - Specified fade curve application timing (during decode or at read-time)
  - Added partial buffer playback handling for skip-ahead scenarios
  - Added buffer underrun handling with automatic pause/resume behavior
  - Added crossfade summation and clipping detection specifications
  - Added Challenge 6 (Buffer Underruns) and Challenge 7 (Fade Curve Timing)
  - Updated memory calculations to reflect 2 fully-decoded passages
- v1.0 (2025-10-16): Initial version
