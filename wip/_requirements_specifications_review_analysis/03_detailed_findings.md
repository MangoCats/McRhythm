# Detailed Findings: Gap Analysis by Component

**Section:** Technical Details on Each Specification Gap
**Parent Document:** [00_ANALYSIS_SUMMARY.md](00_ANALYSIS_SUMMARY.md)

---

## Purpose

This document provides technical details on each gap, ambiguity, and contradiction identified in wkmp-ap specifications, organized by component area.

---

## FINDING 1: SPEC018 Status Unclear (CRITICAL BLOCKER)

### Summary

SPEC018 (Crossfade Completion Coordination) identifies critical gap in mixer-to-engine communication but has "Draft → Implementation" status, unclear if solution is approved/implemented.

### Document References

- **[SPEC018-crossfade_completion_coordination.md](../../docs/SPEC018-crossfade_completion_coordination.md)** (lines 1-100)
- Status field (line 6): "Draft → Implementation"

### Problem Description

**Background (SPEC018 lines 49-90):**

Mixer has three states (SPEC002):
1. `None` - No audio playing
2. `SinglePassage` - One passage playing
3. `Crossfading` - Two passages overlapping

**Current behavior:**
- When crossfade completes (fade-out and fade-in both finish), mixer internally transitions from `Crossfading` to `SinglePassage`
- Incoming passage becomes new current passage in mixer's internal state
- **Gap:** Engine never notified of this transition

**Consequence:**
- Engine's `process_queue()` loop uses `mixer.is_current_finished()` to detect passage completion
- During `Crossfading` state, `is_current_finished()` returns `false`
- Engine never knows when outgoing passage finishes
- Engine attempts queue advancement based on wrong state
- Result: Stops and restarts incoming passage (BUG-003)

**SPEC018 Proposed Solution (lines 94-100):**
- Implement explicit crossfade completion signaling from mixer to engine
- Allow queue advancement without interrupting incoming passage

### Gap Details

**What's Missing:**
1. Is SPEC018 solution approved for implementation?
2. Is it already implemented in current codebase?
3. Which specific signaling mechanism should be used?
   - Option A: Mixer returns completion event on each `mix()` call
   - Option B: Mixer emits completion via event bus
   - Option C: Engine polls mixer for completion status
   - Option D: (Other approach)

**Why It's Critical:**
- Cannot implement queue advancement logic without knowing crossfade completion design
- Guessing wrong approach requires significant refactoring
- Queue advancement is core functionality - must work correctly

### Impact Assessment

**Affected Components:**
- `playback/engine.rs` - Queue processing loop
- `playback/pipeline/mixer.rs` - State machine and completion signaling
- `playback/queue_manager.rs` - Queue entry lifecycle

**Blocker Rationale:**
- Implementation of queue advancement logic depends entirely on crossfade completion design
- Cannot proceed with engine implementation without resolution

### Recommended Resolution

1. **Conduct formal review of SPEC018:**
   - Evaluate proposed solution
   - Consider signaling mechanism options
   - Assess impact on architecture

2. **Make decision:**
   - Approve SPEC018 as-is, OR
   - Revise SPEC018 with specific signaling mechanism, OR
   - Reject and propose alternative solution

3. **Update document status:**
   - Change from "Draft → Implementation" to "Approved" with version number
   - If revised, increment version and update content

4. **Verify implementation state:**
   - Check if solution already implemented in current codebase
   - If so, document actual implementation approach
   - If not, implementation can proceed per approved specification

---

## FINDING 2: Error Handling Strategy Unspecified (HIGH RISK)

### Summary

No comprehensive error handling strategy specified for decode failures, buffer underruns, audio device failures, or queue inconsistencies.

### Document References

No existing document specifies error handling strategy. Gap identified across:
- SPEC016 mentions decode failures (line 80) but doesn't specify handling
- SPEC013, SPEC014 do not address error scenarios
- IMPL001 does not specify error event tables or error state columns

### Error Scenario Inventory

#### 1. Decode Failures

**Scenarios:**
- File corrupted or unreadable (I/O error, permission denied)
- Unsupported codec variant (e.g., non-standard MP3 variant)
- Partial decode (file truncated, download incomplete)
- Decoder library panic (symphonia internal error)
- File format mismatch (database says MP3, actually FLAC)

**Current State:** Unspecified

**Questions Requiring Answers:**
- Should playback skip passage and continue with next?
- Should playback pause and await user intervention?
- Should playback retry decode (how many times)?
- Should event be emitted? Which event type?
- Should error be logged? At what level (error, warning)?
- Should user be notified via UI? Immediately or batched?
- Should passage be marked as "decode failed" in database?

**Typical Industry Approach:**
- Log error at ERROR level with file path and error message
- Emit PassageDecodeFailed event with passage_id and error details
- Remove passage from queue
- Notify user via SSE error event
- Continue with next passage in queue
- Optionally mark passage as problematic in database to prevent re-enqueueing

#### 2. Buffer Underruns

**Scenarios:**
- Decoder too slow to fill buffer before mixer exhausts it
- CPU overload (other processes competing for cycles)
- I/O stall (slow SD card, network drive delay)
- Thread scheduling delays (low-priority thread starved)

**Current State:** Unspecified

**Questions Requiring Answers:**
- Should playback pause immediately?
- Should mixer insert silence to prevent audio artifacts?
- Should playback attempt emergency buffer refill?
- How long to wait for buffer refill before declaring failure?
- Should event be emitted?
- Should user see "buffering" indicator?
- Is underrun recoverable or fatal?

**Typical Industry Approach:**
- Pause playback immediately
- Emit BufferUnderrun event
- Attempt emergency decode of current passage
- If refill succeeds within 2-3 seconds, resume playback
- If refill fails, skip to next passage
- Log warning with buffer state details

#### 3. Audio Device Failures

**Scenarios:**
- Bluetooth headphones disconnected
- HDMI monitor turned off (audio device disappears)
- USB DAC unplugged
- Device becomes unavailable (another app takes exclusive access)
- Device configuration error (unsupported sample rate)
- cpal stream error (platform-specific issues)

**Current State:** Unspecified

**Questions Requiring Answers:**
- Should playback pause and await device reconnection?
- Should playback fallback to different device (e.g., built-in speakers)?
- Should user be prompted to select new device?
- How long to retry reconnection before giving up?
- Should playback state (position, queue) be preserved during device failure?
- Should event be emitted?

**Typical Industry Approach:**
- Pause playback immediately
- Emit AudioDeviceLost event
- Attempt reconnection every 1-2 seconds for up to 30 seconds
- If original device reconnects, resume playback
- If timeout, prompt user to select new device
- If user cancels device selection, remain paused

#### 4. Queue Inconsistencies

**Scenarios:**
- Passage referenced in queue but not in passages table
- File path invalid (file moved, deleted, or renamed)
- Chain assignment impossible (all chains busy, queue depth > maximum_decode_streams)
- Passage timing points invalid (constraints violated)
- Circular queue references (should be impossible but defensive)

**Current State:** Partially specified

**SPEC016 [DBD-LIFECYCLE-050]** addresses chain exhaustion:
- When all maximum_decode_streams chains allocated, newly enqueued passages wait without chains
- "Future enhancement" mentioned for assigning chains when available

**Questions Requiring Answers:**
- Should invalid queue entries be removed automatically?
- Should invalid entries trigger user notification?
- Should queue validation run periodically or only at enqueue time?
- How to handle passage without file (skip or remove from queue)?
- Should database referential integrity constraints prevent these scenarios?

**Typical Industry Approach:**
- Validate queue entries on load from database
- Remove invalid entries automatically
- Log each removal at WARNING level
- Emit QueueValidationError event with details
- Optionally notify user of cleanup actions taken

#### 5. Sample Rate Conversion Errors

**Scenarios:**
- Rubato resampler failure (internal error)
- Invalid source sample rate from decoder (0 Hz, negative, extremely high)
- Resampler state corruption
- Insufficient memory for resampler buffers

**Current State:** Unspecified

**Questions Requiring Answers:**
- Should resampling be bypassed if conversion fails?
- Should passage be skipped?
- How to handle sample rate mismatch if bypass chosen?
- Should event be emitted?

**Typical Industry Approach:**
- If source rate == output rate, bypass resampler (already specified [DBD-RSMP-020])
- If source rate != output rate and resampler fails, skip passage
- Emit ResamplingFailed event
- Log error with source/target rates

### Impact Assessment

**Production Risk:** HIGH
- Without error handling strategy, failures cause crashes or silent failures
- User experience poor (unexplained pauses, skips, crashes)
- Debugging difficult (no events, insufficient logging)
- Recovery impossible (no retry or fallback mechanisms)

**Testing Impact:** Cannot Write Tests
- No error scenarios specified means no error test cases
- Cannot validate error behavior
- Cannot detect error handling regressions

**Implementation Impact:** Ad-hoc Decisions
- Developer forced to make error handling decisions during implementation
- Decisions may not match project conventions or user expectations
- Inconsistent error behavior across different error types

### Recommended Resolution

**Create comprehensive error handling strategy specification:**

1. **Error Taxonomy:**
   - Classify errors by severity: Fatal, Recoverable, Degraded
   - Classify errors by category: Decode, Buffer, Device, Queue, Resampling
   - Define recovery strategies per classification

2. **Per-Error Handling Specification:**
   - For each error scenario (5 categories × ~4 scenarios each = 20 scenarios)
   - Specify: Detection mechanism, immediate action, recovery attempts, failure action
   - Specify: Event emission (type, payload), logging (level, message), user notification

3. **Event Definitions:**
   - Define error-related events to add to WkmpEvent enum (SPEC011)
   - Examples: PassageDecodeFailed, BufferUnderrun, AudioDeviceLost, QueueValidationError, ResamplingFailed

4. **User Notification Strategy:**
   - Which errors trigger immediate notification vs logged-only?
   - How are errors presented in UI (modal, toast, status bar)?
   - How are batched errors presented (e.g., 5 decode failures in a row)?

5. **Logging Requirements:**
   - Define log levels per error type (ERROR, WARNING, INFO)
   - Define required log message components (timestamp, component, error details, context)
   - Define structured logging fields for error events

6. **Graceful Degradation:**
   - Define fallback behaviors (e.g., device failure → fallback to default device)
   - Define minimal viable functionality (e.g., playback paused but queue preserved)

7. **Integration with SPEC011 Event System:**
   - Update event_system.md to include error events
   - Specify SSE broadcasting of error events
   - Specify event handler requirements for error recovery

**Estimated Effort:** 1-2 days of specification work

---

## FINDING 3: SPEC014 vs SPEC016 Decoder Contradiction

### Summary

SPEC014 describes parallel 2-thread decoder pool; SPEC016 specifies serial decode execution. Contradiction may mislead implementers.

### Document References

**SPEC014 (outdated content):**
- Lines 26-106: Detailed description of parallel 2-thread decoder pool
- Lines 94-98: Thread creation, priority queue, shutdown behavior
- Lines 91-93: "Fixed pool: 2 decoder threads" rationale

**SPEC014 (clarification notes):**
- Line 76: "Design evolved to serial decode execution (SPEC016 [DBD-DEC-040])"
- Line 78: "This section describes the original 2-thread pool design"
- Line 85: "New design: Serial decode execution with priority-based switching"

**SPEC016 (authoritative):**
- [DBD-DEC-040]: "serial decoding approach (one decoder at a time) for improved cache coherency"
- [DBD-DEC-050] through [DBD-DEC-080]: Serial decode flow specifications

### Problem Description

**Scenario:**
- Implementer reads GUIDE001 or EXEC001 which references SPEC014
- Implementer reads SPEC014 detailed decoder pool description (lines 26-106)
- Implementer begins implementing 2-thread pool
- Later discovers SPEC016 specifies serial decode
- Realizes wasted effort implementing obsolete design

**Why Notes Insufficient:**
- Notes are mid-document (line 76, line 85)
- Easy to miss if skimming or searching for "decoder" or "thread"
- Detailed parallel design (80 lines) more prominent than brief notes
- No prominent warning at top of document

### Impact Assessment

**Misleading Risk:** MEDIUM
- Implementer may waste 1-3 days implementing parallel pool before discovering obsolescence
- Not critical to correctness (would eventually discover and fix)
- But frustrating and inefficient

**Documentation Clarity:** POOR
- Contradictory content in specification documents undermines trust
- Makes WKMP documentation appear less rigorous than it actually is

### Recommended Resolution

**Option A: Update SPEC014 to Match SPEC016 (Preferred)**

1. Replace parallel decoder pool sections (lines 26-106) with serial decode description matching SPEC016
2. Move parallel decoder pool content to archive (e.g., archive/SPEC014-parallel-decoder-design.md)
3. Add note in SPEC014: "Historical parallel decoder pool design archived; see SPEC016 for current authoritative decoder design"
4. Update SPEC014 to reference SPEC016 for decoder implementation

**Benefits:**
- SPEC014 becomes consistent with SPEC016
- Historical design preserved in archive for reference
- Clear forward reference to authoritative spec

**Effort:** 2-4 hours

**Option B: Add Prominent Warning at Top of SPEC014**

1. Add section at top of document (after metadata, before Overview):
   ```markdown
   ## ⚠️ IMPORTANT: Decoder Design Superseded

   **The decoder pool design described in this document (parallel 2-thread pool) is OBSOLETE.**

   **Current authoritative decoder design:** See [SPEC016 Decoder Buffer Design](SPEC016-decoder_buffer_design.md) [DBD-DEC-040] for serial decode execution.

   **This document remains for historical reference and contains valuable information on other components (buffer manager, mixer, output). For decoder implementation, use SPEC016.**
   ```

2. Add similar warning before "1. Decoder Thread Pool" section (line 26)

**Benefits:**
- Minimal effort
- Preserves full historical context
- Clear warning prevents confusion

**Effort:** 30 minutes

**Option C: Archive SPEC014, Forward to SPEC016**

1. Move SPEC014 to archive
2. Create stub SPEC014 that forwards to SPEC016, SPEC013
3. Update references in other documents

**Benefits:**
- Eliminates contradiction entirely
- Forces use of current specifications

**Drawbacks:**
- Loses other valuable content in SPEC014 (buffer manager, mixer descriptions)
- More disruptive to documentation structure

**Effort:** 1-2 hours

**RECOMMENDATION: Option B (Prominent Warning)**
- Lowest effort
- Preserves content
- Prevents confusion
- Can upgrade to Option A later if desired

---

## FINDING 4: Performance Targets Unspecified

### Summary

No quantified performance specifications despite targeting Raspberry Pi Zero 2W deployment.

### Document References

**Pi Zero 2W Reference:**
- SPEC014 [SSD-DEC-030]: "Raspberry Pi Zero2W resource limits (REQ-TECH-011)"
- REQ001 references Pi Zero 2W as target platform

**Hardware Constraints:**
- **CPU:** 1 GHz quad-core Cortex-A53 (ARMv8)
- **RAM:** 512 MB
- **Storage:** Typically SD card (10-20 MB/s read speed)
- **Architecture:** 64-bit ARM

### Missing Specifications

#### 1. Decode Latency Targets

**What Should Be Specified:**
- Time to fill 15-second buffer (playout_ringbuffer_size = 661941 samples @ 44.1kHz)
- Target: Buffer fills in <1000ms? <500ms?
- Maximum acceptable latency before playback starts
- Recovery time after buffer underrun

**Why It Matters:**
- User experience (how long after hitting play does music start?)
- Queue refill responsiveness (how quickly can next passage be readied?)
- Crossfade quality (buffer must fill before lead-out point or crossfade fails)

**Typical Targets (Desktop):**
- Immediate playback: Buffer fills in <200ms
- Normal playback: Buffer fills in <500ms
- Acceptable: Buffer fills in <1000ms
- Poor: Buffer fills in >1000ms

**Pi Zero 2W Expectations:**
- Likely 2-3x slower than desktop due to ARM CPU and SD card I/O
- Reasonable target: Buffer fills in <1500ms for 44.1kHz FLAC
- Conservative target: Buffer fills in <2000ms

#### 2. CPU Usage Targets

**What Should Be Specified:**
- Maximum acceptable CPU percentage during playback
- Per-core or aggregate?
- Average vs peak?

**Why It Matters:**
- Pi Zero 2W has limited CPU headroom
- Must leave CPU available for UI, API, other modules
- High CPU usage may cause thermal throttling
- Affects feasibility of running multiple WKMP modules on same Pi

**Typical Targets (Desktop):**
- Idle playback (no decode): <5% CPU
- Active decode: <20% CPU average, <40% peak
- Crossfade: <30% CPU average, <50% peak

**Pi Zero 2W Expectations:**
- Single-core performance lower than desktop
- Quad-core available but most audio code single-threaded
- Reasonable target: <50% average, <80% peak (single core)
- Conservative target: <60% average, <90% peak

#### 3. Memory Usage Targets

**What Should Be Specified:**
- Maximum total application memory usage
- Breakdown by component (buffers, decoder state, API, etc.)

**Why It Matters:**
- Pi Zero 2W has only 512 MB total RAM
- Must share with OS, other modules, browser if UI accessed locally
- Memory exhaustion causes OOM killer, crashes
- SPEC016 calculates 60 MB for 12 buffers but is that total app usage?

**Current State:**
- SPEC016 [DBD-PARAM-070]: playout_ringbuffer_size = 661941 samples (15.01s)
- Per buffer: 661941 samples × 2 channels × 4 bytes/f32 = 5,295,528 bytes ≈ 5.3 MB
- 12 buffers (maximum_decode_streams): 12 × 5.3 MB = 63.6 MB for PCM buffers
- Plus decoder state, resampler state, queue, API, event bus, etc.

**Typical Targets (Desktop):**
- Lightweight media player: <100 MB
- Full-featured media player: <200 MB
- Heavy media player: <500 MB

**Pi Zero 2W Expectations:**
- Must be lightweight given 512 MB total RAM
- Reasonable target: Total app <150 MB
- Conservative target: Total app <200 MB
- Critical if exceeded: >250 MB (risk of OOM)

#### 4. Throughput Targets

**What Should Be Specified:**
- How many passages can be decoded per minute?
- Queue refill rate (passages/second)
- Minimum acceptable program director selection speed

**Why It Matters:**
- If decoder is too slow, queue empties faster than it refills
- Program director may select passages faster than decoder can prepare them
- User manually enqueueing multiple passages may experience delays

**Calculation Example:**
- Average passage length: 3 minutes = 180 seconds
- Average file size: 5 MB (FLAC) or 3 MB (MP3)
- Decode time per passage: ?
- If decode time > average passage length, queue will eventually empty

**Typical Desktop Performance:**
- FLAC decode: ~10-20x real-time (decode 3-minute file in 9-18 seconds)
- MP3 decode: ~30-50x real-time (decode 3-minute file in 3-6 seconds)

**Pi Zero 2W Expectations:**
- Likely 3-5x slower than desktop
- FLAC: ~3-6x real-time (decode 3-minute file in 30-60 seconds)
- MP3: ~10-15x real-time (decode 3-minute file in 12-18 seconds)
- Reasonable target: Decode 1 passage per minute
- Sufficient if queue depth ≥ 3 and passage length ≥ 3 minutes

#### 5. API Response Time Targets

**What Should Be Specified:**
- Maximum acceptable response time for control endpoints
- Percentile targets (p50, p95, p99)

**Why It Matters:**
- User experience (UI responsiveness)
- Mobile app tolerance for high latency low
- SSE client timeout considerations

**Typical Targets:**
- Simple GET requests (status, queue): <10ms p50, <30ms p95
- POST requests (enqueue, skip): <50ms p50, <100ms p95
- Complex operations: <200ms p50, <500ms p95

**Pi Zero 2W Expectations:**
- Similar to desktop (API logic simple, not CPU-bound)
- Reasonable target: <50ms p50, <150ms p95

### Impact Assessment

**Cannot Validate Success:** HIGH IMPACT
- Without targets, cannot determine if implementation succeeds
- Cannot perform acceptance testing
- Cannot detect performance regressions
- Cannot validate Pi Zero 2W deployment feasibility

**Development Guidance:** MEDIUM IMPACT
- Developers don't know what's acceptable
- May over-optimize (waste time) or under-optimize (poor performance)

**User Expectations:** MEDIUM IMPACT
- Cannot set user expectations (how fast will this be?)
- Cannot make deployment hardware recommendations

### Recommended Resolution

**Create performance target specification:**

1. **Research Pi Zero 2W Capabilities:**
   - Review existing audio applications on Pi Zero 2W
   - Benchmark symphonia/rubato on ARM architecture
   - Understand SD card I/O characteristics

2. **Define Quantified Targets:**
   - Decode latency: <1500ms for 15s buffer fill (44.1kHz FLAC)
   - CPU usage: <50% average, <80% peak (single core)
   - Memory usage: <150 MB total application
   - Throughput: ≥1 passage decoded per minute
   - API response: <50ms p50, <150ms p95

3. **Create Performance Test Specifications:**
   - Define test scenarios (decode 100 passages, measure time)
   - Define measurement methodologies (CPU profiling tools, memory tracking)
   - Define acceptance criteria (must meet targets on 90% of test runs)

4. **Document Targets in SPEC016 or New SPEC###:**
   - Add "Performance Targets" section to SPEC016
   - Or create new SPEC### document for performance specifications

**Estimated Effort:** 2-3 days (1 day research, 1 day target definition, 0.5 days test spec, 0.5 days documentation)

---

## FINDING 5: Queue Persistence Strategy ~~Unclear~~ **RESOLVED**

### Summary

~~Database schema defines queue table; runtime uses HashMap for chain assignments. When/how is state persisted? How is consistency maintained across restarts?~~

**STATUS: RESOLVED (2025-10-25)**
- SPEC007 now specifies eager persistence on enqueue ([API-QUEUE-PERSIST-010](../../docs/SPEC007-api_design.md#L895-L900))
- SPEC016 now specifies startup reconciliation logic ([DBD-STARTUP-010](../../docs/SPEC016-decoder_buffer_design.md#L112-L137))
- All three gap questions addressed with specifications

### Document References

**Database Schema (IMPL001):**
- Queue table defined with columns: guid, passage_guid, user_guid, play_order, enqueued_at, etc.
- Foreign key constraints to passages table

**Runtime State (SPEC016):**
- [DBD-LIFECYCLE-040]: "PlaybackEngine maintains HashMap<QueueEntryId, ChainIndex> for passage→chain mapping"
- Chain assignments persist throughout passage lifecycle

**Queue Persistence (SPEC007 - NEW):**
- [API-QUEUE-PERSIST-010]: "Queue entry persists to database immediately on successful enqueue"
- Write occurs after validation passes but before response sent to client
- Ensures queue state survives crashes (eventual consistency acceptable)

**Startup Reconciliation (SPEC016 - NEW):**
- [DBD-STARTUP-010]: Complete startup restoration procedure defined
- [DBD-STARTUP-020]: Queue corruption recovery specified
- [DBD-STARTUP-030]: Consistency guarantees documented

### ~~Gap Details~~ Resolution Details

**Question 1: When is queue persisted to database? ✅ RESOLVED**

**Answer (SPEC007 [API-QUEUE-PERSIST-010]):** Eager persistence - queue entry persists to database immediately on successful enqueue
- Write occurs after validation passes but before response sent to client
- Dequeue operations also persist immediately (already specified in SPEC007:919)
- Ensures queue state survives crashes (eventual consistency acceptable)

**Question 2: How is chain assignment state reconciled on restart? ✅ RESOLVED**

**Answer (SPEC016 [DBD-STARTUP-010]):** Complete 5-step startup restoration procedure:
1. Load queue entries from database (ORDER BY play_order ASC)
2. Validate each entry:
   - Check passage_guid exists in passages table
   - Check file_path exists on filesystem
   - Remove invalid entries with log warnings
3. Assign decoder chains to valid entries per [DBD-LIFECYCLE-010]
4. Rebuild HashMap<QueueEntryId, ChainIndex> for passage→chain mapping
5. Emit QueueChanged SSE event with trigger "startup_restore"

**Question 3: Are chain assignments persisted? ✅ RESOLVED**

**Answer:** Chain assignments are **runtime-only** (not persisted)
- On restart, rebuild based on queue position per [DBD-LIFECYCLE-010]
- First `maximum_decode_streams` passages get chains
- Simple approach, no database schema changes needed
- Consistent with [DBD-LIFECYCLE-040] which specifies HashMap (in-memory) tracking

**Question 4: What if database queue and runtime queue diverge? ✅ RESOLVED**

**Answer (SPEC016 [DBD-STARTUP-020]):** Queue corruption recovery specified:
- If database queue table is corrupted: Clear queue entirely
- Log error: "Queue table corrupted, clearing all entries"
- Emit QueueChanged SSE event with trigger "corruption_recovery"
- System continues with empty queue (user can re-enqueue passages via UI)
- Seamless recovery: Invalid entries removed transparently with log messages only

### ~~Impact Assessment~~ Resolution Impact

**Restart Behavior:** ✅ RESOLVED
- ~~Unclear how queue is restored on restart~~
- Now specified: 5-step restoration with validation and seamless error recovery
- User experience: Transparent recovery (invalid entries removed with log messages only)

**State Consistency:** ✅ RESOLVED
- ~~Divergence between database and runtime state possible~~
- Now specified: Database is source of truth, corruption recovery clears queue
- Eventual consistency acceptable per [DBD-STARTUP-030]

**I/O Performance:** ✅ ACCEPTABLE
- Eager persistence confirmed (enqueue/dequeue write immediately)
- I/O overhead acceptable (queue operations infrequent compared to audio reads)
- No SD card wear concerns (typical queue: 3-10 entries, not hundreds)

### ~~Recommended Resolution~~ Implementation Notes

**Specifications now complete for queue persistence:**

1. **Persistence Timing ✅ SPECIFIED**
   - SPEC007 [API-QUEUE-PERSIST-010]: Eager persistence on enqueue
   - SPEC007:919: Eager persistence on dequeue (already existed)
   - Implementation: Every enqueue/dequeue operation writes to database immediately

2. **Restart Reconciliation ✅ SPECIFIED**
   - SPEC016 [DBD-STARTUP-010]: Complete 5-step restoration procedure
   - SPEC016 [DBD-STARTUP-020]: Queue corruption recovery
   - Chain assignments NOT persisted (runtime-only, rebuilt on startup)

3. **Consistency Guarantees ✅ SPECIFIED**
   - SPEC016 [DBD-STARTUP-030]: Eventual consistency acceptable
   - Database is source of truth for queue contents and order
   - Runtime HashMap is source of truth for chain assignments
   - Seamless user experience (invalid entries removed transparently)

4. **Documentation Updates ✅ COMPLETE**
   - SPEC007 updated with [API-QUEUE-PERSIST-010]
   - SPEC016 updated with [DBD-STARTUP-010], [DBD-STARTUP-020], [DBD-STARTUP-030]
   - Cross-references added between SPEC007 and SPEC016

**Actual Effort:** ~2 hours (specification + documentation update)

---

## FINDING 6: Full vs Partial Buffer Strategy Unspecified

### Summary

SPEC016 references "full/partial buffer strategy" but doesn't specify decision logic for when to fully decode vs incrementally decode passages.

### Document References

**SPEC016:**
- Line 62 mentions "Full/partial buffer strategy"
- [DBD-PARAM-070]: playout_ringbuffer_size = 661941 samples (15.01s @ 44.1kHz)
- Incremental decode behavior implied (pause when full, resume when space) but not explicitly specified

**SPEC014:**
- References buffer management but doesn't detail full vs partial decision

### Gap Details

**Question: When to fully decode passage vs incremental decode?**

**Option A: Always Incremental**
- Decode all passages incrementally
- Pause decode when buffer reaches playout_ringbuffer_size
- Resume decode when buffer space available (mixer has consumed samples)
- Pros: Consistent behavior, simple logic, memory-efficient
- Cons: May add latency for short passages that could fit in memory entirely

**Option B: Full for Short, Incremental for Long**
- If passage duration < playout_ringbuffer_size (15s), decode fully into buffer
- If passage duration ≥ playout_ringbuffer_size, decode incrementally
- Pros: Short passages fully buffered (faster startup), long passages memory-efficient
- Cons: More complex logic, two code paths to maintain

**Option C: Based on Queue Depth**
- If passage is currently playing or next: Full decode (priority)
- If passage is prefetch (position ≥ 2): Incremental decode
- Pros: Prioritizes playback immediacy
- Cons: Complex, may not save memory (next passage may be long)

**Option D: Full Always (if memory allows)**
- Decode all passages fully into memory
- Allocate dynamic buffer size based on passage duration
- Pros: Simplest for decoder (just decode entire passage)
- Cons: High memory usage (12 passages × average duration could exceed RAM on Pi Zero 2W)

### Impact Assessment

**Memory Efficiency:** MEDIUM IMPACT
- Full decode of 12 long passages could use excessive memory on Pi Zero 2W
- Incremental decode always safe but may add complexity

**Startup Latency:** LOW IMPACT
- Full decode of first passage faster than incremental (no pause/resume cycles)
- But difference likely <1 second, acceptable

**Code Complexity:** LOW IMPACT
- Incremental decode requires pause/resume logic but well within rubato/symphonia capabilities
- Full decode simpler but memory management more complex

### Recommended Resolution

**Specify buffer decode strategy:**

1. **RECOMMENDATION: Always Incremental (Option A)**
   - Simplest, most predictable
   - Memory-safe for all passage lengths
   - Consistent behavior
   - Pause/resume logic required but straightforward

2. **Specification Details:**
   - Decoder decodes passage from start to end
   - Writes decoded PCM to PassageBuffer until buffer full (reached playout_ringbuffer_size)
   - Pauses decode (stores decoder state)
   - Resumes when buffer space available (check every output_refill_period, default 10ms)
   - Repeats until passage end reached

3. **Edge Case: Passage Shorter than Buffer**
   - If passage duration < playout_ringbuffer_size, decode completes in single pass
   - Buffer never reaches full state
   - Effectively equivalent to full decode for short passages

4. **Update SPEC016:**
   - Add section "Buffer Fill Strategy" with incremental decode specification
   - Remove "full/partial buffer strategy" mention or clarify as "always incremental"

**Estimated Effort:** 2-4 hours (specification + documentation)

---

## FINDING 7: Resampler State Management Details Unspecified

### Summary

SPEC016 references StatefulResampler but doesn't specify state initialization, flush behavior, or edge case handling.

### Document References

**SPEC016:**
- [DBD-RSMP-010]: "rubato StatefulResampler maintains resampling state across chunk boundaries"
- [DBD-RSMP-020]: "Bypass when source_rate == working_sample_rate"
- [DBD-RSMP-030]: Integrated into decoder chain

### Gap Details

**Question 1: State Initialization**
- How is StatefulResampler initialized? (call to `new()` with what parameters?)
- When is state reset? (per passage? reused across passages if same sample rate?)

**Question 2: Flush Behavior**
- When passage end reached, how to flush resampler internal buffers?
- Resampler may have buffered samples not yet output
- Without flush, tail samples lost (click/pop at passage end)

**Question 3: Edge Cases**
- What if sample rate changes mid-file? (unlikely but technically possible)
- What if sample rate is unsupported (e.g., 12345 Hz)?
- What if resampler returns error?

### Impact Assessment

**Implementation Clarity:** LOW IMPACT
- These are implementation details likely covered in rubato library documentation
- Experienced developer can infer from library API
- Not architectural decisions

**Correctness:** LOW-MEDIUM IMPACT
- Flush behavior important to avoid tail sample loss
- But likely discoverable during testing (audible click at passage end)

### Recommended Resolution

**Option A: Specify Resampler Usage (Comprehensive)**
- Add detailed rubato StatefulResampler usage specification to SPEC016
- Cover initialization, flush, edge cases
- Effort: 4-6 hours (research rubato API, write spec)

**Option B: Defer to Library Documentation (Minimal)**
- Add note to SPEC016: "Resampler state management follows rubato StatefulResampler API documentation"
- Specify only critical behavior (bypass if source == working rate, flush on passage end)
- Effort: 1-2 hours

**RECOMMENDATION: Option B (Defer to Library)**
- Rubato documentation is comprehensive
- Not WKMP-specific architectural decision
- Reduces specification maintenance burden (rubato API may change)
- Add note to SPEC016 referencing rubato documentation

---

## FINDING 8: Terminology Inconsistencies (PassageBuffer / ManagedBuffer / DecoderChain)

### Summary

SPEC016 references PassageBuffer, ManagedBuffer, and DecoderChain but relationship between these types is unclear.

### Document References

**SPEC016:**
- Line 112: "decoder-buffer chain (design concept) = PassageBuffer (core data structure) wrapped in ManagedBuffer (lifecycle management)"
- [DBD-OV-040]: Diagram shows DecoderChain
- Text uses all three terms somewhat interchangeably

**SPEC014:**
- Line 131: `PassageBuffer` struct definition
- No mention of ManagedBuffer or DecoderChain

### Gap Details

**Unclear Relationships:**
- Is ManagedBuffer a separate type or just conceptual description?
- Is DecoderChain a type or just diagram label?
- Which type is used at which API boundary?

**Possible Interpretations:**

**Interpretation A:**
- `PassageBuffer` = Core struct holding PCM data
- `ManagedBuffer` = Wrapper adding lifecycle (allocation, release) - separate type
- `DecoderChain` = Full pipeline (Decoder → Resampler → Fader → PassageBuffer) - separate type
- Three distinct types

**Interpretation B:**
- `PassageBuffer` = Core struct
- `ManagedBuffer` = Conceptual term (not actual type), just describes PassageBuffer lifecycle
- `DecoderChain` = Conceptual term for entire pipeline
- One type (PassageBuffer), two conceptual terms

**Interpretation C:**
- `DecoderChain` = Actual struct encapsulating entire pipeline
- `PassageBuffer` = Component within DecoderChain
- `ManagedBuffer` = Another component or wrapper
- Multiple types with hierarchical relationship

### Impact Assessment

**Implementation Clarity:** LOW IMPACT
- Unclear naming but doesn't block implementation
- Developer will choose naming scheme and proceed
- May not match SPEC intent but functionally equivalent

**Code Readability:** LOW IMPACT
- Inconsistent naming across codebase
- But can be refactored later without affecting behavior

### Recommended Resolution

**Clarify and document type relationships:**

1. **Define Types Explicitly in SPEC016:**
   - If three distinct types, provide struct definitions
   - If conceptual terms, clearly label as such
   - Example clarification:
     ```markdown
     **Type Definitions:**
     - `PassageBuffer` (struct): Core PCM buffer, fade application, position tracking
     - `DecoderChain` (struct): Encapsulates Decoder → Resampler → Fader → PassageBuffer pipeline
     - "Managed buffer" (conceptual): Refers to PassageBuffer lifecycle managed by BufferManager
       - Not a separate type
       - Just describes how PassageBuffer is allocated/released
     ```

2. **Update SPEC016 Line 112:**
   - Replace ambiguous description with explicit type definitions

**Estimated Effort:** 1-2 hours

---

## Summary of Findings

| # | Finding | Severity | Impact | Effort to Resolve |
|---|---------|----------|--------|-------------------|
| 1 | SPEC018 Status Unclear | CRITICAL | Blocker | 4-8 hours |
| 2 | Error Handling Unspecified | HIGH | Production risk | 1-2 days |
| 3 | SPEC014 vs SPEC016 Contradiction | MEDIUM | Misleading | 0.5-4 hours |
| 4 | Performance Targets Missing | MEDIUM | Cannot validate | 2-3 days |
| 5 | Queue Persistence Unclear | MEDIUM | Restart behavior | 4-8 hours |
| 6 | Buffer Decode Strategy Unspecified | MEDIUM | Memory efficiency | 2-4 hours |
| 7 | Resampler State Management | LOW | Implementation detail | 1-2 hours |
| 8 | Terminology Inconsistencies | LOW | Code readability | 1-2 hours |

**Total Effort to Resolve All Findings:** ~5-8 days

- Critical + High: ~2-3 days
- Medium: ~2-3 days
- Low: ~0.5-1 day

---

**Section Complete**

**Return to Summary:** [00_ANALYSIS_SUMMARY.md](00_ANALYSIS_SUMMARY.md)

**See Also:**
- [01_specification_analysis.md](01_specification_analysis.md) - Specification completeness assessment
- [02_approach_comparison.md](02_approach_comparison.md) - Implementation approach comparison
