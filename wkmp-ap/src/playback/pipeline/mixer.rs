//! Crossfade Mixer
//!
//! Implements sample-accurate crossfading between passages using a state machine.
//! Applies fade curves and mixes overlapping passages to produce a single audio stream.
//!
//! **Traceability:**
//! - [SSD-MIX-010] Crossfade mixer component
//! - [SSD-MIX-020] Mixer states
//! - [SSD-MIX-030] Single passage playback
//! - [SSD-MIX-040] Crossfade initiation
//! - [SSD-MIX-050] Sample-accurate mixing
//! - [SSD-MIX-060] Passage completion detection
//! - [REV002] Event-driven position tracking

use crate::audio::types::{AudioFrame, BufferStatus, PassageBuffer};
use crate::error::Error;
use crate::playback::buffer_manager::BufferManager;
use crate::playback::events::PlaybackEvent;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{trace, warn};
use uuid::Uuid;
use wkmp_common::FadeCurve;

/// Mixer state information for monitoring
#[derive(Debug, Clone)]
pub struct MixerStateInfo {
    pub current_passage_id: Option<Uuid>,
    pub next_passage_id: Option<Uuid>,
    pub current_position_frames: usize,
    pub next_position_frames: usize,
    pub is_crossfading: bool,
}

/// Standard sample rate for all audio processing
const STANDARD_SAMPLE_RATE: u32 = 44100;

/// Minimum buffer ahead required to resume from underrun (1 second)
///
/// **[SSD-UND-016]** Resume threshold
const UNDERRUN_RESUME_BUFFER_MS: u64 = 1000;

/// Underrun state tracking
///
/// **[SSD-UND-010]** Tracks underrun condition and flatline output
#[derive(Debug, Clone)]
struct UnderrunState {
    /// Passage ID experiencing underrun
    passage_id: Uuid,

    /// Last valid audio frame (for flatline output)
    ///
    /// **[SSD-UND-017]** Pause by re-feeding same audio level
    flatline_frame: AudioFrame,

    /// When underrun was detected
    ///
    /// **[SSD-UND-011]** For diagnostic logging
    started_at: Instant,

    /// Position when underrun occurred
    position_frames: usize,
}

/// Pause state tracking
///
/// **[XFD-PAUS-010]** Tracks when mixer is paused
#[derive(Debug, Clone)]
struct PauseState {
    /// When pause was initiated
    paused_at: Instant,

    /// Frame position when paused (for diagnostics)
    pause_position_frames: usize,
}

/// Resume state tracking
///
/// **[XFD-PAUS-020]** Tracks fade-in after resuming from pause
#[derive(Debug, Clone)]
struct ResumeState {
    /// When resume was initiated
    resumed_at: Instant,

    /// Fade-in duration in frames
    ///
    /// **[XFD-PAUS-020]** e.g., 0.5s * 44100 = 22050 frames
    fade_in_duration_frames: usize,

    /// Fade-in curve (linear, exponential, cosine)
    ///
    /// **[XFD-PAUS-020]** Configurable from database
    fade_in_curve: FadeCurve,

    /// Number of frames since resume (for fade calculation)
    ///
    /// **[XFD-PAUS-030]** Incremented on each get_next_frame() call
    frames_since_resume: usize,
}

/// Crossfade mixer state machine
///
/// **[SSD-MIX-010]** Main component for sample-accurate mixing
pub struct CrossfadeMixer {
    /// Current mixer state
    state: MixerState,

    /// Sample rate (always 44100)
    sample_rate: u32,

    /// Event emission channel (optional)
    ///
    /// **[REV002]** Event-driven position tracking
    /// When configured, mixer emits PositionUpdate events periodically
    event_tx: Option<mpsc::UnboundedSender<PlaybackEvent>>,

    /// Frame counter for position event emission
    ///
    /// Incremented on every frame. When reaches position_event_interval_frames,
    /// a PositionUpdate event is emitted and counter resets to 0.
    frame_counter: usize,

    /// Position event interval in frames
    ///
    /// **[ADDENDUM-interval_configurability]** Configurable from database
    /// Default: 44100 frames (1 second @ 44.1kHz)
    /// Loaded from `position_event_interval_ms` database setting
    position_event_interval_frames: usize,

    /// Buffer manager for checking buffer status
    ///
    /// **[SSD-UND-010]** Used for underrun detection and buffer status queries
    buffer_manager: Option<Arc<BufferManager>>,

    /// Underrun state tracking
    ///
    /// **[SSD-UND-016]** Tracks if mixer is paused due to underrun
    underrun_state: Option<UnderrunState>,

    /// Pause state tracking
    ///
    /// **[XFD-PAUS-010]** Some when paused, None when playing
    /// When paused, get_next_frame() outputs AudioFrame::zero()
    pause_state: Option<PauseState>,

    /// Resume state tracking
    ///
    /// **[XFD-PAUS-020]** Some when fading in from pause, None otherwise
    /// Tracks fade-in progress after resuming from pause
    resume_state: Option<ResumeState>,

    /// Crossfade completion signaling
    ///
    /// **[XFD-COMP-010]** Passage ID of outgoing passage when crossfade just completed
    /// Set by get_next_frame() when Crossfading → SinglePassage transition occurs
    /// Consumed by engine via take_crossfade_completed()
    crossfade_completed_passage: Option<Uuid>,
}

/// Mixer state machine
///
/// **[SSD-MIX-020]** Three states for mixer operation:
/// - None: No audio playing
/// - SinglePassage: One passage playing (no crossfade)
/// - Crossfading: Two passages overlapping with fade curves
#[derive(Debug)]
enum MixerState {
    /// No audio playing
    None,

    /// Single passage playing (no crossfade)
    ///
    /// **[SSD-MIX-030]** Single passage state with optional fade-in
    SinglePassage {
        buffer: Arc<RwLock<PassageBuffer>>,
        passage_id: Uuid,
        position: usize, // Current frame position in buffer
        fade_in_curve: Option<FadeCurve>,
        fade_in_duration_samples: usize,
    },

    /// Crossfading between two passages
    ///
    /// **[SSD-MIX-040]** Crossfade state with dual buffer mixing
    Crossfading {
        // Current passage (fading out)
        current_buffer: Arc<RwLock<PassageBuffer>>,
        current_passage_id: Uuid,
        current_position: usize,
        fade_out_curve: FadeCurve,
        fade_out_duration_samples: usize,
        fade_out_progress: usize, // Samples into fade-out

        // Next passage (fading in)
        next_buffer: Arc<RwLock<PassageBuffer>>,
        next_passage_id: Uuid,
        next_position: usize,
        fade_in_curve: FadeCurve,
        fade_in_duration_samples: usize,
        fade_in_progress: usize, // Samples into fade-in
    },
}

impl CrossfadeMixer {
    /// Create new crossfade mixer
    pub fn new() -> Self {
        CrossfadeMixer {
            state: MixerState::None,
            sample_rate: STANDARD_SAMPLE_RATE,
            event_tx: None,
            frame_counter: 0,
            position_event_interval_frames: 44100, // Default: 1 second
            buffer_manager: None,
            underrun_state: None,
            pause_state: None,    // [XFD-PAUS-010] Initially not paused
            resume_state: None,   // [XFD-PAUS-020] Initially no resume fade-in
            crossfade_completed_passage: None, // [XFD-COMP-010] Initially no completion pending
        }
    }

    /// Set buffer manager for underrun detection
    ///
    /// **[SSD-UND-010]** Enable underrun detection by providing buffer manager
    ///
    /// # Arguments
    /// * `buffer_manager` - Arc to buffer manager for status queries
    pub fn set_buffer_manager(&mut self, buffer_manager: Arc<BufferManager>) {
        self.buffer_manager = Some(buffer_manager);
    }

    /// Set event channel for position updates
    ///
    /// **[REV002]** Configure event emission
    ///
    /// # Arguments
    /// * `tx` - Unbounded sender for position events
    pub fn set_event_channel(&mut self, tx: mpsc::UnboundedSender<PlaybackEvent>) {
        self.event_tx = Some(tx);
    }

    /// Set position event interval from database setting
    ///
    /// **[ADDENDUM-interval_configurability]** Configurable interval
    ///
    /// # Arguments
    /// * `interval_ms` - Interval in milliseconds (will be converted to frames)
    pub fn set_position_event_interval_ms(&mut self, interval_ms: u32) {
        self.position_event_interval_frames =
            ((interval_ms as f32 / 1000.0) * self.sample_rate as f32) as usize;
        trace!(
            "Position event interval set to {} ms ({} frames)",
            interval_ms,
            self.position_event_interval_frames
        );
    }

    /// Start playing a passage (no crossfade)
    ///
    /// **[SSD-MIX-030]** Initiates single passage playback with optional fade-in
    /// **[DBD-MIX-010]** Accepts sample-based duration (not milliseconds)
    ///
    /// # Arguments
    /// * `buffer` - Passage buffer to play
    /// * `passage_id` - UUID of passage
    /// * `fade_in_curve` - Optional fade-in curve
    /// * `fade_in_duration_samples` - Fade-in duration in samples
    pub async fn start_passage(
        &mut self,
        buffer: Arc<RwLock<PassageBuffer>>,
        passage_id: Uuid,
        fade_in_curve: Option<FadeCurve>,
        fade_in_duration_samples: usize,
    ) {
        self.state = MixerState::SinglePassage {
            buffer,
            passage_id,
            position: 0,
            fade_in_curve,
            fade_in_duration_samples,
        };
    }

    /// Start crossfade to next passage
    ///
    /// **[SSD-MIX-040]** Transitions from SinglePassage to Crossfading state
    /// **[DBD-MIX-020]** Accepts sample-based durations (not milliseconds)
    ///
    /// # Arguments
    /// * `next_buffer` - Buffer for next passage
    /// * `next_passage_id` - UUID of next passage
    /// * `fade_out_curve` - Curve for fading out current passage
    /// * `fade_out_duration_samples` - Fade-out duration in samples
    /// * `fade_in_curve` - Curve for fading in next passage
    /// * `fade_in_duration_samples` - Fade-in duration in samples
    ///
    /// # Returns
    /// Ok if crossfade started, Err if no passage is currently playing
    pub async fn start_crossfade(
        &mut self,
        next_buffer: Arc<RwLock<PassageBuffer>>,
        next_passage_id: Uuid,
        fade_out_curve: FadeCurve,
        fade_out_duration_samples: usize,
        fade_in_curve: FadeCurve,
        fade_in_duration_samples: usize,
    ) -> Result<(), Error> {
        match &self.state {
            MixerState::SinglePassage {
                buffer, passage_id, position, ..
            } => {
                self.state = MixerState::Crossfading {
                    current_buffer: buffer.clone(),
                    current_passage_id: *passage_id,
                    current_position: *position,
                    fade_out_curve,
                    fade_out_duration_samples,
                    fade_out_progress: 0,
                    next_buffer,
                    next_passage_id,
                    next_position: 0,
                    fade_in_curve,
                    fade_in_duration_samples,
                    fade_in_progress: 0,
                };
                Ok(())
            }
            _ => Err(Error::InvalidState(
                "Cannot start crossfade: no passage playing".to_string(),
            )),
        }
    }

    /// Get next audio frame
    ///
    /// **[SSD-MIX-050]** Sample-accurate mixing with fade curve application
    /// **[REV002]** Now emits PositionUpdate events periodically
    /// **[SSD-UND-010]** Underrun detection and flatline output
    /// **[XFD-PAUS-010]** Pause handling - output silence when paused
    /// **[XFD-PAUS-020]** Resume fade-in - apply fade when resuming
    ///
    /// # Returns
    /// Next audio frame (stereo sample), or silence if no audio playing
    pub async fn get_next_frame(&mut self) -> AudioFrame {
        // **[XFD-PAUS-010]** If paused, output silence immediately
        if self.pause_state.is_some() {
            return AudioFrame::zero();
        }

        // **[SSD-UND-016]** Check if in underrun state and try to resume
        if let Some(ref underrun) = self.underrun_state.clone() {
            // Check if buffer has caught up
            if self.can_resume_from_underrun(underrun.passage_id, underrun.position_frames).await {
                // **[SSD-UND-018]** Auto-resume
                warn!(
                    "[SSD-UND-018] Resuming from underrun: passage_id={}, elapsed={}ms",
                    underrun.passage_id,
                    underrun.started_at.elapsed().as_millis()
                );
                self.underrun_state = None;
                // Continue to normal frame generation below
            } else {
                // **[SSD-UND-017]** Still in underrun - output flatline
                trace!("Underrun continues: outputting flatline frame");
                return underrun.flatline_frame;
            }
        }

        // Generate frame based on current state and check for underrun
        // Underrun detection info (passage_id, position to check)
        let mut underrun_check: Option<(Uuid, usize, AudioFrame)> = None;

        let frame = match &mut self.state {
            MixerState::None => AudioFrame::zero(),

            MixerState::SinglePassage {
                buffer,
                passage_id,
                position,
                fade_in_curve,
                fade_in_duration_samples,
                ..
            } => {
                // Read frame from buffer
                let mut frame = read_frame(buffer, *position).await;

                // Apply fade-in if active
                if let Some(curve) = fade_in_curve {
                    if *position < *fade_in_duration_samples {
                        let fade_position = *position as f32 / *fade_in_duration_samples as f32;
                        let multiplier = curve.calculate_fade_in(fade_position);
                        frame.apply_volume(multiplier);
                    }
                }

                // **[SSD-UND-010]** Save info for underrun check after match
                let current_passage_id = *passage_id;

                *position += 1;

                // Save underrun check data (will check after match completes)
                underrun_check = Some((current_passage_id, *position, frame));

                frame
            }

            MixerState::Crossfading {
                current_buffer,
                current_passage_id,
                current_position,
                fade_out_curve,
                fade_out_duration_samples,
                fade_out_progress,
                next_buffer,
                next_passage_id,
                next_position,
                fade_in_curve,
                fade_in_duration_samples,
                fade_in_progress,
            } => {
                // Read from both buffers
                let mut current_frame = read_frame(current_buffer, *current_position).await;
                let mut next_frame = read_frame(next_buffer, *next_position).await;

                // Calculate fade positions (0.0 to 1.0)
                let fade_out_pos = *fade_out_progress as f32 / *fade_out_duration_samples as f32;
                let fade_in_pos = *fade_in_progress as f32 / *fade_in_duration_samples as f32;

                // Apply fade curves
                let fade_out_mult = fade_out_curve.calculate_fade_out(fade_out_pos);
                let fade_in_mult = fade_in_curve.calculate_fade_in(fade_in_pos);

                current_frame.apply_volume(fade_out_mult);
                next_frame.apply_volume(fade_in_mult);

                // Mix (sum) the frames
                let mut mixed = current_frame;
                mixed.add(&next_frame);

                // Check for clipping and clamp
                // [SSD-CLIP-010] Clipping detection
                if mixed.left.abs() > 1.0 || mixed.right.abs() > 1.0 {
                    // Log warning (only once per crossfade)
                    // TODO: Add clipping warning log
                }
                mixed.clamp();

                // Advance positions
                *current_position += 1;
                *next_position += 1;
                *fade_out_progress += 1;
                *fade_in_progress += 1;

                // Check if crossfade complete
                if *fade_out_progress >= *fade_out_duration_samples
                    && *fade_in_progress >= *fade_in_duration_samples
                {
                    // **[XFD-COMP-010]** Capture outgoing passage ID BEFORE transition
                    let outgoing_passage_id = *current_passage_id;

                    // Transition to SinglePassage with next buffer
                    let new_passage_id = *next_passage_id;
                    let new_position = *next_position;
                    let new_buffer = next_buffer.clone();

                    self.state = MixerState::SinglePassage {
                        buffer: new_buffer,
                        passage_id: new_passage_id,
                        position: new_position,
                        fade_in_curve: None,
                        fade_in_duration_samples: 0,
                    };

                    // **[XFD-COMP-010]** Signal completion to engine
                    self.crossfade_completed_passage = Some(outgoing_passage_id);

                    tracing::debug!(
                        "[XFD-COMP-010] Crossfade completed: outgoing={}, incoming={} (outgoing faded out)",
                        outgoing_passage_id, new_passage_id
                    );
                }

                mixed
            }
        };

        // **[SSD-UND-010]** Perform underrun detection after match completes (mutable borrow released)
        if let Some((passage_id, next_position, last_frame)) = underrun_check {
            // Detect if next position will cause underrun
            if self.detect_underrun(passage_id, next_position).await {
                // Enter underrun state
                self.log_underrun(passage_id, next_position).await;

                self.underrun_state = Some(UnderrunState {
                    passage_id,
                    flatline_frame: last_frame, // Save last frame for flatline output
                    started_at: Instant::now(),
                    position_frames: next_position - 1, // Current position before increment
                });

                warn!(
                    "[SSD-UND-016] Entering underrun pause: passage_id={}, position={}",
                    passage_id, next_position
                );
            }
        }

        // **[REV002]** Emit position events periodically
        // This runs after frame generation to include position in the event
        self.frame_counter += 1;

        if self.frame_counter >= self.position_event_interval_frames {
            self.frame_counter = 0; // Reset counter

            // Emit PositionUpdate event if channel configured
            if let Some(tx) = &self.event_tx {
                if let Some(passage_id) = self.get_current_passage_id() {
                    let position_ms = self.calculate_position_ms();

                    // Non-blocking send (use try_send to avoid blocking audio thread)
                    let _ = tx.send(PlaybackEvent::PositionUpdate {
                        queue_entry_id: passage_id,
                        position_ms,
                    });

                    trace!(
                        "Emitted PositionUpdate: passage={}, position={}ms",
                        passage_id,
                        position_ms
                    );
                }
            }
        }

        // **[XFD-PAUS-040]** Apply resume fade-in multiplicatively (AFTER all other processing)
        let mut final_frame = frame;
        if let Some(ref mut resume) = self.resume_state {
            if resume.frames_since_resume < resume.fade_in_duration_frames {
                // **[XFD-PAUS-030]** Calculate fade position (0.0 to 1.0)
                let fade_position = resume.frames_since_resume as f32
                                  / resume.fade_in_duration_frames as f32;

                // Calculate fade-in multiplier using curve
                let resume_fade_multiplier = resume.fade_in_curve.calculate_fade_in(fade_position);

                // Apply multiplicatively to frame
                final_frame.apply_volume(resume_fade_multiplier);

                resume.frames_since_resume += 1;

                trace!(
                    "[XFD-PAUS-030] Resume fade-in: position={:.3}, multiplier={:.3}",
                    fade_position, resume_fade_multiplier
                );
            } else {
                // Fade-in complete
                tracing::info!("[XFD-PAUS-020] Resume fade-in complete");
                self.resume_state = None;
            }
        }

        final_frame
    }

    /// Calculate current position in milliseconds
    ///
    /// **[REV002]** Helper for position event emission
    ///
    /// # Returns
    /// Current playback position in milliseconds
    fn calculate_position_ms(&self) -> u64 {
        let position_frames = self.get_position();
        (position_frames as u64 * 1000) / self.sample_rate as u64
    }

    /// Check if current passage finished
    ///
    /// **[SSD-MIX-060]** Passage completion detection
    /// **[PCF-COMP-010]** Uses is_exhausted() for race-free detection
    ///
    /// # Returns
    /// true if current passage has been fully consumed
    pub async fn is_current_finished(&self) -> bool {
        match &self.state {
            MixerState::SinglePassage { buffer, position, .. } => {
                // **[PCF-COMP-010]** Use is_exhausted() for race-free completion detection
                // Checks against cached total_frames set when decode completes,
                // not against growing sample_count
                if let Ok(buf) = buffer.try_read() {
                    buf.is_exhausted(*position)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Stop playback
    ///
    /// Transitions to None state, stopping all audio
    pub fn stop(&mut self) {
        self.state = MixerState::None;
        // **[XFD-COMP-010]** Clear any pending crossfade completion signal
        // (e.g., if user skips to next track during crossfade)
        self.crossfade_completed_passage = None;
    }

    /// Check if a crossfade just completed and consume the signal
    ///
    /// **[XFD-COMP-010]** Crossfade completion detection
    ///
    /// Returns the passage ID of the outgoing passage that finished fading out.
    /// This should be called before is_current_finished() in the engine loop.
    ///
    /// # Returns
    /// - `Some(passage_id)` if a crossfade just completed
    /// - `None` if no crossfade completion pending
    ///
    /// # Side Effects
    /// Clears the internal flag, so subsequent calls return None until
    /// the next crossfade completes.
    ///
    /// # Thread Safety
    /// This method requires `&mut self`, so it's naturally serialized by
    /// Rust's borrow checker. Only one thread can call this at a time.
    pub fn take_crossfade_completed(&mut self) -> Option<Uuid> {
        let result = self.crossfade_completed_passage.take();
        if result.is_some() {
            tracing::trace!(
                "[XFD-COMP-010] Crossfade completion flag consumed: passage_id={:?}",
                result
            );
        }
        result
    }

    /// Enter pause state
    ///
    /// **[XFD-PAUS-010]** Immediate pause with no fade-out.
    /// Audio output will be flatline silence until resume() is called.
    ///
    /// When paused, get_next_frame() returns AudioFrame::zero()
    pub fn pause(&mut self) {
        let current_position = match &self.state {
            MixerState::SinglePassage { position, .. } => *position,
            MixerState::Crossfading { next_position, .. } => *next_position,
            MixerState::None => 0,
        };

        self.pause_state = Some(PauseState {
            paused_at: Instant::now(),
            pause_position_frames: current_position,
        });

        tracing::info!("[XFD-PAUS-010] Mixer paused at frame {}", current_position);
    }

    /// Resume from pause with fade-in
    ///
    /// **[XFD-PAUS-020]** Resume with configurable fade-in curve and duration.
    ///
    /// # Arguments
    /// * `fade_in_duration_ms` - Fade-in duration in milliseconds (e.g., 500 for 0.5s)
    /// * `fade_in_curve_name` - Curve name: "linear", "exponential", or "cosine"
    ///
    /// # Behavior
    /// - Clears pause state
    /// - Enters resume state with fade-in from 0.0 to 1.0
    /// - get_next_frame() applies fade-in multiplicatively to mixed audio
    pub fn resume(&mut self, fade_in_duration_ms: u64, fade_in_curve_name: &str) {
        // Clear pause state
        if let Some(pause_state) = self.pause_state.take() {
            let pause_duration = pause_state.paused_at.elapsed();

            // Calculate fade-in duration in frames
            let fade_in_duration_frames = ((fade_in_duration_ms as f32 / 1000.0)
                                          * self.sample_rate as f32) as usize;

            // Parse fade curve (linear or exponential)
            let fade_in_curve = match fade_in_curve_name {
                "linear" => FadeCurve::Linear,
                "exponential" => FadeCurve::Exponential,
                _ => {
                    warn!("Unknown fade curve '{}', using exponential", fade_in_curve_name);
                    FadeCurve::Exponential
                }
            };

            // Enter resume state
            self.resume_state = Some(ResumeState {
                resumed_at: Instant::now(),
                fade_in_duration_frames,
                fade_in_curve,
                frames_since_resume: 0,
            });

            tracing::info!(
                "[XFD-PAUS-020] Resuming from pause (paused for {:?}), fade-in {}ms with {:?} curve",
                pause_duration, fade_in_duration_ms, fade_in_curve
            );
        } else {
            warn!("Resume called but mixer was not paused");
        }
    }

    /// Check if mixer is paused
    ///
    /// **[XFD-PAUS-010]** Query pause state
    ///
    /// # Returns
    /// true if paused, false if playing
    pub fn is_paused(&self) -> bool {
        self.pause_state.is_some()
    }

    /// Get current passage ID (if any)
    ///
    /// # Returns
    /// UUID of currently playing passage, or None
    pub fn get_current_passage_id(&self) -> Option<Uuid> {
        match &self.state {
            MixerState::SinglePassage { passage_id, .. } => Some(*passage_id),
            MixerState::Crossfading {
                current_passage_id, ..
            } => Some(*current_passage_id),
            MixerState::None => None,
        }
    }

    /// Get next passage ID (during crossfade)
    ///
    /// # Returns
    /// UUID of next passage during crossfade, or None
    pub fn get_next_passage_id(&self) -> Option<Uuid> {
        match &self.state {
            MixerState::Crossfading {
                next_passage_id, ..
            } => Some(*next_passage_id),
            _ => None,
        }
    }

    /// Get current playback position in samples
    ///
    /// # Returns
    /// Current frame position, or 0 if not playing
    pub fn get_position(&self) -> usize {
        match &self.state {
            MixerState::SinglePassage { position, .. } => *position,
            MixerState::Crossfading {
                current_position, ..
            } => *current_position,
            MixerState::None => 0,
        }
    }

    /// Get mixer state information for monitoring
    pub fn get_state_info(&self) -> MixerStateInfo {
        match &self.state {
            MixerState::None => MixerStateInfo {
                current_passage_id: None,
                next_passage_id: None,
                current_position_frames: 0,
                next_position_frames: 0,
                is_crossfading: false,
            },
            MixerState::SinglePassage { passage_id, position, .. } => MixerStateInfo {
                current_passage_id: Some(*passage_id),
                next_passage_id: None,
                current_position_frames: *position,
                next_position_frames: 0,
                is_crossfading: false,
            },
            MixerState::Crossfading {
                current_passage_id,
                current_position,
                next_passage_id,
                next_position,
                ..
            } => MixerStateInfo {
                current_passage_id: Some(*current_passage_id),
                next_passage_id: Some(*next_passage_id),
                current_position_frames: *current_position,
                next_position_frames: *next_position,
                is_crossfading: true,
            },
        }
    }

    /// Set playback position (seek)
    ///
    /// Updates the position in the current passage. Clamps to buffer bounds.
    /// Does nothing if mixer is idle.
    ///
    /// # Arguments
    /// * `position_frames` - Target position in frames
    ///
    /// # Returns
    /// * `Ok(())` if position updated
    /// * `Err` if position invalid or no passage playing
    ///
    /// **[SSD-ENG-026]** Seek position control
    pub async fn set_position(&mut self, position_frames: usize) -> crate::error::Result<()> {
        match &mut self.state {
            MixerState::SinglePassage {
                position, buffer, ..
            } => {
                // Clamp position to buffer bounds
                let buf = buffer.read().await;
                let max_position = buf.sample_count.saturating_sub(1);
                *position = position_frames.min(max_position);
                Ok(())
            }
            MixerState::Crossfading {
                current_position,
                current_buffer,
                ..
            } => {
                // During crossfade, only seek the current (fading out) passage
                // This maintains crossfade state while allowing position control
                let buf = current_buffer.read().await;
                let max_position = buf.sample_count.saturating_sub(1);
                *current_position = position_frames.min(max_position);
                Ok(())
            }
            MixerState::None => {
                Err(crate::error::Error::Playback(
                    "Cannot seek: no passage playing".to_string(),
                ))
            }
        }
    }

    /// Check if currently crossfading
    ///
    /// # Returns
    /// true if in Crossfading state
    pub fn is_crossfading(&self) -> bool {
        matches!(self.state, MixerState::Crossfading { .. })
    }

    // Helper methods

    /// Check if buffer has caught up and can resume from underrun
    ///
    /// **[SSD-UND-018]** Auto-resume when sufficient buffer available
    ///
    /// # Arguments
    /// * `passage_id` - Passage experiencing underrun
    /// * `position_frames` - Current playback position
    ///
    /// # Returns
    /// true if buffer has at least 1 second ahead of position
    async fn can_resume_from_underrun(&self, passage_id: Uuid, position_frames: usize) -> bool {
        if let Some(ref buffer_manager) = self.buffer_manager {
            // Get current buffer
            if let Some(buffer_arc) = buffer_manager.get_buffer(passage_id).await {
                let buffer = buffer_arc.read().await;

                // Calculate how much buffer is available ahead of position
                let available_frames = buffer.sample_count.saturating_sub(position_frames);
                let available_ms = (available_frames as u64 * 1000) / self.sample_rate as u64;

                // Need at least 1 second ahead
                available_ms >= UNDERRUN_RESUME_BUFFER_MS
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Detect underrun condition
    ///
    /// **[SSD-UND-010]** Detect when playback reaches unbuffered region
    ///
    /// # Arguments
    /// * `passage_id` - Passage being played
    /// * `position_frames` - Current playback position
    ///
    /// # Returns
    /// true if underrun detected (position >= buffer.sample_count but buffer still decoding)
    async fn detect_underrun(&self, passage_id: Uuid, position_frames: usize) -> bool {
        if let Some(ref buffer_manager) = self.buffer_manager {
            // Check buffer status
            if let Some(status) = buffer_manager.get_status(passage_id).await {
                // Only underrun if buffer is still Decoding (not Ready/Playing/Exhausted)
                if matches!(status, BufferStatus::Decoding { .. }) {
                    // Check if position has reached end of currently decoded samples
                    if let Some(buffer_arc) = buffer_manager.get_buffer(passage_id).await {
                        let buffer = buffer_arc.read().await;
                        return position_frames >= buffer.sample_count;
                    }
                }
            }
        }
        false
    }

    /// Log underrun diagnostics
    ///
    /// **[SSD-UND-011]** to **[SSD-UND-015]** Diagnostic logging
    async fn log_underrun(&self, passage_id: Uuid, position_frames: usize) {
        if let Some(ref buffer_manager) = self.buffer_manager {
            let status = buffer_manager.get_status(passage_id).await;
            let decode_elapsed = buffer_manager.get_decode_elapsed(passage_id).await;

            let position_ms = (position_frames as u64 * 1000) / self.sample_rate as u64;

            warn!(
                "[SSD-UND-011] Buffer underrun detected: passage_id={}, position={}ms ({} frames)",
                passage_id, position_ms, position_frames
            );

            // [SSD-UND-012] Current buffer status
            if let Some(status) = status {
                warn!("[SSD-UND-012] Buffer status: {:?}", status);
            }

            // [SSD-UND-014] Decode speed
            if let Some(elapsed) = decode_elapsed {
                let decode_ms = elapsed.as_millis() as u64;
                let speed_ratio = if decode_ms > 0 {
                    (position_ms as f64) / (decode_ms as f64)
                } else {
                    0.0
                };
                warn!(
                    "[SSD-UND-014] Decode speed: {} ms decoded in {} ms (ratio: {:.2}x realtime)",
                    position_ms, decode_ms, speed_ratio
                );
            }

            // [SSD-UND-015] Note about pre-buffering
            warn!("[SSD-UND-015] Consider extending pre-buffering time for this passage");
        }
    }
}

/// Read frame from buffer at position
///
/// Returns zero frame if position out of bounds or buffer locked
async fn read_frame(buffer: &Arc<RwLock<PassageBuffer>>, position: usize) -> AudioFrame {
    if let Ok(buf) = buffer.try_read() {
        buf.get_frame(position).unwrap_or_else(AudioFrame::zero)
    } else {
        AudioFrame::zero()
    }
}

impl Default for CrossfadeMixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::types::PassageBuffer;

    /// Create a test buffer with sine wave samples
    ///
    /// **[PCF-DUR-010][PCF-COMP-010]** Test buffers are finalized (simulates completed decode)
    fn create_test_buffer(passage_id: Uuid, sample_count: usize, amplitude: f32) -> Arc<RwLock<PassageBuffer>> {
        let mut samples = Vec::with_capacity(sample_count * 2);
        for i in 0..sample_count {
            let value = amplitude * (i as f32 * 0.01).sin();
            samples.push(value); // left
            samples.push(value); // right
        }

        let mut buffer = PassageBuffer::new(
            passage_id,
            samples,
            44100,
            2,
        );

        // Finalize buffer (test buffers are complete, like a finished decode)
        buffer.finalize();

        Arc::new(RwLock::new(buffer))
    }

    #[tokio::test]
    async fn test_mixer_creation() {
        let mixer = CrossfadeMixer::new();
        assert_eq!(mixer.sample_rate, 44100);
        assert!(mixer.get_current_passage_id().is_none());
    }

    #[tokio::test]
    async fn test_start_passage_no_fade() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 1000, 0.5);

        mixer
            .start_passage(buffer, passage_id, None, 0)
            .await;

        assert_eq!(mixer.get_current_passage_id(), Some(passage_id));
        assert_eq!(mixer.get_position(), 0);
        assert!(!mixer.is_crossfading());
    }

    #[tokio::test]
    async fn test_start_passage_with_fade_in() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 1000, 0.5);

        mixer
            .start_passage(buffer, passage_id, Some(FadeCurve::Linear), 100)
            .await;

        assert_eq!(mixer.get_current_passage_id(), Some(passage_id));

        // First frame should be silent (fade-in start)
        let frame = mixer.get_next_frame().await;
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }

    #[tokio::test]
    async fn test_single_passage_playback() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 100, 0.5);

        mixer
            .start_passage(buffer, passage_id, None, 0)
            .await;

        // Read 50 frames
        for _ in 0..50 {
            let frame = mixer.get_next_frame().await;
            assert!(frame.left.abs() <= 1.0);
            assert!(frame.right.abs() <= 1.0);
        }

        assert_eq!(mixer.get_position(), 50);
        assert!(!mixer.is_current_finished().await);

        // Read remaining frames
        for _ in 0..50 {
            mixer.get_next_frame().await;
        }

        assert_eq!(mixer.get_position(), 100);
        assert!(mixer.is_current_finished().await);
    }

    #[tokio::test]
    async fn test_start_crossfade() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id_1 = Uuid::new_v4();
        let passage_id_2 = Uuid::new_v4();
        let buffer1 = create_test_buffer(passage_id_1, 1000, 0.5);
        let buffer2 = create_test_buffer(passage_id_2, 1000, 0.5);

        // Start first passage
        mixer
            .start_passage(buffer1, passage_id_1, None, 0)
            .await;

        // Start crossfade
        let result = mixer
            .start_crossfade(
                buffer2,
                passage_id_2,
                FadeCurve::Linear,
                1000,
                FadeCurve::Linear,
                1000,
            )
            .await;

        assert!(result.is_ok());
        assert!(mixer.is_crossfading());
        assert_eq!(mixer.get_current_passage_id(), Some(passage_id_1));
        assert_eq!(mixer.get_next_passage_id(), Some(passage_id_2));
    }

    #[tokio::test]
    async fn test_crossfade_mixing() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id_1 = Uuid::new_v4();
        let passage_id_2 = Uuid::new_v4();
        let buffer1 = create_test_buffer(passage_id_1, 1000, 0.5);
        let buffer2 = create_test_buffer(passage_id_2, 1000, 0.5);

        // Start first passage
        mixer
            .start_passage(buffer1, passage_id_1, None, 0)
            .await;

        // Start crossfade with 100ms duration (4410 samples)
        mixer
            .start_crossfade(
                buffer2,
                passage_id_2,
                FadeCurve::Linear,
                100,
                FadeCurve::Linear,
                100,
            )
            .await
            .unwrap();

        // Read frames during crossfade
        let crossfade_samples = 4410;
        for _ in 0..crossfade_samples {
            let frame = mixer.get_next_frame().await;
            // Frames should be within valid range
            assert!(frame.left.abs() <= 1.0);
            assert!(frame.right.abs() <= 1.0);
        }

        // Crossfade should be complete, now on second passage
        assert!(!mixer.is_crossfading());
        assert_eq!(mixer.get_current_passage_id(), Some(passage_id_2));
    }

    #[tokio::test]
    async fn test_crossfade_transition_to_single_passage() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id_1 = Uuid::new_v4();
        let passage_id_2 = Uuid::new_v4();
        let buffer1 = create_test_buffer(passage_id_1, 100, 0.5);
        let buffer2 = create_test_buffer(passage_id_2, 100, 0.5);

        mixer
            .start_passage(buffer1, passage_id_1, None, 0)
            .await;

        mixer
            .start_crossfade(
                buffer2,
                passage_id_2,
                FadeCurve::Linear,
                10,
                FadeCurve::Linear,
                10,
            )
            .await
            .unwrap();

        // 10ms at 44100 = 441 samples
        for _ in 0..441 {
            mixer.get_next_frame().await;
        }

        // Should transition to single passage
        assert!(!mixer.is_crossfading());
        assert_eq!(mixer.get_current_passage_id(), Some(passage_id_2));
    }

    #[tokio::test]
    async fn test_stop() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 1000, 0.5);

        mixer
            .start_passage(buffer, passage_id, None, 0)
            .await;

        assert!(mixer.get_current_passage_id().is_some());

        mixer.stop();

        assert!(mixer.get_current_passage_id().is_none());
        assert_eq!(mixer.get_position(), 0);

        // Next frame should be silent
        let frame = mixer.get_next_frame().await;
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }

    #[tokio::test]
    async fn test_start_crossfade_invalid_state() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 1000, 0.5);

        // Try to start crossfade when nothing is playing
        let result = mixer
            .start_crossfade(
                buffer,
                passage_id,
                FadeCurve::Linear,
                1000,
                FadeCurve::Linear,
                1000,
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ticks_to_samples_conversion() {
        use wkmp_common::timing::{ms_to_ticks, ticks_to_samples};

        // 1 second = 44100 samples
        let one_sec_ticks = ms_to_ticks(1000);
        assert_eq!(ticks_to_samples(one_sec_ticks, 44100), 44100);

        // 100ms = 4410 samples
        let hundred_ms_ticks = ms_to_ticks(100);
        assert_eq!(ticks_to_samples(hundred_ms_ticks, 44100), 4410);

        // 10ms = 441 samples
        let ten_ms_ticks = ms_to_ticks(10);
        assert_eq!(ticks_to_samples(ten_ms_ticks, 44100), 441);
    }

    #[tokio::test]
    async fn test_set_position() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 1000, 0.5);

        // Start passage
        mixer.start_passage(buffer, passage_id, None, 0).await;
        assert_eq!(mixer.get_position(), 0);

        // Seek to position 100
        mixer.set_position(100).await.unwrap();
        assert_eq!(mixer.get_position(), 100);

        // Seek to position 500
        mixer.set_position(500).await.unwrap();
        assert_eq!(mixer.get_position(), 500);

        // Seek beyond buffer (should clamp to max-1)
        mixer.set_position(5000).await.unwrap();
        assert_eq!(mixer.get_position(), 999); // buffer has 1000 frames (0-999)

        // Seek back to 0
        mixer.set_position(0).await.unwrap();
        assert_eq!(mixer.get_position(), 0);
    }

    #[tokio::test]
    async fn test_set_position_no_passage() {
        let mut mixer = CrossfadeMixer::new();

        // Try to seek when nothing is playing
        let result = mixer.set_position(100).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_position_during_crossfade() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id_1 = Uuid::new_v4();
        let passage_id_2 = Uuid::new_v4();
        let buffer1 = create_test_buffer(passage_id_1, 1000, 0.5);
        let buffer2 = create_test_buffer(passage_id_2, 1000, 0.5);

        // Start first passage and play a bit
        mixer.start_passage(buffer1, passage_id_1, None, 0).await;
        for _ in 0..100 {
            mixer.get_next_frame().await;
        }

        // Start crossfade
        mixer
            .start_crossfade(
                buffer2,
                passage_id_2,
                FadeCurve::Linear,
                1000,
                FadeCurve::Linear,
                1000,
            )
            .await
            .unwrap();

        assert!(mixer.is_crossfading());

        // Seek during crossfade (should seek the current/fading out passage)
        mixer.set_position(200).await.unwrap();
        assert_eq!(mixer.get_position(), 200);
    }

    // --- REV004: Tests for buffer underrun detection ---

    #[tokio::test]
    async fn test_set_buffer_manager() {
        let mut mixer = CrossfadeMixer::new();
        let buffer_manager = Arc::new(BufferManager::new());

        // Should be able to set buffer manager
        mixer.set_buffer_manager(Arc::clone(&buffer_manager));

        // Verify it's set (indirectly, by checking underrun detection works)
        // This is tested in the underrun detection tests below
    }

    #[tokio::test]
    async fn test_no_underrun_with_full_buffer() {
        let mut mixer = CrossfadeMixer::new();
        let buffer_manager = Arc::new(BufferManager::new());
        mixer.set_buffer_manager(Arc::clone(&buffer_manager));

        let passage_id = Uuid::new_v4();

        // Create full buffer (100 frames)
        let buffer_handle = buffer_manager.register_decoding(passage_id).await;
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.5; 200]);  // 100 stereo frames
        }
        buffer_manager.mark_ready(passage_id).await;

        // Start playback
        let buffer_arc = buffer_manager.get_buffer(passage_id).await.unwrap();
        mixer.start_passage(buffer_arc, passage_id, None, 0).await;

        // Read all 100 frames - should all succeed without underrun
        for _ in 0..100 {
            let frame = mixer.get_next_frame().await;
            // Should get actual data, not flatline
            assert_eq!(frame.left, 0.5);
            assert_eq!(frame.right, 0.5);
        }

        // At this point we've exhausted the buffer, but not during decoding
        // so no underrun state should have been triggered
    }

    #[tokio::test]
    async fn test_underrun_detection_with_partial_buffer() {
        let mut mixer = CrossfadeMixer::new();
        let buffer_manager = Arc::new(BufferManager::new());
        mixer.set_buffer_manager(Arc::clone(&buffer_manager));

        let passage_id = Uuid::new_v4();

        // Create partial buffer (50 frames) that's still decoding
        let buffer_handle = buffer_manager.register_decoding(passage_id).await;
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.5; 100]);  // 50 stereo frames
        }
        // Note: NOT marking as ready - still in Decoding state

        // Start playback with buffer manager
        let buffer_arc = buffer_manager.get_buffer(passage_id).await.unwrap();
        mixer.start_passage(buffer_arc, passage_id, None, 0).await;

        // Read 50 frames - should work (within buffer)
        for i in 0..50 {
            let frame = mixer.get_next_frame().await;
            assert_eq!(frame.left, 0.5, "Frame {} should be valid", i);
            assert_eq!(frame.right, 0.5);
        }

        // Next frame (51st) would trigger underrun detection
        // Position 50 >= sample_count 50, and buffer status is Decoding
        let frame = mixer.get_next_frame().await;

        // Should output flatline (last valid frame)
        // Last valid frame was 0.5, 0.5
        assert_eq!(frame.left, 0.5, "Should output flatline of last valid frame");
        assert_eq!(frame.right, 0.5);
    }

    #[tokio::test]
    async fn test_underrun_auto_resume() {
        let mut mixer = CrossfadeMixer::new();
        let buffer_manager = Arc::new(BufferManager::new());
        mixer.set_buffer_manager(Arc::clone(&buffer_manager));

        let passage_id = Uuid::new_v4();

        // Create partial buffer (50 frames)
        let buffer_handle = buffer_manager.register_decoding(passage_id).await;
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.5; 100]);  // 50 stereo frames
        }

        // Start playback
        let buffer_arc = buffer_manager.get_buffer(passage_id).await.unwrap();
        mixer.start_passage(buffer_arc, passage_id, None, 0).await;

        // Consume all 50 frames
        for _ in 0..50 {
            mixer.get_next_frame().await;
        }

        // Trigger underrun (position 50 >= sample_count 50)
        let underrun_frame = mixer.get_next_frame().await;
        assert_eq!(underrun_frame.left, 0.5);  // Flatline

        // Still in underrun - get flatline again
        let underrun_frame2 = mixer.get_next_frame().await;
        assert_eq!(underrun_frame2.left, 0.5);  // Still flatline

        // Now append enough data to resume (need 1 second ahead = 44100 frames)
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.7; 88200]);  // 44100 more frames
        }

        // Next frame should auto-resume
        let resumed_frame = mixer.get_next_frame().await;
        // Should now read from position 50 (where we left off)
        assert_eq!(resumed_frame.left, 0.7, "Should auto-resume with new data");
        assert_eq!(resumed_frame.right, 0.7);
    }

    #[tokio::test]
    async fn test_underrun_during_decoding_only() {
        let mut mixer = CrossfadeMixer::new();
        let buffer_manager = Arc::new(BufferManager::new());
        mixer.set_buffer_manager(Arc::clone(&buffer_manager));

        let passage_id = Uuid::new_v4();

        // Create buffer and mark as Ready (not Decoding)
        let buffer_handle = buffer_manager.register_decoding(passage_id).await;
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.5; 100]);  // 50 stereo frames
        }
        buffer_manager.mark_ready(passage_id).await;  // Mark as Ready

        // Finalize the buffer to set total_samples (required for proper EOF detection)
        buffer_manager.finalize_buffer(passage_id, 100).await.unwrap();

        // Start playback (buffer now in Finished state after finalize)
        buffer_manager.start_playback(passage_id).await.unwrap();
        let buffer_arc = buffer_manager.get_buffer(passage_id).await.unwrap();
        mixer.start_passage(buffer_arc, passage_id, None, 0).await;

        // Consume all frames
        for _ in 0..50 {
            mixer.get_next_frame().await;
        }

        // Try to read beyond buffer (position 50 >= sample_count 50)
        // Buffer is now Finished (not Decoding), so no underrun detection
        let frame = mixer.get_next_frame().await;

        // Should get silence (normal end-of-buffer behavior)
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }

    // ========== PAUSE/RESUME TESTS ==========
    // [XFD-PAUS-010] [XFD-PAUS-020] Test coverage for pause/resume functionality

    #[tokio::test]
    async fn test_pause_sets_pause_state() {
        let mut mixer = CrossfadeMixer::new();

        // Start a passage
        let passage_id = Uuid::new_v4();
        let samples = vec![0.5_f32; 88200]; // 1 second of audio
        let buffer = PassageBuffer::new(passage_id, samples, 44100, 2);
        let buffer = Arc::new(RwLock::new(buffer));

        mixer.start_passage(buffer, passage_id, None, 0).await;

        // Verify not paused initially
        assert!(!mixer.is_paused());

        // Pause
        mixer.pause();

        // Verify paused
        assert!(mixer.is_paused());
    }

    #[tokio::test]
    async fn test_pause_during_crossfade() {
        let mut mixer = CrossfadeMixer::new();

        // Start first passage
        let passage_id_1 = Uuid::new_v4();
        let samples1 = vec![0.5_f32; 88200];
        let buffer1 = Arc::new(RwLock::new(PassageBuffer::new(passage_id_1, samples1, 44100, 2)));
        mixer.start_passage(buffer1, passage_id_1, None, 0).await;

        // Start crossfade
        let passage_id_2 = Uuid::new_v4();
        let samples2 = vec![0.3_f32; 88200];
        let buffer2 = Arc::new(RwLock::new(PassageBuffer::new(passage_id_2, samples2, 44100, 2)));
        mixer.start_crossfade(buffer2, passage_id_2, FadeCurve::Linear, 1000, FadeCurve::Linear, 1000).await.unwrap();

        // Pause during crossfade
        mixer.pause();

        // Verify paused
        assert!(mixer.is_paused());
    }

    #[tokio::test]
    async fn test_resume_clears_pause_state() {
        let mut mixer = CrossfadeMixer::new();

        // Start passage and pause
        let passage_id = Uuid::new_v4();
        let samples = vec![0.5_f32; 88200];
        let buffer = Arc::new(RwLock::new(PassageBuffer::new(passage_id, samples, 44100, 2)));
        mixer.start_passage(buffer, passage_id, None, 0).await;
        mixer.pause();
        assert!(mixer.is_paused());

        // Resume
        mixer.resume(500, "exponential");

        // Verify not paused
        assert!(!mixer.is_paused());
    }

    #[tokio::test]
    async fn test_get_next_frame_outputs_silence_when_paused() {
        let mut mixer = CrossfadeMixer::new();

        // Start passage
        let passage_id = Uuid::new_v4();
        let samples = vec![0.5_f32; 88200];
        let buffer = Arc::new(RwLock::new(PassageBuffer::new(passage_id, samples, 44100, 2)));
        mixer.start_passage(buffer, passage_id, None, 0).await;

        // Verify audio output before pause
        let frame_before = mixer.get_next_frame().await;
        assert!(frame_before.left != 0.0 || frame_before.right != 0.0);

        // Pause
        mixer.pause();

        // Verify silence output after pause
        let frame_paused = mixer.get_next_frame().await;
        assert_eq!(frame_paused.left, 0.0);
        assert_eq!(frame_paused.right, 0.0);
    }

    #[tokio::test]
    async fn test_get_next_frame_silence_persists_while_paused() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let samples = vec![0.5_f32; 88200];
        let buffer = Arc::new(RwLock::new(PassageBuffer::new(passage_id, samples, 44100, 2)));
        mixer.start_passage(buffer, passage_id, None, 0).await;
        mixer.pause();

        // Generate 100 frames while paused
        for _ in 0..100 {
            let frame = mixer.get_next_frame().await;
            assert_eq!(frame.left, 0.0);
            assert_eq!(frame.right, 0.0);
        }
    }

    #[tokio::test]
    async fn test_resume_fade_in_starts_at_zero() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let samples = vec![1.0_f32; 88200]; // Full volume samples
        let buffer = Arc::new(RwLock::new(PassageBuffer::new(passage_id, samples, 44100, 2)));
        mixer.start_passage(buffer, passage_id, None, 0).await;
        mixer.pause();
        mixer.resume(500, "exponential");

        // First frame after resume should be silent (fade multiplier = 0)
        let first_frame = mixer.get_next_frame().await;
        assert_eq!(first_frame.left, 0.0);
        assert_eq!(first_frame.right, 0.0);
    }

    #[tokio::test]
    async fn test_resume_fade_in_increases_over_time() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let samples = vec![1.0_f32; 88200]; // Full volume samples
        let buffer = Arc::new(RwLock::new(PassageBuffer::new(passage_id, samples, 44100, 2)));
        mixer.start_passage(buffer, passage_id, None, 0).await;
        mixer.pause();
        mixer.resume(100, "exponential"); // 100ms fade = 4410 frames @ 44.1kHz

        let mut prev_volume = 0.0_f32;

        // Check that volume increases over fade duration (100ms = 4410 frames)
        // Loop beyond fade duration to ensure it completes
        for i in 0..5000 {
            let frame = mixer.get_next_frame().await;
            let volume = frame.left.abs();

            if i > 0 {
                // Volume should be increasing (or stable at 1.0 after fade completes)
                assert!(volume >= prev_volume - 0.001,
                    "Volume decreased at frame {}: {} -> {}", i, prev_volume, volume);
            }

            prev_volume = volume;
        }

        // After fade completes, should be at full volume
        assert!((prev_volume - 1.0).abs() < 0.01,
            "Final volume {} is not close to 1.0", prev_volume);
    }

    #[tokio::test]
    async fn test_resume_fade_in_reaches_full_volume() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let samples = vec![1.0_f32; 88200];
        let buffer = Arc::new(RwLock::new(PassageBuffer::new(passage_id, samples, 44100, 2)));
        mixer.start_passage(buffer, passage_id, None, 0).await;
        mixer.pause();
        mixer.resume(500, "exponential"); // 500ms = 22050 frames @ 44.1kHz

        // Generate frames for fade duration
        for _ in 0..22050 {
            mixer.get_next_frame().await;
        }

        // After fade completes, should output full volume
        let frame_after = mixer.get_next_frame().await;
        assert!((frame_after.left - 1.0).abs() < 0.01);
        assert!((frame_after.right - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_resume_fade_in_linear_curve() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let samples = vec![1.0_f32; 88200];
        let buffer = Arc::new(RwLock::new(PassageBuffer::new(passage_id, samples, 44100, 2)));
        mixer.start_passage(buffer, passage_id, None, 0).await;
        mixer.pause();
        mixer.resume(100, "linear"); // 100ms = ~4410 frames

        // At 50% of fade duration, linear curve should be at ~0.5 volume
        for _ in 0..(4410 / 2) {
            mixer.get_next_frame().await;
        }

        let frame_mid = mixer.get_next_frame().await;
        // Linear: v(t) = t, so at t=0.5, v = 0.5
        assert!((frame_mid.left - 0.5).abs() < 0.1); // Allow some tolerance
    }

    #[tokio::test]
    async fn test_resume_fade_in_exponential_curve() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let samples = vec![1.0_f32; 88200];
        let buffer = Arc::new(RwLock::new(PassageBuffer::new(passage_id, samples, 44100, 2)));
        mixer.start_passage(buffer, passage_id, None, 0).await;
        mixer.pause();
        mixer.resume(100, "exponential"); // 100ms

        // At 50% of fade duration, exponential curve should be at ~0.25 volume
        for _ in 0..(4410 / 2) {
            mixer.get_next_frame().await;
        }

        let frame_mid = mixer.get_next_frame().await;
        // Exponential: v(t) = t², so at t=0.5, v = 0.25
        assert!((frame_mid.left - 0.25).abs() < 0.1);
    }

    // Edge case tests
    #[tokio::test]
    async fn test_multiple_pause_resume_cycles() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let samples = vec![0.5_f32; 88200];
        let buffer = Arc::new(RwLock::new(PassageBuffer::new(passage_id, samples, 44100, 2)));
        mixer.start_passage(buffer, passage_id, None, 0).await;

        // Cycle pause/resume 5 times
        for _ in 0..5 {
            mixer.pause();
            assert!(mixer.is_paused());

            mixer.resume(100, "exponential");
            assert!(!mixer.is_paused());

            // Generate some frames
            for _ in 0..100 {
                mixer.get_next_frame().await;
            }
        }
    }

    #[tokio::test]
    async fn test_pause_when_already_paused() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let samples = vec![0.5_f32; 88200];
        let buffer = Arc::new(RwLock::new(PassageBuffer::new(passage_id, samples, 44100, 2)));
        mixer.start_passage(buffer, passage_id, None, 0).await;

        // Pause twice
        mixer.pause();
        mixer.pause(); // Should not panic or cause issues

        assert!(mixer.is_paused());
    }

    #[tokio::test]
    async fn test_pause_with_no_passage() {
        let mut mixer = CrossfadeMixer::new();

        // Pause with no passage loaded (should not panic)
        mixer.pause();
        assert!(mixer.is_paused());
    }

    #[tokio::test]
    async fn test_resume_fade_in_during_crossfade() {
        let mut mixer = CrossfadeMixer::new();

        // Start first passage
        let passage_id_1 = Uuid::new_v4();
        let samples1 = vec![0.5_f32; 88200];
        let buffer1 = Arc::new(RwLock::new(PassageBuffer::new(passage_id_1, samples1, 44100, 2)));
        mixer.start_passage(buffer1, passage_id_1, None, 0).await;

        // Start crossfade
        let passage_id_2 = Uuid::new_v4();
        let samples2 = vec![0.3_f32; 88200];
        let buffer2 = Arc::new(RwLock::new(PassageBuffer::new(passage_id_2, samples2, 44100, 2)));
        mixer.start_crossfade(buffer2, passage_id_2, FadeCurve::Linear, 1000, FadeCurve::Linear, 1000).await.unwrap();

        // Pause and resume during crossfade
        mixer.pause();
        mixer.resume(500, "exponential");

        // Resume fade-in should work during crossfade
        let first_frame = mixer.get_next_frame().await;
        assert_eq!(first_frame.left, 0.0); // Should start at silence

        // Generate more frames - should fade in
        for _ in 0..100 {
            mixer.get_next_frame().await;
        }

        let later_frame = mixer.get_next_frame().await;
        assert!(later_frame.left > 0.0); // Should have faded in
    }
}
