# Scope Statement: SPEC017 Compliance Remediation

**Plan:** PLAN017
**Created:** 2025-11-02
**Source:** [SPEC_spec017_compliance_remediation.md](../SPEC_spec017_compliance_remediation.md)

---

## In Scope

### ✅ Functional Changes

1. **wkmp-dr UI Enhancement**
   - Add JavaScript tick-to-seconds conversion in `renderTable()`
   - Display format: `{ticks} ({seconds}s)` for all 6 timing columns
   - Applies to passages table only

2. **API Documentation**
   - Add doc comments to wkmp-ap API timing fields (`PositionResponse`, `SeekRequest`)
   - Add doc comments to wkmp-ai API timing fields (`AmplitudeAnalysisRequest`, `AmplitudeAnalysisResponse`)
   - Update SPEC017 with "API Layer Pragmatic Deviation" section

3. **File Duration Migration**
   - Change `AudioFile.duration: Option<f64>` → `duration_ticks: Option<i64>`
   - Update database schema: `duration REAL` → `duration_ticks INTEGER`
   - Update metadata extractor to convert via `seconds_to_ticks()`
   - Update all SQL queries referencing `duration`

4. **Variable Naming Improvements**
   - Add inline unit comments to ambiguous timing variables in:
     - `wkmp-ap/src/playback/pipeline/timing.rs`
     - `wkmp-ai/src/services/silence_detector.rs`

### ✅ Testing

1. **Unit Tests**
   - wkmp-dr: tick-to-seconds conversion accuracy
   - wkmp-ai: file duration roundtrip (seconds → ticks → seconds)

2. **Integration Tests**
   - wkmp-ai: file import with tick duration storage
   - wkmp-dr: display rendering with dual format

3. **Acceptance Tests**
   - End-to-end: passages table displays correctly
   - End-to-end: file duration stored as ticks
   - Manual: API documentation completeness review

### ✅ Documentation

1. **Specification Updates**
   - SPEC017: Add [SRC-API-040] section on pragmatic deviation
   - IMPL001: Update files table schema documentation

2. **Migration Documentation**
   - Database rebuild instructions
   - Breaking change notes in completion report

---

## Out of Scope

### ❌ Not Included in This Plan

1. **Other SPEC017 Issues**
   - wkmp-ai API standardization (uses seconds vs. milliseconds) - LOW priority, deferred
   - wkmp-ai silence detection f32 → f64 migration - LOW priority, deferred
   - Additional variable naming beyond specified files - Incremental improvement, not comprehensive

2. **UI Enhancements Beyond Dual Display**
   - Column header changes (keep existing `start_time_ticks` names)
   - CSS styling for timing cells (optional, not required)
   - Hover tooltips (dual display format is sufficient)

3. **API Layer Changes**
   - **NOT changing API field types** (milliseconds/seconds remain as-is)
   - **NOT implementing raw tick API** (pragmatic deviation accepted)
   - Only adding documentation, not changing functionality

4. **Automated Migration**
   - No automatic database migration from `duration REAL` to `duration_ticks INTEGER`
   - User manual rebuild required (acceptable per user decision)

5. **Test Infrastructure Changes**
   - Using existing test framework (no new tooling)
   - No property-based testing (quickcheck) for this plan
   - No performance benchmarking (functional correctness only)

6. **Additional Modules**
   - wkmp-ui, wkmp-pd, wkmp-le not modified (placeholders, no timing code)

---

## Assumptions

1. **Technical Assumptions**
   - `wkmp_common::timing` module functions are correct and tested (from analysis: 40+ tests exist)
   - TICK_RATE constant (28,224,000 Hz) is correct per SPEC017
   - Database schema changes can be made by deleting and recreating database
   - JavaScript division performance adequate for table rendering (<100ms)

2. **Process Assumptions**
   - User has reviewed and approved SPEC017 compliance analysis results
   - Breaking change (database rebuild) is acceptable
   - No production data to migrate (development/testing phase)
   - Implementation can be done in single increment (4-8 hours estimated)

3. **Dependency Assumptions**
   - wkmp-dr UI uses standard JavaScript (no framework dependencies)
   - wkmp-ai import workflow already uses `wkmp_common::timing`
   - Database schema updates don't require migration framework
   - Documentation tools available (text editor)

4. **Stakeholder Assumptions**
   - User decisions are final (API layer philosophy, display format, file duration migration)
   - User will test changes manually in wkmp-dr UI
   - User accepts pragmatic API deviation with documentation

---

## Constraints

### Technical Constraints

1. **Breaking Change Management**
   - File duration migration requires database rebuild
   - No backward compatibility with existing databases
   - Users must re-import files after migration

2. **Display Precision**
   - 6 decimal places hardcoded (microsecond precision)
   - JavaScript number precision limits (IEEE 754 double)
   - Tick values >2^53 may lose precision in JavaScript (unlikely: max is ~10 years)

3. **Module Interdependencies**
   - wkmp-common changes affect all dependent modules
   - wkmp-dr is independent (JavaScript frontend)
   - wkmp-ai file import depends on database schema

### Process Constraints

1. **Timeline**
   - Single increment implementation
   - Estimated 4-8 hours total effort
   - Must be completed before next database-dependent feature

2. **Testing**
   - Manual testing required for wkmp-dr UI (visual verification)
   - Automated tests for backend changes
   - No UI automation framework available

3. **Documentation**
   - SPEC017 and IMPL001 must be updated before marking complete
   - Migration notes must be clear for users
   - Code comments must follow WKMP standards

### Resource Constraints

1. **No External Dependencies**
   - Uses only existing WKMP codebase and standard libraries
   - No new npm packages, crates, or tools required

2. **Local Development**
   - Changes testable on local development environment
   - No staging/production deployment coordination needed

---

## Dependencies

### Existing Code (Read-Only Reference)

| Component | Location | Purpose | Status |
|-----------|----------|---------|--------|
| SPEC017 | docs/SPEC017-sample_rate_conversion.md | Tick-based timing spec | ✅ Exists (327 lines) |
| SPEC023 | docs/SPEC023-timing_terminology.md | Four timing types | ✅ Exists (233 lines) |
| IMPL001 | docs/IMPL001-database_schema.md | Database schema docs | ✅ Exists |
| wkmp_common::timing | wkmp-common/src/timing.rs | Conversion functions | ✅ Exists (641 lines, tested) |
| wkmp_common::timing_tests | wkmp-common/src/timing_tests.rs | Timing unit tests | ✅ Exists (387 lines, 40+ tests) |

### Integration Points (Will Be Modified)

| Component | Location | Modification Type | Breaking? |
|-----------|----------|-------------------|-----------|
| wkmp-dr UI | wkmp-dr/src/ui/app.js | Add conversion logic | No |
| wkmp-ap API | wkmp-ap/src/api/handlers.rs | Add doc comments | No |
| wkmp-ai API | wkmp-ai/src/api/amplitude_analysis.rs | Add doc comments | No |
| wkmp-ai models | wkmp-ai/src/db/files.rs | Change field type | **YES** |
| Database schema | wkmp-common/src/db/init.rs | Change column type | **YES** |
| SPEC017 docs | docs/SPEC017-sample_rate_conversion.md | Add section | No |
| IMPL001 docs | docs/IMPL001-database_schema.md | Update schema | No |

### External Dependencies

**None** - All changes use existing WKMP code and standard libraries.

---

## Risk Summary

**Overall Risk:** LOW-MEDIUM
- wkmp-dr changes: LOW risk (display only, no data changes)
- API documentation: LOW risk (comments only, no code changes)
- File duration migration: MEDIUM risk (breaking change, requires rebuild)

**Mitigation:**
- Clear migration instructions provided
- Breaking change clearly documented
- Test coverage for all functional changes
- User manual testing for UI changes

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
- ✅ Database schema consistency achieved (all timing fields use ticks)
- ✅ Code maintainability improved (clear variable units)
- ✅ User understands migration path (clear documentation)
