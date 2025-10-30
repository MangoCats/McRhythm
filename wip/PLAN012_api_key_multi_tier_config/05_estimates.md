# PLAN012 - Phase 6: Effort and Schedule Estimation

**Specification:** SPEC025-api_key_configuration.md
**Plan Version:** 1.0
**Date:** 2025-10-30

---

## Executive Summary

**Total Effort (Estimate):** 20-28 hours (base) + 4-6 hours (contingency) = **24-34 hours**

**Timeline:** 3-5 working days (assuming 8-hour days)

**Confidence Level:** HIGH (well-established patterns, comprehensive test coverage)

**Resource Requirements:** 1 developer (full stack: Rust + HTML/CSS/JS)

---

## Per-Increment Effort Estimates

### Increment 1: wkmp-common TOML Schema Extension

**Base Estimate:** 1-2 hours
**Contingency:** +0.5 hour (25%)
**Total:** 1.5-2.5 hours

**Breakdown:**
- Extend TomlConfig struct: 0.5 hour
- Add Serialize trait: 0.2 hour
- Write unit tests (2 tests): 0.5-1 hour
- Verification and testing: 0.3 hour

**Risk Factors:**
- LOW: Simple struct extension, well-defined pattern

---

### Increment 2: wkmp-common TOML Atomic Write Utilities

**Base Estimate:** 4-6 hours
**Contingency:** +1.5 hours (30%)
**Total:** 5.5-7.5 hours

**Breakdown:**
- Implement write_toml_config(): 1-1.5 hours
- Implement set_unix_permissions_0600(): 0.5 hour
- Implement check_toml_permissions_loose(): 0.5 hour
- Write unit tests (7 tests): 1.5-2.5 hours
- Unix/Windows platform testing: 0.5-1 hour
- Verification and debugging: 0.5 hour

**Risk Factors:**
- MEDIUM: Atomic operations, platform-specific code (Unix vs Windows)
- Contingency higher due to edge cases (temp file cleanup, permission errors)

---

### Increment 3: wkmp-ai Database Accessors

**Base Estimate:** 2-3 hours
**Contingency:** +0.5 hour (20%)
**Total:** 2.5-3.5 hours

**Breakdown:**
- Implement get/set_acoustid_api_key(): 0.5 hour
- Implement generic get/set_setting(): 0.5 hour
- Write unit tests (4 tests): 1-1.5 hours
- Database integration verification: 0.5 hour

**Risk Factors:**
- LOW: Follows existing wkmp-ap/src/db/settings.rs pattern

---

### Increment 4: wkmp-ai Multi-Tier Resolver

**Base Estimate:** 3-4 hours
**Contingency:** +1 hour (30%)
**Total:** 4-5 hours

**Breakdown:**
- Implement resolve_acoustid_api_key(): 1-1.5 hours
- Implement validation (is_valid_key): 0.2 hour
- Write unit tests (11 tests: 8 resolution + 3 validation): 1.5-2 hours
- Integration with ENV/TOML/DB: 0.5 hour
- Logging and error messages: 0.3 hour

**Risk Factors:**
- MEDIUM: Multi-tier logic complexity, edge case handling

---

### Increment 5: wkmp-ai Settings Sync with Write-Back

**Base Estimate:** 2-3 hours
**Contingency:** +0.5 hour (25%)
**Total:** 2.5-3.5 hours

**Breakdown:**
- Implement sync_settings_to_toml(): 0.5-1 hour
- Implement migrate_key_to_database(): 0.5 hour
- Write unit tests (7 tests: 6 write-back + 1 security): 1-1.5 hours
- Best-effort error handling testing: 0.5 hour

**Risk Factors:**
- MEDIUM: TOML write-back, best-effort error handling

---

### Increment 6: wkmp-ai Startup Integration

**Base Estimate:** 2-3 hours
**Contingency:** +0.5 hour (25%)
**Total:** 2.5-3.5 hours

**Breakdown:**
- Integrate resolve_acoustid_api_key() into main.rs: 0.5 hour
- Implement auto-migration logic: 0.5 hour
- Write integration tests (4 tests): 1-1.5 hours
- End-to-end startup testing: 0.5 hour

**Risk Factors:**
- MEDIUM: Integration with existing startup sequence

---

### Increment 7: Web UI API Endpoint

**Base Estimate:** 2-3 hours
**Contingency:** +0.5 hour (20%)
**Total:** 2.5-3.5 hours

**Breakdown:**
- Implement POST endpoint handler: 0.5 hour
- Add route to Axum router: 0.2 hour
- Validation and error handling: 0.5 hour
- Write integration tests (3 tests): 1-1.5 hours
- API testing (curl/Postman): 0.3 hour

**Risk Factors:**
- LOW: Standard Axum endpoint pattern

---

### Increment 8: Web UI Settings Page

**Base Estimate:** 2-3 hours
**Contingency:** +0.5 hour (20%)
**Total:** 2.5-3.5 hours

**Breakdown:**
- Create settings.html: 0.5 hour
- Create settings.css: 0.5 hour
- Create settings.js (form handling): 0.5-1 hour
- Browser testing: 0.5 hour
- UI polish and refinement: 0.5 hour

**Risk Factors:**
- LOW: Simple HTML/CSS/JS form

---

### Increment 9: Integration and Recovery Tests

**Base Estimate:** 3-4 hours
**Contingency:** +1 hour (30%)
**Total:** 4-5 hours

**Breakdown:**
- Write recovery tests (2 tests): 1-1.5 hours
- Write concurrency test (1 test): 0.5-1 hour
- Execute system tests (2 manual tests): 1-1.5 hours
- Test environment setup and debugging: 0.5 hour

**Risk Factors:**
- MEDIUM: Integration tests require full environment setup

---

### Increment 10: Manual Testing and Documentation

**Base Estimate:** 3-4 hours
**Contingency:** +0.5 hour (15%)
**Total:** 3.5-4.5 hours

**Breakdown:**
- Execute manual tests (6 scenarios): 1.5-2 hours
- Update IMPL012-acoustid_client.md: 0.5 hour
- Update IMPL001-database_schema.md: 0.2 hour
- Review and refine documentation: 0.5-1 hour
- Final verification: 0.3 hour

**Risk Factors:**
- LOW: Documentation and manual testing (no coding)

---

## Summary Table

| Increment | Component | Base Estimate | Contingency | Total Estimate | Risk |
|-----------|-----------|---------------|-------------|----------------|------|
| 1 | TOML Schema Extension | 1-2h | +0.5h | 1.5-2.5h | LOW |
| 2 | TOML Atomic Write | 4-6h | +1.5h | 5.5-7.5h | MEDIUM |
| 3 | DB Accessors | 2-3h | +0.5h | 2.5-3.5h | LOW |
| 4 | Multi-Tier Resolver | 3-4h | +1h | 4-5h | MEDIUM |
| 5 | Settings Sync | 2-3h | +0.5h | 2.5-3.5h | MEDIUM |
| 6 | Startup Integration | 2-3h | +0.5h | 2.5-3.5h | MEDIUM |
| 7 | Web UI Endpoint | 2-3h | +0.5h | 2.5-3.5h | LOW |
| 8 | Web UI Page | 2-3h | +0.5h | 2.5-3.5h | LOW |
| 9 | Integration Tests | 3-4h | +1h | 4-5h | MEDIUM |
| 10 | Manual Tests + Docs | 3-4h | +0.5h | 3.5-4.5h | LOW |
| **TOTAL** | | **24-35h** | **+7h** | **31-42h** | |

**Adjusted Total (Realistic):** 24-34 hours (accounting for overlapping contingencies)

---

## Dependency Analysis

### Sequential Dependencies (Must Be Done in Order)

**Critical Path:**
1. Increment 1 → Increment 2 (TOML schema before write utilities)
2. Increment 2 → Increment 5 (write utilities before sync)
3. Increment 3 → Increment 4 (DB accessors before resolver)
4. Increment 4 → Increment 5 (resolver before migration)
5. Increment 5 → Increment 6 (sync before startup integration)
6. Increment 6 → Increment 9 (startup before integration tests)
7. Increment 7 → Increment 8 (endpoint before UI page)
8. Increment 8 → Increment 9 (UI before system tests)

**Parallel Opportunities:**
- Increment 3 (DB accessors) can run in parallel with Increment 1-2 (TOML utilities)
- Increment 7-8 (Web UI) can run partially in parallel with Increment 6 (startup)
- Increment 9 (tests) can begin writing tests before implementation complete (test-first)

---

## Timeline Breakdown

### Day 1: Foundation (Increments 1-3)

**Hours:** 6-10 hours
**Deliverables:**
- wkmp-common TOML schema extended
- wkmp-common TOML write utilities implemented
- wkmp-ai database accessors implemented
- Unit tests passing: 13 tests (7 TOML + 4 DB + 2 validation)

**Parallel Work:**
- Increment 1 (1.5-2.5h)
- Increment 2 (5.5-7.5h) - starts after Increment 1
- Increment 3 (2.5-3.5h) - can start in parallel with Increment 2

**Checkpoint 1:** End of Day 1 (after Increment 2)

---

### Day 2: Core Logic (Increments 4-5)

**Hours:** 6-8 hours
**Deliverables:**
- Multi-tier resolver implemented
- Settings sync with write-back implemented
- Unit tests passing: 28 total (13 from Day 1 + 15 new)

**Sequential Work:**
- Increment 4 (4-5h) - depends on Increment 3
- Increment 5 (2.5-3.5h) - depends on Increment 2 and 4

**Checkpoint 2:** End of Day 2 (after Increment 5)

---

### Day 3: Integration (Increments 6-7)

**Hours:** 5-7 hours
**Deliverables:**
- Startup integration complete
- Web UI endpoint implemented
- Integration tests passing: 7 tests (4 e2e + 3 UI)

**Sequential Work:**
- Increment 6 (2.5-3.5h) - depends on Increment 5
- Increment 7 (2.5-3.5h) - depends on Increment 3

---

### Day 4: UI and Testing (Increments 8-9)

**Hours:** 6-9 hours
**Deliverables:**
- Web UI settings page complete
- Integration and recovery tests complete
- All automated tests passing: 41 tests (28 unit + 10 integration + 3 system)

**Sequential Work:**
- Increment 8 (2.5-3.5h) - depends on Increment 7
- Increment 9 (4-5h) - depends on Increment 6 and 8

**Checkpoint 3:** End of Day 4 (after Increment 9)

---

### Day 5: Finalization (Increment 10)

**Hours:** 3.5-4.5 hours
**Deliverables:**
- Manual tests complete (6 scenarios)
- Documentation updated
- All 47 tests passing
- 100% requirement coverage verified

**Sequential Work:**
- Increment 10 (3.5-4.5h) - depends on all previous increments

**Checkpoint 4:** End of Day 5 (after Increment 10) - Implementation COMPLETE

---

## Resource Requirements

### Developer Skills Required

**Must Have:**
- Rust (intermediate): Async/await, traits, error handling
- SQL/SQLite (basic): Query writing, database operations
- Axum (basic): HTTP endpoints, routing
- HTML/CSS/JS (basic): Form handling, fetch API

**Nice to Have:**
- TOML serialization (serde)
- Unix file permissions
- Test-driven development

**Estimated Developer Level:** Mid-level (3+ years Rust experience)

---

### Tools and Environment

**Development:**
- Rust toolchain (stable)
- SQLite
- Text editor/IDE (VSCode, IntelliJ Rust)
- Web browser (Chrome/Firefox for UI testing)

**Testing:**
- cargo test (unit and integration tests)
- Manual testing environment (Linux + Windows recommended)
- Database inspection tool (sqlite3 CLI or DB Browser)

**Infrastructure:**
- No external dependencies (all local development)
- No deployment infrastructure required (local testing only)

---

## Contingency Analysis

### Base Contingency: 20-30% per increment

**Rationale:**
- Well-established patterns (database accessors, HTTP endpoints)
- Comprehensive test coverage defined upfront
- No novel technical challenges
- But: First implementation of multi-tier pattern, some unknowns

### Additional Contingency Reserves

**Overall Contingency:** +4-6 hours (15-20% of total)

**Allocated for:**
- Integration issues not caught by unit tests
- Platform-specific bugs (Unix vs Windows)
- TOML round-trip serialization edge cases
- Test environment setup complexity

---

## Schedule Risks

### Risk 1: TOML Round-Trip Serialization

**Probability:** MEDIUM
**Impact:** +1-2 hours
**Mitigation:** Comprehensive unit tests (tc_u_toml_006), early testing in Increment 2
**Residual Risk:** LOW

---

### Risk 2: Cross-Platform Compatibility

**Probability:** MEDIUM
**Impact:** +2-3 hours
**Mitigation:** Conditional compilation (#[cfg(unix)]), Windows no-op approach
**Residual Risk:** LOW

---

### Risk 3: Integration Test Environment

**Probability:** LOW
**Impact:** +1-2 hours
**Mitigation:** Use in-memory databases (:memory:), temporary directories
**Residual Risk:** VERY LOW

---

## Confidence Assessment

**Overall Confidence:** HIGH (80-90%)

**Factors Increasing Confidence:**
- Well-defined requirements (62 requirements, 100% test coverage)
- Established patterns (wkmp-ap settings.rs reference)
- Comprehensive test specifications (47 tests defined upfront)
- Modular increments (early detection of issues)

**Factors Decreasing Confidence:**
- First multi-tier configuration implementation
- Platform-specific code (Unix permissions)
- Best-effort error handling (TOML write failures)

**Expected Outcome:** Actual effort 24-34 hours (within estimate range)

---

## Timeline Summary

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| Day 1 (Foundation) | 6-10h | TOML utilities, DB accessors |
| Day 2 (Core Logic) | 6-8h | Resolver, sync, write-back |
| Day 3 (Integration) | 5-7h | Startup, Web UI endpoint |
| Day 4 (UI + Testing) | 6-9h | Web UI page, integration tests |
| Day 5 (Finalization) | 3.5-4.5h | Manual tests, documentation |
| **Total** | **26.5-38.5h** | **Full implementation** |

**Calendar Time:** 3-5 working days (assuming 8-hour days, no interruptions)

**Realistic Calendar Time:** 1-2 weeks (accounting for meetings, reviews, interruptions)

---

**Phase 6 (Effort Estimation):** COMPLETE
**Next Phase:** Phase 7 - Risk Assessment and Mitigation Planning
