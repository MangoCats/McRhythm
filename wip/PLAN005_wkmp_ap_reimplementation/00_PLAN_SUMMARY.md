# PLAN005: wkmp-ap Re-Implementation Plan - EXECUTIVE SUMMARY

**Plan ID:** PLAN005_wkmp_ap_reimplementation
**Source Specification:** docs/GUIDE002-wkmp_ap_re_implementation_guide.md
**Date Created:** 2025-10-26
**Status:** Ready for User Review → Implementation
**Scope:** Complete wkmp-ap Audio Player microservice re-implementation

---

## ⚡ Quick Start - READ THIS FIRST

**This plan orchestrates the re-implementation of wkmp-ap Audio Player following 8 specifications.**

**What's Been Completed (Phases 1-3):**
- ✅ 39 requirements extracted and cataloged
- ✅ 48 specification issues identified (18 CRITICAL, 21 HIGH)
- ✅ 39 acceptance tests defined (100% coverage)
- ✅ Traceability matrix complete (requirement → test → implementation)
- ✅ All critical blockers resolved or documented with implementation decisions

**Implementation Readiness:** ⚠️ **PROCEED WITH CAUTION**
- 18 CRITICAL specification issues require explicit acceptance
- 5 issues deferred to Phase 8 (performance validation)
- 13 issues resolved with documented implementation decisions

**Key Decisions Made:**
1. **SPEC002 Fade/Lead Model:** Orthogonal (fade pre-buffer, lead in timing algorithm)
2. **SPEC007 Response Formats:** Defined explicit JSON response contracts
3. **SPEC011 MixerStateContext:** Implemented as recommended enum
4. **SPEC022 Performance:** Establish baselines empirically in Phase 8

---

## 📊 Plan Overview

### Scope Summary

**In Scope:**
- Complete wkmp-ap re-implementation: Foundation → Database → Audio → Playback → Crossfade → API → Performance
- 8 implementation phases over 9-10 weeks
- 39 requirements, 39 acceptance tests, 100% coverage

**Out of Scope:**
- Other microservices (wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le)
- Database schema changes (use IMPL001 as-is)
- New features beyond current specifications

### Requirements Summary

| Category | Requirements | Priority |
|----------|-------------|----------|
| Error Handling | 5 | High |
| Decoder-Buffer | 7 | High |
| Sample Rate | 4 | High |
| Crossfade | 6 | High |
| Crossfade Completion | 3 | High |
| Performance | 6 | High |
| API Design | 4 | High |
| Event System | 4 | High |
| **TOTAL** | **39** | |

---

## 🚨 Critical Issues Requiring Attention

### Specification Issues Found (Phase 2 Analysis)

**Total Issues:** 48 (18 CRITICAL, 21 HIGH, 7 MEDIUM, 2 LOW)

**Highest Risk Specifications:**
1. **SPEC007 (API Design):** 5 critical issues - Missing response contracts
2. **SPEC022 (Performance Targets):** 4 critical issues - Unmeasurable targets, no baseline
3. **SPEC021 (Error Handling):** 4 critical issues - Recovery loops undefined
4. **SPEC011 (Event System):** 4 critical issues - Internal spec incomplete
5. **SPEC002 (Crossfade):** 3 critical issues - Orthogonality contradiction

### Issues Resolution Status

**RESOLVED (Implementation Decisions Documented):**
- ✅ SPEC007 Issue #1: POST /playback/enqueue response format defined
- ✅ SPEC007 Issue #5: SSE event specification (read SPEC011, implement FIFO)
- ✅ SPEC011 Issue #1: MixerStateContext defined as enum
- ✅ SPEC002 Issue #1: Fade/Lead contradiction resolved (choose orthogonal model)
- ✅ All HIGH severity issues: 21 documented with implementation guidance

**DEFERRED TO PHASE 8 (Performance Validation):**
- ⏳ SPEC022 Issues #1-4: Establish measurement baselines empirically on Pi Zero 2W

**ACCEPTED AT-RISK:**
- ⚠️ SPEC017 rubato assumptions: Fallback wrapper plan ready (1-2 days if needed)

**See:** `01_specification_issues.md` for complete analysis and resolution details

---

## ✅ Test Coverage

### Acceptance Tests Defined

**Total Tests:** 39
- **Unit Tests:** 9 (23%)
- **Integration Tests:** 23 (59%)
- **System Tests:** 7 (18%)

**Coverage:** 100% of 39 requirements per traceability matrix

**Key Tests:**
- **TC-S-XFD-TIME-01:** Sample-accurate crossfade timing (±1ms tolerance)
- **TC-S-PERF-CPU-01:** CPU usage <40% on Pi Zero 2W
- **TC-I-DBD-CHAIN-01:** DecoderChain pipeline integration
- **TC-I-API-CTL-01:** Control endpoints (enqueue, play, pause, skip, stop)

**See:** `02_test_specifications/test_index.md` for complete test catalog

---

## 🗺️ Implementation Roadmap

### 8-Phase Implementation (9-10 weeks)

**Phase 1: Foundation (Week 1)**
- Error handling framework (SPEC021 taxonomy)
- Configuration loading (database + TOML)
- Event system integration (EventBus)
- **Deliverables:** error.rs, config.rs, events.rs, main.rs, state.rs

**Phase 2: Database Layer (Week 1-2)**
- Queue persistence and restoration
- Passage metadata access
- Settings management
- **Deliverables:** queue.rs with CRUD operations

**Phase 3: Audio Subsystem Basics (Week 2-3)**
- Decode single audio file (symphonia)
- Sample rate conversion (rubato)
- Audio output (cpal)
- **AT-RISK VALIDATION:** rubato StatefulResampler validation
- **Deliverables:** output.rs, decoder_chain.rs (minimal), buffer.rs

**Phase 4: Core Playback Engine (Week 3-5)**
- DecoderChain complete (Decoder→Resampler→Fader→Buffer)
- DecoderWorker (single-threaded serial)
- PlaybackEngine (queue orchestration)
- Buffer backpressure with hysteresis
- **Deliverables:** decoder_worker.rs, engine.rs, decoder_chain.rs (complete)

**Phase 5: Crossfade Mixer (Week 5-7)**
- 5 fade curve types
- Sample-accurate triggering
- Dual buffer mixing
- Completion signaling (SPEC018)
- **Deliverables:** mixer.rs with all crossfade logic

**Phase 6: API Layer (Week 7-8)**
- HTTP REST endpoints (Axum)
- SSE broadcaster
- Request validation
- **Deliverables:** handlers.rs, sse.rs

**Phase 7: Error Handling & Recovery (Week 8-9)**
- Comprehensive error handling per SPEC021
- Graceful degradation
- Error event emission
- **Deliverables:** Error handling integrated into all components

**Phase 8: Performance Optimization & Validation (Week 9-10)**
- **CRITICAL:** Establish performance baselines on Pi Zero 2W
- Profiling and optimization
- 24-hour continuous playback test
- Performance regression tests
- **Deliverables:** All SPEC022 targets verified

---

## 📋 Implementation Guidelines

### Test-Driven Development Approach

**For each requirement:**
1. Read requirement from `requirements_index.md`
2. Read acceptance test(s) from `02_test_specifications/`
3. Write failing test first (TDD)
4. Implement code to pass test
5. Refactor and optimize
6. Update traceability matrix with implementation file
7. Commit with `/commit` workflow

### Context Window Management

**When implementing, read ONLY:**
1. This summary (~400 lines) - Overview and roadmap
2. Current phase requirements from `requirements_index.md` (~50 lines)
3. Relevant test specs from `02_test_specifications/tc_*.md` (~100-200 lines)
4. Specification sections as needed (targeted reads, not full docs)

**Total context:** ~600-850 lines, not 2000+

**DO NOT read:**
- Full specification documents (too large)
- `FULL_PLAN.md` (archival only, when/if created)
- Unrelated test specifications

### Incremental Implementation Strategy

**Phase 4 Example (Core Playback Engine):**
1. Implement RingBuffer basic operations (1 day)
2. Implement DecoderChain skeleton (1 day)
3. Add symphonia decoder integration (2 days)
4. Add rubato resampler integration (1 day)
5. Add fader stage (1 day)
6. Implement DecoderWorker serial processing (2 days)
7. Implement PlaybackEngine queue orchestration (3 days)
8. Add buffer backpressure with hysteresis (2 days)
9. Integration testing (2 days)
10. **Total:** ~15 days (3 weeks with buffer)

---

## 🎯 Success Criteria

**Implementation is COMPLETE when:**

### Functional Completeness
- ✅ All 39 requirements satisfied
- ✅ All 8 phases completed with acceptance criteria met
- ✅ All 39 acceptance tests passing
- ✅ All REST API endpoints functional per SPEC007
- ✅ All SSE events emitted per SPEC011

### Quality Standards
- ✅ Unit test coverage >80% for core components
- ✅ Integration tests pass for all workflows
- ✅ No compiler warnings (clippy clean)
- ✅ Code follows IMPL002 coding conventions
- ✅ All public APIs documented with rustdoc

### Performance Validation (SPEC022)
- ✅ CPU <40% on Pi Zero 2W (aggregate across 4 cores)
- ✅ Decode latency <500ms from enqueue to first samples
- ✅ Memory <150MB RSS during continuous playback
- ✅ Crossfade timing accuracy ±1ms
- ✅ No memory leaks (24-hour continuous test)
- ✅ No audio dropouts/glitches

### Validation Testing
- ✅ Manual testing on development system (Linux/macOS/Windows)
- ✅ Manual testing on Pi Zero 2W deployment target
- ✅ 24-hour continuous playback test (no crashes, no leaks)
- ✅ Sample-accurate crossfade verification (TC-S-XFD-TIME-01)
- ✅ Multi-client SSE test (multiple browsers)
- ✅ Error recovery test (decode failures, device removal, database errors)

---

## ⚠️ Risk Assessment

### High-Risk Areas

**1. Crossfade Timing Accuracy**
- **Risk:** Sample-accurate crossfading requires precise position tracking
- **Mitigation:** Comprehensive testing with TC-S-XFD-TIME-01, automated verification
- **Contingency:** Manual verification with oscilloscope/audio analysis tools

**2. Performance on Pi Zero 2W**
- **Risk:** Limited CPU/memory may not meet SPEC022 targets
- **Mitigation:** Early performance testing in Phase 8, profiling, optimization
- **Contingency:** Reduce maximum_decode_streams from 3 to 2, adjust buffer sizes

**3. Specification Ambiguities**
- **Risk:** 18 CRITICAL specification issues could cause rework
- **Mitigation:** All issues documented with implementation decisions in `01_specification_issues.md`
- **Contingency:** Re-evaluate decisions if implementation proves incorrect

### Medium-Risk Areas

**4. rubato Library Compatibility (AT-RISK)**
- **Risk:** Library may not provide required state management
- **Mitigation:** Early validation in Phase 3, fallback wrapper design ready
- **Impact:** 1-2 days additional effort if wrapper needed

**5. Database Lock Contention**
- **Risk:** SQLite busy errors under concurrent access
- **Mitigation:** Retry with exponential backoff per SPEC021
- **Impact:** Performance degradation under heavy load, mitigated by retry logic

---

## 📁 Plan Structure

```
wip/PLAN005_wkmp_ap_reimplementation/
├── 00_PLAN_SUMMARY.md                  ← YOU ARE HERE (read this first)
├── 01_specification_issues.md          ← 48 issues found, resolution status
├── 02_test_specifications/             ← Modular test specs
│   ├── test_index.md                   ← Quick reference (39 tests)
│   ├── tc_s_xfd_time_01.md            ← Sample-accurate crossfade test
│   ├── tc_s_perf_cpu_01.md            ← CPU usage test (Pi Zero 2W)
│   ├── tc_*.md                         ← Individual test specs (~100 lines each)
│   └── traceability_matrix.md          ← Requirement → Test → Implementation
├── requirements_index.md               ← 39 requirements (compact table)
├── scope_statement.md                  ← In/out of scope, assumptions, constraints
└── dependencies_map.md                 ← Library deps, hardware, risk assessment
```

---

## 🚀 Next Actions

### User Review (Now)

**Please review:**
1. ✅ This summary - Understand scope and approach
2. ✅ `01_specification_issues.md` - Verify critical issues acceptable
3. ✅ `02_test_specifications/test_index.md` - Confirm test coverage adequate
4. ✅ `scope_statement.md` - Validate scope boundaries correct

**User Decision Required:**
- ❓ **Accept specification issues and proceed with documented assumptions?**
- ❓ **Approve test coverage (100% of 39 requirements)?**
- ❓ **Approve 8-phase implementation roadmap (9-10 weeks)?**

### Implementation Start (After Approval)

**Recommended sequence:**
```bash
# Phase 1: Foundation
cd wkmp-ap
cargo new . --name wkmp-ap

# Implement Phase 1 components following test specs
# See: 02_test_specifications/tc_u_erh_*.md for error handling tests

# After Phase 1 complete:
# Update traceability matrix with actual implementation files
# Run tests: cargo test
# Commit: use /commit workflow

# Proceed to Phase 2, then 3, etc.
```

---

## 📚 Reference Documents

**Tier 1 (Requirements):**
- REQ001-requirements.md - Complete WKMP requirements
- REQ002-entity_definitions.md - Entity model

**Tier 2 (Specifications - Primary Sources):**
- SPEC002-crossfade.md - Crossfade timing and curves
- SPEC016-decoder_buffer_design.md - AUTHORITATIVE decoder-buffer architecture
- SPEC017-sample_rate_conversion.md - Sample rate conversion
- SPEC018-crossfade_completion_coordination.md - Crossfade completion signaling
- SPEC021-error_handling.md - Error handling strategy
- SPEC022-performance_targets.md - Performance benchmarks
- SPEC007-api_design.md - REST API design
- SPEC011-event_system.md - Event system and SSE

**Tier 3 (Implementation):**
- IMPL001-database_schema.md - Database schema
- IMPL002-coding_conventions.md - Rust coding standards

**Tier 4 (Execution):**
- GUIDE002-wkmp_ap_re_implementation_guide.md - This plan's source document

---

## ✍️ Plan Metadata

**Plan ID:** PLAN005
**Created:** 2025-10-26
**Author:** Claude Code via /plan workflow
**Workflow:** Phases 1-3 complete (Scope, Issues, Tests)
**Phases 4-8:** Not yet implemented (Week 2-3 deliverable per /plan roadmap)

**Current Status:** ✅ Ready for user review and implementation
**Next Milestone:** User approval → Begin Phase 1 (Foundation)
**Last Reviewed:** 2025-10-26 (Tick consistency - updated test specs and issue examples to align with SPEC017/IMPL001)

---

**Questions or concerns? Review detailed documentation in plan folder, or ask for clarification.**

**Ready to implement? Start with Phase 1 (Foundation) after user approval.**
