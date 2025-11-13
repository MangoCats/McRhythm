# Requirements Index: PLAN024 - wkmp-ai Refinement

**Source Specification:** [wip/SPEC032_wkmp-ai_refinement_specification.md](../../wip/SPEC032_wkmp-ai_refinement_specification.md)
**Total Requirements:** 26 (21 functional, 5 non-functional)
**Date:** 2025-11-12

---

## Functional Requirements (21)

| Req ID | Brief Description | Source Line | Priority | Type |
|--------|-------------------|-------------|----------|------|
| REQ-SPEC032-001 | Scope Definition - wkmp-ai automatic ingest only | 305-306 | P0 (Critical) | Documentation |
| REQ-SPEC032-002 | Two-Stage Roadmap - Stage One vs Stage Two | 308-309 | P0 (Critical) | Documentation |
| REQ-SPEC032-003 | Five-Step Workflow documentation | 311-312 | P0 (Critical) | Documentation |
| REQ-SPEC032-004 | AcoustID API Key Validation at workflow start | 314-318 | P0 (Critical) | Feature |
| REQ-SPEC032-005 | Folder Selection with Stage One constraints | 320-324 | P0 (Critical) | Feature |
| REQ-SPEC032-006 | Ten-Phase Per-File Pipeline documentation | 326-337 | P0 (Critical) | Documentation |
| REQ-SPEC032-007 | Filename Matching Logic (3 outcomes) | 339-343 | P0 (Critical) | Feature |
| REQ-SPEC032-008 | Hash-Based Duplicate Detection with bidirectional linking | 345-351 | P0 (Critical) | Feature |
| REQ-SPEC032-009 | Metadata Extraction Merging (new overwrites, old preserved) | 353-357 | P1 (High) | Feature |
| REQ-SPEC032-010 | Silence-Based Segmentation with NO AUDIO detection | 359-365 | P0 (Critical) | Feature |
| REQ-SPEC032-011 | Fingerprinting Per Potential Passage via AcoustID | 367-371 | P0 (Critical) | Feature |
| REQ-SPEC032-012 | Song Matching with Confidence (High/Medium/Low/None) | 373-379 | P0 (Critical) | Feature |
| REQ-SPEC032-013 | Passage Recording to database | 381-386 | P0 (Critical) | Feature |
| REQ-SPEC032-014 | Amplitude-Based Lead-In/Lead-Out detection | 388-394 | P0 (Critical) | Feature |
| REQ-SPEC032-015 | Musical Flavor Retrieval (AcousticBrainz → Essentia fallback) | 396-404 | P0 (Critical) | Feature |
| REQ-SPEC032-016 | File Completion marking | 406-407 | P0 (Critical) | Feature |
| REQ-SPEC032-017 | Session Completion determination | 409-410 | P0 (Critical) | Feature |
| REQ-SPEC032-018 | Database Settings Table parameter storage | 412-420 | P0 (Critical) | Feature |
| REQ-SPEC032-019 | Processing Thread Count Auto-Initialization | 422-429 | P1 (High) | Feature |
| REQ-SPEC032-020 | Thirteen UI Progress Sections (SSE-driven) | 431-432 | P1 (High) | Feature |
| REQ-SPEC032-021 | Status Field Enumerations (files/passages/songs) | 434-435 | P0 (Critical) | Feature |

## Non-Functional Requirements (5)

| Req ID | Brief Description | Source Line | Priority | Type |
|--------|-------------------|-------------|----------|------|
| REQ-SPEC032-NF-001 | Parallel Processing using thread count setting | 439-440 | P0 (Critical) | Performance |
| REQ-SPEC032-NF-002 | Real-Time Progress Updates via SSE | 442-443 | P1 (High) | Performance |
| REQ-SPEC032-NF-003 | Sample-Accurate Timing (SPEC017 ticks) | 445-446 | P0 (Critical) | Quality |
| REQ-SPEC032-NF-004 | Symlink/Junction Handling (do not follow) | 448-449 | P1 (High) | Security |
| REQ-SPEC032-NF-005 | Metadata Preservation (all extracted data) | 451-452 | P1 (High) | Quality |

---

## Requirement Categories

**Documentation Requirements (3):**
- REQ-SPEC032-001, REQ-SPEC032-002, REQ-SPEC032-003, REQ-SPEC032-006

These requirements specify updates to SPEC032 documentation itself.

**Workflow Requirements (5):**
- REQ-SPEC032-004 (API key validation)
- REQ-SPEC032-005 (folder selection)
- REQ-SPEC032-016 (file completion)
- REQ-SPEC032-017 (session completion)
- REQ-SPEC032-020 (UI progress display)

**Per-File Processing Pipeline Requirements (10):**
- REQ-SPEC032-007 (filename matching)
- REQ-SPEC032-008 (hash-based deduplication)
- REQ-SPEC032-009 (metadata extraction merging)
- REQ-SPEC032-010 (silence-based segmentation)
- REQ-SPEC032-011 (fingerprinting)
- REQ-SPEC032-012 (song matching)
- REQ-SPEC032-013 (passage recording)
- REQ-SPEC032-014 (amplitude analysis)
- REQ-SPEC032-015 (musical flavor retrieval)
- REQ-SPEC032-021 (status fields)

**Settings & Configuration Requirements (2):**
- REQ-SPEC032-018 (database settings table)
- REQ-SPEC032-019 (thread count auto-initialization)

---

## Priority Distribution

| Priority | Count | Percentage |
|----------|-------|------------|
| P0 (Critical) | 19 | 73% |
| P1 (High) | 7 | 27% |
| **Total** | **26** | **100%** |

**Priority Definitions:**
- **P0 (Critical):** Must be implemented for Stage One release
- **P1 (High):** Should be implemented for Stage One release (improves UX)
- **P2 (Medium):** May be deferred to Stage Two (not present in this spec)

---

## Requirement Dependencies

**Foundational (No Dependencies):**
- REQ-SPEC032-018 (database settings table) - All other requirements depend on this
- REQ-SPEC032-001, REQ-SPEC032-002, REQ-SPEC032-003, REQ-SPEC032-006 (documentation requirements)

**Depends on Settings:**
- REQ-SPEC032-010 (silence detection) → Requires REQ-SPEC032-018 (thresholds)
- REQ-SPEC032-014 (amplitude analysis) → Requires REQ-SPEC032-018 (thresholds)
- REQ-SPEC032-019 (thread count) → Requires REQ-SPEC032-018 (settings table)

**Workflow Sequence Dependencies:**
- REQ-SPEC032-004 (API key validation) must precede REQ-SPEC032-011 (fingerprinting)
- REQ-SPEC032-005 (folder selection) must precede scanning
- REQ-SPEC032-007-015 (10-phase pipeline) must execute in order per file
- REQ-SPEC032-016 (file completion) depends on all 10 phases completing
- REQ-SPEC032-017 (session completion) depends on all files completing

**Data Flow Dependencies:**
- REQ-SPEC032-007 (filename matching) → REQ-SPEC032-008 (hashing) → REQ-SPEC032-009 (metadata extraction)
- REQ-SPEC032-010 (segmentation) → REQ-SPEC032-011 (fingerprinting) → REQ-SPEC032-012 (song matching)
- REQ-SPEC032-012 (song matching) → REQ-SPEC032-013 (passage recording) → REQ-SPEC032-014 (amplitude) → REQ-SPEC032-015 (flavoring)

---

## Requirement Complexity Assessment

**High Complexity (>2 days):**
- REQ-SPEC032-008 (hash-based deduplication with bidirectional linking)
- REQ-SPEC032-010 (silence-based segmentation)
- REQ-SPEC032-012 (song matching with confidence scoring)
- REQ-SPEC032-015 (musical flavor retrieval with fallback)

**Medium Complexity (0.5-2 days):**
- REQ-SPEC032-004 (API key validation)
- REQ-SPEC032-007 (filename matching)
- REQ-SPEC032-009 (metadata merging)
- REQ-SPEC032-011 (fingerprinting)
- REQ-SPEC032-014 (amplitude analysis)
- REQ-SPEC032-019 (thread count auto-init)
- REQ-SPEC032-020 (13 UI sections)

**Low Complexity (<0.5 days):**
- REQ-SPEC032-001, REQ-SPEC032-002, REQ-SPEC032-003, REQ-SPEC032-006 (documentation)
- REQ-SPEC032-005 (folder selection)
- REQ-SPEC032-013 (passage recording)
- REQ-SPEC032-016, REQ-SPEC032-017 (completion marking)
- REQ-SPEC032-021 (status fields)

---

## Requirement Types Summary

| Type | Count |
|------|-------|
| Feature (Code Implementation) | 18 |
| Documentation (SPEC032 updates) | 4 |
| Performance | 2 |
| Quality | 2 |
| Security | 1 |

---

## Cross-References

**Related WKMP Documents:**
- [SPEC032-audio_ingest_architecture.md](../../docs/SPEC032-audio_ingest_architecture.md) - Target document to be updated
- [SPEC002-crossfade.md](../../docs/SPEC002-crossfade.md) - Passage timing definitions (lead-in/lead-out)
- [SPEC017-sample_rate_conversion.md](../../docs/SPEC017-sample_rate_conversion.md) - Tick time units
- [IMPL001-database_schema.md](../../docs/IMPL001-database_schema.md) - Database schema (files, passages, songs tables)
- [GOV002-requirements_enumeration.md](../../docs/GOV002-requirements_enumeration.md) - Requirement numbering conventions

**Source Refinement:**
- [wip/wkmp-ai_refinement.md](../../wip/wkmp-ai_refinement.md) - Original refinement notes

---

## Notes

- All requirements are mandatory for Stage One wkmp-ai release
- Requirements are numbered sequentially from source specification
- Priority assignment based on:
  - P0: Core workflow functionality, data integrity, architectural clarity
  - P1: UX improvements, performance tuning, quality enhancements
- Total estimated effort: 15-25 hours (per specification document)
