# PLAN028: Partial Album Matching - Test Index

## Test Summary

| Total Tests | Unit | Integration | System | Manual |
|-------------|------|-------------|--------|--------|
| 12 | 6 | 3 | 2 | 1 |

---

## Test Catalog

| Test ID | Type | Requirement | Description | Priority |
|---------|------|-------------|-------------|----------|
| TC-U-PAM-001 | Unit | REQ-PAM-001 | Detect partial album candidate (50-85% ratio) | P0 |
| TC-U-PAM-002 | Unit | REQ-PAM-001 | Reject below 50% ratio | P0 |
| TC-U-PAM-003 | Unit | REQ-PAM-001 | Skip above 85% ratio (use full match) | P0 |
| TC-U-PAM-004 | Unit | REQ-PAM-002 | Calculate cumulative track durations | P0 |
| TC-U-PAM-005 | Unit | REQ-PAM-002 | Find best N tracks matching file duration | P0 |
| TC-U-PAM-006 | Unit | REQ-PAM-004 | Enforce 60% minimum track coverage | P1 |
| TC-I-PAM-001 | Integration | REQ-PAM-002, REQ-PAM-003 | Partial match returns valid MBIDs | P0 |
| TC-I-PAM-002 | Integration | REQ-PAM-005 | Quality threshold applies to partial matches | P1 |
| TC-I-PAM-003 | Integration | REQ-PAM-006 | Logging shows partial match details | P2 |
| TC-S-PAM-001 | System | REQ-PAM-001-006 | Fluke/Puppy.mp3 matches 8/11 tracks | P0 |
| TC-S-PAM-002 | System | REQ-PAM-007 | No regressions on 200-album test suite | P0 |
| TC-M-PAM-001 | Manual | REQ-PAM-008 | Performance impact <5% | P2 |

---

## Test Files

- [tc_u_pam_001.md](tc_u_pam_001.md) - Partial candidate detection
- [tc_u_pam_002.md](tc_u_pam_002.md) - Below threshold rejection
- [tc_u_pam_003.md](tc_u_pam_003.md) - Above threshold skip
- [tc_u_pam_004.md](tc_u_pam_004.md) - Cumulative duration calculation
- [tc_u_pam_005.md](tc_u_pam_005.md) - Best N tracks selection
- [tc_u_pam_006.md](tc_u_pam_006.md) - Minimum coverage enforcement
- [tc_i_pam_001.md](tc_i_pam_001.md) - MBID return verification
- [tc_i_pam_002.md](tc_i_pam_002.md) - Quality threshold verification
- [tc_i_pam_003.md](tc_i_pam_003.md) - Logging verification
- [tc_s_pam_001.md](tc_s_pam_001.md) - Puppy.mp3 system test
- [tc_s_pam_002.md](tc_s_pam_002.md) - Regression test
- [tc_m_pam_001.md](tc_m_pam_001.md) - Performance test

---

## Coverage Summary

All 8 requirements have at least one acceptance test:
- REQ-PAM-001: TC-U-PAM-001, TC-U-PAM-002, TC-U-PAM-003
- REQ-PAM-002: TC-U-PAM-004, TC-U-PAM-005, TC-I-PAM-001
- REQ-PAM-003: TC-I-PAM-001
- REQ-PAM-004: TC-U-PAM-006
- REQ-PAM-005: TC-I-PAM-002
- REQ-PAM-006: TC-I-PAM-003
- REQ-PAM-007: TC-S-PAM-002
- REQ-PAM-008: TC-M-PAM-001
