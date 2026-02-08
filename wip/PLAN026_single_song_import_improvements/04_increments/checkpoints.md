# Implementation Checkpoints

**Plan:** PLAN026

---

## Checkpoint 1: Foundation Complete (After Increment 2)

**Trigger:** Increments 1-2 complete

**Verify:**
- [ ] String similarity functions work correctly
- [ ] RecordingMatcher builds valid queries
- [ ] Unit tests pass: `cargo test string_similarity recording_matcher`
- [ ] No clippy warnings

**Decision Point:**
- Pass → Continue to Increment 3
- Fail → Fix issues before proceeding

---

## Checkpoint 2: Core Integration Complete (After Increment 4)

**Trigger:** Increments 1-4 complete

**Verify:**
- [ ] ContentTypeClassifier uses RecordingMatcher as fallback
- [ ] Cache prevents redundant queries
- [ ] Existing album tests still pass
- [ ] All unit and integration tests pass

**Decision Point:**
- Pass → Continue to Increment 5
- Fail → Investigate regression

---

## Checkpoint 3: Full Implementation Complete (After Increment 6)

**Trigger:** All increments complete

**Verify:**
- [ ] All 20 tests pass
- [ ] Coverage ≥80%
- [ ] E2E tests confirm expected behavior
- [ ] No regression in existing functionality
- [ ] Rate limits verified

**Decision Point:**
- Pass → Mark plan complete, run Phase 9
- Fail → Document issues, plan remediation

---

## Test Execution Commands

```bash
# After each increment
cargo test --lib

# Checkpoint 1
cargo test string_similarity recording_matcher

# Checkpoint 2
cargo test content_type_classifier recording_cache

# Checkpoint 3
cargo test --test single_song_integration_tests
cargo tarpaulin --out Html --output-dir coverage/
```

---

## Rollback Plan

If critical issues found at any checkpoint:

1. Revert to previous working state
2. Document issue in 01_specification_issues.md
3. Analyze root cause
4. Update increment with fix
5. Re-run verification

---

## Coverage Tracking

| Increment | Files | Coverage Target |
|-----------|-------|-----------------|
| 1 | string_similarity.rs | 95% |
| 2 | recording_matcher.rs | 90% |
| 3 | recording_cache.rs | 85% |
| 4 | content_type_classifier.rs (modified) | 80% |
| 5 | content_type_classifier.rs (fusion) | 80% |
| 6 | Integration tests | N/A |
| **Overall** | | **≥80%** |
