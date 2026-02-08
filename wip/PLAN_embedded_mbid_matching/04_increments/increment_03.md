# Increment 3: Update IdentityResolver for Stage 0

**Increment:** 3 of 5
**Phase:** Implementation
**Estimated Effort:** 2-3 hours
**Confidence:** HIGH (±25%)
**Prerequisites:** Increment 2 (ConfidenceTier)

---

## Objective

Modify `IdentityResolver` to prioritize embedded MB Recording IDs (Stage 0) before falling back to AcoustID fingerprinting.

---

## Deliverables

1. **Modified:** `wkmp-ai/src/fusion/identity_resolver.rs`
   - Add Stage 0 check before Bayesian fusion
   - Integrate ConfidenceTier into resolution result
   - Skip AcoustID when embedded MB ID present

2. **Modified:** `wkmp-ai/src/types.rs` (if needed)
   - Add `confidence_tier` field to `FusedIdentity`

---

## Requirements Covered

- **FR-05:** Skip AcoustID when embedded MB ID present (complete)
- **FR-06:** Fall back to AcoustID when no embedded ID (complete)

---

## Implementation Notes

**Current Flow (to modify):**
```
1. Run all extractors in parallel (ID3, AcoustID, etc.)
2. Collect identity extractions
3. Bayesian fusion to select best MBID
```

**New Flow:**
```
1. Run ID3Extractor first
2. IF embedded MB ID present:
     - Return immediately with Tier 1A/1B
     - Skip AcoustID (saves API call + latency)
3. ELSE:
     - Run AcoustID extractor
     - Use existing Bayesian fusion
     - Assign Tier 2A/2B/3/4
```

**Key Code Changes:**

```rust
impl IdentityResolver {
    /// Check for Stage 0 match (embedded MB ID)
    fn check_stage0(&self, id3_metadata: &MetadataExtraction) -> Option<FusedIdentity> {
        // Check for embedded MB Recording ID
        let recording_mbid = id3_metadata.recording_mbid.as_ref()?;

        // Validate MBID format
        if !is_valid_mbid(&recording_mbid.value) {
            return None;
        }

        // Determine tier based on ISRC presence
        let has_isrc = id3_metadata.isrc.is_some();
        let tier = if has_isrc {
            ConfidenceTier::Tier1A
        } else {
            ConfidenceTier::Tier1B
        };

        Some(FusedIdentity {
            recording_mbid: Some(recording_mbid.value.clone()),
            confidence: tier.confidence_score(),
            posterior_probability: tier.confidence_score(),
            confidence_tier: tier,
            conflicts: vec![],
            contributing_sources: vec!["ID3-Embedded".to_string()],
        })
    }

    /// Main resolution entry point (updated)
    pub async fn resolve(
        &self,
        id3_metadata: &MetadataExtraction,
        run_acoustid: impl Future<Output = Option<IdentityExtraction>>,
    ) -> Result<FusedIdentity, FusionError> {
        // Stage 0: Check embedded MB ID first
        if let Some(stage0_result) = self.check_stage0(id3_metadata) {
            tracing::debug!(
                tier = %stage0_result.confidence_tier.name(),
                mbid = ?stage0_result.recording_mbid,
                "Stage 0 match found, skipping AcoustID"
            );
            return Ok(stage0_result);
        }

        // Stage 1+: Fall back to existing AcoustID pipeline
        let acoustid_result = run_acoustid.await;
        self.fuse_with_acoustid(id3_metadata, acoustid_result)
    }
}
```

**Update FusedIdentity struct:**
```rust
pub struct FusedIdentity {
    pub recording_mbid: Option<String>,
    pub confidence: f32,
    pub posterior_probability: f32,
    pub confidence_tier: ConfidenceTier,  // NEW
    pub conflicts: Vec<String>,
    pub contributing_sources: Vec<String>,
}
```

---

## Tests to Pass

- **TC-I-001:** Stage 0 returns immediately when embedded MB ID present
- **TC-I-002:** AcoustID skipped when Stage 0 succeeds (verify no API call)
- **TC-I-003:** Falls back to AcoustID when no embedded MB ID

---

## Acceptance Criteria

- [ ] `check_stage0()` method added to IdentityResolver
- [ ] `resolve()` checks Stage 0 before running AcoustID
- [ ] `FusedIdentity` includes `confidence_tier` field
- [ ] Integration tests pass
- [ ] AcoustID API calls reduced for files with embedded IDs

---

## Verification Command

```bash
cargo test -p wkmp-ai identity_resolver::tests
```
