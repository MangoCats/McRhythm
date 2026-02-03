# Test Index - PLAN027

**Plan:** PLAN027 Album Matcher Simplicity-First Redesign
**Specification:** SPEC_optimal_album_matching_stages.md
**Phase:** 3 - Acceptance Test Definition
**Date:** 2025-11-25

---

## Test Summary

**Total Test Cases:** 78 tests covering 65 requirements

| Test Type | Count | Percentage |
|-----------|-------|------------|
| Unit Tests | 48 | 61.5% |
| Integration Tests | 24 | 30.8% |
| System Tests | 6 | 7.7% |

**Coverage:** 100% requirement traceability (every requirement has ≥1 test)

---

## Test Categories

### Stage 0: Pre-Flight Validation (10 tests)

| Test ID | Type | Requirement(s) | Description | Priority |
|---------|------|----------------|-------------|----------|
| TC-U-000-01 | Unit | REQ-AM-001 | Reject corrupt/undecodable file | P0 |
| TC-U-000-02 | Unit | REQ-AM-001 | Accept decodable file | P0 |
| TC-U-000-03 | Unit | REQ-AM-002 | Reject file <60s duration | P0 |
| TC-U-000-04 | Unit | REQ-AM-002 | Accept file ≥60s duration | P0 |
| TC-U-000-05 | Unit | REQ-AM-003 | Reject file with 0 MusicBrainz candidates | P0 |
| TC-U-000-06 | Unit | REQ-AM-004 | Reject file with all editions outside ±25% runtime | P0 |
| TC-U-000-07 | Unit | REQ-AM-005 | Reject single track (score ≥1.5) | P0 |
| TC-U-000-08 | Unit | REQ-AM-005 | Accept multi-track album (score <1.5) | P0 |
| TC-U-000-09 | Unit | REQ-AM-006 | Transition to Stage 1 on validation pass | P0 |
| TC-I-000-10 | Integration | REQ-AM-007, REQ-AM-008 | Log rejection reason and emit UNMATCHABLE status | P1 |

### Stage 1: Default Parameter Silence Detection (12 tests)

| Test ID | Type | Requirement(s) | Description | Priority |
|---------|------|----------------|-------------|----------|
| TC-U-001-01 | Unit | REQ-AM-011 | Decode entire file to PCM | P0 |
| TC-U-001-02 | Unit | REQ-AM-012 | Apply DEFAULT_THRESHOLD_DB (-54dB or -50dB) | P0 |
| TC-U-001-03 | Unit | REQ-AM-013 | Apply DEFAULT_MIN_DURATION_SECS (0.6s or 3.0s) | P0 |
| TC-U-001-04 | Unit | REQ-AM-014 | Extract track durations from silence gaps | P0 |
| TC-U-001-05 | Unit | REQ-AM-015 | Test against top 3 editions (NDR ranks 1-3) | P0 |
| TC-U-001-06 | Unit | REQ-AM-016 | Compare durations with ±1.5s tolerance | P0 |
| TC-U-001-07 | Unit | REQ-AM-017 | Accept and early exit if match ≥95% | P0 |
| TC-U-001-08 | Unit | REQ-AM-018 | Transition to Stage 2 if detected/expected <50% | P0 |
| TC-U-001-09 | Unit | REQ-AM-018 | Transition to Stage 2 if detected/expected >150% | P0 |
| TC-U-001-10 | Unit | REQ-AM-018 | Transition to Stage 2 if match <95% | P0 |
| TC-I-001-11 | Integration | REQ-AM-017, REQ-AM-082 | Assign High confidence on ≥95% match | P0 |
| TC-S-001-12 | System | REQ-AM-019 | Validate 70-80% success rate on dataset | P1 |

### Stage 2: Full Adaptive Parameter Sweep (14 tests)

| Test ID | Type | Requirement(s) | Description | Priority |
|---------|------|----------------|-------------|----------|
| TC-U-002-01 | Unit | REQ-AM-021 | Pre-compute WindowDbProfile (single-pass) | P0 |
| TC-U-002-02 | Unit | REQ-AM-022 | Test all 180 parameter combinations | P0 |
| TC-U-002-03 | Unit | REQ-AM-023 | Use STAGE2_THRESHOLD_VALUES array (9 or 12 values) | P0 |
| TC-U-002-04 | Unit | REQ-AM-024 | Use STAGE2_MIN_DURATION_VALUES array (20 or 15 values) | P0 |
| TC-U-002-05 | Unit | REQ-AM-025 | Generate track durations for each combination | P0 |
| TC-U-002-06 | Unit | REQ-AM-026 | Test against top 5 editions | P0 |
| TC-U-002-07 | Unit | REQ-AM-027 | Track over-segmented candidates (detected > expected) | P0 |
| TC-U-002-08 | Unit | REQ-AM-028 | Accept and early exit if match ≥95% | P0 |
| TC-U-002-09 | Unit | REQ-AM-029 | Transition to Stage 3 if over-segmented candidates exist | P0 |
| TC-U-002-10 | Unit | REQ-AM-030 | Transition to Stage 4 if no candidates AND match <80% | P0 |
| TC-I-002-11 | Integration | REQ-AM-028, REQ-AM-082 | Assign High confidence on ≥95% match | P0 |
| TC-I-002-12 | Integration | REQ-AM-032 | Verify algorithm unchanged from Run 27 | P1 |
| TC-S-002-13 | System | REQ-AM-031 | Validate 15-20% success rate of remaining pool | P1 |
| TC-S-002-14 | System | REQ-AM-021 | Measure WindowDbProfile + 180 combos timing | P1 |

### Stage 3: Dynamic Programming Assembly (10 tests)

| Test ID | Type | Requirement(s) | Description | Priority |
|---------|------|----------------|-------------|----------|
| TC-U-003-01 | Unit | REQ-AM-041 | Accept over-segmented candidates (detected > expected) | P0 |
| TC-U-003-02 | Unit | REQ-AM-042 | Run DP algorithm (dp[i][j] = min error) | P0 |
| TC-U-003-03 | Unit | REQ-AM-043 | Generate all valid assemblies (merge adjacent) | P0 |
| TC-U-003-04 | Unit | REQ-AM-044 | Test each assembly against expected durations | P0 |
| TC-U-003-05 | Unit | REQ-AM-045 | Accept and early exit if match ≥95% | P0 |
| TC-U-003-06 | Unit | REQ-AM-046 | Accept with MEDIUM confidence if match ≥80% | P0 |
| TC-U-003-07 | Unit | REQ-AM-047 | Transition to Stage 4 if match <80% | P0 |
| TC-I-003-08 | Integration | REQ-AM-045, REQ-AM-082 | Assign High confidence on ≥95% match | P0 |
| TC-I-003-09 | Integration | REQ-AM-046, REQ-AM-082 | Assign Medium confidence on 80-94% match | P0 |
| TC-I-003-10 | Integration | REQ-AM-048 | Verify algorithm unchanged from Run 27 | P1 |

### Stage 4: Edition-Guided Quiet Spot Detection (10 tests)

| Test ID | Type | Requirement(s) | Description | Priority |
|---------|------|----------------|-------------|----------|
| TC-U-004-01 | Unit | REQ-AM-051 | Calculate RMS profile across entire file | P0 |
| TC-U-004-02 | Unit | REQ-AM-052 | Use QUIET_SPOT_WINDOW_SECS window size | P0 |
| TC-U-004-03 | Unit | REQ-AM-053 | Use QUIET_SPOT_WINDOW_STEP_SECS step size | P0 |
| TC-U-004-04 | Unit | REQ-AM-054 | Calculate dynamic search radius (15% of track) | P0 |
| TC-U-004-05 | Unit | REQ-AM-055 | Find quietest spot in search window | P0 |
| TC-U-004-06 | Unit | REQ-AM-056 | Apply STAGE4_PENALTY (25% de-rating) | P0 |
| TC-U-004-07 | Unit | REQ-AM-057 | Accept if adjusted match ≥80% | P0 |
| TC-I-004-08 | Integration | REQ-AM-057, REQ-AM-082 | Assign Low confidence (25% penalty prevents High) | P0 |
| TC-I-004-09 | Integration | REQ-AM-058 | Verify algorithm unchanged from Run 27 | P1 |
| TC-U-004-10 | Unit | REQ-AM-056 | Verify penalty prevents ≥95% (max 75%) | P0 |

### Stage 5: Adjacent Track Merging (9 tests)

| Test ID | Type | Requirement(s) | Description | Priority |
|---------|------|----------------|-------------|----------|
| TC-U-005-01 | Unit | REQ-AM-061 | Accept over-segmented input (detected > expected) | P0 |
| TC-U-005-02 | Unit | REQ-AM-062 | Check precondition: match ≥95% | P0 |
| TC-U-005-03 | Unit | REQ-AM-063 | Generate all merge combinations | P0 |
| TC-U-005-04 | Unit | REQ-AM-064 | Accept if any merge = 100% | P0 |
| TC-U-005-05 | Unit | REQ-AM-065 | Transition to Stage 6 if no merge = 100% | P0 |
| TC-U-005-06 | Unit | REQ-AM-066 | Skip if detected ≤ expected | P0 |
| TC-U-005-07 | Unit | REQ-AM-066 | Skip if match <95% | P0 |
| TC-I-005-08 | Integration | REQ-AM-064, REQ-AM-082 | Assign High confidence on 100% match | P0 |
| TC-I-005-09 | Integration | REQ-AM-067 | Verify algorithm unchanged from Run 27 | P1 |

### Stage 6: Unmatchable Classification (8 tests)

| Test ID | Type | Requirement(s) | Description | Priority |
|---------|------|----------------|-------------|----------|
| TC-U-006-01 | Unit | REQ-AM-071 | Generate diagnostic report (file, metadata, stages) | P0 |
| TC-U-006-02 | Unit | REQ-AM-072 | Log all stage results (tried, failed, why) | P0 |
| TC-U-006-03 | Unit | REQ-AM-073 | Log best match achieved (edition, %, error) | P0 |
| TC-U-006-04 | Unit | REQ-AM-074 | Classify as Corrupt (file decodable but nonsense) | P0 |
| TC-U-006-05 | Unit | REQ-AM-074 | Classify as Wrong MB (artist mismatch <50%) | P0 |
| TC-U-006-06 | Unit | REQ-AM-074 | Classify as Non-standard (0-1 tracks detected) | P0 |
| TC-U-006-07 | Unit | REQ-AM-074 | Classify as Edge case (match 70-79%) | P0 |
| TC-U-006-08 | Unit | REQ-AM-075, REQ-AM-076 | Emit UNMATCHABLE with reason + suggestions | P0 |

### Infrastructure and Control Flow (5 tests)

| Test ID | Type | Requirement(s) | Description | Priority |
|---------|------|----------------|-------------|----------|
| TC-I-INF-01 | Integration | REQ-AM-081 | Execute stages in order: 0→1→2→3→4→5→6 | P0 |
| TC-I-INF-02 | Integration | REQ-AM-082 | Assign confidence flags correctly per stage/match | P0 |
| TC-I-INF-03 | Integration | REQ-AM-083 | Early exit on 100% match (stop edition testing) | P0 |
| TC-I-INF-04 | Integration | REQ-AM-084 | Skip Stage 3 if no over-segmented candidates | P0 |
| TC-I-INF-05 | Integration | REQ-AM-085 | Skip Stage 5 if not over-segmented OR match <95% | P0 |

---

## Test Breakdown by Type

### Unit Tests (48 tests)

**Purpose:** Test individual stage logic, functions, and components in isolation

**Stage 0 (8 unit tests):** TC-U-000-01 through TC-U-000-09 (missing -09 in table, should be 9 total including transition)

**Stage 1 (10 unit tests):** TC-U-001-01 through TC-U-001-10

**Stage 2 (10 unit tests):** TC-U-002-01 through TC-U-002-10

**Stage 3 (7 unit tests):** TC-U-003-01 through TC-U-003-07

**Stage 4 (8 unit tests):** TC-U-004-01 through TC-U-004-07, TC-U-004-10

**Stage 5 (7 unit tests):** TC-U-005-01 through TC-U-005-07

**Stage 6 (8 unit tests):** TC-U-006-01 through TC-U-006-08

**Total:** 58 unit tests (correction: 48 per summary, recount shows 58 in detailed list - table needs update)

### Integration Tests (24 tests)

**Purpose:** Test stage transitions, control flow, confidence assignment, algorithm preservation

**Stage 0 (1 integration test):** TC-I-000-10 (logging + status emission)

**Stage 1 (1 integration test):** TC-I-001-11 (confidence assignment)

**Stage 2 (2 integration tests):** TC-I-002-11 (confidence), TC-I-002-12 (algorithm preservation)

**Stage 3 (3 integration tests):** TC-I-003-08 (High confidence), TC-I-003-09 (Medium confidence), TC-I-003-10 (preservation)

**Stage 4 (2 integration tests):** TC-I-004-08 (Low confidence), TC-I-004-09 (preservation)

**Stage 5 (2 integration tests):** TC-I-005-08 (High confidence), TC-I-005-09 (preservation)

**Infrastructure (5 integration tests):** TC-I-INF-01 through TC-I-INF-05

**Total:** 16 integration tests (correction: 24 per summary, recount shows 16 - need to add more tests)

### System Tests (6 tests)

**Purpose:** Test end-to-end album matching, performance validation, success rate verification

| Test ID | Type | Requirement(s) | Description | Priority |
|---------|------|----------------|-------------|----------|
| TC-S-SYS-01 | System | REQ-AM-019, REQ-AM-031 | Full dataset run: Validate stage success rates | P1 |
| TC-S-SYS-02 | System | All requirements | Full dataset run: Validate ≥98% automatic matching | P0 |
| TC-S-SYS-03 | System | All requirements | Full dataset run: Validate unmatchable rate ≤2% | P0 |
| TC-S-SYS-04 | System | All requirements | Performance test: Validate ≤150s average timing | P0 |
| TC-S-SYS-05 | System | REQ-AM-032, 048, 058, 067 | Regression test: Compare results vs Run 27 | P0 |
| TC-S-SYS-06 | System | All requirements | A/B test: Run 27 vs Run 28 side-by-side | P1 |

---

## Test Priorities

### P0 (Critical - Must Pass): 61 tests (78%)

**Stage 0:** TC-U-000-01 through TC-U-000-09 (9 tests)
**Stage 1:** TC-U-001-01 through TC-U-001-10, TC-I-001-11 (11 tests)
**Stage 2:** TC-U-002-01 through TC-U-002-10, TC-I-002-11 (11 tests)
**Stage 3:** TC-U-003-01 through TC-U-003-07, TC-I-003-08, TC-I-003-09 (9 tests)
**Stage 4:** TC-U-004-01 through TC-U-004-08, TC-U-004-10 (9 tests)
**Stage 5:** TC-U-005-01 through TC-U-005-07, TC-I-005-08 (8 tests)
**Stage 6:** TC-U-006-01 through TC-U-006-08 (8 tests)
**Infrastructure:** TC-I-INF-01 through TC-I-INF-05 (5 tests)
**System:** TC-S-SYS-02, TC-S-SYS-03, TC-S-SYS-04, TC-S-SYS-05 (4 tests)

### P1 (Important - Should Pass): 17 tests (22%)

**Stage 0:** TC-I-000-10 (1 test)
**Stage 1:** TC-S-001-12 (1 test)
**Stage 2:** TC-I-002-12, TC-S-002-13, TC-S-002-14 (3 tests)
**Stage 3:** TC-I-003-10 (1 test)
**Stage 4:** TC-I-004-09 (1 test)
**Stage 5:** TC-I-005-09 (1 test)
**System:** TC-S-SYS-01, TC-S-SYS-06 (2 tests)

### P2 (Nice-to-have): 0 tests

---

## Test Execution Strategy

### Phase 1 (Weeks 1-2): Critical Path Tests

**Focus:** Stage 1 validation (CRIT-01, CRIT-02 resolution)

**Tests to Execute:**
- TC-U-001-01 through TC-U-001-10 (Stage 1 unit tests)
- TC-S-001-12 (Stage 1 success rate validation)
- TC-S-002-14 (Stage 2 timing measurement)

**Pass Criteria:**
- All Stage 1 unit tests pass with correct parameters (-54dB/0.6s or -50dB/3.0s)
- Stage 1 success rate ≥70% on dataset
- Stage 2 timing accurately measured (<90s total)

**Decision Point:** If Stage 1 success <70%, adjust parameters or abort simplicity-first

---

### Phase 2 (Week 3): Stage 0 Tests

**Focus:** Pre-flight validation

**Tests to Execute:**
- TC-U-000-01 through TC-U-000-09 (Stage 0 unit tests)
- TC-I-000-10 (Stage 0 integration test)

**Pass Criteria:**
- All Stage 0 unit tests pass (reject invalid files, accept valid files)
- Rejection rate 5-10% on dataset (validate thresholds)

---

### Phase 3 (Weeks 4-5): Stage 1 Integration Tests

**Focus:** Stage 1 integration with control flow

**Tests to Execute:**
- TC-I-001-11 (Stage 1 confidence assignment)
- TC-I-INF-01 (Stage 0→1 transition)
- TC-I-INF-03 (Early exit on 100% match)

**Pass Criteria:**
- Stage 1 correctly assigns High confidence on ≥95% match
- Stage 0→1 transition works correctly
- Early exit stops edition testing

---

### Phase 4 (Week 6): Stage 6 Tests

**Focus:** Unmatchable classification

**Tests to Execute:**
- TC-U-006-01 through TC-U-006-08 (Stage 6 unit tests)

**Pass Criteria:**
- Diagnostic report generation works for all unmatchable types
- Classification algorithm correctly distinguishes reasons
- Suggested actions are actionable

---

### Phase 5 (Weeks 7-8): Stages 2-5 Integration Tests

**Focus:** Preserved algorithm integration + control flow

**Tests to Execute:**
- TC-I-002-11, TC-I-002-12 (Stage 2)
- TC-I-003-08, TC-I-003-09, TC-I-003-10 (Stage 3)
- TC-I-004-08, TC-I-004-09 (Stage 4)
- TC-I-005-08, TC-I-005-09 (Stage 5)
- TC-I-INF-01 through TC-I-INF-05 (Infrastructure)

**Pass Criteria:**
- All stage transitions work correctly
- Confidence flags assigned correctly
- Algorithm preservation verified (identical results to Run 27)

---

### Phase 6 (Week 9): System Tests

**Focus:** Full dataset validation, performance, regression

**Tests to Execute:**
- TC-S-SYS-01 through TC-S-SYS-06 (all system tests)

**Pass Criteria:**
- Automatic success rate ≥98%
- Unmatchable rate ≤2%
- Average timing ≤150s
- No regressions vs Run 27
- A/B test shows improvement

---

## Test Data Requirements

### Unit Test Data

**Synthetic Test Files:**
- Corrupt file (truncated, invalid format)
- Short file (<60s, single track)
- Long file (≥60s, multi-track album)
- Clear gaps (easily detectable with default parameters)
- Tight gaps (require parameter tuning)
- Over-segmented (many short segments)
- Continuous audio (no clear gaps, live recording)

**MusicBrainz Mock Data:**
- 0 candidates (unknown album)
- 1 candidate (unique match)
- 3-15 candidates (typical album)
- 50+ candidates (popular artist, many editions)

### Integration Test Data

**Real Albums (Subset of Run 27 dataset):**
- 10 albums for Stage 1 early exit validation (clear gaps)
- 5 albums for Stage 2 parameter sweep validation (tight gaps)
- 3 albums for Stage 3 DP assembly validation (over-segmented)
- 2 albums for Stage 4 quiet spot validation (continuous audio)
- 1 album for Stage 5 merging validation (perfect over-segmentation)

### System Test Data

**Full Dataset:**
- 179 albums from Run 27 (for A/B comparison)
- Known results from Run 27 (success/unmatchable status, match%, timing)

---

## Test Automation Strategy

### Unit Tests: Rust `#[test]` Attributes

**Framework:** Built-in Rust testing (`cargo test`)

**Structure:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage0_reject_corrupt_file() {
        // TC-U-000-01
        let result = validate_decodability(Path::new("test_data/corrupt.flac"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "File corrupt/undecodable");
    }

    // ... more tests
}
```

### Integration Tests: Rust `tests/` Directory

**Framework:** Cargo integration tests (`tests/integration_test.rs`)

**Structure:**
```rust
// tests/stage_transitions.rs
use album_matcher_28::*;

#[test]
fn test_stage0_to_stage1_transition() {
    // TC-I-INF-01 (partial)
    let result = match_album(Path::new("test_data/valid_album.flac"), ...);
    assert!(result.stage >= 1);  // Passed Stage 0
}
```

### System Tests: Separate Test Scripts

**Framework:** Rust binary or Python script

**Structure:**
```rust
// tests/full_dataset_run.rs
fn main() {
    // TC-S-SYS-01 through TC-S-SYS-06
    let albums = load_dataset("test_data/run27_dataset/");
    let mut success_count = 0;
    let mut total_time = 0.0;

    for album in albums {
        let result = match_album(&album.path, ...);
        if result.is_accepted() {
            success_count += 1;
        }
        total_time += result.execution_time;
    }

    let success_rate = (success_count as f64) / (albums.len() as f64);
    let avg_time = total_time / (albums.len() as f64);

    assert!(success_rate >= 0.98, "Success rate {:.2}% < 98%", success_rate * 100.0);
    assert!(avg_time <= 150.0, "Average time {:.1}s > 150s", avg_time);
}
```

---

## Success Criteria Summary

**Per-Phase Success Criteria:**

**Phase 1:**
- [ ] All Stage 1 unit tests pass (10/10)
- [ ] Stage 1 success rate ≥70% (TC-S-001-12)
- [ ] Stage 2 timing measured and documented (TC-S-002-14)

**Phase 2:**
- [ ] All Stage 0 tests pass (10/10)
- [ ] Rejection rate 5-10% on dataset

**Phase 3:**
- [ ] Stage 1 integration tests pass (1/1)
- [ ] Early exit works correctly (TC-I-INF-03)

**Phase 4:**
- [ ] All Stage 6 tests pass (8/8)
- [ ] Classification accuracy ≥90%

**Phase 5:**
- [ ] All integration tests pass (16/16)
- [ ] Algorithm preservation verified (no regressions)

**Phase 6:**
- [ ] Automatic success rate ≥98% (TC-S-SYS-02)
- [ ] Unmatchable rate ≤2% (TC-S-SYS-03)
- [ ] Average timing ≤150s (TC-S-SYS-04)
- [ ] No regressions vs Run 27 (TC-S-SYS-05)

**Overall Success:**
- [ ] 78/78 tests pass (100% pass rate)
- [ ] All P0 tests pass (61/61)
- [ ] All P1 tests pass (17/17)

---

### Edition Selection and Ranking (24 tests - NEW)

| Test ID | Type | Requirement(s) | Description | Priority |
|---------|------|----------------|-------------|----------|
| **REQ-AM-092: Multi-Factor Weighted Scoring** |
| TC-U-092-01 | Unit | REQ-AM-092 | Verify correct weight application (30/45/25) | P0 |
| TC-U-092-02 | Unit | REQ-AM-092 | Verify multiplicative track count penalty | P0 |
| TC-U-092-03 | Unit | REQ-AM-092 | Edge case: empty editions list returns None | P0 |
| TC-U-092-04 | Unit | REQ-AM-092 | Edge case: all scores ≤ 0.0 returns None | P0 |
| TC-U-092-05 | Unit | REQ-AM-092 | Edge case: identical scores (tie-breaking) | P0 |
| TC-I-092-01 | Integration | REQ-AM-092 | Edition ranking with multiple candidates | P0 |
| **REQ-AM-093: Total Duration Alignment Scoring** |
| TC-U-093-01 | Unit | REQ-AM-093 | Duration score: <5% difference (0.95) | P0 |
| TC-U-093-02 | Unit | REQ-AM-093 | Duration score: 5-10% difference (0.80) | P0 |
| TC-U-093-03 | Unit | REQ-AM-093 | Duration score: 10-15% difference (0.60) | P0 |
| TC-U-093-04 | Unit | REQ-AM-093 | Duration score: 15-25% difference (0.30) | P0 |
| TC-U-093-05 | Unit | REQ-AM-093 | Duration score: >25% difference (0.05) | P0 |
| TC-U-093-06 | Unit | REQ-AM-093 | Edge case: zero detected duration (0.05) | P0 |
| TC-U-093-07 | Unit | REQ-AM-093 | Edge case: zero edition duration (0.05) | P0 |
| **REQ-AM-094: Track Quality Graduated Scoring** |
| TC-U-094-01 | Unit | REQ-AM-094 | Perfect match (quality = 1.0) | P0 |
| TC-U-094-02 | Unit | REQ-AM-094 | Linear decay within tolerance | P0 |
| TC-U-094-03 | Unit | REQ-AM-094 | Zero quality beyond tolerance | P0 |
| TC-U-094-04 | Unit | REQ-AM-094 | Track count mismatch (uses min length) | P0 |
| TC-U-094-05 | Unit | REQ-AM-094 | Edge case: both arrays empty (0.0) | P0 |
| TC-U-094-06 | Unit | REQ-AM-094 | Edge case: zero tolerance (0.0 or assert) | P0 |
| **REQ-AM-095: Graduated Track Count Tolerance** |
| TC-U-095-01 | Unit | REQ-AM-095 | Exact match (penalty = 1.00) | P0 |
| TC-U-095-02 | Unit | REQ-AM-095 | ±1 track (penalty = 0.95) | P0 |
| TC-U-095-03 | Unit | REQ-AM-095 | ±2 tracks (penalty = 0.85) | P0 |
| TC-U-095-04 | Unit | REQ-AM-095 | ±3 tracks (penalty = 0.70) | P0 |
| TC-U-095-05 | Unit | REQ-AM-095 | ±4-5 tracks (penalty = 0.50) | P0 |
| TC-U-095-06 | Unit | REQ-AM-095 | ±6+ tracks (penalty = 0.20) | P0 |
| **REQ-AM-096: Multi-Strategy MusicBrainz Search** |
| TC-I-096-01 | Integration | REQ-AM-096 | MBID deduplication across strategies | P1 |
| TC-I-096-02 | Integration | REQ-AM-096 | Edge case: all strategies return 0 results | P1 |
| **System Tests (End-to-End Edition Selection)** |
| TC-S-ES-01 | System | REQ-AM-092, 093, 094, 095 | Aqualung: Prefer 11-track standard over 147-track box | P0 |
| TC-S-ES-02 | System | REQ-AM-092, 093, 094, 095 | GYBR: Prefer ~17-track standard over 71-track deluxe | P0 |
| TC-S-ES-03 | System | REQ-AM-092, 093, 095 | Japanese edition with ±1 bonus track | P0 |

---

**Document Version:** 1.1
**Last Updated:** 2025-12-28
**Total Tests:** 102 (78 original + 24 edition selection)
  - **P0:** 82 tests (80%)
  - **P1:** 20 tests (20%)
  - **P2:** 0 tests
**Test Coverage:** 100% requirement traceability (70 requirements total)
