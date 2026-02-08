# PLAN: am30 Album Matcher Integration into wkmp-ai Workflow

**Plan ID:** PLAN_am30_integration
**Created:** 2025-12-14
**Status:** Draft
**Complexity:** Standard (15 requirements, 2 components)

---

## Executive Summary

Integrate the existing `AlbumMatcher` (am30 algorithm) into the wkmp-ai workflow pipeline to enable automatic identification and segmentation of full-album audio files.

**Current State:**
- `AlbumMatcher` module fully implemented with Stages 2-5
- `SingleTrackDiscriminator` implemented with 5-layer detection
- Pipeline processes ALL files as single songs via AcoustID
- Album files are NOT detected or handled specially

**Target State:**
- Pipeline detects album vs single-track files early
- Album files routed to `AlbumMatcher.match_album()`
- Per-track passages created with individual Recording MBIDs
- Progress events emitted for album matching stages
- Graceful fallback if album matching fails

---

## Requirements Index

| Req ID | Type | Description | Priority | Complexity |
|--------|------|-------------|----------|------------|
| REQ-AM30-001 | Functional | Single-track discrimination in pipeline | High | Low |
| REQ-AM30-002 | Functional | Route album files to AlbumMatcher | High | Medium |
| REQ-AM30-003 | Functional | Convert AlbumMatchResult to passages | High | Medium |
| REQ-AM30-004 | Functional | Emit progress events for album stages | Medium | Low |
| REQ-AM30-005 | Functional | Error handling and fallback strategy | High | Medium |
| REQ-AM30-006 | Functional | Share MusicBrainz client (rate limiting) | Medium | Medium |
| REQ-AM30-007 | Functional | PipelineConfig extension for album matching | Low | Low |
| REQ-AM30-008 | Functional | Integrate with Stage 0 (embedded MBID) | Medium | Medium |
| REQ-AM30-009 | Functional | Track boundary to PassageBoundary conversion | High | Low |
| REQ-AM30-010 | Performance | Album matching < 2 min for typical album | Medium | Low |
| REQ-AM30-011 | Functional | Handle single-track detection failure | Medium | Low |
| REQ-AM30-012 | Functional | Support album-specific workflow events | Medium | Low |
| REQ-AM30-013 | Functional | ConfidenceTier assignment for album tracks | Medium | Medium |
| REQ-AM30-014 | Test | Unit tests for routing logic | High | Low |
| REQ-AM30-015 | Test | Integration tests for full album flow | High | Medium |

---

## Scope

### In Scope
- Single-track discrimination integration in `process_file()`
- Album file routing to `AlbumMatcher`
- Passage creation from `AlbumMatchResult`
- Progress events for album matching stages
- Error handling with fallback to single-song processing
- Configuration extension for album matching options

### Out of Scope
- Changes to AlbumMatcher algorithm itself (Stages 2-5)
- Changes to SingleTrackDiscriminator logic
- MusicBrainz API changes
- Database schema changes (passages table supports this already)
- UI changes in wkmp-ai web interface

### Assumptions
- `AlbumMatcher.match_album()` is functionally complete and tested
- `SingleTrackDiscriminator` accuracy is sufficient (tested in am30 example)
- Database schema supports multiple passages per file
- MusicBrainz client can be shared with proper rate limiting

---

## Architecture Overview

```
process_file(path)
    │
    ├─► Extract ID3 metadata (for artist/album hints)
    │
    ├─► SingleTrackDiscriminator.analyze_pre_decode(path)
    │       │
    │       ├─► score >= 1.5 → SINGLE TRACK
    │       │       └─► Existing pipeline flow (AcoustID)
    │       │
    │       └─► score < 1.5 → LIKELY ALBUM
    │               │
    │               ├─► AlbumMatcher.match_album(path, artist, album)
    │               │       │
    │               │       ├─► Success: AlbumMatchResult with tracks[]
    │               │       │       └─► convert_to_passages()
    │               │       │
    │               │       └─► Failure: AlbumMatchError
    │               │               └─► Fallback to single-song flow
    │               │
    │               └─► Return Vec<ProcessedPassage>
    │
    └─► Return Vec<ProcessedPassage>
```

---

## Increments Overview

| # | Name | Effort | Prerequisites | Deliverables |
|---|------|--------|---------------|--------------|
| 1 | Pipeline Config Extension | 1-2h | None | `PipelineConfig` album options |
| 2 | Single-Track Check Integration | 2-3h | Inc 1 | Routing logic in `process_file()` |
| 3 | AlbumMatcher Integration | 3-4h | Inc 2 | Call `match_album()` for albums |
| 4 | Passage Conversion | 2-3h | Inc 3 | `convert_album_to_passages()` |
| 5 | Progress Events | 1-2h | Inc 4 | Album-specific workflow events |
| 6 | Error Handling & Fallback | 2-3h | Inc 4 | Graceful degradation |
| 7 | Integration Tests | 3-4h | Inc 6 | End-to-end test suite |

**Total Estimated Effort:** 14-21 hours
**Confidence:** HIGH (±25%) - Well-defined integration task

---

## Risk Summary

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| AlbumMatcher performance too slow | LOW | Medium | Config option to skip; async processing |
| SingleTrackDiscriminator false positives | MEDIUM | Medium | Threshold tuning; fallback to album check |
| MusicBrainz rate limiting conflicts | LOW | Medium | Share rate limiter instance |
| Memory usage during album decode | MEDIUM | Medium | Stream processing; cleanup after match |

---

## Success Criteria

- [ ] Pipeline correctly identifies album files (>90% accuracy)
- [ ] Album files produce N passages for N-track albums
- [ ] Each passage has correct Recording MBID from MusicBrainz
- [ ] Progress events emitted for album matching stages
- [ ] Errors handled gracefully without crashing import
- [ ] All 15 requirements covered by tests
- [ ] Integration tests pass for album + single-song files

---

## Files to Modify

| File | Changes |
|------|---------|
| `wkmp-ai/src/workflow/pipeline.rs` | Add routing, album processing |
| `wkmp-ai/src/workflow/mod.rs` | Add album-specific events |
| `wkmp-ai/src/matching/mod.rs` | Export needed types |
| `wkmp-ai/tests/pipeline_album_integration.rs` | New integration tests |

---

## Related Documents

- [am30 example](wkmp-ai/examples/am30/) - Reference implementation
- [matching module](wkmp-ai/src/matching/) - AlbumMatcher, SingleTrackDiscriminator
- [workflow module](wkmp-ai/src/workflow/) - Pipeline, ProcessedPassage
