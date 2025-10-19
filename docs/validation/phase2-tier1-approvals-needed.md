# Tier 1 Approval Requests - Phase 2 Design Improvements

**Generated:** 2025-10-19
**Agent:** Agent 3B: Design Improvement Classifier
**Status:** PENDING APPROVAL

---

## Overview

During Phase 2 design improvement analysis, **1 critical improvement** was identified that requires Tier 1 (Requirements) approval because it represents a fundamental change to how timing data is stored and represented in the system.

All other improvements (10 of 11) are Tier 2 design decisions that can be resolved through documentation updates without requirements changes.

---

## Tier 1 Approval Request: Tick-Based Timing System

### T1-TIMING-001: Database Timing Format Change

**Priority:** CRITICAL - BLOCKING
**Approval Needed From:** Technical lead, database architect, product owner

#### Current Requirement (Implied)

**Documents:** REQ001-requirements.md, IMPL001-database_schema.md (implied from historical context)

**Current Representation:**
- Passage timing stored as **REAL** (floating-point seconds)
- Example: `start_time = 120.5` (120.5 seconds)
- Issues discovered:
  - Cumulative rounding errors violate sample-accuracy requirements
  - Cannot exactly represent sample boundaries across different sample rates
  - Floating-point arithmetic introduces precision loss
  - Impossible to guarantee bit-exact repeatability

**Evidence of Current Design:**
- IMPL001 does not explicitly specify timing column types (gap in documentation)
- REQ001 implies time-based representation (references to "seconds", "ms")
- No existing specification for sample-accurate timing storage

#### New Design Requirement

**Document:** SPEC017-sample_rate_conversion.md
**Requirement IDs:** SRC-DB-010 through SRC-DB-016

**Proposed Representation:**
- Passage timing stored as **INTEGER** ticks at 28,224,000 Hz
- Tick rate = LCM of all supported sample rates (8kHz, 11.025kHz, 16kHz, 22.05kHz, 32kHz, 44.1kHz, 48kHz, 88.2kHz, 96kHz, 176.4kHz, 192kHz)
- Example: 120.5 seconds = 3,401,088,000 ticks (exact)
- One tick = 1/28,224,000 second ≈ 35.4 nanoseconds

**Database Schema Changes:**
```sql
-- CURRENT (implied):
start_time REAL,
end_time REAL,
fade_in_point REAL,
fade_out_point REAL,
lead_in_point REAL,
lead_out_point REAL

-- PROPOSED:
start_time INTEGER,  -- ticks from file start
end_time INTEGER,    -- ticks from file start
fade_in_point INTEGER,
fade_out_point INTEGER,
lead_in_point INTEGER,
lead_out_point INTEGER
```

**API Conversion:**
- REST API continues to use milliseconds (unsigned integers) for human readability
- Conversion: `ticks = milliseconds × 28,224`
- Example: 234500ms → 6,618,528,000 ticks (exact)

#### Benefits of Tick-Based System

1. **Sample-Accurate Precision:**
   - Can exactly represent any sample boundary from any supported sample rate
   - Zero conversion error between sample rates
   - Example: 5 seconds = 220,500 samples at 44.1kHz = 141,120,000 ticks (exact)
   - Same 141,120,000 ticks = 240,000 samples at 48kHz (exact)

2. **Repeatability:**
   - Integer arithmetic eliminates floating-point rounding errors
   - Bit-exact repeatability across platforms and playback sessions
   - Crossfade timing identical every time passage plays

3. **No Cumulative Errors:**
   - Long passages and complex crossfades maintain precision
   - No drift over time
   - Addition/subtraction of tick values is exact

4. **Future-Proof:**
   - i64 range: ~10.36 years of audio (more than sufficient)
   - Supports high sample rates (up to 192kHz) without precision loss
   - Can add new sample rates by updating LCM calculation

#### Costs and Migration Requirements

1. **Database Migration:**
   - Migration script required to convert existing REAL values to INTEGER ticks
   - Formula: `ticks = CAST(seconds * 28224000 AS INTEGER)`
   - Backup required before migration
   - Migration is one-way (cannot easily revert)

2. **API Changes:**
   - Internal APIs must convert milliseconds → ticks on input
   - Internal APIs must convert ticks → milliseconds on output
   - Client-facing API unchanged (still uses milliseconds)

3. **Code Changes Required:**
   - Database access layer: Update passage timing queries
   - Decoder initialization: Convert timing from ticks to sample counts
   - Crossfade calculations: Operate on tick values
   - Event emissions: Convert ticks → ms for API responses

4. **Testing:**
   - Verify conversion accuracy across all sample rates
   - Test crossfade timing with tick-based calculations
   - Validate migration script on test database
   - Performance testing (integer vs floating-point operations)

#### Risk Assessment

**Technical Risks:**
- Low: Integer arithmetic is faster and more reliable than floating-point
- Migration script risk: Mitigated by thorough testing and backup requirement
- Conversion errors: Mitigated by well-defined conversion formulas

**Requirements Risks:**
- Medium: This is a FUNDAMENTAL change to how timing is represented
- Requires stakeholder approval (not just technical decision)
- May impact future features that assume floating-point time values

**Deployment Risks:**
- High: Database migration required for all existing installations
- Cannot mix old/new database formats
- Rollback difficult after migration

#### Recommended Decision

**Recommendation:** ✅ **APPROVE** tick-based timing system

**Rationale:**
1. Sample-accurate timing is a core requirement (REQ-XFD-030)
2. Current floating-point approach cannot meet precision requirements
3. Benefits (precision, repeatability, no cumulative errors) outweigh migration costs
4. Migration is feasible with proper planning and testing
5. Early adoption (before widespread deployment) minimizes migration impact

**Conditions for Approval:**
1. Comprehensive migration script with rollback plan
2. Testing on production-scale database before deployment
3. Documentation update for IMPL001 to specify tick-based schema
4. API specification update to document ms ↔ ticks conversion
5. User communication plan (if any users have existing databases)

#### Alternative Considered and Rejected

**Alternative:** Use integer samples at working_sample_rate (44.1kHz)

**Rejected Because:**
- Cannot exactly represent sample boundaries from other sample rates
- Example: 1 sample at 48kHz ≠ exact integer at 44.1kHz
- Would introduce conversion errors similar to floating-point
- Tick-based system eliminates this problem by using LCM

---

## Approval Process

### Required Approvals

- [ ] **Technical Lead** - Architecture approval
- [ ] **Database Architect** - Schema migration approval
- [ ] **Product Owner** - Requirements change approval

### Approval Criteria

**Approve if:**
- Sample-accurate timing is a firm requirement
- Migration risk is acceptable
- Benefits justify the implementation effort

**Reject if:**
- Floating-point precision is deemed sufficient for use case
- Migration risk too high for current deployment stage
- Implementation effort not justified by benefits

**Defer if:**
- Need more analysis on migration impact
- Want to evaluate alternative approaches
- Waiting for broader architectural decisions

### Approval Date

**Target:** Within 1 week
**Actual:** _____________

### Decision

**Status:** [ ] APPROVED [ ] REJECTED [ ] DEFERRED

**Decision Rationale:**

_____________________________________________

**Signed:**

_____________________________________________

---

## Impact on Other Documents

### If APPROVED:

**Must Update:**
1. IMPL001-database_schema.md
   - Change passage timing columns to INTEGER
   - Document tick-based representation
   - Add conversion formulas

2. REQ001-requirements.md (if timing precision not explicit)
   - Add requirement for sample-accurate timing precision
   - Specify tick-based representation as architectural decision

3. SPEC007-api_design.md
   - Document ms ↔ ticks conversion in API endpoints
   - Add examples showing conversion

4. Migration Guide (new document)
   - Step-by-step migration procedure
   - Backup requirements
   - Rollback plan
   - Testing checklist

**Should Update:**
5. SPEC002-crossfade.md
   - Update examples to use tick-based calculations

6. SPEC016-decoder_buffer_design.md
   - Already uses tick-based approach (SRC-DB-010 references)

### If REJECTED:

**Must Do:**
1. Update SPEC017 to remove tick-based database storage
2. Update SPEC016 to clarify floating-point timing is acceptable
3. Document acceptable precision loss from floating-point representation
4. Verify sample-accurate requirements can be met with floating-point

---

## Appendix: Technical Details

### Tick Rate Calculation

```
Supported Sample Rates:
8000, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000

LCM Calculation:
LCM(8000, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000)
= 28,224,000 Hz

Verification (all should divide evenly):
28,224,000 ÷ 8,000 = 3,528 ✓
28,224,000 ÷ 11,025 = 2,560 ✓
28,224,000 ÷ 16,000 = 1,764 ✓
28,224,000 ÷ 22,050 = 1,280 ✓
28,224,000 ÷ 32,000 = 882 ✓
28,224,000 ÷ 44,100 = 640 ✓
28,224,000 ÷ 48,000 = 588 ✓
28,224,000 ÷ 88,200 = 320 ✓
28,224,000 ÷ 96,000 = 294 ✓
28,224,000 ÷ 176,400 = 160 ✓
28,224,000 ÷ 192,000 = 147 ✓
```

### Migration SQL Example

```sql
-- Backup current data
CREATE TABLE passages_backup AS SELECT * FROM passages;

-- Migrate timing columns (example)
ALTER TABLE passages RENAME TO passages_old;

CREATE TABLE passages (
  -- ... other columns ...
  start_time INTEGER,  -- was REAL seconds, now ticks
  end_time INTEGER,
  fade_in_point INTEGER,
  fade_out_point INTEGER,
  lead_in_point INTEGER,
  lead_out_point INTEGER
);

INSERT INTO passages
SELECT
  -- ... other columns ...
  CAST(start_time * 28224000 AS INTEGER),
  CAST(end_time * 28224000 AS INTEGER),
  CAST(fade_in_point * 28224000 AS INTEGER),
  CAST(fade_out_point * 28224000 AS INTEGER),
  CAST(lead_in_point * 28224000 AS INTEGER),
  CAST(lead_out_point * 28224000 AS INTEGER)
FROM passages_old;

-- Verify migration
SELECT COUNT(*) FROM passages WHERE start_time IS NULL; -- should be 0
SELECT COUNT(*) FROM passages WHERE end_time < start_time; -- should be 0

-- Drop old table after verification
-- DROP TABLE passages_old;
```

### API Conversion Examples

**Input (API → Database):**
```rust
// Client sends: { "start_time_ms": 120500 }
let start_time_ms: u64 = 120500;
let start_time_ticks: i64 = (start_time_ms * 28224) as i64;
// Result: 3,400,992,000 ticks
```

**Output (Database → API):**
```rust
// Database has: start_time = 3,400,992,000 ticks
let start_time_ticks: i64 = 3400992000;
let start_time_ms: u64 = ((start_time_ticks + 14112) / 28224) as u64; // round to nearest
// Result: 120500 ms
```

---

**End of Tier 1 Approval Requests**
