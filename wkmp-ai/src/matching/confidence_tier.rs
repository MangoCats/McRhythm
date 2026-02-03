//! Confidence Tier for Recording Identification
//!
//! Defines the `ConfidenceTier` enum for categorizing the confidence level
//! of recording identification based on the source of the match.
//!
//! # Tier Hierarchy
//!
//! | Tier | Source | Confidence | Description |
//! |------|--------|------------|-------------|
//! | 1A | Embedded MB ID + ISRC | 100% | Highest confidence - two authoritative identifiers |
//! | 1B | Embedded MB ID only | 98% | Very high - single authoritative identifier |
//! | 2A | AcoustID >= 0.95 | 95% | High confidence fingerprint match |
//! | 2B | AcoustID 0.80-0.95 | 85% | Medium confidence fingerprint match |
//! | 3 | Album-first query | 70% | Metadata-based match |
//! | 4 | No match | 0% | No identification possible |
//!
//! # Implementation
//! - PLAN-EMBID-001 Increment 2
//! - FR-04: Assign confidence tier based on available metadata

use serde::{Deserialize, Serialize};
use std::fmt;

/// Confidence tier for recording identification
///
/// Categorizes the confidence level of a recording identification
/// based on the source and quality of the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceTier {
    /// Tier 1A: Embedded MB Recording ID + ISRC (100% confidence)
    ///
    /// Highest confidence - file contains both an embedded MusicBrainz
    /// Recording ID and an ISRC, providing two independent authoritative
    /// identifiers that can cross-validate.
    #[serde(rename = "1a")]
    Tier1A,

    /// Tier 1B: Embedded MB Recording ID only (98% confidence)
    ///
    /// Very high confidence - file contains an embedded MusicBrainz
    /// Recording ID, which is an authoritative identifier. Slightly
    /// lower than 1A due to lack of cross-validation with ISRC.
    #[serde(rename = "1b")]
    Tier1B,

    /// Tier 2A: AcoustID confidence >= 0.95 (95% confidence)
    ///
    /// High confidence fingerprint match - AcoustID returned a
    /// match with very high confidence score.
    #[serde(rename = "2a")]
    Tier2A,

    /// Tier 2B: AcoustID confidence 0.80-0.95 (85% confidence)
    ///
    /// Medium confidence fingerprint match - AcoustID returned a
    /// match with moderate confidence score.
    #[serde(rename = "2b")]
    Tier2B,

    /// Tier 3: Album-first query match (70% confidence)
    ///
    /// Metadata-based match - identification was achieved through
    /// album/artist/title matching rather than fingerprinting.
    #[serde(rename = "3")]
    Tier3,

    /// Tier 4: No match found (0% confidence)
    ///
    /// No identification was possible through any method.
    #[serde(rename = "4")]
    Tier4,
}

impl ConfidenceTier {
    /// Get numeric confidence score for this tier (0.0-1.0)
    ///
    /// Returns the base confidence score associated with each tier.
    /// These scores represent the probability that the identification
    /// is correct.
    pub fn confidence_score(&self) -> f32 {
        match self {
            Self::Tier1A => 1.0,
            Self::Tier1B => 0.98,
            Self::Tier2A => 0.95,
            Self::Tier2B => 0.85, // midpoint of 0.80-0.95
            Self::Tier3 => 0.70,
            Self::Tier4 => 0.0,
        }
    }

    /// Check if this tier represents a successful match
    ///
    /// Returns `true` for all tiers except Tier4 (no match).
    pub fn is_matched(&self) -> bool {
        !matches!(self, Self::Tier4)
    }

    /// Check if this tier is from Stage 0 (embedded MB ID)
    ///
    /// Returns `true` for Tier1A and Tier1B, which are derived
    /// from embedded MusicBrainz Recording IDs.
    pub fn is_stage0(&self) -> bool {
        matches!(self, Self::Tier1A | Self::Tier1B)
    }

    /// Get tier name for logging and display
    ///
    /// Returns a human-readable description of the tier.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Tier1A => "1A (Embedded + ISRC)",
            Self::Tier1B => "1B (Embedded only)",
            Self::Tier2A => "2A (AcoustID high)",
            Self::Tier2B => "2B (AcoustID medium)",
            Self::Tier3 => "3 (Album-first)",
            Self::Tier4 => "4 (No match)",
        }
    }

    /// Get short tier code for compact display
    ///
    /// Returns a short code like "1A", "1B", "2A", etc.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Tier1A => "1A",
            Self::Tier1B => "1B",
            Self::Tier2A => "2A",
            Self::Tier2B => "2B",
            Self::Tier3 => "3",
            Self::Tier4 => "4",
        }
    }
}

impl fmt::Display for ConfidenceTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Default for ConfidenceTier {
    fn default() -> Self {
        Self::Tier4
    }
}

/// Assign confidence tier based on available metadata
///
/// Determines the appropriate confidence tier for a recording identification
/// based on what data sources are available.
///
/// # Arguments
/// * `has_embedded_mbid` - Whether the file has an embedded MusicBrainz Recording ID
/// * `has_isrc` - Whether the file has an ISRC
/// * `acoustid_confidence` - AcoustID match confidence (if available)
/// * `album_match` - Whether an album-first match was successful
///
/// # Returns
/// The appropriate `ConfidenceTier` based on the priority:
/// 1. Embedded MB ID (with or without ISRC) → Tier 1A/1B
/// 2. AcoustID high confidence → Tier 2A
/// 3. AcoustID medium confidence → Tier 2B
/// 4. Album-first match → Tier 3
/// 5. No match → Tier 4
///
/// # Example
/// ```rust
/// use wkmp_ai::matching::confidence_tier::{assign_tier, ConfidenceTier};
///
/// // File with embedded MB ID and ISRC
/// let tier = assign_tier(true, true, None, false);
/// assert_eq!(tier, ConfidenceTier::Tier1A);
///
/// // File with only AcoustID match
/// let tier = assign_tier(false, false, Some(0.97), false);
/// assert_eq!(tier, ConfidenceTier::Tier2A);
/// ```
pub fn assign_tier(
    has_embedded_mbid: bool,
    has_isrc: bool,
    acoustid_confidence: Option<f32>,
    album_match: bool,
) -> ConfidenceTier {
    // Priority 1: Embedded MusicBrainz Recording ID (Stage 0)
    if has_embedded_mbid {
        if has_isrc {
            return ConfidenceTier::Tier1A;
        } else {
            return ConfidenceTier::Tier1B;
        }
    }

    // Priority 2: AcoustID fingerprint match
    if let Some(conf) = acoustid_confidence {
        if conf >= 0.95 {
            return ConfidenceTier::Tier2A;
        } else if conf >= 0.80 {
            return ConfidenceTier::Tier2B;
        }
        // Below 0.80 - fall through to check album match
    }

    // Priority 3: Album-first metadata match
    if album_match {
        return ConfidenceTier::Tier3;
    }

    // Priority 4: No match
    ConfidenceTier::Tier4
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_confidence_scores() {
        assert_eq!(ConfidenceTier::Tier1A.confidence_score(), 1.0);
        assert_eq!(ConfidenceTier::Tier1B.confidence_score(), 0.98);
        assert_eq!(ConfidenceTier::Tier2A.confidence_score(), 0.95);
        assert_eq!(ConfidenceTier::Tier2B.confidence_score(), 0.85);
        assert_eq!(ConfidenceTier::Tier3.confidence_score(), 0.70);
        assert_eq!(ConfidenceTier::Tier4.confidence_score(), 0.0);
    }

    #[test]
    fn test_tier_is_matched() {
        assert!(ConfidenceTier::Tier1A.is_matched());
        assert!(ConfidenceTier::Tier1B.is_matched());
        assert!(ConfidenceTier::Tier2A.is_matched());
        assert!(ConfidenceTier::Tier2B.is_matched());
        assert!(ConfidenceTier::Tier3.is_matched());
        assert!(!ConfidenceTier::Tier4.is_matched());
    }

    #[test]
    fn test_tier_is_stage0() {
        assert!(ConfidenceTier::Tier1A.is_stage0());
        assert!(ConfidenceTier::Tier1B.is_stage0());
        assert!(!ConfidenceTier::Tier2A.is_stage0());
        assert!(!ConfidenceTier::Tier2B.is_stage0());
        assert!(!ConfidenceTier::Tier3.is_stage0());
        assert!(!ConfidenceTier::Tier4.is_stage0());
    }

    #[test]
    fn test_tier_names() {
        assert_eq!(ConfidenceTier::Tier1A.name(), "1A (Embedded + ISRC)");
        assert_eq!(ConfidenceTier::Tier1B.name(), "1B (Embedded only)");
        assert_eq!(ConfidenceTier::Tier2A.name(), "2A (AcoustID high)");
        assert_eq!(ConfidenceTier::Tier2B.name(), "2B (AcoustID medium)");
        assert_eq!(ConfidenceTier::Tier3.name(), "3 (Album-first)");
        assert_eq!(ConfidenceTier::Tier4.name(), "4 (No match)");
    }

    #[test]
    fn test_tier_codes() {
        assert_eq!(ConfidenceTier::Tier1A.code(), "1A");
        assert_eq!(ConfidenceTier::Tier1B.code(), "1B");
        assert_eq!(ConfidenceTier::Tier2A.code(), "2A");
        assert_eq!(ConfidenceTier::Tier2B.code(), "2B");
        assert_eq!(ConfidenceTier::Tier3.code(), "3");
        assert_eq!(ConfidenceTier::Tier4.code(), "4");
    }

    #[test]
    fn test_tier_display() {
        assert_eq!(format!("{}", ConfidenceTier::Tier1A), "1A (Embedded + ISRC)");
        assert_eq!(format!("{}", ConfidenceTier::Tier4), "4 (No match)");
    }

    #[test]
    fn test_tier_default() {
        assert_eq!(ConfidenceTier::default(), ConfidenceTier::Tier4);
    }

    // ========================================================================
    // assign_tier() tests (TC-U-003 through TC-U-006)
    // ========================================================================

    #[test]
    fn test_assign_tier_embedded_mbid_with_isrc() {
        // TC-U-003: Embedded MB ID + ISRC → Tier1A
        let tier = assign_tier(true, true, None, false);
        assert_eq!(tier, ConfidenceTier::Tier1A);

        // Even with AcoustID, embedded takes priority
        let tier = assign_tier(true, true, Some(0.99), true);
        assert_eq!(tier, ConfidenceTier::Tier1A);
    }

    #[test]
    fn test_assign_tier_embedded_mbid_only() {
        // TC-U-004: Embedded MB ID only → Tier1B
        let tier = assign_tier(true, false, None, false);
        assert_eq!(tier, ConfidenceTier::Tier1B);

        // Even with AcoustID, embedded takes priority
        let tier = assign_tier(true, false, Some(0.99), true);
        assert_eq!(tier, ConfidenceTier::Tier1B);
    }

    #[test]
    fn test_assign_tier_acoustid_high_confidence() {
        // TC-U-005: AcoustID high confidence → Tier2A
        let tier = assign_tier(false, false, Some(0.95), false);
        assert_eq!(tier, ConfidenceTier::Tier2A);

        let tier = assign_tier(false, false, Some(0.99), false);
        assert_eq!(tier, ConfidenceTier::Tier2A);

        let tier = assign_tier(false, true, Some(0.97), false);
        assert_eq!(tier, ConfidenceTier::Tier2A);
    }

    #[test]
    fn test_assign_tier_acoustid_medium_confidence() {
        // AcoustID medium confidence → Tier2B
        let tier = assign_tier(false, false, Some(0.80), false);
        assert_eq!(tier, ConfidenceTier::Tier2B);

        let tier = assign_tier(false, false, Some(0.90), false);
        assert_eq!(tier, ConfidenceTier::Tier2B);

        let tier = assign_tier(false, false, Some(0.94), false);
        assert_eq!(tier, ConfidenceTier::Tier2B);
    }

    #[test]
    fn test_assign_tier_album_match() {
        // Album-first match → Tier3
        let tier = assign_tier(false, false, None, true);
        assert_eq!(tier, ConfidenceTier::Tier3);

        // Low AcoustID + album match → Tier3
        let tier = assign_tier(false, false, Some(0.50), true);
        assert_eq!(tier, ConfidenceTier::Tier3);
    }

    #[test]
    fn test_assign_tier_no_match() {
        // TC-U-006: No match → Tier4
        let tier = assign_tier(false, false, None, false);
        assert_eq!(tier, ConfidenceTier::Tier4);

        // Low AcoustID, no album match → Tier4
        let tier = assign_tier(false, false, Some(0.50), false);
        assert_eq!(tier, ConfidenceTier::Tier4);

        // ISRC without MBID doesn't help
        let tier = assign_tier(false, true, None, false);
        assert_eq!(tier, ConfidenceTier::Tier4);
    }

    #[test]
    fn test_serde_roundtrip() {
        // Test serialization/deserialization
        let tier = ConfidenceTier::Tier1A;
        let serialized = serde_json::to_string(&tier).unwrap();
        assert_eq!(serialized, "\"1a\"");

        let deserialized: ConfidenceTier = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, ConfidenceTier::Tier1A);

        // Test all tiers
        for tier in [
            ConfidenceTier::Tier1A,
            ConfidenceTier::Tier1B,
            ConfidenceTier::Tier2A,
            ConfidenceTier::Tier2B,
            ConfidenceTier::Tier3,
            ConfidenceTier::Tier4,
        ] {
            let serialized = serde_json::to_string(&tier).unwrap();
            let deserialized: ConfidenceTier = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, tier);
        }
    }
}
