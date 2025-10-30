# PLAN008: wkmp-ap Technical Debt Remediation - Plan Summary

**Status:** Phase 1-3 Complete (Phases 4-8 pending Week 2-3)
**Created:** 2025-10-29
**Source:** wip/SPEC024-wkmp_ap_technical_debt_remediation.md + wip/wkmp-ap_technical_debt_report.md

---

## READ THIS FIRST

**What is this plan?**
Systematic remediation of 13 technical debt items identified in wkmp-ap audit, covering security, functionality, and code quality.

**How to use this plan:**
1. **Start here** - Read this summary (you are here)
2. **Review scope** - Read `scope_statement.md` to understand boundaries
3. **Check issues** - Read `01_specification_issues.md` (5 minor issues, proceed anyway)
4. **Review tests** - Read `02_test_specifications/test_index.md` for test overview
5. **Implement sprint by sprint** - Follow 3-sprint timeline below
6. **Track progress** - Update traceability matrix as you implement

**Do NOT read:** `FULL_PLAN.md` during implementation (will be created in Week 3, archival only)

---

## Executive Summary

### Problem Being Solved

wkmp-ap has accumulated 13 technical debt items requiring remediation:

**CRITICAL (1 item):**
- POST/PUT authentication bypass - any client can modify playback state without credentials

**HIGH-PRIORITY (5 items):**
- Missing file paths in decoder errors (debugging difficult)
- Buffer config not reading from database (tuning tool ignored)
- Incomplete developer UI telemetry (missing decoder state, sample rate, fade stage)
- Missing album UUIDs in passage events (metadata incomplete)
- Duration played stubbed as 0.0 seconds (incorrect reporting)

**MEDIUM-PRIORITY (7 items):**
- 376 .unwrap() calls (potential panics)
- Large files needing refactoring (engine.rs = 3,573 lines)
- 21 compiler warnings
- Duplicate config files
- Backup files committed to repo
- Minor enhancements (clipping logs, outdated TODOs)

### Solution Approach

**3-Sprint Incremental Remediation:**

**Sprint 1 (Week 1):** Security & Critical
- Fix authentication bypass (CRITICAL)
- Add file paths to errors (HIGH)
- Read buffer config from database (HIGH)

**Sprint 2 (Week 2):** Functionality & Diagnostics
- Complete telemetry (HIGH)
- Add album metadata (HIGH)
- Calculate duration played (HIGH)
- Cleanup: Resolve config duplication, delete backups

**Sprint 3 (Week 3):** Code Health
- Audit/fix .unwrap() in hot paths
- Refactor engine.rs into 3 modules
- Eliminate compiler warnings
- Add clipping logs

### Implementation Timeline

| Sprint | Duration | Deliverables | Success Criteria |
|--------|----------|--------------|------------------|
| Sprint 1 | Week 1 | Security + 2 HIGH items | Auth enforced, tests pass |
| Sprint 2 | Week 2 | 3 HIGH items + cleanup | Telemetry complete, metadata accurate |
| Sprint 3 | Week 3 | Code quality improvements | Zero warnings, engine.rs split |

**Total:** 3 weeks

### Success Metrics

**Sprint 1 Complete When:**
- All POST/PUT endpoints require authentication
- Security tests pass (6 tests)
- Decoder errors include file paths
- Buffer settings read from database

**Sprint 2 Complete When:**
- Developer UI buffer chain diagnostics complete
- Passage events include album UUIDs
- Duration played calculated accurately (±100ms)
- Single config file only, no backup files

**Sprint 3 Complete When:**
- Zero compiler warnings
- No .unwrap() in audio thread critical paths
- engine.rs <1500 lines (split into 3 modules)
- All integration tests pass

**Overall Success:**
- All 37 requirements met
- All 28 automated tests pass
- Performance within constraints (<1% overhead)
- Code review approved

---

## Scope at a Glance

### In Scope (13 debt items)

1. ✅ POST/PUT authentication (4 requirements)
2. ✅ File paths in decoder errors (3 requirements)
3. ✅ Buffer config from database (4 requirements)
4. ✅ Developer UI telemetry (4 requirements)
5. ✅ Album metadata in events (3 requirements)
6. ✅ Duration played calculation (4 requirements)
7. ✅ .unwrap() audit in hot paths (3 requirements)
8. ✅ engine.rs refactoring (3 requirements)
9. ✅ Compiler warning elimination (3 requirements)
10. ✅ Config file deduplication (3 requirements)
11. ✅ Backup file removal (2 requirements)
12. ✅ Clipping warning logs (1 requirement)
13. ✅ Outdated TODO removal (trivial)

**Total:** 37 requirements

### Out of Scope

- Phase 5 features (seeking, drain-based buffers) - future roadmap
- Complete .unwrap() elimination (test code acceptable)
- Full file refactoring (mixer.rs, handlers.rs remain as-is)
- Performance optimization (separate effort)
- New feature development

---

## Requirements Overview

**37 requirements across 5 categories:**

| Category | Count | Priority | Sprint |
|----------|-------|----------|--------|
| Security | 4 | CRITICAL | 1 |
| Functionality (High) | 17 | HIGH | 1-2 |
| Code Quality | 10 | MEDIUM | 3 |
| Cleanup | 3 | LOW-MEDIUM | 2 |
| Future | 1 | LOW | 3 |

**Detailed Requirements:** See `requirements_index.md` (compact table, 200 lines)

---

## Test Coverage

**28 automated tests + 2 manual verifications:**

| Test Type | Count | Coverage |
|-----------|-------|----------|
| Unit Tests | 18 | 64% |
| Integration Tests | 7 | 25% |
| Build Tests | 3 | 11% |
| Manual Verification | 2 | (inspection) |

**Coverage:** 100% of requirements have verification method

**Test Index:** See `02_test_specifications/test_index.md` (quick reference)
**Traceability:** See `02_test_specifications/traceability_matrix.md` (full mapping)

---

## Specification Issues Found

**Phase 2 Verification Results:**
- CRITICAL issues: 0
- HIGH issues: 0
- MEDIUM issues: 3
- LOW issues: 2

**Recommendation:** ✅ **PROCEED TO IMPLEMENTATION**

**Key Issues:**
1. MEDIUM-001: Authentication test coverage incomplete (add edge case tests)
2. MEDIUM-002: Buffer capacity validation not specified (add validation logic)
3. MEDIUM-003: Telemetry query frequency unspecified (document strategy)
4. LOW-001: Test data not specified (use WAV, 44.1kHz stereo)
5. LOW-002: Clipping log format not specified (add rate-limiting)

**Status:** All issues can be resolved during implementation/testing

**Details:** See `01_specification_issues.md`

---

## Implementation Roadmap

### Sprint 1: Security & Critical (Week 1)

**Focus:** Eliminate security vulnerability and high-impact bugs

**Tasks:**
1. Implement POST/PUT authentication
   - Extract `shared_secret` from JSON body
   - Validate against server secret
   - Return 401 on failure
   - **Files:** `auth_middleware.rs:825-835`

2. Add file paths to decoder errors
   - Add `file_path: PathBuf` to ChunkedDecoder
   - Use in error construction
   - **Files:** `audio/decode.rs:161,176`

3. Read buffer config from database
   - Query settings at BufferManager initialization
   - Use defaults if NULL
   - Validate settings (positive, capacity > headroom)
   - **Files:** `playback/buffer_manager.rs:122`

**Tests:** 11 tests (6 security + 3 decoder + 4 buffer)

**Deliverables:**
- POST/PUT auth enforced
- Decoder errors include paths
- Buffer tuning respected

---

### Sprint 2: Functionality & Diagnostics (Week 2)

**Focus:** Complete missing telemetry and metadata

**Tasks:**
4. Add developer UI telemetry
   - Create DecoderTelemetry struct
   - Query from DecoderWorker
   - Populate BufferChainInfo fields
   - **Files:** `playback/decoder_worker.rs`, `engine.rs:1203-1228`

5. Add album metadata to events
   - Create `get_passage_album_uuids()` database function
   - Query on enqueue
   - Include in PassageStarted/Complete events
   - **Files:** `db/passages.rs` (new), `engine.rs:1840,2396,2687`

6. Calculate duration played
   - Track start time in mixer HashMap
   - Calculate elapsed on completion
   - Include in PassageComplete event
   - **Files:** `playback/pipeline/mixer.rs` (new methods), `engine.rs:2018,2103`

7. Cleanup: Config duplication
   - Check main.rs imports
   - Delete obsolete config file
   - **Files:** `config.rs` OR `config_new.rs`

8. Cleanup: Backup files
   - Delete `events.rs.backup`, `events.rs.backup2`
   - **Files:** Remove files

**Tests:** 9 tests (4 telemetry + 3 albums + 3 duration)

**Deliverables:**
- Complete telemetry in developer UI
- Album UUIDs in passage events
- Accurate duration tracking
- Clean file structure

---

### Sprint 3: Code Health (Week 3)

**Focus:** Improve maintainability and code quality

**Tasks:**
9. Audit/fix .unwrap() calls
   - Focus on audio hot paths (buffer.rs, events.rs)
   - Replace with proper error propagation
   - **Files:** `audio/buffer.rs` (11 instances), `events.rs` (3 instances)

10. Refactor engine.rs
   - Split into: `engine/core.rs`, `engine/queue.rs`, `engine/diagnostics.rs`
   - Maintain public API
   - **Files:** `playback/engine/` (new folder structure)

11. Fix compiler warnings
   - Run `cargo fix --lib -p wkmp-ap`
   - Remove unused imports
   - Mark Axum-routed functions with `#[allow(dead_code)]`
   - **Files:** Various (21 warnings)

12. Add clipping warning log
   - Detect samples exceeding ±1.0
   - Log with rate-limiting (once per second)
   - **Files:** `playback/pipeline/mixer.rs:534`

**Tests:** 8 tests (2 unwrap + 1 refactor + 1 warnings + 1 config + manual clipping)

**Deliverables:**
- Zero compiler warnings
- Safe error handling in hot paths
- Maintainable file structure
- Audio clipping detection

---

## Next Steps (Implementation)

**YOU ARE HERE →** Phase 1-3 Complete (Scope + Issues + Tests)

**Phases 4-8 (Week 2-3):**
- Phase 4: Approach Selection (risk assessment, ADR)
- Phase 5: Implementation Breakdown (sized increments)
- Phase 6: Effort Estimation (timeline, resources)
- Phase 7: Risk Assessment (mitigation planning)
- Phase 8: Plan Documentation (consolidation, approval)

**To Begin Implementation NOW (Without Phases 4-8):**
1. Start with Sprint 1, Task 1 (POST/PUT authentication)
2. Read test specification: `02_test_specifications/tc_sec_001_01.md`
3. Implement to pass tests
4. Update traceability matrix when complete
5. Repeat for remaining Sprint 1 tasks

**To Complete Full Plan (Week 2-3):**
- Continue /plan workflow to generate Phases 4-8
- Provides: risk assessment, sized increments, estimates, full documentation

---

## Key Decisions Required

**Before Sprint 1:**
- [ ] Review security test approach (body-based auth acceptable?)
- [ ] Confirm buffer validation strategy (reject invalid → defaults?)

**Before Sprint 2:**
- [ ] Telemetry query frequency acceptable (on-demand only)?
- [ ] Album UUID query timing acceptable (on enqueue, no caching)?

**Before Sprint 3:**
- [ ] Refactoring scope acceptable (engine only, defer mixer/handlers)?
- [ ] .unwrap() audit scope acceptable (hot paths only, leave tests)?

---

## Dependencies Confirmed

**Existing Code (Available):**
- ✅ Authentication infrastructure (`auth_middleware.rs`)
- ✅ Database layer (`db/settings.rs`, `db/passages.rs`)
- ✅ Audio pipeline (decoder, buffer, mixer)
- ✅ Event system (PublishEvent, EventBus)

**External Libraries (Already in Cargo.toml):**
- ✅ axum, tokio, sqlx, serde, uuid, tracing, symphonia

**Database Schema (Exists per IMPL001):**
- ✅ settings, passages, passage_albums, albums tables

**No new dependencies required**

---

## Risks & Mitigations

**Sprint 1 Risks:**
- Risk: Authentication breaks existing clients
- Mitigation: Comprehensive test suite, backward-compatible deployment option

**Sprint 2 Risks:**
- Risk: Album queries impact performance
- Mitigation: Query once at enqueue, cache in queue entry

**Sprint 3 Risks:**
- Risk: Refactoring introduces regressions
- Mitigation: Full test suite before/after, incremental changes

---

## Plan Document Structure

```
wip/PLAN008_wkmp_ap_technical_debt/
├── 00_PLAN_SUMMARY.md                 ← YOU ARE HERE
├── requirements_index.md              → Compact requirements table
├── scope_statement.md                 → In/out scope, assumptions
├── 01_specification_issues.md         → 5 issues found (proceed anyway)
├── 02_test_specifications/            → Test definitions
│   ├── test_index.md                  → Quick reference (28 tests)
│   ├── tc_sec_001_01.md               → Sample test spec
│   ├── ...                            → (26 more test specs)
│   └── traceability_matrix.md         → Requirements ↔ Tests mapping
├── 03_approach_selection.md           → (Week 2 - Phases 4-5)
├── 04_increments/                     → (Week 2 - Phases 4-5)
├── 05_estimates.md                    → (Week 3 - Phase 6)
├── 06_risks.md                        → (Week 3 - Phase 7)
└── FULL_PLAN.md                       → (Week 3 - Phase 8, archival only)
```

---

## Success Criteria Summary

**This plan succeeds when:**
1. ✅ All 37 requirements implemented
2. ✅ All 28 automated tests pass
3. ✅ Zero compiler warnings
4. ✅ No .unwrap() in audio hot paths
5. ✅ engine.rs refactored (<1500 lines per module)
6. ✅ Performance overhead <1%
7. ✅ Code review approved
8. ✅ Integration tests pass

---

## Contact & Approval

**Plan Created By:** WKMP Development Team
**Date:** 2025-10-29
**Status:** Phase 1-3 Complete, Ready for Implementation or Phase 4-8
**Approval:** Pending stakeholder review

**Questions?** See detailed documents in plan folder or contact project lead.

---

## Plan Summary Checklist

- [x] Problem clearly stated
- [x] Solution approach defined
- [x] Timeline realistic (3 weeks)
- [x] Success criteria measurable
- [x] Scope boundaries clear
- [x] Test coverage 100%
- [x] No blocking specification issues
- [x] Dependencies confirmed
- [x] Risks identified with mitigations
- [x] Next steps actionable

**Phase 1-3 Complete** - Ready to Implement or Continue to Phases 4-8
