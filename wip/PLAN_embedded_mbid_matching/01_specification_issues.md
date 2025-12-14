# Specification Issues

**Plan:** PLAN-EMBID-001
**Specification:** SPEC-EMBID-001
**Phase 2 Analysis Date:** 2024-12-14

---

## Summary

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | - |
| HIGH | 1 | Noted for future |
| MEDIUM | 2 | Documented |
| LOW | 1 | Documented |

**Overall Status:** No blocking issues. Proceed with implementation.

---

## Issues Found

### HIGH-001: MB Release ID Not Used

**Severity:** HIGH
**Confidence:** CERTAIN
**Requirement:** Section 3.3 Data Flow (line 192-194)

**Description:**
The specification mentions extracting MB Release ID and MB Artist ID, but these are not used in the confidence tier logic. These could provide additional validation signals.

**Impact:**
- Missed opportunity for additional confidence boost
- Could detect mismatched release/recording combinations

**Recommendation:**
Document as future enhancement. Current implementation focuses on Recording ID only as specified in FR-01 through FR-08.

**Resolution:** Noted for future enhancement. Not blocking.

---

### MEDIUM-001: Error Handling for Corrupt Tags

**Severity:** MEDIUM
**Confidence:** HIGH
**Requirement:** FR-01 (line 226)

**Description:**
The specification does not address how to handle corrupt or malformed ID3 tags that may cause parsing errors.

**Impact:**
- Potential panics or crashes on malformed files
- May silently skip files

**Recommendation:**
Add error handling that logs warning and falls back to AcoustID when ID3 parsing fails. This is standard practice in existing code.

**Resolution:** Handle in implementation (Increment 1) following existing error handling patterns.

---

### MEDIUM-002: Caching Strategy Not Specified

**Severity:** MEDIUM
**Confidence:** MEDIUM
**Requirement:** NFR-01 (line 239)

**Description:**
The specification requires < 50ms per file but doesn't specify whether results should be cached to avoid re-extracting from the same file.

**Impact:**
- Repeated extractions may waste resources
- Database storage of tier results not addressed

**Recommendation:**
Follow existing caching pattern (e.g., duration_cache). Store tier results in database with file_hash key.

**Resolution:** Use existing caching infrastructure. Add cache lookup before extraction.

---

### LOW-001: Logging Format Not Specified

**Severity:** LOW
**Confidence:** HIGH
**Requirement:** FR-08 (line 233)

**Description:**
FR-08 requires logging confidence tier but doesn't specify format (JSON, text, etc.).

**Impact:**
- Minor inconsistency risk

**Recommendation:**
Follow existing tracing format used throughout wkmp-ai (structured JSON with `tracing` crate).

**Resolution:** Use standard tracing format.

---

## Ambiguity Analysis

| Requirement | Ambiguous? | Resolution |
|-------------|------------|------------|
| FR-01 | No | Clear: extract from ID3 tags |
| FR-02 | No | Clear: extract ISRC |
| FR-03 | No | Clear: validate UUID format |
| FR-04 | No | Clear: tier assignment logic in spec |
| FR-05 | No | Clear: skip when embedded present |
| FR-06 | No | Clear: fall back to AcoustID |
| FR-07 | Minor | "Optionally" - decided by config or always? → Config-driven |
| FR-08 | Minor | Format → Use existing tracing |
| NFR-01 | No | Clear: < 50ms target |
| NFR-02 | No | Clear: >= 98% target |
| NFR-03 | No | Clear: < 0.1% target |
| NFR-04 | No | Clear: offline capability |

**Ambiguity Status:** All ambiguities resolved with reasonable interpretations.

---

## Consistency Check

| Check | Status |
|-------|--------|
| Tier definitions match throughout | PASS |
| Confidence scores consistent | PASS |
| Flow diagram matches data flow | PASS |
| Requirements complete for flow | PASS |
| No timing conflicts | PASS |

---

## Decision: Proceed

All issues are non-blocking:
- HIGH-001: Future enhancement
- MEDIUM-001, MEDIUM-002: Handle in implementation
- LOW-001: Follow existing patterns

**Recommendation:** Proceed to Phase 3 (Test Definition)
