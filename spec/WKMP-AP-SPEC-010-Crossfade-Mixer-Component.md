# WKMP Audio Player - Crossfade Mixer Component Specification

**Document ID:** WKMP-AP-SPEC-010
**Version:** 1.0
**Date:** 2025-10-22
**Parent:** WKMP-AP-SPEC-003 (Audio Processing Subsystem)

---

## 1. Purpose

The CrossfadeMixer is the heart of the audio playback system. It performs sample-accurate mixing of audio passages with seamless crossfading, applying fade curves, detecting passage completion, and generating a single continuous audio stream for output.

**Key Responsibilities:**
- Read audio frames from ring buffers
- Apply fade-in and fade-out curves during transitions
- Mix overlapping audio during crossfades
- Detect passage completion
- Handle underrun (buffer empty) conditions
- Emit position events periodically
- Support pause/resume with fade-in

---

## 2. Component Architecture

### 2.1 State Machine

```
┌──────┐
│ None │ (no audio)
└───┬──┘
    │ start_passage()
    ▼
┌───────────────┐
│SinglePassage  │ (one passage playing)
└───┬───────────┘
    │ start_crossfade()
    ▼
┌───────────────┐
│ Crossfading   │ (two passages mixing)
└───┬───────────┘
    │ crossfade duration complete
    ▼
┌───────────────┐
│SinglePassage  │ (next passage)
└───────────────┘
```

### 2.2 Data Structure

```rust
pub struct CrossfadeMixer {
    /// Current mixer state
    state: MixerState,

    /// Sample rate (always 44100)
    sample_rate: u32,

    /// Event emission channel (optional)
    event_tx: Option<mpsc::UnboundedSender<PlaybackEvent>>,

    /// Frame counter for position event emission
    frame_counter: usize,

    /// Position event interval in frames
    position_event_interval_frames: usize,

    /// Buffer manager for checking buffer status
    buffer_manager: Option<Arc<BufferManager>>,

    /// Minimum buffer samples before starting playback
    mixer_min_start_level: usize,

    /// Total frames mixed (monotonic counter)
    total_frames_mixed: AtomicU64,

    /// Underrun state tracking
    underrun_state: Option<UnderrunState>,

    /// Pause state tracking
    pause_state: Option<PauseState>,

    /// Resume fade-in state
    resume_state: Option<ResumeState>,

    /// Crossfade completion signaling
    crossfade_completed_passage: Option<Uuid>,
}

enum MixerState {
    None,

    SinglePassage {
        passage_id: Uuid,
        fade_in_curve: Option<FadeCurve>,
        fade_in_duration_samples: usize,
        frame_count: usize,  // Frames consumed
    },

    Crossfading {
        // Current passage (fading out)
        current_passage_id: Uuid,
        fade_out_curve: FadeCurve,
        fade_out_duration_samples: usize,

        // Next passage (fading in)
        next_passage_id: Uuid,
        fade_in_curve: FadeCurve,
        fade_in_duration_samples: usize,

        // Progress
        crossfade_frame_count: usize,
    },
}
```

### 2.3 Supporting Structures

**Underrun State:**
```rust
struct UnderrunState {
    passage_id: Uuid,
    flatline_frame: AudioFrame,   // Last valid frame
    started_at: Instant,
    position_frames: usize,
}
```

**Pause State:**
```rust
struct PauseState {
    paused_at: Instant,
    pause_position_frames: usize,
}
```

**Resume State:**
```rust
struct ResumeState {
    resumed_at: Instant,
    fade_in_duration_frames: usize,
    fade_in_curve: FadeCurve,
    frames_since_resume: usize,
}
```

---

## 3. Core Operations

### 3.1 Initialization

```rust
pub fn new() -> Self {
    CrossfadeMixer {
        state: MixerState::None,
        sample_rate: 44100,
        event_tx: None,
        frame_counter: 0,
        position_event_interval_frames: 44100,  // 1 second default
        buffer_manager: None,
        mixer_min_start_level: 44100,  // 1 second default
        total_frames_mixed: AtomicU64::new(0),
        underrun_state: None,
        pause_state: None,
        resume_state: None,
        crossfade_completed_passage: None,
    }
}
```

**Configuration:**
```rust
pub fn set_buffer_manager(&mut self, buffer_manager: Arc<BufferManager>) {
    self.buffer_manager = Some(buffer_manager);
}

pub fn set_event_channel(&mut self, tx: mpsc::UnboundedSender<PlaybackEvent>) {
    self.event_tx = Some(tx);
}

pub fn set_mixer_min_start_level(&mut self, min_level: usize) {
    self.mixer_min_start_level = min_level.clamp(8820, 220500);
}

pub fn set_position_event_interval_ms(&mut self, interval_ms: u32) {
    self.position_event_interval_frames =
        ((interval_ms as f32 / 1000.0) * self.sample_rate as f32) as usize;
}
```

### 3.2 Start Passage (No Crossfade)

```rust
pub async fn start_passage(
    &mut self,
    passage_id: Uuid,
    fade_in_curve: Option<FadeCurve>,
    fade_in_duration_samples: usize,
) {
    self.state = MixerState::SinglePassage {
        passage_id,
        fade_in_curve,
        fade_in_duration_samples,
        frame_count: 0,
    };
}
```

### 3.3 Start Crossfade

```rust
pub async fn start_crossfade(
    &mut self,
    next_passage_id: Uuid,
    fade_out_curve: FadeCurve,
    fade_out_duration_samples: usize,
    fade_in_curve: FadeCurve,
    fade_in_duration_samples: usize,
) -> Result<(), Error> {
    match &self.state {
        MixerState::SinglePassage { passage_id, .. } => {
            // Check if next buffer has minimum samples
            if let Some(ref buffer_manager) = self.buffer_manager {
                if let Some(next_buffer) = buffer_manager.get_buffer(next_passage_id).await {
                    let occupied = next_buffer.occupied();

                    if occupied < self.mixer_min_start_level {
                        return Err(Error::InvalidState(format!(
                            "Next buffer not ready: {} < {}",
                            occupied, self.mixer_min_start_level
                        )));
                    }
                }
            }

            // Transition to Crossfading state
            self.state = MixerState::Crossfading {
                current_passage_id: *passage_id,
                fade_out_curve,
                fade_out_duration_samples,
                next_passage_id,
                fade_in_curve,
                fade_in_duration_samples,
                crossfade_frame_count: 0,
            };

            Ok(())
        }
        _ => Err(Error::InvalidState("No passage playing".to_string())),
    }
}
```

### 3.4 Get Next Frame (Core Mixing Logic)

This is the most critical method, called by the audio output callback for every audio frame.

```rust
pub async fn get_next_frame(&mut self) -> AudioFrame {
    // 1. Check if paused - return silence immediately
    if self.pause_state.is_some() {
        return AudioFrame::zero();
    }

    // 2. Check if in underrun - try to resume or output flatline
    if let Some(ref underrun) = self.underrun_state.clone() {
        if self.can_resume_from_underrun(underrun.passage_id, underrun.position_frames).await {
            // Auto-resume
            self.underrun_state = None;
        } else {
            // Still in underrun - output flatline
            return underrun.flatline_frame;
        }
    }

    // 3. Generate frame based on current state
    let mut underrun_check: Option<(Uuid, usize, AudioFrame)> = None;

    let frame = match &mut self.state {
        MixerState::None => AudioFrame::zero(),

        MixerState::SinglePassage {
            passage_id,
            fade_in_curve,
            fade_in_duration_samples,
            frame_count,
        } => {
            // Check minimum start level on first frame
            if *frame_count == 0 {
                if let Some(ref buffer_manager) = self.buffer_manager {
                    if let Some(buffer_arc) = buffer_manager.get_buffer(*passage_id).await {
                        let occupied = buffer_arc.occupied();

                        if occupied < self.mixer_min_start_level {
                            // Wait for more buffer
                            return AudioFrame::zero();
                        }
                    }
                }
            }

            // Pop frame from buffer
            let mut frame = pop_buffer_frame(&self.buffer_manager, *passage_id).await;

            // Apply fade-in if active
            if let Some(curve) = fade_in_curve {
                if *frame_count < *fade_in_duration_samples {
                    let fade_position = *frame_count as f32 / *fade_in_duration_samples as f32;
                    let multiplier = curve.calculate_fade_in(fade_position);
                    frame.apply_volume(multiplier);
                }
            }

            *frame_count += 1;

            // Save for underrun detection
            underrun_check = Some((*passage_id, *frame_count, frame));

            frame
        }

        MixerState::Crossfading {
            current_passage_id,
            fade_out_curve,
            fade_out_duration_samples,
            next_passage_id,
            fade_in_curve,
            fade_in_duration_samples,
            crossfade_frame_count,
        } => {
            // Pop from both buffers
            let mut current_frame = pop_buffer_frame(&self.buffer_manager, *current_passage_id).await;
            let mut next_frame = pop_buffer_frame(&self.buffer_manager, *next_passage_id).await;

            // Calculate fade positions (0.0 to 1.0)
            let fade_out_pos = *crossfade_frame_count as f32 / *fade_out_duration_samples as f32;
            let fade_in_pos = *crossfade_frame_count as f32 / *fade_in_duration_samples as f32;

            // Apply fade curves
            let fade_out_mult = fade_out_curve.calculate_fade_out(fade_out_pos);
            let fade_in_mult = fade_in_curve.calculate_fade_in(fade_in_pos);

            current_frame.apply_volume(fade_out_mult);
            next_frame.apply_volume(fade_in_mult);

            // Mix (sum) the frames
            let mut mixed = current_frame;
            mixed.add(&next_frame);
            mixed.clamp();  // Prevent clipping

            *crossfade_frame_count += 1;

            // Check if crossfade complete
            let max_duration = (*fade_out_duration_samples).max(*fade_in_duration_samples);
            if *crossfade_frame_count >= max_duration {
                // Capture outgoing passage ID BEFORE transition
                let outgoing_passage_id = *current_passage_id;
                let new_passage_id = *next_passage_id;

                // Transition to SinglePassage
                self.state = MixerState::SinglePassage {
                    passage_id: new_passage_id,
                    fade_in_curve: None,
                    fade_in_duration_samples: 0,
                    frame_count: *crossfade_frame_count,
                };

                // Signal completion
                self.crossfade_completed_passage = Some(outgoing_passage_id);
            }

            mixed
        }
    };

    // 4. Perform underrun detection
    if let Some((passage_id, next_position, last_frame)) = underrun_check {
        if self.detect_underrun(passage_id, next_position).await {
            self.underrun_state = Some(UnderrunState {
                passage_id,
                flatline_frame: last_frame,
                started_at: Instant::now(),
                position_frames: next_position - 1,
            });
        }
    }

    // 5. Update frame counter and emit position events
    self.frame_counter += 1;
    self.total_frames_mixed.fetch_add(1, Ordering::Relaxed);

    if self.frame_counter >= self.position_event_interval_frames {
        self.frame_counter = 0;

        if let Some(tx) = &self.event_tx {
            if let Some(passage_id) = self.get_current_passage_id() {
                let position_ms = self.calculate_position_ms();

                let _ = tx.send(PlaybackEvent::PositionUpdate {
                    queue_entry_id: passage_id,
                    position_ms,
                });
            }
        }
    }

    // 6. Apply resume fade-in (if active)
    let mut final_frame = frame;
    if let Some(ref mut resume) = self.resume_state {
        if resume.frames_since_resume < resume.fade_in_duration_frames {
            let fade_position = resume.frames_since_resume as f32
                              / resume.fade_in_duration_frames as f32;
            let resume_fade_multiplier = resume.fade_in_curve.calculate_fade_in(fade_position);
            final_frame.apply_volume(resume_fade_multiplier);
            resume.frames_since_resume += 1;
        } else {
            // Fade complete
            self.resume_state = None;
        }
    }

    final_frame
}
```

**Helper: Pop Buffer Frame**
```rust
async fn pop_buffer_frame(
    buffer_manager: &Option<Arc<BufferManager>>,
    passage_id: Uuid,
) -> AudioFrame {
    if let Some(ref manager) = buffer_manager {
        manager.pop_frame(passage_id).await.unwrap_or_else(|| AudioFrame::zero())
    } else {
        AudioFrame::zero()
    }
}
```

### 3.5 Pause and Resume

**Pause:**
```rust
pub fn pause(&mut self) {
    let current_position = match &self.state {
        MixerState::SinglePassage { frame_count, .. } => *frame_count,
        MixerState::Crossfading { crossfade_frame_count, .. } => *crossfade_frame_count,
        MixerState::None => 0,
    };

    self.pause_state = Some(PauseState {
        paused_at: Instant::now(),
        pause_position_frames: current_position,
    });
}
```

**Resume:**
```rust
pub fn resume(&mut self, fade_in_duration_ms: u64, fade_in_curve_name: &str) {
    if let Some(pause_state) = self.pause_state.take() {
        // Calculate fade-in duration in frames
        let fade_in_duration_frames = ((fade_in_duration_ms as f32 / 1000.0)
                                      * self.sample_rate as f32) as usize;

        // Parse fade curve
        let fade_in_curve = match fade_in_curve_name {
            "linear" => FadeCurve::Linear,
            "exponential" => FadeCurve::Exponential,
            "cosine" => FadeCurve::Cosine,
            _ => FadeCurve::Exponential,
        };

        // Enter resume state
        self.resume_state = Some(ResumeState {
            resumed_at: Instant::now(),
            fade_in_duration_frames,
            fade_in_curve,
            frames_since_resume: 0,
        });
    }
}
```

### 3.6 Stop

```rust
pub fn stop(&mut self) {
    self.state = MixerState::None;
    self.crossfade_completed_passage = None;
}
```

### 3.7 Query Methods

**Get Current Passage ID:**
```rust
pub fn get_current_passage_id(&self) -> Option<Uuid> {
    match &self.state {
        MixerState::SinglePassage { passage_id, .. } => Some(*passage_id),
        MixerState::Crossfading { current_passage_id, .. } => Some(*current_passage_id),
        MixerState::None => None,
    }
}
```

**Get Next Passage ID (during crossfade):**
```rust
pub fn get_next_passage_id(&self) -> Option<Uuid> {
    match &self.state {
        MixerState::Crossfading { next_passage_id, .. } => Some(*next_passage_id),
        _ => None,
    }
}
```

**Get Position:**
```rust
pub fn get_position(&self) -> usize {
    match &self.state {
        MixerState::SinglePassage { frame_count, .. } => *frame_count,
        MixerState::Crossfading { crossfade_frame_count, .. } => *crossfade_frame_count,
        MixerState::None => 0,
    }
}
```

**Check if Crossfading:**
```rust
pub fn is_crossfading(&self) -> bool {
    matches!(self.state, MixerState::Crossfading { .. })
}
```

**Check if Paused:**
```rust
pub fn is_paused(&self) -> bool {
    self.pause_state.is_some()
}
```

**Check if Current Finished:**
```rust
pub async fn is_current_finished(&self) -> bool {
    match &self.state {
        MixerState::SinglePassage { passage_id, .. } => {
            if let Some(ref buffer_manager) = self.buffer_manager {
                buffer_manager.is_buffer_exhausted(*passage_id).await.unwrap_or(false)
            } else {
                false
            }
        }
        _ => false,
    }
}
```

**Take Crossfade Completed:**
```rust
pub fn take_crossfade_completed(&mut self) -> Option<Uuid> {
    self.crossfade_completed_passage.take()
}
```

---

## 4. Fade Curve Application

### 4.1 Fade Curve Types

```rust
pub enum FadeCurve {
    Linear,        // v(t) = t
    Exponential,   // v(t) = t²
    Cosine,        // v(t) = 0.5 * (1 - cos(π*t))
    SCurve,        // v(t) = smoothstep(t)
    Logarithmic,   // v(t) = log10(1 + 9*t)
}
```

### 4.2 Fade Calculation

**Fade-In (0.0 to 1.0):**
```rust
impl FadeCurve {
    pub fn calculate_fade_in(&self, position: f32) -> f32 {
        let t = position.clamp(0.0, 1.0);

        match self {
            FadeCurve::Linear => t,
            FadeCurve::Exponential => t * t,
            FadeCurve::Cosine => 0.5 * (1.0 - (std::f32::consts::PI * t).cos()),
            FadeCurve::SCurve => {
                // Smoothstep: 3t² - 2t³
                let t2 = t * t;
                let t3 = t2 * t;
                3.0 * t2 - 2.0 * t3
            }
            FadeCurve::Logarithmic => (1.0 + 9.0 * t).log10(),
        }
    }
}
```

**Fade-Out (1.0 to 0.0):**
```rust
pub fn calculate_fade_out(&self, position: f32) -> f32 {
    let t = position.clamp(0.0, 1.0);
    1.0 - self.calculate_fade_in(t)
}
```

### 4.3 Sample Application

```rust
impl AudioFrame {
    pub fn apply_volume(&mut self, volume: f32) {
        self.left *= volume;
        self.right *= volume;
    }
}
```

---

## 5. Underrun Detection and Recovery

### 5.1 Detection

```rust
async fn detect_underrun(&self, passage_id: Uuid, _position_frames: usize) -> bool {
    if let Some(ref buffer_manager) = self.buffer_manager {
        if let Some(status) = buffer_manager.get_status(passage_id).await {
            // Only underrun if buffer is still Decoding
            if matches!(status, BufferStatus::Decoding { .. }) {
                if let Some(buffer_arc) = buffer_manager.get_buffer(passage_id).await {
                    // Check if buffer nearly empty but decode not complete
                    return buffer_arc.fill_percent() < 1.0 && !buffer_arc.is_exhausted();
                }
            }
        }
    }
    false
}
```

### 5.2 Auto-Resume Logic

```rust
async fn can_resume_from_underrun(&self, passage_id: Uuid, _position_frames: usize) -> bool {
    if let Some(ref buffer_manager) = self.buffer_manager {
        if let Some(buffer_arc) = buffer_manager.get_buffer(passage_id).await {
            let fill_pct = buffer_arc.fill_percent();
            fill_pct > 10.0  // Resume if >10% full
        } else {
            false
        }
    } else {
        false
    }
}
```

### 5.3 Logging

```rust
async fn log_underrun(&self, passage_id: Uuid, position_frames: usize) {
    if let Some(ref buffer_manager) = self.buffer_manager {
        let status = buffer_manager.get_status(passage_id).await;
        let decode_elapsed = buffer_manager.get_decode_elapsed(passage_id).await;

        let position_ms = (position_frames as u64 * 1000) / self.sample_rate as u64;

        warn!(
            "Buffer underrun detected: passage_id={}, position={}ms ({} frames)",
            passage_id, position_ms, position_frames
        );

        if let Some(status) = status {
            warn!("Buffer status: {:?}", status);
        }

        if let Some(elapsed) = decode_elapsed {
            let decode_ms = elapsed.as_millis() as u64;
            let speed_ratio = if decode_ms > 0 {
                (position_ms as f64) / (decode_ms as f64)
            } else {
                0.0
            };
            warn!(
                "Decode speed: {} ms decoded in {} ms (ratio: {:.2}x realtime)",
                position_ms, decode_ms, speed_ratio
            );
        }
    }
}
```

---

## 6. Position Tracking and Events

### 6.1 Frame Counter

- Incremented on every `get_next_frame()` call
- Resets to 0 when reaching `position_event_interval_frames`
- Configurable interval (default: 44100 frames = 1 second)

### 6.2 Position Calculation

```rust
fn calculate_position_ms(&self) -> u64 {
    let position_frames = self.get_position();
    (position_frames as u64 * 1000) / self.sample_rate as u64
}
```

### 6.3 Event Emission

```rust
// In get_next_frame()
if self.frame_counter >= self.position_event_interval_frames {
    self.frame_counter = 0;

    if let Some(tx) = &self.event_tx {
        if let Some(passage_id) = self.get_current_passage_id() {
            let position_ms = self.calculate_position_ms();

            let _ = tx.send(PlaybackEvent::PositionUpdate {
                queue_entry_id: passage_id,
                position_ms,
            });
        }
    }
}
```

---

## 7. Performance Characteristics

### 7.1 Timing Precision

- **Sample accuracy:** 1 frame @ 44.1kHz = ~0.0227ms
- **Crossfade timing:** Frame-accurate start and end
- **Fade curve resolution:** Calculated per-frame (no interpolation)

### 7.2 CPU Usage

- **Per-frame overhead:** ~100 CPU cycles (frame pop + fade calc + mix)
- **Audio callback frequency:** 44100 Hz (22.68μs between frames)
- **Total CPU time:** <1% on modern processors

### 7.3 Memory Usage

- **Mixer state:** <1KB (state machine + counters)
- **No sample buffering:** Reads directly from ring buffers
- **Zero-copy:** Frames passed by value (8 bytes per frame)

---

## 8. Thread Safety

### 8.1 Audio Callback Thread

The `get_next_frame()` method is called from the cpal audio callback thread, which has strict real-time requirements:

- **No blocking:** All operations must be non-blocking
- **No allocation:** No heap allocations in hot path
- **Lock-free preferred:** Ring buffer pops are lock-free

### 8.2 Synchronization

- `buffer_manager` uses internal locks (brief, necessary)
- `total_frames_mixed` uses atomics (no locks)
- State machine (`self.state`) accessed only by audio thread (no locks needed)

---

## 9. Error Handling

### 9.1 Buffer Empty (Underrun)

**Detection:** Ring buffer pop returns last frame
**Response:** Enter underrun state, output flatline
**Recovery:** Auto-resume when buffer refills

### 9.2 Invalid State Transitions

**Example:** `start_crossfade()` when not in `SinglePassage` state
**Response:** Return `Error::InvalidState`
**Recovery:** Caller must check mixer state before calling

### 9.3 Configuration Errors

**Example:** Invalid fade curve name
**Response:** Fallback to default (Exponential)
**Recovery:** Log warning, continue with fallback

---

## 10. Testing Approach

### 10.1 Unit Tests

- State transitions (None → SinglePassage → Crossfading → SinglePassage)
- Fade curve calculations (verify correct output for t=0, 0.25, 0.5, 0.75, 1.0)
- Frame counting and position calculation
- Pause/resume logic
- Underrun detection

### 10.2 Integration Tests

- End-to-end crossfade with real audio buffers
- Verify no clipping during crossfade
- Verify smooth fade transitions
- Test all fade curve types
- Verify position events emitted at correct intervals

### 10.3 Audio Quality Tests

- Measure crossfade smoothness (no pops/clicks)
- Verify volume levels during fade (correct curve shape)
- Test edge cases (very short fades, very long fades)
- Verify no clipping with high-amplitude audio

---

## 11. Known Limitations

### 11.1 No Seeking

Drain-based ring buffers don't support seeking. Position can only advance forward.

### 11.2 No Pitch Shifting

All audio played at original pitch (sample rate conversion only).

### 11.3 No Real-Time Effects

No EQ, reverb, compression, or other DSP effects.

### 11.4 Single Output Device

Cannot split audio to multiple devices simultaneously.

---

## 12. Future Enhancements

### 12.1 Dynamic Fade Curves

Allow fade curve to change mid-fade based on audio analysis.

### 12.2 Advanced Crossfade Modes

- Beat-synced crossfades (align to detected beats)
- Frequency-domain crossfades (fade different frequency bands at different rates)
- Dynamic duration adjustment based on audio content

### 12.3 Real-Time Effects Chain

Insert DSP effects between mixer output and audio device:
- Parametric EQ
- Dynamic range compression
- Reverb/delay
- Stereo width adjustment

---

## 13. Related Specifications

- `WKMP-AP-SPEC-001` - System Architecture Overview
- `WKMP-AP-SPEC-002` - Playback Engine Subsystem (parent)
- `WKMP-AP-SPEC-003` - Audio Processing Subsystem (parent)
- `WKMP-AP-SPEC-004` - Buffer Management Subsystem
- `WKMP-AP-SPEC-012` - Ring Buffer Component

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-22 | System Analyst | Initial detailed specification |
