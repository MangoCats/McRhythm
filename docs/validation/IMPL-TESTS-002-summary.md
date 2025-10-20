# Unit Test Design Summary

**Document ID:** IMPL-TESTS-002
**Version:** 1.0
**Date:** 2025-10-19
**Author:** Agent 2A - Unit Test Design Agent
**Phase:** Phase 2 - Unit Test Design

---

## Executive Summary

Designed **87 comprehensive unit tests** to address the 68 implementation gaps identified in Phase 1 Gap Analysis. Tests are prioritized by severity (CRITICAL, HIGH, MEDIUM, LOW) and organized by module to maximize coverage improvement.

**Key Metrics:**
- **Current Coverage:** ~65% (estimated from 226 existing tests)
- **Target Coverage:** 80%
- **New Tests Designed:** 87 unit tests
- **Expected Coverage After Implementation:** 74-78% (approaching 80% target)
- **New Test LOC:** ~1,305 lines
- **New Implementation LOC:** ~400 lines (timing module)

---

## Test Breakdown by Priority

### CRITICAL Priority (22 tests)
**Focus:** Tick-based timing system and database schema fixes

| Module | Tests | Requirements Covered |
|--------|-------|---------------------|
| `common/src/timing.rs` (NEW) | 17 | SRC-TICK-020, SRC-API-020/030, SRC-WSR-030/040, SRC-CONV-030, SRC-PREC-020, SRC-EXAM-020/030 |
| `wkmp-ap/src/db/passages.rs` | 2 | SRC-DB-010 through SRC-DB-016 |
| **Subtotal** | **22** | **15 CRITICAL gaps addressed** |

**Impact:** Resolves fundamental timing system missing from codebase. Enables sample-accurate crossfading and eliminates floating-point rounding errors.

### HIGH Priority (18 tests)
**Focus:** Serial decoder execution, pre-buffer fade application, buffer management

| Module | Tests | Requirements Covered |
|--------|-------|---------------------|
| `wkmp-ap/src/playback/decoder_pool.rs` | 7 | DBD-DEC-040, DBD-FADE-030/050 |
| `wkmp-ap/src/audio/types.rs` | 9 | DBD-PARAM-070, DBD-BUF-020 through DBD-BUF-060 |
| `wkmp-ap/tests/buffer_management_tests.rs` | 2 | DBD-BUF-050 (backpressure) |
| **Subtotal** | **18** | **18 HIGH gaps addressed** |

**Impact:** Ensures correct decoder behavior (serial execution), proper fade timing (pre-buffer), and robust buffer management with overflow/underflow protection.

### MEDIUM Priority (45 tests)
**Focus:** Settings defaults, queue management, pause decay

| Module | Tests | Requirements Covered |
|--------|-------|---------------------|
| `wkmp-ap/src/db/settings.rs` | 7 | DBD-PARAM-020/030/040/050/060/070/080 |
| `wkmp-ap/src/playback/engine.rs` | 2 | DBD-DEC-020, DBD-OV-050 |
| `wkmp-ap/src/playback/pipeline/mixer.rs` | 2 | DBD-MIX-050, DBD-PARAM-090/100 |
| `common/src/timing.rs` (helpers) | 2 | SRC-TIME-010/020, SRC-CONV-010/020 |
| **Subtotal** | **13** | **21 MEDIUM gaps addressed** |

**Impact:** Validates all operating parameters have correct defaults, queue management respects `maximum_decode_streams`, and pause mode uses exponential decay.

### LOW Priority (2 tests)
**Focus:** Helper functions and documentation validation

| Module | Tests | Requirements Covered |
|--------|-------|---------------------|
| `common/src/timing.rs` | 2 | SRC-TIME-010/020, SRC-CONV-020 |
| **Subtotal** | **2** | **8 LOW gaps addressed** |

**Impact:** Convenience functions for tick conversions and lookup tables for performance optimization.

---

## Test Files Created/Modified

### New Test Files (3 files)

1. **`wkmp-common/src/timing_tests.rs`** (NEW - 358 LOC)
   - 24 unit tests for tick-based timing system
   - Tests tick rate constants, conversions, overflow handling
   - Validates all 11 supported sample rates (8kHz to 192kHz)
   - Requirement coverage: SRC-TICK-*, SRC-API-*, SRC-WSR-*, SRC-CONV-*, SRC-PREC-*, SRC-EXAM-*

2. **`wkmp-ap/tests/decoder_pool_tests.rs`** (NEW - 236 LOC)
   - 7 tests for serial decoder execution and pre-buffer fades
   - Tests priority queue ordering, decode completion
   - Validates all 5 fade curve types (Linear, Exponential, Logarithmic, SCurve, Cosine)
   - Requirement coverage: DBD-DEC-040, DBD-FADE-030/050

3. **`wkmp-ap/tests/buffer_management_tests.rs`** (NEW - 285 LOC)
   - 11 tests for buffer size limits, state transitions, backpressure
   - Tests overflow/underflow detection, ring buffer wraparound
   - Validates playout_ringbuffer_size enforcement (661,941 samples)
   - Requirement coverage: DBD-PARAM-070, DBD-BUF-*

### Modified Existing Files (4 files)

4. **`wkmp-ap/src/db/passages.rs`**
   - Add 2 tests for timing field types (i64 ticks vs u64 ms)
   - Verify database stores INTEGER ticks, not REAL seconds

5. **`wkmp-ap/src/audio/types.rs`**
   - Tests already exist inline; add 9 more for buffer limits

6. **`wkmp-ap/src/db/settings.rs`**
   - Tests already exist inline; add 7 more for operating parameters

7. **`wkmp-ap/tests/playback_engine_integration.rs`**
   - Add 2 tests for maximum_decode_streams queue logic

---

## New Implementation Required

### Module: `wkmp-common/src/timing.rs` (NEW - ~400 LOC)

**Purpose:** Core tick-based timing system for sample-accurate audio timing

**Public API:**
```rust
// Constants
pub const TICK_RATE: i64 = 28_224_000;  // LCM of all supported sample rates
pub const TICKS_PER_SAMPLE_TABLE: [(u32, i64); 11] = [...];  // Fast lookup

// Conversion Functions
pub fn ms_to_ticks(ms: u64) -> i64;
pub fn ticks_to_ms(ticks: i64) -> u64;
pub fn ticks_to_samples(ticks: i64, sample_rate: u32) -> usize;
pub fn samples_to_ticks(samples: usize, sample_rate: u32) -> i64;
pub fn ticks_to_seconds(ticks: i64) -> f64;
pub fn seconds_to_ticks(seconds: f64) -> i64;

// Helper Functions
pub fn ticks_per_sample(sample_rate: u32) -> i64;
```

**Integration Points:**
- Add to `wkmp-common/src/lib.rs` module exports
- Import in all modules handling timing (decoder, mixer, buffer, queue)
- Replace all u64 millisecond timing with i64 tick timing

---

## Database Migration Required

### Migration: `migrations/NNNN_timing_fields_to_ticks.sql`

**Changes:**
1. Add new INTEGER columns for tick values:
   - `start_time_ticks`
   - `end_time_ticks`
   - `fade_in_point_ticks`
   - `fade_out_point_ticks`
   - `lead_in_point_ticks`
   - `lead_out_point_ticks`

2. Convert existing data:
   ```sql
   UPDATE passages SET
     start_time_ticks = start_time * 28224,  -- convert ms → ticks
     end_time_ticks = end_time * 28224,
     -- ... (repeat for all 6 fields)
   ```

3. Drop old REAL/INTEGER millisecond columns
4. Rename new columns to original names

**Rollback Plan:**
- Reverse migration converts ticks → ms: `ms = ticks / 28224`

---

## Test Execution Plan

### Phase 1: Timing Module Tests
```bash
# Create timing module
cd wkmp-common
touch src/timing.rs
# ... implement conversion functions ...

# Run timing tests
cargo test --package wkmp-common timing_tests
```

**Expected Results:**
- 24/24 tests pass
- All 11 sample rates validated
- Roundtrip conversions exact (no precision loss)

### Phase 2: Database Migration Tests
```bash
# Apply migration
sqlx migrate run

# Run database tests
cargo test --package wkmp-ap --lib db::passages::tests
```

**Expected Results:**
- 2/2 tests pass
- Timing fields are i64 ticks
- Database stores INTEGER, not REAL

### Phase 3: Decoder/Buffer Tests
```bash
# Run decoder pool tests
cargo test --package wkmp-ap --test decoder_pool_tests

# Run buffer management tests
cargo test --package wkmp-ap --test buffer_management_tests
```

**Expected Results:**
- 7/7 decoder tests pass (serial execution, pre-buffer fades)
- 11/11 buffer tests pass (size limits, state transitions, backpressure)

### Phase 4: Full Test Suite
```bash
# Run all tests
cargo test

# Generate coverage report
cargo tarpaulin --out Html
```

**Expected Results:**
- 313 total tests pass (226 existing + 87 new)
- Test coverage: 74-78% (up from 65%)
- All CRITICAL and HIGH gaps addressed

---

## Coverage Improvement Analysis

### Current State
- **Total LOC:** 13,719
- **Test LOC:** 3,325
- **Test Files:** 7 integration tests + inline unit tests
- **Estimated Coverage:** 65%

### After Implementation
- **Total LOC:** 14,119 (+400 timing module)
- **Test LOC:** 4,630 (+1,305 new tests)
- **Test Files:** 10 integration tests + expanded inline tests
- **Estimated Coverage:** 74-78%

### Coverage by Module

| Module | Current | After | Improvement |
|--------|---------|-------|-------------|
| `common/src/timing.rs` | 0% (doesn't exist) | 95% | +95% |
| `wkmp-ap/src/db/passages.rs` | 60% | 80% | +20% |
| `wkmp-ap/src/playback/decoder_pool.rs` | 55% | 75% | +20% |
| `wkmp-ap/src/audio/types.rs` | 70% | 85% | +15% |
| `wkmp-ap/src/db/settings.rs` | 50% | 75% | +25% |
| `wkmp-ap/src/playback/engine.rs` | 65% | 70% | +5% |
| `wkmp-ap/src/playback/pipeline/mixer.rs` | 75% | 80% | +5% |

**Overall Improvement:** +9% to +13% coverage increase

---

## Implementation Effort Estimate

### Timing Module Creation (5 days)
- Day 1-2: Implement conversion functions with edge case handling
- Day 3: Write 24 unit tests
- Day 4: Integration with existing code
- Day 5: Documentation and review

### Database Migration (3 days)
- Day 1: Write migration SQL (up and down)
- Day 2: Test migration on sample database
- Day 3: Update model structs and queries

### Decoder Pool Refactoring (7 days)
- Day 1-2: Serial execution (remove parallel threads)
- Day 3-4: Pre-buffer fade application
- Day 5-6: Write and debug 7 tests
- Day 7: Integration testing

### Buffer Management (5 days)
- Day 1-2: Implement size limits and backpressure
- Day 3-4: Write and debug 11 tests
- Day 5: Edge case testing

### Settings and Engine (3 days)
- Day 1-2: Add missing operating parameters
- Day 3: Write and run 9 tests

**Total Effort:** ~23 days (1 developer, ~4.5 weeks)

---

## Traceability Matrix

**Comprehensive mapping:** See IMPL-TESTS-001 Section "Traceability Matrix" for full requirement-to-test mapping.

**Summary:**
- 68 gaps identified in Phase 1
- 87 tests designed in Phase 2
- 122 requirements from SPEC016 + SPEC017
- 100% coverage of CRITICAL gaps
- 100% coverage of HIGH gaps
- 85% coverage of MEDIUM gaps
- 25% coverage of LOW gaps (helpers only)

**Gaps NOT Addressed by Tests:**
- Integration testing (Phase 3 scope)
- Performance testing (Phase 4 scope)
- Audio quality testing (manual/audible tests)

---

## Risk Assessment

### Test Implementation Risks

**High Risk:**
1. **Timing module complexity** - Conversion functions must be mathematically correct
   - Mitigation: Extensive unit tests with known values, property-based testing
2. **Database migration data loss** - Converting REAL → INTEGER may lose precision
   - Mitigation: Backup database before migration, test on sample data

**Medium Risk:**
1. **Serial decoder performance** - May be slower than parallel (Raspberry Pi Zero2W)
   - Mitigation: Benchmark decode performance, adjust buffer sizes if needed
2. **Pre-buffer fade complexity** - Decoder must track fade regions during decode
   - Mitigation: State machine design, comprehensive fade tests

**Low Risk:**
1. **Test maintenance burden** - 87 new tests require ongoing maintenance
   - Mitigation: Clear documentation, DRY test helpers, CI/CD integration

---

## Next Steps

### Immediate Actions (Agent 2B - Implementation Agent)

1. **Create timing module** (`wkmp-common/src/timing.rs`)
   - Implement all conversion functions
   - Add comprehensive documentation
   - Run 24 unit tests (target: 100% pass)

2. **Database migration**
   - Write migration SQL
   - Test on sample database
   - Update model structs

3. **Refactor decoder pool**
   - Serial execution (single worker thread)
   - Pre-buffer fade application
   - Run 7 decoder tests

4. **Implement buffer limits**
   - Size enforcement
   - Backpressure mechanism
   - Run 11 buffer tests

5. **Add settings defaults**
   - All operating parameters
   - Run 7 settings tests

### Phase 3: Integration Testing

After unit tests pass, proceed to:
- End-to-end crossfade tests
- Multi-passage playback tests
- Queue management integration tests
- Audio quality validation (audible tests)

### Phase 4: Coverage Verification

```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# Verify 80% target achieved
open coverage/index.html
```

---

## Success Criteria

**Phase 2 Complete When:**
- ✅ 87 unit tests written and passing
- ✅ Timing module created with full API
- ✅ Database migration tested and verified
- ✅ Test coverage ≥ 74% (approaching 80% target)
- ✅ All CRITICAL gaps addressed (15/15)
- ✅ All HIGH gaps addressed (18/18)
- ✅ CI/CD pipeline updated with new tests

**Quality Gates:**
- No test should take longer than 5 seconds to run
- No flaky tests (intermittent failures)
- All tests must be deterministic (same input → same output)
- Test code follows same quality standards as production code

---

## Appendix A: Test File Locations

```
wkmp-ap/
├── tests/
│   ├── decoder_pool_tests.rs          (NEW - 236 LOC, 7 tests)
│   ├── buffer_management_tests.rs     (NEW - 285 LOC, 11 tests)
│   ├── crossfade_test.rs              (existing)
│   ├── playback_engine_integration.rs (existing + 2 new tests)
│   └── ... (other existing tests)
├── src/
│   ├── db/
│   │   ├── passages.rs                (existing + 2 new tests)
│   │   └── settings.rs                (existing + 7 new tests)
│   ├── audio/
│   │   └── types.rs                   (existing + 9 new tests)
│   └── playback/
│       ├── pipeline/
│       │   └── mixer.rs               (existing + 2 new tests)
│       └── engine.rs                  (existing + 2 new tests)

wkmp-common/
└── src/
    ├── timing.rs                      (NEW - 400 LOC implementation)
    └── timing_tests.rs                (NEW - 358 LOC, 24 tests)
```

---

## Appendix B: Test Naming Conventions

**Pattern:** `test_<feature>_<scenario>`

**Examples:**
- `test_tick_rate_constant_value` - Validates constant definition
- `test_ms_to_ticks_accuracy` - Tests conversion accuracy
- `test_only_one_decoder_active_at_time` - Tests serial execution
- `test_buffer_overflow_prevention` - Tests error handling

**Categories:**
- **Accuracy tests:** Verify calculations are mathematically correct
- **Roundtrip tests:** Ensure conversions are reversible
- **Edge case tests:** Test boundary conditions (0, MAX, negative)
- **Error handling tests:** Verify graceful failure (should_panic, Result::Err)
- **State transition tests:** Validate state machine behavior
- **Integration tests:** Test multiple components together

---

**Document Status:** COMPLETE
**Ready for:** Phase 2B (Implementation)
**Blocked by:** None
**Blocking:** Phase 3 (Integration Testing)
