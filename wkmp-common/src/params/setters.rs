//! Parameter setter methods with metadata-based validation
//!
//! **[PLAN019-REQ-DRY-050]** All setters delegate to metadata validators
//!
//! # RwLock Unwrap Justification
//!
//! All setters use `.write().unwrap()` on RwLock-protected fields.
//! This is JUSTIFIABLE because:
//! - RwLock poisoning only occurs if a thread panics while holding the lock
//! - Poisoned lock indicates corrupted process state
//! - Panic is the correct fail-fast behavior in this scenario
//! - Alternative (ignoring poisoning) would propagate corruption

use super::GlobalParams;

impl GlobalParams {
    /// Validate and update working_sample_rate
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [8000, 192000] Hz (see ParamMetadata)
    pub fn set_working_sample_rate(&self, value: u32) -> Result<(), String> {
        // Delegate to metadata validator
        let meta = Self::metadata().iter()
            .find(|m| m.key == "working_sample_rate")
            .expect("working_sample_rate metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.working_sample_rate.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update volume_level
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [0.0, 1.0] (see ParamMetadata)
    pub fn set_volume_level(&self, value: f32) -> Result<(), String> {
        // Delegate to metadata validator
        let meta = Self::metadata().iter()
            .find(|m| m.key == "volume_level")
            .expect("volume_level metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.volume_level.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update output_ringbuffer_size
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [2048, 262144] frames (see ParamMetadata)
    pub fn set_output_ringbuffer_size(&self, value: usize) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "output_ringbuffer_size")
            .expect("output_ringbuffer_size metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.output_ringbuffer_size.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update maximum_decode_streams
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [1, 32] (see ParamMetadata)
    pub fn set_maximum_decode_streams(&self, value: usize) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "maximum_decode_streams")
            .expect("maximum_decode_streams metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.maximum_decode_streams.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update decode_work_period
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [100, 60000] ms (see ParamMetadata)
    pub fn set_decode_work_period(&self, value: u64) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "decode_work_period")
            .expect("decode_work_period metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.decode_work_period.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update chunk_duration_ms
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [250, 5000] ms (see ParamMetadata)
    pub fn set_chunk_duration_ms(&self, value: u64) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "chunk_duration_ms")
            .expect("chunk_duration_ms metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.chunk_duration_ms.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update playout_ringbuffer_size
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [44100, 10000000] samples (see ParamMetadata)
    pub fn set_playout_ringbuffer_size(&self, value: usize) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "playout_ringbuffer_size")
            .expect("playout_ringbuffer_size metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.playout_ringbuffer_size.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update playout_ringbuffer_headroom
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [2205, 88200] samples (see ParamMetadata)
    pub fn set_playout_ringbuffer_headroom(&self, value: usize) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "playout_ringbuffer_headroom")
            .expect("playout_ringbuffer_headroom metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.playout_ringbuffer_headroom.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update decoder_resume_hysteresis_samples
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [2205, 441000] samples (see ParamMetadata)
    pub fn set_decoder_resume_hysteresis_samples(&self, value: u64) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "decoder_resume_hysteresis_samples")
            .expect("decoder_resume_hysteresis_samples metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.decoder_resume_hysteresis_samples.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update mixer_min_start_level
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [2205, 88200] samples (see ParamMetadata)
    pub fn set_mixer_min_start_level(&self, value: usize) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "mixer_min_start_level")
            .expect("mixer_min_start_level metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.mixer_min_start_level.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update pause_decay_factor
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [0.5, 0.99] (see ParamMetadata)
    pub fn set_pause_decay_factor(&self, value: f64) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "pause_decay_factor")
            .expect("pause_decay_factor metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.pause_decay_factor.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update pause_decay_floor
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [0.00001, 0.001] (see ParamMetadata)
    pub fn set_pause_decay_floor(&self, value: f64) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "pause_decay_floor")
            .expect("pause_decay_floor metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.pause_decay_floor.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update audio_buffer_size
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [512, 8192] frames (see ParamMetadata)
    pub fn set_audio_buffer_size(&self, value: u32) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "audio_buffer_size")
            .expect("audio_buffer_size metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.audio_buffer_size.write().unwrap() = value;
        Ok(())
    }

    /// Validate and update mixer_check_interval_ms
    ///
    /// **[PLAN019-REQ-DRY-050]** Refactored to use metadata validator.
    ///
    /// # Validation
    /// - Delegates to metadata validator for range checking
    /// - Must be in range [5, 100] ms (see ParamMetadata)
    pub fn set_mixer_check_interval_ms(&self, value: u64) -> Result<(), String> {
        let meta = Self::metadata().iter()
            .find(|m| m.key == "mixer_check_interval_ms")
            .expect("mixer_check_interval_ms metadata must exist");

        (meta.validator)(&value.to_string())?;

        *self.mixer_check_interval_ms.write().unwrap() = value;
        Ok(())
    }
}
