//! Curve fitting and recommendation logic
//!
//! **Purpose:** Analyze interval→buffer relationship curve to find optimal balance.
//!
//! **Traceability:** TUNE-CURVE-010, TUNE-CURVE-020, TUNE-ALG-020

use crate::tuning::metrics::Verdict;
use serde::{Deserialize, Serialize};

/// Data point on the interval-buffer curve
///
/// Represents the minimum stable buffer size for a given mixer interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePoint {
    /// Mixer check interval in milliseconds
    pub interval_ms: u64,

    /// Minimum stable buffer size (frames), or None if interval is unstable
    pub min_stable_buffer: Option<u32>,

    /// Test verdict for this interval
    pub status: CurveStatus,
}

/// Status of a curve point
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurveStatus {
    /// Interval achieved stable performance
    Stable,
    /// Interval showed marginal performance (warnings)
    Marginal,
    /// Interval was unstable (too aggressive)
    Unstable,
}

impl From<Verdict> for CurveStatus {
    fn from(verdict: Verdict) -> Self {
        match verdict {
            Verdict::Stable => CurveStatus::Stable,
            Verdict::Warning => CurveStatus::Marginal,
            Verdict::Unstable => CurveStatus::Unstable,
        }
    }
}

/// Parameter recommendation with rationale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Recommended mixer check interval (milliseconds)
    pub mixer_check_interval_ms: u64,

    /// Recommended audio buffer size (frames)
    pub audio_buffer_size: u32,

    /// Expected audio latency in milliseconds (at 44.1kHz)
    pub expected_latency_ms: f64,

    /// Confidence level in this recommendation
    pub confidence: ConfidenceLevel,

    /// Human-readable rationale for this recommendation
    pub rationale: String,
}

/// Confidence level for recommendations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    /// Very high confidence (tested stable, good margin)
    VeryHigh,
    /// High confidence (tested stable)
    High,
    /// Medium confidence (based on extrapolation or limited testing)
    Medium,
    /// Low confidence (limited data, may need retesting)
    Low,
}

/// Complete recommendations package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendations {
    /// Primary recommendation (balanced latency and stability)
    pub primary: Recommendation,

    /// Conservative recommendation (maximum stability)
    pub conservative: Recommendation,
}

/// Analyze curve and generate recommendations
///
/// **Algorithm (TUNE-CURVE-020):**
/// 1. Filter to stable points only
/// 2. Find smallest buffer sizes
/// 3. Apply conservative scaling (2x buffer)
/// 4. Generate primary (optimal) and conservative recommendations
///
/// **Traceability:** TUNE-CURVE-010, TUNE-CURVE-020, TUNE-ALG-020
pub fn generate_recommendations(curve: &[CurvePoint]) -> Option<Recommendations> {
    // Filter to stable/marginal points
    let viable_points: Vec<_> = curve
        .iter()
        .filter(|p| {
            matches!(p.status, CurveStatus::Stable | CurveStatus::Marginal)
                && p.min_stable_buffer.is_some()
        })
        .collect();

    if viable_points.is_empty() {
        return None; // No stable configurations found
    }

    // Find the "sweet spot": lowest interval with reasonable buffer size
    // Target: 256-512 frame buffer (TUNE-CURVE-020)
    let primary = find_primary_recommendation(&viable_points)?;
    let conservative = find_conservative_recommendation(&viable_points, &primary)?;

    Some(Recommendations {
        primary,
        conservative,
    })
}

/// Find primary recommendation (balanced latency/stability)
///
/// **Strategy:**
/// - Target: 256-512 frame buffer (5.8-11.6ms @ 44.1kHz)
/// - Find smallest interval that achieves this target
/// - Apply 2x safety margin (conservative strategy per TUNE-Q-030)
fn find_primary_recommendation(viable_points: &[&CurvePoint]) -> Option<Recommendation> {
    // Sort by interval (ascending)
    let mut sorted = viable_points.to_vec();
    sorted.sort_by_key(|p| p.interval_ms);

    // Find smallest interval where buffer <= 512 (before safety margin)
    for point in sorted.iter() {
        if let Some(min_buffer) = point.min_stable_buffer {
            // Apply 2x conservative safety margin (TUNE-Q-030)
            let recommended_buffer = (min_buffer * 2).min(8192);

            // Check if this meets our target after safety margin
            if recommended_buffer <= 1024 {
                // Good candidate
                let latency_ms = calculate_latency(recommended_buffer, 44100);

                return Some(Recommendation {
                    mixer_check_interval_ms: point.interval_ms,
                    audio_buffer_size: recommended_buffer,
                    expected_latency_ms: latency_ms,
                    confidence: if point.status == CurveStatus::Stable {
                        ConfidenceLevel::High
                    } else {
                        ConfidenceLevel::Medium
                    },
                    rationale: format!(
                        "Lowest interval ({}ms) with reasonable buffer size ({}). 2x safety margin applied.",
                        point.interval_ms, recommended_buffer
                    ),
                });
            }
        }
    }

    // Fallback: Use smallest available buffer (with safety margin)
    let best = sorted
        .iter()
        .filter_map(|p| p.min_stable_buffer.map(|b| (p, b)))
        .min_by_key(|(_, buffer)| *buffer)?;

    let recommended_buffer = (best.1 * 2).min(8192);
    let latency_ms = calculate_latency(recommended_buffer, 44100);

    Some(Recommendation {
        mixer_check_interval_ms: best.0.interval_ms,
        audio_buffer_size: recommended_buffer,
        expected_latency_ms: latency_ms,
        confidence: ConfidenceLevel::Medium,
        rationale: format!(
            "No interval achieved target buffer size. Using smallest available ({}). 2x safety margin applied.",
            recommended_buffer
        ),
    })
}

/// Find conservative recommendation (maximum stability)
///
/// **Strategy:**
/// - Use longer interval than primary (more stability)
/// - Use larger buffer than primary (more headroom)
/// - Ensure conservative buffer >= primary buffer
fn find_conservative_recommendation(
    viable_points: &[&CurvePoint],
    primary: &Recommendation,
) -> Option<Recommendation> {
    // Sort by interval (ascending)
    let mut sorted = viable_points.to_vec();
    sorted.sort_by_key(|p| p.interval_ms);

    // Strategy 1: Find next interval above primary with safety margin
    let candidate1 = sorted
        .iter()
        .find(|p| p.interval_ms > primary.mixer_check_interval_ms)
        .and_then(|p| {
            let min_buffer = p.min_stable_buffer?;
            let recommended_buffer = (min_buffer * 3).min(8192);

            Some((p.interval_ms, recommended_buffer))
        });

    // Strategy 2: Use primary interval with larger safety margin
    let candidate2 = sorted
        .iter()
        .find(|p| p.interval_ms == primary.mixer_check_interval_ms)
        .and_then(|p| {
            let min_buffer = p.min_stable_buffer?;
            let recommended_buffer = ((primary.audio_buffer_size * 3) / 2).min(8192);

            Some((p.interval_ms, recommended_buffer))
        });

    // Choose the candidate with larger buffer, or strategy 1 if equal
    let (interval, buffer) = match (candidate1, candidate2) {
        (Some(c1), Some(c2)) => {
            if c1.1 >= c2.1 {
                c1
            } else {
                c2
            }
        }
        (Some(c1), None) => c1,
        (None, Some(c2)) => c2,
        (None, None) => {
            // Fallback: use largest buffer from any viable point
            let best = sorted.iter().max_by_key(|p| p.min_stable_buffer)?;
            let min_buffer = best.min_stable_buffer?;
            let recommended_buffer = (min_buffer * 3).min(8192);
            (best.interval_ms, recommended_buffer)
        }
    };

    // Ensure conservative buffer is at least as large as primary
    let buffer = buffer.max(primary.audio_buffer_size);
    let latency_ms = calculate_latency(buffer, 44100);

    Some(Recommendation {
        mixer_check_interval_ms: interval,
        audio_buffer_size: buffer,
        expected_latency_ms: latency_ms,
        confidence: ConfidenceLevel::VeryHigh,
        rationale: format!(
            "Extra safety margin for maximum stability. Buffer size: {}. Recommended for systems with variable load.",
            buffer
        ),
    })
}

/// Calculate audio latency in milliseconds
///
/// Latency = buffer_size / sample_rate * 1000
fn calculate_latency(buffer_size: u32, sample_rate: u32) -> f64 {
    (buffer_size as f64 / sample_rate as f64) * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_curve() -> Vec<CurvePoint> {
        vec![
            CurvePoint {
                interval_ms: 1,
                min_stable_buffer: None,
                status: CurveStatus::Unstable,
            },
            CurvePoint {
                interval_ms: 2,
                min_stable_buffer: None,
                status: CurveStatus::Unstable,
            },
            CurvePoint {
                interval_ms: 5,
                min_stable_buffer: Some(256),
                status: CurveStatus::Stable,
            },
            CurvePoint {
                interval_ms: 10,
                min_stable_buffer: Some(128),
                status: CurveStatus::Stable,
            },
            CurvePoint {
                interval_ms: 20,
                min_stable_buffer: Some(128),
                status: CurveStatus::Stable,
            },
            CurvePoint {
                interval_ms: 50,
                min_stable_buffer: Some(64),
                status: CurveStatus::Stable,
            },
        ]
    }

    #[test]
    fn test_generate_recommendations() {
        let curve = create_test_curve();
        let recs = generate_recommendations(&curve);

        assert!(recs.is_some());

        let recs = recs.unwrap();

        // Primary should favor lower latency
        assert!(recs.primary.mixer_check_interval_ms <= 10);

        // Conservative should have larger buffer
        assert!(recs.conservative.audio_buffer_size >= recs.primary.audio_buffer_size);

        // Conservative should have higher confidence
        assert_eq!(recs.conservative.confidence, ConfidenceLevel::VeryHigh);

        println!("Primary: {:?}", recs.primary);
        println!("Conservative: {:?}", recs.conservative);
    }

    #[test]
    fn test_no_stable_points() {
        let curve = vec![
            CurvePoint {
                interval_ms: 1,
                min_stable_buffer: None,
                status: CurveStatus::Unstable,
            },
            CurvePoint {
                interval_ms: 2,
                min_stable_buffer: None,
                status: CurveStatus::Unstable,
            },
        ];

        let recs = generate_recommendations(&curve);
        assert!(recs.is_none()); // Should return None when no stable points
    }

    #[test]
    fn test_calculate_latency() {
        // 512 frames @ 44.1kHz = ~11.6ms
        let latency = calculate_latency(512, 44100);
        assert!((latency - 11.6).abs() < 0.1);

        // 256 frames @ 44.1kHz = ~5.8ms
        let latency = calculate_latency(256, 44100);
        assert!((latency - 5.8).abs() < 0.1);
    }

    #[test]
    fn test_curve_status_from_verdict() {
        assert_eq!(
            CurveStatus::from(Verdict::Stable),
            CurveStatus::Stable
        );
        assert_eq!(
            CurveStatus::from(Verdict::Warning),
            CurveStatus::Marginal
        );
        assert_eq!(
            CurveStatus::from(Verdict::Unstable),
            CurveStatus::Unstable
        );
    }

    #[test]
    fn test_conservative_larger_than_primary() {
        let curve = create_test_curve();
        let recs = generate_recommendations(&curve).unwrap();

        // Conservative should have equal or larger buffer
        assert!(recs.conservative.audio_buffer_size >= recs.primary.audio_buffer_size);

        // Conservative should have equal or longer interval
        assert!(recs.conservative.mixer_check_interval_ms >= recs.primary.mixer_check_interval_ms);
    }
}
