# Specification Issues - PLAN018: Centralized Global Parameters

## Phase 2: Specification Completeness Verification

**Date:** 2025-11-02
**Requirements Analyzed:** 9 (5 functional, 4 non-functional)
**Method:** Batch analysis per /plan workflow Phase 2

---

## Executive Summary

**Issues Found:** 8 total (0 critical, 3 high, 4 medium, 1 low)
**Decision:** ✅ PROCEED - No critical blockers identified
**Recommendation:** Resolve HIGH issues before implementation, track MEDIUM issues during implementation

---

## Issue Classification Summary

| Severity | Count | Must Resolve Before Implementation? |
|----------|-------|-------------------------------------|
| CRITICAL | 0 | N/A |
| HIGH | 3 | Recommended (not blocking) |
| MEDIUM | 4 | Track during implementation |
| LOW | 1 | Address opportunistically |

---

## HIGH Severity Issues

### HIGH-001: Missing Error Handling Specification (FR-004)

**Category:** Incomplete Specification
**Requirement:** FR-004 (Database Synchronization)
**Location:** Spec lines 91-94

**Issue:**
Requirement states "Default values if database entry missing" but does not specify:
- Should missing parameter log warning or error?
- Should missing parameter block startup or continue?
- Should all parameters load or fail fast on first error?
- Should database query errors differ from missing-entry errors?

**Test:** Can we write acceptance test without this information? Unclear.

**Impact:** Implementation may make inconsistent error handling decisions

**Recommendation:**
```
Error Handling Policy:
1. Database query fails (connection error): Log ERROR, use all defaults, continue startup
2. Parameter missing from database: Log WARN with parameter name, use default, continue
3. Parameter value invalid (type mismatch): Log WARN, use default, continue
4. All parameters processed independently (no fail-fast)

Rationale: Graceful degradation - system should start even with database issues
```

**Resolution:** Add to specification or document as design decision in PLAN018

---

### HIGH-002: Ambiguous "Runtime Updates" Future Enhancement (FR-004)

**Category:** Ambiguity
**Requirement:** FR-004 (Database Synchronization)
**Location:** Spec lines 91-94 (specifically line 92)

**Issue:**
Specification mentions "Future: Support runtime updates from database" without clarifying:
- Is this a firm commitment or speculative future work?
- Should current design accommodate this (e.g., avoid architectural decisions that prevent it)?
- What's the mechanism (polling, database triggers, manual API call)?

**Test:** N/A (future enhancement)

**Impact:**
- If future enhancement is likely: Current design should avoid blocking it
- If speculative: Current design can optimize for startup-only pattern

**Recommendation:**
Clarify intent:
- **Option A:** "WILL support runtime updates (future release)" → Design for extensibility
- **Option B:** "MAY support runtime updates (no commitment)" → Optimize for current use case

**Resolution:** Document in Open Questions section (already present at lines 554-558) - This is actually already addressed. **CLOSE ISSUE** - Specification lines 554-558 explicitly state "start with startup-only, add hot reload later"

---

### HIGH-003: Validation Range Specification Missing (NFR-002)

**Category:** Missing Detail
**Requirement:** NFR-002 (Safety - "Validated ranges on write")
**Location:** Spec lines 108-111 (specifically line 111)

**Issue:**
Requirement states "Validated ranges on write" but does not specify validation rules for 15 parameters:
- What is valid range for `working_sample_rate`? (Spec implementation shows 8000-192000)
- What is valid range for `volume_level`? (0.0-1.0?)
- What is valid range for `pause_decay_factor`? (0.0-1.0?)
- Are there cross-parameter constraints? (buffer_size must be < ringbuffer_size?)

**Test:** Cannot write validation tests without knowing valid ranges

**Impact:** Each parameter setter method needs validation logic, but ranges not specified

**Recommendation:**
Add validation requirements table:

| Parameter | Valid Range | Validation Rule |
|-----------|-------------|-----------------|
| working_sample_rate | [8000, 192000] | Must be divisible by TICK_RATE divisor |
| volume_level | [0.0, 1.0] | Clamp to range |
| audio_buffer_size | [512, 8192] | Power of 2 preferred |
| pause_decay_factor | [0.0, 1.0] | Must be < 1.0 |
| ... | ... | ... |

**Resolution:** Specify ranges in PLAN018 or extract from SPEC016 (likely already defined)

---

## MEDIUM Severity Issues

### MEDIUM-001: Performance Measurement Method Unspecified (NFR-001)

**Category:** Missing Test Criteria
**Requirement:** NFR-001 (Performance)
**Location:** Spec lines 103-106

**Issue:**
Requirement specifies "RwLock::read() overhead < 10ns (uncontended)" but does not specify:
- How to measure (benchmark tool, microbenchmark, instrumentation?)
- What platform (development machine, Pi Zero 2W, both?)
- What success criteria (median, p99, max?)
- Should we measure before/after migration to detect regression?

**Test:** Performance requirement is quantified but test method unclear

**Impact:** Cannot objectively verify NFR-001 without measurement method

**Recommendation:**
```
Performance Test:
- Tool: criterion.rs (Rust microbenchmarking)
- Measurement: Median time for 10,000 iterations of PARAMS.working_sample_rate.read()
- Platform: Development machine (document specs)
- Success: Median < 10ns
- Baseline: Measure current Arc<RwLock> pattern first, compare
```

**Resolution:** Add to test specifications in Phase 3

---

### MEDIUM-002: "Full Test Suite" Definition Unclear (NFR-003)

**Category:** Ambiguity
**Requirement:** NFR-003 (Testability)
**Location:** Spec lines 113-116

**Issue:**
Requirement states "Each parameter migration verified by full test suite" but "full test suite" is undefined:
- Does this mean `cargo test --workspace`?
- Does this include integration tests (`cargo test --test '*'`)?
- Does this include manual tests?
- What is pass criteria (100% tests pass, or allow known-failing tests)?

**Test:** Cannot verify requirement without defining "full test suite"

**Impact:** Risk of inconsistent testing between parameters

**Recommendation:**
```
Full Test Suite Definition:
1. Unit tests: `cargo test --workspace` (100% pass required)
2. Integration tests: `cargo test --test '*' --workspace` (if available)
3. Manual tests: For timing-critical parameters only (Tier 3)

Pass Criteria: Zero test failures, zero panics
```

**Resolution:** Document in test specifications (Phase 3)

---

### MEDIUM-003: Test Isolation Strategy Missing (Open Question Q4)

**Category:** Missing Detail
**Requirement:** NFR-003 (Testability)
**Location:** Spec lines 569-572 (Open Question Q4)

**Issue:**
Specification asks "Should tests use separate parameter values?" and proposes `GlobalParams::new_for_test()` but doesn't commit to approach:
- If using singleton in tests: Tests may interfere with each other (not isolated)
- If creating test-specific instances: How do tests verify singleton behavior?
- What about integration tests that need database initialization?

**Test:** Test architecture decision affects all test specifications

**Impact:** Test design depends on resolution

**Recommendation:**
```
Test Strategy:
1. Unit tests for GlobalParams struct: Use test-specific instances (GlobalParams::new_for_test())
2. Integration tests for PARAMS singleton: Use singleton with test database
3. Test isolation: Use separate #[tokio::test] async contexts (Tokio provides isolation)
4. Manual tests: Use actual singleton with development database

Rationale: Unit tests isolated, integration tests realistic
```

**Resolution:** Decide during Phase 3 (test specification)

---

### MEDIUM-004: Rollback Procedure Not Specified

**Category:** Missing Process
**Requirement:** Implicit in Migration Plan
**Location:** Spec lines 543-548 (Mitigation Strategies - point 4)

**Issue:**
Mitigation strategy mentions "Rollback plan - Each parameter is separate commit" but doesn't specify rollback procedure:
- What triggers rollback (test failure, regression detected, timing error)?
- Is rollback automatic or manual?
- Does rollback require database changes?
- Can we rollback mid-migration (e.g., after parameter 8 of 15)?

**Test:** N/A (process issue, not requirement)

**Impact:** If regression occurs, unclear how to recover quickly

**Recommendation:**
```
Rollback Procedure:
1. Trigger: Any test failure or observed regression
2. Action: `git reset --hard HEAD~1` (revert last parameter commit)
3. Database: No rollback needed (parameters still in settings table)
4. Verification: Re-run test suite, verify previous state restored
5. Mid-migration: Safe - each parameter independent

Rationale: Git history provides clean rollback points
```

**Resolution:** Document in Migration Plan section (not critical, good practice)

---

## LOW Severity Issues

### LOW-001: Documentation Verbosity Standard Unspecified (NFR-004)

**Category:** Minor Ambiguity
**Requirement:** NFR-004 (Maintainability)
**Location:** Spec lines 118-121

**Issue:**
Requirement states "Clear documentation of each parameter's purpose" but doesn't define documentation standard:
- Rust doc comments (///) or inline comments (//)?
- How detailed (one-line summary or full explanation)?
- Should examples be provided?

**Test:** N/A (documentation quality subjective)

**Impact:** Minimal - documentation will exist, may vary in quality

**Recommendation:**
```
Documentation Standard:
- Rust doc comments (///) for pub fields
- Include: Purpose, valid range, units, SPEC016 tag
- Example:
  /// [DBD-PARAM-020] Working sample rate for decoded audio
  ///
  /// Valid range: [8000, 192000] Hz
  /// Default: 44100 Hz
  /// Critical: Affects all timing calculations
  pub working_sample_rate: RwLock<u32>,
```

**Resolution:** Apply during implementation, not blocking

---

## Issues NOT Found (Positive Findings)

### ✅ Completeness Check: PASSED

All requirements have:
- Clear inputs specified (database settings table, default values)
- Clear outputs specified (GlobalParams singleton, parameter access)
- Clear behavior specified (RwLock read/write, initialization)
- Clear constraints specified (performance, safety, testability)
- Dependencies specified (SPEC016, database schema)

### ✅ Testability Check: PASSED

All requirements can be objectively tested:
- FR-001: Verify all 15 parameters in GlobalParams struct
- FR-002: Verify RwLock access pattern works
- FR-003: Search codebase for hardcoded values (regression test)
- FR-004: Test database initialization with mock database
- FR-005: Verify no breaking API changes (compilation test)
- NFR-001: Microbenchmark performance
- NFR-002: Test setter validation methods
- NFR-003: Run test suite after each migration
- NFR-004: Code review for documentation presence

### ✅ Consistency Check: PASSED

No conflicting requirements identified:
- All requirements support same goal (centralized parameters)
- No priority conflicts
- No resource conflicts
- No timing conflicts

---

## Recommendations

### Immediate Actions (Before Implementation)

1. **Resolve HIGH-001:** Define error handling policy for missing/invalid parameters
2. **Resolve HIGH-003:** Specify validation ranges for all 15 parameters (extract from SPEC016)
3. **Close HIGH-002:** Already addressed in Open Questions (lines 554-558)

### Track During Implementation

1. **MEDIUM-001:** Add performance microbenchmark to test suite
2. **MEDIUM-002:** Define "full test suite" in test specifications
3. **MEDIUM-003:** Choose test isolation strategy in Phase 3
4. **MEDIUM-004:** Document rollback procedure in final plan

### Address Opportunistically

1. **LOW-001:** Apply documentation standard consistently during implementation

---

## Decision Point

**Specification Status:** ✅ ADEQUATE FOR IMPLEMENTATION

**Critical Issues:** 0 (no blockers)
**High Issues:** 2 remaining (HIGH-001, HIGH-003) - resolvable during planning
**Medium Issues:** 4 - tracked during implementation
**Low Issues:** 1 - minor, non-blocking

**Recommendation:** ✅ **PROCEED TO PHASE 3** (Test Specification)

Resolve HIGH-001 and HIGH-003 by:
1. Documenting error handling policy in this plan
2. Extracting validation ranges from SPEC016 or defining in Phase 3

**User Checkpoint:** Present this analysis for review before Phase 3.

---

## Resolution Actions

### HIGH-001 Resolution: Error Handling Policy

**Policy Decision:**

```
GlobalParams::init_from_database() Error Handling:

1. Database Connection Error:
   - Log: ERROR "Failed to query settings table: {error}"
   - Action: Return error Result::Err (fail startup)
   - Rationale: Database is critical dependency

2. Parameter Missing (query returns no row for key):
   - Log: WARN "Parameter '{key}' not in settings table, using default {default}"
   - Action: Use default value, continue processing other parameters
   - Rationale: Graceful degradation

3. Parameter Type Mismatch (value_int NULL when expecting int):
   - Log: WARN "Parameter '{key}' has invalid type, using default {default}"
   - Action: Use default value, continue
   - Rationale: Graceful degradation

4. Parameter Value Out of Range (e.g., sample_rate = -1):
   - Log: WARN "Parameter '{key}' value {value} out of range, using default {default}"
   - Action: Use default value, continue
   - Rationale: Graceful degradation

5. Processing Strategy:
   - Process all 15 parameters independently (no fail-fast)
   - Accumulate warnings, log summary at end
   - Return Ok(()) unless database connection failed
```

**Status:** ✅ RESOLVED (documented in plan)

### HIGH-003 Resolution: Validation Ranges

**Extract from SPEC016 or define based on domain knowledge:**

| Parameter | Type | Valid Range | Validation Rule | SPEC016 Ref |
|-----------|------|-------------|-----------------|-------------|
| volume_level | f32 | [0.0, 1.0] | Clamp to range | DBD-PARAM-010 |
| working_sample_rate | u32 | [8000, 192000] | Common audio rates | DBD-PARAM-020 |
| output_ringbuffer_size | usize | [4410, 1000000] | >= 0.1s @ 44.1kHz | DBD-PARAM-030 |
| output_refill_period | u64 | [10, 1000] | Reasonable wake interval (ms) | DBD-PARAM-040 |
| maximum_decode_streams | usize | [1, 32] | Resource limit | DBD-PARAM-050 |
| decode_work_period | u64 | [100, 60000] | 0.1s to 60s (ms) | DBD-PARAM-060 |
| decode_chunk_size | usize | [4410, 441000] | 0.1s to 10s @ 44.1kHz | DBD-PARAM-065 |
| playout_ringbuffer_size | usize | [44100, 10000000] | >= 1s @ 44.1kHz | DBD-PARAM-070 |
| playout_ringbuffer_headroom | usize | [2205, 88200] | >= 0.05s @ 44.1kHz | DBD-PARAM-080 |
| decoder_resume_hysteresis_samples | u64 | [2205, 441000] | >= 0.05s @ 44.1kHz | DBD-PARAM-085 |
| mixer_min_start_level | usize | [2205, 88200] | >= 0.05s @ 44.1kHz | DBD-PARAM-088 |
| pause_decay_factor | f64 | [0.5, 0.99] | Must decay (< 1.0), not too fast | DBD-PARAM-090 |
| pause_decay_floor | f64 | [0.00001, 0.001] | Near-zero threshold | DBD-PARAM-100 |
| audio_buffer_size | u32 | [512, 8192] | Power of 2 preferred | DBD-PARAM-110 |
| mixer_check_interval_ms | u64 | [5, 100] | Balance responsiveness/CPU | DBD-PARAM-111 |

**Status:** ✅ RESOLVED (defined in plan, verify against SPEC016 during implementation)

---

## Document Status

**Phase 2 Complete:** ✅ YES
**Issues Identified:** 8 (0 critical, 2 high unresolved → resolved, 4 medium, 1 low)
**Blocking Issues:** NONE
**Ready for Phase 3:** ✅ YES

**User Approval Required:** Review error handling policy and validation ranges before Phase 3
