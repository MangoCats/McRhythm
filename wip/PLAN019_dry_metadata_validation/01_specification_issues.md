# PLAN019: Specification Issues Report

**Purpose:** Document all specification issues discovered during Phase 2 verification
**Date:** 2025-11-03
**Status:** All HIGH issues resolved, MEDIUM issues documented

---

## Executive Summary

**Total Issues:** 5 (0 Critical, 3 High, 2 Medium, 0 Low)
**Decision:** ✅ PROCEED - No blockers, all HIGH issues resolved during test definition

**Issue Breakdown:**
- **CRITICAL:** 0 - No fundamental problems blocking implementation
- **HIGH:** 3 - All resolved with clear decisions during Phase 3
- **MEDIUM:** 2 - Documented as assumptions, no action required
- **LOW:** 0 - No minor issues

---

## CRITICAL Issues

**None identified** - No blockers to implementation.

---

## HIGH Issues (All Resolved)

### HIGH-001: Error Message Format Not Standardized

**Requirement:** REQ-DRY-030 (Validation closures)
**Category:** Ambiguity
**Severity:** High

**Issue Description:**
Validators return `String` errors, but format not specified. Without standardization, error messages may be inconsistent, confusing users and developers.

**Examples of Inconsistency:**
```
"Invalid value"                    // No parameter name or reason
"Out of range"                     // Which parameter? What range?
"volume_level must be 0.0-1.0"     // Format differs per validator
"Value 2.0 not in range [0.0, 1.0]" // Different format again
```

**Impact:**
- User confusion (unclear what failed)
- Difficult debugging (which validator failed?)
- Inconsistent API error responses

**Resolution:** ✅ **Standardized during Phase 3 test definition**

**Adopted Format:**
```
"{param_name}: {specific_reason}"
```

**Examples:**
```
"volume_level: value 2.0 out of range [0.0, 1.0]"
"working_sample_rate: must be one of: 44100, 48000, 88200, 96000"
"audio_buffer_size: invalid number format"
```

**Benefits:**
- Clear identification of failing parameter
- Consistent format across all validators
- Easy to parse for API error reporting

**Implementation Note:**
All 15 validators must follow this format. Test TC-U-030-01 verifies error messages include parameter name.

---

### HIGH-002: Multiple Validation Failures Handling Undefined

**Requirement:** REQ-DRY-060 (API validation)
**Category:** Ambiguity
**Severity:** High

**Issue Description:**
API `bulk_update_settings()` may receive multiple settings, some valid, some invalid. Behavior with multiple errors not specified:
- **Option A:** Fail-fast (return first error only)
- **Option B:** Batch validation (collect all errors, return list)

**User Experience Comparison:**

**Option A - Fail-Fast:**
```
POST /settings/bulk_update
{
  "volume_level": "2.0",     // Invalid
  "audio_buffer_size": "100000", // Invalid
  "working_sample_rate": "48000"  // Valid
}

Response (400):
{
  "status": "Validation failed: volume_level: value 2.0 out of range [0.0, 1.0]"
}

// User fixes volume_level, resubmits
// Response (400):
// {
//   "status": "Validation failed: audio_buffer_size: value 100000 out of range [512, 8192]"
// }

// User must iterate multiple times to discover all errors
```

**Option B - Batch Validation:**
```
POST /settings/bulk_update
{
  "volume_level": "2.0",     // Invalid
  "audio_buffer_size": "100000", // Invalid
  "working_sample_rate": "48000"  // Valid
}

Response (400):
{
  "status": "Validation failed: volume_level: value 2.0 out of range [0.0, 1.0], audio_buffer_size: value 100000 out of range [512, 8192]"
}

// User sees all errors at once, fixes both in one attempt
```

**Impact:**
- **Option A:** Poor UX (requires multiple round trips)
- **Option B:** Good UX (see all errors at once)

**Resolution:** ✅ **Option B - Batch Validation**

**Rationale:**
- Better user experience (fewer round trips)
- Standard practice in form validation
- Minimal implementation overhead (collect errors in Vec)

**Implementation Pattern:**
```rust
let mut errors = Vec::new();

for (key, value) in &req.settings {
    if let Some(meta) = metadata_map.get(key.as_str()) {
        if let Err(e) = (meta.validator)(value) {
            errors.push(format!("{}: {}", key, e)); // Collect all
        }
    }
}

if !errors.is_empty() {
    return Err((
        StatusCode::BAD_REQUEST,
        Json(StatusResponse {
            status: format!("Validation failed: {}", errors.join(", ")),
        }),
    ));
}
```

**Test Coverage:**
- TC-I-060-01: Single error validation
- TC-I-060-02: Batch error validation (3 errors)

---

### HIGH-003: Documentation Placement Not Specified

**Requirement:** REQ-DRY-100 (Documentation)
**Category:** Ambiguity
**Severity:** High

**Issue Description:**
"Document metadata pattern" is vague. Where should documentation be placed? Risk of incomplete or scattered documentation.

**Possible Locations:**
1. Module-level (`params.rs` header)
2. Struct-level (`ParamMetadata` struct)
3. Function-level (each validator)
4. API handler (`bulk_update_settings()` comments)
5. Separate markdown document
6. All of the above?

**Impact:**
- Incomplete documentation (some locations skipped)
- Redundant documentation (same info repeated)
- Future developers don't understand pattern

**Resolution:** ✅ **3 Strategic Locations**

**1. Module-Level Documentation** (`wkmp-common/src/params.rs`):
- **Purpose:** Explain overall pattern and philosophy
- **Content:**
  - What is metadata-based validation
  - Why this approach (DRY benefits)
  - Where metadata is used (init_from_database, setters, API)
  - Example of accessing metadata
- **Audience:** Developers maintaining or extending GlobalParams

**2. Struct-Level Documentation** (`ParamMetadata` struct):
- **Purpose:** Explain structure and usage
- **Content:**
  - Field descriptions (all 6 fields)
  - Validator closure signature and behavior
  - Example ParamMetadata instance
- **Audience:** Developers adding new parameters

**3. API Handler Comments** (`wkmp-ap/src/api/handlers.rs`):
- **Purpose:** Explain validation flow in API context
- **Content:**
  - How validation uses metadata
  - Step-by-step validation flow
  - Example error response
- **Audience:** Developers maintaining API or debugging validation

**NOT Included:**
- ❌ Function-level docs for each validator (too verbose, self-explanatory)
- ❌ Separate markdown document (overkill for internal pattern)

**Test Coverage:**
- TC-M-100-01: Manual review verifies all 3 locations documented

---

## MEDIUM Issues (Documented as Assumptions)

### MEDIUM-001: Type-to-String Conversion Approach

**Requirement:** REQ-DRY-050 (Refactor setters)
**Category:** Implementation Detail
**Severity:** Medium

**Issue Description:**
Setter methods receive typed values (f32, u32, u64, usize, f64), but metadata validators expect `&str`. How to convert?

**Options:**
1. **Use `.to_string()`** - Simple, standard Rust
2. **Use `format!("{}", value)`** - Equivalent to .to_string()
3. **Custom formatting** (e.g., `format!("{:.6}", f64_value)`) - Control precision

**Edge Cases:**
- `f64::INFINITY` → `.to_string()` = `"inf"`
- `f64::NAN` → `.to_string()` = `"NaN"`
- Very large/small floats → Scientific notation

**Impact:**
Low - Validators handle parse errors gracefully (return Err, use default)

**Chosen Approach:** ✅ **Use `.to_string()`**

**Rationale:**
- Simplest approach
- Standard Rust practice
- Edge cases handled by validators (parse errors detected)
- Consistent behavior across all types

**Edge Case Handling:**
```rust
// Setter receives f64::INFINITY
let value = f64::INFINITY;
let value_str = value.to_string(); // "inf"

// Validator parses string
let parsed: f64 = value_str.parse(); // Ok(inf)

// Validator checks range
if parsed < 0.5 || parsed > 0.99 {
    return Err(...); // INFINITY fails range check
}
```

**Result:** Edge cases correctly rejected by range validation, not special-cased.

**Test Coverage:**
TC-U-030-02 includes edge case tests for validators (parse errors, out-of-range).

---

### MEDIUM-002: Test Count Not Quantified

**Requirement:** REQ-DRY-090 (Test coverage)
**Category:** Incomplete Specification
**Severity:** Medium

**Issue Description:**
"Add tests for new validation paths" is vague. How many tests? Which scenarios? Without quantification, unclear when test coverage is sufficient.

**Impact:**
- Risk of insufficient test coverage
- Unclear acceptance criteria
- Difficult to assess completeness

**Resolution:** ✅ **10 New Tests Defined**

**Test Breakdown:**

**Metadata Infrastructure (5 tests):**
1. TC-U-010-01: ParamMetadata struct definition
2. TC-U-010-02: All 15 parameters in metadata
3. TC-U-020-01: metadata() accessor returns static reference
4. TC-U-030-01: Volume level validator (example validator test)
5. TC-U-030-02: All 15 validators tested (batch test)

**Integration Tests (5 tests):**
6. TC-I-040-01: Database loading uses metadata validators
7. TC-U-050-01: Setter methods delegate to validators
8. TC-I-060-01: API rejects invalid settings (single error)
9. TC-I-060-02: API batch error reporting (multiple errors)
10. TC-I-070-01: Database integrity after failed validation
11. TC-I-080-01: Volume functions use metadata

**(Note: Actually 11 tests, updated count during Phase 3)**

**Acceptance Criteria:**
- All 24 existing tests pass (no regressions)
- All 11 new tests pass
- Total: 35/35 tests passing (100% coverage)

**Test Coverage:**
TC-U-090-01 and TC-U-090-02 verify all tests pass.

---

## LOW Issues

**None identified** - No minor issues during specification review.

---

## Summary Statistics

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | N/A |
| HIGH | 3 | All resolved |
| MEDIUM | 2 | Documented |
| LOW | 0 | N/A |
| **Total** | **5** | **Resolved** |

---

## Decision Rationale

**Why PROCEED with 3 HIGH issues?**

All HIGH issues were **ambiguities**, not **fundamental problems**:
- HIGH-001: Error format → Standardized in Phase 3
- HIGH-002: Batch validation → Decision made in Phase 3
- HIGH-003: Documentation → Locations defined in Phase 3

**No blockers identified:**
- Requirements are well-defined and testable
- Implementation approach is clear
- All issues have resolutions

**Risk Assessment:**
- **Risk of rework:** Low (ambiguities resolved early)
- **Risk of missed requirements:** Low (100% test coverage)
- **Risk of implementation failure:** Low (proven patterns)

**Recommendation:** ✅ Proceed to implementation with confidence

---

## Lessons Learned

**Process Improvements:**
1. **Error message formats should be specified upfront** in all validation requirements
2. **Batch vs. fail-fast validation** should be explicitly called out in API requirements
3. **Documentation locations** should be part of documentation requirements

**For Future Plans:**
- Add "Error Message Format" as standard requirement attribute
- Add "Error Handling Strategy" (batch vs. fail-fast) as API requirement attribute
- Define documentation checklist (module/struct/function levels) in template

---

## Appendix: Issue Discovery Process

**Phase 2 Completeness Check:**
- Reviewed all 10 requirements systematically
- Verified inputs, outputs, behavior, constraints, error cases
- 5 ambiguities identified during this process

**Phase 2 Ambiguity Check:**
- Applied "Could two engineers implement differently?" test
- All 5 issues identified as ambiguous

**Phase 3 Resolution:**
- All HIGH issues resolved during test definition phase
- Resolutions incorporated into test specifications
- MEDIUM issues documented as assumptions

**Result:** Clean specification ready for implementation
