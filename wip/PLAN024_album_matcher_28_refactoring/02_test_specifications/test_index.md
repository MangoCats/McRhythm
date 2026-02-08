# Test Specifications Index

**Plan:** PLAN024
**Specification:** SPEC_album_matcher_28_refactoring.md
**Phase:** 3 - Acceptance Test Definition
**Date:** 2025-11-25
**Status:** Phase 3 - Test Specifications

---

## Test Coverage Summary

**Total Requirements:** 22
**Total Tests:** 28 (some requirements have multiple test cases)

**Test Categories:**
- Functional Tests: 9 tests (REQ-FUNC-001 to REQ-FUNC-005)
- Structural Tests: 8 tests (REQ-STRUCT-001 to REQ-STRUCT-006)
- Build Tests: 4 tests (REQ-BUILD-001 to REQ-BUILD-003)
- Integration Tests: 4 tests (REQ-TEST-001 to REQ-TEST-003)
- Unit Tests: 1 test (REQ-TEST-004)
- Documentation Tests: 2 tests (REQ-DOC-001 to REQ-DOC-004)

---

## 1. Functional Tests (Algorithm Preservation)

### TEST-FUNC-001: Identical Results on Full Dataset

**Requirement:** REQ-FUNC-001
**Priority:** CRITICAL
**Type:** Integration Test

**Preconditions:**
1. album_matcher_28.rs (monolithic) compiled successfully
2. Baseline output exists: album_matcher_output_run28.txt
3. Refactored code compiled successfully
4. training_set.txt available (200 albums)

**Test Procedure:**
1. Run monolithic version: `cargo run --release --example album_matcher_28 -- --training-set training_set.txt --output baseline_run28.txt`
2. Run refactored version: `cargo run --release --example album_matcher_28 -- --training-set training_set.txt --output refactored_run28.txt`
3. Extract structured data from both outputs using analyze_best_params.py
4. Compare extracted data (album IDs, best parameters, success/failure status)

**Acceptance Criteria:**
- ✅ All 193 successful albums have identical "Best parameters" (threshold, min_duration)
- ✅ Same 5 albums fail with same error conditions
- ✅ Success/failure counts match exactly

**Test Data:**
- Input: training_set.txt (200 albums)
- Expected: 193 successes, 5 failures (based on Run 28 baseline)

**Failure Handling:**
- If any album differs: Investigate which refactoring phase introduced regression
- If parameters differ: Verify STAGE2 arrays preserved exactly
- If success/failure counts differ: Check stage execution logic

---

### TEST-FUNC-002a: Parameter Array Preservation (Thresholds)

**Requirement:** REQ-FUNC-002
**Priority:** CRITICAL
**Type:** Code Inspection

**Test Procedure:**
1. Inspect wkmp-ai/examples/am28/constants.rs
2. Locate STAGE2_THRESHOLD_VALUES array
3. Compare against baseline from album_matcher_28.rs

**Acceptance Criteria:**
- ✅ Array has exactly 12 values
- ✅ Values match Run 28 order exactly:
  ```rust
  [-50.0, -58.0, -60.0, -62.0, -56.0, -52.0,
   -54.0, -48.0, -40.0, -38.0, -34.0, -30.0]
  ```
- ✅ Array type is `[f64; 12]`

**Test Data:**
- Baseline: album_matcher_28.rs lines 812-825
- Expected: Exact match with Run 28 parameter ordering

---

### TEST-FUNC-002b: Parameter Array Preservation (Min Durations)

**Requirement:** REQ-FUNC-002
**Priority:** CRITICAL
**Type:** Code Inspection

**Test Procedure:**
1. Inspect wkmp-ai/examples/am28/constants.rs
2. Locate STAGE2_MIN_DURATION_VALUES array
3. Compare against baseline from album_matcher_28.rs

**Acceptance Criteria:**
- ✅ Array has exactly 15 values
- ✅ Values match Run 28 order exactly:
  ```rust
  [3.0, 2.0, 0.5, 1.5, 0.3, 0.8, 0.1, 4.0,
   0.2, 2.5, 1.0, 0.4, 5.0, 3.5, 0.05]
  ```
- ✅ Array type is `[f64; 15]`

**Test Data:**
- Baseline: album_matcher_28.rs lines 842-858
- Expected: Exact match with Run 28 parameter ordering

---

### TEST-FUNC-003: Early-Exit Behavior Verification

**Requirement:** REQ-FUNC-003
**Priority:** CRITICAL
**Type:** Behavioral Test

**Preconditions:**
1. Identify albums achieving 100% match from baseline (expected: 169 albums)
2. Enable debug logging for parameter testing

**Test Procedure:**
1. Run refactored version on 10 albums known to achieve 100% match
2. Parse logs to extract number of parameters tested before early-exit
3. Compare with expected exit ranks from analyze_early_exit_potential.py

**Acceptance Criteria:**
- ✅ All 10 test albums exit immediately upon 100% match
- ✅ Exit ranks match expected values (median: 4.0 params)
- ✅ No albums test all 180 parameters if 100% match found early

**Test Data:**
- Input: Subset of 10 albums from training_set.txt (known 100% matches)
- Expected: Early exit at ranks 1-20 (per empirical analysis)

**Verification Method:**
Parse log lines like:
```
[A123] Testing parameter 4/180: -58dB, 2.0s
[A123] Found 100% match, exiting early
```

---

### TEST-FUNC-004a: MusicBrainz Cache Read Compatibility

**Requirement:** REQ-FUNC-004
**Priority:** HIGH
**Type:** Integration Test

**Preconditions:**
1. Existing cache/ folder with Run 27/28 cache files
2. Run refactored code with --cache-mode readonly

**Test Procedure:**
1. Run refactored version: `cargo run --release --example album_matcher_28 -- --training-set training_set.txt --limit 10 --cache-mode readonly`
2. Verify cache hits in logs
3. Confirm no new cache files created

**Acceptance Criteria:**
- ✅ All cached albums hit cache (no live API calls)
- ✅ Cache files read successfully (no deserialization errors)
- ✅ No new cache files written (readonly mode respected)

**Test Data:**
- Input: First 10 albums from training_set.txt (should be cached)
- Expected: 10 cache hits, 0 cache misses

---

### TEST-FUNC-004b: MusicBrainz Cache Write Compatibility

**Requirement:** REQ-FUNC-004
**Priority:** HIGH
**Type:** Integration Test

**Preconditions:**
1. Empty cache/ folder or cache disabled
2. Run refactored code with --cache-mode readwrite

**Test Procedure:**
1. Run refactored version: `cargo run --release --example album_matcher_28 -- --training-set training_set.txt --limit 5 --cache-mode readwrite`
2. Verify cache files created in cache/ folder
3. Inspect cache file format (JSON structure)

**Acceptance Criteria:**
- ✅ Cache files created for all 5 albums
- ✅ JSON format matches Run 28 cache structure
- ✅ Subsequent run with --cache-mode readonly hits cache

**Test Data:**
- Input: 5 uncached albums
- Expected: 5 new cache files, valid JSON serialization

---

### TEST-FUNC-005a: Stage 2 Logic Preservation

**Requirement:** REQ-FUNC-005
**Priority:** CRITICAL
**Type:** Code Review + Behavioral Test

**Test Procedure:**
1. Code Review: Inspect am28/stages/stage2.rs for algorithm changes
2. Behavioral Test: Run on album known to use Stage 2 (180-param sweep)
3. Verify same parameters selected as baseline

**Acceptance Criteria:**
- ✅ Stage 2 algorithm unchanged (WindowDbProfile, silence detection)
- ✅ Test album selects same parameters as Run 28 baseline
- ✅ Early-exit logic present and functional

**Test Data:**
- Input: 1 album requiring Stage 2 (not single-track, valid MB data)
- Expected: Identical parameter selection

---

### TEST-FUNC-005b: Stage 3-5 Logic Preservation

**Requirement:** REQ-FUNC-005
**Priority:** CRITICAL
**Type:** Code Review

**Test Procedure:**
1. Code Review: Inspect am28/stages/stage3.rs, stage4.rs, stage5.rs
2. Verify DP assembly algorithm unchanged (stage3.rs)
3. Verify RMS profiling algorithm unchanged (stage4.rs)
4. Verify track merging algorithm unchanged (stage5.rs)

**Acceptance Criteria:**
- ✅ DP assembly logic identical to Run 28
- ✅ RMS profiling logic identical to Run 28
- ✅ Track merging logic identical to Run 28
- ✅ Complex algorithm comments preserved

**Test Data:**
- Baseline: album_matcher_28.rs stage implementations
- Expected: Identical algorithm logic (modulo refactoring structure)

---

### TEST-FUNC-005c: Validation Logic Preservation

**Requirement:** REQ-FUNC-005
**Priority:** CRITICAL
**Type:** Code Review + Behavioral Test

**Test Procedure:**
1. Code Review: Inspect am28/matching/validation.rs
2. Verify single-track discriminator logic unchanged
3. Test on album known to be single-track (should reject)
4. Test on album with artist/album name mismatch (should warn/reject)

**Acceptance Criteria:**
- ✅ Single-track discriminator logic identical
- ✅ Artist/album name validation logic identical
- ✅ Test albums handled identically to Run 28

**Test Data:**
- Input: 1-2 albums with validation edge cases
- Expected: Same validation outcomes as Run 28

---

## 2. Structural Tests (Modularity)

### TEST-STRUCT-001: Folder Structure Verification

**Requirement:** REQ-STRUCT-001
**Priority:** HIGH
**Type:** Filesystem Inspection

**Test Procedure:**
1. Check wkmp-ai/examples/am28/ folder exists
2. Verify presence of all expected files and subfolders
3. Check no unexpected files present

**Acceptance Criteria:**
- ✅ Folder wkmp-ai/examples/am28/ exists
- ✅ Files present: main.rs, mod.rs, types.rs, constants.rs, silence_detection.rs, README.md
- ✅ Subfolders present: musicbrainz/, stages/, matching/, utils/
- ✅ Each subfolder has mod.rs and expected modules

**Test Data:**
Expected structure:
```
am28/
├── main.rs
├── mod.rs
├── types.rs
├── constants.rs
├── silence_detection.rs
├── README.md
├── musicbrainz/
│   ├── mod.rs
│   ├── api.rs
│   ├── cache.rs
│   └── types.rs
├── stages/
│   ├── mod.rs
│   ├── stage2.rs
│   ├── stage3.rs
│   ├── stage4.rs
│   └── stage5.rs
├── matching/
│   ├── mod.rs
│   ├── candidate.rs
│   ├── edition.rs
│   └── validation.rs
└── utils/
    ├── mod.rs
    ├── audio.rs
    ├── fingerprint.rs
    └── timing.rs
```

---

### TEST-STRUCT-002a: Standalone Example Compilation

**Requirement:** REQ-STRUCT-002
**Priority:** CRITICAL
**Type:** Build Test

**Test Procedure:**
1. Run: `cargo run --example album_matcher_28 -- --help`
2. Verify help text displays correctly
3. Verify exit code 0

**Acceptance Criteria:**
- ✅ Command executes without errors
- ✅ Help text displays all CLI arguments (--training-set, --output, --limit, --cache-mode, etc.)
- ✅ Exit code 0

---

### TEST-STRUCT-002b: CLI Argument Preservation

**Requirement:** REQ-STRUCT-002
**Priority:** HIGH
**Type:** Functional Test

**Test Procedure:**
1. Run with various CLI argument combinations
2. Verify arguments parsed correctly

**Test Cases:**
```bash
# Test 1: All arguments
cargo run --example album_matcher_28 -- \
  --training-set training_set.txt \
  --output test_output.txt \
  --limit 10 \
  --cache-mode readonly \
  --stagger 5 \
  --mb-search-throttle 2

# Test 2: Minimal arguments
cargo run --example album_matcher_28 -- \
  --training-set training_set.txt

# Test 3: Invalid argument (should error)
cargo run --example album_matcher_28 -- --invalid-arg
```

**Acceptance Criteria:**
- ✅ Test 1: All arguments accepted, values used correctly
- ✅ Test 2: Defaults applied correctly
- ✅ Test 3: Error message displayed, exit code non-zero

---

### TEST-STRUCT-003: Module Domain Organization

**Requirement:** REQ-STRUCT-003
**Priority:** HIGH
**Type:** Code Review

**Test Procedure:**
1. Review each module file
2. Verify functionality matches documented domain
3. Check for misplaced functions

**Acceptance Criteria:**
- ✅ silence_detection.rs: Only silence detection logic
- ✅ musicbrainz/: Only MB API client, cache, types
- ✅ stages/: Only stage 2-5 execution logic
- ✅ matching/: Only candidate testing, edition selection, validation
- ✅ utils/: Only audio decoding, fingerprinting, timing utilities
- ✅ No module contains logic belonging to another domain

---

### TEST-STRUCT-004: Module Size Constraint

**Requirement:** REQ-STRUCT-004
**Priority:** MEDIUM
**Type:** Automated Check

**Test Procedure:**
1. Count lines in each .rs file under am28/
2. Verify all files <1000 lines

**Acceptance Criteria:**
- ✅ All .rs files in am28/ have <1000 lines
- ✅ If any file exceeds 1000 lines, split into smaller modules

**Test Script:**
```bash
find wkmp-ai/examples/am28 -name "*.rs" -exec wc -l {} \; | awk '{if ($1 >= 1000) print $2 " EXCEEDS 1000 lines: " $1}'
```

**Expected Output:** (empty, no files exceed limit)

---

### TEST-STRUCT-005: Explicit Dependencies Check

**Requirement:** REQ-STRUCT-005
**Priority:** HIGH
**Type:** Code Review

**Test Procedure:**
1. Review use statements in each module
2. Verify dependencies are explicit (no hidden coupling)
3. Check for circular dependencies

**Acceptance Criteria:**
- ✅ All module dependencies declared via `use` statements
- ✅ No circular dependencies (e.g., A uses B, B uses A)
- ✅ Function signatures show explicit parameter types (no hidden global state)

**Verification Method:**
- Manual code review of `use` statements
- Cargo compilation succeeds (would fail on circular deps)

---

### TEST-STRUCT-006: Shared Data Structures

**Requirement:** REQ-STRUCT-006
**Priority:** HIGH
**Type:** Code Review

**Test Procedure:**
1. Inspect am28/types.rs
2. Verify all shared data structures defined there
3. Check for duplicate definitions in other modules

**Acceptance Criteria:**
- ✅ types.rs contains: CandidateTestResult, OverSegmentedCandidate, SingleEditionStage2Results
- ✅ No duplicate struct/enum definitions across modules
- ✅ All modules import shared types from types.rs

**Verification Method:**
```bash
# Search for duplicate struct definitions
grep -r "struct CandidateTestResult" wkmp-ai/examples/am28/
# Should only appear in types.rs
```

---

## 3. Build Tests

### TEST-BUILD-001: Warning-Free Compilation

**Requirement:** REQ-BUILD-001
**Priority:** HIGH
**Type:** Build Test

**Test Procedure:**
1. Run: `cargo check --example album_matcher_28 2>&1 | tee build_warnings.txt`
2. Parse output for warnings
3. Verify zero warnings from am28/ code

**Acceptance Criteria:**
- ✅ Zero warnings from am28/ modules
- ✅ Any warnings are from dependencies (wkmp-common, external crates)
- ✅ Intentional `#[allow(...)]` attributes preserved

**Expected Output:**
```
Checking wkmp-ai v0.1.0 (...)
Finished dev [unoptimized + debuginfo] target(s) in X.XXs
```

---

### TEST-BUILD-002: Release Mode Compilation

**Requirement:** REQ-BUILD-002
**Priority:** HIGH
**Type:** Build Test

**Test Procedure:**
1. Run: `cargo build --release --example album_matcher_28`
2. Verify successful compilation
3. Check binary size comparable to baseline

**Acceptance Criteria:**
- ✅ Release compilation succeeds
- ✅ Binary size within ±10% of Run 28 baseline
- ✅ Binary executes correctly

**Verification Method:**
```bash
cargo build --release --example album_matcher_28
ls -lh target/release/examples/album_matcher_28
# Compare size with baseline binary
```

---

### TEST-BUILD-003a: Compilation Time Measurement

**Requirement:** REQ-BUILD-003
**Priority:** MEDIUM
**Type:** Performance Test

**Test Procedure:**
1. Run: `cargo clean`
2. Run: `cargo build --release --example album_matcher_28 --timings`
3. Review HTML report: target/cargo-timings/cargo-timing.html
4. Compare total time with baseline

**Acceptance Criteria:**
- ✅ Total compilation time ≤110% of Run 28 baseline
- ✅ No excessive template instantiation overhead visible
- ✅ Module parallelization visible in timings report

**Baseline:** (Measure during testing)

---

### TEST-BUILD-003b: Incremental Compilation

**Requirement:** REQ-BUILD-003
**Priority:** LOW
**Type:** Performance Test

**Test Procedure:**
1. Build once: `cargo build --example album_matcher_28`
2. Touch one module: `touch wkmp-ai/examples/am28/stages/stage2.rs`
3. Rebuild: `time cargo build --example album_matcher_28`
4. Verify only affected modules rebuild

**Acceptance Criteria:**
- ✅ Incremental rebuild faster than full rebuild (by 50%+ ideally)
- ✅ Only stage2.rs and dependent modules rebuild

---

## 4. Integration Tests

### TEST-INT-001: Full Dataset Validation

**Requirement:** REQ-TEST-001
**Priority:** CRITICAL
**Type:** Integration Test

**Test Procedure:**
1. Run refactored code on full training_set.txt (200 albums)
2. Compare outputs with Run 28 baseline
3. Use analyze_best_params.py to extract structured data

**Acceptance Criteria:**
- ✅ Exactly 193 albums succeed
- ✅ Exactly 5 albums fail (same albums as Run 28)
- ✅ All successful albums match with same parameters
- ✅ Output format matches Run 28 (parseable by existing scripts)

**Test Data:**
- Input: training_set.txt (200 albums)
- Baseline: album_matcher_output_run28.txt
- Comparison: Line-by-line for album IDs, parameters, success/failure

**Duration:** ~200.5s average × 200 albums = ~6.7 hours (parallelize if possible)

---

### TEST-INT-002: Parameter Match Verification

**Requirement:** REQ-TEST-002
**Priority:** CRITICAL
**Type:** Data Validation

**Test Procedure:**
1. Extract "Best parameters" from refactored output
2. Extract "Best parameters" from Run 28 baseline
3. Compare parameter-by-parameter for all 193 successful albums

**Acceptance Criteria:**
- ✅ All 193 albums report identical (threshold, min_duration)
- ✅ No parameter drift or rounding errors

**Test Script:**
```python
# Use analyze_best_params.py
baseline_params = extract_best_params("album_matcher_output_run28.txt")
refactored_params = extract_best_params("refactored_run28.txt")

for album_id in baseline_params.keys():
    assert baseline_params[album_id] == refactored_params[album_id], \
        f"Album {album_id} parameters differ"
```

---

### TEST-INT-003: Success Rate Preservation

**Requirement:** REQ-TEST-003
**Priority:** CRITICAL
**Type:** Statistical Test

**Test Procedure:**
1. Count successful albums in refactored output
2. Count failed albums in refactored output
3. Compare with Run 28 baseline

**Acceptance Criteria:**
- ✅ Exactly 193 successful albums
- ✅ Exactly 5 failed albums
- ✅ Same album IDs fail as Run 28

**Verification Method:**
```bash
# Count successes
grep "SUCCESS" refactored_run28.txt | wc -l  # Expect: 193

# Count failures
grep "FAILED" refactored_run28.txt | wc -l   # Expect: 5

# Identify failed albums
grep "FAILED" refactored_run28.txt | grep -oP '\[A\d+\]'
```

---

### TEST-INT-004: Performance Baseline

**Requirement:** REQ-TEST-001 (performance clause)
**Priority:** HIGH
**Type:** Performance Test

**Test Procedure:**
1. Run refactored code on 20 albums (subset)
2. Measure total execution time
3. Measure per-album average time
4. Compare with Run 28 baseline (200.5s average)

**Acceptance Criteria:**
- ✅ Per-album average within ±10% of 200.5s (180.5s - 220.5s range)
- ✅ No albums take >2x Run 28 time
- ✅ Performance regression acceptable if <10%

**Test Script:**
```bash
time cargo run --release --example album_matcher_28 -- \
  --training-set training_set.txt --limit 20 --output perf_test.txt

# Extract timing from logs or use analyze_timing.py
```

---

## 5. Unit Tests (Testability Demonstration)

### TEST-UNIT-001: Demonstrate Unit Testability

**Requirement:** REQ-TEST-004
**Priority:** MEDIUM
**Type:** Proof of Concept

**Test Procedure:**
1. Write 3 example unit tests for key modules
2. Execute tests with `cargo test --example album_matcher_28`
3. Verify tests pass

**Example Unit Tests:**

**Test 1: Candidate Matching Logic (matching/candidate.rs)**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_match_percentage() {
        // Mock candidate with 8/10 tracks matched
        let result = calculate_match_percentage(8, 10);
        assert_eq!(result, 80.0);
    }
}
```

**Test 2: Silence Detection (silence_detection.rs)**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_detection_threshold() {
        // Test vector: 5 samples with known dB levels
        let samples = vec![-60.0, -40.0, -55.0, -50.0, -45.0];
        let regions = find_silence_regions(&samples, -50.0, 1);
        // Expect samples 0, 2 to be silence (below -50dB)
        assert_eq!(regions.len(), 2);
    }
}
```

**Test 3: Parameter Ordering (stages/stage2.rs)**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::*;

    #[test]
    fn test_stage2_array_order_preserved() {
        // Verify STAGE2_THRESHOLD_VALUES first element is -50.0 (most common)
        assert_eq!(STAGE2_THRESHOLD_VALUES[0], -50.0);
        assert_eq!(STAGE2_MIN_DURATION_VALUES[0], 3.0);
    }
}
```

**Acceptance Criteria:**
- ✅ At least 3 unit tests compile and pass
- ✅ Tests demonstrate testability of: matching logic, silence detection, configuration
- ✅ Tests can be run via `cargo test --example album_matcher_28` (if Cargo supports)

---

## 6. Documentation Tests

### TEST-DOC-001: Module-Level Documentation

**Requirement:** REQ-DOC-001
**Priority:** HIGH
**Type:** Code Review

**Test Procedure:**
1. Check all .rs files in am28/ for module doc comments (`//!`)
2. Verify doc comments explain module purpose

**Acceptance Criteria:**
- ✅ All .rs files have `//!` module doc comment at top
- ✅ Doc comments explain module purpose and key responsibilities
- ✅ Cross-references to related modules included where appropriate

**Example Expected Documentation:**
```rust
//! # Silence Detection Module
//!
//! Provides silence detection functionality using WindowDbProfile for
//! efficient 180-parameter sweep in Stage 2.
//!
//! ## Key Types
//! - `WindowDbProfile`: Pre-computed dB levels for fast parameter testing
//!
//! ## Related Modules
//! - `stages::stage2`: Primary consumer of silence detection
```

---

### TEST-DOC-002: Function Documentation

**Requirement:** REQ-DOC-002
**Priority:** MEDIUM
**Type:** Automated Check

**Test Procedure:**
1. Run: `cargo doc --example album_matcher_28 --no-deps`
2. Verify documentation generates successfully
3. Review generated docs in target/doc/

**Acceptance Criteria:**
- ✅ Documentation builds without errors
- ✅ Public functions have `///` doc comments
- ✅ Parameters and return values documented

**Verification Method:**
```bash
cargo doc --example album_matcher_28 --no-deps --open
# Manual review of generated HTML documentation
```

---

### TEST-DOC-003: README Existence and Content

**Requirement:** REQ-DOC-003
**Priority:** HIGH
**Type:** Manual Review

**Test Procedure:**
1. Check am28/README.md exists
2. Review content for completeness

**Acceptance Criteria:**
- ✅ am28/README.md exists
- ✅ Documents module hierarchy (folder structure)
- ✅ Explains module responsibilities
- ✅ Provides navigation guide for developers

**Expected Sections:**
- Overview (purpose of refactoring)
- Module Structure (folder layout)
- Module Responsibilities (what each module does)
- Dependencies (which modules depend on which)
- Getting Started (how to compile and run)

---

### TEST-DOC-004: Algorithm Comment Preservation

**Requirement:** REQ-DOC-004
**Priority:** MEDIUM
**Type:** Code Review

**Test Procedure:**
1. Review stages/stage3.rs for DP assembly comments
2. Review stages/stage4.rs for RMS profiling comments
3. Verify comments preserved from album_matcher_28.rs

**Acceptance Criteria:**
- ✅ DP assembly algorithm comments preserved in stage3.rs
- ✅ RMS profiling algorithm comments preserved in stage4.rs
- ✅ No reduction in comment quality or clarity

**Verification Method:**
Manual side-by-side comparison of complex algorithm sections:
- album_matcher_28.rs (baseline)
- am28/stages/stage3.rs, am28/stages/stage4.rs (refactored)

---

## Test Execution Order

**Phase 4 (Structure Setup):**
1. TEST-STRUCT-001 (folder structure verification)

**Phase 5 (Types and Constants):**
2. TEST-FUNC-002a, TEST-FUNC-002b (parameter array preservation)
3. TEST-STRUCT-006 (shared data structures)

**Phase 6 (Core Modules):**
4. TEST-BUILD-001 (warning-free compilation after each module)
5. TEST-STRUCT-004 (module size constraint)

**Phase 7 (Stage Modules):**
6. TEST-FUNC-005a, TEST-FUNC-005b (stage logic preservation)

**Phase 8 (Matching and Orchestration):**
7. TEST-FUNC-005c (validation logic preservation)
8. TEST-STRUCT-002a, TEST-STRUCT-002b (CLI compilation)

**Phase 9 (Final Verification):**
9. TEST-BUILD-002 (release mode compilation)
10. TEST-BUILD-003a (compilation time)
11. TEST-FUNC-001 (identical results - full dataset)
12. TEST-INT-001, TEST-INT-002, TEST-INT-003 (full integration validation)
13. TEST-INT-004 (performance baseline)
14. TEST-FUNC-003, TEST-FUNC-004a, TEST-FUNC-004b (behavioral verification)
15. TEST-UNIT-001 (unit testability demonstration)
16. TEST-DOC-001, TEST-DOC-002, TEST-DOC-003, TEST-DOC-004 (documentation verification)

---

## Test Artifacts

**Generated During Testing:**
- baseline_run28.txt (from monolithic version)
- refactored_run28.txt (from refactored version)
- build_warnings.txt (compilation warnings log)
- perf_test.txt (performance test output)
- target/cargo-timings/cargo-timing.html (compilation timing report)

**Analysis Scripts:**
- analyze_best_params.py (extract "Best parameters")
- analyze_timing.py (measure component timing)
- analyze_early_exit_potential.py (verify early-exit behavior)

---

## Success Criteria

**Critical Tests (Must Pass):**
- TEST-FUNC-001 (identical results)
- TEST-FUNC-002a, TEST-FUNC-002b (parameter preservation)
- TEST-FUNC-005a, TEST-FUNC-005b, TEST-FUNC-005c (stage logic preservation)
- TEST-STRUCT-002a (standalone compilation)
- TEST-INT-001, TEST-INT-002, TEST-INT-003 (integration validation)

**High Priority Tests (Should Pass):**
- TEST-BUILD-001, TEST-BUILD-002 (build quality)
- TEST-INT-004 (performance baseline)
- TEST-STRUCT-001, TEST-STRUCT-003, TEST-STRUCT-005, TEST-STRUCT-006 (structural quality)

**Medium/Low Priority Tests (May Defer):**
- TEST-BUILD-003a (compilation time - informational)
- TEST-UNIT-001 (proof of concept only)
- TEST-DOC-002, TEST-DOC-004 (documentation quality)

**Pass Threshold:** All CRITICAL tests pass, ≥80% HIGH tests pass

---

## Document Control

**Version:** 1.0
**Status:** Phase 3 - Test Specifications
**Created:** 2025-11-25
**Last Updated:** 2025-11-25
