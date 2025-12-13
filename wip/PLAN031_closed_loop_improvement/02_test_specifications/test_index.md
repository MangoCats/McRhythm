# PLAN031 Test Index

## Unit Tests

| Test ID | Requirement | Description | Category |
|---------|-------------|-------------|----------|
| TC-U-GT-001 | SPEC031-GT-010 | Load ground truth from embedded MBID tags | Happy Path |
| TC-U-GT-002 | SPEC031-GT-010 | Load ground truth from JSON file | Happy Path |
| TC-U-GT-003 | SPEC031-GT-010 | Handle missing ground truth file | Error |
| TC-U-GT-004 | SPEC031-GT-020 | Ground truth schema validation | Happy Path |
| TC-U-EV-001 | SPEC031-EV-020 | Classify TRUE_POSITIVE correctly | Happy Path |
| TC-U-EV-002 | SPEC031-EV-020 | Classify FALSE_POSITIVE correctly | Happy Path |
| TC-U-EV-003 | SPEC031-EV-020 | Classify TRUE_NEGATIVE correctly | Happy Path |
| TC-U-EV-004 | SPEC031-EV-020 | Classify FALSE_NEGATIVE correctly | Happy Path |
| TC-U-EV-005 | SPEC031-EV-030 | Calculate accuracy metrics correctly | Happy Path |
| TC-U-EV-006 | SPEC031-EV-030 | Handle zero division in precision/recall | Edge Case |
| TC-U-BM-001 | SPEC031-BM-010 | BatchConfig default values | Happy Path |
| TC-U-BM-002 | SPEC031-BM-010 | Batch stops at failure threshold | Boundary |
| TC-U-BM-003 | SPEC031-BM-010 | Success resets failure counter | Boundary |
| TC-U-FA-001 | SPEC031-FA-010 | Categorize failure as WrongRecording | Happy Path |
| TC-U-FA-002 | SPEC031-FA-010 | Categorize failure as MissedIdentification | Happy Path |
| TC-U-FA-003 | SPEC031-FA-020 | Detect pattern in multiple failures | Happy Path |
| TC-U-DB-001 | SPEC031-DB-010 | ResetMode::TestDataOnly preserves ground truth | Happy Path |
| TC-U-DB-002 | SPEC031-DB-010 | ResetMode::Full recreates schema | Happy Path |
| TC-U-RP-001 | SPEC031-RP-010 | Generate batch report with all fields | Happy Path |
| TC-U-RP-002 | SPEC031-RP-020 | Calculate accuracy trend correctly | Happy Path |

## Integration Tests

| Test ID | Requirement | Description | Category |
|---------|-------------|-------------|----------|
| TC-I-BM-001 | SPEC031-BM-010 | Execute batch against real files | Happy Path |
| TC-I-BM-002 | SPEC031-BM-020 | Phase 1 to Phase 2 progression | State Transition |
| TC-I-API-001 | SPEC031-API-010 | Pipeline hooks record results | Happy Path |
| TC-I-API-002 | SPEC031-API-010 | Hook failure doesn't break import | Error |
| TC-I-FA-001 | SPEC031-FA-030 | Generate improvement suggestions | Happy Path |

## System Tests

| Test ID | Requirement | Description | Category |
|---------|-------------|-------------|----------|
| TC-S-001 | All | End-to-end test run with 10 files | User Flow |
| TC-S-002 | SPEC031-BM-020 | Complete Phase 1 → Phase 2 workflow | User Flow |
| TC-S-003 | SPEC031-RP-010 | CLI produces readable report | User Flow |

## Test Coverage Summary

| Requirement | Unit | Integration | System | Total |
|-------------|------|-------------|--------|-------|
| SPEC031-GT-010 | 3 | 0 | 1 | 4 |
| SPEC031-GT-020 | 1 | 0 | 0 | 1 |
| SPEC031-BM-010 | 3 | 1 | 1 | 5 |
| SPEC031-BM-020 | 0 | 1 | 1 | 2 |
| SPEC031-EV-010 | 0 | 0 | 1 | 1 |
| SPEC031-EV-020 | 4 | 0 | 0 | 4 |
| SPEC031-EV-030 | 2 | 0 | 0 | 2 |
| SPEC031-FA-010 | 2 | 0 | 0 | 2 |
| SPEC031-FA-020 | 1 | 0 | 0 | 1 |
| SPEC031-FA-030 | 0 | 1 | 0 | 1 |
| SPEC031-DB-010 | 2 | 0 | 0 | 2 |
| SPEC031-RP-010 | 1 | 0 | 1 | 2 |
| SPEC031-RP-020 | 1 | 0 | 0 | 1 |
| SPEC031-API-010 | 0 | 2 | 0 | 2 |
| **Total** | **20** | **5** | **3** | **28** |
