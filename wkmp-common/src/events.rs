//! Event types for WKMP event system

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WKMP event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WkmpEvent {
    /// Playback state changed
    PlaybackStateChanged {
        state: PlaybackState,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage started playing
    PassageStarted {
        passage_id: Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage completed
    PassageCompleted {
        passage_id: Uuid,
        completed: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Current song changed
    CurrentSongChanged {
        passage_id: Uuid,
        song_id: Option<Uuid>,
        position_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Playback progress update
    PlaybackProgress {
        passage_id: Uuid,
        position_ms: u64,
        duration_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Queue changed (notification only - no data)
    QueueChanged {
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Queue state update (full queue contents for SSE)
    /// [SSE-UI-020] Queue Updates
    QueueStateUpdate {
        timestamp: chrono::DateTime<chrono::Utc>,
        queue: Vec<QueueEntryInfo>,
    },

    /// Playback position update (sent every 1s during playback)
    /// [SSE-UI-030] Playback Position Updates
    PlaybackPosition {
        timestamp: chrono::DateTime<chrono::Utc>,
        passage_id: Uuid,
        position_ms: u64,
        duration_ms: u64,
        playing: bool,
    },

    /// Volume changed
    VolumeChanged {
        volume: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Initial state sent on SSE connection
    /// [SSE-UI-050] Initial State on Connection
    InitialState {
        timestamp: chrono::DateTime<chrono::Utc>,
        queue: Vec<QueueEntryInfo>,
        position: Option<PlaybackPositionInfo>,
        volume: f32,
    },

    /// Crossfade started
    CrossfadeStarted {
        from_passage_id: Uuid,
        to_passage_id: Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Buffer chain status update (sent every 1s when data changes)
    /// Shows decoder-resampler-fade-buffer chains for monitoring
    BufferChainStatus {
        timestamp: chrono::DateTime<chrono::Utc>,
        chains: Vec<BufferChainInfo>,
    },

    /// Endpoint discovered during decode
    /// **[DBD-DEC-095]** Emitted when decoder discovers actual file duration for undefined endpoints
    /// Sent by decoder → buffer manager when passage has NULL end_time_ticks
    EndpointDiscovered {
        queue_entry_id: Uuid,
        actual_end_ticks: i64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Pipeline validation succeeded (periodic check)
    /// **[ARCH-AUTO-VAL-001]** Automatic validation service - success result
    ValidationSuccess {
        timestamp: chrono::DateTime<chrono::Utc>,
        passage_count: usize,
        total_decoder_samples: u64,
        total_buffer_written: u64,
        total_buffer_read: u64,
        total_mixer_frames: u64,
    },

    /// Pipeline validation failed (conservation law violated)
    /// **[ARCH-AUTO-VAL-001]** Automatic validation service - failure result
    ValidationFailure {
        timestamp: chrono::DateTime<chrono::Utc>,
        passage_count: usize,
        total_decoder_samples: u64,
        total_buffer_written: u64,
        total_buffer_read: u64,
        total_mixer_frames: u64,
        errors: Vec<String>,
    },

    /// Pipeline validation warning (approaching tolerance threshold)
    /// **[ARCH-AUTO-VAL-001]** Automatic validation service - warning result (>80% of tolerance)
    ValidationWarning {
        timestamp: chrono::DateTime<chrono::Utc>,
        passage_count: usize,
        total_decoder_samples: u64,
        total_buffer_written: u64,
        total_buffer_read: u64,
        total_mixer_frames: u64,
        warnings: Vec<String>,
    },
}

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

    // Resampler stage visibility **[DBD-OV-010]** **[DBD-RSMP-010]**
    /// Source file sample rate (Hz)
    pub source_sample_rate: Option<u32>,
    /// Resampler active (true if source rate != working rate)
    pub resampler_active: Option<bool>,
    /// Target sample rate (always 44100 Hz)
    #[serde(default = "default_working_sample_rate")]
    pub target_sample_rate: u32,

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
            source_sample_rate: None,
            resampler_active: Some(false),
            target_sample_rate: 44100,
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

/// Decoder state enumeration
/// **[DBD-DEC-030]** Decoder pauses when buffer full, resumes as data consumed
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum DecoderState {
    /// Waiting for work
    Idle,
    /// Actively decoding audio
    Decoding,
    /// Paused (buffer full or lower priority)
    Paused,
}

impl std::fmt::Display for DecoderState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecoderState::Idle => write!(f, "Idle"),
            DecoderState::Decoding => write!(f, "Decoding"),
            DecoderState::Paused => write!(f, "Paused"),
        }
    }
}

/// Fade processing stage enumeration
/// **[DBD-FADE-010]** through **[DBD-FADE-060]**
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum FadeStage {
    /// Before passage start (discarding samples) **[DBD-FADE-020]**
    PreStart,
    /// Applying fade-in curve **[DBD-FADE-030]**
    FadeIn,
    /// No fade applied (passthrough) **[DBD-FADE-040]**
    Body,
    /// Applying fade-out curve **[DBD-FADE-050]**
    FadeOut,
    /// After passage end (decode complete) **[DBD-FADE-060]**
    PostEnd,
}

impl std::fmt::Display for FadeStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FadeStage::PreStart => write!(f, "PreStart"),
            FadeStage::FadeIn => write!(f, "FadeIn"),
            FadeStage::Body => write!(f, "Body"),
            FadeStage::FadeOut => write!(f, "FadeOut"),
            FadeStage::PostEnd => write!(f, "PostEnd"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

impl std::fmt::Display for PlaybackState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaybackState::Playing => write!(f, "playing"),
            PlaybackState::Paused => write!(f, "paused"),
            PlaybackState::Stopped => write!(f, "stopped"),
        }
    }
}

impl WkmpEvent {
    /// Get event type as string for filtering
    pub fn event_type(&self) -> &str {
        match self {
            WkmpEvent::PlaybackStateChanged { .. } => "PlaybackStateChanged",
            WkmpEvent::PassageStarted { .. } => "PassageStarted",
            WkmpEvent::PassageCompleted { .. } => "PassageCompleted",
            WkmpEvent::CurrentSongChanged { .. } => "CurrentSongChanged",
            WkmpEvent::PlaybackProgress { .. } => "PlaybackProgress",
            WkmpEvent::QueueChanged { .. } => "QueueChanged",
            WkmpEvent::QueueStateUpdate { .. } => "QueueStateUpdate",
            WkmpEvent::PlaybackPosition { .. } => "PlaybackPosition",
            WkmpEvent::VolumeChanged { .. } => "VolumeChanged",
            WkmpEvent::InitialState { ..} => "InitialState",
            WkmpEvent::CrossfadeStarted { .. } => "CrossfadeStarted",
            WkmpEvent::BufferChainStatus { .. } => "BufferChainStatus",
            WkmpEvent::EndpointDiscovered { .. } => "EndpointDiscovered",
            WkmpEvent::ValidationSuccess { .. } => "ValidationSuccess",
            WkmpEvent::ValidationFailure { .. } => "ValidationFailure",
            WkmpEvent::ValidationWarning { .. } => "ValidationWarning",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **[SPEC020-TEST-010]** Test queue position semantics (0-based indexing per SPEC008)
    ///
    /// Verifies that queue_position follows SPEC008 convention:
    /// - Position 0 = "now playing" [SPEC020-MONITOR-050]
    /// - Position 1 = "up next"
    /// - Position 2+ = queued passages
    /// - None = idle chain
    #[test]
    fn test_buffer_chain_info_queue_position_semantics() {
        // Position 0 = "now playing" [SPEC020-MONITOR-050]
        let chain_now_playing = BufferChainInfo {
            slot_index: 0,
            queue_entry_id: Some(uuid::Uuid::new_v4()),
            passage_id: Some(uuid::Uuid::new_v4()),
            file_name: Some("test.mp3".to_string()),
            queue_position: Some(0),  // 0-BASED: now playing
            decoder_state: Some(DecoderState::Decoding),
            decode_progress_percent: Some(45),
            is_actively_decoding: Some(true),
            source_sample_rate: Some(44100),
            resampler_active: Some(false),
            target_sample_rate: 44100,
            fade_stage: Some(FadeStage::Body),
            buffer_state: Some("Playing".to_string()),
            buffer_fill_percent: 65.5,
            buffer_fill_samples: 28900,
            buffer_capacity_samples: 44100,
            total_decoded_frames: 41000,
            playback_position_frames: 12000,
            playback_position_ms: 272,
            duration_ms: Some(180000),
            is_active_in_mixer: true,
            mixer_role: "Current".to_string(),
            started_at: Some("2025-10-20T12:00:00Z".to_string()),
        };

        assert_eq!(
            chain_now_playing.queue_position,
            Some(0),
            "[SPEC008] Position 0 should be 'now playing'"
        );
        assert!(chain_now_playing.is_active_in_mixer, "Now playing should be active in mixer");
        assert_eq!(chain_now_playing.mixer_role, "Current");

        // Position 1 = "up next" [SPEC020-MONITOR-050]
        let chain_up_next = BufferChainInfo {
            slot_index: 1,
            queue_entry_id: Some(uuid::Uuid::new_v4()),
            passage_id: Some(uuid::Uuid::new_v4()),
            file_name: Some("next.mp3".to_string()),
            queue_position: Some(1),  // 0-BASED: up next
            decoder_state: Some(DecoderState::Decoding),
            decode_progress_percent: Some(12),
            is_actively_decoding: Some(true),
            source_sample_rate: Some(48000),
            resampler_active: Some(true),
            target_sample_rate: 44100,
            fade_stage: Some(FadeStage::PreStart),
            buffer_state: Some("Filling".to_string()),
            buffer_fill_percent: 15.2,
            buffer_fill_samples: 6703,
            buffer_capacity_samples: 44100,
            total_decoded_frames: 6703,
            playback_position_frames: 0,
            playback_position_ms: 0,
            duration_ms: Some(240000),
            is_active_in_mixer: false,
            mixer_role: "Idle".to_string(),
            started_at: None,
        };

        assert_eq!(
            chain_up_next.queue_position,
            Some(1),
            "[SPEC008] Position 1 should be 'up next'"
        );
        assert!(chain_up_next.resampler_active.unwrap(), "48kHz source should require resampling");

        // Position 2+ = queued passages [SPEC020-MONITOR-050]
        let chain_queued = BufferChainInfo {
            slot_index: 2,
            queue_entry_id: Some(uuid::Uuid::new_v4()),
            passage_id: Some(uuid::Uuid::new_v4()),
            file_name: Some("queued.mp3".to_string()),
            queue_position: Some(2),  // 0-BASED: queued
            decoder_state: Some(DecoderState::Decoding),
            decode_progress_percent: Some(5),
            is_actively_decoding: Some(true),
            source_sample_rate: Some(44100),
            resampler_active: Some(false),
            target_sample_rate: 44100,
            fade_stage: Some(FadeStage::PreStart),
            buffer_state: Some("Filling".to_string()),
            buffer_fill_percent: 3.1,
            buffer_fill_samples: 1367,
            buffer_capacity_samples: 44100,
            total_decoded_frames: 1367,
            playback_position_frames: 0,
            playback_position_ms: 0,
            duration_ms: Some(200000),
            is_active_in_mixer: false,
            mixer_role: "Idle".to_string(),
            started_at: None,
        };

        assert_eq!(
            chain_queued.queue_position,
            Some(2),
            "[SPEC008] Position 2+ should be queued passages"
        );
        assert!(!chain_queued.is_active_in_mixer, "Queued passages should not be in mixer");

        // None = idle chain [SPEC020-MONITOR-050]
        let idle_chain = BufferChainInfo::idle(5);
        assert_eq!(
            idle_chain.queue_position,
            None,
            "[SPEC008] Idle chain should have queue_position None"
        );
        assert_eq!(idle_chain.buffer_state, Some("Idle".to_string()));
        assert_eq!(idle_chain.buffer_fill_percent, 0.0);
        assert!(!idle_chain.is_active_in_mixer);
    }

    /// **[SPEC020-TEST-020]** Test BufferChainInfo::idle() constructor
    ///
    /// Verifies that idle chains are properly initialized with default values
    #[test]
    fn test_buffer_chain_info_idle_constructor() {
        for slot in 0..12 {
            let idle = BufferChainInfo::idle(slot);

            assert_eq!(idle.slot_index, slot, "slot_index should match");
            assert_eq!(idle.queue_entry_id, None, "idle chain has no queue_entry_id");
            assert_eq!(idle.passage_id, None, "idle chain has no passage_id");
            assert_eq!(idle.file_name, None, "idle chain has no file_name");
            assert_eq!(idle.queue_position, None, "idle chain has no queue_position");
            assert_eq!(idle.decoder_state, Some(DecoderState::Idle));
            assert_eq!(idle.decode_progress_percent, Some(0));
            assert_eq!(idle.is_actively_decoding, Some(false));
            assert_eq!(idle.source_sample_rate, None);
            assert_eq!(idle.resampler_active, Some(false));
            assert_eq!(idle.target_sample_rate, 44100, "target rate always 44100 Hz");
            assert_eq!(idle.fade_stage, None);
            assert_eq!(idle.buffer_state, Some("Idle".to_string()));
            assert_eq!(idle.buffer_fill_percent, 0.0);
            assert_eq!(idle.buffer_fill_samples, 0);
            assert_eq!(idle.buffer_capacity_samples, 0);
            assert_eq!(idle.playback_position_frames, 0);
            assert_eq!(idle.playback_position_ms, 0);
            assert_eq!(idle.duration_ms, None);
            assert!(!idle.is_active_in_mixer);
            assert_eq!(idle.mixer_role, "Idle");
            assert_eq!(idle.started_at, None);
        }
    }

    /// **[SPEC020-TEST-030]** Test BufferChainInfo JSON serialization for SSE
    ///
    /// Verifies that BufferChainInfo serializes correctly for SSE BufferChainStatus events
    #[test]
    fn test_buffer_chain_info_serialization() {
        let chain = BufferChainInfo {
            slot_index: 0,
            queue_entry_id: Some(uuid::Uuid::from_u128(0x12345678_1234_1234_1234_123456789abc)),
            passage_id: Some(uuid::Uuid::from_u128(0x87654321_4321_4321_4321_cba987654321)),
            file_name: Some("test.mp3".to_string()),
            queue_position: Some(0),  // 0-based: now playing
            decoder_state: Some(DecoderState::Decoding),
            decode_progress_percent: Some(45),
            is_actively_decoding: Some(true),
            source_sample_rate: Some(48000),
            resampler_active: Some(true),
            target_sample_rate: 44100,
            fade_stage: Some(FadeStage::FadeIn),
            buffer_state: Some("Playing".to_string()),
            buffer_fill_percent: 65.5,
            buffer_fill_samples: 28900,
            buffer_capacity_samples: 44100,
            total_decoded_frames: 41000,
            playback_position_frames: 12000,
            playback_position_ms: 272,
            duration_ms: Some(180000),
            is_active_in_mixer: true,
            mixer_role: "Current".to_string(),
            started_at: Some("2025-10-20T12:00:00Z".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&chain).expect("Serialization should succeed");

        // Verify key fields in JSON
        assert!(json.contains("\"slot_index\":0"), "slot_index should serialize");
        assert!(json.contains("\"queue_position\":0"), "queue_position 0 (now playing) should serialize");
        assert!(json.contains("\"buffer_fill_percent\":65.5"), "buffer_fill_percent should serialize");
        assert!(json.contains("\"target_sample_rate\":44100"), "target_sample_rate should serialize");
        assert!(json.contains("\"mixer_role\":\"Current\""), "mixer_role should serialize");
        assert!(json.contains("\"decoder_state\":\"Decoding\""), "decoder_state should serialize");
        assert!(json.contains("\"fade_stage\":\"FadeIn\""), "fade_stage should serialize");

        // Deserialize back
        let deserialized: BufferChainInfo = serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(deserialized.slot_index, 0);
        assert_eq!(deserialized.queue_position, Some(0));
        assert_eq!(deserialized.buffer_fill_percent, 65.5);
        assert_eq!(deserialized.mixer_role, "Current");
    }

    /// **[SPEC020-TEST-040]** Test DecoderState enum variants
    #[test]
    fn test_decoder_state_enum() {
        assert_eq!(DecoderState::Idle.to_string(), "Idle");
        assert_eq!(DecoderState::Decoding.to_string(), "Decoding");
        assert_eq!(DecoderState::Paused.to_string(), "Paused");

        // Test equality
        assert_eq!(DecoderState::Idle, DecoderState::Idle);
        assert_ne!(DecoderState::Idle, DecoderState::Decoding);
    }

    /// **[SPEC020-TEST-050]** Test FadeStage enum variants
    #[test]
    fn test_fade_stage_enum() {
        assert_eq!(FadeStage::PreStart.to_string(), "PreStart");
        assert_eq!(FadeStage::FadeIn.to_string(), "FadeIn");
        assert_eq!(FadeStage::Body.to_string(), "Body");
        assert_eq!(FadeStage::FadeOut.to_string(), "FadeOut");
        assert_eq!(FadeStage::PostEnd.to_string(), "PostEnd");

        // Test equality
        assert_eq!(FadeStage::Body, FadeStage::Body);
        assert_ne!(FadeStage::FadeIn, FadeStage::FadeOut);
    }

    /// **[SPEC020-TEST-060]** Test BufferChainStatus SSE event structure
    #[test]
    fn test_buffer_chain_status_sse_event() {
        use chrono::Utc;

        let chains = vec![
            BufferChainInfo::idle(0),
            BufferChainInfo::idle(1),
        ];

        let event = WkmpEvent::BufferChainStatus {
            timestamp: Utc::now(),
            chains: chains.clone(),
        };

        assert_eq!(event.event_type(), "BufferChainStatus");

        // Serialize event
        let json = serde_json::to_string(&event).expect("Event serialization should succeed");
        assert!(json.contains("\"type\":\"BufferChainStatus\""));
        assert!(json.contains("\"chains\":"));

        // Deserialize back
        let deserialized: WkmpEvent = serde_json::from_str(&json).expect("Event deserialization should succeed");
        match deserialized {
            WkmpEvent::BufferChainStatus { chains: deserialized_chains, .. } => {
                assert_eq!(deserialized_chains.len(), 2);
                assert_eq!(deserialized_chains[0].slot_index, 0);
                assert_eq!(deserialized_chains[1].slot_index, 1);
            }
            _ => panic!("Wrong event type deserialized"),
        }
    }
}
