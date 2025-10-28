# User Story - Audio Import

Refrences: [REQ001 - Requirements](REQ001-requirements.md), [REQ002 - Entity Definitions](REQ002-entity_definitions.md)

## Initial State

A new wkmp user has just installed the software.

Their music collection is organized under their /home/username/Music folder in various sub-folders, some representing artists, some representing albums, some random collections. Most files are named by song title, many filenames have additional information like artist or album or track number, most of those are accurate but some arr not.  The files themselves are mainly .mp3, some .ogg, some .flac, some .opus, some .wav.  There are also scattered image files in various formats, many are album cover art, but also random things and other random files here and there.  Within the music files there are various formats of tag information, a lot of ID3 mostly accurate, some not.  Most files represent a single song, some are continuous recordings of entire albums or live concerts.

## Next Step

In order to access the power of the WKMP program director, their music files need to be indexed with accurate MusicBrainz MBIDs.  This will be the index into the AcousticBrainz high level descriptors of the recordings that enable the program director to put together playlists which match the taste and mood of the listener at various times of day.

Ideally, this process will be as automated as possible for the user, gathering the available information about each audio file, identifying the MBID(s) of the recording(s) accurately through the most reliable information available.

## wkmp-ai

The task is read REQ001 and related documents, consider the overall wkmp system and create a high level specification for wkmp-ai, the module which will guide a new user through the process of importing their music into the wkmp database.  wkmp-ai features include:

- correct identification of recordings within audio files
- automatic definitions of passage start and end points within audio files corresponding to those recordings
- all values in these algorithms are user adjustable parameters
- automatic definitions of passage lead-in and lead-out points based on audio levels of the passages:
  - if a passage has a long slow increase of amplitude at the start, lead-in can be as far as the point which reaches 1/4 perceived audible intensity of the early part of the passage, but no more than 5 seconds
  - if a passage ramps up amplitude quickly, over 3/4 intensity within less than 1 second, then lead-in duration is zero
  - if a passage has a long slow decrease of amplitude at the end, lead-out can be as far as the point which drops below 1/4 perceived audible intensity of the late part of the passage, but no more than 5 seconds
  - if a passage ramps down amplitude quickly, 3/4 intensity or more within 1 second or less, then lead-out duration is zero
- automatic definitions of appropriate fade-in and fade-out points when a passage is cutting from continuous audio with no silence between recordings.
- a flexible architecture permitting addition of many more parameters describing the nature of the recordings and passages in the database, such as:
  - How much of a seasonal year end holiday song is this? 0.0-1.0
  - How profane are the lyrics in this song? 0.0-1.0
- Additional parameters may be user edited, automatically determined, or both

---

## After Analysis

**Analysis Date:** 2025-10-27
**Analysis Method:** `/think` Multi-Agent Workflow (8-Phase Analysis)
**Analysis Output:** See [_user_story_analysis/](wip/_user_story_analysis/) folder

### Quick Summary

**Current State:** wkmp-ai is placeholder only (4-line main.rs) - full greenfield specification required

**Specification Coverage:**
- ✅ **4 of 6 features fully specified** in existing docs (SPEC008, IMPL005)
  - Recording identification (Chromaprint → AcoustID → MusicBrainz)
  - Passage boundary detection (silence-based segmentation)
  - MusicBrainz integration (recording/artist/work metadata)
  - AcousticBrainz integration (musical flavor data)

- ⚠️ **2 of 6 features require new design:**
  - **Amplitude-based lead-in/lead-out detection** (NOT in existing docs)
  - **Extensible metadata framework** (seasonal, profanity, etc.)

### Key Findings

1. **Specification Gap:** Amplitude analysis for lead-in/lead-out NOT specified
   - User story requests "1/4 perceived audible intensity" thresholds
   - Research: RMS with A-weighting approximates perception (simpler than EBU R128)
   - Requires new SPEC###-amplitude_analysis.md document

2. **Architecture Alignment:** WKMP patterns support proposed design
   - JSON metadata (precedent: `musical_flavor_vector` in passages table)
   - Async Tokio workflows, SSE real-time updates
   - Tick-based timing (28,224,000 ticks/second)

3. **Risk-Optimized Recommendations** (using CLAUDE.md Risk-First framework):
   - Amplitude: RMS with A-weighting (lowest risk, adequate accuracy)
   - Parameters: Hybrid JSON (global defaults + per-passage overrides)
   - Essentia: Subprocess calls (cleanest, avoids FFI memory issues)
   - Workflow: Async + SSE (best UX, proven WKMP pattern)
   - UI: Progressive disclosure (simple wizard → advanced tuning)

### Deliverables Required

**New Specifications:**
1. SPEC###-audio_ingest_architecture.md (~300 lines)
2. SPEC###-amplitude_analysis.md (~400 lines)
3. IMPL###-audio_ingest_api.md (~300 lines)
4. IMPL###-amplitude_analyzer_implementation.md (~400 lines)
5. IMPL###-parameter_management.md (~250 lines)

**Documentation Updates:**
6. REQ001-requirements.md (~50 lines - add amplitude detection requirements)
7. SPEC008-library_management.md (~30 lines - reference amplitude module)
8. IMPL001-database_schema.md (~50 lines - add `additional_metadata` column)

**Estimated Effort:** 8-12 hours (specification docs only, no coding)

### Detailed Analysis Sections

**Read analysis in this order:**
1. **Start here:** [00_SUMMARY.md](_user_story_analysis/00_SUMMARY.md) - 5-minute executive summary
2. **Current state:** [01_current_state.md](_user_story_analysis/01_current_state.md) - Implementation status, docs reviewed
3. **Options analysis:** [05_option_comparisons.md](_user_story_analysis/05_option_comparisons.md) - 5 architectural decisions (risk-based)
4. **Recommendations:** [06_recommendations.md](_user_story_analysis/06_recommendations.md) - Consolidated recommendations, deliverables

### Next Steps

**To proceed with specification creation:**
1. Review analysis summary and recommendations
2. Approve or modify architectural choices
3. Create 5 new specification documents + 3 updates (8-12 hours)

**To proceed directly to implementation:**
1. After specifications complete, run `/plan [specification_file]`
2. `/plan` generates requirements analysis, test specs, increment breakdown
3. Estimated implementation: 3-4 weeks (full wkmp-ai module)

**Decisions Required:**
- Approve RMS-based amplitude analysis approach?
- Approve hybrid JSON parameter storage?
- Proceed with specification document creation?
- Sequence: Tier 2 first vs. critical path first vs. parallel?

**Analysis Status:** ✅ **COMPLETE** - Ready for stakeholder decision
