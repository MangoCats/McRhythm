# PLAN031 Traceability Matrix

| Requirement | Unit Tests | Integration Tests | System Tests | Implementation File(s) | Status | Coverage | Confidence |
|-------------|------------|-------------------|--------------|------------------------|--------|----------|------------|
| SPEC031-GT-010 | TC-U-GT-001, TC-U-GT-002, TC-U-GT-003 | - | TC-S-001 | `testing/ground_truth.rs` | Pending | Complete | HIGH |
| SPEC031-GT-020 | TC-U-GT-004 | - | - | `testing/ground_truth.rs`, migration | Pending | Complete | HIGH |
| SPEC031-BM-010 | TC-U-BM-001, TC-U-BM-002, TC-U-BM-003 | TC-I-BM-001 | TC-S-001 | `testing/batch_orchestrator.rs` | Pending | Complete | HIGH |
| SPEC031-BM-020 | - | TC-I-BM-002 | TC-S-002 | `testing/batch_orchestrator.rs` | Pending | Adequate | MEDIUM |
| SPEC031-EV-010 | - | - | TC-S-001 | `testing/evaluation.rs` | Pending | Minimal | MEDIUM |
| SPEC031-EV-020 | TC-U-EV-001, TC-U-EV-002, TC-U-EV-003, TC-U-EV-004 | - | - | `testing/evaluation.rs` | Pending | Complete | HIGH |
| SPEC031-EV-030 | TC-U-EV-005, TC-U-EV-006 | - | - | `testing/evaluation.rs` | Pending | Complete | HIGH |
| SPEC031-FA-010 | TC-U-FA-001, TC-U-FA-002 | - | - | `testing/failure_analyzer.rs` | Pending | Adequate | MEDIUM |
| SPEC031-FA-020 | TC-U-FA-003 | - | - | `testing/failure_analyzer.rs` | Pending | Minimal | MEDIUM |
| SPEC031-FA-030 | - | TC-I-FA-001 | - | `testing/failure_analyzer.rs` | Pending | Minimal | LOW |
| SPEC031-DB-010 | TC-U-DB-001, TC-U-DB-002 | - | - | `testing/reset.rs` | Pending | Complete | HIGH |
| SPEC031-RP-010 | TC-U-RP-001 | - | TC-S-003 | `testing/reporting.rs` | Pending | Complete | HIGH |
| SPEC031-RP-020 | TC-U-RP-002 | - | - | `testing/reporting.rs` | Pending | Adequate | MEDIUM |
| SPEC031-API-010 | - | TC-I-API-001, TC-I-API-002 | - | Services with hooks | Pending | Complete | HIGH |

## Coverage Quality Summary

| Coverage Level | Requirements | Percentage |
|----------------|--------------|------------|
| Complete (all categories) | 8 | 57% |
| Adequate (happy + boundaries) | 4 | 29% |
| Minimal (happy path only) | 2 | 14% |

## Notes

- SPEC031-EV-010 relies on system test for integration; individual unit tests covered by EV-020/030
- SPEC031-FA-030 (Improvement suggestions) is P2 priority; minimal testing acceptable
- All P0 requirements have Complete or Adequate coverage
