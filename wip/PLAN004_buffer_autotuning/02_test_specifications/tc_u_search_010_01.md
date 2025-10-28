# TC-U-SEARCH-010-01: Binary Search Convergence

**Requirement:** TUNE-SEARCH-010 (lines 185-205)
**Test Type:** Unit Test
**Priority:** High
**Estimated Effort:** 30 minutes

---

## Test Objective

Verify that the binary search algorithm correctly finds the minimum stable buffer size and converges within 128 frames of the boundary.

---

## Scope

**Component Under Test:** `wkmp-ap/src/tuning/search.rs::binary_search_min_buffer()`

**Test Coverage:**
- Binary search logic correctness
- Convergence criterion (high - low ≤ 128)
- Returns smallest stable buffer size
- Handles boundary cases (all stable, all unstable, exact boundary)

---

## Test Specification

### Given: Mock test function that returns stability based on buffer size

```rust
// Mock: Buffers ≥384 are stable, <384 are unstable
fn mock_test_config(interval_ms: u64, buffer_size: u32) -> TestResult {
    TestResult {
        underrun_rate: if buffer_size >= 384 { 0.05 } else { 1.5 },
        verdict: if buffer_size >= 384 { Verdict::Stable } else { Verdict::Unstable },
    }
}
```

### When: Binary search executed with mocked test function

```rust
let min_stable = binary_search_min_buffer(
    interval_ms: 10,
    low: 64,
    high: 4096,
    test_fn: mock_test_config
);
```

### Then: Algorithm finds minimum stable buffer within convergence threshold

**Expected Behavior:**
1. Search starts with low=64, high=4096
2. Mid = 2080, test returns stable → high = 2080
3. Mid = 1072, test returns stable → high = 1072
4. Mid = 568, test returns stable → high = 568
5. Mid = 316, test returns unstable → low = 317
6. Mid = 442, test returns stable → high = 442
7. Mid = 379, test returns unstable → low = 380
8. Mid = 411, test returns stable → high = 411
9. Convergence: high - low = 31 ≤ 128 → return 411

**Expected Result:** Returns value in range [384, 512] (within 128 frames of true boundary 384)

---

## Verify

### Assertions

```rust
let result = binary_search_min_buffer(10, 64, 4096, mock_test_config);

// Primary assertion: Result is within convergence threshold of true boundary
assert!(result >= 384, "Result {} too small (below stability boundary)", result);
assert!(result <= 512, "Result {} too large (beyond convergence threshold)", result);

// Verify result is actually stable
let verification = mock_test_config(10, result);
assert_eq!(verification.verdict, Verdict::Stable, "Returned buffer size must be stable");

// Verify convergence efficiency (should take ~6-8 iterations)
let iterations = search_stats.iteration_count;
assert!(iterations <= 10, "Too many iterations: {}", iterations);
```

### Pass Criteria

- ✓ Returns buffer size within 128 frames of stability boundary
- ✓ Returned value tests as stable
- ✓ Converges in reasonable number of iterations (≤10)
- ✓ No panics or errors during search

### Fail Criteria

- ✗ Returns buffer size >128 frames away from boundary
- ✗ Returned value tests as unstable
- ✗ Infinite loop (doesn't converge)
- ✗ Panic or error

---

## Edge Cases

### Edge Case 1: All buffer sizes stable

```rust
fn all_stable(interval_ms: u64, buffer_size: u32) -> TestResult {
    TestResult { underrun_rate: 0.0, verdict: Verdict::Stable }
}

let result = binary_search_min_buffer(10, 64, 4096, all_stable);
assert_eq!(result, 64, "Should return minimum (64) when all stable");
```

### Edge Case 2: All buffer sizes unstable

```rust
fn all_unstable(interval_ms: u64, buffer_size: u32) -> TestResult {
    TestResult { underrun_rate: 5.0, verdict: Verdict::Unstable }
}

let result = binary_search_min_buffer(10, 64, 4096, all_unstable);
assert_eq!(result, 4096, "Should return maximum (4096) when all unstable");
```

### Edge Case 3: Boundary at minimum

```rust
fn boundary_at_min(interval_ms: u64, buffer_size: u32) -> TestResult {
    TestResult {
        underrun_rate: if buffer_size >= 64 { 0.0 } else { 5.0 },
        verdict: if buffer_size >= 64 { Verdict::Stable } else { Verdict::Unstable },
    }
}

let result = binary_search_min_buffer(10, 64, 4096, boundary_at_min);
assert_eq!(result, 64, "Should return minimum (64) when boundary is at min");
```

### Edge Case 4: Boundary at maximum

```rust
fn boundary_at_max(interval_ms: u64, buffer_size: u32) -> TestResult {
    TestResult {
        underrun_rate: if buffer_size >= 4096 { 0.0 } else { 5.0 },
        verdict: if buffer_size >= 4096 { Verdict::Stable } else { Verdict::Unstable },
    }
}

let result = binary_search_min_buffer(10, 64, 4096, boundary_at_max);
assert_eq!(result, 4096, "Should return maximum (4096) when boundary is at max");
```

---

## Test Data

**Input Ranges:**
- interval_ms: 1-100 (test with 10 as typical)
- low: 64 (minimum buffer size)
- high: 4096 (maximum search range)

**Mock Stability Boundaries:**
- Test 1: Boundary at 384 (middle range)
- Test 2: Boundary at 64 (minimum)
- Test 3: Boundary at 4096 (maximum)
- Test 4: All stable (no boundary)
- Test 5: All unstable (no stability)

---

## Implementation Notes

### Convergence Logic

```rust
pub fn binary_search_min_buffer<F>(
    interval_ms: u64,
    mut low: u32,
    mut high: u32,
    test_fn: F,
) -> u32
where
    F: Fn(u64, u32) -> TestResult,
{
    let mut best_stable = high; // Start with maximum as fallback

    while (high - low) > 128 {
        let mid = (low + high) / 2;
        let result = test_fn(interval_ms, mid);

        if result.verdict == Verdict::Stable {
            best_stable = mid;
            high = mid; // Try smaller
        } else {
            low = mid + 1; // Need larger
        }
    }

    best_stable
}
```

### Test Harness

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_search_convergence() {
        // Test case 1: Boundary at 384
        fn mock_boundary_384(_interval_ms: u64, buffer_size: u32) -> TestResult {
            TestResult {
                underrun_rate: if buffer_size >= 384 { 0.05 } else { 1.5 },
                verdict: if buffer_size >= 384 { Verdict::Stable } else { Verdict::Unstable },
            }
        }

        let result = binary_search_min_buffer(10, 64, 4096, mock_boundary_384);

        assert!(result >= 384, "Result {} below stability boundary", result);
        assert!(result <= 512, "Result {} beyond convergence threshold", result);

        // Verify result is stable
        let verification = mock_boundary_384(10, result);
        assert_eq!(verification.verdict, Verdict::Stable);
    }

    #[test]
    fn test_binary_search_all_stable() {
        fn all_stable(_interval_ms: u64, _buffer_size: u32) -> TestResult {
            TestResult { underrun_rate: 0.0, verdict: Verdict::Stable }
        }

        let result = binary_search_min_buffer(10, 64, 4096, all_stable);
        assert_eq!(result, 64, "Should return minimum when all stable");
    }

    #[test]
    fn test_binary_search_all_unstable() {
        fn all_unstable(_interval_ms: u64, _buffer_size: u32) -> TestResult {
            TestResult { underrun_rate: 5.0, verdict: Verdict::Unstable }
        }

        let result = binary_search_min_buffer(10, 64, 4096, all_unstable);
        assert_eq!(result, 4096, "Should return maximum when all unstable");
    }
}
```

---

## Dependencies

**Code Dependencies:**
- `wkmp-ap/src/tuning/search.rs` (implementation)
- `wkmp-ap/src/tuning/metrics.rs` (TestResult, Verdict types)

**No External Dependencies:** Pure unit test with mocked data

---

## Traceability

**Requirement:** TUNE-SEARCH-010
**Related Tests:** TC-U-SEARCH-020-01, TC-U-SEARCH-020-02 (early termination)
**Validates:** Binary search algorithm correctness and convergence
