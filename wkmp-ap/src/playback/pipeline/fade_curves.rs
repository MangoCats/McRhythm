//! Fade curve implementations for crossfading
//!
//! Provides five fade curve types with precise mathematical formulas
//! for sample-accurate crossfade mixing.
//!
//! **Traceability:**
//! - [XFD-IMPL-090] Fade curve formulas
//! - [XFD-IMPL-091] Linear fade-in
//! - [XFD-IMPL-092] Exponential fade-in
//! - [XFD-IMPL-093] Cosine fade-in (S-Curve)
//! - [XFD-IMPL-094] Linear fade-out
//! - [XFD-IMPL-095] Logarithmic fade-out
//! - [XFD-IMPL-096] Cosine fade-out (S-Curve)

use std::f32::consts::FRAC_PI_2;

/// Fade curve types for crossfading
///
/// Each curve type provides a different perceptual quality:
/// - Linear: Constant rate of change (precise, predictable)
/// - Exponential: Slow start, fast finish (natural-sounding fade-in)
/// - Logarithmic: Fast start, slow finish (natural-sounding fade-out)
/// - SCurve: Smooth acceleration and deceleration (gentle, musical)
/// - EqualPower: Constant perceived loudness during crossfade
///
/// **[XFD-CURV-020]** Fade-in curves increase volume from 0.0 to 1.0
/// **[XFD-CURV-030]** Fade-out curves decrease volume from 1.0 to 0.0
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeCurve {
    /// **[XFD-IMPL-091]** Linear: v(t) = t
    /// Constant rate of change, precise and predictable
    Linear,

    /// **[XFD-IMPL-092]** Exponential: v(t) = t²
    /// Slow start, fast finish - natural-sounding fade-in
    Exponential,

    /// **[XFD-IMPL-095]** Logarithmic: v(t) = (1-t)² (for fade-out)
    /// Fast start, slow finish - natural-sounding fade-out
    Logarithmic,

    /// **[XFD-IMPL-093/096]** S-Curve: v(t) = 0.5 × (1 - cos(π × t))
    /// Smooth acceleration and deceleration - gentle, musical
    SCurve,

    /// **[XFD-CURVE-050]** Equal-Power: v(t) = sin(t × π/2)
    /// Maintains constant perceived loudness during crossfade
    EqualPower,
}

impl FadeCurve {
    /// Calculate fade-in multiplier at given position
    ///
    /// **[XFD-IMPL-100]** Normalized time calculation:
    /// - position: 0.0 (start of fade) to 1.0 (end of fade)
    /// - Returns: volume multiplier (0.0 to 1.0)
    ///
    /// # Arguments
    /// * `position` - Normalized position through fade (0.0 to 1.0)
    ///
    /// # Returns
    /// Volume multiplier to apply to sample (0.0 = silence, 1.0 = full volume)
    pub fn calculate_fade_in(&self, position: f32) -> f32 {
        let t = position.clamp(0.0, 1.0);

        match self {
            FadeCurve::Linear => {
                // [XFD-IMPL-091] Linear: y = t
                t
            }
            FadeCurve::Exponential => {
                // [XFD-IMPL-092] Exponential: y = t²
                t * t
            }
            FadeCurve::Logarithmic => {
                // Logarithmic is for fade-out, but when used for fade-in:
                // Use sqrt to invert the quadratic curve
                t.sqrt()
            }
            FadeCurve::SCurve => {
                // [XFD-IMPL-093] S-Curve: y = 0.5 × (1 - cos(π × t))
                0.5 * (1.0 - (std::f32::consts::PI * t).cos())
            }
            FadeCurve::EqualPower => {
                // [XFD-CURVE-050] Equal-Power: y = sin(t × π/2)
                (t * FRAC_PI_2).sin()
            }
        }
    }

    /// Calculate fade-out multiplier at given position
    ///
    /// **[XFD-IMPL-100]** For fade-out, we invert the curve:
    /// - position: 0.0 (start of fade-out) to 1.0 (end of fade-out)
    /// - Returns: volume multiplier (1.0 at start, 0.0 at end)
    ///
    /// # Arguments
    /// * `position` - Normalized position through fade (0.0 to 1.0)
    ///
    /// # Returns
    /// Volume multiplier to apply to sample (1.0 = full volume, 0.0 = silence)
    pub fn calculate_fade_out(&self, position: f32) -> f32 {
        let t = position.clamp(0.0, 1.0);

        match self {
            FadeCurve::Linear => {
                // [XFD-IMPL-094] Linear fade-out: y = 1.0 - t
                1.0 - t
            }
            FadeCurve::Exponential => {
                // Exponential is for fade-in, but when used for fade-out:
                // Invert: (1-t)²
                let inv = 1.0 - t;
                inv * inv
            }
            FadeCurve::Logarithmic => {
                // [XFD-IMPL-095] Logarithmic fade-out: y = (1-t)²
                let inv = 1.0 - t;
                inv * inv
            }
            FadeCurve::SCurve => {
                // [XFD-IMPL-096] S-Curve fade-out: y = 0.5 × (1 + cos(π × t))
                0.5 * (1.0 + (std::f32::consts::PI * t).cos())
            }
            FadeCurve::EqualPower => {
                // Equal-Power fade-out: cos(t × π/2)
                (t * FRAC_PI_2).cos()
            }
        }
    }

    /// Get the recommended paired curve for crossfading
    ///
    /// Some curves work better in pairs for perceptually balanced crossfades.
    /// For example, exponential fade-in pairs well with logarithmic fade-out.
    pub fn recommended_pair(&self) -> FadeCurve {
        match self {
            FadeCurve::Exponential => FadeCurve::Logarithmic,
            FadeCurve::Logarithmic => FadeCurve::Exponential,
            FadeCurve::SCurve => FadeCurve::SCurve,
            FadeCurve::EqualPower => FadeCurve::EqualPower,
            FadeCurve::Linear => FadeCurve::Linear,
        }
    }

    /// Parse curve from string (from database)
    ///
    /// Supports database values from [XFD-DB-010]:
    /// - 'linear'
    /// - 'exponential'
    /// - 'logarithmic'
    /// - 'cosine' (maps to SCurve)
    /// - 'equal_power'
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "linear" => Some(FadeCurve::Linear),
            "exponential" => Some(FadeCurve::Exponential),
            "logarithmic" => Some(FadeCurve::Logarithmic),
            "cosine" | "scurve" | "s-curve" | "s_curve" => Some(FadeCurve::SCurve),
            "equal_power" | "equalpower" => Some(FadeCurve::EqualPower),
            _ => None,
        }
    }

    /// Convert to database string representation
    pub fn to_db_string(&self) -> &'static str {
        match self {
            FadeCurve::Linear => "linear",
            FadeCurve::Exponential => "exponential",
            FadeCurve::Logarithmic => "logarithmic",
            FadeCurve::SCurve => "cosine",
            FadeCurve::EqualPower => "equal_power",
        }
    }
}

impl Default for FadeCurve {
    /// Default fade curve is Exponential (natural-sounding)
    ///
    /// **[XFD-DEF-071]** Global default is exponential/logarithmic pair
    fn default() -> Self {
        FadeCurve::Exponential
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_fade_in() {
        let curve = FadeCurve::Linear;

        // At start: 0.0
        assert_eq!(curve.calculate_fade_in(0.0), 0.0);

        // At 25%: 0.25
        assert!((curve.calculate_fade_in(0.25) - 0.25).abs() < 0.001);

        // At 50%: 0.5
        assert!((curve.calculate_fade_in(0.5) - 0.5).abs() < 0.001);

        // At 75%: 0.75
        assert!((curve.calculate_fade_in(0.75) - 0.75).abs() < 0.001);

        // At end: 1.0
        assert_eq!(curve.calculate_fade_in(1.0), 1.0);
    }

    #[test]
    fn test_linear_fade_out() {
        let curve = FadeCurve::Linear;

        // At start: 1.0
        assert_eq!(curve.calculate_fade_out(0.0), 1.0);

        // At 25%: 0.75
        assert!((curve.calculate_fade_out(0.25) - 0.75).abs() < 0.001);

        // At 50%: 0.5
        assert!((curve.calculate_fade_out(0.5) - 0.5).abs() < 0.001);

        // At 75%: 0.25
        assert!((curve.calculate_fade_out(0.75) - 0.25).abs() < 0.001);

        // At end: 0.0
        assert_eq!(curve.calculate_fade_out(1.0), 0.0);
    }

    #[test]
    fn test_exponential_fade_in() {
        let curve = FadeCurve::Exponential;

        // At start: 0.0
        assert_eq!(curve.calculate_fade_in(0.0), 0.0);

        // At 50%: 0.25 (t² = 0.5² = 0.25)
        assert!((curve.calculate_fade_in(0.5) - 0.25).abs() < 0.001);

        // At end: 1.0
        assert_eq!(curve.calculate_fade_in(1.0), 1.0);

        // Slow start: value at 0.3 should be less than 0.3
        assert!(curve.calculate_fade_in(0.3) < 0.3);
    }

    #[test]
    fn test_logarithmic_fade_out() {
        let curve = FadeCurve::Logarithmic;

        // At start: 1.0
        assert_eq!(curve.calculate_fade_out(0.0), 1.0);

        // At 50%: 0.25 ((1-0.5)² = 0.5² = 0.25)
        assert!((curve.calculate_fade_out(0.5) - 0.25).abs() < 0.001);

        // At end: 0.0
        assert_eq!(curve.calculate_fade_out(1.0), 0.0);

        // Fast start: value at 0.3 should be less than 0.7
        assert!(curve.calculate_fade_out(0.3) < 0.7);
    }

    #[test]
    fn test_scurve_fade_in() {
        let curve = FadeCurve::SCurve;

        // At start: 0.0
        assert_eq!(curve.calculate_fade_in(0.0), 0.0);

        // At 50%: 0.5 (symmetric S-curve)
        assert!((curve.calculate_fade_in(0.5) - 0.5).abs() < 0.001);

        // At end: 1.0
        assert!((curve.calculate_fade_in(1.0) - 1.0).abs() < 0.001);

        // Smooth: starts slower than linear
        assert!(curve.calculate_fade_in(0.2) < 0.2);

        // Smooth: ends slower than linear
        assert!(curve.calculate_fade_in(0.8) > 0.8);
    }

    #[test]
    fn test_scurve_fade_out() {
        let curve = FadeCurve::SCurve;

        // At start: 1.0
        assert!((curve.calculate_fade_out(0.0) - 1.0).abs() < 0.001);

        // At 50%: 0.5 (symmetric S-curve)
        assert!((curve.calculate_fade_out(0.5) - 0.5).abs() < 0.001);

        // At end: 0.0
        assert_eq!(curve.calculate_fade_out(1.0), 0.0);
    }

    #[test]
    fn test_equal_power_fade_in() {
        let curve = FadeCurve::EqualPower;

        // At start: 0.0
        assert_eq!(curve.calculate_fade_in(0.0), 0.0);

        // At end: 1.0
        assert!((curve.calculate_fade_in(1.0) - 1.0).abs() < 0.001);

        // At 50%: ~0.707 (sin(π/4) ≈ 0.707)
        assert!((curve.calculate_fade_in(0.5) - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_equal_power_fade_out() {
        let curve = FadeCurve::EqualPower;

        // At start: 1.0
        assert!((curve.calculate_fade_out(0.0) - 1.0).abs() < 0.001);

        // At end: 0.0 (allow for floating point precision)
        assert!(curve.calculate_fade_out(1.0).abs() < 0.001);

        // At 50%: ~0.707 (cos(π/4) ≈ 0.707)
        assert!((curve.calculate_fade_out(0.5) - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_fade_in_out_symmetry() {
        // For symmetric curves, fade_in(t) + fade_out(t) should equal 1.0 for equal power
        let curve = FadeCurve::EqualPower;

        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let fade_in = curve.calculate_fade_in(t);
            let fade_out = curve.calculate_fade_out(t);
            let sum_of_squares = fade_in * fade_in + fade_out * fade_out;

            // Equal-power curves maintain constant power: sin²(t) + cos²(t) = 1
            assert!((sum_of_squares - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_clamping() {
        let curve = FadeCurve::Linear;

        // Values outside [0.0, 1.0] should be clamped
        assert_eq!(curve.calculate_fade_in(-0.5), 0.0);
        assert_eq!(curve.calculate_fade_in(1.5), 1.0);
        assert_eq!(curve.calculate_fade_out(-0.5), 1.0);
        assert_eq!(curve.calculate_fade_out(1.5), 0.0);
    }

    #[test]
    fn test_recommended_pairs() {
        assert_eq!(
            FadeCurve::Exponential.recommended_pair(),
            FadeCurve::Logarithmic
        );
        assert_eq!(
            FadeCurve::Logarithmic.recommended_pair(),
            FadeCurve::Exponential
        );
        assert_eq!(FadeCurve::SCurve.recommended_pair(), FadeCurve::SCurve);
        assert_eq!(
            FadeCurve::EqualPower.recommended_pair(),
            FadeCurve::EqualPower
        );
        assert_eq!(FadeCurve::Linear.recommended_pair(), FadeCurve::Linear);
    }

    #[test]
    fn test_from_str() {
        assert_eq!(FadeCurve::from_str("linear"), Some(FadeCurve::Linear));
        assert_eq!(
            FadeCurve::from_str("exponential"),
            Some(FadeCurve::Exponential)
        );
        assert_eq!(
            FadeCurve::from_str("logarithmic"),
            Some(FadeCurve::Logarithmic)
        );
        assert_eq!(FadeCurve::from_str("cosine"), Some(FadeCurve::SCurve));
        assert_eq!(FadeCurve::from_str("s-curve"), Some(FadeCurve::SCurve));
        assert_eq!(
            FadeCurve::from_str("equal_power"),
            Some(FadeCurve::EqualPower)
        );
        assert_eq!(FadeCurve::from_str("invalid"), None);
    }

    #[test]
    fn test_to_db_string() {
        assert_eq!(FadeCurve::Linear.to_db_string(), "linear");
        assert_eq!(FadeCurve::Exponential.to_db_string(), "exponential");
        assert_eq!(FadeCurve::Logarithmic.to_db_string(), "logarithmic");
        assert_eq!(FadeCurve::SCurve.to_db_string(), "cosine");
        assert_eq!(FadeCurve::EqualPower.to_db_string(), "equal_power");
    }

    #[test]
    fn test_default() {
        assert_eq!(FadeCurve::default(), FadeCurve::Exponential);
    }
}
