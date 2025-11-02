# Requirements Index: SPEC017 Compliance Remediation

**Source:** [SPEC_spec017_compliance_remediation.md](../SPEC_spec017_compliance_remediation.md)
**Total Requirements:** 7 (4 functional, 3 non-functional)

---

## Functional Requirements

| Req ID | Type | Brief Description | Source Line | Priority |
|--------|------|-------------------|-------------|----------|
| REQ-F-001 | Functional | wkmp-dr dual time display (ticks + seconds) | 38 | HIGH |
| REQ-F-002 | Functional | API timing unit documentation | 64 | MEDIUM |
| REQ-F-003 | Functional | File duration migration to i64 ticks | 91 | MEDIUM |
| REQ-F-004 | Functional | Variable naming clarity (unit suffixes/comments) | 112 | LOW |

## Non-Functional Requirements

| Req ID | Type | Brief Description | Source Line | Priority |
|--------|------|-------------------|-------------|----------|
| REQ-NF-001 | Testing | Automated test coverage for all changes | 135 | REQUIRED |
| REQ-NF-002 | Documentation | Update SPEC017, IMPL001, code comments | 148 | REQUIRED |
| REQ-NF-003 | Compatibility | Document breaking changes, migration path | 161 | REQUIRED |

---

## Requirement Details

### REQ-F-001: wkmp-dr Dual Time Display

**Priority:** HIGH
**Rationale:** Violates [SRC-LAYER-011](../../docs/SPEC017-sample_rate_conversion.md)

**Acceptance Criteria:**
- Format: `{ticks} ({seconds}s)` e.g., `141120000 (5.000000s)`
- Applies to 6 timing columns: start_time_ticks, end_time_ticks, fade_in_start_ticks, fade_out_start_ticks, lead_in_start_ticks, lead_out_start_ticks
- Decimal precision: 6 places
- NULL values: display as `null` (no conversion)

---

### REQ-F-002: API Timing Unit Documentation

**Priority:** MEDIUM
**Rationale:** Pragmatic API deviation from SPEC017 SRC-API-010 (accept ms/seconds for ergonomics)

**Acceptance Criteria:**
- Every API timing field has doc comment with unit
- Function parameters use unit suffixes (`_ms`, `_ticks`, `_seconds`)
- SPEC017 updated with "API Layer Pragmatic Deviation" section
- Error messages reference correct units

---

### REQ-F-003: File Duration Migration to Ticks

**Priority:** MEDIUM (Breaking Change)
**Rationale:** Consistency with passage timing representation

**Acceptance Criteria:**
- `AudioFile.duration: Option<f64>` → `duration_ticks: Option<i64>`
- Database schema: `duration REAL` → `duration_ticks INTEGER`
- Import converts via `seconds_to_ticks()`
- Breaking change documented

---

### REQ-F-004: Variable Naming Clarity

**Priority:** LOW
**Rationale:** Code maintainability

**Acceptance Criteria:**
- Variables use unit suffixes OR inline comments
- Applies to: wkmp-ap/src/playback/pipeline/timing.rs, wkmp-ai/src/services/silence_detector.rs

---

### REQ-NF-001: Test Coverage

**Acceptance Criteria:**
- wkmp-dr: UI rendering test verifies dual display
- wkmp-ai: Database roundtrip test verifies tick storage
- wkmp-ai: File duration conversion test
- Integration test: end-to-end file import

---

### REQ-NF-002: Documentation Updates

**Acceptance Criteria:**
- SPEC017 updated (API deviation section)
- IMPL001 updated (duration_ticks schema)
- Code documentation follows standards
- Migration notes written

---

### REQ-NF-003: Backward Compatibility

**Acceptance Criteria:**
- Database rebuild instructions provided
- Users informed (existing databases incompatible)
- No automated migration (acceptable)
