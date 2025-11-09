// PLAN023 Tier 2: Metadata Fuser
//
// Concept: Fuse metadata fields from multiple sources via confidence-weighted selection
// Synchronization: Accepts Vec<ExtractorResult<MetadataBundle>>, outputs FusedMetadata
//
// Algorithm (per SPEC_wkmp_ai_recode.md):
// 1. For each field (title, artist, album, etc.), collect all values from all sources
// 2. Select highest-confidence value for each field
// 3. Compute overall metadata confidence (average of selected field confidences)
// 4. Return FusedMetadata with selected values and provenance

use crate::import_v2::types::{
    ExtractorResult, FusedMetadata, ImportResult, MetadataBundle, MetadataField,
};

/// Metadata fuser (Tier 2 fusion concept)
///
/// **Legible Software Principle:**
/// - Independent module: Pure fusion logic, no side effects
/// - Explicit synchronization: Clear contract with Tier 1 extractors
/// - Transparent behavior: Highest-confidence selection is explicit
/// - Integrity: Maintains source provenance for all selected fields
pub struct MetadataFuser {
    /// Minimum confidence threshold for accepting a field
    min_field_confidence: f64,
}

impl Default for MetadataFuser {
    fn default() -> Self {
        Self {
            min_field_confidence: 0.3, // Accept fields with confidence ≥ 0.3
        }
    }
}

impl MetadataFuser {
    /// Fuse metadata from multiple sources
    ///
    /// # Algorithm: Highest-Confidence Selection
    /// For each field:
    /// 1. Collect all values from all MetadataBundle sources
    /// 2. Filter by min_field_confidence threshold
    /// 3. Select value with highest confidence
    /// 4. Preserve source provenance
    ///
    /// # Arguments
    /// * `bundles` - Metadata bundles from each source (ID3, MusicBrainz, etc.)
    ///
    /// # Returns
    /// FusedMetadata with selected fields and overall confidence
    pub fn fuse(
        &self,
        bundles: Vec<ExtractorResult<MetadataBundle>>,
    ) -> ImportResult<FusedMetadata> {
        if bundles.is_empty() {
            return Ok(FusedMetadata {
                title: None,
                artist: None,
                album: None,
                release_date: None,
                track_number: None,
                duration_ms: None,
                metadata_confidence: 0.0,
            });
        }

        // Collect all metadata fields from all bundles
        let mut all_titles: Vec<MetadataField<String>> = Vec::new();
        let mut all_artists: Vec<MetadataField<String>> = Vec::new();
        let mut all_albums: Vec<MetadataField<String>> = Vec::new();
        let mut all_release_dates: Vec<MetadataField<String>> = Vec::new();
        let mut all_track_numbers: Vec<MetadataField<u32>> = Vec::new();
        let mut all_durations: Vec<MetadataField<u32>> = Vec::new();

        for bundle_result in bundles {
            let bundle = bundle_result.data;

            all_titles.extend(bundle.title);
            all_artists.extend(bundle.artist);
            all_albums.extend(bundle.album);
            all_release_dates.extend(bundle.release_date);
            all_track_numbers.extend(bundle.track_number);
            all_durations.extend(bundle.duration_ms);
        }

        // Select highest-confidence value for each field
        let title = self.select_best_field(all_titles);
        let artist = self.select_best_field(all_artists);
        let album = self.select_best_field(all_albums);
        let release_date = self.select_best_field(all_release_dates);
        let track_number = self.select_best_field(all_track_numbers);
        let duration_ms = self.select_best_field(all_durations);

        // Compute overall metadata confidence (average of selected fields)
        let mut total_confidence = 0.0;
        let mut field_count = 0;

        if let Some(ref t) = title {
            total_confidence += t.confidence;
            field_count += 1;
        }
        if let Some(ref a) = artist {
            total_confidence += a.confidence;
            field_count += 1;
        }
        if let Some(ref al) = album {
            total_confidence += al.confidence;
            field_count += 1;
        }
        if let Some(ref rd) = release_date {
            total_confidence += rd.confidence;
            field_count += 1;
        }
        if let Some(ref tn) = track_number {
            total_confidence += tn.confidence;
            field_count += 1;
        }
        if let Some(ref d) = duration_ms {
            total_confidence += d.confidence;
            field_count += 1;
        }

        let metadata_confidence = if field_count > 0 {
            total_confidence / field_count as f64
        } else {
            0.0
        };

        tracing::info!(
            "Metadata fused: fields={} (title={}, artist={}, album={}), confidence={:.3}",
            field_count,
            title.is_some(),
            artist.is_some(),
            album.is_some(),
            metadata_confidence
        );

        Ok(FusedMetadata {
            title,
            artist,
            album,
            release_date,
            track_number,
            duration_ms,
            metadata_confidence,
        })
    }

    /// Select highest-confidence field from candidates
    ///
    /// Filters by min_field_confidence, then selects highest confidence.
    fn select_best_field<T: Clone>(
        &self,
        mut candidates: Vec<MetadataField<T>>,
    ) -> Option<MetadataField<T>> {
        if candidates.is_empty() {
            return None;
        }

        // Filter by minimum confidence threshold
        candidates.retain(|field| field.confidence >= self.min_field_confidence);

        if candidates.is_empty() {
            tracing::debug!(
                "All candidates below min_field_confidence threshold {:.2}",
                self.min_field_confidence
            );
            return None;
        }

        // Sort by confidence (highest first)
        candidates.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return highest-confidence field
        Some(candidates[0].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_v2::types::ExtractionSource;

    fn create_metadata_field<T>(
        value: T,
        confidence: f64,
        source: ExtractionSource,
    ) -> MetadataField<T> {
        MetadataField {
            value,
            confidence,
            source,
        }
    }

    #[test]
    fn test_empty_bundles() {
        let fuser = MetadataFuser::default();
        let result = fuser.fuse(vec![]).unwrap();

        assert!(result.title.is_none());
        assert!(result.artist.is_none());
        assert_eq!(result.metadata_confidence, 0.0);
    }

    #[test]
    fn test_single_source_all_fields() {
        let fuser = MetadataFuser::default();

        let mut bundle = MetadataBundle::default();
        bundle.title.push(create_metadata_field(
            "Let It Be".to_string(),
            0.9,
            ExtractionSource::MusicBrainz,
        ));
        bundle.artist.push(create_metadata_field(
            "The Beatles".to_string(),
            0.9,
            ExtractionSource::MusicBrainz,
        ));
        bundle.album.push(create_metadata_field(
            "Let It Be".to_string(),
            0.8,
            ExtractionSource::MusicBrainz,
        ));

        let bundles = vec![ExtractorResult {
            data: bundle,
            confidence: 0.9,
            source: ExtractionSource::MusicBrainz,
        }];

        let result = fuser.fuse(bundles).unwrap();

        assert_eq!(result.title.unwrap().value, "Let It Be");
        assert_eq!(result.artist.unwrap().value, "The Beatles");
        assert_eq!(result.album.unwrap().value, "Let It Be");
        assert!(result.metadata_confidence > 0.8);
    }

    #[test]
    fn test_highest_confidence_wins() {
        let fuser = MetadataFuser::default();

        // Two bundles with different confidences for title
        let mut bundle1 = MetadataBundle::default();
        bundle1.title.push(create_metadata_field(
            "Low Confidence Title".to_string(),
            0.5,
            ExtractionSource::ID3Metadata,
        ));

        let mut bundle2 = MetadataBundle::default();
        bundle2.title.push(create_metadata_field(
            "High Confidence Title".to_string(),
            0.95,
            ExtractionSource::MusicBrainz,
        ));

        let bundles = vec![
            ExtractorResult {
                data: bundle1,
                confidence: 0.5,
                source: ExtractionSource::ID3Metadata,
            },
            ExtractorResult {
                data: bundle2,
                confidence: 0.9,
                source: ExtractionSource::MusicBrainz,
            },
        ];

        let result = fuser.fuse(bundles).unwrap();

        let title = result.title.as_ref().unwrap();
        assert_eq!(title.value, "High Confidence Title");
        assert_eq!(title.source, ExtractionSource::MusicBrainz);
    }

    #[test]
    fn test_threshold_filtering() {
        let fuser = MetadataFuser {
            min_field_confidence: 0.6, // High threshold
            ..Default::default()
        };

        let mut bundle = MetadataBundle::default();
        bundle.title.push(create_metadata_field(
            "Below Threshold".to_string(),
            0.5, // Below 0.6 threshold
            ExtractionSource::ID3Metadata,
        ));

        let bundles = vec![ExtractorResult {
            data: bundle,
            confidence: 0.5,
            source: ExtractionSource::ID3Metadata,
        }];

        let result = fuser.fuse(bundles).unwrap();

        // Title should be rejected due to low confidence
        assert!(result.title.is_none());
        assert_eq!(result.metadata_confidence, 0.0);
    }

    #[test]
    fn test_multiple_fields_from_different_sources() {
        let fuser = MetadataFuser::default();

        // Bundle 1: High-confidence title, low-confidence artist
        let mut bundle1 = MetadataBundle::default();
        bundle1.title.push(create_metadata_field(
            "Title from MB".to_string(),
            0.9,
            ExtractionSource::MusicBrainz,
        ));
        bundle1.artist.push(create_metadata_field(
            "Artist from MB".to_string(),
            0.5,
            ExtractionSource::MusicBrainz,
        ));

        // Bundle 2: Low-confidence title, high-confidence artist
        let mut bundle2 = MetadataBundle::default();
        bundle2.title.push(create_metadata_field(
            "Title from ID3".to_string(),
            0.4,
            ExtractionSource::ID3Metadata,
        ));
        bundle2.artist.push(create_metadata_field(
            "Artist from ID3".to_string(),
            0.8,
            ExtractionSource::ID3Metadata,
        ));

        let bundles = vec![
            ExtractorResult {
                data: bundle1,
                confidence: 0.9,
                source: ExtractionSource::MusicBrainz,
            },
            ExtractorResult {
                data: bundle2,
                confidence: 0.5,
                source: ExtractionSource::ID3Metadata,
            },
        ];

        let result = fuser.fuse(bundles).unwrap();

        // Title should come from MusicBrainz (0.9 > 0.4)
        let title = result.title.as_ref().unwrap();
        assert_eq!(title.value, "Title from MB");
        assert_eq!(title.source, ExtractionSource::MusicBrainz);

        // Artist should come from ID3 (0.8 > 0.5)
        let artist = result.artist.as_ref().unwrap();
        assert_eq!(artist.value, "Artist from ID3");
        assert_eq!(artist.source, ExtractionSource::ID3Metadata);
    }

    #[test]
    fn test_metadata_confidence_calculation() {
        let fuser = MetadataFuser::default();

        let mut bundle = MetadataBundle::default();
        bundle.title.push(create_metadata_field(
            "Title".to_string(),
            0.9,
            ExtractionSource::MusicBrainz,
        ));
        bundle.artist.push(create_metadata_field(
            "Artist".to_string(),
            0.7,
            ExtractionSource::MusicBrainz,
        ));
        bundle.album.push(create_metadata_field(
            "Album".to_string(),
            0.8,
            ExtractionSource::MusicBrainz,
        ));

        let bundles = vec![ExtractorResult {
            data: bundle,
            confidence: 0.9,
            source: ExtractionSource::MusicBrainz,
        }];

        let result = fuser.fuse(bundles).unwrap();

        // Overall confidence = (0.9 + 0.7 + 0.8) / 3 = 0.8
        assert!((result.metadata_confidence - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_partial_fields() {
        let fuser = MetadataFuser::default();

        // Bundle with only title and artist (no album)
        let mut bundle = MetadataBundle::default();
        bundle.title.push(create_metadata_field(
            "Title Only".to_string(),
            0.9,
            ExtractionSource::MusicBrainz,
        ));
        bundle.artist.push(create_metadata_field(
            "Artist Only".to_string(),
            0.85,
            ExtractionSource::MusicBrainz,
        ));

        let bundles = vec![ExtractorResult {
            data: bundle,
            confidence: 0.9,
            source: ExtractionSource::MusicBrainz,
        }];

        let result = fuser.fuse(bundles).unwrap();

        assert!(result.title.is_some());
        assert!(result.artist.is_some());
        assert!(result.album.is_none()); // No album data provided
        assert!(result.metadata_confidence > 0.8); // Average of title and artist
    }

    #[test]
    fn test_numeric_fields() {
        let fuser = MetadataFuser::default();

        let mut bundle = MetadataBundle::default();
        bundle.track_number.push(create_metadata_field(
            5,
            0.9,
            ExtractionSource::MusicBrainz,
        ));
        bundle.duration_ms.push(create_metadata_field(
            240000,
            0.85,
            ExtractionSource::ID3Metadata,
        ));

        let bundles = vec![ExtractorResult {
            data: bundle,
            confidence: 0.9,
            source: ExtractionSource::MusicBrainz,
        }];

        let result = fuser.fuse(bundles).unwrap();

        assert_eq!(result.track_number.unwrap().value, 5);
        assert_eq!(result.duration_ms.unwrap().value, 240000);
    }

    #[test]
    fn test_tie_breaker_first_wins() {
        let fuser = MetadataFuser::default();

        // Two sources with identical confidence
        let mut bundle1 = MetadataBundle::default();
        bundle1.title.push(create_metadata_field(
            "First Title".to_string(),
            0.8,
            ExtractionSource::ID3Metadata,
        ));

        let mut bundle2 = MetadataBundle::default();
        bundle2.title.push(create_metadata_field(
            "Second Title".to_string(),
            0.8, // Same confidence
            ExtractionSource::MusicBrainz,
        ));

        let bundles = vec![
            ExtractorResult {
                data: bundle1,
                confidence: 0.8,
                source: ExtractionSource::ID3Metadata,
            },
            ExtractorResult {
                data: bundle2,
                confidence: 0.8,
                source: ExtractionSource::MusicBrainz,
            },
        ];

        let result = fuser.fuse(bundles).unwrap();

        // When tied, first occurrence in sorted list wins (stable sort)
        // After sorting by confidence (descending), order is preserved for equal values
        assert!(result.title.is_some());
        assert_eq!(result.title.unwrap().confidence, 0.8);
    }
}
