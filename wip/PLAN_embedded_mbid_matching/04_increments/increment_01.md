# Increment 1: Add ISRC Extraction to ID3Extractor

**Increment:** 1 of 5
**Phase:** Implementation
**Estimated Effort:** 1-2 hours
**Confidence:** HIGH (±20%)
**Prerequisites:** None

---

## Objective

Add ISRC (International Standard Recording Code) extraction to the existing `ID3Extractor` to enable Tier 1A confidence classification.

---

## Deliverables

1. **Modified:** `wkmp-ai/src/extractors/id3_extractor.rs`
   - Add `extract_isrc()` method
   - Add ISRC to `MetadataExtraction` output
   - Add unit tests for ISRC extraction

---

## Requirements Covered

- **FR-02:** Extract ISRC from ID3 tags (complete)

---

## Implementation Notes

**ISRC Location in ID3 Tags:**
```rust
// ISRC stored in TSRC frame (ID3v2)
ItemKey::Isrc  // lofty's standard key
```

**ISRC Format:**
- 12 characters: CC-XXX-YY-NNNNN
- Country code (2), Registrant (3), Year (2), Designation (5)
- May be stored with or without hyphens

**Code Pattern (follow existing style):**
```rust
fn extract_isrc(&self, tag: &Tag) -> Option<ConfidenceValue<String>> {
    tag.get_string(&ItemKey::Isrc)
        .map(|isrc| {
            // Normalize: remove hyphens if present
            let normalized = isrc.replace("-", "");
            ConfidenceValue::new(normalized, self.base_confidence, "ID3-ISRC")
        })
}
```

**Update MetadataExtraction struct:**
```rust
pub struct MetadataExtraction {
    pub title: Option<ConfidenceValue<String>>,
    pub artist: Option<ConfidenceValue<String>>,
    pub album: Option<ConfidenceValue<String>>,
    pub recording_mbid: Option<ConfidenceValue<String>>,
    pub isrc: Option<ConfidenceValue<String>>,  // NEW
    pub additional: HashMap<String, ConfidenceValue<String>>,
}
```

---

## Tests to Pass

- **TC-U-001:** ISRC extraction from valid ID3 tags
- **TC-U-002:** ISRC normalization (hyphens removed)

---

## Acceptance Criteria

- [ ] `extract_isrc()` method added to `ID3Extractor`
- [ ] ISRC field added to `MetadataExtraction` struct
- [ ] ISRC normalized to 12-char format (no hyphens)
- [ ] Unit tests pass: `cargo test -p wkmp-ai id3_extractor`
- [ ] No regression in existing ID3 extraction

---

## Verification Command

```bash
cargo test -p wkmp-ai id3_extractor::tests
```
