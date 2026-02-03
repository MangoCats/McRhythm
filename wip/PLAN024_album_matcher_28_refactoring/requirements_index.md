# Requirements Index: Album Matcher 28 Refactoring

**Plan:** PLAN024
**Specification:** SPEC_album_matcher_28_refactoring.md
**Date:** 2025-11-25
**Status:** Phase 1 - Requirements Extraction

---

## Requirements Summary

**Total Requirements:** 22 (5 functional + 6 structural + 3 build + 4 testing + 4 documentation)

**Primary Objective:** Refactor monolithic album_matcher_28.rs (~7,400 lines) into modular folder structure (am28/) while preserving 100% functionality.

---

## 1. Functional Requirements (Preservation)

### REQ-FUNC-001: Identical Results
**Priority:** CRITICAL
**Source:** SPEC_album_matcher_28_refactoring.md:45-46
**Description:** The refactored implementation SHALL produce identical results to album_matcher_28.rs for all inputs.
**Verification:** Run both versions on same dataset, compare outputs
**Acceptance Criteria:**
- All 193 successful albums from Run 28 match with identical "Best parameters"
- Same 5 albums fail with same error messages
- Output logs match line-by-line (except timestamps)

### REQ-FUNC-002: Parameter Order Preservation
**Priority:** CRITICAL
**Source:** SPEC_album_matcher_28_refactoring.md:48-49
**Description:** All 180-parameter combinations SHALL be tested in the same order as album_matcher_28.rs.
**Verification:** Verify STAGE2_THRESHOLD_VALUES and STAGE2_MIN_DURATION_VALUES arrays preserved
**Acceptance Criteria:**
- STAGE2_THRESHOLD_VALUES array identical (12 values in Run 28 order)
- STAGE2_MIN_DURATION_VALUES array identical (15 values in Run 28 order)
- Parameter iteration logic unchanged

### REQ-FUNC-003: Early-Exit Logic Preservation
**Priority:** CRITICAL
**Source:** SPEC_album_matcher_28_refactoring.md:51-52
**Description:** Early-exit optimization SHALL trigger at the same conditions as album_matcher_28.rs.
**Verification:** Compare early-exit behavior for 100% match albums
**Acceptance Criteria:**
- 169 albums achieving 100% match exit at same parameter rank as Run 28
- Partial matches continue testing all 180 parameters (no premature exit)

### REQ-FUNC-004: MusicBrainz Caching Preservation
**Priority:** HIGH
**Source:** SPEC_album_matcher_28_refactoring.md:54-55
**Description:** MusicBrainz API caching SHALL function identically to album_matcher_28.rs.
**Verification:** Compare cache hit/miss patterns
**Acceptance Criteria:**
- Cache modes (Disabled, ReadWrite, ReadOnly) function identically
- Cache file format unchanged (backward compatible)
- API rate limiting identical (2-second interval)

### REQ-FUNC-005: Stage Logic Preservation
**Priority:** CRITICAL
**Source:** SPEC_album_matcher_28_refactoring.md:57-58
**Description:** All 5 matching stages (Stages 2-5 plus validation) SHALL execute with same logic.
**Verification:** Trace execution for sample albums, verify stage transitions
**Acceptance Criteria:**
- Stage 2: 180-parameter sweep with early-exit
- Stage 3: DP assembly for over-segmented candidates
- Stage 4: Edition-guided quiet spot detection (RMS profiling)
- Stage 5: Extra track merging
- Single-track discriminator and name validation unchanged

---

## 2. Structural Requirements (Modularity)

### REQ-STRUCT-001: Folder Organization
**Priority:** HIGH
**Source:** SPEC_album_matcher_28_refactoring.md:62-63
**Description:** The refactored code SHALL be organized in a folder structure: `wkmp-ai/examples/am28/`
**Verification:** Verify folder structure exists and matches proposed layout
**Acceptance Criteria:**
- Folder structure: am28/ with main.rs, mod.rs, types.rs, constants.rs
- Subfolders: musicbrainz/, stages/, matching/, utils/
- README.md present in am28/ root

### REQ-STRUCT-002: Standalone Example Compilation
**Priority:** CRITICAL
**Source:** SPEC_album_matcher_28_refactoring.md:65-66
**Description:** The refactored code SHALL compile as a standalone example using: `cargo run --example album_matcher_28`
**Verification:** Execute command and verify successful compilation
**Acceptance Criteria:**
- `cargo run --example album_matcher_28` compiles without errors
- Same CLI arguments supported (--training-set, --output, --limit, --cache-mode)
- Output written to same locations as Run 28

### REQ-STRUCT-003: Logical Domain Organization
**Priority:** HIGH
**Source:** SPEC_album_matcher_28_refactoring.md:68-69
**Description:** Modules SHALL be logically organized by functional domain (e.g., silence detection, MusicBrainz API, stage execution).
**Verification:** Review module organization against proposed structure
**Acceptance Criteria:**
- silence_detection.rs: All silence detection logic
- musicbrainz/: All MB API client, cache, types
- stages/: All stage2-5 execution logic
- matching/: All candidate testing, edition selection
- utils/: All utility functions (audio, fingerprint, timing)

### REQ-STRUCT-004: Module Size Limit
**Priority:** MEDIUM
**Source:** SPEC_album_matcher_28_refactoring.md:71-72
**Description:** Each module file SHALL be <1000 lines.
**Verification:** Count lines in each module file
**Acceptance Criteria:**
- No single .rs file exceeds 1000 lines
- Largest modules split further if needed

### REQ-STRUCT-005: Explicit Dependencies
**Priority:** HIGH
**Source:** SPEC_album_matcher_28_refactoring.md:74-75
**Description:** Module dependencies SHALL be explicit (via `use` statements and function signatures).
**Verification:** Review module imports and function signatures
**Acceptance Criteria:**
- All module dependencies declared via `use` statements
- No hidden global state (all state passed via parameters)
- Function signatures clearly show data flow

### REQ-STRUCT-006: Shared Data Structures
**Priority:** HIGH
**Source:** SPEC_album_matcher_28_refactoring.md:77-78
**Description:** Common data structures SHALL be defined once and shared across modules.
**Verification:** Verify types.rs contains all shared structs/enums
**Acceptance Criteria:**
- types.rs defines: CandidateTestResult, OverSegmentedCandidate, SingleEditionStage2Results
- No duplicate struct definitions across modules
- All modules import from types.rs

---

## 3. Build Requirements

### REQ-BUILD-001: Warning-Free Compilation
**Priority:** HIGH
**Source:** SPEC_album_matcher_28_refactoring.md:82-83
**Description:** The refactored code SHALL compile without warnings using `cargo check --example album_matcher_28`.
**Verification:** Run cargo check, verify zero warnings
**Acceptance Criteria:**
- `cargo check --example album_matcher_28` reports 0 warnings
- All `#[allow(...)]` attributes preserved where intentional
- Unused code eliminated or marked appropriately

### REQ-BUILD-002: Release Mode Compilation
**Priority:** HIGH
**Source:** SPEC_album_matcher_28_refactoring.md:85-86
**Description:** The refactored code SHALL compile in release mode using `cargo build --release --example album_matcher_28`.
**Verification:** Run cargo build, verify successful compilation
**Acceptance Criteria:**
- Release mode compilation succeeds
- Optimizations applied (inlining, dead code elimination)
- Binary size comparable to Run 28 (within 10%)

### REQ-BUILD-003: Compilation Time Constraint
**Priority:** MEDIUM
**Source:** SPEC_album_matcher_28_refactoring.md:88-89
**Description:** Compilation time SHALL NOT increase by more than 10% compared to monolithic version.
**Verification:** Measure `cargo clean && cargo build --timings`, compare
**Acceptance Criteria:**
- Total compilation time ≤ 110% of Run 28 baseline
- No excessive template instantiation overhead
- Module parallelization benefits visible in --timings report

---

## 4. Testing Requirements

### REQ-TEST-001: Run 27 Dataset Validation
**Priority:** CRITICAL
**Source:** SPEC_album_matcher_28_refactoring.md:93-94
**Description:** The refactored code SHALL pass all existing validation when run on Run 27 dataset.
**Verification:** Run on 200 albums from training_set.txt, compare with Run 28 baseline
**Acceptance Criteria:**
- All 193 albums succeed (same as Run 28)
- Same 5 albums fail (A14, A17, A30, A53, A83 or whatever Run 28 failed)
- Output format identical to Run 28

### REQ-TEST-002: Parameter Match Verification
**Priority:** CRITICAL
**Source:** SPEC_album_matcher_28_refactoring.md:96-97
**Description:** The refactored code SHALL produce identical "Best parameters" for each album.
**Verification:** Compare best parameters reported for 193 successful albums
**Acceptance Criteria:**
- All 193 albums report same (threshold, min_duration) as Run 28
- No parameter drift or rounding errors

### REQ-TEST-003: Success Rate Preservation
**Priority:** CRITICAL
**Source:** SPEC_album_matcher_28_refactoring.md:99-100
**Description:** The refactored code SHALL have same success/failure rate (193 success, 5 failed).
**Verification:** Count successful vs failed albums, compare totals
**Acceptance Criteria:**
- Exactly 193 successful albums
- Exactly 5 failed albums
- Same albums fail as Run 28

### REQ-TEST-004: Unit Testability
**Priority:** MEDIUM
**Source:** SPEC_album_matcher_28_refactoring.md:102-103
**Description:** Individual modules SHALL be testable in isolation (unit testable).
**Verification:** Demonstrate ability to write unit tests for at least 3 key modules
**Acceptance Criteria:**
- At least 3 modules have demonstrable unit test capability
- Example: silence_detection.rs, candidate.rs, validation.rs
- Tests can run via `cargo test --example album_matcher_28` (optional, proof of concept)

---

## 5. Documentation Requirements

### REQ-DOC-001: Module-Level Documentation
**Priority:** HIGH
**Source:** SPEC_album_matcher_28_refactoring.md:107-108
**Description:** Each module file SHALL have a module-level documentation comment explaining its purpose.
**Verification:** Check each module has `//!` comment at top
**Acceptance Criteria:**
- All .rs files in am28/ have `//!` module doc comment
- Comment explains module purpose, key responsibilities
- Cross-references to related modules included

### REQ-DOC-002: Function Documentation
**Priority:** MEDIUM
**Source:** SPEC_album_matcher_28_refactoring.md:110-111
**Description:** Public functions SHALL have doc comments (`///`) explaining parameters and return values.
**Verification:** Run `cargo doc` for example, verify documentation generated
**Acceptance Criteria:**
- All public functions have `///` doc comments
- Parameters and return values documented
- Examples included for complex functions

### REQ-DOC-003: Module Structure README
**Priority:** HIGH
**Source:** SPEC_album_matcher_28_refactoring.md:113-114
**Description:** A README.md SHALL exist in am28/ folder explaining module structure and organization.
**Verification:** Check am28/README.md exists and documents module map
**Acceptance Criteria:**
- am28/README.md exists
- Documents module hierarchy and responsibilities
- Provides navigation guide for developers

### REQ-DOC-004: Algorithm Comment Preservation
**Priority:** MEDIUM
**Source:** SPEC_album_matcher_28_refactoring.md:116-117
**Description:** Complex algorithms (DP assembly, RMS profiling) SHALL retain existing detailed comments.
**Verification:** Verify comments preserved during refactoring
**Acceptance Criteria:**
- DP assembly algorithm comments preserved in stage3.rs
- RMS profiling comments preserved in stage4.rs
- No reduction in comment quality or clarity

---

## Requirements Traceability

| Category | Count | Priority Breakdown |
|----------|-------|-------------------|
| Functional (FUNC) | 5 | CRITICAL: 4, HIGH: 1 |
| Structural (STRUCT) | 6 | CRITICAL: 1, HIGH: 4, MEDIUM: 1 |
| Build (BUILD) | 3 | HIGH: 2, MEDIUM: 1 |
| Testing (TEST) | 4 | CRITICAL: 3, MEDIUM: 1 |
| Documentation (DOC) | 4 | HIGH: 2, MEDIUM: 2 |
| **TOTAL** | **22** | **CRITICAL: 8, HIGH: 9, MEDIUM: 5** |

**Critical Path Requirements (Must Pass):**
- REQ-FUNC-001, REQ-FUNC-002, REQ-FUNC-003, REQ-FUNC-005 (algorithm preservation)
- REQ-STRUCT-002 (compilation)
- REQ-TEST-001, REQ-TEST-002, REQ-TEST-003 (verification)

---

## Dependencies

**Upstream:**
- SPEC_album_matcher_28_refactoring.md (specification)
- album_matcher_28.rs (source code to refactor)
- album_matcher_output_run28.txt (baseline results, when available)

**Downstream:**
- Implementation plan (Phase 2)
- Test specifications (Phase 3)
- Modular implementation (execution)

---

## Open Issues

None at this time. All requirements extracted from specification.

---

## Document Control

**Version:** 1.0
**Status:** Phase 1 Complete
**Created:** 2025-11-25
**Last Updated:** 2025-11-25
