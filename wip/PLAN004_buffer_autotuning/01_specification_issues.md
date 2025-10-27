# Specification Issues - Buffer Auto-Tuning

**Analysis Date:** 2025-10-26
**Analyzed By:** /plan workflow Phase 2
**Total Issues Found:** 8 (0 Critical, 3 High, 4 Medium, 1 Low)

---

## Executive Summary

The specification is **well-formed and implementation-ready** with no CRITICAL blockers. Three HIGH-priority issues require clarification before implementation begins to reduce risk of rework. Four MEDIUM issues are minor ambiguities that can be resolved during implementation. One LOW issue is a minor documentation gap.

**Recommendation:** Resolve HIGH-priority issues (estimated 1-2 hours), then proceed with implementation.

---

## CRITICAL Issues (Block Implementation)

**None found** ✓

---

## HIGH Priority Issues (High Risk Without Resolution)

### ISSUE-H-001: Test Audio Generation Not Specified

**Requirement:** TUNE-TEST-020 (line 171-175)
**Severity:** HIGH
**Category:** Missing Information

**Problem:**
Specification states "Fallback: Generate test tone (440 Hz sine wave)" but doesn't specify:
- Duration of generated tone (should match test duration 30-60s)
- Sample rate (44.1kHz assumed but not stated)
- Bit depth (f32 assumed but not stated)
- Whether tone should be mono or stereo
- Whether tone exercises resampler (if generated at different rate)

**Impact:**
- May not exercise full pipeline if tone bypasses resampler
- Test validity unclear if tone doesn't match real audio characteristics
- Implementation ambiguity

**Recommended Resolution:**
Add to specification:
```
Generated test tone specification:
- Frequency: 440 Hz sine wave
- Duration: Match test_duration (30-60 seconds)
- Sample rate: Generate at 48kHz, resample to 44.1kHz (exercises resampler)
- Format: f32 stereo (2 channels)
- Amplitude: -6dB (0.5 peak to avoid clipping)
```

**Traceability:** TUNE-TEST-020

---

### ISSUE-H-002: CPU Usage Measurement Method Unspecified

**Requirement:** TUNE-DET-020, TUNE-TEST-030 (lines 67-70, 177-181)
**Severity:** HIGH
**Category:** Missing Information

**Problem:**
Specification requires "CPU usage per parameter combination" but doesn't specify:
- Measurement method (process CPU time vs. system CPU time?)
- Measurement interval (sample every 100ms? average over test duration?)
- Platform-specific APIs (Linux /proc/stat, Windows Performance Counters?)
- What to do if measurement unavailable

**Impact:**
- Implementation unclear, multiple interpretations possible
- Platform portability uncertain
- May spend significant time on complex CPU measurement

**Recommended Resolution:**
Add to specification:
```
CPU usage measurement (TUNE-TEST-030):
- Method: Process CPU time (user + system) divided by wall clock time
- Platform: Linux via /proc/self/stat, other platforms mark as "unavailable"
- Sampling: Measure at start and end of test, compute average percentage
- Optional: If measurement fails, log warning and continue without CPU data
- Report as: Percentage (0-100%) representing CPU usage during test
```

**Traceability:** TUNE-DET-020, TUNE-TEST-030

---

### ISSUE-H-003: Binary Search Convergence Boundary Ambiguous

**Requirement:** TUNE-SEARCH-010 (lines 185-205)
**Severity:** HIGH
**Category:** Ambiguity

**Problem:**
Algorithm states "while (high - low) > 128: # Converge to within 128 frames" but:
- Why 128 frames? Rationale not provided
- Is 128 frames appropriate for all sample rates? (At 44.1kHz, 128 frames = 2.9ms, but at 48kHz = 2.7ms)
- Should convergence be sample-rate-dependent?
- What if optimal is exactly on boundary? (e.g., 256 vs. 384)

**Impact:**
- May converge too early or too late depending on sample rate
- Potential for off-by-one errors at boundaries
- Test duration uncertainty (affects --quick vs --thorough timing)

**Recommended Resolution:**
Add to specification:
```
Binary search convergence (TUNE-SEARCH-010):
- Convergence threshold: 128 frames (fixed, sample-rate-independent)
- Rationale: Represents ~3ms at 44.1kHz, sufficient precision for parameter tuning
- Example: If low=256, high=512, mid=384. If mid stable, try low=256, high=384, mid=320, etc.
- Termination: When high-low ≤ 128, return best_stable (smallest known stable value)
- Edge case: If search oscillates at boundary, return larger (more conservative) value
```

**Traceability:** TUNE-SEARCH-010

---

## MEDIUM Priority Issues (Should Resolve Before Implementation)

### ISSUE-M-001: "Anomalous Results" Definition Missing

**Requirement:** TUNE-SAFE-030 (lines 381-384)
**Severity:** MEDIUM
**Category:** Ambiguity

**Problem:**
Specification requires "Warn if results seem anomalous" but doesn't define what constitutes anomalous:
- Interval-buffer relationship inverted? (smaller interval needs smaller buffer - unexpected)
- All intervals fail? Already covered by TUNE-SEARCH-020?
- Recommendations outside expected ranges?

**Impact:**
- Sanity check implementation unclear
- May miss actual anomalies or flag false positives

**Recommended Resolution:**
```
Anomalous results detection (TUNE-SAFE-030):
- Inverted relationship: If smaller interval has smaller min_buffer than larger interval (unexpected)
- Excessive buffer requirements: If all intervals need >2048 frames at default (system may be inadequate)
- Inconsistent test results: If same configuration produces >30% variation on repeated tests
- No viable intervals: If all tested intervals fail even with maximum buffer (4096 frames)
- Action: Log warning, export data for analysis, still generate best-effort recommendation
```

**Traceability:** TUNE-SAFE-030

---

### ISSUE-M-002: Backup Location and Format Unspecified

**Requirement:** TUNE-SAFE-010 (lines 371-374)
**Severity:** MEDIUM
**Category:** Missing Information

**Problem:**
Specification requires "Store in memory/temp file as backup" but doesn't specify:
- Prefer memory or temp file? Both?
- If temp file: where? (/tmp? same directory as binary?)
- Temp file naming convention?
- Cleanup temp file on success?

**Impact:**
- Implementation ambiguity
- Potential file permission issues
- Leftover temp files if not cleaned up

**Recommended Resolution:**
```
Settings backup (TUNE-SAFE-010):
- Primary: Store in memory (simple struct with old values)
- Secondary: Also write to temp file as safety measure
- Location: std::env::temp_dir() / "wkmp_tuning_backup.json"
- Format: JSON with timestamp, old values
- Cleanup: Remove temp file on successful completion
- Restore: Use memory copy first, fall back to temp file if process crashed
```

**Traceability:** TUNE-SAFE-010

---

### ISSUE-M-003: System Hang Detection Method Unclear

**Requirement:** TUNE-SAFE-020 (lines 376-379)
**Severity:** MEDIUM
**Category:** Missing Information

**Problem:**
Specification requires "If any test hangs >60 seconds: Abort" but doesn't specify:
- How to detect hang? (timeout wrapper? separate watchdog thread?)
- What constitutes "hang" vs. slow system?
- Can user override 60-second timeout?
- What happens to audio device on abort?

**Impact:**
- Complex to implement reliably
- May false-positive on slow systems (Raspberry Pi Zero 2W)
- Resource cleanup on abort unclear

**Recommended Resolution:**
```
Hang detection (TUNE-SAFE-020):
- Method: Spawn test in tokio task with timeout wrapper (tokio::time::timeout)
- Timeout: 60 seconds per test (configurable via hidden CLI arg for slow systems)
- Detection: If test doesn't complete within timeout, consider hung
- Action on hang:
  1. Attempt graceful audio device shutdown
  2. Log error with configuration that caused hang
  3. Mark configuration as TIMEOUT in results
  4. Continue with next test (don't abort entire tuning run)
- Abort only if: 3 consecutive timeouts (system truly unresponsive)
```

**Traceability:** TUNE-SAFE-020

---

### ISSUE-M-004: Recommendation Logic Missing "No Viable Intervals" Case

**Requirement:** TUNE-CURVE-020 (lines 219-229)
**Severity:** MEDIUM
**Category:** Missing Information

**Problem:**
Recommendation logic has "Fallback" case for no interval achieving <512 frames, but doesn't specify what to recommend if:
- All intervals are unstable (even with 4096 frame buffer)
- Only intervals ≥50ms are stable (excessive latency)
- Results are inconsistent (can't determine clear winner)

**Impact:**
- Edge case handling unclear
- User may receive confusing "no recommendation" message

**Recommended Resolution:**
```
Recommendation logic extensions (TUNE-CURVE-020):
- If all intervals unstable: Recommend largest interval + largest buffer tested, warn "system may be inadequate"
- If only slow intervals stable (≥50ms): Recommend best slow interval, warn "high latency unavoidable"
- If results inconsistent: Recommend most conservative stable configuration, warn "results inconsistent, retry recommended"
- Always provide at least one recommendation (even if not ideal)
```

**Traceability:** TUNE-CURVE-020

---

## LOW Priority Issues (Minor, Can Address During Implementation)

### ISSUE-L-001: Success Criteria Metrics Not Measurable During Implementation

**Requirement:** TUNE-SUCCESS-020 (lines 452-456)
**Severity:** LOW
**Category:** Testability

**Problem:**
Success criterion "Results reproducible (±10% variation)" can only be validated by running tuning multiple times, which is expensive (10+ minutes per run). Not practical for unit/integration tests.

**Impact:**
- Can't verify in CI
- Relies on manual validation

**Recommended Resolution:**
```
Reproducibility validation (TUNE-SUCCESS-020):
- Defer to manual validation phase (TUNE-TEST-060)
- In CI: Test that algorithm is deterministic given same mock data
- In manual testing: Run tuning 3 times, verify recommendations within ±10%
- Document as known limitation: Requires real hardware validation
```

**Traceability:** TUNE-SUCCESS-020

---

## Issues by Requirement

| Requirement ID | Issues Found | Highest Severity |
|----------------|--------------|------------------|
| TUNE-TEST-020 | ISSUE-H-001 | HIGH |
| TUNE-DET-020 | ISSUE-H-002 | HIGH |
| TUNE-TEST-030 | ISSUE-H-002 | HIGH |
| TUNE-SEARCH-010 | ISSUE-H-003 | HIGH |
| TUNE-SAFE-030 | ISSUE-M-001 | MEDIUM |
| TUNE-SAFE-010 | ISSUE-M-002 | MEDIUM |
| TUNE-SAFE-020 | ISSUE-M-003 | MEDIUM |
| TUNE-CURVE-020 | ISSUE-M-004 | MEDIUM |
| TUNE-SUCCESS-020 | ISSUE-L-001 | LOW |

**Requirements with no issues:** 22 of 31 (71% clear)

---

## Consistency Check Results

**Cross-requirement analysis:**

✓ **No contradictions found** between requirements
✓ **Timing budgets reasonable:** 30-60s per test × 20-50 tests = 10-50 minutes total (within <15 min thorough target with parallel execution)
✓ **Resource allocations feasible:** 8192 frames × 2 channels × 4 bytes = 64KB per buffer (trivial)
✓ **Interface consistency:** All components use same parameter types (u64, u32, f32)
✓ **Priority alignment:** All high-priority requirements are testable and achievable

---

## Testability Assessment

**All requirements testable:**
- Detection (TUNE-DET-*): ✓ Can mock underrun counts
- Search (TUNE-SRC-*): ✓ Can verify algorithm behavior with mocked tests
- Output (TUNE-OUT-*): ✓ Can verify JSON structure, DB updates
- Integration (TUNE-INT-*): ✓ Can test on CI system
- Algorithm (TUNE-ALG-*, TUNE-SEARCH-*, TUNE-CURVE-*): ✓ Can test with synthetic data
- UI (TUNE-UI-*): ✓ Can capture stdout/stderr
- Architecture (TUNE-ARCH-*): ✓ Can verify component structure
- Safety (TUNE-SAFE-*): ✓ Can test backup/restore, error handling
- Testing (TUNE-TEST-*): ✓ Meta-testable (tests for tests)
- Success Criteria (TUNE-SUCCESS-*): ⚠ ISSUE-L-001 (reproducibility requires manual validation)

**Overall Testability:** 97% (30 of 31 requirements fully testable in CI)

---

## Dependency Validation Results

✓ **All internal dependencies exist**
✓ **All external dependencies already in Cargo.toml**
✓ **No new dependencies required**
✓ **All referenced specifications exist** (SPEC016, IMPL001)
✓ **Hardware dependencies reasonable** (audio device available on development system)

**Overall Dependency Status:** ✓ Ready to implement

---

## Recommendations

### Before Implementation Begins

1. **Resolve HIGH issues** (estimated 1-2 hours):
   - ISSUE-H-001: Add test tone generation specification
   - ISSUE-H-002: Define CPU measurement approach (or mark optional)
   - ISSUE-H-003: Clarify binary search convergence criteria

2. **Acknowledge MEDIUM issues** (can resolve during implementation):
   - ISSUE-M-001: Define anomalous results heuristics
   - ISSUE-M-002: Specify backup file location/format
   - ISSUE-M-003: Detail hang detection method
   - ISSUE-M-004: Add edge case recommendations

3. **Accept LOW issue**:
   - ISSUE-L-001: Reproducibility validation deferred to manual testing

### Implementation Strategy

Given the low number of issues and their nature (mostly missing details, not fundamental flaws):

**Option A (Recommended):** Proceed with implementation after HIGH issues resolved
- Resolve HIGH issues first (1-2 hours)
- Implement with reasonable defaults for MEDIUM issues
- Document assumptions in code comments
- Validate in testing phase

**Option B (Conservative):** Resolve all issues before implementation
- Update specification with resolutions for all 8 issues
- Re-run /plan to verify completeness
- Then proceed with implementation

**Recommendation:** **Option A** - Specification is solid, issues are minor clarifications

---

## Phase 2 Summary

**Completeness:** 71% of requirements fully specified (22 of 31)
**Ambiguity:** 3 HIGH ambiguities require clarification
**Consistency:** ✓ No contradictions found
**Testability:** 97% requirements testable in CI (30 of 31)
**Dependencies:** ✓ All validated and available

**Overall Assessment:** **GOOD** - Specification is implementation-ready after resolving 3 HIGH issues

**Proceed to Phase 3:** ✓ Yes (after HIGH issue resolution)
