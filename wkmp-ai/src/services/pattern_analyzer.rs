//! Pattern Analyzer Service
//!
//! **[REQ-PATT-010]** Segment pattern analysis for source media classification
//! **[PLAN025 Phase 2]** Intelligence-gathering component
//!
//! Analyzes audio segment patterns to determine:
//! - Track count (REQ-PATT-020)
//! - Gap patterns: mean, std dev (REQ-PATT-030)
//! - Source media type: CD/Vinyl/Cassette/Unknown (REQ-PATT-040)

use thiserror::Error;

/// Pattern analyzer errors
#[derive(Debug, Error)]
pub enum PatternError {
    /// Invalid input (empty segments, negative durations)
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Analysis failed
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
}

/// Source media type classification
///
/// **[REQ-PATT-040]** Heuristic-based classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceMedia {
    /// CD (Compact Disc) - consistent gaps, 1.5-3.5s mean gap
    CD,
    /// Vinyl record - variable gaps, >3.0s mean gap
    Vinyl,
    /// Cassette tape - similar to vinyl, lower confidence
    Cassette,
    /// Unknown source media (cannot classify)
    Unknown,
}

impl SourceMedia {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceMedia::CD => "CD",
            SourceMedia::Vinyl => "Vinyl",
            SourceMedia::Cassette => "Cassette",
            SourceMedia::Unknown => "Unknown",
        }
    }
}

/// Gap pattern classification
///
/// **[REQ-PATT-030]** Gap pattern analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapPattern {
    /// Consistent gaps (low std dev < 0.5s)
    Consistent,
    /// Variable gaps (high std dev >= 0.5s)
    Variable,
    /// No gaps detected (single segment)
    None,
}

impl GapPattern {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            GapPattern::Consistent => "Consistent",
            GapPattern::Variable => "Variable",
            GapPattern::None => "None",
        }
    }
}

/// Segment boundary for pattern analysis
#[derive(Debug, Clone)]
pub struct Segment {
    /// Start time in seconds
    pub start_seconds: f32,
    /// End time in seconds
    pub end_seconds: f32,
}

impl Segment {
    /// Create new segment
    pub fn new(start_seconds: f32, end_seconds: f32) -> Self {
        Self {
            start_seconds,
            end_seconds,
        }
    }

    /// Calculate segment duration
    pub fn duration(&self) -> f32 {
        self.end_seconds - self.start_seconds
    }
}

/// Pattern metadata output
///
/// **[REQ-PATT-010]** Complete pattern analysis results
#[derive(Debug, Clone)]
pub struct PatternMetadata {
    /// Number of tracks/segments detected (REQ-PATT-020)
    pub track_count: usize,

    /// Source media type classification (REQ-PATT-040)
    pub likely_source_media: SourceMedia,

    /// Gap pattern classification (REQ-PATT-030)
    pub gap_pattern: GapPattern,

    /// Segment durations in seconds
    pub segment_durations: Vec<f32>,

    /// Mean gap duration in seconds (REQ-PATT-030)
    pub mean_gap_duration: Option<f32>,

    /// Gap standard deviation in seconds (REQ-PATT-030)
    pub gap_std_dev: Option<f32>,

    /// Analysis confidence (0.0-1.0)
    pub confidence: f32,
}

/// Pattern Analyzer
///
/// **[REQ-PATT-010]** Analyzes segment patterns to classify source media
pub struct PatternAnalyzer;

impl PatternAnalyzer {
    /// Create new pattern analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyze segment patterns
    ///
    /// **[REQ-PATT-010]** Complete pattern analysis
    ///
    /// # Arguments
    /// * `segments` - List of segment boundaries
    ///
    /// # Returns
    /// `PatternMetadata` with classification results
    ///
    /// # Errors
    /// Returns `PatternError::InvalidInput` if segments are empty or invalid
    pub fn analyze(&self, segments: &[Segment]) -> Result<PatternMetadata, PatternError> {
        // Validate input
        if segments.is_empty() {
            return Err(PatternError::InvalidInput(
                "Empty segment list".to_string(),
            ));
        }

        // REQ-PATT-020: Track count detection
        let track_count = segments.len();

        // Calculate segment durations
        let segment_durations: Vec<f32> = segments.iter().map(|s| s.duration()).collect();

        // REQ-PATT-030: Gap pattern analysis
        let (mean_gap, gap_std_dev, gap_pattern) = if segments.len() > 1 {
            let gaps = self.calculate_gaps(segments);
            let mean = self.calculate_mean(&gaps);
            let std_dev = self.calculate_std_dev(&gaps, mean);

            let pattern = if std_dev < 0.5 {
                GapPattern::Consistent
            } else {
                GapPattern::Variable
            };

            (Some(mean), Some(std_dev), pattern)
        } else {
            (None, None, GapPattern::None)
        };

        // REQ-PATT-040: Source media classification
        let (likely_source_media, confidence) = self.classify_source_media(
            track_count,
            mean_gap,
            gap_std_dev,
            gap_pattern,
        );

        Ok(PatternMetadata {
            track_count,
            likely_source_media,
            gap_pattern,
            segment_durations,
            mean_gap_duration: mean_gap,
            gap_std_dev,
            confidence,
        })
    }

    /// Calculate gaps between segments
    ///
    /// **[REQ-PATT-030]** Gap detection
    fn calculate_gaps(&self, segments: &[Segment]) -> Vec<f32> {
        let mut gaps = Vec::new();
        for i in 0..segments.len() - 1 {
            let gap = segments[i + 1].start_seconds - segments[i].end_seconds;
            if gap > 0.0 {
                gaps.push(gap);
            }
        }
        gaps
    }

    /// Calculate mean of values
    fn calculate_mean(&self, values: &[f32]) -> f32 {
        if values.is_empty() {
            return 0.0;
        }
        values.iter().sum::<f32>() / values.len() as f32
    }

    /// Calculate standard deviation
    fn calculate_std_dev(&self, values: &[f32], mean: f32) -> f32 {
        if values.is_empty() {
            return 0.0;
        }
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32;
        variance.sqrt()
    }

    /// Classify source media based on heuristics
    ///
    /// **[REQ-PATT-040]** Source media classification
    ///
    /// # Heuristics (from SPEC032 analysis):
    /// - **CD**: Gap std dev < 0.5s, mean gap 1.5-3.5s, track count 8-20, confidence 0.9
    /// - **Vinyl**: Gap std dev >= 0.5s, mean gap > 3.0s, track count 4-12, confidence 0.7
    /// - **Cassette**: Similar to vinyl but lower confidence 0.5
    /// - **Unknown**: None match, confidence 0.3
    fn classify_source_media(
        &self,
        track_count: usize,
        mean_gap: Option<f32>,
        gap_std_dev: Option<f32>,
        gap_pattern: GapPattern,
    ) -> (SourceMedia, f32) {
        // Single segment files - likely single track (cannot classify)
        if track_count == 1 {
            return (SourceMedia::Unknown, 0.3);
        }

        // Need gap data for classification
        let Some(mean) = mean_gap else {
            return (SourceMedia::Unknown, 0.3);
        };
        let Some(std_dev) = gap_std_dev else {
            return (SourceMedia::Unknown, 0.3);
        };

        // CD Classification
        // - Consistent gaps (std dev < 0.5s)
        // - Mean gap 1.5-3.5s (typical CD inter-track silence)
        // - Track count 8-20 (typical album)
        if std_dev < 0.5 && (1.5..=3.5).contains(&mean) && (8..=20).contains(&track_count) {
            return (SourceMedia::CD, 0.9);
        }

        // Vinyl Classification
        // - Variable gaps (std dev >= 0.5s) OR long gaps (mean > 3.0s)
        // - Track count 4-12 per side (8-24 total for both sides, but we see one side)
        if (std_dev >= 0.5 || mean > 3.0) && (4..=12).contains(&track_count) {
            return (SourceMedia::Vinyl, 0.7);
        }

        // Cassette Classification (similar to vinyl but lower confidence)
        // - Variable gaps, longer gaps possible
        // - Similar track count range
        if (std_dev >= 0.5 || mean > 3.0) && (4..=15).contains(&track_count) {
            return (SourceMedia::Cassette, 0.5);
        }

        // Default: Unknown
        (SourceMedia::Unknown, 0.3)
    }
}

impl Default for PatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **[TC-U-PATT-010-01]** Unit test: Verify pattern analyzer accepts segment list
    #[test]
    fn tc_u_patt_010_01_accepts_segment_list() {
        let analyzer = PatternAnalyzer::new();
        let segments = vec![
            Segment::new(0.0, 180.0),
            Segment::new(182.0, 360.0),
        ];

        let result = analyzer.analyze(&segments);
        assert!(result.is_ok(), "Analyzer should accept valid segment list");
    }

    /// **[TC-U-PATT-010-02]** Unit test: Verify pattern metadata output format
    #[test]
    fn tc_u_patt_010_02_output_format() {
        let analyzer = PatternAnalyzer::new();
        let segments = vec![
            Segment::new(0.0, 180.0),
            Segment::new(182.0, 360.0),
            Segment::new(362.5, 540.0),
        ];

        let result = analyzer.analyze(&segments).unwrap();

        // Verify all fields present
        assert_eq!(result.track_count, 3);
        assert!(result.segment_durations.len() == 3);
        assert!(result.mean_gap_duration.is_some());
        assert!(result.gap_std_dev.is_some());
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    /// **[TC-U-PATT-020-01]** Unit test: Verify track count detection
    #[test]
    fn tc_u_patt_020_01_track_count_detection() {
        let analyzer = PatternAnalyzer::new();

        // Test single segment
        let segments_1 = vec![Segment::new(0.0, 180.0)];
        let result_1 = analyzer.analyze(&segments_1).unwrap();
        assert_eq!(result_1.track_count, 1);

        // Test multiple segments
        let segments_10 = vec![
            Segment::new(0.0, 180.0),
            Segment::new(182.0, 360.0),
            Segment::new(362.0, 540.0),
            Segment::new(542.0, 720.0),
            Segment::new(722.0, 900.0),
            Segment::new(902.0, 1080.0),
            Segment::new(1082.0, 1260.0),
            Segment::new(1262.0, 1440.0),
            Segment::new(1442.0, 1620.0),
            Segment::new(1622.0, 1800.0),
        ];
        let result_10 = analyzer.analyze(&segments_10).unwrap();
        assert_eq!(result_10.track_count, 10);
    }

    /// **[TC-U-PATT-030-01]** Unit test: Verify gap pattern analysis (consistent gaps)
    #[test]
    fn tc_u_patt_030_01_consistent_gaps() {
        let analyzer = PatternAnalyzer::new();

        // CD-like: 10 tracks with 2-second gaps (very consistent)
        let mut segments = Vec::new();
        for i in 0..10 {
            let start = i as f32 * 182.0;
            let end = start + 180.0;
            segments.push(Segment::new(start, end));
        }

        let result = analyzer.analyze(&segments).unwrap();
        assert_eq!(result.gap_pattern, GapPattern::Consistent);
        assert!(result.gap_std_dev.unwrap() < 0.5);
    }

    /// **[TC-U-PATT-030-02]** Unit test: Verify gap pattern analysis (variable gaps)
    #[test]
    fn tc_u_patt_030_02_variable_gaps() {
        let analyzer = PatternAnalyzer::new();

        // Vinyl-like: variable gaps (1s, 5s, 2s, 8s)
        let segments = vec![
            Segment::new(0.0, 180.0),
            Segment::new(181.0, 360.0),   // 1s gap
            Segment::new(365.0, 540.0),   // 5s gap
            Segment::new(542.0, 720.0),   // 2s gap
            Segment::new(728.0, 900.0),   // 8s gap
        ];

        let result = analyzer.analyze(&segments).unwrap();
        assert_eq!(result.gap_pattern, GapPattern::Variable);
        assert!(result.gap_std_dev.unwrap() >= 0.5);
    }

    /// **[TC-U-PATT-040-01]** Unit test: Verify source media classification (CD)
    #[test]
    fn tc_u_patt_040_01_classify_cd() {
        let analyzer = PatternAnalyzer::new();

        // CD characteristics: 12 tracks, consistent 2s gaps, mean ~2s
        let mut segments = Vec::new();
        for i in 0..12 {
            let start = i as f32 * 182.0;
            let end = start + 180.0;
            segments.push(Segment::new(start, end));
        }

        let result = analyzer.analyze(&segments).unwrap();
        assert_eq!(result.likely_source_media, SourceMedia::CD);
        assert!(result.confidence >= 0.8, "CD classification should have high confidence");
    }

    /// **[TC-U-PATT-040-02]** Unit test: Verify source media classification (Vinyl)
    #[test]
    fn tc_u_patt_040_02_classify_vinyl() {
        let analyzer = PatternAnalyzer::new();

        // Vinyl characteristics: 6 tracks, variable gaps (2s, 6s, 3s, 7s, 4s)
        let segments = vec![
            Segment::new(0.0, 240.0),
            Segment::new(242.0, 480.0),   // 2s gap
            Segment::new(486.0, 720.0),   // 6s gap
            Segment::new(723.0, 960.0),   // 3s gap
            Segment::new(967.0, 1200.0),  // 7s gap
            Segment::new(1204.0, 1440.0), // 4s gap
        ];

        let result = analyzer.analyze(&segments).unwrap();
        assert_eq!(result.likely_source_media, SourceMedia::Vinyl);
        assert!(result.confidence >= 0.6, "Vinyl classification should have medium-high confidence");
    }

    /// **[TC-U-PATT-010-03]** Unit test: Verify empty segment list rejected
    #[test]
    fn tc_u_patt_010_03_empty_segments_rejected() {
        let analyzer = PatternAnalyzer::new();
        let segments: Vec<Segment> = vec![];

        let result = analyzer.analyze(&segments);
        assert!(result.is_err(), "Empty segment list should be rejected");
    }

    /// **[TC-U-PATT-030-03]** Unit test: Verify single segment has no gap pattern
    #[test]
    fn tc_u_patt_030_03_single_segment_no_gaps() {
        let analyzer = PatternAnalyzer::new();
        let segments = vec![Segment::new(0.0, 180.0)];

        let result = analyzer.analyze(&segments).unwrap();
        assert_eq!(result.gap_pattern, GapPattern::None);
        assert!(result.mean_gap_duration.is_none());
        assert!(result.gap_std_dev.is_none());
    }
}
