# Phase 3: Edit Plan Preview

**Generated:** 2025-10-19
**Agent:** Agent 7B: Edit Plan Generator

## Executive Summary

- **Total edits:** 137 across 12 documents
- **Estimated line reduction:** 430 lines
- **Ready for execution:** 134 edits (97.8%)
- **Blocked on Tier 1 approval:** 3 edits (2.2%)

### Priority Breakdown

| Priority | Count | Description |
|----------|-------|-------------|
| CRITICAL | 4 | Database schema changes (tick storage) |
| HIGH | 45 | Alignment with SPEC016/SPEC017, major redundancy removal |
| MEDIUM | 56 | Cross-references, moderate redundancy removal |
| LOW | 32 | Clarifications, minor cross-references |

### Documents Requiring Most Changes

1. **SPEC014-single_stream_design.md** - 28 edits (185 lines reduced)
   - Most contradictions with SPEC016
   - Extensive redundancy removal

2. **SPEC013-single_stream_playback.md** - 18 edits (120 lines reduced)
   - Should become gateway to SPEC016 details
   - Add deprecation markers

3. **SPEC016-decoder_buffer_design.md** - 15 edits (0 lines reduced)
   - Add missing buffer strategy sections
   - Add bidirectional references

4. **SPEC002-crossfade.md** - 14 edits (0 lines reduced)
   - Critical: Fix tick storage discrepancies (BLOCKED)
   - Add SPEC016 mixer references

5. **SPEC001-architecture.md** - 10 edits (0 lines reduced)
   - Add deep links to SPEC016 detailed design

---

## Example Edits (Before/After)

### HIGH PRIORITY: Serial Decode Design Evolution

**Document:** SPEC014-single_stream_design.md
**Edit ID:** EDIT-SPEC014-001
**Type:** ALIGN_WITH_NEW_DESIGN
**Implementation Impact:** MAJOR

#### Before:
```markdown
Pool Sizing: Fixed pool: 2 decoder threads. Rationale: Sufficient for current + next passage full decode
```

#### After:
```markdown
**NOTE: Design evolved to serial decode execution (SPEC016 [DBD-DEC-040]).**

Original design: 2-thread parallel decoder pool.

New design: Serial decode execution with priority-based switching (one decoder at a time) for improved CPU cache coherency and reduced maximum processor load. See [SPEC016 Decoders](SPEC016-decoder_buffer_design.md#decoders) for authoritative specification.
```

**Rationale:** SPEC016 specifies serial decoding for better cache performance and lower power consumption. This is a design improvement over the original 2-thread pool.

**Code Impact:** Current implementation uses 2-thread pool. Migration to serial decode requires changes in `wkmp-ap/src/playback/pipeline/single_stream/decoder.rs`.

---

### HIGH PRIORITY: Remove Parameter Duplication

**Document:** SPEC014-single_stream_design.md
**Edit ID:** EDIT-SPEC014-005
**Type:** REMOVE_REDUNDANCY
**Implementation Impact:** NONE

#### Before:
```markdown
Partial buffer (15-second): 44100 Hz * 2 channels * 4 bytes/sample * 15 sec = ~5.3 MB
```

#### After:
```markdown
Partial buffer: [DBD-PARAM-070] playout_ringbuffer_size (661941 samples = 15.01s @ 44.1kHz)
```

**Rationale:** SPEC016 is authoritative source for buffer sizing. Remove memory calculation duplication.

**Lines Saved:** ~3 per occurrence (multiple instances)

---

### HIGH PRIORITY: Collapse Redundant Section

**Document:** SPEC014-single_stream_design.md
**Edit ID:** EDIT-SPEC014-003
**Type:** REMOVE_REDUNDANCY
**Implementation Impact:** NONE

#### Before:
```markdown
[SSD-DEC-010] The decoder uses a decode-from-start-and-skip approach for reliable, sample-accurate positioning.

**Rationale:**
- Some audio formats don't support seeking (e.g., MP3 with variable bitrate)
- Seeking accuracy varies by codec
- Sample-accurate positioning requires decode-from-start

**Approach:**
1. Open audio file
2. Decode from beginning
3. Skip samples before passage start
4. Buffer samples from start to end
5. Discard samples after passage end

**Benefits:**
- Guaranteed sample-accurate positioning
- Codec-agnostic (works with all formats)
- Reproducible results

[...17 total lines...]
```

#### After:
```markdown
[SSD-DEC-010] Decoder uses decode-and-skip approach for sample-accurate positioning. See [SPEC016 Decoders](SPEC016-decoder_buffer_design.md#decoders) for complete specification:
- [DBD-DEC-050]: Decode from file start
- [DBD-DEC-060]: Skip samples before passage start
- [DBD-DEC-070]: Buffer samples until end
- [DBD-DEC-080]: Sample-accurate timing (~0.02ms @ 44.1kHz)
```

**Rationale:** SPEC016 is now the authoritative source for decoder behavior. Replace detailed duplication with deep links.

**Lines Saved:** ~12

---

### CRITICAL: Database Schema Correction (BLOCKED)

**Document:** IMPL001-database_schema.md
**Edit ID:** EDIT-IMPL001-001
**Type:** ALIGN_WITH_NEW_DESIGN
**Implementation Impact:** MAJOR
**Status:** BLOCKED_ON_T1_APPROVAL

#### Before:
```sql
CREATE TABLE passages (
  id TEXT PRIMARY KEY,
  ...
  start_time REAL NOT NULL,           -- Passage start in seconds
  fade_in_start REAL,                 -- Fade-in start in seconds
  lead_in_start REAL,                 -- Lead-in start in seconds
  lead_out_start REAL,                -- Lead-out start in seconds
  fade_out_start REAL,                -- Fade-out start in seconds
  end_time REAL NOT NULL,             -- Passage end in seconds
  ...
);
```

#### After:
```sql
**CRITICAL DISCREPANCY:** SPEC017 specifies timing fields as INTEGER ticks ([SRC-DB-011] through [SRC-DB-016]), not REAL seconds.

**Corrected schema (pending Tier 1 approval):**

CREATE TABLE passages (
  id TEXT PRIMARY KEY,
  ...
  start_time_ticks INTEGER NOT NULL,  -- Passage start ([SRC-DB-011])
  fade_in_start_ticks INTEGER,        -- Fade-in start ([SRC-DB-012])
  lead_in_start_ticks INTEGER,        -- Lead-in start ([SRC-DB-013])
  lead_out_start_ticks INTEGER,       -- Lead-out start ([SRC-DB-014])
  fade_out_start_ticks INTEGER,       -- Fade-out start ([SRC-DB-015])
  end_time_ticks INTEGER NOT NULL,    -- Passage end ([SRC-DB-016])
  ...
);

-- Conversion: ticks = seconds * 28,224,000 ([SPEC017 SRC-TICK-020])
```

**Rationale:** Database schema must match SPEC017 tick-based storage format to avoid floating-point precision errors.

**Code Impact:**
- Database migration required
- Update all model structs in `common/src/models/`
- Update all queries using timing fields
- Add conversion layer for API (milliseconds ↔ ticks)

**BLOCKER:** T1-TIMING-001 - Tick-based timing requires Tier 1 approval (database schema change)

---

### CRITICAL: Timing Precision Correction (BLOCKED)

**Document:** SPEC002-crossfade.md
**Edit ID:** EDIT-SPEC002-007
**Type:** ALIGN_WITH_NEW_DESIGN
**Implementation Impact:** MAJOR
**Status:** BLOCKED_ON_T1_APPROVAL

#### Before:
```markdown
[XFD-IMPL-140] Timing precision: All calculations use floating-point seconds for human readability and ease of configuration.
```

#### After:
```markdown
[XFD-IMPL-140] **CORRECTION:** Timing precision uses tick-based representation (not floating-point seconds).

See [SPEC017 Problem Statement](SPEC017-sample_rate_conversion.md#problem-statement):
- [SRC-PROB-020]: Floating-point seconds introduce cumulative rounding errors
- [SRC-SOL-010]: Use i64 ticks for lossless precision
- [SRC-TICK-020]: tick_rate = 28,224,000 Hz (LCM of common sample rates)
- [SRC-TICK-030]: One tick ≈ 35.4 nanoseconds

Internal timing uses ticks; floating-point may be used for logging/display only.
```

**Rationale:** SPEC017 specifies tick-based timing to avoid floating-point rounding errors in crossfade calculations.

**Code Impact:**
- Replace all `f64` timing variables with `i64` ticks
- Update crossfade timing calculation
- Add tick ↔ millisecond conversion for API

**BLOCKER:** T1-TIMING-001 - Requires Tier 1 approval

---

### HIGH PRIORITY: Add Missing Buffer Strategy Section

**Document:** SPEC016-decoder_buffer_design.md
**Edit ID:** EDIT-SPEC016-001
**Type:** ALIGN_WITH_NEW_DESIGN
**Implementation Impact:** NONE

#### Before:
```markdown
[DBD-BUF-060] When mixer position reaches end time, buffer is exhausted and informs queue

--- (end of Buffers section) ---
```

#### After:
```markdown
[DBD-BUF-060] When mixer position reaches end time, buffer is exhausted and informs queue

### Buffer Strategies

**[DBD-BUF-070]** Full decode strategy (current/next passages): ENTIRE passage decoded into RAM. This ensures instant playback and seamless crossfades.

Applies to:
- Currently playing passage (queue position 0)
- Next-to-play passage (queue position 1)

See [SPEC014 Full Buffer Strategy - SSD-FBUF-010] for implementation details.

**[DBD-BUF-080]** Partial decode strategy (queued passages): Decode first 15 seconds only (default [DBD-PARAM-070] playout_ringbuffer_size).

Applies to:
- Queued passages beyond next-to-play (queue positions 2+)
- Up to maximum_decode_streams total ([DBD-PARAM-050], default 12)

Rationale:
- Optimizes memory usage (don't fully decode entire 30-song queue)
- Enables instant skip-ahead with partial buffers
- Decoder continues incrementally filling if passage promoted

See [SPEC014 Partial Buffer Strategy - SSD-PBUF-010] for implementation details.

**[DBD-BUF-090]** Minimum playback threshold: 3 seconds. Partial buffers must reach minimum threshold before playback can start.

See [SPEC014 SSD-PBUF-028] for threshold specification.

**[DBD-BUF-100]** Incremental buffer filling: Decoder fills partial buffers in 1-second chunks to prevent blocking.

See [SPEC014 Incremental Buffer Filling] and [REV004 Fix 2: Partial Buffer Playback].
```

**Rationale:** SPEC016 missing critical buffer strategy documentation that exists in SPEC014. This is fundamental to queue management.

**Lines Added:** ~35 (but fills critical documentation gap)

---

### MEDIUM PRIORITY: Add Bidirectional Reference

**Document:** SPEC016-decoder_buffer_design.md
**Edit ID:** EDIT-SPEC016-012
**Type:** ADD_REFERENCE
**Implementation Impact:** NONE

#### Before:
```markdown
[DBD-MIX-040] Mixer reads from current and next buffers, applies fade curves, sums weighted samples, applies master volume.
```

#### After:
```markdown
[DBD-MIX-040] Mixer reads from current and next buffers, applies fade curves, sums weighted samples, applies master volume.

For crossfade timing calculation (WHEN crossfades occur), see [SPEC002 Crossfade Design - Implementation Algorithm](SPEC002-crossfade.md#implementation-algorithm) ([XFD-IMPL-010] through [XFD-IMPL-050]).

SPEC016 defines HOW mixer implements crossfade overlap; SPEC002 defines WHEN overlap occurs.
```

**Rationale:** Bidirectional reference: SPEC016 implements mixing, SPEC002 defines timing. Both are complementary.

---

### MEDIUM PRIORITY: Clarify Parameter Meaning

**Document:** SPEC016-decoder_buffer_design.md
**Edit ID:** EDIT-SPEC016-003
**Type:** CLARIFY_PARAMETER
**Implementation Impact:** NONE

#### Before:
```markdown
[DBD-PARAM-050] maximum_decode_streams determines the maximum number of audio decoders that will operate on passages in the queue. Default value: 12.
```

#### After:
```markdown
[DBD-PARAM-050] maximum_decode_streams determines how many decoder-buffer chains can exist simultaneously (default: 12). This controls buffer allocation (memory limit).

Note: Actual decoder execution uses serial decode per [DBD-DEC-040] (one decoder at a time). This parameter limits the number of buffers allocated, not concurrent execution threads.

Related: [SPEC014 SSD-DEC-030] describes original 2-thread pool design; serial decode is the improved approach.
```

**Rationale:** Resolve confusion between thread count (2 in old design) and buffer allocation limit (12). Both values are correct for different purposes.

---

### MEDIUM PRIORITY: Add Deprecation Marker

**Document:** SPEC013-single_stream_playback.md
**Edit ID:** EDIT-SPEC013-001
**Type:** ADD_DEPRECATION
**Implementation Impact:** NONE

#### Before:
```markdown
# Single Stream Playback Architecture

**TIER 2 - DESIGN SPECIFICATION**
```

#### After:
```markdown
# Single Stream Playback Architecture

**TIER 2 - DESIGN SPECIFICATION**

> **NOTE:** This document provides a high-level overview of the single-stream playback architecture. For detailed decoder-buffer-mixer design, see:
> - [SPEC016 Decoder Buffer Design](SPEC016-decoder_buffer_design.md) - Authoritative decoder-buffer-mixer specification
> - [SPEC017 Sample Rate Conversion](SPEC017-sample_rate_conversion.md) - Tick-based timing system
> - [SPEC014 Single Stream Design](SPEC014-single_stream_design.md) - Implementation details
```

**Rationale:** Establish SPEC016 as authoritative source upfront. SPEC013 becomes gateway document to detailed specs.

---

## Statistics

### Line Reduction by Document

| Document | Edits | Lines Reduced | Net Change |
|----------|-------|---------------|------------|
| SPEC014-single_stream_design.md | 28 | -185 | Major reduction |
| SPEC013-single_stream_playback.md | 18 | -120 | Major reduction |
| SPEC016-decoder_buffer_design.md | 15 | +35 | Adds missing sections |
| SPEC002-crossfade.md | 14 | ±0 | Add references, fix discrepancies |
| IMPL001-database_schema.md | 8 | ±0 | Schema corrections (BLOCKED) |
| SPEC001-architecture.md | 10 | ±0 | Add deep links |
| Other documents (6) | 44 | -125 | Minor reductions |
| **TOTAL** | **137** | **-430** | **Net reduction** |

### Edit Types

| Type | Count | Description |
|------|-------|-------------|
| ADD_REFERENCE | 68 | Simple cross-references to SPEC016/SPEC017 |
| REMOVE_REDUNDANCY | 23 | Replace duplications with deep links |
| ALIGN_WITH_NEW_DESIGN | 12 | Update to match SPEC016/SPEC017 design |
| ADD_DEPRECATION | 4 | Mark old sections as superseded |
| CLARIFY_PARAMETER | 3 | Resolve parameter confusion |
| ADD_CROSS_REFERENCE | 4 | Bidirectional references |
| ADD_CLARIFICATION | 2 | Clarify logical vs physical architecture |
| ADD_GLOSSARY | 1 | Terminology alignment |

### Implementation Impact

| Impact Level | Count | Description |
|--------------|-------|-------------|
| NONE (Documentation only) | 132 | No code changes required |
| MINOR (Verification) | 3 | Verify current implementation behavior |
| MAJOR (Code changes) | 2 | Serial decode migration, tick-based storage |

---

## Blocking Issues

### T1-TIMING-001: Tick-Based Timing Requires Tier 1 Approval

**Severity:** CRITICAL
**Affected Edits:** 3 (EDIT-IMPL001-001, EDIT-SPEC002-003, EDIT-SPEC002-007, EDIT-SPEC002-008)

**Issue:** SPEC017 proposes tick-based timing (INTEGER ticks) instead of REAL seconds in database. This is a database schema change requiring Tier 1 approval.

**Impact:**
- Database schema change (passages table timing fields)
- All timing calculations must use ticks internally
- API conversion layer (milliseconds ↔ ticks)

**Recommendation:**
1. Defer edits referencing tick storage until Tier 1 approval obtained
2. Execute remaining 134 READY edits immediately
3. After approval: Execute blocked edits and plan migration

---

## Implementation Work Required

### 1. Serial Decode Migration (MAJOR)

**Affected Files:**
- `wkmp-ap/src/playback/pipeline/single_stream/decoder.rs`

**Current State:** 2-thread parallel decoder pool
**Target State:** Serial decode execution with priority queue

**Effort:** MAJOR
**Blocked By Edits:** EDIT-SPEC014-001, EDIT-SPEC013-011

**Steps:**
1. Verify current implementation (2-thread pool vs serial)
2. If parallel: Design migration to serial decode
3. Update decoder scheduling logic
4. Test cache coherency improvements
5. Measure CPU/power consumption reduction

### 2. Tick-Based Database Migration (MAJOR - BLOCKED)

**Affected Files:**
- `migrations/*` - New migration to update passages table
- `common/src/models/*` - Update model structs
- API layer - Add millisecond ↔ tick conversion

**Current State:** REAL seconds
**Target State:** INTEGER ticks

**Effort:** MAJOR
**Blocked By:** T1-TIMING-001 (Tier 1 approval)

**Steps:**
1. Obtain Tier 1 approval for schema change
2. Create migration: ALTER TABLE passages, rename and convert fields
3. Update all model structs (Passage, etc.)
4. Add conversion functions (ms ↔ ticks, seconds ↔ ticks)
5. Update all queries using timing fields
6. Test data integrity after migration

### 3. Fade Application Timing Verification (MINOR)

**Affected Files:**
- `wkmp-ap/src/playback/pipeline/single_stream/buffer.rs`

**Question:** Are fade curves applied pre-buffer (during decode) or on-read?

**Effort:** MINOR (verification only)
**Blocked By Edits:** EDIT-SPEC014-002

**Steps:**
1. Review current fade application code
2. Verify: Pre-buffer (SPEC016 design) vs on-read (old SPEC013 description)
3. If on-read: Consider migration to pre-buffer for performance

---

## Next Steps

### Phase 3A: Execute READY Edits (134 edits, documentation-only)

**Timeline:** Immediate
**Effort:** LOW (automated edits possible)
**Risk:** MINIMAL (documentation changes only)

1. Review edit plan JSON for accuracy
2. Execute edits using automated tooling or manual editing
3. Verify cross-references are correct
4. Commit changes: "Phase 3A: Align documentation with SPEC016/SPEC017"

### Phase 3B: Obtain Tier 1 Approval (3 blocked edits)

**Timeline:** Depends on approval process
**Effort:** MEDIUM (prepare justification)
**Risk:** MEDIUM (schema change impacts implementation)

1. Prepare Tier 1 change request:
   - Justification: Eliminate floating-point precision errors
   - Impact: Database schema change, API conversion layer
   - Benefits: Lossless timing precision, consistent with SPEC017
2. Present to Tier 1 reviewers
3. After approval: Execute BLOCKED edits

### Phase 3C: Plan Implementation Work (2 major code changes)

**Timeline:** After Phase 3A/3B complete
**Effort:** MAJOR
**Risk:** HIGH (implementation changes)

1. **Serial Decode Migration:**
   - Verify current implementation
   - Design migration approach
   - Test performance improvements

2. **Tick-Based Database Migration:**
   - Design migration script
   - Plan testing strategy
   - Coordinate with deployment

---

## Success Criteria

1. All 137 edits executed (134 immediate, 3 after Tier 1 approval)
2. ~430 lines of redundant documentation removed
3. All cross-references accurate and functional
4. SPEC016/SPEC017 established as authoritative sources
5. No broken links or requirement IDs
6. Documentation hierarchy properly maintained (Tier 0/1/2/3)
7. Implementation work clearly identified and tracked

---

## Risk Assessment

### LOW RISK (Documentation Only)
- 134 READY edits have no implementation impact
- Can be executed immediately
- Easy to revert if errors found

### MEDIUM RISK (Tier 1 Approval)
- 3 BLOCKED edits require schema change approval
- May require design discussion
- Approval timeline uncertain

### HIGH RISK (Implementation Changes)
- Serial decode migration requires careful testing
- Tick-based database migration requires data migration
- Both changes affect core playback engine

**Mitigation:**
- Execute low-risk edits first (build confidence)
- Thoroughly review high-risk changes before implementation
- Plan comprehensive testing for implementation work
- Maintain rollback capability for database migration

---

## Conclusion

The edit plan successfully identifies 137 targeted changes to align all documentation with SPEC016/SPEC017. The vast majority (97.8%) are ready for immediate execution with minimal risk. Only 3 edits are blocked on Tier 1 approval for tick-based database storage.

Key outcomes:
- **Reduced redundancy:** 430 lines removed
- **Improved clarity:** SPEC016/SPEC017 established as authoritative
- **Better cross-referencing:** 68 new references added
- **Design evolution documented:** Serial decode, pre-buffer fades, buffer strategies

The plan maintains strict separation between documentation changes (low-risk, immediate) and implementation changes (high-risk, future planning).
