# Test Index: Context Engineering Implementation

**Project:** WKMP Music Player - Context Engineering Improvements
**Plan ID:** PLAN003
**Date:** 2025-10-25

---

## Test Summary

**Total Tests:** 23
- **Unit Tests:** 11 (file content verification, structure validation)
- **Integration Tests:** 6 (workflow interaction, cross-file consistency)
- **System Tests:** 6 (end-to-end scenarios, user-facing validation)

**Coverage:** 100% of 13 requirements (16 total with 3 monitoring requirements)

---

## Test Index by Requirement

| Test ID | Type | Requirement | Brief Description | File |
|---------|------|-------------|-------------------|------|
| **Phase 1 - Intervention 1C: Mandate /plan Workflow** |||||
| TC-U-P1-010-01 | Unit | REQ-CE-P1-010 | CLAUDE.md contains mandatory `/plan` section | [tc_u_p1_010_01.md](tc_u_p1_010_01.md) |
| TC-U-P1-010-02 | Unit | REQ-CE-P1-010 | Threshold value specified in CLAUDE.md | [tc_u_p1_010_02.md](tc_u_p1_010_02.md) |
| TC-S-P1-020-01 | System | REQ-CE-P1-020 | Workshop materials exist and are complete | [tc_s_p1_020_01.md](tc_s_p1_020_01.md) |
| TC-S-P1-030-01 | System | REQ-CE-P1-030 | Workshop conducted with attendance recorded | [tc_s_p1_030_01.md](tc_s_p1_030_01.md) |
| **Phase 1 - Intervention 2A: Verbosity Constraints** |||||
| TC-U-P1-040-01 | Unit | REQ-CE-P1-040 | CLAUDE.md contains verbosity standards section | [tc_u_p1_040_01.md](tc_u_p1_040_01.md) |
| TC-U-P1-040-02 | Unit | REQ-CE-P1-040 | Verbosity standards include quantified targets | [tc_u_p1_040_02.md](tc_u_p1_040_02.md) |
| TC-U-P1-050-01 | Unit | REQ-CE-P1-050 | All 6 workflows updated with size targets | [tc_u_p1_050_01.md](tc_u_p1_050_01.md) |
| TC-I-P1-050-01 | Integration | REQ-CE-P1-050 | Size targets consistent across workflows | [tc_i_p1_050_01.md](tc_i_p1_050_01.md) |
| **Phase 1 - Intervention 2D: Summary-First Reading** |||||
| TC-U-P1-060-01 | Unit | REQ-CE-P1-060 | CLAUDE.md contains reading protocol section | [tc_u_p1_060_01.md](tc_u_p1_060_01.md) |
| TC-U-P1-060-02 | Unit | REQ-CE-P1-060 | Reading protocol has 4 required steps | [tc_u_p1_060_02.md](tc_u_p1_060_02.md) |
| TC-U-P1-070-01 | Unit | REQ-CE-P1-070 | All 6 workflows updated with reading guidance | [tc_u_p1_070_01.md](tc_u_p1_070_01.md) |
| TC-I-P1-070-01 | Integration | REQ-CE-P1-070 | Reading guidance consistent with CLAUDE.md | [tc_i_p1_070_01.md](tc_i_p1_070_01.md) |
| **Phase 1 - Intervention 2B: Modular Structure** |||||
| TC-U-P1-080-01 | Unit | REQ-CE-P1-080 | GOV001 contains document standards section | [tc_u_p1_080_01.md](tc_u_p1_080_01.md) |
| TC-U-P1-080-02 | Unit | REQ-CE-P1-080 | Standards specify 300-line and 1200-line thresholds | [tc_u_p1_080_02.md](tc_u_p1_080_02.md) |
| TC-U-P1-090-01 | Unit | REQ-CE-P1-090 | /doc-name checks size and recommends structure | [tc_u_p1_090_01.md](tc_u_p1_090_01.md) |
| TC-I-P1-090-01 | Integration | REQ-CE-P1-090 | /doc-name recommendations match GOV001 standards | [tc_i_p1_090_01.md](tc_i_p1_090_01.md) |
| TC-U-P1-100-01 | Unit | REQ-CE-P1-100 | All 3 template files exist | [tc_u_p1_100_01.md](tc_u_p1_100_01.md) |
| TC-S-P1-100-01 | System | REQ-CE-P1-100 | Templates are usable by team member | [tc_s_p1_100_01.md](tc_s_p1_100_01.md) |
| **Phase 2 - GOV001 Formalization** |||||
| TC-I-P2-010-01 | Integration | REQ-CE-P2-010 | GOV001 draft matches Phase 1 implementation | [tc_i_p2_010_01.md](tc_i_p2_010_01.md) |
| TC-S-P2-020-01 | System | REQ-CE-P2-020 | GOV001 update reviewed and approved | [tc_s_p2_020_01.md](tc_s_p2_020_01.md) |
| TC-S-P2-030-01 | System | REQ-CE-P2-030 | Team education session conducted | [tc_s_p2_030_01.md](tc_s_p2_030_01.md) |
| **Monitoring and Enforcement** |||||
| TC-S-MON-010-01 | System | REQ-CE-MON-010 | Document size metrics collected and tracked | [tc_s_mon_010_01.md](tc_s_mon_010_01.md) |
| TC-S-MON-020-01 | System | REQ-CE-MON-020 | `/plan` usage tracked and reported | [tc_s_mon_020_01.md](tc_s_mon_020_01.md) |
| TC-S-MON-030-01 | System | REQ-CE-MON-030 | Team feedback collected via survey | [tc_s_mon_030_01.md](tc_s_mon_030_01.md) |

---

## Test Execution Order

### Phase 1 Tests (Week 1-2)

**Session 1: CLAUDE.md Updates**
1. TC-U-P1-010-01 (mandatory `/plan` section exists)
2. TC-U-P1-010-02 (threshold specified)
3. TC-U-P1-040-01 (verbosity standards section exists)
4. TC-U-P1-040-02 (targets quantified)
5. TC-U-P1-060-01 (reading protocol section exists)
6. TC-U-P1-060-02 (4 steps present)

**Session 2: Workflow Updates**
7. TC-U-P1-050-01 (all 6 workflows updated - size)
8. TC-U-P1-070-01 (all 6 workflows updated - reading)
9. TC-I-P1-050-01 (size targets consistent)
10. TC-I-P1-070-01 (reading guidance consistent)

**Session 3: Templates**
11. TC-U-P1-100-01 (all 3 templates exist)
12. TC-S-P1-100-01 (templates usable)

**Session 4: Workshop**
13. TC-S-P1-020-01 (materials complete)
14. TC-S-P1-030-01 (workshop conducted)

**Session 5: GOV001 Draft** (Phase 1, prepare for Phase 2)
15. TC-U-P1-080-01 (standards section exists)
16. TC-U-P1-080-02 (thresholds specified)
17. TC-U-P1-090-01 (/doc-name enhanced)
18. TC-I-P1-090-01 (/doc-name matches GOV001)

### Phase 2 Tests (Week 3-4)

**Session 6: GOV001 Formalization**
19. TC-I-P2-010-01 (draft matches implementation)
20. TC-S-P2-020-01 (reviewed and approved)
21. TC-S-P2-030-01 (education conducted)

**Session 7: Monitoring**
22. TC-S-MON-010-01 (document size tracked)
23. TC-S-MON-020-01 (`/plan` usage tracked)
24. TC-S-MON-030-01 (feedback collected)

---

## Test Types Distribution

**Unit Tests (11):** File content, structure, presence/absence
- Fast execution (<1 minute total)
- Automatable (grep, file existence checks)
- High confidence in pass/fail

**Integration Tests (6):** Cross-file consistency, alignment
- Medium execution (2-5 minutes)
- Semi-automatable (requires multi-file analysis)
- Medium confidence (some subjective assessment)

**System Tests (6):** End-to-end scenarios, user-facing
- Slow execution (variable, depends on human actions)
- Manual execution (workshop, surveys, approvals)
- Lower confidence (depends on human judgment)

---

## Pass/Fail Criteria Summary

**Automated Tests (17):**
- File exists: `test -f [path]` returns 0
- Section exists: `grep -q "[section header]" [file]` returns 0
- Threshold specified: Regex match for specific value
- Count matches: Number of files/sections equals expected

**Manual Tests (6):**
- Workshop materials complete: Human review checklist
- Workshop conducted: Attendance sheet, completion confirmation
- GOV001 approved: Commit exists in repository
- Education conducted: Attendance sheet, materials delivered
- Metrics collected: Spreadsheet populated, data valid
- Feedback collected: Survey responses ≥ minimum count

---

## Test Data Requirements

**None for automated tests** (tests verify file structure/content only)

**For manual tests:**
- Workshop materials (slides, handouts)
- Example specification for workshop
- Attendance sheets
- Survey tool (Google Forms or equivalent)
- Metrics tracking spreadsheet

---

## Test Environment Requirements

**Development Machine:**
- File system access (c:\Users\Mango Cat\Dev\McRhythm)
- Git repository access
- Markdown viewer
- Bash/shell for automated tests (Git Bash or WSL)

**Team Environment:**
- Meeting space (2 hours, Intervention 1C)
- Meeting space (2 hours, Phase 2 education)
- Survey tool access
- Email/communication for review requests

---

## Traceability Notes

**Forward Traceability:** Every requirement has ≥1 test
**Backward Traceability:** Every test traces to specific requirement
**Coverage:** 100% (13 functional requirements + 3 monitoring requirements = 16 total)

See [traceability_matrix.md](traceability_matrix.md) for complete requirement ↔ test mapping.

---

**Test Index Complete**
**Next:** Review individual test specifications
**Status:** Ready for implementation
