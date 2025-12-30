# Requirements Index - PLAN027

**Plan:** PLAN027 Album Matcher Simplicity-First Redesign
**Specification:** SPEC_optimal_album_matching_stages.md
**Total Requirements:** 65

---

## Requirements by Stage

### Stage 0: Pre-Flight Validation (8 requirements)

| Req ID | Priority | Description | Spec Ref |
|--------|----------|-------------|----------|
| REQ-AM-001 | P0 | Validate file is decodable (try decode first 10 seconds) | Lines 122-124 |
| REQ-AM-002 | P0 | Validate file duration > 60 seconds (reject singles/fragments) | Lines 124 |
| REQ-AM-003 | P0 | Validate MusicBrainz search returns > 0 candidates | Lines 125-126 |
| REQ-AM-004 | P0 | Validate at least one edition passes runtime filter (±25%) | Lines 126 |
| REQ-AM-005 | P0 | Single-track detection (score threshold ≥1.5 → reject) | Lines 127-128 |
| REQ-AM-006 | P0 | Transition to Stage 1 if all validations pass | Lines 136-137 |
| REQ-AM-007 | P1 | Log rejection reason for each failed validation | Lines 138 |
| REQ-AM-008 | P1 | Emit UNMATCHABLE status with reason code for rejections | Lines 138 |

### Stage 1: Default Parameter Silence Detection (9 requirements)

| Req ID | Priority | Description | Spec Ref |
|--------|----------|-------------|----------|
| REQ-AM-011 | P0 | Decode entire file to PCM | Lines 153 |
| REQ-AM-012 | P0 | Apply single silence detection pass with DEFAULT_THRESHOLD_DB (-54dB) | Lines 154-156 |
| REQ-AM-013 | P0 | Apply single silence detection pass with DEFAULT_MIN_DURATION_SECS (0.6s) | Lines 154-157 |
| REQ-AM-014 | P0 | Extract track durations from silence gaps | Lines 158 |
| REQ-AM-015 | P0 | Test against top 3 editions (NDR ranks 1-3) | Lines 159 |
| REQ-AM-016 | P0 | Compare detected vs expected durations with ±1.5s tolerance | Lines 160 |
| REQ-AM-017 | P0 | Accept and early exit if match ≥95% | Lines 161, 549-555 |
| REQ-AM-018 | P0 | Transition to Stage 2 if match <95% OR detected/expected ratio <50% or >150% | Lines 163-166, 556-559 |
| REQ-AM-019 | P1 | Expected success rate: 70-80% of albums | Lines 173 |

### Stage 2: Full Adaptive Parameter Sweep (12 requirements)

| Req ID | Priority | Description | Spec Ref |
|--------|----------|-------------|----------|
| REQ-AM-021 | P0 | Pre-compute WindowDbProfile (single-pass dB profiling) | Lines 184-187 |
| REQ-AM-022 | P0 | Test all 180 parameter combinations (9 thresholds × 20 min durations) | Lines 188-192 |
| REQ-AM-023 | P0 | Use STAGE2_THRESHOLD_VALUES array (9 values: -42 to -66 dB in 3dB steps) | Lines 189, 871-873 |
| REQ-AM-024 | P0 | Use STAGE2_MIN_DURATION_VALUES array (20 values: 0.1 to 2.0s) | Lines 189, 876-879 |
| REQ-AM-025 | P0 | Generate track durations for each parameter combination | Lines 193 |
| REQ-AM-026 | P0 | Test against top 5 editions (expanded from Stage 1) | Lines 194 |
| REQ-AM-027 | P0 | Track over-segmented candidates (detected > expected) for Stage 3 | Lines 195 |
| REQ-AM-028 | P0 | Accept and early exit if match ≥95% | Lines 196, 562-568 |
| REQ-AM-029 | P0 | Transition to Stage 3 if over-segmented candidates exist | Lines 209-211, 570-573 |
| REQ-AM-030 | P0 | Transition to Stage 4 if no over-segmented candidates AND match <80% | Lines 211, 571 |
| REQ-AM-031 | P1 | Expected success rate: 15-20% of remaining pool | Lines 215-216 |
| REQ-AM-032 | P1 | Preserve algorithm unchanged from Run 27 | Lines 214, 218 |

### Stage 3: Dynamic Programming Assembly (8 requirements)

| Req ID | Priority | Description | Spec Ref |
|--------|----------|-------------|----------|
| REQ-AM-041 | P0 | Input: Over-segmented candidates from Stage 2 (detected > expected) | Lines 226-227 |
| REQ-AM-042 | P0 | Run DP algorithm: dp[i][j] = minimum error grouping i segments into j tracks | Lines 228-229 |
| REQ-AM-043 | P0 | Generate all valid assemblies (merge adjacent segments) | Lines 230 |
| REQ-AM-044 | P0 | Test each assembly against expected track durations | Lines 231 |
| REQ-AM-045 | P0 | Accept and early exit if match ≥95% | Lines 233, 577-583 |
| REQ-AM-046 | P0 | Accept with MEDIUM confidence if match ≥80% | Lines 241-242, 584-588 |
| REQ-AM-047 | P0 | Transition to Stage 4 if match <80% | Lines 238-239, 242 |
| REQ-AM-048 | P1 | Preserve algorithm unchanged from Run 27 | Lines 244, 248 |

### Stage 4: Edition-Guided Quiet Spot Detection (8 requirements)

| Req ID | Priority | Description | Spec Ref |
|--------|----------|-------------|----------|
| REQ-AM-051 | P0 | Calculate RMS profile across entire audio file | Lines 257-258 |
| REQ-AM-052 | P0 | Use QUIET_SPOT_WINDOW_SECS window size | Lines 258, 886 |
| REQ-AM-053 | P0 | Use QUIET_SPOT_WINDOW_STEP_SECS step size | Lines 258, 887 |
| REQ-AM-054 | P0 | Calculate dynamic search radius (15% of preceding track duration, min 2s, max 10s) | Lines 260-261, 888-890 |
| REQ-AM-055 | P0 | Find quietest spot within search window for each expected boundary | Lines 262-263 |
| REQ-AM-056 | P0 | Apply STAGE4_PENALTY (25% de-rating) to match percentage | Lines 266-267, 788 |
| REQ-AM-057 | P0 | Accept if adjusted match ≥80% | Lines 267, 595-609 |
| REQ-AM-058 | P1 | Preserve algorithm unchanged from Run 27 | Lines 277, 281 |

### Stage 5: Adjacent Track Merging (7 requirements)

| Req ID | Priority | Description | Spec Ref |
|--------|----------|-------------|----------|
| REQ-AM-061 | P0 | Input: Over-segmented result (detected > expected) | Lines 289-290 |
| REQ-AM-062 | P0 | Precondition: Match ≥95% (near-perfect alignment despite extra tracks) | Lines 291 |
| REQ-AM-063 | P0 | Generate all merge combinations to reduce count to expected | Lines 292-293 |
| REQ-AM-064 | P0 | Accept if any merge produces match = 100% | Lines 294, 614-621 |
| REQ-AM-065 | P0 | Transition to Stage 6 if no merge = 100% | Lines 299, 302 |
| REQ-AM-066 | P1 | Skip if detected ≤ expected OR match <95% | Lines 296-298, 612-622 |
| REQ-AM-067 | P1 | Preserve algorithm unchanged from Run 27 | Lines 305, 309 |

### Stage 6: Unmatchable Classification (6 requirements)

| Req ID | Priority | Description | Spec Ref |
|--------|----------|-------------|----------|
| REQ-AM-071 | P0 | Generate comprehensive diagnostic report (file path, duration, metadata) | Lines 323-326 |
| REQ-AM-072 | P0 | Log all stage results (what was tried, why it failed) | Lines 327 |
| REQ-AM-073 | P0 | Log best match achieved (edition, percentage, mean error) | Lines 328 |
| REQ-AM-074 | P0 | Classify unmatchable reason (corrupt, wrong MB data, non-standard, edge case) | Lines 318-322 |
| REQ-AM-075 | P0 | Emit UNMATCHABLE status with reason code | Lines 330 |
| REQ-AM-076 | P1 | Suggest user actions (override edition, report MB issue, accept unmatchable, report bug) | Lines 329, 337-341 |

### Infrastructure and Control Flow (7 requirements)

| Req ID | Priority | Description | Spec Ref |
|--------|----------|-------------|----------|
| REQ-AM-081 | P0 | Main match_album() function executes stages 0→1→2→3→4→5→6 | Lines 537-637 |
| REQ-AM-082 | P0 | Assign confidence flags: High (≥95% Stages 1-2), Medium (80-94% Stage 3), Low (80-94% Stages 4-5) | Lines 399-413 |
| REQ-AM-083 | P0 | Early exit on 100% match (stop edition testing immediately) | Lines 549-555 |
| REQ-AM-084 | P0 | Skip Stage 3 if no over-segmented candidates from Stage 2 | Lines 575-577 |
| REQ-AM-085 | P0 | Skip Stage 5 if not over-segmented OR match <95% | Lines 611-622 |
| REQ-AM-086 | P1 | Track per-stage metrics (attempts, success, rejection, time, early exit rate) | Lines 643-655 |
| REQ-AM-087 | P2 | Generate performance dashboard with weighted average timing | Lines 657-668 |

### Edition Selection and Ranking (5 requirements - NEW in SPEC Part 2.2)

| Req ID | Priority | Description | Spec Ref |
|--------|----------|-------------|----------|
| REQ-AM-092 | P0 | Multi-factor weighted scoring: duration (30%), quality (45%), name (25%), track count penalty | Lines 395-406 |
| REQ-AM-093 | P0 | Total duration alignment with graduated penalties (<5%=0.95, >25%=0.05) | Lines 418-449 |
| REQ-AM-094 | P0 | Track quality graduated scoring: quality = 1.0 - (error / tolerance) | Lines 455-501 |
| REQ-AM-095 | P0 | Graduated track count tolerance (exact=1.00, ±1=0.95, ±2=0.85, ±6+=0.20) | Lines 507-550 |
| REQ-AM-096 | P1 | Multi-strategy MusicBrainz search (7 strategies, already implemented) | Lines 556-576 |

---

## Requirements by Priority

### P0 (Critical - Must Implement): 55 requirements (79%)

**Stage 0:** REQ-AM-001 through REQ-AM-006
**Stage 1:** REQ-AM-011 through REQ-AM-018
**Stage 2:** REQ-AM-021 through REQ-AM-030
**Stage 3:** REQ-AM-041 through REQ-AM-047
**Stage 4:** REQ-AM-051 through REQ-AM-057
**Stage 5:** REQ-AM-061 through REQ-AM-065
**Stage 6:** REQ-AM-071 through REQ-AM-075
**Infrastructure:** REQ-AM-081 through REQ-AM-085
**Edition Selection:** REQ-AM-092 through REQ-AM-095

### P1 (Important - Should Implement): 14 requirements (20%)

**Stage 0:** REQ-AM-007, REQ-AM-008
**Stage 1:** REQ-AM-019
**Stage 2:** REQ-AM-031, REQ-AM-032
**Stage 3:** REQ-AM-048
**Stage 4:** REQ-AM-058
**Stage 5:** REQ-AM-066, REQ-AM-067
**Stage 6:** REQ-AM-076
**Infrastructure:** REQ-AM-086
**Edition Selection:** REQ-AM-096

### P2 (Nice-to-have - Could Implement): 1 requirement (1%)

**Infrastructure:** REQ-AM-087 (Performance dashboard)

---

## Requirements by Functional Area

### Validation and Rejection (10 requirements)
REQ-AM-001 through REQ-AM-008 (Stage 0), REQ-AM-071, REQ-AM-074, REQ-AM-076 (Stage 6)

### Silence Detection (21 requirements)
REQ-AM-011 through REQ-AM-019 (Stage 1), REQ-AM-021 through REQ-AM-032 (Stage 2)

### Track Assembly and Merging (15 requirements)
REQ-AM-041 through REQ-AM-048 (Stage 3), REQ-AM-061 through REQ-AM-067 (Stage 5)

### Alternative Detection (8 requirements)
REQ-AM-051 through REQ-AM-058 (Stage 4)

### Edition Selection and Ranking (5 requirements - NEW)
REQ-AM-092 through REQ-AM-096 (Multi-factor scoring, duration alignment, track quality, track count tolerance, multi-strategy search)

### Control Flow and Infrastructure (11 requirements)
REQ-AM-072, REQ-AM-073, REQ-AM-075 (Stage 6 reporting), REQ-AM-081 through REQ-AM-087 (Infrastructure)

---

## Requirement Dependencies

**Critical Path (All albums):**
- REQ-AM-001 through REQ-AM-008 (Stage 0) → Required for all albums
- REQ-AM-011 through REQ-AM-018 (Stage 1) → Required for all valid albums
- REQ-AM-081 (Main control flow) → Required for execution

**Conditional (Based on Stage 1 Result):**
- REQ-AM-021 through REQ-AM-032 (Stage 2) → If Stage 1 fails (15-20% of albums)
- REQ-AM-041 through REQ-AM-048 (Stage 3) → If Stage 2 produces over-segmented (5-10%)
- REQ-AM-051 through REQ-AM-058 (Stage 4) → If Stages 1-3 fail (3-5%)
- REQ-AM-061 through REQ-AM-067 (Stage 5) → If over-segmented with ≥95% match (<1%)
- REQ-AM-071 through REQ-AM-076 (Stage 6) → If all stages fail (≤2%)

**Algorithm Preservation (Non-negotiable):**
- REQ-AM-032 (Stage 2 unchanged)
- REQ-AM-048 (Stage 3 unchanged)
- REQ-AM-058 (Stage 4 unchanged)
- REQ-AM-067 (Stage 5 unchanged)

---

## Requirement Validation Approach

### Unit Tests (48 tests)
Cover individual stage logic, rejection criteria, parameter handling

### Integration Tests (24 tests)
Cover stage transitions, control flow, confidence assignment

### System Tests (6 tests)
Cover end-to-end album matching, performance validation, success rate verification

**Traceability:** See [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md) for requirement ↔ test mapping.

---

## Notes

1. **Default parameters:** REQ-AM-012 and REQ-AM-013 reference DEFAULT_THRESHOLD_DB and DEFAULT_MIN_DURATION_SECS constants from album_matcher_27.rs (lines 610-611). Current values are -50.0dB and 3.0s, but specification states -54dB and 0.6s. **CRITICAL ISSUE CRIT-02** - verify which values are correct.

2. **Stage 2 parameter grid:** REQ-AM-023 and REQ-AM-024 reference STAGE2_THRESHOLD_VALUES and STAGE2_MIN_DURATION_VALUES arrays. Specification states 9 thresholds × 20 durations = 180 combinations. Run 27 has 12 thresholds × 15 durations = 180 combinations. **SPECIFICATION ISSUE HIGH-01** - arrays differ but total matches.

3. **Performance targets:** REQ-AM-019 (70-80% Stage 1 success) and REQ-AM-031 (15-20% Stage 2 success) are assumptions requiring empirical validation. **CRITICAL ISSUE CRIT-01** - Phase 1 validation required.

4. **Preservation requirements:** REQ-AM-032, REQ-AM-048, REQ-AM-058, REQ-AM-067 mandate that Stages 2-5 algorithms remain unchanged. This is a hard constraint to prevent algorithm regressions.

5. **Confidence flags:** REQ-AM-082 defines three confidence levels. High confidence triggers 100% early exit, preventing unnecessary edition testing. Medium/Low confidence still accepts matches but signals lower reliability.

---

## Success Metrics (Edition Selection)

**Primary Metric:** Passage-level MBID accuracy (ground truth validation)
- **Baseline:** 85.3% album-level success (145/170 albums, full library test)
- **Target:** ≥98% album-level success
- **Secondary:** ≥99.5% passage-level accuracy (all passages have correct MBIDs)

**NOT Success Metrics (Intermediate):**
- Number of editions found (more editions could degrade performance if wrong ones rank higher)
- Match percentage (high match % with WRONG edition = failure, all MBIDs wrong)

**Validation Approach:**
- Ground truth dataset with known-correct MBIDs
- 80% standard albums, 10% deluxe/special editions, 10% edge cases
- Monitor edition type distribution (verify standard preferred when available)

---

**Document Version:** 1.1
**Last Updated:** 2025-12-28
**Total Requirements:** 70 (55 P0, 14 P1, 1 P2)

**Changes from v1.0:**
- Added 5 edition selection requirements (REQ-AM-092 through REQ-AM-096)
- Added success metrics section with outcome-focused validation
- Updated priority distribution and total counts
