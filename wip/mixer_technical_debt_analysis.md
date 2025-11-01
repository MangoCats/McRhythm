# Mixer-Related Technical Debt Analysis

**Date:** 2025-11-01
**Status:** For Planning After PLAN014 Merge
**Context:** Analysis performed during pitch shift bug fix session

---

## Executive Summary

Comprehensive technical debt analysis identified 12 debt items related to mixer and playback subsystems. Analysis reconciled with PLAN014 mixer refactoring (currently at Sub-Increment 4b, testing complete).

**Key Decisions:**
1. **PLAN014 Testing:** Option B (Unit Test Validation) - Database not yet reliably available
2. **Engine Refactoring:** Option B (Defer to new plan) - Use PLAN016 or next available number
3. **Sequencing:** Complete PLAN014 first, then address functional/quality debt

---

## PLAN014 Completion Plan

### Immediate Next Steps

**Phase 5.4: Unit Test Validation** (Option B - 1-2 hours)
- Run: `cargo test -p wkmp-ap --test mixer_tests`
- Verify all 42 tests pass with Sub-Increment 4b integration
- Tests cover: marker storage, position tracking, EOF handling, crossfades, extended playback, state transitions

**Sub-Increment 4c: Remove Legacy Mixer** (30 minutes)
- Delete `wkmp-ap/src/playback/pipeline/mixer.rs` (1,069 lines)
- Remove from `wkmp-ap/src/playback/pipeline/mod.rs`
- Verify build: `cargo check -p wkmp-ap`

**Merge to Main** (1 hour)
- Create PR from `feature/plan014-sub-increment-4b`
- Code review
- Merge and update change_history.md
- Archive PLAN014 documentation

**Total Estimated Time:** 2.5-3.5 hours

---

## Technical Debt Items (Post-PLAN014)

### Category 1: Resolved by PLAN014

| Debt ID | Description | PLAN014 Resolution | Verification |
|---------|-------------|-------------------|--------------|
| DEBT-001 | Duplicate Mixer Implementations | Sub-Increment 4c removes legacy mixer | Verify after merge |
| DEBT-002 | Missing Features (underrun, position events, resume fade) | REQ-MIX-007, REQ-MIX-009 implemented | Tests verify |
| DEBT-010 | Stubbed Crossfade Logic | Marker system implemented | Sub-Increment 4b |
| DEBT-011 | Phase Tracking TODOs | Update comments to reflect marker system | 15 min cleanup |

**Action:** Verify these are resolved after PLAN014 merge.

---

### Category 2: High-Priority Functional Debt (Week 2)

**Address AFTER PLAN014 merge, in priority order:**

#### DEBT-012: Integration Test Compilation Errors (1-2 hours)
**Location:** Multiple integration test files
**Tests Affected:**
- `serial_decoder_tests.rs` (8 test functions)
- `decoder_pool_tests.rs` (3 test functions)
- `comprehensive_playback_test.rs` (3 test functions)
- `real_audio_playback_test.rs` (2 test functions)

**Error:** `DecoderWorker::new()` signature changed to require 4 arguments (added `working_sample_rate: Arc<RwLock<u32>>` parameter), but 16 test instantiations still pass only 3 arguments

**Root Cause:** SPEC016 integration added `working_sample_rate` parameter to DecoderWorker constructor, but integration tests not updated

**Action:**
1. Add `working_sample_rate` parameter to all 16 test DecoderWorker instantiations
2. Use `Arc::new(RwLock::new(44100))` or appropriate test value
3. Verify all integration tests compile and pass

**Plan:** PLAN008 REQ-DEBT-QUALITY-003-010/020
**Priority:** High - blocks `cargo test -p wkmp-ap` (full test suite)
**Estimated Time:** 1-2 hours

#### DEBT-003: Hardcoded Master Volume (30 min)
**Location:** `engine.rs:259`
```rust
let master_volume = 1.0; // TODO: Load from settings
```
**Action:** Load from `settings.master_volume`, default 1.0
**Plan:** PLAN008 (not explicitly enumerated)

#### DEBT-004: Hardcoded Position Event Interval (30 min)
**Location:** `engine.rs:2255, 3253`
```rust
let position_interval_ms = 100i64; // TODO: Get from settings
```
**Action:** Load from `settings.position_event_interval_ms`, default 100ms
**Plan:** PLAN008 (not explicitly enumerated)

#### DEBT-005: Missing Album UUID Metadata (2-3 hours)
**Location:** Multiple in `engine.rs` (lines 2193, 2906, 3202)
```rust
song_albums: Vec::new(), // TODO: Fetch album UUIDs from database
```
**Action:** Query database: passages→passage_albums→albums
**Plan:** PLAN008 REQ-DEBT-FUNC-004-010/020/030

#### DEBT-006: Missing Duration Tracking (2-3 hours)
**Location:** Multiple in `engine.rs` (lines 1018, 1030, 2473, 2485, 2580, 2592)
```rust
// Calculate duration_played **[REQ-DEBT-FUNC-002]**
```
**Action:** Track `passage_start_time` (already added in Sub-Increment 4b!), calculate elapsed
**Plan:** PLAN008 REQ-DEBT-FUNC-005-010/020/030/040
**Note:** `passage_start_time` field already exists in PlaybackEngine struct (Sub-Increment 4b)

**Subtotal:** 5.5-7.5 hours

---

### Category 3: Medium-Priority Quality Debt (Week 3)

**Address AFTER functional debt:**

#### DEBT-007: Incomplete Buffer Chain Telemetry (2-3 hours)
**Location:** `engine.rs:1447, 1456, 1465, 1480`
```rust
decoder_state: None,         // TODO: Query decoder pool status
source_sample_rate: Some(44100), // TODO: Get actual from decoder metadata
fade_stage: None,            // TODO: Get from decoder
started_at: None,            // TODO: Track start time
```
**Progress:** `resampler_algorithm` added today (2025-11-01)
**Remaining:** Get actual `source_sample_rate` from decoder, `decoder_state`, `fade_stage`, `started_at`
**Plan:** PLAN008 REQ-DEBT-FUNC-003-010/020/030/040

#### DEBT-008: Unused Mixer Variables (30 min)
**Location:** `engine.rs:208, 436, 437, 2037, 2042`
```rust
let mixer_min_start_level = mixer_min_start_level?; // unused
let batch_size_low = mixer_config.batch_size_low; // unused
let batch_size_optimal = mixer_config.batch_size_optimal; // unused
let fade_out_duration_samples = ...; // unused
let fade_in_duration_samples = ...; // unused
```
**Action:** Remove or use variables, fix 7 unused variable warnings
**Plan:** PLAN008 REQ-DEBT-QUALITY-001-010/020/030

#### DEBT-009: Large Monolithic Engine File (8-12 hours) - DEFERRED
**Location:** `engine.rs` (3,373 lines)
**Action:** Split into 3 modules: engine core, buffer management, event handling
**Plan:** PLAN008 REQ-DEBT-QUALITY-002-010/020/030
**Decision:** **Defer to PLAN016** (or next available plan number)
**Rationale:** Large refactoring, separate planning needed

**Subtotal:** 3-3.5 hours (excluding engine refactoring)

---

## Implementation Sequence

### Week 1: PLAN014 Completion (CRITICAL PATH)

**Total:** 2.5-3.5 hours

1. ✅ Phase 5.4: Unit Test Validation (Option B) - 1-2 hours
2. ✅ Sub-Increment 4c: Remove Legacy Mixer - 30 minutes
3. ✅ Merge to main - 1 hour

**Deliverable:** Mixer refactoring complete, 1,069 lines of dead code removed

---

### Week 2: High-Priority Functional Debt

**Total:** 7-9.5 hours

4. DEBT-012: Integration Test Compilation Errors - 1-2 hours
5. DEBT-003: Master Volume from Database - 30 min
6. DEBT-004: Position Interval from Database - 30 min
7. DEBT-005: Album UUID Metadata - 2-3 hours
8. DEBT-006: Duration Tracking - 2-3 hours

**Deliverable:** All functional gaps closed, full test suite operational

---

### Week 3: Medium-Priority Quality Debt

**Total:** 3-3.5 hours

9. DEBT-007: Complete Buffer Chain Telemetry - 2-3 hours
10. DEBT-008: Unused Variables Cleanup - 30 min
11. DEBT-009: Engine Refactoring - **DEFERRED TO PLAN016**

**Deliverable:** Code quality improved, warnings eliminated

---

## Risk Assessment

### High Risk: NOT Completing PLAN014 First
- **Impact:** Critical - 11.5 hours of mixer refactoring abandoned
- **Mitigation:** Complete PLAN014 Phase 5.4 testing FIRST

### Medium Risk: Functional Debt Before Merge
- **Impact:** Medium - Merge conflicts with active branch
- **Mitigation:** Merge PLAN014 before functional debt

### Low Risk: Documentation Gaps
- **Impact:** Low - Already resolved by PLAN014
- **Mitigation:** Verify after merge

---

## Success Metrics

### PLAN014 Completion
- ✅ All 42 mixer tests passing
- ✅ Legacy mixer removed (1,069 lines deleted)
- ✅ Branch merged to main
- ✅ PLAN014 archived

### Functional Debt (Week 2)
- ✅ Integration tests compile and pass
- ✅ Master volume configurable
- ✅ Position interval configurable
- ✅ Album metadata in events
- ✅ Duration tracking accurate

### Quality Debt (Week 3)
- ✅ Buffer chain telemetry complete
- ✅ Zero unused variable warnings
- ⚠️ Engine refactoring deferred to PLAN016

---

## Decisions Made (2025-11-01)

### Decision 1: PLAN014 Testing Approach
**Chosen:** **Option B - Unit Test Validation** (1-2 hours)
**Rationale:** Reliable database not yet available for full integration testing
**Action:** Run `cargo test -p wkmp-ap --test mixer_tests`, verify 42 tests pass

### Decision 2: Engine Refactoring (DEBT-009)
**Chosen:** **Option B - Defer to Separate Plan**
**Plan Number:** PLAN016 (or next available at time of planning)
**Rationale:** Large refactoring (8-12 hours), requires separate planning
**Note:** PLAN015 currently in progress on dev branch

---

## Dead Code Analysis

### Unused Mixer Functions (36 warnings)

**In `pipeline/mixer.rs` (legacy mixer - 1,069 lines):**
- 27 methods never used
- 3 structs never constructed
- 2 enum variants never constructed
- 11 struct fields never read

**Root Cause:** Entire `pipeline/mixer.rs` is inactive (replaced by `mixer.rs`)

**Resolution:** Delete in Sub-Increment 4c

---

## Context for Future Planning

### PLAN016: Engine Refactoring (Future)

**Scope:**
- Split `engine.rs` (3,373 lines) into 3 modules
- Modules: engine core, buffer management, event handling
- Maintain public API (internal refactor only)

**Effort:** 8-12 hours
**Plan:** PLAN008 REQ-DEBT-QUALITY-002-010/020/030
**Constraints:**
- Each module <1500 lines
- Public API unchanged
- Comprehensive testing after refactoring

**Prerequisites:**
- PLAN014 merged and stable
- Functional debt (DEBT-003 through DEBT-006) resolved
- Quality debt (DEBT-007, DEBT-008) resolved

---

## Documentation References

### Related Plans
- **PLAN008:** wkmp-ap Technical Debt Remediation (37 requirements)
- **PLAN014:** Mixer Refactoring (12 requirements, Sub-Increment 4b complete)
- **PLAN015:** Currently in progress on dev branch (unrelated)
- **PLAN016:** Future - Engine Refactoring (to be created)

### Related Specifications
- **SPEC016:** Decoder Buffer Design (mixer architectural separation)
- **SPEC002:** Crossfade Design (marker system, execution architecture)
- **ADR-001:** Mixer Refactoring Decision
- **ADR-002:** Event-Driven Position Tracking Architecture

---

## Today's Progress (2025-11-01)

### Completed Work
1. ✅ Fixed pitch shift bug (device-native sample rate implementation)
2. ✅ Enhanced Buffer Chain Monitor resample column
   - Added `resampler_algorithm` field to `BufferChainInfo`
   - Updated UI to display: `44100 Hz → 48000 Hz [Septic polynomial]`
   - Partially addressed DEBT-007
3. ✅ Conducted comprehensive mixer technical debt review
4. ✅ Reconciled technical debt with PLAN014 status
5. ✅ Created this analysis document

### Files Modified Today
- `wkmp-common/src/events.rs` - Added `resampler_algorithm` field
- `wkmp-ap/src/playback/engine.rs` - Populated resampling telemetry
- `wkmp-ap/src/api/developer_ui.html` - Enhanced resample column display
- `docs/SPEC016-decoder_buffer_design.md` - Updated working_sample_rate docs
- `docs/SPEC020-developer_ui_design.md` - Updated resample column spec

---

## Next Session Action Items

### Immediate (PLAN014 Completion)
1. ✅ Run unit tests: `cargo test -p wkmp-ap --test mixer_integration` - ALL 42 PASSED
2. Execute Sub-Increment 4c (remove legacy mixer)
3. Create PR and merge to main
4. Archive PLAN014

### Week 2 (Functional Debt)
5. DEBT-012: Fix integration test compilation errors
6. DEBT-003: Master volume from database
7. DEBT-004: Position interval from database
8. DEBT-005: Album UUID metadata
9. DEBT-006: Duration tracking

### Week 3 (Quality Debt)
10. DEBT-007: Complete buffer chain telemetry
11. DEBT-008: Cleanup unused variables
12. Defer DEBT-009 to PLAN016

---

**Analysis Complete**
**Date:** 2025-11-01 (updated with DEBT-012)
**Estimated Total Remaining Effort:** 12.5-17 hours (excluding engine refactoring)
**Critical Path:** PLAN014 merge (1-1.5 hours remaining)
