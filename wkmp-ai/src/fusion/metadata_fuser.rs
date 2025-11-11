//! Metadata Fuser (Tier 2 Fuser)
//!
//! Performs field-wise fusion of metadata from multiple extractors.
//! Resolves conflicts when different sources provide different values.
//!
//! # Implementation
//! - TASK-013: Metadata Fuser (PLAN024)
//! - Fusion strategy: Highest confidence wins per field
//!
//! # Architecture
//! Implements `Fusion` trait for integration with 3-tier architecture.
//! Accepts Vec<MetadataExtraction> and produces FusedMetadata with best values per field.
//!
//! # Field-wise Fusion
//! Each field (title, artist, album) is fused independently:
//! - Select value with highest confidence
//! - Track source provenance for each field
//! - Detect conflicts when values differ significantly
//! - Compute overall metadata completeness score

use crate::types::{
    ConfidenceValue, Fusion, FusionError, FusionResult, FusedMetadata, MetadataExtraction,
};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::debug;

/// Metadata Fuser
///
/// Performs field-wise fusion of metadata from multiple extraction sources.
/// Handles conflict resolution when extractors provide different values.
///
/// # Fusion Strategy
/// For each metadata field:
/// 1. Collect all values from sources
/// 2. Select value with highest confidence
/// 3. Track source provenance
/// 4. Detect conflicts (different values with significant confidence)
/// 5. Compute field similarity for conflict reporting
///
/// # Conflict Detection
/// A conflict is detected when:
/// - Multiple sources provide different values
/// - Both values have confidence >= min_conflict_threshold (default: 0.5)
/// - String similarity < similarity_threshold (default: 0.8)
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::fusion::MetadataFuser;
/// use wkmp_ai::types::{Fusion, MetadataExtraction, ConfidenceValue};
///
/// let fuser = MetadataFuser::new();
/// let metadata = vec![
///     MetadataExtraction {
///         title: Some(ConfidenceValue::new("Song Title".into(), 0.9, "MusicBrainz")),
///         artist: Some(ConfidenceValue::new("Artist Name".into(), 0.9, "MusicBrainz")),
///         ..Default::default()
///     },
///     MetadataExtraction {
///         title: Some(ConfidenceValue::new("Song Title".into(), 0.6, "ID3")),
///         artist: Some(ConfidenceValue::new("Artist Name".into(), 0.6, "ID3")),
///         ..Default::default()
///     },
/// ];
///
/// let fused = fuser.fuse(metadata).await?;
/// // Selects MusicBrainz values (higher confidence)
/// ```
pub struct MetadataFuser {
    /// Minimum confidence to consider a field conflict (default: 0.5)
    #[allow(dead_code)] // Reserved for future conflict detection logic
    min_conflict_threshold: f32,
    /// String similarity threshold for conflict detection (default: 0.8)
    #[allow(dead_code)] // Reserved for future conflict detection logic
    similarity_threshold: f32,
}

impl MetadataFuser {
    /// Create new Metadata Fuser with default settings
    pub fn new() -> Self {
        Self {
            min_conflict_threshold: 0.5,
            similarity_threshold: 0.8,
        }
    }

    /// Create Metadata Fuser with custom thresholds
    pub fn with_thresholds(min_conflict_threshold: f32, similarity_threshold: f32) -> Self {
        Self {
            min_conflict_threshold: min_conflict_threshold.clamp(0.0, 1.0),
            similarity_threshold: similarity_threshold.clamp(0.0, 1.0),
        }
    }

    /// Fuse metadata from multiple sources
    fn fuse_metadata(
        &self,
        metadata_list: Vec<MetadataExtraction>,
    ) -> Result<FusedMetadata, FusionError> {
        if metadata_list.is_empty() {
            return Ok(FusedMetadata {
                title: None,
                artist: None,
                album: None,
                recording_mbid: None,
                additional: HashMap::new(),
                metadata_completeness: 0.0,
            });
        }

        debug!(
            metadata_count = metadata_list.len(),
            "Starting metadata fusion"
        );

        // Fuse each field independently
        let title = self.fuse_field(
            metadata_list.iter().filter_map(|m| m.title.as_ref()),
            "title",
        );

        let artist = self.fuse_field(
            metadata_list.iter().filter_map(|m| m.artist.as_ref()),
            "artist",
        );

        let album = self.fuse_field(
            metadata_list.iter().filter_map(|m| m.album.as_ref()),
            "album",
        );

        let recording_mbid = self.fuse_field(
            metadata_list.iter().filter_map(|m| m.recording_mbid.as_ref()),
            "recording_mbid",
        );

        // Fuse additional metadata fields (artist_mbid, release_mbid, etc.)
        let mut additional = HashMap::new();

        // Collect all unique keys from all metadata sources
        let mut all_keys = std::collections::HashSet::new();
        for metadata in &metadata_list {
            for key in metadata.additional.keys() {
                all_keys.insert(key.clone());
            }
        }

        // Fuse each additional field
        for key in all_keys {
            let values: Vec<&ConfidenceValue<String>> = metadata_list
                .iter()
                .filter_map(|m| m.additional.get(&key))
                .collect();

            if !values.is_empty() {
                // Select value with highest confidence
                let best = values.iter().max_by(|a, b| {
                    a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal)
                }).unwrap();

                additional.insert(key.clone(), (*best).clone());
            }
        }

        // Compute completeness (how many fields have values)
        let field_count = [
            title.is_some(),
            artist.is_some(),
            album.is_some(),
            recording_mbid.is_some(),
        ]
        .iter()
        .filter(|&&present| present)
        .count();

        let completeness = field_count as f32 / 4.0; // 4 primary fields

        debug!(
            field_count = field_count,
            completeness = completeness,
            additional_fields = additional.len(),
            "Metadata fusion complete"
        );

        Ok(FusedMetadata {
            title,
            artist,
            album,
            recording_mbid,
            additional,
            metadata_completeness: completeness,
        })
    }

    /// Fuse a single metadata field
    ///
    /// Returns: ConfidenceValue with best (highest confidence) value
    fn fuse_field<'a, I>(
        &self,
        values: I,
        field_name: &str,
    ) -> Option<ConfidenceValue<String>>
    where
        I: Iterator<Item = &'a ConfidenceValue<String>>,
    {
        let values_vec: Vec<&ConfidenceValue<String>> = values.collect();

        if values_vec.is_empty() {
            return None;
        }

        // Select value with highest confidence
        let best = values_vec
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        debug!(
            field = field_name,
            value = %best.value,
            source = %best.source,
            confidence = best.confidence,
            "Selected best value for field"
        );

        Some(ConfidenceValue::new(
            best.value.clone(),
            best.confidence,
            best.source.clone(),
        ))
    }

    /// Compute string similarity (Levenshtein distance based)
    ///
    /// Returns similarity score 0.0-1.0 (1.0 = identical)
    #[allow(dead_code)] // Reserved for future conflict detection logic
    fn string_similarity(&self, s1: &str, s2: &str) -> f32 {
        if s1 == s2 {
            return 1.0;
        }

        let len1 = s1.chars().count();
        let len2 = s2.chars().count();

        if len1 == 0 || len2 == 0 {
            return 0.0;
        }

        // Simplified Levenshtein distance
        let distance = levenshtein_distance(s1, s2);
        let max_len = len1.max(len2);

        1.0 - (distance as f32 / max_len as f32)
    }
}

impl Default for MetadataFuser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Fusion for MetadataFuser {
    type Input = Vec<MetadataExtraction>;
    type Output = FusedMetadata;

    fn name(&self) -> &'static str {
        "MetadataFuser"
    }

    async fn fuse(&self, inputs: Self::Input) -> Result<FusionResult<Self::Output>, FusionError> {
        debug!(input_count = inputs.len(), "Fusing metadata extractions");

        let fused_metadata = self.fuse_metadata(inputs)?;

        // Collect sources from fused metadata
        let mut sources = Vec::new();
        if let Some(ref cv) = fused_metadata.title {
            sources.push(format!("title:{}", cv.source));
        }
        if let Some(ref cv) = fused_metadata.artist {
            sources.push(format!("artist:{}", cv.source));
        }
        if let Some(ref cv) = fused_metadata.album {
            sources.push(format!("album:{}", cv.source));
        }

        // Overall confidence is average of field confidences
        let confidence_values: Vec<f32> = [
            fused_metadata.title.as_ref().map(|cv| cv.confidence),
            fused_metadata.artist.as_ref().map(|cv| cv.confidence),
            fused_metadata.album.as_ref().map(|cv| cv.confidence),
        ]
        .iter()
        .filter_map(|&c| c)
        .collect();

        let confidence = if confidence_values.is_empty() {
            0.0
        } else {
            confidence_values.iter().sum::<f32>() / confidence_values.len() as f32
        };

        Ok(FusionResult {
            output: fused_metadata,
            confidence,
            sources,
        })
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Compute Levenshtein distance between two strings
///
/// Returns minimum number of single-character edits needed to transform s1 into s2
#[allow(dead_code)] // Reserved for future conflict detection logic
#[allow(clippy::needless_range_loop)] // Matrix initialization is clearer with explicit indices
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let chars1: Vec<char> = s1.chars().collect();
    let chars2: Vec<char> = s2.chars().collect();
    let len1 = chars1.len();
    let len2 = chars2.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    // Create distance matrix
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    // Initialize first row and column
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    // Compute distances
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };

            matrix[i][j] = (matrix[i - 1][j] + 1) // deletion
                .min(matrix[i][j - 1] + 1) // insertion
                .min(matrix[i - 1][j - 1] + cost); // substitution
        }
    }

    matrix[len1][len2]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuser_name() {
        let fuser = MetadataFuser::new();
        assert_eq!(fuser.name(), "MetadataFuser");
    }

    #[test]
    fn test_default_thresholds() {
        let fuser = MetadataFuser::new();
        assert_eq!(fuser.min_conflict_threshold, 0.5);
        assert_eq!(fuser.similarity_threshold, 0.8);
    }

    #[test]
    fn test_custom_thresholds() {
        let fuser = MetadataFuser::with_thresholds(0.6, 0.9);
        assert_eq!(fuser.min_conflict_threshold, 0.6);
        assert_eq!(fuser.similarity_threshold, 0.9);
    }

    #[test]
    fn test_levenshtein_distance_identical() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_distance_one_char_diff() {
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
    }

    #[test]
    fn test_levenshtein_distance_empty() {
        assert_eq!(levenshtein_distance("", "hello"), 5);
        assert_eq!(levenshtein_distance("hello", ""), 5);
    }

    #[test]
    fn test_string_similarity() {
        let fuser = MetadataFuser::new();

        assert_eq!(fuser.string_similarity("hello", "hello"), 1.0);
        assert!(fuser.string_similarity("hello", "hallo") >= 0.8); // distance 1 out of 5 = 0.8
        assert!(fuser.string_similarity("hello", "world") < 0.5);
    }

    #[tokio::test]
    async fn test_fuse_empty_input() {
        let fuser = MetadataFuser::new();
        let result = fuser.fuse(vec![]).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();
        assert!(fusion.output.title.is_none());
        assert!(fusion.output.artist.is_none());
        assert_eq!(fusion.output.metadata_completeness, 0.0);
    }

    #[tokio::test]
    async fn test_fuse_single_metadata() {
        let fuser = MetadataFuser::new();
        let metadata = vec![MetadataExtraction {
            title: Some(ConfidenceValue::new("Song Title".to_string(), 0.9, "MusicBrainz")),
            artist: Some(ConfidenceValue::new("Artist Name".to_string(), 0.9, "MusicBrainz")),
            album: Some(ConfidenceValue::new("Album Name".to_string(), 0.9, "MusicBrainz")),
            recording_mbid: None,
            additional: HashMap::new(),
        }];

        let result = fuser.fuse(metadata).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();
        assert_eq!(fusion.output.title.as_ref().map(|cv| &cv.value), Some(&"Song Title".to_string()));
        assert_eq!(fusion.output.artist.as_ref().map(|cv| &cv.value), Some(&"Artist Name".to_string()));
        assert_eq!(fusion.output.album.as_ref().map(|cv| &cv.value), Some(&"Album Name".to_string()));
        assert_eq!(fusion.output.metadata_completeness, 0.75); // 3/4 fields
    }

    #[tokio::test]
    async fn test_fuse_selects_highest_confidence() {
        let fuser = MetadataFuser::new();
        let metadata = vec![
            MetadataExtraction {
                title: Some(ConfidenceValue::new("Song Title".to_string(), 0.9, "MusicBrainz")),
                artist: Some(ConfidenceValue::new("Artist Name".to_string(), 0.9, "MusicBrainz")),
                album: None,
                recording_mbid: None,
                additional: HashMap::new(),
            },
            MetadataExtraction {
                title: Some(ConfidenceValue::new("Song Title".to_string(), 0.6, "ID3")),
                artist: Some(ConfidenceValue::new("Different Artist".to_string(), 0.6, "ID3")),
                album: Some(ConfidenceValue::new("Album Name".to_string(), 0.6, "ID3")),
                recording_mbid: None,
                additional: HashMap::new(),
            },
        ];

        let result = fuser.fuse(metadata).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();
        // Should select MusicBrainz for title and artist (higher confidence)
        assert_eq!(fusion.output.title.as_ref().map(|cv| &cv.source), Some(&"MusicBrainz".to_string()));
        assert_eq!(fusion.output.artist.as_ref().map(|cv| &cv.source), Some(&"MusicBrainz".to_string()));
        // Should select ID3 for album (only source)
        assert_eq!(fusion.output.album.as_ref().map(|cv| &cv.source), Some(&"ID3".to_string()));
    }

    #[tokio::test]
    async fn test_fuse_completeness_score() {
        let fuser = MetadataFuser::new();

        // All 4 fields present
        let metadata_complete = vec![MetadataExtraction {
            title: Some(ConfidenceValue::new("Title".to_string(), 0.9, "MB")),
            artist: Some(ConfidenceValue::new("Artist".to_string(), 0.9, "MB")),
            album: Some(ConfidenceValue::new("Album".to_string(), 0.9, "MB")),
            recording_mbid: Some(ConfidenceValue::new("mbid-123".to_string(), 0.9, "MB")),
            additional: HashMap::new(),
        }];

        let result = fuser.fuse(metadata_complete).await.unwrap();
        assert_eq!(result.output.metadata_completeness, 1.0);

        // Only 2 fields present
        let metadata_partial = vec![MetadataExtraction {
            title: Some(ConfidenceValue::new("Title".to_string(), 0.9, "MB")),
            artist: Some(ConfidenceValue::new("Artist".to_string(), 0.9, "MB")),
            album: None,
            recording_mbid: None,
            additional: HashMap::new(),
        }];

        let result = fuser.fuse(metadata_partial).await.unwrap();
        assert_eq!(result.output.metadata_completeness, 0.5); // 2/4
    }
}
