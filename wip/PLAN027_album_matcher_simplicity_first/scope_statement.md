# Scope Statement - PLAN027

**Plan:** PLAN027 Album Matcher Simplicity-First Redesign
**Specification:** SPEC_optimal_album_matching_stages.md
**Date:** 2025-11-25

---

## In Scope

### New Functionality

**Stage 0: Pre-Flight Validation (NEW)**
- File decodability check (decode first 10 seconds, handle decode failures)
- File duration validation (reject files <60 seconds as singles/fragments)
- MusicBrainz candidate validation (reject if search returns 0 results)
- Edition runtime filter validation (reject if all editions outside ±25% duration)
- Single-track detection consolidation (existing logic from Run 27, reorganized):
  - Filename pattern matching (track numbers in filename)
  - Directory file counting (multiple files = likely single tracks)
  - ID3 tag analysis (track number/total fields)
  - Duration heuristics (short files <8 min, long albums >20 min)
  - Silence gap counting (few gaps = likely single track)
  - Scoring threshold (≥1.5 score → reject as single track)
- Explicit rejection reasons (error codes for each rejection type)
- UNMATCHABLE status emission with reason codes

**Stage 1: Default Parameter Silence Detection (NEW)**
- Single-pass silence detection with proven default parameters:
  - Threshold: -54dB (from DEFAULT_THRESHOLD_DB constant)
  - Min duration: 0.6s (from DEFAULT_MIN_DURATION_SECS constant)
- Test against top 3 editions only (NDR ranks 1-3, 92% coverage per Run 23)
- Duration comparison with ±1.5s tolerance (MATCH_TOLERANCE_SECS)
- Early exit on ≥95% match (High confidence)
- Rejection criteria:
  - Detected/expected ratio <50% or >150% → Stage 2 (wrong parameters)
  - Match <95% → Stage 2 (need parameter tuning)
- Target: 70-80% success rate (early exit, avoid 180-parameter sweep)

**Stage 6: Unmatchable Classification and Diagnostics (NEW)**
- Comprehensive diagnostic report generation:
  - File metadata (path, duration, artist, album, tags)
  - All stage execution results (attempted, succeeded, failed, why)
  - Best match achieved (edition, percentage, mean timing error, detected vs expected)
  - Stage-by-stage detailed results (parameters tested, candidates generated, assemblies tried)
- Unmatchable reason classification:
  - Corrupt file (decodable but produces nonsense audio, truncated)
  - Wrong MusicBrainz data (file metadata points to wrong album, MB data incorrect)
  - Non-standard format (medleys, DJ mixes, live continuous recordings)
  - Edge case failure (algorithm should have matched but didn't, potential bug)
- User action suggestions:
  - "Manually specify correct edition ID" (override algorithm)
  - "Report MusicBrainz data as incorrect" (flag for MB community review)
  - "Accept file as unmatchable" (skip, exclude from statistics)
  - "Report as potential bug" (if file appears deterministically matchable)
- Terminal UNMATCHABLE status with reason code

**Control Flow Restructure (MODIFIED)**
- Main match_album() function refactor for 0→6 stage progression
- Explicit stage transition logic with rejection criteria
- Confidence flag assignment based on stage and match quality:
  - High: ≥95% match in Stages 1-2 (clear algorithmic success)
  - Medium: 80-94% match in Stage 3 (DP assembly succeeded but not perfect)
  - Low: 80-94% match in Stages 4-5 (heuristic approaches, higher false positive risk)
  - Unmatchable: <80% in all stages (not automatically matchable)
- Fast-fail shortcuts to skip impossible stages:
  - Skip Stage 3 if Stage 2 produces no over-segmented candidates
  - Skip Stage 5 if not over-segmented OR match <95%
  - Skip to Stage 4 if detected/expected ratio is 0% or >200% in Stage 1
- Early exit on 100% match (stop edition testing, preserve Run 27 behavior)

### Preserved Functionality (UNCHANGED)

**Stage 2: Full 180-Parameter Adaptive Sweep (PRESERVED from Run 27)**
- WindowDbProfile pre-computation (single-pass dB profiling, ~500ms)
- All 180 parameter combinations:
  - STAGE2_THRESHOLD_VALUES: 9 thresholds (-42 to -66 dB in 3dB steps)
  - STAGE2_MIN_DURATION_VALUES: 20 durations (0.1 to 2.0s)
  - Total: 180 combinations (9 × 20)
- Cheap filtering via pre-computed profile (no re-scanning audio)
- Test against top 5 editions (expanded from Stage 1's top 3)
- Track over-segmented candidates for Stage 3 assembly
- Early exit on ≥95% match
- Algorithm logic UNCHANGED from Run 27 (lines ~2000-3200)

**Stage 3: Dynamic Programming Assembly (PRESERVED from Run 27)**
- DP algorithm: dp[i][j] = minimum error grouping i segments into j tracks
- Generate all valid assemblies by merging adjacent segments
- Test each assembly against expected track durations
- Accept with High confidence if match ≥95%
- Accept with Medium confidence if match 80-94%
- Transition to Stage 4 if match <80%
- Algorithm logic UNCHANGED from Run 27 (lines ~3200-3600)

**Stage 4: Edition-Guided Quiet Spot Detection (PRESERVED from Run 27)**
- RMS profile calculation across entire audio file
- Window size: QUIET_SPOT_WINDOW_SECS (0.5s)
- Step size: QUIET_SPOT_WINDOW_STEP_SECS (0.25s, 50% overlap)
- Dynamic search radius (15% of preceding track, min 2s, max 10s)
- Find quietest spot within search window for each expected boundary
- Score candidates: RMS in dB + distance penalty
- Apply STAGE4_PENALTY (25% de-rating) to match percentage
- Accept if adjusted match ≥80%
- Algorithm logic UNCHANGED from Run 27 (lines ~3600-3900)

**Stage 5: Adjacent Track Merging (PRESERVED from Run 27)**
- Input: Over-segmented result (detected > expected AND match ≥95%)
- Generate all merge combinations to reduce count to expected
- Test each merge combination
- Accept if match = 100%
- Skip if detected ≤ expected OR match <95%
- Algorithm logic UNCHANGED from Run 27 (lines ~3900-4100)

**MusicBrainz Integration (PRESERVED from Run 27)**
- Search API (release search by artist + album)
- Combination A filter (weighted<42% OR various/album<35%)
- NDR ranking (name distance ratio, artist + album similarity)
- Release details API (fetch track counts and durations)
- Runtime filter (±25% duration tolerance)
- API rate limiting (1550ms between requests, 2x safety margin)
- Retry logic with exponential backoff
- Cache infrastructure (PLAN026: search and release caching, 3 modes)

**Edition Scoring and Ranking (PRESERVED from Run 27)**
- CD bonus (-50s penalty)
- Official status bonus (-40s)
- US release bonus (-30s)
- Track count penalty (60s per track difference)
- Artist fallback algorithm (prevent wrong-artist matches)
- Time-fit ranking adjustments (prefer better duration matches)
- Quality-over-quantity ranking (prefer higher match% over closer track count per Run 27)

**Constants and Thresholds (PRESERVED from Run 27, except stage-specific)**
- All silence detection parameters (RMS windows, thresholds)
- All MusicBrainz parameters (rate limits, max releases, retry delays)
- All scoring parameters (bonuses, penalties, weights)
- All similarity parameters (Jaro-Winkler, Levenshtein thresholds)
- All control flow parameters (concurrency limits, delays, timeouts)

---

## Out of Scope

### NOT Changing

**Algorithm Modifications:**
- Stage 2 adaptive sweep algorithm (preserve exactly as Run 27)
- Stage 3 DP assembly algorithm (preserve exactly as Run 27)
- Stage 4 quiet spot detection algorithm (preserve exactly as Run 27)
- Stage 5 merging algorithm (preserve exactly as Run 27)
- WindowDbProfile computation (preserve structure and caching)
- Silence detection core logic (RMS calculation, dB conversion, gap extraction)

**MusicBrainz Integration:**
- Search query construction (artist + album format)
- Combination A filter logic or thresholds (keep 42% and 35%)
- NDR ranking formula (name distance ratio weights)
- Release details parsing (JSON structure, field extraction)
- Runtime filter thresholds (keep ±25%)
- API client implementation (HTTP requests, headers, user agent)
- Rate limiting strategy (keep 1550ms delay)
- Retry logic and backoff delays (keep [0, 5, 15, 45, 60] pattern)

**Cache Infrastructure:**
- Cache mode configuration (Disabled, ReadWrite, ReadOnly)
- Cache directory structure (searches/, releases/, metadata.json)
- Cache key hashing (SHA-256 first 16 hex chars)
- Cache file format (JSON with timestamp and query/mbid)
- Cache metadata tracking (version, counts, last updated)
- Cache statistics (hits, misses, hit rate)
- Cache failure handling (corrupted files, read errors, parse errors)

**Edition Scoring:**
- CD bonus value (-50s)
- Official status bonus value (-40s)
- US release bonus value (-30s)
- Track count penalty formula (60s per track difference)
- Artist fallback thresholds and delta logic
- Time-fit delta threshold (-0.5)

**Constants (Except Stage-Specific):**
- DEFAULT_THRESHOLD_DB and DEFAULT_MIN_DURATION_SECS (used by Stage 1 only)
- MATCH_TOLERANCE_SECS (±1.5s per track, used by all stages)
- MB_RATE_LIMIT_MS, MB_REQUEST_TIMEOUT_SECS, MB_MAX_RELEASES (MusicBrainz config)
- RUNTIME_FILTER_MIN_RATIO, RUNTIME_FILTER_MAX_RATIO (±25% filter)
- RMS window constants (RMS_WINDOW_SHORT_SECS, etc.)
- NAME_DISTANCE weights and thresholds
- ARTIST_MISMATCH_THRESHOLD, ALBUM_MISMATCH_THRESHOLD (false positive detection)

### NOT Adding

**New Algorithms:**
- Alternative silence detection methods (spectral analysis, machine learning, etc.)
- Alternative assembly strategies (greedy, simulated annealing, etc.)
- Alternative quiet spot detection (FFT-based, energy-based, etc.)
- New MusicBrainz search strategies (acoustic fingerprinting, advanced filters)

**New Infrastructure:**
- GUI or web interface changes
- Database schema changes
- New cache mechanisms (Redis, memcached, etc.)
- Distributed processing (multi-machine parallelism)
- Cloud API integrations (beyond MusicBrainz)

**User-Facing Features:**
- Manual editing UI for track boundaries
- Visualization of waveforms or silence gaps
- Interactive parameter tuning interface
- Batch processing dashboard
- Progress reporting beyond existing heartbeat

**Optimization Beyond Scope:**
- Audio decode optimization (Symphonia library changes)
- RMS calculation vectorization (SIMD optimizations)
- Parallel stage execution (current design is sequential)
- Incremental processing (re-use partial results across runs)

---

## Assumptions

### Empirical Assumptions (Require Validation)

**A1: Stage 1 Success Rate (CRITICAL ASSUMPTION)**
- **Assumption:** 70-80% of albums match with default parameters (-54dB, 0.6s)
- **Basis:** Specification analysis, commercial album characteristics (clear track gaps)
- **Validation:** Phase 1 empirical test on Run 27 dataset (179 albums)
- **Risk:** If <70%, performance gain is lower than expected
- **Mitigation:** Adjust parameters or add 2-3 default parameter sets
- **Fallback:** Stage 2 always available for comprehensive sweep

**A2: Default Parameters Are Optimal (CRITICAL ASSUMPTION)**
- **Assumption:** -54dB threshold and 0.6s min duration are proven optimal
- **Basis:** Specification states "proven optimal from Run 27 analysis"
- **Validation:** Verify DEFAULT_THRESHOLD_DB and DEFAULT_MIN_DURATION_SECS in Run 27 code
- **Risk:** Constants may be different values (-50dB, 3.0s in Run 27 lines 610-611)
- **Mitigation:** Phase 1 validation will test both sets of values
- **Fallback:** Use empirically validated values, not specification assumptions

**A3: 180-Parameter Sweep Cost (CRITICAL ASSUMPTION)**
- **Assumption:** ~60-90s total for WindowDbProfile + 180 combinations + testing
- **Basis:** Specification empirical data, Run 27 timing analysis
- **Validation:** Measure Stage 2 timing in isolation during Phase 1
- **Risk:** Cost may be higher (100-120s) or lower (40-60s), affecting speedup calculation
- **Mitigation:** Adjust performance targets based on actual measurements
- **Fallback:** Accept actual speedup if ≥1.33x (even if <1.74x target)

**A4: Top 3 Edition Coverage (HIGH CONFIDENCE)**
- **Assumption:** Top 3 editions by NDR capture 92% of wins
- **Basis:** Run 23 empirical data (album_matcher_25_designs.md, 179/195 wins in ranks 1-2)
- **Validation:** Already validated in Run 23, high confidence
- **Risk:** Dataset-specific, may vary for different album collections
- **Mitigation:** Stage 1 tests top 3, Stage 2 tests top 5 (expanded coverage)
- **Fallback:** If Stage 1 misses many wins, adjust to top 5 in Stage 1

### Algorithmic Assumptions (High Confidence)

**A5: Stages 2-5 Produce Identical Results (MUST BE TRUE)**
- **Assumption:** Preserving Stages 2-5 logic exactly produces same matches as Run 27
- **Basis:** No changes to algorithm code, only reordering and control flow
- **Validation:** Regression testing on Run 27 dataset (identical match results expected)
- **Risk:** Accidental logic changes during refactor (control flow bugs)
- **Mitigation:** Extensive unit and integration tests, side-by-side A/B comparison
- **Fallback:** Revert to Run 27 if any match regressions detected

**A6: Simplicity-First Doesn't Increase False Positives (HIGH CONFIDENCE)**
- **Assumption:** Stage 1 default parameters don't produce more false positives than Stage 2
- **Basis:** Default parameters are a single point from Stage 2's 180-point search space
- **Validation:** Track false positive rate in Phase 1 (compare Stage 1 vs Stage 2 results)
- **Risk:** Default parameters may be "lucky" for some albums, "unlucky" for others
- **Mitigation:** Artist verification and album verification (existing false positive detection)
- **Fallback:** Lower Stage 1 acceptance threshold from 95% to 98%

**A7: Single-Track Detection Consolidation Doesn't Break Logic (MEDIUM CONFIDENCE)**
- **Assumption:** Extracting single-track detection to Stage 0 preserves existing behavior
- **Basis:** Logic is moved, not changed; same inputs, same outputs
- **Validation:** Unit tests on single-track files (compare Stage 0 vs scattered Run 27 logic)
- **Risk:** Scattered logic may have subtle interactions not obvious from code review
- **Mitigation:** Test on diverse single-track files (various patterns, metadata, durations)
- **Fallback:** Revert to scattered logic if detection accuracy degrades

### Performance Assumptions (Medium Confidence)

**A8: Stage 1 Cost <10s (MEDIUM CONFIDENCE)**
- **Assumption:** Single-pass silence detection with default params takes <10 seconds
- **Basis:** Stage 2 tests 180 combinations in ~60-90s, so 1 combination ~0.3-0.5s
- **Validation:** Measure Stage 1 timing in Phase 1
- **Risk:** Audio decode dominates (30-60s), making parameter testing cost insignificant
- **Mitigation:** Profile Stage 1 execution, optimize decode if needed
- **Fallback:** Accept slower Stage 1 if overall average still improves

**A9: Complex Albums Don't Regress Significantly (MEDIUM CONFIDENCE)**
- **Assumption:** Albums requiring Stage 2+ don't take much longer (Stage 1 overhead + Stage 2)
- **Basis:** Fast-fail rejection criteria prevent wasted time in Stage 1
- **Validation:** Measure Stage 2+ albums in Phase 5 (compare to Run 27 timing)
- **Risk:** Stage 1 adds 5-10s overhead, Stage 2 albums take 205-210s vs 200s
- **Mitigation:** Optimize Stage 1 rejection logic, skip Stage 1 for extreme cases
- **Fallback:** Accept slight regression for 15-20% of albums if overall average improves

**A10: Weighted Average Performance Improvement (HIGH CONFIDENCE)**
- **Assumption:** 1.74x speedup calculation is correct (75% × 90s + 17.5% × 180s + 7.5% × 215s = 115s)
- **Basis:** Specification weighted average formula, assumes Stage 1 success = 75%
- **Validation:** Recalculate after Phase 1 validation with actual Stage 1 success rate
- **Risk:** Stage 1 success rate may be 60% or 85%, changing weighted average
- **Mitigation:** Update performance targets based on Phase 1 results
- **Fallback:** Accept actual speedup if ≥1.33x

### Architectural Assumptions (High Confidence)

**A11: Deterministically Matchable Target (HIGH CONFIDENCE)**
- **Assumption:** 98-100% of valid albums (clear tracks, correct MB data, standard format) are algorithmically matchable
- **Basis:** Run 27 achieves 93-99%, specification targets 98-100% with better stage coverage
- **Validation:** Measure success rate in Phase 5 on full dataset
- **Risk:** Remaining 1-7% may include albums that are deterministically matchable but algorithms miss
- **Mitigation:** Analyze Stage 6 unmatchable albums, identify algorithm gaps, add new stages if needed
- **Fallback:** Accept 95-98% if remaining unmatchable albums are truly problematic

**A12: Unmatchable Rate ≤2% (MEDIUM CONFIDENCE)**
- **Assumption:** Only 0-2% of albums are truly unmatchable (corrupt, wrong MB, non-standard)
- **Basis:** Run 27 shows ~1-5% unmatchable, specification expects ~0.5-1% corrupt, ~0.5-2% wrong MB, ~0.5-2% non-standard
- **Validation:** Classify Stage 6 albums by reason in Phase 5
- **Risk:** Dataset may have higher corrupt/wrong MB rate (3-5%)
- **Mitigation:** Document unmatchable reasons, improve MB data quality where possible
- **Fallback:** Accept 2-4% unmatchable if reasons are valid (not algorithm gaps)

---

## Constraints

### Technical Constraints (Non-Negotiable)

**C1: Algorithm Preservation (HARD CONSTRAINT)**
- **Constraint:** Stages 2-5 must remain byte-for-byte identical to Run 27 (except variable/function renaming)
- **Rationale:** Run 27 achieves 93-99% success through careful tuning; changes risk regressions
- **Enforcement:** Regression tests with Run 27 dataset, side-by-side result comparison
- **Impact:** Any algorithm modifications must be in new stages (0, 1, 6) or control flow, never in Stages 2-5
- **Violation:** If any Stage 2-5 logic changes, revert to Run 27 baseline

**C2: Performance Target (HARD CONSTRAINT)**
- **Constraint:** Average time per album ≤150s (≥1.33x speedup vs Run 27's 200s)
- **Rationale:** 26% code increase (+550 lines) must be justified by significant performance gain
- **Enforcement:** Full dataset timing in Phase 5, weighted average calculation
- **Impact:** If <1.33x speedup, either optimize Stage 1 or abandon simplicity-first approach
- **Violation:** Reject plan if speedup <1.33x (cost exceeds benefit)

**C3: Success Rate Target (HARD CONSTRAINT)**
- **Constraint:** Automatic success rate ≥98% (no regression from Run 27's 93-99%)
- **Rationale:** Simplicity-first must not sacrifice match quality for speed
- **Enforcement:** Full dataset matching in Phase 5, compare accepted vs unmatchable counts
- **Impact:** If <98%, analyze Stage 6 albums, add recovery stages if needed
- **Violation:** Reject plan if success rate <95% (significant regression)

**C4: Test Coverage (HARD CONSTRAINT)**
- **Constraint:** 100% requirement traceability (every requirement has ≥1 acceptance test)
- **Rationale:** CLAUDE.md mandates "/plan workflow MUST achieve 100% test coverage per traceability matrix"
- **Enforcement:** Traceability matrix verification, no untested requirements allowed
- **Impact:** Any untested requirement blocks implementation
- **Violation:** Plan incomplete until all requirements have acceptance tests

### Resource Constraints (Practical Limits)

**C5: Implementation Timeline (FLEXIBLE CONSTRAINT)**
- **Constraint:** 6-9 weeks total (6 phases)
- **Rationale:** Realistic estimate for 650 lines new code + refactoring + validation
- **Impact:** Timeline may extend to 10-12 weeks if critical issues arise
- **Flexibility:** Phase 1 results may accelerate or delay subsequent phases

**C6: Dataset Size (PRACTICAL CONSTRAINT)**
- **Constraint:** Validation on 179 albums (Run 27 dataset)
- **Rationale:** Existing dataset with known results, enables A/B comparison
- **Impact:** Larger datasets would increase validation confidence but require more time
- **Flexibility:** May add targeted test albums (single tracks, corrupt files, edge cases)

**C7: Code Complexity (FLEXIBLE CONSTRAINT)**
- **Constraint:** ~650 lines new code (+8.9% vs Run 27)
- **Rationale:** Specification estimates 150 (Stage 0) + 200 (Stage 1) + 200 (Stage 6) + 100 (control flow)
- **Impact:** Actual implementation may be 500-800 lines depending on refactor needs
- **Flexibility:** Accept up to +15% code increase if justified by functionality

### External Constraints (Environment Dependencies)

**C8: MusicBrainz API Availability (EXTERNAL CONSTRAINT)**
- **Constraint:** MusicBrainz API must be accessible and operational
- **Rationale:** All edition discovery depends on MB search and release APIs
- **Impact:** API outages block validation and testing
- **Mitigation:** Use CacheMode::ReadOnly for algorithm tuning (no live API calls)
- **Fallback:** Validation phases may be delayed if API unavailable

**C9: Rust Toolchain Stability (EXTERNAL CONSTRAINT)**
- **Constraint:** Rust stable channel, no breaking changes to dependencies
- **Rationale:** Symphonia, lofty, Chromaprint, rayon, tokio must remain compatible
- **Impact:** Dependency updates may introduce regressions or API changes
- **Mitigation:** Lock dependency versions during implementation
- **Fallback:** Pin to known-good dependency versions if updates cause issues

**C10: Audio File Availability (EXTERNAL CONSTRAINT)**
- **Constraint:** Test dataset (179 albums) must be available for validation
- **Rationale:** Cannot validate without actual audio files
- **Impact:** Missing files block dataset-wide validation
- **Mitigation:** Use cache for MusicBrainz API, but audio decoding requires files
- **Fallback:** Validate on available subset if some files missing

---

## Boundary Clarifications

### What Stage 0 Does vs. Doesn't Do

**Does (In Scope):**
- Reject files that are corrupt, undecodable, or produce nonsense audio
- Reject files <60 seconds (singles, fragments, test files)
- Reject files with no MusicBrainz matches (unknown albums, unindexed artists)
- Reject files with all editions outside ±25% runtime (wrong album, incorrect duration)
- Reject files detected as single tracks (via consolidated scoring algorithm)

**Doesn't (Out of Scope):**
- Attempt to repair corrupt files (beyond scope, user must fix)
- Search alternative MusicBrainz queries (single search attempt only)
- Adjust runtime filter thresholds dynamically (fixed ±25%)
- Attempt matching with partial album metadata (artist-only, album-only)

### What Stage 1 Does vs. Doesn't Do

**Does (In Scope):**
- Test single default parameter set (-54dB, 0.6s) on entire audio file
- Test against top 3 editions by NDR (ranks 1-3)
- Accept if match ≥95% (early exit, skip Stages 2-6)
- Reject if detected/expected ratio <50% or >150% (transition to Stage 2)
- Reject if match <95% (transition to Stage 2)

**Doesn't (Out of Scope):**
- Test multiple default parameter sets (single set only, not 3-5 sets)
- Test against all editions (top 3 only, not top 5 or all)
- Accept matches <95% (strict threshold, no "good enough" early exits)
- Attempt parameter tuning (that's Stage 2's job)
- Generate over-segmented candidates (Stage 2 only)

### What Stage 6 Does vs. Doesn't Do

**Does (In Scope):**
- Generate comprehensive diagnostic report (all stage results, best attempt, failure reasons)
- Classify unmatchable reason (corrupt, wrong MB, non-standard, edge case)
- Suggest user actions (override, report MB issue, accept unmatchable, report bug)
- Emit terminal UNMATCHABLE status

**Doesn't (Out of Scope):**
- Attempt additional matching strategies (all stages exhausted)
- Query alternative MusicBrainz data sources (single MB search only)
- Automatically repair or correct issues (user intervention required)
- Implement fallback matching (that's Stages 1-5's job)

---

## Success Criteria (Restated from Summary)

**Must Have (Go/No-Go):**
1. Automatic success rate ≥98%
2. Average time ≤150s per album (≥1.33x speedup)
3. Stage 1 success rate ≥70% (validates simplicity-first)
4. 100% test coverage per traceability matrix
5. All CRITICAL specification issues resolved

**Should Have (Quality Gates):**
1. Automatic success rate ≥99% (stretch goal)
2. Average time ≤120s per album (≥1.67x speedup)
3. Stage 1 success rate ≥80% (strong simplicity-first)
4. Unmatchable rate ≤1% (ideal)
5. All HIGH specification issues resolved

**Could Have (Aspirational):**
1. Average time ≤115s per album (1.74x speedup per spec)
2. False positive rate ≤1% (high confidence)
3. False negative rate = 0% (no missed deterministic matches)

---

**Document Version:** 1.0
**Last Updated:** 2025-11-25
