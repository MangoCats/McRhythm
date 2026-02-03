# Traceability Matrix

**Plan:** PLAN-EMBID-001 (Embedded MusicBrainz ID Matching)
**Coverage:** 100% of requirements have tests

---

## Requirements → Tests → Implementation

| Req ID | Requirement | Unit Tests | Integration Tests | System Tests | Implementation | Status | Confidence |
|--------|-------------|------------|-------------------|--------------|----------------|--------|------------|
| FR-01 | Extract MB Recording ID | TC-U-001 | - | - | id3_extractor.rs:172-195 | Exists | HIGH |
| FR-02 | Extract ISRC | TC-U-002 | - | - | id3_extractor.rs (new) | Increment 1 | HIGH |
| FR-03 | Validate MB ID format | TC-U-003 | - | - | id3_extractor.rs:257-279 | Exists | HIGH |
| FR-04 | Assign confidence tier | TC-U-004, TC-U-005, TC-U-006 | - | - | confidence_tier.rs (new) | Increment 2 | HIGH |
| FR-05 | Skip AcoustID when embedded | - | TC-I-001, TC-I-002 | - | identity_resolver.rs | Increment 3 | HIGH |
| FR-06 | Fall back to AcoustID | - | TC-I-003 | - | identity_resolver.rs | Exists | HIGH |
| FR-07 | Optional MB validation | - | TC-I-004 | - | identity_resolver.rs | P2 (defer) | MEDIUM |
| FR-08 | Log confidence tier | TC-U-007, TC-U-008 | - | - | identity_resolver.rs | Increment 4 | HIGH |
| NFR-01 | < 50ms per file | - | - | TC-S-002 | N/A (measure) | Increment 5 | HIGH |
| NFR-02 | >= 98% coverage | - | - | TC-S-003 | N/A (measure) | Increment 5 | HIGH |
| NFR-03 | < 0.1% false positives | - | - | TC-S-003 | N/A (measure) | Increment 5 | MEDIUM |
| NFR-04 | Offline capability | - | - | TC-S-004 | Design | Increment 5 | HIGH |

---

## Tests Index

### Unit Tests (8 tests)

| Test ID | Description | Requirement | Increment |
|---------|-------------|-------------|-----------|
| TC-U-001 | MB Recording ID extraction | FR-01 | - (exists) |
| TC-U-002 | ISRC extraction and normalization | FR-02 | 1 |
| TC-U-003 | MB ID UUID format validation | FR-03 | - (exists) |
| TC-U-004 | Tier 1A assignment (embedded + ISRC) | FR-04 | 2 |
| TC-U-005 | Tier 1B assignment (embedded only) | FR-04 | 2 |
| TC-U-006 | Tier 2A/2B/3/4 assignment | FR-04 | 2 |
| TC-U-007 | ConfidenceTier Display trait | FR-08 | 4 |
| TC-U-008 | ConfidenceTier serialization | FR-08 | 4 |

### Integration Tests (4 tests)

| Test ID | Description | Requirement | Increment |
|---------|-------------|-------------|-----------|
| TC-I-001 | Stage 0 returns when embedded MB ID present | FR-05 | 3 |
| TC-I-002 | AcoustID skipped when Stage 0 succeeds | FR-05 | 3 |
| TC-I-003 | Falls back to AcoustID when no embedded | FR-06 | 3 |
| TC-I-004 | Optional MB ID validation | FR-07 | P2 |

### System Tests (4 tests)

| Test ID | Description | Requirement | Increment |
|---------|-------------|-------------|-----------|
| TC-S-001 | Full pipeline integration | All | 5 |
| TC-S-002 | Performance benchmark | NFR-01 | 5 |
| TC-S-003 | Coverage validation | NFR-02, NFR-03 | 5 |
| TC-S-004 | Offline capability | NFR-04 | 5 |

---

## Coverage Summary

| Category | Total | Covered | Coverage |
|----------|-------|---------|----------|
| Functional Requirements | 8 | 8 | 100% |
| Non-Functional Requirements | 4 | 4 | 100% |
| **All Requirements** | **12** | **12** | **100%** |

---

## Validation Report (Phase 3.5)

**Completeness:** PASS - All 12 requirements have tests
**Redundancy:** PASS - No over-tested requirements
**Quality:** PASS - All requirements have appropriate test categories
**Implementation:** PASS - All implementation files identified

**Status:** Ready to proceed
