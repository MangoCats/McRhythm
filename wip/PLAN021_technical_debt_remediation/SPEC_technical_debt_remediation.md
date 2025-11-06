# Technical Debt Remediation Specification

**Document Type:** Implementation Specification
**Status:** Draft - Awaiting /plan execution
**Created:** 2025-11-05
**Target Modules:** wkmp-ap, wkmp-common
**Test Scope:** All WKMP modules (wkmp-ap, wkmp-common, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le, wkmp-dr)

---

## Document Information

**Purpose:** Specify implementation approach for remediating technical debt identified in wkmp-ap technical debt review (2025-11-04).

**Stakeholders:**
- WKMP Development Team
- Module Maintainers (wkmp-ap, wkmp-common)

**Scope Boundaries:**
- **Code Modifications:** wkmp-ap and wkmp-common ONLY
- **Test Execution:** ALL WKMP modules (to detect cross-module regressions)
- **No modifications to:** wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le, wkmp-dr code

**Timeline:** To be determined by /plan workflow

**Related Documents:**
- Technical Debt Review Report (2025-11-04)
- [IMPL002-coding_conventions.md](../docs/IMPL002-coding_conventions.md)
- [GOV001-document_hierarchy.md](../docs/GOV001-document_hierarchy.md)

---

## Executive Summary

### Problems to Address

Based on technical debt review findings, this specification covers remediation of:

**Code Technical Debt:** *(UPDATED after Phase 2 analysis)*

1. **[HIGH PRIORITY] File Size Concentration** - playback/engine/core.rs at 3,156 LOC
2. ~~**[MEDIUM PRIORITY] Module Duplication**~~ ← **REMOVED** (misdiagnosis - proper separation, not duplication)
3. **[MEDIUM PRIORITY] Deprecated Code Retention** - 530+ LOC of unused auth middleware
4. **[MEDIUM PRIORITY] DEBT Markers** - 8 tracked incomplete features (3 to implement)
5. **[LOW PRIORITY] Code Quality** - 19 clippy warnings, 1 doctest failure
6. **[LOW PRIORITY] Configuration Fragmentation** - Legacy Config struct cleanup

**Documentation Technical Debt:**

7. **[LOW PRIORITY] Documentation Gaps** - 3 identified implementation documentation gaps
8. **[LOW PRIORITY] Documentation Updates** - Module structure changes require doc updates
9. **[LOW PRIORITY] API Documentation** - Public API completeness review

**Total Items:** 8 code + 3 documentation = **11 items** (was 12 - item #2 removed)

### Quality Assurance Strategy

**Continuous Validation Approach:**

All code modifications must be validated against the ENTIRE WKMP test suite to ensure:
- No regressions in wkmp-ap functionality
- No breaking changes in wkmp-common affecting downstream modules
- All existing tests continue to pass
- New functionality is adequately tested

**Test Baseline Requirement:**

Before ANY code modifications:
1. Establish baseline: Run complete test suite across ALL modules
2. Document current test results (pass/fail counts per module)
3. Identify any pre-existing test failures (exclude from regression detection)

**Test-After-Modify Protocol:**

After EACH logical code change (per increment or sub-increment):
1. Run complete test suite: `cargo test --workspace`
2. Compare results to baseline
3. If ANY new failures detected:
   - STOP immediately
   - Do NOT proceed to next change
   - Analyze failure root cause
   - Determine if test needs modification OR code change needs revision
4. If test modification appears necessary:
   - STOP for review and consultation with user
   - Present test failure details
   - Explain why test modification might be needed
   - Await explicit user approval before modifying test
   - Document rationale for test modification

**Test Modification Approval Required:**

Test modifications require explicit user review when:
- Test expectations need updating due to behavior change
- Test implementation contains bugs being fixed
- Test coverage gaps being addressed

Tests must NOT be modified to "make them pass" without understanding root cause.

### Critical Constraints

1. **No Breaking Changes:** Public APIs in wkmp-common must remain compatible
2. **Test Suite Integrity:** All 219+ existing tests must continue passing
3. **Incremental Progress:** Each change validated independently before proceeding
4. **User Consultation:** Required for any ambiguous test modification scenarios

---

## Current State Assessment

### Test Baseline (Pre-Modification)

**Requirement:** Document current state of all module tests

**Modules to Test:**

| Module | Purpose | Expected Test Types |
|--------|---------|-------------------|
| wkmp-ap | Audio Player | Unit (219), Integration, Benchmarks (8) |
| wkmp-common | Shared Library | Unit tests for models, utilities |
| wkmp-ui | User Interface | Unit tests, API tests |
| wkmp-pd | Program Director | Unit tests, selection algorithm tests |
| wkmp-ai | Audio Ingest | Unit tests, file scanning tests |
| wkmp-le | Lyric Editor | Unit tests |
| wkmp-dr | Database Review | Unit tests |

**Baseline Commands:**

```bash
# Full workspace test suite
cargo test --workspace

# Per-module detailed results
cargo test --package wkmp-ap --lib
cargo test --package wkmp-ap --test '*'
cargo test --package wkmp-ap --bench '*' --no-run
cargo test --package wkmp-common --lib
cargo test --package wkmp-ui --lib
cargo test --package wkmp-pd --lib
cargo test --package wkmp-ai --lib
cargo test --package wkmp-le --lib
cargo test --package wkmp-dr --lib

# Doctest validation
cargo test --doc --workspace

# Clippy baseline
cargo clippy --workspace -- -W clippy::all
```

**Expected Outputs:**
- Pass/fail counts per module
- Total test count across workspace
- Pre-existing failures documented (if any)
- Clippy warning count baseline

### Known Issues (Pre-Remediation)

From technical debt review:

1. **wkmp-ap:**
   - 76 compiler warnings (expected after dead_code pragma removal)
   - 1 doctest failure in api/handlers.rs
   - 19 clippy warnings

2. **wkmp-common:**
   - 5 clippy warnings (from shared review)
   - Large error variant warnings

3. **Other modules:**
   - To be documented in baseline

### Technical Debt Inventory

**HIGH Severity (1 item):**
- TD-H-001: File size - playback/engine/core.rs (3,156 LOC → target <1,000 LOC per file)

**MEDIUM Severity (2 items):** *(CORRECTED: TD-M-001 removed - was misdiagnosis)*
- ~~TD-M-001: Diagnostics duplication~~ ← **REMOVED** (Phase 2 analysis: not duplication, proper separation)
- TD-M-002: Deprecated auth middleware (api/auth_middleware.rs lines 250-800)
- TD-M-003: DEBT markers - 8 tracked incomplete features

**LOW Severity (5 items):**
- TD-L-001: Code quality - 19 clippy warnings, 1 doctest failure
- TD-L-002: Configuration fragmentation - Legacy Config struct
- TD-L-003: Documentation gap - IMPL008 DecoderWorker implementation doc missing
- TD-L-004: Documentation gap - Buffer tuning workflow not documented for operators
- TD-L-005: Documentation sync - IMPL003 project structure outdated

**Total Items:** 8 (was 9 - TD-M-001 removed after analysis)

---

## Requirements

### Functional Requirements

**FR-001: Code Organization**
- Core.rs must be refactored into modules each <1,000 LOC
- Module boundaries must be clear and logical
- Public API surface preserved (no breaking changes)
- **Refactoring Strategy (from Phase 2 analysis):**
  - Split into 4-5 component-based modules following existing pattern
  - core.rs (retained, ~800 LOC): PlaybackEngine struct, lifecycle
  - playback.rs (~600-800 LOC): Playback control methods
  - chains.rs (~600-800 LOC): Buffer chain management
  - events.rs (~400-600 LOC): Event emission logic (includes DEBT-002/003)
  - lifecycle.rs (optional, ~200-400 LOC): Initialization/shutdown helpers

**FR-002: Code Cleanup**
- Deprecated code removed (auth_middleware.rs lines 250-800)
- ~~Duplicate modules consolidated (diagnostics)~~ ← **REMOVED** (no consolidation needed)
- Obsolete files removed (decoder_pool, serial_decoder legacy)
- Config struct replaced with simple u16 port parameter
- **Config Removal Strategy (from Phase 2 analysis):**
  - api/server.rs: Change signature to `run(port: u16, ...)`
  - main.rs: Pass port directly, remove Config construction
  - Delete config.rs OR mark #[deprecated]

**FR-003: Feature Completion**
- DEBT-007: Source sample rate telemetry implementation
- REQ-DEBT-FUNC-002: Duration_played calculation
- REQ-DEBT-FUNC-003: Album metadata for events
- DEBT-002: Audio clipping detection (optional - LOW priority)
- **Implementation Details (from Phase 2 analysis):**
  - **DEBT-007:** decoder_worker.rs extracts rate from symphonia Track metadata, sets `metadata.source_sample_rate = Some(rate)`
  - **FUNC-002:** Calculate at PassageCompleted emission: `(end_frame - start_frame) / 44100.0 * 1000.0` → milliseconds
  - **FUNC-003:** Call existing `db::passages::get_passage_album_uuids()`, add `album_uuids: Vec<Uuid>` to events

**FR-004: Code Quality**
- All clippy warnings addressed
- Doctest failures fixed
- Dead code warnings resolved or marked with rationale

**FR-005: Documentation Completeness**
- IMPL008-decoder_worker_implementation.md created
- Buffer tuning workflow documented for operators
- IMPL003-project_structure.md updated to reflect current organization
- Module-level documentation updated for refactored code
- Public API documentation completeness verified

**FR-006: Documentation Accuracy**
- All code-to-documentation references validated
- Obsolete documentation removed or marked deprecated
- README files updated where applicable
- Architecture diagrams updated if structure changed

### Non-Functional Requirements

**NFR-001: Test Coverage Preservation**
- All existing tests must pass after modifications
- No reduction in test coverage percentage
- New code adequately tested (target: 90%+ line coverage)

**NFR-002: Performance Maintenance**
- No performance regressions in critical paths
- Benchmark suite results within ±5% of baseline
- Audio callback latency unchanged

**NFR-003: API Compatibility**
- No breaking changes to wkmp-common public API
- Deprecated items use #[deprecated] annotation with guidance
- Semantic versioning followed for any API changes

**NFR-004: Documentation Quality**
- All public APIs documented
- Module-level documentation updated
- README files updated if structure changes
- Traceability comments preserved

---

## Proposed Implementation Approach

### Approach A: Incremental Refactoring with Continuous Testing (RECOMMENDED)

**Description:**

Address technical debt items in priority order (HIGH → MEDIUM → LOW), validating against full test suite after each logical change.

**Increments:**

1. **Increment 1: Establish Test Baseline** (1 hour)
   - Run complete test suite
   - Document baseline results
   - Identify pre-existing issues

2. **Increment 2: Refactor core.rs** (HIGH priority, 2-3 days)
   - Split into domain-specific modules
   - Maintain existing test coverage
   - Validate after each file split

3. **Increment 3: Remove Deprecated Code** (MEDIUM priority, 0.5 day) *(UPDATED: reduced from 1 day)*
   - Delete auth_middleware deprecated section (lines 250-800)
   - ~~Consolidate diagnostics modules~~ ← **REMOVED** (no consolidation needed)
   - Remove obsolete decoder files (if any found)
   - Remove Config struct (replace with u16 port parameter)

4. **Increment 4: Complete DEBT Markers** (MEDIUM priority, 2-3 days)
   - DEBT-007: Source sample rate telemetry
   - REQ-DEBT-FUNC-002 & 003: Event metadata
   - Test after each feature completion

5. **Increment 5: Code Quality Improvements** (LOW priority, 1 day)
   - Fix clippy warnings
   - Resolve doctest failure
   - Address dead code warnings
   - **Address wkmp-common test coverage gaps** (identified 2025-11-05):
     - Add unit tests for uuid_utils.rs (13 LOC)
     - Add unit tests for time.rs (13 LOC)
     - Increase test coverage for events.rs (currently 6 tests for 1,567 LOC)
     - See wkmp_common_test_coverage_report.md for details

6. **Increment 6: (REMOVED - Config cleanup moved to Increment 3)**
   - ~~Evaluate Config struct usage~~ ← Moved to Increment 3
   - ~~Remove if truly obsolete~~ ← Completed in Increment 3
   - ~~Update documentation~~ ← Completed in Increment 7

7. **Increment 7: Documentation Remediation** (LOW priority, 2-3 days)
   - Create IMPL008-decoder_worker_implementation.md (500-1000 lines)
   - Document buffer tuning workflow → **IMPL009-buffer_tuning_guide.md** (300-500 lines)
   - Update IMPL003-project_structure.md (remove obsolete refs, add current structure)
   - Update module-level docs for refactored code (core.rs split)
   - Verify public API documentation completeness (cargo doc)
   - Validate all code-to-doc references ([REQ-*], [SPEC-*], [DEBT-*])
   - Update README files where applicable (wkmp-ap/README.md)

**Risk Assessment:**

Failure Risk: **LOW**

Failure Modes:
1. Test failure after code change - Probability: MEDIUM - Impact: LOW
   - Mitigation: Immediate stop, rollback, analyze
   - Validation: Full test suite after each increment

2. Breaking change in wkmp-common - Probability: LOW - Impact: HIGH
   - Mitigation: No wkmp-common API changes in scope
   - Validation: All modules test after common changes

3. Performance regression - Probability: LOW - Impact: MEDIUM
   - Mitigation: Run benchmarks after core.rs refactor
   - Validation: Compare benchmark results to baseline

Residual Risk: **LOW**

**Quality Characteristics:**
- Maintainability: HIGH (addresses root cause of debt)
- Test Coverage: HIGH (continuous validation)
- Architectural Alignment: STRONG (follows existing patterns)

**Implementation Effort:** *(UPDATED based on Phase 2 corrections)*
- Upfront: LOW (planning and baseline - COMPLETE)
- Per-increment: 0.5-3 days each
- Total: **9-14 days** (reduced from 10-15 days, excluding Increment 6)
- Dependencies: Documentation work depends on code refactoring completion
- **Effort Breakdown:**
  - Increment 1 (Baseline): 1 hour
  - Increment 2 (core.rs refactor): 2-3 days
  - Increment 3 (Cleanup): 0.5 day (reduced - no diagnostics consolidation)
  - Increment 4 (DEBT markers): 2-3 days
  - Increment 5 (Code quality): 1 day
  - Increment 6: REMOVED (merged into Increment 3)
  - Increment 7 (Documentation): 2-3 days

**Advantages:**
- Continuous test validation catches regressions immediately
- Incremental progress allows early stopping if issues arise
- Preserves working state at each increment boundary
- User consultation points built-in

**Disadvantages:**
- Longer calendar time due to validation steps
- Requires discipline to validate fully after each change
- May identify pre-existing test issues requiring triage

---

### Approach B: Batch Refactoring with End-to-End Testing

**Description:**

Complete all refactoring work in a feature branch, then validate entire suite at end.

**Workflow:**
1. Create feature branch
2. Complete all HIGH priority items
3. Complete all MEDIUM priority items
4. Complete all LOW priority items
5. Run full test suite
6. Fix any failures found
7. Merge to main

**Risk Assessment:**

Failure Risk: **MEDIUM-HIGH**

Failure Modes:
1. Multiple interacting failures at end - Probability: HIGH - Impact: HIGH
   - Mitigation: Good luck debugging interactions
   - Difficult to isolate which change caused issue

2. Large merge conflicts with main - Probability: MEDIUM - Impact: MEDIUM
   - Long-lived branch diverges from main

3. Rollback requires discarding all work - Probability: LOW - Impact: CRITICAL
   - If fundamental issue found late, entire batch lost

Residual Risk: **MEDIUM**

**Quality Characteristics:**
- Maintainability: MEDIUM (harder to debug issues)
- Test Coverage: MEDIUM (delayed validation)
- Architectural Alignment: MEDIUM

**Implementation Effort:**
- Upfront: LOW
- Development: 6-8 days
- Debugging: UNKNOWN (2-5 days if issues found)
- Total: 8-13 days
- Dependencies: Long-lived feature branch

**Advantages:**
- Faster development (no validation pauses)
- Simpler git workflow (single feature branch)

**Disadvantages:**
- High risk of late-stage issues difficult to debug
- No early detection of problems
- Potential for complete rework if fundamental issue found
- Does NOT align with project requirement for continuous testing

---

### Approach C: Parallel Track Development

**Description:**

Multiple developers work on different technical debt items simultaneously, integrating incrementally.

**Risk Assessment:**

Failure Risk: **MEDIUM-HIGH**

Failure Modes:
1. Integration conflicts - Probability: HIGH - Impact: MEDIUM
2. Duplicated effort - Probability: MEDIUM - Impact: LOW
3. Inconsistent approaches - Probability: MEDIUM - Impact: MEDIUM

Residual Risk: **MEDIUM**

**Not Applicable:** Single developer scenario (Claude Code)

---

## Comparison Matrix

| Criterion | Approach A (Incremental) | Approach B (Batch) | Approach C (Parallel) |
|-----------|-------------------------|-------------------|---------------------|
| **Residual Risk** | LOW | MEDIUM | MEDIUM |
| **Test Validation** | Continuous | End-only | Per-integration |
| **Issue Detection** | Immediate | Delayed | Moderate |
| **Rollback Impact** | Minimal (per-increment) | CRITICAL (all work) | Moderate |
| **Debugging Ease** | HIGH (isolated changes) | LOW (interactions) | MEDIUM |
| **Development Time** | 8-12 days | 8-13 days | N/A |
| **Calendar Time** | Longest (validation pauses) | Shortest | N/A |
| **User Consultation** | Built-in checkpoints | Single end approval | Multiple integrations |
| **Aligns with Requirements** | ✅ YES | ❌ NO | N/A |

---

## Risk-Based Ranking

1. **Approach A (Incremental)** - Lowest residual risk (LOW)
2. **Approach B (Batch)** - Medium residual risk (MEDIUM)
3. **Approach C (Parallel)** - Not applicable (single developer)

---

## Recommendation

**Choose Approach A (Incremental Refactoring with Continuous Testing)**

**Rationale:**

Per Risk-First Framework, Approach A has the lowest failure risk (LOW residual risk) and directly addresses the project requirement for continuous test validation after each modification.

**Key Decision Factors:**

1. **Risk**: Approach A provides immediate detection of regressions vs. delayed detection in Approach B
2. **Requirement Alignment**: Project explicitly requires "re-run ENTIRE suite of tests after each modification"
3. **Debugging**: Isolated changes much easier to debug than interacting changes
4. **Rollback**: Minimal impact (single increment) vs. potentially losing all work

**Effort Consideration:**

While Approach A may take 8-12 days vs. Approach B's 8-13 days, the effort is equivalent when accounting for Approach B's unknown debugging time (2-5 days). More importantly, Approach A's incremental validation reduces the RISK of requiring extensive rework.

**Per project charter:** Quality-absolute goals ("flawless audio playback") require risk-minimizing approach. The modest additional effort of continuous validation is justified by dramatically lower risk.

---

## Implementation Constraints

### Code Modification Boundaries

**ALLOWED Modifications:**
- ✅ wkmp-ap source code (src/, tests/, benches/)
- ✅ wkmp-common source code (src/, tests/)
- ✅ wkmp-ap Cargo.toml (dependencies only if necessary)
- ✅ wkmp-common Cargo.toml (dependencies only if necessary)

**PROHIBITED Modifications:**
- ❌ wkmp-ui code (any changes)
- ❌ wkmp-pd code (any changes)
- ❌ wkmp-ai code (any changes)
- ❌ wkmp-le code (any changes)
- ❌ wkmp-dr code (any changes)
- ❌ Root Cargo.toml workspace definition
- ❌ Migration files (migrations/*)
- ❌ Specification tier documents (SPEC*, REQ*, GOV*) - read-only for reference
- ✅ Implementation tier documents (IMPL*) - may be updated/created as needed
- ✅ Module README files (wkmp-ap/README.md, etc.) - may be updated
- ✅ Inline code documentation (doc comments) - should be updated

### Test Execution Scope

**MUST Execute After Each Change:**
```bash
cargo test --workspace
```

**Detailed Validation When Needed:**
```bash
# Specific module validation
cargo test --package wkmp-ap --lib
cargo test --package wkmp-ap --test '*'
cargo test --package wkmp-common --lib

# Affected downstream modules (if wkmp-common changed)
cargo test --package wkmp-ui --lib
cargo test --package wkmp-pd --lib
# etc.

# Benchmark validation (after performance-sensitive changes)
cargo bench --package wkmp-ap --no-run  # Verify compilation
```

### Test Modification Protocol

**When Test Modification May Be Needed:**

1. **Code behavior intentionally changed**
   - Example: Event metadata now includes album UUIDs (DEBT-003)
   - Test expects old event structure
   - Action: STOP, present to user, explain change, await approval

2. **Test implementation bug discovered**
   - Example: Test has incorrect assertion logic
   - Action: STOP, present bug details, propose fix, await approval

3. **Coverage gap being filled**
   - Example: Adding missing edge case test
   - Action: OK to add NEW tests without approval
   - NOT OK to modify existing test expectations without approval

**NEVER Allowed:**
- ❌ Modifying test to pass without understanding failure root cause
- ❌ Commenting out failing tests to "make CI green"
- ❌ Changing assertions to match new behavior without user consultation
- ❌ Removing tests that "no longer make sense"

**Approval Required Format:**

When stopping for test modification approval, provide:
```
TEST MODIFICATION REQUIRED

Test: [test name and file:line]
Status: FAILING after code change

Code Change: [brief description of what was modified]

Failure Details:
[test output showing failure]

Root Cause Analysis:
[explanation of why test is failing]

Proposed Test Modification:
[specific change to test]

Rationale:
[why this test modification is correct and necessary]

Alternatives Considered:
[other approaches and why rejected]

REQUEST: Approval to modify test as described above
```

---

## Success Criteria

### Completion Criteria

**Technical Debt Resolution:**
- [ ] TD-H-001: core.rs refactored into modules each <1,000 LOC
- [ ] TD-M-001: Diagnostics duplication resolved
- [ ] TD-M-002: Deprecated auth middleware removed
- [ ] TD-M-003: DEBT-007, FUNC-002, FUNC-003 implemented
- [ ] TD-L-001: Clippy warnings addressed, doctest fixed
- [ ] TD-L-002: Configuration fragmentation evaluated and addressed

**Quality Assurance:**
- [ ] All existing tests pass (baseline preserved)
- [ ] New tests added for new functionality (DEBT completions)
- [ ] Benchmark results within ±5% of baseline
- [ ] No increase in compiler warnings (excluding expected dead_code)
- [ ] Clippy clean (or warnings explicitly approved)

**Documentation:**
- [ ] TD-L-003: IMPL008-decoder_worker_implementation.md created
- [ ] TD-L-004: Buffer tuning workflow documented (operator guide)
- [ ] TD-L-005: IMPL003-project_structure.md updated
- [ ] Module-level documentation updated for refactored code
- [ ] Public API documentation completeness verified
- [ ] README files updated (wkmp-ap, wkmp-common if applicable)
- [ ] CHANGELOG entries created
- [ ] Traceability comments preserved and updated
- [ ] All code-to-doc references validated
- [ ] Obsolete documentation marked or removed

**Cross-Module Validation:**
- [ ] wkmp-common changes tested against ALL downstream modules
- [ ] No breaking changes introduced
- [ ] Public API compatibility verified

### Acceptance Criteria

**User Acceptance:**

This remediation is SUCCESSFUL when:

1. ✅ All technical debt items addressed (or explicitly deferred with rationale)
2. ✅ Complete test suite passes across ALL modules
3. ✅ No regressions introduced in any module
4. ✅ Code quality improved (fewer warnings, better structure)
5. ✅ Codebase more maintainable (smaller files, less duplication)
6. ✅ No consultation required after-the-fact (all approvals obtained during work)

**Validation Commands:**

Final acceptance validation:
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

---

## Dependencies and Prerequisites

### Required Tools

- Rust toolchain (stable channel)
- cargo workspace commands
- git for version control
- Access to all WKMP module source code

### Required Documentation

- Technical Debt Review Report (completed)
- IMPL002-coding_conventions.md
- SPEC016-decoder_buffer_design.md (for core.rs refactoring)
- SPEC028-playback_orchestration.md (for engine structure)

### Environment Setup

- Database migrations applied
- Test audio files available (for integration tests)
- System audio devices available (for output tests)

---

## Documentation Requirements

### Overview

Documentation work addresses three identified gaps from technical debt review plus updates required by code changes. All documentation follows WKMP's tier-based hierarchy per [GOV001-document_hierarchy.md](../docs/GOV001-document_hierarchy.md).

### Documentation Gaps to Fill

**Gap 1: IMPL008 - DecoderWorker Implementation (NEW DOCUMENT)**

**Requirement:** Create comprehensive implementation documentation for DecoderWorker architecture.

**Rationale:** DecoderWorker is a critical 1,061 LOC component implementing serial decoder strategy per SPEC016, but lacks dedicated implementation documentation.

**Target Audience:** Future developers, maintainers

**Required Content:**
- **Architecture Overview**
  - Single-threaded decoder task orchestration
  - DecoderChain integration pattern
  - Serial processing rationale (cache coherency benefits)
  - Comparison to previous decoder_pool approach

- **Component Structure**
  - DecoderWorker struct fields and responsibilities
  - DecoderChain lifecycle management
  - Task communication (channels, events)
  - State machine description

- **Key Algorithms**
  - Decode request prioritization (Immediate/Next/Prefetch)
  - Buffer capacity management
  - Decode resumption logic (decoder_resume_hysteresis_samples)
  - Error handling and recovery

- **Performance Characteristics**
  - Memory usage (per-chain overhead)
  - CPU utilization patterns
  - Latency targets (decode initiation, buffer fill time)
  - Cache coherency benefits (quantified if available)

- **Integration Points**
  - BufferManager interaction
  - PlaybackEngine coordination
  - Event emission patterns
  - Database passage retrieval

- **Testing Approach**
  - Unit test coverage areas
  - Integration test scenarios
  - Performance benchmark methodology

- **Traceability**
  - Link to SPEC016 authoritative design
  - Reference to SPEC028 orchestration patterns
  - Map to REQ-* requirements

**Format:** IMPL008-decoder_worker_implementation.md (~500-1000 lines)

**Success Criteria:**
- Standalone comprehensible (reader can understand without reading all code)
- Answers "why" questions (design rationale documented)
- Enables modification confidence (future developers can refactor safely)

---

**Gap 2: Buffer Tuning Workflow (OPERATOR GUIDE)**

**Requirement:** Document buffer auto-tuning workflow for system operators/administrators.

**Rationale:** `tune-buffers` binary exists (~2,300 LOC tuning module) but lacks user-facing documentation. Operators need guidance on when/how to tune.

**Target Audience:** System administrators, DevOps, advanced users

**Required Content:**
- **When to Tune**
  - Symptoms indicating tuning needed (audio underruns, stuttering)
  - Hardware changes warranting re-tune (CPU upgrade, different audio device)
  - After configuration changes (sample rate, decode streams)

- **Tuning Process**
  - Prerequisites (test audio files, stable system state)
  - Command invocation: `tune-buffers [options]`
  - Command-line options explained
  - Expected runtime duration
  - Output interpretation

- **Parameter Recommendations**
  - What each tuned parameter controls:
    - `playout_ringbuffer_size`
    - `playout_ringbuffer_headroom`
    - `decoder_resume_hysteresis_samples`
    - `mixer_batch_size_low` / `mixer_batch_size_optimal`
  - How recommendations are generated (convergence algorithm overview)
  - Safety margins and conservative defaults

- **Applying Tuning Results**
  - Where parameters are stored (database settings table)
  - How to review current values
  - How to manually override if needed
  - Rollback procedure if tuning causes issues

- **Troubleshooting**
  - Tuning fails to converge - what to check
  - Recommendations seem wrong - validation steps
  - Audio issues persist after tuning - next steps

- **Examples**
  - Example tuning session output with annotations
  - Before/after parameter comparisons
  - Common scenarios (low-power device, high-end workstation)

**Format:** **IMPL009-buffer_tuning_guide.md** (DECIDED in Phase 2)
- ~~OR section in wkmp-ap README.md~~ ← Not chosen (keeps README focused)
- ~~OR separate operators guide~~ ← Not chosen (follows IMPL tier)

**Length:** ~300-500 lines

**Success Criteria:**
- Non-developer can successfully tune system
- Troubleshooting section addresses common issues
- Examples provide confidence

---

**Gap 3: IMPL003 Project Structure (UPDATE EXISTING)**

**Requirement:** Update IMPL003-project_structure.md to reflect current wkmp-ap module organization.

**Rationale:** Project structure has evolved (decoder pool → decoder worker, diagnostics split, tuning module added). Documentation must reflect current reality.

**Target File:** [docs/IMPL003-project_structure.md](../docs/IMPL003-project_structure.md)

**Required Updates:**

1. **wkmp-ap Module Structure Section**
   - Update to show current src/ organization:
     ```
     wkmp-ap/
     ├── src/
     │   ├── api/          (HTTP server, handlers, SSE)
     │   ├── audio/        (decoder, resampler, output)
     │   ├── db/           (database access layer)
     │   ├── playback/     (engine, mixer, buffers, pipeline)
     │   │   ├── engine/   (core orchestration, queue, diagnostics)
     │   │   └── pipeline/ (decoder_chain, fader, timing)
     │   ├── tuning/       (auto-tuning system)
     │   ├── bin/          (tune_buffers utility)
     │   └── ...
     ├── tests/           (integration tests)
     ├── benches/         (performance benchmarks)
     └── test_assets/     (test audio files)
     ```

2. **Module Descriptions**
   - Add/update descriptions for:
     - `playback/engine/` - Core orchestration (lifecycle, queue, diagnostics)
     - `playback/pipeline/` - Audio processing chain components
     - `tuning/` - Automatic buffer parameter optimization
   - Remove references to obsolete modules:
     - decoder_pool (removed)
     - serial_decoder (removed)

3. **Key Files List**
   - Update largest/most important files list (reflect core.rs refactoring if completed)
   - Add decoder_worker.rs description
   - **Clarify diagnostics files** (Phase 2 finding):
     - playback/diagnostics.rs (512 LOC) - TYPE DEFINITIONS (PassageMetrics, PipelineMetrics)
     - playback/engine/diagnostics.rs (859 LOC) - IMPLEMENTATION METHODS (impl PlaybackEngine)
     - NOT duplication - proper separation of concerns

4. **Test Organization**
   - Document integration test structure (test files, helpers, fixtures)
   - Note benchmark suite organization

5. **Cross-References**
   - Ensure links to SPEC documents remain accurate
   - Update file line counts if significantly changed
   - Verify all referenced files exist

**Success Criteria:**
- Document matches actual current structure
- New developer can navigate codebase using doc
- No references to removed/obsolete code

---

### Documentation Updates Required by Code Changes

These updates become necessary AFTER code refactoring is complete:

**Update 1: Module-Level Documentation (Refactored Files)**

After core.rs refactoring into smaller modules:
- Each new module file needs comprehensive module-level doc comment (`//!`)
- Explain module's role in overall system
- Document public API contracts
- Link to relevant SPEC documents
- Provide usage examples for complex APIs

**Affected Files:** (TBD based on refactoring decisions)
- `wkmp-ap/src/playback/engine/*.rs` (split from core.rs)

**Update 2: Public API Documentation Completeness**

Verify all public items have documentation:
```bash
cargo doc --workspace --no-deps --open
```

Check for:
- Missing doc comments on `pub fn`, `pub struct`, `pub enum`
- Incomplete documentation (just "TODO" or one-liners)
- Broken doc links (`[link]` that don't resolve)

**Priority:** All items in public API surface (exported from lib.rs)

**Update 3: README Files**

**wkmp-ap/README.md** (if exists, create if not):
- Overview of audio player module
- Architecture summary (single-stream design)
- Key components (engine, mixer, decoder worker)
- Build/test instructions
- Performance characteristics
- Link to main docs/ specifications

**wkmp-common/README.md** (if exists):
- Verify accuracy after any wkmp-common changes
- Document any new shared utilities added

**Update 4: Code-to-Doc Reference Validation**

After code changes, validate traceability comments:
- `[REQ-*]` requirement IDs - still accurate?
- `[SPEC-*]` specification references - point to correct docs?
- `[DEBT-*]` markers - resolved ones removed, new issues documented?
- File:line references in docs - updated if code moved?

**Approach:**
- Grep for requirement IDs in changed files
- Cross-reference with specifications
- Update or remove outdated references

**Update 5: CHANGELOG Entry**

Create entry in CHANGELOG.md (if exists) or project_management/change_history.md summarizing:
- Technical debt remediation completed (summary)
- Major code changes (core.rs refactoring)
- Features completed (DEBT markers resolved)
- Documentation added/updated
- Breaking changes (if any - should be none)

---

### Documentation Quality Standards

All documentation must meet these standards:

**Clarity:**
- Target audience clearly identified
- Technical level appropriate for audience
- Examples provided for complex concepts
- Jargon defined on first use

**Accuracy:**
- Matches current code implementation
- No references to removed/obsolete code
- Version-specific information noted (if applicable)
- Date of last update documented

**Completeness:**
- All "TBD" or "TODO" items resolved or removed
- No obvious gaps in coverage
- Cross-references resolve correctly
- Code examples compile and run

**Maintainability:**
- Modular structure (sections <300 lines per CLAUDE.md)
- Clear section headers for navigation
- Table of contents for documents >500 lines
- External links working (for web references)

**Traceability:**
- Links to authoritative specifications
- Requirement IDs cited where applicable
- Source code file:line references for examples
- Related documents cross-referenced

---

### Documentation Validation Checklist

Before marking documentation complete:

**For New Documents:**
- [ ] Follows WKMP documentation hierarchy (correct tier)
- [ ] Assigned document number via /doc-name workflow
- [ ] Registered in workflows/REG001_number_registry.md
- [ ] Linked from relevant index/navigation pages
- [ ] Reviewed for clarity by another person (if possible)

**For Updated Documents:**
- [ ] Changes tracked in document revision history (if present)
- [ ] "Last Updated" date refreshed
- [ ] All sections reviewed (not just updated parts)
- [ ] No orphaned references to removed content
- [ ] Git commit message explains what changed and why

**For All Documentation:**
- [ ] Markdown renders correctly (check with preview)
- [ ] Internal links resolve (`[text](path)` work)
- [ ] Code blocks have language specifiers (```rust, ```bash)
- [ ] No typos or grammar issues (run spell check)
- [ ] Consistent terminology (matches project glossary/conventions)

---

### Documentation Effort Estimate

| Task | Estimated Effort | Priority |
|------|-----------------|----------|
| Create IMPL008-decoder_worker_implementation.md | 1-2 days | LOW |
| Document buffer tuning workflow | 0.5-1 day | LOW |
| Update IMPL003-project_structure.md | 0.5-1 day | LOW |
| Update module-level docs (post-refactor) | 0.5-1 day | LOW |
| Verify public API documentation | 0.5 day | LOW |
| Validate code-to-doc references | 0.5 day | LOW |
| Update README files | 0.5 day | LOW |
| Create CHANGELOG entry | 0.25 day | LOW |
| **Total Documentation Work** | **4-6.75 days** | LOW |

**Note:** Documentation work is LOW priority and can be done after all code changes are complete and validated. However, completing it ensures long-term maintainability.

---

## Out of Scope

The following are explicitly OUT OF SCOPE for this remediation:

### Code Changes
- ❌ New feature development
- ❌ API enhancements beyond DEBT marker completion
- ❌ Performance optimizations (unless addressing regression)
- ❌ UI/UX changes in any module
- ❌ Database schema modifications
- ❌ External dependency upgrades (unless fixing clippy warnings)

### Modules
- ❌ wkmp-ui modifications
- ❌ wkmp-pd modifications
- ❌ wkmp-ai modifications
- ❌ wkmp-le modifications
- ❌ wkmp-dr modifications

### Documentation (Out of Scope)
- ❌ Comprehensive API documentation overhaul (only gap-filling and validation)
- ❌ User-facing end-user documentation (operator guide is in-scope)
- ❌ Specification document rewrites (SPEC*, REQ*, GOV* are read-only)
- ❌ Architecture documentation changes (unless reflecting actual code changes)
- ❌ Tutorial or getting-started guides
- ❌ Contribution guidelines or developer onboarding docs

### Testing
- ❌ Adding completely new test categories
- ❌ Performance test suite expansion
- ❌ Test infrastructure improvements
- ❌ CI/CD pipeline modifications

---

## Open Questions

**For /plan workflow to address:**

1. **core.rs Refactoring:**
   - Q: What is the optimal module split strategy for core.rs?
   - Q: Should chain management be extracted to separate module or kept in engine?
   - Q: How to minimize public API surface during refactoring?

2. **DEBT Marker Implementation:**
   - Q: What is complete specification for DEBT-007 source sample rate telemetry?
   - Q: Where should album UUID fetching logic reside (db layer? event layer?)?
   - Q: Should duration_played be calculated at event emission or stored?

3. **Configuration Cleanup:**
   - Q: Is Config struct actually used anywhere? (grep needed)
   - Q: Are there external dependencies on Config that prevent removal?
   - Q: Should removal be phased with deprecation period?

4. **Test Baseline:**
   - Q: Are there pre-existing test failures in any module?
   - Q: What is current benchmark baseline for performance comparison?
   - Q: Are there known flaky tests to exclude from regression detection?

5. **Documentation Decisions:**
   - Q: Where should buffer tuning guide reside (IMPL009, README, or docs/operators/)?
   - Q: Should IMPL008 include performance benchmark results or reference them externally?
   - Q: What level of detail needed for DecoderWorker state machine documentation?
   - Q: Should obsolete modules be documented in IMPL003 as "historical" or completely removed?

---

## Next Steps

**To proceed with this specification:**

1. **Review and approve** this specification document
2. **Run /plan** workflow with this document as input:
   ```
   /plan wip/SPEC_technical_debt_remediation.md
   ```
3. **/plan will produce:**
   - Detailed requirements analysis with completeness verification
   - Acceptance test definitions (Given/When/Then)
   - Traceability matrix (100% requirement coverage)
   - Increment breakdown with task sequencing
   - Test specifications for each requirement

4. **Implementation begins** only after /plan output reviewed and approved

---

## Appendices

### Appendix A: Technical Debt Items Detail

**From Technical Debt Review Report (2025-11-04):**

See full report for complete analysis. Key excerpts:

- **TD-H-001 (File Size):** playback/engine/core.rs at 3,156 LOC violates project's own REQ-DEBT-QUALITY-002-010 requirement (<1,500 LOC limit)

- **TD-M-001 (Duplication):** playback/diagnostics.rs (514 LOC) vs playback/engine/diagnostics.rs (860 LOC) - consolidation needed

- **TD-M-002 (Deprecated Code):** api/auth_middleware.rs lines 250-800+ marked DEPRECATED with clear warnings, ready for removal

- **TD-M-003 (DEBT Markers):** 8 distinct markers across codebase representing incomplete features, most concentrated around telemetry and event metadata

### Appendix B: Test Suite Structure

**Current Test Organization:**

- **wkmp-ap:**
  - Unit tests: 219 passing (as of baseline)
  - Integration tests: 10 test files (~5,250 LOC)
  - Benchmarks: 8 criterion benchmarks (~1,549 LOC)

- **wkmp-common:**
  - Unit tests: TBD (to be documented in baseline)

- **Other modules:**
  - Test counts TBD (baseline documentation)

### Appendix C: Risk Mitigation Checklist

**Before Each Code Modification:**
- [ ] Current working state committed to git
- [ ] Understand scope of change (files affected)
- [ ] Identify tests likely to be affected
- [ ] Plan rollback strategy if tests fail

**After Each Code Modification:**
- [ ] Run `cargo test --workspace`
- [ ] Compare pass/fail counts to baseline
- [ ] If failures: STOP and analyze immediately
- [ ] If pass: Commit change with clear message
- [ ] Update baseline if intentional behavior change (with approval)

**For wkmp-common Changes:**
- [ ] Extra attention to downstream module tests
- [ ] Consider API compatibility impact
- [ ] Document any semantic changes
- [ ] Verify all modules still compile

---

**END OF SPECIFICATION**

---

## Document Summary

**Specification Type:** Technical Debt Remediation
**Scope:** wkmp-ap and wkmp-common (code + documentation)
**Test Validation:** Full workspace (all 7 modules)

**Technical Debt Addressed:** *(UPDATED - Phase 2 corrections)*
- **HIGH (1):** File size concentration (core.rs → 4-5 modules <1,000 LOC)
- **MEDIUM (2):** ~~Module duplication~~ (REMOVED), deprecated code, DEBT markers
- **LOW (5):** Code quality, configuration cleanup, documentation gaps
- **Total:** 8 technical debt items (was 9)

**Approach:** Incremental refactoring with continuous testing (Approach A)
- 6 active increments (baseline + 5 remediation, Increment 6 merged into 3)
- Full test suite validation after each change
- User consultation for test modifications
- **9-14 days total effort** (reduced from 10-15)

**Documentation Work:**
- Create IMPL008 (DecoderWorker implementation doc) - 500-1000 lines
- Create IMPL009 (Buffer tuning workflow guide) - 300-500 lines
- Update IMPL003 (project structure) - clarify diagnostics separation
- Update module-level docs post-refactoring
- Verify public API documentation
- Validate code-to-doc references
- 4-6.75 days effort (unchanged)

**Risk Assessment:** LOW residual risk
- Continuous validation catches regressions immediately
- Incremental approach allows early issue detection
- User consultation built into workflow

**Quality Assurance:**
- Zero breaking changes (wkmp-common API preserved)
- All 219+ existing tests pass
- Benchmark results within ±5% baseline
- Clippy clean or explicitly approved

---

## Document Status

**Status:** Planning Complete - Ready for implementation
**Author:** WKMP Development Team (via technical debt review analysis)
**Reviewers:** Claude Code /plan workflow (Phases 1-3 complete)
**Approval:** Pending user review of Phase 2 corrections
**Version:** 1.1-corrected (Phase 2 findings incorporated)
**Last Updated:** 2025-11-05

**Phase 2 Corrections Applied:**
- TD-M-001 removed (diagnostics not duplicated)
- core.rs refactoring strategy defined (4-5 file split)
- Config struct removal strategy clarified
- DEBT marker implementations detailed with code snippets
- Buffer tuning guide location decided (IMPL009)
- Effort estimate reduced (9-14 days from 10-15)
