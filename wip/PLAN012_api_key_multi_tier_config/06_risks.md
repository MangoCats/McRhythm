# PLAN012 - Phase 7: Risk Assessment and Mitigation Planning

**Specification:** SPEC025-api_key_configuration.md
**Plan Version:** 1.0
**Date:** 2025-10-30

---

## Executive Summary

**Total Risks Identified:** 12
**Risk Distribution:**
- HIGH Risk: 0
- MEDIUM Risk: 3 (before mitigation)
- LOW Risk: 9

**Overall Residual Risk:** LOW (all MEDIUM risks mitigated to LOW or LOW-MEDIUM)

**Critical Risks:** None (no blockers to implementation)

---

## Risk Categories

1. **Technical Risks** - Implementation challenges, platform issues
2. **Integration Risks** - Cross-module interactions, existing code conflicts
3. **Testing Risks** - Test coverage gaps, environment setup
4. **Schedule Risks** - Timeline delays, dependency blocks

---

## Risk Register

### Technical Risks

#### RISK-001: TOML Round-Trip Serialization Data Loss

**Category:** Technical
**Phase:** Increment 2 (TOML utilities)

**Description:** TOML serialization may lose comments or unknown fields during write-back, frustrating users who manually edited config files.

**Likelihood:** MEDIUM (toml crate behavior well-documented but lossy)
**Impact:** LOW (documented limitation, acceptable trade-off)
**Risk Level:** MEDIUM × LOW = **MEDIUM**

**Mitigation Strategies:**
1. Document limitation explicitly in IMPL012 (comments lost on write-back)
2. Unit test tc_u_toml_003 verifies known fields preserved
3. Struct-based serialization preserves TomlConfig fields (not arbitrary TOML)
4. Add warning in documentation: "Manual TOML edits outside schema may be lost"

**Residual Risk:** LOW (documented and tested, users informed)

---

#### RISK-002: Windows Atomic Rename Failure

**Category:** Technical
**Phase:** Increment 2 (TOML utilities)

**Description:** std::fs::rename may fail on Windows if target file exists, leading to incomplete writes.

**Likelihood:** LOW (rare race condition, wkmp-ai single instance)
**Impact:** MEDIUM (TOML write fails, warning logged, database write succeeds)
**Risk Level:** LOW × MEDIUM = **LOW-MEDIUM**

**Mitigation Strategies:**
1. Best-effort approach already specified (APIK-WB-040)
2. TOML write failure logged as warning, doesn't block operation
3. Database write is authoritative (TOML write is backup only)
4. Document Windows limitation in implementation notes
5. Manual test tc_m_failure_001 verifies graceful degradation

**Residual Risk:** VERY LOW (acceptable limitation, well-handled)

---

#### RISK-003: Unix File Permission Edge Cases

**Category:** Technical
**Phase:** Increment 2 (TOML utilities)

**Description:** Setting 0600 permissions may fail on unusual filesystems (NFS, FUSE) or read-only mounts.

**Likelihood:** LOW (most deployments use standard filesystems)
**Impact:** LOW (warning logged, operation continues)
**Risk Level:** LOW × LOW = **LOW**

**Mitigation Strategies:**
1. Best-effort permission setting (no operation failure)
2. Error logged as warning if permission set fails
3. check_toml_permissions_loose() detects misconfigurations
4. Manual test tc_m_failure_002 verifies warning behavior

**Residual Risk:** VERY LOW (edge case, gracefully handled)

---

#### RISK-004: Environment Variable Visibility

**Category:** Technical (Security)
**Phase:** Increment 4 (resolver)

**Description:** ENV variables are visible to all processes, potentially exposing API key to malicious software.

**Likelihood:** LOW (API keys are read-only, low security risk)
**Impact:** LOW (read-only API key, acceptable exposure)
**Risk Level:** LOW × LOW = **LOW**

**Mitigation Strategies:**
1. Document security consideration in IMPL012
2. Auto-migration encourages moving to database + TOML (more secure)
3. Recommend web UI or TOML for production (ENV for CI/CD only)
4. Plain text storage acceptable per APIK-FUT-050 (read-only keys)

**Residual Risk:** VERY LOW (documented, user choice)

---

### Integration Risks

#### RISK-005: wkmp-common Breaking Changes

**Category:** Integration
**Phase:** Increment 2 (TOML utilities)

**Description:** Changes to wkmp-common (Serialize trait, new functions) may break other modules (wkmp-ap, wkmp-ui).

**Likelihood:** LOW (additive changes only, backward compatible)
**Impact:** MEDIUM (if breaks, other modules fail to compile)
**Risk Level:** LOW × MEDIUM = **LOW-MEDIUM**

**Mitigation Strategies:**
1. Additive changes only (extend TomlConfig, add new functions)
2. Existing wkmp-common tests continue to pass (regression check)
3. Serialize trait addition is non-breaking (existing code unaffected)
4. Test all modules compile after wkmp-common changes
5. Semantic versioning (major version bump if breaking change needed)

**Residual Risk:** VERY LOW (careful API design, comprehensive testing)

---

#### RISK-006: Startup Sequence Integration Conflicts

**Category:** Integration
**Phase:** Increment 6 (startup integration)

**Description:** Adding resolver to wkmp-ai startup may conflict with existing initialization order (database, TOML loading).

**Likelihood:** MEDIUM (startup code is complex, order-sensitive)
**Impact:** MEDIUM (startup fails, module unusable)
**Risk Level:** MEDIUM × MEDIUM = **MEDIUM**

**Mitigation Strategies:**
1. Review existing main.rs before integration (understand current flow)
2. Add resolver after database initialization, before client creation
3. Integration tests tc_i_e2e_001-004 verify startup behavior
4. Checkpoint 3 ensures startup working before proceeding
5. Rollback plan defined (revert main.rs changes)

**Residual Risk:** LOW (careful integration, comprehensive testing)

---

#### RISK-007: Database Migration Schema Conflicts

**Category:** Integration
**Phase:** Increment 3 (DB accessors)

**Description:** Settings table may not exist or have different schema in older WKMP installations.

**Likelihood:** VERY LOW (settings table exists per IMPL001)
**Impact:** HIGH (database operations fail, module unusable)
**Risk Level:** VERY LOW × HIGH = **LOW**

**Mitigation Strategies:**
1. Verify settings table exists (migrations/up.sql already has it)
2. No schema changes required (APIK-DB-030)
3. Use existing key-value pattern (consistent with wkmp-ap)
4. Integration tests run migrations before testing

**Residual Risk:** VERY LOW (no schema changes, existing pattern)

---

### Testing Risks

#### RISK-008: Cross-Module Test Coverage Gaps

**Category:** Testing
**Phase:** Increment 9 (integration tests)

**Description:** Unit tests pass in isolation, but cross-module interactions (wkmp-common ↔ wkmp-ai) fail at runtime.

**Likelihood:** LOW (integration tests defined: tc_i_e2e_001-004)
**Impact:** MEDIUM (runtime failures in production)
**Risk Level:** LOW × MEDIUM = **LOW-MEDIUM**

**Mitigation Strategies:**
1. Comprehensive integration tests (tc_i_e2e_*, tc_i_ui_*, tc_i_recovery_*)
2. System tests verify end-to-end workflows (tc_s_workflow_001-003)
3. Manual tests catch edge cases (tc_m_migration_*, tc_m_failure_*)
4. Checkpoint 3 ensures integration working before finalization
5. 100% requirement coverage (62/62 requirements → tests)

**Residual Risk:** VERY LOW (comprehensive test coverage)

---

#### RISK-009: Platform-Specific Test Failures

**Category:** Testing
**Phase:** Increments 2, 9 (TOML utils, integration tests)

**Description:** Tests pass on Linux but fail on Windows (permissions, file paths, line endings).

**Likelihood:** LOW (conditional compilation, platform-aware tests)
**Impact:** LOW (Windows-specific behavior documented as best-effort)
**Risk Level:** LOW × LOW = **LOW**

**Mitigation Strategies:**
1. Conditional compilation: #[cfg(unix)] for permission tests
2. Windows-specific tests: tc_u_toml_005 (graceful no-op)
3. Manual test tc_m_failure_002 verifies both platforms
4. Best-effort approach documented (Windows limitations acceptable)
5. CI/CD can run tests on multiple platforms (if available)

**Residual Risk:** VERY LOW (platform-aware design)

---

#### RISK-010: Test Environment Setup Complexity

**Category:** Testing
**Phase:** Increment 9 (integration tests)

**Description:** Integration tests require complex setup (database, TOML files, ENV variables), increasing flakiness.

**Likelihood:** LOW (use in-memory DB, temp directories)
**Impact:** LOW (test failures delay verification, but recoverable)
**Risk Level:** LOW × LOW = **LOW**

**Mitigation Strategies:**
1. Use in-memory SQLite (:memory:) for unit/integration tests
2. Use tempfile crate for temporary TOML files (auto-cleanup)
3. ENV variable cleanup in test fixtures (isolation)
4. Integration tests use independent test data (no sharing)
5. Clear setup/teardown patterns in test code

**Residual Risk:** VERY LOW (isolated test environments)

---

### Schedule Risks

#### RISK-011: Dependency Blocking

**Category:** Schedule
**Phase:** All increments

**Description:** Sequential dependencies block progress (e.g., Increment 4 cannot start until Increment 3 complete).

**Likelihood:** MEDIUM (many sequential dependencies)
**Impact:** LOW (timeline includes sequential time, not overly optimistic)
**Risk Level:** MEDIUM × LOW = **MEDIUM**

**Mitigation Strategies:**
1. Dependency analysis done upfront (Phase 6: Timeline)
2. Identify parallel opportunities (Increment 3 || Increment 2)
3. Modular increments allow early detection of issues
4. Checkpoints every 5-10 increments ensure progress
5. Rollback plans defined per increment

**Residual Risk:** LOW (dependencies managed, timeline realistic)

---

#### RISK-012: Manual Testing Delays

**Category:** Schedule
**Phase:** Increment 10 (manual tests)

**Description:** Manual testing (6 scenarios) takes longer than estimated, delaying finalization.

**Likelihood:** LOW (manual tests well-defined, execution straightforward)
**Impact:** LOW (extra 1-2 hours, within contingency)
**Risk Level:** LOW × LOW = **LOW**

**Mitigation Strategies:**
1. Manual test procedures documented in Increment 10
2. Automated tests (41 tests) catch most issues before manual testing
3. Contingency included (15% for Increment 10)
4. Manual tests can run in parallel (different scenarios)

**Residual Risk:** VERY LOW (low probability, low impact)

---

## Risk Summary Table

| Risk ID | Category | Description | Likelihood | Impact | Risk Level | Residual Risk |
|---------|----------|-------------|------------|--------|------------|---------------|
| RISK-001 | Technical | TOML data loss | MEDIUM | LOW | MEDIUM | LOW |
| RISK-002 | Technical | Windows rename | LOW | MEDIUM | LOW-MEDIUM | VERY LOW |
| RISK-003 | Technical | Unix permissions | LOW | LOW | LOW | VERY LOW |
| RISK-004 | Technical | ENV visibility | LOW | LOW | LOW | VERY LOW |
| RISK-005 | Integration | wkmp-common breaking | LOW | MEDIUM | LOW-MEDIUM | VERY LOW |
| RISK-006 | Integration | Startup conflicts | MEDIUM | MEDIUM | MEDIUM | LOW |
| RISK-007 | Integration | Schema conflicts | VERY LOW | HIGH | LOW | VERY LOW |
| RISK-008 | Testing | Test coverage gaps | LOW | MEDIUM | LOW-MEDIUM | VERY LOW |
| RISK-009 | Testing | Platform failures | LOW | LOW | LOW | VERY LOW |
| RISK-010 | Testing | Test setup | LOW | LOW | LOW | VERY LOW |
| RISK-011 | Schedule | Dependency blocking | MEDIUM | LOW | MEDIUM | LOW |
| RISK-012 | Schedule | Manual test delays | LOW | LOW | LOW | VERY LOW |

---

## Risk Mitigation Priority

### Priority 1: Address Before Implementation (CRITICAL)

**None** - All risks have acceptable residual risk levels

---

### Priority 2: Address During Implementation (HIGH)

**RISK-006: Startup Sequence Integration Conflicts**
- **When:** Increment 6 (startup integration)
- **Action:** Review main.rs thoroughly before integration
- **Verification:** Integration tests tc_i_e2e_001-004 pass

---

### Priority 3: Monitor During Implementation (MEDIUM)

**RISK-001: TOML Round-Trip Serialization Data Loss**
- **When:** Increment 2 (TOML utilities)
- **Action:** Test tc_u_toml_003 and tc_u_toml_006 extensively
- **Verification:** Documentation updated with limitation

**RISK-011: Dependency Blocking**
- **When:** All increments
- **Action:** Follow dependency order strictly, use checkpoints
- **Verification:** Timeline adherence

---

### Priority 4: Accept (LOW)

All other risks (RISK-002, 003, 004, 005, 007, 008, 009, 010, 012)
- **Rationale:** Low residual risk, well-mitigated, acceptable trade-offs

---

## Risk Monitoring Plan

### Per-Increment Risk Checks

**Increment 2 (TOML Utilities):**
- Monitor: RISK-001 (data loss), RISK-002 (Windows rename), RISK-003 (permissions)
- Verify: All tc_u_toml_* tests pass on both Unix and Windows

**Increment 6 (Startup Integration):**
- Monitor: RISK-006 (startup conflicts)
- Verify: Integration tests tc_i_e2e_001-004 pass

**Increment 9 (Integration Tests):**
- Monitor: RISK-008 (test coverage), RISK-009 (platform tests), RISK-010 (test setup)
- Verify: All integration tests pass, no flakiness

---

### Checkpoint Risk Reviews

**Checkpoint 1 (After Increment 2):**
- Review RISK-001, 002, 003, 005 (TOML and wkmp-common risks)
- Verify: wkmp-common tests pass, no regressions

**Checkpoint 2 (After Increment 5):**
- Review RISK-004 (ENV visibility), RISK-011 (dependencies)
- Verify: Core logic complete, no blocking issues

**Checkpoint 3 (After Increment 9):**
- Review RISK-006 (startup), RISK-007 (schema), RISK-008 (coverage)
- Verify: Integration complete, all automated tests pass

**Checkpoint 4 (After Increment 10):**
- Review RISK-012 (manual tests)
- Verify: All 47 tests pass, 100% coverage achieved

---

## Contingency Triggers

### Trigger 1: Test Failure Rate >10%

**Condition:** More than 4-5 tests fail during any increment
**Response:**
1. STOP implementation
2. Root cause analysis (why multiple failures?)
3. Review design assumptions
4. Add extra testing increment if needed
5. Re-estimate timeline

---

### Trigger 2: Critical Integration Issue

**Condition:** wkmp-common changes break other modules
**Response:**
1. ROLLBACK wkmp-common changes
2. Create bugfix increment
3. Add regression tests
4. Re-verify all modules compile and test

---

### Trigger 3: Checkpoint Failure

**Condition:** Any checkpoint fails verification
**Response:**
1. STOP implementation (do not proceed to next increment)
2. Create bugfix increment(s)
3. Re-run checkpoint verification
4. Update risk assessment if new risks discovered

---

## Overall Risk Assessment

**Residual Risk Level:** LOW

**Confidence:** HIGH (80-90%)

**Risk Factors:**
- 0 HIGH risks (no blockers)
- 3 MEDIUM risks mitigated to LOW or LOW-MEDIUM
- 9 LOW risks with acceptable residual risk

**Acceptable for Implementation:** YES

**Conditions:**
- Follow increment order (respect dependencies)
- Run checkpoints as defined
- Monitor risks during implementation
- Address integration conflicts proactively (RISK-006)

---

## Risk-First Framework Alignment

**Primary Decision Factor: Risk**
- Approach B selected (Phase 4) specifically for LOW residual risk
- All MEDIUM risks mitigated to LOW or LOW-MEDIUM
- No HIGH risks identified

**Implementation Approach:**
- Test-first methodology (47 tests defined before coding)
- Incremental delivery (early detection of issues)
- Comprehensive checkpoints (4 verification points)

**Risk Reduction Strategies:**
- 100% requirement coverage (no missed requirements)
- Integration tests for cross-module interactions
- Platform-aware design (Unix/Windows differences handled)
- Best-effort error handling (graceful degradation)

---

**Phase 7 (Risk Assessment):** COMPLETE
**Overall Residual Risk:** LOW (acceptable for implementation)
**Next Phase:** Phase 8 - Plan Documentation and Finalization
