# PLAN018: Centralized Global Parameters - PLAN SUMMARY

**Status:** Ready for Implementation (Phases 1-3 Complete)
**Created:** 2025-11-02
**Specification Source:** [wip/IMPL-GLOBAL-PARAMS-centralized_global_parameters.md](../../wip/IMPL-GLOBAL-PARAMS-centralized_global_parameters.md)
**Plan Location:** `wip/PLAN018_centralized_global_parameters/`

---

## READ THIS FIRST

This document provides the complete summary for PLAN018: Centralized Global Parameters migration. The plan systematically migrates 15 SPEC016 database-backed parameters from scattered hardcoded values to a centralized singleton, eliminating the timing bug that caused 10% position error.

**For Implementation:**
- Read this summary (~480 lines)
- Review detailed requirements: [requirements_index.md](requirements_index.md)
- Review test specifications: [02_test_specifications/test_index.md](02_test_specifications/test_index.md)
- Follow traceability matrix: [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md)

**Context Window Budget:** Summary + per-parameter guidance = ~650 lines per migration

---

## Executive Summary

### Problem Being Solved

**Current State - Critical Bug:**
- Global parameters scattered across module-specific fields
- Hardcoded values in 8+ locations (e.g., `44100` sample rate)
- **Today's bug (2025-11-02):** Position counter 10% too fast due to hardcoded 44100 Hz when device runs at 48 kHz
- No single source of truth for parameters
- Parameter updates require code changes in multiple places

**Impact:**
```rust
// WRONG (caused 10% timing error)
let tick_increment = samples_to_ticks(frames_read, 44100);

// CORRECT (actual device rate)
let sample_rate = *PARAMS.working_sample_rate.read().unwrap();
let tick_increment = samples_to_ticks(frames_read, sample_rate);
```

**Position tracking error:** 48000/44100 = 1.088x too fast (10 seconds displayed after 9.18 seconds actual)

### Solution Approach

**Centralized Singleton in wkmp-common:**

```rust
// wkmp-common/src/params.rs
pub static PARAMS: Lazy<GlobalParams> = Lazy::new(GlobalParams::default);

pub struct GlobalParams {
    pub working_sample_rate: RwLock<u32>,
    pub mixer_min_start_level: RwLock<usize>,
    // ... 15 parameters total
}
```

**Access Pattern:**
```rust
// Read (fast, uncontended)
let rate = *wkmp_common::params::PARAMS.working_sample_rate.read().unwrap();

// Write (rare, initialization only)
*wkmp_common::params::PARAMS.working_sample_rate.write().unwrap() = 48000;
```

**Migration Strategy:** Incremental - one parameter at a time, full testing after each, 15 separate commits

### Implementation Status

**Phases 1-3 Complete:**
- ✅ Phase 1: Scope Definition - 9 requirements extracted, 15 parameters identified
- ✅ Phase 2: Specification Verification - 2 HIGH issues resolved, 4 MEDIUM tracked, no blockers
- ✅ Phase 3: Test Definition - 29 tests defined (21 unit, 6 integration, 2 manual), 100% coverage

**Phases 4-8 Status:** Not applicable - Phases 1-3 sufficient for this systematic refactoring

---

## Requirements Summary

**Total Requirements:** 9 (5 functional, 4 non-functional)

### Functional Requirements

| ID | Priority | Summary |
|----|----------|---------|
| FR-001 | HIGH | Centralized parameter storage in GlobalParams singleton |
| FR-002 | HIGH | RwLock<T> for read-frequently/write-rarely pattern |
| FR-003 | **CRITICAL** | Zero hardcoded parameter values in production code |
| FR-004 | HIGH | Parameters loaded from database settings table on startup |
| FR-005 | MEDIUM | Backward compatible (no breaking API changes) |

### Non-Functional Requirements

| ID | Priority | Summary |
|----|----------|---------|
| NFR-001 | HIGH | RwLock::read() < 10ns uncontended, zero allocation |
| NFR-002 | HIGH | Type-safe values, validated ranges on write |
| NFR-003 | HIGH | Full test suite passes after each parameter migration |
| NFR-004 | MEDIUM | Clear documentation with SPEC016 traceability |

**Full Requirements:** See [requirements_index.md](requirements_index.md)

---

## Parameter Inventory

**15 Parameters Total (SPEC016 Database-Backed):**

| Risk Tier | Count | Parameters |
|-----------|-------|------------|
| **Tier 1 (Low Risk)** | 5 | maximum_decode_streams, decode_work_period, pause_decay_factor, pause_decay_floor, volume_level |
| **Tier 2 (Medium Risk)** | 4 | output_ringbuffer_size, playout_ringbuffer_size, playout_ringbuffer_headroom, mixer_min_start_level |
| **Tier 3 (High Risk - Timing Critical)** | 6 | working_sample_rate, audio_buffer_size, mixer_check_interval_ms, decode_chunk_size, output_refill_period, decoder_resume_hysteresis_samples |

**Migration Order:** Tier 1 → Tier 2 → Tier 3 (lowest risk first)

**Critical Parameters (Extra Testing Required):**
- `working_sample_rate` (DBD-PARAM-020) - **Already caused 10% timing bug**
- `audio_buffer_size` (DBD-PARAM-110) - Real-time audio callback timing
- `mixer_check_interval_ms` (DBD-PARAM-111) - Mixer wake timing

**Full Inventory:** See [requirements_index.md](requirements_index.md) lines 108-127

---

## Scope

### ✅ In Scope

1. **GlobalParams Singleton:** Create `wkmp-common/src/params.rs` with 15 RwLock<T> fields
2. **Database Initialization:** Load all parameters from settings table at startup
3. **Parameter Migration:** All 15 parameters, one at a time, with full testing
4. **Validation Methods:** Setter methods with range validation for each parameter
5. **Testing:** Unit tests, integration tests, manual tests (29 total)
6. **Documentation:** Doc comments, DBD-PARAM-XXX tags, migration commit messages

### ❌ Out of Scope

1. **Hot-Reloadable Parameters** - Deferred to future enhancement
2. **Parameter History/Audit Trail** - Not required
3. **Per-User Parameter Overrides** - Not applicable
4. **GUI Configuration Interface** - Settings table is source
5. **New Parameters** - Only existing SPEC016 parameters
6. **Database Schema Changes** - Use existing settings table

**Full Scope:** See [scope_statement.md](scope_statement.md)

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0
- **HIGH Issues:** 3 (2 resolved in plan, 1 closed as already addressed)
- **MEDIUM Issues:** 4 (tracked during implementation)
- **LOW Issues:** 1 (apply opportunistically)

**Decision:** ✅ PROCEED - No blocking issues

### HIGH Issues (Resolved)

**HIGH-001: Error Handling Policy** - ✅ RESOLVED
- **Policy:** Database connection error = fatal, missing parameter = WARN + use default, gracefully degrade
- **Documented:** See [01_specification_issues.md](01_specification_issues.md) lines 271-308

**HIGH-003: Validation Ranges** - ✅ RESOLVED
- **Ranges Defined:** All 15 parameters have validation rules
- **Documented:** See [01_specification_issues.md](01_specification_issues.md) lines 363-382

### MEDIUM Issues (Tracked)

1. **Performance Measurement:** Add criterion.rs microbenchmark
2. **Full Test Suite Definition:** `cargo test --workspace` = 100% pass
3. **Test Isolation Strategy:** Use `GlobalParams::new_for_test()` for unit tests
4. **Rollback Procedure:** `git reset --hard HEAD~1` per parameter

**Full Analysis:** See [01_specification_issues.md](01_specification_issues.md)

---

## Implementation Roadmap

### Pre-Migration Setup

**Task:** Create GlobalParams infrastructure
**Estimated Effort:** 1-2 hours

**Deliverables:**
1. Create `wkmp-common/src/params.rs` with:
   - GlobalParams struct (15 RwLock<T> fields)
   - Default trait implementation (SPEC016 defaults)
   - init_from_database() method (load from settings table)
   - 15 setter methods with validation
2. Modify `wkmp-common/src/lib.rs`: Add `pub mod params;`
3. Modify `wkmp-ap/src/main.rs`: Call init_from_database() after pool creation
4. Run pre-migration tests: TC-U-001-01, TC-U-001-02, TC-U-002-*, TC-I-004-*

**Success Criteria:**
- [ ] GlobalParams struct compiles
- [ ] All pre-migration tests pass
- [ ] Database initialization succeeds

---

### Per-Parameter Migration (Repeat 15 Times)

**Process:** 7-step migration + commit (see [spec lines 336-467](../../wip/IMPL-GLOBAL-PARAMS-centralized_global_parameters.md#L336-L467))

**Tier 1 Parameters (Low Risk - Migrate First):**

#### Parameter 1: maximum_decode_streams
- **Type:** usize
- **Default:** 12
- **Range:** [1, 32]
- **Risk:** Low (resource usage only)
- **Effort:** 30-45 minutes

#### Parameter 2: decode_work_period
- **Type:** u64
- **Default:** 5000 ms
- **Range:** [100, 60000]
- **Risk:** Low (background task timing)
- **Effort:** 30-45 minutes

#### Parameter 3: pause_decay_factor
- **Type:** f64
- **Default:** 0.95
- **Range:** [0.5, 0.99]
- **Risk:** Low (audio quality, gradual)
- **Effort:** 30-45 minutes

#### Parameter 4: pause_decay_floor
- **Type:** f64
- **Default:** 0.0001778
- **Range:** [0.00001, 0.001]
- **Risk:** Low (audio quality threshold)
- **Effort:** 30-45 minutes

#### Parameter 5: volume_level
- **Type:** f32
- **Default:** 0.5
- **Range:** [0.0, 1.0]
- **Risk:** Low (user-visible but non-critical)
- **Effort:** 30-45 minutes

**Tier 1 Subtotal:** 2.5-4 hours

---

**Tier 2 Parameters (Medium Risk - Buffer Sizes):**

#### Parameter 6: output_ringbuffer_size
- **Type:** usize
- **Default:** 88200 samples
- **Range:** [4410, 1000000]
- **Risk:** Medium (affects latency)
- **Effort:** 30-45 minutes

#### Parameter 7: playout_ringbuffer_size
- **Type:** usize
- **Default:** 661941 samples
- **Range:** [44100, 10000000]
- **Risk:** Medium (affects buffering)
- **Effort:** 30-45 minutes

#### Parameter 8: playout_ringbuffer_headroom
- **Type:** usize
- **Default:** 4410 samples
- **Range:** [2205, 88200]
- **Risk:** Medium (safety margin)
- **Effort:** 30-45 minutes

#### Parameter 9: mixer_min_start_level
- **Type:** usize
- **Default:** 22050 samples
- **Range:** [2205, 88200]
- **Risk:** Medium (startup behavior)
- **Effort:** 30-45 minutes

**Tier 2 Subtotal:** 2-3 hours

---

**Tier 3 Parameters (High Risk - Timing Critical - Migrate Last):**

#### Parameter 10: working_sample_rate ⚠️ CRITICAL
- **Type:** u32
- **Default:** 44100 Hz
- **Range:** [8000, 192000]
- **Risk:** **HIGH** (already caused 10% timing bug)
- **Effort:** 60-90 minutes (extra testing required)
- **Extra Verification:**
  - Stopwatch test (60s playback, verify ±100ms accuracy)
  - Position tracking spot checks
  - Manual code review for all occurrences

#### Parameter 11: audio_buffer_size ⚠️ CRITICAL
- **Type:** u32
- **Default:** 2208 frames
- **Range:** [512, 8192]
- **Risk:** **HIGH** (audio callback timing)
- **Effort:** 60-90 minutes (extensive manual testing)
- **Extra Verification:**
  - Test multiple buffer sizes (512, 1024, 2048, 4096)
  - Listen for glitches
  - Monitor for underruns

#### Parameter 12: mixer_check_interval_ms ⚠️ CRITICAL
- **Type:** u64
- **Default:** 10 ms
- **Range:** [5, 100]
- **Risk:** **HIGH** (mixer loop timing)
- **Effort:** 45-60 minutes
- **Extra Verification:**
  - Monitor CPU usage
  - Verify no underruns
  - Check buffer levels

#### Parameter 13: decode_chunk_size
- **Type:** usize
- **Default:** 25000 samples
- **Range:** [4410, 441000]
- **Risk:** Medium-High (decoder timing)
- **Effort:** 30-45 minutes

#### Parameter 14: output_refill_period
- **Type:** u64
- **Default:** 90 ms
- **Range:** [10, 1000]
- **Risk:** Medium-High (mixer wake timing)
- **Effort:** 30-45 minutes

#### Parameter 15: decoder_resume_hysteresis_samples
- **Type:** u64
- **Default:** 44100 samples
- **Range:** [2205, 441000]
- **Risk:** Medium-High (backpressure timing)
- **Effort:** 30-45 minutes

**Tier 3 Subtotal:** 4.5-7 hours

---

### Post-Migration Verification

**Task:** Final validation
**Estimated Effort:** 30-60 minutes

**Deliverables:**
1. Run TC-U-005-01: Verify API compilation unchanged
2. Run TC-U-101-01, TC-U-101-02: Performance verification
3. Run TC-M-003-01: Final manual code review (all parameters)
4. Run TC-M-104-01: Documentation review
5. Full regression test: `cargo test --workspace`
6. Manual smoke test: Play audio, verify timing, check logs

**Success Criteria:**
- [ ] All 29 tests pass
- [ ] No hardcoded values remain
- [ ] Position timing accurate (stopwatch verified)
- [ ] Audio playback quality unchanged
- [ ] Performance baseline maintained

**Total Estimated Effort:** 10-15 hours (15 parameters + setup + verification)

---

## Per-Parameter Migration Steps

**For EACH parameter, follow this 7-step process:**

### Step 1: Add to GlobalParams
Add parameter field to `wkmp-common/src/params.rs`:
```rust
/// [DBD-PARAM-XXX] Description
///
/// Valid range: [min, max] units
/// Default: N units
pub parameter_name: RwLock<Type>,
```

### Step 2: Find All Hardcoded References **WITH CONTEXT VERIFICATION**

⚠️ **CRITICAL:** Multiple parameters share default values (e.g., `44100` used by BOTH working_sample_rate AND decoder_resume_hysteresis_samples). **NEVER bulk-replace without context verification.**

**See [migration_context_verification.md](migration_context_verification.md) for full procedure.**

```bash
# Get matches WITH CONTEXT (±10 lines)
rg "hardcoded_value" --type rust -g '!*test*.rs' -C 10 > matches.txt

# For EACH match:
# 1. Read context (check for DBD-PARAM tags, variable names, semantics)
# 2. Apply decision tree to classify which parameter
# 3. Document: file:line, value, parameter identified, reasoning
# 4. Replace ONLY after verification
```

**Create migration_log.md to track each replacement decision.**

Document all locations found and classification reasoning.

### Step 3: Replace Hardcoded Values
```rust
// BEFORE
let threshold = 22050;

// AFTER
let threshold = *wkmp_common::params::PARAMS.mixer_min_start_level.read().unwrap();
```

### Step 4: Remove Arc<RwLock> Field
Remove from struct definitions, update constructors.

### Step 5: Update Initialization Code
```rust
let value = load_from_db(&db_pool).await?;
*wkmp_common::params::PARAMS.parameter_name.write().unwrap() = value;
```

### Step 6: Full Test Suite
```bash
cargo test --workspace              # Must pass 100%
cargo test --test '*' --workspace   # Integration tests
cargo run -p wkmp-ap                # Manual smoke test
```

### Step 7: Single Commit
```bash
git add -A
git commit -m "Migrate parameter_name to GlobalParams

- Add parameter_name to wkmp-common::params::GlobalParams
- Replace N hardcoded references
- Remove Arc<RwLock> field from [structs]
- Initialize from database settings table
- All tests passing

[DBD-PARAM-XXX]"
```

**CRITICAL:** Use manual `git commit` (NOT `/commit` workflow) to avoid spamming change_history.md with 15 repetitive entries.

---

## Test Coverage Summary

**Total Tests:** 29 (21 unit, 6 integration, 2 manual)
**Coverage:** 100% (9/9 requirements, 15/15 parameters)

### Test Categories

| Category | Count | Purpose |
|----------|-------|---------|
| **Pre-Migration** | 6 | Verify GlobalParams structure, RwLock access, database init |
| **Per-Parameter** | 17 | Migration verification (grep + test suite) + validation |
| **Post-Migration** | 6 | Performance, API compatibility, manual review |

### Key Tests

**TC-U-001-01:** GlobalParams has all 15 fields ✅
**TC-U-003-01:** Grep finds zero hardcoded values (automated)
**TC-M-003-01:** Manual code review for hardcoded values (CRITICAL)
**TC-I-004-01:** Load all parameters from database
**TC-U-102-XX:** Validation for each of 15 parameters (range checking)
**TC-I-103-01:** Full test suite passes after each migration

**Detailed Tests:** See [02_test_specifications/test_index.md](02_test_specifications/test_index.md)
**Traceability:** See [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md)

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of [/plan workflow](.claude/commands/plan.md) for 7-step technical debt discovery process.

---

## Success Metrics

### Quantitative

- ✅ Zero hardcoded parameter values in production code
- ✅ Position timing accurate within ±100ms (stopwatch verified over 60s)
- ✅ All 29 tests pass (100% success rate)
- ✅ Performance: RwLock::read() < 10ns median
- ✅ Code reduction: 15 Arc<RwLock> fields removed

### Qualitative

- ✅ Single source of truth (PARAMS singleton)
- ✅ Clear documentation (doc comments with SPEC016 tags)
- ✅ Easy to add new parameters in future
- ✅ Consistent access pattern throughout codebase
- ✅ Developer confidence (full test coverage + manual verification)

---

## Dependencies

### Existing Documents (Read-Only)

| Document | Lines | Purpose |
|----------|-------|---------|
| SPEC016-mixer_and_buffers.md | ~800 | Parameter definitions and defaults |
| IMPL001-database_schema.md | ~400 | Settings table schema |
| GOV002-requirements_enumeration.md | ~150 | Requirement ID format |

### Integration Points

1. **Database Settings Table:** Query 15 parameter keys at startup
2. **wkmp-ap Startup:** Initialize after DB pool, before audio output
3. **Test Suite:** All existing tests must continue passing

### No External Dependencies

All required crates already in Cargo.toml:
- `once_cell` (Lazy static initialization)
- `std::sync::RwLock` (standard library)
- `sqlx` (database queries)

**Full Dependencies:** See [dependencies_map.md](dependencies_map.md)

---

## Constraints

### Technical

- No breaking API changes during migration
- Performance: RwLock::read() < 10ns, zero allocation
- Safety: No panics in audio hot path
- Rust idioms: Use std::sync, not external crates

### Process

- Incremental migration: One parameter at a time
- Commit discipline: Manual `git commit` per parameter
- Risk-based ordering: Tier 1 → Tier 2 → Tier 3
- Full testing: All tests pass after each parameter

### Timeline

- Estimated: 10-15 hours total
- Critical path: working_sample_rate (highest risk)
- No hard deadline: Quality over speed

---

## Risk Assessment

### High-Risk Parameters (Extra Caution)

**working_sample_rate (DBD-PARAM-020):**
- **Risk:** 10% timing error already occurred
- **Impact:** Position tracking, crossfade timing, marker events
- **Mitigation:** Migrate LAST, stopwatch verification, maximum testing

**audio_buffer_size (DBD-PARAM-110):**
- **Risk:** Audio glitches/dropouts
- **Impact:** Real-time callback timing
- **Mitigation:** Extensive manual testing, multiple buffer sizes

**mixer_check_interval_ms (DBD-PARAM-111):**
- **Risk:** Underruns or excess CPU
- **Impact:** Mixer wake timing
- **Mitigation:** Monitor CPU usage, verify no underruns

### Mitigation Strategies

1. **Incremental Migration:** One parameter at a time (immediate regression detection)
2. **Test After Each Step:** Catch problems immediately
3. **Low-Risk First:** Build confidence before timing-critical parameters
4. **Rollback Plan:** Each parameter = separate commit (easy revert)
5. **Manual Verification:** Stopwatch test for timing parameters

**Full Risk Assessment:** See specification lines 523-548

---

## Rollback Procedure

**If regression detected:**

1. **Stop immediately** - Do not proceed to next parameter
2. **Document issue** - File:line, symptoms, test failure
3. **Rollback:** `git reset --hard HEAD~1` (revert last parameter commit)
4. **Verify:** Re-run test suite, confirm previous state restored
5. **Analyze:** Investigate root cause, fix, re-test
6. **Resume:** Only continue after issue resolved

**Database:** No rollback needed (parameters still in settings table, singleton reverts to defaults)

---

## Next Steps

### Immediate (Ready Now)

1. ✅ Review this plan summary
2. ✅ Confirm approach and scope with stakeholders
3. ⏭️ Begin implementation: Create GlobalParams infrastructure
4. ⏭️ Run pre-migration tests (TC-U-001-*, TC-U-002-*, TC-I-004-*)
5. ⏭️ Migrate Parameter 1 (maximum_decode_streams - lowest risk)

### Implementation Sequence

**Setup Phase (1-2 hours):**
- Create `wkmp-common/src/params.rs` with GlobalParams struct
- Implement init_from_database() method
- Add module to wkmp-common/src/lib.rs
- Add initialization call to wkmp-ap/src/main.rs
- Run pre-migration tests

**Tier 1 (2.5-4 hours):**
1. maximum_decode_streams
2. decode_work_period
3. pause_decay_factor
4. pause_decay_floor
5. volume_level

**Tier 2 (2-3 hours):**
6. output_ringbuffer_size
7. playout_ringbuffer_size
8. playout_ringbuffer_headroom
9. mixer_min_start_level

**Tier 3 (4.5-7 hours):**
10. working_sample_rate ⚠️ CRITICAL
11. audio_buffer_size ⚠️ CRITICAL
12. mixer_check_interval_ms ⚠️ CRITICAL
13. decode_chunk_size
14. output_refill_period
15. decoder_resume_hysteresis_samples

**Verification Phase (30-60 minutes):**
- Final tests (TC-U-005-01, TC-U-101-*, TC-M-003-01, TC-M-104-01)
- Full regression test
- Manual smoke test

### After Implementation

1. Execute Phase 9: Post-Implementation Review (MANDATORY)
2. Generate technical debt report
3. Run all 29 tests, verify 100% pass
4. Verify traceability matrix 100% complete
5. Create final implementation report
6. Archive plan using `/archive-plan PLAN018`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- [requirements_index.md](requirements_index.md) - All requirements with priorities
- [scope_statement.md](scope_statement.md) - In/out scope, assumptions, constraints
- [dependencies_map.md](dependencies_map.md) - What exists, what's needed
- [01_specification_issues.md](01_specification_issues.md) - Phase 2 analysis

**Test Specifications:**
- [02_test_specifications/test_index.md](02_test_specifications/test_index.md) - All tests quick reference
- [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md) - Requirements ↔ Tests mapping
- [02_test_specifications/TC-*.md](02_test_specifications/) - Individual test specs (samples provided)

**For Implementation:**
- Read this summary (~480 lines)
- Read per-parameter migration steps (above)
- Read relevant test specs (~100-150 lines per parameter)
- **Total context:** ~650-800 lines per parameter migration

---

## Plan Status

**Phase 1-3 Status:** ✅ COMPLETE
**Phases 4-8 Status:** N/A (systematic refactoring, not novel implementation)
**Current Status:** Ready for Implementation
**Estimated Timeline:** 10-15 hours over 1-2 weeks

---

## Approval and Sign-Off

**Plan Created:** 2025-11-02
**Plan Status:** Ready for Implementation Review

**Next Action:** User review and approval to begin implementation

---

**Document Status:** PLAN SUMMARY COMPLETE
**Line Count:** ~480 lines (within 500-line target per CLAUDE.md)
**Context Window Optimized:** ✅ Yes
**Ready for Implementation:** ✅ Yes
