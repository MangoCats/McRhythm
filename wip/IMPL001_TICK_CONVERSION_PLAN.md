# IMPL001 Tick Conversion Plan

**Date:** 2025-10-26
**Issue:** IMPL001 database schema uses milliseconds for audio timing fields, violating approved SPEC017 architecture
**User Concern:** Should IMPL001 be corrected before implementation proceeds?
**Answer:** **YES - Critical alignment issue**

---

## Problem Statement

**Approved SPEC017 architecture (Fix #1):**
- Developer-facing layers (API, Database, Developer UI, SSE) use **TICKS (i64)**
- User-facing layers (end-user UI) use **SECONDS (f64)**
- Database is developer-facing → should use ticks

**Current IMPL001 state:**
- `passages` table: ✅ Uses ticks (correct)
- `passage_songs` table: ❌ Uses milliseconds (WRONG)
- `song_play_history` table: ⚠️ Mixed (some should be ticks)
- `settings` table: ⚠️ Mixed (some should be ticks)

**Inconsistency:** Schema spec doesn't align with architecture spec

---

## Document Hierarchy Impact

Per GOV001 document hierarchy:

```
Tier 2: SPEC017 (Architecture - UPDATED ✅)
   ↓ (information flows down)
Tier 3: IMPL001 (Database Schema - NEEDS UPDATE ❌)
   ↓ (information flows down)
Tier 4: GUIDE002 (Implementation Guide - MAY NEED UPDATE ⚠️)
   ↓ (information flows down)
Implementation: config.rs, db models, etc. - BLOCKED ⛔
```

**Proper process:** Update Tier 3 to align with Tier 2 BEFORE implementing code

---

## Fields Requiring Conversion

### 1. `passage_songs` Table (Song Boundaries)

**Current Schema:**
```sql
CREATE TABLE passage_songs (
    passage_id TEXT NOT NULL REFERENCES passages(guid) ON DELETE CASCADE,
    song_id TEXT NOT NULL REFERENCES songs(guid) ON DELETE CASCADE,
    start_time_ms INTEGER NOT NULL,  -- ❌ WRONG
    end_time_ms INTEGER NOT NULL,    -- ❌ WRONG
    ...
);
```

**Corrected Schema:**
```sql
CREATE TABLE passage_songs (
    passage_id TEXT NOT NULL REFERENCES passages(guid) ON DELETE CASCADE,
    song_id TEXT NOT NULL REFERENCES songs(guid) ON DELETE CASCADE,
    start_time_ticks INTEGER NOT NULL,  -- ✅ CORRECT
    end_time_ticks INTEGER NOT NULL,    -- ✅ CORRECT
    ...
);
```

**Rationale:** Song boundaries are audio timing (sample-accurate precision required)

**Constraints to update:**
- CHECK: `start_time_ticks >= 0`
- CHECK: `end_time_ticks > start_time_ticks`

**Indexes to update:**
- `idx_passage_songs_timing` on `(passage_id, start_time_ticks)`

---

### 2. `song_play_history` Table (Playback Duration)

**Current Schema:**
```sql
CREATE TABLE song_play_history (
    start_timestamp_ms INTEGER NOT NULL,  -- ✅ OK (Unix timestamp)
    stop_timestamp_ms INTEGER NOT NULL,   -- ✅ OK (Unix timestamp)
    audio_played_ms INTEGER NOT NULL,     -- ❌ WRONG (audio duration)
    pause_duration_ms INTEGER NOT NULL,   -- ❌ WRONG (audio duration)
    ...
);
```

**Corrected Schema:**
```sql
CREATE TABLE song_play_history (
    start_timestamp_ms INTEGER NOT NULL,     -- ✅ OK (Unix timestamp)
    stop_timestamp_ms INTEGER NOT NULL,      -- ✅ OK (Unix timestamp)
    audio_played_ticks INTEGER NOT NULL,     -- ✅ CORRECT (audio duration)
    pause_duration_ticks INTEGER NOT NULL,   -- ✅ CORRECT (audio duration)
    ...
);
```

**Rationale:**
- Timestamps are system time (milliseconds OK)
- Audio durations are audio timing (ticks required)

**Documentation to update:**
- Line 559: Skip detection formula needs tick conversion
- Line 560: Rewind detection formula needs tick conversion

---

### 3. `settings` Table (Playback Position)

**Current Schema:**
```
| Key | Type | Default | Purpose |
|-----|------|---------|---------|
| last_played_position | INTEGER (ms) | 0 | Position in milliseconds | ❌ WRONG
```

**Corrected Schema:**
```
| Key | Type | Default | Purpose |
|-----|------|---------|---------|
| last_played_position_ticks | INTEGER | 0 | Position in ticks (audio timing) | ✅ CORRECT
```

**Rationale:** Playback position is audio timing (sample-accurate precision)

---

## Fields That Stay Milliseconds

**These are SYSTEM TIMING, not audio timing:**

### Settings Table (System Intervals)
```
position_event_interval_ms         ✅ OK (event emission interval)
playback_progress_interval_ms      ✅ OK (event emission interval)
backup_interval_ms                 ✅ OK (system timing)
backup_minimum_interval_ms         ✅ OK (system timing)
last_backup_timestamp_ms           ✅ OK (Unix timestamp)
volume_fade_update_period          ✅ OK (UI update period)
http_request_timeout_ms            ✅ OK (HTTP timeout)
http_keepalive_timeout_ms          ✅ OK (HTTP timeout)
```

**Rationale:** These control WHEN system events occur, not audio playback precision

---

## Migration Strategy

### For Existing Databases

**Option A: Schema Migration (Recommended)**
```sql
-- Migration script (example for passage_songs)
ALTER TABLE passage_songs RENAME COLUMN start_time_ms TO start_time_ms_old;
ALTER TABLE passage_songs RENAME COLUMN end_time_ms TO end_time_ms_old;

ALTER TABLE passage_songs ADD COLUMN start_time_ticks INTEGER;
ALTER TABLE passage_songs ADD COLUMN end_time_ticks INTEGER;

-- Convert milliseconds to ticks (ms * 28224)
UPDATE passage_songs
SET start_time_ticks = start_time_ms_old * 28224,
    end_time_ticks = end_time_ms_old * 28224;

-- Add NOT NULL constraint after data populated
-- Drop old columns
ALTER TABLE passage_songs DROP COLUMN start_time_ms_old;
ALTER TABLE passage_songs DROP COLUMN end_time_ms_old;
```

**Option B: Fresh Database (Simpler for Development)**
- Drop existing database
- Create new database with corrected schema
- Re-import music library

---

## Conversion Factors

**Milliseconds to Ticks:**
```
ticks = milliseconds * 28,224
```

**Why 28,224?**
- Tick rate: 28,224,000 Hz (SPEC017 SRC-TICK-020)
- 1 second = 28,224,000 ticks
- 1 millisecond = 28,224 ticks
- Exact conversion (no rounding)

**Example:**
```
5 seconds = 5000 ms = 141,120,000 ticks
Position 2.5s = 2500 ms = 70,560,000 ticks
```

---

## Documentation Updates Required

### 1. IMPL001-database_schema.md

**Sections to update:**
- ✅ Line 369-381: `passage_songs` table definition
- ✅ Line 546-562: `song_play_history` table definition
- ✅ Line 757: `settings.last_played_position` definition
- ✅ All constraint checks using old field names
- ✅ All index definitions using old field names
- ✅ All documentation examples/formulas

**Estimated changes:** ~50 lines

---

### 2. GUIDE002-wkmp_ap_re_implementation_guide.md

**Check for:**
- References to millisecond fields in passages/songs
- API examples using milliseconds
- Implementation guidance for duration calculations

**Action:** Review after IMPL001 updated

---

### 3. PLAN005 (Implementation Plan)

**Check for:**
- Test specifications referencing millisecond fields
- Requirements expecting millisecond precision
- Traceability matrix field mappings

**Action:** Review test specs in `02_test_specifications/`

---

## Implementation Impact

**Code Already Written:**
- ✅ `error.rs` - No impact (no database fields)
- ⚠️ `config_new.rs` - Uses milliseconds in RuntimeSettings
  - **Needs update:** Settings that should be ticks

**Code NOT Yet Written:**
- Database models (Tier 3 spec must be correct first)
- API handlers (Tier 3 spec must be correct first)
- Audio pipeline (Tier 3 spec must be correct first)

**Good news:** Caught early! Only 1 file needs minor updates.

---

## Approval Required

**Question for user:**

1. **Proceed with IMPL001 tick conversion?**
   - Update passage_songs table (2 fields)
   - Update song_play_history table (2 fields)
   - Update settings table (1 field)
   - Update all related constraints, indexes, documentation

2. **Review GUIDE002 after IMPL001 updated?**
   - Check for downstream impacts

3. **Update PLAN005 test specifications?**
   - Ensure tests expect ticks, not milliseconds

4. **Then resume Phase 1 implementation?**
   - With corrected specifications

---

## Estimated Effort

**IMPL001 updates:** 2-3 hours
- Update table definitions
- Update constraints
- Update indexes
- Update documentation examples
- Update formulas

**GUIDE002 review:** 1 hour
- Search for millisecond references
- Update if needed

**PLAN005 review:** 30 minutes
- Check test specs for field name assumptions

**Total:** ~4 hours to ensure consistency

**vs. Implementation without fixes:**
- Technical debt in database schema
- Conversion code scattered throughout application
- Potential precision loss bugs
- Refactoring cost: Days/weeks

**Verdict:** **Invest 4 hours now, save days/weeks later**

---

## Recommendation

✅ **YES - Update IMPL001 and downstream documents BEFORE proceeding with implementation**

**Rationale:**
1. Tier 2 (SPEC017) architecture approved with ticks
2. Tier 3 (IMPL001) must align with Tier 2
3. Implementing against misaligned spec creates technical debt
4. Caught early (only 1 code file affected)
5. ~4 hour investment prevents future refactoring

**Next steps:**
1. User approves tick conversion plan
2. Update IMPL001 database schema
3. Review GUIDE002 for impacts
4. Update PLAN005 test specs if needed
5. Update `config_new.rs` for settings fields
6. Resume Phase 1 implementation

---

**Status:** ⏳ AWAITING USER APPROVAL
**Created:** 2025-10-26
**User Concern:** Absolutely valid - critical alignment issue identified
