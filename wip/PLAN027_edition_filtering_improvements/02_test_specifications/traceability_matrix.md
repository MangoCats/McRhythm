# Traceability Matrix - Edition Filtering Improvements

**Plan:** PLAN027
**Date:** 2026-01-16
**Purpose:** Map requirements → tests → implementation for complete coverage verification

---

## Requirements Coverage Summary

| Requirement | Tests | Implementation File(s) | Status | Coverage |
|-------------|-------|------------------------|--------|----------|
| REQ-EF-010 | TC-U-010-01, TC-U-010-02, TC-U-010-03, TC-U-010-04, TC-I-010-01, TC-S-010-01 | wkmp-ai/src/matching/edition_filter.rs | Pending | Complete |
| REQ-EF-020 | TC-U-020-01, TC-U-020-02, TC-U-020-03, TC-U-020-04, TC-I-020-01, TC-S-020-01 | wkmp-ai/src/matching/edition_filter.rs | Pending | Complete |
| REQ-EF-030 | TC-U-030-01, TC-U-030-02, TC-U-030-03, TC-U-030-04, TC-U-030-05, TC-I-030-01, TC-I-030-02, TC-I-030-03, TC-S-030-01 | wkmp-ai/src/matching/edition_filter.rs, wkmp-ai/src/matching/stages/stage2_album_first.rs | Pending | Complete |
| REQ-EF-040 | TC-U-040-01, TC-U-040-02, TC-U-040-03, TC-U-040-04, TC-I-040-01, TC-I-040-02, TC-S-040-01 | wkmp-ai/src/matching/edition_filter.rs | Pending | Complete |
| REQ-EF-050 | TC-U-050-01, TC-U-050-02, TC-U-050-03, TC-I-050-01, TC-S-050-01 | wkmp-ai/src/matching/edition_filter.rs | Pending | Complete |
| REQ-EF-060 | TC-S-060-01, TC-S-060-02, TC-S-060-03 | All of the above | Pending | Complete |

**Total:** 6 requirements, 36 tests, 100% coverage

---

## Detailed Traceability

### REQ-EF-010: Edition Preference Scoring - Deluxe Penalty

**Requirement:** Penalize deluxe editions with 0.7× score multiplier

**Tests:**
- **TC-U-010-01:** Deluxe keyword detection (lowercase "deluxe")
- **TC-U-010-02:** Deluxe keyword detection (mixed case "Deluxe", "DELUXE")
- **TC-U-010-03:** Expanded keyword detection ("expanded", "Expanded")
- **TC-U-010-04:** Score multiplier calculation (verify score *= 0.7)
- **TC-I-010-01:** Deluxe penalty integration with matching pipeline
- **TC-S-010-01:** Chemical Brothers album (deluxe edition) system test

**Implementation:**
- File: `wkmp-ai/src/matching/edition_filter.rs`
- Function: `score_edition_preference(edition: &Edition, detected_track_count: usize) -> f64`
- Lines: ~20-40 (to be implemented)

**Verification:**
- Unit tests verify keyword matching and score calculation
- Integration test verifies scoring affects edition selection
- System test verifies problem album behavior

**Coverage:** ✅ Complete (normal operation, edge cases, integration, end-to-end)

---

### REQ-EF-020: Edition Preference Scoring - Compilation Penalty

**Requirement:** Penalize compilation editions with 0.6× score multiplier

**Tests:**
- **TC-U-020-01:** "Collection" keyword detection
- **TC-U-020-02:** "Anthology" keyword detection
- **TC-U-020-03:** "Best of" keyword detection
- **TC-U-020-04:** Score multiplier calculation (verify score *= 0.6)
- **TC-I-020-01:** Compilation penalty integration
- **TC-S-020-01:** James Gang album (compilation) system test

**Implementation:**
- File: `wkmp-ai/src/matching/edition_filter.rs`
- Function: `score_edition_preference(edition: &Edition, detected_track_count: usize) -> f64` (same function as REQ-EF-010)
- Lines: ~20-40 (to be implemented)

**Verification:**
- Unit tests verify keyword matching and score calculation
- Integration test verifies compilation rejection
- System test verifies problem album fixed

**Coverage:** ✅ Complete

---

### REQ-EF-030: Track Count Pre-Filtering

**Requirement:** Filter editions with |track_count - detected| > 3

**Tests:**
- **TC-U-030-01:** Exact match (11 detected, 11 in edition → pass)
- **TC-U-030-02:** Within ±3 (11 detected, 13 in edition → pass)
- **TC-U-030-03:** Outside ±3 (11 detected, 16 in edition → filtered)
- **TC-U-030-04:** Empty edition list (0 editions → return empty)
- **TC-U-030-05:** All editions filtered (fallback to unfiltered list)
- **TC-I-030-01:** Track count filter before Stage 2 (integration)
- **TC-I-030-02:** Track count filter with scoring (combined)
- **TC-I-030-03:** Fallback behavior (all filtered, matching proceeds)
- **TC-S-030-01:** Imagine Dragons album (11 tracks vs 16-track deluxe)

**Implementation:**
- File 1: `wkmp-ai/src/matching/edition_filter.rs`
  - Function: `filter_by_track_count(editions: &[Edition], detected: usize) -> Vec<Edition>`
  - Lines: ~50-80 (to be implemented)
- File 2: `wkmp-ai/src/matching/stages/stage2_album_first.rs`
  - Modification: Apply filter before matching
  - Lines: ~10-15 modified

**Verification:**
- Unit tests verify filtering logic and edge cases
- Integration tests verify filter applied correctly in pipeline
- System test verifies problem album fixed

**Coverage:** ✅ Complete (normal, edge cases, fallback, integration, end-to-end)

---

### REQ-EF-040: Artist Consistency Validation

**Requirement:** Reject editions with >3 unique artists (multi-artist compilations)

**Tests:**
- **TC-U-040-01:** Single artist (all tracks same artist → accept)
- **TC-U-040-02:** 2-3 unique artists (featured artists → accept)
- **TC-U-040-03:** 4+ unique artists (compilation → reject)
- **TC-U-040-04:** Fuzzy artist matching ("The Beatles" vs "Beatles")
- **TC-I-040-01:** Artist consistency in Stage 2 integration
- **TC-I-040-02:** Multi-artist rejection flow
- **TC-S-040-01:** James Gang album (multi-artist compilation rejected)

**Implementation:**
- File: `wkmp-ai/src/matching/edition_filter.rs`
- Function: `validate_artist_consistency(edition: &Edition, source_artist: &str) -> bool`
- Lines: ~90-120 (to be implemented)

**Verification:**
- Unit tests verify unique artist extraction and threshold
- Unit test verifies fuzzy matching algorithm
- Integration tests verify rejection flow
- System test verifies problem album fixed

**Coverage:** ✅ Complete

---

### REQ-EF-050: Remix Track Error Tolerance

**Requirement:** Accept up to 120s error for tracks with remix keywords

**Tests:**
- **TC-U-050-01:** Remix keyword detection ("remix", "extended", "mix)", "version")
- **TC-U-050-02:** Extended keyword detection
- **TC-U-050-03:** Error tolerance application (85s error → accepted)
- **TC-I-050-01:** Remix tolerance in validation stage
- **TC-S-050-01:** Chemical Brothers album (remix track with -83s error)

**Implementation:**
- File: `wkmp-ai/src/matching/edition_filter.rs`
- Function: `is_remix_track(title: &str) -> bool`
- Lines: ~130-150 (to be implemented)
- Integration: Validation logic in matching pipeline (location TBD)

**Verification:**
- Unit tests verify keyword detection
- Unit test verifies tolerance threshold (120s)
- Integration test verifies validation accepts remix errors
- System test verifies problem album passes

**Coverage:** ✅ Complete

---

### REQ-EF-060: Zero Regression Validation

**Requirement:** No regressions on currently-passing albums (≥186/200 baseline)

**Tests:**
- **TC-S-060-01:** Full regression suite (200 albums, compare to baseline)
- **TC-S-060-02:** Parameter tuning - penalty multipliers (find optimal values)
- **TC-S-060-03:** Parameter tuning - track count tolerance (find optimal value)

**Implementation:**
- All of the above (REQ-EF-010 through REQ-EF-050)
- Configuration constants for tunable parameters

**Verification:**
- System test runs full 200-album suite
- Compare results to baseline: passes (≥186), errors (no increase), MBIDs (≥90% stable)
- Parameter tuning tests iterate over values to find zero-regression configuration

**Coverage:** ✅ Complete (end-to-end validation)

---

## Forward Traceability (Requirement → Tests)

**Verification:** Every requirement has tests. ✅ Complete

| Requirement | Test Count | Test Types | Coverage |
|-------------|-----------|------------|----------|
| REQ-EF-010 | 6 | 4 unit, 1 integration, 1 system | Complete |
| REQ-EF-020 | 6 | 4 unit, 1 integration, 1 system | Complete |
| REQ-EF-030 | 9 | 5 unit, 3 integration, 1 system | Complete |
| REQ-EF-040 | 7 | 4 unit, 2 integration, 1 system | Complete |
| REQ-EF-050 | 5 | 3 unit, 1 integration, 1 system | Complete |
| REQ-EF-060 | 3 | 0 unit, 0 integration, 3 system | Complete |

---

## Backward Traceability (Tests → Requirements)

**Verification:** Every test traces to a requirement. ✅ Complete (no orphaned tests)

---

## Implementation Status Tracking

**How to Use This Matrix During Implementation:**

1. **Before implementing:** All requirements status = "Pending"
2. **During implementation:** Update "Implementation File(s)" column with actual file paths
3. **After implementing:** Update status to "In Progress" when code written
4. **After unit tests pass:** Update status to "Unit Tested"
5. **After integration tests pass:** Update status to "Integration Tested"
6. **After system tests pass:** Update status to "Verified"
7. **Final state:** All requirements status = "Verified", Coverage = "Complete"

**Current Status:** All requirements = "Pending" (planning phase)

---

## Test Execution Checklist

**Phase 1: Unit Tests**
- [ ] TC-U-010-01 through TC-U-010-04 (Deluxe penalty)
- [ ] TC-U-020-01 through TC-U-020-04 (Compilation penalty)
- [ ] TC-U-030-01 through TC-U-030-05 (Track count filter)
- [ ] TC-U-040-01 through TC-U-040-04 (Artist consistency)
- [ ] TC-U-050-01 through TC-U-050-03 (Remix tolerance)

**Phase 2: Integration Tests**
- [ ] TC-I-010-01 (Deluxe integration)
- [ ] TC-I-020-01 (Compilation integration)
- [ ] TC-I-030-01, TC-I-030-02, TC-I-030-03 (Track count integration)
- [ ] TC-I-040-01, TC-I-040-02 (Artist consistency integration)
- [ ] TC-I-050-01 (Remix tolerance integration)

**Phase 3: System Tests - Parameter Tuning**
- [ ] TC-S-060-02 (Tune penalty multipliers)
- [ ] TC-S-060-03 (Tune track count tolerance)

**Phase 4: System Tests - Problem Albums**
- [ ] TC-S-010-01 (Chemical Brothers)
- [ ] TC-S-020-01 (James Gang compilation)
- [ ] TC-S-030-01 (Imagine Dragons track count)
- [ ] TC-S-040-01 (James Gang multi-artist)
- [ ] TC-S-050-01 (Chemical Brothers remix)

**Phase 5: System Tests - Regression Validation**
- [ ] TC-S-060-01 (Full 200-album regression suite)

**Success Criteria:** All 36 checkboxes checked ✅

---

## Test Coverage Gaps Analysis

**Coverage Type:** Completeness

**Questions:**
- ✅ Does every requirement have unit tests? YES (all have 3-5 unit tests)
- ✅ Does every requirement have integration tests? YES (all have 1-3 integration tests)
- ✅ Does every requirement have system tests? YES (all have 1 system test)
- ✅ Are edge cases covered? YES (empty lists, NULL values, boundary conditions)
- ✅ Is regression testing covered? YES (REQ-EF-060 with 3 system tests)
- ✅ Is parameter tuning covered? YES (TC-S-060-02, TC-S-060-03)

**Gaps Identified:** NONE

**Conclusion:** 100% test coverage, no gaps

---

## Maintenance Notes

**When to Update This Matrix:**
- Add new requirement → Add new row, create new tests
- Add new test → Add to appropriate requirement row
- Implement code → Update "Implementation File(s)" column
- Pass tests → Update "Status" column
- Discover coverage gap → Add test, update matrix

**Version Control:**
This file should be committed with code changes to maintain traceability in version history.
