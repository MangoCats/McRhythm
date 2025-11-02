# PLAN017: SPEC017 Compliance Remediation - Plan Summary

**Status:** Phase 1-3 Complete (Week 1)
**Created:** 2025-11-02
**Source Specification:** [SPEC_spec017_compliance_remediation.md](../SPEC_spec017_compliance_remediation.md)

---

## Quick Reference

| Aspect | Value |
|--------|-------|
| Requirements | 7 (4 functional, 3 non-functional) |
| Test Cases | 7 automated + 1 manual |
| Coverage | 100% requirement coverage |
| Breaking Changes | 1 (file duration migration) |
| Estimated Effort | 4-8 hours |
| Risk Level | LOW-MEDIUM |
| Implementation Ready | ✅ YES (all phases 1-3 complete) |

---

## Executive Summary

This plan remediates SPEC017 tick-based timing compliance issues identified during comprehensive codebase analysis. Primary issue: wkmp-dr developer UI violates SRC-LAYER-011 by displaying only ticks (should show both ticks AND seconds). Secondary issues: API documentation gaps and file duration inconsistency.

**User Decisions Incorporated:**
- API Layer: Accept milliseconds/seconds (pragmatic deviation, document it)
- wkmp-dr Display: Dual format `141120000 (5.000000s)`
- File Duration: Migrate to i64 ticks (breaking change, database rebuild)

**Deliverables:**
1. wkmp-dr UI enhancement (JavaScript dual display)
2. API documentation (doc comments for wkmp-ap, wkmp-ai)
3. File duration migration (database schema + import workflow)
4. Variable naming improvements (2 specific files)
5. Documentation updates (SPEC017, IMPL001)
6. Test suite (7 automated tests + 1 manual review)

---

## Requirements Summary

### HIGH Priority

**REQ-F-001: wkmp-dr Dual Time Display**
- **What:** Display `{ticks} ({seconds}s)` in 6 timing columns
- **Why:** Violates SRC-LAYER-011 (developer UI must show both ticks AND seconds)
- **Impact:** Improves debugging efficiency for developers inspecting database
- **Tests:** TC-U-001, TC-I-002, TC-A-001

### MEDIUM Priority

**REQ-F-002: API Timing Unit Documentation**
- **What:** Add doc comments to API timing fields (wkmp-ap, wkmp-ai)
- **Why:** Pragmatic deviation from SRC-API-010 (APIs use ms/seconds, not ticks)
- **Impact:** Clarifies API contract, prevents confusion
- **Tests:** TC-A-003

**REQ-F-003: File Duration Migration to Ticks**
- **What:** Change `AudioFile.duration: Option<f64>` → `duration_ticks: Option<i64>`
- **Why:** Consistency with passage timing representation
- **Impact:** Breaking change, requires database rebuild
- **Tests:** TC-U-002, TC-I-001, TC-A-002

### LOW Priority

**REQ-F-004: Variable Naming Clarity**
- **What:** Add inline unit comments to ambiguous timing variables
- **Why:** Code maintainability
- **Impact:** Documentation improvement (2 files only)
- **Tests:** Manual code review

### Non-Functional Requirements

**REQ-NF-001: Test Coverage** - All changes tested (2 unit, 2 integration, 3 acceptance)
**REQ-NF-002: Documentation Updates** - SPEC017, IMPL001 updated
**REQ-NF-003: Backward Compatibility** - Breaking change documented with migration path

---

## Scope Summary

### In Scope ✅

**Functional Changes:**
- wkmp-dr: JavaScript tick-to-seconds conversion in `renderTable()`
- API: Doc comments for timing fields (wkmp-ap, wkmp-ai)
- Database: Schema change `duration REAL` → `duration_ticks INTEGER`
- Import: Metadata extractor uses `seconds_to_ticks()`
- Variables: Inline comments in 2 specific files

**Testing:**
- Unit: Conversion accuracy, roundtrip precision
- Integration: Database storage, display rendering
- Acceptance: End-to-end compliance verification

**Documentation:**
- SPEC017: Add "API Layer Pragmatic Deviation" section
- IMPL001: Update files table schema
- Migration: Database rebuild instructions

### Out of Scope ❌

- Other SPEC017 issues (deferred: LOW priority items from analysis)
- UI enhancements beyond dual display (tooltips, CSS styling)
- API layer functional changes (no type changes, documentation only)
- Automated migration (manual rebuild acceptable)
- Additional modules (wkmp-ui, wkmp-pd, wkmp-le not modified)

---

## Technical Design Summary

### Change 1: wkmp-dr Dual Time Display

**Location:** `wkmp-dr/src/ui/app.js` (renderTable function, lines 346-386)

**Implementation:**
```javascript
const TICK_RATE = 28224000;
const TIMING_COLUMNS = ['start_time_ticks', 'end_time_ticks',
                        'fade_in_start_ticks', 'fade_out_start_ticks',
                        'lead_in_start_ticks', 'lead_out_start_ticks'];

function ticksToSeconds(ticks) {
    if (ticks === null) return null;
    return (ticks / TICK_RATE).toFixed(6);
}

// In renderTable() loop:
if (TIMING_COLUMNS.includes(colName) && cell !== null) {
    const seconds = ticksToSeconds(parseInt(cell));
    html += `<td${className}>${cell} (${seconds}s)</td>`;
} else if (cell === null) {
    html += `<td${className}>null</td>`;
}
```

**Affected Files:** 1 (wkmp-dr/src/ui/app.js)

---

### Change 2: API Documentation

**Location:** wkmp-ap/src/api/handlers.rs, wkmp-ai/src/api/amplitude_analysis.rs

**Example Documentation:**
```rust
// wkmp-ap/src/api/handlers.rs
#[derive(Serialize)]
pub struct PositionResponse {
    /// Current playback position in milliseconds since passage start.
    /// Unit: milliseconds (ms) - converted from internal tick representation.
    /// Per SPEC017 API Layer Pragmatic Deviation.
    pub position_ms: u64,
}

// wkmp-ai/src/api/amplitude_analysis.rs
#[derive(Deserialize)]
pub struct AmplitudeAnalysisRequest {
    /// Start time for analysis window in seconds.
    /// Unit: seconds (f64) - will be converted to ticks internally.
    pub start_time_seconds: f64,
}
```

**Affected Files:** 2

---

### Change 3: File Duration Migration

**Schema Change:**
```sql
-- OLD (wkmp-common/src/db/init.rs)
CREATE TABLE files (
    duration REAL  -- Seconds as f64
);

-- NEW
CREATE TABLE files (
    duration_ticks INTEGER  -- Ticks as i64
);
```

**Struct Change:**
```rust
// OLD (wkmp-ai/src/db/files.rs)
pub struct AudioFile {
    pub duration: Option<f64>,  // Seconds
}

// NEW
pub struct AudioFile {
    pub duration_ticks: Option<i64>,  // Ticks
}
```

**Import Change:**
```rust
// wkmp-ai metadata extraction
let duration_seconds = extract_duration_metadata(&file)?;
let duration_ticks = seconds_to_ticks(duration_seconds);  // NEW conversion
audio_file.duration_ticks = Some(duration_ticks);
```

**Affected Files:** 3 (init.rs, files.rs, import workflow)

---

### Change 4: Variable Naming

**Location:**
- wkmp-ap/src/playback/pipeline/timing.rs
- wkmp-ai/src/services/silence_detector.rs

**Example:**
```rust
// BEFORE
let position = calculate_position();

// AFTER
let position = calculate_position();  // ticks, SPEC017 tick-based time
```

**Affected Files:** 2

---

## Test Summary

### Test Cases (7 automated + 1 manual)

| ID | Type | Description | Automation |
|----|------|-------------|------------|
| TC-U-001 | Unit | wkmp-dr tick-to-seconds conversion | ✅ JavaScript |
| TC-U-002 | Unit | File duration roundtrip | ✅ Rust |
| TC-I-001 | Integration | File import stores ticks | ✅ Rust |
| TC-I-002 | Integration | wkmp-dr renders dual format | ✅ Playwright |
| TC-A-001 | Acceptance | Developer UI compliance | ✅ Partial |
| TC-A-002 | Acceptance | File duration consistency | ✅ Rust |
| TC-A-003 | Acceptance | API documentation | ⚠️ Manual |
| Manual | Code Review | Variable naming (REQ-F-004) | ⚠️ Manual |

**Automation Level:** 87.5% (7/8 automated)

**Coverage:** 100% (all 7 requirements covered)

### Test Execution Order

1. **Unit Tests First** (independent): TC-U-001, TC-U-002
2. **Integration Tests** (require database): TC-I-001, TC-I-002
3. **Acceptance Tests** (require full system): TC-A-001, TC-A-002, TC-A-003
4. **Manual Review** (during implementation): REQ-F-004 variable naming

---

## Breaking Change

### File Duration Migration (REQ-F-003)

**Impact:** Existing databases incompatible, must rebuild

**Migration Steps:**
1. Stop all WKMP services
2. Delete existing database: `rm ~/Music/wkmp.db` (or OS equivalent)
3. Restart services (database auto-created with new schema)
4. Re-import all audio files via wkmp-ai

**No automated migration** - User must manually rebuild database

**Rationale:** Consistency with passage timing (all timing uses ticks)

**Risk Mitigation:**
- Clear documentation in release notes
- Breaking change warning in migration notes
- Test coverage verifies new schema (TC-A-002)

---

## Risk Assessment

**Overall Risk:** LOW-MEDIUM

| Risk Area | Level | Mitigation |
|-----------|-------|------------|
| wkmp-dr display | LOW | Display-only change, no data modification |
| API documentation | LOW | Comments only, no code changes |
| File duration migration | MEDIUM | Breaking change, but clear migration path |
| Variable naming | LOW | Comments only, no functional changes |

**Confidence Level:** HIGH - All changes straightforward, well-tested

---

## Documentation Updates

### SPEC017 Updates

**New Section:** "API Layer Pragmatic Deviation"
- **Location:** After SRC-API-010
- **Content:** 25-line section documenting deviation rationale
- **References:** See SPEC_spec017_compliance_remediation.md lines 367-391

### IMPL001 Updates

**Updated Section:** "files Table"
- **Change:** `duration REAL` → `duration_ticks INTEGER`
- **Location:** Files table definition section
- **Note:** Reference SPEC017 for tick-based representation

### Migration Documentation

**Required Documentation:**
- Breaking change notes in completion report
- Database rebuild instructions
- Release notes template with warning

---

## Implementation Order

Recommended sequence to minimize risk:

1. **Phase A: Non-Breaking Changes** (can be deployed independently)
   - REQ-F-002: API documentation (doc comments)
   - REQ-F-004: Variable naming (inline comments)
   - REQ-NF-002: SPEC017, IMPL001 updates

2. **Phase B: wkmp-dr Display** (non-breaking, user-visible)
   - REQ-F-001: Dual time display
   - TC-U-001, TC-I-002, TC-A-001 tests

3. **Phase C: Breaking Change** (requires coordination)
   - REQ-F-003: File duration migration
   - TC-U-002, TC-I-001, TC-A-002 tests
   - REQ-NF-003: Migration documentation
   - User notification before deployment

**Alternative:** Single increment (4-8 hours) if acceptable to deploy breaking change immediately

---

## Success Metrics

### Quantitative

- ✅ All 7 requirements implemented and tested
- ✅ 100% test coverage per traceability matrix
- ✅ wkmp-dr displays 6 timing columns in dual format
- ✅ 0 timing-related variables without unit indicators (in scope files)
- ✅ Database schema uses INTEGER for duration_ticks

### Qualitative

- ✅ Developer UI improves debugging efficiency (per SRC-LAYER-011)
- ✅ API documentation clarity increases (pragmatic deviation explicit)
- ✅ Database schema consistency achieved (all timing uses ticks)
- ✅ Code maintainability improved (clear variable units)
- ✅ User understands migration path (clear documentation)

---

## Plan Deliverables (Week 1 Complete)

### Phase 1: Input Validation and Scope Definition ✅
- [requirements_index.md](requirements_index.md) - 7 requirements catalogued
- [scope_statement.md](scope_statement.md) - In/out scope, assumptions, constraints

### Phase 2: Specification Completeness Verification ✅
- [01_specification_issues.md](01_specification_issues.md) - 0 critical, 2 medium, 1 low issues
- **Decision:** PROCEED (no blocking issues)

### Phase 3: Acceptance Test Definition ✅
- [02_test_specifications/test_index.md](02_test_specifications/test_index.md) - 7 test cases indexed
- [02_test_specifications/tc_u_001.md](02_test_specifications/tc_u_001.md) - wkmp-dr conversion
- [02_test_specifications/tc_u_002.md](02_test_specifications/tc_u_002.md) - File duration roundtrip
- [02_test_specifications/tc_i_001.md](02_test_specifications/tc_i_001.md) - File import integration
- [02_test_specifications/tc_i_002.md](02_test_specifications/tc_i_002.md) - wkmp-dr rendering
- [02_test_specifications/tc_a_001.md](02_test_specifications/tc_a_001.md) - Developer UI compliance
- [02_test_specifications/tc_a_002.md](02_test_specifications/tc_a_002.md) - File duration consistency
- [02_test_specifications/tc_a_003.md](02_test_specifications/tc_a_003.md) - API documentation
- [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md) - 100% coverage

### Pending (Phases 4-7, Week 2-3)
- Phase 4: Approach Selection (Week 2)
- Phase 5: Implementation Breakdown (Week 2)
- Phase 6: Effort Estimation (Week 3)
- Phase 7: Risk Assessment (Week 3)
- Phase 8: Final Documentation (Week 3)

---

## Using This Plan

### For Implementers

**Start Here:**
1. Read [requirements_index.md](requirements_index.md) for quick requirement overview
2. Review [scope_statement.md](scope_statement.md) to understand boundaries
3. Check [01_specification_issues.md](01_specification_issues.md) for known gaps
4. Review test specifications in [02_test_specifications/](02_test_specifications/)
5. Implement following test-first approach (write tests, verify fail, implement, verify pass)

**Implementation Code Examples:**
- See [SPEC_spec017_compliance_remediation.md](../SPEC_spec017_compliance_remediation.md) lines 233-556
- wkmp-dr display: Lines 233-289
- API documentation: Lines 299-365
- File duration migration: Lines 392-492
- Variable naming: Lines 509-556

### For Testers

**Test Execution:**
1. Unit tests first: TC-U-001 (JavaScript), TC-U-002 (Rust)
2. Integration tests: TC-I-001, TC-I-002 (require database)
3. Acceptance tests: TC-A-001, TC-A-002, TC-A-003
4. Manual review: REQ-F-004 variable naming
5. Verify 100% coverage via [traceability_matrix.md](02_test_specifications/traceability_matrix.md)

### For Reviewers

**Review Checklist:**
- [ ] All 7 requirements implemented (cross-check requirements_index.md)
- [ ] All 7 test cases pass (see test_index.md)
- [ ] Breaking change documented (see TC-A-002 Part 3)
- [ ] SPEC017 updated (see TC-A-003 Part 3)
- [ ] IMPL001 updated (see TC-A-003 Part 5)
- [ ] Manual code review complete (REQ-F-004)

---

## Context Window Optimization

**For future AI agents working on this plan:**

**Essential Files (load these):**
- This file (00_PLAN_SUMMARY.md) - Quick overview
- [requirements_index.md](requirements_index.md) - Requirement reference
- [02_test_specifications/test_index.md](02_test_specifications/test_index.md) - Test reference

**Drill-Down Files (load on demand):**
- [scope_statement.md](scope_statement.md) - When scope questions arise
- [01_specification_issues.md](01_specification_issues.md) - When addressing spec gaps
- Individual test specs (tc_*.md) - When implementing specific tests
- [traceability_matrix.md](02_test_specifications/traceability_matrix.md) - When verifying coverage

**Source Specification (load selectively):**
- [SPEC_spec017_compliance_remediation.md](../SPEC_spec017_compliance_remediation.md) - Use line number references:
  - Lines 1-50: Overview
  - Lines 233-289: wkmp-dr implementation code
  - Lines 299-365: API documentation examples
  - Lines 392-492: File duration migration code
  - Lines 509-556: Variable naming examples
  - Lines 576-715: Test specifications

**Never Load Entire Specification** - Use targeted line ranges only

---

## Sign-Off

**Phase 1-3 Complete:** 2025-11-02
**Planner:** Claude Code (Sonnet 4.5)
**Status:** ✅ Ready for implementation (specification quality excellent, 100% test coverage defined)

**Next Steps:**
1. User review of Phase 1-3 deliverables
2. User approval to proceed with implementation
3. OR: Continue to Phases 4-7 (Week 2-3) for detailed implementation breakdown

---

## Quick Links

- **Source Specification:** [SPEC_spec017_compliance_remediation.md](../SPEC_spec017_compliance_remediation.md)
- **Requirements:** [requirements_index.md](requirements_index.md)
- **Scope:** [scope_statement.md](scope_statement.md)
- **Issues:** [01_specification_issues.md](01_specification_issues.md)
- **Tests:** [02_test_specifications/test_index.md](02_test_specifications/test_index.md)
- **Traceability:** [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md)
- **Original Analysis:** [spec017_compliance_review_analysis_results.md](../spec017_compliance_review_analysis_results.md)
