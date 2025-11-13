# Test Index: PLAN023 - WKMP-AI Ground-Up Recode

**Plan:** PLAN023 - wkmp-ai Ground-Up Recode
**Created:** 2025-01-08
**Total Tests:** 78 (estimated: ~1.5-2 tests per requirement ID average)

---

## Test Organization

**Test Naming Convention:**
- `TC-[Type]-[ReqID]-[Sequence]`
- Type: U (Unit), I (Integration), S (System), M (Manual)
- ReqID: Requirement number (010, 011, etc.)
- Sequence: 01, 02, 03...

**Example:** `TC-U-021-01` = Unit test for REQ-AI-021, test #1

---

## Quick Reference by Requirement

| Req ID | Requirement | Unit Tests | Integration Tests | System Tests | Total |
|--------|-------------|------------|-------------------|--------------|-------|
| REQ-AI-010 | Per-Song Import Workflow (parent) | - | - | TC-S-010-01 | 1 |
| REQ-AI-011 | Phase 0: Passage Boundary Detection | TC-U-011-01, TC-U-011-02 | - | - | 2 |
| REQ-AI-012 | Phase 1-6: Per-Song Processing | - | TC-I-012-01 | TC-S-012-01 | 2 |
| REQ-AI-013 | Per-Song Error Isolation | TC-U-013-01 | TC-I-013-01 | - | 2 |
| REQ-AI-020 | Identity Resolution (parent) | - | - | - | 0 |
| REQ-AI-021 | Multi-Source MBID Resolution | TC-U-021-01, TC-U-021-02 | TC-I-021-01 | - | 3 |
| REQ-AI-022 | Conflict Detection | TC-U-022-01, TC-U-022-02 | - | - | 2 |
| REQ-AI-023 | Bayesian Update Algorithm | TC-U-023-01, TC-U-023-02, TC-U-023-03 | - | - | 3 |
| REQ-AI-024 | Low-Confidence Flagging | TC-U-024-01, TC-U-024-02 | - | - | 2 |
| REQ-AI-030 | Metadata Fusion (parent) | - | - | - | 0 |
| REQ-AI-031 | Multi-Source Metadata Extraction | TC-U-031-01 | TC-I-031-01 | - | 2 |
| REQ-AI-032 | Quality Scoring | TC-U-032-01 | - | - | 1 |
| REQ-AI-033 | Field-Wise Selection Strategy | TC-U-033-01, TC-U-033-02, TC-U-033-03 | - | - | 3 |
| REQ-AI-034 | Consistency Validation | TC-U-034-01 | - | - | 1 |
| REQ-AI-040 | Musical Flavor Synthesis (parent) | - | - | - | 0 |
| REQ-AI-041 | Multi-Source Flavor Extraction | TC-U-041-01 | TC-I-041-01 | - | 2 |
| REQ-AI-042 | Source Priority and Confidence | TC-U-042-01 | - | - | 1 |
| REQ-AI-043 | Characteristic-Wise Weighted Averaging | TC-U-043-01, TC-U-043-02 | - | - | 2 |
| REQ-AI-044 | Normalization | TC-U-044-01, TC-U-044-02 | - | - | 2 |
| REQ-AI-045 | Completeness Scoring | TC-U-045-01 | - | - | 1 |
| REQ-AI-050 | Passage Boundary Detection (parent) | - | - | - | 0 |
| REQ-AI-051 | Silence Detection (Baseline) | TC-U-051-01 | - | - | 1 |
| REQ-AI-052 | Multi-Strategy Fusion (Future) | - | - | - | 0 |
| REQ-AI-053 | Boundary Validation | TC-U-053-01, TC-U-053-02 | - | - | 2 |
| REQ-AI-060 | Quality Validation (parent) | - | - | - | 0 |
| REQ-AI-061 | Title Consistency Check | TC-U-061-01, TC-U-061-02, TC-U-061-03 | - | - | 3 |
| REQ-AI-062 | Duration Consistency Check | TC-U-062-01, TC-U-062-02 | - | - | 2 |
| REQ-AI-063 | Genre-Flavor Alignment Check | TC-U-063-01, TC-U-063-02 | - | - | 2 |
| REQ-AI-064 | Overall Quality Score | TC-U-064-01 | - | - | 1 |
| REQ-AI-070 | Real-Time SSE Event Streaming (parent) | - | - | - | 0 |
| REQ-AI-071 | Event Types | TC-U-071-01 | TC-I-071-01 | TC-S-071-01 | 3 |
| REQ-AI-072 | Event Format | TC-U-072-01 | - | - | 1 |
| REQ-AI-073 | Event Throttling | TC-U-073-01, TC-U-073-02 | TC-I-073-01 | - | 3 |
| REQ-AI-080 | Database Schema Extensions (parent) | - | - | - | 0 |
| REQ-AI-081 | Flavor Source Provenance | TC-I-081-01 | - | - | 1 |
| REQ-AI-082 | Metadata Source Provenance | TC-I-082-01 | - | - | 1 |
| REQ-AI-083 | Identity Resolution Tracking | TC-I-083-01 | - | - | 1 |
| REQ-AI-084 | Quality Scores | TC-I-084-01 | - | - | 1 |
| REQ-AI-085 | Validation Flags | TC-I-085-01 | - | - | 1 |
| REQ-AI-086 | Import Metadata | TC-I-086-01 | - | - | 1 |
| REQ-AI-087 | Import Provenance Log Table | TC-I-087-01 | - | - | 1 |
| REQ-AI-NF-010 | Performance (parent) | - | - | - | 0 |
| REQ-AI-NF-011 | Sequential Processing Performance | - | - | TC-S-NF-011-01 | 1 |
| REQ-AI-NF-012 | Parallel Extraction | TC-U-NF-012-01 | - | - | 1 |
| REQ-AI-NF-020 | Reliability (parent) | - | - | - | 0 |
| REQ-AI-NF-021 | Error Isolation | TC-U-NF-021-01 | TC-I-NF-021-01 | - | 2 |
| REQ-AI-NF-022 | Graceful Degradation | TC-U-NF-022-01, TC-U-NF-022-02 | TC-I-NF-022-01 | - | 3 |
| REQ-AI-NF-030 | Maintainability (parent) | - | - | - | 0 |
| REQ-AI-NF-031 | Modular Architecture | TC-M-NF-031-01 | - | - | 1 |
| REQ-AI-NF-032 | Testability | TC-M-NF-032-01 | - | - | 1 |
| REQ-AI-NF-040 | Extensibility (parent) | - | - | - | 0 |
| REQ-AI-NF-041 | New Source Integration | TC-U-NF-041-01 | - | - | 1 |
| REQ-AI-NF-042 | Future Optimizations | - | - | - | 0 |

**Test Count Summary:**
- **Unit Tests:** 51
- **Integration Tests:** 17
- **System Tests:** 4
- **Manual Tests:** 4
- **TOTAL:** 76 tests

**Coverage:** 100% of P0 and P1 requirements have tests
- P2 requirements (REQ-AI-052, REQ-AI-NF-042): No tests (future enhancements)
- Parent requirements (REQ-AI-010, REQ-AI-020, etc.): Covered by child requirement tests

---

## Test Files

**Detailed test specifications are in individual files:**
- Unit tests: `tc_u_[reqid]_[seq].md`
- Integration tests: `tc_i_[reqid]_[seq].md`
- System tests: `tc_s_[reqid]_[seq].md`
- Manual tests: `tc_m_[reqid]_[seq].md`

**Example:**
- `tc_u_021_01.md` - Unit test for REQ-AI-021 (Bayesian update agreement case)
- `tc_i_012_01.md` - Integration test for REQ-AI-012 (per-song workflow)
- `tc_s_010_01.md` - System test for REQ-AI-010 (end-to-end import)

---

## Test Priority Levels

**Critical Path Tests (Run First):**
1. TC-S-010-01 (End-to-end import workflow)
2. TC-I-012-01 (Per-song processing)
3. TC-U-023-01/02/03 (Bayesian update algorithm)
4. TC-U-043-01/02 (Characteristic-wise fusion)
5. TC-U-044-01/02 (Normalization)

**Foundational Tests (Run Next):**
- All Unit tests for Tier 1 extractors (TC-U-031, TC-U-041)
- All Unit tests for Tier 2 fusers (TC-U-021, TC-U-033, TC-U-043)
- All Unit tests for Tier 3 validators (TC-U-061, TC-U-062, TC-U-063, TC-U-064)

**Integration Tests (After Unit Tests Pass):**
- Database schema tests (TC-I-081 through TC-I-087)
- SSE event tests (TC-I-071, TC-I-073)
- Error handling tests (TC-I-013, TC-I-NF-021, TC-I-NF-022)

**System Tests (After Integration Tests Pass):**
- Performance tests (TC-S-NF-011)
- End-to-end scenarios (TC-S-012, TC-S-071)

**Manual Tests (After All Automated Tests Pass):**
- Architecture review (TC-M-NF-031)
- Test coverage verification (TC-M-NF-032)

---

## Traceability Matrix

See `traceability_matrix.md` for complete requirement → test → implementation mapping.

---

## Test Execution Order

**Suggested Execution Sequence:**

**Stage 1: Core Algorithm Unit Tests**
1. Bayesian update (TC-U-023-01/02/03)
2. Normalization (TC-U-044-01/02)
3. Weighted averaging (TC-U-043-01/02)
4. Fuzzy matching (TC-U-061-01/02/03)

**Stage 2: Extractor Unit Tests**
5. ID3 extraction (TC-U-031-01)
6. Flavor extraction (TC-U-041-01)
7. Boundary detection (TC-U-051-01, TC-U-053-01/02)

**Stage 3: Fusion Unit Tests**
8. Identity resolution (TC-U-021-01/02, TC-U-022-01/02, TC-U-024-01/02)
9. Metadata fusion (TC-U-032-01, TC-U-033-01/02/03, TC-U-034-01)
10. Flavor synthesis (TC-U-042-01, TC-U-045-01)

**Stage 4: Validation Unit Tests**
11. Consistency checks (TC-U-062-01/02, TC-U-063-01/02)
12. Quality scoring (TC-U-064-01)

**Stage 5: Event System Tests**
13. Event types (TC-U-071-01, TC-U-072-01)
14. Event throttling (TC-U-073-01/02)

**Stage 6: Integration Tests**
15. Per-song workflow (TC-I-012-01)
16. Error isolation (TC-I-013-01, TC-I-NF-021-01)
17. Graceful degradation (TC-I-NF-022-01)
18. Database schema (TC-I-081 through TC-I-087)
19. SSE events (TC-I-071-01, TC-I-073-01)

**Stage 7: System Tests**
20. End-to-end import (TC-S-010-01, TC-S-012-01)
21. SSE real-time events (TC-S-071-01)
22. Performance baseline (TC-S-NF-011-01)

**Stage 8: Manual Verification**
23. Architecture review (TC-M-NF-031-01)
24. Test coverage check (TC-M-NF-032-01)

---

**Navigation:**
- **Start Here:** This index (quick reference)
- **Detailed Tests:** Individual tc_*.md files
- **Traceability:** traceability_matrix.md
- **Back to Plan:** ../requirements_index.md
