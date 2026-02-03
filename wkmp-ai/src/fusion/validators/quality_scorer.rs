// Quality Scorer - Overall Quality Score Calculation
//
// PLAN023: REQ-AI-064 - Compute overall quality from validation checks

use crate::fusion::{ValidationCheck, ValidationStatus};

/// Calculate overall quality score from validation checks
///
/// # Arguments
/// * `checks` - Individual validation check results
///
/// # Returns
/// * (quality_score, status) - Score 0-100%, status based on thresholds
pub fn calculate_quality_score(checks: &[ValidationCheck]) -> (f64, ValidationStatus) {
    if checks.is_empty() {
        return (0.0, ValidationStatus::Pending);
    }

    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total_count = checks.len();

    let quality_score = (passed_count as f64 / total_count as f64) * 100.0;

    // Determine status based on score thresholds
    let status = if quality_score >= 90.0 {
        ValidationStatus::Pass
    } else if quality_score >= 70.0 {
        ValidationStatus::Warning
    } else {
        ValidationStatus::Fail
    };

    (quality_score, status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_score_all_pass() {
        let checks = vec![
            ValidationCheck {
                name: "Check 1".to_string(),
                passed: true,
                score: None,
                message: None,
            },
            ValidationCheck {
                name: "Check 2".to_string(),
                passed: true,
                score: None,
                message: None,
            },
        ];

        let (score, status) = calculate_quality_score(&checks);
        assert_eq!(score, 100.0);
        assert_eq!(status, ValidationStatus::Pass);
    }

    #[test]
    fn test_quality_score_warning() {
        let checks = vec![
            ValidationCheck {
                name: "Check 1".to_string(),
                passed: true,
                score: None,
                message: None,
            },
            ValidationCheck {
                name: "Check 2".to_string(),
                passed: false,
                score: None,
                message: Some("Failed".to_string()),
            },
            ValidationCheck {
                name: "Check 3".to_string(),
                passed: true,
                score: None,
                message: None,
            },
        ];

        let (score, status) = calculate_quality_score(&checks);
        assert!((score - 66.67).abs() < 0.1);
        assert_eq!(status, ValidationStatus::Fail); // < 70%
    }

    #[test]
    fn test_quality_score_empty() {
        let (score, status) = calculate_quality_score(&[]);
        assert_eq!(score, 0.0);
        assert_eq!(status, ValidationStatus::Pending);
    }
}
