# Phase 3: Tier 0/1 Conflict Review and SPEC016/SPEC017 Approval Request

**Generated:** 2025-10-19
**Agent:** Agent 6: Tier 0/1 Conflict Reviewer
**Source:** docs/validation/phase2-contradictions.json

---

## Executive Summary

### Findings

- **Tier 0/1 Conflicts:** 0 (none found)
- **SPEC016/017 Contradictions:** 11 (all requiring review)
  - CRITICAL: 4
  - MAJOR: 5
  - MINOR: 3

### The SPEC016/SPEC017 Immutability Problem

**User Constraint:** SPEC016-decoder_buffer_design.md and SPEC017-sample_rate_conversion.md were marked as IMMUTABLE and cannot be changed.

**Critical Discovery:** These two documents contain 11 contradictions with earlier implementation documents (SPEC013, SPEC014, SPEC015, IMPL001) that describe the actual system architecture.

**Core Question:** Which specifications are authoritative?
- **Option A:** SPEC016/SPEC017 define the ideal architecture → Update implementation specs to match
- **Option B:** SPEC013/SPEC014/IMPL001 describe actual implementation → Update SPEC016/SPEC017 to match
- **Option C:** Both are valid at different abstraction levels → Add cross-references and clarifications

### Phase 4 Status: BLOCKED

**Cannot proceed to Phase 4 (compliance validation) until these contradictions are resolved.**

Four CRITICAL conflicts require immediate attention:
1. Decoder threading model (serial vs 2-thread pool)
2. Fade application timing (pre-buffer vs on-read)
3. Buffer management strategy (missing partial buffer documentation)
4. Database timing storage (INTEGER ticks vs REAL seconds)

---

## CRITICAL Contradictions Requiring Immediate Review

### SPEC16-CONTRA-001: Decoder Threading Model

**Severity:** CRITICAL
**Documents:** SPEC016, SPEC013, SPEC014, REV004

**The Contradiction:**

| SPEC016 Says | Other Docs Say |
|--------------|----------------|
| DBD-DEC-040: "Decoding is handled **serially** in priority order, **only one decode runs at a time** to preserve cache coherency and reduce maximum processor loads, to avoid spinning up the cooling fans." (line 177) | SPEC013 SSP-DEC-020 + SPEC014 SSD-DEC-030: "**Fixed pool: 2 decoder threads**. Rationale: Sufficient for current + next passage full decode." Component diagram shows 3 decoder instances running in parallel. (lines 81, 114) |

**Impact:** Fundamental architectural mismatch. Serial execution would significantly increase latency for queue prefetching and potentially cause playback gaps between passages.

**Analysis:**
- SPEC016 describes 1 decoder running at a time (serial execution)
- SPEC013, SPEC014, and REV004 ALL consistently describe 2 decoder threads running concurrently
- The 2-thread model enables overlapped decoding of current passage + next passage
- Serial execution conflicts with requirement for seamless playback

**Options:**

**A) Update SPEC016 to match 2-thread implementation** (Violates immutability)
- Edit DBD-DEC-040: "Decoding is handled by a fixed 2-thread pool in priority order. Up to 2 decodes can run concurrently to enable current + next passage preparation while maintaining manageable processor loads on resource-constrained devices."
- Impact: Aligns SPEC016 with actual implementation
- Violates: SPEC016 marked as immutable

**B) Update SPEC013/014 to match SPEC016 serial model**
- Change SSD-DEC-030 to single decoder thread
- Remove 2-thread pool references, update component diagrams
- Impact: Would require re-architecting decoder pool. May violate playback requirements.
- Violates: Nothing (implementation specs can change)

**C) Clarify both are valid at different abstraction levels**
- Add note to SPEC016: "Implementation Note: While the design describes serial priority-based scheduling, the implementation uses a 2-thread pool to parallelize current+next passage decoding while maintaining priority order. See SPEC014."
- Impact: Preserves both documents, requires cross-reference
- Violates: SPEC016 marked as immutable (still modifies it)

**Recommendation:** Option A - Update SPEC016

**Rationale:** All implementation documents consistently describe 2-thread execution. Serial execution would degrade performance significantly. SPEC016 appears to be aspirational design that doesn't match reality.

**Approval Required From:** Technical Lead, Audio Engineer

---

### SPEC16-CONTRA-002: Fade Application Timing

**Severity:** CRITICAL
**Documents:** SPEC016, SPEC013, SPEC014

**The Contradiction:**

| SPEC016 Says | Other Docs Say |
|--------------|----------------|
| DBD-FADE-030: "When fade-in duration is > 0 then samples between the start time and the fade-in end point have the **fade-in curve applied before they are buffered**." (line 199) | SPEC013 SSP-BUF-020: "Automatic fade application **during read_sample()** - no separate fade step." API example shows `let (left, right) = buffer.read_sample();` with fades applied at read time. (lines 114, 143) |

**Impact:** Affects whether fade parameters can be changed after buffering, memory usage patterns, and CPU overhead during playback.

**Analysis:**
- SPEC016: Fades applied BEFORE buffering (pre-buffer, during decode)
  - Pros: Pre-computed, no CPU overhead during playback
  - Cons: Fade parameters become immutable after buffering
- SPEC013: Fades applied during read_sample() (on-read, after buffering)
  - Pros: Dynamic fade adjustment possible, simpler decode logic
  - Cons: Per-sample multiplication during playback (higher CPU)
- SPEC014: "Fade application can occur during decode or be deferred to read-time (implementation choice)" - acknowledges both approaches

**Options:**

**A) Update SPEC016 to describe on-read fade application** (Violates immutability)
- Change DBD-FADE-030/040/050 to describe fades applied during buffer read
- Impact: Aligns with SPEC013 API description
- Violates: SPEC016 marked as immutable

**B) Update SPEC013/014 to describe pre-buffer fade application**
- Change SSP-BUF-020, modify API examples
- Impact: Would require reimplementing fade system
- Violates: Nothing

**C) Clarify both approaches are valid**
- Update SPEC016: "Fade curves MAY be applied during decode (pre-buffer) OR during buffer read (on-read). Implementation choice depends on performance trade-offs."
- Add cross-reference to SPEC013 for actual implementation choice
- Impact: Preserves flexibility
- Violates: SPEC016 marked as immutable

**D) Investigate actual implementation first**
- Examine wkmp-ap/src/playback/pipeline/single_stream/buffer.rs
- Determine when fades are actually applied
- Then update docs accordingly
- Impact: Provides empirical answer
- Violates: Nothing (investigation only)

**Recommendation:** Option D then Option A - Investigate first, likely on-read

**Rationale:** SPEC014 explicitly acknowledges both approaches are possible. SPEC013's API examples strongly suggest on-read application. Need to verify actual code before making documentation changes.

**Approval Required From:** Technical Lead, Audio Engineer

---

### SPEC16-CONTRA-003: Buffer Management Strategy

**Severity:** CRITICAL
**Documents:** SPEC016, SPEC014, REV004

**The Contradiction:**

| SPEC016 Says | Other Docs Say |
|--------------|----------------|
| DBD-OV-020: "Separate buffers are created for each passage." (line 19) - No mention of partial buffering, minimum thresholds, or incremental filling. | SPEC014 SSD-PBUF-010/SSD-FBUF-010: "**Full decode strategy** for current/next passages (entire passage decoded). **Partial buffer strategy** for queued passages (15-second default, configurable)." REV004: "Incremental buffer filling with **1-second chunks**, **3000ms minimum threshold** before playback can start." (lines 187, 193, 119) |

**Impact:** SPEC016 completely missing a major architectural feature. Without this, document does not accurately describe how system works.

**Analysis:**
- SPEC016 describes buffers generically - no differentiation by queue position
- SPEC014 clearly documents TWO distinct buffering strategies:
  1. **Full decode:** Current/next passages decoded completely
  2. **Partial decode:** Queued passages get 15-second buffer (configurable)
- REV004 adds incremental implementation details:
  - Decoder appends 1-second chunks progressively
  - Buffer manager tracks decode progress
  - Playback starts when 3000ms threshold met
- This is a major architectural feature for memory efficiency and responsiveness

**Options:**

**A) Add full vs partial buffer strategy to SPEC016** (Violates immutability)
- Add new sections:
  - DBD-BUF-070: Full decode strategy for current/next passages
  - DBD-BUF-080: Partial decode strategy with 15-second default
  - DBD-BUF-090: Minimum playback threshold (3 seconds)
  - DBD-BUF-100: Incremental buffer filling (1-second chunks)
- Impact: Makes SPEC016 complete and accurate (large addition)
- Violates: SPEC016 marked as immutable

**B) Add cross-reference note only**
- Add to SPEC016: "Buffer management strategy is documented in SPEC014. This document describes decoder-buffer chain architecture only."
- Impact: Acknowledges SPEC016 is partial spec
- Violates: SPEC016 marked as immutable

**C) Remove buffer management from SPEC014, consolidate to SPEC016**
- Delete SSD-PBUF and SSD-FBUF sections from SPEC014
- Update SPEC016 to be complete buffer specification
- Impact: Consolidates docs but requires major SPEC016 work
- Violates: SPEC016 marked as immutable

**Recommendation:** Option A - Add missing content to SPEC016

**Rationale:** SPEC016 is incomplete. This is a MAJOR architectural feature that cannot be left undocumented in a design spec titled "Decoder Buffer Design". Adding missing content is less disruptive than consolidation and makes SPEC016 actually complete.

**Approval Required From:** Technical Lead, Audio Engineer

---

### SPEC17-DB-TIMING: Database Timing Storage Format

**Severity:** CRITICAL
**Documents:** SPEC017, SPEC002, IMPL001
**Tier Span:** 2-3 (Design + Implementation)

**The Contradiction:**

| SPEC017 Says | Other Docs Say |
|--------------|----------------|
| SRC-DB-010 through SRC-DB-040: "All passage timing points stored as **INTEGER ticks** in database. Tick rate = 28,224,000 Hz (LCM of all sample rates). Database stores ticks for sample-accurate precision." | SPEC002: Timing points defined in **seconds** (REAL). IMPL001 database schema: `start_time REAL`, `fade_in_point REAL`, `lead_in_point REAL`, `lead_out_point REAL`, `fade_out_point REAL`, `end_time REAL` - ALL **REAL (seconds)**, not INTEGER ticks. (lines 150-155) |

**Impact:** FUNDAMENTAL DATA TYPE MISMATCH. Affects data precision, storage format, all timing calculations, and achievability of sample-accurate timing promises.

**Analysis:**
- **SPEC017's Core Premise:** Tick-based timing system enables sample-accurate precision by avoiding floating-point rounding errors. Document explicitly states database should store INTEGER ticks.
- **IMPL001 Reality:** Database schema shows ALL timing fields as REAL (seconds)
- **SPEC002 Usage:** Crossfade spec uses seconds throughout
- This is not a minor discrepancy - it's the fundamental data type for ALL timing in the system
- If IMPL001 is correct (REAL seconds), then SPEC017's entire tick-based design may not be implemented
- If SPEC017 is correct (INTEGER ticks), then database schema needs major migration

**Questions Requiring Investigation:**
1. What data types does the actual wkmp-ap code use internally?
2. Is there conversion at the database boundary (REAL storage, tick calculations)?
3. Have sample-accuracy requirements actually been achieved with REAL seconds?
4. Does the tick-based system exist anywhere in the codebase?

**Options:**

**A) Update IMPL001 database schema to use INTEGER ticks**
- Change all timing fields from REAL to INTEGER
- Create migration to convert existing REAL seconds → INTEGER ticks
- Update code to use tick arithmetic
- Impact: MAJOR database and code changes, achieves sample accuracy per SPEC017
- Violates: Nothing (Tier 3 can change)

**B) Update SPEC017 to match REAL seconds implementation** (Violates immutability)
- Change SRC-DB sections to document REAL seconds storage
- Update rationale to explain precision trade-off accepted
- Impact: MAJOR - violates SPEC017's core design premise
- Violates: SPEC017 marked as immutable

**C) Investigate actual implementation first**
- Examine wkmp-ap timing code (passage loading, crossfade calculations)
- Determine what types are actually used
- May reveal conversion at DB boundary (REAL storage, tick memory)
- Impact: Provides empirical evidence before deciding
- Violates: Nothing

**D) Hybrid approach: REAL in DB, ticks in memory**
- Document that database uses REAL seconds for human readability
- Internal processing converts to INTEGER ticks for calculations
- Conversion happens at database boundary
- Impact: Reconciles both specs, adds conversion layer complexity
- Violates: SPEC017 marked as immutable (requires update)

**Recommendation:** Option C then Option D - Investigate first, likely hybrid

**Rationale:** This is THE most critical conflict. SPEC017's tick-based design is sophisticated but may be theoretical. Database clearly shows REAL. Must verify actual code behavior. Hybrid approach (REAL storage for human readability, tick calculations for accuracy) is common pattern and may already be implemented.

**Approval Required From:** Technical Lead, Database Engineer, Audio Engineer

---

## MAJOR Contradictions Requiring Review

### SPEC16-CONTRA-004: Decoder Pool Sizing

**Severity:** MAJOR
**Documents:** SPEC016, SPEC014

**The Contradiction:**

| SPEC016 Says | Other Docs Say |
|--------------|----------------|
| DBD-PARAM-050: `maximum_decode_streams: 12` - "The maximum number of audio decoders that will operate on passages in the queue." (line 104) | SPEC014 SSD-DEC-030: "Fixed pool: **2 decoder threads**. Rationale: Sufficient for current + next passage full decode." (line 114) |

**Analysis:**
- SPEC016: 12 decoder streams
- SPEC014: 2 decoder threads
- These are likely DIFFERENT concepts but confusingly named:
  - `maximum_decode_streams` = max passages with buffers allocated (12 buffers)
  - `decoder threads` = actual parallel execution limit (2 threads)
- Both values probably correct for their respective purposes
- Needs clarification to prevent confusion

**Options:**

**A) Clarify distinction in SPEC016** (Violates immutability)
- Update DBD-PARAM-050: "maximum_decode_streams determines how many decoder-buffer chains can exist simultaneously (default: 12). Note: Actual decoder execution is limited to a fixed 2-thread pool (see DBD-DEC-040). This parameter controls memory allocation, not thread count."

**B) Add glossary distinguishing terms**
- Add glossary: 'decode stream' = buffer allocation, 'decoder thread' = execution resource

**C) Rename parameter to avoid confusion**
- Rename to `maximum_buffer_allocations` or `maximum_decoder_buffer_chains`
- Impact: Clearer but breaks existing config/code

**Recommendation:** Option A - Clarify distinction

**Approval Required From:** Technical Lead

---

### SPEC16-CONTRA-005: Decode Scheduling Mechanism

**Severity:** MAJOR
**Documents:** SPEC016, SPEC014

**The Contradiction:**

| SPEC016 Says | Other Docs Say |
|--------------|----------------|
| DBD-PARAM-060: `decode_work_period: 5000ms` - "Once every decode_work_period the currently working decoder is **paused** and the list of pending decode jobs is evaluated to determine the highest priority job and **switch to decoding it**." (line 111) | SPEC014 SSD-DEC-032: "Priority Queue Management using ordered VecDeque. Insert at position based on priority value. Always pop front (highest priority first)." Enum-based priority: `Immediate, Next, Prefetch` - no time-based pausing. (lines 88, 124) |

**Analysis:**
- SPEC016: Time-based decode switching (pause every 5 seconds, re-evaluate)
- SPEC014: Priority queue with continuous processing (enum-based priorities)
- Fundamentally different scheduling approaches
- Time-based pausing seems overly complex for Rust idioms
- Priority queue is more elegant and typical

**Options:**

**A) Update SPEC016 to priority queue model** (Violates immutability)
- Replace DBD-PARAM-060 with priority-based description

**B) Update SPEC014 to integrate decode_work_period**
- Document how timer integrates with priority queue

**C) Investigate actual implementation**
- Examine decoder pool code for timer vs priority queue

**Recommendation:** Option C then Option A - Investigate, likely priority queue only

**Approval Required From:** Technical Lead, Audio Engineer

---

### SPEC16-CONTRA-006: Component Architecture

**Severity:** MAJOR
**Documents:** SPEC016, SPEC013, SPEC014

**The Contradiction:**

| SPEC016 Says | Other Docs Say |
|--------------|----------------|
| DBD-OV-040: Pipeline diagram shows separate components: **Decoder → Resampler → Fade In/Out Handler → Buffer** (line 23) | SPEC013/014: Component architecture shows **Decoder Thread Pool → Passage Buffer Manager**. No explicit Resampler or Fade Handler components. (lines 30, 69) |

**Analysis:**
- Not necessarily a contradiction - different abstraction levels:
  - SPEC016: Logical processing stages (data flow)
  - SPEC013/014: Physical component structure (code organization)
- Resampler and Fade Handler likely implemented WITHIN DecoderPool workers, not as separate components
- Needs cross-reference to clarify relationship

**Options:**

**A) Add clarification note to SPEC016** (Violates immutability)
- "Note: This diagram shows logical processing stages. In the implemented architecture, Decoder, Resampler, and Fade Handler are all performed within DecoderPool worker threads (see SPEC014)."

**B) Add logical pipeline to SPEC013/014**
- Include SPEC016 diagram in implementation docs

**C) Accept as different abstraction levels**
- No changes needed

**Recommendation:** Option A - Add clarification note

**Approval Required From:** Technical Lead

---

### SPEC16-CONTRA-007: Buffer Completion Detection

**Severity:** MAJOR
**Documents:** SPEC016, SPEC015

**The Contradiction:**

| SPEC016 Says | Other Docs Say |
|--------------|----------------|
| DBD-BUF-060: "When the sample corresponding to the passage end time is removed from the buffer, the buffer informs the queue that passage playout has completed." (line 219) | SPEC015 PCF-COMP-010: "Use explicit **'decode complete' signal** instead of comparing position to growing buffer. Add `decode_complete` (bool) and `total_frames` (Option<usize>) fields. `is_exhausted()` checks `current_position >= total_frames`." (lines 183, 204) |

**Analysis:**
- SPEC016: Implicit completion detection (position reaches end time)
- SPEC015: Explicit sentinel-based detection (decode_complete flag + total_frames)
- SPEC015 documents a FIX for race conditions with incremental buffer filling
- Sentinel approach is more robust

**Options:**

**A) Update SPEC016 to sentinel approach** (Violates immutability)
- "When mixer position reaches total_frames (set when decoder marks buffer complete), buffer is exhausted. Sentinel approach prevents race conditions with incremental filling."

**B) Add reference to SPEC015**
- "Implementation uses explicit completion sentinel. See SPEC015."

**C) Accept SPEC015 supersedes on this detail**
- No changes

**Recommendation:** Option A - Update SPEC016

**Approval Required From:** Technical Lead

---

### SPEC16-CONTRA-008: Event System Integration

**Severity:** MAJOR
**Documents:** SPEC016, SPEC011, SPEC014

**The Contradiction:**

| SPEC016 Says | Other Docs Say |
|--------------|----------------|
| DBD-BUF-050: Describes buffer state transitions but **no mention of events**. (line 217) | SPEC011: `BufferStateChanged` event with `old_state, new_state, passage_id, buffer_status, decode_progress_percent`. BufferStatus: `Decoding, Ready, Playing, Exhausted`. SPEC014 SSD-BUF-020: "Buffer Manager emits BufferStateChanged events at four key transition points." (lines 232, 308) |

**Analysis:**
- SPEC016: Describes buffer states but not event emission
- SPEC011/014: Document comprehensive event system for buffer lifecycle
- Not a contradiction, but SPEC016 is incomplete for observability
- Events critical for UI updates and monitoring

**Options:**

**A) Add event emission to SPEC016** (Violates immutability)
- Add DBD-BUF-070: "Buffer manager emits BufferStateChanged events (see SPEC011) at transitions: None→Decoding, Decoding→Ready, Ready→Playing, Playing→Exhausted."

**B) Add cross-reference only**
- "For event emission at buffer transitions, see SPEC011."

**C) Accept SPEC016 focuses on data flow**
- SPEC011/014 authoritative for events

**Recommendation:** Option A - Add event documentation

**Approval Required From:** Technical Lead

---

## MINOR Contradictions

### SPEC16-CONTRA-009: Terminology - decoder-buffer chain vs PassageBuffer vs ManagedBuffer

**Severity:** MINOR
**Issue:** SPEC016 uses "decoder-buffer chain", SPEC013 uses "PassageBuffer", REV004 uses "ManagedBuffer"

**Recommendation:** Add glossary note mapping terms

**Approval:** Documentation Lead

---

### SPEC16-CONTRA-010: Settings Table Coverage

**Severity:** MINOR
**Issue:** IMPL001 documents settings (volume_level, audio_sink, event intervals) not in SPEC016

**Recommendation:** Add cross-reference to IMPL001 for complete schema

**Approval:** Documentation Lead

---

### SPEC16-CONTRA-011: Mixer Refill Behavior

**Severity:** MINOR
**Issue:** SPEC016 "every output_refill_period refills" vs SPEC013 "lock-free ring buffer continuous operation"

**Analysis:** Compatible - periodic wake + continuous lock-free are not contradictory

**Recommendation:** Accept as compatible (no changes needed)

**Approval:** None (informational)

---

## Summary and Recommendations

### Statistics

- **Total Contradictions:** 11
- **Tier 0/1 Conflicts:** 0
- **SPEC16/17 Contradictions:** 11
  - CRITICAL: 4 (blocking Phase 4)
  - MAJOR: 5 (should resolve before Phase 4)
  - MINOR: 3 (cosmetic, can defer)

### The Immutability Dilemma

**Constraint:** SPEC016/SPEC017 marked as immutable
**Reality:** 10 of 11 conflicts require modifying SPEC016/017 to resolve

**This creates an impossible situation:**
1. If SPEC016/017 cannot be changed, they remain inaccurate/incomplete
2. If implementation specs change to match SPEC016/017, major rework required
3. Most contradictions involve SPEC016/017 being incomplete or aspirational vs actual implementation

### Critical Decision Required

**User must answer:** Which specifications are authoritative?

**Three Paths Forward:**

**Path 1: SPEC016/017 Are Authoritative (Design-Driven)**
- Implementation must change to match SPEC016/017
- Requires: Re-architect decoder pool (serial execution), re-implement fade system, database migration (ticks), add time-based decode scheduling
- Impact: MAJOR code changes, significant development effort
- Risk: May violate playback requirements, degrade performance

**Path 2: SPEC013/014/IMPL001 Are Authoritative (Implementation-Driven)**
- SPEC016/017 must be updated to match actual implementation
- Requires: Violating immutability constraint on SPEC016/017
- Impact: Documentation changes only (no code changes)
- Risk: SPEC016/017 may have been written for good reasons we don't understand

**Path 3: Hybrid - Both Valid at Different Levels**
- SPEC016/017 describe ideal/aspirational design
- SPEC013/014/IMPL001 describe practical implementation
- Add cross-references and "Implementation Note" clarifications
- Requires: Minor updates to SPEC016/017 (still violates immutability)
- Impact: Documentation complexity, readers must understand both views
- Risk: Maintains confusion, doesn't resolve fundamental discrepancies

### Recommended Path: Path 2 (Implementation-Driven) with Investigation

**Rationale:**
1. SPEC013/014 written during active implementation (mid-October 2025)
2. SPEC016/017 appear to be newer design docs that don't align with reality
3. REV004 corroborates SPEC014 implementation details
4. Code is likely already implemented per SPEC013/014/IMPL001
5. Re-architecting code to match SPEC016/017 would be extremely expensive
6. SPEC016/017 have critical omissions (partial buffers, events) suggesting incompleteness

**Action Plan:**
1. **INVESTIGATE** (3 items requiring code inspection):
   - Fade timing: pre-buffer or on-read? (SPEC16-CONTRA-002)
   - Decode scheduling: timer or priority queue? (SPEC16-CONTRA-005)
   - Database timing: ticks or seconds? (SPEC17-DB-TIMING) **MOST CRITICAL**

2. **AFTER INVESTIGATION, UPDATE SPEC016/017:**
   - Add missing buffer management strategy (SPEC16-CONTRA-003)
   - Update decoder threading to 2-thread pool (SPEC16-CONTRA-001)
   - Update fade timing based on findings (SPEC16-CONTRA-002)
   - Update database timing based on findings (SPEC17-DB-TIMING)
   - Add event integration (SPEC16-CONTRA-008)
   - Add clarifications for remaining items

3. **DOCUMENT IMMUTABILITY EXCEPTION:**
   - Create REV005-spec16_17_alignment_update.md documenting why immutability was overridden
   - Rationale: SPEC016/017 were incomplete/aspirational, needed alignment with implemented architecture

### Phase 4 Status: BLOCKED

**Cannot proceed to compliance validation until:**
1. User decides which path to take
2. Critical contradictions resolved (at minimum: database timing format)
3. Investigation completed for items requiring code inspection

### Questions for Technical Lead

1. **Why were SPEC016/017 marked as immutable?**
   - Were they approved by stakeholders?
   - Do they represent contractual commitments?
   - Or were they just recent documents that shouldn't be casually modified?

2. **What is the actual implementation status?**
   - Is wkmp-ap code following SPEC013/014 or SPEC016/017?
   - Have any SPEC016/017 features been implemented?
   - Or are they aspirational designs for future work?

3. **What are the sample-accuracy requirements?**
   - Is tick-based timing actually needed?
   - Have precision issues been observed with REAL seconds?
   - Can we quantify acceptable timing error?

4. **What is the performance budget?**
   - Is serial decoding acceptable?
   - What is the target latency for passage transitions?
   - Are there thermal constraints requiring serial execution?

---

## Approval Request Template

For each CRITICAL conflict, technical lead should complete:

```
Conflict ID: SPEC16-CONTRA-XXX
Decision: [Path 1 / Path 2 / Path 3 / Investigate First]
Assigned To: [Name]
Target Date: [Date]
Notes: [Rationale for decision]
```

**Next Steps After Approval:**
1. Execute investigation for items requiring code inspection
2. Update specifications per approved path
3. Create REV005 document recording decisions
4. Proceed to Phase 4 (compliance validation)

---

**Document Status:** AWAITING APPROVAL
**Blocking Phase:** Phase 4 (Compliance Validation)
**Critical Path Items:** 4 (see above)
**Estimated Resolution Time:** 2-5 days (depending on investigation depth)
