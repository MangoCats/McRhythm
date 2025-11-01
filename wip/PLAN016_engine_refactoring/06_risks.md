# Risk Assessment: PLAN016 Engine Refactoring

**Plan:** PLAN016
**Date:** 2025-11-01
**Risk Framework:** Probability × Impact = Risk Level
**Overall Risk Rating:** LOW

---

## Executive Summary

**Total Risks Identified:** 12
- **HIGH:** 0
- **MEDIUM:** 3
- **LOW:** 6
- **VERY LOW:** 3

**Overall Residual Risk (After Mitigation):** **LOW**

**Top 3 Risks:**
1. **RISK-004:** Import/Dependency Errors (Medium) - Mitigated with incremental compilation
2. **RISK-005:** Async Handler Lifetime Issues (Medium) - Mitigated with handler extraction strategy
3. **RISK-007:** Unexpected Test Failures (Medium) - Mitigated with baseline and comprehensive tests

**Risk Acceptance:** All medium risks have effective mitigations. Residual risk acceptable for implementation.

---

## Risk Register

### RISK-001: Module Exceeds Line Limit

**Category:** Technical / Requirements Compliance
**Probability:** Low (15%)
**Impact:** Medium (requires rework)
**Risk Level:** **LOW**

**Description:**
One or more modules (core.rs, queue.rs, diagnostics.rs) exceeds 1,500 line limit after code migration.

**Scenario:**
- Most likely: core.rs approaches 1,450-1,500 lines due to import overhead
- Current estimate: core.rs ~1,380 lines (120 line headroom)
- Boilerplate overhead: ~50-100 lines (imports, comments, spacing)

**Impact if Occurs:**
- Requirement REQ-DEBT-QUALITY-002-020 violated
- Must further decompose oversized module
- Rework: 1-2 hours
- Delayed completion

**Probability Justification:**
- Phase 4 analysis shows natural fit within limits
- 4,251 lines < 6,000 line capacity (4 files × 1,500)
- 29% overhead budget available

**Mitigation Strategy:**

**Preventive:**
1. Monitor line counts during migration (after each increment)
2. Track headroom: `wc -l core.rs` after each method moved
3. If core.rs > 1,400 lines during Increment 5: Pause, reassess

**Responsive:**
1. If limit exceeded: Split large module into sub-modules
   - Example: core.rs → core.rs + core_orchestration.rs
2. Update mod.rs to include new sub-module
3. Maintain single responsibility within split

**Acceptance Criteria:**
- Allowable per scope statement (further decomposition if needed)
- Document deviation in implementation report

**Residual Risk:** Very Low (effective monitoring, clear action plan)

---

### RISK-002: Compilation Errors After Module Split

**Category:** Technical / Implementation
**Probability:** Medium (40%)
**Impact:** Low (easily fixed)
**Risk Level:** **LOW**

**Description:**
Compilation errors occur after extracting code to modules (missing imports, visibility issues, type mismatches).

**Scenario:**
- Missing `use` statements in queue.rs or diagnostics.rs
- Incorrect visibility modifiers (pub vs. pub(super) vs. pub(crate))
- Type imports missing (PlaybackEngine, events, etc.)

**Impact if Occurs:**
- Delays implementation while fixing imports
- Time to fix: 15-30 minutes per module
- No functional impact (compile-time errors only)

**Probability Justification:**
- Common in Rust refactoring (explicit imports required)
- Multiple modules = multiple import lists to manage
- Higher probability but low severity

**Mitigation Strategy:**

**Preventive:**
1. **Incremental Compilation:** Run `cargo check -p wkmp-ap` after EACH method moved
2. **Import Template:** Create import checklist for each module
   ```rust
   // Common imports for all modules:
   use crate::playback::{PlaybackEngine, events::*};
   use tokio::sync::{RwLock, broadcast};
   use std::sync::Arc;
   // ... module-specific imports
   ```
3. **Start with Clean Module:** queue.rs first (lowest coupling, clearest imports)

**Responsive:**
1. Read compiler error messages carefully
2. Add missing `use` statements as indicated
3. If stuck: Use IDE "auto-import" or `cargo fix` suggestions
4. Verify compilation before proceeding to next method

**Residual Risk:** Very Low (standard Rust workflow, easily fixable)

---

### RISK-003: API Breakage (Public Interface Changed)

**Category:** Technical / Requirements Compliance
**Probability:** Very Low (<5%)
**Impact:** High (handlers/tests break, REQ-030 violated)
**Risk Level:** **LOW**

**Description:**
Public API inadvertently changes during refactoring (methods become private, signatures change, types unavailable).

**Scenario:**
- Forget to re-export public method in mod.rs
- Change visibility from `pub` to `pub(super)`
- Restructure breaks external callers (handlers.rs, tests)

**Impact if Occurs:**
- REQ-DEBT-QUALITY-002-030 violated
- Handlers fail to compile (TC-I-030-01 fails)
- Tests fail to compile or run (TC-I-030-02 fails)
- Rework required: 30-60 minutes

**Probability Justification:**
- Very low due to comprehensive API tests
- TC-I-030-01 (handler compilation) catches breaks immediately
- TC-I-030-02 (test suite) provides comprehensive coverage
- Rust compiler enforces visibility rules (compile-time errors, not runtime)

**Mitigation Strategy:**

**Preventive:**
1. **Document Public API** (Increment 1): List all `pub fn` before refactoring
2. **Preserve Signatures:** Do not modify function signatures, only move code
3. **Use Re-Exports:** mod.rs re-exports ensure API visibility
   ```rust
   // mod.rs
   pub use core::PlaybackEngine;
   // Public methods auto-available via impl blocks
   ```

**Responsive:**
1. If TC-I-030-01 fails: Review mod.rs re-exports
2. If methods missing: Add explicit re-export or change visibility
3. Run `cargo check -p wkmp-ap --lib` after each increment

**Detection:**
- TC-I-030-01: Handler compilation (immediate detection)
- TC-I-030-02: Test suite pass rate (comprehensive detection)
- Automated in Increment 6 verification

**Residual Risk:** Very Low (strong preventive measures + comprehensive tests)

---

### RISK-004: Import/Dependency Errors

**Category:** Technical / Implementation
**Probability:** Medium (50%)
**Impact:** Low-Medium (time to debug, delays implementation)
**Risk Level:** **MEDIUM**

**Description:**
Complex import dependencies cause compilation errors that are difficult to resolve.

**Scenario:**
- Circular dependencies between modules (core imports queue, queue imports core)
- Type re-export issues (PlaybackEngine not visible where needed)
- Trait import missing (method not available on type)

**Impact if Occurs:**
- Debugging time: 30-90 minutes
- May require restructuring imports or visibility
- Delays implementation but solvable

**Probability Justification:**
- Engine has many Arc<RwLock<T>> fields with complex types
- Multiple modules importing each other
- Higher probability due to complexity

**Mitigation Strategy:**

**Preventive:**
1. **Import Hierarchy:**
   ```
   mod.rs (root)
     ├─→ core.rs (defines PlaybackEngine)
     ├─→ queue.rs (imports PlaybackEngine via super::core)
     └─→ diagnostics.rs (imports PlaybackEngine via super::core)
   ```
2. **Avoid Circular Imports:** Core exports types, queue/diagnostics import from core
3. **Use `super::`:** Relative imports within engine/ directory

**Responsive:**
1. If circular dependency: Extract shared types to separate module
   - Example: Create `types.rs` for shared structs/enums
2. Use `pub(super)` for engine-internal items (not fully public)
3. Read compiler messages carefully (often suggests fix)

**Detection:**
- Compile-time errors (immediate feedback)
- `cargo check -p wkmp-ap` after each increment

**Residual Risk:** Low-Medium (mitigated by clear import strategy, but some uncertainty remains)

---

### RISK-005: Async Handler Lifetime Issues

**Category:** Technical / Rust Complexity
**Probability:** Medium (40%)
**Impact:** Medium (requires debugging, possible restructuring)
**Risk Level:** **MEDIUM**

**Description:**
Moving event handler functions to diagnostics.rs causes async lifetime errors when spawning tasks.

**Scenario:**
- Handler functions moved to diagnostics.rs
- core.rs start() method spawns handlers with `tokio::spawn(diagnostics::position_event_handler(...))`
- Lifetime errors: "borrowed value does not live long enough"

**Impact if Occurs:**
- Must clone Arc handles before spawning (already done in engine.rs)
- May need `'static` lifetime bounds
- Debugging time: 30-90 minutes
- Possible restructuring of handler signatures

**Probability Justification:**
- Async lifetimes are complex in Rust
- Moving handlers across modules may expose lifetime issues
- Current code works, but module boundary may change lifetime analysis

**Mitigation Strategy:**

**Preventive:**
1. **Keep Handler Spawning Pattern:** Do not change how handlers are called
   ```rust
   // Current (in engine.rs):
   let handles = self.clone_handles();
   tokio::spawn(async move {
       position_event_handler(handles, ...).await
   });

   // After (in core.rs):
   let handles = self.clone_handles();
   tokio::spawn(async move {
       super::diagnostics::position_event_handler(handles, ...).await
   });
   ```
2. **Make Handlers `pub`:** Public functions in diagnostics.rs
3. **Test Incrementally:** Compile after moving EACH handler

**Responsive:**
1. If lifetime errors: Ensure all Arc handles cloned before spawn
2. Add `'static` bounds to handler parameters if needed
3. If stuck: Keep handlers in core.rs (acceptable deviation)

**Fallback:**
- Handlers can stay in core.rs if extraction proves too complex
- Document in implementation report as technical constraint

**Residual Risk:** Low-Medium (clear mitigation path, fallback available)

---

### RISK-006: Behavior Changes (Logic Errors)

**Category:** Functional / Correctness
**Probability:** Very Low (<5%)
**Impact:** High (playback breaks, silent bugs)
**Risk Level:** **LOW**

**Description:**
Code movement introduces subtle logic errors that change playback behavior.

**Scenario:**
- Copy/paste error (wrong method moved)
- Incomplete method extraction (missing code)
- Accidental code modification during movement

**Impact if Occurs:**
- Playback logic broken
- Tests may catch (TC-I-030-02)
- Runtime failures possible (worse than compile errors)
- Debugging time: 1-4 hours

**Probability Justification:**
- Very low because refactoring is pure code movement (no rewrites)
- Test suite provides comprehensive behavioral verification
- process_queue() stays intact (no splitting 580-line function)

**Mitigation Strategy:**

**Preventive:**
1. **No Code Rewrites:** Move code AS-IS (copy exact text)
2. **Comment Out Original:** Do not delete from engine.rs until verified
3. **Keep Complex Functions Intact:** Do not split process_queue() or large handlers

**Responsive:**
1. If TC-I-030-02 fails: Compare with baseline
2. Use `git diff` to verify only movement (no logic changes)
3. Rollback to previous increment if behavior change detected

**Detection:**
- TC-I-030-02: Test suite pass rate (100% baseline must match)
- Comprehensive integration tests exercise playback logic

**Residual Risk:** Very Low (strong preventive measures + comprehensive tests)

---

### RISK-007: Unexpected Test Failures

**Category:** Quality / Verification
**Probability:** Medium (30%)
**Impact:** Medium (requires investigation, possible rework)
**Risk Level:** **MEDIUM**

**Description:**
Tests fail in Increment 6 verification despite passing in earlier increments.

**Scenario:**
- Integration tests fail (not caught by per-increment testing)
- Timing-sensitive tests flake
- Environment-specific failures

**Impact if Occurs:**
- Must investigate root cause
- May require code adjustments
- Delays final verification: 1-3 hours
- Possible rollback if unfixable

**Probability Justification:**
- Some tests may not run during per-increment checks
- Integration tests may exercise cross-module behavior
- Flaky tests possible (timing, environment)

**Mitigation Strategy:**

**Preventive:**
1. **Baseline Establishment** (Increment 1): Record exact test results before refactoring
2. **Run Full Suite Early:** Run `cargo test -p wkmp-ap` after Increment 3 and 4 (not just quick checks)
3. **Compare Results:** Use `diff` to compare test output with baseline

**Responsive:**
1. If test fails: Isolate which test (`cargo test -p wkmp-ap <test_name>`)
2. Check if test was passing in baseline (may be pre-existing failure)
3. Debug with `RUST_BACKTRACE=1 cargo test`
4. If refactoring-related: Fix code in affected module
5. If pre-existing: Document as known issue (out of scope)

**Detection:**
- TC-I-030-02: Explicit test suite comparison

**Residual Risk:** Low (baseline comparison, clear investigation process)

---

### RISK-008: Code Duplication Introduced

**Category:** Quality / Maintainability
**Probability:** Low (20%)
**Impact:** Low (technical debt, not functional)
**Risk Level:** **LOW**

**Description:**
Shared code duplicated across modules instead of factored into helpers.

**Scenario:**
- Helper function needed in both core.rs and queue.rs
- Copy function to both modules (DRY violation)

**Impact if Occurs:**
- Maintenance burden (changes required in multiple places)
- Does not violate requirements (functional correctness unchanged)
- Technical debt

**Probability Justification:**
- Low because Phase 4 analysis identified shared code
- Most helpers localized to one functional area

**Mitigation Strategy:**

**Preventive:**
1. **Identify Shared Helpers** (Phase 4 analysis):
   - clone_handles() - core.rs (used by core only)
   - emit_queue_change_events() - queue.rs (queue-specific)
2. **Create Helpers Module If Needed:**
   ```rust
   // engine/helpers.rs (optional)
   pub(super) fn shared_helper() { }
   ```

**Responsive:**
1. During manual code review (Increment 6 TC-S-010-02): Check for duplication
2. If found: Extract to helpers module or keep in primary location

**Detection:**
- TC-S-010-02: Manual code organization review

**Residual Risk:** Very Low (preventive identification, low impact)

---

### RISK-009: Performance Degradation

**Category:** Non-Functional / Performance
**Probability:** Very Low (<5%)
**Impact:** Low (slight overhead from module indirection)
**Risk Level:** **VERY LOW**

**Description:**
Module split introduces performance overhead (function call indirection, cache misses).

**Scenario:**
- Cross-module function calls add overhead
- Compiler fails to inline across module boundaries

**Impact if Occurs:**
- Slight performance degradation (<1% expected)
- Not noticeable in practice (network/IO dominates)
- Does not violate requirements (no performance requirements specified)

**Probability Justification:**
- Rust compiler optimizes aggressively (LTO, inlining)
- No change to algorithms, just code organization
- Indirection overhead negligible compared to I/O

**Mitigation Strategy:**

**Preventive:**
1. Trust Rust compiler optimizations
2. Use LTO (Link-Time Optimization) in release builds (already enabled)

**Responsive:**
1. If performance issues observed: Profile with `cargo flamegraph`
2. Add `#[inline]` hints to hot-path functions if needed
3. Not expected to be necessary

**Detection:**
- Would require profiling (out of scope for this refactoring)

**Residual Risk:** Very Low (negligible impact, no evidence of concern)

---

### RISK-010: Incomplete Documentation

**Category:** Process / Knowledge Transfer
**Probability:** Low (20%)
**Impact:** Low (maintenance friction, not functional)
**Risk Level:** **LOW**

**Description:**
Module responsibilities not clearly documented, making future maintenance difficult.

**Scenario:**
- Developers don't know which module contains specific functionality
- No comments explaining module boundaries

**Impact if Occurs:**
- Slower development (search for code)
- Does not affect functionality
- Addressable post-refactoring

**Probability Justification:**
- Phase 4 analysis documents responsibilities
- Increment 5 includes file-level comments
- Low impact (solvable with documentation)

**Mitigation Strategy:**

**Preventive:**
1. **Module-Level Documentation** (Increment 5):
   ```rust
   //! Core playback engine - State management & lifecycle
   //!
   //! **[PLAN016]** Contains:
   //! - PlaybackEngine struct definition
   //! - Lifecycle methods (new, start, stop, play, pause)
   //! ...
   ```
2. **Update mod.rs with Guide:**
   ```rust
   //! # Module Organization
   //!
   //! - `core`: Lifecycle, orchestration, state management
   //! - `queue`: Queue operations (enqueue, skip, etc.)
   //! - `diagnostics`: Status queries, event handlers
   ```

**Responsive:**
1. If unclear during code review: Add clarifying comments

**Detection:**
- TC-S-010-02: Manual code organization review

**Residual Risk:** Very Low (clear documentation plan)

---

### RISK-011: Merge Conflicts (Concurrent Changes)

**Category:** Process / Coordination
**Probability:** Low (10%)
**Impact:** Medium (requires conflict resolution)
**Risk Level:** **LOW**

**Description:**
Another developer modifies engine.rs concurrently, causing merge conflicts.

**Scenario:**
- Refactoring on branch A
- Bug fix committed to main touching engine.rs
- Merge conflicts when integrating

**Impact if Occurs:**
- Manual conflict resolution: 30-60 minutes
- May require re-running tests
- Risk of merge errors

**Probability Justification:**
- Low probability if work coordinated
- Current branch: feature/plan016-engine-refactoring (isolated)
- Short duration (2-3 days) reduces window

**Mitigation Strategy:**

**Preventive:**
1. **Check Active Branches:** Before starting, check for other work on engine.rs
2. **Communicate:** Announce refactoring, request freeze on engine.rs changes
3. **Short Timeline:** Complete refactoring in 2-3 days (minimize conflict window)

**Responsive:**
1. If conflicts occur: Use `git mergetool` or manual resolution
2. After resolving: Re-run full test suite (TC-I-030-02)
3. If complex conflicts: Rebase refactoring on top of main

**Detection:**
- Git will report conflicts during merge

**Residual Risk:** Very Low (short timeline, communication)

---

### RISK-012: Rollback Required

**Category:** Process / Contingency
**Probability:** Low (15%)
**Impact:** Medium (lost implementation time)
**Risk Level:** **LOW**

**Description:**
Critical issue discovered in Increment 6 requiring full rollback to original engine.rs.

**Scenario:**
- Tests fail with no clear fix
- Behavior change discovered too late
- API breakage cannot be resolved

**Impact if Occurs:**
- Rollback to original engine.rs
- Lost implementation time: 6-10 hours
- Must restart refactoring with revised approach

**Probability Justification:**
- Low because incremental verification catches issues early
- engine.rs preserved until Increment 6 (fallback available)
- Comprehensive tests prevent undetected issues

**Mitigation Strategy:**

**Preventive:**
1. **Incremental Verification:** Test after each increment (catch issues early)
2. **Checkpoints:** Major checkpoints after Increments 3, 4, 5 (decision points)
3. **Keep Original File:** Do not delete engine.rs until Increment 6 passes all tests

**Responsive:**
1. If rollback needed: `git reset --hard <previous_commit>`
2. Analyze root cause before retrying
3. Revise approach based on lessons learned

**Fallback:**
- Original engine.rs available throughout Increments 1-5
- Git history preserves all intermediate states

**Residual Risk:** Very Low (strong incremental verification, fallback available)

---

## Risk Matrix

| Risk ID | Description | Probability | Impact | Risk Level | Residual |
|---------|-------------|-------------|--------|------------|----------|
| RISK-001 | Module exceeds line limit | Low | Medium | Low | Very Low |
| RISK-002 | Compilation errors | Medium | Low | Low | Very Low |
| RISK-003 | API breakage | Very Low | High | Low | Very Low |
| RISK-004 | Import/dependency errors | Medium | Low-Med | **Medium** | Low-Med |
| RISK-005 | Async handler lifetimes | Medium | Medium | **Medium** | Low-Med |
| RISK-006 | Behavior changes | Very Low | High | Low | Very Low |
| RISK-007 | Test failures | Medium | Medium | **Medium** | Low |
| RISK-008 | Code duplication | Low | Low | Low | Very Low |
| RISK-009 | Performance degradation | Very Low | Low | Very Low | Very Low |
| RISK-010 | Incomplete documentation | Low | Low | Low | Very Low |
| RISK-011 | Merge conflicts | Low | Medium | Low | Very Low |
| RISK-012 | Rollback required | Low | Medium | Low | Very Low |

---

## Overall Risk Assessment

**Risk Distribution:**
- High: 0
- Medium: 3 (RISK-004, RISK-005, RISK-007)
- Low: 6
- Very Low: 3

**Overall Risk Rating:** **LOW**

**Rationale:**
- All high-impact risks have very low probability (RISK-003, RISK-006)
- Medium risks have effective mitigations (RISK-004, RISK-005, RISK-007)
- No showstoppers identified
- Comprehensive test coverage reduces behavior risk
- Incremental approach allows early detection and rollback

**Risk Acceptance:**

All identified risks are acceptable for implementation:
- Medium risks mitigated to Low-Medium residual
- Clear mitigation strategies defined
- Fallback options available (rollback, keep handlers in core)
- Test coverage provides strong safety net

**Recommendation:** **PROCEED WITH IMPLEMENTATION**

---

## Monitoring and Control

**Risk Monitoring During Implementation:**

1. **After Each Increment:** Review checklist
   - Did it take longer than estimated? (schedule risk)
   - Any compilation errors? (RISK-002, RISK-004)
   - Tests still passing? (RISK-006, RISK-007)

2. **At Checkpoints (Increments 3, 4, 5):**
   - Review risk register
   - Update probability if new information
   - Decide: Proceed / Pause / Rollback

3. **Metrics to Track:**
   - Line counts (RISK-001)
   - Test pass rate (RISK-006, RISK-007)
   - Compilation time (indicator of complexity)

**Escalation Criteria:**

**PAUSE if:**
- Any increment takes 3x estimated time (indicates underestimated complexity)
- More than 2 medium risks materialize
- Test failures cannot be explained

**ROLLBACK if:**
- Tests fail in Increment 6 with no clear fix
- API breakage cannot be resolved
- Behavior change detected and unfixable

**PROCEED if:**
- All tests pass at each checkpoint
- Increments complete within 2x estimated time
- No unexpected risks emerge

---

**Risk Assessment Complete**
**Phase 7 Status:** ✓ 12 risks identified, assessed, and mitigated
**Overall Risk:** LOW (acceptable for implementation)
**Recommendation:** PROCEED
**Next Phase:** Final Documentation (Phase 8)
