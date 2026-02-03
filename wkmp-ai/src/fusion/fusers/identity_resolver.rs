// Identity Resolver - Bayesian MBID Resolution
//
// PLAN023: REQ-AI-020 series - Multi-source identity resolution with Bayesian updates
// Confidence formula: posterior = 1 - (1 - prior) * (1 - new_evidence)

use crate::fusion::{ExtractionResult, FusedIdentity, ConflictReport, Confidence};
use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Resolve Recording MBID from multiple sources using Bayesian update
///
/// # Arguments
/// * `extractions` - Results from extractors with identity information
///
/// # Returns
/// * `FusedIdentity` with posterior confidence and conflict reports
pub fn resolve_identity(extractions: &[ExtractionResult]) -> Result<FusedIdentity> {
    // Filter extractions with identity information
    let identity_extractions: Vec<_> = extractions
        .iter()
        .filter(|e| e.identity.is_some())
        .collect();

    if identity_extractions.is_empty() {
        debug!("No identity information found in extractions");
        return Ok(FusedIdentity {
            recording_mbid: None,
            confidence: 0.0,
            conflicts: vec![],
        });
    }

    // Group by MBID
    let mut mbid_groups: HashMap<String, Vec<&ExtractionResult>> = HashMap::new();
    for extraction in &identity_extractions {
        if let Some(identity) = &extraction.identity {
            mbid_groups
                .entry(identity.recording_mbid.clone())
                .or_default()
                .push(extraction);
        }
    }

    debug!("Found {} unique MBIDs", mbid_groups.len());

    // Single MBID case - apply Bayesian update
    if mbid_groups.len() == 1 {
        // Safe: we just verified len == 1, so next() cannot be None
        let (mbid, sources) = mbid_groups.into_iter().next().unwrap();
        let posterior = calculate_bayesian_posterior(sources);

        debug!(
            "Single MBID consensus: {} (posterior confidence: {:.3})",
            mbid, posterior
        );

        return Ok(FusedIdentity {
            recording_mbid: Some(mbid),
            confidence: posterior,
            conflicts: vec![],
        });
    }

    // Multiple MBIDs - detect conflicts
    let mut conflicts = Vec::new();
    let mut best_mbid: Option<String> = None;
    let mut best_confidence = 0.0;

    // Calculate posterior for each MBID group
    for (mbid, sources) in mbid_groups {
        let posterior = calculate_bayesian_posterior(sources.clone());

        if posterior > best_confidence {
            best_confidence = posterior;
            best_mbid = Some(mbid.clone());
        }

        // Check for conflicts with other MBIDs
        for other_extraction in &identity_extractions {
            if let Some(other_identity) = &other_extraction.identity {
                if other_identity.recording_mbid != mbid {
                    // Check title similarity if both have metadata
                    let similarity = calculate_title_similarity(
                        sources.first().and_then(|e| e.metadata.as_ref()),
                        other_extraction.metadata.as_ref(),
                    );

                    if similarity.is_some() {
                        conflicts.push(ConflictReport {
                            field: "recording_mbid".to_string(),
                            // Safe: sources is non-empty (came from grouping extractions by MBID)
                            source1: sources.first().unwrap().source.clone(),
                            value1: mbid.clone(),
                            source2: other_extraction.source.clone(),
                            value2: other_identity.recording_mbid.clone(),
                            similarity,
                        });
                    }
                }
            }
        }
    }

    // Deduplicate conflicts
    conflicts.sort_by(|a, b| {
        format!("{}{}", a.value1, a.value2).cmp(&format!("{}{}", b.value1, b.value2))
    });
    conflicts.dedup_by(|a, b| a.value1 == b.value1 && a.value2 == b.value2);

    if !conflicts.is_empty() {
        warn!(
            "Identity conflicts detected: {} different MBIDs",
            conflicts.len()
        );
    }

    Ok(FusedIdentity {
        recording_mbid: best_mbid,
        confidence: best_confidence,
        conflicts,
    })
}

/// Calculate Bayesian posterior confidence from multiple sources
fn calculate_bayesian_posterior(sources: Vec<&ExtractionResult>) -> Confidence {
    if sources.is_empty() {
        return 0.0;
    }

    // Start with first source's confidence as prior
    // Note: sources is guaranteed non-empty (checked above) and comes from identity extractions,
    // so identity field must exist
    let mut posterior = sources[0]
        .identity
        .as_ref()
        .map(|id| id.confidence)
        .unwrap_or(0.0);

    // Apply Bayesian update for each additional source
    for source in sources.iter().skip(1) {
        if let Some(identity) = &source.identity {
            posterior = bayesian_update(posterior, identity.confidence);
        }
    }

    posterior
}

/// Calculate title similarity between two metadata sources
fn calculate_title_similarity(
    metadata1: Option<&crate::fusion::MetadataExtraction>,
    metadata2: Option<&crate::fusion::MetadataExtraction>,
) -> Option<f64> {
    let title1 = metadata1?.title.as_ref()?;
    let title2 = metadata2?.title.as_ref()?;

    Some(strsim::normalized_levenshtein(title1, title2))
}

/// Bayesian update formula (REQ-AI-023)
///
/// # Arguments
/// * `prior` - Prior confidence (0.0-1.0)
/// * `evidence` - New evidence confidence (0.0-1.0)
///
/// # Returns
/// * Posterior confidence (0.0-1.0)
pub fn bayesian_update(prior: Confidence, evidence: Confidence) -> Confidence {
    1.0 - (1.0 - prior) * (1.0 - evidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bayesian_update_agreement() {
        // REQ-AI-023: Test case from TC-U-023-01
        let prior = 0.9;
        let evidence = 0.9;
        let posterior = bayesian_update(prior, evidence);
        let expected = 1.0 - (1.0 - 0.9) * (1.0 - 0.9); // = 0.99
        assert!((posterior - expected).abs() < 0.0001);
    }

    #[test]
    fn test_bayesian_update_strengthens_confidence() {
        let prior = 0.7;
        let evidence = 0.8;
        let posterior = bayesian_update(prior, evidence);
        assert!(posterior > prior, "Posterior should be stronger than prior");
        assert!(posterior > evidence, "Posterior should be stronger than evidence");
    }

    #[test]
    fn test_bayesian_update_bounds() {
        // Posterior should never exceed 1.0
        let posterior = bayesian_update(0.99, 0.99);
        assert!(posterior <= 1.0);
        assert!(posterior >= 0.0);
    }
}
