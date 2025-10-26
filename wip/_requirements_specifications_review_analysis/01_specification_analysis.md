# Specification Analysis: wkmp-ap Completeness and Readiness

**Section:** Detailed Specification Gap Analysis
**Parent Document:** [00_ANALYSIS_SUMMARY.md](00_ANALYSIS_SUMMARY.md)

---

## Purpose

This document provides detailed analysis of wkmp-ap specification completeness, identifying gaps, ambiguities, and implementation blockers across all relevant Tier 1-4 documents.

---

## Documents Reviewed

### Tier 1 (Requirements)
- **[REQ001-requirements.md](../../docs/REQ001-requirements.md)** - Core playback requirements ([REQ-CF-*], [REQ-XFD-*])
- **[REQ002-entity_definitions.md](../../docs/REQ002-entity_definitions.md)** - Passage, Song, Recording, Work, Artist ([ENT-*])

### Tier 2 (Design Specifications)
- **[SPEC002-crossfade.md](../../docs/SPEC002-crossfade.md)** - Crossfade timing, curves, behavior ([XFD-*])
- **[SPEC007-api_design.md](../../docs/SPEC007-api_design.md)** - HTTP/SSE API endpoints
- **[SPEC011-event_system.md](../../docs/SPEC011-event_system.md)** - Event emission patterns
- **[SPEC013-single_stream_playback.md](../../docs/SPEC013-single_stream_playback.md)** - Architecture overview ([SSP-*])
- **[SPEC014-single_stream_design.md](../../docs/SPEC014-single_stream_design.md)** - Component structure ([SSD-*])
- **[SPEC016-decoder_buffer_design.md](../../docs/SPEC016-decoder_buffer_design.md)** - Decoder-buffer chain ([DBD-*])
- **[SPEC017-sample_rate_conversion.md](../../docs/SPEC017-sample_rate_conversion.md)** - Tick-based timing ([SRC-*])
- **[SPEC018-crossfade_completion_coordination.md](../../docs/SPEC018-crossfade_completion_coordination.md)** - Mixer-to-engine signaling ([XFD-COMP-*])

### Tier 3 (Implementation)
- **[IMPL001-database_schema.md](../../docs/IMPL001-database_schema.md)** - Queue, passages, settings tables
- **[IMPL002-coding_conventions.md](../../docs/IMPL002-coding_conventions.md)** - Rust coding standards
- **[IMPL003-project_structure.md](../../docs/IMPL003-project_structure.md)** - Module organization

### Tier 4 (Execution)
- **[EXEC001-implementation_order.md](../../docs/EXEC001-implementation_order.md)** - Build sequence
- **[GUIDE001-wkmp_ap_implementation_plan.md](../../docs/GUIDE001-wkmp_ap_implementation_plan.md)** - Phased implementation roadmap

---

## Specification Completeness Assessment

### ✅ WELL-SPECIFIED AREAS (Ready for Implementation)

#### 1. Tick-Based Timing System (SPEC017)
**Readiness:** EXCELLENT - Formula-complete, mathematically precise

**Specifications:**
- Tick rate: 28,224,000 Hz (LCM of all supported sample rates) [SRC-TICK-020]
- One tick = 35.4 nanoseconds [SRC-TICK-030]
- Sample-to-tick conversion formulas provided [SRC-CONV-030]
- Ticks-per-sample table for common rates [SRC-CONV-020]

**Implementation Clarity:**
- Rust data type: `i64` (ticks)
- Conversion functions directly implementable from formulas
- Database storage: INTEGER (ticks) [SRC-DB-*]

**Assessment:** Can implement immediately with zero ambiguity.

---

#### 2. Fade Curve Algorithms (SPEC002, SPEC013)
**Readiness:** EXCELLENT - Mathematical formulas provided

**Specifications:**
- 5 fade curve types: Linear, Exponential, Logarithmic, S-Curve, Equal-Power
- Formulas for each curve type [XFD-IMPL-091] through [XFD-IMPL-096]
- Independent fade-in and fade-out curve selection [XFD-CURV-040]
- Per-passage configuration via database [IMPL001 queue table]

**Implementation Clarity:**
- Input: position (samples), duration (samples)
- Output: gain (0.0 to 1.0)
- Pure functions, easily testable

**Assessment:** Can implement and unit test immediately.

---

#### 3. Crossfade Timing Model (SPEC002)
**Readiness:** EXCELLENT - Comprehensive 6-point model

**Specifications:**
- 6 timing points: Start, Fade-In, Lead-In, Lead-Out, Fade-Out, End [XFD-PT-*]
- Constraint chains [XFD-CONS-010] through [XFD-CONS-030]
- Zero-duration behavior [XFD-OV-020]
- Overlap scenarios (Case 1 and Case 2) [XFD-BEH-C1-010], [XFD-BEH-C2-010]

**Implementation Clarity:**
- Database storage defined [IMPL001 passages table, SPEC017 SRC-DB-*]
- Validation logic derivable from constraints
- Overlap calculation formulas provided

**Assessment:** Can implement timing calculations with high confidence.

---

#### 4. Entity Model (REQ002)
**Readiness:** EXCELLENT - Clear definitions aligned with MusicBrainz

**Specifications:**
- Passage [ENT-MP-030]: Audio file span with timing points
- Song [ENT-MP-010]: Recording + Works + Artists with weights
- Recording, Work, Artist [ENT-MB-*]: MusicBrainz-aligned definitions
- Relationships [ENT-REL-*]: Well-defined cardinalities

**Implementation Clarity:**
- Rust struct definitions directly derivable
- Database schema comprehensive [IMPL001]
- Foreign key relationships specified

**Assessment:** Can implement entity models immediately.

---

#### 5. Database Schema (IMPL001)
**Readiness:** EXCELLENT - Comprehensive table definitions

**Specifications:**
- Queue table with all columns, constraints, indexes
- Passages table with timing points (ticks)
- Settings table with operating parameters
- Triggers for timestamp updates [IMPL001]
- Foreign key cascades defined

**Implementation Clarity:**
- SQL CREATE TABLE statements essentially ready
- Migrations strategy defined (created on first startup)
- Default values specified

**Assessment:** Can implement database layer immediately.

---

### ⚠️ PARTIAL SPECIFICATIONS (Missing Details)

#### 6. Decoder-Buffer Chain Lifecycle (SPEC016)
**Readiness:** GOOD - Lifecycle clear, some terminology fuzzy

**Well-Specified:**
- Chain assignment triggers [DBD-LIFECYCLE-010]
- Chain release conditions [DBD-LIFECYCLE-020]
- Chain allocation strategy (min-heap) [DBD-LIFECYCLE-030]
- HashMap tracking [DBD-LIFECYCLE-040]

**Gaps:**
- Relationship between `PassageBuffer`, `ManagedBuffer`, `DecoderChain` unclear
- SPEC016 line 112: "PassageBuffer wrapped in ManagedBuffer" but ManagedBuffer not defined
- Are these three separate types or aliases?
- Which type is used at which layer?

**Impact:** LOW - Naming conventions, likely resolvable from implementation context

**Workaround:** Assume PassageBuffer is core data structure, ManagedBuffer adds lifecycle, DecoderChain integrates pipeline

---

#### 7. Queue Persistence Strategy (IMPL001, SPEC016)
**Readiness:** FAIR - Database schema clear, persistence timing unclear

**Well-Specified:**
- Queue table structure [IMPL001]
- play_order column for queue sequencing
- Runtime chain assignment HashMap [DBD-LIFECYCLE-040]

**Gaps:**
- When is queue persisted to database?
  - On every enqueue operation?
  - Periodically (every N seconds)?
  - Only on shutdown?
- How is runtime chain assignment state reconciled with database on restart?
  - [DBD-LIFECYCLE-060] mentions "database restore" but doesn't specify reconciliation logic
- What happens if database queue and runtime queue diverge?

**Impact:** MEDIUM - Affects restart behavior and state consistency

**Workaround:** Implement eager persistence (every enqueue/dequeue writes to database) + startup reconciliation that rebuilds chain assignments

---

#### 8. Full vs Partial Buffer Strategy (SPEC016, SPEC014)
**Readiness:** FAIR - Buffer size specified, decode strategy unclear

**Well-Specified:**
- playout_ringbuffer_size = 661941 samples (15.01s @ 44.1kHz) [DBD-PARAM-070]
- Per-buffer memory: ~5.3 MB
- Total memory for 12 buffers: ~60 MB

**Gaps:**
- SPEC014 line 62 mentions "Full/partial buffer strategy"
- When to fully decode passage into buffer vs incremental decode?
- Decision criteria not specified:
  - Always full buffer for passages <15s?
  - Incremental decode for long passages?
  - Based on queue depth (immediate playback vs prefetch)?
- Incremental decode resumption logic not specified

**Impact:** MEDIUM - Affects memory efficiency and startup latency

**Workaround:** Implement incremental decode for all passages (pause when buffer full, resume when space available)

---

#### 9. Resampler State Management (SPEC016)
**Readiness:** FAIR - Resampling behavior specified, state details unclear

**Well-Specified:**
- Use rubato StatefulResampler [DBD-RSMP-010]
- Bypass when source_rate == working_sample_rate [DBD-RSMP-020]
- Integrate into decoder chain [DBD-RSMP-030]

**Gaps:**
- State initialization details (rubato-specific)
- Flush behavior when stream ends (prevent buffer tail loss)
- Edge case: What if sample rate changes mid-file? (unlikely but possible)

**Impact:** LOW - Implementation details likely inferrable from rubato library documentation

**Workaround:** Consult rubato documentation for StatefulResampler usage patterns

---

### ❌ CRITICAL GAPS (Implementation Blockers)

#### 10. SPEC018 Status and Approval (BLOCKER)
**Readiness:** UNKNOWN - Draft status concerning

**Problem Identified:**
- SPEC018 identifies critical gap: Mixer completes crossfade internally but engine never notified
- Consequence: Queue advancement stops/restarts incoming passage incorrectly
- SPEC018 proposes solution (mixer-to-engine completion signaling)

**Gaps:**
- Document status: "Draft → Implementation" (line 6)
- Is this solution approved?
- Is it already implemented in current codebase?
- If not implemented, which specific approach should be used?

**Impact:** CRITICAL - Incorrect queue advancement is a **blocking bug**

**Blocker Rationale:**
- Cannot implement queue processing logic without knowing crossfade completion signaling design
- Guessing at solution may conflict with unstated design intent
- Re-work risk: HIGH if wrong approach implemented

**Required Resolution:**
1. Clarify SPEC018 status: Draft or Approved?
2. If Draft: Conduct formal review and approval
3. If Approved: Update status in document
4. If already implemented: Document actual implementation approach

---

#### 11. Error Handling Strategy (HIGH RISK)
**Readiness:** NONE - Completely unspecified

**Scenarios Requiring Error Handling:**
1. **Decode Failures:**
   - File corrupted or unreadable
   - Unsupported codec variant
   - Partial decode (file truncated)
   - **Action:** Skip passage? Retry? User notification? Event emission?

2. **Buffer Underruns:**
   - Decoder too slow to keep buffer filled
   - CPU overload or I/O stall
   - **Action:** Pause playback? Insert silence? Emergency buffer refill?

3. **Audio Device Failures:**
   - Device disconnected (Bluetooth, HDMI)
   - Device becomes unavailable (another app takes exclusive access)
   - **Action:** Pause? Reconnect? Fallback to different device? User alert?

4. **Queue Inconsistencies:**
   - Passage referenced in queue but not in database
   - File path invalid (file moved/deleted)
   - Chain assignment impossible (all chains busy, queue > maximum_decode_streams)
   - **Action:** Remove from queue? Skip? User notification?

5. **Sample Rate Conversion Errors:**
   - Rubato resampler failure
   - Invalid sample rate from decoder
   - **Action:** Bypass resampling? Skip passage? Log and continue?

**Impact:** HIGH RISK - Production system requires comprehensive error handling

**Gaps:**
- No specifications for:
  - Error detection mechanisms
  - Error recovery strategies
  - User notification patterns
  - Event emission for error conditions
  - Graceful degradation behavior
  - Logging/telemetry requirements

**Blocker Rationale:**
- Ad-hoc error handling leads to inconsistent behavior
- Missing errors cause crashes or silent failures
- No testing possible without error scenarios specified

**Required Specification:**
- Error taxonomy (categorize by severity and recoverability)
- Per-category handling strategies
- Event definitions for error conditions
- User notification triggers
- Logging requirements
- Graceful degradation behaviors

---

#### 12. Performance Targets (Cannot Validate Success)
**Readiness:** NONE - No quantified specifications

**Context:**
- WKMP targets Raspberry Pi Zero 2W deployment [SPEC014 SSD-DEC-030]
- Pi Zero 2W: 1 GHz quad-core ARMv8, 512 MB RAM
- Limited CPU and memory compared to desktop

**Missing Specifications:**
1. **Decode Latency:**
   - What is acceptable buffer fill time?
   - Target: Buffer fills in <500ms? <1s? <2s?
   - Maximum tolerable latency before playback starts?

2. **CPU Usage:**
   - Maximum acceptable CPU percentage?
   - Target: <50% average? <80% peak?
   - Per-core or aggregate?

3. **Memory Limits:**
   - Maximum total memory usage?
   - SPEC016 calculates 60 MB for 12 buffers, but is that total app memory?
   - Target: Total app <100 MB? <200 MB?

4. **Throughput:**
   - How many passages can be decoded per minute?
   - Queue refill rate requirement?

5. **Response Time:**
   - API endpoint response time targets?
   - Skip command latency requirement?
   - UI update delay tolerance?

**Impact:** MEDIUM-HIGH - Cannot validate implementation success without targets

**Gaps:**
- No quantified acceptance criteria
- Cannot perform performance testing without benchmarks
- Cannot detect regressions
- Cannot validate Pi Zero 2W suitability

**Workaround:** Implement without targets, then measure empirically on Pi Zero 2W and set targets based on observed performance

---

#### 13. SPEC014 vs SPEC016 Contradiction
**Readiness:** CONFLICTING - Outdated content may mislead implementers

**Contradiction:**
- **SPEC014 lines 26-106:** Describes parallel 2-thread decoder pool
  - "Fixed pool: 2 decoder threads"
  - "Rationale: Sufficient for current + next passage full decode"
  - Thread creation, priority queue management specified
- **SPEC016 [DBD-DEC-040]:** Specifies serial decode execution
  - "one decoder at a time"
  - "serial decoding approach for improved cache coherency"

**Clarification in SPEC014:**
- Line 76 note: "Design evolved to serial decode execution (SPEC016 [DBD-DEC-040])"
- Line 78 note: "This section describes the original 2-thread pool design"

**Impact:** MEDIUM - Misleading if implementer reads SPEC014 first without seeing notes

**Problem:**
- SPEC014 contains detailed design for approach that is no longer used
- Implementer may waste time implementing 2-thread pool before discovering it's obsolete
- SPEC016 is authoritative per note, but SPEC014 not updated to match

**Required Resolution:**
1. **Option A:** Update SPEC014 to describe serial decode design matching SPEC016
2. **Option B:** Move SPEC014 parallel decode content to archive and add forward reference to SPEC016
3. **Option C:** Add prominent notice at top of SPEC014 that SPEC016 supersedes decoder design sections

---

## Specification Readiness Summary

### By Component

| Component | Readiness | Can Implement? | Blocker |
|-----------|-----------|----------------|---------|
| Tick-based timing | ✅ EXCELLENT | Yes | None |
| Fade curves | ✅ EXCELLENT | Yes | None |
| Crossfade timing | ✅ EXCELLENT | Yes | None |
| Entity model | ✅ EXCELLENT | Yes | None |
| Database schema | ✅ EXCELLENT | Yes | None |
| Decoder-buffer chain | ⚠️ GOOD | Yes | Terminology minor |
| Queue persistence | ⚠️ FAIR | Yes | Timing unclear |
| Buffer strategy | ⚠️ FAIR | Yes | Incremental vs full |
| Resampler state | ⚠️ FAIR | Yes | Library-specific |
| **Crossfade completion** | ❌ **UNKNOWN** | **NO** | **SPEC018 status** |
| **Error handling** | ❌ **NONE** | **NO** | **Unspecified** |
| **Performance targets** | ❌ **NONE** | **Risky** | **No validation** |
| SPEC014 decoder | ❌ **CONFLICT** | **Confusing** | **Outdated content** |

### Overall Assessment

**Can wkmp-ap be implemented from current specifications?**

**ANSWER: PARTIALLY**

- **60% of implementation** can proceed with high confidence (core audio algorithms, database, entity model)
- **30% of implementation** can proceed with workarounds (chain lifecycle, queue persistence, buffer strategy)
- **10% of implementation** is **blocked or high-risk** (crossfade completion, error handling, performance validation)

**For production-quality implementation:** Must resolve critical gaps before coding begins.

**For prototype/experimental implementation:** Could proceed with core audio pipeline, defer error handling and performance optimization.

---

## Recommendations

### Immediate Actions Required

1. **Resolve SPEC018 status** (CRITICAL)
   - Conduct formal review of SPEC018 crossfade completion solution
   - Approve or revise specification
   - Update document status
   - If already implemented, document actual implementation

2. **Specify error handling strategy** (HIGH PRIORITY)
   - Create comprehensive error taxonomy
   - Define handling strategies per error category
   - Specify event emissions for error conditions
   - Define user notification triggers

3. **Update or archive SPEC014** (MEDIUM PRIORITY)
   - Resolve contradiction with SPEC016
   - Either update to match serial decode or archive parallel decode content
   - Prevent implementer confusion

### Recommended Before Implementation

4. **Define performance targets** (MEDIUM PRIORITY)
   - Quantify acceptable latency, CPU %, memory usage
   - Create Pi Zero 2W-specific benchmarks
   - Enable validation of implementation success

5. **Clarify queue persistence strategy** (LOW PRIORITY)
   - Specify when queue is written to database
   - Define restart reconciliation logic
   - Ensure state consistency

6. **Specify buffer decode strategy** (LOW PRIORITY)
   - Clarify full vs incremental decode decision logic
   - Specify resumption behavior
   - Optimize for memory vs latency trade-off

---

**Section Complete**

**Next Section:** [02_approach_comparison.md](02_approach_comparison.md) - Detailed comparison of 4 implementation approaches

**Return to Summary:** [00_ANALYSIS_SUMMARY.md](00_ANALYSIS_SUMMARY.md)
