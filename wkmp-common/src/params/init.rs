//! Parameter initialization and database loading
//!
//! **[PLAN019-REQ-DRY-040]** Metadata-based parameter loading from database

use super::{GlobalParams, ParamMetadata, PARAMS};

impl GlobalParams {
    /// Reset all parameters to defaults (for testing only)
    #[cfg(test)]
    pub(super) fn reset_to_defaults(&self) {
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
    /// **[PLAN019-REQ-DRY-040]** Refactored to use metadata validators for all parameters.
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
    ///
    /// # Metadata-Based Loading
    ///
    /// Uses ParamMetadata validators to eliminate duplication. For each parameter:
    /// 1. Load string value from database
    /// 2. Validate using metadata validator
    /// 3. If valid, call setter to update value
    /// 4. If invalid/missing, log warning and use default
    pub async fn init_from_database(
        db_pool: &sqlx::SqlitePool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use tracing::{info, warn};

        info!("Loading GlobalParams from database...");

        // Helper: Load and validate parameter using metadata
        async fn load_and_validate_param(
            db_pool: &sqlx::SqlitePool,
            meta: &ParamMetadata,
        ) -> Option<String> {
            use tracing::warn;

            match load_string_param(db_pool, meta.key).await {
                Ok(Some(value_str)) => {
                    match (meta.validator)(&value_str) {
                        Ok(()) => Some(value_str),
                        Err(e) => {
                            warn!("{}, using default ({})", e, meta.default_value);
                            None
                        }
                    }
                }
                Ok(None) => {
                    warn!("{} not found in database, using default ({})", meta.key, meta.default_value);
                    None
                }
                Err(e) => {
                    warn!("Failed to load {}: {}, using default ({})", meta.key, e, meta.default_value);
                    None
                }
            }
        }

        // Get metadata array
        let metadata = Self::metadata();

        // Process each parameter using metadata validators
        for meta in metadata {
            if let Some(value_str) = load_and_validate_param(db_pool, meta).await {
                // Value validated successfully, now call setter with parsed value
                // Setters handle type conversion and provide additional safeguards
                let _ = match meta.key {
                    "volume_level" => {
                        let v: f32 = value_str.parse().unwrap(); // Already validated
                        PARAMS.set_volume_level(v)
                    }
                    "working_sample_rate" => {
                        let v: u32 = value_str.parse().unwrap();
                        PARAMS.set_working_sample_rate(v)
                    }
                    "output_ringbuffer_size" => {
                        let v: usize = value_str.parse().unwrap();
                        PARAMS.set_output_ringbuffer_size(v)
                    }
                    "maximum_decode_streams" => {
                        let v: usize = value_str.parse().unwrap();
                        PARAMS.set_maximum_decode_streams(v)
                    }
                    "decode_work_period" => {
                        let v: u64 = value_str.parse().unwrap();
                        PARAMS.set_decode_work_period(v)
                    }
                    "chunk_duration_ms" => {
                        let v: u64 = value_str.parse().unwrap();
                        PARAMS.set_chunk_duration_ms(v)
                    }
                    "playout_ringbuffer_size" => {
                        let v: usize = value_str.parse().unwrap();
                        PARAMS.set_playout_ringbuffer_size(v)
                    }
                    "playout_ringbuffer_headroom" => {
                        let v: usize = value_str.parse().unwrap();
                        PARAMS.set_playout_ringbuffer_headroom(v)
                    }
                    "decoder_resume_hysteresis_samples" => {
                        let v: u64 = value_str.parse().unwrap();
                        PARAMS.set_decoder_resume_hysteresis_samples(v)
                    }
                    "mixer_min_start_level" => {
                        let v: usize = value_str.parse().unwrap();
                        PARAMS.set_mixer_min_start_level(v)
                    }
                    "pause_decay_factor" => {
                        let v: f64 = value_str.parse().unwrap();
                        PARAMS.set_pause_decay_factor(v)
                    }
                    "pause_decay_floor" => {
                        let v: f64 = value_str.parse().unwrap();
                        PARAMS.set_pause_decay_floor(v)
                    }
                    "audio_buffer_size" => {
                        let v: u32 = value_str.parse().unwrap();
                        PARAMS.set_audio_buffer_size(v)
                    }
                    "mixer_check_interval_ms" => {
                        let v: u64 = value_str.parse().unwrap();
                        PARAMS.set_mixer_check_interval_ms(v)
                    }
                    _ => Ok(()), // Unknown parameter, skip
                };
            }
        }

        info!("GlobalParams initialized from database");
        Ok(())
    }
}

/// Helper function to load string parameter from database (used by metadata validators)
///
/// **[PLAN019-REQ-DRY-040]** Generic string loader for metadata-based validation.
/// Replaces type-specific loaders (load_f32_param, load_u32_param, etc.) which
/// are no longer needed with metadata-based validation.
async fn load_string_param(
    pool: &sqlx::SqlitePool,
    key: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match row {
        Some((Some(value_str),)) => Ok(Some(value_str)),
        Some((None,)) => Ok(None), // NULL value
        None => Ok(None),           // Missing row
    }
}
