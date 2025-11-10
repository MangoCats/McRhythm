//! Global parameter management
//!
//! **[PLAN018]** Centralized singleton for all SPEC016 database-backed parameters.
//! Read-frequently, write-rarely access pattern using RwLock.
//!
//! # Architecture
//!
//! All global parameters are stored in a single `GlobalParams` struct, accessible
//! via the `PARAMS` static singleton. This provides:
//! - Single source of truth for all configuration parameters
//! - Thread-safe access across all microservices
//! - Low-contention read access (readers don't block each other)
//! - Eliminates hardcoded parameter values
//!
//! # Usage
//!
//! ```rust
//! use wkmp_common::params::PARAMS;
//!
//! // Read (fast, uncontended)
//! let sample_rate = *PARAMS.working_sample_rate.read().unwrap();
//!
//! // Write (rare, initialization only)
//! *PARAMS.working_sample_rate.write().unwrap() = 48000;
//! ```

use once_cell::sync::Lazy;
use std::sync::RwLock;

// Module declarations
mod metadata;
mod setters;
mod init;
#[cfg(test)]
mod tests;

/// Global parameters singleton
///
/// Initialized once from database, accessed everywhere.
/// Read-frequently (hot path), write-rarely (startup/config change).
pub static PARAMS: Lazy<GlobalParams> = Lazy::new(GlobalParams::default);

/// Global parameter storage
///
/// All parameters stored with RwLock for thread-safe access.
/// Readers don't block each other (shared read lock).
pub struct GlobalParams {
    /// **[DBD-PARAM-010]** Audio output volume
    ///
    /// Valid range: [0.0, 1.0]
    /// Default: 0.5
    /// Controls overall output volume level
    pub volume_level: RwLock<f32>,

    /// **[DBD-PARAM-020]** Working sample rate for decoded audio
    ///
    /// Valid range: [8000, 192000] Hz
    /// Default: 44100 Hz
    /// CRITICAL: Affects all timing calculations, position tracking, crossfade timing
    pub working_sample_rate: RwLock<u32>,

    /// **[DBD-PARAM-030]** Output ring buffer capacity (mixer → audio callback)
    ///
    /// Valid range: [2048, 262144] frames (stereo pairs)
    /// Default: 8192 frames (186ms @ 44.1kHz)
    /// Lock-free SPSC ring buffer between mixer thread and audio callback
    pub output_ringbuffer_size: RwLock<usize>,

    /// **[DBD-PARAM-050]** Max parallel decoder chains
    ///
    /// Valid range: [1, 32]
    /// Default: 12
    /// Maximum number of concurrent decode-buffer chains
    pub maximum_decode_streams: RwLock<usize>,

    /// **[DBD-PARAM-060]** Decode priority evaluation period
    ///
    /// Valid range: [100, 60000] ms
    /// Default: 5000 ms (5 seconds)
    /// How often decoders check priority queue
    pub decode_work_period: RwLock<u64>,

    /// **[DBD-PARAM-065]** Decode chunk duration
    ///
    /// Valid range: [250, 5000] ms
    /// Default: 1000 ms (1 second)
    ///
    /// **[DBD-DEC-110]** Duration of audio decoded per chunk. Controls decoder
    /// memory usage, CPU overhead, and buffer management granularity.
    ///
    /// **Time-Based Chunking:** Chunks are defined by duration (ms), not sample count.
    /// Converted to samples at source rate: `chunk_samples = source_rate * duration_ms / 1000`
    ///
    /// **Trade-offs:**
    /// - **Smaller (250-500ms):** Lower memory, faster startup, finer buffer control, higher CPU overhead
    /// - **Larger (1500-2000ms):** Lower CPU overhead, higher memory, slower startup, coarser buffer control
    ///
    /// **Performance Impact (12 chains, 96kHz source):**
    /// - 250ms:  2.3 MB memory,  2880 decode calls/min (2.4% CPU)
    /// - 500ms:  4.6 MB memory,  1440 decode calls/min (1.2% CPU)
    /// - 1000ms: 9.2 MB memory,  720 decode calls/min (0.6% CPU) ← Recommended
    /// - 2000ms: 18.4 MB memory, 360 decode calls/min (0.3% CPU)
    ///
    /// **Current value (1000ms) is optimal** for general use:
    /// - ✅ Low CPU overhead (half of 500ms)
    /// - ✅ Moderate memory usage (acceptable on modern systems)
    /// - ✅ Good I/O efficiency (fewer syscalls)
    /// - ✅ Acceptable buffer management overshoot
    /// - ✅ Meets mixer_min_start_level in 1 chunk
    ///
    /// See: PLAN018 ANALYSIS_chunk_duration_ms.md for detailed analysis
    pub chunk_duration_ms: RwLock<u64>,

    /// **[DBD-PARAM-070]** Decoded audio buffer size
    ///
    /// Valid range: [44100, 10000000] samples
    /// Default: 661941 samples (15.01s @ 44.1kHz)
    /// PlayoutRingBuffer capacity for each passage
    pub playout_ringbuffer_size: RwLock<usize>,

    /// **[DBD-PARAM-080]** Buffer headroom for late samples
    ///
    /// Valid range: [2205, 88200] samples
    /// Default: 4410 samples (0.1s @ 44.1kHz)
    /// Decoder pause threshold (free_space ≤ headroom)
    pub playout_ringbuffer_headroom: RwLock<usize>,

    /// **[DBD-PARAM-085]** Hysteresis for decoder pause/resume
    ///
    /// Valid range: [2205, 441000] samples
    /// Default: 44100 samples (1.0s @ 44.1kHz)
    /// Gap between pause and resume thresholds to prevent oscillation
    /// Resume when: free_space ≥ decoder_resume_hysteresis_samples + playout_ringbuffer_headroom
    pub decoder_resume_hysteresis_samples: RwLock<u64>,

    /// **[DBD-PARAM-088]** Min samples before mixer starts
    ///
    /// Valid range: [2205, 88200] samples
    /// Default: 22050 samples (0.5s @ 44.1kHz)
    /// Buffer ready threshold for starting playback
    pub mixer_min_start_level: RwLock<usize>,

    /// **[DBD-PARAM-090]** Exponential decay in pause mode
    ///
    /// Valid range: [0.5, 0.99]
    /// Default: 0.95
    /// Decay factor applied per sample when paused (creates fade-out effect)
    pub pause_decay_factor: RwLock<f64>,

    /// **[DBD-PARAM-100]** Min level before zero output
    ///
    /// Valid range: [0.00001, 0.001]
    /// Default: 0.0001778
    /// Threshold below which output goes to zero (prevents denormals)
    pub pause_decay_floor: RwLock<f64>,

    /// **[DBD-PARAM-110]** Audio output buffer (frames/callback)
    ///
    /// Valid range: [512, 8192] frames
    /// Default: 2208 frames
    /// CRITICAL: Audio callback buffer size, affects latency
    pub audio_buffer_size: RwLock<u32>,

    /// **[DBD-PARAM-111]** Mixer thread check interval
    ///
    /// Valid range: [5, 100] ms
    /// Default: 10 ms
    /// CRITICAL: How often mixer loop wakes up
    pub mixer_check_interval_ms: RwLock<u64>,
}

impl Default for GlobalParams {
    fn default() -> Self {
        Self {
            // [DBD-PARAM-010] Audio output volume
            volume_level: RwLock::new(0.5),

            // [DBD-PARAM-020] Working sample rate (CRITICAL - timing accuracy)
            working_sample_rate: RwLock::new(44100),

            // [DBD-PARAM-030] Output ring buffer capacity (mixer → callback, 186ms @ 44.1kHz)
            output_ringbuffer_size: RwLock::new(8192),

            // [DBD-PARAM-050] Maximum decode streams
            maximum_decode_streams: RwLock::new(12),

            // [DBD-PARAM-060] Decode work period
            decode_work_period: RwLock::new(5000),

            // [DBD-PARAM-065] Decode chunk duration (ms)
            chunk_duration_ms: RwLock::new(1000),

            // [DBD-PARAM-070] Playout ring buffer size
            playout_ringbuffer_size: RwLock::new(661941),

            // [DBD-PARAM-080] Playout ring buffer headroom
            playout_ringbuffer_headroom: RwLock::new(4410),

            // [DBD-PARAM-085] Decoder resume hysteresis
            decoder_resume_hysteresis_samples: RwLock::new(44100),

            // [DBD-PARAM-088] Mixer min start level
            mixer_min_start_level: RwLock::new(22050),

            // [DBD-PARAM-090] Pause decay factor
            pause_decay_factor: RwLock::new(0.95),

            // [DBD-PARAM-100] Pause decay floor
            pause_decay_floor: RwLock::new(0.0001778),

            // [DBD-PARAM-110] Audio buffer size (CRITICAL - audio callback)
            audio_buffer_size: RwLock::new(2208),

            // [DBD-PARAM-111] Mixer check interval (CRITICAL - mixer loop timing)
            mixer_check_interval_ms: RwLock::new(10),
        }
    }
}

// ============================================================================
// **[PLAN019]** Centralized Parameter Metadata
// ============================================================================

/// Metadata for a single GlobalParam parameter
///
/// **[PLAN019-REQ-DRY-010]** Encapsulates all metadata about a parameter including
/// its validation logic. This eliminates 3-way duplication across:
/// - Database loading (`init_from_database()`)
/// - Setter methods (`set_volume_level()`, etc.)
/// - API validation (`bulk_update_settings()`)
///
/// # Fields
///
/// - `key`: Parameter name (e.g., "volume_level")
/// - `data_type`: Rust type as string (e.g., "f32")
/// - `default_value`: Default value as string (e.g., "0.5")
/// - `description`: Human-readable description with traceability ID
/// - `validation_range`: Valid range as string (e.g., "0.0-1.0")
/// - `validator`: Closure that validates string input
///
/// # Validator Closure Signature
///
/// All validators must have signature: `fn(&str) -> Result<(), String>`
///
/// **Error Format Standard** ([PLAN019-HIGH-001]):
/// `"{param_name}: {specific_reason}"`
///
/// # Example
///
/// ```rust
/// # use wkmp_common::params::ParamMetadata;
/// let meta = ParamMetadata {
///     key: "volume_level",
///     data_type: "f32",
///     default_value: "0.5",
///     description: "[DBD-PARAM-010] Audio output volume",
///     validation_range: "0.0-1.0",
///     validator: |s| {
///         let v: f32 = s.parse()
///             .map_err(|_| "volume_level: invalid number format".to_string())?;
///         if v < 0.0 || v > 1.0 {
///             return Err(format!("volume_level: value {} out of range [0.0, 1.0]", v));
///         }
///         Ok(())
///     },
/// };
///
/// assert!(meta.validator("0.5").is_ok());
/// assert!(meta.validator("2.0").is_err());
/// ```
pub struct ParamMetadata {
    pub key: &'static str,
    pub data_type: &'static str,
    pub default_value: &'static str,
    pub description: &'static str,
    pub validation_range: &'static str,
    pub validator: fn(&str) -> Result<(), String>,
}
