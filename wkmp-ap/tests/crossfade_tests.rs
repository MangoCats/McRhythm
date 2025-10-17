//! Unit tests for crossfade mixing and fade curves
//!
//! Tests fade curve calculations and crossfade timing
//!
//! Implements requirements from crossfade.md - 6-point timing model

use wkmp_ap::playback::pipeline::single_stream::mixer::{
    calculate_fade_gain,
    CrossfadePoints,
};
use wkmp_ap::playback::pipeline::single_stream::buffer::FadeCurve;

const EPSILON: f32 = 1e-6;

#[test]
fn test_linear_fade_curve() {
    // Linear fade in: 0.0 -> 1.0
    assert!((calculate_fade_gain(FadeCurve::Linear, 0.0, true) - 0.0).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Linear, 0.25, true) - 0.25).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Linear, 0.5, true) - 0.5).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Linear, 0.75, true) - 0.75).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Linear, 1.0, true) - 1.0).abs() < EPSILON);

    // Linear fade out: 1.0 -> 0.0
    assert!((calculate_fade_gain(FadeCurve::Linear, 0.0, false) - 1.0).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Linear, 0.25, false) - 0.75).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Linear, 0.5, false) - 0.5).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Linear, 0.75, false) - 0.25).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Linear, 1.0, false) - 0.0).abs() < EPSILON);
}

#[test]
fn test_exponential_fade_curve() {
    // Exponential fade in: slow start, fast end
    assert!((calculate_fade_gain(FadeCurve::Exponential, 0.0, true) - 0.0).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Exponential, 0.5, true) - 0.25).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Exponential, 1.0, true) - 1.0).abs() < EPSILON);

    // Exponential fade out: fast start, slow end
    assert!((calculate_fade_gain(FadeCurve::Exponential, 0.0, false) - 1.0).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Exponential, 0.5, false) - 0.25).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Exponential, 1.0, false) - 0.0).abs() < EPSILON);

    // Verify curve shape (should be quadratic)
    let mid_fade_in = calculate_fade_gain(FadeCurve::Exponential, 0.5, true);
    assert!(mid_fade_in < 0.5); // Should be below linear at midpoint
}

#[test]
fn test_logarithmic_fade_curve() {
    // Logarithmic fade in: fast start, slow end
    assert!((calculate_fade_gain(FadeCurve::Logarithmic, 0.0, true) - 0.0).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Logarithmic, 0.5, true) - 0.75).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Logarithmic, 1.0, true) - 1.0).abs() < EPSILON);

    // Logarithmic fade out: slow start, fast end
    assert!((calculate_fade_gain(FadeCurve::Logarithmic, 0.0, false) - 1.0).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Logarithmic, 0.5, false) - 0.75).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::Logarithmic, 1.0, false) - 0.0).abs() < EPSILON);

    // Verify curve shape
    let mid_fade_in = calculate_fade_gain(FadeCurve::Logarithmic, 0.5, true);
    assert!(mid_fade_in > 0.5); // Should be above linear at midpoint
}

#[test]
fn test_scurve_fade() {
    // S-Curve (cosine): smooth at both ends
    assert!((calculate_fade_gain(FadeCurve::SCurve, 0.0, true) - 0.0).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::SCurve, 0.5, true) - 0.5).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::SCurve, 1.0, true) - 1.0).abs() < EPSILON);

    // S-Curve fade out
    assert!((calculate_fade_gain(FadeCurve::SCurve, 0.0, false) - 1.0).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::SCurve, 0.5, false) - 0.5).abs() < EPSILON);
    assert!((calculate_fade_gain(FadeCurve::SCurve, 1.0, false) - 0.0).abs() < EPSILON);

    // Verify smooth acceleration at edges
    let near_start = calculate_fade_gain(FadeCurve::SCurve, 0.1, true);
    let near_end = calculate_fade_gain(FadeCurve::SCurve, 0.9, true);

    // S-curve should have slower change rate at edges
    assert!(near_start < 0.1 * 1.5); // Less than 1.5x linear
    assert!(near_end > 0.9 * 0.85);   // More than 0.85x linear
}

#[test]
fn test_crossfade_points_calculation() {
    // Test crossfade point calculation with typical values
    let points = CrossfadePoints::calculate(
        0,          // start_a
        1000,       // fade_in_start_b (1 second)
        2000,       // lead_in_end_b (2 seconds)
        8000,       // lead_out_start_a (8 seconds)
        9000,       // fade_out_end_a (9 seconds)
        10000,      // end_b (10 seconds)
    );

    assert_eq!(points.start_a, 0);
    assert_eq!(points.fade_in_start, 1000);
    assert_eq!(points.lead_in_end, 2000);
    assert_eq!(points.lead_out_start, 8000);
    assert_eq!(points.fade_out_end, 9000);
    assert_eq!(points.end_b, 10000);

    // Verify overlap duration
    let overlap = points.fade_out_end - points.fade_in_start;
    assert_eq!(overlap, 8000); // 8 seconds of overlap

    // Verify fade durations
    let fade_in_duration = points.lead_in_end - points.fade_in_start;
    assert_eq!(fade_in_duration, 1000); // 1 second fade in

    let fade_out_duration = points.fade_out_end - points.lead_out_start;
    assert_eq!(fade_out_duration, 1000); // 1 second fade out
}

#[test]
fn test_crossfade_timing_validation() {
    // Test that crossfade points maintain proper ordering
    let points = CrossfadePoints::calculate(
        0,      // start_a
        5000,   // fade_in_start_b
        6000,   // lead_in_end_b
        9000,   // lead_out_start_a
        10000,  // fade_out_end_a
        15000,  // end_b
    );

    // All points should be in increasing order
    assert!(points.start_a <= points.fade_in_start);
    assert!(points.fade_in_start <= points.lead_in_end);
    assert!(points.lead_in_end <= points.lead_out_start);
    assert!(points.lead_out_start <= points.fade_out_end);
    assert!(points.fade_out_end <= points.end_b);
}

#[test]
fn test_gain_summation_during_crossfade() {
    // During crossfade, sum of gains should equal 1.0 for constant power
    let test_points = vec![0.0, 0.25, 0.5, 0.75, 1.0];

    for progress in test_points {
        // Linear crossfade - should sum to 1.0
        let fade_out = calculate_fade_gain(FadeCurve::Linear, progress, false);
        let fade_in = calculate_fade_gain(FadeCurve::Linear, progress, true);
        assert!((fade_out + fade_in - 1.0).abs() < EPSILON);

        // S-Curve crossfade - should also sum to 1.0
        let fade_out_s = calculate_fade_gain(FadeCurve::SCurve, progress, false);
        let fade_in_s = calculate_fade_gain(FadeCurve::SCurve, progress, true);
        assert!((fade_out_s + fade_in_s - 1.0).abs() < EPSILON);
    }
}

#[test]
fn test_edge_cases() {
    // Test extreme progress values
    for curve in [FadeCurve::Linear, FadeCurve::Exponential, FadeCurve::Logarithmic, FadeCurve::SCurve] {
        // Fade in
        let gain_negative = calculate_fade_gain(curve.clone(), -0.1, true);
        let gain_over_one = calculate_fade_gain(curve.clone(), 1.1, true);
        assert!(gain_negative >= 0.0 && gain_negative <= 1.0);
        assert!(gain_over_one >= 0.0 && gain_over_one <= 1.0);

        // Fade out
        let gain_negative_out = calculate_fade_gain(curve.clone(), -0.1, false);
        let gain_over_one_out = calculate_fade_gain(curve.clone(), 1.1, false);
        assert!(gain_negative_out >= 0.0 && gain_negative_out <= 1.0);
        assert!(gain_over_one_out >= 0.0 && gain_over_one_out <= 1.0);
    }
}

#[test]
fn test_crossfade_precision() {
    // Test sample-accurate crossfade timing
    // At 44.1kHz, 0.02ms = ~0.88 samples precision required

    let sample_rate = 44100;
    let precision_ms = 0.02;
    let precision_samples = (sample_rate as f64 * precision_ms / 1000.0) as u64;

    assert!(precision_samples < 1, "Precision requirement is sub-sample");

    // Verify we can represent timing accurately
    let fade_start_ms = 1000.0;
    let fade_start_samples = ((fade_start_ms / 1000.0) * sample_rate as f64) as u64;
    let reconstructed_ms = (fade_start_samples as f64 / sample_rate as f64) * 1000.0;

    let error_ms = (fade_start_ms - reconstructed_ms).abs();
    assert!(error_ms < precision_ms, "Timing error {} exceeds precision requirement {}", error_ms, precision_ms);
}