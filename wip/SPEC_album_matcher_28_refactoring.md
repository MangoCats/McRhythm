# Specification: Album Matcher 28 Refactoring

**Document Type:** Design Specification
**Status:** Draft
**Created:** 2025-11-25
**Purpose:** Define requirements for refactoring album_matcher_28.rs into modular architecture

---

## 1. Overview

### 1.1 Problem Statement

The current album_matcher_28.rs is a monolithic file (~7,400 lines) that implements the complete album matching pipeline. While functional, this structure presents maintenance challenges:

- Difficult to understand: All code in single file
- Difficult to test: Tight coupling between components
- Difficult to modify: Changes affect large code surface area
- Difficult to reuse: No clean module boundaries

### 1.2 Solution Approach

Refactor album_matcher_28.rs into a modular folder structure (am28/) with logical component separation while preserving all existing functionality. The refactored code SHALL remain a self-contained standalone example.

### 1.3 Goals

**Primary Goals:**
- Improve maintainability through clear module boundaries
- Enable focused testing of individual components
- Preserve 100% of existing functionality
- Maintain standalone example capability

**Non-Goals:**
- Change algorithm behavior (Run 28 functionality preserved exactly)
- Add new features
- Optimize performance beyond existing implementation
- Create reusable library crate (remains example-specific)

---

## 2. Requirements

### 2.1 Functional Requirements (Preservation)

**REQ-FUNC-001:** The refactored implementation SHALL produce identical results to album_matcher_28.rs for all inputs.
- **Verification:** Run both versions on same dataset, compare outputs

**REQ-FUNC-002:** All 180-parameter combinations SHALL be tested in the same order as album_matcher_28.rs.
- **Verification:** Verify STAGE2_THRESHOLD_VALUES and STAGE2_MIN_DURATION_VALUES arrays preserved

**REQ-FUNC-003:** Early-exit optimization SHALL trigger at the same conditions as album_matcher_28.rs.
- **Verification:** Compare early-exit behavior for 100% match albums

**REQ-FUNC-004:** MusicBrainz API caching SHALL function identically to album_matcher_28.rs.
- **Verification:** Compare cache hit/miss patterns

**REQ-FUNC-005:** All 5 matching stages (Stages 2-5 plus validation) SHALL execute with same logic.
- **Verification:** Trace execution for sample albums, verify stage transitions

### 2.2 Structural Requirements (Modularity)

**REQ-STRUCT-001:** The refactored code SHALL be organized in a folder structure: `wkmp-ai/examples/am28/`
- **Rationale:** Clear separation from monolithic file, indicates modular structure

**REQ-STRUCT-002:** The refactored code SHALL compile as a standalone example using: `cargo run --example album_matcher_28`
- **Rationale:** Maintains existing invocation method, no workflow disruption

**REQ-STRUCT-003:** Modules SHALL be logically organized by functional domain (e.g., silence detection, MusicBrainz API, stage execution).
- **Rationale:** Clear separation of concerns, easier to locate code

**REQ-STRUCT-004:** Each module file SHALL be <1000 lines.
- **Rationale:** Manageable file sizes for editing and review

**REQ-STRUCT-005:** Module dependencies SHALL be explicit (via `use` statements and function signatures).
- **Rationale:** Clear understanding of component interactions

**REQ-STRUCT-006:** Common data structures SHALL be defined once and shared across modules.
- **Rationale:** DRY principle, single source of truth

### 2.3 Compilation and Build Requirements

**REQ-BUILD-001:** The refactored code SHALL compile without warnings using `cargo check --example album_matcher_28`.
- **Verification:** Run cargo check, verify zero warnings

**REQ-BUILD-002:** The refactored code SHALL compile in release mode using `cargo build --release --example album_matcher_28`.
- **Verification:** Run cargo build, verify successful compilation

**REQ-BUILD-003:** Compilation time SHALL NOT increase by more than 10% compared to monolithic version.
- **Verification:** Measure `cargo clean && cargo build --timings`, compare

### 2.4 Testing Requirements

**REQ-TEST-001:** The refactored code SHALL pass all existing validation when run on Run 27 dataset.
- **Verification:** Run on 200 albums from training_set.txt, compare with Run 28 baseline

**REQ-TEST-002:** The refactored code SHALL produce identical "Best parameters" for each album.
- **Verification:** Compare best parameters reported for 193 successful albums

**REQ-TEST-003:** The refactored code SHALL have same success/failure rate (193 success, 5 failed).
- **Verification:** Count successful vs failed albums, compare totals

**REQ-TEST-004:** Individual modules SHALL be testable in isolation (unit testable).
- **Verification:** Demonstrate ability to write unit tests for at least 3 key modules

### 2.5 Documentation Requirements

**REQ-DOC-001:** Each module file SHALL have a module-level documentation comment explaining its purpose.
- **Verification:** Check each module has `//!` comment at top

**REQ-DOC-002:** Public functions SHALL have doc comments (`///`) explaining parameters and return values.
- **Verification:** Run `cargo doc` for example, verify documentation generated

**REQ-DOC-003:** A README.md SHALL exist in am28/ folder explaining module structure and organization.
- **Verification:** Check am28/README.md exists and documents module map

**REQ-DOC-004:** Complex algorithms (DP assembly, RMS profiling) SHALL retain existing detailed comments.
- **Verification:** Verify comments preserved during refactoring

---

## 3. Proposed Module Structure

### 3.1 High-Level Organization

```
wkmp-ai/examples/am28/
├── main.rs                          # Entry point, CLI parsing, orchestration
├── mod.rs                           # Module declarations
├── types.rs                         # Common data structures (shared across modules)
├── constants.rs                     # Configuration constants (STAGE2 arrays, etc.)
├── silence_detection.rs             # Silence detection, WindowDbProfile
├── musicbrainz/
│   ├── mod.rs                       # MusicBrainz module root
│   ├── api.rs                       # API client, rate limiting
│   ├── cache.rs                     # Cache implementation (read/write/metadata)
│   └── types.rs                     # MB-specific types (MBSearchResponse, etc.)
├── stages/
│   ├── mod.rs                       # Stage module root
│   ├── stage2.rs                    # Parameter optimization (180-param sweep)
│   ├── stage3.rs                    # Dynamic programming assembly
│   ├── stage4.rs                    # Edition-guided quiet spot detection (RMS)
│   └── stage5.rs                    # Extra track merging
├── matching/
│   ├── mod.rs                       # Matching module root
│   ├── candidate.rs                 # Candidate testing logic
│   ├── edition.rs                   # Edition processing, winner selection
│   └── validation.rs                # Single-track detection, name validation
├── utils/
│   ├── mod.rs                       # Utilities module root
│   ├── audio.rs                     # Audio decoding (symphonia integration)
│   ├── fingerprint.rs               # AcoustID fingerprinting
│   └── timing.rs                    # Timing, heartbeat, stagger logic
└── README.md                        # Module structure documentation
```

### 3.2 Module Responsibilities

**main.rs:**
- CLI argument parsing (--training-set, --output, --limit, --cache-mode, etc.)
- High-level orchestration (album loop, progress reporting)
- Logging setup (tracing subscriber)
- Error handling for top-level execution

**types.rs:**
- CandidateTestResult
- OverSegmentedCandidate
- SingleEditionStage2Results
- Shared enums and structs used across modules

**constants.rs:**
- STAGE2_THRESHOLD_VALUES (12 values)
- STAGE2_MIN_DURATION_VALUES (15 values)
- DEFAULT_THRESHOLD_DB, DEFAULT_MIN_DURATION_SECS
- Tolerance, grace periods, penalties

**silence_detection.rs:**
- WindowDbProfile struct and implementation
- find_silence_regions_from_profile()
- detect_silence() (original implementation)
- Silence cache type (Vec<Vec<u32>>)

**musicbrainz/ module:**
- API client with rate limiting (Semaphore, sleep between requests)
- Search and release detail fetching
- Cache implementation (3 modes: Disabled, ReadWrite, ReadOnly)
- File-based cache (JSON serialization)

**stages/ module:**
- stage2.rs: run_stage2_single_edition_cached() - 180-parameter sweep with early-exit
- stage3.rs: run_stage3_single_edition() - DP assembly for over-segmented candidates
- stage4.rs: run_stage4_single_edition() - RMS profiling, quiet spot detection
- stage5.rs: run_stage5_single_edition() - Extra track merging

**matching/ module:**
- candidate.rs: test_segmentation_against_single_edition() - match quality calculation
- edition.rs: Edition processing loop, winner selection with track count penalty
- validation.rs: Single-track discriminator, artist/album name validation

**utils/ module:**
- audio.rs: Symphonia-based MP3 decoding, sample extraction
- fingerprint.rs: Chromaprint integration for AcoustID
- timing.rs: Instant-based timing, heartbeat thread, album stagger logic

### 3.3 Key Refactoring Principles

**Preserve Algorithm Integrity:**
- No changes to algorithm logic
- Preserve exact numerical constants
- Maintain same execution order

**Minimize Code Duplication:**
- Extract common patterns into shared functions
- Use consistent error handling patterns
- Centralize type definitions

**Clear Dependencies:**
- Modules depend on types.rs and constants.rs (shared state)
- Higher-level modules (main, edition) orchestrate lower-level modules (stages, silence)
- No circular dependencies

**Testability:**
- Public module functions take explicit parameters (no hidden global state)
- Return values enable verification
- Side effects (logging, I/O) isolated where possible

---

## 4. Migration Strategy

### 4.1 Incremental Approach

The refactoring SHALL be performed incrementally to minimize risk:

**Phase 1: Structure Setup**
1. Create am28/ folder
2. Create stub files for all modules
3. Move main() to main.rs
4. Verify compilation (empty modules)

**Phase 2: Types and Constants**
1. Extract all struct/enum definitions to types.rs
2. Extract all const definitions to constants.rs
3. Update references in main.rs
4. Verify compilation

**Phase 3: Core Modules (Bottom-Up)**
1. Extract silence_detection.rs (no dependencies on other modules)
2. Extract utils/ modules (audio, fingerprint, timing)
3. Extract musicbrainz/ modules (depends only on types)
4. Verify each module compiles independently

**Phase 4: Stage Modules**
1. Extract stage2.rs (depends on silence_detection, types)
2. Extract stage3.rs (depends on types, candidate matching)
3. Extract stage4.rs (depends on utils/audio)
4. Extract stage5.rs (depends on types)

**Phase 5: Matching and Orchestration**
1. Extract matching/ modules (candidate, edition, validation)
2. Update main.rs to call modular functions
3. Remove old monolithic code

**Phase 6: Verification**
1. Run on full Run 27 dataset (200 albums)
2. Compare outputs line-by-line
3. Verify performance within 10% of baseline
4. Document any differences found

### 4.2 Verification at Each Phase

After each phase:
- [ ] Code compiles without errors or warnings
- [ ] Run subset test (10 albums) - results match baseline
- [ ] Document what was extracted and what remains

### 4.3 Rollback Plan

If refactoring introduces regressions:
1. Identify which phase introduced the issue
2. Revert to previous phase (git checkpoint)
3. Fix issue in isolation
4. Re-verify before proceeding

---

## 5. Success Criteria

### 5.1 Functional Success

- [ ] All 193 albums from Run 28 match with identical parameters
- [ ] Same 5 albums fail with same error messages
- [ ] Performance within ±10% of Run 28 baseline (200.5s average)

### 5.2 Structural Success

- [ ] All module files <1000 lines
- [ ] Clear module boundaries (no circular dependencies)
- [ ] README.md documents module organization
- [ ] Cargo example compiles without warnings

### 5.3 Quality Success

- [ ] Module-level documentation for all modules
- [ ] Public function documentation (doc comments)
- [ ] At least 3 modules have demonstrable unit tests
- [ ] Code maintainability improved (subjective but reviewable)

---

## 6. Constraints

### 6.1 Technical Constraints

- MUST remain Rust stable channel compatible
- MUST compile as Cargo example (not library crate)
- MUST preserve all dependencies (no new crates added)
- MUST preserve existing CLI interface

### 6.2 Timeline Constraints

- Target: Complete refactoring in 8-12 hours of development time
- Verification: 2-3 hours for full dataset testing

### 6.3 Compatibility Constraints

- MUST work on Windows (user's primary platform)
- MUST preserve MusicBrainz cache compatibility (existing cache/ folder)
- MUST preserve training_set.txt compatibility

---

## 7. Risks

### 7.1 Algorithm Behavior Change Risk

**Risk:** Subtle changes in module extraction cause different results

**Mitigation:**
- Extract pure functions first (no side effects)
- Verify outputs at each phase (10-album subset)
- Full verification at end (200-album dataset)

### 7.2 Performance Regression Risk

**Risk:** Additional function call overhead slows execution

**Mitigation:**
- Measure compilation in release mode (inlining enabled)
- Profile if >10% regression observed
- Optimize hot paths if needed

### 7.3 Compilation Complexity Risk

**Risk:** Modular structure increases compilation time significantly

**Mitigation:**
- Measure with `cargo build --timings`
- Evaluate if >10% slower compilation
- Simplify module structure if needed

---

## 8. Open Questions

1. **Module Granularity:** Is proposed structure too fine-grained or too coarse?
   - Consider: Each stage could be 1-2 files vs. current proposal of 4 separate stage files

2. **Testing Strategy:** Should unit tests be included in Phase 1 or added later?
   - Proposal: Demonstrate capability but defer comprehensive testing to separate effort

3. **Documentation Depth:** How detailed should module README.md be?
   - Proposal: High-level organization map + per-module purpose (not full API docs)

---

## 9. References

- **Source:** wkmp-ai/examples/album_matcher_28.rs (7,400 lines)
- **Baseline Results:** album_matcher_output_run28.txt (when available)
- **Phase 1 Validation Report:** wip/PLAN027_album_matcher_simplicity_first/PHASE1_VALIDATION_REPORT.md
- **Implementation Summary:** wip/PLAN027_album_matcher_simplicity_first/IMPLEMENTATION_SUMMARY_RUN28.md

---

## Document Control

**Version:** 1.0 (Draft)
**Status:** Ready for Planning
**Author:** WKMP Development Team
**Date:** 2025-11-25

**Next Steps:**
1. Execute /plan workflow (Phases 1-3)
2. Generate implementation plan with test specifications
3. Begin refactoring following plan increments
