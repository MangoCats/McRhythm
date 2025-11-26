//! # AcoustID Fingerprinting
//!
//! Chromaprint integration for audio fingerprinting and AcoustID API lookups.
//!
//! ## Features
//! - **Chromaprint FFI**: Direct integration with chromaprint-sys-next library
//! - **AcoustID API**: Lookup recordings by audio fingerprint
//! - **Rate limiting**: 550ms between API requests (AcoustID requirement)
//!
//! ## Optional Feature
//! AcoustID verification is optional (Run 24 feature). Used to validate
//! MusicBrainz matches against actual audio fingerprints.
//!
//! ## Configuration
//! Requires AcoustID API key:
//! - TOML: ~/.config/wkmp/wkmp-ai.toml (acoustid_api_key = "your-key")
//! - Environment: WKMP_ACOUSTID_API_KEY=your-key

use chromaprint_sys_next as chromaprint_ffi;
use std::path::PathBuf;

// NOTE: This module provides fingerprinting infrastructure but is not used
// in the core matching algorithm (Stages 2-5). It was added for optional
// AcoustID verification in Run 24.

/// Generate audio fingerprint using Chromaprint library
///
/// Uses chromaprint-sys-next FFI bindings to generate compressed fingerprints
/// compatible with AcoustID API.
///
/// # Arguments
/// * `samples` - Mono f32 audio samples (-1.0 to 1.0)
/// * `sample_rate` - Sample rate in Hz (e.g., 44100)
///
/// # Returns
/// Tuple of (fingerprint_base64, duration_seconds) or error message
///
/// # Algorithm
/// Uses CHROMAPRINT_ALGORITHM_TEST2 (same as fpcalc default)
pub(crate) fn generate_chromaprint_fingerprint(
    samples: &[f32],
    sample_rate: u32,
) -> Result<(String, u64), String> {
    // Convert f32 samples to i16 (Chromaprint expects 16-bit PCM)
    let samples_i16: Vec<i16> = samples
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect();

    unsafe {
        // Create chromaprint context with ALGORITHM_TEST2 (default, same as fpcalc)
        // TEST1=0, TEST2=1, TEST3=2, TEST4=3, TEST5=4; DEFAULT=TEST2
        let ctx = chromaprint_ffi::chromaprint_new(1); // 1 = CHROMAPRINT_ALGORITHM_TEST2
        if ctx.is_null() {
            return Err("chromaprint-sys-next: failed to create context".to_string());
        }

        // Start fingerprinting (mono, 1 channel)
        let start_result = chromaprint_ffi::chromaprint_start(ctx, sample_rate as i32, 1);
        if start_result != 1 {
            chromaprint_ffi::chromaprint_free(ctx);
            return Err("chromaprint-sys-next: start failed".to_string());
        }

        // Feed samples
        let feed_result = chromaprint_ffi::chromaprint_feed(
            ctx,
            samples_i16.as_ptr(),
            samples_i16.len() as i32,
        );
        if feed_result != 1 {
            chromaprint_ffi::chromaprint_free(ctx);
            return Err("chromaprint-sys-next: feed failed".to_string());
        }

        // Finish fingerprinting
        let finish_result = chromaprint_ffi::chromaprint_finish(ctx);
        if finish_result != 1 {
            chromaprint_ffi::chromaprint_free(ctx);
            return Err("chromaprint-sys-next: finish failed".to_string());
        }

        // Get compressed fingerprint (base64, compatible with AcoustID API)
        let mut fingerprint_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let get_result =
            chromaprint_ffi::chromaprint_get_fingerprint(ctx, &mut fingerprint_ptr);
        if get_result != 1 || fingerprint_ptr.is_null() {
            chromaprint_ffi::chromaprint_free(ctx);
            return Err("chromaprint-sys-next: get_fingerprint failed".to_string());
        }

        // Convert C string to Rust string
        let fingerprint = std::ffi::CStr::from_ptr(fingerprint_ptr)
            .to_string_lossy()
            .into_owned();

        // Free the fingerprint memory and context
        chromaprint_ffi::chromaprint_dealloc(fingerprint_ptr as *mut std::ffi::c_void);
        chromaprint_ffi::chromaprint_free(ctx);

        let duration = samples.len() as u64 / sample_rate as u64;
        Ok((fingerprint, duration))
    }
}

/// Resolve AcoustID API key from TOML config or environment variable
///
/// Priority: TOML config (~/.config/wkmp/wkmp-ai.toml) → Environment variable
///
/// # Returns
/// API key string or error with configuration instructions
pub(crate) fn resolve_acoustid_api_key() -> Result<String, String> {
    // Try TOML config first
    // On Windows: %USERPROFILE%\.config\wkmp\wkmp-ai.toml
    // On Unix: ~/.config/wkmp/wkmp-ai.toml
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok();

    if let Some(home_path) = home {
        let toml_path = PathBuf::from(home_path).join(".config/wkmp/wkmp-ai.toml");
        if toml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&toml_path) {
                if let Ok(config) = toml::from_str::<toml::Value>(&content) {
                    if let Some(key) = config
                        .get("acoustid_api_key")
                        .and_then(|v| v.as_str())
                    {
                        if !key.trim().is_empty() {
                            return Ok(key.to_string());
                        }
                    }
                }
            }
        }
    }

    // Fall back to environment variable
    match std::env::var("WKMP_ACOUSTID_API_KEY") {
        Ok(key) => Ok(key),
        Err(_) => Err(
            "AcoustID API key not configured. Please configure using one of:\n\
             1. TOML config: ~/.config/wkmp/wkmp-ai.toml (acoustid_api_key = \"your-key\")\n\
             2. Environment: WKMP_ACOUSTID_API_KEY=your-key-here\n\
             \n\
             Obtain API key at: https://acoustid.org/new-application"
                .to_string(),
        ),
    }
}

// NOTE: AcoustID API lookup functions (lookup_acoustid, etc.) are currently
// embedded in main.rs as they require async runtime and HTTP client integration.
// They could be extracted here in future refactoring if needed.
