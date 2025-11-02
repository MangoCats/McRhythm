//! Audio mixer for crossfade overlap and master volume
//!
//! Implements mixer per SPEC016 DBD-MIX-010.
//!
//! # Architecture
//!
//! Per SPEC016 DBD-MIX-041:
//! - Reads **pre-faded samples** from buffers (Fader applied curves before buffering)
//! - Sums overlapping samples (simple addition - no runtime fade calculations)
//! - Applies master volume
//! - Outputs single stream for audio device
//!
//! # Architectural Separation
//!
//! Per SPEC016 DBD-MIX-042:
//! - **Fader**: Applies passage-specific fade curves BEFORE buffering
//! - **Buffer**: Stores pre-faded samples
//! - **Mixer**: Reads pre-faded samples, sums overlaps, applies master volume
//!
//! # Phase 5 Scope
//!
//! Phase 5: Basic mixing (single passage, master volume)
//! Phase 6+: Crossfade overlap, pause mode decay, advanced features

use crate::error::{Error, Result};
use crate::playback::buffer_manager::BufferManager;
use wkmp_common::FadeCurve;
use std::collections::BinaryHeap;
use std::cmp::{Ordering, Reverse};
use std::sync::Arc;
use uuid::Uuid;

/// Audio mixer state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MixerState {
    /// Mixer is playing (reading from buffers)
    Playing,

    /// Mixer is paused (outputting silence)
    Paused,
}

/// Position marker for event-driven signaling
///
/// PlaybackEngine calculates when events should occur (tick count),
/// Mixer signals when those ticks are actually reached during playback.
///
/// # Event-Driven Architecture
///
/// - **PlaybackEngine (Calculation Layer):** Determines WHAT and WHEN
/// - **Mixer (Execution Layer):** Executes and signals when events occur
///
/// # Example
///
/// ```ignore
/// // Engine calculates crossfade start point
/// let crossfade_start_tick = end_tick - crossfade_duration_tick;
///
/// // Engine tells mixer about this point
/// mixer.add_marker(PositionMarker {
///     tick: crossfade_start_tick,
///     passage_id: current_passage_id,
///     event_type: MarkerEvent::StartCrossfade { next_passage_id },
/// });
///
/// // Later, mixer signals when tick reached
/// let events = mixer.mix_single(&mut buffer, &mut output)?;
/// // events contains: [MarkerEvent::StartCrossfade { next_passage_id }]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarker {
    /// Tick count when this marker should trigger
    /// (relative to passage start, in file ticks)
    pub tick: i64,

    /// Which passage this marker belongs to
    pub passage_id: Uuid,

    /// What event to signal when reached
    pub event_type: MarkerEvent,
}

impl Ord for PositionMarker {
    fn cmp(&self, other: &Self) -> Ordering {
        // Sort by tick (earliest first)
        self.tick.cmp(&other.tick)
    }
}

impl PartialOrd for PositionMarker {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Event types that mixer can signal
///
/// Mixer signals these events when position markers are reached during playback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerEvent {
    /// Position update milestone reached
    ///
    /// Engine uses this for periodic position reporting (e.g., every 5 seconds).
    PositionUpdate {
        /// Position in milliseconds (for convenience)
        position_ms: u64,
    },

    /// Start crossfade to next passage
    ///
    /// Mixer handles this internally by beginning crossfade mixing.
    StartCrossfade {
        /// Next passage to crossfade to
        next_passage_id: Uuid,
    },

    /// Song boundary crossed (for multi-song passages)
    ///
    /// Engine uses this to emit CurrentSongChanged events.
    SongBoundary {
        /// New song ID (None if exiting last song)
        new_song_id: Option<Uuid>,
    },

    /// Passage playback completed
    ///
    /// Signals that passage has been fully mixed to output.
    PassageComplete,

    /// End of file reached
    ///
    /// **[REQ-MIX-EOF-001]** Emitted when mixer reaches EOF in audio file.
    /// Includes any markers that were set for ticks beyond EOF (unreachable).
    ///
    /// Engine uses this to:
    /// - Handle unreachable events appropriately
    /// - Start next passage if available
    EndOfFile {
        /// Markers that were set beyond EOF and never reached
        unreachable_markers: Vec<PositionMarker>,
    },

    /// End of file reached before planned crossfade point
    ///
    /// **[REQ-MIX-EOF-002]** Emitted when EOF occurs before calculated lead-out.
    /// Indicates next passage should start immediately (no wait for crossfade point).
    EndOfFileBeforeLeadOut {
        /// Tick where crossfade was planned to start
        planned_crossfade_tick: i64,
        /// Markers that were set beyond EOF and never reached
        unreachable_markers: Vec<PositionMarker>,
    },
}

/// Resume fade-in state
///
/// Tracks fade-in progress after resuming from pause.
/// Per SPEC016 [DBD-MIX-040], this is a mixer-level fade applied multiplicatively
/// to the final mixed output (orthogonal to passage-level fades applied by Fader).
#[derive(Debug, Clone)]
struct ResumeState {
    /// Fade-in duration in samples
    ///
    /// Example: 0.5s * 44100 = 22050 samples
    fade_duration_samples: usize,

    /// Fade-in curve (linear, exponential, cosine, etc.)
    fade_in_curve: FadeCurve,

    /// Number of samples processed since resume (for fade calculation)
    samples_since_resume: usize,
}

/// Audio mixer
///
/// Reads pre-faded samples from passage buffers, sums overlaps, applies master volume.
///
/// # Examples
///
/// ```ignore
/// let mut mixer = Mixer::new(1.0); // Master volume = 100%
///
/// // Mix samples for audio callback
/// mixer.mix_frame(&mut buffer)?;
/// ```
pub struct Mixer {
    /// Master volume (0.0 to 1.0)
    master_volume: f32,

    /// Mixer state (Playing/Paused)
    state: MixerState,

    /// Last output sample (for pause mode decay)
    last_sample_left: f32,
    last_sample_right: f32,

    /// Pause decay factor per SPEC016 DBD-PARAM-090
    pause_decay_factor: f32,

    /// Pause decay floor per SPEC016 DBD-PARAM-100
    pause_decay_floor: f32,

    /// Resume fade-in state
    ///
    /// Some when fading in after resume from pause, None otherwise.
    /// Per SPEC016 [DBD-MIX-040], this is a mixer-level fade (orthogonal to passage-level fades).
    resume_state: Option<ResumeState>,

    /// Position markers (min-heap sorted by tick, soonest first)
    ///
    /// PlaybackEngine adds markers, mixer pops when reached.
    /// Uses BinaryHeap<Reverse<T>> for min-heap (earliest tick at top).
    markers: BinaryHeap<Reverse<PositionMarker>>,

    /// Current tick count in current passage
    ///
    /// Incremented as samples are mixed. Used to check when markers reached.
    /// Tick = sample count in original file timeline.
    current_tick: i64,

    /// Current passage ID
    ///
    /// Used for marker validation (markers for old passages ignored).
    current_passage_id: Option<Uuid>,

    /// Current queue entry ID
    ///
    /// Used for buffer lookup in BufferManager (buffers are keyed by queue_entry_id).
    /// For ephemeral passages: passage_id is None, but queue_entry_id is always present.
    current_queue_entry_id: Option<Uuid>,

    /// Total frames written to output
    ///
    /// Used for accurate playback position reporting (accounting for buffering).
    frames_written: u64,
}

impl Mixer {
    /// Create new mixer
    ///
    /// # Arguments
    ///
    /// * `master_volume` - Master volume (0.0 to 1.0)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mixer = Mixer::new(1.0); // 100% volume
    /// let mixer = Mixer::new(0.5); // 50% volume
    /// ```
    pub fn new(master_volume: f32) -> Self {
        Self {
            master_volume: master_volume.clamp(0.0, 1.0),
            state: MixerState::Playing,
            last_sample_left: 0.0,
            last_sample_right: 0.0,
            pause_decay_factor: 0.96875, // 31/32 per SPEC016 DBD-PARAM-090
            pause_decay_floor: 0.0001778, // per SPEC016 DBD-PARAM-100
            resume_state: None,
            markers: BinaryHeap::new(),
            current_tick: 0,
            current_passage_id: None,
            current_queue_entry_id: None,
            frames_written: 0,
        }
    }

    /// Set master volume
    ///
    /// # Arguments
    ///
    /// * `volume` - New master volume (0.0 to 1.0), will be clamped
    ///
    /// # Examples
    ///
    /// ```ignore
    /// mixer.set_master_volume(0.75); // 75% volume
    /// ```
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Get master volume
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Set mixer state
    ///
    /// # Arguments
    ///
    /// * `state` - New state (Playing/Paused)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// mixer.set_state(MixerState::Paused);
    /// ```
    pub fn set_state(&mut self, state: MixerState) {
        self.state = state;
    }

    /// Get mixer state
    pub fn state(&self) -> MixerState {
        self.state
    }

    /// Start resume fade-in
    ///
    /// Initiates a fade-in from 0.0 to 1.0 applied multiplicatively to the final mixed output.
    /// This is a mixer-level fade (orthogonal to passage-level fades applied by Fader component).
    ///
    /// # Arguments
    ///
    /// * `fade_duration_samples` - Fade-in duration in samples (e.g., 22050 for 0.5s at 44.1kHz)
    /// * `fade_in_curve` - Fade curve to use (Linear, Exponential, etc.)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Start 500ms fade-in at 44.1kHz
    /// mixer.start_resume_fade(22050, FadeCurve::Exponential);
    /// ```
    pub fn start_resume_fade(&mut self, fade_duration_samples: usize, fade_in_curve: FadeCurve) {
        self.resume_state = Some(ResumeState {
            fade_duration_samples,
            fade_in_curve,
            samples_since_resume: 0,
        });
    }

    /// Check if currently fading in from resume
    ///
    /// # Returns
    ///
    /// `true` if resume fade-in is active, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if mixer.is_resume_fading() {
    ///     println!("Fading in from pause");
    /// }
    /// ```
    pub fn is_resume_fading(&self) -> bool {
        self.resume_state.is_some()
    }

    /// Add position marker
    ///
    /// PlaybackEngine calculates when events should occur (tick count),
    /// mixer signals when those ticks are reached during playback.
    ///
    /// # Arguments
    ///
    /// * `marker` - Position marker with tick, passage_id, and event type
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Engine calculates crossfade start point
    /// let crossfade_start_tick = end_tick - crossfade_duration_tick;
    ///
    /// // Tell mixer about this point
    /// mixer.add_marker(PositionMarker {
    ///     tick: crossfade_start_tick,
    ///     passage_id: current_passage_id,
    ///     event_type: MarkerEvent::StartCrossfade { next_passage_id },
    /// });
    /// ```
    pub fn add_marker(&mut self, marker: PositionMarker) {
        self.markers.push(Reverse(marker));
    }

    /// Clear all markers for a specific passage
    ///
    /// Used when skipping or stopping a passage to prevent stale markers from triggering.
    ///
    /// # Arguments
    ///
    /// * `passage_id` - Passage ID whose markers should be cleared
    pub fn clear_markers_for_passage(&mut self, passage_id: Uuid) {
        self.markers.retain(|m| m.0.passage_id != passage_id);
    }

    /// Clear all position markers
    ///
    /// Used when stopping playback or resetting mixer state.
    pub fn clear_all_markers(&mut self) {
        self.markers.clear();
    }

    /// Set current passage
    ///
    /// Resets tick counter and updates current passage ID.
    /// Call this when starting a new passage.
    ///
    /// # Arguments
    ///
    /// * `passage_id` - ID of passage being started
    /// * `start_tick` - Starting tick offset (usually 0, or non-zero if seeking)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Starting new passage from beginning
    /// mixer.set_current_passage(passage_id, queue_entry_id, 0);
    ///
    /// // Starting passage at 30 seconds (seeking)
    /// let seek_tick = wkmp_common::timing::ms_to_ticks(30000);
    /// mixer.set_current_passage(passage_id, queue_entry_id, seek_tick);
    /// ```
    pub fn set_current_passage(&mut self, passage_id: Uuid, queue_entry_id: Uuid, start_tick: i64) {
        self.current_passage_id = Some(passage_id);
        self.current_queue_entry_id = Some(queue_entry_id);
        self.current_tick = start_tick;
        self.state = MixerState::Playing; // Resume playback when new passage starts
    }

    /// Clear current passage (for stop operation)
    ///
    /// Sets current_passage_id and current_queue_entry_id to None, indicating no passage is active.
    /// Transitions mixer to Paused state to output exponential decay instead of errors.
    /// Typically called when stopping playback or when passage completes.
    ///
    /// [SUB-INC-4B] Added for PlaybackEngine integration
    pub fn clear_passage(&mut self) {
        self.current_passage_id = None;
        self.current_queue_entry_id = None;
        self.state = MixerState::Paused; // Output decay instead of error when no passage active
    }

    /// Get current tick position
    ///
    /// Returns current playback position in file ticks.
    pub fn get_current_tick(&self) -> i64 {
        self.current_tick
    }

    /// Get current passage ID
    ///
    /// Returns ID of currently playing passage, or None if no passage active.
    pub fn get_current_passage_id(&self) -> Option<Uuid> {
        self.current_passage_id
    }

    /// Get current queue entry ID
    ///
    /// Returns the queue_entry_id of the currently playing passage, or None if no passage active.
    ///
    /// [SUB-INC-4B] Added for event emission (PassageComplete needs queue_entry_id)
    pub fn get_current_queue_entry_id(&self) -> Option<Uuid> {
        self.current_queue_entry_id
    }

    /// Get total frames written to output
    ///
    /// Used for accurate playback position reporting (accounting for buffering).
    pub fn get_frames_written(&self) -> u64 {
        self.frames_written
    }

    /// Check markers and return triggered events
    ///
    /// Called internally by mix methods after updating current_tick.
    /// Pops all markers that have been reached and returns their events.
    fn check_markers(&mut self) -> Vec<MarkerEvent> {
        let mut events = Vec::new();

        // Process all markers up to current tick
        while let Some(marker_ref) = self.markers.peek() {
            if self.current_tick >= marker_ref.0.tick {
                let marker = self.markers.pop().unwrap().0;

                // Only process markers for current passage (ignore stale markers)
                if Some(marker.passage_id) == self.current_passage_id {
                    events.push(marker.event_type);
                }
            } else {
                break; // Markers sorted, stop checking
            }
        }

        events
    }

    /// Collect unreachable markers beyond current tick
    ///
    /// **[REQ-MIX-EOF-001]** Called when EOF detected to collect markers
    /// that were set for ticks beyond end of file.
    ///
    /// Returns all markers for current passage that are beyond current tick,
    /// and removes them from the marker heap.
    fn collect_unreachable_markers(&mut self) -> Vec<PositionMarker> {
        let current_tick = self.current_tick;
        let current_passage = self.current_passage_id;

        // Drain all remaining markers and filter for current passage
        let mut unreachable = Vec::new();
        while let Some(marker) = self.markers.pop() {
            let marker = marker.0;
            if Some(marker.passage_id) == current_passage && marker.tick > current_tick {
                unreachable.push(marker);
            }
        }

        // Sort by tick (earliest first) for consistent ordering
        unreachable.sort_by_key(|m| m.tick);
        unreachable
    }

    /// Mix single passage into output buffer
    ///
    /// Per SPEC016 DBD-MIX-040: Reads pre-faded samples, applies master volume.
    ///
    /// Returns list of marker events triggered during this mix operation.
    ///
    /// # Arguments
    ///
    /// * `buffer_manager` - Buffer manager to retrieve passage buffer
    /// * `passage_id` - UUID of passage to mix
    /// * `output` - Output buffer to fill (interleaved stereo f32)
    ///
    /// # Returns
    ///
    /// Vector of marker events that were triggered (reached) during mixing.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut output = vec![0.0f32; 1024]; // 512 stereo samples
    /// let events = mixer.mix_single(&buffer_manager, passage_id, &mut output).await?;
    ///
    /// // Process triggered events
    /// for event in events {
    ///     match event {
    ///         MarkerEvent::PositionUpdate { position_ms } => {
    ///             println!("Position: {}ms", position_ms);
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub async fn mix_single(&mut self, buffer_manager: &Arc<BufferManager>, _passage_id: Uuid, output: &mut [f32]) -> Result<Vec<MarkerEvent>> {
        // Validate stereo sample count
        if output.len() % 2 != 0 {
            return Err(Error::Config(format!(
                "Invalid sample count: {} (must be even for stereo)", output.len()
            )));
        }

        match self.state {
            MixerState::Playing => {
                // Get buffer from BufferManager using current_queue_entry_id
                // (buffers are keyed by queue_entry_id, not passage_id)
                let queue_entry_id = self.current_queue_entry_id
                    .ok_or_else(|| Error::Config("No current queue entry ID set".to_string()))?;

                let buffer_arc = buffer_manager.get_buffer(queue_entry_id).await
                    .ok_or_else(|| Error::Config(format!("No buffer found for queue_entry_id {}", queue_entry_id)))?;

                // Read pre-faded samples from buffer
                let frames_requested = output.len() / 2;
                let mut frames_read = 0;
                let mut output_idx = 0;

                // Read frames one by one from PlayoutRingBuffer
                while frames_read < frames_requested {
                    match buffer_arc.pop_frame() {
                        Ok(frame) => {
                            // Apply master volume
                            let mut left = frame.left * self.master_volume;
                            let mut right = frame.right * self.master_volume;

                            // Apply resume fade-in multiplicatively (mixer-level fade)
                            if let Some(ref resume) = self.resume_state {
                                let sample_pos = frames_read * 2; // Position in samples (not frames)
                                if resume.samples_since_resume + sample_pos < resume.fade_duration_samples {
                                    let fade_position = (resume.samples_since_resume + sample_pos) as f32
                                        / resume.fade_duration_samples as f32;
                                    let multiplier = resume.fade_in_curve.calculate_fade_in(fade_position);
                                    left *= multiplier;
                                    right *= multiplier;
                                }
                            }

                            output[output_idx] = left;
                            output[output_idx + 1] = right;
                            output_idx += 2;
                            frames_read += 1;

                            // Store last sample for potential pause mode
                            self.last_sample_left = left;
                            self.last_sample_right = right;
                        }
                        Err(_) => {
                            // Buffer underrun - fill remainder with silence
                            break;
                        }
                    }
                }

                // Update resume fade progress and clear if complete
                if let Some(ref mut resume) = self.resume_state {
                    resume.samples_since_resume += frames_read * 2; // Samples, not frames
                    if resume.samples_since_resume >= resume.fade_duration_samples {
                        self.resume_state = None; // Fade complete
                    }
                }

                // Fill remainder with silence if buffer underrun
                for i in output_idx..output.len() {
                    output[i] = 0.0;
                }

                // Update position tracking
                // Convert frames to ticks for marker comparison (TICK_RATE = 28.224MHz vs 44.1kHz sample rate)
                let tick_increment = wkmp_common::timing::samples_to_ticks(frames_read, 44100);
                self.current_tick += tick_increment;
                self.frames_written += frames_read as u64;

                // Check for EOF (decode complete AND buffer empty)
                // **[REQ-MIX-EOF-001]** Detect EOF and collect unreachable markers
                let mut events = self.check_markers();
                if frames_read < frames_requested && buffer_arc.is_exhausted() {
                    // EOF reached - collect unreachable markers
                    let unreachable = self.collect_unreachable_markers();

                    // Check if EOF occurred before a planned crossfade
                    // **[REQ-MIX-EOF-002]** Signal early EOF before lead-out
                    let crossfade_marker = unreachable.iter().find(|m| {
                        matches!(m.event_type, MarkerEvent::StartCrossfade { .. })
                    });

                    if let Some(crossfade) = crossfade_marker {
                        events.push(MarkerEvent::EndOfFileBeforeLeadOut {
                            planned_crossfade_tick: crossfade.tick,
                            unreachable_markers: unreachable,
                        });
                    } else {
                        // **[REQ-MIX-EOF-003]** Regular EOF, automatic queue advancement
                        events.push(MarkerEvent::EndOfFile {
                            unreachable_markers: unreachable,
                        });
                    }
                }

                // Return all events (regular markers + EOF if detected)
                Ok(events)
            }

            MixerState::Paused => {
                // Pause mode: Exponential decay per SPEC016 DBD-MIX-050
                self.fill_pause_mode(output);

                // No position updates during pause
                Ok(Vec::new())
            }
        }
    }

    /// Mix two passages with crossfade overlap
    ///
    /// Per SPEC016 DBD-MIX-041: Simple addition of pre-faded samples.
    ///
    /// Returns list of marker events triggered during this mix operation.
    ///
    /// # Arguments
    ///
    /// * `buffer_manager` - Buffer manager to retrieve passage buffers
    /// * `current_passage_id` - UUID of current passage (fading out)
    /// * `next_passage_id` - UUID of next passage (fading in)
    /// * `output` - Output buffer to fill
    ///
    /// # Returns
    ///
    /// Vector of marker events that were triggered (reached) during mixing.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let events = mixer.mix_crossfade(&buffer_manager, current_id, next_id, &mut output).await?;
    /// ```
    pub async fn mix_crossfade(
        &mut self,
        buffer_manager: &Arc<BufferManager>,
        current_passage_id: Uuid,
        next_passage_id: Uuid,
        output: &mut [f32],
    ) -> Result<Vec<MarkerEvent>> {
        // Validate stereo sample count
        if output.len() % 2 != 0 {
            return Err(Error::Config(format!(
                "Invalid sample count: {} (must be even for stereo)", output.len()
            )));
        }

        if self.state != MixerState::Playing {
            // Not playing, output silence
            self.fill_pause_mode(output);
            return Ok(Vec::new());
        }

        // Get buffers from BufferManager
        let current_buffer = buffer_manager.get_buffer(current_passage_id).await
            .ok_or_else(|| Error::Config(format!("No buffer found for current passage {}", current_passage_id)))?;
        let next_buffer = buffer_manager.get_buffer(next_passage_id).await
            .ok_or_else(|| Error::Config(format!("No buffer found for next passage {}", next_passage_id)))?;

        let frames_requested = output.len() / 2;
        let mut frames_read = 0;
        let mut output_idx = 0;

        // Read and mix frames from both buffers
        while frames_read < frames_requested {
            let current_frame = current_buffer.pop_frame();
            let next_frame = next_buffer.pop_frame();

            match (current_frame, next_frame) {
                (Ok(cur), Ok(next)) => {
                    // Mix: Simple addition per SPEC016 DBD-MIX-041
                    // Both samples already have fade curves applied by Fader
                    let mixed_left = cur.left + next.left;
                    let mixed_right = cur.right + next.right;

                    // Apply master volume
                    let mut left = mixed_left * self.master_volume;
                    let mut right = mixed_right * self.master_volume;

                    // Apply resume fade-in multiplicatively (mixer-level fade)
                    if let Some(ref resume) = self.resume_state {
                        let sample_pos = frames_read * 2;
                        if resume.samples_since_resume + sample_pos < resume.fade_duration_samples {
                            let fade_position = (resume.samples_since_resume + sample_pos) as f32
                                / resume.fade_duration_samples as f32;
                            let multiplier = resume.fade_in_curve.calculate_fade_in(fade_position);
                            left *= multiplier;
                            right *= multiplier;
                        }
                    }

                    output[output_idx] = left;
                    output[output_idx + 1] = right;
                    output_idx += 2;
                    frames_read += 1;

                    // Store last sample
                    self.last_sample_left = left;
                    self.last_sample_right = right;
                }
                _ => {
                    // One or both buffers exhausted - stop crossfade
                    break;
                }
            }
        }

        // Update resume fade progress and clear if complete
        if let Some(ref mut resume) = self.resume_state {
            resume.samples_since_resume += frames_read * 2;
            if resume.samples_since_resume >= resume.fade_duration_samples {
                self.resume_state = None; // Fade complete
            }
        }

        // Fill remainder with silence
        for i in output_idx..output.len() {
            output[i] = 0.0;
        }

        // Update position tracking
        // Convert frames to ticks for marker comparison (TICK_RATE = 28.224MHz vs 44.1kHz sample rate)
        let tick_increment = wkmp_common::timing::samples_to_ticks(frames_read, 44100);
        self.current_tick += tick_increment;
        self.frames_written += frames_read as u64;

        // Check markers and return events
        Ok(self.check_markers())
    }

    /// Fill output buffer with pause mode decay
    ///
    /// Per SPEC016 DBD-MIX-050: Exponential decay from last sample.
    fn fill_pause_mode(&mut self, output: &mut [f32]) {
        for i in (0..output.len()).step_by(2) {
            // Apply decay
            self.last_sample_left *= self.pause_decay_factor;
            self.last_sample_right *= self.pause_decay_factor;

            // Floor check per SPEC016 DBD-MIX-052
            if self.last_sample_left.abs() < self.pause_decay_floor {
                self.last_sample_left = 0.0;
            }
            if self.last_sample_right.abs() < self.pause_decay_floor {
                self.last_sample_right = 0.0;
            }

            output[i] = self.last_sample_left;
            output[i + 1] = self.last_sample_right;
        }
    }
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixer_creation() {
        let mixer = Mixer::new(1.0);
        assert_eq!(mixer.master_volume(), 1.0);
        assert_eq!(mixer.state(), MixerState::Playing);
    }

    #[test]
    fn test_master_volume_clamping() {
        let mut mixer = Mixer::new(1.5); // Over 1.0
        assert_eq!(mixer.master_volume(), 1.0);

        mixer.set_master_volume(-0.5); // Below 0.0
        assert_eq!(mixer.master_volume(), 0.0);

        mixer.set_master_volume(0.75);
        assert_eq!(mixer.master_volume(), 0.75);
    }

    #[test]
    fn test_mixer_state() {
        let mut mixer = Mixer::new(1.0);
        assert_eq!(mixer.state(), MixerState::Playing);

        mixer.set_state(MixerState::Paused);
        assert_eq!(mixer.state(), MixerState::Paused);
    }

    // NOTE: Integration tests with BufferManager deferred to Increment 5 (Option B testing strategy)
    // See: wip/PLAN014_mixer_refactoring/testing_plan_increments_5_7.md
    //
    // Tests to be written:
    // - test_mix_single_with_buffer_manager() - async test with real BufferManager
    // - test_mix_crossfade_with_buffer_manager() - async test with two buffers
    // - test_marker_event_emission() - verify marker system works
    // - test_position_tracking() - verify tick advancement
    //
    // For now, basic unit tests only:

    #[test]
    fn test_pause_mode_output() {
        let mut mixer = Mixer::new(1.0);

        // Set last sample
        mixer.last_sample_left = 1.0;
        mixer.last_sample_right = 1.0;

        // Enter pause mode
        mixer.set_state(MixerState::Paused);

        let mut output = vec![0.0f32; 8]; // 4 stereo samples
        mixer.fill_pause_mode(&mut output);

        // First sample should be decayed
        assert!(output[0] < 1.0);
        assert!(output[1] < 1.0);

        // Each subsequent sample should be smaller
        assert!(output[2] < output[0]);
        assert!(output[4] < output[2]);
    }
}
