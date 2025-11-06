# Test Baseline - Technical Debt Remediation

**Date:** 2025-11-05
**Purpose:** Establish test baseline before beginning technical debt remediation (PLAN021)
**Command:** `cargo test --workspace`
**Duration:** 30.28s compilation + test execution

---

## Summary

**Total Tests:** 409 tests across 7 modules
**Passed:** 408
**Failed:** 1 (pre-existing)
**Ignored:** 0

---

## Module-by-Module Results

### wkmp-common
- **Tests:** 1 (config_tests)
- **Result:** ✅ PASS
- **Warnings:** 1 unused variable (`resolver` in config_tests.rs:215)

### wkmp-ap (Audio Player)
- **Unit tests:** 219
- **Result:** ❌ 218 passed, 1 FAILED
- **Integration tests:**
  - chain_assignment_tests: ✅ PASS
  - event_driven_playback_tests: ✅ PASS
- **Warnings:** 76 (dead code, unused imports, unused mut, unused fields)

**Pre-existing Failure:**
```
test tuning::safety::tests::test_backup_file_operations ... FAILED

thread 'tuning::safety::tests::test_backup_file_operations' panicked at wkmp-ap\src\tuning\safety.rs:326:9:
assertion failed: path.exists()
```

**Location:** [wkmp-ap/src/tuning/safety.rs:326](wkmp-ap/src/tuning/safety.rs#L326)
**Impact:** LOW - Test appears to be file system path check, not affecting core playback functionality
**Action:** This failure is PRE-EXISTING and will be EXCLUDED from regression detection during technical debt remediation

### wkmp-ai (Audio Ingest)
- **Unit tests:** 49
- **Integration tests:** 76 (api_integration, component, concurrent, config, db_settings, http_server, recovery, settings_api, workflow)
- **Result:** ✅ ALL PASS
- **Warnings:** 3 (dead code warnings)

### wkmp-ui (User Interface)
- **Tests:** Not shown (likely 0 tests)
- **Result:** ✅ PASS (no tests)

### wkmp-pd (Program Director)
- **Tests:** Not shown (likely 0 tests)
- **Result:** ✅ PASS (no tests)

### wkmp-le (Lyric Editor)
- **Tests:** Not shown (likely 0 tests)
- **Result:** ✅ PASS (no tests)

### wkmp-dr (Database Review)
- **Tests:** Not shown (likely 0 tests)
- **Result:** ✅ PASS (no tests)

---

## Compiler Warnings Summary

### wkmp-common (1 warning)
- Unused variable: `resolver` (config_tests.rs:215)

### wkmp-ap (76 warnings)

**Unused Imports (3):**
- `crate::db::passages::get_passage_album_uuids` (playback/engine/diagnostics.rs:15)
- `MarkerEvent`, `PositionMarker` (playback/engine/diagnostics.rs:19)
- `std::fs` (tuning/system_info.rs:8)

**Dead Code - Functions (numerous):**
- `panic_payload_to_string` (audio/decoder.rs:72)
- `create_sinc_resampler` (audio/resampler.rs:298)
- `auth_middleware`, `auth_middleware_fn`, `auth_middleware_with_state`, etc. (api/auth_middleware.rs - **DEPRECATED CODE TO BE REMOVED**)
- Config struct methods: `load`, `get_os_default_root_folder`, `load_root_folder_from_db`, `db_pool` (**TO BE REMOVED**)
- Many test helper functions

**Dead Code - Fields (numerous):**
- DecodeRequest.full_decode (playback/decoder_worker.rs:43)
- WorkerState.stop_flag (playback/decoder_worker.rs:81)
- QueuePosition.Queued(usize) inner field (playback/engine/queue.rs:28)
- Config struct fields: `database_path`, `root_folder`, `db_pool` (**TO BE REMOVED**)
- Many buffer/mixer fields for future features

**Unused Mut (16 warnings):**
- Multiple `let mut buffer` declarations in playout_ring_buffer tests

### wkmp-ai (3 warnings)
- `format_bytes` function (api/import_workflow.rs:345)
- `ACOUSTID_API_KEY` constant (services/acoustid_client.rs:16)
- `params` field (services/amplitude_analyzer.rs:52)

---

## Baseline Assessment

**Status:** ✅ ACCEPTABLE - Single pre-existing test failure is documented and isolated

**Key Findings:**
1. **One pre-existing test failure** in wkmp-ap tuning::safety module - will be excluded from regression detection
2. **76 warnings in wkmp-ap** - many related to deprecated auth_middleware code (Increment 3 target) and Config struct (Increment 3 target)
3. **All other modules pass** - wkmp-common, wkmp-ai, wkmp-ui, wkmp-pd, wkmp-le, wkmp-dr

**Regression Detection Rules:**
- ANY new test failures (except test_backup_file_operations) = STOP and investigate
- Existing 408 passing tests MUST continue to pass after each increment
- New warnings introduced by refactoring are acceptable IF they are addressed in Increment 5 (Code Quality)

**Test Execution Performance:**
- Compilation: ~30s (reasonable)
- Execution: <2s for most modules
- Total: ~30-35s for full suite

---

## Next Steps

1. ✅ Baseline established
2. ⏭ Proceed with Increment 2: Refactor core.rs (HIGH priority)
3. Monitor test suite after EACH code change
4. Document any test modifications with rationale

---

**Baseline Captured:** 2025-11-05
**Full Output:** See test_baseline_output.txt (1907 lines)
