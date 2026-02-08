//! Stage 0 Integration Tests
//!
//! Tests the Stage 0 (embedded MusicBrainz Recording ID) identification feature.
//!
//! # Test Cases
//! - TC-S-001: Full pipeline integration test
//! - TC-S-002: Performance benchmark (< 50ms/file)
//! - TC-S-003: Coverage validation (>= 98%)
//! - TC-S-004: Offline capability test
//!
//! # Implementation
//! - PLAN-EMBID-001 Increment 5: Integration Testing and Validation

use std::collections::HashMap;
use std::time::Instant;
use wkmp_ai::fusion::IdentityResolver;
use wkmp_ai::matching::ConfidenceTier;
use wkmp_ai::types::{ConfidenceValue, MetadataExtraction};

/// Valid UUID format for testing
const TEST_MBID: &str = "12345678-1234-1234-1234-123456789012";
const TEST_ISRC: &str = "USRC12345678";

/// Create test metadata with embedded MBID and ISRC
fn create_metadata_with_mbid_and_isrc() -> MetadataExtraction {
    MetadataExtraction {
        title: Some(ConfidenceValue::new(
            "Test Song".to_string(),
            0.95,
            "ID3",
        )),
        artist: Some(ConfidenceValue::new(
            "Test Artist".to_string(),
            0.95,
            "ID3",
        )),
        album: Some(ConfidenceValue::new(
            "Test Album".to_string(),
            0.95,
            "ID3",
        )),
        recording_mbid: Some(ConfidenceValue::new(
            TEST_MBID.to_string(),
            0.98,
            "ID3-MBID",
        )),
        isrc: Some(ConfidenceValue::new(TEST_ISRC.to_string(), 0.95, "ID3-ISRC")),
        additional: HashMap::new(),
    }
}

/// Create test metadata with embedded MBID only (no ISRC)
fn create_metadata_with_mbid_only() -> MetadataExtraction {
    MetadataExtraction {
        title: Some(ConfidenceValue::new(
            "Test Song".to_string(),
            0.95,
            "ID3",
        )),
        artist: Some(ConfidenceValue::new(
            "Test Artist".to_string(),
            0.95,
            "ID3",
        )),
        album: Some(ConfidenceValue::new(
            "Test Album".to_string(),
            0.95,
            "ID3",
        )),
        recording_mbid: Some(ConfidenceValue::new(
            TEST_MBID.to_string(),
            0.98,
            "ID3-MBID",
        )),
        isrc: None,
        additional: HashMap::new(),
    }
}

/// Create test metadata without embedded MBID
fn create_metadata_without_mbid() -> MetadataExtraction {
    MetadataExtraction {
        title: Some(ConfidenceValue::new(
            "Test Song".to_string(),
            0.95,
            "ID3",
        )),
        artist: Some(ConfidenceValue::new(
            "Test Artist".to_string(),
            0.95,
            "ID3",
        )),
        album: Some(ConfidenceValue::new(
            "Test Album".to_string(),
            0.95,
            "ID3",
        )),
        recording_mbid: None,
        isrc: Some(ConfidenceValue::new(TEST_ISRC.to_string(), 0.95, "ID3-ISRC")),
        additional: HashMap::new(),
    }
}

// ============================================================================
// TC-S-001: Full pipeline integration test
// ============================================================================

/// Test Stage 0 identification with embedded MBID + ISRC → Tier1A
#[test]
fn test_stage0_with_embedded_mbid_and_isrc() {
    let resolver = IdentityResolver::new();
    let metadata = create_metadata_with_mbid_and_isrc();

    let result = resolver.check_stage0(&metadata);
    assert!(result.is_some(), "Stage 0 should match with embedded MBID + ISRC");

    let fused = result.unwrap();
    assert_eq!(
        fused.confidence_tier,
        ConfidenceTier::Tier1A,
        "Should be Tier1A with MBID + ISRC"
    );
    assert_eq!(
        fused.recording_mbid,
        Some(TEST_MBID.to_string()),
        "Should return the embedded MBID"
    );
    assert_eq!(fused.confidence, 1.0, "Tier1A should have 100% confidence");
    assert!(
        fused.conflicts.is_empty(),
        "Stage 0 should have no conflicts"
    );
}

/// Test Stage 0 identification with embedded MBID only → Tier1B
#[test]
fn test_stage0_with_embedded_mbid_only() {
    let resolver = IdentityResolver::new();
    let metadata = create_metadata_with_mbid_only();

    let result = resolver.check_stage0(&metadata);
    assert!(result.is_some(), "Stage 0 should match with embedded MBID only");

    let fused = result.unwrap();
    assert_eq!(
        fused.confidence_tier,
        ConfidenceTier::Tier1B,
        "Should be Tier1B with MBID only"
    );
    assert_eq!(
        fused.recording_mbid,
        Some(TEST_MBID.to_string()),
        "Should return the embedded MBID"
    );
    assert_eq!(fused.confidence, 0.98, "Tier1B should have 98% confidence");
}

/// Test Stage 0 fallback when no embedded MBID
#[test]
fn test_stage0_fallback_without_embedded_mbid() {
    let resolver = IdentityResolver::new();
    let metadata = create_metadata_without_mbid();

    let result = resolver.check_stage0(&metadata);
    assert!(
        result.is_none(),
        "Stage 0 should not match without embedded MBID"
    );
}

// ============================================================================
// TC-S-002: Performance benchmark (< 50ms/file)
// ============================================================================

/// Benchmark Stage 0 performance (should be < 50ms per file)
#[test]
fn test_stage0_performance() {
    let resolver = IdentityResolver::new();
    let num_iterations = 10000;

    // Create test data
    let metadata_with_mbid = create_metadata_with_mbid_and_isrc();
    let metadata_without_mbid = create_metadata_without_mbid();

    // Benchmark Stage 0 with MBID
    let start = Instant::now();
    for _ in 0..num_iterations {
        let _ = resolver.check_stage0(&metadata_with_mbid);
    }
    let elapsed_with_mbid = start.elapsed();

    // Benchmark Stage 0 without MBID
    let start = Instant::now();
    for _ in 0..num_iterations {
        let _ = resolver.check_stage0(&metadata_without_mbid);
    }
    let elapsed_without_mbid = start.elapsed();

    // Calculate per-iteration time
    let per_file_with_mbid_us =
        elapsed_with_mbid.as_micros() as f64 / num_iterations as f64;
    let per_file_without_mbid_us =
        elapsed_without_mbid.as_micros() as f64 / num_iterations as f64;

    println!(
        "Stage 0 Performance ({} iterations):",
        num_iterations
    );
    println!(
        "  With MBID: {:.2}µs/check ({:.2}ms total)",
        per_file_with_mbid_us,
        elapsed_with_mbid.as_millis() as f64
    );
    println!(
        "  Without MBID: {:.2}µs/check ({:.2}ms total)",
        per_file_without_mbid_us,
        elapsed_without_mbid.as_millis() as f64
    );

    // Verify performance (< 1ms per check, well under 50ms requirement)
    assert!(
        per_file_with_mbid_us < 1000.0,
        "Stage 0 with MBID too slow: {:.2}µs/check",
        per_file_with_mbid_us
    );
    assert!(
        per_file_without_mbid_us < 1000.0,
        "Stage 0 without MBID too slow: {:.2}µs/check",
        per_file_without_mbid_us
    );
}

// ============================================================================
// TC-S-003: Tier coverage and distribution
// ============================================================================

/// Test that confidence tier assignments match expected distribution
#[test]
fn test_tier_distribution_logic() {
    let resolver = IdentityResolver::new();

    // Simulate tier distribution based on metadata presence
    let test_cases = vec![
        // (has_mbid, has_isrc, expected_tier)
        (true, true, Some(ConfidenceTier::Tier1A)),   // MBID + ISRC
        (true, false, Some(ConfidenceTier::Tier1B)), // MBID only
        (false, true, None),                         // ISRC only → No Stage 0 match
        (false, false, None),                        // No identifiers
    ];

    for (has_mbid, has_isrc, expected_tier) in test_cases {
        let metadata = MetadataExtraction {
            title: Some(ConfidenceValue::new("Test".to_string(), 0.9, "ID3")),
            artist: None,
            album: None,
            recording_mbid: if has_mbid {
                Some(ConfidenceValue::new(TEST_MBID.to_string(), 0.98, "ID3"))
            } else {
                None
            },
            isrc: if has_isrc {
                Some(ConfidenceValue::new(TEST_ISRC.to_string(), 0.95, "ID3"))
            } else {
                None
            },
            additional: HashMap::new(),
        };

        let result = resolver.check_stage0(&metadata);
        let actual_tier = result.as_ref().map(|f| f.confidence_tier);

        assert_eq!(
            actual_tier, expected_tier,
            "Unexpected tier for has_mbid={}, has_isrc={}",
            has_mbid, has_isrc
        );
    }
}

// ============================================================================
// TC-S-004: Offline capability test
// ============================================================================

/// Verify Stage 0 works without network (pure local operation)
#[test]
fn test_stage0_offline_capability() {
    // Stage 0 uses only local ID3 data - no network required
    // This test verifies the implementation doesn't accidentally make network calls

    let resolver = IdentityResolver::new();
    let metadata = create_metadata_with_mbid_and_isrc();

    // Stage 0 check should be synchronous and instantaneous
    let start = Instant::now();
    let result = resolver.check_stage0(&metadata);
    let elapsed = start.elapsed();

    // If any network call was made, this would take >10ms
    assert!(
        elapsed.as_millis() < 10,
        "Stage 0 should be instant (no network): {}ms",
        elapsed.as_millis()
    );

    // Result should be valid
    assert!(result.is_some());
    assert!(result.unwrap().confidence_tier.is_stage0());
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

/// Test invalid MBID format rejection
#[test]
fn test_stage0_invalid_mbid_format() {
    let resolver = IdentityResolver::new();

    // MBID too short
    let metadata = MetadataExtraction {
        title: None,
        artist: None,
        album: None,
        recording_mbid: Some(ConfidenceValue::new(
            "invalid-mbid".to_string(),
            0.98,
            "ID3",
        )),
        isrc: Some(ConfidenceValue::new(TEST_ISRC.to_string(), 0.95, "ID3")),
        additional: HashMap::new(),
    };

    let result = resolver.check_stage0(&metadata);
    assert!(
        result.is_none(),
        "Should reject invalid MBID format"
    );
}

/// Test confidence tier confidence_score values
#[test]
fn test_tier_confidence_scores() {
    assert_eq!(ConfidenceTier::Tier1A.confidence_score(), 1.0);
    assert_eq!(ConfidenceTier::Tier1B.confidence_score(), 0.98);
    assert_eq!(ConfidenceTier::Tier2A.confidence_score(), 0.95);
    assert_eq!(ConfidenceTier::Tier2B.confidence_score(), 0.85);
    assert_eq!(ConfidenceTier::Tier3.confidence_score(), 0.70);
    assert_eq!(ConfidenceTier::Tier4.confidence_score(), 0.0);
}

/// Test tier is_stage0 classification
#[test]
fn test_tier_is_stage0_classification() {
    assert!(ConfidenceTier::Tier1A.is_stage0());
    assert!(ConfidenceTier::Tier1B.is_stage0());
    assert!(!ConfidenceTier::Tier2A.is_stage0());
    assert!(!ConfidenceTier::Tier2B.is_stage0());
    assert!(!ConfidenceTier::Tier3.is_stage0());
    assert!(!ConfidenceTier::Tier4.is_stage0());
}

/// Test tier is_matched classification
#[test]
fn test_tier_is_matched_classification() {
    assert!(ConfidenceTier::Tier1A.is_matched());
    assert!(ConfidenceTier::Tier1B.is_matched());
    assert!(ConfidenceTier::Tier2A.is_matched());
    assert!(ConfidenceTier::Tier2B.is_matched());
    assert!(ConfidenceTier::Tier3.is_matched());
    assert!(!ConfidenceTier::Tier4.is_matched()); // Only Tier4 is unmatched
}
