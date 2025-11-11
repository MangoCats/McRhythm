//! Chromaprint FFI wrapper for audio fingerprinting
//!
//! Safe Rust wrapper around libchromaprint C library.
//! Implements RAII pattern for automatic memory management.
//!
//! # Architecture
//! - FFI bindings to libchromaprint C API
//! - RAII wrapper with Drop trait for memory safety
//! - Audio format conversion (f32 → i16 PCM)
//! - Base64 fingerprint generation
//!
//! # Implementation
//! Per IMPL015-chromaprint_integration.md (PLAN024)
//!
//! # Safety
//! All unsafe FFI calls wrapped with error checking and proper resource cleanup.

use std::os::raw::{c_char, c_int, c_void};
use thiserror::Error;

// ============================================================================
// FFI Bindings
// ============================================================================

mod ffi {
    use super::*;

    pub type ChromaprintContextPtr = *mut c_void;

    pub const CHROMAPRINT_ALGORITHM_TEST2: c_int = 2;

    #[link(name = "chromaprint")]
    extern "C" {
        pub fn chromaprint_new(algorithm: c_int) -> ChromaprintContextPtr;
        pub fn chromaprint_free(ctx: ChromaprintContextPtr);

        pub fn chromaprint_start(
            ctx: ChromaprintContextPtr,
            sample_rate: c_int,
            num_channels: c_int,
        ) -> c_int;

        pub fn chromaprint_feed(
            ctx: ChromaprintContextPtr,
            data: *const i16,
            size: c_int,
        ) -> c_int;

        pub fn chromaprint_finish(ctx: ChromaprintContextPtr) -> c_int;

        pub fn chromaprint_get_fingerprint(
            ctx: ChromaprintContextPtr,
            fingerprint: *mut *mut c_char,
        ) -> c_int;

        pub fn chromaprint_dealloc(ptr: *mut c_void);

        pub fn chromaprint_get_version() -> *const c_char;
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during Chromaprint audio fingerprinting
///
/// Wraps errors from the Chromaprint C library FFI calls.
#[derive(Debug, Error)]
pub enum ChromaprintError {
    /// Failed to create Chromaprint context from FFI
    #[error("Failed to create Chromaprint context")]
    ContextCreationFailed,

    /// Sample rate outside valid range (8000-192000 Hz)
    #[error("Invalid sample rate: {0} Hz (must be 8000-192000 Hz)")]
    InvalidSampleRate(u32),

    /// Channel count not supported (must be 1 or 2)
    #[error("Invalid channel count: {0} (must be 1 or 2)")]
    InvalidChannelCount(u8),

    /// Failed to start fingerprinting session
    #[error("Failed to start fingerprinting")]
    StartFailed,

    /// Failed to feed audio data to fingerprinter
    #[error("Failed to feed audio data")]
    FeedFailed,

    /// Failed to finalize fingerprinting
    #[error("Failed to finish fingerprinting")]
    FinishFailed,

    /// Failed to generate fingerprint string
    #[error("Failed to generate fingerprint")]
    FingerprintGenerationFailed,

    /// FFI function returned null pointer
    #[error("FFI returned null pointer")]
    NullPointerReturned,
}

/// Result type for Chromaprint operations
pub type Result<T> = std::result::Result<T, ChromaprintError>;

// ============================================================================
// RAII Wrapper
// ============================================================================

/// Safe wrapper around Chromaprint context
///
/// Ensures automatic cleanup via Drop trait, preventing memory leaks.
/// Not Send/Sync - FFI context not thread-safe.
pub struct ChromaprintContext {
    ctx: ffi::ChromaprintContextPtr,
}

impl ChromaprintContext {
    /// Create new Chromaprint context with TEST2 algorithm
    ///
    /// # Errors
    /// Returns `ChromaprintError::ContextCreationFailed` if libchromaprint allocation fails
    ///
    /// # Example
    /// ```no_run
    /// use wkmp_ai::ffi::chromaprint::ChromaprintContext;
    ///
    /// let ctx = ChromaprintContext::new()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new() -> Result<Self> {
        let ctx = unsafe { ffi::chromaprint_new(ffi::CHROMAPRINT_ALGORITHM_TEST2) };

        if ctx.is_null() {
            return Err(ChromaprintError::ContextCreationFailed);
        }

        Ok(Self { ctx })
    }

    /// Generate acoustic fingerprint from audio samples
    ///
    /// # Arguments
    /// * `samples` - Audio samples in f32 format [-1.0, 1.0] (symphonia output)
    /// * `sample_rate` - Sample rate in Hz (e.g., 44100)
    /// * `num_channels` - 1 (mono) or 2 (stereo)
    ///
    /// # Returns
    /// Base64-encoded fingerprint string (suitable for AcoustID API)
    ///
    /// # Errors
    /// Returns `ChromaprintError` if:
    /// - Sample rate invalid (< 8000 or > 192000)
    /// - Channel count invalid (not 1 or 2)
    /// - FFI operation fails
    /// - Insufficient audio data (< 1 second recommended)
    ///
    /// # Example
    /// ```no_run
    /// use wkmp_ai::ffi::chromaprint::ChromaprintContext;
    ///
    /// let mut ctx = ChromaprintContext::new()?;
    /// let samples = vec![0.0f32; 44100]; // 1 second of silence
    /// let fingerprint = ctx.generate_fingerprint(&samples, 44100, 1)?;
    /// println!("Fingerprint: {}", fingerprint);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn generate_fingerprint(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        num_channels: u8,
    ) -> Result<String> {
        // 1. Validate parameters
        self.validate_parameters(sample_rate, num_channels)?;

        // 2. Start fingerprinting
        self.start(sample_rate, num_channels)?;

        // 3. Convert f32 → i16 PCM
        let pcm_samples = convert_f32_to_i16(samples);

        // 4. Feed audio data
        self.feed(&pcm_samples)?;

        // 5. Finish computation
        self.finish()?;

        // 6. Retrieve fingerprint
        self.get_fingerprint_raw()
    }

    // ------------------------------------------------------------------------
    // Private Helper Methods
    // ------------------------------------------------------------------------

    fn validate_parameters(&self, sample_rate: u32, num_channels: u8) -> Result<()> {
        // Chromaprint supports 8kHz - 192kHz
        if !(8000..=192000).contains(&sample_rate) {
            return Err(ChromaprintError::InvalidSampleRate(sample_rate));
        }

        // Only mono or stereo
        if !(1..=2).contains(&num_channels) {
            return Err(ChromaprintError::InvalidChannelCount(num_channels));
        }

        Ok(())
    }

    fn start(&mut self, sample_rate: u32, num_channels: u8) -> Result<()> {
        let result = unsafe {
            ffi::chromaprint_start(self.ctx, sample_rate as c_int, num_channels as c_int)
        };

        if result == 0 {
            Err(ChromaprintError::StartFailed)
        } else {
            Ok(())
        }
    }

    fn feed(&mut self, samples: &[i16]) -> Result<()> {
        let result =
            unsafe { ffi::chromaprint_feed(self.ctx, samples.as_ptr(), samples.len() as c_int) };

        if result == 0 {
            Err(ChromaprintError::FeedFailed)
        } else {
            Ok(())
        }
    }

    fn finish(&mut self) -> Result<()> {
        let result = unsafe { ffi::chromaprint_finish(self.ctx) };

        if result == 0 {
            Err(ChromaprintError::FinishFailed)
        } else {
            Ok(())
        }
    }

    fn get_fingerprint_raw(&self) -> Result<String> {
        let mut c_fingerprint: *mut c_char = std::ptr::null_mut();

        let result =
            unsafe { ffi::chromaprint_get_fingerprint(self.ctx, &mut c_fingerprint) };

        if result == 0 {
            return Err(ChromaprintError::FingerprintGenerationFailed);
        }

        if c_fingerprint.is_null() {
            return Err(ChromaprintError::NullPointerReturned);
        }

        // Copy to Rust String
        let fingerprint = unsafe {
            let c_str = std::ffi::CStr::from_ptr(c_fingerprint);
            c_str.to_string_lossy().into_owned()
        };

        // Deallocate C string (CRITICAL: prevents memory leak)
        unsafe {
            ffi::chromaprint_dealloc(c_fingerprint as *mut c_void);
        }

        Ok(fingerprint)
    }
}

impl Drop for ChromaprintContext {
    fn drop(&mut self) {
        unsafe {
            ffi::chromaprint_free(self.ctx);
        }
    }
}

// Note: !Send and !Sync are unstable features
// For now, we document that ChromaprintContext is not thread-safe
// and rely on proper usage patterns. In production, wrap in Arc<Mutex<>> if needed.

// ============================================================================
// Audio Format Conversion
// ============================================================================

/// Convert f32 samples [-1.0, 1.0] to i16 PCM [-32768, 32767]
///
/// # Arguments
/// * `samples` - f32 samples from symphonia decoder
///
/// # Returns
/// i16 PCM samples suitable for Chromaprint
///
/// # Performance
/// ~50-100 MB/s conversion rate (acceptable for non-critical path)
///
/// # Implementation
/// Per REQ-AI-012-01: Symphonia outputs f32 [-1.0, 1.0], Chromaprint requires i16
fn convert_f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&sample| {
            // Scale to i16 range and clamp
            let scaled = sample * 32767.0;
            scaled.clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get Chromaprint library version
///
/// # Returns
/// Version string (e.g., "1.5.1")
pub fn get_version() -> String {
    unsafe {
        let c_version = ffi::chromaprint_get_version();
        let c_str = std::ffi::CStr::from_ptr(c_version);
        c_str.to_string_lossy().into_owned()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = ChromaprintContext::new();
        assert!(ctx.is_ok(), "Failed to create Chromaprint context");
    }

    #[test]
    fn test_invalid_sample_rate_too_low() {
        let mut ctx = ChromaprintContext::new().unwrap();
        let samples = vec![0.0f32; 44100];

        let result = ctx.generate_fingerprint(&samples, 4000, 1);
        assert!(matches!(
            result,
            Err(ChromaprintError::InvalidSampleRate(4000))
        ));
    }

    #[test]
    fn test_invalid_sample_rate_too_high() {
        let mut ctx = ChromaprintContext::new().unwrap();
        let samples = vec![0.0f32; 44100];

        let result = ctx.generate_fingerprint(&samples, 384000, 1);
        assert!(matches!(
            result,
            Err(ChromaprintError::InvalidSampleRate(384000))
        ));
    }

    #[test]
    fn test_invalid_channel_count_zero() {
        let mut ctx = ChromaprintContext::new().unwrap();
        let samples = vec![0.0f32; 44100];

        let result = ctx.generate_fingerprint(&samples, 44100, 0);
        assert!(matches!(
            result,
            Err(ChromaprintError::InvalidChannelCount(0))
        ));
    }

    #[test]
    fn test_invalid_channel_count_three() {
        let mut ctx = ChromaprintContext::new().unwrap();
        let samples = vec![0.0f32; 44100];

        let result = ctx.generate_fingerprint(&samples, 44100, 3);
        assert!(matches!(
            result,
            Err(ChromaprintError::InvalidChannelCount(3))
        ));
    }

    #[test]
    fn test_audio_conversion_boundary_cases() {
        let test_cases = vec![
            (0.0f32, 0i16),         // Zero
            (1.0f32, 32767i16),     // Max positive
            (-1.0f32, -32767i16),   // Max negative
            (1.5f32, 32767i16),     // Clamp positive overflow
            (-1.5f32, -32768i16),   // Clamp negative overflow
            (0.5f32, 16383i16),     // Mid positive
            (-0.5f32, -16383i16),   // Mid negative
        ];

        for (input, expected) in test_cases {
            let result = convert_f32_to_i16(&[input]);
            assert_eq!(
                result[0], expected,
                "Failed for input {}: got {}, expected {}",
                input, result[0], expected
            );
        }
    }

    #[test]
    fn test_fingerprint_generation_sine_wave() {
        // Generate 1 second of 440 Hz sine wave
        let sample_rate = 44100;
        let samples = generate_sine_wave(440.0, 1.0, sample_rate);

        // Generate fingerprint
        let mut ctx = ChromaprintContext::new().unwrap();
        let fingerprint = ctx
            .generate_fingerprint(&samples, sample_rate, 1)
            .unwrap();

        // Verify fingerprint is base64-encoded string
        assert!(!fingerprint.is_empty(), "Fingerprint should not be empty");
        assert!(
            fingerprint
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='),
            "Fingerprint should be base64-encoded"
        );

        // Fingerprint should be deterministic
        let mut ctx2 = ChromaprintContext::new().unwrap();
        let fingerprint2 = ctx2
            .generate_fingerprint(&samples, sample_rate, 1)
            .unwrap();
        assert_eq!(
            fingerprint, fingerprint2,
            "Fingerprints should be deterministic"
        );
    }

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty(), "Version string should not be empty");
        // Version should be in format like "1.5.1"
        assert!(
            version.contains('.'),
            "Version should contain dot separator"
        );
    }

    // Helper: Generate sine wave for testing
    fn generate_sine_wave(frequency: f32, duration: f32, sample_rate: u32) -> Vec<f32> {
        let num_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * std::f32::consts::PI * frequency * t).sin();
            samples.push(sample);
        }

        samples
    }
}
