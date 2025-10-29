# Specification Issues - wkmp-ap Technical Debt Remediation

**Plan:** PLAN008
**Phase:** 2 - Specification Completeness Verification
**Date:** 2025-10-29

---

## Executive Summary

**Specification Quality:** GOOD - Minor issues only
**Issue Count:** 5 issues found
- CRITICAL: 0
- HIGH: 0
- MEDIUM: 3
- LOW: 2

**Recommendation:** ✅ **PROCEED TO IMPLEMENTATION**

All issues are MEDIUM or LOW severity. No blocking issues found. Specification provides sufficient detail for implementation.

---

## Issues Found

### MEDIUM-001: Authentication Test Coverage Incomplete

**Affected Requirements:** REQ-DEBT-SEC-001-*
**Category:** Testability
**Severity:** MEDIUM

**Issue:**
Specification provides one acceptance test for POST authentication but doesn't specify:
- PUT request authentication test
- Edge case: JSON body without shared_secret field
- Edge case: Malformed JSON body
- Edge case: shared_secret field with wrong type (not string)

**Impact:** Test suite may miss edge cases

**Recommendation:**
Add explicit test cases for:
```rust
// PUT authentication
#[tokio::test]
async fn test_put_requires_authentication() { ... }

// Missing shared_secret field
#[tokio::test]
async fn test_post_without_secret_field() { ... }

// Malformed JSON
#[tokio::test]
async fn test_post_malformed_json() { ... }

// Wrong type for shared_secret
#[tokio::test]
async fn test_post_secret_wrong_type() { ... }
```

**Resolution:** Add to test specifications in Phase 3

---

### MEDIUM-002: Buffer Capacity Validation Not Specified

**Affected Requirements:** REQ-DEBT-FUNC-002-*
**Category:** Completeness - Error Handling
**Severity:** MEDIUM

**Issue:**
Specification doesn't define behavior when database settings contain invalid values:
- Negative buffer capacity
- Zero buffer capacity
- Capacity smaller than headroom
- Non-numeric values in TEXT field

**Current Spec:**
> "SHALL use compiled defaults if settings are NULL/missing"

**Missing:**
> "SHALL validate settings are positive integers and capacity > headroom"

**Impact:** Could create buffers with invalid parameters

**Recommendation:**
Add validation requirement:

```
REQ-DEBT-FUNC-002-035: The system SHALL validate buffer settings are positive integers
REQ-DEBT-FUNC-002-036: If capacity ≤ headroom, SHALL log warning and use compiled defaults
REQ-DEBT-FUNC-002-037: If settings are non-numeric, SHALL log warning and use compiled defaults
```

**Resolution:** Add validation logic in implementation, document in code comments

---

### MEDIUM-003: Decoder Telemetry Query Frequency Unspecified

**Affected Requirements:** REQ-DEBT-FUNC-003-*
**Category:** Completeness - Performance
**Severity:** MEDIUM

**Issue:**
Specification doesn't define how often telemetry is queried:
- On every status request (could be many per second)?
- Cached and refreshed periodically?
- Only when developer UI is open?

**Current Spec:**
> "BufferChainInfo SHALL include decoder_state populated from decoder worker status"

**Missing:**
- Query frequency
- Caching strategy
- Performance budget

**Impact:** Could query telemetry too frequently, impacting performance

**Recommendation:**
Clarify in implementation:
- Query telemetry only when `get_buffer_chain_state()` is called
- No automatic background polling
- Implementer decides if caching needed based on profiling

**Resolution:** Document query strategy in implementation, profile if needed

---

### LOW-001: Test Data Not Specified for Duration Tests

**Affected Requirements:** REQ-DEBT-FUNC-005-*
**Category:** Testability - Test Data
**Severity:** LOW

**Issue:**
Acceptance test says "short passage (3 seconds)" but doesn't specify:
- Audio format (MP3, FLAC, WAV?)
- Sample rate (44.1kHz, 48kHz?)
- Channels (stereo assumed, but not stated)
- How to create test file

**Current Spec:**
```rust
let passage = create_test_passage_3s();
```

**Missing:** Definition of `create_test_passage_3s()` function

**Impact:** Test implementer must decide audio format

**Recommendation:**
Specify in test implementation:
- Use WAV format (simplest, no decode complexity)
- 44.1kHz stereo (standard)
- Generate programmatically (sine wave, 3.0 seconds exact)

**Resolution:** Document in test specification (Phase 3)

---

### LOW-002: Clipping Log Format Not Specified

**Affected Requirements:** REQ-DEBT-FUTURE-003-010
**Category:** Completeness - Output Format
**Severity:** LOW

**Issue:**
Requirement says "SHALL log a warning" but doesn't specify:
- Log level (warn! assumed, but not stated)
- Message format
- Frequency (every clipped frame? Rate-limited?)

**Current Spec:**
> "SHALL log warning when audio samples exceed ±1.0"

**Impact:** Logs could spam if many frames clip

**Recommendation:**
Add implementation details:
```rust
// Log first occurrence, then rate-limit to once per second
if clipping_detected && last_clip_warning.elapsed() > Duration::from_secs(1) {
    warn!("Audio clipping detected at frame {}: L={:.2}, R={:.2}",
          frame_pos, mixed_left, mixed_right);
    last_clip_warning = Instant::now();
}
```

**Resolution:** Document rate-limiting in implementation

---

## Issues by Category

### Completeness (3 issues)
- MEDIUM-002: Buffer validation not specified
- MEDIUM-003: Telemetry query frequency unspecified
- LOW-002: Clipping log format not specified

### Testability (2 issues)
- MEDIUM-001: Authentication test coverage incomplete
- LOW-001: Test data not specified

### No Issues Found For
- ✅ Inputs/Outputs: Well-specified for all requirements
- ✅ Behavior: Clear processing steps defined
- ✅ Constraints: Performance/timing specified
- ✅ Dependencies: All identified and documented
- ✅ Ambiguity: No vague language found
- ✅ Consistency: No contradictions found

---

## Verification Status by Requirement Batch

### Batch 1: REQ-DEBT-SEC-001-* (Authentication)
- **Completeness:** ✅ GOOD
- **Ambiguity:** ✅ CLEAR
- **Testability:** ⚠️ MEDIUM-001 (test coverage)
- **Dependencies:** ✅ SATISFIED

### Batch 2: REQ-DEBT-FUNC-001-* (File Paths)
- **Completeness:** ✅ GOOD
- **Ambiguity:** ✅ CLEAR
- **Testability:** ✅ GOOD
- **Dependencies:** ✅ SATISFIED

### Batch 3: REQ-DEBT-FUNC-002-* (Buffer Config)
- **Completeness:** ⚠️ MEDIUM-002 (validation)
- **Ambiguity:** ✅ CLEAR
- **Testability:** ✅ GOOD
- **Dependencies:** ✅ SATISFIED

### Batch 4: REQ-DEBT-FUNC-003-* (Telemetry)
- **Completeness:** ⚠️ MEDIUM-003 (query frequency)
- **Ambiguity:** ✅ CLEAR
- **Testability:** ✅ GOOD
- **Dependencies:** ✅ SATISFIED

### Batch 5: REQ-DEBT-FUNC-004-* (Albums)
- **Completeness:** ✅ GOOD
- **Ambiguity:** ✅ CLEAR
- **Testability:** ✅ GOOD
- **Dependencies:** ✅ SATISFIED

### Batch 6: REQ-DEBT-FUNC-005-* (Duration)
- **Completeness:** ✅ GOOD
- **Ambiguity:** ✅ CLEAR
- **Testability:** ⚠️ LOW-001 (test data)
- **Dependencies:** ✅ SATISFIED

### Batch 7: REQ-DEBT-QUALITY-* (Code Quality)
- **Completeness:** ✅ GOOD
- **Ambiguity:** ✅ CLEAR
- **Testability:** ✅ GOOD
- **Dependencies:** ✅ SATISFIED

### Batch 8: REQ-DEBT-FUTURE-* (Enhancements)
- **Completeness:** ⚠️ LOW-002 (log format)
- **Ambiguity:** ✅ CLEAR
- **Testability:** ✅ GOOD
- **Dependencies:** ✅ SATISFIED

---

## Resolution Strategy

### Issues Requiring Specification Updates
**None** - All issues can be resolved in implementation/test phases

### Issues Resolved in Phase 3 (Test Specifications)
- MEDIUM-001: Add comprehensive authentication test cases
- LOW-001: Specify test data generation for duration tests

### Issues Resolved in Implementation
- MEDIUM-002: Add validation logic with logging
- MEDIUM-003: Document telemetry query strategy
- LOW-002: Implement rate-limited clipping logs

---

## Recommendation

**✅ PROCEED TO PHASE 3: ACCEPTANCE TEST DEFINITION**

**Rationale:**
- No CRITICAL or HIGH issues blocking implementation
- MEDIUM issues are implementer decisions, not specification gaps
- LOW issues are minor details easily addressed
- Specification provides sufficient detail for success
- All requirements testable with minor test case additions

**Next Steps:**
1. Proceed to Phase 3: Define acceptance tests
2. Address MEDIUM-001 by adding authentication edge case tests
3. Address LOW-001 by specifying test data generation
4. Document MEDIUM-002, MEDIUM-003, LOW-002 resolutions in implementation

---

## Phase 2 Verification Checklist

- [x] All 37 requirements analyzed for completeness
- [x] Ambiguity check performed (no vague language found)
- [x] Consistency check performed (no contradictions found)
- [x] Testability check performed (all requirements testable)
- [x] Dependency validation performed (all dependencies available)
- [x] Issues documented with severity and recommendations
- [x] Resolution strategy defined
- [x] Proceed/stop recommendation made

**Phase 2 Complete** - Ready for Phase 3
