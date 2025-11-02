# wkmp-ap Test Compilation Investigation

**Date:** 2025-10-30
**Status:** Investigation Complete - Recommendations Provided
**Severity:** HIGH - 12+ test files cannot compile

---

## Executive Summary

**Finding:** wkmp-ap main binary compiles successfully, but 12+ test files fail to compile due to:
1. Missing `shared_secret` field in test AppContext initialization (2 files)
2. Missing module re-exports for test imports (7+ files)
3. Type inference failures (5+ files)

**Root Cause:** Tests not updated after architectural refactoring (likely during single-stream pipeline implementation).

**Impact:** Cannot run wkmp-ap test suite, unknown test coverage, potential regressions undetected.

**Recommendation:** Major test refactoring required (6-10 hours estimated).

---

## Compilation Status

### Main Binary: ✅ SUCCESS
```bash
$ cargo build -p wkmp-ap
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.28s
```

**Result:** 19 warnings (dead code), 0 errors

### Test Suite: ❌ FAILURE
```bash
$ cargo test -p wkmp-ap --no-run
   error: could not compile `wkmp-ap` (test "integration_basic_playback") due to 1 previous error
   error: could not compile `wkmp-ap` (test "error_handling_unit_tests") due to 1 previous error
   (12+ test files affected)
```

---

## Error Categories

### Error 1: Missing `shared_secret` Field (E0063)

**Affected Files:**
- [wkmp-ap/tests/api_integration.rs:93](../wkmp-ap/tests/api_integration.rs:93)
- [wkmp-ap/tests/helpers/test_server.rs:54](../wkmp-ap/tests/helpers/test_server.rs:54)

**Error Message:**
```
error[E0063]: missing field `shared_secret` in initializer of `AppContext`
  --> wkmp-ap\tests\api_integration.rs:93:15
   |
93 |     let ctx = AppContext {
   |               ^^^^^^^^^^ missing `shared_secret`
```

**Root Cause:**
AppContext struct was updated to include `shared_secret` field:

```rust
// From: wkmp-ap/src/api/server.rs:28-38
pub struct AppContext {
    pub state: Arc<SharedState>,
    pub engine: Arc<PlaybackEngine>,
    pub db_pool: Pool<Sqlite>,
    pub volume: Arc<Mutex<f32>>,
    pub shared_secret: i64,  // ← NEW FIELD (added for API authentication)
}
```

**Fix Required:**
Add `shared_secret: 0` (auth disabled) to test AppContext initialization:

```rust
let ctx = AppContext {
    state: state.clone(),
    engine: engine.clone(),
    db_pool: pool.clone(),
    volume: volume.clone(),
    shared_secret: 0,  // Auth disabled for tests
};
```

**Effort:** 5 minutes (2 locations)

---

### Error 2: Unresolved Imports - Missing Re-exports (E0432)

**Affected Files (7+):**
1. crossfade_integration_tests.rs
2. comprehensive_playback_test.rs
3. serial_decoder_tests.rs
4. real_audio_playback_test.rs
5. decoder_pool_tests.rs
6. audible_crossfade_test.rs
7. (others)

**Missing Imports:**

| Import | Exists? | Exported? | Location |
|--------|---------|-----------|----------|
| `wkmp_ap::playback::BufferManager` | ✅ Yes | ❌ No | src/playback/buffer_manager.rs |
| `wkmp_ap::playback::DecoderWorker` | ✅ Yes | ❌ No | src/playback/decoder_worker.rs |
| `wkmp_ap::playback::pipeline::CrossfadeMixer` | ✅ Yes (as mixer.rs) | ❌ No | src/playback/pipeline/mixer.rs |
| `wkmp_ap::audio::PassageBuffer` | ❓ Unknown | ❌ No | - |
| `wkmp_ap::audio::AudioOutput` | ✅ Yes | ❌ No | src/audio/output.rs |
| `wkmp_ap::audio::Resampler` | ✅ Yes | ❌ No | src/audio/resampler.rs |
| `wkmp_ap::audio::SimpleDecoder` | ❓ Unknown | ❌ No | - |

**Error Examples:**
```
error[E0432]: unresolved import `wkmp_ap::playback::BufferManager`
  --> wkmp-ap\tests\comprehensive_playback_test.rs:20:25
   |
20 | use wkmp_ap::playback::{BufferManager, DecoderWorker};
   |                         ^^^^^^^^^^^^^ no `BufferManager` in `playback`
```

**Root Cause Analysis:**

**Current Re-exports (playback/mod.rs:24-28):**
```rust
// Re-exports for external use (tests, other modules)
pub use diagnostics::{PassageMetrics, PipelineMetrics};
// Export from pipeline submodule
```

**Missing Re-exports:**
```rust
pub use buffer_manager::BufferManager;
pub use decoder_worker::DecoderWorker;
```

**Current Re-exports (playback/pipeline/mod.rs:12-15):**
```rust
// Re-exports for external use (tests, other modules)
pub use decoder_chain::{ChunkProcessResult, DecoderChain};
pub use fader::Fader;
```

**Missing Re-exports:**
```rust
pub use mixer::CrossfadeMixer;  // If this is the intended name
```

**Current Re-exports (audio/mod.rs:21-23):**
```rust
// Re-exports for external use (tests, other modules)
pub use types::AudioFrame;
```

**Missing Re-exports:**
```rust
pub use output::AudioOutput;
pub use resampler::Resampler;
pub use decoder::SimpleDecoder;  // If this type exists
```

**Fix Required:**

**Option A: Add Re-exports (Recommended if types are test-relevant)**
Update module re-export sections to include types used by tests.

**Option B: Update Test Imports (Recommended if types are deprecated)**
Change tests to use new architecture types (e.g., `PlaybackEngine` instead of `BufferManager`).

**Option C: Hybrid Approach**
- Add re-exports for stable types still in use
- Refactor tests to use new types for deprecated modules

**Effort:**
- Option A: 30 minutes (add re-exports)
- Option B: 4-6 hours (refactor all affected tests)
- Option C: 2-3 hours (mixed approach)

---

### Error 3: Type Inference Failures (E0282)

**Affected Files (5+):**
- serial_decoder_tests.rs:107, 133
- comprehensive_playback_test.rs:154, 210, 272
- decoder_pool_tests.rs:392

**Error Example:**
```
error[E0282]: type annotations needed for `(Arc<_, _>, _, _)`
   --> wkmp-ap\tests\serial_decoder_tests.rs:107:9
    |
107 |     let (buffer_manager, shared_state, db_pool) = create_test_deps().await;
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Root Cause:**
Helper function `create_test_deps()` returns generic types that compiler cannot infer.

**Fix Required:**
Add explicit type annotations:

```rust
// Before:
let (buffer_manager, shared_state, db_pool) = create_test_deps().await;

// After:
let (buffer_manager, shared_state, db_pool): (Arc<BufferManager>, Arc<SharedState>, Pool<Sqlite>)
    = create_test_deps().await;
```

**Effort:** 15 minutes (5+ locations)

---

## Test File Inventory

**Total Test Files:** 30 files in `wkmp-ap/tests/`

**Broken Test Files (12+):**
1. ✅ api_integration.rs - Missing shared_secret (EASY FIX)
2. ❌ crossfade_integration_tests.rs - Missing PassageBuffer, CrossfadeMixer
3. ❌ comprehensive_playback_test.rs - Missing BufferManager, DecoderWorker + type annotations
4. ❌ serial_decoder_tests.rs - Missing BufferManager, DecoderWorker + type annotations
5. ❌ real_audio_playback_test.rs - Missing BufferManager, DecoderWorker
6. ❌ decoder_pool_tests.rs - Missing BufferManager, DecoderWorker + type annotations
7. ❌ audible_crossfade_test.rs - Missing 5 audio types
8. ✅ helpers/test_server.rs - Missing shared_secret (EASY FIX)
9. ❌ (4+ more files with similar issues)

**Working Test Files:** Unknown (cannot compile to verify)

---

## Architectural Context

### Single-Stream Pipeline Refactoring

The test failures suggest recent architectural changes:

**Evidence:**
1. Comment in `playback/mod.rs:10-12`:
   ```rust
   // Removed: decoder_pool (obsolete - replaced by DecoderWorker)
   pub mod decoder_worker; // New single-threaded worker
   // Removed: serial_decoder (obsolete - replaced by DecoderWorker)
   ```

2. Tests reference old architecture:
   - `BufferManager` exists but not exported
   - `DecoderWorker` exists but not exported
   - `CrossfadeMixer` exists as `mixer.rs` but not exported with that name
   - `PassageBuffer`, `SimpleDecoder` may not exist (replaced by new types)

**Implication:** Major architectural refactoring occurred, tests were not updated.

### API Authentication Addition

**Evidence:**
1. `AppContext.shared_secret` field added (lines 35-38)
2. Comment references `SPEC007 API-AUTH-026`
3. Value of `0` means authentication disabled

**Implication:** Recent feature addition, tests need trivial update.

---

## Recommendations

### Priority 1: Quick Wins (30 minutes)

**Goal:** Get some tests compiling

**Actions:**
1. **Fix shared_secret** (2 files, 5 minutes):
   - api_integration.rs:93
   - helpers/test_server.rs:54
   - Add `shared_secret: 0` to AppContext initialization

2. **Add essential re-exports** (3 modules, 15 minutes):
   - `playback/mod.rs`: Add `pub use buffer_manager::BufferManager;`
   - `playback/mod.rs`: Add `pub use decoder_worker::DecoderWorker;`
   - `playback/pipeline/mod.rs`: Add `pub use mixer::CrossfadeMixer;`

3. **Fix type annotations** (5+ files, 10 minutes):
   - Add explicit types to `create_test_deps()` calls

**Expected Result:** 5-7 test files compiling

---

### Priority 2: Module Investigation (1 hour)

**Goal:** Understand current architecture vs test expectations

**Actions:**
1. **Catalog current audio module types:**
   - What replaced `PassageBuffer`?
   - What replaced `SimpleDecoder`?
   - What replaced `AudioOutput`?
   - Document in table

2. **Review recent commits:**
   - Identify when single-stream refactoring occurred
   - Identify original purpose of each broken test
   - Determine if tests are still relevant

3. **Create migration guide:**
   - Old type → New type mapping
   - Deprecated patterns → New patterns
   - Example test refactoring

**Expected Result:** Clear understanding of refactoring scope

---

### Priority 3: Systematic Test Refactoring (4-8 hours)

**Goal:** Restore full test suite functionality

**Actions:**
1. **Phase 1: Architectural tests (2-3 hours)**
   - Update tests using deprecated types
   - Refactor to new PlaybackEngine-based architecture
   - Files: comprehensive_playback_test.rs, serial_decoder_tests.rs, etc.

2. **Phase 2: Integration tests (2-3 hours)**
   - Update crossfade tests for new mixer architecture
   - Update audio output tests for new output types
   - Files: crossfade_integration_tests.rs, audible_crossfade_test.rs, etc.

3. **Phase 3: Verification (1-2 hours)**
   - Run all tests
   - Fix remaining compilation issues
   - Document test coverage gaps

**Expected Result:** Full test suite compiling and running

---

### Priority 4: Test Quality Improvements (Optional, 2-4 hours)

**Goal:** Modernize test infrastructure

**Actions:**
1. Add `serial_test` for shared state tests (like wkmp-common, wkmp-ai)
2. Improve test organization (group by feature)
3. Add traceability comments (requirement IDs)
4. Document test helpers

**Expected Result:** Test suite matching wkmp-ai quality standard

---

## Implementation Approach

### Recommended Sequence

**Week 1: Quick Restoration**
1. Day 1: Priority 1 (shared_secret + re-exports + type annotations)
2. Day 2: Priority 2 (module investigation + migration guide)
3. Day 3-5: Priority 3 Phase 1 (architectural tests)

**Week 2: Complete Restoration**
4. Day 6-7: Priority 3 Phase 2 (integration tests)
5. Day 8: Priority 3 Phase 3 (verification)
6. Day 9-10: Priority 4 (quality improvements - optional)

**Minimum Viable:** Priority 1 only (30 minutes) gets partial test coverage
**Recommended:** Priority 1-3 (6-10 hours total) gets full test coverage
**Ideal:** Priority 1-4 (8-14 hours total) achieves wkmp-ai quality level

---

## Risk Assessment

### Risks of NOT Fixing

**HIGH RISK:**
- ❌ No automated test coverage for wkmp-ap
- ❌ Regressions undetected
- ❌ Future refactoring difficult without test safety net
- ❌ Violates project test-first principles

**MEDIUM RISK:**
- ⚠️ New features cannot follow test-driven development
- ⚠️ Integration issues with other modules undetected
- ⚠️ Performance regressions undetected

**LOW RISK:**
- ⚙️ Main binary compiles (production code works)
- ⚙️ Temporary workaround: Manual testing only

### Risks of Fixing (Option B: Full Refactor)

**MEDIUM RISK:**
- ⚠️ Time investment (6-10 hours)
- ⚠️ May discover deeper architectural issues during refactoring
- ⚠️ Requires understanding of single-stream pipeline design

**LOW RISK:**
- ⚙️ Breaking production code (tests are isolated)
- ⚙️ Introducing new bugs (fixing tests, not production code)

---

## Decision Framework

### When to Fix?

**Fix Immediately If:**
- Planning new features for wkmp-ap
- Need test coverage for upcoming releases
- Want to follow test-first development

**Defer If:**
- wkmp-ap is stable, no changes planned
- Focus is on other modules (wkmp-ui, wkmp-ai, etc.)
- Accepting manual testing risk

### Which Priority Level?

**Priority 1 Only:** If you need basic smoke tests quickly
**Priority 1-3:** If you need full test coverage (RECOMMENDED)
**Priority 1-4:** If you want best-in-class test quality

---

## Comparison with Other Modules

| Module | Test Status | Pass Rate | Quality Level |
|--------|-------------|-----------|---------------|
| **wkmp-ai** | ✅ All passing | 100% (29 tests) | ⭐⭐⭐⭐⭐ Excellent |
| **wkmp-common** | ✅ All passing | 100% (18+29 tests) | ⭐⭐⭐⭐⭐ Excellent |
| **wkmp-ap** | ❌ Won't compile | N/A (0 tests run) | ⭐ Poor |

**Gap:** wkmp-ap significantly below project quality standard

---

## Conclusion

**Finding:** wkmp-ap test suite is broken but fixable.

**Root Cause:** Architectural refactoring (single-stream pipeline) left tests outdated.

**Severity:** HIGH - No automated test coverage for core playback module.

**Recommendation:**
1. **Minimum:** Implement Priority 1 (30 minutes) to get partial coverage
2. **Recommended:** Implement Priority 1-3 (6-10 hours) to restore full coverage
3. **Ideal:** Implement Priority 1-4 (8-14 hours) to achieve wkmp-ai quality level

**Next Steps:**
1. Review this investigation with team/lead
2. Decide on priority level and timeline
3. Create PLAN### document if choosing Priority 2-4 (non-trivial refactoring)
4. Execute fixes per chosen priority level

---

**Investigation completed by:** Claude (Sonnet 4.5)
**Date:** 2025-10-30
**Files Analyzed:** 30 test files, 3 module definitions, 1 struct definition
**Compilation Errors Cataloged:** 4 unique error types, 12+ affected files
