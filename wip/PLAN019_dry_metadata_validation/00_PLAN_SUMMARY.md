# PLAN019: DRY Metadata Validation - PLAN SUMMARY

**Status:** Ready for Implementation
**Created:** 2025-11-03
**Specification Source:** Code review analysis (developer UI settings management)
**Plan Location:** `wip/PLAN019_dry_metadata_validation/`

---

## READ THIS FIRST

This plan implements three DRY (Don't Repeat Yourself) improvements to eliminate duplicated validation logic and metadata across the GlobalParams system. The plan addresses **~160 lines of duplication** across 3 files while improving maintainability and preventing database corruption from invalid settings.

**For Implementation:**
- Read this summary (executive overview)
- Review detailed requirements: `requirements_index.md`
- Review test specifications: `02_test_specifications/test_index.md`
- Follow traceability matrix: `02_test_specifications/traceability_matrix.md`

**Context Window Budget:** ~600-850 lines per implementation session

---

## Executive Summary

### Problem Being Solved

The WKMP GlobalParams system (15 database-backed parameters) has **critical DRY violations**:

1. **3-Way Metadata Duplication:**
   - Validation ranges defined in setter methods ([wkmp-common/src/params.rs:473-651](../../../wkmp-common/src/params.rs#L473-L651))
   - Same ranges hardcoded in API handler ([wkmp-ap/src/api/handlers.rs:1252-1289](../../../wkmp-ap/src/api/handlers.rs#L1252-L1289))
   - Defaults duplicated in database init ([wkmp-common/src/db/init.rs:198-213](../../../wkmp-common/src/db/init.rs#L198-L213))
   - **Problem:** Change validation range = edit 3 files, high risk of inconsistency

2. **No Server-Side Validation:**
   - `bulk_update_settings()` writes values directly to database ([wkmp-ap/src/api/handlers.rs:1346-1384](../../../wkmp-ap/src/api/handlers.rs#L1346-L1384))
   - Invalid values accepted, then silently ignored at startup
   - **Problem:** User sees "success" but value is rejected (bad UX)

3. **Duplicated Volume Validation:**
   - `.clamp(0.0, 1.0)` logic in `wkmp-ap/src/db/settings.rs` ([lines 15-35](../../../wkmp-ap/src/db/settings.rs#L15-L35))
   - Same validation in `GlobalParams::set_volume_level()` ([wkmp-common/src/params.rs:485-492](../../../wkmp-common/src/params.rs#L485-L492))
   - **Problem:** 2 places to update if volume range changes

### Solution Approach

**Centralize metadata and validation in single source of truth:**

1. Create `ParamMetadata` struct with validation closures (REQ-DRY-010/020/030)
2. Refactor database loading and setters to use metadata (REQ-DRY-040/050)
3. Add API validation using metadata (REQ-DRY-060/070) - **CRITICAL**
4. Refactor volume functions to use metadata (REQ-DRY-080)
5. Maintain 100% test coverage (REQ-DRY-090)

**Benefits:**
- ✅ Single source of truth for validation rules
- ✅ Prevents database corruption (invalid values rejected at API)
- ✅ Eliminates ~160 lines of duplication
- ✅ Easier maintenance (change validation = edit 1 place)

### Implementation Status

**Phases 1-3 Complete:**
- ✅ Phase 1: Scope Definition - 10 requirements extracted
- ✅ Phase 2: Specification Verification - 0 Critical, 3 High (all resolved), 2 Medium
- ✅ Phase 3: Test Definition - 13 tests defined, 100% coverage

**Phases 4-8 Status:** Not applicable (small refactoring, direct implementation)

---

## Requirements Summary

**Total Requirements:** 10 (3 Critical/High functional, 4 High refactoring, 3 Medium)

| Req ID | Priority | Brief Description | Impact |
|--------|----------|-------------------|--------|
| REQ-DRY-010 | High | Create ParamMetadata struct (15 parameters) | +150 LOC |
| REQ-DRY-020 | High | Implement GlobalParams::metadata() accessor | +20 LOC |
| REQ-DRY-030 | High | Add validation closures to metadata | Incl. in 010 |
| REQ-DRY-040 | High | Refactor init_from_database() to use metadata | ~0 (replace) |
| REQ-DRY-050 | High | Refactor 15 setters to delegate to validators | ~-80 LOC |
| REQ-DRY-060 | **Critical** | Add validation to bulk_update_settings() API | +30 LOC |
| REQ-DRY-070 | **Critical** | Prevent invalid database writes | Part of 060 |
| REQ-DRY-080 | Medium | Refactor volume functions to use metadata | ~-10 LOC |
| REQ-DRY-090 | High | Maintain 100% test coverage (24 existing + 10 new) | +50 LOC tests |
| REQ-DRY-100 | Medium | Document metadata pattern | +30 LOC docs |

**Full Requirements:** See `requirements_index.md`

---

## Scope

### ✅ In Scope

**Primary Goals:**
1. Create centralized parameter metadata system (`wkmp-common/src/params.rs`)
2. Refactor database loading to use metadata validators (eliminate duplication)
3. Refactor 15 setter methods to delegate to metadata validators (DRY)
4. **Add server-side validation to `bulk_update_settings()`** (CRITICAL - prevent corruption)
5. Refactor volume functions (`wkmp-ap/src/db/settings.rs`)
6. Maintain all 24 existing tests + add 10 new tests

**Files Modified:**
- `wkmp-common/src/params.rs` (major refactoring, +170 LOC, -80 LOC)
- `wkmp-ap/src/api/handlers.rs` (add validation, +30 LOC)
- `wkmp-ap/src/db/settings.rs` (refactor volume, -10 LOC)

**Net Effect:**
- **Eliminate:** ~160 lines of duplicated validation/metadata
- **Add:** ~200 lines of centralized metadata + validation
- **Net:** +40 lines, **3 sources of truth → 1 source of truth**

### ❌ Out of Scope

**Explicitly NOT included:**
1. Modifying other settings functions (audio_sink, crossfade defaults) - volume only as proof-of-concept
2. Creating UI validation (frontend validation unchanged)
3. Changing database schema (settings table unchanged)
4. Adding new parameters (only 15 existing GlobalParams)
5. Performance optimization (focus is correctness and maintainability)

**Full Scope:** See `scope_statement.md`

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0 (no blockers)
- **HIGH Issues:** 3 (all resolved during test definition)
  - HIGH-001: Error message format standardized (`"{param}: {reason}"`)
  - HIGH-002: Multiple validation failures handling defined (collect all errors)
  - HIGH-003: Documentation placement specified (module + struct + API example)
- **MEDIUM Issues:** 2 (documented as assumptions)
  - MEDIUM-001: Type-to-string conversion approach (use `.to_string()`)
  - MEDIUM-002: Test count quantified (10 new tests)

**Decision:** ✅ PROCEED - No blockers, all issues resolved

**Full Analysis:** See `01_specification_issues.md`

---

## Implementation Roadmap

### Increment 1: Centralized Metadata Infrastructure (REQ-DRY-010/020/030)
**Objective:** Create ParamMetadata struct with all 15 parameter definitions and validation closures
**Effort:** 1.5-2 hours
**Deliverables:**
- `ParamMetadata` struct with 6 fields (key, data_type, default_value, description, validation_range, validator)
- `GlobalParams::metadata()` static accessor returning `&'static [ParamMetadata; 15]`
- 15 validation closures (one per parameter)
**Tests:** TC-U-010-01, TC-U-010-02, TC-U-020-01, TC-U-030-01, TC-U-030-02 (5 tests)
**Success Criteria:**
- Metadata array has exactly 15 entries
- All validators accept their default values
- All validators reject out-of-range values

---

### Increment 2: Refactor Database Loading (REQ-DRY-040)
**Objective:** Refactor `init_from_database()` to use metadata validators instead of duplicated logic
**Effort:** 1-1.5 hours
**Deliverables:**
- Simplified `init_from_database()` (eliminate ~80 lines of duplicated validation)
- Uses metadata validators for all 15 parameters
**Tests:** TC-I-040-01 (1 test)
**Success Criteria:**
- All 24 existing tests pass
- Invalid database values rejected via metadata validators

---

### Increment 3: Refactor Setter Methods (REQ-DRY-050)
**Objective:** Refactor 15 setter methods to delegate to metadata validators
**Effort:** 1.5-2 hours
**Deliverables:**
- Simplified setters (eliminate duplicated range checks)
- Delegate to metadata validators
**Tests:** TC-U-050-01 (1 test)
**Success Criteria:**
- All setters use metadata validators (no duplicated logic)
- Error messages match metadata validator format

---

### Increment 4: Add API Validation (REQ-DRY-060/070) **CRITICAL**
**Objective:** Add server-side validation to `bulk_update_settings()` to prevent database corruption
**Effort:** 1-1.5 hours
**Deliverables:**
- Validation logic in `bulk_update_settings()` handler
- Batch error reporting (collect all errors)
- 400 Bad Request on validation failure
**Tests:** TC-I-060-01, TC-I-060-02, TC-I-070-01 (3 tests)
**Success Criteria:**
- Invalid values rejected with clear error messages
- Database unchanged after validation failure (no partial writes)
- User sees immediate feedback (not silent failure)

---

### Increment 5: Refactor Volume Functions (REQ-DRY-080)
**Objective:** Refactor `get_volume/set_volume` to use metadata validators (remove `.clamp()` duplication)
**Effort:** 0.5-1 hour
**Deliverables:**
- Simplified `get_volume/set_volume` using metadata validators
- Remove duplicated `.clamp(0.0, 1.0)` logic
**Tests:** TC-I-080-01 (1 test)
**Success Criteria:**
- Volume validation via metadata (no hardcoded clamping)
- Existing volume tests pass

---

### Increment 6: Documentation and Final Verification (REQ-DRY-090/100)
**Objective:** Document metadata pattern and verify all tests pass
**Effort:** 0.5-1 hour
**Deliverables:**
- Module-level documentation (`params.rs`)
- Struct-level documentation (`ParamMetadata`)
- API handler comments (`bulk_update_settings()`)
- Test verification report
**Tests:** TC-U-090-01, TC-U-090-02, TC-M-100-01 (3 tests)
**Success Criteria:**
- All 34 tests pass (24 existing + 10 new)
- Documentation complete in all 3 locations

---

**Total Estimated Effort:** 6-8 hours

---

## Test Coverage Summary

**Total Tests:** 13 (7 unit, 5 integration, 0 system, 1 manual)
**Coverage:** 100% - All 10 requirements have acceptance tests

**Test Breakdown:**
- **Unit Tests (7):** Struct validation, accessor, validators, setter delegation, test suite
- **Integration Tests (5):** Database loading, API validation, database integrity, volume refactor
- **Manual Tests (1):** Documentation review

**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`

---

## Risk Assessment

**Residual Risk:** **Low**

**Top Risks:**
1. **Test regressions** (24 existing tests fail after refactoring)
   - Mitigation: Incremental implementation, run tests after each increment
   - Residual Risk: Low (serial_test infrastructure proven)

2. **Type conversion edge cases** (f64::INFINITY, NaN not handled)
   - Mitigation: Validators detect invalid string formats, use defaults
   - Residual Risk: Low (existing error handling pattern)

3. **API breaking changes** (bulk_update_settings() behavior changes)
   - Mitigation: Returns 400 Bad Request (standard HTTP), backward compatible
   - Residual Risk: Low (improves correctness, no breaking changes)

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of `/plan` workflow for 7-step technical debt discovery process.

---

## Success Metrics

**Quantitative:**
- ✅ ~160 lines of duplication eliminated
- ✅ Single source of truth for validation (1 file vs. 3 files)
- ✅ 100% test coverage (34 tests: 24 existing + 10 new)
- ✅ 0 regressions (all existing tests pass)

**Qualitative:**
- ✅ Maintainability improved (change validation = edit 1 place)
- ✅ Database integrity guaranteed (no invalid values)
- ✅ User experience improved (immediate validation feedback)
- ✅ Developer experience improved (clear error messages)

---

## Dependencies

**Existing Documents (Read-Only):**
- CLAUDE.md (DRY principle, decision-making framework)
- GOV002 (requirements enumeration)
- IMPL002 (Rust coding conventions)

**Integration Points:**
- `wkmp-common/src/params.rs` (major refactoring)
- `wkmp-ap/src/api/handlers.rs` (add validation)
- `wkmp-ap/src/db/settings.rs` (refactor volume)

**No External Dependencies** (all required libraries already present)

---

## Constraints

**Technical:**
- Must maintain backward compatibility with existing database values
- Cannot break existing 24 tests
- Must use `Result<(), String>` for validation errors (consistent with existing)
- Validation closures: `fn(&str) -> Result<(), String>` (string input from database)

**Process:**
- Follow test-first approach (write tests before implementing)
- Use `#[serial_test::serial]` for database tests
- Maintain traceability (requirement → test → implementation)

**Timeline:**
- Estimated 6-8 hours total
- Can be completed in 2-3 sessions

---

## Next Steps

### Immediate (Ready Now)
1. Review this summary and requirements_index.md
2. Review test specifications (test_index.md)
3. Confirm approach acceptable
4. Begin Increment 1: Centralized Metadata Infrastructure

### Implementation Sequence
1. **Increment 1:** Create ParamMetadata struct + validators (1.5-2 hrs)
2. **Increment 2:** Refactor init_from_database() (1-1.5 hrs)
3. **Increment 3:** Refactor 15 setter methods (1.5-2 hrs)
4. **Increment 4:** Add API validation (**CRITICAL**, 1-1.5 hrs)
5. **Increment 5:** Refactor volume functions (0.5-1 hr)
6. **Increment 6:** Documentation + verification (0.5-1 hr)

### After Implementation
1. Execute Phase 9: Post-Implementation Review (MANDATORY)
2. Generate technical debt report
3. Run all 34 tests (verify 100% pass)
4. Verify traceability matrix complete
5. Create final implementation report
6. Archive plan using `/archive-plan PLAN019`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- `requirements_index.md` - All 10 requirements with priorities
- `scope_statement.md` - In/out scope, assumptions, constraints
- `01_specification_issues.md` - Phase 2 analysis (0 critical, 3 high resolved)

**Test Specifications:**
- `02_test_specifications/test_index.md` - All 13 tests quick reference
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping
- `02_test_specifications/tc_*.md` - Individual test specifications (detailed)

**For Implementation:**
- Read this summary (~500 lines)
- Read current increment specification (~150 lines)
- Read relevant test specs (~100-150 lines)
- **Total context:** ~650-800 lines per increment

---

## Plan Status

**Phase 1-3 Status:** ✅ Complete
**Phases 4-8 Status:** N/A (direct implementation, no phased approach needed)
**Current Status:** ✅ Ready for Implementation
**Estimated Timeline:** 6-8 hours over 2-3 sessions

---

## Approval and Sign-Off

**Plan Created:** 2025-11-03
**Plan Status:** Ready for Implementation Review

**Next Action:** User review and approval to begin Increment 1
