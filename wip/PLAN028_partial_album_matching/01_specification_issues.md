# PLAN028: Specification Issues

## Phase 2 Analysis Results

| Severity | Count |
|----------|-------|
| CRITICAL | 0 |
| HIGH | 0 |
| MEDIUM | 2 |
| LOW | 1 |

**Decision:** PROCEED - No blocking issues

---

## Issues Found

### MEDIUM-001: Edge Case - Very Short Partial Files

**Requirement:** REQ-PAM-004 (60% minimum track coverage)

**Issue:** Minimum 60% may be too restrictive for some cases. A 12-track album at 60% = 8 tracks minimum, but 7 tracks might still provide useful AcousticBrainz data.

**Impact:** Some valid partial files may be rejected.

**Recommendation:** Consider 50% minimum as alternative, or make threshold configurable.

**Resolution:** Accept 60% as reasonable balance. Can adjust in future if needed.

---

### MEDIUM-002: Multiple Editions with Similar Partial Match

**Requirement:** REQ-PAM-002 (Partial Track Matching)

**Issue:** Multiple editions may have similar track-1-N durations. Need selection criteria for partial matches.

**Example:** Standard 11-track and Deluxe 14-track editions may both match tracks 1-8.

**Recommendation:**
1. Prefer edition where partial matches more tracks (higher N)
2. If equal, prefer edition with fewer total tracks (closer to complete)
3. Use existing name similarity as tiebreaker

**Resolution:** Use multi-factor scoring with track coverage as additional factor.

---

### LOW-001: Logging Format Consistency

**Requirement:** REQ-PAM-006 (Logging)

**Issue:** Existing log format doesn't have `partial` field. Need to maintain backward compatibility for log parsers.

**Recommendation:** Add new field rather than modifying existing format:
```
Album match complete: matched=true, stage=Some(Stage2), percentage=100.0%, partial_match=8/11
```

**Resolution:** Accepted - add `partial_match` field when applicable, omit for full matches.

---

## Specification Completeness Check

| Requirement | Inputs | Outputs | Behavior | Constraints | Errors | Dependencies |
|-------------|--------|---------|----------|-------------|--------|--------------|
| REQ-PAM-001 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| REQ-PAM-002 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| REQ-PAM-003 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| REQ-PAM-004 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| REQ-PAM-005 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| REQ-PAM-006 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| REQ-PAM-007 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| REQ-PAM-008 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

All requirements are complete and testable.
