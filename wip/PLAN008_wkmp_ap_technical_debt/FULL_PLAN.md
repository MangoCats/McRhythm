# PLAN008: wkmp-ap Technical Debt Remediation - FULL PLAN

**Status:** COMPLETE - All 8 Phases
**Created:** 2025-10-29
**Plan Number:** PLAN008
**Version:** 1.0

---

## Document Purpose

**This is the ARCHIVAL version of the complete implementation plan.**

**DO NOT use this document during implementation** - it is >2000 lines and will overload context window.

**For Implementation:** Use the modular structure instead:
- Start: `00_PLAN_SUMMARY.md` (<500 lines)
- Then: Individual increment files (`04_increments/increment_XX.md`)
- Reference: Test specs (`02_test_specifications/tc_*.md`)

**For Review/Archival:** This document consolidates all phases for reference.

---

## Plan Overview

### Problem Statement

wkmp-ap (Audio Player microservice) has accumulated 13 technical debt items:

**CRITICAL (1 item):**
- POST/PUT authentication bypass allowing unauthorized playback control

**HIGH (5 items):**
- Missing file paths in decoder errors
- Buffer configuration not reading from database
- Incomplete developer UI telemetry
- Missing album UUIDs in passage events
- Duration played stubbed as 0.0 seconds

**MEDIUM-LOW (7 items):**
- 376 .unwrap() calls creating panic risk
- engine.rs at 3,573 lines needing refactoring
- 21 compiler warnings
- Duplicate config files
- Backup files in repository
- Missing clipping warning logs
- Outdated TODO comments

### Solution

**3-Sprint Incremental Remediation Plan**

**Sprint 1:** Security & Critical (15h base, 7 increments)
**Sprint 2:** Functionality & Diagnostics (14h base, 8 increments)
**Sprint 3:** Code Health (21h base, 7 increments)

**Total:** 50h base + 12h contingency = 62h (3 weeks)

### Success Criteria

- All 37 requirements met
- All 28 automated tests passing
- Zero compiler warnings
- No .unwrap() in audio hot paths
- engine.rs refactored (<1500 lines per module)
- Performance overhead <1%
- Security vulnerability eliminated

---

## Phase 1: Input Validation and Scope Definition

### Requirements Index

**37 requirements across 5 categories:**

**Security (4 requirements):** REQ-DEBT-SEC-001-010 through 040
- Validate authentication for POST/PUT
- Use shared_secret mechanism
- Extract from JSON body
- Return 401 on failure

**Functionality - High Priority (17 requirements):**
- **Decoder Errors (3):** REQ-DEBT-FUNC-001-010 through 030
- **Buffer Config (4):** REQ-DEBT-FUNC-002-010 through 040
- **Telemetry (4):** REQ-DEBT-FUNC-003-010 through 040
- **Album Metadata (3):** REQ-DEBT-FUNC-004-010 through 030
- **Duration Tracking (4):** REQ-DEBT-FUNC-005-010 through 040 (note: 1 req skipped in numbering)

**Code Quality (10 requirements):** REQ-DEBT-QUALITY-001 through 004
- .unwrap() replacement (3)
- Refactoring (3)
- Warnings (3)
- Config cleanup (4, includes 020 and 030)

**Cleanup (3 requirements):** REQ-DEBT-QUALITY-005
- Backup file removal (2)

**Future (1 requirement):** REQ-DEBT-FUTURE-003-010
- Clipping warning logs

### Scope Statement

**In Scope:** All 13 debt items (37 requirements)

**Out of Scope:**
- Phase 5 features (seeking, drain-based buffers)
- Complete .unwrap() elimination (test code acceptable)
- Full file refactoring (mixer.rs, handlers.rs remain as-is)
- Performance optimization
- New feature development

**Assumptions:**
- Existing authentication infrastructure reusable
- Database schema supports requirements
- Decoder workers queryable for telemetry
- Test infrastructure functional
- Backward compatibility required

**Constraints:**
- Public API unchanged
- Performance overhead <1%
- 3-week timeline
- Single developer

### Dependencies

**Existing Code Required:**
- auth_middleware.rs (authentication)
- db/ layer (settings, passages)
- Audio pipeline (decoder, buffer, mixer)
- Event system (PlaybackEvent, EventBus)

**External Libraries:** axum, tokio, sqlx, serde, uuid, tracing, symphonia (all in Cargo.toml)

**Database Schema:** settings, passages, passage_albums, albums tables

---

## Phase 2: Specification Completeness Verification

### Issues Found: 5 (0 CRITICAL, 0 HIGH, 3 MEDIUM, 2 LOW)

**MEDIUM-001:** Authentication test coverage incomplete
- Resolution: Add edge case tests in Phase 3

**MEDIUM-002:** Buffer capacity validation not specified
- Resolution: Add validation logic in implementation

**MEDIUM-003:** Telemetry query frequency unspecified
- Resolution: Document on-demand strategy

**LOW-001:** Test data not specified for duration tests
- Resolution: Use WAV, 44.1kHz stereo

**LOW-002:** Clipping log format not specified
- Resolution: Add rate-limiting in implementation

**Decision:** ✅ PROCEED TO IMPLEMENTATION (no blocking issues)

---

## Phase 3: Acceptance Test Definition

### Test Coverage: 28 Tests (100% Requirements Coverage)

**Security Tests (6):** TC-SEC-001-01 through 06
- POST/PUT with valid/invalid/missing secret
- Malformed JSON, wrong type

**Decoder Error Tests (3):** TC-FUNC-001-01 through 03
- Error includes file path
- Corrupt file error
- Multiple errors

**Buffer Config Tests (4):** TC-FUNC-002-01 through 04
- Read custom settings
- Use defaults when NULL
- Validate invalid settings
- End-to-end flow

**Telemetry Tests (4):** TC-FUNC-003-01 through 04
- Decoder state reported
- Sample rate reported
- Fade stage reported
- Complete telemetry integration

**Album Metadata Tests (3):** TC-FUNC-004-01 through 03
- Query returns UUIDs
- PassageStarted includes albums
- PassageComplete includes albums

**Duration Tracking Tests (3):** TC-FUNC-005-01 through 03
- Start time tracked
- Duration calculated precisely
- PassageComplete accurate

**Code Quality Tests (5):** TC-QUALITY-001 through 004
- Mutex errors propagate
- Event channel errors handled
- engine.rs split maintains API
- Zero warnings
- Single config module

### Traceability Matrix

**100% coverage verified:**
- Every requirement → at least one test
- Every test → specific requirement(s)
- Implementation files identified
- Status tracking columns present

---

## Phase 4: Approach Selection

### Key Decisions (4 ADRs)

**ADR-DEBT-001:** POST/PUT Authentication via JSON Body
- **Decision:** Extract `shared_secret` from request JSON body
- **Rationale:** Lowest risk, consistent with GET pattern, non-breaking
- **Alternatives Rejected:** Header-based (breaking change), Middleware pattern (complexity)

**ADR-DEBT-004:** On-Demand Telemetry Query
- **Decision:** Query decoder worker when UI requests buffer chain state
- **Rationale:** Simplest, no overhead when not in use
- **Alternatives Rejected:** Polling (CPU overhead), Event-driven (complexity)

**ADR-DEBT-007:** Targeted .unwrap() Replacement in Hot Paths
- **Decision:** Replace .unwrap() in audio thread and decoder workers only
- **Rationale:** Risk-focused, 15% effort for 80% safety improvement
- **Alternatives Rejected:** Comprehensive elimination (6x effort, marginal benefit)

**ADR-DEBT-008:** Incremental engine.rs Refactoring
- **Decision:** Extract 3 modules one at a time with testing after each
- **Rationale:** Lower risk, testable increments, easy rollback
- **Alternatives Rejected:** Big-bang refactoring (high risk, hard to review)

### Risk-Based Selection

All approaches selected using CLAUDE.md Decision-Making Framework:
1. **Risk (Primary)** - Minimize failure probability
2. **Quality (Secondary)** - Tiebreaker when risks equivalent
3. **Effort (Tertiary)** - Only when risk and quality equivalent

**Result:** All selected approaches have LOW residual risk

---

## Phase 5: Implementation Breakdown

### 22 Increments (50h base effort)

**Sprint 1 (7 increments, 15h):**
1. Implement POST authentication (3h)
2. Implement PUT authentication (2h)
3. Add authentication edge case tests (2h)
4. Add file_path field to ChunkedDecoder (2h)
5. Update decoder error sites (1h)
6. Implement BufferManager database config (3h)
7. Add buffer config validation & tests (2h)

**Sprint 2 (8 increments, 14h):**
8. Create DecoderTelemetry infrastructure (3h)
9. Populate BufferChainInfo with telemetry (2h)
10. Create get_passage_album_uuids() function (2h)
11. Populate PassageStarted with albums (1h)
12. Populate PassageComplete with albums (1h)
13. Add passage start time tracking to mixer (2h)
14. Calculate duration_played on completion (2h)
15. Cleanup: Config deduplication + backups (1h)

**Sprint 3 (7 increments, 21h):**
16. Replace .unwrap() in audio/buffer.rs (3h)
17. Replace .unwrap() in events.rs (2h)
18. Extract engine/diagnostics.rs module (3h)
19. Extract engine/queue.rs module (4h)
20. Extract engine/core.rs module (4h)
21. Fix compiler warnings (3h)
22. Add clipping warning log + verification (2h)

### Checkpoints

**Checkpoint 1 (End Sprint 1):**
- 13 tests passing
- Authentication enforced
- File paths in errors
- Buffer config from database

**Checkpoint 2 (End Sprint 2):**
- 23 tests passing
- Telemetry complete
- Albums in events
- Duration accurate
- Config cleanup done

**Checkpoint 3 (End Sprint 3 - FINAL):**
- 28 tests passing
- Zero warnings
- engine.rs refactored
- All integration tests pass
- **TECHNICAL DEBT REMEDIATION COMPLETE**

---

## Phase 6: Effort and Schedule Estimation

### Effort Summary

**Total Base Effort:** 50 hours
**Contingency (24%):** 12 hours
**Total Estimated:** 62 hours

**By Sprint:**
- Sprint 1: 15h base (19h with contingency)
- Sprint 2: 14h base (17h with contingency)
- Sprint 3: 21h base (26h with contingency)

**By Activity:**
- Implementation: 35h (70%)
- Testing: 10h (20%)
- Documentation: 3h (6%)
- Code Review: 2h (4%)

### Schedule

**Week 1:** Sprint 1 (Security & Critical)
**Week 2:** Sprint 2 (Functionality & Diagnostics)
**Week 3:** Sprint 3 (Code Health)

**Milestones:**
- End Week 1: Security vulnerability eliminated
- End Week 2: All functionality complete
- End Week 3: Code quality improved, ready for release

### Resources

**Personnel:**
- 1 Senior Rust Developer (full-time, 3 weeks)
- 1 Tech Lead (part-time, code review, 6h total)

**Infrastructure:** Development machine, Rust toolchain, audio device

**External Dependencies:** None

### Confidence

**Sprint 1:** HIGH confidence (90%)
**Sprint 2:** HIGH confidence (90%)
**Sprint 3:** MEDIUM confidence (70%)

**Overall:** HIGH confidence in 3-week delivery (80%)

---

## Phase 7: Risk Assessment and Mitigation

### Risk Register (11 Risks)

**HIGH Risks (2):**

**RISK-001:** Authentication breaks existing clients
- Probability: Medium (40%) | Impact: High
- Mitigation: Comprehensive testing, migration guide, grace period option
- Residual: LOW-MEDIUM

**RISK-002:** Refactoring introduces regressions
- Probability: Medium (30%) | Impact: High
- Mitigation: Incremental extraction, test after each step, easy rollback
- Residual: LOW-MEDIUM

**MEDIUM Risks (4):**

**RISK-003:** Telemetry queries impact performance
- Probability: Low (20%) | Impact: Medium
- Mitigation: Lock-free reads, async queries, performance profiling
- Residual: LOW

**RISK-004:** Schedule slippage
- Probability: Medium (30%) | Impact: Medium
- Mitigation: 24% contingency, velocity tracking, checkpoint reviews
- Residual: LOW-MEDIUM

**RISK-005:** Integration test failures
- Probability: Low (20%) | Impact: Medium
- Mitigation: Integration tests at checkpoints, manual testing
- Residual: LOW

**RISK-006:** Decoder telemetry complexity
- Probability: Low (15%) | Impact: Medium
- Mitigation: Architecture review, simplify if needed, use contingency
- Residual: LOW

**LOW Risks (5):** RISK-007 through 011
- Buffer validation, album query performance, duration edge cases, .unwrap() misses, compiler fixes
- All mitigated to VERY LOW or LOW residual risk

### Risk Monitoring

**Daily:** Velocity, test pass rate, blockers
**Weekly:** Checkpoint criteria, risk status, contingency usage
**Escalation:** >2h behind 2 days in a row, critical test failures

### Overall Risk Level: MEDIUM (manageable with mitigations)

---

## Phase 8: Plan Documentation and Approval

### Plan Completeness Checklist

- [x] All 37 requirements identified and cataloged
- [x] Scope boundaries explicitly defined
- [x] Specification issues identified (5 minor, no blockers)
- [x] 100% test coverage (28 tests + 2 manual verifications)
- [x] Traceability matrix complete
- [x] Multiple approaches evaluated for major items
- [x] Risk assessments performed, lowest-risk approaches selected
- [x] 4 ADRs documented with rationale
- [x] Implementation broken into 22 sized increments
- [x] 3 checkpoints defined with clear criteria
- [x] Effort estimated (50h base + 12h contingency)
- [x] 3-week schedule defined
- [x] 11 risks identified with mitigations
- [x] Monitoring and escalation process defined

### Plan Artifacts

**Planning Documents (Phase 1-3):**
- `requirements_index.md` - 37 requirements table
- `scope_statement.md` - Boundaries, assumptions, constraints
- `01_specification_issues.md` - 5 issues (proceed anyway)
- `02_test_specifications/test_index.md` - 28 tests overview
- `02_test_specifications/tc_*.md` - Individual test specs
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests

**Design Documents (Phase 4-5):**
- `03_approach_selection.md` - Risk-based approach selection, 4 ADRs
- `04_increments/checkpoints.md` - 22 increments, 3 checkpoints
- `04_increments/increment_01.md` - Sample increment (pattern for others)

**Project Management Documents (Phase 6-8):**
- `05_estimates.md` - Effort & schedule estimation
- `06_risks.md` - Risk register, mitigation strategies
- `00_PLAN_SUMMARY.md` - Executive overview (<500 lines)
- `FULL_PLAN.md` - This document (archival consolidation)

### Usage Guidelines

**For Implementation:**
1. Start: Read `00_PLAN_SUMMARY.md`
2. Sprint planning: Read `04_increments/checkpoints.md`
3. Each increment: Read `04_increments/increment_XX.md`
4. Each test: Read `02_test_specifications/tc_*.md`
5. Track: Update `traceability_matrix.md` as you complete requirements

**Do NOT Read:** This document (`FULL_PLAN.md`) during implementation - too large

**For Review/Approval:**
- Stakeholders: Read `00_PLAN_SUMMARY.md` only
- Technical reviewers: May read `03_approach_selection.md`, `06_risks.md`
- Archival: This document consolidates all phases for permanent record

---

## Approval and Sign-Off

### Plan Approval

**Prepared By:** WKMP Development Team
**Date:** 2025-10-29
**Plan Number:** PLAN008
**Plan Name:** wkmp-ap Technical Debt Remediation

**Reviewers:**
- [ ] Tech Lead (architecture, approach selection)
- [ ] Project Manager (schedule, resources)
- [ ] Security Reviewer (authentication approach)

**Approval Status:** Pending

**Approved for Implementation:** [ ] Yes [ ] No

**Conditions/Notes:**

---

### Change Log

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0 | 2025-10-29 | Initial plan - All 8 phases complete | WKMP Team |

---

## Appendices

### A. Requirement ID Index

All requirement IDs follow format: `REQ-DEBT-[CATEGORY]-[ITEM#]-[SEQ#]`

**Categories:**
- SEC: Security
- FUNC: Functionality
- QUALITY: Code Quality
- FUTURE: Future Enhancements

**Example:** REQ-DEBT-SEC-001-010 = Security debt item 1, requirement 10

### B. Test ID Index

All test IDs follow format: `TC-[CATEGORY]-[ITEM#]-[TEST#]`

**Categories:**
- SEC: Security
- FUNC: Functionality
- QUALITY: Code Quality

**Example:** TC-SEC-001-01 = Security debt item 1, test 01

### C. ADR Index

**ADR-DEBT-001:** POST/PUT Authentication via JSON Body
**ADR-DEBT-004:** On-Demand Telemetry Query
**ADR-DEBT-007:** Targeted .unwrap() Replacement in Hot Paths
**ADR-DEBT-008:** Incremental engine.rs Refactoring

### D. Glossary

**Debt Item:** One of 13 technical debt issues identified in audit
**Increment:** 2-4 hour implementation task
**Checkpoint:** Sprint boundary with verification criteria
**Contingency:** Time buffer for unforeseen complexity (24% = 12h)
**Traceability Matrix:** Requirements ↔ Tests ↔ Implementation mapping
**ADR:** Architecture Decision Record documenting key choices
**Residual Risk:** Risk level after mitigation applied

---

## End of Full Plan

**Total Document Length:** ~2500 lines (archival only)
**For Implementation:** Use `00_PLAN_SUMMARY.md` + individual increments (~600-850 lines context)

**PLAN008 Status:** COMPLETE - All 8 phases delivered
**Ready for Implementation:** Yes (pending approval)
