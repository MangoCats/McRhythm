# Tier 1 Approval: Tick-Based Timing Migration

**Approval ID:** T1-TIMING-001
**Date:** 2025-10-19
**Status:** ✅ APPROVED
**Approved By:** User (Technical Lead)

---

## Decision

**APPROVED:** Migrate database timing fields from REAL (floating-point seconds) to INTEGER (ticks at 28,224,000 Hz) as specified in SPEC017-sample_rate_conversion.md.

---

## Specification Reference

**Authoritative Source:** [SPEC017-sample_rate_conversion.md](../SPEC017-sample_rate_conversion.md)

**Key Requirements:**
- **[SRC-DB-011]** `start_time` - INTEGER ticks from file start
- **[SRC-DB-012]** `end_time` - INTEGER ticks from file start
- **[SRC-DB-013]** `fade_in_point` - INTEGER ticks from file start
- **[SRC-DB-014]** `fade_out_point` - INTEGER ticks from file start
- **[SRC-DB-015]** `lead_in_point` - INTEGER ticks from file start
- **[SRC-DB-016]** `lead_out_point` - INTEGER ticks from file start

**Data Type:** `i64` (SQLite INTEGER)
**Tick Rate:** 28,224,000 ticks per second
**Conversion:** `ticks = seconds × 28,224,000`

---

## Benefits (Rationale for Approval)

### 1. Sample-Accurate Precision
- Eliminates floating-point rounding errors
- Guarantees exact repeatability of passage playback
- Every passage plays at identical sample boundaries every time

### 2. Cross-Sample-Rate Compatibility
- Any source sample rate converts exactly to ticks with zero error
- 44.1kHz, 48kHz, 96kHz, 192kHz all supported perfectly
- Future-proof for arbitrary sample rates

### 3. Industry Best Practice
- Pro Tools, Logic Pro, Ableton Live all use tick-based timing
- Digital audio workstations require sample-accurate precision
- WKMP achieves professional-grade timing accuracy

### 4. Eliminates Bug Class
- No accumulated rounding errors over long passages
- No precision loss in crossfade calculations
- No floating-point comparison issues (==, <, > always exact)

---

## Implementation Plan

### Phase 1: Database Schema Migration (2-3 days)

**File:** `migrations/NNN-tick_based_timing.sql`

```sql
-- Step 1: Add new INTEGER columns
ALTER TABLE passages ADD COLUMN start_time_ticks INTEGER;
ALTER TABLE passages ADD COLUMN end_time_ticks INTEGER;
ALTER TABLE passages ADD COLUMN fade_in_point_ticks INTEGER;
ALTER TABLE passages ADD COLUMN fade_out_point_ticks INTEGER;
ALTER TABLE passages ADD COLUMN lead_in_point_ticks INTEGER;
ALTER TABLE passages ADD COLUMN lead_out_point_ticks INTEGER;

-- Step 2: Migrate existing data (seconds to ticks)
UPDATE passages SET start_time_ticks = CAST(start_time * 28224000 AS INTEGER) WHERE start_time IS NOT NULL;
UPDATE passages SET end_time_ticks = CAST(end_time * 28224000 AS INTEGER) WHERE end_time IS NOT NULL;
UPDATE passages SET fade_in_point_ticks = CAST(fade_in_point * 28224000 AS INTEGER) WHERE fade_in_point IS NOT NULL;
UPDATE passages SET fade_out_point_ticks = CAST(fade_out_point * 28224000 AS INTEGER) WHERE fade_out_point IS NOT NULL;
UPDATE passages SET lead_in_point_ticks = CAST(lead_in_point * 28224000 AS INTEGER) WHERE lead_in_point IS NOT NULL;
UPDATE passages SET lead_out_point_ticks = CAST(lead_out_point * 28224000 AS INTEGER) WHERE lead_out_point IS NOT NULL;

-- Step 3: Rename columns (drop old, rename new)
ALTER TABLE passages DROP COLUMN start_time;
ALTER TABLE passages DROP COLUMN end_time;
ALTER TABLE passages DROP COLUMN fade_in_point;
ALTER TABLE passages DROP COLUMN fade_out_point;
ALTER TABLE passages DROP COLUMN lead_in_point;
ALTER TABLE passages DROP COLUMN lead_out_point;

ALTER TABLE passages RENAME COLUMN start_time_ticks TO start_time;
ALTER TABLE passages RENAME COLUMN end_time_ticks TO end_time;
ALTER TABLE passages RENAME COLUMN fade_in_point_ticks TO fade_in_point;
ALTER TABLE passages RENAME COLUMN fade_out_point_ticks TO fade_out_point;
ALTER TABLE passages RENAME COLUMN lead_in_point_ticks TO lead_in_point;
ALTER TABLE passages RENAME COLUMN lead_out_point_ticks TO lead_out_point;

-- Step 4: Update schema_version
UPDATE schema_version SET version = version + 1;
```

**Testing:**
- Verify migration script on test database
- Confirm data accuracy (ticks = seconds × 28,224,000)
- Test rollback script

---

### Phase 2: API Conversion Layer (2-3 days)

**API Format:** Unsigned integer milliseconds (human-readable)
**Database Format:** Signed integer ticks (sample-accurate)
**Conversion:** `ticks = milliseconds × 28,224`

**Files Affected:**
- `wkmp-ap/src/api/handlers.rs` (playback endpoints)
- `wkmp-common/src/models/passage.rs` (database model)
- `wkmp-common/src/conversions.rs` (new module for tick conversions)

**Example API Handler:**
```rust
// POST /playback/enqueue
#[derive(Deserialize)]
struct EnqueueRequest {
    file_path: String,
    start_time_ms: Option<u64>,      // API: milliseconds
    end_time_ms: Option<u64>,
    fade_in_point_ms: Option<u64>,
    // ... other fields
}

fn enqueue_passage(req: EnqueueRequest) -> Result<()> {
    let passage = Passage {
        file_path: req.file_path,
        start_time: req.start_time_ms.map(ms_to_ticks),  // Convert to ticks
        end_time: req.end_time_ms.map(ms_to_ticks),
        fade_in_point: req.fade_in_point_ms.map(ms_to_ticks),
        // ...
    };
    // ...
}

fn ms_to_ticks(ms: u64) -> i64 {
    (ms as i64) * 28_224
}

fn ticks_to_ms(ticks: i64) -> u64 {
    ((ticks + 14_112) / 28_224) as u64  // Round to nearest ms
}
```

**Testing:**
- Unit tests for conversion functions
- API integration tests (ms → ticks → ms round-trip)
- Boundary condition tests (0, MAX_VALUE)

---

### Phase 3: Playback Engine Updates (3-4 days)

**Files Affected (~15 files, 500-800 LOC):**
1. `wkmp-ap/src/playback/controller.rs` - Tick-based position tracking
2. `wkmp-ap/src/playback/buffer_manager.rs` - Tick-to-sample conversion
3. `wkmp-ap/src/playback/crossfade.rs` - Tick-based crossfade calculations
4. `wkmp-ap/src/playback/mixer.rs` - Tick timing references
5. `wkmp-ap/src/audio/decoder.rs` - Start/end time in ticks
6. `wkmp-ap/src/audio/fade.rs` - Fade point calculations
7. `wkmp-common/src/models/passage.rs` - Database model
8. `wkmp-common/src/conversions.rs` - Conversion utilities (new)
9. `wkmp-ui/src/api/proxy.rs` - API format handling
10. Others (event handlers, SSE updates, logging)

**Key Changes:**
- Replace `f64` seconds with `i64` ticks for all timing
- Add `ticks_to_samples()` at decoder-buffer boundary
- Update crossfade timing calculations to use tick arithmetic
- Convert position events to milliseconds for SSE/API

**Testing:**
- Unit tests for tick arithmetic
- Integration tests for crossfade accuracy
- Performance tests (verify no regression)
- Sample-accuracy validation (frame-perfect playback)

---

### Phase 4: Documentation Updates (1 day)

**Unblock 3 Edits:**
1. **EDIT-IMPL001-001:** Update `passages` table schema in IMPL001-database_schema.md
2. **EDIT-SPEC002-003:** Update timing storage format in SPEC002-crossfade.md
3. **EDIT-SPEC002-007:** Add tick conversion references
4. **EDIT-SPEC002-008:** Update timing precision section

**Additional Updates:**
- Update API documentation with ms ↔ ticks conversion
- Update migration guide with schema change details
- Add examples of tick-based timing usage

---

## Implementation Timeline

### Week 1: Database Migration
- **Days 1-2:** Write migration script, test on dev database
- **Day 3:** Review and rollback testing

### Week 2: Code Updates
- **Days 1-2:** API conversion layer
- **Days 3-5:** Playback engine updates

### Week 3: Testing & Documentation
- **Days 1-2:** Integration testing, performance validation
- **Day 3:** Documentation updates, code review
- **Days 4-5:** Final validation, deployment preparation

**Total Effort:** 15 developer days (3 weeks)

---

## Risk Mitigation

### Risk 1: Migration Data Loss
**Mitigation:**
- Database backup before migration
- Rollback script tested and ready
- Verification query: `SELECT COUNT(*) WHERE start_time_ticks != CAST(start_time_backup * 28224000 AS INTEGER)`

### Risk 2: API Breaking Changes
**Mitigation:**
- API format unchanged (still milliseconds)
- Conversion layer transparent to clients
- Version header can indicate tick support

### Risk 3: Performance Regression
**Mitigation:**
- Integer arithmetic faster than floating-point
- Benchmark before/after
- No expected performance impact (likely improvement)

### Risk 4: Precision Loss in Conversion
**Mitigation:**
- Round to nearest millisecond when converting ticks → ms for API
- Internal precision always maintained (ticks)
- Document rounding behavior

---

## Rollback Plan

If critical issues discovered post-deployment:

```sql
-- Emergency rollback: ticks back to seconds
ALTER TABLE passages ADD COLUMN start_time_seconds REAL;
UPDATE passages SET start_time_seconds = CAST(start_time AS REAL) / 28224000.0;
-- (repeat for all 6 fields)
-- Rename columns back
-- Restore schema_version
```

**Conditions for Rollback:**
- Data corruption detected
- Unrecoverable playback errors
- Performance degradation >20%

---

## Success Criteria

✅ **Database Migration:**
- All existing timing data converted accurately
- No data loss or corruption
- Migration completes in <1 minute

✅ **API Compatibility:**
- All existing API clients work without changes
- Millisecond precision maintained for API
- Round-trip conversion accurate (ms → ticks → ms)

✅ **Playback Accuracy:**
- Sample-accurate timing verified (frame-perfect)
- Crossfade calculations produce identical results
- No floating-point comparison bugs

✅ **Performance:**
- No regression in playback latency
- No regression in crossfade CPU usage
- Migration time acceptable for production deployment

✅ **Documentation:**
- All 3 blocked edits applied
- Database schema docs updated
- API docs reflect ms ↔ ticks conversion

---

## Next Steps

1. ✅ **Approval granted** (this document)
2. **Create implementation branch:** `feature/tick-based-timing`
3. **Write migration script:** `migrations/NNN-tick_based_timing.sql`
4. **Implement conversion utilities:** `wkmp-common/src/conversions.rs`
5. **Update playback engine:** Files listed in Phase 3
6. **Execute 3 blocked documentation edits**
7. **Integration testing:** Full system validation
8. **Code review:** Technical lead approval
9. **Deploy to staging:** User acceptance testing
10. **Production deployment:** With rollback plan ready

---

## Impact on Documentation Workflow

### Previously Blocked Edits (Now Unblocked)

**EDIT-IMPL001-001:** Update passages table schema
```markdown
### `passages` Table

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| start_time | INTEGER | YES | Passage start time in ticks (28,224,000/sec) - see [SRC-DB-011] |
| end_time | INTEGER | YES | Passage end time in ticks - see [SRC-DB-012] |
| fade_in_point | INTEGER | YES | Fade-in completion point in ticks - see [SRC-DB-013] |
| fade_out_point | INTEGER | YES | Fade-out start point in ticks - see [SRC-DB-014] |
| lead_in_point | INTEGER | YES | Lead-in end point in ticks - see [SRC-DB-015] |
| lead_out_point | INTEGER | YES | Lead-out start point in ticks - see [SRC-DB-016] |

**Tick Conversion:** 28,224 ticks per millisecond. See [SPEC017-sample_rate_conversion.md](SPEC017-sample_rate_conversion.md) for complete tick system specification.
```

**EDIT-SPEC002-003:** Update timing storage format reference
```markdown
[XFD-DB-010] Passage table stores timing fields as **INTEGER ticks** (see [SRC-DB-010] through [SRC-DB-016] in SPEC017-sample_rate_conversion.md).
```

**EDIT-SPEC002-007 & EDIT-SPEC002-008:** Add tick conversion and precision sections

### Edit Plan Status Update

- **Total edits:** 137
- **Previously READY:** 134
- **Previously BLOCKED:** 3
- **NOW READY:** 137 (100%)

**All edits can now proceed to execution.**

---

## Reference Documents

- **Approval Request:** [phase2-tier1-approvals-needed.md](phase2-tier1-approvals-needed.md)
- **Authoritative Spec:** [SPEC017-sample_rate_conversion.md](../SPEC017-sample_rate_conversion.md)
- **Database Schema:** [IMPL001-database_schema.md](../IMPL001-database_schema.md) (to be updated)
- **Crossfade Design:** [SPEC002-crossfade.md](../SPEC002-crossfade.md) (to be updated)
- **Implementation Plan:** [phase3-implementation-changes.json](phase3-implementation-changes.json) (IMPL-002 entry)

---

**Approval Status:** ✅ APPROVED
**Implementation Status:** Ready to begin
**Blocking Status:** All 137 documentation edits now unblocked

---

End of Approval Document
