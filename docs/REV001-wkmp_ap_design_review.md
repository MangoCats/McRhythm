# wkmp-ap (Audio Player) Design Review Report

**📋 TIER R - REVIEW & CHANGE CONTROL**

Identifies critical design issues requiring project-architect review. This is a historical snapshot (immutable after creation). See [Document Hierarchy](GOV001-document_hierarchy.md) for Tier R classification details.

**Authority:** Historical reference only - Tier 1-4 documents are authoritative for current system state

**Date:** 2025-10-17
**Reviewer:** Claude (docs-specialist)
**Scope:** Architecture, Single-Stream Design, Crossfade, API, Implementation Plan
**Purpose:** Identify critical design issues requiring project-architect review

---

## Executive Summary

**Top 5 Critical Issues:**

1. **[ISSUE-1] Buffer State Transitions Missing from API/Events** - No SSE events or API endpoints expose buffer decode status, creating blind spots for monitoring and debugging buffer underruns.

2. **[ISSUE-2] Passage Timing Validation Logic Gap** - Validation algorithm exists in crossfade.md but implementation responsibility and error recovery strategy unclear across enqueue, decode, and playback phases.

3. **[ISSUE-3] Queue Entry Timing Override Schema Incomplete** - database_schema.md references JSON schema for queue entry timing overrides, but schema definition missing from both database_schema.md and api_design.md.

4. **[ISSUE-4] CurrentSongChanged Event Contradicts Passage Model** - Event emits audio_file_path for passages without passage_id, but database schema requires all playable audio to have passage definitions. Edge case handling inconsistent.

5. **[ISSUE-5] Decoder Thread Pool Lifecycle Unspecified** - single-stream-design.md describes decoder pool but lacks specifications for pool sizing, thread lifecycle, priority queue management, and shutdown behavior.

---

## Critical Issues (Priority 1 - Blockers)

### ISSUE-1: Buffer State Transitions Missing from API/Events
**Severity:** High
**Documents:** single-stream-design.md v1.2, api_design.md, event_system.md

**Problem:**
Single-stream design introduces buffer decode states (Decoding, Ready, Playing, Exhausted) but no SSE events or API endpoints expose this state. Users and monitoring tools cannot detect:
- When passages are still decoding (potential underrun risk)
- Whether partial buffers are in use
- Fade curve application timing issues

**Requirements Impact:**
- Violates [REQ-ERR-010] - developer interface must enable troubleshooting
- Contradicts [SSD-UND-011] through [SSD-UND-026] - buffer underrun warnings require state visibility

**Recommendation:**
Add `BufferStateChanged` SSE event with fields: `passage_id`, `old_state`, `new_state`, `decode_progress_percent`, `timestamp`. Add `GET /playback/buffer_status` endpoint returning decode states for all buffered passages.

---

### ISSUE-2: Passage Timing Validation Logic Gap
**Severity:** High
**Documents:** crossfade.md, api_design.md, single-stream-design.md

**Problem:**
crossfade.md [XFD-IMPL-040] through [XFD-IMPL-043] defines validation algorithm for passage timing (boundary clamping, ordering violations, midpoint correction). Implementation responsibility unspecified:
- Should validation occur on `POST /playback/enqueue` or during decode?
- Who validates passage definitions loaded from database?
- How do validation failures propagate through the system?

single-stream-design.md [SSD-DEC-014] requires "skip samples until reaching passage start time" but doesn't specify behavior when start_time validation failed.

**Requirements Impact:**
- Risks [REQ-ERR-020] - playback errors should skip to next passage
- Contradicts [ARCH-ERRH-010] - audio playback error recovery requires clear failure points

**Recommendation:**
Specify three-phase validation: (1) on enqueue (reject invalid requests with 400), (2) on database read (log warnings, apply corrections), (3) pre-decode validation (emit PassageCompleted with reason="invalid_timing"). Document in both crossfade.md and api_design.md.

---

### ISSUE-3: Queue Entry Timing Override Schema Incomplete
**Severity:** High
**Documents:** database_schema.md, api_design.md

**Problem:**
api_design.md states "Override keyed by generated queue_entry_id" and references "See database_schema.md - Queue Entry Timing Overrides for complete JSON schema". database_schema.md has no section with this heading. Actual storage mechanism unclear:
- Is this a JSON column in the `queue` table?
- A separate `queue_entry_timing_overrides` table?
- A key in the `settings` table (per api_design.md line 843)?

Implementation cannot proceed without knowing storage schema, query patterns, and cleanup triggers.

**Requirements Impact:**
- Blocks [REQ-XFD-020] - per-passage timing configuration
- Prevents implementation of api_design.md `POST /playback/enqueue` timing override feature

**Recommendation:**
Define complete schema in database_schema.md. Recommend JSON column in `queue` table named `timing_override` (nullable) for simple per-entry storage with automatic cleanup on queue entry deletion. Include example JSON and query patterns.

---

### ISSUE-4: CurrentSongChanged Event Contradicts Passage Model
**Severity:** Medium
**Documents:** architecture.md, api_design.md, single-stream-design.md

**Problem:**
architecture.md [ARCH-SNGC-030] states CurrentSongChanged emits `audio_file_path` when passage_id is None (lines 369-371: "when passage starts, song_id and passage_id are None, audio_file_path contains path"). This contradicts entity_definitions.md which requires all playable audio to have passage definitions. Edge case creates inconsistency:
- Is it valid to enqueue audio files without passage_id?
- How do crossfade calculations work without passage timing points?
- What does "passage starts" mean if passage_id is None?

**Requirements Impact:**
- Contradicts [REQ-DEF-010] - passages are fundamental playback units
- Creates ambiguity for [REQ-PI-040] - "each audio file may contain one or more passages"

**Recommendation:**
Clarify entity model: either (1) require all enqueued audio to have passage definitions (even if passage uses file-level defaults), or (2) define "ephemeral passage" concept for temporary playback without database entry. Update architecture.md [ARCH-SNGC-030] and api_design.md `POST /playback/enqueue` to reflect chosen model.

---

### ISSUE-5: Decoder Thread Pool Lifecycle Unspecified
**Severity:** Medium
**Documents:** single-stream-design.md v1.2

**Problem:**
single-stream-design.md describes DecoderPool structure and decode flow but lacks critical operational specifications:
- How many decoder threads? Fixed pool or dynamic sizing?
- Thread creation/destruction timing (on startup, on-demand, per-passage)?
- Priority queue management (how are Immediate/Next/Prefetch priorities enforced)?
- Shutdown behavior (how to cleanly stop in-progress decodes)?

Without these specs, implementation must make architectural decisions that affect memory, CPU, and shutdown latency.

**Requirements Impact:**
- Affects [REQ-TECH-011] - Raspberry Pi Zero2W resource constraints require pool sizing strategy
- Impacts [REQ-NF-010] - performance targets depend on thread pool design

**Recommendation:**
Specify in single-stream-design.md: (1) fixed pool of 2 threads (sufficient for current + next passage), (2) threads created on first decode request and persist until shutdown, (3) priority queue implemented as ordered VecDeque with priority-based insertion, (4) shutdown sends stop signal and joins threads with 5s timeout.

---

## High Priority Issues (Priority 2 - Gaps)

### ISSUE-6: Resume Fade-In Settings Not in database_schema.md
**Severity:** Medium
**Documents:** crossfade.md [XFD-PAUS-060], database_schema.md

**Problem:**
crossfade.md specifies two settings for resume-from-pause behavior (`resume_from_pause_fade_in_duration` and `resume_from_pause_fade_in_curve`) but database_schema.md settings table does not list these keys. Implementation will discover missing defaults.

**Recommendation:**
Add to database_schema.md settings table specification with defaults: `resume_from_pause_fade_in_duration` (REAL, 0.5), `resume_from_pause_fade_in_curve` (TEXT, "exponential").

---

### ISSUE-7: Crossfade Timing Calculation Location Ambiguous
**Severity:** Medium
**Documents:** crossfade.md [XFD-IMPL-020], single-stream-design.md

**Problem:**
crossfade.md provides pseudocode for crossfade timing calculation but doesn't specify which component executes it. Candidates: PlaybackEngine (queue manager), PassageBufferManager, or CrossfadeMixer. Different choices affect when timing errors are detected.

**Recommendation:**
Specify in single-stream-design.md that CrossfadeMixer calculates timing when queuing next passage (on `queue_next_passage()` call). This enables early detection of timing issues before decode starts.

---

### ISSUE-8: Partial Buffer Playback Handoff Mechanism Missing
**Severity:** Medium
**Documents:** single-stream-design.md [SSD-PBUF-026]

**Problem:**
[SSD-PBUF-026] states "seamlessly switch to the same sample point in the buffer that is being completely filled through decoding" but provides no technical mechanism for this handoff. How does playback determine when to switch? How are sample positions synchronized between partial and complete buffers?

**Recommendation:**
Add implementation note: CrossfadeMixer maintains two buffer references (partial and complete). When complete buffer decode finishes, atomically swap buffer references at next mixer cycle boundary. Sample position tracking remains continuous across swap.

---

### ISSUE-9: Module Health Check Detail Inconsistency
**Severity:** Low
**Documents:** api_design.md, architecture.md

**Problem:**
api_design.md `GET /health` specifies detailed health checks (database, audio_device, audio_subsystem) returning 503 on failure. architecture.md [ARCH-INIT-005] states "Currently, any HTTP response (even error codes) indicates the service is alive" suggesting minimal health check. Mismatch between architectural intent and API specification.

**Recommendation:**
Clarify in architecture.md that initial implementation uses basic health check (200 = alive) but API reserves structure for future detailed checks. Mark api_design.md health check details as "future enhancement".

---

### ISSUE-10: Queue Refill Request Deduplication Missing
**Severity:** Medium
**Documents:** implementation_order.md Phase 2.7

**Problem:**
Phase 2.7 describes queue refill request system but doesn't specify deduplication strategy when Audio Player sends duplicate requests (e.g., due to network retry). Program Director could select and enqueue duplicate passages.

**Recommendation:**
Add request_id tracking in both Audio Player and Program Director. Audio Player generates UUID for each refill request. Program Director tracks in-flight request IDs and returns cached acknowledgment for duplicates without re-selecting.

---

## Recommendations for project-architect

### Immediate Actions (Before Phase 2 Implementation)

1. **Buffer State API Design**: Design complete buffer state visibility system including SSE events and status endpoints. Consider implications for debugging tools and monitoring interfaces.

2. **Passage Timing Validation Strategy**: Define authoritative validation points and error propagation paths. Create sequence diagrams showing validation flow from enqueue through decode to playback.

3. **Queue Entry Timing Override Schema**: Define complete storage schema with examples. Consider foreign key cascades for cleanup and query performance implications.

4. **Passage Identity Model Clarification**: Resolve contradiction between ephemeral playback and passage-required model. Update entity_definitions.md and architecture.md consistently.

5. **Decoder Thread Pool Specification**: Define complete lifecycle and resource management strategy. Consider CPU/memory tradeoffs for Raspberry Pi Zero2W target.

### Design Review Actions

- **Review single-stream-design.md v1.2** for other unspecified lifecycle and error handling behaviors
- **Cross-reference api_design.md and architecture.md** for other health check and error response inconsistencies
- **Audit database_schema.md** for missing settings keys referenced in other documents
- **Validate implementation_order.md Phase 2** against single-stream-design.md for completeness

### Documentation Debt

- Missing sections in database_schema.md (queue entry timing overrides, several settings keys)
- Incomplete error handling specifications in single-stream-design.md (decoder pool, buffer manager)
- Ambiguous component responsibility boundaries between crossfade.md and single-stream-design.md

---

## Affected Documents Summary

| Document | Issues Found | Changes Needed |
|----------|--------------|----------------|
| **single-stream-design.md v1.2** | 4 | Add buffer state transitions, decoder pool lifecycle, buffer handoff mechanism, validation timing specs |
| **api_design.md** | 3 | Add buffer status endpoints, clarify timing override storage, resolve health check ambiguity |
| **crossfade.md** | 2 | Specify validation implementation points, clarify calculation component responsibility |
| **database_schema.md** | 2 | Add queue entry timing override schema, add missing settings keys |
| **architecture.md** | 2 | Clarify passage identity model, resolve health check specification contradiction |
| **implementation_order.md** | 1 | Add request deduplication strategy to Phase 2.7 |

---

**Note:** This review focused on architectural and integration issues. Implementation-level details (code structure, algorithm optimizations) were out of scope per review charter.
