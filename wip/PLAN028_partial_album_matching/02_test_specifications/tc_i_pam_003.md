# TC-I-PAM-003: Logging Shows Partial Match Details

**Test Type:** Integration Test
**Requirement:** REQ-PAM-006
**Priority:** P2

---

## Scope

Verify partial match logging format

---

## Test Case

**Given:**
- Partial match completes for 8/11 tracks
- Match percentage: 100%
- Stage: Stage2

**When:** Match completes and is logged

**Then:** Log output contains:
```
Album match complete: matched=true, stage=Some(Stage2), percentage=100.0%, partial_match=8/11
```

---

## Verification

- Log contains `partial_match=8/11` field
- Field format is `N/M` where N=matched tracks, M=total tracks
- Field omitted for full matches (backward compatibility)
- Field present for all partial matches

---

## Pass Criteria

- Log format matches specification
- Existing log parsers not broken (new field added, existing fields unchanged)
- Partial match clearly distinguishable from full match in logs

---

## Estimated Effort

Implementation: 10 minutes
Test writing: 10 minutes
