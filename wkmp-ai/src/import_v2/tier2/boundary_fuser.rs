// PLAN023 Tier 2: Boundary Fuser
//
// Concept: Fuse passage boundaries from multiple detection strategies
// Synchronization: Accepts Vec<ExtractorResult<Vec<PassageBoundary>>>, outputs FusedBoundary
//
// Algorithm (per SPEC_wkmp_ai_recode.md):
// 1. Collect all boundary candidates from all detection methods
// 2. Cluster nearby boundaries (within tolerance window)
// 3. For each cluster, compute weighted average of start/end times
// 4. Select highest-confidence boundary from each cluster
// 5. Return list of non-overlapping boundaries with confidence scores

use crate::import_v2::types::{
    BoundaryDetectionMethod, ExtractorResult, ImportResult, PassageBoundary,
};

/// Fused boundary with provenance
///
/// **[SRC-DB-010]** Time values are stored as ticks (i64) for sample-accurate precision.
/// Tick rate: 28,224,000 Hz (1 tick ≈ 35.4 nanoseconds)
#[derive(Debug, Clone)]
pub struct FusedBoundary {
    pub start_ticks: i64,
    pub end_ticks: i64,
    pub confidence: f64,
    pub methods_used: Vec<BoundaryDetectionMethod>,
}

/// Boundary fuser (Tier 2 fusion concept)
///
/// **Legible Software Principle:**
/// - Independent module: Pure fusion logic, no audio processing
/// - Explicit synchronization: Clear contract with Tier 1 boundary detectors
/// - Transparent behavior: Clustering and weighting are explicit
/// - Integrity: Ensures non-overlapping boundaries
pub struct BoundaryFuser {
    /// Clustering tolerance (ticks) - boundaries within this window are considered same
    /// **[SRC-DB-010]** 500ms = 14,112,000 ticks (500ms * 28,224 ticks/ms)
    clustering_tolerance_ticks: i64,
    /// Minimum confidence threshold for accepting a boundary
    min_confidence: f64,
}

impl Default for BoundaryFuser {
    fn default() -> Self {
        Self {
            clustering_tolerance_ticks: 14_112_000, // 500ms in ticks (500 * 28,224)
            min_confidence: 0.3,          // Accept boundaries with confidence ≥ 0.3
        }
    }
}

impl BoundaryFuser {
    /// Fuse boundaries from multiple detection methods
    ///
    /// # Algorithm: Clustering + Weighted Averaging
    /// 1. Collect all boundaries from all methods
    /// 2. Sort by start time
    /// 3. Cluster nearby boundaries (within tolerance window)
    /// 4. For each cluster:
    ///    - Compute confidence-weighted average of start/end times
    ///    - Select detection methods that contributed
    ///    - Calculate overall cluster confidence
    /// 5. Filter by min_confidence threshold
    /// 6. Ensure non-overlapping (merge overlapping clusters)
    ///
    /// # Arguments
    /// * `boundary_lists` - Boundaries from each detection method
    ///
    /// # Returns
    /// Vector of fused boundaries with confidence scores
    pub fn fuse(
        &self,
        boundary_lists: Vec<ExtractorResult<Vec<PassageBoundary>>>,
    ) -> ImportResult<Vec<FusedBoundary>> {
        if boundary_lists.is_empty() {
            return Ok(vec![]);
        }

        // Step 1: Collect all boundaries from all sources
        let mut all_boundaries: Vec<PassageBoundary> = Vec::new();

        for result in boundary_lists {
            all_boundaries.extend(result.data);
        }

        if all_boundaries.is_empty() {
            return Ok(vec![]);
        }

        // Step 2: Sort by start time
        all_boundaries.sort_by_key(|b| b.start_ticks);

        tracing::debug!(
            "Boundary fusion: {} candidates from {} methods",
            all_boundaries.len(),
            all_boundaries
                .iter()
                .map(|b| b.detection_method)
                .collect::<std::collections::HashSet<_>>()
                .len()
        );

        // Step 3: Cluster nearby boundaries
        let clusters = self.cluster_boundaries(&all_boundaries);

        tracing::debug!(
            "Boundary clustering: {} boundaries → {} clusters",
            all_boundaries.len(),
            clusters.len()
        );

        // Step 4: Fuse each cluster into a single boundary
        let mut fused_boundaries: Vec<FusedBoundary> = clusters
            .into_iter()
            .filter_map(|cluster| self.fuse_cluster(&cluster))
            .collect();

        // Step 5: Ensure non-overlapping (merge if needed)
        fused_boundaries = self.merge_overlapping(fused_boundaries);

        tracing::info!(
            "Boundary fusion complete: {} final boundaries",
            fused_boundaries.len()
        );

        Ok(fused_boundaries)
    }

    /// Cluster boundaries that are within tolerance window
    fn cluster_boundaries(&self, boundaries: &[PassageBoundary]) -> Vec<Vec<PassageBoundary>> {
        if boundaries.is_empty() {
            return vec![];
        }

        let mut clusters: Vec<Vec<PassageBoundary>> = Vec::new();
        let mut current_cluster: Vec<PassageBoundary> = vec![boundaries[0]];

        for boundary in boundaries.iter().skip(1) {
            let last_in_cluster = &current_cluster.last().unwrap();

            // Check if this boundary is within tolerance of cluster
            let start_diff = (boundary.start_ticks - last_in_cluster.start_ticks).abs();

            if start_diff <= self.clustering_tolerance_ticks {
                // Add to current cluster
                current_cluster.push(*boundary);
            } else {
                // Start new cluster
                clusters.push(current_cluster);
                current_cluster = vec![*boundary];
            }
        }

        // Add final cluster
        clusters.push(current_cluster);

        clusters
    }

    /// Fuse a cluster of boundaries into a single boundary
    fn fuse_cluster(&self, cluster: &[PassageBoundary]) -> Option<FusedBoundary> {
        if cluster.is_empty() {
            return None;
        }

        // Compute confidence-weighted average of start/end times
        let total_confidence: f64 = cluster.iter().map(|b| b.confidence).sum();

        if total_confidence == 0.0 {
            return None;
        }

        let weighted_start: f64 = cluster
            .iter()
            .map(|b| (b.start_ticks as f64) * b.confidence)
            .sum::<f64>()
            / total_confidence;

        let weighted_end: f64 = cluster
            .iter()
            .map(|b| (b.end_ticks as f64) * b.confidence)
            .sum::<f64>()
            / total_confidence;

        // Overall cluster confidence = average of member confidences
        let cluster_confidence = total_confidence / cluster.len() as f64;

        // Reject if below threshold
        if cluster_confidence < self.min_confidence {
            tracing::debug!(
                "Rejecting cluster with confidence {:.3} < threshold {:.2}",
                cluster_confidence,
                self.min_confidence
            );
            return None;
        }

        // Collect methods used
        let methods_used: Vec<BoundaryDetectionMethod> = cluster
            .iter()
            .map(|b| b.detection_method)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Some(FusedBoundary {
            start_ticks: weighted_start.round() as i64,
            end_ticks: weighted_end.round() as i64,
            confidence: cluster_confidence,
            methods_used,
        })
    }

    /// Merge overlapping boundaries (keep higher-confidence one)
    fn merge_overlapping(&self, mut boundaries: Vec<FusedBoundary>) -> Vec<FusedBoundary> {
        if boundaries.len() < 2 {
            return boundaries;
        }

        // Sort by start time
        boundaries.sort_by_key(|b| b.start_ticks);

        let mut merged: Vec<FusedBoundary> = Vec::new();
        let mut current = boundaries[0].clone();

        for next in boundaries.into_iter().skip(1) {
            // Check for overlap: current.end > next.start
            if current.end_ticks > next.start_ticks {
                // Overlap detected - keep higher confidence
                if next.confidence > current.confidence {
                    tracing::debug!(
                        "Merging overlapping boundaries: keeping higher confidence ({:.3} > {:.3})",
                        next.confidence,
                        current.confidence
                    );
                    current = next;
                }
                // else: keep current, discard next
            } else {
                // No overlap - save current and move to next
                merged.push(current);
                current = next;
            }
        }

        // Add final boundary
        merged.push(current);

        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_v2::types::ExtractionSource;

    /// Helper to convert milliseconds to ticks
    /// Tick rate: 28,224 ticks/ms (28,224,000 Hz)
    const TICKS_PER_MS: i64 = 28_224;

    fn ms_to_ticks(ms: u32) -> i64 {
        (ms as i64) * TICKS_PER_MS
    }

    fn create_boundary(
        start_ms: u32,
        end_ms: u32,
        confidence: f64,
        method: BoundaryDetectionMethod,
    ) -> PassageBoundary {
        PassageBoundary {
            start_ticks: ms_to_ticks(start_ms),
            end_ticks: ms_to_ticks(end_ms),
            confidence,
            detection_method: method,
        }
    }

    #[test]
    fn test_empty_boundaries() {
        let fuser = BoundaryFuser::default();
        let result = fuser.fuse(vec![]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_boundary() {
        let fuser = BoundaryFuser::default();

        let boundaries = vec![ExtractorResult {
            data: vec![create_boundary(
                0,
                180000, // 3 minutes
                0.9,
                BoundaryDetectionMethod::SilenceDetection,
            )],
            confidence: 0.9,
            source: ExtractionSource::AudioDerived,
        }];

        let result = fuser.fuse(boundaries).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].start_ticks, ms_to_ticks(0));
        assert_eq!(result[0].end_ticks, ms_to_ticks(180000));
        assert_eq!(result[0].confidence, 0.9);
    }

    #[test]
    fn test_clustering_nearby_boundaries() {
        let fuser = BoundaryFuser::default();

        // Two boundaries with starts within 500ms tolerance
        let boundaries = vec![ExtractorResult {
            data: vec![
                create_boundary(
                    1000,
                    180000,
                    0.8,
                    BoundaryDetectionMethod::SilenceDetection,
                ),
                create_boundary(
                    1200, // 200ms difference < 500ms tolerance
                    180500,
                    0.7,
                    BoundaryDetectionMethod::BeatTracking,
                ),
            ],
            confidence: 0.8,
            source: ExtractionSource::AudioDerived,
        }];

        let result = fuser.fuse(boundaries).unwrap();

        // Should cluster into single boundary
        assert_eq!(result.len(), 1);

        // Weighted average: (1000*0.8 + 1200*0.7) / (0.8+0.7) = (800+840) / 1.5 = 1093.33 ms
        // In ticks: 1093.33 * 28,224 = 30,850,099 ticks
        let expected_start_ticks = ms_to_ticks(1093);
        assert!((result[0].start_ticks - expected_start_ticks).abs() < ms_to_ticks(2));

        // Both methods should be recorded
        assert_eq!(result[0].methods_used.len(), 2);
    }

    #[test]
    fn test_no_clustering_distant_boundaries() {
        let fuser = BoundaryFuser::default();

        // Two boundaries far apart (> 500ms tolerance) and NON-OVERLAPPING
        let boundaries = vec![ExtractorResult {
            data: vec![
                create_boundary(
                    1000,
                    180000, // Ends at 180s
                    0.8,
                    BoundaryDetectionMethod::SilenceDetection,
                ),
                create_boundary(
                    200000, // Starts at 200s (after first boundary ends)
                    380000,
                    0.7,
                    BoundaryDetectionMethod::SilenceDetection,
                ),
            ],
            confidence: 0.8,
            source: ExtractionSource::AudioDerived,
        }];

        let result = fuser.fuse(boundaries).unwrap();

        // Should remain as 2 separate boundaries (distant + non-overlapping)
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].start_ticks, ms_to_ticks(1000));
        assert_eq!(result[1].start_ticks, ms_to_ticks(200000));
    }

    #[test]
    fn test_threshold_filtering() {
        let fuser = BoundaryFuser {
            min_confidence: 0.6, // High threshold
            ..Default::default()
        };

        let boundaries = vec![ExtractorResult {
            data: vec![
                create_boundary(
                    1000,
                    180000,
                    0.5, // Below 0.6 threshold
                    BoundaryDetectionMethod::SilenceDetection,
                ),
                create_boundary(
                    10000,
                    190000,
                    0.8, // Above threshold
                    BoundaryDetectionMethod::SilenceDetection,
                ),
            ],
            confidence: 0.8,
            source: ExtractionSource::AudioDerived,
        }];

        let result = fuser.fuse(boundaries).unwrap();

        // Only high-confidence boundary should pass
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].start_ticks, ms_to_ticks(10000));
    }

    #[test]
    fn test_overlap_resolution_keeps_higher_confidence() {
        let fuser = BoundaryFuser::default();

        // Two overlapping boundaries (first ends after second starts)
        let boundaries = vec![ExtractorResult {
            data: vec![
                create_boundary(
                    0,
                    100000, // Ends at 100s
                    0.7,
                    BoundaryDetectionMethod::SilenceDetection,
                ),
                create_boundary(
                    50000, // Starts at 50s (overlap!)
                    150000,
                    0.9, // Higher confidence
                    BoundaryDetectionMethod::BeatTracking,
                ),
            ],
            confidence: 0.8,
            source: ExtractionSource::AudioDerived,
        }];

        let result = fuser.fuse(boundaries).unwrap();

        // Should keep only higher-confidence boundary
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].start_ticks, ms_to_ticks(50000)); // Second boundary wins
        assert_eq!(result[0].confidence, 0.9);
    }

    #[test]
    fn test_multiple_clusters() {
        let fuser = BoundaryFuser::default();

        let boundaries = vec![ExtractorResult {
            data: vec![
                // Cluster 1: around 1000ms
                create_boundary(1000, 180000, 0.8, BoundaryDetectionMethod::SilenceDetection),
                create_boundary(1200, 180500, 0.7, BoundaryDetectionMethod::BeatTracking),
                // Cluster 2: around 200000ms (far away)
                create_boundary(200000, 380000, 0.9, BoundaryDetectionMethod::SilenceDetection),
                create_boundary(200300, 380200, 0.85, BoundaryDetectionMethod::BeatTracking),
            ],
            confidence: 0.8,
            source: ExtractionSource::AudioDerived,
        }];

        let result = fuser.fuse(boundaries).unwrap();

        // Should produce 2 fused boundaries (one per cluster)
        assert_eq!(result.len(), 2);
        assert!(result[0].start_ticks < ms_to_ticks(2000)); // First cluster around 1000ms
        assert!(result[1].start_ticks > ms_to_ticks(190000)); // Second cluster around 200000ms
    }

    #[test]
    fn test_weighted_averaging() {
        let fuser = BoundaryFuser::default();

        // Two boundaries in same cluster with different confidences
        let boundaries = vec![ExtractorResult {
            data: vec![
                create_boundary(
                    1000,
                    180000,
                    0.9, // High confidence
                    BoundaryDetectionMethod::SilenceDetection,
                ),
                create_boundary(
                    1400, // 400ms difference
                    180400,
                    0.3, // Low confidence
                    BoundaryDetectionMethod::BeatTracking,
                ),
            ],
            confidence: 0.8,
            source: ExtractionSource::AudioDerived,
        }];

        let result = fuser.fuse(boundaries).unwrap();

        // Weighted average should favor high-confidence value
        // (1000*0.9 + 1400*0.3) / (0.9+0.3) = (900+420) / 1.2 = 1100 ms
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].start_ticks, ms_to_ticks(1100));
    }

    #[test]
    fn test_confidence_calculation() {
        let fuser = BoundaryFuser::default();

        // Cluster with 3 members
        let boundaries = vec![ExtractorResult {
            data: vec![
                create_boundary(1000, 180000, 0.9, BoundaryDetectionMethod::SilenceDetection),
                create_boundary(1200, 180200, 0.7, BoundaryDetectionMethod::BeatTracking),
                create_boundary(1100, 180100, 0.8, BoundaryDetectionMethod::StructuralAnalysis),
            ],
            confidence: 0.8,
            source: ExtractionSource::AudioDerived,
        }];

        let result = fuser.fuse(boundaries).unwrap();

        // Cluster confidence = average of members = (0.9 + 0.7 + 0.8) / 3 = 0.8
        assert_eq!(result.len(), 1);
        assert!((result[0].confidence - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_methods_used_tracking() {
        let fuser = BoundaryFuser::default();

        let boundaries = vec![ExtractorResult {
            data: vec![
                create_boundary(1000, 180000, 0.8, BoundaryDetectionMethod::SilenceDetection),
                create_boundary(1200, 180200, 0.7, BoundaryDetectionMethod::BeatTracking),
            ],
            confidence: 0.8,
            source: ExtractionSource::AudioDerived,
        }];

        let result = fuser.fuse(boundaries).unwrap();

        assert_eq!(result.len(), 1);

        // Both methods should be tracked
        assert_eq!(result[0].methods_used.len(), 2);
        assert!(result[0]
            .methods_used
            .contains(&BoundaryDetectionMethod::SilenceDetection));
        assert!(result[0]
            .methods_used
            .contains(&BoundaryDetectionMethod::BeatTracking));
    }
}
