# PLAN013 Phase 4: Implementation Plan

**Specification:** docs/IMPL012-acoustid_client.md (v1.1)
**Date:** 2025-10-30
**Status:** Phase 4 Complete - **READY FOR IMPLEMENTATION**

---

## Executive Summary

**Implementation Strategy:** 5 increments, test-first approach, ~630 LOC estimated

**Timeline:** ~4-6 hours total (including testing)

**Risk Mitigation:** Each increment delivers working intermediate state with tests passing

---

## Implementation Increments

### Increment 1: Database Schema & Caching Foundation

**Goal:** Create database table and implement caching layer without fingerprinting

**Duration:** 30-45 minutes

**Requirements Addressed:**
- REQ-CA-010 (Cache lookup)
- REQ-CA-020 (Cache storage)
- REQ-CA-030 (Fingerprint hashing)

**Deliverables:**

**1.1 Database Schema (wkmp-common/src/db/init.rs)**
- Add `create_acoustid_cache_table()` function
- Add table creation call to `init_database()`
- Add index creation for `cached_at` column

**1.2 Caching Functions (wkmp-ai/src/services/acoustid_client.rs)**
- Add `db: SqlitePool` field to `AcoustIDClient`
- Implement `hash_fingerprint()` using sha2 crate
- Implement `get_cached_mbid()` - SELECT query
- Implement `cache_mbid()` - UPSERT query
- Update `lookup()` to check cache before API call

**1.3 Tests**
- AT-CA-030-01: test_fingerprint_hash() determinism
- AT-CA-030-02: test_hash_uniqueness()
- AT-CA-010-01: test_cache_hit()
- AT-CA-010-02: test_cache_miss()
- AT-CA-020-01: test_cache_insert()
- AT-CA-020-02: test_cache_upsert()

**Acceptance Criteria:**
- [ ] acoustid_cache table created in database
- [ ] All 6 caching tests pass
- [ ] `lookup()` checks cache before making API call
- [ ] No compiler warnings

**Estimated LOC:** ~130 lines
- Database schema: 30 lines
- Caching functions: 80 lines
- Tests: 120 lines

---

### Increment 2: Audio Decoding (Symphonia Integration)

**Goal:** Implement audio file decoding to PCM samples

**Duration:** 45-60 minutes

**Requirements Addressed:**
- REQ-FP-010 (Audio decoding)
- REQ-FP-040 (Error handling - partial)

**Deliverables:**

**2.1 Audio Decoding (wkmp-ai/src/services/fingerprinter.rs)**
- Replace placeholder `decode_audio()` with Symphonia integration
- Implement `convert_to_mono_f32()` helper for channel mixing
- Decode up to 120 seconds (Chromaprint recommendation)
- Return AudioData struct with samples and sample_rate

**2.2 Tests**
- AT-FP-010-01: test_decode_audio_valid_file() (with mock)
- AT-FP-010-02: test_decode_audio_unsupported() (error handling)
- AT-FP-010-03: test_decode_audio_no_track() (error handling)
- AT-FP-040-01: test_io_error_propagation()

**Acceptance Criteria:**
- [ ] `decode_audio()` successfully decodes test audio (mocked)
- [ ] All 4 tests pass
- [ ] Graceful error handling for unsupported codecs
- [ ] Graceful error handling for missing audio tracks
- [ ] No compiler warnings

**Estimated LOC:** ~150 lines
- decode_audio(): 60 lines
- convert_to_mono_f32(): 30 lines
- Tests: 100 lines

---

### Increment 3: Audio Resampling (Rubato Integration)

**Goal:** Implement audio resampling to 44.1kHz mono

**Duration:** 30-45 minutes

**Requirements Addressed:**
- REQ-FP-020 (Audio resampling)

**Deliverables:**

**3.1 Audio Resampling (wkmp-ai/src/services/fingerprinter.rs)**
- Replace placeholder `resample_to_44100()` with Rubato integration
- Skip resampling if already 44.1kHz (optimization)
- Use Rubato SincFixedIn with spec-defined parameters
- Convert to mono if multi-channel

**3.2 Tests**
- AT-FP-020-01: test_resample_48k_to_44k()
- AT-FP-020-02: test_resample_already_44k() (optimization check)
- AT-FP-020-03: test_convert_stereo_to_mono()

**Acceptance Criteria:**
- [ ] `resample_to_44100()` correctly resamples audio
- [ ] All 3 tests pass
- [ ] Optimization works (skip resampling if already 44.1kHz)
- [ ] No compiler warnings

**Estimated LOC:** ~80 lines
- resample_to_44100(): 50 lines
- Tests: 60 lines

---

### Increment 4: Chromaprint Fingerprinting (FFI Integration)

**Goal:** Implement Chromaprint fingerprint generation using unsafe FFI

**Duration:** 60-90 minutes (most complex increment - unsafe code)

**Requirements Addressed:**
- REQ-FP-030 (Chromaprint fingerprinting)

**Deliverables:**

**4.1 Chromaprint Integration (wkmp-ai/src/services/fingerprinter.rs)**
- Replace placeholder `generate_chromaprint()` with chromaprint-sys-next FFI
- Wrap all FFI calls in unsafe blocks
- Implement proper resource cleanup (chromaprint_free, chromaprint_dealloc)
- Convert f32 samples to i16 for Chromaprint
- Return Base64-encoded fingerprint string

**4.2 Tests**
- AT-FP-030-01: test_generate_fingerprint_valid() (manual integration test)
- AT-FP-030-02: test_generate_fingerprint_empty() (unit test with mock)

**Acceptance Criteria:**
- [ ] `generate_chromaprint()` generates valid fingerprints
- [ ] Manual integration test passes with real audio file
- [ ] No memory leaks (chromaprint_free always called)
- [ ] Error handling for all FFI call failures
- [ ] Fingerprint starts with "AQAD" (Base64 Chromaprint format)
- [ ] No compiler warnings

**Estimated LOC:** ~120 lines
- generate_chromaprint(): 80 lines (many error checks)
- Tests: 60 lines

**Special Considerations:**
- **High Risk:** Unsafe FFI code requires careful testing
- **Verification:** Run with valgrind or similar memory checker
- **Documentation:** Add safety comments for all unsafe blocks

---

### Increment 5: End-to-End Integration & Polish

**Goal:** Connect all components, add missing tests, verify complete workflow

**Duration:** 45-60 minutes

**Requirements Addressed:**
- REQ-AC-010, REQ-AC-020, REQ-AC-040 (API client tests)
- REQ-PERF-030 (Rate limiting verification)
- REQ-TEST-010, REQ-TEST-020 (Test completeness)

**Deliverables:**

**5.1 API Client Tests (wkmp-ai/src/services/acoustid_client.rs)**
- AT-AC-010-01: test_lookup_successful() (mock HTTP)
- AT-AC-010-02: test_lookup_network_timeout() (mock timeout)
- AT-AC-020-01: test_parse_valid_json()
- AT-AC-020-02: test_parse_invalid_json()
- AT-AC-040-01: test_invalid_api_key() (mock 401 response)

**5.2 End-to-End Workflow Test**
- Create integration test combining all components:
  - Decode audio → Resample → Fingerprint → Lookup MBID (cached)
  - Mark as `#[ignore]` (manual test with real audio)

**5.3 Documentation**
- Update inline code comments
- Document unsafe blocks with safety rationale
- Add module-level documentation

**Acceptance Criteria:**
- [ ] All 24 acceptance tests pass (except manual integration tests)
- [ ] Manual integration test passes with real audio file
- [ ] End-to-end workflow test passes
- [ ] Code coverage ≥ 80%
- [ ] No compiler warnings
- [ ] All unsafe blocks documented

**Estimated LOC:** ~150 lines
- API client tests: 100 lines
- End-to-end test: 30 lines
- Documentation: 50 lines

---

## Implementation Order Summary

| Inc | Component | Requirements | Tests | LOC | Duration |
|---|---|---|---|---|---|
| 1 | Database + Caching | REQ-CA-* (3) | 6 | 130 | 30-45 min |
| 2 | Audio Decoding | REQ-FP-010, 040 | 4 | 150 | 45-60 min |
| 3 | Audio Resampling | REQ-FP-020 | 3 | 80 | 30-45 min |
| 4 | Chromaprint FFI | REQ-FP-030 | 2 | 120 | 60-90 min |
| 5 | Integration + Tests | REQ-AC-*, TEST-* | 9 | 150 | 45-60 min |
| **Total** | | **18** | **24** | **630** | **4-6 hours** |

---

## Test-First Workflow (Per Increment)

**Red-Green-Refactor Cycle:**

1. **RED: Write Failing Tests**
   ```bash
   # Add test functions (all should fail initially)
   cargo test fingerprinter  # Should see failures
   ```

2. **GREEN: Implement Minimum Code**
   ```bash
   # Implement feature to make tests pass
   cargo test fingerprinter  # Should see all passing
   ```

3. **REFACTOR: Improve Code Quality**
   ```bash
   # Refactor while maintaining test coverage
   cargo test fingerprinter  # Should still pass
   ```

4. **VERIFY: Check Coverage**
   ```bash
   # Optional: Measure coverage
   cargo tarpaulin --out Html --output-dir coverage
   ```

5. **COMMIT: Save Progress**
   ```bash
   # Commit increment with tests
   git add .
   git commit -m "Increment N: Feature X implemented with tests"
   ```

---

## Risk-First Implementation Considerations

**High-Risk Increments:**

**Increment 4 (Chromaprint FFI)** - Highest Risk
- **Risk:** Memory leaks, crashes, undefined behavior
- **Mitigation:**
  - Implement cleanup in all error paths
  - Use RAII pattern (impl Drop for context wrapper)
  - Test with memory checker (valgrind/miri)
  - Add extensive safety documentation
- **Checkpoint:** Run `cargo test --test fingerprinter_tests` with memory checker before proceeding

**Increment 2 (Symphonia Decoding)** - Medium Risk
- **Risk:** Unsupported codecs causing panics
- **Mitigation:**
  - Wrap all Symphonia calls in Result
  - Test error paths explicitly
  - Log unsupported formats gracefully
- **Checkpoint:** Verify error handling tests pass before proceeding

---

## Intermediate Working States

**After Increment 1:**
- ✅ Caching layer functional (can cache/retrieve dummy fingerprints)
- ⚠️ Fingerprinting returns placeholder data
- **Test:** `lookup()` with cached dummy fingerprint returns MBID without API call

**After Increment 2:**
- ✅ Audio decoding functional (real PCM data)
- ✅ Caching layer functional
- ⚠️ Resampling returns input unchanged
- ⚠️ Fingerprinting returns placeholder data
- **Test:** `decode_audio()` with test file returns valid PCM

**After Increment 3:**
- ✅ Audio decoding functional
- ✅ Audio resampling functional (44.1kHz mono)
- ✅ Caching layer functional
- ⚠️ Fingerprinting returns placeholder data
- **Test:** Full decode → resample pipeline produces 44.1kHz mono PCM

**After Increment 4:**
- ✅ Complete fingerprinting pipeline functional
- ✅ Caching layer functional
- ⚠️ Missing some API client tests
- **Test:** `fingerprint_file()` with real audio returns valid Chromaprint fingerprint

**After Increment 5:**
- ✅ All components functional
- ✅ All tests passing
- ✅ Documentation complete
- **Test:** End-to-end workflow (file → fingerprint → AcoustID → MBID) works

---

## Dependencies Between Increments

```
Increment 1 (Caching) ←────────┐
                               │
Increment 2 (Decode) ──┐       │
                       ↓       │
Increment 3 (Resample) ┘       │
                       ↓       │
Increment 4 (Chromaprint)      │
                       ↓       │
                       └───→ Increment 5 (Integration)
```

**Parallelization Opportunity:**
- Increment 1 (Caching) can be developed in parallel with Increments 2-4 (Audio pipeline)
- However, sequential implementation recommended for clarity

---

## Effort Estimates by Component

| Component | Implementation | Tests | Documentation | Total |
|---|---|---|---|---|
| Database schema | 30 LOC | - | 10 LOC | 40 LOC |
| Caching layer | 80 LOC | 120 LOC | 20 LOC | 220 LOC |
| Audio decoding | 90 LOC | 100 LOC | 30 LOC | 220 LOC |
| Audio resampling | 50 LOC | 60 LOC | 20 LOC | 130 LOC |
| Chromaprint FFI | 80 LOC | 60 LOC | 40 LOC | 180 LOC |
| API client tests | - | 100 LOC | - | 100 LOC |
| End-to-end tests | - | 30 LOC | - | 30 LOC |
| **Total** | **330 LOC** | **470 LOC** | **120 LOC** | **920 LOC** |

**Note:** Total LOC increased from initial estimate (630 → 920) after detailed breakdown. Includes comprehensive tests and documentation.

---

## Quality Gates (Per Increment)

**Before Marking Increment Complete:**

- [ ] All acceptance tests for increment pass
- [ ] No compiler warnings (`cargo build --all-targets`)
- [ ] No clippy warnings (`cargo clippy --all-targets`)
- [ ] Code formatted (`cargo fmt --check`)
- [ ] Manual integration test passes (Increments 4-5 only)
- [ ] Increment deliverables checklist 100% complete
- [ ] Git commit created with descriptive message

**Before Final Completion:**

- [ ] All 24 acceptance tests pass (automated)
- [ ] Manual integration test with real audio passes
- [ ] Code coverage ≥ 80% (`cargo tarpaulin`)
- [ ] No unsafe code without safety documentation
- [ ] Traceability matrix verified (all 18 requirements → tests)
- [ ] Specification (IMPL012) matches implementation
- [ ] Change history updated (if using /commit workflow)

---

## Rollback Strategy

**If Increment Fails:**

1. **Identify Blocker:** Review failing tests and error messages
2. **Attempt Quick Fix:** If fixable in <30 minutes, proceed
3. **Otherwise Rollback:**
   ```bash
   git reset --hard HEAD~1  # Rollback to previous commit
   git clean -fd            # Remove untracked files
   ```
4. **Re-plan Increment:** Break into smaller sub-increments if needed
5. **Resume:** Retry with adjusted approach

**Critical Blockers:**
- chromaprint-sys-next won't compile (LLVM missing)
- Symphonia can't decode any test audio (codec support issues)
- Memory leaks in Chromaprint FFI (safety issues)

**Mitigation:**
- Document build requirements clearly
- Test with multiple audio formats
- Use memory checker early and often

---

## Success Metrics

**Implementation Complete When:**
- [ ] All 18 requirements satisfied
- [ ] All 24 acceptance tests pass
- [ ] Traceability matrix shows 100% coverage
- [ ] Manual integration test passes
- [ ] Code coverage ≥ 80%
- [ ] No unsafe code without documentation
- [ ] Performance meets spec (3-min MP3 in < 5 seconds)
- [ ] Memory usage meets spec (< 100MB per concurrent operation)

---

## Phase 4 Completeness Checklist

- [✅] Defined 5 implementation increments
- [✅] Assigned all 18 requirements to increments
- [✅] Defined intermediate working states
- [✅] Created test-first workflow per increment
- [✅] Estimated effort (920 LOC total)
- [✅] Defined quality gates per increment
- [✅] Created rollback strategy
- [✅] Identified dependencies between increments
- [✅] Defined success metrics

---

## Next Steps

**Phase 4 Status:** ✅ COMPLETE

**Ready for Implementation:**

**Start with Increment 1:**
```bash
# Step 1: Create database schema
# Edit: wkmp-common/src/db/init.rs
# Add: create_acoustid_cache_table() function

# Step 2: Write caching tests (RED phase)
# Edit: wkmp-ai/src/services/acoustid_client.rs
# Add: #[cfg(test)] mod tests { ... }

# Step 3: Implement caching layer (GREEN phase)
# Edit: wkmp-ai/src/services/acoustid_client.rs
# Add: hash_fingerprint(), get_cached_mbid(), cache_mbid()

# Step 4: Verify tests pass
cargo test acoustid_client

# Step 5: Commit increment
git add .
git commit -m "Increment 1: Database schema & caching layer with tests"
```

**Or Use /commit Workflow:**
- Implement increment
- Run `/commit` to create commit with automatic change history tracking

---

**PLAN013 Complete:** All 4 phases finished, ready for implementation.
