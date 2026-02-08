# Specification Issues - PLAN027

**Plan:** PLAN027 Album Matcher Simplicity-First Redesign
**Specification:** SPEC_optimal_album_matching_stages.md
**Phase:** 2 - Specification Completeness Verification
**Date:** 2025-11-25

---

## Executive Summary

**Total Issues Found:** 18 issues across 4 severity levels

| Severity | Count | Action Required |
|----------|-------|-----------------|
| CRITICAL | 3 | Must resolve before implementation |
| HIGH | 6 | Should resolve, may proceed with caution |
| MEDIUM | 7 | Document decisions, address during implementation |
| LOW | 2 | Minor clarifications, address as needed |

**Key Findings:**
- 3 CRITICAL issues block implementation (default parameters unclear, success rate assumptions unvalidated, timing breakdown conflicts)
- 6 HIGH issues create implementation ambiguity (parameter arrays differ, thresholds unclear, edge cases undefined)
- 9 MEDIUM/LOW issues are addressable during implementation (documentation gaps, optimization opportunities)

**Recommendation:** Resolve CRITICAL issues in Phase 1 validation before proceeding to full implementation. HIGH issues can be addressed iteratively during Phases 2-5.

---

## CRITICAL Issues (Must Resolve Before Implementation)

### CRIT-01: Stage 1 Success Rate Assumption Unvalidated

**Severity:** CRITICAL
**Requirement:** REQ-AM-019
**Spec Reference:** Lines 173, 455, 490-496

**Issue:**
Specification assumes Stage 1 (default parameters) will match 70-80% of albums, enabling simplicity-first optimization. This assumption is UNVALIDATED and critical to performance improvement claim (1.74x speedup).

**Evidence:**
- Line 173: "Expected success: 70-80% (albums with clear gaps + correct edition in top 3)"
- Line 455: "Stage 1 (70-80% of albums): Default parameters only"
- Line 512-519: Weighted average calculation assumes 75% Stage 1 success: `(75% × 90s) + (17.5% × 180s) + (7.5% × 215s) = 115s`

**Impact:**
- If actual success rate is 50-60%: Weighted average becomes ~140s (1.43x speedup, not 1.74x)
- If actual success rate is 85-90%: Weighted average becomes ~95s (2.1x speedup, better than expected)
- Performance target (≤150s) depends critically on this assumption

**Resolution Required:**
1. Phase 1 empirical validation on Run 27 dataset (179 albums)
2. Implement Stage 1 in isolation, log success rate
3. Measure actual success rate with default parameters (-54dB, 0.6s or -50dB, 3.0s)
4. Recalculate weighted average performance with actual success rate
5. Adjust implementation plan if success rate <70%:
   - Option A: Optimize Stage 1 parameters (test 2-3 default sets)
   - Option B: Adjust performance targets (accept 1.5x vs 1.74x)
   - Option C: Abandon simplicity-first if success rate <50%

**Blocking:** YES - Cannot proceed to Phase 3+ without validating this assumption

---

### CRIT-02: Default Parameters Unclear (Specification vs. Run 27 Conflict)

**Severity:** CRITICAL
**Requirement:** REQ-AM-012, REQ-AM-013
**Spec Reference:** Lines 154-157, 862-865

**Issue:**
Specification states Stage 1 uses DEFAULT_THRESHOLD_DB = -54dB and DEFAULT_MIN_DURATION_SECS = 0.6s (lines 154-157, 862-864). However, album_matcher_27.rs defines:
- `const DEFAULT_THRESHOLD_DB: f64 = -50.0;` (line 610)
- `const DEFAULT_MIN_DURATION_SECS: f64 = 3.0;` (line 611)

**Evidence:**
- **Specification (line 154-157):**
  ```
  Apply SINGLE silence detection pass with proven defaults:
    - Threshold: -54dB (from Run 27 constants: DEFAULT_THRESHOLD_DB)
    - Min duration: 0.6s (from Run 27 constants: DEFAULT_MIN_DURATION_SECS)
  ```
- **Specification (line 862-864):**
  ```rust
  const DEFAULT_THRESHOLD_DB: f64 = -54.0;  // Proven optimal from analysis
  const DEFAULT_MIN_DURATION_SECS: f64 = 0.6;  // Proven optimal
  ```
- **Run 27 (line 610-611):**
  ```rust
  const DEFAULT_THRESHOLD_DB: f64 = -50.0;
  const DEFAULT_MIN_DURATION_SECS: f64 = 3.0;
  ```

**Impact:**
- -54dB vs -50dB: 4dB difference, significantly more sensitive (detects quieter silences)
- 0.6s vs 3.0s: 5x difference, detects much shorter gaps
- Wrong parameters will produce incorrect Stage 1 results
- Stage 1 success rate (70-80% assumption) depends on correct parameters

**Resolution Required:**
1. Verify which parameters are actually "proven optimal":
   - Option A: Specification is correct (-54dB, 0.6s are optimal from earlier analysis)
   - Option B: Run 27 is correct (-50dB, 3.0s are currently used defaults)
   - Option C: Neither is optimal, need empirical validation
2. Phase 1 validation: Test BOTH parameter sets on dataset
   - Measure success rate with -54dB/0.6s
   - Measure success rate with -50dB/3.0s
   - Choose parameter set with higher success rate
3. Update specification or Run 27 constants to match validated values
4. Document rationale for chosen parameters

**Blocking:** YES - Cannot implement Stage 1 without knowing correct parameters

---

### CRIT-03: Stage 2 Timing Breakdown Conflicts with Total Cost

**Severity:** CRITICAL
**Requirement:** REQ-AM-031
**Spec Reference:** Lines 184-192, 469-487

**Issue:**
Specification states WindowDbProfile computation costs ~500ms (line 186), and testing 180 combinations costs ~5-6 seconds total (line 192). However, empirical data states 180-parameter sweep costs ~60-90 seconds total (line 73, 486). These numbers don't align:
- If WindowDbProfile = 0.5s, and 180 combos = 5-6s, total = 5.5-6.5s
- But empirical data says total = 60-90s (10-15x higher)

**Evidence:**
- **Lines 184-187:** "Pre-compute WindowDbProfile (single-pass dB profiling across entire file). Scan audio ONCE, store dB level per window. Cost: ~500ms, O(audio_length)"
- **Line 192:** "Cost per combo: ~10-50ms (filter pre-computed profile, not re-scan audio). Total cost: ~5-6 seconds for all 180 combinations"
- **Line 73:** "Empirical cost: ~60-90 seconds total for 180-parameter sweep (including WindowDbProfile computation, filtering, and DP assembly)"
- **Line 486:** "180-parameter sweep (WindowDbProfile + filtering): ~60-90s"

**Impact:**
- If actual Stage 2 cost is 60-90s (not 5-6s), performance improvement is MUCH larger:
  - Current system: 200s average, 60-90s of which is Stage 2
  - Simplicity-first: 75% of albums skip Stage 2 entirely
  - Savings: 75% × 75s = 56s average savings (massive)
- If actual Stage 2 cost is 5-6s (not 60-90s), performance improvement is MUCH smaller:
  - Savings: 75% × 5.5s = 4s average savings (negligible)
- Performance target (1.74x speedup) depends on correct Stage 2 cost

**Possible Explanations:**
1. **60-90s includes DP assembly (Stage 3):** Lines 73 and 486 say "including DP assembly", but Stage 3 is supposed to be separate (~20-40s per line 484). If DP is included in 60-90s, then Stage 2 alone = 20-50s.
2. **5-6s is theoretical, 60-90s is empirical:** Specification underestimated actual cost due to audio decode overhead, edition testing, or other factors.
3. **Different datasets:** Specification timing is from different albums (shorter files, fewer editions tested).

**Resolution Required:**
1. Phase 1 timing measurement: Isolate Stage 2 execution time
   - Measure WindowDbProfile computation alone
   - Measure 180-parameter filtering alone
   - Measure edition testing overhead
   - Separate Stage 3 DP assembly timing
2. Clarify whether "60-90s" includes:
   - Audio decode (30-60s typically)
   - Edition testing (10-30s typically)
   - DP assembly (20-40s typically, should be Stage 3)
3. Update specification with correct breakdown:
   - Audio decode: Xs
   - WindowDbProfile: Ys
   - 180-parameter filtering: Zs
   - Edition testing: Ws
   - Total Stage 2: X+Y+Z+W seconds
4. Recalculate weighted average performance with actual Stage 2 cost

**Blocking:** YES - Cannot validate performance claims without accurate timing data

---

## HIGH Issues (Should Resolve, May Proceed with Caution)

### HIGH-01: Stage 2 Parameter Grid Differs (9×20 vs 12×15)

**Severity:** HIGH
**Requirement:** REQ-AM-023, REQ-AM-024
**Spec Reference:** Lines 189, 871-879 vs Run 27 lines 792-799

**Issue:**
Specification states Stage 2 uses 9 thresholds × 20 min durations = 180 combinations (lines 189, 871-873, 876-879). Run 27 uses 12 thresholds × 15 min durations = 180 combinations (lines 792-799). Both produce 180 total, but the arrays differ.

**Evidence:**
- **Specification (line 871-873):**
  ```rust
  const STAGE2_THRESHOLD_VALUES: [f64; 9] = [
      -42.0, -45.0, -48.0, -51.0, -54.0, -57.0, -60.0, -63.0, -66.0
  ];
  ```
  (9 values, -42 to -66 dB in 3dB steps)

- **Specification (line 876-879):**
  ```rust
  const STAGE2_MIN_DURATION_VALUES: [f64; 20] = [
      0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
      1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0
  ];
  ```
  (20 values, 0.1 to 2.0s in 0.1s steps)

- **Run 27 (line 792-794):**
  ```rust
  const STAGE2_THRESHOLD_VALUES: [f64; 12] = [
      -42.0, -45.0, -48.0, -51.0, -54.0, -57.0, -60.0, -63.0, -66.0, -69.0, -72.0, -75.0,
  ];
  ```
  (12 values, -42 to -75 dB in 3dB steps)

- **Run 27 (line 798-800):**
  ```rust
  const STAGE2_MIN_DURATION_VALUES: [f64; 15] = [
      0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5,
  ];
  ```
  (15 values, 0.1 to 1.5s in 0.1s steps)

**Impact:**
- Different parameter coverage: Spec covers -42 to -66 dB and 0.1-2.0s, Run 27 covers -42 to -75 dB and 0.1-1.5s
- Run 27 tests lower thresholds (-69, -72, -75 dB = more sensitive)
- Spec tests longer min durations (1.6-2.0s = detects longer gaps)
- Both are valid, but results may differ for edge case albums

**Resolution Options:**
1. **Option A (Use Run 27 arrays):** Preserve Run 27 parameter grid unchanged (12×15)
   - Rationale: Run 27 is proven to work (93-99% success)
   - Risk: Specification analysis may have identified better parameter coverage
2. **Option B (Use Specification arrays):** Adopt specification parameter grid (9×20)
   - Rationale: Specification may have optimized based on Run 27 analysis
   - Risk: Untested, may reduce success rate if lower thresholds (-69 to -75 dB) are needed
3. **Option C (Hybrid):** Test both grids in Phase 1, choose best performer
   - Rationale: Empirical validation of parameter coverage
   - Risk: More validation time (test 180×2 = 360 combinations)

**Recommended Resolution:** Option A (use Run 27 arrays). Specification preserves all Run 27 algorithms unchanged (REQ-AM-032), so parameter grid should also be preserved. Document discrepancy, note that spec arrays are aspirational optimization for future work.

**Blocking:** NO - Can proceed with Run 27 arrays (Option A), defer spec arrays to future optimization

---

### HIGH-02: Stage 1 Top 3 Edition Assumption May Be Too Restrictive

**Severity:** HIGH
**Requirement:** REQ-AM-015
**Spec Reference:** Lines 159, 901-902

**Issue:**
Stage 1 tests only top 3 editions by NDR (lines 159, 901-902). Specification cites Run 23 data showing 92% of wins occur in ranks 1-2 (line 901), suggesting top 3 is sufficient. However, this assumes:
1. Run 23 data applies to all album datasets (may be dataset-specific)
2. Default parameters don't change edition ranking effectiveness
3. Top 3 by NDR is still optimal when testing single parameter set (not 180 sets)

**Evidence:**
- **Line 159:** "Test against top 3 editions (NDR ranks 1-3)"
- **Line 901:** "Edition ranks 1-2 win: 91.8% (179/195)" (from album_matcher_25_designs.md Run 23 data)
- **Line 902:** "Implication: Stage 1 testing top 3 editions captures ~92% of potential wins"

**Impact:**
- If top 3 is too restrictive (actual win rate 80-85%, not 92%), Stage 1 success rate drops
- Missing correct edition in top 3 forces Stage 2 escalation (performance regression)
- Stage 2 tests top 5 editions (line 194), suggesting top 3 may be insufficient

**Resolution Options:**
1. **Option A (Keep top 3):** Maintain top 3 for Stage 1 simplicity
   - Rationale: 92% coverage is good, Stage 2 tests top 5 as fallback
   - Risk: 8% of albums miss correct edition in Stage 1, force Stage 2
2. **Option B (Increase to top 5):** Match Stage 2's top 5 for consistency
   - Rationale: Better coverage (likely 95-97%), minimal complexity increase
   - Risk: Slightly slower Stage 1 (test 5 editions vs 3, ~66% more work)
3. **Option C (Validate in Phase 1):** Test both top 3 and top 5 on dataset
   - Rationale: Empirical validation of edition coverage
   - Risk: More validation work

**Recommended Resolution:** Option C (validate in Phase 1). If top 3 coverage is <90%, increase to top 5. Document trade-off between Stage 1 speed and edition coverage.

**Blocking:** NO - Can proceed with top 3 (Option A), validate in Phase 1, adjust if needed

---

### HIGH-03: Stage 4 Penalty Prevents 100% Early Exit

**Severity:** HIGH
**Requirement:** REQ-AM-056
**Spec Reference:** Lines 266-267, 788

**Issue:**
Stage 4 applies 25% penalty to match percentage (line 266, 788), meaning maximum adjusted match is 75% (not 100%). Specification states "Stage 4 can never trigger a 100% early exit due to this penalty" (line 788). However, early exit logic checks for ≥95% match (lines 595-609), which Stage 4 can never achieve (75% max).

**Evidence:**
- **Line 266-267:** "Apply STAGE4_PENALTY (25% de-rating) to match percentage"
- **Line 788:** "Stage 4 can never trigger a 100% early exit due to this penalty."
- **Line 595-609:** Pseudo-code shows Stage 4 early exit checks `if stage4.match_pct >= 95.0` and `if stage4.match_pct >= 80.0`

**Impact:**
- Stage 4 can NEVER achieve ≥95% (maximum is 75%, even if raw match is 100%)
- Early exit at ≥95% is impossible for Stage 4 (pseudo-code line 596-600 is unreachable)
- Only ≥80% early exit is possible for Stage 4 (pseudo-code line 603-609)

**Resolution Required:**
1. Clarify whether Stage 4 penalty should prevent ≥95% early exit:
   - **Option A (Specification is correct):** Stage 4 is intentionally less reliable, penalty prevents false positives, only Low confidence allowed
   - **Option B (Penalty is too harsh):** Reduce penalty to 10-15% to allow ≥95% for excellent Stage 4 results
2. Update pseudo-code to reflect actual Stage 4 behavior:
   - Remove unreachable ≥95% early exit (lines 596-600)
   - Document that Stage 4 can only achieve Low confidence (≥80%)
3. Verify this is intentional in Run 27 (check if Stage 4 ever produced High confidence)

**Recommended Resolution:** Option A (penalty is intentional). Stage 4 is heuristic (RMS profiling, not silence-based), so 25% penalty reflects lower reliability. Remove unreachable ≥95% early exit from pseudo-code. Document that Stage 4 only produces Low confidence.

**Blocking:** NO - Implementation is clear (apply 25% penalty), but documentation needs correction

---

### HIGH-04: Single-Track Detection Scoring Threshold Justification Missing

**Severity:** HIGH
**Requirement:** REQ-AM-005
**Spec Reference:** Lines 127-128, 831

**Issue:**
Stage 0 rejects files with single-track detection score ≥1.5 (lines 127-128, 831). However, specification provides no justification for 1.5 threshold. Why not 1.0 (one strong indicator)? Why not 2.0 (two strong indicators)?

**Evidence:**
- **Line 127-128:** "Single-track detection (score threshold ≥1.5 → likely single track)"
- **Line 831:** "const SINGLE_TRACK_SCORE_THRESHOLD: f64 = 1.5;  // Score >= 1.5 = likely single track"
- **Lines 834-843:** Scoring constants (0.3 to 1.0 per indicator)

**Impact:**
- Threshold too low (e.g., 1.0): False positives, reject valid multi-track albums
- Threshold too high (e.g., 2.5): False negatives, accept single tracks as albums
- No guidance on tuning threshold if Stage 0 rejection rate is too high/low

**Resolution Required:**
1. Provide rationale for 1.5 threshold:
   - Example: "1.5 = one strong indicator (1.0) + one medium indicator (0.5)"
   - Example: "1.5 empirically validated to minimize false positives/negatives"
2. Phase 2 validation: Test single-track detection on diverse files
   - Measure false positive rate (multi-track albums scored ≥1.5)
   - Measure false negative rate (single tracks scored <1.5)
   - Adjust threshold if error rates too high
3. Document threshold tuning guidance for future adjustments

**Recommended Resolution:** Phase 2 validation with diverse test files. Start with 1.5, measure error rates, adjust if needed. Document rationale in code comments.

**Blocking:** NO - Can proceed with 1.5, validate in Phase 2

---

### HIGH-05: Stage 6 Unmatchable Reason Classification Algorithm Unclear

**Severity:** HIGH
**Requirement:** REQ-AM-074
**Spec Reference:** Lines 318-322

**Issue:**
Stage 6 classifies unmatchable albums into 4 categories (corrupt, wrong MB data, non-standard, edge case) based on stage results (lines 318-322). However, classification algorithm is not specified. How does the system distinguish between "wrong MB data" and "non-standard format"? What thresholds determine "edge case failure"?

**Evidence:**
- **Line 318-322:**
  ```
  Classification:
  - Corrupt file: File decodes but produces nonsense audio (silence, noise, truncated)
  - Wrong MusicBrainz data: File metadata points to wrong album, or MB data is incorrect
  - Non-standard format: Medleys, DJ mixes, non-standard formats that defeat all algorithms
  - Edge case failure: Algorithm should have matched but didn't (potential bug)
  ```
- No algorithm provided for distinguishing these categories

**Impact:**
- Ambiguous classification: Albums may be misclassified, leading to incorrect user actions
- "Wrong MB data" vs "Non-standard" distinction unclear: Both may produce similar failure patterns (no matches across all stages)
- "Edge case failure" threshold undefined: What match% is "should have matched"? 70-79%? 60-79%?

**Resolution Required:**
1. Define classification algorithm with explicit criteria:
   - **Corrupt:** Stage 0 decodability check fails OR audio is all silence/noise
   - **Wrong MB data:** Best attempt has wrong artist (similarity <50%) OR best attempt has very different track count (|detected - expected| > 5)
   - **Non-standard:** All stages detect 0 or 1 tracks (continuous audio, no gaps)
   - **Edge case:** Best attempt match% is 70-79% (close but not accepted)
2. Add thresholds to constants:
   - `CLASSIFY_WRONG_MB_ARTIST_THRESHOLD: f64 = 0.5` (50% similarity)
   - `CLASSIFY_WRONG_MB_TRACK_COUNT_DELTA: usize = 5` (±5 tracks)
   - `CLASSIFY_EDGE_CASE_MIN_MATCH_PCT: f64 = 70.0` (70% = algorithm came close)
   - `CLASSIFY_EDGE_CASE_MAX_MATCH_PCT: f64 = 79.9` (79.9% = not quite accepted)
3. Document classification decision tree in Stage 6 implementation

**Recommended Resolution:** Define explicit classification algorithm with thresholds. Test on diverse unmatchable albums in Phase 4, refine criteria based on results.

**Blocking:** NO - Can define initial algorithm in Phase 4, iterate based on testing

---

### HIGH-06: Fast-Fail Shortcuts May Skip Viable Stages

**Severity:** HIGH
**Requirement:** REQ-AM-084, REQ-AM-085
**Spec Reference:** Lines 556-559, 571, 575-577, 611-622

**Issue:**
Control flow includes "fast-fail shortcuts" to skip stages deemed impossible (e.g., skip Stage 3 if no over-segmented candidates from Stage 2). However, specification doesn't fully define when to skip. Example: If Stage 1 produces detected/expected ratio >150% (over-segmentation), should Stage 2 be skipped entirely (go straight to Stage 3)?

**Evidence:**
- **Line 556-559:** "if !stage1.should_try_next_stage() { return try_stage_3_or_later(file, editions); }" - What is "try_stage_3_or_later"? Does it skip Stage 2?
- **Line 575-577:** "if !stage2.over_segmented_candidates.is_empty() { ... }" - Stage 3 only runs if over-segmented candidates exist
- **Line 611-622:** "if best_so_far.detected > best_so_far.expected && best_so_far.match_pct >= 95.0 { ... }" - Stage 5 only runs if over-segmented AND high match
- **Spec doesn't define:** When to skip Stage 2 (if Stage 1 already over-segmented), when to skip Stage 4 (if Stage 3 succeeded), etc.

**Impact:**
- Missing fast-fail logic: Performance regression (test stages unnecessarily)
- Over-aggressive fast-fail: Success rate regression (skip stages that might succeed)
- Example: Stage 1 produces 120% ratio (over-segmented, 18 detected, 15 expected, match 85%). Should Stage 2 run (try different parameters)? Or skip directly to Stage 3 (assemble existing segments)?

**Resolution Required:**
1. Define complete fast-fail decision tree:
   - Stage 1 ratio >150%: Skip Stage 2, go to Stage 3 (existing segments)
   - Stage 1 ratio <50%: Skip Stage 2, go to Stage 4 (silence detection completely failed)
   - Stage 2 no over-segmented AND <80% match: Skip Stage 3, go to Stage 4
   - Stage 3 succeeded ≥80%: Skip Stage 4 (no need for heuristic fallback)
   - Not over-segmented OR <95%: Skip Stage 5 (merging not applicable)
2. Add constants for fast-fail thresholds:
   - `FAST_FAIL_OVER_SEGMENTED_RATIO: f64 = 1.5` (150% = skip to Stage 3)
   - `FAST_FAIL_UNDER_SEGMENTED_RATIO: f64 = 0.5` (50% = skip to Stage 4)
3. Document fast-fail logic in control flow pseudo-code

**Recommended Resolution:** Define complete fast-fail decision tree in Phase 5 (control flow refactor). Test on dataset to validate skips don't regress success rate.

**Blocking:** NO - Can implement conservative fast-fail logic (skip only when obvious), refine in Phase 5

---

## MEDIUM Issues (Document Decisions, Address During Implementation)

### MED-01: Stage 1 Edition Testing Overhead Not Quantified

**Severity:** MEDIUM
**Requirement:** REQ-AM-015
**Spec Reference:** Line 159

**Issue:**
Stage 1 tests detected durations against top 3 editions (line 159). Testing involves comparing durations, calculating match%, which takes time. Specification doesn't quantify this overhead (compare to Stage 2 testing top 5 editions).

**Impact:**
- If edition testing is slow (~5-10s), Stage 1 total time increases
- If edition testing is fast (~1-2s), negligible impact
- Performance target depends on accurate Stage 1 timing

**Recommended Resolution:**
Phase 1 timing measurement. Isolate edition testing overhead, document in timing breakdown. If >5s, consider optimizing comparison algorithm.

**Blocking:** NO - Measure in Phase 1, optimize if needed

---

### MED-02: Confidence Flag Usage Not Fully Defined

**Severity:** MEDIUM
**Requirement:** REQ-AM-082
**Spec Reference:** Lines 399-413

**Issue:**
Confidence flags (High/Medium/Low) are assigned based on stage and match% (lines 399-413). However, specification doesn't define how confidence affects behavior AFTER acceptance. Does Low confidence trigger manual review? Does High confidence skip verification? Or are flags purely informational?

**Impact:**
- If flags affect behavior: Must define acceptance thresholds and actions per confidence level
- If flags are informational: Document in logs/statistics only, no behavioral impact

**Recommended Resolution:**
Clarify confidence flag usage. Recommended: Informational only (log confidence, track statistics, allow user filtering). No behavioral impact (all accepted matches are treated equally).

**Blocking:** NO - Can implement informational flags, defer behavioral usage to future work

---

### MED-03: Stage 3 DP Algorithm Complexity Not Specified

**Severity:** MEDIUM
**Requirement:** REQ-AM-042
**Spec Reference:** Lines 228-229, 244

**Issue:**
Stage 3 uses "DP algorithm: dp[i][j] = minimum error grouping i segments into j tracks" (line 228-229). Specification preserves Run 27 algorithm unchanged (line 244), but doesn't document algorithm complexity (time/space). For large segment counts (e.g., 50 detected, 15 expected), DP may be slow.

**Impact:**
- If DP is O(segments² × tracks), performance may degrade for highly over-segmented albums
- If DP is O(segments × tracks), performance is acceptable

**Recommended Resolution:**
Document DP algorithm complexity from Run 27 (review existing code). If complexity is high (>O(segments × tracks)), consider optimization in future work. For PLAN027, preserve unchanged per REQ-AM-048.

**Blocking:** NO - Preserve Run 27 algorithm unchanged, document complexity for future optimization

---

### MED-04: Stage 5 Merge Combination Count May Be Exponential

**Severity:** MEDIUM
**Requirement:** REQ-AM-063
**Spec Reference:** Line 292-293

**Issue:**
Stage 5 generates "all merge combinations to reduce count to expected" (line 292-293). For large over-segmentation (e.g., 20 detected, 12 expected, need 8 merges from 19 adjacencies), combination count is C(19, 8) = 75,582. Testing all combinations may be slow.

**Impact:**
- If combination count is exponential and unbounded, Stage 5 may be very slow
- If Run 27 already handles this (pruning, heuristics), no issue

**Recommended Resolution:**
Review Run 27 Stage 5 implementation. If combination generation is bounded (e.g., test only first 1000 combinations), document limit. If exponential, consider adding combination limit constant (e.g., MAX_MERGE_COMBINATIONS = 10000).

**Blocking:** NO - Preserve Run 27 behavior unchanged, document if slow

---

### MED-05: Diagnostic Report Size May Be Large

**Severity:** MEDIUM
**Requirement:** REQ-AM-071
**Spec Reference:** Lines 323-326

**Issue:**
Stage 6 generates "comprehensive diagnostic report" including all stage results, parameters tested, assemblies tried, etc. (lines 323-326). For complex albums (180 parameter combos, dozens of assemblies, hundreds of quiet spots), report may be 100KB+ of JSON or text.

**Impact:**
- Large reports may be unwieldy for user review
- Logging large reports may slow down processing
- Storage of diagnostics may consume significant disk space

**Recommended Resolution:**
Implement diagnostic report with size limits. Options:
1. Summary report (default): Top 10 parameter combos, top 5 assemblies, key statistics only
2. Detailed report (optional flag): Full data for debugging
Document report format and size expectations.

**Blocking:** NO - Implement summary report in Phase 4, add detailed report if needed

---

### MED-06: MusicBrainz API Rate Limit May Cause Delays

**Severity:** MEDIUM
**Requirement:** N/A (inherited from Run 27)
**Spec Reference:** Lines 617-620 (Run 27 constants)

**Issue:**
MusicBrainz API has rate limit (1 request per second). Run 27 uses MB_RATE_LIMIT_MS = 1550ms (1.55s, 2x safety margin). For albums with many editions (fetch 50 editions), this adds 50 × 1.55s = 77.5s delay. Specification doesn't address this bottleneck.

**Impact:**
- API rate limit dominates execution time for some albums (77s of 200s = 38%)
- Simplicity-first optimization (avoid Stage 2) doesn't address API delays
- Cache (PLAN026) mitigates this for repeated albums, but not first-time processing

**Recommended Resolution:**
Document that API rate limit is unavoidable bottleneck. For albums with many editions, execution time is dominated by API calls (not acoustic matching). Simplicity-first optimization still improves acoustic matching time (Stages 1-5), but total time includes API delays. Consider caching MusicBrainz data for future optimization.

**Blocking:** NO - Inherited constraint from Run 27, already mitigated by cache (PLAN026)

---

### MED-07: Unmatchable Rate Target (≤2%) May Be Optimistic

**Severity:** MEDIUM
**Requirement:** See scope_statement.md Assumption A12
**Spec Reference:** Lines 347-353, 461

**Issue:**
Specification targets unmatchable rate ≤2% (corrupt 0.5-1%, wrong MB 0.5-2%, non-standard 0.5-2%, line 347-353). Run 27 shows ~1-5% unmatchable (line 101). Target of ≤2% may be optimistic given dataset characteristics.

**Impact:**
- If actual unmatchable rate is 3-4%, success rate is 96-97% (not 98-100%)
- May fail success criteria (≥98% automatic matching)

**Recommended Resolution:**
Phase 5 validation: Measure unmatchable rate on full dataset. Classify unmatchable albums by reason. If >2%, analyze whether failures are due to:
1. Corrupt/wrong MB data (acceptable, not algorithm failure)
2. Algorithm gaps (need new stages or enhancements)
Document actual unmatchable rate, adjust success criteria if needed (accept 95-98% if valid reasons).

**Blocking:** NO - Validate in Phase 5, adjust expectations if needed

---

## LOW Issues (Minor Clarifications, Address as Needed)

### LOW-01: Stage Numbering Inconsistency (0-6 vs 1-6)

**Severity:** LOW
**Requirement:** N/A (documentation only)
**Spec Reference:** Throughout specification

**Issue:**
Specification uses "Stage 0" for pre-flight validation (lines 118-145), but also says "6 stages" (0-5) in some places and "7 stages" (0-6) in others. Numbering is inconsistent.

**Impact:**
- Minor documentation confusion
- No impact on implementation (stage numbers are clear in context)

**Recommended Resolution:**
Standardize on "7 stages (0-6)" throughout documentation. Update executive summary (line 13, says "5-stage pipeline" should be "6 acoustic stages + 1 validation stage").

**Blocking:** NO - Documentation cleanup only

---

### LOW-02: "via assembly" Log Message Not Defined

**Severity:** LOW
**Requirement:** N/A (logging only)
**Spec Reference:** Lines 51, 67, 904-905

**Issue:**
Specification references "via assembly" log messages from Run 27 (lines 51, 67, 904-905) to justify Stage 3 preservation. However, specification doesn't define logging format or requirements for Run 28. Should Run 28 produce identical log messages? Or is this analysis-only?

**Impact:**
- Minor: Logging format is implementation detail
- No impact on functionality

**Recommended Resolution:**
Preserve Run 27 logging format for consistency. Document key log messages for stage success (e.g., "via assembly", "via guided quiet spots", "via merging") to support future analysis.

**Blocking:** NO - Implementation detail, address during coding

---

## Summary of Resolution Actions

### Phase 1 (Weeks 1-2): Resolve CRITICAL Issues

**CRIT-01 (Stage 1 Success Rate):**
- [ ] Implement Stage 1 in isolation
- [ ] Run on 179-album dataset
- [ ] Measure actual success rate (target: 70-80%)
- [ ] Recalculate weighted average performance with actual rate
- [ ] Decision: Proceed, adjust, or abort simplicity-first

**CRIT-02 (Default Parameters):**
- [ ] Test -54dB/0.6s on dataset
- [ ] Test -50dB/3.0s on dataset
- [ ] Choose parameter set with higher success rate
- [ ] Document rationale
- [ ] Update specification or Run 27 constants

**CRIT-03 (Stage 2 Timing):**
- [ ] Isolate Stage 2 execution time (separate audio decode, WindowDbProfile, filtering, edition testing, DP assembly)
- [ ] Measure each component separately
- [ ] Document actual timing breakdown
- [ ] Recalculate weighted average performance with actual Stage 2 cost
- [ ] Update specification timing estimates

### Phase 2 (Week 3): Resolve HIGH Issues for Stage 0

**HIGH-04 (Single-Track Detection Threshold):**
- [ ] Test on diverse single-track and multi-track files
- [ ] Measure false positive/negative rates
- [ ] Adjust threshold if error rate >5%
- [ ] Document rationale

### Phase 3 (Weeks 4-5): Resolve HIGH Issues for Stage 1

**HIGH-01 (Stage 2 Parameter Grid):**
- [ ] Decision: Use Run 27 arrays (12×15)
- [ ] Document discrepancy with specification (9×20)
- [ ] Defer specification arrays to future optimization

**HIGH-02 (Stage 1 Top 3 Editions):**
- [ ] Measure edition coverage (top 3 vs top 5)
- [ ] If top 3 coverage <90%, increase to top 5
- [ ] Document trade-off

### Phase 4 (Week 6): Resolve HIGH Issues for Stage 6

**HIGH-05 (Unmatchable Reason Classification):**
- [ ] Define explicit classification algorithm with thresholds
- [ ] Test on diverse unmatchable albums
- [ ] Refine criteria based on results
- [ ] Document decision tree

### Phase 5 (Weeks 7-8): Resolve HIGH Issues for Control Flow

**HIGH-03 (Stage 4 Penalty):**
- [ ] Confirm 25% penalty is intentional
- [ ] Remove unreachable ≥95% early exit from pseudo-code
- [ ] Document Stage 4 only produces Low confidence

**HIGH-06 (Fast-Fail Shortcuts):**
- [ ] Define complete fast-fail decision tree
- [ ] Add thresholds for over/under-segmentation
- [ ] Test on dataset to validate no success rate regression
- [ ] Document fast-fail logic

### Phase 6 (Week 9): Validate MEDIUM Issues

**MED-01 through MED-07:** Measure, document, optimize if needed (non-blocking)

---

## Risk Assessment Per Issue

| Issue | Severity | Blocking | Resolution Phase | Risk Level |
|-------|----------|----------|------------------|------------|
| CRIT-01 | CRITICAL | YES | Phase 1 | HIGH |
| CRIT-02 | CRITICAL | YES | Phase 1 | HIGH |
| CRIT-03 | CRITICAL | YES | Phase 1 | HIGH |
| HIGH-01 | HIGH | NO | Phase 3 | MEDIUM |
| HIGH-02 | HIGH | NO | Phase 3 | MEDIUM |
| HIGH-03 | HIGH | NO | Phase 5 | LOW |
| HIGH-04 | HIGH | NO | Phase 2 | MEDIUM |
| HIGH-05 | HIGH | NO | Phase 4 | MEDIUM |
| HIGH-06 | HIGH | NO | Phase 5 | MEDIUM |
| MED-01 | MEDIUM | NO | Phase 1 | LOW |
| MED-02 | MEDIUM | NO | Phase 5 | LOW |
| MED-03 | MEDIUM | NO | Documentation | LOW |
| MED-04 | MEDIUM | NO | Documentation | LOW |
| MED-05 | MEDIUM | NO | Phase 4 | LOW |
| MED-06 | MEDIUM | NO | Documentation | LOW |
| MED-07 | MEDIUM | NO | Phase 5 | LOW |
| LOW-01 | LOW | NO | Documentation | NEGLIGIBLE |
| LOW-02 | LOW | NO | Implementation | NEGLIGIBLE |

**Overall Risk:** MEDIUM-HIGH (3 CRITICAL blockers in Phase 1, 6 HIGH issues addressable iteratively)

---

**Document Version:** 1.0
**Last Updated:** 2025-11-25
**Total Issues:** 18 (3 CRITICAL, 6 HIGH, 7 MEDIUM, 2 LOW)
