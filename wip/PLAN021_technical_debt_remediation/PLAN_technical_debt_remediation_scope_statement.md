# Technical Debt Remediation - Scope Statement

**Source Specification:** SPEC_technical_debt_remediation.md
**Generated:** 2025-11-05
**Plan Workflow Phase:** Phase 1 - Input Validation and Scope Definition

---

## Executive Summary

**Project Name:** Technical Debt Remediation for wkmp-ap and wkmp-common

**Objective:** Systematically address 9 identified technical debt items (6 code + 3 documentation) through incremental refactoring with continuous test validation.

**Scope:** Code modifications limited to wkmp-ap and wkmp-common. Test validation across entire WKMP workspace (all 7 modules).

**Approach:** Incremental refactoring (Approach A) with full test suite execution after each modification.

**Success Criteria:** All technical debt items resolved, all tests passing, no regressions, improved code quality and maintainability.

---

## In Scope

### Code Modifications

**ALLOWED:**

✅ **wkmp-ap source code** (src/, tests/, benches/)
- Refactor playback/engine/core.rs (3,156 LOC → modules <1,000 LOC)
- Remove deprecated auth_middleware.rs code (lines 250-800)
- Consolidate diagnostics modules (resolve duplication)
- Implement DEBT markers (DEBT-007, FUNC-002, FUNC-003)
- Fix clippy warnings (19 in wkmp-ap)
- Fix doctest failures (1 in api/handlers.rs)
- Address dead code warnings (76 expected after pragma removal)
- Remove obsolete files (legacy decoder code)

✅ **wkmp-common source code** (src/, tests/)
- Fix clippy warnings (5 in wkmp-common)
- Address large error variant warnings
- API compatibility MUST be preserved (no breaking changes)

✅ **Cargo.toml files** (wkmp-ap, wkmp-common)
- Dependency updates ONLY if necessary to fix clippy warnings
- No major version bumps
- No new dependencies without justification

---

### Documentation Modifications

**ALLOWED:**

✅ **Implementation Tier Documents (IMPL*)** - May be updated or created

**Create New:**
1. **IMPL008-decoder_worker_implementation.md**
   - Comprehensive DecoderWorker architecture documentation
   - Target: 500-1000 lines
   - Content: Architecture, components, algorithms, performance, integration

2. **Buffer Tuning Workflow Documentation**
   - Operator guide for tune-buffers utility
   - Target: 300-500 lines
   - Location: TBD (IMPL009, README section, or docs/operators/)
   - Content: When to tune, process, parameters, troubleshooting

**Update Existing:**
3. **IMPL003-project_structure.md**
   - Reflect current wkmp-ap module organization
   - Remove obsolete module references (decoder_pool, serial_decoder)
   - Update key files list (post-refactoring)
   - Document test organization

4. **Module-Level Documentation**
   - Add comprehensive `//!` docs to new modules from core.rs split
   - Update existing module docs for refactored code
   - Link to relevant SPEC documents

5. **README Files**
   - wkmp-ap/README.md: Overview, architecture, build/test instructions
   - wkmp-common/README.md: Verify accuracy after changes

6. **CHANGELOG / change_history.md**
   - Summarize technical debt remediation
   - Document major changes (core.rs refactoring)
   - List completed features (DEBT markers resolved)

✅ **Inline Code Documentation** (doc comments)
- Update doc comments in refactored code
- Fix broken doc links
- Complete missing public API documentation

✅ **Traceability Comments**
- Preserve existing `[REQ-*]`, `[SPEC-*]` references
- Update references if code moved
- Remove resolved `[DEBT-*]` markers after implementation

---

### Testing Activities

**REQUIRED:**

✅ **Baseline Establishment** (Increment 1)
- Run complete test suite: `cargo test --workspace`
- Document baseline results (pass/fail counts, duration)
- Identify pre-existing issues (if any)
- Run benchmarks: `cargo bench --package wkmp-ap`
- Establish performance baseline

✅ **Continuous Validation** (After each modification)
- Execute: `cargo test --workspace`
- STOP if any test fails
- Present failure details to user for approval before test modification

✅ **Performance Validation** (After core.rs refactoring)
- Re-run benchmarks
- Compare to baseline (must be within ±5%)
- Verify audio callback latency unchanged

✅ **Clippy Validation** (Final acceptance)
- Execute: `cargo clippy --workspace -- -W clippy::all -D warnings`
- Must pass cleanly

✅ **Documentation Builds** (Final acceptance)
- Execute: `cargo doc --workspace --no-deps`
- Verify no broken doc links

---

## Out of Scope

### Code Changes - NOT ALLOWED

❌ **New feature development**
- Only complete DEBT markers (existing planned features)
- No net-new functionality beyond DEBT scope

❌ **API enhancements**
- Only DEBT marker implementation (FUNC-002, FUNC-003)
- No additional API features

❌ **Performance optimizations**
- Maintain performance (no regressions)
- Do NOT add optimizations unless addressing regression

❌ **UI/UX changes**
- No changes to any user interface code

❌ **Database schema modifications**
- No migration file creation
- No schema changes
- Exception: May read/write settings table for DEBT implementations

❌ **External dependency upgrades**
- No version bumps unless fixing specific clippy warning
- No adding dependencies for convenience

---

### Modules - NOT ALLOWED

❌ **wkmp-ui modifications**
- No code changes
- Testing ONLY (validate no regressions from wkmp-common changes)

❌ **wkmp-pd modifications**
- No code changes
- Testing ONLY

❌ **wkmp-ai modifications**
- No code changes
- Testing ONLY

❌ **wkmp-le modifications**
- No code changes
- Testing ONLY

❌ **wkmp-dr modifications**
- No code changes
- Testing ONLY

❌ **Root workspace files**
- No changes to root Cargo.toml workspace definition
- No changes to .gitignore, .github/, CI/CD configs
- No changes to build scripts (scripts/)

❌ **Migration files**
- migrations/* are read-only
- No new migrations, no modifications to existing

---

### Documentation - NOT ALLOWED

❌ **Specification tier documents (SPEC*, REQ*, GOV*)**
- These are READ-ONLY for reference
- No updates even if inconsistencies found
- Report issues but do NOT modify

❌ **Comprehensive API documentation overhaul**
- Only gap-filling and validation
- Only updates necessitated by code changes
- Do NOT rewrite existing adequate docs

❌ **User-facing end-user documentation**
- Operator guide (buffer tuning) is IN scope
- General user guides, tutorials OUT of scope

❌ **Architecture documentation changes**
- Only update if reflecting actual code changes
- Do NOT redesign architecture in documentation

❌ **Tutorial or getting-started guides**
- Not part of remediation scope

❌ **Contribution guidelines**
- Not part of remediation scope

❌ **Developer onboarding documentation**
- Not part of remediation scope

---

### Testing - NOT ALLOWED

❌ **Adding completely new test categories**
- May add tests for DEBT implementations
- Do NOT add test infrastructure or frameworks

❌ **Performance test suite expansion**
- Use existing benchmark suite only
- Do NOT add new benchmark categories

❌ **Test infrastructure improvements**
- Use existing test harness
- Do NOT refactor test utilities

❌ **CI/CD pipeline modifications**
- Out of scope
- Run tests locally, do NOT modify CI configs

---

## Boundaries and Constraints

### Code Modification Boundaries

**Spatial Constraint:**
- Modifications confined to wkmp-ap/ and wkmp-common/ directories
- Other modules tested but NOT modified

**Functional Constraint:**
- Preserve all existing functionality
- No breaking changes to public APIs
- Semantic versioning enforced

**Quality Constraint:**
- Must pass all existing tests
- No performance regressions
- Clippy clean by end

---

### Test Modification Protocol

**When test modification is allowed:**

1. **Code behavior intentionally changed** (e.g., DEBT-003 adds album UUID to events)
   - Test expects old structure → needs update
   - **Action:** STOP, present to user, explain change, await approval

2. **Test implementation bug discovered**
   - Test has incorrect assertion logic
   - **Action:** STOP, present bug details, propose fix, await approval

3. **Coverage gap being filled**
   - Adding NEW test for edge case
   - **Action:** OK to add without approval
   - **NOT OK:** Modifying existing test expectations without approval

**NEVER allowed:**
- ❌ Modify test to pass without understanding failure root cause
- ❌ Comment out failing tests
- ❌ Change assertions to match new behavior without consultation
- ❌ Remove tests that "no longer make sense"

---

### Documentation Tier Constraints

**Per GOV001-document_hierarchy.md:**

- **Tier 0 (Governance):** GOV001, GOV002 - READ-ONLY
- **Tier 1 (Requirements):** REQ001, REQ002 - READ-ONLY
- **Tier 2 (Design):** SPEC001-SPEC028 - READ-ONLY
- **Tier 3 (Implementation):** IMPL001-IMPL008+ - MAY UPDATE/CREATE
- **Tier 4 (Execution):** EXEC001 - READ-ONLY

**Information Flow:**
- Upward flow PROHIBITED (implementation may NOT change requirements)
- Downward flow NORMAL (requirements inform implementation)

---

## Assumptions

**A1: Pre-existing Test Baseline**
- Assumption: No critical pre-existing test failures in workspace
- Validation: Establish baseline in Increment 1
- Risk: If major pre-existing failures found, may require triage/deferral

**A2: Benchmark Stability**
- Assumption: Benchmarks are stable and repeatable
- Validation: Run benchmarks multiple times in baseline
- Risk: Flaky benchmarks may cause false regression alerts

**A3: DEBT Marker Specifications**
- Assumption: DEBT markers have sufficient specification to implement
- Validation: Phase 2 specification completeness check
- Risk: Underspecified DEBT may require additional research

**A4: Config Struct Removal**
- Assumption: Legacy Config struct is truly obsolete
- Validation: Grep for usage in Increment 6
- Risk: External dependencies may prevent removal

**A5: Single Developer**
- Assumption: Single developer (Claude Code) executing work
- Implication: Approach C (Parallel) not applicable
- Risk: None (assumption validated)

**A6: Stable Dependencies**
- Assumption: Current dependency versions stable
- Validation: No major version bumps needed
- Risk: Clippy warnings may require minor version updates

---

## Dependencies

### External Dependencies

**Required Tools:**
- Rust toolchain (stable channel) - ASSUMED AVAILABLE
- cargo workspace commands - ASSUMED AVAILABLE
- git for version control - ASSUMED AVAILABLE
- Access to all WKMP module source code - ASSUMED AVAILABLE

**Required Documentation:**
- Technical Debt Review Report (completed) - AVAILABLE
- IMPL002-coding_conventions.md - AVAILABLE
- SPEC016-decoder_buffer_design.md - AVAILABLE (for core.rs refactoring guidance)
- SPEC028-playback_orchestration.md - AVAILABLE (for engine structure)
- GOV001-document_hierarchy.md - AVAILABLE (for documentation tier rules)

**Required Environment:**
- Database migrations applied - ASSUMED READY
- Test audio files available - ASSUMED AVAILABLE (for integration tests)
- System audio devices available - ASSUMED AVAILABLE (for output tests)

---

### Internal Dependencies (Between Requirements)

**Critical Path:**

```
NFR-001 (Test Coverage Preservation)
    ↓ [GATES ALL WORK]
FR-001 (Refactor core.rs) → Increment 2 (2-3 days)
    ↓
FR-002 (Code Cleanup) → Increment 3 (1 day)
    ↓
FR-003 (DEBT Completion) → Increment 4 (2-3 days)
    ↓
FR-004 (Code Quality) → Increment 5 (1 day)
    ↓
FR-002 (Config Cleanup) → Increment 6 (1 day)
    ↓
FR-005, FR-006 (Documentation) → Increment 7 (2-3 days)
```

**Key Dependency Relationships:**

1. **NFR-001 → ALL** (Test coverage gates all work)
2. **FR-001 → FR-005** (Refactored modules need updated docs)
3. **FR-001 → FR-006** (References updated after code moves)
4. **FR-001 → NFR-002** (Performance validation after refactoring)
5. **FR-003 → FR-006** (DEBT markers removed from docs after implementation)
6. **ALL CODE → FR-005/FR-006** (Documentation updated after code complete)

---

## Success Criteria

### Completion Criteria

**Technical Debt Resolution:**
- ✅ TD-H-001: core.rs refactored into modules each <1,000 LOC
- ✅ TD-M-001: Diagnostics duplication resolved
- ✅ TD-M-002: Deprecated auth middleware removed
- ✅ TD-M-003: DEBT-007, FUNC-002, FUNC-003 implemented
- ✅ TD-L-001: Clippy warnings addressed, doctest fixed
- ✅ TD-L-002: Configuration fragmentation evaluated and addressed
- ✅ TD-L-003: IMPL008-decoder_worker_implementation.md created
- ✅ TD-L-004: Buffer tuning workflow documented
- ✅ TD-L-005: IMPL003-project_structure.md updated

**Quality Assurance:**
- ✅ All existing tests pass (baseline preserved)
- ✅ New tests added for new functionality (DEBT completions)
- ✅ Benchmark results within ±5% of baseline
- ✅ No increase in compiler warnings (excluding expected dead_code)
- ✅ Clippy clean: `cargo clippy --workspace -- -W clippy::all -D warnings` passes

**Cross-Module Validation:**
- ✅ wkmp-common changes tested against ALL downstream modules
- ✅ No breaking changes introduced
- ✅ Public API compatibility verified

**Documentation:**
- ✅ All completion criteria met per FR-005, FR-006
- ✅ Documentation passes quality standards checklist
- ✅ CHANGELOG entry created

---

### Acceptance Criteria

**This remediation is SUCCESSFUL when:**

1. ✅ All technical debt items addressed (or explicitly deferred with rationale)
2. ✅ Complete test suite passes: `cargo test --workspace`
3. ✅ No regressions introduced in any module
4. ✅ Code quality improved (fewer warnings, better structure)
5. ✅ Codebase more maintainable (smaller files, less duplication)
6. ✅ No consultation required after-the-fact (all approvals obtained during work)

**Final Validation Commands:**
```bash
# Full test suite
cargo test --workspace

# Clean clippy
cargo clippy --workspace -- -W clippy::all -D warnings

# Documentation builds
cargo doc --workspace --no-deps

# Benchmarks compile
cargo bench --workspace --no-run

# No uncommitted changes
git status
```

**All commands above must succeed for acceptance.**

---

## Risk and Mitigation

### Scope Creep Risks

**Risk:** Discovering additional technical debt during refactoring
- **Mitigation:** Document discovered issues, defer to future work
- **Protocol:** Add to tech debt backlog, do NOT expand scope

**Risk:** User requests additional features during remediation
- **Mitigation:** Acknowledge request, schedule separately
- **Protocol:** Complete current scope before new work

---

### Technical Risks

**Risk:** Test failures after code changes
- **Probability:** MEDIUM (expected during refactoring)
- **Impact:** LOW (STOP protocol catches immediately)
- **Mitigation:** Incremental validation, immediate rollback
- **Residual Risk:** LOW

**Risk:** Breaking changes in wkmp-common
- **Probability:** LOW (scope explicitly prohibits)
- **Impact:** HIGH (breaks all downstream modules)
- **Mitigation:** No wkmp-common API changes in scope, full workspace testing
- **Residual Risk:** LOW

**Risk:** Performance regression
- **Probability:** LOW (refactoring preserves logic)
- **Impact:** MEDIUM (affects audio quality)
- **Mitigation:** Benchmark validation after core.rs refactor
- **Residual Risk:** LOW

**Risk:** Underspecified DEBT markers
- **Probability:** MEDIUM (specifications may be incomplete)
- **Impact:** MEDIUM (implementation blocked)
- **Mitigation:** Phase 2 completeness verification, user consultation
- **Residual Risk:** LOW

---

### Schedule Risks

**Risk:** Effort underestimated
- **Probability:** MEDIUM (refactoring complexity unknown)
- **Impact:** MEDIUM (schedule slip)
- **Mitigation:** Incremental approach allows early stopping
- **Residual Risk:** MEDIUM

**Risk:** Discovering pre-existing failures
- **Probability:** LOW (project well-maintained)
- **Impact:** HIGH (blocks all work)
- **Mitigation:** Baseline establishment in Increment 1, triage protocol
- **Residual Risk:** LOW

---

## Stakeholder Communication

### User Consultation Points

**Built into Incremental Approach:**

1. **After Increment 1 (Baseline):** Present baseline results, confirm no blockers
2. **After Increment 2 (core.rs refactor):** Present refactored structure, confirm approach
3. **If test modification needed:** STOP, present details, await approval
4. **After Increment 4 (DEBT completion):** Confirm implementations meet intent
5. **After Increment 7 (Documentation):** Review documentation deliverables

**STOP Protocol Triggers:**
- Any test failure after modification
- Need to modify test expectations
- Discovery of specification ambiguity
- Discovery of unexpected dependencies

---

## Timeline

**Total Estimated Effort:** 14-21.75 days

**Breakdown by Increment:**
1. Increment 1 (Baseline): 1 hour
2. Increment 2 (core.rs refactor): 2-3 days
3. Increment 3 (Code cleanup): 1 day
4. Increment 4 (DEBT completion): 2-3 days
5. Increment 5 (Code quality): 1 day
6. Increment 6 (Config cleanup): 1 day
7. Increment 7 (Documentation): 4-6.75 days

**Note:** Documentation work (Increment 7) accounts for 28-31% of total effort.

---

## Next Steps

**Phase 1 Status:** Scope defined and documented.

**Next Phase:** Phase 2 - Specification Completeness Verification
- Answer open questions (core.rs split strategy, DEBT specifications)
- Identify gaps and ambiguities
- Validate all requirements are testable

**Approval Required:** User should review and approve scope statement before proceeding to Phase 2.

---

*End of Scope Statement*
