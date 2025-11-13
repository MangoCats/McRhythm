//! Settings browser API with parameter metadata
//!
//! Provides comprehensive view of all settings table parameters with:
//! - Database key and current value
//! - Units, spec designators, aliases
//! - Rich descriptions of usage and purpose

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use sqlx::Row;

use crate::AppState;

/// Settings browser response
#[derive(Debug, Serialize)]
pub struct SettingsBrowserResponse {
    pub total_settings: usize,
    pub settings: Vec<SettingRow>,
}

/// Individual setting row with metadata
#[derive(Debug, Serialize)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
    pub units: String,
    pub default_value: String,
    pub spec_id: String,
    pub aliases: Vec<String>,
    pub description: String,
}

/// Parameter metadata definition
struct ParamMetadata {
    key: &'static str,
    units: &'static str,
    default_value: &'static str,
    spec_id: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
}

/// Comprehensive parameter metadata catalog
///
/// Based on:
/// - SPEC016-decoder_buffer_design.md (DBD-PARAM-### parameters)
/// - IMPL001-database_schema.md (settings table documentation)
/// - wkmp-common/src/params.rs (GlobalParams structure)
fn get_parameter_metadata() -> Vec<ParamMetadata> {
    vec![
        // Audio Configuration
        ParamMetadata {
            key: "volume_level",
            units: "ratio (0.0-1.0)",
            default_value: "0.5",
            spec_id: "DB-SET-060, DBD-PARAM-010",
            aliases: &["volume"],
            description: "Audio output volume level. Runtime-modifiable. Used by wkmp-ap mixer to scale all audio samples before output. Applied as linear multiplier to decoded PCM data. UI displays as 0-100 (display = round(volume * 100)). See IMPL001:834, SPEC016.",
        },
        ParamMetadata {
            key: "audio_sink",
            units: "device identifier",
            default_value: "default",
            spec_id: "DB-SET-060",
            aliases: &["output_device"],
            description: "Selected audio output device identifier. Runtime-modifiable. Used by wkmp-ap audio output layer to select which hardware device receives audio. Value 'default' uses system default audio device. See IMPL001:835.",
        },
        ParamMetadata {
            key: "working_sample_rate",
            units: "Hz",
            default_value: "44100",
            spec_id: "DBD-PARAM-020",
            aliases: &["sample_rate", "target_sample_rate"],
            description: "Target sample rate for all decoded audio. STRUCTURAL (restart required). Used by wkmp-ap decoder-buffer chain to resample all audio before buffering. Affects all timing calculations, position tracking, and crossfade timing. Standard values: 44100, 48000, 96000 Hz.",
        },

        // Buffer Sizing Parameters (DBD-PARAM-030 through DBD-PARAM-088)
        ParamMetadata {
            key: "output_ringbuffer_size",
            units: "stereo frames",
            default_value: "8192",
            spec_id: "DBD-PARAM-030",
            aliases: &["output_ringbuffer_capacity", "output_buffer_capacity"],
            description: "Lock-free SPSC ring buffer capacity (mixer → audio callback). STRUCTURAL. Used by wkmp-ap audio output layer. Typical value 8192 frames = 186ms @ 44.1kHz. Larger values reduce underruns but increase latency.",
        },
        ParamMetadata {
            key: "maximum_decode_streams",
            units: "count",
            default_value: "12",
            spec_id: "DBD-PARAM-050",
            aliases: &["max_decoders", "decode_streams"],
            description: "Maximum number of parallel decoder-buffer chains. STRUCTURAL. Used by wkmp-ap decoder pool to limit concurrent decode operations. Controls memory usage and CPU utilization. Typical: 12 streams.",
        },
        ParamMetadata {
            key: "decode_work_period",
            units: "ms",
            default_value: "5000",
            spec_id: "DBD-PARAM-060",
            aliases: &["decoder_check_interval"],
            description: "Interval between decode job priority evaluations. STRUCTURAL. Used by wkmp-ap decoder worker to determine how often to re-check priority queue. Affects responsiveness vs CPU usage trade-off. Default: 5000ms (5s).",
        },
        ParamMetadata {
            key: "chunk_duration_ms",
            units: "ms",
            default_value: "1000",
            spec_id: "DBD-PARAM-065",
            aliases: &["decode_chunk_duration"],
            description: "Duration of audio decoded per chunk. STRUCTURAL. Used by wkmp-ap decoder to determine chunk size. Smaller values = lower latency/memory, higher CPU overhead. Larger = opposite. Default 1000ms balances memory, CPU, I/O efficiency.",
        },
        ParamMetadata {
            key: "playout_ringbuffer_size",
            units: "stereo samples",
            default_value: "661941",
            spec_id: "DBD-PARAM-070",
            aliases: &["playout_ringbuffer_capacity", "playout_buffer_size", "decoded_buffer_size"],
            description: "Decoded/resampled audio buffer capacity per passage. STRUCTURAL. Used by wkmp-ap PlayoutRingBuffer to store decoded PCM audio. Default 661941 samples = 15.01s @ 44.1kHz. Holds ready-to-play audio for mixer.",
        },
        ParamMetadata {
            key: "playout_ringbuffer_headroom",
            units: "stereo samples",
            default_value: "4410",
            spec_id: "DBD-PARAM-080",
            aliases: &["buffer_headroom"],
            description: "Buffer headroom for late resampler samples. STRUCTURAL. Used by wkmp-ap decoder worker as pause threshold (pauses when free_space ≤ headroom). Default 4410 samples = 0.1s @ 44.1kHz. Prevents buffer overflow.",
        },
        ParamMetadata {
            key: "decoder_resume_hysteresis_samples",
            units: "stereo samples",
            default_value: "44100",
            spec_id: "DBD-PARAM-085",
            aliases: &["pause_resume_hysteresis"],
            description: "Hysteresis gap between decoder pause/resume thresholds. STRUCTURAL. Used by wkmp-ap decoder worker to prevent oscillation. Resumes when free_space ≥ headroom + hysteresis. Default 44100 samples = 1.0s @ 44.1kHz.",
        },
        ParamMetadata {
            key: "mixer_min_start_level",
            units: "stereo samples",
            default_value: "22050",
            spec_id: "DBD-PARAM-088",
            aliases: &["buffer_ready_threshold"],
            description: "Minimum samples buffered before mixer starts playback. STRUCTURAL. Used by wkmp-ap mixer engine to determine when passage has enough audio to start. Default 22050 samples = 0.5s @ 44.1kHz. Prevents immediate underruns.",
        },

        // Pause Mode Parameters
        ParamMetadata {
            key: "pause_decay_factor",
            units: "ratio (0.5-0.99)",
            default_value: "0.95",
            spec_id: "DBD-PARAM-090",
            aliases: &["pause_fade_factor"],
            description: "Exponential decay multiplier per sample in pause mode. STRUCTURAL. Used by wkmp-ap mixer to create smooth fade-out when pausing. Applied per-sample: output *= decay_factor. Default 0.95 creates gentle fade.",
        },
        ParamMetadata {
            key: "pause_decay_floor",
            units: "ratio",
            default_value: "0.0001778",
            spec_id: "DBD-PARAM-100",
            aliases: &["pause_silence_threshold"],
            description: "Minimum level before outputting zero in pause mode. STRUCTURAL. Used by wkmp-ap mixer to transition from fade to silence. When level < floor, output becomes zero. Default 0.0001778 prevents denormal floats.",
        },

        // Audio Output Parameters
        ParamMetadata {
            key: "audio_buffer_size",
            units: "frames per callback",
            default_value: "2208",
            spec_id: "DBD-PARAM-110",
            aliases: &["callback_buffer_size", "output_buffer_frames"],
            description: "Audio output buffer size (frames per callback). STRUCTURAL. Used by wkmp-ap audio output layer (cpal) to configure hardware buffer. Affects latency and underrun risk. Default 2208 frames = 50.1ms @ 44.1kHz.",
        },
        ParamMetadata {
            key: "mixer_check_interval_ms",
            units: "ms",
            default_value: "10",
            spec_id: "DBD-PARAM-111",
            aliases: &["mixer_tick_interval"],
            description: "Mixer thread check interval for buffer filling. STRUCTURAL. Used by wkmp-ap mixer engine to determine how often to wake up and fill output ring buffer. Default 10ms. Critical for buffer refill responsiveness.",
        },
        ParamMetadata {
            key: "mixer_batch_size_low",
            units: "frames",
            default_value: "512",
            spec_id: "DBD-PARAM-112",
            aliases: &["aggressive_refill_size"],
            description: "Frames filled when output ring buffer <50% full. STRUCTURAL. Used by wkmp-ap mixer for aggressive buffer recovery. Larger batches catch up faster when buffer depleted. Default 512 frames.",
        },
        ParamMetadata {
            key: "mixer_batch_size_optimal",
            units: "frames",
            default_value: "256",
            spec_id: "DBD-PARAM-113",
            aliases: &["steady_state_refill_size"],
            description: "Frames filled when output ring buffer 50-75% full. STRUCTURAL. Used by wkmp-ap mixer for steady-state operation. Smaller batches reduce CPU overhead. Default 256 frames.",
        },

        // Event Timing Configuration
        ParamMetadata {
            key: "position_event_interval_ms",
            units: "ms",
            default_value: "1000",
            spec_id: "DB-SET-210",
            aliases: &["internal_event_interval"],
            description: "Interval for mixer to emit internal PositionUpdate events. Used by wkmp-ap mixer for song boundary detection. Affects CurrentSongChanged event latency. See IMPL001 lines 892-998, SPEC011 lines 630-695. Range: 100-5000ms.",
        },
        ParamMetadata {
            key: "playback_progress_interval_ms",
            units: "ms",
            default_value: "5000",
            spec_id: "DB-SET-220",
            aliases: &["ui_progress_interval", "sse_progress_interval"],
            description: "Interval for emitting PlaybackProgress SSE events to UI clients. Controls UI progress bar update frequency. Based on playback time not wall clock. See IMPL001 lines 892-998, SPEC011 lines 630-695. Range: 1000-10000ms.",
        },

        // Queue State Persistence
        ParamMetadata {
            key: "queue_current_id",
            units: "UUID",
            default_value: "(none)",
            spec_id: "DBD-PARAM-125, ARCH-QP-020",
            aliases: &[],
            description: "UUID of currently playing queue entry. STATE PERSISTENCE (runtime-managed, not user-configurable). Automatically set by PlaybackEngine on play start, cleared on stop. Enables resume-where-you-left-off across restarts. Read-only. See SPEC016 lines 441-462.",
        },

        // Validation Service Configuration (Diagnostic)
        ParamMetadata {
            key: "validation_enabled",
            units: "boolean",
            default_value: "true",
            spec_id: "DBD-PARAM-130, ARCH-AUTO-VAL-001",
            aliases: &[],
            description: "Master switch for automatic validation service. DIAGNOSTIC (restart required). Enables periodic pipeline integrity checks (decoder → buffer → mixer sample conservation). Minimal overhead when enabled. See SPEC016 lines 473-491.",
        },
        ParamMetadata {
            key: "validation_interval_secs",
            units: "seconds",
            default_value: "10",
            spec_id: "DBD-PARAM-131, ARCH-AUTO-VAL-001",
            aliases: &[],
            description: "Time interval between validation checks. DIAGNOSTIC (restart required). Controls validation frequency during playback. Range: 1-3600 seconds (recommended 5-60). Shorter = faster issue detection, higher overhead. See SPEC016 lines 493-513.",
        },
        ParamMetadata {
            key: "validation_tolerance_samples",
            units: "samples",
            default_value: "8192",
            spec_id: "DBD-PARAM-132, ARCH-AUTO-VAL-001",
            aliases: &[],
            description: "Allowable sample count discrepancy before validation failure. DIAGNOSTIC (restart required). Default 8192 samples (~186ms @ 44.1kHz) accounts for async timing edge cases. Range: 0-88200 samples (recommended 4096-16384). Lower = stricter validation. See SPEC016 lines 515-540.",
        },

        // Crossfade Settings
        ParamMetadata {
            key: "global_crossfade_time",
            units: "seconds",
            default_value: "2.0",
            spec_id: "DB-SET-090, XFD-GLOB-010",
            aliases: &["crossfade_duration", "fade_time"],
            description: "Global crossfade duration for all passages. Used by wkmp-ap when passage lacks per-passage timing. Defines overlap between consecutive passages. Applied to both fade-out and fade-in. See IMPL001:850, SPEC002:344,357,1216-1218.",
        },
        ParamMetadata {
            key: "global_fade_curve",
            units: "enum",
            default_value: "exponential_logarithmic",
            spec_id: "DB-SET-090, XFD-GLOB-020",
            aliases: &["fade_curve_pair"],
            description: "Default fade curve pair for crossfades. Used by wkmp-ap when passage lacks per-passage curves. Options: 'exponential_logarithmic', 'linear_linear', 'cosine_cosine'. Default provides smooth perceived loudness. See IMPL001:851, SPEC002:1216-1218.",
        },

        // Pause/Resume Settings
        ParamMetadata {
            key: "resume_from_pause_fade_in_duration",
            units: "seconds",
            default_value: "0.5",
            spec_id: "DB-SET-100",
            aliases: &["resume_fade_duration"],
            description: "Fade-in duration when resuming from pause. Used by wkmp-ap mixer to smoothly restore volume after pause. See SPEC002 lines 903-916. Range: 0.0-5.0s. Provides gentle transition without abrupt volume change.",
        },
        ParamMetadata {
            key: "resume_from_pause_fade_in_curve",
            units: "enum",
            default_value: "exponential",
            spec_id: "DB-SET-100",
            aliases: &["resume_fade_curve"],
            description: "Fade-in curve type for pause resume. Used by wkmp-ap mixer to shape resume fade. Options: 'linear', 'exponential', 'cosine'. See SPEC002 lines 903-916. Default 'exponential' matches perceived loudness growth.",
        },

        // Volume Fade Settings
        ParamMetadata {
            key: "volume_fade_update_period",
            units: "ms",
            default_value: "10",
            spec_id: "DB-SET-110",
            aliases: &["volume_transition_period"],
            description: "Volume fade update period for smooth transitions. Used by wkmp-ap when changing volume_level. Controls granularity of volume fades. Range: 1-100ms. See IMPL001:856.",
        },

        // Queue Management Settings
        ParamMetadata {
            key: "queue_entry_timing_overrides",
            units: "JSON object",
            default_value: "{}",
            spec_id: "DB-SET-300",
            aliases: &["per_entry_timing"],
            description: "Per-queue-entry timing overrides (JSON). Used by wkmp-ap to override start/end/lead/fade points and curves for specific queue entries. See IMPL001 lines 999-1046, SPEC007 lines 978-1030. Allows one-time playback customization.",
        },
        ParamMetadata {
            key: "queue_refill_threshold_passages",
            units: "count",
            default_value: "2",
            spec_id: "DB-SET-120",
            aliases: &["refill_passage_threshold"],
            description: "Minimum passages in queue before refill request. Used by wkmp-ap to trigger Program Director refill. See SPEC007 lines 1705-1746. Default 2 passages. Full/Lite versions only.",
        },
        ParamMetadata {
            key: "queue_refill_threshold_seconds",
            units: "seconds",
            default_value: "900",
            spec_id: "DB-SET-120",
            aliases: &["refill_time_threshold"],
            description: "Minimum playback time remaining before refill request. Used by wkmp-ap to trigger Program Director refill. See SPEC007 lines 1705-1746. Default 900s (15 min). Full/Lite versions only.",
        },
        ParamMetadata {
            key: "queue_refill_request_throttle_seconds",
            units: "seconds",
            default_value: "10",
            spec_id: "DB-SET-120",
            aliases: &["refill_throttle"],
            description: "Minimum interval between refill requests. Used by wkmp-ap to prevent flooding Program Director with requests. See SPEC007 lines 1705-1746. Default 10s. Full/Lite versions only.",
        },
        ParamMetadata {
            key: "queue_refill_acknowledgment_timeout_seconds",
            units: "seconds",
            default_value: "5",
            spec_id: "DB-SET-120",
            aliases: &["refill_ack_timeout"],
            description: "Timeout for Program Director acknowledgment. Used by wkmp-ap to detect PD failures. If no ACK within timeout logs warning. See SPEC007 lines 1705-1746. Default 5s. Full/Lite versions only.",
        },
        ParamMetadata {
            key: "queue_max_size",
            units: "passages",
            default_value: "100",
            spec_id: "DB-SET-120",
            aliases: &["max_queue_length"],
            description: "Maximum queue size (passage count). Used by wkmp-ap to enforce queue limits. Prevents unbounded memory growth. See IMPL001:863.",
        },
        ParamMetadata {
            key: "queue_max_enqueue_batch",
            units: "passages",
            default_value: "5",
            spec_id: "DB-SET-120",
            aliases: &["max_refill_batch"],
            description: "Maximum passages Program Director can enqueue at once. Used by wkmp-ap to enforce batch size limits. Full/Lite versions only. See IMPL001:864.",
        },

        // Playback State Settings
        ParamMetadata {
            key: "initial_play_state",
            units: "enum (playing|paused)",
            default_value: "playing",
            spec_id: "DB-SET-050",
            aliases: &["startup_state"],
            description: "Initial playback state on application launch. Used by wkmp-ap at startup. Options: 'playing' (auto-start), 'paused' (manual start required). See IMPL001:829.",
        },
        ParamMetadata {
            key: "currently_playing_passage_id",
            units: "UUID",
            default_value: "",
            spec_id: "DB-SET-050",
            aliases: &["current_passage"],
            description: "UUID of passage currently playing. Used by wkmp-ap to persist playback state across restarts. NULL when nothing playing. Updated in real-time during playback. See IMPL001:830.",
        },
        ParamMetadata {
            key: "last_played_passage_id",
            units: "UUID",
            default_value: "",
            spec_id: "DB-SET-050",
            aliases: &["last_passage"],
            description: "UUID of last played passage. Used by wkmp-ap to track playback history. NULL when no passages played yet. Updated when passage completes. See IMPL001:831.",
        },
        ParamMetadata {
            key: "last_played_position_ticks",
            units: "ticks",
            default_value: "0",
            spec_id: "DB-SET-050",
            aliases: &["last_position"],
            description: "Playback position in ticks (28,224,000 ticks/sec per SPEC017). Used by wkmp-ap for resume-after-restart. Updated only on clean shutdown. Reset to 0 on queue change. See IMPL001:832.",
        },

        // Database Backup Settings
        ParamMetadata {
            key: "backup_location",
            units: "path",
            default_value: "(same folder as wkmp.db)",
            spec_id: "DB-SET-080",
            aliases: &["backup_directory"],
            description: "Path to backup directory. Used by wkmp-ui for automatic database backups. Default: same folder as wkmp.db. Can be absolute or relative path. See IMPL001:844.",
        },
        ParamMetadata {
            key: "backup_interval_ms",
            units: "ms",
            default_value: "7776000000",
            spec_id: "DB-SET-080",
            aliases: &["periodic_backup_interval"],
            description: "Periodic backup interval. Used by wkmp-ui backup scheduler. Default 7776000000ms (90 days). Backups created if this interval elapsed since last backup. See IMPL001:845.",
        },
        ParamMetadata {
            key: "backup_minimum_interval_ms",
            units: "ms",
            default_value: "1209600000",
            spec_id: "DB-SET-080",
            aliases: &["startup_backup_minimum"],
            description: "Minimum time between startup backups. Used by wkmp-ui to throttle startup backups. Default 1209600000ms (14 days). Prevents backup on every startup. See IMPL001:846.",
        },
        ParamMetadata {
            key: "backup_retention_count",
            units: "count",
            default_value: "3",
            spec_id: "DB-SET-080",
            aliases: &["max_backups"],
            description: "Number of timestamped backups to keep. Used by wkmp-ui backup cleanup. Oldest backups deleted when count exceeded. See IMPL001:847.",
        },
        ParamMetadata {
            key: "last_backup_timestamp_ms",
            units: "ms (Unix epoch)",
            default_value: "",
            spec_id: "DB-SET-080",
            aliases: &["last_backup_time"],
            description: "Unix milliseconds of last successful backup. Used by wkmp-ui to track when last backup occurred. NULL if never backed up. Updated after each successful backup. See IMPL001:848.",
        },

        // Module Management Settings
        ParamMetadata {
            key: "relaunch_delay",
            units: "seconds",
            default_value: "5",
            spec_id: "DB-SET-130",
            aliases: &["module_restart_delay"],
            description: "Seconds between module relaunch attempts. Used by wkmp-ui module management. Prevents rapid relaunch loops on persistent failures. See IMPL001:866.",
        },
        ParamMetadata {
            key: "relaunch_attempts",
            units: "count",
            default_value: "20",
            spec_id: "DB-SET-130",
            aliases: &["max_relaunch_count"],
            description: "Maximum relaunch attempts before giving up. Used by wkmp-ui module management. After this many failures, stops trying to relaunch module. See IMPL001:867.",
        },

        // Session Management Settings
        ParamMetadata {
            key: "session_timeout_seconds",
            units: "seconds",
            default_value: "31536000",
            spec_id: "DB-SET-140",
            aliases: &["auth_timeout"],
            description: "Session timeout duration. Used by wkmp-ui authentication. Default 31536000s (1 year). Sessions expired after this duration of inactivity. See IMPL001:869.",
        },

        // File Ingest Settings (Full version only)
        ParamMetadata {
            key: "ingest_max_concurrent_jobs",
            units: "count",
            default_value: "4",
            spec_id: "DB-SET-150",
            aliases: &["import_parallelism"],
            description: "Maximum concurrent file processing jobs. Used by wkmp-ai import workflow. Controls CPU/disk I/O utilization during import. Full version only. See IMPL001:871, IMPL010.",
        },

        // Library Settings (Full version only)
        ParamMetadata {
            key: "music_directories",
            units: "JSON array",
            default_value: "[]",
            spec_id: "DB-SET-160",
            aliases: &["scan_directories"],
            description: "Directories to scan for music files (JSON array). Used by wkmp-ai library scanner. Paths relative to root folder or absolute. Full version only. See IMPL001:873.",
        },
        ParamMetadata {
            key: "temporary_flavor_override",
            units: "JSON object",
            default_value: "",
            spec_id: "DB-SET-160",
            aliases: &["flavor_override"],
            description: "Temporary musical flavor override with expiration (JSON). Used by Program Director for time-limited flavor overrides. NULL = no override. Full/Lite versions only. See IMPL001:874.",
        },

        // HTTP Server Configuration Settings
        ParamMetadata {
            key: "http_base_ports",
            units: "JSON array",
            default_value: "[5720, 15720, 25720, 17200, 23400]",
            spec_id: "DB-SET-170",
            aliases: &["base_ports"],
            description: "Base port numbers for HTTP servers (JSON array). Used by all modules. Modules increment to find free port. See IMPL001:876, IMPL004:947-1270.",
        },
        ParamMetadata {
            key: "http_request_timeout_ms",
            units: "ms",
            default_value: "30000",
            spec_id: "DB-SET-170",
            aliases: &["request_timeout"],
            description: "HTTP request timeout. Used by all modules with HTTP servers. Default 30000ms (30s). Requests exceeding this timeout are terminated. See IMPL001:877, IMPL004:947-1270.",
        },
        ParamMetadata {
            key: "http_keepalive_timeout_ms",
            units: "ms",
            default_value: "60000",
            spec_id: "DB-SET-170",
            aliases: &["keepalive_timeout"],
            description: "HTTP keepalive timeout. Used by all modules with HTTP servers. Default 60000ms (60s). Idle connections closed after this timeout. See IMPL001:878, IMPL004:947-1270.",
        },
        ParamMetadata {
            key: "http_max_body_size_bytes",
            units: "bytes",
            default_value: "1048576",
            spec_id: "DB-SET-170",
            aliases: &["max_request_body"],
            description: "Maximum HTTP request body size. Used by all modules with HTTP servers. Default 1048576 bytes (1 MB). Requests with larger bodies rejected. See IMPL001:879, IMPL004:947-1270.",
        },

        // Program Director Settings (Full/Lite only)
        ParamMetadata {
            key: "playback_failure_threshold",
            units: "count",
            default_value: "3",
            spec_id: "DB-SET-180",
            aliases: &["max_playback_failures"],
            description: "Failures before stopping automatic selection. Used by Program Director to detect persistent playback issues. Full/Lite versions only. See IMPL001:881.",
        },
        ParamMetadata {
            key: "playback_failure_window_seconds",
            units: "seconds",
            default_value: "60",
            spec_id: "DB-SET-180",
            aliases: &["failure_window"],
            description: "Time window for playback failure counting. Used by Program Director to group related failures. Failures outside window don't count. Full/Lite versions only. See IMPL001:882.",
        },
    ]
}

/// GET /api/settings/browser
///
/// Returns all settings from database with rich metadata
pub async fn get_settings_browser(
    State(state): State<AppState>,
) -> Result<Json<SettingsBrowserResponse>, SettingsError> {
    // Fetch all settings from database (only key and value columns exist)
    let rows = sqlx::query("SELECT key, value FROM settings ORDER BY key")
        .fetch_all(&state.db)
        .await
        .map_err(|e| SettingsError::DatabaseError(e.to_string()))?;

    // Build metadata lookup (store metadata to avoid temporary value issues)
    let metadata = get_parameter_metadata();
    let metadata_map: std::collections::HashMap<&str, &ParamMetadata> = metadata
        .iter()
        .map(|m| (m.key, m))
        .collect();

    // Build response
    let settings: Vec<SettingRow> = rows
        .iter()
        .map(|row| {
            let key: String = row.get("key");
            let value_text: Option<String> = row.get("value");
            let value = value_text.unwrap_or_else(|| "NULL".to_string());

            // Lookup metadata for this key
            if let Some(meta) = metadata_map.get(key.as_str()) {
                SettingRow {
                    key: key.clone(),
                    value,
                    units: meta.units.to_string(),
                    default_value: meta.default_value.to_string(),
                    spec_id: meta.spec_id.to_string(),
                    aliases: meta.aliases.iter().map(|s| s.to_string()).collect(),
                    description: meta.description.to_string(),
                }
            } else {
                // Unknown parameter - show placeholder
                SettingRow {
                    key: key.clone(),
                    value,
                    units: String::new(),
                    default_value: String::new(),
                    spec_id: String::new(),
                    aliases: vec![],
                    description: format!("unknown parameter {}", key),
                }
            }
        })
        .collect();

    Ok(Json(SettingsBrowserResponse {
        total_settings: settings.len(),
        settings,
    }))
}

/// Error type for settings operations
#[derive(Debug)]
pub enum SettingsError {
    DatabaseError(String),
}

impl IntoResponse for SettingsError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            SettingsError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
