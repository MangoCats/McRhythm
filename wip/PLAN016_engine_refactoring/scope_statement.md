# Scope Statement: PLAN016 Engine Refactoring

**Plan:** PLAN016
**Feature:** Engine Refactoring (engine.rs decomposition)
**Source Spec:** SPEC024-wkmp_ap_technical_debt_remediation.md (REQ-DEBT-QUALITY-002)
**Date:** 2025-11-01

---

## In Scope

### ✅ What WILL Be Implemented

**1. Directory Structure Conversion**
- Convert `wkmp-ap/src/playback/engine.rs` (single file) → `wkmp-ap/src/playback/engine/` (directory)
- Create 4 module files:
  - `mod.rs` - Public API re-exports
  - `core.rs` - State management and lifecycle
  - `queue.rs` - Queue operations (enqueue, skip, advance)
  - `diagnostics.rs` - Status queries and telemetry

**2. Code Migration**
- Move all code from engine.rs into appropriate modules
- Organize by functional responsibility:
  - **Core:** `PlaybackEngine` struct, `new()`, lifecycle methods, internal state
  - **Queue:** Queue manipulation, enqueue/skip operations
  - **Diagnostics:** `get_status()`, `get_playback_state()`, telemetry methods
- Preserve ALL functionality (no removals, no additions)

**3. Module Boundaries**
- Define clear responsibilities for each module
- Minimize inter-module dependencies
- Use `pub(super)` for module-private but engine-public items
- Public API only in `mod.rs` re-exports

**4. Line Count Optimization**
- Ensure each module < 1500 lines (REQ-DEBT-QUALITY-002-020)
- Balance code distribution across modules
- If any module exceeds 1499 lines: Further decompose

**5. API Stability**
- Preserve exact public API surface
- No changes to public function signatures
- No changes to public struct fields
- External callers compile without modification

**6. Verification**
- All existing unit tests pass without modification
- All integration tests pass without modification
- API handlers compile without changes
- Line count verification (`wc -l` for each file)

---

## Out of Scope

### ❌ What Will NOT Be Implemented

**1. Other Technical Debt Items**
- DEBT-SEC-001 (Authentication bypass) - Separate plan required
- DEBT-FUNC-001 through DEBT-FUNC-005 - Separate plan required
- DEBT-QUALITY-001 (.unwrap() reduction) - Separate plan required
- DEBT-QUALITY-003 (Compiler warnings) - Separate plan required
- DEBT-QUALITY-004/005 (Duplicate/backup files) - Separate plan required

**2. Functional Changes**
- No new features added
- No bug fixes (unless discovered during refactoring)
- No performance optimizations
- No API enhancements

**3. Mixer Refactoring**
- mixer.rs already refactored in PLAN014 (now 866 lines)
- No changes to mixer module structure

**4. Handlers Refactoring**
- handlers.rs (1,305 lines) mentioned in SPEC024 but not required
- Deferred to future work if needed

**5. Test Modifications**
- No test rewrites
- No test additions (unless API compatibility test)
- Tests should compile and pass as-is

**6. Documentation Updates**
- No architecture documentation changes (SPEC001 unchanged)
- No implementation guide updates (IMPL003 unchanged)
- Only code-level documentation (inline comments) adjusted

---

## Assumptions

**A1. Current Code Compiles**
- Assumption: engine.rs currently compiles without errors
- Validation: Run `cargo check -p wkmp-ap` before refactoring
- If fails: Resolve compilation errors first

**A2. Tests Exist and Pass**
- Assumption: Existing test suite provides adequate coverage
- Validation: Run `cargo test -p wkmp-ap` before refactoring
- Baseline: Record which tests pass/fail before changes

**A3. Module Separation Feasible**
- Assumption: Code can be cleanly separated into 3 functional areas
- Risk: High coupling may require additional refactoring
- Mitigation: Analyze dependencies before splitting

**A4. No Breaking Changes in Dependencies**
- Assumption: No concurrent changes to PlaybackEngine by other work
- Coordination: Check for active branches touching engine.rs
- Timing: Execute refactoring on clean main branch

**A5. Line Count Target Achievable**
- Assumption: 4,251 lines can fit into 4 files <1500 lines each (max 6,000 lines total)
- Validation: 4,251 < 6,000 ✓ (headroom exists)
- Risk: If code grows during refactoring, may need 5th module

---

## Constraints

### Technical Constraints

**TC1. API Stability (Hard Constraint)**
- Cannot change public interface
- Cannot break existing callers
- Enforced by: REQ-DEBT-QUALITY-002-030

**TC2. Line Count Limit (Hard Constraint)**
- Each module MUST be <1500 lines
- Enforced by: REQ-DEBT-QUALITY-002-020
- Verification: Automated line count check in acceptance tests

**TC3. Rust Module System**
- Must use valid Rust module syntax
- Must respect visibility rules (pub, pub(crate), pub(super))
- Must compile with current Rust version (stable channel)

**TC4. No Code Duplication**
- Shared code MUST be in one location (likely core.rs or helpers)
- Avoid copy/paste across modules
- Prefer internal helper functions over duplication

### Process Constraints

**PC1. Test-First Refactoring**
- Run tests BEFORE refactoring (establish baseline)
- Run tests AFTER refactoring (verify no regressions)
- All tests MUST pass before marking complete

**PC2. Incremental Commits**
- Commit after each major module migration
- Keep commits focused and reversible
- Enable easy rollback if issues discovered

**PC3. Documentation Standards**
- Maintain WKMP coding conventions (IMPL002)
- Preserve existing traceability comments ([REQ-XXX-YYY])
- Update file-level module documentation

### Timeline Constraints

**TL1. Estimated Effort**
- Initial estimate: 8-12 hours (per wip/mixer_technical_debt_analysis.md)
- Conservative estimate: Allow up to 15 hours for complexity
- Checkpoint reviews every 4-5 hours

---

## Dependencies

### Existing Code (Read-Only)

**D1. Current engine.rs Implementation**
- File: `wkmp-ap/src/playback/engine.rs` (4,251 lines)
- Status: Implemented, in use
- Dependency: Complete understanding required before splitting
- Action: Analyze functional areas (Phase 4 - Approach Selection)

**D2. Existing Test Suite**
- Files: `wkmp-ap/tests/**/*.rs`
- Dependency: Tests define API contract
- Action: Run full test suite, identify engine-specific tests
- Coverage: Unknown - will be measured

**D3. API Handlers**
- Files: `wkmp-ap/src/api/handlers.rs` (1,305 lines)
- Dependency: Handlers call PlaybackEngine methods
- Action: Identify all call sites to ensure API compatibility

### Internal Dependencies

**ID1. Playback Subsystem**
- Related modules: mixer.rs, buffer_manager.rs, queue_manager.rs
- Integration: Engine coordinates these components
- Risk: Changes to engine structure may require coordination
- Mitigation: Internal refactoring only (no interface changes)

**ID2. Event System**
- Module: playback/events.rs
- Dependency: Engine emits PlaybackEvent via tokio::broadcast
- Status: Should be unaffected (internal implementation)

**ID3. Database Access**
- Via: sqlx queries in engine methods
- Dependency: Database schema unchanged
- Status: Should be unaffected (queries move with methods)

### External Dependencies

**ED1. Rust Standard Library**
- Version: Stable channel (current)
- Usage: Arc, RwLock, tokio, etc.
- Status: No changes expected

**ED2. Tokio Async Runtime**
- Version: 1.35 (workspace dependency)
- Usage: async/await, RwLock, broadcast channels
- Status: No changes expected

**ED3. Database (SQLite)**
- Version: sqlx 0.7
- Usage: Passage queries, queue queries
- Status: No schema changes

**No New Dependencies:** This refactoring introduces NO new crates or libraries.

---

## Integration Points

**Where Changes Will Be Made:**

1. **wkmp-ap/src/playback/engine.rs**
   - DELETE: Entire file (4,251 lines)
   - REPLACE WITH: Directory `engine/` with 4 module files

2. **wkmp-ap/src/playback/mod.rs**
   - MODIFY: Change `pub mod engine;` declaration (if needed for directory)
   - Rust automatically recognizes `engine/mod.rs` as module

3. **Potentially: wkmp-ap/Cargo.toml**
   - NO CHANGES EXPECTED (module reorganization doesn't affect Cargo.toml)

**No Changes to:**
- API handlers
- Tests
- Other playback modules (mixer, buffer_manager, etc.)
- Database schema
- Configuration files

---

## Success Criteria

**Quantitative Metrics:**

1. ✅ **Line Count:** All 4 modules < 1500 lines
2. ✅ **API Stability:** `cargo check -p wkmp-ap` succeeds without code changes to callers
3. ✅ **Test Pass Rate:** 100% of baseline tests still pass
4. ✅ **Code Coverage:** No reduction in test coverage (measured via cargo-tarpaulin if available)

**Qualitative Criteria:**

1. ✅ **Logical Organization:** Code grouped by clear functional responsibility
2. ✅ **Readability:** Each module easier to understand than original monolith
3. ✅ **Maintainability:** Future changes isolated to specific modules
4. ✅ **No Duplication:** Shared code factored into helpers, not copied

---

## Risk Summary

**See 06_risks.md for full risk assessment (Phase 7).**

**Top 3 Risks (Preview):**

1. **High Coupling Risk:** Code may be tightly coupled, making clean separation difficult
   - Mitigation: Analyze dependencies before splitting, introduce helper modules if needed

2. **API Break Risk:** Refactoring may inadvertently change public API
   - Mitigation: Compile-time verification, comprehensive API compatibility test

3. **Test Coverage Risk:** Tests may be insufficient to catch regressions
   - Mitigation: Run full test suite before/after, measure coverage if possible

---

**Scope Statement Complete**
**Phase 1.3 Status:** ✓ Scope boundaries clearly defined
