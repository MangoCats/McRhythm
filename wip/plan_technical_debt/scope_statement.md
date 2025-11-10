# Scope Statement: Technical Debt Reduction

**Project:** WKMP Technical Debt Reduction
**Specification:** SPEC_technical_debt_reduction.md
**Generated:** 2025-11-10

---

## 1. In Scope

### Modules
- **wkmp-ai:** All Rust source files in `wkmp-ai/src/**/*.rs`
- **wkmp-common:** All Rust source files in `wkmp-common/src/**/*.rs`

### Work Categories
1. **File Reorganization:**
   - Split monolithic files (>800 lines) into modular structures
   - workflow_orchestrator.rs (2,253 lines) → 7-8 modules
   - events.rs (1,711 lines) → 3-4 modules
   - params.rs (1,450 lines) → 4-5 modules
   - api/ui.rs (1,308 lines) → 5-6 modules

2. **Error Handling:**
   - Audit 506 unwrap()/expect() calls
   - Convert 200+ user-facing unwrap() to proper error handling
   - Add error context with anyhow::Context
   - Replace panic! with Result types

3. **Dead Code Removal:**
   - Delete song_processor.rs (368 lines, unused)
   - Remove unused imports (4 instances)
   - Remove dead code fields/methods (5 instances)

4. **Code Quality:**
   - Fix 11 compiler warnings
   - Fix 22 clippy lints
   - Extract rate limiter utility (eliminate 4 duplicates)
   - Break up 450+ long functions (>150 lines)
   - Consolidate configuration structs
   - Remove magic numbers

5. **Documentation:**
   - Enable missing_docs lint
   - Document 100% of public modules
   - Document 100% of public functions (294 total)
   - Add module-level documentation with examples

6. **Critical Fixes:**
   - Replace blocking sleep in async context (time.rs:37)
   - Fix panic statements (2 instances)

---

## 2. Out of Scope

### Explicitly Excluded

**Functional Changes:**
- NO behavior modifications (all changes must be refactoring-only)
- NO new features or capabilities
- NO algorithm changes
- NO performance optimizations (defer to separate effort)

**Test Changes:**
- NO test additions (maintain existing 216 tests)
- NO test modifications (assertions unchanged)
- Tests must continue passing after each increment

**Other Modules:**
- NO changes to wkmp-ap (Audio Player)
- NO changes to wkmp-ui (User Interface)
- NO changes to wkmp-pd (Program Director)
- NO changes to wkmp-le (Lyric Editor)
- NO changes to wkmp-dr (Database Review)

**Architecture:**
- NO architectural redesign
- NO database schema changes
- NO API contract changes (backward compatibility required)
- NO dependency version upgrades (unless required for fixes)

**Dependencies:**
- NO new dependencies without explicit approval
- NO removal of existing dependencies

---

## 3. Assumptions

### Technical Assumptions

1. **Test Coverage:**
   - Existing 216 tests provide adequate coverage for regression detection
   - Tests accurately verify current behavior
   - Test failures reliably indicate breaking changes

2. **Build Environment:**
   - Rust stable channel available
   - Cargo clippy available
   - Cargo fmt available
   - All existing dependencies resolvable

3. **Code State:**
   - PLAN024 implementation is complete and stable
   - No concurrent development on wkmp-ai or wkmp-common
   - Working directory clean (no uncommitted changes)

4. **Documentation:**
   - Technical debt report (wip/technical_debt_report.md) accurately identifies issues
   - Specification (SPEC_technical_debt_reduction.md) is complete and approved

### Process Assumptions

1. **Incremental Delivery:**
   - Each phase can be completed independently
   - Phases do not block each other
   - Partial completion is acceptable (phases are independently valuable)

2. **Review and Approval:**
   - Each phase has clear acceptance criteria
   - Tests passing = acceptance criteria met
   - User reviews and approves each phase before proceeding

3. **Time Availability:**
   - Estimated 4-6 weeks total effort
   - No hard deadlines (quality > speed)
   - Can pause between phases if needed

---

## 4. Constraints

### Technical Constraints

1. **Backward Compatibility (CRITICAL):**
   - Public APIs MUST NOT change in breaking ways
   - All public function signatures preserved
   - All public types preserved
   - All public constants preserved
   - Semantic versioning: patch version bumps only

2. **Test Preservation (CRITICAL):**
   - All 216 tests MUST pass after each increment
   - No tests skipped or disabled
   - No test behavior changes
   - Test execution time MUST NOT increase >10%

3. **Rust Toolchain:**
   - Rust stable channel only (no nightly features)
   - Cargo 1.70+ required
   - Edition 2021

4. **Dependencies:**
   - No new dependencies without approval
   - Existing dependencies:
     - tokio (async runtime)
     - sqlx (database)
     - axum (web framework)
     - anyhow (error handling)
     - thiserror (custom errors)
     - serde (serialization)

### Process Constraints

1. **Incremental Refactoring:**
   - NO "big bang" rewrites
   - Each increment independently testable
   - Continuous integration (tests pass after each change)
   - Git commit after each working increment

2. **Phase Ordering:**
   - Phases SHOULD be completed in order (1 → 5)
   - Exception: Cross-phase requirements apply to all phases
   - Rationale: Phase 1 (Quick Wins) reduces noise for later phases

3. **Code Review:**
   - All changes subject to review
   - User approval required before proceeding to next phase
   - Clear justification for any deviations from specification

---

## 5. Success Metrics

### Phase 1 Metrics
- ✅ Zero compiler warnings: `cargo build` clean
- ✅ Zero clippy warnings: `cargo clippy` clean
- ✅ Zero panics in production code
- ✅ Async sleep used in async contexts
- ✅ Dead code removed (-368 lines)

### Phase 2 Metrics
- ✅ All files <800 lines
- ✅ workflow_orchestrator.rs → 7-8 modules, largest <650 lines
- ✅ events.rs → 3-4 modules, largest <600 lines
- ✅ params.rs → 4-5 modules, largest <400 lines
- ✅ api/ui.rs → 5-6 modules, largest <300 lines

### Phase 3 Metrics
- ✅ Unwrap audit complete (506 calls classified)
- ✅ <50 remaining unwrap() calls (>90% reduction)
- ✅ User-facing paths use Result types
- ✅ Error messages include context

### Phase 4 Metrics
- ✅ missing_docs lint enabled
- ✅ 100% module documentation
- ✅ 100% public function documentation (294 functions)
- ✅ `cargo doc` generates complete docs

### Phase 5 Metrics
- ✅ Rate limiter utility extracted (4 → 1 implementation)
- ✅ All functions <200 lines
- ✅ Single WorkflowConfig struct
- ✅ Magic numbers replaced with constants

### Overall Metrics
- ✅ 216/216 tests passing
- ✅ Zero API breaking changes
- ✅ Build time unchanged (±10%)
- ✅ Documentation complete

---

## 6. Risks and Mitigations

### Risk 1: Unexpected Test Failures
- **Probability:** Medium
- **Impact:** High (blocks phase completion)
- **Mitigation:** Test after each small increment, isolate failures quickly

### Risk 2: Breaking Public APIs
- **Probability:** Low-Medium
- **Impact:** Critical (breaks backward compatibility)
- **Mitigation:** Review all public API changes, verify semantic versioning

### Risk 3: Phase Dependencies
- **Probability:** Medium
- **Impact:** Medium (blocks later phases)
- **Mitigation:** Complete phases in order, resolve blockers before proceeding

### Risk 4: Scope Creep
- **Probability:** Medium
- **Impact:** Medium (increases effort, delays completion)
- **Mitigation:** Strict adherence to scope statement, defer out-of-scope work

### Risk 5: Merge Conflicts (if concurrent work)
- **Probability:** Low (assuming no concurrent development)
- **Impact:** Medium (rework required)
- **Mitigation:** Verify clean working directory, coordinate with team

---

## 7. Deliverables

### Per-Phase Deliverables
1. **Phase 1:** Clean build (zero warnings/lints), dead code removed
2. **Phase 2:** Modular file structure, all files <800 lines
3. **Phase 3:** Unwrap audit report, error handling improvements
4. **Phase 4:** Complete API documentation, missing_docs lint enabled
5. **Phase 5:** Extracted utilities, refactored functions, consolidated config

### Documentation Deliverables
- CHANGELOG.md entries for each phase
- Updated module documentation
- Unwrap audit report (Phase 3)
- Implementation plan (this /plan workflow output)

### Code Deliverables
- Refactored source files (wkmp-ai, wkmp-common)
- Git commits per increment (tests passing)
- Git tags per phase completion

---

## 8. Acceptance Criteria (Overall)

**Phase completion requires ALL of the following:**

1. ✅ All 216 tests passing
2. ✅ Zero compiler warnings
3. ✅ Zero clippy warnings
4. ✅ All acceptance criteria from specification met
5. ✅ No breaking API changes
6. ✅ Documentation updated
7. ✅ Git commit with passing tests
8. ✅ User review and approval

**Project completion requires:**

- ✅ All 5 phases complete
- ✅ All cross-phase requirements satisfied
- ✅ CHANGELOG.md updated
- ✅ Technical debt metrics achieved (see Section 5)
- ✅ Final user acceptance
