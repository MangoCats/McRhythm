// Consistency Validator - Title, Duration, Genre-Flavor Alignment Checks
//
// PLAN023: REQ-AI-061, REQ-AI-062, REQ-AI-063

use crate::fusion::{FusedFlavor, ValidationCheck};

/// Validate title consistency using Levenshtein similarity
///
/// # Arguments
/// * `title1` - First title
/// * `title2` - Second title
///
/// # Returns
/// * `ValidationCheck` with pass/fail and similarity score
pub fn validate_title_consistency(title1: &str, title2: &str) -> ValidationCheck {
    let similarity = strsim::normalized_levenshtein(title1, title2);

    ValidationCheck {
        name: "Title Consistency".to_string(),
        passed: similarity >= 0.8,
        score: Some(similarity),
        message: if similarity < 0.8 {
            Some(format!(
                "Titles differ significantly: '{}' vs '{}' (similarity: {:.2})",
                title1, title2, similarity
            ))
        } else {
            None
        },
    }
}

/// Validate duration consistency
///
/// # Arguments
/// * `duration1` - First duration (seconds)
/// * `duration2` - Second duration (seconds)
///
/// # Returns
/// * `ValidationCheck` with pass/fail
pub fn validate_duration_consistency(duration1: f64, duration2: f64) -> ValidationCheck {
    let diff = (duration1 - duration2).abs();
    let percent_diff = diff / duration1.max(duration2);

    ValidationCheck {
        name: "Duration Consistency".to_string(),
        passed: percent_diff <= 0.05, // 5% tolerance
        score: Some(1.0 - percent_diff),
        message: if percent_diff > 0.05 {
            Some(format!(
                "Duration differs by {:.1}% ({:.1}s vs {:.1}s)",
                percent_diff * 100.0,
                duration1,
                duration2
            ))
        } else {
            None
        },
    }
}

/// Validate genre-flavor alignment
///
/// # Arguments
/// * `genre` - ID3 genre string
/// * `flavor` - Fused musical flavor
///
/// # Returns
/// * `ValidationCheck` with alignment score
///
/// **[REQ-AI-063]** Validate genre-flavor alignment
///
/// **TODO (Non-Critical):** Full implementation pending
///
/// Future implementation should:
/// 1. Map genre to expected characteristics using `genre_mapping` module
/// 2. Compare expected vs actual flavor characteristics (vector distance)
/// 3. Calculate average alignment score across all characteristics
/// 4. Pass: avg_alignment > 0.7, Warning: 0.5-0.7, Fail: < 0.5
///
/// **Current behavior:** Always passes with a note (non-blocking for production)
pub fn validate_genre_flavor_alignment(
    _genre: &str,
    _flavor: &FusedFlavor,
) -> ValidationCheck {
    ValidationCheck {
        name: "Genre-Flavor Alignment".to_string(),
        passed: true,
        score: None,
        message: Some("Genre-flavor alignment check not yet implemented (non-critical)".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_consistency_identical() {
        let check = validate_title_consistency("Breathe", "Breathe");
        assert!(check.passed);
        assert_eq!(check.score, Some(1.0));
    }

    #[test]
    fn test_title_consistency_similar() {
        // Test with titles that are similar but not identical (e.g., different punctuation)
        let check = validate_title_consistency("Wish You Were Here", "Wish You Were Here!");
        assert!(check.passed, "Should pass for very similar titles");
        assert!(check.score.unwrap() > 0.8, "Should have high similarity");
    }

    #[test]
    fn test_title_consistency_different() {
        let check = validate_title_consistency("Breathe", "Time");
        assert!(!check.passed);
        assert!(check.message.is_some());
    }

    #[test]
    fn test_duration_consistency_pass() {
        let check = validate_duration_consistency(180.0, 182.0);
        assert!(check.passed, "2-second difference should pass");
    }

    #[test]
    fn test_duration_consistency_fail() {
        let check = validate_duration_consistency(180.0, 200.0);
        assert!(!check.passed, "20-second difference should fail");
        assert!(check.message.is_some());
    }
}
