# Test Index: Single Song Import Improvements

**Plan:** PLAN026
**Total Tests:** 20

---

## Test Summary

| Category | Count | Coverage |
|----------|-------|----------|
| Unit Tests (TC-U-*) | 12 | Core logic |
| Integration Tests (TC-I-*) | 6 | Multi-component |
| System Tests (TC-S-*) | 2 | End-to-end |
| **Total** | **20** | |

---

## Unit Tests

| Test ID | Requirement | Description |
|---------|-------------|-------------|
| TC-U-MB-010-01 | SSI-MB-010 | Recording matcher triggers when AcoustID fails |
| TC-U-MB-010-02 | SSI-MB-010 | Recording matcher triggers when AcoustID confidence < 0.80 |
| TC-U-MB-020-01 | SSI-MB-020 | Query includes artist, title, and duration range |
| TC-U-MB-020-02 | SSI-MB-020 | Duration tolerance is ±10 seconds (±10000ms) |
| TC-U-MB-030-01 | SSI-MB-030 | Candidates sorted by descending similarity score |
| TC-U-VAL-010-01 | SSI-VAL-010 | Jaro-Winkler returns value 0.0-1.0 |
| TC-U-VAL-010-02 | SSI-VAL-010 | Similar strings score > 0.85 |
| TC-U-VAL-020-01 | SSI-VAL-020 | Normalization removes "The " prefix |
| TC-U-VAL-020-02 | SSI-VAL-020 | Normalization folds Unicode to ASCII |
| TC-U-VAL-030-01 | SSI-VAL-030 | Title threshold 0.85 applied correctly |
| TC-U-VAL-030-02 | SSI-VAL-030 | Artist threshold 0.80 applied correctly |
| TC-U-FUS-030-01 | SSI-FUS-030 | Bayesian posterior formula correct |

---

## Integration Tests

| Test ID | Requirement | Description |
|---------|-------------|-------------|
| TC-I-FUS-010-01 | SSI-FUS-010 | IdentityResolver integrates with single-song path |
| TC-I-FUS-020-01 | SSI-FUS-020 | Three sources combined correctly |
| TC-I-FUS-020-02 | SSI-FUS-020 | Agreeing sources boost confidence |
| TC-I-INT-010-01 | SSI-INT-010 | Recording matcher integrates with ContentTypeClassifier |
| TC-I-INT-020-01 | SSI-INT-020 | Rate limits respected (1 req/sec MB) |
| TC-I-INT-030-01 | SSI-INT-030 | Cache prevents redundant queries |

---

## System Tests

| Test ID | Requirement | Description |
|---------|-------------|-------------|
| TC-S-E2E-01 | SSI-MB-010, SSI-FUS-010 | Single song classified when AcoustID fails but MB succeeds |
| TC-S-E2E-02 | SSI-MB-010, SSI-FUS-020 | Confidence boosted when AcoustID and MB agree |

---

## Coverage by Requirement

| Requirement | Tests |
|-------------|-------|
| SSI-MB-010 | TC-U-MB-010-01, TC-U-MB-010-02, TC-S-E2E-01 |
| SSI-MB-020 | TC-U-MB-020-01, TC-U-MB-020-02 |
| SSI-MB-030 | TC-U-MB-030-01 |
| SSI-VAL-010 | TC-U-VAL-010-01, TC-U-VAL-010-02 |
| SSI-VAL-020 | TC-U-VAL-020-01, TC-U-VAL-020-02 |
| SSI-VAL-030 | TC-U-VAL-030-01, TC-U-VAL-030-02 |
| SSI-FUS-010 | TC-I-FUS-010-01, TC-S-E2E-01 |
| SSI-FUS-020 | TC-I-FUS-020-01, TC-I-FUS-020-02, TC-S-E2E-02 |
| SSI-FUS-030 | TC-U-FUS-030-01 |
| SSI-INT-010 | TC-I-INT-010-01 |
| SSI-INT-020 | TC-I-INT-020-01 |
| SSI-INT-030 | TC-I-INT-030-01 |
| SSI-QUA-010 | All TC-U-* tests |
| SSI-QUA-020 | All TC-I-* tests |
| SSI-QUA-030 | Coverage report (80% target) |
