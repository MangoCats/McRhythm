# PLAN019: Scope Statement

**Purpose:** Define explicit boundaries for DRY metadata validation implementation
**Date:** 2025-11-03

---

## Executive Summary

This plan implements **centralized parameter metadata** to eliminate ~160 lines of duplication across 3 files while preventing database corruption from invalid settings. The scope is tightly bounded to 3 files and 15 existing parameters.

**Primary Goal:** Single source of truth for validation rules

**Secondary Goals:**
- Prevent invalid database writes (API validation)
- Improve maintainability (change = 1 place, not 3)
- Better user experience (immediate validation feedback)

---

## In Scope

### ✅ 1. Centralized Metadata System

**What:**
- Create `ParamMetadata` struct with 6 fields (key, data_type, default, description, range, validator)
- Implement `GlobalParams::metadata()` static accessor
- Define metadata for all 15 existing GlobalParams parameters

**Where:**
- `wkmp-common/src/params.rs` (single file)

**Why:**
- Eliminates 3-way duplication (setters, API handler, DB init)
- Single source of truth for validation rules

---

### ✅ 2. Database Loading Refactoring

**What:**
- Refactor `GlobalParams::init_from_database()` to use metadata validators
- Remove ~80 lines of duplicated validation logic

**Where:**
- `wkmp-common/src/params.rs` (lines 245-410)

**Why:**
- Eliminates duplication with setter methods
- Consistent validation across load and runtime

**Maintains:**
- Existing error handling (warn + default)
- Existing function signature
- All 24 existing tests pass

---

### ✅ 3. Setter Methods Refactoring

**What:**
- Refactor 15 setter methods to delegate to metadata validators
- Remove hardcoded range checks

**Where:**
- `wkmp-common/src/params.rs` (lines 473-651)

**Why:**
- Eliminates duplication with metadata validators
- Single place to update validation ranges

**Maintains:**
- Existing function signatures
- Existing return types (Result<(), String>)
- All setter tests pass

---

### ✅ 4. API Validation (CRITICAL)

**What:**
- Add server-side validation to `bulk_update_settings()` API handler
- Validate all settings BEFORE writing to database
- Batch error reporting (collect all errors)
- Return 400 Bad Request on validation failure

**Where:**
- `wkmp-ap/src/api/handlers.rs` (lines 1346-1384)

**Why:**
- **Prevents database corruption** (invalid values rejected)
- Better user experience (immediate feedback, not silent failure)
- Database integrity guaranteed

**Critical Behavior:**
- Invalid values → 400 status, NO database write
- Valid values → 200 status, write + graceful shutdown (existing)

---

### ✅ 5. Volume Functions Refactoring

**What:**
- Refactor `get_volume/set_volume` in `wkmp-ap/src/db/settings.rs`
- Replace `.clamp(0.0, 1.0)` with metadata validator lookup
- Remove duplicated validation logic

**Where:**
- `wkmp-ap/src/db/settings.rs` (lines 15-35)

**Why:**
- Eliminates duplication with `GlobalParams::set_volume_level()`
- Proof-of-concept for other settings functions (future work)

**Maintains:**
- Existing function signatures
- Existing tests pass

---

### ✅ 6. Test Coverage

**What:**
- Maintain all 24 existing tests (no regressions)
- Add 13 new tests for metadata system

**Where:**
- `wkmp-common/src/params.rs` tests module (unit tests)
- `wkmp-ap/tests/` or handlers tests (integration tests)

**Coverage:**
- All 10 requirements have acceptance tests (100%)
- Test suite verification (TC-U-090-01/02)

---

### ✅ 7. Documentation

**What:**
- Module-level documentation (params.rs header)
- Struct-level documentation (ParamMetadata)
- API handler comments (bulk_update_settings)

**Where:**
- 3 strategic locations (not scattered)

**Why:**
- Help future developers understand pattern
- Explain validation flow
- Provide usage examples

---

## Out of Scope

### ❌ 1. Other Settings Functions

**NOT Included:**
- `get_audio_sink/set_audio_sink` in `wkmp-ap/src/db/settings.rs`
- `get_crossfade_defaults` and related functions
- Other settings beyond volume

**Rationale:**
- Volume refactoring is **proof-of-concept**
- Other settings can be migrated in future work
- Keeps scope manageable (6-8 hours)

**Future Work:**
- If volume refactoring proves successful, migrate other settings
- Estimate: 1-2 hours per setting function group

---

### ❌ 2. Frontend/UI Validation

**NOT Included:**
- JavaScript validation in developer UI HTML
- Client-side validation in browser
- UI error message display improvements

**Rationale:**
- Scope is **backend validation only**
- UI improvements are separate concern
- Server-side validation sufficient for correctness

**Future Work:**
- Add client-side validation for better UX (optional)
- Estimate: 2-3 hours

---

### ❌ 3. Database Schema Changes

**NOT Included:**
- Modifying `settings` table structure
- Adding new columns (e.g., `validation_rule` column)
- Database migrations

**Rationale:**
- Current schema is adequate (key-value pairs as TEXT)
- Schema changes add complexity and risk
- Validation belongs in application layer, not database

**Explicitly Maintaining:**
- `settings (key TEXT PRIMARY KEY, value TEXT, updated_at TIMESTAMP)`

---

### ❌ 4. New Parameters

**NOT Included:**
- Adding 16th, 17th, etc. GlobalParams
- Expanding parameter set beyond current 15

**Rationale:**
- Scope is **refactoring existing parameters**
- New parameters are feature additions (separate plan)

**Future Work:**
- New parameters can use metadata system (pattern established)
- Each new parameter: add to metadata array, define validator

---

### ❌ 5. Performance Optimization

**NOT Included:**
- Optimizing validator execution time
- Caching metadata lookups
- Profiling validation overhead

**Rationale:**
- Validation overhead is negligible (string parsing + range check)
- Premature optimization (no evidence of bottleneck)
- Focus is **correctness and maintainability**, not performance

**Measured Baseline (estimated):**
- Validator execution: <1μs per parameter
- Batch validation (15 params): <15μs total
- API handler overhead: <0.1ms (acceptable)

---

### ❌ 6. Error Handling Strategy Changes

**NOT Included:**
- Changing warn + default pattern to fail-fast
- Adding retry logic for database errors
- Changing error message format beyond standardization

**Rationale:**
- Existing error handling is proven and safe
- Graceful degradation is design goal
- Scope is **DRY refactoring**, not error handling redesign

**Explicitly Maintaining:**
- Database errors → warn + default + continue
- Validation errors → warn + default + continue (init_from_database)
- API validation errors → 400 Bad Request (new behavior)

---

### ❌ 7. Configuration File Format

**NOT Included:**
- Adding parameter validation to TOML config
- Migrating settings from database to TOML
- Supporting multiple configuration sources

**Rationale:**
- Database-first configuration is design decision (ARCH-OD-010)
- TOML is bootstrap only (root folder path, logging)
- Scope is database validation, not config format redesign

---

### ❌ 8. Audit Logging

**NOT Included:**
- Logging all settings changes to audit trail
- Recording who changed what and when
- Settings change history in database

**Rationale:**
- Audit logging is separate concern (future requirement)
- Not required for correctness or DRY goals
- Adds complexity beyond scope

**Future Work:**
- Add audit logging if required (estimate: 3-4 hours)

---

## Assumptions

**Technical Assumptions:**
1. All 15 GlobalParams validation rules are **correct as currently implemented**
   - If validation rule is wrong, fix is still in metadata (single place)

2. Database-first configuration pattern is **stable** (no migration to TOML)
   - WKMP architecture decision (ARCH-OD-010)

3. Graceful degradation pattern (warn + default) is **correct error handling**
   - Proven safe in existing implementation

4. Test infrastructure (serial_test) is **working and sufficient**
   - 24 existing tests already use this pattern

5. User understands **trade-off: +40 LOC for DRY benefits**
   - More code to centralize metadata
   - Less duplication overall

**Process Assumptions:**
1. Implementation will follow **test-first approach**
   - Write tests, then implement to pass tests

2. Each increment will be **committed separately**
   - Enables bisecting if issues found

3. User will **review and approve** before starting
   - Confirmation that scope is acceptable

---

## Constraints

### Technical Constraints

**1. Backward Compatibility:**
- Must maintain existing database values (no migration required)
- Existing 24 tests must continue passing
- Existing function signatures unchanged (setters, init_from_database)

**2. Error Handling:**
- Must use `Result<(), String>` for validation errors (existing pattern)
- Validation closures: `fn(&str) -> Result<(), String>` (string input from DB)
- API errors: Standard HTTP status codes (400 Bad Request)

**3. Thread Safety:**
- Metadata must be `&'static` (accessed from multiple threads)
- Validator closures must be `Fn` (not `FnMut` - immutable)

**4. Type System:**
- Validators receive `&str` (from database TEXT column)
- Must parse to typed values (f32, u32, u64, usize, f64)

### Process Constraints

**1. Test-First:**
- Cannot mark increment complete without passing tests
- Must run full test suite after each increment
- No skipping test writing

**2. Serial Development:**
- One increment at a time (no parallelization)
- Complete increment before starting next
- Maintain traceability throughout

**3. Code Review:**
- All changes reviewed before committing
- Documentation reviewed for completeness

### Timeline Constraints

**1. Estimated Duration:**
- 6-8 hours total (over 2-3 sessions)
- No hard deadline (quality over speed)

**2. Increment Sizing:**
- Each increment: 0.5-2 hours
- Prevents context overload

---

## Dependencies

### Existing Code (Required)

**1. wkmp-common/src/params.rs:**
- ✅ Exists - GlobalParams struct with 15 fields
- ✅ Exists - 15 setter methods with validation
- ✅ Exists - init_from_database() with database loading
- ✅ Exists - 24 passing tests

**2. wkmp-ap/src/api/handlers.rs:**
- ✅ Exists - get_all_settings() with hardcoded metadata (lines 1232-1335)
- ✅ Exists - bulk_update_settings() without validation (lines 1346-1384)

**3. wkmp-ap/src/db/settings.rs:**
- ✅ Exists - get_volume/set_volume with .clamp() (lines 15-35)

### External Libraries (Required)

**1. Rust Standard Library:**
- ✅ Available - std::sync::RwLock (existing usage)
- ✅ Available - Result, Option types

**2. sqlx:**
- ✅ Available - Database access (workspace dependency)

**3. serde:**
- ✅ Available - Serialization for API responses (workspace dependency)

**4. serial_test:**
- ✅ Available - Serial test execution (dev-dependency)

**5. tokio:**
- ✅ Available - Async runtime (workspace dependency)

**No New Dependencies Required** - All libraries present

### Knowledge/Skills (Required)

**1. Rust Programming:**
- Closures and function pointers
- Static lifetime and thread safety
- Error handling with Result
- Test-driven development

**2. WKMP Architecture:**
- GlobalParams singleton pattern
- Database-first configuration
- Microservices communication

**3. Testing:**
- Unit testing with #[test]
- Integration testing with tokio::test
- Serial testing with serial_test

---

## Risk Assessment

### Identified Risks

**1. Test Regressions (Medium Risk)**
- **Probability:** Medium (refactoring touches existing code)
- **Impact:** High (24 tests must continue passing)
- **Mitigation:** Incremental implementation, run tests after each increment
- **Residual Risk:** Low (proven patterns, comprehensive tests)

**2. Type Conversion Edge Cases (Low Risk)**
- **Probability:** Low (edge cases rare: f64::INFINITY, NaN)
- **Impact:** Medium (incorrect validation)
- **Mitigation:** Validators detect invalid string formats, use defaults
- **Residual Risk:** Low (existing error handling pattern)

**3. API Breaking Changes (Low Risk)**
- **Probability:** Low (400 Bad Request is standard HTTP)
- **Impact:** Low (improves correctness, no existing consumers affected)
- **Mitigation:** Backward compatible (valid requests unchanged)
- **Residual Risk:** Low (improvement, not breaking change)

### Overall Risk: **Low**

---

## Success Metrics

### Quantitative Metrics

**1. Code Duplication Eliminated:**
- Target: ~160 lines of duplication removed
- Measure: Line count in git diff

**2. Single Source of Truth:**
- Before: Validation in 3 files (params.rs, handlers.rs, init.rs)
- After: Validation in 1 file (params.rs metadata)

**3. Test Coverage:**
- Target: 100% (37 tests: 24 existing + 13 new)
- Measure: `cargo test` output

**4. No Regressions:**
- Target: 0 test failures
- Measure: All 24 existing tests pass

### Qualitative Metrics

**1. Maintainability:**
- Changing validation range: Before 3 edits → After 1 edit
- Adding new parameter: Pattern established, metadata-driven

**2. Database Integrity:**
- Invalid values rejected at API layer (not silently ignored)
- User sees immediate feedback (not silent failure)

**3. Developer Experience:**
- Clear error messages (parameter name + reason)
- Consistent validation pattern across system

**4. Code Quality:**
- DRY principle applied
- Single responsibility (metadata owns validation)
- Testable and tested (100% coverage)

---

## Acceptance Criteria

**Plan is considered successfully complete when:**

✅ **All 10 requirements implemented**
- REQ-DRY-010 through REQ-DRY-100

✅ **All 37 tests passing**
- 24 existing tests (no regressions)
- 13 new tests (metadata system)

✅ **Traceability complete**
- 100% requirement → test mapping
- 100% test → requirement mapping

✅ **Files modified as planned**
- wkmp-common/src/params.rs (refactored)
- wkmp-ap/src/api/handlers.rs (validation added)
- wkmp-ap/src/db/settings.rs (volume refactored)

✅ **Documentation complete**
- Module-level (params.rs)
- Struct-level (ParamMetadata)
- API handler (bulk_update_settings)

✅ **Code reviewed**
- No obvious bugs or issues
- Follows Rust conventions
- Passes clippy lints

✅ **Phase 9 complete**
- Technical debt report generated
- Known issues documented
- Future work identified

---

## Out-of-Scope Future Work

**Potential future enhancements (NOT in this plan):**

1. **Migrate Other Settings Functions** (1-2 hours)
   - Refactor audio_sink, crossfade_defaults using metadata pattern

2. **Add Client-Side Validation** (2-3 hours)
   - JavaScript validation in developer UI
   - Real-time feedback in browser

3. **Add Audit Logging** (3-4 hours)
   - Log all settings changes to database
   - Track who changed what and when

4. **Performance Profiling** (1-2 hours)
   - Measure validation overhead
   - Optimize if bottleneck found (unlikely)

5. **Extend to Other Modules** (variable)
   - Apply metadata pattern to wkmp-ui, wkmp-pd settings
   - Estimate: 4-6 hours per module

---

## Scope Change Control

**If scope needs to change during implementation:**

1. **STOP implementation** at current increment
2. **Document proposed change** (what, why, impact)
3. **Request user approval** explicitly
4. **Update plan documents** (requirements, tests, scope)
5. **Resume implementation** only after approval

**No silent scope changes permitted** - all changes require explicit approval.

---

## Summary

**In Scope:** Centralized metadata system, refactor 3 files, API validation, 13 new tests, documentation
**Out of Scope:** Other settings, UI validation, schema changes, new parameters, performance
**Constraints:** Backward compatible, test-first, no new dependencies
**Success:** 100% test coverage, ~160 LOC duplication eliminated, single source of truth

**Status:** ✅ Scope clearly defined, ready for implementation
