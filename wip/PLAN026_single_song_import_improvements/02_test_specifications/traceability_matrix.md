# Traceability Matrix: Single Song Import Improvements

**Plan:** PLAN026
**Coverage:** 100% (20/20 requirements have tests)

---

## Requirements → Tests → Implementation

| Requirement | Priority | Unit Tests | Integration Tests | System Tests | Implementation File(s) | Status |
|-------------|----------|------------|-------------------|--------------|------------------------|--------|
| SSI-MB-010 | P0 | TC-U-MB-010-01, TC-U-MB-010-02 | - | TC-S-E2E-01 | services/recording_matcher.rs | Pending |
| SSI-MB-020 | P0 | TC-U-MB-020-01, TC-U-MB-020-02 | - | - | services/recording_matcher.rs | Pending |
| SSI-MB-030 | P1 | TC-U-MB-030-01 | - | - | services/recording_matcher.rs | Pending |
| SSI-VAL-010 | P0 | TC-U-VAL-010-01, TC-U-VAL-010-02 | - | - | utils/string_similarity.rs | Pending |
| SSI-VAL-020 | P0 | TC-U-VAL-020-01, TC-U-VAL-020-02 | - | - | utils/string_similarity.rs | Pending |
| SSI-VAL-030 | P1 | TC-U-VAL-030-01, TC-U-VAL-030-02 | - | - | services/recording_matcher.rs | Pending |
| SSI-FUS-010 | P1 | - | TC-I-FUS-010-01 | TC-S-E2E-01 | services/content_type_classifier.rs | Pending |
| SSI-FUS-020 | P1 | - | TC-I-FUS-020-01, TC-I-FUS-020-02 | TC-S-E2E-02 | services/content_type_classifier.rs | Pending |
| SSI-FUS-030 | P1 | TC-U-FUS-030-01 | - | - | fusion/identity_resolver.rs (existing) | Pending |
| SSI-INT-010 | P0 | - | TC-I-INT-010-01 | - | services/content_type_classifier.rs | Pending |
| SSI-INT-020 | P0 | - | TC-I-INT-020-01 | - | services/recording_matcher.rs | Pending |
| SSI-INT-030 | P1 | - | TC-I-INT-030-01 | - | services/recording_matcher.rs, db/recording_cache.rs | Pending |
| SSI-QUA-010 | P0 | (All unit tests) | - | - | - | Pending |
| SSI-QUA-020 | P1 | - | (All integration tests) | - | - | Pending |
| SSI-QUA-030 | P1 | - | - | - | Coverage report | Pending |
| SSI-CTX-010 | P2 | - | - | - | (Deferred) | Out of Scope |
| SSI-CTX-020 | P2 | - | - | - | (Deferred) | Out of Scope |
| SSI-SEG-010 | P2 | - | - | - | (Deferred) | Out of Scope |
| SSI-SEG-020 | P2 | - | - | - | (Deferred) | Out of Scope |

---

## Implementation Files Summary

| File | New/Modified | Requirements Covered | Test Count |
|------|--------------|---------------------|------------|
| `services/recording_matcher.rs` | New | SSI-MB-010, SSI-MB-020, SSI-MB-030, SSI-VAL-030, SSI-INT-020, SSI-INT-030 | 7 |
| `utils/string_similarity.rs` | New | SSI-VAL-010, SSI-VAL-020 | 4 |
| `services/content_type_classifier.rs` | Modified | SSI-FUS-010, SSI-FUS-020, SSI-INT-010 | 4 |
| `db/recording_cache.rs` | New | SSI-INT-030 | 1 |
| `fusion/identity_resolver.rs` | Existing | SSI-FUS-030 | 1 |

---

## Test Coverage Summary

| Category | Requirements | Tests | Coverage |
|----------|--------------|-------|----------|
| In Scope (P0+P1) | 15 | 20 | 100% |
| Out of Scope (P2) | 4 | 0 | N/A |
| **Total In Scope** | **15** | **20** | **100%** |

---

## Verification Checklist

- [ ] All P0 requirements have unit tests
- [ ] All P1 requirements have integration tests
- [ ] E2E tests cover critical paths
- [ ] Coverage target (80%) achieved
- [ ] All tests pass before merge
