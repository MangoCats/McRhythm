# Traceability Matrix: Database Review Module (wkmp-dr)

**Plan:** PLAN015 - Database Review Module (wkmp-dr)
**Created:** 2025-11-01
**Coverage:** 100% (24/24 requirements have tests)

---

## Purpose

This matrix ensures:
1. **Forward Traceability:** Every requirement has acceptance tests (requirement → test)
2. **Backward Traceability:** Every test traces to a requirement (test → requirement)
3. **Implementation Tracking:** Requirements mapped to implementation files
4. **Verification:** All acceptance criteria testable

---

## Complete Traceability Matrix

| Req ID | Requirement | Unit Tests | Integration Tests | System Tests | Manual Tests | Implementation File(s) | Status | Coverage |
|--------|-------------|------------|-------------------|--------------|--------------|------------------------|--------|----------|
| REQ-DR-F-010 | Table viewing | TC-U-010-01 | TC-I-010-01 | - | - | wkmp-dr/src/db/tables.rs, wkmp-dr/src/api/tables.rs | Pending | Complete |
| REQ-DR-F-020 | Pagination | TC-U-020-01, TC-U-020-02 | TC-I-020-01 | - | - | wkmp-dr/src/db/pagination.rs, wkmp-dr/src/api/tables.rs | Pending | Complete |
| REQ-DR-F-030 | Row counts | TC-U-030-01 | - | - | - | wkmp-dr/src/db/tables.rs | Pending | Complete |
| REQ-DR-F-040 | Filter: passages no MBID | TC-U-040-01 | TC-I-040-01 | - | - | wkmp-dr/src/db/filters.rs, wkmp-dr/src/api/filters.rs | Pending | Complete |
| REQ-DR-F-050 | Filter: files no passages | TC-U-050-01 | TC-I-050-01 | - | - | wkmp-dr/src/db/filters.rs, wkmp-dr/src/api/filters.rs | Pending | Complete |
| REQ-DR-F-060 | Search by Work ID | TC-U-060-01, TC-U-060-02 | TC-I-060-01 | - | - | wkmp-dr/src/db/search.rs, wkmp-dr/src/api/search.rs | Pending | Complete |
| REQ-DR-F-070 | Search by file path | TC-U-070-01 | TC-I-070-01 | - | - | wkmp-dr/src/db/search.rs, wkmp-dr/src/api/search.rs | Pending | Complete |
| REQ-DR-F-080 | Sort columns | TC-U-080-01, TC-U-080-02 | TC-I-080-01 | - | - | wkmp-dr/src/db/sorting.rs, wkmp-dr/src/api/tables.rs | Pending | Complete |
| REQ-DR-F-090 | Save searches | TC-U-090-01, TC-U-090-02 | - | TC-S-090-01 | - | wkmp-dr/static/database_viewer.js (localStorage) | Pending | Complete |
| REQ-DR-F-100 | Preference persistence | TC-U-100-01 | - | TC-S-100-01 | - | wkmp-dr/static/database_viewer.js (localStorage) | Pending | Complete |
| REQ-DR-F-110 | Extensible views | - | - | TC-S-110-01 | TC-M-110-01 | wkmp-dr/src/db/filters.rs (plugin system) | Pending | Complete |
| REQ-DR-NF-010 | Zero-config startup | - | TC-I-NF010-01, TC-I-NF010-02, TC-I-NF010-03, TC-I-NF010-04 | - | - | wkmp-dr/src/main.rs:10-35 | Pending | Complete |
| REQ-DR-NF-020 | Read-only database | TC-U-NF020-01 | TC-I-NF020-01 | - | - | wkmp-dr/src/db/mod.rs:15-25 | Pending | Complete |
| REQ-DR-NF-030 | API authentication | - | TC-I-NF030-01, TC-I-NF030-02, TC-I-NF030-03 | - | - | wkmp-dr/src/api/auth.rs, wkmp-dr/src/lib.rs | Pending | Complete |
| REQ-DR-NF-040 | Health endpoint | - | TC-I-NF040-01, TC-I-NF040-02 | - | - | wkmp-dr/src/api/health.rs | Pending | Complete |
| REQ-DR-NF-050 | Port 5725 | - | TC-I-NF050-01 | - | - | wkmp-dr/src/main.rs:40-50 | Pending | Complete |
| REQ-DR-NF-060 | On-demand pattern | - | - | TC-S-NF060-01 | - | wkmp-dr/src/api/ui.rs (UI pages) | Pending | Complete |
| REQ-DR-NF-070 | wkmp-ui launch button | - | - | - | TC-M-NF070-01 | wkmp-ui/src/api/ui.rs | Pending | Complete |
| REQ-DR-NF-080 | Full version packaging | - | - | TC-S-NF080-01 | - | scripts/package-full.sh | Pending | Complete |
| REQ-DR-UI-010 | Inline HTML | - | - | TC-S-UI010-01 | - | wkmp-dr/src/api/ui.rs | Pending | Complete |
| REQ-DR-UI-020 | Vanilla JavaScript | - | - | TC-S-UI020-01 | - | wkmp-dr/static/*.js | Pending | Complete |
| REQ-DR-UI-030 | CSS custom properties | - | - | TC-S-UI030-01 | - | wkmp-dr/static/*.css | Pending | Complete |
| REQ-DR-UI-040 | Mobile-responsive | - | - | TC-S-UI040-01 | - | wkmp-dr/static/*.css | Pending | Complete |
| REQ-DR-UI-050 | Table rendering | - | - | TC-S-UI050-01, TC-S-UI050-02 | - | wkmp-dr/static/database_viewer.html/js | Pending | Complete |

---

## Coverage Statistics

### Requirements Coverage
- **Total Requirements:** 24
- **Requirements with Tests:** 24
- **Requirements without Tests:** 0
- **Coverage:** 100%

### Test Type Distribution
- **Unit Tests:** 18 (40%)
- **Integration Tests:** 15 (33.3%)
- **System Tests:** 10 (22.2%)
- **Manual Tests:** 2 (4.4%)
- **Total Tests:** 45

### Priority Coverage
- **P0 Requirements (16):** 33 tests (avg 2.1 tests/req)
- **P1 Requirements (6):** 10 tests (avg 1.7 tests/req)
- **P2 Requirements (2):** 2 tests (avg 1.0 tests/req)

---

## Test Execution Status

| Status | Requirements | Tests | Percentage |
|--------|--------------|-------|------------|
| **Pending** (Not implemented) | 24 | 45 | 100% |
| **In Progress** (Implementing) | 0 | 0 | 0% |
| **Complete** (Tests pass) | 0 | 0 | 0% |
| **Verified** (Code reviewed) | 0 | 0 | 0% |

**Note:** Status updated during implementation. All tests pending until coding begins.

---

## Critical Path Tests (Must Pass for MVP)

**P0 Requirements - 16 total:**
1. REQ-DR-F-010: Table viewing → TC-U-010-01, TC-I-010-01
2. REQ-DR-F-020: Pagination → TC-U-020-01, TC-U-020-02, TC-I-020-01
3. REQ-DR-F-030: Row counts → TC-U-030-01
4. REQ-DR-F-040: Filter passages no MBID → TC-U-040-01, TC-I-040-01
5. REQ-DR-F-060: Search by Work ID → TC-U-060-01, TC-U-060-02, TC-I-060-01
6. REQ-DR-F-080: Sort columns → TC-U-080-01, TC-U-080-02, TC-I-080-01
7. REQ-DR-F-090: Save searches → TC-U-090-01, TC-U-090-02, TC-S-090-01
8. REQ-DR-F-100: Preferences → TC-U-100-01, TC-S-100-01
9. REQ-DR-NF-010: Zero-config → TC-I-NF010-01, TC-I-NF010-02, TC-I-NF010-03, TC-I-NF010-04
10. REQ-DR-NF-020: Read-only → TC-U-NF020-01, TC-I-NF020-01
11. REQ-DR-NF-030: Authentication → TC-I-NF030-01, TC-I-NF030-02, TC-I-NF030-03
12. REQ-DR-NF-040: Health endpoint → TC-I-NF040-01, TC-I-NF040-02
13. REQ-DR-NF-050: Port 5725 → TC-I-NF050-01
14. REQ-DR-NF-060: On-demand pattern → TC-S-NF060-01
15. REQ-DR-NF-080: Packaging → TC-S-NF080-01
16. REQ-DR-UI-050: Table rendering → TC-S-UI050-01, TC-S-UI050-02

**Total Critical Path Tests:** 33 (must all pass before MVP release)

---

## Implementation Sequence (Test-First)

**Recommended order (by dependency):**

### Increment 1: Database Foundation
- REQ-DR-NF-020 → TC-U-NF020-01, TC-I-NF020-01 (read-only connection)
- REQ-DR-F-010 → TC-U-010-01, TC-I-010-01 (table listing)
- REQ-DR-F-030 → TC-U-030-01 (row counts)

### Increment 2: Zero-Config & Health
- REQ-DR-NF-010 → TC-I-NF010-01 through TC-I-NF010-04 (zero-config)
- REQ-DR-NF-040 → TC-I-NF040-01, TC-I-NF040-02 (health endpoint)
- REQ-DR-NF-050 → TC-I-NF050-01 (port binding)

### Increment 3: Authentication
- REQ-DR-NF-030 → TC-I-NF030-01, TC-I-NF030-02, TC-I-NF030-03 (API auth)

### Increment 4: Pagination & Sorting
- REQ-DR-F-020 → TC-U-020-01, TC-U-020-02, TC-I-020-01 (pagination)
- REQ-DR-F-080 → TC-U-080-01, TC-U-080-02, TC-I-080-01 (sorting)

### Increment 5: Filters & Searches
- REQ-DR-F-040 → TC-U-040-01, TC-I-040-01 (filter: passages no MBID)
- REQ-DR-F-050 → TC-U-050-01, TC-I-050-01 (filter: files no passages)
- REQ-DR-F-060 → TC-U-060-01, TC-U-060-02, TC-I-060-01 (search: Work ID)
- REQ-DR-F-070 → TC-U-070-01, TC-I-070-01 (search: file path)

### Increment 6: User Preferences
- REQ-DR-F-090 → TC-U-090-01, TC-U-090-02, TC-S-090-01 (saved searches)
- REQ-DR-F-100 → TC-U-100-01, TC-S-100-01 (persistence)

### Increment 7: UI & Integration
- REQ-DR-UI-010, REQ-DR-UI-020, REQ-DR-UI-030 → TC-S-UI010-01, TC-S-UI020-01, TC-S-UI030-01
- REQ-DR-UI-050 → TC-S-UI050-01, TC-S-UI050-02 (table rendering)
- REQ-DR-NF-060 → TC-S-NF060-01 (on-demand pattern)
- REQ-DR-NF-070 → TC-M-NF070-01 (wkmp-ui button)

### Increment 8: Packaging & Extensibility
- REQ-DR-NF-080 → TC-S-NF080-01 (Full version packaging)
- REQ-DR-F-110 → TC-S-110-01, TC-M-110-01 (extensible views)
- REQ-DR-UI-040 → TC-S-UI040-01 (mobile-responsive)

---

## Test Dependencies

**Sequential Dependencies (must run in order):**
1. TC-U-NF020-01 → TC-I-NF020-01 (unit before integration)
2. TC-I-NF010-04 → TC-I-010-01 (zero-config before API tests)
3. TC-U-090-01 → TC-S-090-01 (unit before system)

**Parallel Tests (can run simultaneously):**
- All unit tests (independent)
- Most integration tests (separate endpoints)
- System tests (if test databases isolated)

---

## Verification Checklist

**Before Implementation:**
- [ ] All 24 requirements have test IDs assigned
- [ ] Test index complete (45 tests documented)
- [ ] Traceability matrix shows 100% coverage

**During Implementation:**
- [ ] Write tests BEFORE implementing feature (TDD)
- [ ] Each test passes before marking increment complete
- [ ] Update matrix status as tests pass

**Before Release:**
- [ ] All P0 tests pass (33 critical tests)
- [ ] 80%+ P1 tests pass (target: 8/10 tests)
- [ ] Test coverage ≥80% (measured by cargo-tarpaulin)
- [ ] No P0 requirements without passing tests
- [ ] Manual tests executed and documented

---

## Risk Analysis

**High-Risk Requirements (Complex Testing):**
1. REQ-DR-NF-030 (Authentication) - Complex protocol, security-critical
2. REQ-DR-F-090/100 (Persistence) - Browser localStorage, cross-session state
3. REQ-DR-NF-010 (Zero-config) - 4 tiers, platform-specific paths

**Mitigation:**
- Extra test coverage for high-risk (2-3 tests per requirement)
- Manual verification for security-critical features
- Platform-specific testing (Linux, macOS)

---

## Test Maintenance

**Update Triggers:**
1. **Requirement Change:** Update affected tests, re-verify pass criteria
2. **Schema Change:** Update test data, regenerate test database
3. **API Change:** Update integration tests, check backward compatibility
4. **Bug Found:** Add regression test to prevent recurrence

**Ownership:**
- Traceability matrix: Updated by implementer after each increment
- Test specs: Owned by feature implementer
- Test data: Shared responsibility (document changes)

---

**Traceability Matrix Complete**
**Coverage:** 100% (24/24 requirements covered)
**Status:** Ready for implementation
**Next:** Begin Increment 1 (Database Foundation) tests
