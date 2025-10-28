# PLAN004: Buffer Auto-Tuning Implementation Plan

**Status:** Ready for Implementation (Phases 1-3 Complete)
**Created:** 2025-10-26
**Specification:** wip/_buffer_autotuning.md

---

## READ THIS FIRST

This is the **implementation-ready plan** for buffer auto-tuning. Start here, then navigate to specific sections as needed.

**Context Window Budget:**
- This summary: ~450 lines
- Typical increment: ~250 lines
- Combined reading: ~700 lines (optimal for AI/human)

**Do NOT load the full plan document** (use only for archival/review).

---

## Executive Summary

### Problem

Audio playback exhibits gaps when buffer parameters (mixer_check_interval_ms and audio_buffer_size) are misconfigured. Manual tuning is time-consuming and hardware-specific.

### Solution

Standalone utility (`wkmp-ap tune-buffers`) that automatically:
1. Tests parameter combinations systematically
2. Measures buffer underruns for each
3. Finds optimal balance (minimum latency + maximum stability)
4. Recommends safe values for the current system

### Approach

**Two-phase search algorithm:**
- Phase 1: Coarse sweep across mixer intervals (1-100ms) with default buffer
- Phase 2: Binary search for minimum stable buffer size per viable interval

**Result:** Characterization curve mapping interval → minimum buffer size

---

## Plan Status

### Completed (Week 1)

✅ **Phase 1:** Scope Definition
- 31 requirements extracted (29 High, 2 Medium)
- Dependencies identified: 7 existing components, 8 Cargo crates (all available)
- Estimated ~1,600 LOC new code

✅ **Phase 2:** Specification Verification
- 8 issues found (0 Critical, 3 High, 4 Medium, 1 Low)
- HIGH issues resolved with accepted recommendations
- Specification implementation-ready

✅ **Phase 3:** Test Definition
- 47 tests defined (23 unit, 12 integration, 5 system, 7 manual)
- 100% requirement coverage
- Traceability matrix complete

### Pending (Weeks 2-3)

⏳ **Phase 4:** Approach Selection (Week 2)
⏳ **Phase 5:** Implementation Breakdown (Week 2)
⏳ **Phase 6:** Effort Estimation (Week 3)
⏳ **Phase 7:** Risk Assessment (Week 3)
⏳ **Phase 8:** Final Documentation (Week 3)

**Current Capability:** Can analyze specifications, identify issues, define comprehensive test coverage

---

## Requirements Summary

**Total:** 31 requirements (5 objectives + 26 functional)

**By Category:**
- Detection (3): Underrun monitoring, jitter, thresholds
- Search (4): Parameter exploration, binary search, safety
- Output (4): Reports, database updates, JSON export
- Integration (3): Standalone utility, reuse infrastructure
- Algorithm (9): Two-phase search, curve fitting, recommendations
- UI (2): CLI interface, progress feedback
- Architecture (3): Component structure, minimize dependencies
- Safety (3): Backup/restore, error handling
- Testing (3): Unit/integration/manual validation
- Success Criteria (3): Functional, quality, usability

**Priority Distribution:**
- High: 29 (94%)
- Medium: 2 (6%)

**Full Index:** See requirements_index.md

---

## Test Coverage

**Total:** 47 tests

**Distribution:**
- Unit Tests: 23 (fast feedback, mocked data)
- Integration Tests: 12 (component interaction)
- System Tests: 5 (end-to-end on hardware)
- Manual Tests: 7 (quality validation, human evaluation)

**Coverage:** 100% of requirements have acceptance tests

**Key Tests:**
- TC-U-SEARCH-010-01: Binary search convergence
- TC-U-SAFE-010-01: Settings backup/restore
- TC-S-OBJ-010-01: End-to-end parameter determination
- TC-M-TEST-060-02: 1+ hour stability validation

**Full Index:** See 02_test_specifications/test_index.md
**Traceability:** See 02_test_specifications/traceability_matrix.md

---

## Architecture Overview

### New Components (To Be Implemented)

```
wkmp-ap/src/bin/tune_buffers.rs         (~400 LOC)
  └─ Main entry point, CLI parsing, workflow orchestration

wkmp-ap/src/tuning/
  ├─ mod.rs                               (~20 LOC)
  ├─ test_harness.rs                      (~300 LOC)
  │   └─ Simplified playback for testing
  ├─ metrics.rs                           (~200 LOC)
  │   └─ Underrun detection, jitter, occupancy
  ├─ search.rs                            (~250 LOC)
  │   └─ Binary search, parameter exploration
  ├─ curve.rs                             (~150 LOC)
  │   └─ Curve fitting, recommendation logic
  ├─ report.rs                            (~200 LOC)
  │   └─ CLI output, JSON export
  ├─ safety.rs                            (~100 LOC)
  │   └─ Settings backup/restore
  └─ system_info.rs                       (~100 LOC)
      └─ CPU, OS, audio device detection
```

**Total New Code:** ~1,720 LOC

### Existing Components (Reused)

- `playback/ring_buffer.rs` - Underrun monitoring
- `audio/output.rs` - Audio playback via cpal
- `db/settings.rs` - Parameter persistence
- `audio/decoder.rs` - Audio decoding
- `audio/resampler.rs` - Sample rate conversion
- `playback/callback_monitor.rs` - Jitter measurement

**Dependencies:** All available, no new Cargo crates required

---

## Implementation Workflow

### Pre-Implementation

1. Read this summary (~450 lines)
2. Review specification issues: 01_specification_issues.md
3. Understand test expectations: 02_test_specifications/test_index.md
4. Review traceability: 02_test_specifications/traceability_matrix.md

### During Implementation

**Test-First Approach:**
1. Pick a requirement from requirements_index.md
2. Read its test specification from 02_test_specifications/
3. Implement to pass the test
4. Verify traceability (test → requirement → implementation)
5. Commit after tests pass

**Suggested Order:**
1. Core data structures (metrics.rs types)
2. Test harness (simplified playback)
3. Binary search algorithm (search.rs)
4. Curve fitting (curve.rs)
5. Safety (backup/restore)
6. Report generation (CLI + JSON)
7. Main orchestration (tune_buffers.rs)

### Testing Strategy

**During Development:**
```bash
# Run unit tests frequently (~seconds)
cargo test --lib -p wkmp-ap tuning::

# Run integration tests periodically (~minutes)
cargo test --test integration_tests
```

**Before PR:**
```bash
# Full test suite
cargo test -p wkmp-ap

# Target: All unit + integration tests pass
```

**Before Release:**
```bash
# System test: Full tuning run
wkmp-ap tune-buffers --thorough

# Manual validation: 1+ hour stability test
cargo run -p wkmp-ap --release
# Monitor logs for underruns
```

---

## Key Design Decisions

### Resolved HIGH-Priority Issues

**ISSUE-H-001: Test Audio Generation**
- Generate 440 Hz sine at 48kHz, resample to 44.1kHz (exercises full pipeline)
- Duration: 30-60 seconds
- Format: f32 stereo, -6dB amplitude

**ISSUE-H-002: CPU Usage Measurement**
- Use /proc/self/stat on Linux for process CPU time
- Mark "unavailable" on other platforms
- Optional: Skip if measurement fails

**ISSUE-H-003: Binary Search Convergence**
- Threshold: 128 frames (sample-rate-independent)
- Rationale: ~3ms precision at 44.1kHz is sufficient
- Return larger value if oscillating at boundary (conservative)

### Conservative Recommendation Strategy

Per TUNE-Q-030 decision:
- If variability measurable: Target 6-sigma (very rare problems)
- If variability not practical: Recommend 2x buffer size of trouble point
- Always provide primary + conservative recommendations

---

## Success Criteria

### Functional (TUNE-SUCCESS-010)

- ✓ Completes tuning in <5 min (quick) or <15 min (thorough)
- ✓ Identifies stable parameter combinations
- ✓ Generates actionable recommendations
- ✓ Exports valid JSON

### Quality (TUNE-SUCCESS-020)

- ✓ Recommended values produce <0.1% underruns
- ✓ No false positives/negatives
- ✓ Results reproducible (±10% variation)

### Usability (TUNE-SUCCESS-030)

- ✓ Clear progress indication
- ✓ Understandable recommendations
- ✓ Easy to apply (single confirmation)
- ✓ Useful error messages

---

## Example Usage

### Basic Tuning

```bash
# Quick tuning (~5 min)
wkmp-ap tune-buffers --quick

# Thorough tuning (~15 min)
wkmp-ap tune-buffers --thorough

# Auto-apply recommendations
wkmp-ap tune-buffers --apply
```

### Hardware Profiling

```bash
# Export results for comparison
wkmp-ap tune-buffers --thorough --export dev_machine.json

# On different hardware
wkmp-ap tune-buffers --thorough --export pi_zero_2w.json

# Compare profiles
diff <(jq '.recommendations' dev_machine.json) \
     <(jq '.recommendations' pi_zero_2w.json)
```

---

## Expected Output

```
Starting buffer auto-tuning...

System: AMD Ryzen 5 5600X, Linux 6.8.0, ALSA
Current: mixer_check_interval_ms=5, audio_buffer_size=512

Phase 1: Testing mixer intervals with default buffer...
[✓] 1ms: FAIL (23% underruns)
[✓] 2ms: FAIL (12% underruns)
[✓] 5ms: OK (0.02% underruns)
[✓] 10ms: OK (0.01% underruns)
[✓] 20ms: OK (0% underruns)
[✓] 50ms: OK (0% underruns)

Phase 2: Finding minimum buffer sizes...
[✓] 5ms interval: Testing buffers 64-4096... → 256 frames
[✓] 10ms interval: Testing buffers 64-4096... → 128 frames
[✓] 20ms interval: Testing buffers 64-4096... → 128 frames
[✓] 50ms interval: Testing buffers 64-4096... → 64 frames

Results:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Recommended: mixer_check_interval_ms = 10
             audio_buffer_size = 128

Rationale: Lowest interval with minimal buffer size
Expected latency: ~2.9ms
Stability: Excellent (0.01% underruns)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Apply these values? [Y/n]
```

---

## Risk Summary

### Mitigated Risks

**False negatives** (marking unstable as stable)
- Mitigation: Conservative thresholds (<0.1%), 6-sigma logic
- Tests: TC-U-DET-030-01, TC-M-SUCCESS-020-01

**System hang**
- Mitigation: 60-second timeout per test, abort mechanism
- Tests: TC-U-SAFE-020-01

**Settings not restored**
- Mitigation: Backup to memory + temp file, signal handlers
- Tests: TC-U-SAFE-010-01, TC-U-SAFE-010-02

### Accepted Risks

**Test duration may be too short** (30s insufficient for intermittent issues)
- Acceptance: Users can extend to 60s with --thorough
- Validation: Manual 1+ hour test before release

**Results may vary** (±10% between runs)
- Acceptance: Hardware scheduler is non-deterministic
- Mitigation: Conservative recommendations account for variation

---

## Document Navigation

### Planning Documents (Read as Needed)

| Document | Purpose | When to Read |
|----------|---------|--------------|
| **00_PLAN_SUMMARY.md** | Overview, start here | Always read first |
| requirements_index.md | Compact requirements list | Reference during implementation |
| scope_statement.md | Boundaries, assumptions, constraints | When clarifying scope |
| dependencies_map.md | Internal/external dependencies | When setting up environment |
| 01_specification_issues.md | Problems found, resolutions | When clarifying ambiguities |
| 02_test_specifications/ | Acceptance tests | When implementing a requirement |
| traceability_matrix.md | Req → Test → Code mapping | Verification, review |

### Implementation Order

1. **Start:** Read 00_PLAN_SUMMARY.md (this document)
2. **Setup:** Review dependencies_map.md, verify environment
3. **Implement:** Pick requirement → read its test → implement → verify
4. **Verify:** Check traceability_matrix.md for completeness

### Context Window Management

**While implementing requirement TUNE-SEARCH-010:**
1. Read 00_PLAN_SUMMARY.md (~450 lines)
2. Read 02_test_specifications/tc_u_search_010_01.md (~200 lines)
3. **Total:** ~650 lines (optimal)

**Do NOT read:**
- Full specification (wip/_buffer_autotuning.md) - too large
- Unrelated test specifications - not needed yet
- Full traceability matrix - reference specific rows only

---

## Quick Start

```bash
# 1. Verify environment
cargo build -p wkmp-ap

# 2. Create tuning module
mkdir -p wkmp-ap/src/tuning
touch wkmp-ap/src/tuning/mod.rs

# 3. Start with metrics types
# Read: 02_test_specifications/tc_u_det_030_01.md
# Implement: wkmp-ap/src/tuning/metrics.rs

# 4. Build test harness
# Read: 02_test_specifications/tc_u_test_040_01.md
# Implement: wkmp-ap/src/tuning/test_harness.rs

# 5. Continue with search algorithm
# Read: 02_test_specifications/tc_u_search_010_01.md
# Implement: wkmp-ap/src/tuning/search.rs
```

---

## Support and Questions

**Specification unclear?**
- Check 01_specification_issues.md for known clarifications
- HIGH-priority issues have accepted resolutions

**Test expectations unclear?**
- Read detailed test spec in 02_test_specifications/
- Examples include Given/When/Then and code snippets

**Implementation blocked?**
- Review dependencies_map.md
- All internal dependencies exist (reuse available)
- All external dependencies in Cargo.toml

---

## Completion Checklist

Before marking feature complete:

- [ ] All unit tests pass (23/23)
- [ ] All integration tests pass (12/12)
- [ ] At least 1 system test passes
- [ ] Manual validation on dev hardware complete
- [ ] Reproducibility test passes (3 runs, ±10%)
- [ ] 1+ hour stability test with recommended values
- [ ] JSON export validates
- [ ] Documentation updated (CLI help)
- [ ] Code review complete
- [ ] Traceability matrix updated (status columns)

---

## Estimated Effort

**New Code:** ~1,720 LOC
**Tests:** ~1,200 LOC (test implementations)
**Total:** ~2,920 LOC

**Time Estimates:**
- Core implementation: 40-60 hours
- Test implementation: 20-30 hours
- Integration and debugging: 10-20 hours
- Manual validation: 5-10 hours
- **Total:** 75-120 hours (2-3 weeks full-time)

---

## Next Steps

**Implementer:**
1. Read this summary
2. Set up environment (verify dependencies)
3. Start with metrics.rs (simplest component)
4. Follow test-first approach
5. Verify traceability after each component

**Reviewer:**
1. Read this summary
2. Review traceability_matrix.md for completeness
3. Verify test coverage (100% required)
4. Check implementation matches test specifications

**User:**
1. Wait for implementation to complete
2. Run tuning on your hardware
3. Review recommendations
4. Apply if acceptable
5. Validate with 1+ hour playback

---

**Status:** Planning Phases 1-3 Complete ✓
**Ready for:** Implementation (test-first approach)
**Estimated Duration:** 2-3 weeks
**Risk Level:** Low (specification solid, tests comprehensive)
