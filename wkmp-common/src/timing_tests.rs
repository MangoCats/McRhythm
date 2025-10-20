//! Unit tests for tick-based timing system
//!
//! Tests conversion functions and tick arithmetic for sample-accurate timing
//! across all supported sample rates (8kHz to 192kHz).
//!
//! Requirement Traceability:
//! - [SRC-TICK-020]: TICK_RATE constant validation
//! - [SRC-TICK-040]: Tick rate divisibility by all sample rates
//! - [SRC-API-020]: Millisecond to tick conversion
//! - [SRC-API-030]: Tick to millisecond conversion
//! - [SRC-WSR-030]: Tick to sample conversion
//! - [SRC-WSR-040]: Optimized 44.1kHz conversion
//! - [SRC-CONV-030]: Sample to tick conversion
//! - [SRC-PREC-020]: i64 overflow protection
//! - [SRC-EXAM-020]: Crossfade example validation

use super::*;

// ============================================================================
// Test Group 1: Tick Rate Constants
// ============================================================================

#[test]
fn test_tick_rate_constant_value() {
    // [SRC-TICK-020]
    assert_eq!(TICK_RATE, 28_224_000_i64);
}

#[test]
fn test_tick_rate_divides_all_sample_rates() {
    // [SRC-TICK-040]
    const SUPPORTED_RATES: [u32; 11] = [
        8000, 11025, 16000, 22050, 32000, 44100,
        48000, 88200, 96000, 176400, 192000,
    ];

    for rate in SUPPORTED_RATES {
        let ticks_per_sample = TICK_RATE / rate as i64;
        let remainder = TICK_RATE % rate as i64;

        assert_eq!(
            remainder, 0,
            "TICK_RATE {} must divide evenly into sample rate {}",
            TICK_RATE, rate
        );
        assert!(
            ticks_per_sample > 0,
            "ticks_per_sample must be positive for rate {}",
            rate
        );
    }
}

// ============================================================================
// Test Group 2: Millisecond ↔ Tick Conversions
// ============================================================================

#[test]
fn test_ms_to_ticks_accuracy() {
    // [SRC-API-020]
    assert_eq!(ms_to_ticks(0), 0);
    assert_eq!(ms_to_ticks(1), 28_224);
    assert_eq!(ms_to_ticks(1000), 28_224_000);
    assert_eq!(ms_to_ticks(60000), 1_693_440_000);

    // 5 minutes = 300 seconds = 300,000 ms
    assert_eq!(ms_to_ticks(300_000), 8_467_200_000);
}

#[test]
fn test_ticks_to_ms_roundtrip() {
    // [SRC-API-030]
    let test_cases = vec![
        0_i64,
        28_224,           // 1 ms
        28_224_000,       // 1 second
        141_120_000,      // 5 seconds
        1_693_440_000,    // 60 seconds
    ];

    for original_ticks in test_cases {
        let ms = ticks_to_ms(original_ticks);
        let roundtrip_ticks = ms_to_ticks(ms);

        assert_eq!(
            roundtrip_ticks, original_ticks,
            "Roundtrip failed for {} ticks",
            original_ticks
        );
    }
}

#[test]
fn test_ticks_to_ms_rounding_behavior() {
    // [SRC-API-030] - Rounding behavior
    // 28,224 ticks = 1 ms exactly
    assert_eq!(ticks_to_ms(28_224), 1);

    // 28,223 ticks < 1 ms, should round down to 0
    assert_eq!(ticks_to_ms(28_223), 0);

    // 28,225 ticks > 1 ms, should round down to 1
    assert_eq!(ticks_to_ms(28_225), 1);

    // 56,447 ticks = 1.999... ms, should round down to 1
    assert_eq!(ticks_to_ms(56_447), 1);

    // 56,448 ticks = 2 ms exactly
    assert_eq!(ticks_to_ms(56_448), 2);
}

// ============================================================================
// Test Group 3: Tick ↔ Sample Conversions
// ============================================================================

#[test]
fn test_ticks_to_samples_accuracy_44100() {
    // [SRC-WSR-030, SRC-WSR-040]
    const RATE_44100: u32 = 44100;

    // 0 ticks = 0 samples
    assert_eq!(ticks_to_samples(0, RATE_44100), 0);

    // 640 ticks = 1 sample @ 44.1kHz
    assert_eq!(ticks_to_samples(640, RATE_44100), 1);

    // 28,224,000 ticks = 1 second = 44,100 samples @ 44.1kHz
    assert_eq!(ticks_to_samples(28_224_000, RATE_44100), 44_100);

    // 141,120,000 ticks = 5 seconds = 220,500 samples @ 44.1kHz
    assert_eq!(ticks_to_samples(141_120_000, RATE_44100), 220_500);
}

#[test]
fn test_ticks_to_samples_accuracy_48000() {
    // [SRC-WSR-030]
    const RATE_48000: u32 = 48000;

    // 588 ticks = 1 sample @ 48kHz (28,224,000 / 48,000 = 588)
    assert_eq!(ticks_to_samples(588, RATE_48000), 1);

    // 28,224,000 ticks = 1 second = 48,000 samples @ 48kHz
    assert_eq!(ticks_to_samples(28_224_000, RATE_48000), 48_000);

    // 141,120,000 ticks = 5 seconds = 240,000 samples @ 48kHz
    assert_eq!(ticks_to_samples(141_120_000, RATE_48000), 240_000);
}

#[test]
fn test_ticks_to_samples_all_supported_rates() {
    // [SRC-WSR-030]
    const ONE_SECOND_TICKS: i64 = 28_224_000;

    let test_cases = vec![
        (8000, 8000),
        (11025, 11025),
        (16000, 16000),
        (22050, 22050),
        (32000, 32000),
        (44100, 44100),
        (48000, 48000),
        (88200, 88200),
        (96000, 96000),
        (176400, 176400),
        (192000, 192000),
    ];

    for (sample_rate, expected_samples) in test_cases {
        let samples = ticks_to_samples(ONE_SECOND_TICKS, sample_rate);
        assert_eq!(
            samples, expected_samples,
            "1 second @ {}Hz should equal {} samples",
            sample_rate, expected_samples
        );
    }
}

#[test]
fn test_samples_to_ticks_accuracy() {
    // [SRC-CONV-030]
    // 44.1kHz: 1 sample = 640 ticks
    assert_eq!(samples_to_ticks(1, 44100), 640);
    assert_eq!(samples_to_ticks(44100, 44100), 28_224_000);

    // 48kHz: 1 sample = 588 ticks
    assert_eq!(samples_to_ticks(1, 48000), 588);
    assert_eq!(samples_to_ticks(48000, 48000), 28_224_000);

    // 8kHz: 1 sample = 3,528 ticks
    assert_eq!(samples_to_ticks(1, 8000), 3_528);
    assert_eq!(samples_to_ticks(8000, 8000), 28_224_000);
}

#[test]
fn test_samples_to_ticks_roundtrip() {
    // [SRC-CONV-030]
    let test_cases = vec![
        (44100, 1),
        (44100, 100),
        (44100, 44100),    // 1 second
        (44100, 220500),   // 5 seconds
        (48000, 1),
        (48000, 48000),
        (48000, 240000),
        (8000, 8000),
        (192000, 192000),
    ];

    for (rate, original_samples) in test_cases {
        let ticks = samples_to_ticks(original_samples, rate);
        let roundtrip_samples = ticks_to_samples(ticks, rate);

        assert_eq!(
            roundtrip_samples, original_samples,
            "Roundtrip failed for {} samples @ {}Hz",
            original_samples, rate
        );
    }
}

// ============================================================================
// Test Group 4: Edge Cases and Overflow
// ============================================================================

#[test]
fn test_tick_overflow_detection() {
    // [SRC-PREC-020]
    const MAX_TICKS: i64 = i64::MAX;

    // Convert to seconds (should be ~326,791,809,696 seconds = ~10,355 years)
    let max_seconds = ticks_to_seconds(MAX_TICKS);
    assert!(
        max_seconds > 326_000_000_000.0 && max_seconds < 327_000_000_000.0,
        "Max representable time should be ~10,355 years (got {} seconds)",
        max_seconds
    );

    // Verify we can represent 10 years safely
    let ten_years_seconds = 10 * 365 * 24 * 60 * 60;
    let ten_years_ticks = seconds_to_ticks(ten_years_seconds as f64);
    assert!(
        ten_years_ticks < MAX_TICKS,
        "10 years should be representable in i64 ticks"
    );
}

#[test]
fn test_negative_tick_handling() {
    // [SRC-PREC-010]
    // Negative ticks should convert to negative milliseconds
    assert_eq!(ticks_to_ms(-28_224), -1);
    assert_eq!(ticks_to_ms(-28_224_000), -1000);

    // Negative milliseconds should convert to negative ticks
    assert_eq!(ms_to_ticks(-1), -28_224);
    assert_eq!(ms_to_ticks(-1000), -28_224_000);

    // Tick arithmetic with negatives
    let start_ticks = ms_to_ticks(5000); // 5 seconds
    let duration_ticks = ms_to_ticks(3000); // 3 seconds
    let offset_ticks = -ms_to_ticks(1000); // -1 second

    assert_eq!(start_ticks + duration_ticks, ms_to_ticks(8000));
    assert_eq!(start_ticks + offset_ticks, ms_to_ticks(4000));
}

#[test]
#[should_panic(expected = "sample_rate must be > 0")]
fn test_zero_sample_rate_protection() {
    // [SRC-WSR-030]
    ticks_to_samples(28_224_000, 0);
}

// ============================================================================
// Test Group 5: Cross-Rate Conversion Examples
// ============================================================================

#[test]
fn test_crossfade_duration_example() {
    // [SRC-EXAM-020, SRC-EXAM-030]
    const CROSSFADE_TICKS: i64 = 84_672_000; // 3 seconds

    // 44.1kHz: 84,672,000 ticks = 132,300 samples
    let samples_44100 = ticks_to_samples(CROSSFADE_TICKS, 44100);
    assert_eq!(
        samples_44100, 132_300,
        "3 second crossfade @ 44.1kHz should be 132,300 samples"
    );

    // 48kHz: 84,672,000 ticks = 144,000 samples
    let samples_48000 = ticks_to_samples(CROSSFADE_TICKS, 48000);
    assert_eq!(
        samples_48000, 144_000,
        "3 second crossfade @ 48kHz should be 144,000 samples"
    );

    // Verify both convert back to same tick value
    assert_eq!(samples_to_ticks(samples_44100, 44100), CROSSFADE_TICKS);
    assert_eq!(samples_to_ticks(samples_48000, 48000), CROSSFADE_TICKS);
}

#[test]
fn test_five_second_passage_example() {
    // [SRC-CONV-040]
    const FIVE_SECONDS_MS: i64 = 5000;
    const FIVE_SECONDS_TICKS: i64 = 141_120_000;
    const FIVE_SECONDS_SAMPLES_44100: usize = 220_500;

    // ms → ticks
    assert_eq!(ms_to_ticks(FIVE_SECONDS_MS), FIVE_SECONDS_TICKS);

    // ticks → samples @ 44.1kHz
    assert_eq!(
        ticks_to_samples(FIVE_SECONDS_TICKS, 44100),
        FIVE_SECONDS_SAMPLES_44100
    );

    // samples @ 44.1kHz → ticks
    assert_eq!(
        samples_to_ticks(FIVE_SECONDS_SAMPLES_44100, 44100),
        FIVE_SECONDS_TICKS
    );

    // ticks → ms
    assert_eq!(ticks_to_ms(FIVE_SECONDS_TICKS), FIVE_SECONDS_MS);
}

// ============================================================================
// Test Group 6: Helper Functions
// ============================================================================

#[test]
fn test_ticks_to_seconds_conversion() {
    // [SRC-TIME-010, SRC-TIME-020]
    assert_eq!(ticks_to_seconds(0), 0.0);
    assert_eq!(ticks_to_seconds(28_224_000), 1.0);
    assert_eq!(ticks_to_seconds(141_120_000), 5.0);

    // Verify precision (3.5 seconds = 98,784,000 ticks)
    let three_point_five_seconds = ticks_to_seconds(98_784_000);
    assert!(
        (three_point_five_seconds - 3.5).abs() < 0.0001,
        "3.5 seconds should convert accurately (got {})",
        three_point_five_seconds
    );
}

#[test]
fn test_ticks_per_sample_lookup_table() {
    // [SRC-CONV-010, SRC-CONV-020]
    const EXPECTED: [(u32, i64); 11] = [
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

    for (rate, expected_ticks) in EXPECTED {
        let ticks = ticks_per_sample(rate);
        assert_eq!(
            ticks, expected_ticks,
            "Ticks per sample @ {}Hz should be {}",
            rate, expected_ticks
        );

        // Verify lookup table matches (if it exists)
        if let Some(&table_value) = TICKS_PER_SAMPLE_TABLE
            .iter()
            .find(|(r, _)| *r == rate)
            .map(|(_, t)| t)
        {
            assert_eq!(
                table_value, expected_ticks,
                "Lookup table entry for {}Hz should match",
                rate
            );
        }
    }
}
