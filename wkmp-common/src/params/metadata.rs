//! Parameter metadata definitions
//!
//! **[PLAN019-REQ-DRY-020]** Single source of truth for parameter validation

use super::{GlobalParams, ParamMetadata};

impl GlobalParams {
    /// Get metadata for all 14 database-backed parameters
    ///
    /// **[PLAN019-REQ-DRY-020]** Returns static reference to parameter metadata array.
    /// This is the single source of truth for:
    /// - Parameter names and types
    /// - Default values
    /// - Validation ranges
    /// - Validation logic
    ///
    /// # Example: Validating a Parameter
    ///
    /// ```rust
    /// # use wkmp_common::params::GlobalParams;
    /// let metadata = GlobalParams::metadata();
    /// let volume_meta = metadata.iter()
    ///     .find(|m| m.key == "volume_level")
    ///     .unwrap();
    ///
    /// // Validate value using metadata
    /// assert!((volume_meta.validator)("0.5").is_ok());
    /// assert!((volume_meta.validator)("2.0").is_err());
    /// ```
    pub fn metadata() -> &'static [ParamMetadata] {
        &[
            // [DBD-PARAM-010] Volume Level
            ParamMetadata {
                key: "volume_level",
                data_type: "f32",
                default_value: "0.5",
                description: "[DBD-PARAM-010] Audio output volume",
                validation_range: "0.0-1.0",
                validator: |s| {
                    let v: f32 = s.parse()
                        .map_err(|_| "volume_level: invalid number format".to_string())?;
                    if !(0.0..=1.0).contains(&v) {
                        return Err(format!("volume_level: value {} out of range [0.0, 1.0]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-020] Working Sample Rate
            ParamMetadata {
                key: "working_sample_rate",
                data_type: "u32",
                default_value: "44100",
                description: "[DBD-PARAM-020] Working sample rate for decoded audio (Hz)",
                validation_range: "8000-192000",
                validator: |s| {
                    let v: u32 = s.parse()
                        .map_err(|_| "working_sample_rate: invalid number format".to_string())?;
                    if !(8000..=192000).contains(&v) {
                        return Err(format!("working_sample_rate: value {} out of range [8000, 192000]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-030] Output Ring Buffer Size
            ParamMetadata {
                key: "output_ringbuffer_size",
                data_type: "usize",
                default_value: "8192",
                description: "[DBD-PARAM-030] Output ring buffer capacity (stereo frames)",
                validation_range: "2048-262144",
                validator: |s| {
                    let v: usize = s.parse()
                        .map_err(|_| "output_ringbuffer_size: invalid number format".to_string())?;
                    if !(2048..=262144).contains(&v) {
                        return Err(format!("output_ringbuffer_size: value {} out of range [2048, 262144]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-050] Maximum Decode Streams
            ParamMetadata {
                key: "maximum_decode_streams",
                data_type: "usize",
                default_value: "12",
                description: "[DBD-PARAM-050] Maximum parallel decoder chains",
                validation_range: "1-32",
                validator: |s| {
                    let v: usize = s.parse()
                        .map_err(|_| "maximum_decode_streams: invalid number format".to_string())?;
                    if !(1..=32).contains(&v) {
                        return Err(format!("maximum_decode_streams: value {} out of range [1, 32]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-060] Decode Work Period
            ParamMetadata {
                key: "decode_work_period",
                data_type: "u64",
                default_value: "5000",
                description: "[DBD-PARAM-060] Decode priority evaluation period (ms)",
                validation_range: "100-60000",
                validator: |s| {
                    let v: u64 = s.parse()
                        .map_err(|_| "decode_work_period: invalid number format".to_string())?;
                    if !(100..=60000).contains(&v) {
                        return Err(format!("decode_work_period: value {} out of range [100, 60000]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-065] Chunk Duration
            ParamMetadata {
                key: "chunk_duration_ms",
                data_type: "u64",
                default_value: "1000",
                description: "[DBD-PARAM-065] Decode chunk duration (ms)",
                validation_range: "250-5000",
                validator: |s| {
                    let v: u64 = s.parse()
                        .map_err(|_| "chunk_duration_ms: invalid number format".to_string())?;
                    if !(250..=5000).contains(&v) {
                        return Err(format!("chunk_duration_ms: value {} out of range [250, 5000]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-070] Playout Ring Buffer Size
            ParamMetadata {
                key: "playout_ringbuffer_size",
                data_type: "usize",
                default_value: "661941",
                description: "[DBD-PARAM-070] Decoded audio buffer size (samples)",
                validation_range: "44100-10000000",
                validator: |s| {
                    let v: usize = s.parse()
                        .map_err(|_| "playout_ringbuffer_size: invalid number format".to_string())?;
                    if !(44100..=10000000).contains(&v) {
                        return Err(format!("playout_ringbuffer_size: value {} out of range [44100, 10000000]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-080] Playout Buffer Headroom
            ParamMetadata {
                key: "playout_ringbuffer_headroom",
                data_type: "usize",
                default_value: "4410",
                description: "[DBD-PARAM-080] Buffer headroom for late resampler samples (stereo frames)",
                validation_range: "2205-88200",
                validator: |s| {
                    let v: usize = s.parse()
                        .map_err(|_| "playout_ringbuffer_headroom: invalid number format".to_string())?;
                    if !(2205..=88200).contains(&v) {
                        return Err(format!("playout_ringbuffer_headroom: value {} out of range [2205, 88200]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-085] Decoder Resume Hysteresis
            ParamMetadata {
                key: "decoder_resume_hysteresis_samples",
                data_type: "u64",
                default_value: "44100",
                description: "[DBD-PARAM-085] Hysteresis for decoder pause/resume (samples)",
                validation_range: "2205-441000",
                validator: |s| {
                    let v: u64 = s.parse()
                        .map_err(|_| "decoder_resume_hysteresis_samples: invalid number format".to_string())?;
                    if !(2205..=441000).contains(&v) {
                        return Err(format!("decoder_resume_hysteresis_samples: value {} out of range [2205, 441000]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-088] Mixer Minimum Start Level
            ParamMetadata {
                key: "mixer_min_start_level",
                data_type: "usize",
                default_value: "22050",
                description: "[DBD-PARAM-088] Min samples before mixer starts playback",
                validation_range: "2205-88200",
                validator: |s| {
                    let v: usize = s.parse()
                        .map_err(|_| "mixer_min_start_level: invalid number format".to_string())?;
                    if !(2205..=88200).contains(&v) {
                        return Err(format!("mixer_min_start_level: value {} out of range [2205, 88200]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-090] Pause Decay Factor
            ParamMetadata {
                key: "pause_decay_factor",
                data_type: "f64",
                default_value: "0.95",
                description: "[DBD-PARAM-090] Exponential decay factor in pause mode",
                validation_range: "0.5-0.99",
                validator: |s| {
                    let v: f64 = s.parse()
                        .map_err(|_| "pause_decay_factor: invalid number format".to_string())?;
                    if !(0.5..=0.99).contains(&v) {
                        return Err(format!("pause_decay_factor: value {} out of range [0.5, 0.99]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-100] Pause Decay Floor
            ParamMetadata {
                key: "pause_decay_floor",
                data_type: "f64",
                default_value: "0.0001778",
                description: "[DBD-PARAM-100] Minimum level before outputting zero",
                validation_range: "0.00001-0.001",
                validator: |s| {
                    let v: f64 = s.parse()
                        .map_err(|_| "pause_decay_floor: invalid number format".to_string())?;
                    if !(0.00001..=0.001).contains(&v) {
                        return Err(format!("pause_decay_floor: value {} out of range [0.00001, 0.001]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-110] Audio Buffer Size
            ParamMetadata {
                key: "audio_buffer_size",
                data_type: "u32",
                default_value: "2208",
                description: "[DBD-PARAM-110] Audio output buffer size (frames/callback)",
                validation_range: "512-8192",
                validator: |s| {
                    let v: u32 = s.parse()
                        .map_err(|_| "audio_buffer_size: invalid number format".to_string())?;
                    if !(512..=8192).contains(&v) {
                        return Err(format!("audio_buffer_size: value {} out of range [512, 8192]", v));
                    }
                    Ok(())
                },
            },

            // [DBD-PARAM-111] Mixer Check Interval
            ParamMetadata {
                key: "mixer_check_interval_ms",
                data_type: "u64",
                default_value: "10",
                description: "[DBD-PARAM-111] Mixer thread check interval (ms)",
                validation_range: "5-100",
                validator: |s| {
                    let v: u64 = s.parse()
                        .map_err(|_| "mixer_check_interval_ms: invalid number format".to_string())?;
                    if !(5..=100).contains(&v) {
                        return Err(format!("mixer_check_interval_ms: value {} out of range [5, 100]", v));
                    }
                    Ok(())
                },
            },
        ]
    }
}
