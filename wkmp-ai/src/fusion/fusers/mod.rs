// Tier 2 Fusers - Confidence-Weighted Fusion
//
// PLAN023: REQ-AI-020 series - Multi-source data fusion with Bayesian updates and weighted averaging
// 4 fusers: Identity Resolver, Metadata Fuser, Flavor Synthesizer, Boundary Fuser

pub mod identity_resolver;
pub mod metadata_fuser;
pub mod flavor_synthesizer;
// pub mod boundary_fuser; // Deferred (uses simple silence detection baseline)

use crate::fusion::{ExtractionResult, FusionResult};
use anyhow::Result;

/// Fuse multiple extraction results into single consensus result
///
/// # Arguments
/// * `extractions` - Results from all Tier 1 extractors
///
/// # Returns
/// * `FusionResult` with fused identity, metadata, and musical flavor
pub async fn fuse_extractions(extractions: Vec<ExtractionResult>) -> Result<FusionResult> {
    use tracing::{debug, info};

    debug!("Starting fusion pipeline with {} extractions", extractions.len());

    // 1. Identity resolution (Bayesian update)
    let identity = identity_resolver::resolve_identity(&extractions)?;
    info!(
        "Identity resolved: MBID={:?}, confidence={:.3}",
        identity.recording_mbid, identity.confidence
    );

    // 2. Metadata fusion (weighted selection)
    let metadata = metadata_fuser::fuse_metadata(&extractions)?;
    info!(
        "Metadata fused: title={:?}, artist={:?}, completeness={:.1}%",
        metadata.title, metadata.artist, metadata.completeness * 100.0
    );

    // 3. Musical flavor synthesis (characteristic-wise averaging)
    let flavor = flavor_synthesizer::synthesize_flavor(&extractions)?;
    info!(
        "Flavor synthesized: {} characteristics, completeness={:.1}%",
        flavor.characteristics.len(),
        flavor.completeness * 100.0
    );

    Ok(FusionResult {
        identity,
        metadata,
        flavor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fusion_empty_input() {
        // Empty extraction list should still produce valid fusion result
        // (identity resolver, metadata fuser, and flavor synthesizer all handle empty input)
        let result = fuse_extractions(vec![]).await;
        assert!(result.is_ok(), "Fusion should handle empty input gracefully");

        let fusion = result.unwrap();
        assert!(fusion.identity.recording_mbid.is_none(), "No MBID with no input");
        assert!(fusion.metadata.title.is_none(), "No title with no input");
    }
}
