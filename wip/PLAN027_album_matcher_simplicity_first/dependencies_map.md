# Dependencies Map - PLAN027

**Plan:** PLAN027 Album Matcher Simplicity-First Redesign
**Specification:** SPEC_optimal_album_matching_stages.md
**Date:** 2025-11-25

---

## Overview

This document maps what code exists in album_matcher_27.rs (7324 lines) and what new code must be added for album_matcher_28.rs.

---

## Existing Code to Preserve (album_matcher_27.rs)

### Stage 2: Full 180-Parameter Adaptive Sweep

**Location:** Lines ~2000-3200 (estimated, ~1200 lines)

**Key Components:**
- `WindowDbProfile` struct and implementation
  - Pre-computed dB levels per analysis window
  - Single-pass audio scanning for dB profiling
  - Fast filtering via cached profile (no audio re-scanning)
- `compute_window_db_profile()` function
  - Decodes audio, calculates RMS per window
  - Converts RMS to dB, stores in profile
  - Handles variable window sizes (short/medium/standard)
- `test_parameter_combination()` function
  - Tests single threshold + min duration combination
  - Filters WindowDbProfile, extracts silence gaps
  - Returns detected track durations
- Parameter grid constants:
  - `STAGE2_THRESHOLD_VALUES: [f64; 12]` (12 thresholds in Run 27)
  - `STAGE2_MIN_DURATION_VALUES: [f64; 15]` (15 durations in Run 27)
  - Note: Specification states 9×20=180, Run 27 has 12×15=180 (arrays differ, total matches)
- Stage 2 main loop:
  - Iterate over all 180 combinations (nested loops: threshold × duration)
  - Test each combination against top 5 editions
  - Track over-segmented candidates (detected > expected)
  - Early exit on ≥95% match
  - Return best result + over-segmented candidates for Stage 3

**Preservation Strategy:** Copy entire Stage 2 implementation unchanged. Rename wrapper function from `stage2_adaptive_sweep()` (if named) to preserve in album_matcher_28.rs. No logic changes.

**Dependencies:**
- Symphonia audio decoding (decode entire file to PCM)
- RMS calculation utilities (existing in Run 27)
- dB conversion utilities (existing in Run 27)
- Duration comparison logic (±1.5s tolerance, MATCH_TOLERANCE_SECS)

---

### Stage 3: Dynamic Programming Assembly

**Location:** Lines ~3200-3600 (estimated, ~400 lines)

**Key Components:**
- `DPState` struct (or equivalent)
  - dp[i][j] = minimum error when grouping first i segments into j tracks
  - Backtracking to reconstruct optimal assembly
- `assemble_segments_dp()` function
  - Input: Over-segmented candidate (detected > expected)
  - DP algorithm: iterate over segments and target track count
  - Generate all valid assemblies by backtracking
  - Test each assembly against expected track durations
- Assembly testing loop:
  - Compare assembled track durations vs expected
  - Calculate match percentage
  - Track best assembly (highest match %)
- Early exit logic:
  - If match ≥95%: Return with High confidence
  - If match ≥80%: Return with Medium confidence
  - If match <80%: Return failure, proceed to Stage 4

**Preservation Strategy:** Copy entire Stage 3 implementation unchanged. Preserve DP algorithm logic, backtracking, assembly testing. No modifications.

**Dependencies:**
- Over-segmented candidates from Stage 2 (detected > expected)
- Duration comparison logic (same as all stages)
- Edition expected durations (from MusicBrainz release details)

---

### Stage 4: Edition-Guided Quiet Spot Detection

**Location:** Lines ~3600-3900 (estimated, ~300 lines)

**Key Components:**
- `RMSProfile` struct (or equivalent)
  - RMS levels per time window across entire audio file
  - Window size: QUIET_SPOT_WINDOW_SECS (0.5s)
  - Step size: QUIET_SPOT_WINDOW_STEP_SECS (0.25s, 50% overlap)
- `compute_rms_profile()` function
  - Decodes audio (if not already decoded)
  - Calculates RMS per window
  - Returns profile for quiet spot search
- `find_guided_quiet_spots()` function
  - Input: RMS profile + expected track boundaries (from edition)
  - For each expected boundary:
    - Calculate dynamic search radius (15% of preceding track, min 2s, max 10s)
    - Find quietest spot within search window
    - Score candidates: RMS in dB + distance penalty (QUIET_SPOT_PROXIMITY_PENALTY)
  - Convert detected boundaries to track durations
- Penalty application:
  - Apply STAGE4_PENALTY (25% de-rating) to match percentage
  - Prevents Stage 4 from ever achieving 100% (maximum 75%)
- Early exit logic:
  - If adjusted match ≥95%: Return with High confidence (rare due to penalty)
  - If adjusted match ≥80%: Return with Low confidence
  - If adjusted match <80%: Return failure, proceed to Stage 5

**Preservation Strategy:** Copy entire Stage 4 implementation unchanged. Preserve RMS profiling, guided search, penalty application. No modifications.

**Dependencies:**
- Symphonia audio decoding (decode entire file to PCM, if not cached)
- RMS calculation utilities
- Edition expected durations and track count (from MusicBrainz)
- Constants: QUIET_SPOT_* parameters (lines 644-649)

---

### Stage 5: Adjacent Track Merging

**Location:** Lines ~3900-4100 (estimated, ~200 lines)

**Key Components:**
- `merge_adjacent_tracks()` function
  - Input: Over-segmented result (detected > expected AND match ≥95%)
  - Precondition check: Is over-segmented? Is match ≥95%?
  - Generate all merge combinations (merge adjacent pairs to reduce count)
  - Test each merge combination
- Merge combination generation:
  - Combinatorial algorithm: choose (detected - expected) adjacent pairs to merge
  - Example: 14 detected, 12 expected → merge 2 pairs from 13 adjacencies
- Merge testing loop:
  - Apply merges to detected durations (sum adjacent durations)
  - Compare merged result vs expected
  - Calculate match percentage
- Early exit logic:
  - If any merge produces 100% match: Return with High confidence
  - Otherwise: Return failure, proceed to Stage 6

**Preservation Strategy:** Copy entire Stage 5 implementation unchanged. Preserve merge combination logic, testing, 100% threshold. No modifications.

**Dependencies:**
- Over-segmented result from prior stages (Stage 2, 3, or 4)
- Duration comparison logic
- Edition expected durations

---

### MusicBrainz API Client

**Location:** Lines ~380-600 (estimated, ~220 lines)

**Key Components:**
- `MBClient` struct (lines 378+)
  - HTTP client (reqwest)
  - Rate limiter (tokio sleep, MB_RATE_LIMIT_MS = 1550ms)
  - User agent string
  - Retry logic with exponential backoff (MB_RETRY_DELAYS_SECS)
- `search_releases()` async function
  - Query construction: `artist:{artist} AND release:{album}`
  - HTTP GET to MusicBrainz /ws/2/release endpoint
  - Parse JSON response → `MBSearchResponse`
  - Return list of candidate releases (up to MB_MAX_RELEASES = 150)
- `get_release_details()` async function
  - HTTP GET to /ws/2/release/{mbid} with inc=recordings
  - Parse JSON response → `MBReleaseDetails`
  - Extract track count and durations
- Rate limiting and retry:
  - Sleep MB_RATE_LIMIT_MS between requests
  - Retry on 503 (rate limit), 500 (server error), timeout
  - Exponential backoff: [0, 5, 15, 45, 60] seconds
- Cache integration:
  - Load from cache before API call
  - Store to cache after successful API call
  - Handle cache modes (Disabled, ReadWrite, ReadOnly)

**Preservation Strategy:** Copy entire MBClient implementation unchanged. No API changes, no rate limit changes, no retry logic changes.

**Dependencies:**
- reqwest (HTTP client)
- tokio (async runtime)
- serde_json (JSON parsing)
- Cache infrastructure (PLAN026: CacheConfig, CacheStats, load/store functions)

---

### Edition Discovery Pipeline

**Location:** Lines ~4500-5500 (estimated, ~1000 lines)

**Key Components:**
- `discover_editions()` function
  - Step 1: MusicBrainz search (artist + album query)
  - Step 2: Combination A filter (weighted<42% OR various/album<35%)
    - Constants: MIN_COMBINED_RATIO (0.42), MIN_VARIOUS_ALBUM_RATIO (0.35)
    - Weighted similarity: ARTIST_WEIGHT (0.4) × artist_sim + ALBUM_WEIGHT (0.6) × album_sim
  - Step 3: NDR ranking (name distance ratio, sort by similarity score)
    - Formula: sqrt(artist_weight² × artist_sim² + album_weight² × album_sim²)
    - Constants: NAME_DISTANCE_ARTIST_WEIGHT (1.0), NAME_DISTANCE_ALBUM_WEIGHT (1.414)
  - Step 4: Fetch release details via API (top 50 by NDR)
    - Constant: MAX_NAME_DISTANCE_RANK (50)
  - Step 5: Runtime filter (±25% duration match)
    - Constants: RUNTIME_FILTER_MIN_RATIO (0.75), RUNTIME_FILTER_MAX_RATIO (1.25)
  - Result: 3-15 editions to test acoustically
- `score_edition()` function
  - CD bonus: -50s penalty (SCORE_CD_BONUS)
  - Official status bonus: -40s (SCORE_OFFICIAL_BONUS)
  - US release bonus: -30s (SCORE_US_BONUS)
  - Track count penalty: +60s per track difference (SCORE_TRACK_COUNT_PENALTY)
  - Return: Total penalty score (lower = better)
- `rank_editions()` function
  - Sort by total penalty score (ascending)
  - Apply artist fallback (if top result is wrong artist, promote correct artist)
  - Apply time-fit adjustments (prefer better duration matches)
  - Apply quality-over-quantity (Run 27 enhancement):
    - If match quality differs >20%, prefer higher match% (MATCH_QUALITY_THRESHOLD_PCT)
    - Track count penalty: +4% per track difference (TRACK_COUNT_PENALTY_PER_TRACK)
  - Return: Ranked list of editions (best first)

**Preservation Strategy:** Copy entire edition discovery pipeline unchanged. No filter changes, no ranking changes, no scoring changes.

**Dependencies:**
- MBClient (search and details APIs)
- String similarity functions (Jaro-Winkler, Levenshtein)
- File metadata extraction (lofty: artist, album, duration)

---

### Constants and Configuration

**Location:** Lines 610-843 (233 lines)

**Key Constants to Preserve:**
- Silence detection: RMS_WINDOW_* values, SILENCE_DB_FLOOR, DB_MULTIPLIER
- MusicBrainz: MB_RATE_LIMIT_MS, MB_REQUEST_TIMEOUT_SECS, MB_MAX_RELEASES, MB_RETRY_DELAYS_SECS
- Edition scoring: SCORE_CD_BONUS, SCORE_OFFICIAL_BONUS, SCORE_US_BONUS, SCORE_TRACK_COUNT_PENALTY
- Runtime filter: RUNTIME_FILTER_MIN_RATIO, RUNTIME_FILTER_MAX_RATIO
- Quiet spot detection: QUIET_SPOT_* constants (window, step, search radius, penalty)
- Stage 2 parameter grids: STAGE2_THRESHOLD_VALUES, STAGE2_MIN_DURATION_VALUES
- Name distance: NAME_DISTANCE_* weights and thresholds
- Combination A filter: MIN_COMBINED_RATIO, MIN_VARIOUS_ALBUM_RATIO, ARTIST_WEIGHT, ALBUM_WEIGHT
- Artist fallback: ARTIST_FALLBACK_* thresholds and ratios
- Single-track detection: SINGLE_TRACK_* patterns, thresholds, and scores
- Control flow: MAX_CONCURRENT_ALBUMS, EARLY_EXIT_GRACE_PERIOD_SECS, EDITION_FEED_DELAY_SECS, HEARTBEAT_INTERVAL_SECS
- Stage penalties: STAGE4_PENALTY_PERCENT (25%)

**Constants to Use in New Stages:**
- Stage 1: DEFAULT_THRESHOLD_DB, DEFAULT_MIN_DURATION_SECS (lines 610-611)
  - **NOTE:** Run 27 has -50.0dB and 3.0s, spec states -54dB and 0.6s
  - **CRITICAL ISSUE CRIT-02:** Verify which values are correct
- Match tolerance: MATCH_TOLERANCE_SECS (±1.5s per track, line 614)
- Edition ranking: MAX_NAME_DISTANCE_RANK (50, line 680)
- Quality ranking: MATCH_QUALITY_THRESHOLD_PCT (20%, line 753), TRACK_COUNT_PENALTY_PER_TRACK (4%, line 758)

**Preservation Strategy:** Copy all constants unchanged. Add new constants for Stage 0, Stage 1, Stage 6 thresholds as needed.

---

### Cache Infrastructure (PLAN026)

**Location:** Lines ~53-329 (estimated, ~276 lines)

**Key Components:**
- `CacheMode` enum: Disabled, ReadWrite, ReadOnly (lines 57-64)
- `CacheConfig` struct: mode, cache_dir (lines 67-71)
- `CacheStats` struct: hit/miss tracking (lines 101-146)
- `hash_query()` function: SHA-256 first 16 hex chars (lines 152-158)
- `load_cached_search()` function: Load search response from cache (lines 163-183)
- `store_search_cache()` function: Store search response to cache (lines 187-220)
- `load_cached_release()` function: Load release details from cache (lines 221-244)
- `store_release_cache()` function: Store release details to cache (lines 245-277)
- `update_metadata()` function: Update cache metadata.json (lines 278-328)
- `print_cache_statistics()` function: Log cache hit/miss rates (lines 329-376)

**Preservation Strategy:** Copy entire cache infrastructure unchanged. No structural changes, no key hashing changes, no metadata format changes.

**Dependencies:**
- sha2 (SHA-256 hashing)
- serde_json (JSON serialization/deserialization)
- std::fs (file I/O)
- Atomic counters (AtomicU64 for thread-safe statistics)

---

### Utility Functions

**Location:** Scattered throughout Run 27

**Key Utilities:**
- RMS calculation: `calculate_rms()` (convert PCM samples to RMS value)
- dB conversion: `rms_to_db()` (convert RMS to decibels)
- Silence gap extraction: `extract_silence_gaps()` (convert dB profile to gap list)
- Duration comparison: `compare_durations()` (calculate match percentage with tolerance)
- String similarity: `calculate_jaro_winkler()`, `calculate_levenshtein()` (via strsim crate)
- Metadata extraction: `extract_file_metadata()` (via lofty crate)
- Single-track detection: `detect_single_track()` (scoring algorithm, scattered in Run 27)

**Preservation Strategy:** Copy all utility functions unchanged. May reorganize into helper modules for clarity, but no logic changes.

---

## New Code to Add (album_matcher_28.rs)

### Stage 0: Pre-Flight Validation (~150 lines)

**Location:** Beginning of match pipeline (before edition discovery)

**Components to Implement:**

**1. File Decodability Check (~30 lines)**
```rust
fn validate_decodability(file: &Path) -> Result<(), String> {
    // Try to decode first 10 seconds
    // Use Symphonia probe, format reader, decoder
    // If decode fails or produces nonsense: Return Err("File corrupt/undecodable")
    // If decode succeeds: Return Ok(())
}
```

**2. Duration Validation (~20 lines)**
```rust
fn validate_duration(file: &Path) -> Result<f64, String> {
    // Extract file duration (from metadata or decode)
    // If duration < 60s: Return Err("File too short (single track/fragment)")
    // Else: Return Ok(duration)
    // Constant: MIN_ALBUM_DURATION_SECS = 60.0
}
```

**3. MusicBrainz Candidate Validation (~20 lines)**
```rust
fn validate_musicbrainz_candidates(search_results: &[MBSearchResult]) -> Result<(), String> {
    // If search_results.is_empty(): Return Err("No MusicBrainz matches found")
    // Else: Return Ok(())
}
```

**4. Edition Runtime Filter Validation (~30 lines)**
```rust
fn validate_edition_runtime(file_duration: f64, editions: &[Edition]) -> Result<(), String> {
    // For each edition:
    //   Check if edition.duration in [file_duration × 0.75, file_duration × 1.25]
    //   If any pass: Return Ok(())
    // If all fail: Return Err("All editions outside ±25% runtime")
    // Constants: RUNTIME_FILTER_MIN_RATIO (0.75), RUNTIME_FILTER_MAX_RATIO (1.25)
}
```

**5. Single-Track Detection Consolidation (~50 lines)**
```rust
fn detect_single_track_consolidated(file: &Path, metadata: &FileMetadata) -> Result<bool, String> {
    // Consolidate scattered single-track detection logic from Run 27
    // Scoring algorithm:
    //   score = 0.0
    //   If filename matches SINGLE_TRACK_FILENAME_PATTERN: score += 1.0
    //   If directory has >= 4 audio files: score += 0.8
    //   If ID3 track total > 1: score += 1.0
    //   If duration < 8 min: score += 0.7
    //   If silence gaps < 3: score += 1.0
    //   etc. (see lines 815-843 for score constants)
    // If score >= SINGLE_TRACK_SCORE_THRESHOLD (1.5): Return Ok(true)
    // Else: Return Ok(false)
}
```

**6. Stage 0 Main Function (~30 lines)**
```rust
fn stage0_preflight_validation(file: &Path, metadata: &FileMetadata, editions: &[Edition]) -> Result<(), UnmatchableReason> {
    // Run all validation checks
    // If any fail: Return Err(UnmatchableReason::Corrupt/NoMBMatches/RuntimeMismatch/SingleTrack)
    // If all pass: Return Ok(())
}
```

**New Constants:**
- `MIN_ALBUM_DURATION_SECS: f64 = 60.0` (minimum album duration)

**New Types:**
```rust
enum UnmatchableReason {
    Corrupt(String),         // File corrupt/undecodable
    TooShort(f64),           // Duration < 60s
    NoMBMatches,             // MusicBrainz search returned 0 results
    RuntimeMismatch,         // All editions outside ±25%
    SingleTrack(f64),        // Detected as single track (score)
}
```

**Dependencies:**
- Existing Symphonia decode infrastructure (reuse from Stage 2)
- Existing single-track detection logic (consolidate from Run 27)
- Existing MusicBrainz search (already implemented)
- Existing runtime filter logic (already implemented)

---

### Stage 1: Default Parameter Silence Detection (~200 lines)

**Location:** First acoustic matching stage (before Stage 2)

**Components to Implement:**

**1. Single-Pass Silence Detection (~80 lines)**
```rust
fn stage1_default_silence_detection(file: &Path, editions: &[Edition; 3]) -> Stage1Result {
    // Decode entire file to PCM (reuse Stage 2 decode logic)
    // Apply single silence detection pass:
    //   threshold = DEFAULT_THRESHOLD_DB (verify: -54dB or -50dB?)
    //   min_duration = DEFAULT_MIN_DURATION_SECS (verify: 0.6s or 3.0s?)
    // Extract silence gaps → track durations
    // Test against top 3 editions (NDR ranks 1-3)
    // Return Stage1Result { best_match, detected_count, expected_count, match_pct, ... }
}
```

**2. Stage 1 Result Struct (~30 lines)**
```rust
struct Stage1Result {
    best_edition: Option<Edition>,
    match_pct: f64,
    detected_count: usize,
    expected_count: usize,
    detected_durations: Vec<f64>,
    mean_error: f64,
    should_try_stage2: bool,  // Rejection criteria flag
}

impl Stage1Result {
    fn should_try_next_stage(&self) -> bool {
        // Rejection criteria:
        // - detected/expected ratio < 0.5 OR > 1.5 → try Stage 2
        // - match_pct < 95% → try Stage 2
        // Else: skip to Stage 3+ (fast-fail)
        let ratio = self.detected_count as f64 / self.expected_count as f64;
        if ratio < 0.5 || ratio > 1.5 {
            return true;  // Wrong parameters, try Stage 2
        }
        if self.match_pct < 95.0 {
            return true;  // Need parameter tuning
        }
        false  // Fast-fail to Stage 3 or 4
    }
}
```

**3. Top 3 Edition Testing (~50 lines)**
```rust
fn test_top_3_editions(detected_durations: &[f64], editions: &[Edition]) -> Option<EditionMatch> {
    // Take top 3 editions by NDR rank
    // For each edition:
    //   Compare detected durations vs expected durations (±1.5s tolerance)
    //   Calculate match percentage
    // Return best match (highest %)
}
```

**4. Early Exit Logic (~20 lines)**
```rust
fn check_stage1_early_exit(result: &Stage1Result) -> Option<MatchResult> {
    // If match_pct >= 95.0:
    //   Return Some(MatchResult::Accepted { confidence: High, stage: 1, ... })
    // Else:
    //   Return None (proceed to Stage 2)
}
```

**5. Stage 1 Main Function (~20 lines)**
```rust
fn execute_stage1(file: &Path, editions: &[Edition]) -> Stage1Result {
    // Call stage1_default_silence_detection()
    // Check early exit
    // Return result with should_try_stage2 flag
}
```

**New Constants (Clarify CRITICAL):**
- `STAGE1_THRESHOLD_DB: f64 = ???` (spec says -54dB, Run 27 has -50dB)
- `STAGE1_MIN_DURATION_SECS: f64 = ???` (spec says 0.6s, Run 27 has 3.0s)
- `STAGE1_TOP_N_EDITIONS: usize = 3` (test top 3 by NDR)
- `STAGE1_ACCEPT_THRESHOLD_PCT: f64 = 95.0` (early exit threshold)
- `STAGE1_RATIO_MIN: f64 = 0.5` (under-segmentation threshold)
- `STAGE1_RATIO_MAX: f64 = 1.5` (over-segmentation threshold)

**Dependencies:**
- Audio decoding (Symphonia, reuse from Stage 2)
- Silence detection (RMS calculation, dB conversion, gap extraction, all exist in Run 27)
- Duration comparison (compare_durations(), exists in Run 27)
- Edition ranking (editions already ranked by NDR before Stage 1)

---

### Stage 6: Unmatchable Classification and Diagnostics (~200 lines)

**Location:** Terminal stage (after Stage 5 fails)

**Components to Implement:**

**1. Diagnostic Report Generation (~100 lines)**
```rust
struct DiagnosticReport {
    file_path: PathBuf,
    file_duration: f64,
    file_metadata: FileMetadata,  // artist, album, tags
    stage_results: Vec<StageResult>,  // All stage attempts
    best_attempt: BestAttempt,  // Highest match achieved
    unmatchable_reason: UnmatchableReason,
    suggested_actions: Vec<String>,
}

fn generate_diagnostic_report(
    file: &Path,
    editions: &[Edition],
    stage_results: &[StageResult],
) -> DiagnosticReport {
    // Collect all stage results:
    //   Stage 0: Pass/Fail + rejection reason (if failed)
    //   Stage 1: Detected count, match%, parameters used
    //   Stage 2: Parameter combinations tested, over-segmented candidates generated
    //   Stage 3: Assemblies tried, best assembly match%
    //   Stage 4: Quiet spots found, penalty-adjusted match%
    //   Stage 5: Merges tried, best merge match%
    // Identify best attempt (highest match% across all stages)
    // Classify unmatchable reason (analyze failure patterns)
    // Generate suggested actions
    // Return DiagnosticReport
}
```

**2. Unmatchable Reason Classification (~50 lines)**
```rust
enum UnmatchableClassification {
    Corrupt(String),         // File decodable but produces nonsense audio
    WrongMBData(String),     // File metadata points to wrong album, MB data incorrect
    NonStandard(String),     // Medleys, DJ mixes, live continuous recordings
    EdgeCase(String),        // Algorithm should have matched but didn't (potential bug)
}

fn classify_unmatchable_reason(report: &DiagnosticReport) -> UnmatchableClassification {
    // Analyze stage results and best attempt:
    // If Stage 0 failed with Corrupt: Return Corrupt
    // If best attempt is wrong artist (artist similarity <50%): Return WrongMBData
    // If all stages detected 0 or 1 tracks: Return NonStandard (continuous audio)
    // If best attempt match% is close (70-79%): Return EdgeCase (algorithm should work)
    // Else: Return NonStandard (fundamentally unmatchable)
}
```

**3. User Action Suggestions (~30 lines)**
```rust
fn suggest_user_actions(classification: &UnmatchableClassification, report: &DiagnosticReport) -> Vec<String> {
    // Based on classification:
    // Corrupt: "Check file integrity, try re-ripping CD, verify download"
    // WrongMBData: "Verify file metadata (artist/album tags), search MusicBrainz for correct album, report MB data issue"
    // NonStandard: "Consider manual track boundary editing, or accept as unmatchable"
    // EdgeCase: "Report as potential bug to developer with diagnostic report"
    // Return list of suggested actions
}
```

**4. Stage 6 Main Function (~20 lines)**
```rust
fn execute_stage6(
    file: &Path,
    editions: &[Edition],
    stage_results: &[StageResult],
) -> MatchResult {
    // Generate diagnostic report
    // Classify unmatchable reason
    // Suggest user actions
    // Log comprehensive diagnostic
    // Return MatchResult::Unmatchable { reason, diagnostic, stage: 6 }
}
```

**New Types:**
```rust
struct StageResult {
    stage: u8,
    attempted: bool,
    succeeded: bool,
    match_pct: Option<f64>,
    detected_count: Option<usize>,
    expected_count: Option<usize>,
    failure_reason: Option<String>,
    parameters_used: Option<serde_json::Value>,  // JSON for flexibility
}

struct BestAttempt {
    stage: u8,
    edition: Edition,
    match_pct: f64,
    detected_count: usize,
    expected_count: usize,
    mean_error: f64,
    detected_durations: Vec<f64>,
}
```

**Dependencies:**
- All stage results (passed from main match loop)
- File metadata extraction (lofty, exists in Run 27)
- String similarity (for wrong artist detection, exists in Run 27)

---

### Control Flow Refactoring (~100 lines)

**Location:** Main match_album() function

**Components to Refactor:**

**1. Main Match Function Restructure (~60 lines)**
```rust
fn match_album(file: &Path, mb_client: &MBClient, cache_config: &CacheConfig) -> MatchResult {
    // Extract metadata
    let metadata = extract_file_metadata(file)?;

    // Discover editions (existing logic, unchanged)
    let editions = discover_editions(&metadata, mb_client, cache_config).await?;

    // Stage 0: Pre-flight validation
    if let Err(reason) = stage0_preflight_validation(file, &metadata, &editions) {
        return MatchResult::Unmatchable { reason, stage: 0 };
    }

    // Stage 1: Default parameter silence detection
    let stage1_result = execute_stage1(file, &editions[0..3])?;
    if stage1_result.match_pct >= 95.0 {
        return MatchResult::Accepted { confidence: High, stage: 1, ... };
    }
    if !stage1_result.should_try_next_stage() {
        // Fast-fail to Stage 3 or 4
        return try_stage_3_or_4(file, editions);
    }

    // Stage 2: Full 180-parameter sweep (PRESERVED from Run 27)
    let stage2_result = execute_stage2(file, &editions[0..5])?;
    if stage2_result.match_pct >= 95.0 {
        return MatchResult::Accepted { confidence: High, stage: 2, ... };
    }

    // Stage 3: DP assembly (PRESERVED, conditional)
    if !stage2_result.over_segmented_candidates.is_empty() {
        let stage3_result = execute_stage3(&stage2_result.over_segmented_candidates, &editions[0..5])?;
        if stage3_result.match_pct >= 95.0 {
            return MatchResult::Accepted { confidence: High, stage: 3, ... };
        }
        if stage3_result.match_pct >= 80.0 {
            return MatchResult::Accepted { confidence: Medium, stage: 3, ... };
        }
    }

    // Stage 4: Quiet spots (PRESERVED)
    let stage4_result = execute_stage4(file, &editions[0..5])?;
    if stage4_result.match_pct >= 80.0 {
        return MatchResult::Accepted { confidence: Low, stage: 4, ... };
    }

    // Stage 5: Merging (PRESERVED, conditional)
    let best_so_far = get_best_result(&[stage1_result, stage2_result, stage3_result, stage4_result]);
    if best_so_far.detected > best_so_far.expected && best_so_far.match_pct >= 95.0 {
        let stage5_result = execute_stage5(&best_so_far, &editions[0..5])?;
        if stage5_result.match_pct == 100.0 {
            return MatchResult::Accepted { confidence: High, stage: 5, ... };
        }
    }

    // Stage 6: Unmatchable
    let all_stage_results = vec![stage1_result, stage2_result, stage3_result, stage4_result, stage5_result];
    return execute_stage6(file, &editions, &all_stage_results);
}
```

**2. Confidence Flag Assignment (~20 lines)**
```rust
enum Confidence {
    High,    // ≥95% match in Stages 1-2, or 100% in Stage 5
    Medium,  // 80-94% match in Stage 3
    Low,     // 80-94% match in Stages 4-5
}

fn assign_confidence(stage: u8, match_pct: f64) -> Confidence {
    match (stage, match_pct) {
        (1..=2, 95.0..) => Confidence::High,
        (5, 100.0) => Confidence::High,
        (3, 80.0..=94.9) => Confidence::Medium,
        (4..=5, 80.0..=94.9) => Confidence::Low,
        _ => Confidence::Low,  // Fallback
    }
}
```

**3. Fast-Fail Shortcuts (~20 lines)**
```rust
fn should_skip_to_stage_4(stage1_result: &Stage1Result) -> bool {
    // If detected/expected ratio is 0% or >200%, skip directly to Stage 4
    // Stage 2 won't help (fundamental mismatch)
    let ratio = stage1_result.detected_count as f64 / stage1_result.expected_count as f64;
    ratio < 0.01 || ratio > 2.0
}
```

**New Types:**
```rust
enum MatchResult {
    Accepted {
        edition: Edition,
        match_pct: f64,
        detected_durations: Vec<f64>,
        confidence: Confidence,
        stage: u8,
    },
    Unmatchable {
        reason: UnmatchableClassification,
        best_attempt: Option<BestAttempt>,
        diagnostic: DiagnosticReport,
        stage: u8,
    },
}
```

**Dependencies:**
- All stage execution functions (Stage 0-6)
- Edition discovery (unchanged)
- Confidence assignment logic
- Fast-fail heuristics

---

## External Dependencies (Unchanged)

### Rust Crates (from Cargo.toml)

**Audio Processing:**
- `symphonia = { version = "0.5", features = ["all"] }` - Audio decoding (FLAC, MP3, etc.)
- `lofty = "0.18"` - Metadata extraction (ID3 tags)
- `chromaprint-sys-next = "0.1"` - Audio fingerprinting (via FFI to C library)

**Async and Concurrency:**
- `tokio = { version = "1", features = ["full"] }` - Async runtime
- `rayon = "1.7"` - Data parallelism (parallel edition processing)
- `futures = "0.3"` - Async utilities (stream, StreamExt)

**HTTP and JSON:**
- `reqwest = { version = "0.11", features = ["json"] }` - HTTP client (MusicBrainz API)
- `serde = { version = "1.0", features = ["derive"] }` - Serialization framework
- `serde_json = "1.0"` - JSON parsing (MB responses, cache files)

**String Similarity:**
- `strsim = "0.10"` - Levenshtein and Jaro-Winkler algorithms

**Logging:**
- `tracing = "0.1"` - Structured logging
- `tracing-subscriber = { version = "0.3", features = ["env-filter", "time"] }` - Logging configuration
- `time = "0.3"` - Timestamp formatting (OffsetTime)

**Crypto:**
- `sha2 = "0.10"` - SHA-256 hashing (cache keys)

**Utilities:**
- `regex = "1.10"` - Regular expressions (single-track filename patterns)

**No New Dependencies Required:** All functionality can be implemented with existing crates.

---

## Dependency Risk Analysis

### Low Risk (Well-Tested)

**Stages 2-5 Preservation:**
- Risk: LOW
- Rationale: Exact copy of Run 27 code, proven to work
- Mitigation: No modifications, only renaming/reorganization

**MusicBrainz API Client:**
- Risk: LOW
- Rationale: Existing implementation, well-tested
- Mitigation: No changes to API interaction

**Cache Infrastructure:**
- Risk: LOW
- Rationale: PLAN026 implementation, validated
- Mitigation: No structural changes

### Medium Risk (New Code, Reusing Components)

**Stage 0 Validation:**
- Risk: MEDIUM
- Rationale: Consolidates scattered logic, potential for integration bugs
- Mitigation: Extensive unit tests on each validation check
- Fallback: Keep scattered validation if consolidation breaks

**Stage 1 Default Parameters:**
- Risk: MEDIUM
- Rationale: Reuses silence detection from Stage 2, but different parameters
- Mitigation: Validate parameters in Phase 1, test on full dataset
- Fallback: Adjust parameters or abandon Stage 1 if success rate <70%

**Control Flow Refactor:**
- Risk: MEDIUM
- Rationale: Restructure main loop, risk of breaking stage transitions
- Mitigation: Integration tests on stage transitions, A/B comparison vs Run 27
- Fallback: Revert to Run 27 control flow if regressions detected

### High Risk (New Functionality)

**Stage 6 Diagnostics:**
- Risk: HIGH
- Rationale: New diagnostic generation, classification logic is heuristic
- Mitigation: Test on diverse unmatchable albums, validate classification accuracy
- Fallback: Simplify diagnostics if classification is unreliable (basic logging only)

**Default Parameter Assumption:**
- Risk: HIGH
- Rationale: CRITICAL ISSUE CRIT-02 - spec says -54dB/0.6s, Run 27 has -50dB/3.0s
- Mitigation: Phase 1 empirical validation of both parameter sets
- Fallback: Use empirically validated parameters, not specification assumptions

---

## Implementation Sequence

**Phase 1 (Weeks 1-2): Validate Stage 1 Assumptions**
1. Extract Stage 1 default parameters from Run 27 or specification (resolve CRIT-02)
2. Implement Stage 1 alongside Run 27 (parallel execution, no refactoring yet)
3. Run on 179-album dataset, log Stage 1 results (success rate, timing)
4. Validate 70-80% success rate assumption
5. Measure Stage 1 execution time (<10s target)
6. **Decision Point:** If Stage 1 success <70%, adjust parameters or abort simplicity-first

**Phase 2 (Week 3): Implement Stage 0**
1. Consolidate validation logic from Run 27
2. Implement single-track detection consolidation
3. Unit tests on invalid files (corrupt, too short, singles)
4. Validate 5-10% rejection rate on dataset
5. **Decision Point:** If rejection accuracy <90%, refine validation logic

**Phase 3 (Weeks 4-5): Integrate Stage 1**
1. Add Stage 1 as first matching attempt (before Stage 2)
2. Implement rejection criteria and Stage 2 transition
3. Test early exit on clear-gap albums
4. Validate no regressions vs Run 27 (same match results)
5. **Decision Point:** If regressions detected, fix or revert

**Phase 4 (Week 6): Implement Stage 6**
1. Implement diagnostic report generation
2. Add unmatchable reason classification
3. Test on Run 27 failures
4. Validate diagnostic quality (actionable information)
5. **Decision Point:** If diagnostics are unclear, simplify or enhance

**Phase 5 (Weeks 7-8): Control Flow Refactor**
1. Restructure main loop for 0→6 progression
2. Implement confidence flags and fast-fail shortcuts
3. Preserve Stages 2-5 logic (no changes)
4. Full regression testing vs Run 27
5. **Decision Point:** If any regressions, fix or revert to Run 27

**Phase 6 (Week 9): Performance Validation**
1. Full dataset run (179 albums)
2. Measure weighted average timing (target ≤150s)
3. Measure automatic success rate (target ≥98%)
4. Measure unmatchable rate (target ≤2%)
5. A/B comparison vs Run 27 (success rate, timing, match quality)
6. **Decision Point:** If targets not met, optimize or abandon

---

**Document Version:** 1.0
**Last Updated:** 2025-11-25
**Total Existing Code:** ~7324 lines (Run 27)
**Total New Code:** ~650 lines (+8.9%)
**Final Size:** ~7974 lines (Run 28)
