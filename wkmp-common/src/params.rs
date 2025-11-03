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

    /// **[DBD-PARAM-040]** Milliseconds between mixer checks
    ///
    /// Valid range: [10, 1000] ms
    /// Default: 90 ms
    /// How often mixer wakes to refill output buffer
    pub output_refill_period: RwLock<u64>,

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

            // [DBD-PARAM-040] Output refill period
            output_refill_period: RwLock::new(90),

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
        _db_pool: &sqlx::SqlitePool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement database loading
        // For now, just use defaults
        tracing::info!("GlobalParams initialized with default values (database loading not yet implemented)");
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

    // TODO: Add setter methods for remaining 13 parameters with validation
    // Following same pattern as above:
    // - Range validation
    // - Clear error messages
    // - Type-safe
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

        // DBD-PARAM-040
        let _: u64 = *params.output_refill_period.read().unwrap();

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
        let _: u64 = *params.output_refill_period.read().unwrap();
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
        assert_eq!(*params.output_refill_period.read().unwrap(), 90);
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
}
