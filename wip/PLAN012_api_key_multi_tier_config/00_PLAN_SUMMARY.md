# PLAN012: Multi-Tier API Key Configuration - Implementation Plan Summary

**READ THIS FIRST**

**Specification:** wip/SPEC025-api_key_configuration.md
**Plan Version:** 1.0
**Date:** 2025-10-30
**Status:** Ready for Implementation (Phases 1-3 Complete)

---

## Executive Summary

**Objective:** Implement multi-tier API key configuration system for wkmp-ai with automatic migration and durable TOML backup.

**Status:** Specification verified, tests defined, ready for implementation

**Complexity:** MODERATE
- Well-established patterns (database accessors, HTTP endpoints)
- Standard libraries (toml, serde, sqlx)
- No novel technical challenges

**Risk:** LOW
- Zero critical specification issues
- 100% test coverage defined
- All dependencies verified existing

**Timeline:** Phases 4-8 to be implemented (approach selection, breakdown, effort estimation, risk assessment, final documentation)

---

## What This Plan Delivers

### User-Facing Features

**Configuration Methods (3 tiers):**
1. **Web UI** - POST /api/settings/acoustid_api_key endpoint + settings page
2. **Environment Variable** - WKMP_ACOUSTID_API_KEY (auto-migrates to DB + TOML)
3. **TOML Config** - ~/.config/wkmp/wkmp-ai.toml (auto-migrates to DB)

**Key Behaviors:**
- Database is authoritative (ignores ENV/TOML when key exists in DB)
- Auto-migration from ENV/TOML to database + TOML backup
- Database deletion recovery (TOML restores key automatically)
- Best-effort TOML write (graceful degradation if read-only)
- Security: TOML permissions 0600 (Unix), permission warnings

### Technical Components

**wkmp-common extensions:**
- TomlConfig struct: Add acoustid_api_key field
- Atomic TOML write utilities (temp file + rename)
- Unix permission setting (0600)

**wkmp-ai implementation:**
- resolve_acoustid_api_key() - Multi-tier resolution
- Database accessors (get/set_acoustid_api_key)
- Generic sync_settings_to_toml() - HashMap-based, extensible
- POST /api/settings/acoustid_api_key endpoint
- Web UI settings page at /settings

**Testing:**
- 28 unit tests (resolution, write-back, TOML utilities, validation, security)
- 10 integration tests (end-to-end startup, web UI, recovery, concurrency)
- 3 system tests (user workflows)
- 6 manual tests (migration scenarios, failure cases)

---

## Plan Structure

### Phase 1: Input Validation and Scope Definition (COMPLETE)

**Deliverables:**
- requirements_index.md - 62 requirements extracted and categorized
- scope_statement.md - In/out scope, assumptions, constraints, dependencies
- dependencies_map.md - Dependency graph, external/internal dependencies, implementation order

**Key Findings:**
- All dependencies verified existing (wkmp-common/config.rs, wkmp-ai/src/db/, settings table)
- No new external dependencies required (toml, serde, sqlx, tokio all existing)
- Reference pattern exists in wkmp-ap/src/db/settings.rs

### Phase 2: Specification Completeness Verification (COMPLETE)

**Deliverables:**
- 01_specification_issues.md - Completeness, ambiguity, consistency, testability analysis

**Quality Assessment:**
- **Specification Quality:** HIGH
- **Critical Issues:** 0
- **High-Priority Issues:** 0
- **Medium-Priority Issues:** 3 (TOML field preservation, test coverage metrics, Serialize trait)
- **Low-Priority Issues:** 5 (Windows NTFS, atomic rename, validation consistency, deferred items)

**Recommendation:** PROCEED to implementation (no blockers)

### Phase 3: Acceptance Test Definition (COMPLETE)

**Deliverables:**
- 02_test_specifications/test_index.md - 47 test cases organized by type
- tc_u_res_001_to_008.md - Multi-tier resolution unit tests
- tc_u_wb_001_to_006.md - Write-back behavior unit tests
- tc_u_val_001_to_003.md - Validation unit tests
- tc_u_toml_001_to_007.md - TOML utilities unit tests
- tc_u_db_001_to_002.md - Database accessor unit tests
- tc_u_sec_001_to_002.md - Security unit tests
- tc_i_e2e_001_to_004.md - End-to-end integration tests
- tc_i_ui_001_to_003.md - Web UI integration tests
- tc_i_recovery_001_to_002.md - Database recovery integration tests
- tc_i_concurrent_001.md - Concurrency integration test
- tc_s_workflow_001_to_003.md - User workflow system tests
- tc_m_migration_001_to_003.md - Migration manual tests
- tc_m_failure_001_to_003.md - Failure scenario manual tests
- traceability_matrix.md - 100% requirement coverage validation

**Coverage:**
- 62/62 requirements covered (100%)
- 21/21 acceptance criteria covered (100%)
- All test specifications <100 lines (modular structure)

### Phase 4: Approach Selection (To Be Implemented - Week 2)

**Planned Activities:**
- Select implementation approaches for resolver, TOML utilities, web UI
- Evaluate trade-offs (complexity, maintainability, testability)
- Document rationale per Risk-First Decision Framework

### Phase 5: Implementation Breakdown (To Be Implemented - Week 2)

**Planned Activities:**
- Break implementation into increments
- Define increment order (foundation → core → integration → UI)
- Create increment checklists with verification criteria

### Phase 6: Effort Estimation (To Be Implemented - Week 3)

**Planned Activities:**
- Estimate implementation time (design, coding, testing)
- Identify dependencies between increments
- Create timeline with milestones

### Phase 7: Risk Assessment (To Be Implemented - Week 3)

**Planned Activities:**
- Identify implementation risks (TOML round-trip, Windows atomic rename)
- Define mitigation strategies
- Assess residual risk

### Phase 8: Plan Documentation (To Be Implemented - Week 3)

**Planned Activities:**
- Finalize 00_PLAN_SUMMARY.md with full plan details
- Create implementation guide
- Document verification procedures

---

## Requirements Summary

**Total Requirements:** 62

**By Category:**
- Scope (3), Overview (3)
- Priority Resolution (5), Write-Back Behavior (4)
- TOML Persistence (5), Validation (2), Error Handling (3)
- Architecture (3), Generic Sync (3)
- AcoustID Specific (4), TOML Schema (2)
- Database Storage (3), Atomic TOML Write (2)
- Security (9), Web UI Integration (6)
- Logging and Observability (4)
- Testing Requirements (3), Migration (3)
- Future Extensions (5 - out of scope)

**Key Design Decisions:**
- Generic settings sync (Option B - HashMap interface) per user request
- Database-first priority (consistent with WKMP patterns)
- Best-effort TOML write (graceful degradation)
- Plain text storage (acceptable for read-only API keys)

---

## Test Summary

**Total Test Cases:** 47

**By Type:**
- Unit Tests: 28 (resolution, write-back, validation, TOML, DB, security)
- Integration Tests: 10 (end-to-end, web UI, recovery, concurrency)
- System Tests: 3 (user workflows)
- Manual Tests: 6 (migration, failure scenarios)

**Coverage Validation:**
- All 62 requirements traced to tests
- All 21 acceptance criteria covered
- No requirements with zero coverage
- Most requirements have 2+ tests (unit + integration or manual)

**Test Organization:**
- Modular structure (individual tc_*.md files <100 lines)
- Clear requirement traceability
- Setup/execution/verification format

---

## Dependencies

### External Dependencies (Existing - No Changes)

- toml (0.8.x) - TOML parsing/serialization
- serde (1.0.x) - Serialization framework
- sqlx (0.8.x) - Database access
- tokio (1.x) - Async runtime
- axum (0.7.x) - HTTP server

**No new external dependencies required.**

### Internal Dependencies (Verified Existing)

**wkmp-common:**
- src/config.rs - TomlConfig struct (extend with acoustid_api_key field)
- Utilities - Atomic file write (new), TOML read/write (new)

**wkmp-ai:**
- src/db/ - Database initialization (existing pattern)
- src/api/ - HTTP server (existing Axum setup)
- static/ - Web UI assets (existing directory)

**Database:**
- Settings table (existing, no schema changes)

**Reference Pattern:**
- wkmp-ap/src/db/settings.rs - Settings accessor pattern

---

## Specification Issues

### Critical Issues: 0

**No blockers to implementation.**

### High-Priority Issues: 0

**All high-risk requirements well-specified.**

### Medium-Priority Issues: 3

**SPEC-IMPL-001:** TOML schema Serialize trait
- **Resolution:** Code example is authoritative (includes Serialize)
- **Action:** None required

**SPEC-AMB-001:** TOML field preservation mechanism
- **Resolution:** Struct-based serialization preserves TomlConfig fields only
- **Action:** Document limitation (comments lost)

**SPEC-TEST-001:** Test coverage metrics not quantified
- **Resolution:** 100% requirement coverage target (implied by traceability matrix)
- **Action:** Clarified in Phase 3 (traceability matrix)

### Low-Priority Issues: 5

**SPEC-SEC-001:** Windows NTFS ACL behavior
- **Resolution:** Best-effort (no Windows-specific enforcement)
- **Action:** Document Windows security limitations

**SPEC-AMB-002:** "Valid key" definition for ENV/TOML
- **Resolution:** Apply APIK-VAL-010 validation to all sources
- **Action:** Clarify in implementation

**SPEC-IMPL-002:** Atomic rename on Windows
- **Resolution:** Best-effort approach (document limitation)
- **Action:** Note Windows rename limitations

**SPEC-DEFER-001:** Key format validation deferred
- **Resolution:** Acceptable design (AcoustID client has format knowledge)
- **Action:** None required

**SPEC-FUT-001:** Bulk settings sync details
- **Resolution:** Out of scope for PLAN012
- **Action:** Defer to future plan

---

## Implementation Approach (Preliminary)

**Based on dependencies, recommended implementation order:**

### Increment 1: wkmp-common TOML Utilities (Foundation)

**Files:**
- wkmp-common/src/config.rs (modify)
- wkmp-common/tests/toml_utils_tests.rs (new)

**Deliverables:**
- Extend TomlConfig struct (acoustid_api_key field)
- Implement write_toml_config() (atomic write)
- Implement set_unix_permissions_0600()
- Unit tests (tc_u_toml_001-007)

**Verification:**
- All TOML utility tests pass
- Round-trip serialization preserves data
- Atomic write creates temp file + renames

### Increment 2: wkmp-ai Database Accessors

**Files:**
- wkmp-ai/src/db/settings.rs (new or extend)
- wkmp-ai/tests/unit/db_settings_tests.rs (new)

**Deliverables:**
- Implement get_acoustid_api_key()
- Implement set_acoustid_api_key()
- Unit tests (tc_u_db_001-002)

**Verification:**
- Database read/write functions work
- Follows wkmp-ap/src/db/settings.rs pattern

### Increment 3: wkmp-ai Resolver and Sync

**Files:**
- wkmp-ai/src/config.rs (new or extend)
- wkmp-ai/tests/unit/config_tests.rs (new)

**Deliverables:**
- Implement resolve_acoustid_api_key() (multi-tier resolution)
- Implement sync_settings_to_toml() (generic HashMap-based)
- Implement validation (empty, whitespace, NULL)
- Unit tests (tc_u_res_001-008, tc_u_wb_001-006, tc_u_val_001-003, tc_u_sec_001-002)

**Verification:**
- All resolution unit tests pass
- All write-back unit tests pass
- All validation unit tests pass

### Increment 4: wkmp-ai Startup Integration

**Files:**
- wkmp-ai/src/main.rs (modify)
- wkmp-ai/tests/integration/api_key_resolution_tests.rs (new)

**Deliverables:**
- Integrate resolve_acoustid_api_key() at startup
- Add logging for resolution source and migration
- Integration tests (tc_i_e2e_001-004)

**Verification:**
- End-to-end startup tests pass
- Logs show resolution source
- Migration works (ENV/TOML → DB + TOML)

### Increment 5: Web UI Endpoint

**Files:**
- wkmp-ai/src/api/handlers.rs (extend)
- wkmp-ai/tests/integration/ui_endpoint_tests.rs (new)

**Deliverables:**
- Implement POST /api/settings/acoustid_api_key endpoint
- Integration tests (tc_i_ui_001-003)

**Verification:**
- Endpoint saves to database and TOML
- Validation rejects empty keys
- Success/error responses correct

### Increment 6: Web UI Settings Page

**Files:**
- wkmp-ai/static/settings.html (new)
- wkmp-ai/static/settings.css (new)
- wkmp-ai/static/settings.js (new)

**Deliverables:**
- Settings page at /settings
- Input field, save button, link to acoustid.org
- No key display (security)
- System tests (tc_s_workflow_001)

**Verification:**
- Settings page renders correctly
- Save functionality works
- Security verified (no key display)

### Increment 7: Testing and Documentation

**Files:**
- docs/IMPL012-acoustid_client.md (update)
- docs/IMPL001-database_schema.md (update)
- User guide (new section)

**Deliverables:**
- Manual testing (tc_m_migration_001-003, tc_m_failure_001-003)
- Integration tests (tc_i_recovery_001-002, tc_i_concurrent_001)
- System tests (tc_s_workflow_002-003)
- Documentation updates

**Verification:**
- All 47 tests pass
- 100% coverage verified
- Documentation accurate and complete

---

## Acceptance Criteria

**Implementation complete when all 21 criteria verified:**

1. ✅ Multi-tier resolution works (Database → ENV → TOML → Error)
2. ✅ Database is authoritative (ignores ENV/TOML when database has key)
3. ✅ Auto-migration works (ENV → Database + TOML)
4. ✅ Auto-migration works (TOML → Database)
5. ✅ TOML write-back works (ENV or UI update → Database + TOML)
6. ✅ TOML write is atomic (temp + rename)
7. ✅ TOML write preserves other fields
8. ✅ TOML permissions set to 0600 (Unix)
9. ✅ TOML write failures are graceful (warn, don't fail)
10. ✅ Generic settings sync supports multiple keys
11. ✅ Web UI endpoint works (POST /api/settings/acoustid_api_key)
12. ✅ Web UI settings page works
13. ✅ All unit tests pass (28 tests)
14. ✅ All integration tests pass (10 tests)
15. ✅ Manual testing complete (6 scenarios)
16. ✅ Documentation updated (IMPL012, IMPL001 reference)
17. ✅ Logging provides clear observability
18. ✅ Error messages list all 3 configuration methods
19. ✅ Security warnings for loose permissions work
20. ✅ Backward compatibility maintained
21. ✅ Extensibility to future API keys demonstrated

**Verification:** Each criterion traced to specific tests in traceability_matrix.md

---

## Risk Assessment (Preliminary)

### Specification Risks: LOW

**Mitigations:**
- Zero critical issues
- All requirements testable and traceable
- Specification quality: HIGH

### Implementation Risks: LOW

**Known Challenges:**
- TOML round-trip serialization (may lose comments) - MITIGATED by testing
- Windows atomic rename edge cases - MITIGATED by best-effort approach
- NTFS ACL behavior - DOCUMENTED limitation

**Risk Factors (All Low):**
- Established patterns (database accessors, HTTP endpoints)
- Standard libraries (no novel dependencies)
- Comprehensive test coverage (100% requirements)

### Residual Risks: LOW

**Acceptable limitations:**
- TOML comments/unknown fields lost on write (documented)
- Windows security weaker than Unix (best-effort)
- Key format validation deferred to client (design decision)

---

## Next Steps

### For User (Now)

1. **Review Phase 1-3 Deliverables:**
   - requirements_index.md (62 requirements)
   - scope_statement.md (in/out scope)
   - dependencies_map.md (dependency graph)
   - 01_specification_issues.md (quality assessment)
   - 02_test_specifications/ (47 tests, 100% coverage)
   - traceability_matrix.md (coverage validation)

2. **Approve or Request Changes:**
   - If approved: Proceed to Phase 4 (Approach Selection)
   - If changes needed: Address specification issues

### For Implementation (Week 2-3)

3. **Phase 4: Approach Selection**
   - Evaluate implementation approaches
   - Document decisions per Risk-First Framework

4. **Phase 5: Implementation Breakdown**
   - Create detailed increment checklists
   - Define verification criteria

5. **Phase 6: Effort Estimation**
   - Estimate time per increment
   - Create timeline

6. **Phase 7: Risk Assessment**
   - Identify implementation risks
   - Define mitigation strategies

7. **Phase 8: Plan Finalization**
   - Complete 00_PLAN_SUMMARY.md
   - Create implementation guide

### For Execution (Post-Planning)

8. **Implementation** (following increment order)
9. **Testing** (unit → integration → system → manual)
10. **Documentation** (update IMPL012, IMPL001, user guide)
11. **Code Review** (verify all requirements satisfied)
12. **/commit** (with change_history.md update)

---

## Document Index

**Start Here:**
- 00_PLAN_SUMMARY.md (this file) - Overview and navigation

**Phase 1 - Scope:**
- requirements_index.md - All 62 requirements extracted
- scope_statement.md - In/out scope, assumptions, constraints
- dependencies_map.md - Dependency graph and implementation order

**Phase 2 - Verification:**
- 01_specification_issues.md - Quality assessment (0 critical, 0 high)

**Phase 3 - Testing:**
- 02_test_specifications/test_index.md - 47 tests organized by type
- 02_test_specifications/tc_u_*.md - Unit test specifications
- 02_test_specifications/tc_i_*.md - Integration test specifications
- 02_test_specifications/tc_s_*.md - System test specifications
- 02_test_specifications/tc_m_*.md - Manual test specifications
- 02_test_specifications/traceability_matrix.md - 100% coverage validation

**Phases 4-8:** To be created during Week 2-3

---

## Success Metrics

**Planning Phase (Complete):**
- ✅ All requirements extracted (62/62)
- ✅ Scope clearly defined (in/out, assumptions, constraints)
- ✅ Dependencies mapped (existing code verified)
- ✅ Specification quality assessed (HIGH, 0 critical issues)
- ✅ All tests defined (47 tests, 100% coverage)
- ✅ Traceability matrix complete (62/62 requirements → tests)

**Implementation Phase (Pending):**
- All 47 tests pass
- All 21 acceptance criteria verified
- 100% requirement satisfaction
- Zero regressions in existing functionality
- Documentation updated and accurate

---

## Estimated Effort (Preliminary)

**Planning:** 4-6 hours (Phases 1-3 complete)
**Implementation:** TBD (Phases 4-8)
**Testing:** TBD (Phases 4-8)
**Documentation:** TBD (Phases 4-8)

**Note:** Detailed effort estimation in Phase 6 (Week 3)

---

**Plan Status:** Phases 1-3 COMPLETE, Ready for Review
**Next Action:** User review and approval to proceed to Phase 4
**Plan Quality:** HIGH (comprehensive scope, 100% test coverage, low risk)

---

**END OF PLAN SUMMARY**
