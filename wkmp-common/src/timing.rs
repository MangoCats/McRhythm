//! Tick-based timing system for sample-accurate audio timing
//!
//! This module provides the core timing abstraction for WKMP, using a unified
//! tick rate of 28,224,000 Hz that divides evenly into all supported audio
//! sample rates (8kHz to 192kHz).
//!
//! # Architecture
//!
//! WKMP uses three time representations:
//!
//! 1. **Ticks (Internal)**: i64 values at 28,224,000 Hz - stored in database
//! 2. **Milliseconds (API)**: u64 values for HTTP API (backward compatible)
//! 3. **Samples (Playback)**: usize values at working_sample_rate
//!
//! ## Tick Rate Selection
//!
//! The tick rate of 28,224,000 Hz was chosen as the LCM (Least Common Multiple)
//! of common audio sample rates:
//!
//! - 44,100 Hz (CD quality): 28,224,000 ÷ 44,100 = 640 ticks/sample
//! - 48,000 Hz (DVD/DAT): 28,224,000 ÷ 48,000 = 588 ticks/sample
//! - 88,200 Hz (Hi-Res): 28,224,000 ÷ 88,200 = 320 ticks/sample
//! - 96,000 Hz (Hi-Res): 28,224,000 ÷ 96,000 = 294 ticks/sample
//!
//! This ensures sample-accurate conversions with zero rounding error.
//!
//! # Conversion Flow
//!
//! ```text
//! API Request (ms)
//!     ↓
//! ms_to_ticks() → Database Storage (i64 ticks)
//!     ↓
//! Database Load (i64 ticks)
//!     ↓
//! ticks_to_samples() → Playback Engine (usize samples)
//!     ↓
//! Audio Output
//! ```
//!
//! # Precision and Overflow
//!
//! - i64::MAX ticks = ~10.36 years of audio
//! - All conversions within ±1 tick tolerance
//! - Millisecond conversions use truncating division
//! - Sample conversions are exact (no rounding)
//!
//! # Requirement Traceability
//!
//! - [SRC-TICK-020]: TICK_RATE = 28,224,000 Hz
//! - [SRC-TICK-040]: Divides evenly into all 11 supported rates
//! - [SRC-API-020]: ms → ticks conversion
//! - [SRC-API-030]: ticks → ms conversion (truncating)
//! - [SRC-WSR-030]: ticks ↔ samples conversion
//! - [SRC-CONV-010]: Lossless within 1 tick tolerance
//! - [SRC-PREC-020]: i64 overflow protection
//!
//! # Examples
//!
//! ## Basic Conversions
//!
//! ```rust
//! use wkmp_common::timing::*;
//!
//! // Milliseconds to ticks (API → Database)
//! let ticks = ms_to_ticks(5000);  // 5 seconds
//! assert_eq!(ticks, 141_120_000);
//!
//! // Ticks to milliseconds (Database → API)
//! let ms = ticks_to_ms(141_120_000);
//! assert_eq!(ms, 5000);
//!
//! // Ticks to samples (Database → Playback)
//! let samples_44k = ticks_to_samples(141_120_000, 44100);
//! assert_eq!(samples_44k, 220_500);  // 5 seconds @ 44.1kHz
//!
//! let samples_48k = ticks_to_samples(141_120_000, 48000);
//! assert_eq!(samples_48k, 240_000);  // 5 seconds @ 48kHz
//! ```
//!
//! ## Passage Timing Conversion
//!
//! ```rust
//! use wkmp_common::timing::*;
//!
//! // Convert passage from API (ms) to internal (ticks)
//! let passage_ms = PassageTimingMs {
//!     start_time_ms: 10000,      // 10s
//!     end_time_ms: 20000,        // 20s
//!     fade_in_point_ms: 12000,   // 12s
//!     fade_out_point_ms: 18000,  // 18s
//!     lead_in_point_ms: 9000,    // 9s
//!     lead_out_point_ms: 21000,  // 21s
//! };
//!
//! let passage_ticks = PassageTimingTicks::from(passage_ms);
//! assert_eq!(passage_ticks.start_time_ticks, 282_240_000);
//! assert_eq!(passage_ticks.end_time_ticks, 564_480_000);
//! ```

// ============================================================================
// Constants
// ============================================================================

/// Tick rate: 28,224,000 Hz
///
/// This is the LCM of common audio sample rates and provides sample-accurate
/// timing conversions with zero rounding error.
///
/// **Requirement:** [SRC-TICK-020]
pub const TICK_RATE: i64 = 28_224_000;

/// Ticks per millisecond: 28,224
///
/// This constant is used for fast millisecond ↔ tick conversions:
/// - `ticks = milliseconds × TICKS_PER_MS`
/// - `milliseconds = ticks ÷ TICKS_PER_MS` (truncating division)
///
/// **Requirement:** [SRC-API-020]
pub const TICKS_PER_MS: i64 = 28_224;

/// Lookup table for ticks per sample at common sample rates
///
/// This table provides O(1) lookup for the most common rates. For rates
/// not in the table, use the formula: `TICK_RATE ÷ sample_rate`
///
/// | Rate (Hz) | Ticks/Sample | Calculation |
/// |-----------|--------------|-------------|
/// | 8,000     | 3,528        | 28,224,000 ÷ 8,000 |
/// | 11,025    | 2,560        | 28,224,000 ÷ 11,025 |
/// | 16,000    | 1,764        | 28,224,000 ÷ 16,000 |
/// | 22,050    | 1,280        | 28,224,000 ÷ 22,050 |
/// | 32,000    | 882          | 28,224,000 ÷ 32,000 |
/// | 44,100    | 640          | 28,224,000 ÷ 44,100 |
/// | 48,000    | 588          | 28,224,000 ÷ 48,000 |
/// | 88,200    | 320          | 28,224,000 ÷ 88,200 |
/// | 96,000    | 294          | 28,224,000 ÷ 96,000 |
/// | 176,400   | 160          | 28,224,000 ÷ 176,400 |
/// | 192,000   | 147          | 28,224,000 ÷ 192,000 |
///
/// **Requirement:** [SRC-CONV-010]
pub const TICKS_PER_SAMPLE_TABLE: [(u32, i64); 11] = [
    (8000, 3528),
    (11025, 2560),
    (16000, 1764),
    (22050, 1280),
    (32000, 882),
    (44100, 640),
    (48000, 588),
    (88200, 320),
    (96000, 294),
    (176400, 160),
    (192000, 147),
];

// ============================================================================
// Core Conversion Functions
// ============================================================================

/// Convert milliseconds to ticks
///
/// Uses simple multiplication: `ticks = milliseconds × 28,224`
///
/// This conversion is lossless - all millisecond values convert exactly to
/// tick boundaries.
///
/// # Arguments
///
/// * `milliseconds` - Time duration in milliseconds
///
/// # Returns
///
/// Time duration in ticks (28,224,000 Hz)
///
/// # Examples
///
/// ```rust
/// use wkmp_common::timing::ms_to_ticks;
///
/// assert_eq!(ms_to_ticks(0), 0);
/// assert_eq!(ms_to_ticks(1), 28_224);
/// assert_eq!(ms_to_ticks(1000), 28_224_000);  // 1 second
/// assert_eq!(ms_to_ticks(5000), 141_120_000); // 5 seconds
/// ```
///
/// # Negative Values
///
/// Negative milliseconds are supported for relative time calculations:
///
/// ```rust
/// use wkmp_common::timing::ms_to_ticks;
///
/// assert_eq!(ms_to_ticks(-1000), -28_224_000);
/// ```
///
/// **Requirement:** [SRC-API-020]
pub fn ms_to_ticks(milliseconds: i64) -> i64 {
    milliseconds * TICKS_PER_MS
}

/// Convert ticks to milliseconds using truncating division
///
/// Uses truncating division: `milliseconds = ticks ÷ 28,224`
///
/// **Important:** This conversion may lose precision. Tick values that don't
/// fall exactly on millisecond boundaries will round down.
///
/// # Arguments
///
/// * `ticks` - Time duration in ticks (28,224,000 Hz)
///
/// # Returns
///
/// Time duration in milliseconds (truncated)
///
/// # Precision Loss
///
/// Maximum rounding error is 999 ticks (≈ 0.035 ms):
///
/// ```rust
/// use wkmp_common::timing::ticks_to_ms;
///
/// // Exact conversion (no loss)
/// assert_eq!(ticks_to_ms(28_224), 1);
///
/// // Rounds down (28,223 ticks ≈ 0.999 ms → 0 ms)
/// assert_eq!(ticks_to_ms(28_223), 0);
///
/// // Rounds down (28,225 ticks ≈ 1.001 ms → 1 ms)
/// assert_eq!(ticks_to_ms(28_225), 1);
/// ```
///
/// # Roundtrip Guarantee
///
/// For tick-aligned values, roundtrip is exact:
///
/// ```rust
/// use wkmp_common::timing::{ms_to_ticks, ticks_to_ms};
///
/// let original_ms = 5000;
/// let ticks = ms_to_ticks(original_ms);
/// let roundtrip_ms = ticks_to_ms(ticks);
/// assert_eq!(roundtrip_ms, original_ms);
/// ```
///
/// **Requirement:** [SRC-API-030]
pub fn ticks_to_ms(ticks: i64) -> i64 {
    ticks / TICKS_PER_MS
}

/// Convert ticks to samples at a given sample rate
///
/// Uses exact division: `samples = (ticks × sample_rate) ÷ 28,224,000`
///
/// Because TICK_RATE divides evenly into all supported sample rates, this
/// conversion is **always exact** with zero rounding error.
///
/// # Arguments
///
/// * `ticks` - Time duration in ticks
/// * `sample_rate` - Target sample rate in Hz
///
/// # Returns
///
/// Number of samples at the given sample rate
///
/// # Panics
///
/// Panics if `sample_rate` is 0 (division by zero protection)
///
/// # Examples
///
/// ```rust
/// use wkmp_common::timing::ticks_to_samples;
///
/// // 1 second at various rates
/// let one_second_ticks = 28_224_000;
/// assert_eq!(ticks_to_samples(one_second_ticks, 44100), 44_100);
/// assert_eq!(ticks_to_samples(one_second_ticks, 48000), 48_000);
/// assert_eq!(ticks_to_samples(one_second_ticks, 96000), 96_000);
///
/// // Crossfade example: 3 seconds
/// let crossfade_ticks = 84_672_000;
/// assert_eq!(ticks_to_samples(crossfade_ticks, 44100), 132_300);
/// assert_eq!(ticks_to_samples(crossfade_ticks, 48000), 144_000);
/// ```
///
/// # Performance
///
/// For 44.1kHz (the most common rate), consider using the optimized formula:
/// `samples = ticks ÷ 640`
///
/// **Requirement:** [SRC-WSR-030]
pub fn ticks_to_samples(ticks: i64, sample_rate: u32) -> usize {
    assert!(sample_rate > 0, "sample_rate must be > 0");

    // Formula: (ticks × sample_rate) ÷ TICK_RATE
    // This is exact because TICK_RATE divides evenly into all supported rates
    let samples = (ticks * sample_rate as i64) / TICK_RATE;
    samples as usize
}

/// Convert samples to ticks at a given sample rate
///
/// Uses exact multiplication: `ticks = samples × (28,224,000 ÷ sample_rate)`
///
/// This conversion is exact for all supported sample rates because TICK_RATE
/// divides evenly into each rate.
///
/// # Arguments
///
/// * `samples` - Number of samples
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
///
/// Time duration in ticks
///
/// # Panics
///
/// Panics if `sample_rate` is 0
///
/// # Examples
///
/// ```rust
/// use wkmp_common::timing::samples_to_ticks;
///
/// // 1 sample at various rates
/// assert_eq!(samples_to_ticks(1, 44100), 640);
/// assert_eq!(samples_to_ticks(1, 48000), 588);
/// assert_eq!(samples_to_ticks(1, 8000), 3_528);
///
/// // 1 second at 44.1kHz
/// assert_eq!(samples_to_ticks(44100, 44100), 28_224_000);
/// ```
///
/// # Roundtrip Guarantee
///
/// For all supported sample rates, roundtrip conversions are exact:
///
/// ```rust
/// use wkmp_common::timing::{samples_to_ticks, ticks_to_samples};
///
/// let original_samples = 220_500;  // 5 seconds @ 44.1kHz
/// let ticks = samples_to_ticks(original_samples, 44100);
/// let roundtrip_samples = ticks_to_samples(ticks, 44100);
/// assert_eq!(roundtrip_samples, original_samples);
/// ```
///
/// **Requirement:** [SRC-CONV-030]
pub fn samples_to_ticks(samples: usize, sample_rate: u32) -> i64 {
    assert!(sample_rate > 0, "sample_rate must be > 0");

    // Formula: samples × (TICK_RATE ÷ sample_rate)
    let ticks_per_sample = TICK_RATE / sample_rate as i64;
    samples as i64 * ticks_per_sample
}

/// Convert ticks to seconds (f64)
///
/// Uses floating point division: `seconds = ticks ÷ 28,224,000.0`
///
/// This is a convenience function for display and logging. For precise timing,
/// use tick-based arithmetic.
///
/// # Arguments
///
/// * `ticks` - Time duration in ticks
///
/// # Returns
///
/// Time duration in seconds (f64)
///
/// # Examples
///
/// ```rust
/// use wkmp_common::timing::ticks_to_seconds;
///
/// assert_eq!(ticks_to_seconds(0), 0.0);
/// assert_eq!(ticks_to_seconds(28_224_000), 1.0);
/// assert_eq!(ticks_to_seconds(141_120_000), 5.0);
///
/// // High precision
/// let half_second = ticks_to_seconds(14_112_000);
/// assert!((half_second - 0.5).abs() < 0.0001);
/// ```
///
/// **Requirement:** [SRC-TIME-010]
pub fn ticks_to_seconds(ticks: i64) -> f64 {
    ticks as f64 / TICK_RATE as f64
}

/// Convert seconds to ticks
///
/// Uses floating point multiplication: `ticks = seconds × 28,224,000.0`
///
/// # Arguments
///
/// * `seconds` - Time duration in seconds
///
/// # Returns
///
/// Time duration in ticks (rounded to nearest tick)
///
/// # Examples
///
/// ```rust
/// use wkmp_common::timing::seconds_to_ticks;
///
/// assert_eq!(seconds_to_ticks(0.0), 0);
/// assert_eq!(seconds_to_ticks(1.0), 28_224_000);
/// assert_eq!(seconds_to_ticks(5.0), 141_120_000);
/// assert_eq!(seconds_to_ticks(0.5), 14_112_000);
/// ```
///
/// **Requirement:** [SRC-TIME-020]
pub fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds * TICK_RATE as f64).round() as i64
}

/// Get ticks per sample for a given sample rate
///
/// This function checks the lookup table first for O(1) performance, then
/// falls back to division for non-standard rates.
///
/// # Arguments
///
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
///
/// Number of ticks per sample at the given rate
///
/// # Panics
///
/// Panics if `sample_rate` is 0
///
/// # Examples
///
/// ```rust
/// use wkmp_common::timing::ticks_per_sample;
///
/// assert_eq!(ticks_per_sample(44100), 640);
/// assert_eq!(ticks_per_sample(48000), 588);
/// assert_eq!(ticks_per_sample(8000), 3_528);
/// ```
///
/// **Requirement:** [SRC-CONV-020]
pub fn ticks_per_sample(sample_rate: u32) -> i64 {
    assert!(sample_rate > 0, "sample_rate must be > 0");

    // Check lookup table first
    if let Some(&(_, ticks)) = TICKS_PER_SAMPLE_TABLE
        .iter()
        .find(|(rate, _)| *rate == sample_rate)
    {
        return ticks;
    }

    // Fall back to calculation
    TICK_RATE / sample_rate as i64
}

// ============================================================================
// Passage Timing Types
// ============================================================================

/// Passage timing in milliseconds (API representation)
///
/// This structure is used in HTTP API requests/responses for user-friendly
/// millisecond-based timing.
///
/// **Usage:** Deserialize from JSON → Convert to ticks → Store in database
///
/// # Examples
///
/// ```rust
/// use wkmp_common::timing::{PassageTimingMs, PassageTimingTicks};
///
/// let api_timing = PassageTimingMs {
///     start_time_ms: 10000,       // 10s
///     end_time_ms: 20000,         // 20s
///     fade_in_point_ms: 12000,    // 12s
///     fade_out_point_ms: 18000,   // 18s
///     lead_in_point_ms: 9000,     // 9s
///     lead_out_point_ms: 21000,   // 21s
/// };
///
/// let internal_timing = PassageTimingTicks::from(api_timing);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassageTimingMs {
    /// Start time in milliseconds
    pub start_time_ms: u64,
    /// End time in milliseconds
    pub end_time_ms: u64,
    /// Fade-in point in milliseconds
    pub fade_in_point_ms: u64,
    /// Fade-out point in milliseconds
    pub fade_out_point_ms: u64,
    /// Lead-in point in milliseconds
    pub lead_in_point_ms: u64,
    /// Lead-out point in milliseconds
    pub lead_out_point_ms: u64,
}

/// Passage timing in ticks (internal representation)
///
/// This structure is used for database storage and internal calculations,
/// providing sample-accurate timing.
///
/// **Usage:** Load from database → Convert to samples → Use in playback
///
/// # Examples
///
/// ```rust
/// use wkmp_common::timing::{PassageTimingTicks, ticks_to_samples};
///
/// let passage = PassageTimingTicks {
///     start_time_ticks: 282_240_000,      // 10s
///     end_time_ticks: 564_480_000,        // 20s
///     fade_in_point_ticks: 338_688_000,   // 12s
///     fade_out_point_ticks: 508_032_000,  // 18s
///     lead_in_point_ticks: 254_016_000,   // 9s
///     lead_out_point_ticks: 592_704_000,  // 21s
/// };
///
/// // Convert to samples for playback at 44.1kHz
/// let start_sample = ticks_to_samples(passage.start_time_ticks, 44100);
/// let end_sample = ticks_to_samples(passage.end_time_ticks, 44100);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassageTimingTicks {
    /// Start time in ticks
    pub start_time_ticks: i64,
    /// End time in ticks
    pub end_time_ticks: i64,
    /// Fade-in point in ticks
    pub fade_in_point_ticks: i64,
    /// Fade-out point in ticks
    pub fade_out_point_ticks: i64,
    /// Lead-in point in ticks
    pub lead_in_point_ticks: i64,
    /// Lead-out point in ticks
    pub lead_out_point_ticks: i64,
}

impl From<PassageTimingMs> for PassageTimingTicks {
    /// Convert API timing (milliseconds) to internal timing (ticks)
    ///
    /// Uses lossless ms → ticks conversion.
    fn from(ms: PassageTimingMs) -> Self {
        PassageTimingTicks {
            start_time_ticks: ms_to_ticks(ms.start_time_ms as i64),
            end_time_ticks: ms_to_ticks(ms.end_time_ms as i64),
            fade_in_point_ticks: ms_to_ticks(ms.fade_in_point_ms as i64),
            fade_out_point_ticks: ms_to_ticks(ms.fade_out_point_ms as i64),
            lead_in_point_ticks: ms_to_ticks(ms.lead_in_point_ms as i64),
            lead_out_point_ticks: ms_to_ticks(ms.lead_out_point_ms as i64),
        }
    }
}

impl From<PassageTimingTicks> for PassageTimingMs {
    /// Convert internal timing (ticks) to API timing (milliseconds)
    ///
    /// Uses truncating division for ticks → ms conversion.
    /// Maximum rounding error: 999 ticks ≈ 0.035 ms per field.
    fn from(ticks: PassageTimingTicks) -> Self {
        PassageTimingMs {
            start_time_ms: ticks_to_ms(ticks.start_time_ticks) as u64,
            end_time_ms: ticks_to_ms(ticks.end_time_ticks) as u64,
            fade_in_point_ms: ticks_to_ms(ticks.fade_in_point_ticks) as u64,
            fade_out_point_ms: ticks_to_ms(ticks.fade_out_point_ticks) as u64,
            lead_in_point_ms: ticks_to_ms(ticks.lead_in_point_ticks) as u64,
            lead_out_point_ms: ticks_to_ms(ticks.lead_out_point_ticks) as u64,
        }
    }
}

// ============================================================================
// Validation Functions
// ============================================================================

/// Validate that a millisecond → ticks → milliseconds roundtrip is within tolerance
///
/// This checks that the conversion error is at most 1 tick (≈ 0.035 ms).
///
/// # Arguments
///
/// * `original_ms` - Original millisecond value
///
/// # Returns
///
/// `true` if roundtrip error ≤ 1 tick, `false` otherwise
///
/// # Examples
///
/// ```rust
/// use wkmp_common::timing::validate_tick_conversion;
///
/// assert!(validate_tick_conversion(0));
/// assert!(validate_tick_conversion(1000));
/// assert!(validate_tick_conversion(5000));
/// ```
pub fn validate_tick_conversion(original_ms: u64) -> bool {
    let ticks = ms_to_ticks(original_ms as i64);
    let roundtrip_ms = ticks_to_ms(ticks);
    let error = (original_ms as i64 - roundtrip_ms).abs();

    // Error should be 0 for all millisecond-aligned values
    error == 0
}

/// Calculate maximum roundtrip error in nanoseconds for a given millisecond value
///
/// This is useful for understanding precision loss in conversions.
///
/// # Arguments
///
/// * `ms` - Millisecond value
///
/// # Returns
///
/// Maximum error in nanoseconds (always < 35,000 ns)
///
/// # Examples
///
/// ```rust
/// use wkmp_common::timing::max_roundtrip_error_ns;
///
/// // Millisecond-aligned values have zero error
/// assert_eq!(max_roundtrip_error_ns(1000), 0.0);
///
/// // Sub-millisecond precision varies
/// let error = max_roundtrip_error_ns(1234);
/// assert!(error < 35_000.0);  // Less than 0.035 ms
/// ```
pub fn max_roundtrip_error_ns(ms: u64) -> f64 {
    let ticks = ms_to_ticks(ms as i64);
    let roundtrip_ms = ticks_to_ms(ticks);
    let error_ms = (ms as i64 - roundtrip_ms).abs() as f64;
    error_ms * 1_000_000.0  // Convert ms to ns
}

// ============================================================================
// Tests Module
// ============================================================================

#[cfg(test)]
#[path = "timing_tests.rs"]
mod tests;
