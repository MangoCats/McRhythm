# Increment 2: Define ConfidenceTier Enum and Logic

**Increment:** 2 of 5
**Phase:** Implementation
**Estimated Effort:** 2-3 hours
**Confidence:** HIGH (±25%)
**Prerequisites:** Increment 1 (ISRC extraction)

---

## Objective

Create the `ConfidenceTier` enum and implement tier assignment logic based on available metadata.

---

## Deliverables

1. **New:** `wkmp-ai/src/matching/confidence_tier.rs`
   - `ConfidenceTier` enum (Tier1A, Tier1B, Tier2A, Tier2B, Tier3, Tier4)
   - `assign_tier()` function
   - `tier_confidence()` function

2. **Modified:** `wkmp-ai/src/matching/mod.rs`
   - Export new module

---

## Requirements Covered

- **FR-04:** Assign confidence tier based on available metadata (complete)

---

## Implementation Notes

**ConfidenceTier Enum:**
```rust
/// Confidence tier for recording identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfidenceTier {
    /// Tier 1A: Embedded MB Recording ID + ISRC (100% confidence)
    Tier1A,
    /// Tier 1B: Embedded MB Recording ID only (98% confidence)
    Tier1B,
    /// Tier 2A: AcoustID confidence >= 0.95 (95%+ confidence)
    Tier2A,
    /// Tier 2B: AcoustID confidence 0.80-0.95 (80-95% confidence)
    Tier2B,
    /// Tier 3: Album-first query match (variable confidence)
    Tier3,
    /// Tier 4: No match found
    Tier4,
}

impl ConfidenceTier {
    /// Get numeric confidence score for this tier
    pub fn confidence_score(&self) -> f32 {
        match self {
            Self::Tier1A => 1.0,
            Self::Tier1B => 0.98,
            Self::Tier2A => 0.95,
            Self::Tier2B => 0.85,  // midpoint of 0.80-0.95
            Self::Tier3 => 0.70,
            Self::Tier4 => 0.0,
        }
    }

    /// Check if this tier represents a successful match
    pub fn is_matched(&self) -> bool {
        !matches!(self, Self::Tier4)
    }

    /// Get tier name for logging
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
}
```

**Tier Assignment Function:**
```rust
/// Assign confidence tier based on available metadata
pub fn assign_tier(
    has_embedded_mbid: bool,
    has_isrc: bool,
    acoustid_confidence: Option<f32>,
    album_match: bool,
) -> ConfidenceTier {
    if has_embedded_mbid {
        if has_isrc {
            ConfidenceTier::Tier1A
        } else {
            ConfidenceTier::Tier1B
        }
    } else if let Some(conf) = acoustid_confidence {
        if conf >= 0.95 {
            ConfidenceTier::Tier2A
        } else if conf >= 0.80 {
            ConfidenceTier::Tier2B
        } else {
            ConfidenceTier::Tier4
        }
    } else if album_match {
        ConfidenceTier::Tier3
    } else {
        ConfidenceTier::Tier4
    }
}
```

---

## Tests to Pass

- **TC-U-003:** Tier assignment with embedded MB ID + ISRC → Tier1A
- **TC-U-004:** Tier assignment with embedded MB ID only → Tier1B
- **TC-U-005:** Tier assignment with AcoustID high confidence → Tier2A
- **TC-U-006:** Tier assignment with no match → Tier4

---

## Acceptance Criteria

- [ ] `ConfidenceTier` enum created with all 6 tiers
- [ ] `confidence_score()` returns correct values
- [ ] `assign_tier()` correctly prioritizes embedded MB ID
- [ ] Unit tests pass for all tier assignment scenarios
- [ ] Code compiles with `cargo check -p wkmp-ai`

---

## Verification Command

```bash
cargo test -p wkmp-ai confidence_tier::tests
```
