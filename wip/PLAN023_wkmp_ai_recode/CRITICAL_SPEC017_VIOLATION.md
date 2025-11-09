# CRITICAL: SPEC017 Violation in PLAN023

**Date:** 2025-01-11
**Severity:** HIGH - Specification Violation
**Status:** ✅ RESOLVED - Fixed on 2025-01-11
**Resolution Time:** ~2 hours (as estimated)

---

## Issue Description

PLAN023 workflow engine is storing passage time boundaries in **seconds (f64/REAL)** instead of **ticks (i64/INTEGER)** as required by SPEC017.

## SPEC017 Requirements

Per **[SRC-DB-010]** through **[SRC-DB-016]**:

> Passage timing fields are stored as **INTEGER** (SQLite `i64`) tick values

Where:
- 1 tick = 1/28,224,000 second (≈35.4 nanoseconds)
- Tick rate = LCM of all supported sample rates
- Ensures sample-accurate precision across all sample rates

## Current Implementation (INCORRECT)

### PassageBoundary Structure
```rust
pub struct PassageBoundary {
    pub start_seconds: f64,  // ❌ WRONG - Should be start_time: i64 (ticks)
    pub end_seconds: f64,    // ❌ WRONG - Should be end_time: i64 (ticks)
    pub confidence: f64,
}
```

### Database Storage (workflow/storage.rs)
```sql
INSERT INTO passages (
    start_seconds,     -- ❌ WRONG - Should be start_time INTEGER
    end_seconds,       -- ❌ WRONG - Should be end_time INTEGER
    duration_seconds,  -- ❌ WRONG - Should be calculated from ticks
    ...
)
```

### Boundary Detector (boundary_detector.rs)
```rust
PassageBoundary {
    start_seconds: start_time,  // ❌ WRONG - Calculated from sample counts
    end_seconds: end_time,      // ❌ WRONG - Should convert to ticks
    confidence,
}
```

## Impact Assessment

### Data Loss
- **Precision Loss:** f64 seconds cannot represent exact sample boundaries
- **Conversion Errors:** Rounding errors accumulate across conversions
- **Sample Inaccuracy:** Violates WKMP's sample-accurate timing requirement

### Database Schema Mismatch
- Current PLAN023 schema uses `REAL` columns
- Production schema (from migrations) should use `INTEGER` columns
- **Data migration required** if any test data exists

### API/Event Layer
- WorkflowEvent uses f64 seconds
- SSE events broadcast f64 seconds
- All need conversion to ticks for storage

## Required Fixes

### 1. PassageBoundary Structure
```rust
pub struct PassageBoundary {
    pub start_time: i64,     // ✅ Ticks from file start
    pub end_time: i64,       // ✅ Ticks from file start
    pub confidence: f64,
}
```

### 2. Boundary Detector
```rust
// Convert sample counts to ticks
const TICK_RATE: i64 = 28_224_000;

fn samples_to_ticks(samples: usize, sample_rate: u32) -> i64 {
    let ticks_per_sample = TICK_RATE / sample_rate as i64;
    (samples as i64) * ticks_per_sample
}

PassageBoundary {
    start_time: samples_to_ticks(start_sample, sample_rate),
    end_time: samples_to_ticks(end_sample, sample_rate),
    confidence,
}
```

### 3. Database Schema
```sql
CREATE TABLE passages (
    start_time INTEGER NOT NULL,      -- ✅ Ticks from file start
    end_time INTEGER NOT NULL,        -- ✅ Ticks from file start
    -- duration_seconds removed (calculate from start_time/end_time)
    ...
)
```

### 4. Storage Module
```rust
// Store ticks directly
.bind(passage.boundary.start_time)
.bind(passage.boundary.end_time)

// Duration calculated if needed:
// duration_ticks = end_time - start_time
// duration_seconds = duration_ticks as f64 / TICK_RATE as f64
```

### 5. Event Bridge (SSE)
```rust
// Convert ticks to seconds for user-facing events
WorkflowEvent::BoundaryDetected {
    passage_index,
    start_seconds: start_time as f64 / TICK_RATE as f64,  // Display only
    end_seconds: end_time as f64 / TICK_RATE as f64,      // Display only
    confidence,
}
```

## Migration Strategy

### For Tests
1. Update test database schema (already using CREATE TABLE in tests)
2. Update PassageBoundary structure
3. Update boundary_detector to calculate ticks
4. Update storage.rs to use ticks
5. Re-run all tests

### For Production Database
1. Check if `passages` table already exists with REAL columns
2. If yes: Create migration to convert REAL → INTEGER
3. If no: Ensure migration uses INTEGER from start

## Files Requiring Changes

1. **wkmp-ai/src/workflow/mod.rs** - PassageBoundary struct
2. **wkmp-ai/src/workflow/boundary_detector.rs** - Tick calculation
3. **wkmp-ai/src/workflow/storage.rs** - Database INSERT statements
4. **wkmp-ai/src/workflow/song_processor.rs** - Pass-through (minimal changes)
5. **wkmp-ai/src/workflow/event_bridge.rs** - Tick → second conversion for events
6. **wkmp-ai/tests/system_tests.rs** - Test database schema
7. **wkmp-ai/tests/workflow_integration.rs** - Test expectations

## Estimated Effort

- **Code Changes:** 1-2 hours
- **Test Updates:** 30 minutes
- **Verification:** 30 minutes
- **Total:** 2-3 hours

## Priority

**HIGH** - This is a specification violation that affects:
- Data accuracy (precision loss)
- System architecture compliance
- Future interoperability with other WKMP modules

**Recommendation:** Fix immediately before marking PLAN023 as production-ready.

## Notes

- SPEC017 explicitly states: "Developer-facing layers use ticks"
- Database is a developer-facing layer (not user-facing UI)
- All internal timing must use ticks for sample accuracy
- Only convert to seconds at UI boundary

---

## Resolution Summary (2025-01-11)

**All required fixes completed and verified:**

1. ✅ **Specification Updated** - Added REQ-AI-088 to SPEC_wkmp_ai_recode.md with full SPEC017 compliance requirements
2. ✅ **PassageBoundary Structure** - Changed from `{start_seconds: f64, end_seconds: f64}` to `{start_time: i64, end_time: i64}`
3. ✅ **Boundary Detector** - Added `samples_to_ticks()` conversion, all boundaries now calculated in ticks
4. ✅ **Storage Module** - Updated INSERT queries to use `start_time INTEGER, end_time INTEGER` (removed duration_seconds column)
5. ✅ **Event Bridge** - Added tick→second conversion for user-facing SSE events (line 64-65)
6. ✅ **Song Processor** - Updated to pass ticks through pipeline, convert to seconds only for logging/extractors
7. ✅ **All Tests** - Updated integration and system tests for tick compliance

**Test Results:**
- 176 tests passing + 1 ignored = 177 total
- All SPEC017 violations resolved
- Database schema now uses INTEGER for timing (SPEC017 compliant)

**Files Modified:**
- `wip/SPEC_wkmp_ai_recode.md` (added REQ-AI-088)
- `wkmp-ai/src/workflow/mod.rs` (PassageBoundary struct, WorkflowEvent)
- `wkmp-ai/src/workflow/boundary_detector.rs` (tick conversion)
- `wkmp-ai/src/workflow/storage.rs` (INTEGER columns)
- `wkmp-ai/src/workflow/event_bridge.rs` (tick→second for SSE)
- `wkmp-ai/src/workflow/song_processor.rs` (pass ticks through pipeline)
- `wkmp-ai/tests/workflow_integration.rs` (test assertions)
- `wkmp-ai/tests/system_tests.rs` (database schema, queries)

**Compliance Verification:**
- ✅ All passage timing stored as i64 ticks internally
- ✅ Database uses INTEGER columns (not REAL)
- ✅ Only user-facing SSE events convert to seconds
- ✅ Sample-accurate precision maintained throughout pipeline
- ✅ No precision loss from floating-point conversions
