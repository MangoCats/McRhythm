//! Passage Song Matching for Import Pipeline
//!
//! **Traceability:** [REQ-SPEC032-013] Song Matching (Phase 6)
//!
//! Combines metadata and fingerprint evidence to determine MBID with confidence level
//! for each passage. Supports zero-song passages and adjacent passage merging.

use super::confidence_assessor::{ConfidenceAssessor, Evidence};
use super::metadata_merger::MergedMetadata;
use super::passage_fingerprinter::{FingerprintResult, PassageFingerprint};
use super::passage_segmenter::PassageBoundary;

/// Confidence level for song identification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    /// High confidence (≥0.85) - fingerprint + metadata match
    High,
    /// Medium confidence (0.70-0.85) - partial match
    Medium,
    /// Low confidence (0.60-0.70) - weak match
    Low,
    /// No match (<0.60 or no candidates) - zero-song passage
    None,
}

impl ConfidenceLevel {
    /// Convert confidence score to level
    pub fn from_score(score: f32) -> Self {
        if score >= 0.85 {
            ConfidenceLevel::High
        } else if score >= 0.70 {
            ConfidenceLevel::Medium
        } else if score >= 0.60 {
            ConfidenceLevel::Low
        } else {
            ConfidenceLevel::None
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfidenceLevel::High => "High",
            ConfidenceLevel::Medium => "Medium",
            ConfidenceLevel::Low => "Low",
            ConfidenceLevel::None => "None",
        }
    }
}

/// Song match for a passage
#[derive(Debug, Clone)]
pub struct PassageSongMatch {
    /// Passage boundary (ticks)
    pub passage: PassageBoundary,
    /// MusicBrainz Recording ID (None for zero-song passages)
    pub mbid: Option<String>,
    /// Confidence level
    pub confidence: ConfidenceLevel,
    /// Raw confidence score (0.0-1.0)
    pub score: f32,
    /// Recording title (from AcoustID or metadata)
    pub title: Option<String>,
}

/// Song matching result
#[derive(Debug, Clone)]
pub struct SongMatchResult {
    /// Matched passages with MBIDs
    pub matches: Vec<PassageSongMatch>,
    /// Statistics
    pub stats: SongMatchStats,
}

/// Song matching statistics
#[derive(Debug, Clone)]
pub struct SongMatchStats {
    /// Total passages processed
    pub total_passages: usize,
    /// High confidence matches
    pub high_confidence: usize,
    /// Medium confidence matches
    pub medium_confidence: usize,
    /// Low confidence matches
    pub low_confidence: usize,
    /// Zero-song passages (None confidence)
    pub zero_song: usize,
}

/// Passage Song Matcher
///
/// **Traceability:** [REQ-SPEC032-013] (Phase 6: SONG MATCHING)
pub struct PassageSongMatcher {
    assessor: ConfidenceAssessor,
}

impl PassageSongMatcher {
    /// Create new passage song matcher
    pub fn new() -> Self {
        Self {
            assessor: ConfidenceAssessor::new(),
        }
    }

    /// Match passages to songs
    ///
    /// **Algorithm:**
    /// 1. Check fingerprint result:
    ///    - If Skipped: Use metadata-only matching (all passages get Low confidence)
    ///    - If Failed: Return error
    ///    - If Success: Combine metadata + fingerprint evidence
    /// 2. For each passage:
    ///    a. Get fingerprint candidates (if available)
    ///    b. Get metadata (artist, title)
    ///    c. Calculate metadata match score (fuzzy string match)
    ///    d. Combine evidence using ConfidenceAssessor
    ///    e. Determine MBID with highest combined score
    ///    f. Classify as High/Medium/Low/None confidence
    /// 3. Merge adjacent zero-song passages (None confidence)
    /// 4. Return matches with statistics
    ///
    /// **Traceability:** [REQ-SPEC032-013]
    pub fn match_passages(
        &self,
        passages: &[PassageBoundary],
        fingerprint_result: &FingerprintResult,
        metadata: &MergedMetadata,
    ) -> SongMatchResult {
        tracing::debug!(
            passage_count = passages.len(),
            "Matching passages to songs"
        );

        let mut matches = Vec::new();

        match fingerprint_result {
            FingerprintResult::Success(fingerprints) => {
                // Combine metadata + fingerprint evidence
                for (idx, passage) in passages.iter().enumerate() {
                    let passage_match = if let Some(fp) = fingerprints.get(idx) {
                        self.match_passage_with_fingerprint(passage, fp, metadata)
                    } else {
                        // Passage skipped in fingerprinting (too short)
                        self.match_passage_metadata_only(passage, metadata)
                    };

                    matches.push(passage_match);
                }
            }
            FingerprintResult::Skipped => {
                // Metadata-only matching (no API key)
                tracing::info!("Using metadata-only matching (no fingerprints available)");
                for passage in passages {
                    matches.push(self.match_passage_metadata_only(passage, metadata));
                }
            }
            FingerprintResult::Failed(err) => {
                // Fingerprinting failed - fall back to metadata-only
                tracing::warn!(error = %err, "Fingerprinting failed, using metadata-only matching");
                for passage in passages {
                    matches.push(self.match_passage_metadata_only(passage, metadata));
                }
            }
        }

        // Merge adjacent zero-song passages
        let merged_matches = self.merge_zero_song_passages(matches);

        // Calculate statistics
        let stats = self.calculate_stats(&merged_matches);

        tracing::info!(
            total = stats.total_passages,
            high = stats.high_confidence,
            medium = stats.medium_confidence,
            low = stats.low_confidence,
            none = stats.zero_song,
            "Song matching complete"
        );

        SongMatchResult {
            matches: merged_matches,
            stats,
        }
    }

    /// Match passage with fingerprint + metadata
    fn match_passage_with_fingerprint(
        &self,
        passage: &PassageBoundary,
        fingerprint: &PassageFingerprint,
        metadata: &MergedMetadata,
    ) -> PassageSongMatch {
        // No candidates = zero-song passage
        if fingerprint.candidates.is_empty() {
            return PassageSongMatch {
                passage: passage.clone(),
                mbid: None,
                confidence: ConfidenceLevel::None,
                score: 0.0,
                title: None,
            };
        }

        // Get best candidate (already sorted by score in PassageFingerprinter)
        let best_candidate = &fingerprint.candidates[0];

        // Calculate metadata match score
        let metadata_score = self.calculate_metadata_score(
            &best_candidate.title,
            &metadata.artist,
            &metadata.title,
        );

        // Combine evidence
        let evidence = Evidence {
            metadata_score,
            fingerprint_score: best_candidate.score as f32,
            duration_match: 1.0, // Assume duration matches (passage from audio file)
        };

        let confidence_result = self
            .assessor
            .assess_single_segment(evidence)
            .unwrap_or_else(|_| {
                // Fallback if evidence invalid
                super::confidence_assessor::ConfidenceResult {
                    confidence: 0.0,
                    decision: super::confidence_assessor::Decision::Reject,
                    evidence: Evidence {
                        metadata_score: 0.0,
                        fingerprint_score: 0.0,
                        duration_match: 0.0,
                    },
                }
            });

        let confidence_level = ConfidenceLevel::from_score(confidence_result.confidence);

        // If confidence too low, treat as zero-song passage
        if confidence_level == ConfidenceLevel::None {
            return PassageSongMatch {
                passage: passage.clone(),
                mbid: None,
                confidence: ConfidenceLevel::None,
                score: confidence_result.confidence,
                title: None,
            };
        }

        PassageSongMatch {
            passage: passage.clone(),
            mbid: Some(best_candidate.mbid.clone()),
            confidence: confidence_level,
            score: confidence_result.confidence,
            title: best_candidate.title.clone(),
        }
    }

    /// Match passage using metadata only (no fingerprint)
    fn match_passage_metadata_only(
        &self,
        passage: &PassageBoundary,
        metadata: &MergedMetadata,
    ) -> PassageSongMatch {
        // Metadata-only matching has Low confidence at best
        // If metadata is present, assign Low confidence
        // If no metadata, treat as zero-song passage
        if metadata.artist.is_some() || metadata.title.is_some() {
            PassageSongMatch {
                passage: passage.clone(),
                mbid: None, // No MBID without fingerprint lookup
                confidence: ConfidenceLevel::Low,
                score: 0.65, // Low confidence (0.60-0.70 range)
                title: metadata.title.clone(),
            }
        } else {
            PassageSongMatch {
                passage: passage.clone(),
                mbid: None,
                confidence: ConfidenceLevel::None,
                score: 0.0,
                title: None,
            }
        }
    }

    /// Calculate metadata match score (fuzzy string comparison)
    fn calculate_metadata_score(
        &self,
        candidate_title: &Option<String>,
        _file_artist: &Option<String>,
        file_title: &Option<String>,
    ) -> f32 {
        // Simple heuristic: if title matches, score = 0.8, else 0.3
        // In production, use fuzzy string matching (e.g., Levenshtein distance)
        if let (Some(cand), Some(file)) = (candidate_title, file_title) {
            if cand.to_lowercase() == file.to_lowercase() {
                0.8
            } else {
                0.3 // Partial match or no match
            }
        } else {
            0.3 // Missing metadata
        }
    }

    /// Merge adjacent zero-song passages
    fn merge_zero_song_passages(&self, matches: Vec<PassageSongMatch>) -> Vec<PassageSongMatch> {
        if matches.is_empty() {
            return matches;
        }

        let original_count = matches.len();
        let mut merged = Vec::new();
        let mut current_zero_song: Option<PassageSongMatch> = None;

        for match_item in matches {
            if match_item.confidence == ConfidenceLevel::None {
                // Accumulate zero-song passage
                if let Some(ref mut current) = current_zero_song {
                    // Extend end of current zero-song passage
                    current.passage.end_ticks = match_item.passage.end_ticks;
                } else {
                    // Start new zero-song passage
                    current_zero_song = Some(match_item);
                }
            } else {
                // Non-zero-song passage: flush accumulated zero-song if any
                if let Some(zero_song) = current_zero_song.take() {
                    merged.push(zero_song);
                }
                merged.push(match_item);
            }
        }

        // Flush final zero-song passage if any
        if let Some(zero_song) = current_zero_song {
            merged.push(zero_song);
        }

        tracing::debug!(
            original_count,
            merged_count = merged.len(),
            "Merged adjacent zero-song passages"
        );

        merged
    }

    /// Calculate statistics
    fn calculate_stats(&self, matches: &[PassageSongMatch]) -> SongMatchStats {
        let mut stats = SongMatchStats {
            total_passages: matches.len(),
            high_confidence: 0,
            medium_confidence: 0,
            low_confidence: 0,
            zero_song: 0,
        };

        for match_item in matches {
            match match_item.confidence {
                ConfidenceLevel::High => stats.high_confidence += 1,
                ConfidenceLevel::Medium => stats.medium_confidence += 1,
                ConfidenceLevel::Low => stats.low_confidence += 1,
                ConfidenceLevel::None => stats.zero_song += 1,
            }
        }

        stats
    }
}

impl Default for PassageSongMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::passage_fingerprinter::MBIDCandidate;

    #[test]
    fn test_confidence_level_from_score() {
        assert_eq!(ConfidenceLevel::from_score(0.95), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(0.85), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(0.75), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.65), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(0.50), ConfidenceLevel::None);
    }

    #[test]
    fn test_matcher_creation() {
        let _matcher = PassageSongMatcher::new();
        // Just verify it can be created
    }

    #[test]
    fn test_merge_zero_song_passages() {
        let matcher = PassageSongMatcher::new();

        let passages = vec![
            PassageSongMatch {
                passage: PassageBoundary::new(0, 1000),
                mbid: Some("mbid1".to_string()),
                confidence: ConfidenceLevel::High,
                score: 0.9,
                title: Some("Song 1".to_string()),
            },
            PassageSongMatch {
                passage: PassageBoundary::new(1000, 2000),
                mbid: None,
                confidence: ConfidenceLevel::None,
                score: 0.0,
                title: None,
            },
            PassageSongMatch {
                passage: PassageBoundary::new(2000, 3000),
                mbid: None,
                confidence: ConfidenceLevel::None,
                score: 0.0,
                title: None,
            },
            PassageSongMatch {
                passage: PassageBoundary::new(3000, 4000),
                mbid: Some("mbid2".to_string()),
                confidence: ConfidenceLevel::Medium,
                score: 0.75,
                title: Some("Song 2".to_string()),
            },
        ];

        let merged = matcher.merge_zero_song_passages(passages);

        // Should have 3 passages: Song 1, merged zero-song (1000-3000), Song 2
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].mbid, Some("mbid1".to_string()));
        assert_eq!(merged[1].mbid, None);
        assert_eq!(merged[1].passage.start_ticks, 1000);
        assert_eq!(merged[1].passage.end_ticks, 3000); // Merged
        assert_eq!(merged[2].mbid, Some("mbid2".to_string()));
    }

    #[test]
    fn test_calculate_stats() {
        let matcher = PassageSongMatcher::new();

        let matches = vec![
            PassageSongMatch {
                passage: PassageBoundary::new(0, 1000),
                mbid: Some("mbid1".to_string()),
                confidence: ConfidenceLevel::High,
                score: 0.9,
                title: Some("Song 1".to_string()),
            },
            PassageSongMatch {
                passage: PassageBoundary::new(1000, 2000),
                mbid: Some("mbid2".to_string()),
                confidence: ConfidenceLevel::Medium,
                score: 0.75,
                title: Some("Song 2".to_string()),
            },
            PassageSongMatch {
                passage: PassageBoundary::new(2000, 3000),
                mbid: Some("mbid3".to_string()),
                confidence: ConfidenceLevel::Low,
                score: 0.65,
                title: Some("Song 3".to_string()),
            },
            PassageSongMatch {
                passage: PassageBoundary::new(3000, 4000),
                mbid: None,
                confidence: ConfidenceLevel::None,
                score: 0.0,
                title: None,
            },
        ];

        let stats = matcher.calculate_stats(&matches);

        assert_eq!(stats.total_passages, 4);
        assert_eq!(stats.high_confidence, 1);
        assert_eq!(stats.medium_confidence, 1);
        assert_eq!(stats.low_confidence, 1);
        assert_eq!(stats.zero_song, 1);
    }
}
