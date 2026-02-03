// Metadata Fuser - Field-Wise Weighted Selection
//
// PLAN023: REQ-AI-030 series - Multi-source metadata fusion with quality scoring

use crate::fusion::{ExtractionResult, FusedMetadata, ConflictReport, Confidence};
use anyhow::Result;
use tracing::{debug, warn};

/// Fuse metadata from multiple sources using weighted selection
///
/// # Arguments
/// * `extractions` - Results from extractors with metadata
///
/// # Returns
/// * `FusedMetadata` with selected values, source provenance, and conflicts
pub fn fuse_metadata(extractions: &[ExtractionResult]) -> Result<FusedMetadata> {
    let metadata_extractions: Vec<_> = extractions
        .iter()
        .filter(|e| e.metadata.is_some())
        .collect();

    if metadata_extractions.is_empty() {
        debug!("No metadata found in extractions");
        return Ok(FusedMetadata {
            title: None,
            title_source: None,
            title_confidence: None,
            artist: None,
            artist_source: None,
            artist_confidence: None,
            album: None,
            completeness: 0.0,
            conflicts: vec![],
        });
    }

    let mut conflicts = Vec::new();

    // Fuse title
    let (title, title_source, title_confidence, title_conflicts) =
        fuse_field(&metadata_extractions, |m| m.title.as_ref(), "title");
    conflicts.extend(title_conflicts);

    // Fuse artist
    let (artist, artist_source, artist_confidence, artist_conflicts) =
        fuse_field(&metadata_extractions, |m| m.artist.as_ref(), "artist");
    conflicts.extend(artist_conflicts);

    // Fuse album (no confidence tracking)
    let album = metadata_extractions
        .iter()
        .filter_map(|e| e.metadata.as_ref()?.album.as_ref())
        .next()
        .cloned();

    // Calculate completeness (3 fields: title, artist, album)
    let mut filled_fields = 0;
    if title.is_some() {
        filled_fields += 1;
    }
    if artist.is_some() {
        filled_fields += 1;
    }
    if album.is_some() {
        filled_fields += 1;
    }
    let completeness = filled_fields as f64 / 3.0;

    debug!(
        "Metadata fusion complete: {} fields filled ({:.1}%), {} conflicts",
        filled_fields,
        completeness * 100.0,
        conflicts.len()
    );

    Ok(FusedMetadata {
        title,
        title_source,
        title_confidence,
        artist,
        artist_source,
        artist_confidence,
        album,
        completeness,
        conflicts,
    })
}

/// Fuse a single metadata field from multiple sources
///
/// Returns: (value, source, confidence, conflicts)
fn fuse_field<F>(
    extractions: &[&ExtractionResult],
    field_accessor: F,
    field_name: &str,
) -> (
    Option<String>,
    Option<String>,
    Option<Confidence>,
    Vec<ConflictReport>,
)
where
    F: Fn(&crate::fusion::MetadataExtraction) -> Option<&String>,
{
    // Collect all non-empty values with sources and confidence
    let values: Vec<_> = extractions
        .iter()
        .filter_map(|e| {
            let metadata = e.metadata.as_ref()?;
            let value = field_accessor(metadata)?;
            let confidence = e.confidence;
            Some((value.clone(), e.source.clone(), confidence))
        })
        .collect();

    if values.is_empty() {
        return (None, None, None, vec![]);
    }

    // If only one value, use it
    if values.len() == 1 {
        let (value, source, confidence) = &values[0];
        return (
            Some(value.clone()),
            Some(source.clone()),
            Some(*confidence),
            vec![],
        );
    }

    // Check if all values match
    let first_value = &values[0].0;
    let all_match = values.iter().all(|(v, _, _)| v == first_value);

    if all_match {
        // All values match - select source with highest confidence
        let best = values
            .iter()
            .max_by(|a, b| {
                a.2.partial_cmp(&b.2)
                    .unwrap_or(std::cmp::Ordering::Equal) // Treat NaN as equal
            })
            .expect("values is non-empty (we have at least one extraction)");
        return (
            Some(best.0.clone()),
            Some(best.1.clone()),
            Some(best.2),
            vec![],
        );
    }

    // Values differ - detect conflicts
    let mut conflicts = Vec::new();
    for i in 0..values.len() {
        for j in (i + 1)..values.len() {
            let (value1, source1, _) = &values[i];
            let (value2, source2, _) = &values[j];

            if value1 != value2 {
                let similarity = strsim::normalized_levenshtein(value1, value2);
                conflicts.push(ConflictReport {
                    field: field_name.to_string(),
                    source1: source1.clone(),
                    value1: value1.clone(),
                    source2: source2.clone(),
                    value2: value2.clone(),
                    similarity: Some(similarity),
                });
            }
        }
    }

    // Select value with highest confidence despite conflicts
    let best = values
        .iter()
        .max_by(|a, b| {
            a.2.partial_cmp(&b.2)
                .unwrap_or(std::cmp::Ordering::Equal) // Treat NaN as equal
        })
        .expect("values is non-empty (we have at least one extraction)");

    if !conflicts.is_empty() {
        warn!(
            "Conflicts detected for field '{}': {} different values",
            field_name,
            values.len()
        );
    }

    (
        Some(best.0.clone()),
        Some(best.1.clone()),
        Some(best.2),
        conflicts,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_fuser_empty() {
        let result = fuse_metadata(&[]).unwrap();
        assert!(result.title.is_none());
        assert_eq!(result.completeness, 0.0);
    }

    #[test]
    fn test_completeness_calculation() {
        // Create test extraction with all fields
        let extraction = ExtractionResult {
            source: "Test".to_string(),
            confidence: 0.9,
            timestamp: 0,
            metadata: Some(crate::fusion::MetadataExtraction {
                title: Some("Song".to_string()),
                artist: Some("Artist".to_string()),
                album: Some("Album".to_string()),
                duration_seconds: None,
                title_confidence: Some(0.9),
                artist_confidence: Some(0.9),
            }),
            flavor: None,
            identity: None,
        };

        let result = fuse_metadata(&[extraction]).unwrap();
        assert_eq!(result.completeness, 1.0); // 3/3 fields filled
    }
}

