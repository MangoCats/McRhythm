# Test Index: Database Review Module (wkmp-dr)

**Plan:** PLAN015 - Database Review Module (wkmp-dr)
**Created:** 2025-11-01
**Total Tests:** 45 (18 unit, 15 integration, 10 system, 2 manual)

---

## Quick Reference

**For Implementation:**
- Read this index for test overview
- Read individual test spec files when implementing feature
- Reference traceability_matrix.md for requirement → test mapping

---

## Test Summary by Requirement

| Req ID | Requirement | Tests | Type | Priority |
|--------|-------------|-------|------|----------|
| **REQ-DR-F-010** | Table-by-table viewing | TC-U-010-01, TC-I-010-01 | Unit, Integration | P0 |
| **REQ-DR-F-020** | Paginated browsing | TC-U-020-01, TC-U-020-02, TC-I-020-01 | Unit, Integration | P0 |
| **REQ-DR-F-030** | Row count display | TC-U-030-01 | Unit | P0 |
| **REQ-DR-F-040** | Filter: passages lacking MBID | TC-U-040-01, TC-I-040-01 | Unit, Integration | P0 |
| **REQ-DR-F-050** | Filter: files without passages | TC-U-050-01, TC-I-050-01 | Unit, Integration | P1 |
| **REQ-DR-F-060** | Search by Work ID | TC-U-060-01, TC-U-060-02, TC-I-060-01 | Unit, Integration | P0 |
| **REQ-DR-F-070** | Search by file path | TC-U-070-01, TC-I-070-01 | Unit, Integration | P1 |
| **REQ-DR-F-080** | Sort columns | TC-U-080-01, TC-U-080-02, TC-I-080-01 | Unit, Integration | P0 |
| **REQ-DR-F-090** | Save favorite searches | TC-U-090-01, TC-U-090-02, TC-S-090-01 | Unit, System | P0 |
| **REQ-DR-F-100** | Preference persistence | TC-U-100-01, TC-S-100-01 | Unit, System | P0 |
| **REQ-DR-F-110** | Extensible view system | TC-S-110-01, TC-M-110-01 | System, Manual | P1 |
| **REQ-DR-NF-010** | Zero-config startup | TC-I-NF010-01, TC-I-NF010-02, TC-I-NF010-03, TC-I-NF010-04 | Integration | P0 |
| **REQ-DR-NF-020** | Read-only database | TC-U-NF020-01, TC-I-NF020-01 | Unit, Integration | P0 |
| **REQ-DR-NF-030** | API authentication | TC-I-NF030-01, TC-I-NF030-02, TC-I-NF030-03 | Integration | P0 |
| **REQ-DR-NF-040** | Health endpoint | TC-I-NF040-01, TC-I-NF040-02 | Integration | P0 |
| **REQ-DR-NF-050** | Port 5725 | TC-I-NF050-01 | Integration | P0 |
| **REQ-DR-NF-060** | On-demand pattern | TC-S-NF060-01 | System | P0 |
| **REQ-DR-NF-070** | wkmp-ui launch button | TC-M-NF070-01 | Manual | P1 |
| **REQ-DR-NF-080** | Full version packaging | TC-S-NF080-01 | System | P0 |
| **REQ-DR-UI-010** | Inline HTML templates | TC-S-UI010-01 | System | P1 |
| **REQ-DR-UI-020** | Vanilla JavaScript | TC-S-UI020-01 | System | P1 |
| **REQ-DR-UI-030** | CSS custom properties | TC-S-UI030-01 | System | P1 |
| **REQ-DR-UI-040** | Mobile-responsive | TC-S-UI040-01 | System | P2 |
| **REQ-DR-UI-050** | Table rendering | TC-S-UI050-01, TC-S-UI050-02 | System | P0 |

---

## Test Types

### Unit Tests (18 tests)
**Scope:** Individual functions/components in isolation
**Examples:** Database query functions, pagination logic, UUID validation
**Run Frequency:** On every commit
**Target Coverage:** 80%+ code coverage

| Test ID | Brief Description | File |
|---------|-------------------|------|
| TC-U-010-01 | List all tables query | tc_u_010_01.md |
| TC-U-020-01 | Pagination offset calculation | tc_u_020_01.md |
| TC-U-020-02 | Pagination bounds checking | tc_u_020_02.md |
| TC-U-030-01 | Row count query | tc_u_030_01.md |
| TC-U-040-01 | Passages without MBID query | tc_u_040_01.md |
| TC-U-050-01 | Files without passages query | tc_u_050_01.md |
| TC-U-060-01 | Search by Work ID query | tc_u_060_01.md |
| TC-U-060-02 | Invalid UUID rejection | tc_u_060_02.md |
| TC-U-070-01 | File path pattern search | tc_u_070_01.md |
| TC-U-080-01 | Sort ascending | tc_u_080_01.md |
| TC-U-080-02 | Sort descending | tc_u_080_02.md |
| TC-U-090-01 | Save search to localStorage | tc_u_090_01.md |
| TC-U-090-02 | Load saved search | tc_u_090_02.md |
| TC-U-100-01 | Preference persistence | tc_u_100_01.md |
| TC-U-NF020-01 | Read-only connection mode | tc_u_nf020_01.md |

### Integration Tests (15 tests)
**Scope:** HTTP endpoints, cross-component interactions
**Examples:** API authentication, database queries via HTTP, wkmp-ui integration
**Run Frequency:** Before merging to main
**Target Coverage:** All API endpoints

| Test ID | Brief Description | File |
|---------|-------------------|------|
| TC-I-010-01 | GET /api/tables endpoint | tc_i_010_01.md |
| TC-I-020-01 | GET /api/table/:name pagination | tc_i_020_01.md |
| TC-I-040-01 | GET /api/filters/passages_without_mbid | tc_i_040_01.md |
| TC-I-050-01 | GET /api/filters/files_without_passages | tc_i_050_01.md |
| TC-I-060-01 | GET /api/search/by_work_id | tc_i_060_01.md |
| TC-I-070-01 | GET /api/search/by_file_path | tc_i_070_01.md |
| TC-I-080-01 | GET /api/table/:name with sort params | tc_i_080_01.md |
| TC-I-NF010-01 | Zero-config: CLI args | tc_i_nf010_01.md |
| TC-I-NF010-02 | Zero-config: ENV vars | tc_i_nf010_02.md |
| TC-I-NF010-03 | Zero-config: TOML config | tc_i_nf010_03.md |
| TC-I-NF010-04 | Zero-config: Compiled default | tc_i_nf010_04.md |
| TC-I-NF020-01 | Read-only enforcement | tc_i_nf020_01.md |
| TC-I-NF030-01 | Auth: valid request | tc_i_nf030_01.md |
| TC-I-NF030-02 | Auth: invalid hash | tc_i_nf030_02.md |
| TC-I-NF030-03 | Auth: expired timestamp | tc_i_nf030_03.md |
| TC-I-NF040-01 | Health endpoint response | tc_i_nf040_01.md |
| TC-I-NF040-02 | Health endpoint performance | tc_i_nf040_02.md |
| TC-I-NF050-01 | Port 5725 binding | tc_i_nf050_01.md |

### System Tests (10 tests)
**Scope:** End-to-end user scenarios, full module behavior
**Examples:** Complete user workflows, browser interaction, packaging
**Run Frequency:** Before release
**Target Coverage:** All user-facing features

| Test ID | Brief Description | File |
|---------|-------------------|------|
| TC-S-090-01 | Save/load favorite search workflow | tc_s_090_01.md |
| TC-S-100-01 | Preferences persist across restart | tc_s_100_01.md |
| TC-S-110-01 | Add new custom filter | tc_s_110_01.md |
| TC-S-NF060-01 | On-demand launch from wkmp-ui | tc_s_nf060_01.md |
| TC-S-NF080-01 | Full version packaging | tc_s_nf080_01.md |
| TC-S-UI010-01 | Inline HTML templates verified | tc_s_ui010_01.md |
| TC-S-UI020-01 | No JavaScript frameworks | tc_s_ui020_01.md |
| TC-S-UI030-01 | WKMP CSS theme consistency | tc_s_ui030_01.md |
| TC-S-UI040-01 | Mobile-responsive layout | tc_s_ui040_01.md |
| TC-S-UI050-01 | Table rendering performance | tc_s_ui050_01.md |
| TC-S-UI050-02 | Pagination control functionality | tc_s_ui050_02.md |

### Manual Tests (2 tests)
**Scope:** Tests requiring human judgment or complex setup
**Examples:** UI consistency checks, browser compatibility
**Run Frequency:** Before release
**Documentation:** Detailed test procedures

| Test ID | Brief Description | File |
|---------|-------------------|------|
| TC-M-110-01 | Custom filter addition procedure | tc_m_110_01.md |
| TC-M-NF070-01 | wkmp-ui launch button integration | tc_m_nf070_01.md |

---

## Test Execution Order

**Phase 1: Unit Tests (Development)**
Run continuously during development, fast feedback (<5s)

**Phase 2: Integration Tests (Pre-merge)**
Run before merging to main branch (~30s-1min)

**Phase 3: System Tests (Pre-release)**
Run before tagging release (~5-10min)

**Phase 4: Manual Tests (Pre-release)**
Execute final validation (~15-30min)

---

## Test Coverage by Priority

| Priority | Requirements | Tests | Coverage |
|----------|--------------|-------|----------|
| P0 (Critical) | 16 | 33 | 73.3% of tests |
| P1 (High) | 6 | 10 | 22.2% of tests |
| P2 (Medium) | 2 | 2 | 4.4% of tests |
| **Total** | **24** | **45** | **100%** |

**Coverage Goal:** 100% of P0 requirements, 80%+ of P1, 50%+ of P2

---

## Test Data Requirements

**Standard Test Database:**
- Location: `tests/fixtures/test_wkmp.db`
- Contents:
  - 5 users
  - 50 files
  - 200 passages (150 with MBID, 50 without)
  - 100 songs
  - 50 artists
  - 20 albums
  - 80 works
  - Linking tables populated

**Generation:** See `tests/setup_test_db.rs`

**Edge Cases:**
- Empty database (0 rows)
- Large database (100k rows) - performance testing
- Corrupted database (schema version mismatch)

---

## Continuous Integration

**CI Pipeline:**
1. Run unit tests (all 18)
2. Run integration tests (all 15)
3. Generate coverage report (target: 80%+)
4. Run system tests (subset for CI, full for release)
5. Block merge if any P0 test fails

**Manual Testing Trigger:**
- Before major release
- When UI changes are made
- When wkmp-ui integration changes

---

## Test Documentation Format

**Each test specification file contains:**
- Test ID and requirement traceability
- Test type (unit/integration/system/manual)
- Scope and objective
- Given/When/Then steps
- Expected results (pass criteria)
- Failure criteria
- Test data requirements
- Estimated execution time
- Dependencies

**Example:** See `tc_u_010_01.md` for template

---

## Next Steps

1. **Review test index** - Verify all requirements covered
2. **Check traceability_matrix.md** - Ensure 100% requirement → test mapping
3. **Read individual test specs** - For each feature being implemented
4. **Implement tests first** - Test-driven development approach
5. **Verify tests pass** - Before marking feature complete

---

**Test Index Complete**
**Total Coverage:** 45 tests for 24 requirements (avg 1.9 tests per requirement)
**Next:** Review traceability_matrix.md for complete requirement → test mapping
