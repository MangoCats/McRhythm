// PLAN023 Tier 3: Completeness Scorer
//
// Concept: Calculate quality score based on metadata completeness
// Synchronization: Accepts FusedMetadata, outputs quality score
//
// Algorithm (per SPEC_wkmp_ai_recode.md):
// 1. Count present fields (non-None values)
// 2. Weight fields by importance (required vs. optional)
// 3. Factor in field confidence scores
// 4. Compute overall completeness score [0.0, 1.0]

use crate::import_v2::types::FusedMetadata;

/// Field importance weights
#[derive(Debug, Clone)]
pub struct FieldWeights {
    pub title: f64,
    pub artist: f64,
    pub album: f64,
    pub release_date: f64,
    pub track_number: f64,
    pub duration_ms: f64,
}

impl Default for FieldWeights {
    fn default() -> Self {
        Self {
            title: 1.0,         // Required
            artist: 1.0,        // Required
            album: 0.7,         // Important but not required
            release_date: 0.5,  // Optional
            track_number: 0.3,  // Optional
            duration_ms: 0.3,   // Optional
        }
    }
}

/// Completeness scorer (Tier 3 validation concept)
///
/// **Legible Software Principle:**
/// - Independent module: Pure scoring logic, no side effects
/// - Explicit synchronization: Clear contract with Tier 2 fusers
/// - Transparent behavior: Weighting formula is explicit
/// - Integrity: Score always in [0.0, 1.0] range
pub struct CompletenessScorer {
    /// Field importance weights
    weights: FieldWeights,
}

impl Default for CompletenessScorer {
    fn default() -> Self {
        Self {
            weights: FieldWeights::default(),
        }
    }
}

impl CompletenessScorer {
    /// Calculate completeness score for metadata
    ///
    /// # Algorithm: Weighted Presence + Confidence
    /// 1. For each field:
    ///    - If present: score = weight × confidence
    ///    - If absent: score = 0
    /// 2. Sum all field scores
    /// 3. Divide by sum of all weights
    /// 4. Result is in [0.0, 1.0] where 1.0 = all fields present with max confidence
    ///
    /// # Arguments
    /// * `metadata` - Fused metadata from Tier 2
    ///
    /// # Returns
    /// Completeness score in range [0.0, 1.0]
    pub fn score(&self, metadata: &FusedMetadata) -> f64 {
        let mut total_score = 0.0;
        let total_weight = self.total_weight();

        // Score each field (present fields contribute weight × confidence)
        if let Some(ref field) = metadata.title {
            total_score += self.weights.title * field.confidence;
        }

        if let Some(ref field) = metadata.artist {
            total_score += self.weights.artist * field.confidence;
        }

        if let Some(ref field) = metadata.album {
            total_score += self.weights.album * field.confidence;
        }

        if let Some(ref field) = metadata.release_date {
            total_score += self.weights.release_date * field.confidence;
        }

        if let Some(ref field) = metadata.track_number {
            total_score += self.weights.track_number * field.confidence;
        }

        if let Some(ref field) = metadata.duration_ms {
            total_score += self.weights.duration_ms * field.confidence;
        }

        // Normalize by total possible weight
        let score = if total_weight > 0.0 {
            total_score / total_weight
        } else {
            0.0
        };

        // Ensure score is in valid range [0.0, 1.0]
        score.clamp(0.0, 1.0)
    }

    /// Calculate total weight (sum of all field weights)
    fn total_weight(&self) -> f64 {
        self.weights.title
            + self.weights.artist
            + self.weights.album
            + self.weights.release_date
            + self.weights.track_number
            + self.weights.duration_ms
    }

    /// Count number of present fields
    pub fn count_present_fields(&self, metadata: &FusedMetadata) -> usize {
        let mut count = 0;

        if metadata.title.is_some() {
            count += 1;
        }
        if metadata.artist.is_some() {
            count += 1;
        }
        if metadata.album.is_some() {
            count += 1;
        }
        if metadata.release_date.is_some() {
            count += 1;
        }
        if metadata.track_number.is_some() {
            count += 1;
        }
        if metadata.duration_ms.is_some() {
            count += 1;
        }

        count
    }

    /// Check if required fields are present (title + artist)
    pub fn has_required_fields(&self, metadata: &FusedMetadata) -> bool {
        metadata.title.is_some() && metadata.artist.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_v2::types::{ExtractionSource, MetadataField};

    fn create_field<T>(value: T, confidence: f64) -> MetadataField<T> {
        MetadataField {
            value,
            confidence,
            source: ExtractionSource::MusicBrainz,
        }
    }

    #[test]
    fn test_empty_metadata() {
        let scorer = CompletenessScorer::default();

        let metadata = FusedMetadata {
            title: None,
            artist: None,
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.0,
        };

        let score = scorer.score(&metadata);
        assert_eq!(score, 0.0);
        assert_eq!(scorer.count_present_fields(&metadata), 0);
        assert!(!scorer.has_required_fields(&metadata));
    }

    #[test]
    fn test_required_fields_only() {
        let scorer = CompletenessScorer::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Let It Be".to_string(), 0.9)),
            artist: Some(create_field("The Beatles".to_string(), 0.9)),
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.9,
        };

        // Score = (1.0*0.9 + 1.0*0.9) / (1.0+1.0+0.7+0.5+0.3+0.3) = 1.8 / 3.8 = 0.474
        let score = scorer.score(&metadata);
        assert!((score - 0.474).abs() < 0.01);
        assert_eq!(scorer.count_present_fields(&metadata), 2);
        assert!(scorer.has_required_fields(&metadata));
    }

    #[test]
    fn test_all_fields_perfect_confidence() {
        let scorer = CompletenessScorer::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Let It Be".to_string(), 1.0)),
            artist: Some(create_field("The Beatles".to_string(), 1.0)),
            album: Some(create_field("Let It Be".to_string(), 1.0)),
            release_date: Some(create_field("1970-05-08".to_string(), 1.0)),
            track_number: Some(create_field(6, 1.0)),
            duration_ms: Some(create_field(240000, 1.0)),
            metadata_confidence: 1.0,
        };

        // All fields present with perfect confidence = 1.0
        let score = scorer.score(&metadata);
        assert_eq!(score, 1.0);
        assert_eq!(scorer.count_present_fields(&metadata), 6);
    }

    #[test]
    fn test_all_fields_low_confidence() {
        let scorer = CompletenessScorer::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Let It Be".to_string(), 0.5)),
            artist: Some(create_field("The Beatles".to_string(), 0.5)),
            album: Some(create_field("Let It Be".to_string(), 0.5)),
            release_date: Some(create_field("1970-05-08".to_string(), 0.5)),
            track_number: Some(create_field(6, 0.5)),
            duration_ms: Some(create_field(240000, 0.5)),
            metadata_confidence: 0.5,
        };

        // All fields present but with 0.5 confidence
        // Score = (1.0*0.5 + 1.0*0.5 + 0.7*0.5 + 0.5*0.5 + 0.3*0.5 + 0.3*0.5) / 3.8
        // = (0.5 + 0.5 + 0.35 + 0.25 + 0.15 + 0.15) / 3.8 = 1.9 / 3.8 = 0.5
        let score = scorer.score(&metadata);
        assert_eq!(score, 0.5);
        assert_eq!(scorer.count_present_fields(&metadata), 6);
    }

    #[test]
    fn test_partial_fields_high_confidence() {
        let scorer = CompletenessScorer::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Let It Be".to_string(), 0.95)),
            artist: Some(create_field("The Beatles".to_string(), 0.95)),
            album: Some(create_field("Let It Be".to_string(), 0.9)),
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.93,
        };

        // Score = (1.0*0.95 + 1.0*0.95 + 0.7*0.9) / 3.8
        // = (0.95 + 0.95 + 0.63) / 3.8 = 2.53 / 3.8 = 0.666
        let score = scorer.score(&metadata);
        assert!((score - 0.666).abs() < 0.01);
        assert_eq!(scorer.count_present_fields(&metadata), 3);
    }

    #[test]
    fn test_varying_confidence() {
        let scorer = CompletenessScorer::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Title".to_string(), 1.0)),
            artist: Some(create_field("Artist".to_string(), 0.8)),
            album: Some(create_field("Album".to_string(), 0.6)),
            release_date: Some(create_field("2024-01-01".to_string(), 0.4)),
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.75,
        };

        // Score = (1.0*1.0 + 1.0*0.8 + 0.7*0.6 + 0.5*0.4) / 3.8
        // = (1.0 + 0.8 + 0.42 + 0.2) / 3.8 = 2.42 / 3.8 = 0.637
        let score = scorer.score(&metadata);
        assert!((score - 0.637).abs() < 0.01);
    }

    #[test]
    fn test_custom_weights() {
        let scorer = CompletenessScorer {
            weights: FieldWeights {
                title: 2.0,    // Higher importance
                artist: 2.0,   // Higher importance
                album: 0.5,    // Lower importance
                release_date: 0.1,
                track_number: 0.1,
                duration_ms: 0.1,
            },
        };

        let metadata = FusedMetadata {
            title: Some(create_field("Title".to_string(), 1.0)),
            artist: Some(create_field("Artist".to_string(), 1.0)),
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 1.0,
        };

        // Total weight = 2.0 + 2.0 + 0.5 + 0.1 + 0.1 + 0.1 = 4.8
        // Score = (2.0*1.0 + 2.0*1.0) / 4.8 = 4.0 / 4.8 = 0.833
        let score = scorer.score(&metadata);
        assert!((score - 0.833).abs() < 0.01);
    }

    #[test]
    fn test_score_clamping() {
        let scorer = CompletenessScorer::default();

        // Even with confidence > 1.0 (shouldn't happen but test clamping)
        let metadata = FusedMetadata {
            title: Some(create_field("Title".to_string(), 1.5)),
            artist: Some(create_field("Artist".to_string(), 1.5)),
            album: Some(create_field("Album".to_string(), 1.5)),
            release_date: Some(create_field("2024-01-01".to_string(), 1.5)),
            track_number: Some(create_field(1, 1.5)),
            duration_ms: Some(create_field(180000, 1.5)),
            metadata_confidence: 1.5,
        };

        let score = scorer.score(&metadata);
        assert!(score <= 1.0); // Should be clamped to 1.0
        assert!(score >= 0.0); // Should never be negative
    }

    #[test]
    fn test_missing_title_no_required_fields() {
        let scorer = CompletenessScorer::default();

        let metadata = FusedMetadata {
            title: None,
            artist: Some(create_field("Artist".to_string(), 1.0)),
            album: Some(create_field("Album".to_string(), 1.0)),
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 1.0,
        };

        assert!(!scorer.has_required_fields(&metadata));
    }

    #[test]
    fn test_missing_artist_no_required_fields() {
        let scorer = CompletenessScorer::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Title".to_string(), 1.0)),
            artist: None,
            album: Some(create_field("Album".to_string(), 1.0)),
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 1.0,
        };

        assert!(!scorer.has_required_fields(&metadata));
    }
}
