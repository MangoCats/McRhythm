# Scope Statement: Album Matcher 28 Refactoring

**Plan:** PLAN024
**Specification:** SPEC_album_matcher_28_refactoring.md
**Date:** 2025-11-25
**Status:** Phase 1 - Scope Definition

---

## 1. In Scope

### 1.1 Code Refactoring

**Structural Changes:**
- Extract monolithic album_matcher_28.rs into modular folder structure (am28/)
- Create 13+ module files organized by functional domain
- Define shared types and constants in central modules

**Module Creation:**
- main.rs: Entry point, CLI, orchestration
- types.rs: Shared data structures
- constants.rs: Configuration constants
- silence_detection.rs: Silence detection logic
- musicbrainz/ folder: API client, cache, types (3 files)
- stages/ folder: Stage 2-5 implementations (4 files)
- matching/ folder: Candidate testing, edition selection (3 files)
- utils/ folder: Audio, fingerprint, timing utilities (3 files)
- README.md: Module documentation

**Functional Preservation:**
- Preserve 100% of Run 28 algorithm logic
- Maintain identical parameter arrays (STAGE2_THRESHOLD_VALUES, STAGE2_MIN_DURATION_VALUES)
- Preserve early-exit optimization behavior
- Maintain MusicBrainz API caching (3 modes)
- Preserve all 5 matching stages

### 1.2 Compilation and Build

**Build System:**
- Maintain Cargo example compilation: `cargo run --example album_matcher_28`
- Support same CLI arguments (--training-set, --output, --limit, --cache-mode, etc.)
- Preserve dependencies (no new crates)

**Build Verification:**
- Compile without warnings (cargo check)
- Compile in release mode (cargo build --release)
- Verify compilation time within 110% of baseline

### 1.3 Testing and Verification

**Functional Testing:**
- Run on full Run 27 dataset (200 albums)
- Verify identical results (193 success, 5 failed)
- Compare "Best parameters" for all successful albums
- Verify output format matches Run 28

**Module Testing:**
- Demonstrate unit testability for ≥3 key modules
- Provide example test patterns (proof of concept)

### 1.4 Documentation

**Code Documentation:**
- Module-level doc comments (`//!`) for all modules
- Function-level doc comments (`///`) for public functions
- Preserve complex algorithm comments (DP assembly, RMS profiling)

**Module Documentation:**
- README.md in am28/ folder
- Module hierarchy and responsibility map
- Developer navigation guide

---

## 2. Out of Scope

### 2.1 Algorithm Changes

**Explicitly Excluded:**
- NO changes to matching algorithm logic
- NO changes to parameter values or ordering
- NO optimization beyond existing Run 28 implementation
- NO new features or capabilities

**Rationale:** This is a pure refactoring effort to improve maintainability, not an algorithm enhancement.

### 2.2 Performance Optimization

**Explicitly Excluded:**
- NO performance tuning beyond baseline
- NO profiling or bottleneck identification
- NO algorithm improvements for speed

**Acceptable:** Performance within ±10% of Run 28 baseline (200.5s average per album)

**Rationale:** Refactoring may introduce minor overhead; optimization is separate effort.

### 2.3 Library Creation

**Explicitly Excluded:**
- NO conversion to library crate
- NO public API design for external use
- NO version management or semver compliance

**Rationale:** Remains standalone example, not reusable library.

### 2.4 Comprehensive Testing

**Explicitly Excluded:**
- NO full unit test suite (only proof-of-concept tests)
- NO integration test framework
- NO continuous integration setup

**Acceptable:** Demonstrate testability, defer comprehensive testing to future effort.

**Rationale:** Testing infrastructure is valuable but not required for refactoring success.

### 2.5 UI or Visualization

**Explicitly Excluded:**
- NO new output formats
- NO visualization tools
- NO progress bars or interactive features

**Acceptable:** Preserve existing logging and output format.

**Rationale:** Refactoring is code structure only, not user experience.

---

## 3. Assumptions

### 3.1 Code Assumptions

**A1:** album_matcher_28.rs is the authoritative source for Run 28 behavior.
- Baseline: Run 28 results (when available) or Run 27 results as proxy
- Verification: Compare outputs line-by-line

**A2:** No undocumented external dependencies exist.
- All dependencies listed in Cargo.toml
- No environment-specific behavior

**A3:** Run 28 has been validated and approved.
- Results are correct and reproducible
- No known bugs in Run 28 implementation

### 3.2 Tooling Assumptions

**A4:** Rust stable channel is sufficient.
- No nightly features required
- All dependencies compatible with stable

**A5:** Windows platform is primary target.
- Refactored code works on Windows (user's platform)
- Cross-platform compatibility expected but not verified

### 3.3 Data Assumptions

**A6:** Run 27 dataset (training_set.txt) is stable.
- 200 albums listed
- Album files exist and accessible
- No changes to audio files during refactoring

**A7:** MusicBrainz cache is backward compatible.
- Existing cache/ folder readable
- No cache format changes

---

## 4. Constraints

### 4.1 Technical Constraints

**C1:** MUST compile as Cargo example, NOT library crate.
- Folder structure: wkmp-ai/examples/am28/
- Entry point: examples/album_matcher_28.rs (wrapper pointing to am28/main.rs)

**C2:** MUST preserve exact CLI interface.
- Same arguments: --training-set, --output, --limit, --cache-mode, --stagger, --mb-search-throttle
- Same default values
- Same help text

**C3:** MUST use existing dependencies.
- No new crates added to Cargo.toml
- Version pins unchanged

**C4:** MUST work on Rust stable channel.
- No nightly features
- Stable 1.70+ assumed (current stable)

### 4.2 Quality Constraints

**C5:** MUST compile without warnings.
- Zero warnings in `cargo check`
- Intentional `#[allow(...)]` preserved

**C6:** MUST preserve algorithm correctness.
- 100% identical results on Run 27 dataset
- No regressions in match quality

**C7:** Module files MUST be <1000 lines.
- Split large modules further if needed
- Balance granularity vs. overhead

### 4.3 Process Constraints

**C8:** Incremental migration required.
- 6-phase approach (per specification)
- Verify at each phase (10-album subset test)
- Git checkpoints for rollback

**C9:** Full verification before completion.
- Run on complete Run 27 dataset (200 albums)
- Document any differences found
- Investigate and resolve regressions

---

## 5. Success Criteria

### 5.1 Functional Success (CRITICAL)

**All must pass:**
- ✅ 193 albums succeed with identical "Best parameters"
- ✅ Same 5 albums fail with same error messages
- ✅ Output format matches Run 28
- ✅ Performance within ±10% of baseline (200.5s average)

### 5.2 Structural Success (HIGH)

**All must pass:**
- ✅ All module files <1000 lines
- ✅ Clear module boundaries (no circular dependencies)
- ✅ Folder structure matches proposal
- ✅ Compiles as standalone example

### 5.3 Quality Success (MEDIUM)

**Most should pass:**
- ✅ Module-level documentation for all modules
- ✅ Public function documentation
- ✅ README.md documents organization
- ⚠️ At least 3 modules have demonstrable unit tests (proof of concept)

### 5.4 Build Success (HIGH)

**All must pass:**
- ✅ Zero warnings in `cargo check`
- ✅ Release mode compilation succeeds
- ✅ Compilation time ≤110% of baseline

---

## 6. Risks and Mitigations

### 6.1 Algorithm Behavior Change Risk

**Risk:** Subtle changes in module extraction cause different results.

**Impact:** HIGH - Defeats purpose of refactoring if correctness compromised.

**Likelihood:** MEDIUM - Complex code with many edge cases.

**Mitigation:**
- Extract pure functions first (no side effects)
- Verify outputs at each phase (10-album subset)
- Full verification at end (200-album dataset)
- Git checkpoints for rollback

**Residual Risk:** LOW - Incremental verification catches issues early.

### 6.2 Performance Regression Risk

**Risk:** Additional function call overhead slows execution.

**Impact:** MEDIUM - Acceptable if within ±10%, problematic if >20% slower.

**Likelihood:** LOW - Release mode optimizations (inlining) mitigate.

**Mitigation:**
- Measure in release mode only
- Profile if >10% regression observed
- Optimize hot paths if needed (inline hints)

**Residual Risk:** LOW - Modern compilers inline effectively.

### 6.3 Compilation Complexity Risk

**Risk:** Modular structure increases compilation time significantly.

**Impact:** LOW - Developer inconvenience, not user-facing.

**Likelihood:** LOW - Modular code often compiles faster (better parallelization).

**Mitigation:**
- Measure with `cargo build --timings`
- Evaluate if >10% slower
- Simplify module structure if needed

**Residual Risk:** LOW - Acceptable tradeoff for maintainability.

### 6.4 Developer Understanding Risk

**Risk:** Refactored code is harder to understand than monolithic file.

**Impact:** MEDIUM - Defeats maintainability goal if overly complex.

**Likelihood:** LOW - Clear module boundaries improve understanding.

**Mitigation:**
- README.md documents module organization
- Module-level documentation explains purpose
- Logical domain separation (not arbitrary splitting)

**Residual Risk:** LOW - Documentation and naming clarity mitigate.

---

## 7. Deliverables

### 7.1 Code Deliverables

**Primary:**
- wkmp-ai/examples/am28/ folder with modular implementation
- wkmp-ai/examples/album_matcher_28.rs (wrapper entry point)

**Module Files:**
- main.rs, mod.rs, types.rs, constants.rs, silence_detection.rs
- musicbrainz/: mod.rs, api.rs, cache.rs, types.rs
- stages/: mod.rs, stage2.rs, stage3.rs, stage4.rs, stage5.rs
- matching/: mod.rs, candidate.rs, edition.rs, validation.rs
- utils/: mod.rs, audio.rs, fingerprint.rs, timing.rs

### 7.2 Documentation Deliverables

**Required:**
- am28/README.md: Module structure and organization
- Module-level doc comments (`//!`) in all .rs files
- Function-level doc comments (`///`) for public functions

### 7.3 Verification Deliverables

**Required:**
- Verification report: Compare refactored vs Run 28 baseline
- Performance measurement: Timing comparison
- Unit test examples: Demonstrate testability for ≥3 modules

### 7.4 Plan Deliverables (This Plan)

**Required:**
- requirements_index.md (this document's companion)
- scope_statement.md (this document)
- dependencies_map.md (next to create)
- Implementation plan (Phase 2)
- Test specifications (Phase 3)

---

## 8. Timeline and Milestones

**Note:** Per CLAUDE.md, timelines are NOT decision factors. This is for planning only.

### Phase 1: Structure Setup (1-2 hours)
- Create am28/ folder structure
- Create stub module files
- Verify compilation (empty modules)

### Phase 2: Types and Constants (1-2 hours)
- Extract types.rs
- Extract constants.rs
- Update main.rs references
- Verify compilation

### Phase 3: Core Modules (2-3 hours)
- Extract silence_detection.rs
- Extract utils/ modules
- Extract musicbrainz/ modules
- Verify each module compiles

### Phase 4: Stage Modules (2-3 hours)
- Extract stages/stage2.rs
- Extract stages/stage3.rs
- Extract stages/stage4.rs
- Extract stages/stage5.rs
- Verify compilation and 10-album test

### Phase 5: Matching and Orchestration (2-3 hours)
- Extract matching/ modules
- Update main.rs orchestration
- Remove old code
- Verify compilation

### Phase 6: Verification and Documentation (2-3 hours)
- Run full 200-album dataset
- Compare outputs line-by-line
- Write README.md
- Add module/function documentation
- Final verification

**Total Estimated Effort:** 10-16 hours (developer time)

---

## Document Control

**Version:** 1.0
**Status:** Phase 1 Complete
**Created:** 2025-11-25
**Last Updated:** 2025-11-25

**Dependencies:**
- Upstream: SPEC_album_matcher_28_refactoring.md
- Downstream: Implementation plan (Phase 2), Test specifications (Phase 3)
