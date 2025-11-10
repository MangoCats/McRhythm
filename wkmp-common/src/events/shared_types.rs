//! Shared type definitions for event data
//!
//! Common structs used across multiple event types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::playback_types::{DecoderState, FadeStage};

/// Queue entry information for SSE events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntryInfo {
    pub queue_entry_id: Uuid,
    pub passage_id: Option<Uuid>,
    pub file_path: String,
}

/// Playback position information for SSE events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackPositionInfo {
    pub passage_id: Uuid,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub playing: bool,
}

/// Buffer chain information for SSE monitoring
///
/// **[DBD-OV-040]** Full pipeline visibility: Decoder → Resampler → Fade → Buffer → Mixer
/// **[DBD-OV-080]** Passage-based chain association (not position-based)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BufferChainInfo {
    pub slot_index: usize,
    pub queue_entry_id: Option<Uuid>,
    pub passage_id: Option<Uuid>,
    pub file_name: Option<String>,

    // Queue position tracking **[DBD-OV-060]** [DBD-OV-070]**
    /// Position in queue (1 = now playing, 2 = next, 3+ = pre-buffering, None = idle)
    pub queue_position: Option<usize>,

    // Decoder stage visibility
    /// Decoder state: Idle, Decoding, Paused
    pub decoder_state: Option<DecoderState>,
    /// Decode progress (0-100%)
    pub decode_progress_percent: Option<u8>,
    /// Currently being processed by decoder pool
    pub is_actively_decoding: Option<bool>,

    // Decoder telemetry **[REQ-DEBT-FUNC-001]**
    /// Duration of decode operation in milliseconds
    pub decode_duration_ms: Option<u64>,
    /// Source file path
    pub source_file_path: Option<String>,

    // Resampler stage visibility **[DBD-OV-010]** **[DBD-RSMP-010]**
    /// Source file sample rate (Hz)
    pub source_sample_rate: Option<u32>,
    /// Resampler active (true if source rate != working rate)
    pub resampler_active: Option<bool>,
    /// Target sample rate (matches device native rate per [DBD-PARAM-020], typically 44100 or 48000 Hz)
    #[serde(default = "default_working_sample_rate")]
    pub target_sample_rate: u32,
    /// Resampler algorithm name (e.g., "Septic polynomial", "Linear", "PassThrough")
    pub resampler_algorithm: Option<String>,

    // Fade handler stage visibility **[DBD-FADE-010]**
    /// Current fade stage: PreStart, FadeIn, Body, FadeOut, PostEnd
    pub fade_stage: Option<FadeStage>,

    // Buffer stage visibility **[DBD-BUF-020]** through **[DBD-BUF-060]**
    /// Buffer state: Empty, Filling, Ready, Playing, Finished
    pub buffer_state: Option<String>,
    pub buffer_fill_percent: f32,
    pub buffer_fill_samples: usize,
    pub buffer_capacity_samples: usize,
    /// Total frames written to buffer (cumulative, for decode progress tracking)
    pub total_decoded_frames: usize,

    // Mixer stage visibility
    pub playback_position_frames: usize,
    pub playback_position_ms: u64,
    pub duration_ms: Option<u64>,
    pub is_active_in_mixer: bool,
    pub mixer_role: String, // "Idle", "Current", "Next", "Crossfading"
    pub started_at: Option<String>,
}

fn default_working_sample_rate() -> u32 {
    44100 // **[DBD-PARAM-020]** working_sample_rate default
}

impl BufferChainInfo {
    /// Create an idle chain info for unused slots
    pub fn idle(slot_index: usize) -> Self {
        Self {
            slot_index,
            queue_entry_id: None,
            passage_id: None,
            file_name: None,
            queue_position: None,
            decoder_state: Some(DecoderState::Idle),
            decode_progress_percent: Some(0),
            is_actively_decoding: Some(false),
            decode_duration_ms: None,
            source_file_path: None,
            source_sample_rate: None,
            resampler_active: Some(false),
            target_sample_rate: 44100,
            resampler_algorithm: None,
            fade_stage: None,
            buffer_state: Some("Idle".to_string()),
            buffer_fill_percent: 0.0,
            buffer_fill_samples: 0,
            buffer_capacity_samples: 0,
            total_decoded_frames: 0,
            playback_position_frames: 0,
            playback_position_ms: 0,
            duration_ms: None,
            is_active_in_mixer: false,
            mixer_role: "Idle".to_string(),
            started_at: None,
        }
    }
}
