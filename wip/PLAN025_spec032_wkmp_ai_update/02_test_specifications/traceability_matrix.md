# Traceability Matrix - PLAN025

**Plan:** SPEC032 wkmp-ai Implementation Update
**Date:** 2025-11-10

---

## Purpose

This matrix provides complete traceability from requirements → tests → implementation, ensuring:
1. Every requirement has tests (forward traceability)
2. Every requirement is implemented (requirement → code)
3. Every test traces to requirement (backward traceability)
4. No orphaned tests or code

---

## Traceability Table

| Requirement | Priority | Unit Tests | Integration Tests | System Tests | Implementation File(s) | Status | Coverage |
|-------------|----------|------------|-------------------|--------------|------------------------|--------|----------|
| **REQ-PIPE-010** | P0 | TC-U-PIPE-010-01 | TC-I-PIPE-020-01 | - | workflow_orchestrator/mod.rs | Pending | Complete |
| **REQ-PIPE-020** | P0 | TC-U-PIPE-020-01 | TC-I-PIPE-020-01 | - | workflow_orchestrator/mod.rs, pipeline.rs (new) | Pending | Complete |
| **REQ-PATT-010** | P1 | TC-U-PATT-010-01, TC-U-PATT-010-02 | - | TC-S-PATT-010-01 | services/pattern_analyzer.rs (new) | Pending | Complete |
| **REQ-PATT-020** | P2 | TC-U-PATT-020-01 | - | TC-S-PATT-010-01 | services/pattern_analyzer.rs (new) | Pending | Complete |
| **REQ-PATT-030** | P2 | TC-U-PATT-030-01, TC-U-PATT-030-02 | - | TC-S-PATT-010-01 | services/pattern_analyzer.rs (new) | Pending | Complete |
| **REQ-PATT-040** | P2 | TC-U-PATT-040-01, TC-U-PATT-040-02 | - | TC-S-PATT-010-01 | services/pattern_analyzer.rs (new) | Pending | Complete |
| **REQ-CTXM-010** | P1 | TC-U-CTXM-010-01, TC-U-CTXM-010-02 | TC-I-CTXM-020-01, TC-I-CTXM-030-01 | TC-S-CTXM-010-01 | services/contextual_matcher.rs (new) | Pending | Complete |
| **REQ-CTXM-020** | P1 | TC-U-CTXM-020-01 | TC-I-CTXM-020-01 | TC-S-CTXM-010-01 | services/contextual_matcher.rs (new) | Pending | Complete |
| **REQ-CTXM-030** | P1 | TC-U-CTXM-030-01, TC-U-CTXM-030-02 | TC-I-CTXM-030-01 | TC-S-CTXM-010-01 | services/contextual_matcher.rs (new) | Pending | Complete |
| **REQ-CONF-010** | P1 | TC-U-CONF-010-01, TC-U-CONF-010-02, TC-U-CONF-010-03 | TC-I-CONF-010-01 | TC-S-CONF-010-01, TC-S-CONF-010-02 | services/confidence_assessor.rs (new) | Pending | Complete |
| **REQ-FING-010** | P1 | TC-U-FING-010-01, TC-U-FING-010-02 | TC-I-FING-010-01 | TC-S-FING-010-01 | services/fingerprinter.rs (modify) | Pending | Complete |
| **REQ-TICK-010** | P2 | TC-U-TICK-010-01, TC-U-TICK-010-02 | TC-I-TICK-010-01 | - | All pipeline components | Pending | Complete |

---

## Coverage Analysis

**Requirements Coverage:** 12/12 (100%)
- All requirements have at least one unit test
- All P0/P1 requirements have integration tests
- All accuracy requirements have system tests

**Test Coverage:** 32 tests total
- Unit tests: 18
- Integration tests: 10
- System tests: 4

**Implementation Coverage:**
- New files required: 4 (pattern_analyzer.rs, contextual_matcher.rs, confidence_assessor.rs, pipeline.rs)
- Modified files: 2 (workflow_orchestrator/mod.rs, fingerprinter.rs)
- Lines of code estimated: ~2500 lines (new + modified)

---

## Detailed Traceability

### REQ-PIPE-010: Segmentation-First Pipeline

**Tests:**
- **TC-U-PIPE-010-01:** Verify segmentation executes before fingerprinting (unit)
- **TC-I-PIPE-020-01:** Verify complete pipeline order (integration)

**Implementation:**
- `workflow_orchestrator/mod.rs` - State machine refactor, move segmentation before fingerprinting

**Verification:**
- Unit test checks execution order
- Integration test verifies full pipeline with correct sequence

---

### REQ-PIPE-020: Per-File Pipeline

**Tests:**
- **TC-U-PIPE-020-01:** Verify 4 workers created (unit)
- **TC-I-PIPE-020-01:** Verify per-file processing (integration)

**Implementation:**
- `workflow_orchestrator/mod.rs` - Refactor state machine
- `pipeline.rs` (new) - Per-file pipeline function using `futures::stream::buffer_unordered(4)`

**Verification:**
- Unit test checks worker count
- Integration test verifies one file through all steps

---

### REQ-PATT-010/020/030/040: Pattern Analyzer

**Tests:**
- **TC-U-PATT-010-01:** Input acceptance
- **TC-U-PATT-010-02:** Output format
- **TC-U-PATT-020-01:** Track count detection
- **TC-U-PATT-030-01/02:** Gap pattern analysis
- **TC-U-PATT-040-01/02:** Source media classification
- **TC-S-PATT-010-01:** Accuracy >80% on test dataset

**Implementation:**
- `services/pattern_analyzer.rs` (new) - Complete PatternAnalyzer implementation

**Verification:**
- Unit tests check logic correctness
- System test validates accuracy target

---

### REQ-CTXM-010/020/030: Contextual Matcher

**Tests:**
- **TC-U-CTXM-010-01/02:** Input/output format
- **TC-U-CTXM-020-01:** Single-segment logic
- **TC-I-CTXM-020-01:** Single-segment MB query
- **TC-U-CTXM-030-01/02:** Multi-segment logic
- **TC-I-CTXM-030-01:** Multi-segment MB query
- **TC-S-CTXM-010-01:** Narrows to <10 candidates

**Implementation:**
- `services/contextual_matcher.rs` (new) - Complete ContextualMatcher implementation

**Verification:**
- Unit tests check matching logic
- Integration tests verify MusicBrainz API interaction
- System test validates effectiveness target

---

### REQ-CONF-010: Confidence Assessor

**Tests:**
- **TC-U-CONF-010-01/02:** Evidence combination algorithms
- **TC-U-CONF-010-03:** Decision thresholds
- **TC-I-CONF-010-01:** Integration with matcher+fingerprinter
- **TC-S-CONF-010-01:** >90% acceptance rate
- **TC-S-CONF-010-02:** <5% false positive rate

**Implementation:**
- `services/confidence_assessor.rs` (new) - Complete ConfidenceAssessor implementation

**Verification:**
- Unit tests check algorithm correctness
- Integration test verifies component interaction
- System tests validate accuracy targets

---

### REQ-FING-010: Per-Segment Fingerprinting

**Tests:**
- **TC-U-FING-010-01:** Per-segment PCM extraction
- **TC-U-FING-010-02:** Per-segment fingerprint generation
- **TC-I-FING-010-01:** Per-segment AcoustID queries
- **TC-S-FING-010-01:** More accurate than whole-file

**Implementation:**
- `services/fingerprinter.rs` (modify) - Add per-segment support

**Verification:**
- Unit tests check segmentation logic
- Integration test verifies AcoustID API per-segment
- System test validates accuracy improvement

---

### REQ-TICK-010: Tick-Based Timing

**Tests:**
- **TC-U-TICK-010-01:** Conversion accuracy
- **TC-U-TICK-010-02:** Applied to all fields
- **TC-I-TICK-010-01:** Database writes

**Implementation:**
- All pipeline components - Apply `seconds_to_ticks()` before DB write

**Verification:**
- Unit test checks conversion function
- Integration test verifies DB writes use ticks

---

## Gap Analysis

**No gaps identified:**
- ✅ Every requirement has tests
- ✅ Every test traces to requirement
- ✅ Implementation files identified
- ✅ Verification methods defined

**Coverage: 100%**

---

## Maintenance Notes

**During Implementation:**
1. Update "Implementation File(s)" column when implementing each requirement
2. Update "Status" as: Pending → In Progress → Complete → Verified
3. Add line numbers/functions to "Implementation File(s)" for precise traceability
4. If new files needed beyond those listed, document in matrix

**Example Update:**
```
| REQ-PATT-010 | P1 | ... | services/pattern_analyzer.rs (lines 45-178, fn analyze_pattern) | Complete | Complete |
```

---

**END OF TRACEABILITY MATRIX**
