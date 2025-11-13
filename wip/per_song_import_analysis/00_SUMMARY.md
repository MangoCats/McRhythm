# Per-Song Import System with Real-Time UI: Analysis Summary

**Analysis Date:** 2025-01-08
**Analysis Method:** Multi-agent research and synthesis (/think workflow)
**Analyst:** Claude Code
**Scope:** Design per-song import workflow with real-time graphical progress, integrating hybrid fusion concepts

---

## Quick Reference

- **Topic:** Per-song import workflow with real-time UI
- **Previous Concepts:** Musical Flavor Fusion, Identity Resolution, Validation (from hybrid_import_analysis.md)
- **New Focus:** Granular per-song processing + information-dense real-time UI
- **Recommendation:** SSE-based progressive disclosure dashboard with per-song cards

---

## Executive Summary (5-minute read)

### Problem Statement

Current wkmp-ai import processes **audio files as atomic units** (all-or-nothing):
- User sees file-level progress only
- Cannot see which songs within a file succeeded/failed
- Multi-song files (albums, live recordings) lack granular feedback
- Hybrid fusion concepts (from previous analysis) need integration into per-song workflow

**User Need:**
"When importing a 10-song album as one FLAC file, show me what's happening with **each song** in real-time—which are identified, which have conflicts, which flavors were found—without overwhelming me with details I don't need right now."

### Proposed Solution

**3-Component Architecture:**

**Component 1: Per-Song Import Engine**
- Process audio file sequentially, song-by-song
- For each song: Identity Resolution → Metadata Fusion → Flavor Synthesis → Passage Creation → Validation
- Emit granular events per song (e.g., "Song 2: AcoustID matched 0.92", "Song 3: Flavor fused from Essentia + ID3")

**Component 2: Real-Time Event Stream (SSE)**
- Server-Sent Events broadcast per-song progress
- Event types: SongDiscovered, IdentityResolved, FlavorSynthesized, ValidationComplete, SongCompleted
- Throttling: Max 30 updates/sec (prevent UI flooding)

**Component 3: Progressive Disclosure Dashboard**
- **Simple Mode (default):** Per-song progress cards with status icons (✓ Complete, ⚙️ In Progress, ⚠️ Warning, ✗ Failed)
- **Detailed Mode (expandable):** Show fusion confidence scores, source provenance, validation conflicts
- **Aggregate View:** File-level summary (e.g., "8/10 songs complete, 2 warnings")

### Key Innovation: Information Layering

**Layer 1 (Always Visible):**
```
┌─ Album: Dark Side of the Moon ───────────────────────┐
│ File: dsotm.flac │ 10 songs │ 8 ✓ 1 ⚙️ 1 ⚠️          │
├──────────────────────────────────────────────────────┤
│ ⚙️ Breathe (In The Air) │ Identifying... 45%       │
│ ✓ On The Run │ Complete │ MusicBrainz + Essentia   │
│ ⚠️ Time │ Complete │ Low confidence match (0.68)  │
└──────────────────────────────────────────────────────┘
```

**Layer 2 (Click to Expand):**
```
┌─ Song: Time ─────────────────────────────────────────┐
│ ⚠️ Identity Resolution: Low Confidence               │
│   ├─ ID3 MBID: abc-123 (confidence: 0.9)           │
│   ├─ AcoustID: xyz-789 (confidence: 0.68) ⚠️       │
│   └─ CONFLICT: Different MBIDs → Using ID3         │
│ ✓ Musical Flavor: Fused                             │
│   ├─ AcousticBrainz: 60% (pre-2022)                │
│   ├─ Essentia: 30% (computed)                      │
│   └─ ID3 Genre: 10% (Progressive Rock)             │
│ ✓ Passage: Created (3:45 - 7:20 in file)            │
│ [View Full Details] [Retry with Deep Scan]          │
└──────────────────────────────────────────────────────┘
```

**Layer 3 (Full Details Modal):**
```
[Full technical report: raw JSON, API responses, debug logs]
```

### Critical Findings

1. **SSE > WebSocket for this use case** - Unidirectional updates (server → client), simpler, auto-reconnect
2. **Per-song granularity enables user intervention** - User can see Song 3 has conflict → Click "Review" → Decide before proceeding
3. **Progressive disclosure balances simplicity and detail** - Casual users see clean progress, power users expand for fusion details
4. **Integration with hybrid fusion is natural** - Each song goes through 3-tier fusion (Extract → Fuse → Validate)
5. **Workflow pause points enable quality control** - Option to "Pause on conflicts" → User reviews before continuing

### Recommendation

Implement **per-song import with SSE-based progressive disclosure dashboard** in staged rollout:

**Phase 1: Per-Song Engine (Backend)**
- Refactor import workflow to process song-by-song
- Emit granular SSE events per song
- Integrate hybrid fusion modules (from previous analysis)

**Phase 2: Simple Dashboard (Frontend)**
- Per-song progress cards (Layer 1 only)
- Status icons and aggregate metrics
- SSE connection with auto-reconnect

**Phase 3: Progressive Disclosure (Enhancement)**
- Expandable details (Layer 2)
- Fusion confidence display
- Conflict flagging and resolution UI

**Phase 4: Advanced Features (Future)**
- Pause on conflicts
- Manual override interface
- Export detailed reports

---

## Document Map (Navigation Guide)

**For Quick Overview:**
- Read this summary only (~500 lines)

**For Specific Topics:**
- Workflow Architecture: [01_workflow_architecture.md] (~400 lines)
- Real-Time UI Design: [02_realtime_ui_design.md] (~450 lines)
- Progressive Disclosure Patterns: [03_progressive_disclosure.md] (~350 lines)
- Integration with Hybrid Fusion: [04_fusion_integration.md] (~400 lines)

**For Complete Context:**
- Full consolidated analysis: [FULL_ANALYSIS.md] (~1800 lines)
- Use only when comprehensive view required

---

## Questions Addressed

**Q1: How to process audio files song-by-song instead of file-by-file?**
- **Answer:** Sequential song processing with passage boundary detection first, then per-song fusion pipeline
- **See:** [01_workflow_architecture.md] Section 1.2

**Q2: What real-time technology for progress updates?**
- **Answer:** Server-Sent Events (SSE) - simpler than WebSocket for unidirectional updates, built-in auto-reconnect
- **See:** [02_realtime_ui_design.md] Section 2.1

**Q3: How to balance simple UX with information density?**
- **Answer:** Progressive disclosure - 3-layer design (simple cards → expandable details → full debug modal)
- **See:** [03_progressive_disclosure.md] Section 3.2

**Q4: How to integrate hybrid fusion concepts?**
- **Answer:** Each song flows through 3-tier fusion (Extract → Fuse → Validate), emit events at each stage
- **See:** [04_fusion_integration.md] Section 4.1

---

## Approaches Compared

### Approach A: File-Level Import (Current)

**Description:** Process entire audio file as atomic unit, show file-level progress only

**Pros:**
- Simple architecture (no song segmentation needed first)
- Existing implementation (already working)

**Cons:**
- No granularity (user doesn't know which songs succeeded)
- Multi-song files lack per-song feedback
- Cannot intervene on conflicts mid-file

**Risk:** Low (proven)
**Effort:** Zero (status quo)

---

### Approach B: Per-Song Sequential (Recommended)

**Description:** Detect passage boundaries first → Process each song sequentially → Real-time SSE updates

**Pros:**
- Granular feedback (user sees per-song progress)
- Enables conflict resolution mid-import
- Natural integration with hybrid fusion
- SSE provides real-time updates with simple code

**Cons:**
- Requires passage detection first (add latency)
- More complex event handling
- UI must handle dynamic song list

**Risk:** Low-Medium
**Effort:** Medium (1-2 weeks backend + 1 week frontend)
**Ranking:** #1 (Recommended)

---

### Approach C: Per-Song Parallel

**Description:** Detect passages → Process songs in parallel (4-8 workers) → Aggregate events

**Pros:**
- Faster than sequential (2-4x speedup)
- Still provides per-song feedback

**Cons:**
- Complex coordination (songs finish out of order)
- Resource contention (4-8 concurrent audio analyses)
- Harder to debug (interleaved logs)
- Conflict resolution UI more complex (multiple songs may have conflicts simultaneously)

**Risk:** Medium
**Effort:** High (2-3 weeks)
**Ranking:** #3 (Future optimization, not initial implementation)

---

### Approach D: Hybrid (Sequential with Background Upgrade)

**Description:**
- Sequential per-song processing (simple pass)
- Background worker later upgrades passages (deep analysis)

**Pros:**
- Fast initial import (user sees results quickly)
- Deep analysis doesn't block user
- Can retry failed songs independently

**Cons:**
- Two-phase complexity
- User may not realize analysis is ongoing
- Must track "analysis pending" state

**Risk:** Medium
**Effort:** Medium-High (2 weeks)
**Ranking:** #2 (Good for "Quick Import" mode from UX analysis)

---

## Next Steps

This analysis is complete. Implementation planning requires explicit user authorization.

**To proceed with implementation:**

1. **Review analysis findings** and select preferred approach
   - Recommended: Approach B (Per-Song Sequential)
   - Alternative: Approach D (Hybrid with background upgrade) for Quick Import mode

2. **Make decisions on identified decision points:**
   - SSE event granularity: Per-song or per-phase-per-song?
   - UI default state: Simple cards or show some details initially?
   - Conflict handling: Pause on conflicts or continue with warnings?
   - Performance: Sequential (simpler) or parallel (faster) song processing?

3. **Run `/plan [specification_file]`** to create detailed implementation plan
   - /plan will generate: requirements analysis, test specifications, increment breakdown

4. **/plan will generate:**
   - Requirements analysis and traceability
   - Acceptance test specifications (Given/When/Then)
   - Increment breakdown with tasks and deliverables
   - Risk assessment and mitigation steps

**User retains full authority over:**
- Whether to implement any recommendations
- Which approach to adopt
- When to proceed to implementation
- Modifications to suggested approaches

---

**Analysis Status:** Complete
**Document Quality:** Comprehensive (workflow design, UI patterns, technology research, integration strategy)
**Implementation Readiness:** Detailed designs provided, ready for /plan workflow if approved
