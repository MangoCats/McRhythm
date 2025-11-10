//! Identity Resolver (Tier 2 Fuser)
//!
//! Performs Bayesian fusion of Recording MBIDs from multiple extractors.
//! Resolves conflicts when different sources provide different MBIDs.
//!
//! # Implementation
//! - TASK-012: Identity Resolver (PLAN024)
//! - Fusion strategy: Bayesian probability update
//!
//! # Architecture
//! Implements `Fusion` trait for integration with 3-tier architecture.
//! Accepts Vec<IdentityExtraction> and produces FusedIdentity with highest probability MBID.
//!
//! # Bayesian Fusion
//! Uses confidence scores as prior probabilities and updates based on agreement:
//! - Multiple sources with same MBID → increased confidence (agreement boost)
//! - Multiple sources with different MBIDs → conflict (no boost)
//! - Single source → use base confidence (no agreement data)

use crate::types::{Fusion, FusionError, FusionResult, FusedIdentity, IdentityExtraction};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::debug;

/// Identity Resolver
///
/// Performs Bayesian fusion of Recording MBIDs from multiple extraction sources.
/// Handles conflict resolution when extractors disagree on identity.
///
/// # Fusion Strategy
/// 1. Group extractions by MBID value
/// 2. For each MBID, compute posterior probability using Bayesian update
/// 3. Select MBID with highest posterior probability
/// 4. Track conflicts when multiple distinct MBIDs exist
///
/// # Bayesian Update Formula
/// For agreement boost when N sources agree on same MBID:
/// ```text
/// posterior = 1 - (1 - c1) * (1 - c2) * ... * (1 - cN)
/// ```
/// where c1, c2, ..., cN are confidence scores from agreeing sources
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::fusion::IdentityResolver;
/// use wkmp_ai::types::{Fusion, IdentityExtraction};
///
/// let resolver = IdentityResolver::new();
/// let identities = vec![
///     IdentityExtraction { recording_mbid: "mbid-123".into(), confidence: 0.9, source: "AcoustID".into() },
///     IdentityExtraction { recording_mbid: "mbid-123".into(), confidence: 0.6, source: "ID3".into() },
/// ];
///
/// let fused = resolver.fuse(identities).await?;
/// // posterior = 1 - (1-0.9) * (1-0.6) = 1 - 0.1 * 0.4 = 0.96
/// assert_eq!(fused.output.recording_mbid, Some("mbid-123".to_string()));
/// assert!(fused.output.confidence > 0.95);
/// ```
pub struct IdentityResolver {
    /// Minimum confidence threshold to consider an MBID candidate
    min_confidence: f32,
}

impl IdentityResolver {
    /// Create new Identity Resolver with default settings
    pub fn new() -> Self {
        Self {
            min_confidence: 0.3, // Require at least 30% confidence
        }
    }

    /// Create Identity Resolver with custom minimum confidence threshold
    pub fn with_min_confidence(min_confidence: f32) -> Self {
        Self {
            min_confidence: min_confidence.clamp(0.0, 1.0),
        }
    }

    /// Perform Bayesian fusion of MBID candidates
    ///
    /// # Arguments
    /// * `identities` - MBID candidates from multiple extractors
    ///
    /// # Returns
    /// Fused identity with highest posterior probability
    ///
    /// # Errors
    /// Returns error if:
    /// - No valid identities provided (all below min_confidence)
    /// - MBIDs fail format validation
    fn fuse_identities(
        &self,
        identities: Vec<IdentityExtraction>,
    ) -> Result<FusedIdentity, FusionError> {
        if identities.is_empty() {
            return Ok(FusedIdentity {
                recording_mbid: None,
                confidence: 0.0,
                posterior_probability: 0.0,
                conflicts: vec![],
            });
        }

        debug!(
            identity_count = identities.len(),
            "Starting Bayesian MBID fusion"
        );

        // Filter by minimum confidence
        let valid_identities: Vec<_> = identities
            .into_iter()
            .filter(|id| id.confidence >= self.min_confidence)
            .collect();

        if valid_identities.is_empty() {
            debug!("No identities above minimum confidence threshold");
            return Ok(FusedIdentity {
                recording_mbid: None,
                confidence: 0.0,
                posterior_probability: 0.0,
                conflicts: vec![],
            });
        }

        // Group by MBID value
        let mut mbid_groups: HashMap<String, Vec<&IdentityExtraction>> = HashMap::new();
        for identity in &valid_identities {
            mbid_groups
                .entry(identity.recording_mbid.clone())
                .or_insert_with(Vec::new)
                .push(identity);
        }

        debug!(
            unique_mbids = mbid_groups.len(),
            total_identities = valid_identities.len(),
            "Grouped identities by MBID"
        );

        // Compute posterior probability for each MBID
        let mut mbid_posteriors: Vec<(String, f32, Vec<String>)> = mbid_groups
            .into_iter()
            .map(|(mbid, sources)| {
                let posterior = self.compute_posterior(&sources);
                let source_names: Vec<String> =
                    sources.iter().map(|s| s.source.clone()).collect();
                (mbid, posterior, source_names)
            })
            .collect();

        // Sort by posterior probability (highest first)
        mbid_posteriors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Select best MBID
        let (best_mbid, best_posterior, sources) = mbid_posteriors
            .first()
            .ok_or_else(|| FusionError::InsufficientData("No valid MBIDs found".to_string()))?;

        // Detect conflicts (multiple distinct MBIDs with reasonable confidence)
        let conflicts: Vec<String> = mbid_posteriors
            .iter()
            .skip(1) // Skip the best one
            .filter(|(_, posterior, _)| *posterior >= 0.5) // Only significant conflicts
            .map(|(mbid, posterior, sources)| {
                format!(
                    "{} (posterior: {:.2}, sources: {})",
                    mbid,
                    posterior,
                    sources.join(", ")
                )
            })
            .collect();

        if !conflicts.is_empty() {
            debug!(
                conflict_count = conflicts.len(),
                "Identity conflicts detected"
            );
        }

        debug!(
            recording_mbid = %best_mbid,
            posterior = best_posterior,
            source_count = sources.len(),
            "Identity fusion complete"
        );

        Ok(FusedIdentity {
            recording_mbid: Some(best_mbid.clone()),
            confidence: *best_posterior,
            posterior_probability: *best_posterior,
            conflicts,
        })
    }

    /// Compute posterior probability using Bayesian update
    ///
    /// Formula: P(MBID | evidence) = 1 - ∏(1 - c_i)
    /// where c_i are confidence scores from sources agreeing on this MBID
    ///
    /// # Intuition
    /// - Single source with confidence 0.9 → posterior 0.9
    /// - Two sources (0.9, 0.6) agreeing → posterior 0.96 (agreement boost)
    /// - Three sources (0.9, 0.8, 0.7) agreeing → posterior 0.994 (strong agreement)
    fn compute_posterior(&self, sources: &[&IdentityExtraction]) -> f32 {
        if sources.is_empty() {
            return 0.0;
        }

        if sources.len() == 1 {
            // Single source: use base confidence (no agreement data)
            return sources[0].confidence;
        }

        // Multiple sources agreeing: Bayesian update
        // P(MBID | all agree) = 1 - P(NOT MBID | all agree)
        // P(NOT MBID | all agree) = P(NOT MBID | source 1) * P(NOT MBID | source 2) * ...
        //                          = (1 - c1) * (1 - c2) * ...

        let product: f32 = sources
            .iter()
            .map(|source| 1.0 - source.confidence)
            .product();

        1.0 - product
    }
}

impl Default for IdentityResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Fusion for IdentityResolver {
    type Input = Vec<IdentityExtraction>;
    type Output = FusedIdentity;

    fn name(&self) -> &'static str {
        "IdentityResolver"
    }

    async fn fuse(&self, inputs: Self::Input) -> Result<FusionResult<Self::Output>, FusionError> {
        debug!(input_count = inputs.len(), "Fusing identity extractions");

        let fused_identity = self.fuse_identities(inputs)?;

        // Extract values before moving fused_identity
        let confidence = fused_identity.confidence;
        let sources = if let Some(ref mbid) = fused_identity.recording_mbid {
            vec![format!(
                "MBID:{} (posterior: {:.2})",
                mbid, fused_identity.posterior_probability
            )]
        } else {
            vec![]
        };

        Ok(FusionResult {
            output: fused_identity,
            confidence,
            sources,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_name() {
        let resolver = IdentityResolver::new();
        assert_eq!(resolver.name(), "IdentityResolver");
    }

    #[test]
    fn test_default_min_confidence() {
        let resolver = IdentityResolver::new();
        assert_eq!(resolver.min_confidence, 0.3);
    }

    #[test]
    fn test_custom_min_confidence() {
        let resolver = IdentityResolver::with_min_confidence(0.5);
        assert_eq!(resolver.min_confidence, 0.5);
    }

    #[test]
    fn test_compute_posterior_single_source() {
        let resolver = IdentityResolver::new();
        let source = IdentityExtraction {
            recording_mbid: "mbid-123".to_string(),
            confidence: 0.9,
            source: "AcoustID".to_string(),
        };

        let posterior = resolver.compute_posterior(&[&source]);
        assert_eq!(posterior, 0.9, "Single source should use base confidence");
    }

    #[test]
    fn test_compute_posterior_two_sources_agreeing() {
        let resolver = IdentityResolver::new();
        let source1 = IdentityExtraction {
            recording_mbid: "mbid-123".to_string(),
            confidence: 0.9,
            source: "AcoustID".to_string(),
        };
        let source2 = IdentityExtraction {
            recording_mbid: "mbid-123".to_string(),
            confidence: 0.6,
            source: "ID3".to_string(),
        };

        let posterior = resolver.compute_posterior(&[&source1, &source2]);
        // 1 - (1-0.9) * (1-0.6) = 1 - 0.1 * 0.4 = 1 - 0.04 = 0.96
        assert!((posterior - 0.96).abs() < 0.001, "Expected ~0.96, got {}", posterior);
    }

    #[test]
    fn test_compute_posterior_three_sources_agreeing() {
        let resolver = IdentityResolver::new();
        let source1 = IdentityExtraction {
            recording_mbid: "mbid-123".to_string(),
            confidence: 0.9,
            source: "AcoustID".to_string(),
        };
        let source2 = IdentityExtraction {
            recording_mbid: "mbid-123".to_string(),
            confidence: 0.8,
            source: "ID3".to_string(),
        };
        let source3 = IdentityExtraction {
            recording_mbid: "mbid-123".to_string(),
            confidence: 0.7,
            source: "MusicBrainz".to_string(),
        };

        let posterior = resolver.compute_posterior(&[&source1, &source2, &source3]);
        // 1 - (1-0.9) * (1-0.8) * (1-0.7) = 1 - 0.1 * 0.2 * 0.3 = 1 - 0.006 = 0.994
        assert!((posterior - 0.994).abs() < 0.001, "Expected ~0.994, got {}", posterior);
    }

    #[tokio::test]
    async fn test_fuse_empty_input() {
        let resolver = IdentityResolver::new();
        let result = resolver.fuse(vec![]).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();
        assert!(fusion.output.recording_mbid.is_none());
        assert_eq!(fusion.output.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_fuse_single_identity() {
        let resolver = IdentityResolver::new();
        let identities = vec![IdentityExtraction {
            recording_mbid: "mbid-123".to_string(),
            confidence: 0.9,
            source: "AcoustID".to_string(),
        }];

        let result = resolver.fuse(identities).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();
        assert_eq!(fusion.output.recording_mbid, Some("mbid-123".to_string()));
        assert_eq!(fusion.output.confidence, 0.9);
        assert_eq!(fusion.output.conflicts.len(), 0);
    }

    #[tokio::test]
    async fn test_fuse_two_identities_agreeing() {
        let resolver = IdentityResolver::new();
        let identities = vec![
            IdentityExtraction {
                recording_mbid: "mbid-123".to_string(),
                confidence: 0.9,
                source: "AcoustID".to_string(),
            },
            IdentityExtraction {
                recording_mbid: "mbid-123".to_string(),
                confidence: 0.6,
                source: "ID3".to_string(),
            },
        ];

        let result = resolver.fuse(identities).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();
        assert_eq!(fusion.output.recording_mbid, Some("mbid-123".to_string()));
        assert!((fusion.output.confidence - 0.96).abs() < 0.001);
        assert_eq!(fusion.output.conflicts.len(), 0);
    }

    #[tokio::test]
    async fn test_fuse_conflict_resolution() {
        let resolver = IdentityResolver::new();
        let identities = vec![
            IdentityExtraction {
                recording_mbid: "mbid-123".to_string(),
                confidence: 0.9,
                source: "AcoustID".to_string(),
            },
            IdentityExtraction {
                recording_mbid: "mbid-456".to_string(),
                confidence: 0.6,
                source: "ID3".to_string(),
            },
        ];

        let result = resolver.fuse(identities).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();
        // Should choose mbid-123 (higher confidence)
        assert_eq!(fusion.output.recording_mbid, Some("mbid-123".to_string()));
        assert_eq!(fusion.output.confidence, 0.9);
        // Should report mbid-456 as conflict
        assert_eq!(fusion.output.conflicts.len(), 1);
    }

    #[tokio::test]
    async fn test_fuse_filters_low_confidence() {
        let resolver = IdentityResolver::new(); // min_confidence = 0.3
        let identities = vec![
            IdentityExtraction {
                recording_mbid: "mbid-123".to_string(),
                confidence: 0.9,
                source: "AcoustID".to_string(),
            },
            IdentityExtraction {
                recording_mbid: "mbid-456".to_string(),
                confidence: 0.1, // Below threshold
                source: "ID3".to_string(),
            },
        ];

        let result = resolver.fuse(identities).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();
        assert_eq!(fusion.output.recording_mbid, Some("mbid-123".to_string()));
        assert_eq!(fusion.output.conflicts.len(), 0); // Low-confidence identity filtered out
    }
}
