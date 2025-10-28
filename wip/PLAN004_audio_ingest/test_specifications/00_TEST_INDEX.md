# Test Specifications Index - PLAN004

**Plan:** PLAN004 - wkmp-ai Audio Ingest Implementation
**Created:** 2025-10-27

---

## Test Organization

Tests organized by functional area, each file <300 lines per CLAUDE.md standards.

| File | Requirements Covered | Test Count | Priority |
|------|---------------------|------------|----------|
| **01_http_server_tests.md** | AIA-OV-010, AIA-MS-010 | 8 | P0 |
| **02_workflow_tests.md** | AIA-WF-010, AIA-WF-020, AIA-ASYNC-010, AIA-ASYNC-020 | 12 | P0, P1 |
| **03_integration_tests.md** | AIA-INT-010, AIA-INT-020, AIA-INT-030 | 9 | P0 |
| **04_events_tests.md** | AIA-SSE-010, AIA-POLL-010 | 10 | P0, P1 |
| **05_error_handling_tests.md** | AIA-ERR-010, AIA-ERR-020 | 11 | P0 |
| **06_performance_tests.md** | AIA-PERF-010, AIA-PERF-020 | 6 | P1 |
| **07_security_tests.md** | AIA-SEC-010, AIA-SEC-020 | 7 | P0 |
| **08_database_tests.md** | AIA-DB-010 | 8 | P0 |
| **09_component_tests.md** | AIA-COMP-010 | 9 | P0 |
| **10_testing_framework_tests.md** | AIA-TEST-010, AIA-TEST-020, AIA-TEST-030 | 15 | P0 |

**Total:** 95 acceptance tests covering 23 requirements

---

## Test Execution Strategy

### Unit Tests
- Location: `wkmp-ai/tests/unit/`
- Run: `cargo test --lib -p wkmp-ai`
- Coverage: Component logic, state machine, validation

### Integration Tests
- Location: `wkmp-ai/tests/integration/`
- Run: `cargo test --test '*' -p wkmp-ai`
- Coverage: API endpoints, database, mock external APIs

### End-to-End Tests
- Location: `wkmp-ai/tests/e2e/`
- Run: `cargo test --test e2e -p wkmp-ai`
- Coverage: Complete import workflow with sample library

---

## Coverage Targets

- **P0 Requirements:** 100% test coverage (17/17)
- **P1 Requirements:** 100% test coverage (5/5)
- **P3 Requirements:** 0% test coverage (future enhancement)

---

## Test Data Requirements

### Sample Audio Files
- **Formats:** MP3, FLAC, OGG, M4A, WAV (5 files minimum)
- **Durations:** 30s, 3min, 10min, 45min (varied)
- **Quality:** 128kbps, 320kbps, lossless (varied)
- **Content:** Silence, fade-in, fade-out, constant (varied)

### Mock API Responses
- **MusicBrainz:** Recording, artist, work, album JSON
- **AcoustID:** Fingerprint match results
- **AcousticBrainz:** Musical flavor vectors

### Database Fixtures
- **Empty database:** Fresh schema for clean tests
- **Pre-populated:** 100 passages for performance tests

---

## Acceptance Criteria

All tests MUST pass before implementation considered complete:
- ✅ 95/95 acceptance tests pass
- ✅ Zero test failures or panics
- ✅ Performance tests within ±20% of targets
- ✅ E2E test completes successfully

---

End of test index
