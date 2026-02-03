# Dependencies Map: Album Matcher 28 Refactoring

**Plan:** PLAN024
**Specification:** SPEC_album_matcher_28_refactoring.md
**Date:** 2025-11-25
**Status:** Phase 1 - Dependencies Mapping

---

## 1. Source Code Dependencies

### 1.1 Primary Source

**File:** wkmp-ai/examples/album_matcher_28.rs
- **Size:** ~7,400 lines
- **Status:** Implemented, compiled successfully
- **Purpose:** Source code to be refactored
- **Preservation:** 100% algorithm logic must be preserved

### 1.2 Reference Implementations

**File:** wkmp-ai/examples/album_matcher_27.rs
- **Purpose:** Run 27 baseline implementation
- **Relevance:** Design patterns, algorithm structure
- **Usage:** Reference only (not modified)

**Document:** album_matcher_27_designs.md
- **Purpose:** Design documentation for Run 27
- **Relevance:** User stated "should mostly apply to album_matcher_28.rs' designs as well"
- **Usage:** Reference for architectural patterns

---

## 2. External Dependencies (Cargo Crates)

### 2.1 Audio Processing

**symphonia (0.5.x):**
- **Purpose:** MP3 decoding, format detection
- **Usage:** utils/audio.rs module
- **Critical:** Exact version must be preserved (API stability)

**rubato:**
- **Purpose:** Sample rate conversion (if used)
- **Usage:** Audio pipeline
- **Note:** May not be used in album_matcher_28.rs (verify)

### 2.2 HTTP and Networking

**reqwest:**
- **Purpose:** MusicBrainz API client
- **Usage:** musicbrainz/api.rs module
- **Critical:** Async runtime (Tokio) integration required

**tokio:**
- **Purpose:** Async runtime, rate limiting (Semaphore)
- **Usage:** Main runtime, MB API throttling, heartbeat thread
- **Critical:** Version must support current code patterns

### 2.3 Serialization and Data

**serde, serde_json:**
- **Purpose:** JSON serialization (MusicBrainz responses, cache)
- **Usage:** musicbrainz/cache.rs, API response parsing
- **Critical:** JSON schema must be preserved for cache compatibility

### 2.4 Audio Fingerprinting

**chromaprint-rust:**
- **Purpose:** AcoustID fingerprinting
- **Usage:** utils/fingerprint.rs module
- **Note:** May be optional feature (verify usage)

### 2.5 Logging and Tracing

**tracing, tracing-subscriber:**
- **Purpose:** Structured logging, debug output
- **Usage:** All modules (info!, debug!, warn! macros)
- **Critical:** Log output format should match Run 28

### 2.6 CLI and Utilities

**clap:**
- **Purpose:** CLI argument parsing
- **Usage:** main.rs argument handling
- **Critical:** Preserve exact argument names and defaults

**anyhow:**
- **Purpose:** Error handling, Result types
- **Usage:** All modules for error propagation
- **Note:** Consider if thiserror would be better (out of scope)

---

## 3. Data Dependencies

### 3.1 Input Data

**File:** training_set.txt
- **Format:** Text file with album file paths (one per line)
- **Usage:** Input dataset for testing/validation
- **Location:** Project root
- **Preservation:** MUST NOT modify, read-only

**Audio Files:** Referenced by training_set.txt
- **Format:** MP3 files (full albums)
- **Usage:** Album matching input
- **Location:** User's music library
- **Preservation:** Read-only access

### 3.2 Cache Data

**Folder:** cache/
- **Format:** JSON files (MusicBrainz API responses)
- **Usage:** MB API caching (3 modes: Disabled, ReadWrite, ReadOnly)
- **Location:** Project root or configurable
- **Preservation:** MUST maintain backward compatibility (read existing cache)

**Cache Structure:**
- File naming: `{album_fingerprint}.json` or similar
- Content: JSON serialized MB API responses
- Metadata: Timestamps, cache validity markers

### 3.3 Output Data

**File:** album_matcher_output_run28.txt (or user-specified)
- **Format:** Text log output
- **Usage:** Refactored code verification baseline
- **Location:** Project root or --output argument
- **Preservation:** Output format MUST match Run 28

---

## 4. Algorithm Dependencies

### 4.1 Constants and Configuration

**STAGE2_THRESHOLD_VALUES (12 values):**
```rust
[-50.0, -58.0, -60.0, -62.0, -56.0, -52.0, -54.0, -48.0, -40.0, -38.0, -34.0, -30.0]
```
- **Source:** album_matcher_28.rs lines 812-825
- **Purpose:** Silence detection thresholds (dB)
- **Preservation:** Exact order CRITICAL (Run 28 parameter reordering)

**STAGE2_MIN_DURATION_VALUES (15 values):**
```rust
[3.0, 2.0, 0.5, 1.5, 0.3, 0.8, 0.1, 4.0, 0.2, 2.5, 1.0, 0.4, 5.0, 3.5, 0.05]
```
- **Source:** album_matcher_28.rs lines 842-858
- **Purpose:** Minimum silence duration (seconds)
- **Preservation:** Exact order CRITICAL (Run 28 parameter reordering)

**Other Constants:**
- DEFAULT_THRESHOLD_DB = -50.0
- DEFAULT_MIN_DURATION_SECS = 3.0
- Tolerance percentages (track duration matching)
- Grace periods, penalties (edition scoring)

### 4.2 Core Algorithms

**WindowDbProfile:**
- **Purpose:** Pre-computed dB levels for 180-parameter sweep
- **Location:** silence_detection.rs module
- **Preservation:** Algorithm unchanged, API preserved

**Silence Detection:**
- **Purpose:** Find silence regions given threshold and min_duration
- **Location:** silence_detection.rs module
- **Preservation:** Algorithm unchanged

**Dynamic Programming Assembly (Stage 3):**
- **Purpose:** Assemble over-segmented tracks into proper track boundaries
- **Location:** stages/stage3.rs module
- **Preservation:** Algorithm unchanged, detailed comments preserved

**RMS Profiling (Stage 4):**
- **Purpose:** Edition-guided quiet spot detection using RMS levels
- **Location:** stages/stage4.rs module
- **Preservation:** Algorithm unchanged, complex logic preserved

**Track Merging (Stage 5):**
- **Purpose:** Merge extra tracks to improve match
- **Location:** stages/stage5.rs module
- **Preservation:** Algorithm unchanged

### 4.3 Matching Logic

**Candidate Testing:**
- **Purpose:** Compare detected tracks vs expected durations, calculate match %
- **Location:** matching/candidate.rs module
- **Preservation:** Tolerance logic, percentage calculation unchanged

**Edition Selection:**
- **Purpose:** Select winning edition considering track count penalty
- **Location:** matching/edition.rs module
- **Preservation:** Scoring logic unchanged

**Validation:**
- **Purpose:** Single-track discriminator, artist/album name validation
- **Location:** matching/validation.rs module
- **Preservation:** Heuristics unchanged

---

## 5. External Service Dependencies

### 5.1 MusicBrainz API

**Endpoint:** https://musicbrainz.org/ws/2/
- **Purpose:** Search for releases, fetch release details
- **Usage:** musicbrainz/api.rs module
- **Rate Limiting:** 2-second interval between requests (MUST preserve)
- **Authentication:** None required (public API)

**API Calls:**
- Search: `/ws/2/release/?query={artist}+{album}&fmt=json`
- Release Details: `/ws/2/release/{mbid}?inc=recordings&fmt=json`

**Response Format:**
- JSON with releases, recordings, track durations
- MUST preserve response parsing logic (field names, structure)

---

## 6. Build System Dependencies

### 6.1 Cargo Configuration

**File:** Cargo.toml
- **Dependencies:** All crates listed above
- **Workspace:** wkmp-ai package
- **Example Configuration:** [[example]] section for album_matcher_28

**Example Entry:**
```toml
[[example]]
name = "album_matcher_28"
path = "examples/album_matcher_28.rs"  # Wrapper file pointing to am28/main.rs
```

### 6.2 Compilation Requirements

**Rust Version:** Stable channel (1.70+ assumed)
- **Features:** No nightly-only features
- **Platform:** Windows primary, cross-platform expected

**Optimization:**
- Release mode: `-O` flag, inlining enabled
- Debug mode: Full symbols, no optimizations

---

## 7. Testing Dependencies

### 7.1 Validation Baseline

**File:** album_matcher_output_run28.txt
- **Status:** Not yet generated (Run 28 not tested yet)
- **Fallback:** album_matcher_output_run27.txt (Run 27 baseline)
- **Purpose:** Line-by-line comparison for regression testing

**Expected Results (from Run 27):**
- 193 successful albums
- 5 failed albums (IDs unknown until Run 28 tested)
- Specific "Best parameters" for each successful album

### 7.2 Test Dataset

**File:** training_set.txt (200 albums)
- **Purpose:** Full dataset validation
- **Subset:** First 10 albums for incremental verification
- **Usage:** Each refactoring phase runs 10-album test before proceeding

---

## 8. Documentation Dependencies

### 8.1 Reference Documentation

**PLAN027 Phase 1 Validation Report:**
- **File:** wip/PLAN027_album_matcher_simplicity_first/PHASE1_VALIDATION_REPORT.md
- **Purpose:** Empirical analysis justifying Run 28 parameter reordering
- **Relevance:** Explains why parameter order matters (early-exit optimization)

**Run 28 Implementation Summary:**
- **File:** wip/PLAN027_album_matcher_simplicity_first/IMPLEMENTATION_SUMMARY_RUN28.md
- **Purpose:** Documents what changed from Run 27 to Run 28
- **Relevance:** Identifies critical areas to preserve

**Specification:**
- **File:** wip/SPEC_album_matcher_28_refactoring.md
- **Purpose:** Defines requirements for this refactoring
- **Relevance:** Source of truth for requirements

### 8.2 Analysis Scripts

**analyze_best_params.py:**
- **Purpose:** Extract "Best parameters" from output logs
- **Usage:** Verify REQ-TEST-002 (parameter match verification)

**analyze_timing.py:**
- **Purpose:** Measure component timing
- **Usage:** Verify performance within ±10% (REQ-TEST-001)

**analyze_early_exit_potential.py:**
- **Purpose:** Analyze early-exit optimization effectiveness
- **Usage:** Understand Run 28 optimization benefits

---

## 9. Dependency Graph

### 9.1 Module Dependency Flow

```
main.rs
├── types.rs (shared data structures)
├── constants.rs (configuration)
├── silence_detection.rs
│   └── types.rs
├── musicbrainz/
│   ├── api.rs (reqwest, tokio)
│   ├── cache.rs (serde_json)
│   └── types.rs
├── stages/
│   ├── stage2.rs → silence_detection.rs, types.rs, constants.rs
│   ├── stage3.rs → types.rs, matching/candidate.rs
│   ├── stage4.rs → utils/audio.rs, types.rs
│   └── stage5.rs → types.rs, matching/candidate.rs
├── matching/
│   ├── candidate.rs → types.rs, constants.rs
│   ├── edition.rs → stages/, matching/candidate.rs, types.rs
│   └── validation.rs → types.rs
└── utils/
    ├── audio.rs (symphonia)
    ├── fingerprint.rs (chromaprint-rust)
    └── timing.rs (tokio)
```

**Key Observations:**
- types.rs is foundation (shared by all modules)
- constants.rs used by stages and matching
- No circular dependencies (bottom-up design)

### 9.2 External Dependency Graph

```
album_matcher_28 (refactored)
├── Cargo dependencies (reqwest, tokio, symphonia, serde, clap, etc.)
├── MusicBrainz API (external service)
├── Audio files (user's music library)
├── training_set.txt (input dataset)
├── cache/ folder (MB API cache)
└── Output file (verification baseline)
```

---

## 10. Risks Related to Dependencies

### 10.1 Version Compatibility Risk

**Risk:** Cargo dependency version updates break code during refactoring.

**Mitigation:**
- Do NOT update dependencies during refactoring
- Preserve exact Cargo.lock state
- Update dependencies in separate effort (out of scope)

**Residual Risk:** LOW (frozen dependencies)

### 10.2 Cache Format Compatibility Risk

**Risk:** Refactored code cannot read existing MusicBrainz cache.

**Mitigation:**
- Preserve exact JSON serialization logic (serde derives)
- Test with existing cache/ folder
- Verify cache read/write in Phase 3 (musicbrainz/ module extraction)

**Residual Risk:** LOW (JSON schema unchanged)

### 10.3 External Service Availability Risk

**Risk:** MusicBrainz API unavailable during testing.

**Mitigation:**
- Use cache mode: ReadOnly (--cache-mode readonly)
- Test with cached data from previous runs
- Only use live API if cache miss (rare for training_set.txt)

**Residual Risk:** LOW (cache available)

### 10.4 Data File Availability Risk

**Risk:** training_set.txt references missing audio files.

**Mitigation:**
- Use --limit 10 for incremental verification (fewer files)
- Verify audio files exist before full run
- Document any missing files (expected failures)

**Residual Risk:** LOW (user has existing dataset)

---

## Document Control

**Version:** 1.0
**Status:** Phase 1 Complete
**Created:** 2025-11-25
**Last Updated:** 2025-11-25

**Dependencies:**
- Upstream: SPEC_album_matcher_28_refactoring.md, album_matcher_28.rs
- Downstream: Implementation plan (Phase 2), Test specifications (Phase 3)
