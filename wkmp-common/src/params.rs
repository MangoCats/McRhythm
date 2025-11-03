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

impl GlobalParams {
    /// Reset all parameters to defaults (for testing only)
    #[cfg(test)]
    fn reset_to_defaults(&self) {
        *self.volume_level.write().unwrap() = 0.5;
        *self.working_sample_rate.write().unwrap() = 44100;
        *self.output_ringbuffer_size.write().unwrap() = 8192;
        *self.maximum_decode_streams.write().unwrap() = 12;
        *self.decode_work_period.write().unwrap() = 5000;
        *self.chunk_duration_ms.write().unwrap() = 1000;
        *self.playout_ringbuffer_size.write().unwrap() = 661941;
        *self.playout_ringbuffer_headroom.write().unwrap() = 4410;
        *self.decoder_resume_hysteresis_samples.write().unwrap() = 44100;
        *self.mixer_min_start_level.write().unwrap() = 22050;
        *self.pause_decay_factor.write().unwrap() = 0.95;
        *self.pause_decay_floor.write().unwrap() = 0.0001778;
        *self.audio_buffer_size.write().unwrap() = 2208;
        *self.mixer_check_interval_ms.write().unwrap() = 10;
    }

    /// Initialize all parameters from database
    ///
    /// Called once at wkmp-ap startup. Loads values from settings table.
    /// Falls back to defaults if database entry missing.
    ///
    /// # Error Handling Policy (from 01_specification_issues.md)
    ///
    /// 1. Database connection error: Return Err (fail startup)
    /// 2. Parameter missing: Log WARN, use default, continue
    /// 3. Type mismatch: Log WARN, use default, continue
    /// 4. Out of range: Log WARN, use default, continue
    /// 5. Process all independently (no fail-fast)
    pub async fn init_from_database(
        db_pool: &sqlx::SqlitePool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use tracing::{info, warn};

        info!("Loading GlobalParams from database...");

        // Process each parameter independently (no fail-fast)
        // Each failure logs warning and uses default value

        // [DBD-PARAM-010] volume_level (f32, range: [0.0, 1.0])
        match load_f32_param(db_pool, "volume_level").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_volume_level(value) {
                    warn!("volume_level validation failed: {}, using default", e);
                }
            }
            Ok(None) => warn!("volume_level not found in database, using default (0.5)"),
            Err(e) => warn!("Failed to load volume_level: {}, using default", e),
        }

        // [DBD-PARAM-020] working_sample_rate (u32, range: [8000, 192000])
        match load_u32_param(db_pool, "working_sample_rate").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_working_sample_rate(value) {
                    warn!("working_sample_rate validation failed: {}, using default", e);
                }
            }
            Ok(None) => warn!("working_sample_rate not found in database, using default (44100)"),
            Err(e) => warn!("Failed to load working_sample_rate: {}, using default", e),
        }

        // [DBD-PARAM-030] output_ringbuffer_size (usize, range: [2048, 262144])
        match load_usize_param(db_pool, "output_ringbuffer_size").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_output_ringbuffer_size(value) {
                    warn!("{}, using default (8192)", e);
                }
            }
            Ok(None) => warn!("output_ringbuffer_size not found in database, using default (8192)"),
            Err(e) => warn!("Failed to load output_ringbuffer_size: {}, using default", e),
        }

        // [DBD-PARAM-050] maximum_decode_streams (usize, range: [1, 32])
        match load_usize_param(db_pool, "maximum_decode_streams").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_maximum_decode_streams(value) {
                    warn!("{}, using default (12)", e);
                }
            }
            Ok(None) => warn!("maximum_decode_streams not found in database, using default (12)"),
            Err(e) => warn!("Failed to load maximum_decode_streams: {}, using default", e),
        }

        // [DBD-PARAM-060] decode_work_period (u64, range: [100, 60000])
        match load_u64_param(db_pool, "decode_work_period").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_decode_work_period(value) {
                    warn!("{}, using default (5000)", e);
                }
            }
            Ok(None) => warn!("decode_work_period not found in database, using default (5000)"),
            Err(e) => warn!("Failed to load decode_work_period: {}, using default", e),
        }

        // [DBD-PARAM-065] chunk_duration_ms (u64, range: [250, 5000])
        match load_u64_param(db_pool, "chunk_duration_ms").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_chunk_duration_ms(value) {
                    warn!("{}, using default (1000)", e);
                }
            }
            Ok(None) => warn!("chunk_duration_ms not found in database, using default (1000)"),
            Err(e) => warn!("Failed to load chunk_duration_ms: {}, using default", e),
        }

        // [DBD-PARAM-070] playout_ringbuffer_size (usize, range: [44100, 10000000])
        match load_usize_param(db_pool, "playout_ringbuffer_size").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_playout_ringbuffer_size(value) {
                    warn!("{}, using default (661941)", e);
                }
            }
            Ok(None) => warn!("playout_ringbuffer_size not found in database, using default (661941)"),
            Err(e) => warn!("Failed to load playout_ringbuffer_size: {}, using default", e),
        }

        // [DBD-PARAM-080] playout_ringbuffer_headroom (usize, range: [2205, 88200])
        match load_usize_param(db_pool, "playout_ringbuffer_headroom").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_playout_ringbuffer_headroom(value) {
                    warn!("{}, using default (4410)", e);
                }
            }
            Ok(None) => warn!("playout_ringbuffer_headroom not found in database, using default (4410)"),
            Err(e) => warn!("Failed to load playout_ringbuffer_headroom: {}, using default", e),
        }

        // [DBD-PARAM-085] decoder_resume_hysteresis_samples (u64, range: [2205, 441000])
        match load_u64_param(db_pool, "decoder_resume_hysteresis_samples").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_decoder_resume_hysteresis_samples(value) {
                    warn!("{}, using default (44100)", e);
                }
            }
            Ok(None) => warn!("decoder_resume_hysteresis_samples not found in database, using default (44100)"),
            Err(e) => warn!("Failed to load decoder_resume_hysteresis_samples: {}, using default", e),
        }

        // [DBD-PARAM-088] mixer_min_start_level (usize, range: [2205, 88200])
        match load_usize_param(db_pool, "mixer_min_start_level").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_mixer_min_start_level(value) {
                    warn!("{}, using default (22050)", e);
                }
            }
            Ok(None) => warn!("mixer_min_start_level not found in database, using default (22050)"),
            Err(e) => warn!("Failed to load mixer_min_start_level: {}, using default", e),
        }

        // [DBD-PARAM-090] pause_decay_factor (f64, range: [0.5, 0.99])
        match load_f64_param(db_pool, "pause_decay_factor").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_pause_decay_factor(value) {
                    warn!("{}, using default (0.95)", e);
                }
            }
            Ok(None) => warn!("pause_decay_factor not found in database, using default (0.95)"),
            Err(e) => warn!("Failed to load pause_decay_factor: {}, using default", e),
        }

        // [DBD-PARAM-100] pause_decay_floor (f64, range: [0.00001, 0.001])
        match load_f64_param(db_pool, "pause_decay_floor").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_pause_decay_floor(value) {
                    warn!("{}, using default (0.0001778)", e);
                }
            }
            Ok(None) => warn!("pause_decay_floor not found in database, using default (0.0001778)"),
            Err(e) => warn!("Failed to load pause_decay_floor: {}, using default", e),
        }

        // [DBD-PARAM-110] audio_buffer_size (u32, range: [512, 8192])
        match load_u32_param(db_pool, "audio_buffer_size").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_audio_buffer_size(value) {
                    warn!("{}, using default (2208)", e);
                }
            }
            Ok(None) => warn!("audio_buffer_size not found in database, using default (2208)"),
            Err(e) => warn!("Failed to load audio_buffer_size: {}, using default", e),
        }

        // [DBD-PARAM-111] mixer_check_interval_ms (u64, range: [5, 100])
        match load_u64_param(db_pool, "mixer_check_interval_ms").await {
            Ok(Some(value)) => {
                if let Err(e) = PARAMS.set_mixer_check_interval_ms(value) {
                    warn!("{}, using default (10)", e);
                }
            }
            Ok(None) => warn!("mixer_check_interval_ms not found in database, using default (10)"),
            Err(e) => warn!("Failed to load mixer_check_interval_ms: {}, using default", e),
        }

        info!("GlobalParams initialized from database");
        Ok(())
    }

    /// Validate and update working_sample_rate
    ///
    /// # Validation
    /// - Must be in range [8000, 192000] Hz
    /// - Common audio sample rates preferred
    pub fn set_working_sample_rate(&self, value: u32) -> Result<(), String> {
        if value < 8000 || value > 192000 {
            return Err(format!(
                "working_sample_rate {} out of range [8000, 192000]",
                value
            ));
        }

        *self.working_sample_rate.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update volume_level
    ///
    /// # Validation
    /// - Must be in range [0.0, 1.0]
    /// - Values clamped to range
    pub fn set_volume_level(&self, value: f32) -> Result<(), String> {
        let clamped = value.clamp(0.0, 1.0);
        if clamped != value {
            tracing::warn!(
                "volume_level {} clamped to {}",
                value,
                clamped
            );
        }

        *self.volume_level.write().unwrap() = clamped;
        Ok(())
    }

    /// Validate and update output_ringbuffer_size
    ///
    /// # Validation
    /// - Must be in range [2048, 262144] frames
    pub fn set_output_ringbuffer_size(&self, value: usize) -> Result<(), String> {
        if value < 2048 || value > 262144 {
            return Err(format!(
                "output_ringbuffer_size {} out of range [2048, 262144]",
                value
            ));
        }
        *self.output_ringbuffer_size.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update maximum_decode_streams
    ///
    /// # Validation
    /// - Must be in range [1, 32]
    pub fn set_maximum_decode_streams(&self, value: usize) -> Result<(), String> {
        if value < 1 || value > 32 {
            return Err(format!(
                "maximum_decode_streams {} out of range [1, 32]",
                value
            ));
        }
        *self.maximum_decode_streams.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update decode_work_period
    ///
    /// # Validation
    /// - Must be in range [100, 60000] ms
    pub fn set_decode_work_period(&self, value: u64) -> Result<(), String> {
        if value < 100 || value > 60000 {
            return Err(format!(
                "decode_work_period {} out of range [100, 60000]",
                value
            ));
        }
        *self.decode_work_period.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update chunk_duration_ms
    ///
    /// # Validation
    /// - Must be in range [250, 5000] ms
    pub fn set_chunk_duration_ms(&self, value: u64) -> Result<(), String> {
        if value < 250 || value > 5000 {
            return Err(format!(
                "chunk_duration_ms {} out of range [250, 5000]",
                value
            ));
        }
        *self.chunk_duration_ms.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update playout_ringbuffer_size
    ///
    /// # Validation
    /// - Must be in range [44100, 10000000] samples
    pub fn set_playout_ringbuffer_size(&self, value: usize) -> Result<(), String> {
        if value < 44100 || value > 10000000 {
            return Err(format!(
                "playout_ringbuffer_size {} out of range [44100, 10000000]",
                value
            ));
        }
        *self.playout_ringbuffer_size.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update playout_ringbuffer_headroom
    ///
    /// # Validation
    /// - Must be in range [2205, 88200] samples
    pub fn set_playout_ringbuffer_headroom(&self, value: usize) -> Result<(), String> {
        if value < 2205 || value > 88200 {
            return Err(format!(
                "playout_ringbuffer_headroom {} out of range [2205, 88200]",
                value
            ));
        }
        *self.playout_ringbuffer_headroom.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update decoder_resume_hysteresis_samples
    ///
    /// # Validation
    /// - Must be in range [2205, 441000] samples
    pub fn set_decoder_resume_hysteresis_samples(&self, value: u64) -> Result<(), String> {
        if value < 2205 || value > 441000 {
            return Err(format!(
                "decoder_resume_hysteresis_samples {} out of range [2205, 441000]",
                value
            ));
        }
        *self.decoder_resume_hysteresis_samples.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update mixer_min_start_level
    ///
    /// # Validation
    /// - Must be in range [2205, 88200] samples
    pub fn set_mixer_min_start_level(&self, value: usize) -> Result<(), String> {
        if value < 2205 || value > 88200 {
            return Err(format!(
                "mixer_min_start_level {} out of range [2205, 88200]",
                value
            ));
        }
        *self.mixer_min_start_level.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update pause_decay_factor
    ///
    /// # Validation
    /// - Must be in range [0.5, 0.99]
    pub fn set_pause_decay_factor(&self, value: f64) -> Result<(), String> {
        if value < 0.5 || value > 0.99 {
            return Err(format!(
                "pause_decay_factor {} out of range [0.5, 0.99]",
                value
            ));
        }
        *self.pause_decay_factor.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update pause_decay_floor
    ///
    /// # Validation
    /// - Must be in range [0.00001, 0.001]
    pub fn set_pause_decay_floor(&self, value: f64) -> Result<(), String> {
        if value < 0.00001 || value > 0.001 {
            return Err(format!(
                "pause_decay_floor {} out of range [0.00001, 0.001]",
                value
            ));
        }
        *self.pause_decay_floor.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update audio_buffer_size
    ///
    /// # Validation
    /// - Must be in range [512, 8192] frames
    pub fn set_audio_buffer_size(&self, value: u32) -> Result<(), String> {
        if value < 512 || value > 8192 {
            return Err(format!(
                "audio_buffer_size {} out of range [512, 8192]",
                value
            ));
        }
        *self.audio_buffer_size.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update mixer_check_interval_ms
    ///
    /// # Validation
    /// - Must be in range [5, 100] ms
    pub fn set_mixer_check_interval_ms(&self, value: u64) -> Result<(), String> {
        if value < 5 || value > 100 {
            return Err(format!(
                "mixer_check_interval_ms {} out of range [5, 100]",
                value
            ));
        }
        *self.mixer_check_interval_ms.write().unwrap() = value;
        Ok(())
    }
}

/// Helper function to load f32 parameter from database
async fn load_f32_param(
    pool: &sqlx::SqlitePool,
    key: &str,
) -> Result<Option<f32>, Box<dyn std::error::Error>> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match row {
        Some((Some(value_str),)) => {
            let value = value_str.parse::<f32>()?;
            Ok(Some(value))
        }
        Some((None,)) => Ok(None), // NULL value
        None => Ok(None),           // Missing row
    }
}

/// Helper function to load f64 parameter from database
async fn load_f64_param(
    pool: &sqlx::SqlitePool,
    key: &str,
) -> Result<Option<f64>, Box<dyn std::error::Error>> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match row {
        Some((Some(value_str),)) => {
            let value = value_str.parse::<f64>()?;
            Ok(Some(value))
        }
        Some((None,)) => Ok(None), // NULL value
        None => Ok(None),           // Missing row
    }
}

/// Helper function to load u32 parameter from database
async fn load_u32_param(
    pool: &sqlx::SqlitePool,
    key: &str,
) -> Result<Option<u32>, Box<dyn std::error::Error>> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match row {
        Some((Some(value_str),)) => {
            let value = value_str.parse::<u32>()?;
            Ok(Some(value))
        }
        Some((None,)) => Ok(None), // NULL value
        None => Ok(None),           // Missing row
    }
}

/// Helper function to load u64 parameter from database
async fn load_u64_param(
    pool: &sqlx::SqlitePool,
    key: &str,
) -> Result<Option<u64>, Box<dyn std::error::Error>> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match row {
        Some((Some(value_str),)) => {
            let value = value_str.parse::<u64>()?;
            Ok(Some(value))
        }
        Some((None,)) => Ok(None), // NULL value
        None => Ok(None),           // Missing row
    }
}

/// Helper function to load usize parameter from database
async fn load_usize_param(
    pool: &sqlx::SqlitePool,
    key: &str,
) -> Result<Option<usize>, Box<dyn std::error::Error>> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match row {
        Some((Some(value_str),)) => {
            let value = value_str.parse::<usize>()?;
            Ok(Some(value))
        }
        Some((None,)) => Ok(None), // NULL value
        None => Ok(None),           // Missing row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_params_has_all_fields() {
        // TC-U-001-01: Verify all 15 parameter fields exist
        let params = GlobalParams::default();

        // DBD-PARAM-010 - dereference to avoid lock warning
        let _: f32 = *params.volume_level.read().unwrap();

        // DBD-PARAM-020
        let _: u32 = *params.working_sample_rate.read().unwrap();

        // DBD-PARAM-030
        let _: usize = *params.output_ringbuffer_size.read().unwrap();

        // DBD-PARAM-050
        let _: usize = *params.maximum_decode_streams.read().unwrap();

        // DBD-PARAM-060
        let _: u64 = *params.decode_work_period.read().unwrap();

        // DBD-PARAM-065
        let _: u64 = *params.chunk_duration_ms.read().unwrap();

        // DBD-PARAM-070
        let _: usize = *params.playout_ringbuffer_size.read().unwrap();

        // DBD-PARAM-080
        let _: usize = *params.playout_ringbuffer_headroom.read().unwrap();

        // DBD-PARAM-085
        let _: u64 = *params.decoder_resume_hysteresis_samples.read().unwrap();

        // DBD-PARAM-088
        let _: usize = *params.mixer_min_start_level.read().unwrap();

        // DBD-PARAM-090
        let _: f64 = *params.pause_decay_factor.read().unwrap();

        // DBD-PARAM-100
        let _: f64 = *params.pause_decay_floor.read().unwrap();

        // DBD-PARAM-110
        let _: u32 = *params.audio_buffer_size.read().unwrap();

        // DBD-PARAM-111
        let _: u64 = *params.mixer_check_interval_ms.read().unwrap();

        // If we reach here, all 15 fields exist and are accessible
        assert!(true, "All 15 parameter fields exist");
    }

    #[test]
    fn test_parameter_field_types() {
        // TC-U-001-01: Verify types (compile-time check via type inference)
        let params = GlobalParams::default();

        let _: f32 = *params.volume_level.read().unwrap();
        let _: u32 = *params.working_sample_rate.read().unwrap();
        let _: usize = *params.output_ringbuffer_size.read().unwrap();
        let _: usize = *params.maximum_decode_streams.read().unwrap();
        let _: u64 = *params.decode_work_period.read().unwrap();
        let _: u64 = *params.chunk_duration_ms.read().unwrap();
        let _: usize = *params.playout_ringbuffer_size.read().unwrap();
        let _: usize = *params.playout_ringbuffer_headroom.read().unwrap();
        let _: u64 = *params.decoder_resume_hysteresis_samples.read().unwrap();
        let _: usize = *params.mixer_min_start_level.read().unwrap();
        let _: f64 = *params.pause_decay_factor.read().unwrap();
        let _: f64 = *params.pause_decay_floor.read().unwrap();
        let _: u32 = *params.audio_buffer_size.read().unwrap();
        let _: u64 = *params.mixer_check_interval_ms.read().unwrap();
    }

    #[test]
    fn test_default_values() {
        // TC-U-001-02: Verify default values match SPEC016
        let params = GlobalParams::default();

        assert_eq!(*params.volume_level.read().unwrap(), 0.5);
        assert_eq!(*params.working_sample_rate.read().unwrap(), 44100);
        assert_eq!(*params.output_ringbuffer_size.read().unwrap(), 8192); // [DBD-PARAM-030] 8192 frames = 186ms @ 44.1kHz
        assert_eq!(*params.maximum_decode_streams.read().unwrap(), 12);
        assert_eq!(*params.decode_work_period.read().unwrap(), 5000);
        assert_eq!(*params.chunk_duration_ms.read().unwrap(), 1000);
        assert_eq!(*params.playout_ringbuffer_size.read().unwrap(), 661941);
        assert_eq!(*params.playout_ringbuffer_headroom.read().unwrap(), 4410);
        assert_eq!(*params.decoder_resume_hysteresis_samples.read().unwrap(), 44100);
        assert_eq!(*params.mixer_min_start_level.read().unwrap(), 22050);
        assert_eq!(*params.pause_decay_factor.read().unwrap(), 0.95);
        assert_eq!(*params.pause_decay_floor.read().unwrap(), 0.0001778);
        assert_eq!(*params.audio_buffer_size.read().unwrap(), 2208);
        assert_eq!(*params.mixer_check_interval_ms.read().unwrap(), 10);
    }

    #[test]
    fn test_rwlock_read_access() {
        // TC-U-002-01: Verify RwLock read access succeeds
        let params = GlobalParams::default();

        let sample_rate = *params.working_sample_rate.read().unwrap();
        assert_eq!(sample_rate, 44100);
    }

    #[test]
    fn test_rwlock_write_access() {
        // TC-U-002-02: Verify RwLock write access succeeds
        let params = GlobalParams::default();

        *params.working_sample_rate.write().unwrap() = 48000;
        assert_eq!(*params.working_sample_rate.read().unwrap(), 48000);
    }

    #[test]
    fn test_concurrent_reads() {
        // TC-U-002-03: Verify concurrent RwLock reads succeed
        use std::sync::Arc;
        use std::thread;

        let params = Arc::new(GlobalParams::default());
        let mut handles = vec![];

        // Spawn 10 threads all reading simultaneously
        for _ in 0..10 {
            let params_clone = Arc::clone(&params);
            let handle = thread::spawn(move || {
                let rate = *params_clone.working_sample_rate.read().unwrap();
                assert_eq!(rate, 44100);
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_set_working_sample_rate_valid() {
        // TC-U-102-02: Validate working_sample_rate range
        let params = GlobalParams::default();

        assert!(params.set_working_sample_rate(48000).is_ok());
        assert_eq!(*params.working_sample_rate.read().unwrap(), 48000);

        assert!(params.set_working_sample_rate(44100).is_ok());
        assert_eq!(*params.working_sample_rate.read().unwrap(), 44100);
    }

    #[test]
    fn test_set_working_sample_rate_out_of_range() {
        // TC-U-102-02: Validate working_sample_rate range enforcement
        let params = GlobalParams::default();

        assert!(params.set_working_sample_rate(7999).is_err());
        assert!(params.set_working_sample_rate(192001).is_err());

        // Value should remain at default after failed set
        assert_eq!(*params.working_sample_rate.read().unwrap(), 44100);
    }

    #[test]
    fn test_set_volume_level_clamping() {
        // TC-U-102-01: Validate volume_level range (with clamping)
        let params = GlobalParams::default();

        assert!(params.set_volume_level(0.75).is_ok());
        assert_eq!(*params.volume_level.read().unwrap(), 0.75);

        // Out of range values get clamped
        assert!(params.set_volume_level(1.5).is_ok());
        assert_eq!(*params.volume_level.read().unwrap(), 1.0);

        assert!(params.set_volume_level(-0.1).is_ok());
        assert_eq!(*params.volume_level.read().unwrap(), 0.0);
    }

    // Database loading tests
    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_with_all_values() {
        // TC-DB-001: Load all parameters from database when all values present
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert all parameter values
        insert_setting(&pool, "volume_level", "0.75").await;
        insert_setting(&pool, "working_sample_rate", "48000").await;
        insert_setting(&pool, "output_ringbuffer_size", "16384").await;
        insert_setting(&pool, "maximum_decode_streams", "8").await;
        insert_setting(&pool, "decode_work_period", "3000").await;
        insert_setting(&pool, "chunk_duration_ms", "500").await;
        insert_setting(&pool, "playout_ringbuffer_size", "882000").await;
        insert_setting(&pool, "playout_ringbuffer_headroom", "8820").await;
        insert_setting(&pool, "decoder_resume_hysteresis_samples", "88200").await;
        insert_setting(&pool, "mixer_min_start_level", "44100").await;
        insert_setting(&pool, "pause_decay_factor", "0.90").await;
        insert_setting(&pool, "pause_decay_floor", "0.0002").await;
        insert_setting(&pool, "audio_buffer_size", "4096").await;
        insert_setting(&pool, "mixer_check_interval_ms", "20").await;

        // Initialize from database
        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify all values loaded
        assert_eq!(*PARAMS.volume_level.read().unwrap(), 0.75);
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 48000);
        assert_eq!(*PARAMS.output_ringbuffer_size.read().unwrap(), 16384);
        assert_eq!(*PARAMS.maximum_decode_streams.read().unwrap(), 8);
        assert_eq!(*PARAMS.decode_work_period.read().unwrap(), 3000);
        assert_eq!(*PARAMS.chunk_duration_ms.read().unwrap(), 500);
        assert_eq!(*PARAMS.playout_ringbuffer_size.read().unwrap(), 882000);
        assert_eq!(*PARAMS.playout_ringbuffer_headroom.read().unwrap(), 8820);
        assert_eq!(*PARAMS.decoder_resume_hysteresis_samples.read().unwrap(), 88200);
        assert_eq!(*PARAMS.mixer_min_start_level.read().unwrap(), 44100);
        assert_eq!(*PARAMS.pause_decay_factor.read().unwrap(), 0.90);
        assert_eq!(*PARAMS.pause_decay_floor.read().unwrap(), 0.0002);
        assert_eq!(*PARAMS.audio_buffer_size.read().unwrap(), 4096);
        assert_eq!(*PARAMS.mixer_check_interval_ms.read().unwrap(), 20);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_with_missing_values() {
        // TC-DB-002: Use defaults when parameters missing from database
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Don't insert any parameters
        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify defaults used (should match Default implementation)
        assert_eq!(*PARAMS.volume_level.read().unwrap(), 0.5);
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 44100);
        assert_eq!(*PARAMS.output_ringbuffer_size.read().unwrap(), 8192);
        assert_eq!(*PARAMS.maximum_decode_streams.read().unwrap(), 12);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_with_out_of_range_values() {
        // TC-DB-003: Use defaults when parameters out of range
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert out-of-range values
        insert_setting(&pool, "working_sample_rate", "7000").await;  // Too low (min: 8000)
        insert_setting(&pool, "maximum_decode_streams", "50").await;  // Too high (max: 32)
        insert_setting(&pool, "audio_buffer_size", "100000").await;   // Too high (max: 8192)

        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify defaults used for out-of-range values
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 44100);
        assert_eq!(*PARAMS.maximum_decode_streams.read().unwrap(), 12);
        assert_eq!(*PARAMS.audio_buffer_size.read().unwrap(), 2208);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_with_type_mismatch() {
        // TC-DB-004: Use defaults when type mismatch (invalid parse)
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert non-numeric values for numeric parameters
        insert_setting(&pool, "working_sample_rate", "not-a-number").await;
        insert_setting(&pool, "volume_level", "invalid").await;

        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify defaults used when parsing fails
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 44100);
        assert_eq!(*PARAMS.volume_level.read().unwrap(), 0.5);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_with_null_values() {
        // TC-DB-005: Use defaults when values are NULL
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert NULL values
        sqlx::query("INSERT INTO settings (key, value) VALUES (?, NULL)")
            .bind("working_sample_rate")
            .execute(&pool)
            .await
            .unwrap();

        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify defaults used for NULL values
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 44100);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_init_from_database_partial_values() {
        // TC-DB-006: Load some parameters, use defaults for others
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert only some parameters
        insert_setting(&pool, "volume_level", "0.8").await;
        insert_setting(&pool, "working_sample_rate", "96000").await;
        // Omit other parameters

        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify loaded parameters
        assert_eq!(*PARAMS.volume_level.read().unwrap(), 0.8);
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 96000);

        // Verify defaults for missing parameters
        assert_eq!(*PARAMS.output_ringbuffer_size.read().unwrap(), 8192);
        assert_eq!(*PARAMS.maximum_decode_streams.read().unwrap(), 12);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_volume_level_clamping_from_database() {
        // TC-DB-007: Volume level clamping works when loading from database
        PARAMS.reset_to_defaults(); // Reset before test
        let pool = create_test_db().await;

        // Insert out-of-range volume (should be clamped)
        insert_setting(&pool, "volume_level", "1.5").await;

        GlobalParams::init_from_database(&pool).await.unwrap();

        // Verify clamped to 1.0
        assert_eq!(*PARAMS.volume_level.read().unwrap(), 1.0);
    }

    // Setter validation tests
    #[test]
    fn test_set_output_ringbuffer_size_valid() {
        let params = GlobalParams::default();

        // Valid values
        assert!(params.set_output_ringbuffer_size(2048).is_ok());
        assert_eq!(*params.output_ringbuffer_size.read().unwrap(), 2048);

        assert!(params.set_output_ringbuffer_size(16384).is_ok());
        assert_eq!(*params.output_ringbuffer_size.read().unwrap(), 16384);

        assert!(params.set_output_ringbuffer_size(262144).is_ok());
        assert_eq!(*params.output_ringbuffer_size.read().unwrap(), 262144);
    }

    #[test]
    fn test_set_output_ringbuffer_size_out_of_range() {
        let params = GlobalParams::default();

        // Out of range values
        assert!(params.set_output_ringbuffer_size(2047).is_err());
        assert!(params.set_output_ringbuffer_size(262145).is_err());

        // Value should remain at default after failed set
        assert_eq!(*params.output_ringbuffer_size.read().unwrap(), 8192);
    }

    #[test]
    fn test_set_pause_decay_factor_valid() {
        let params = GlobalParams::default();

        // Valid values
        assert!(params.set_pause_decay_factor(0.5).is_ok());
        assert_eq!(*params.pause_decay_factor.read().unwrap(), 0.5);

        assert!(params.set_pause_decay_factor(0.90).is_ok());
        assert_eq!(*params.pause_decay_factor.read().unwrap(), 0.90);

        assert!(params.set_pause_decay_factor(0.99).is_ok());
        assert_eq!(*params.pause_decay_factor.read().unwrap(), 0.99);
    }

    #[test]
    fn test_set_pause_decay_factor_out_of_range() {
        let params = GlobalParams::default();

        // Out of range values
        assert!(params.set_pause_decay_factor(0.49).is_err());
        assert!(params.set_pause_decay_factor(1.0).is_err());

        // Value should remain at default after failed set
        assert_eq!(*params.pause_decay_factor.read().unwrap(), 0.95);
    }

    #[test]
    fn test_set_audio_buffer_size_valid() {
        let params = GlobalParams::default();

        // Valid values
        assert!(params.set_audio_buffer_size(512).is_ok());
        assert_eq!(*params.audio_buffer_size.read().unwrap(), 512);

        assert!(params.set_audio_buffer_size(4096).is_ok());
        assert_eq!(*params.audio_buffer_size.read().unwrap(), 4096);

        assert!(params.set_audio_buffer_size(8192).is_ok());
        assert_eq!(*params.audio_buffer_size.read().unwrap(), 8192);
    }

    #[test]
    fn test_set_audio_buffer_size_out_of_range() {
        let params = GlobalParams::default();

        // Out of range values
        assert!(params.set_audio_buffer_size(511).is_err());
        assert!(params.set_audio_buffer_size(8193).is_err());

        // Value should remain at default after failed set
        assert_eq!(*params.audio_buffer_size.read().unwrap(), 2208);
    }

    #[test]
    fn test_set_maximum_decode_streams_valid() {
        let params = GlobalParams::default();

        // Valid values
        assert!(params.set_maximum_decode_streams(1).is_ok());
        assert_eq!(*params.maximum_decode_streams.read().unwrap(), 1);

        assert!(params.set_maximum_decode_streams(16).is_ok());
        assert_eq!(*params.maximum_decode_streams.read().unwrap(), 16);

        assert!(params.set_maximum_decode_streams(32).is_ok());
        assert_eq!(*params.maximum_decode_streams.read().unwrap(), 32);
    }

    #[test]
    fn test_set_maximum_decode_streams_out_of_range() {
        let params = GlobalParams::default();

        // Out of range values
        assert!(params.set_maximum_decode_streams(0).is_err());
        assert!(params.set_maximum_decode_streams(33).is_err());

        // Value should remain at default after failed set
        assert_eq!(*params.maximum_decode_streams.read().unwrap(), 12);
    }

    // Test helper functions
    async fn create_test_db() -> sqlx::SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Create settings table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create settings table");

        pool
    }

    async fn insert_setting(pool: &sqlx::SqlitePool, key: &str, value: &str) {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(pool)
            .await
            .expect("Failed to insert setting");
    }
}
