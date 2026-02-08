# TC-U-VAL-010: Jaro-Winkler Validation

**Requirement:** SSI-VAL-010 (Upgrade to Jaro-Winkler)
**Type:** Unit Test

---

## TC-U-VAL-010-01: Returns Value 0.0-1.0

**Given:**
- Two strings of any content

**When:**
- `jaro_winkler_similarity()` is called

**Then:**
- Returns a value between 0.0 (completely different) and 1.0 (identical)

**Verify:**
```rust
#[test]
fn test_jaro_winkler_range() {
    // Identical strings = 1.0
    assert_eq!(jaro_winkler_similarity("hello", "hello"), 1.0);

    // Completely different = close to 0.0
    let score = jaro_winkler_similarity("abc", "xyz");
    assert!(score >= 0.0 && score <= 1.0);
    assert!(score < 0.5);

    // Partially similar = between 0.0 and 1.0
    let score = jaro_winkler_similarity("hello", "hallo");
    assert!(score > 0.5 && score < 1.0);
}
```

**Pass Criteria:** All returned values in [0.0, 1.0] range
**Fail Criteria:** Any value outside range

---

## TC-U-VAL-010-02: Similar Strings Score > 0.85

**Given:**
- Pairs of strings that represent the same entity with minor variations

**When:**
- `jaro_winkler_similarity()` is called

**Then:**
- Similar strings score above title threshold (0.85)

**Verify:**
```rust
#[test]
fn test_jaro_winkler_similar_strings() {
    // Same with case difference
    assert!(jaro_winkler_similarity("Yesterday", "yesterday") > 0.85);

    // Minor spelling variation
    assert!(jaro_winkler_similarity("Strawberry Fields", "Strawberry Field") > 0.85);

    // With "The" prefix difference
    assert!(jaro_winkler_similarity("the beatles", "beatles") > 0.75);

    // Completely different should fail
    assert!(jaro_winkler_similarity("Yesterday", "Come Together") < 0.50);
}
```

**Pass Criteria:** Similar music titles/artists score above thresholds
**Fail Criteria:** False negatives on clearly similar strings
