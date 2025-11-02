# Test Index: SPEC017 Compliance Remediation

**Plan:** PLAN017
**Total Test Cases:** 7 (2 unit, 2 integration, 3 acceptance)
**Requirements Coverage:** 7/7 (100%)

---

## Test Summary by Type

| Type | Count | Pass Criteria |
|------|-------|---------------|
| Unit Tests | 2 | Automated, deterministic |
| Integration Tests | 2 | Automated, database-dependent |
| Acceptance Tests | 3 | 2 automated, 1 manual |

---

## Test Cases

| ID | Type | Description | Requirements | Priority | Automation |
|----|------|-------------|--------------|----------|------------|
| TC-U-001 | Unit | wkmp-dr tick-to-seconds conversion accuracy | REQ-F-001 | HIGH | Yes |
| TC-U-002 | Unit | File duration roundtrip (seconds → ticks → seconds) | REQ-F-003 | MEDIUM | Yes |
| TC-I-001 | Integration | File import stores duration as ticks | REQ-F-003 | MEDIUM | Yes |
| TC-I-002 | Integration | wkmp-dr renders dual time display | REQ-F-001 | HIGH | Yes |
| TC-A-001 | Acceptance | Developer UI compliance (SRC-LAYER-011) | REQ-F-001, REQ-NF-001 | HIGH | Partial |
| TC-A-002 | Acceptance | File duration storage consistency | REQ-F-003, REQ-NF-003 | MEDIUM | Yes |
| TC-A-003 | Acceptance | API documentation completeness | REQ-F-002, REQ-NF-002 | MEDIUM | Manual |

---

## Requirements Coverage Matrix

| Requirement | Test Cases | Coverage |
|-------------|------------|----------|
| REQ-F-001 | TC-U-001, TC-I-002, TC-A-001 | ✅ 100% |
| REQ-F-002 | TC-A-003 | ✅ 100% |
| REQ-F-003 | TC-U-002, TC-I-001, TC-A-002 | ✅ 100% |
| REQ-F-004 | (Manual code review) | ✅ 100% |
| REQ-NF-001 | TC-A-001 | ✅ 100% |
| REQ-NF-002 | TC-A-003 | ✅ 100% |
| REQ-NF-003 | TC-A-002 | ✅ 100% |

**Note:** REQ-F-004 (variable naming clarity) verified via manual code review during implementation, no executable test.

---

## Test Execution Order

### Phase 1: Unit Tests (Independent)
1. TC-U-001 - wkmp-dr conversion accuracy
2. TC-U-002 - File duration roundtrip

### Phase 2: Integration Tests (Database Required)
3. TC-I-001 - File import with ticks
4. TC-I-002 - wkmp-dr display rendering

### Phase 3: Acceptance Tests (End-to-End)
5. TC-A-001 - Developer UI compliance
6. TC-A-002 - File duration consistency
7. TC-A-003 - API documentation review (manual)

---

## Test Data Requirements

| Test | Data Needed | Source |
|------|-------------|--------|
| TC-U-001 | Known tick/second pairs | SPEC017 conversion table |
| TC-U-002 | Sample duration values | Test fixtures (0.5s, 5.0s, 180.5s) |
| TC-I-001 | Sample audio file | Test assets (5-second WAV) |
| TC-I-002 | Database with passages | Existing test database or import |
| TC-A-001 | wkmp-dr running instance | Dev environment |
| TC-A-002 | Fresh database | Empty wkmp.db |
| TC-A-003 | Source code | wkmp-ap, wkmp-ai API files |

---

## Success Criteria

**All tests pass = Ready for deployment**

- ✅ TC-U-001, TC-U-002 pass (conversion accuracy verified)
- ✅ TC-I-001, TC-I-002 pass (database integration verified)
- ✅ TC-A-001 pass (developer UI displays both ticks and seconds)
- ✅ TC-A-002 pass (file duration stored as ticks)
- ✅ TC-A-003 pass (all API timing fields documented)

**Partial pass = Requires remediation**

---

## Modular Test Files

Each test case has detailed specification:

- [TC-U-001: wkmp-dr Tick-to-Seconds Conversion](tc_u_001.md)
- [TC-U-002: File Duration Roundtrip](tc_u_002.md)
- [TC-I-001: File Import with Tick Duration](tc_i_001.md)
- [TC-I-002: wkmp-dr Display Rendering](tc_i_002.md)
- [TC-A-001: Developer UI Compliance](tc_a_001.md)
- [TC-A-002: File Duration Storage Consistency](tc_a_002.md)
- [TC-A-003: API Documentation Completeness](tc_a_003.md)
