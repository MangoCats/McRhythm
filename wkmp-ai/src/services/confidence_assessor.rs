//! Confidence Assessor Service
//!
//! **[REQ-CONF-010]** Evidence-based confidence assessment for MBID identification
//! **[PLAN025 Phase 2]** Intelligence-gathering component
//!
//! Combines multiple evidence sources (metadata, fingerprint, duration) to produce
//! confidence score and decision (Accept/Review/Reject).

use thiserror::Error;

/// Confidence assessor errors
#[derive(Debug, Error)]
pub enum ConfidenceError {
    /// Invalid input (missing evidence, out-of-range values)
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Identification decision based on confidence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Accept identification (confidence >= 0.85)
    Accept,
    /// Review manually (confidence 0.60-0.85)
    Review,
    /// Reject identification (confidence < 0.60)
    Reject,
}

impl Decision {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Decision::Accept => "Accept",
            Decision::Review => "Review",
            Decision::Reject => "Reject",
        }
    }
}

/// Evidence for confidence assessment
#[derive(Debug, Clone)]
pub struct Evidence {
    /// Metadata match score (0.0-1.0) from ContextualMatcher
    pub metadata_score: f32,

    /// Fingerprint match score (0.0-1.0) from AcoustID
    pub fingerprint_score: f32,

    /// Duration match score (0.0 or 1.0) - exact match or not
    pub duration_match: f32,
}

/// Confidence assessment result
#[derive(Debug, Clone)]
pub struct ConfidenceResult {
    /// Combined confidence score (0.0-1.0)
    pub confidence: f32,

    /// Decision based on threshold
    pub decision: Decision,

    /// Evidence summary
    pub evidence: Evidence,
}

/// Confidence Assessor
///
/// **[REQ-CONF-010]** Evidence-based confidence assessment
pub struct ConfidenceAssessor {
    /// Metadata weight (default 0.30 = 30%)
    metadata_weight: f32,

    /// Fingerprint weight (default 0.60 = 60%)
    fingerprint_weight: f32,

    /// Duration weight (default 0.10 = 10%)
    duration_weight: f32,

    /// Accept threshold (default 0.85)
    accept_threshold: f32,

    /// Review threshold (default 0.60)
    review_threshold: f32,
}

impl ConfidenceAssessor {
    /// Create new confidence assessor with default weights and thresholds
    ///
    /// **Default Weights (Single-Segment):**
    /// - Metadata: 30%
    /// - Fingerprint: 60%
    /// - Duration: 10%
    ///
    /// **Default Thresholds:**
    /// - Accept: ≥0.85
    /// - Review: 0.60-0.85
    /// - Reject: <0.60
    pub fn new() -> Self {
        Self {
            metadata_weight: 0.30,
            fingerprint_weight: 0.60,
            duration_weight: 0.10,
            accept_threshold: 0.85,
            review_threshold: 0.60,
        }
    }

    /// Assess confidence for single-segment file
    ///
    /// **[REQ-CONF-010]** Single-segment evidence combination
    ///
    /// # Arguments
    /// * `evidence` - Evidence from metadata, fingerprint, and duration matching
    ///
    /// # Returns
    /// Confidence result with score and decision
    ///
    /// # Errors
    /// Returns error if evidence scores are out of range (must be 0.0-1.0)
    pub fn assess_single_segment(&self, evidence: Evidence) -> Result<ConfidenceResult, ConfidenceError> {
        // Validate evidence scores
        if evidence.metadata_score < 0.0 || evidence.metadata_score > 1.0 {
            return Err(ConfidenceError::InvalidInput(format!(
                "Metadata score out of range: {}",
                evidence.metadata_score
            )));
        }
        if evidence.fingerprint_score < 0.0 || evidence.fingerprint_score > 1.0 {
            return Err(ConfidenceError::InvalidInput(format!(
                "Fingerprint score out of range: {}",
                evidence.fingerprint_score
            )));
        }
        if evidence.duration_match < 0.0 || evidence.duration_match > 1.0 {
            return Err(ConfidenceError::InvalidInput(format!(
                "Duration match out of range: {}",
                evidence.duration_match
            )));
        }

        // **[REQ-CONF-010]** Weighted combination: 30% metadata + 60% fingerprint + 10% duration
        let confidence = (evidence.metadata_score * self.metadata_weight)
            + (evidence.fingerprint_score * self.fingerprint_weight)
            + (evidence.duration_match * self.duration_weight);

        // Determine decision based on thresholds
        let decision = if confidence >= self.accept_threshold {
            Decision::Accept
        } else if confidence >= self.review_threshold {
            Decision::Review
        } else {
            Decision::Reject
        };

        Ok(ConfidenceResult {
            confidence,
            decision,
            evidence,
        })
    }

    /// Assess confidence for multi-segment file (album)
    ///
    /// **[REQ-CONF-010]** Multi-segment evidence combination
    ///
    /// For albums, evidence includes:
    /// - Album metadata match (artist + album title + track count)
    /// - Per-track fingerprint matches (aggregated)
    /// - Track duration alignment
    ///
    /// # Arguments
    /// * `evidence` - Evidence from contextual matching and fingerprinting
    ///
    /// # Returns
    /// Confidence result with score and decision
    pub fn assess_multi_segment(&self, evidence: Evidence) -> Result<ConfidenceResult, ConfidenceError> {
        // For multi-segment, use similar weighting but adjust for album-level matching
        // Metadata weight slightly higher (album structure evidence)
        let metadata_weight = 0.35;
        let fingerprint_weight = 0.55;
        let duration_weight = 0.10;

        // Validate evidence scores
        if evidence.metadata_score < 0.0 || evidence.metadata_score > 1.0 {
            return Err(ConfidenceError::InvalidInput(format!(
                "Metadata score out of range: {}",
                evidence.metadata_score
            )));
        }
        if evidence.fingerprint_score < 0.0 || evidence.fingerprint_score > 1.0 {
            return Err(ConfidenceError::InvalidInput(format!(
                "Fingerprint score out of range: {}",
                evidence.fingerprint_score
            )));
        }
        if evidence.duration_match < 0.0 || evidence.duration_match > 1.0 {
            return Err(ConfidenceError::InvalidInput(format!(
                "Duration match out of range: {}",
                evidence.duration_match
            )));
        }

        // Weighted combination
        let confidence = (evidence.metadata_score * metadata_weight)
            + (evidence.fingerprint_score * fingerprint_weight)
            + (evidence.duration_match * duration_weight);

        // Determine decision based on thresholds
        let decision = if confidence >= self.accept_threshold {
            Decision::Accept
        } else if confidence >= self.review_threshold {
            Decision::Review
        } else {
            Decision::Reject
        };

        Ok(ConfidenceResult {
            confidence,
            decision,
            evidence,
        })
    }
}

impl Default for ConfidenceAssessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **[TC-U-CONF-010-01]** Unit test: Verify evidence combination (single-segment)
    #[test]
    fn tc_u_conf_010_01_single_segment_evidence() {
        let assessor = ConfidenceAssessor::new();

        // Test high confidence case: strong metadata + fingerprint match
        let evidence_high = Evidence {
            metadata_score: 0.9,
            fingerprint_score: 0.95,
            duration_match: 1.0,
        };

        let result = assessor.assess_single_segment(evidence_high).unwrap();
        assert!(result.confidence > 0.9, "High evidence should yield high confidence");
        assert_eq!(result.decision, Decision::Accept);
    }

    /// **[TC-U-CONF-010-02]** Unit test: Verify evidence combination (multi-segment)
    #[test]
    fn tc_u_conf_010_02_multi_segment_evidence() {
        let assessor = ConfidenceAssessor::new();

        // Test medium confidence case: good metadata, weak fingerprint
        let evidence_medium = Evidence {
            metadata_score: 0.85,
            fingerprint_score: 0.60,
            duration_match: 1.0,
        };

        let result = assessor.assess_multi_segment(evidence_medium).unwrap();
        assert!(result.confidence >= 0.70, "Medium evidence should yield medium confidence");
    }

    /// **[TC-U-CONF-010-03]** Unit test: Verify decision thresholds
    #[test]
    fn tc_u_conf_010_03_decision_thresholds() {
        let assessor = ConfidenceAssessor::new();

        // Test Accept threshold (>= 0.85)
        let evidence_accept = Evidence {
            metadata_score: 0.9,
            fingerprint_score: 0.9,
            duration_match: 1.0,
        };
        let result_accept = assessor.assess_single_segment(evidence_accept).unwrap();
        assert_eq!(result_accept.decision, Decision::Accept);
        assert!(result_accept.confidence >= 0.85);

        // Test Review threshold (0.60-0.85)
        let evidence_review = Evidence {
            metadata_score: 0.7,
            fingerprint_score: 0.7,
            duration_match: 0.0,
        };
        let result_review = assessor.assess_single_segment(evidence_review).unwrap();
        assert_eq!(result_review.decision, Decision::Review);
        assert!(result_review.confidence >= 0.60 && result_review.confidence < 0.85);

        // Test Reject threshold (< 0.60)
        let evidence_reject = Evidence {
            metadata_score: 0.3,
            fingerprint_score: 0.4,
            duration_match: 0.0,
        };
        let result_reject = assessor.assess_single_segment(evidence_reject).unwrap();
        assert_eq!(result_reject.decision, Decision::Reject);
        assert!(result_reject.confidence < 0.60);
    }

    /// **[TC-U-CONF-010-04]** Unit test: Verify weighted combination
    #[test]
    fn tc_u_conf_010_04_weighted_combination() {
        let assessor = ConfidenceAssessor::new();

        // Test that fingerprint weight (60%) dominates
        let evidence = Evidence {
            metadata_score: 0.3,  // Low metadata
            fingerprint_score: 0.95, // High fingerprint
            duration_match: 1.0,
        };

        let result = assessor.assess_single_segment(evidence).unwrap();
        // Expected: 0.3*0.3 + 0.95*0.6 + 1.0*0.1 = 0.09 + 0.57 + 0.1 = 0.76
        assert!(result.confidence > 0.7, "High fingerprint should dominate confidence");
        assert_eq!(result.decision, Decision::Review); // 0.76 is in Review range
    }

    /// **[TC-U-CONF-010-05]** Unit test: Verify out-of-range rejection
    #[test]
    fn tc_u_conf_010_05_out_of_range_rejection() {
        let assessor = ConfidenceAssessor::new();

        // Test out-of-range metadata score
        let evidence_invalid = Evidence {
            metadata_score: 1.5, // Invalid: > 1.0
            fingerprint_score: 0.8,
            duration_match: 1.0,
        };

        let result = assessor.assess_single_segment(evidence_invalid);
        assert!(result.is_err(), "Out-of-range evidence should be rejected");
    }

    /// **[TC-U-CONF-010-06]** Unit test: Verify boundary cases
    #[test]
    fn tc_u_conf_010_06_boundary_cases() {
        let assessor = ConfidenceAssessor::new();

        // Test exact Accept threshold (0.85)
        let evidence_boundary_accept = Evidence {
            metadata_score: 0.85,
            fingerprint_score: 0.85,
            duration_match: 0.85,
        };
        let result = assessor.assess_single_segment(evidence_boundary_accept).unwrap();
        assert_eq!(result.decision, Decision::Accept, "Confidence at 0.85 should Accept");

        // Test exact Review threshold (0.60)
        let evidence_boundary_review = Evidence {
            metadata_score: 0.6,
            fingerprint_score: 0.6,
            duration_match: 0.6,
        };
        let result_review = assessor.assess_single_segment(evidence_boundary_review).unwrap();
        assert_eq!(result_review.decision, Decision::Review, "Confidence at 0.60 should Review");
    }
}
