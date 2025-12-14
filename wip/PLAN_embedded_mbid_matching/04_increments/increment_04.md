# Increment 4: Add Logging and Metrics

**Increment:** 4 of 5
**Phase:** Implementation
**Estimated Effort:** 1-2 hours
**Confidence:** HIGH (±20%)
**Prerequisites:** Increment 3 (IdentityResolver update)

---

## Objective

Add structured logging for confidence tiers and metrics for monitoring Stage 0 effectiveness.

---

## Deliverables

1. **Modified:** `wkmp-ai/src/fusion/identity_resolver.rs`
   - Add tracing spans with tier information
   - Log tier distribution statistics

2. **Modified:** `wkmp-ai/src/matching/confidence_tier.rs`
   - Add `Display` trait implementation
   - Add serialization support

---

## Requirements Covered

- **FR-08:** Log confidence tier with match results (complete)

---

## Implementation Notes

**Logging Format:**
```rust
// Per-file logging
tracing::info!(
    file = %file_path.display(),
    tier = %result.confidence_tier,
    mbid = ?result.recording_mbid,
    confidence = %result.confidence,
    "Recording identification complete"
);

// Batch summary logging
tracing::info!(
    total = files_processed,
    tier_1a = tier_counts[ConfidenceTier::Tier1A],
    tier_1b = tier_counts[ConfidenceTier::Tier1B],
    tier_2a = tier_counts[ConfidenceTier::Tier2A],
    tier_2b = tier_counts[ConfidenceTier::Tier2B],
    tier_3 = tier_counts[ConfidenceTier::Tier3],
    tier_4 = tier_counts[ConfidenceTier::Tier4],
    "Batch identification summary"
);
```

**Display Trait:**
```rust
impl std::fmt::Display for ConfidenceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
```

**Serde Support:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceTier {
    #[serde(rename = "1a")]
    Tier1A,
    #[serde(rename = "1b")]
    Tier1B,
    // ...
}
```

---

## Tests to Pass

- **TC-U-007:** Display trait outputs correct tier names
- **TC-U-008:** Serialization/deserialization roundtrip works

---

## Acceptance Criteria

- [ ] Confidence tier logged with every identification result
- [ ] Batch summaries include tier distribution
- [ ] `ConfidenceTier` implements Display and Serialize
- [ ] Log output is parseable (structured JSON in production)

---

## Verification Command

```bash
RUST_LOG=wkmp_ai=debug cargo test -p wkmp-ai -- --nocapture 2>&1 | grep -i tier
```
