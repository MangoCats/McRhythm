# Analysis Summary: wkmp-ai Specification (Audio Import Module)

**Analysis Date:** 2025-10-27
**Document Analyzed:** `wip/_user_story.md`
**Analysis Method:** 8-Phase Multi-Agent Workflow (/think command)
**Analyst:** Claude Code (Risk-First Software Engineering)
**Stakeholders:** WKMP Development Team

---

## Quick Reference

- **Current State:** wkmp-ai is placeholder (4-line main.rs only)
- **Specification Gaps:** 2 of 6 requested features require new design (amplitude analysis, extensible metadata)
- **Key Finding:** User story introduces amplitude-based lead-in/lead-out detection NOT in existing docs
- **Recommendation:** Use RMS-based amplitude analysis, hybrid JSON parameters, async+SSE workflow
- **Deliverables Required:** 5 new spec documents, 3 doc updates
- **Estimated Effort:** 8-12 hours (specification only, before implementation)

## Document Map (Navigation Guide)

**For Quick Overview:**
- Read this summary only (~400 lines, 5-minute read)

**For Specific Topics:**
- Current state & docs reviewed: [01_current_state.md](01_current_state.md) (~350 lines)
- User story requirements breakdown: [02_requirements_analysis.md](02_requirements_analysis.md) (~400 lines)
- Technical challenges & solutions: [03_technical_challenges.md](03_technical_challenges.md) (~600 lines)
- Proposed architecture: [04_architecture_design.md](04_architecture_design.md) (~800 lines)
- Option comparisons (5 decisions): [05_option_comparisons.md](05_option_comparisons.md) (~1200 lines)
- Recommendations & deliverables: [06_recommendations.md](06_recommendations.md) (~300 lines)

**For Complete Context:**
- Full consolidated analysis: [FULL_ANALYSIS.md](FULL_ANALYSIS.md) (~4900 lines)
- Use only when comprehensive view required

---

## Executive Summary (5-minute read)

### Problem Statement

User story requests high-level specification for **wkmp-ai** (Audio Ingest microservice) to guide new users through importing their music collection into WKMP database.

**User Scenario:**
- New WKMP user with music collection in `/home/username/Music`
- Mixed file formats (.mp3, .flac, .ogg, .wav, etc.)
- Variable organization and metadata quality
- Needs MusicBrainz MBID identification for Program Director functionality

**6 Features Requested:**
1. Recording identification (via fingerprinting)
2. Automatic passage boundary detection
3. **Lead-in/lead-out point detection (amplitude-based)** ⚠️ NEW
4. Fade point detection (continuous audio)
5. User-adjustable algorithm parameters
6. **Flexible metadata architecture (extensible parameters)** ⚠️ NEW

### Current State Assessment

**wkmp-ai Status:**
- Placeholder implementation only (`main.rs` prints "wkmp-ai placeholder")
- No HTTP server, no routes, no business logic
- Full greenfield specification required

**Existing Documentation:**
- **SPEC008** (Library Management): File scanning, metadata extraction, MusicBrainz/AcousticBrainz integration - **COMPLETE**
- **IMPL005** (Audio File Segmentation): 5-step workflow for multi-passage files - **COMPLETE**
- **IMPL001** (Database Schema): Tick-based passage timing, JSON metadata support - **COMPLETE**

**Coverage Analysis:**
- ✅ **4 of 6 features fully specified:** Recording identification, passage boundary detection (silence-based), MusicBrainz integration, AcousticBrainz integration
- ⚠️ **2 of 6 features require new design:** Amplitude-based lead-in/lead-out detection, extensible metadata framework

### Key Findings

**Finding 1: Specification Gap - Amplitude Analysis**

User story requests: *"If a passage has a long slow increase of amplitude at the start, lead-in can be as far as the point which reaches 1/4 perceived audible intensity of the early part of the passage, but no more than 5 seconds."*

**Status:** NOT SPECIFIED in existing documentation
- IMPL005 specifies silence detection only (threshold-based boundary finding)
- Amplitude-based analysis is a NEW requirement
- Requires dedicated Tier 2 specification document

**Finding 2: Technical Definition Required - "Perceived Audible Intensity"**

User story uses term "1/4 perceived audible intensity" without technical definition.

**Research Findings:**
- Raw amplitude (peak) ≠ perceived loudness
- **RMS (Root Mean Square)** approximates perceived loudness
- A-weighting filter improves perceptual accuracy
- -12 dB threshold ≈ 1/4 intensity (factor of ~4 in power)

**Recommendation:** RMS-based analysis with A-weighting (simple, adequate, user-adjustable)

**Finding 3: Architecture Alignment Strong**

WKMP existing patterns support proposed design:
- ✅ JSON metadata storage (`musical_flavor_vector` precedent in passages table)
- ✅ Async/Tokio workflow architecture (all microservices)
- ✅ SSE real-time updates (wkmp-ap playback events)
- ✅ Tick-based timing precision (28,224,000 ticks/second)

**Finding 4: Implementation Readiness**

**Fully Specified (can implement now):**
- File scanner (SPEC008)
- Metadata extractor (SPEC008)
- Chromaprint fingerprinting (SPEC008)
- MusicBrainz client (SPEC008)
- AcousticBrainz client (SPEC008)
- Silence-based segmentation (IMPL005)

**Requires Specification:**
- **Amplitude analyzer** (lead-in/lead-out detection) - NEW
- **Parameter management** (user-adjustable thresholds) - NEW
- **Additional metadata framework** (seasonal, profanity, etc.) - NEW
- **HTTP API design** (import workflow endpoints) - needs IMPL doc

**Finding 5: Risk-Optimized Path Identified**

Using CLAUDE.md Risk-First Decision Framework, analyzed 5 key architectural choices:
1. Amplitude method: **RMS** (lowest risk) vs. EBU R128 (complex)
2. Parameter storage: **Hybrid JSON** (matches patterns) vs. dedicated table
3. Essentia integration: **Subprocess** (cleanest) vs. FFI (unsafe) vs. Python service
4. Workflow model: **Async + SSE** (best UX) vs. synchronous (timeouts)
5. UI complexity: **Progressive disclosure** (beginner + power-user) vs. simple-only

All recommendations prioritize **lowest failure risk** (primary criterion per CLAUDE.md).

### Specification Gaps Summary

| Feature | User Story Requirement | Existing Spec | Status | Action Required |
|---------|----------------------|---------------|--------|-----------------|
| Recording identification | Fingerprinting → MusicBrainz | SPEC008 | ✅ Complete | None |
| Passage boundaries | Silence detection | IMPL005 | ✅ Complete | None |
| **Lead-in/lead-out** | **Amplitude-based (1/4 intensity)** | **None** | **⚠️ Missing** | **Create SPEC###-amplitude_analysis.md** |
| Fade points | Continuous audio handling | Partial (IMPL005) | ⚠️ Clarify | Update SPEC008 |
| User parameters | All values adjustable | Partial (UI shown) | ⚠️ Incomplete | Create IMPL###-parameter_management.md |
| **Extensible metadata** | **Seasonal, profanity, etc.** | **None** | **⚠️ Missing** | **Update IMPL001 (schema), document in SPEC** |

### Technical Challenges Addressed

**Challenge 1: Amplitude Detection Algorithm**

- **Solution:** RMS with 100ms sliding window + A-weighting filter
- **Threshold:** -12 dB relative to passage peak = "1/4 intensity"
- **Quick ramp detection:** If 3/4 intensity reached < 1s, zero lead-in
- **Libraries:** `dasp` (RMS module) or `audio-processor-analysis` (2024, maintained)

**Challenge 2: Tick-Based Timing Integration**

- **Problem:** Analysis works in seconds (float), database uses ticks (integer)
- **Solution:** Convert at boundary: `ticks = seconds * 28_224_000`
- **Example:** 2.5s lead-in → 70,560,000 ticks (stored in `lead_in_start_ticks`)

**Challenge 3: Parameter Architecture**

- **Solution:** Hybrid approach
  - Global defaults in `settings` table (JSON blob)
  - Per-passage overrides in `passages.import_metadata` column (JSON)
  - Matches existing `musical_flavor_vector` pattern
- **Extensible:** No schema migrations for new parameters

**Challenge 4: Essentia Integration (No Rust Bindings)**

- **Problem:** Essentia is C++/Python/JS only (no Rust bindings found in 2024)
- **Solution:** Subprocess calls to `essentia_streaming_extractor_music` binary
- **Rationale:** Cleanest separation, lowest risk (vs. FFI memory safety issues)

### Proposed Architecture Summary

**wkmp-ai Module Structure:**
```
wkmp-ai/
├── src/
│   ├── main.rs                    # Axum HTTP server, port 5723
│   ├── api/
│   │   ├── import_workflow.rs     # POST /import/start, GET /import/status
│   │   ├── amplitude_analysis.rs  # POST /analyze/amplitude
│   │   └── parameters.rs          # GET/POST /parameters
│   ├── services/
│   │   ├── file_scanner.rs        # (SPEC008 - already specified)
│   │   ├── metadata_extractor.rs  # (SPEC008 - already specified)
│   │   ├── fingerprinter.rs       # (SPEC008 - already specified)
│   │   ├── musicbrainz_client.rs  # (SPEC008 - already specified)
│   │   ├── acousticbrainz_client.rs  # (SPEC008 - already specified)
│   │   ├── amplitude_analyzer.rs  # NEW: RMS analysis, lead-in/lead-out
│   │   ├── silence_detector.rs    # (IMPL005 - already specified)
│   │   └── parameter_manager.rs   # NEW: Parameter loading/saving
│   └── db/
│       └── queries.rs             # Database access layer
```

**HTTP API Endpoints:**
- `POST /import/start` - Begin import, return session_id
- `GET /import/status/{session_id}` - Poll progress (also SSE available)
- `POST /import/cancel/{session_id}` - Cancel in-progress import
- `POST /analyze/amplitude` - Analyze file, return RMS profile + lead-in/lead-out
- `GET /parameters/global` - Get default parameters
- `POST /parameters/global` - Update defaults
- `GET /metadata/{passage_id}` - Get additional metadata
- `POST /metadata/{passage_id}` - Update additional metadata

**Workflow Model:**
- Async background jobs (Tokio tasks)
- SSE for real-time progress updates
- Persistent job state (resume after restart)
- Graceful cancellation support

### Recommendations (Risk-Based)

Using CLAUDE.md Risk-First Decision Framework, all choices prioritize lowest failure risk:

**1. Amplitude Analysis: RMS with A-weighting**
- Lowest risk vs. EBU R128 (simpler, adequate accuracy)
- User-adjustable thresholds compensate for perception modeling imperfections

**2. Parameter Storage: Hybrid (Global JSON + Per-Passage Overrides)**
- Matches existing `musical_flavor_vector` pattern
- Extensible without schema migrations

**3. Essentia Integration: Subprocess Calls**
- Lowest risk vs. FFI (memory safety) or Python microservice (complexity)
- Clean separation, proven pattern

**4. Workflow Model: Async Background + SSE**
- Lowest risk vs. synchronous (HTTP timeouts)
- Best UX, matches WKMP SSE architecture

**5. UI Complexity: Progressive Disclosure (Simple → Advanced)**
- Lowest risk vs. simple-only (users can't fix edge cases) or advanced-only (overwhelming)
- Beginner-friendly with power-user option

### Deliverables Required

To complete wkmp-ai specification:

**New Specification Documents (Tier 2 - Design):**
1. **SPEC###-audio_ingest_architecture.md** (~300 lines)
   - Module architecture, integration points, event system
2. **SPEC###-amplitude_analysis.md** (~400 lines)
   - Lead-in/lead-out algorithms, RMS calculation, parameter definitions

**New Implementation Specifications (Tier 3):**
3. **IMPL###-audio_ingest_api.md** (~300 lines)
   - Complete HTTP API spec (endpoints, schemas, errors)
4. **IMPL###-amplitude_analyzer_implementation.md** (~400 lines)
   - Rust implementation details, library choices, testing strategy
5. **IMPL###-parameter_management.md** (~250 lines)
   - Database schema, default values, validation rules

**Documentation Updates:**
6. **REQ001-requirements.md** (~50 lines changes)
   - Add requirements for amplitude-based lead-in/lead-out detection
7. **SPEC008-library_management.md** (~30 lines changes)
   - Reference new amplitude analysis module
8. **IMPL001-database_schema.md** (~50 lines changes)
   - Add `additional_metadata` column to `passages` table

**Estimated Effort:** 8-12 hours (specification documentation only, no coding)

### Decision Points for Stakeholder

**Decision 1:** Approve recommended architectural choices?
- RMS amplitude analysis
- Hybrid JSON parameter storage
- Subprocess Essentia integration
- Async + SSE workflow
- Progressive disclosure UI

**Decision 2:** Proceed with specification document creation?
- 5 new documents + 3 updates required
- Estimated 8-12 hours effort
- Blocks implementation until complete

**Decision 3:** Sequence of document creation?
- **Option A:** Tier 2 first (SPEC docs), then Tier 3 (IMPL docs) - follows hierarchy
- **Option B:** Critical path first (amplitude analysis, API design) - faster to implementation
- **Option C:** Parallel (multiple documents simultaneously) - fastest but requires coordination

### Next Steps

**Analysis is complete.** Implementation planning requires explicit authorization.

**To proceed with specification:**
1. Review this summary and detailed sections (see Document Map above)
2. Make decisions on 3 decision points above
3. Authorize specification document creation

**To proceed to implementation:**
1. Create specification documents listed above (8-12 hours)
2. Run `/plan [specification_file]` for implementation plan
3. `/plan` generates: requirements analysis, test specifications, increment breakdown

**Estimated Timeline (Full wkmp-ai Module):**
- Specification: 8-12 hours (documentation)
- Implementation: 3-4 weeks (coding, testing, integration)

---

**Analysis Status:** ✅ **COMPLETE**
**Document Status:** Ready for stakeholder review and decision
**Contact:** See detailed analysis sections for technical depth

---

**For detailed analysis, see:**
- [01_current_state.md](01_current_state.md) - Implementation status & docs reviewed
- [02_requirements_analysis.md](02_requirements_analysis.md) - User story breakdown
- [03_technical_challenges.md](03_technical_challenges.md) - Amplitude detection, parameters
- [04_architecture_design.md](04_architecture_design.md) - Proposed wkmp-ai design
- [05_option_comparisons.md](05_option_comparisons.md) - 5 architectural decisions (risk-based)
- [06_recommendations.md](06_recommendations.md) - Consolidated recommendations & deliverables
