# Increment 1: String Similarity Utilities

**Estimated Effort:** 2-3 hours
**Dependencies:** None (foundational)
**Tests:** TC-U-VAL-010-01, TC-U-VAL-010-02, TC-U-VAL-020-01, TC-U-VAL-020-02

---

## Objective

Port string similarity and normalization functions from am29 examples to main wkmp-ai crate.

---

## Deliverables

### 1. New File: `src/utils/string_similarity.rs`

```rust
//! String similarity utilities for metadata matching
//!
//! Provides Jaro-Winkler similarity and string normalization for
//! comparing artist/title metadata against MusicBrainz records.

use strsim::jaro_winkler;

/// Normalize string for comparison
///
/// Transformations:
/// - Lowercase
/// - Remove leading "The ", "A ", "An "
/// - Remove punctuation
/// - Fold Unicode to ASCII
/// - Collapse whitespace
pub fn normalize_for_comparison(s: &str) -> String {
    // Port from examples/am29/matching/validation.rs:227
    // ...
}

/// Calculate Jaro-Winkler similarity between two strings
///
/// Returns value 0.0 (different) to 1.0 (identical)
pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f64 {
    let norm1 = normalize_for_comparison(s1);
    let norm2 = normalize_for_comparison(s2);
    jaro_winkler(&norm1, &norm2)
}

/// Find best similarity from candidate to any source variant
pub fn best_similarity(candidate: &str, sources: &[String]) -> f64 {
    // Port from examples/am29/matching/validation.rs:147
    // ...
}
```

### 2. Update: `src/utils/mod.rs`

Add module declaration:
```rust
pub mod string_similarity;
```

### 3. Tests in `src/utils/string_similarity.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_removes_the_prefix() { ... }

    #[test]
    fn test_normalize_unicode_folding() { ... }

    #[test]
    fn test_jaro_winkler_range() { ... }

    #[test]
    fn test_jaro_winkler_similar_strings() { ... }
}
```

---

## Files Modified

| File | Action | Lines |
|------|--------|-------|
| `src/utils/string_similarity.rs` | Create | ~150 |
| `src/utils/mod.rs` | Modify | +1 |

---

## Verification

- [ ] TC-U-VAL-010-01 passes (returns 0.0-1.0)
- [ ] TC-U-VAL-010-02 passes (similar strings > 0.85)
- [ ] TC-U-VAL-020-01 passes (removes "The " prefix)
- [ ] TC-U-VAL-020-02 passes (Unicode folding)
- [ ] `cargo test --lib string_similarity` passes
- [ ] No clippy warnings

---

## Success Criteria

- String normalization handles all test cases
- Jaro-Winkler returns expected values for known test pairs
- Functions are public and documented
