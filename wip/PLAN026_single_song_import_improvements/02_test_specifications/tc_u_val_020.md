# TC-U-VAL-020: String Normalization

**Requirement:** SSI-VAL-020 (Normalize strings before comparison)
**Type:** Unit Test

---

## TC-U-VAL-020-01: Removes "The " Prefix

**Given:**
- Artist names with "The " prefix

**When:**
- `normalize_for_comparison()` is called

**Then:**
- "The " prefix is removed
- Comparison treats "The Beatles" same as "Beatles"

**Verify:**
```rust
#[test]
fn test_normalize_removes_the_prefix() {
    assert_eq!(normalize_for_comparison("The Beatles"), "beatles");
    assert_eq!(normalize_for_comparison("the rolling stones"), "rolling stones");
    assert_eq!(normalize_for_comparison("A Tribe Called Quest"), "tribe called quest");

    // "The" in middle should NOT be removed
    assert!(normalize_for_comparison("Over The Rainbow").contains("the"));
}
```

**Pass Criteria:** Leading articles removed, internal words preserved
**Fail Criteria:** "The" removed from middle of string

---

## TC-U-VAL-020-02: Folds Unicode to ASCII

**Given:**
- Artist/title names with accented characters

**When:**
- `normalize_for_comparison()` is called

**Then:**
- Unicode characters folded to ASCII equivalents

**Verify:**
```rust
#[test]
fn test_normalize_unicode_folding() {
    // German umlauts
    assert_eq!(normalize_for_comparison("Björk"), "bjork");
    assert_eq!(normalize_for_comparison("Motörhead"), "motorhead");

    // French accents
    assert_eq!(normalize_for_comparison("Édith Piaf"), "edith piaf");
    assert_eq!(normalize_for_comparison("Café del Mar"), "cafe del mar");

    // Spanish
    assert_eq!(normalize_for_comparison("Señorita"), "senorita");

    // Punctuation removed
    assert_eq!(normalize_for_comparison("P!nk"), "pink");
    assert_eq!(normalize_for_comparison("Guns N' Roses"), "guns n roses");
}
```

**Pass Criteria:** Common accented characters normalized correctly
**Fail Criteria:** Unicode characters cause comparison failures
