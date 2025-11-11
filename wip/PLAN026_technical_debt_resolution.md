# SPEC026: Technical Debt Resolution - Critical and High Priority Issues

**Document Type:** Implementation Specification
**Status:** Draft
**Created:** 2025-11-10
**Related Analysis:** Technical Debt Report (generated 2025-11-10)
**Priority:** Critical (Blocks Production Use)

---

## Executive Summary

This specification defines implementation requirements for resolving **3 CRITICAL** and **8 HIGH** severity technical debt issues discovered during PLAN023 (wkmp-ai recode) analysis. These issues block production deployment and degrade core functionality.

**Primary Impact:** Boundary detection completely non-functional - all audio files treated as single passages regardless of track gaps.

**Secondary Impacts:** Missing amplitude analysis, non-functional metadata validation, incomplete API integration.

**Target Completion:** Sprint 1 (Critical), Sprint 2 (High), Sprint 3 (Medium)

---

## Problem Statement

### Current State

1. **Boundary Detection Stub**: SessionOrchestrator hardcodes single passage per file, ignoring actual `SilenceDetector` implementation
2. **Audio Segment Extraction Missing**: Cannot extract time-range subsets from audio files for per-passage processing
3. **Amplitude Analysis Stubbed**: Entire `/analyze/amplitude` endpoint returns hardcoded fake data
4. **Consistency Checker Ineffective**: Always returns `Pass`, never detects metadata conflicts
5. **Event Bridge Incomplete**: Missing `session_id` fields prevent session correlation
6. **Flavor Synthesis Skipped**: Multi-source flavor fusion not happening
7. **MBID Extraction Blocked**: Cannot read MusicBrainz IDs from ID3 UFID frames (library limitation)
8. **Chromaprint Compression**: Using raw fingerprints instead of compressed format

### Business Impact

- **Users cannot import multi-track albums** - primary use case blocked
- **Metadata quality degraded** - conflicts not detected or reported
- **API endpoints non-functional** - amplitude analysis returns fake data
- **Event correlation broken** - cannot track session progress across events

---

## Requirements

### CRITICAL Requirements (Sprint 1 - MUST Fix Before Production)

#### REQ-TD-001: Functional Boundary Detection

**Priority:** CRITICAL
**Effort:** 4-6 hours
**Location:** [session_orchestrator.rs:232-239](wkmp-ai/src/import_v2/session_orchestrator.rs#L232-L239)

**SHALL:**
1. Replace boundary detection stub with actual `SilenceDetector` integration
2. Detect multiple passages within single audio file based on silence gaps
3. Configure silence detector with parameters:
   - Threshold: -60dB (vinyl preset)
   - Minimum silence duration: 0.5 seconds
   - RMS window: 100ms (4410 samples @ 44.1kHz)
4. Convert `SilenceRegion` output to `PassageBoundary` format (ticks-based)
5. Handle audio files with no silence gaps (single passage, full file duration)
6. Handle audio files with multiple silence gaps (create passage per track)

**Acceptance Criteria:**
- Album with 10 tracks separated by 2-second silence gaps → detected as 10 passages
- Single song file with no silence → detected as 1 passage spanning full duration
- Silence regions <0.5 seconds ignored (not passage boundaries)
- All passage boundaries expressed in ticks (28,224,000 Hz tick rate)

**Test Requirements:**
- Unit test: `SilenceDetector` with known audio patterns
- Integration test: SessionOrchestrator processes multi-track file
- System test: End-to-end import of album FLAC file with metadata

---

#### REQ-TD-002: Audio Segment Extraction

**Priority:** CRITICAL
**Effort:** 6-8 hours
**Location:** [song_workflow_engine.rs:252-253](wkmp-ai/src/import_v2/song_workflow_engine.rs#L252-L253)

**SHALL:**
1. Implement `extract_audio_segment()` method in `AudioLoader` or new `AudioSegmentExtractor`
2. Use symphonia to decode audio file
3. Extract samples for specified time range (start_ticks, end_ticks)
4. Resample to target sample rate if needed (using rubato)
5. Convert to mono if stereo (channel averaging)
6. Return PCM samples as `Vec<f32>` normalized to [-1.0, 1.0]

**Input:**
- File path: `&Path`
- Start position: `i64` (ticks)
- End position: `i64` (ticks)
- Target sample rate: `Option<u32>` (default: preserve source rate)

**Output:**
- `Result<Vec<f32>>` - PCM samples for time range

**Error Handling:**
- File not found → `ImportError::FileNotFound`
- Decode failure → `ImportError::DecodeFailed`
- Time range invalid (start > end) → `ImportError::InvalidTimeRange`
- Time range exceeds file duration → `ImportError::TimeRangeOutOfBounds`

**Acceptance Criteria:**
- Extract 30-second segment from 3-minute file → exactly 30 seconds of audio
- Extract segment starting at 1:45.000 → samples start at precise tick position
- Stereo input → mono output via channel averaging
- 48kHz input, request 44.1kHz → resampled output at 44.1kHz

**Test Requirements:**
- Unit test: Extract known segment from test WAV file, verify duration and sample count
- Integration test: `SongWorkflowEngine` uses extracted segment for fingerprinting
- Edge case test: Extract full file duration (start=0, end=file_duration)
- Error test: Request time range beyond file duration

---

#### REQ-TD-003: Remove or Implement Amplitude Analysis

**Priority:** CRITICAL (User-Facing API)
**Effort:** 2 hours (remove) OR 8-12 hours (implement)
**Location:** [api/amplitude_analysis.rs:24-35](wkmp-ai/src/api/amplitude_analysis.rs#L24-L35)

**Decision Required:** Remove stub endpoint OR implement functionality

**Option A: Remove Stub (RECOMMENDED for Sprint 1)**

SHALL:
1. Remove `/analyze/amplitude` endpoint from API routes
2. Remove `AmplitudeAnalysisRequest` and `AmplitudeAnalysisResponse` models
3. Update API documentation to remove references
4. Add TODO comment: "Amplitude analysis deferred to future release"

Effort: 2 hours
Risk: Low (removes non-functional feature)

**Option B: Implement Functionality**

SHALL:
1. Load audio file using `AudioLoader`
2. Calculate RMS amplitude profile (windowed RMS every 100ms)
3. Detect peak RMS value
4. Detect lead-in duration (time to reach 80% of peak RMS)
5. Detect lead-out duration (time from 80% peak to silence)
6. Detect quick ramp-up (reaches 80% peak within 2 seconds)
7. Detect quick ramp-down (drops to silence within 2 seconds)
8. Return actual analysis results (not stub data)

Effort: 8-12 hours
Risk: Medium (new feature implementation)

**Acceptance Criteria (Option B):**
- Analyze song with 2-second fade-in → lead_in_duration ≈ 2.0 seconds
- Analyze song with 3-second fade-out → lead_out_duration ≈ 3.0 seconds
- Analyze song with sudden start → quick_ramp_up = true
- RMS profile has time resolution of 100ms (one value per 100ms)

**Test Requirements (Option B):**
- Unit test: Analyze known test audio file, verify metrics
- Integration test: API endpoint returns real data (not stub values)
- Edge case test: Analyze silent audio file

**Recommendation:** Option A (remove) for Sprint 1, implement in future release when use case is clear.

---

### HIGH Priority Requirements (Sprint 2 - Should Fix Soon)

#### REQ-TD-004: MBID Extraction from ID3 Tags

**Priority:** HIGH
**Effort:** 4-6 hours (workaround) OR blocked by library
**Location:** [tier1/id3_extractor.rs:208-209](wkmp-ai/src/import_v2/tier1/id3_extractor.rs#L208-L209)

**Problem:** `lofty` crate does not expose UFID (Unique File Identifier) frames via public API

**Workaround Strategy:**

SHALL:
1. Attempt to read UFID frame using lofty's raw tag access (if available)
2. If lofty doesn't support, use `id3` crate as fallback for MP3 files only
3. Search UFID frames for MusicBrainz identifier (`http://musicbrainz.org`)
4. Parse MBID from UFID frame data
5. Return as `Option<Uuid>`

**Acceptance Criteria:**
- MP3 file with MusicBrainz UFID frame → extract MBID successfully
- FLAC file with MusicBrainz vorbis comment → extract MBID (if supported)
- File without MBID → return `None` (fallback to AcoustID fingerprinting)

**Test Requirements:**
- Unit test: Read MBID from test MP3 file with known UFID frame
- Integration test: Workflow uses embedded MBID instead of fingerprinting

**Fallback:** If workaround not feasible, document limitation and rely on AcoustID fingerprinting

---

#### REQ-TD-005: Consistency Checker Implementation

**Priority:** HIGH
**Effort:** 6-8 hours
**Location:** [tier3/consistency_checker.rs:51-70](wkmp-ai/src/import_v2/tier3/consistency_checker.rs#L51-L70)

**Current Problem:** Validation methods always return `Pass` because they only have access to fused result, not all candidates

**Solution Approach:**

SHALL:
1. Modify `MetadataFuser` to preserve all candidates alongside fused result
2. Pass `MetadataBundle` (all candidates) to `ConsistencyChecker` instead of `FusedMetadata`
3. Implement `validate_title()` to compare all title candidates:
   - If any two candidates differ significantly (strsim < 0.85) → `Conflict`
   - If all candidates similar but minor differences → `Warning`
   - If all identical or only one candidate → `Pass`
4. Implement same logic for `validate_artist()` and `validate_album()`
5. Return `ValidationResult` with specific conflict messages

**Acceptance Criteria:**
- ID3 title "The Beatles" vs. MusicBrainz "Beatles, The" → Conflict detected
- ID3 title "Let It Be" vs. MusicBrainz "Let It Be" → Pass
- ID3 title "Let it Be" vs. MusicBrainz "Let It Be" (case difference) → Warning

**Test Requirements:**
- Unit test: Two conflicting candidates → Conflict returned
- Unit test: Similar candidates with typo → Warning returned
- Unit test: Identical candidates → Pass returned
- Integration test: Workflow reports metadata conflicts to user

---

#### REQ-TD-006: Event Bridge session_id Fields

**Priority:** HIGH
**Effort:** 2-3 hours
**Location:** [event_bridge.rs:110, 128, 147, etc.](wkmp-ai/src/event_bridge.rs#L110)

**SHALL:**
1. Add `session_id` field to `ImportEvent` variants that currently use `Uuid::nil()`:
   - `PassagesDiscovered`
   - `SongStarted`
   - `SongProgress`
   - `SongComplete`
   - `SongFailed`
   - `SessionProgress`
   - `SessionComplete`
   - `SessionFailed`
2. Update `ImportEventBridge::convert()` to extract `session_id` from events
3. Include `session_id` in converted `WkmpEvent` payloads

**Acceptance Criteria:**
- All `ImportEvent` variants include valid `session_id` (no `Uuid::nil()`)
- Event stream filters by `session_id` correctly
- UI can correlate all events for single import session

**Test Requirements:**
- Unit test: Event conversion preserves `session_id`
- Integration test: UI receives events with correct `session_id`

---

#### REQ-TD-007: Flavor Synthesis Implementation

**Priority:** HIGH
**Effort:** 4-6 hours
**Location:** [song_workflow_engine.rs:369-370](wkmp-ai/src/import_v2/song_workflow_engine.rs#L369-L370)

**SHALL:**
1. Convert `ExtractorResult<MusicalFlavor>` from audio analysis to `FlavorExtraction`
2. Pass all flavor sources to `FlavorSynthesizer`:
   - Audio-derived features (tempo, energy, spectral)
   - AcousticBrainz data (if available)
   - Genre inference (if available)
3. Synthesize weighted flavor vector combining all sources
4. Calculate synthesis confidence based on source agreement
5. Return `SynthesizedFlavor` with provenance (sources used)

**Acceptance Criteria:**
- Multiple flavor sources available → synthesized flavor combines all
- Single flavor source → synthesis uses that source with appropriate confidence
- No flavor sources → default flavor with low confidence

**Test Requirements:**
- Unit test: Two agreeing sources → high synthesis confidence
- Unit test: Two conflicting sources → lower synthesis confidence
- Integration test: Workflow uses synthesized flavor in passage metadata

---

#### REQ-TD-008: Chromaprint Compressed Fingerprint

**Priority:** HIGH
**Effort:** 3-4 hours
**Location:** [tier1/chromaprint_analyzer.rs:93-94](wkmp-ai/src/import_v2/tier1/chromaprint_analyzer.rs#L93-L94)

**Current Problem:** Using raw fingerprint + hash instead of Chromaprint's standard compressed format

**Workaround (if library doesn't support compression):**

SHALL:
1. Check if `chromaprint-rust` exposes compression API
2. If YES: Use native compression
3. If NO: Implement minimal compression algorithm:
   - Encode fingerprint as base64 string (AcoustID standard)
   - Store both raw and compressed versions
4. Update database schema to store compressed fingerprint
5. Use compressed format for AcoustID API queries

**Acceptance Criteria:**
- Fingerprint stored in AcoustID-compatible compressed format
- AcoustID API accepts fingerprint without errors
- Compressed size ~50-70% smaller than raw fingerprint

**Test Requirements:**
- Unit test: Compress fingerprint, verify base64 encoding
- Integration test: AcoustID lookup succeeds with compressed fingerprint

---

### MEDIUM Priority Requirements (Sprint 3 - Defer to Future)

#### REQ-TD-009: Waveform Rendering

**Priority:** MEDIUM
**Effort:** 6-8 hours
**Location:** [api/ui.rs:864](wkmp-ai/src/api/ui.rs#L864)

**Deferred:** Not blocking core functionality, can implement in future release

---

#### REQ-TD-010: Duration Tracking in File Stats

**Priority:** MEDIUM
**Effort:** 1-2 hours
**Location:** [session_orchestrator.rs:430](wkmp-ai/src/import_v2/session_orchestrator.rs#L430)

**Deferred:** Nice-to-have feature, not critical

---

#### REQ-TD-011: Flavor Confidence Calculation

**Priority:** MEDIUM
**Effort:** 2-3 hours
**Location:** [session_orchestrator.rs:521](wkmp-ai/src/import_v2/session_orchestrator.rs#L521)

**Deferred:** Will be addressed by REQ-TD-007 (Flavor Synthesis)

---

#### REQ-TD-012: Flavor Data Persistence

**Priority:** MEDIUM
**Effort:** 2-3 hours
**Location:** [db_repository.rs:173](wkmp-ai/src/import_v2/db_repository.rs#L173)

**Deferred:** Blocked by flavor synthesis implementation (REQ-TD-007)

---

## Implementation Strategy

### Sprint 1: Critical Path (Week 1)

**Goal:** Unblock production use - fix boundary detection and remove stub endpoints

**Increments:**
1. **Integrate SilenceDetector** (REQ-TD-001) - 4-6 hours
2. **Implement Audio Segment Extraction** (REQ-TD-002) - 6-8 hours
3. **Remove Amplitude Analysis Stub** (REQ-TD-003 Option A) - 2 hours

**Total Effort:** 12-16 hours
**Deliverable:** Users can import multi-track albums successfully

---

### Sprint 2: High Priority (Week 2)

**Goal:** Improve metadata quality and event correlation

**Increments:**
1. **MBID Extraction Workaround** (REQ-TD-004) - 4-6 hours
2. **Consistency Checker Implementation** (REQ-TD-005) - 6-8 hours
3. **Event Bridge session_id** (REQ-TD-006) - 2-3 hours
4. **Flavor Synthesis** (REQ-TD-007) - 4-6 hours
5. **Chromaprint Compression** (REQ-TD-008) - 3-4 hours

**Total Effort:** 19-27 hours
**Deliverable:** Metadata quality improved, events properly correlated

---

### Sprint 3: Medium Priority (Future Release)

**Goal:** Polish and completeness

**Increments:**
- Waveform rendering
- Duration tracking
- Flavor data persistence (depends on Sprint 2 synthesis)

**Total Effort:** 10-14 hours
**Deliverable:** Feature completeness

---

## Dependencies

### Existing Components (No Changes Required)

- `SilenceDetector` - [services/silence_detector.rs](wkmp-ai/src/services/silence_detector.rs) - Already implemented
- `AudioLoader` - [tier1/audio_loader.rs](wkmp-ai/src/import_v2/tier1/audio_loader.rs) - Needs extension
- `BoundaryFuser` - [tier2/boundary_fuser.rs](wkmp-ai/src/import_v2/tier2/boundary_fuser.rs) - Works correctly
- `SessionOrchestrator` - [session_orchestrator.rs](wkmp-ai/src/import_v2/session_orchestrator.rs) - Needs stub removal

### External Libraries

- `symphonia` - Audio decoding (already in use)
- `rubato` - Resampling (already in use)
- `strsim` - String similarity for validation (already in use)
- `id3` - Fallback for UFID extraction (may need to add)

---

## Success Metrics

### Quantitative

- ✅ Multi-track album (10 songs) → detected as 10 passages (not 1)
- ✅ Test suite: 100% of new code covered by tests
- ✅ Performance: Boundary detection <200ms per file (current: instant but wrong)
- ✅ Performance: Segment extraction <100ms per passage

### Qualitative

- ✅ User can import album FLAC files without manual intervention
- ✅ Metadata conflicts visible to user during import
- ✅ Event streams properly correlated to import sessions
- ✅ No stub endpoints returning fake data

---

## Constraints

### Technical

- Must maintain backward compatibility with existing database schema
- Must use existing libraries (symphonia, rubato, lofty)
- Must follow PLAN023 architecture (3-tier hybrid fusion)

### Timeline

- Sprint 1 (Critical): 1 week
- Sprint 2 (High): 1-2 weeks
- Sprint 3 (Medium): Future release (not time-bound)

---

## Risk Assessment

### Highest Risk: Audio Segment Extraction (REQ-TD-002)

**Risk:** Symphonia API complexity may introduce edge cases

**Mitigation:**
- Start with simple test cases (WAV files, common formats)
- Add comprehensive error handling
- Test with multiple audio formats (FLAC, MP3, WAV, AAC)
- Unit test boundary conditions (zero-length segments, full-file segments)

**Residual Risk:** Low (library is mature, widely used)

---

### Medium Risk: MBID Extraction (REQ-TD-004)

**Risk:** `lofty` library may not expose UFID frames at all

**Mitigation:**
- Fallback to `id3` crate for MP3 files
- Document limitation for FLAC files (no UFID access)
- Rely on AcoustID fingerprinting as alternative
- Add library feature request to lofty project

**Residual Risk:** Low-Medium (workaround exists, but may be incomplete)

---

## Acceptance Tests Summary

### REQ-TD-001: Boundary Detection
- TC-TD-001-01: Import 10-track album → 10 passages detected
- TC-TD-001-02: Import single song → 1 passage detected
- TC-TD-001-03: Silence <0.5s ignored (not boundary)

### REQ-TD-002: Segment Extraction
- TC-TD-002-01: Extract 30s from 3min file → exactly 30s
- TC-TD-002-02: Extract starting at precise tick → correct position
- TC-TD-002-03: Stereo input → mono output
- TC-TD-002-04: Time range exceeds file → error

### REQ-TD-003: Amplitude Analysis
- TC-TD-003-01: Endpoint removed from routes
- TC-TD-003-02: API documentation updated

### REQ-TD-004: MBID Extraction
- TC-TD-004-01: Read MBID from MP3 UFID frame
- TC-TD-004-02: File without MBID → returns None

### REQ-TD-005: Consistency Checker
- TC-TD-005-01: Conflicting candidates → Conflict
- TC-TD-005-02: Similar candidates → Warning
- TC-TD-005-03: Identical candidates → Pass

### REQ-TD-006: Event Bridge
- TC-TD-006-01: All events include valid session_id
- TC-TD-006-02: UI correlates events correctly

### REQ-TD-007: Flavor Synthesis
- TC-TD-007-01: Multiple sources → combined flavor
- TC-TD-007-02: Agreeing sources → high confidence

### REQ-TD-008: Chromaprint Compression
- TC-TD-008-01: Fingerprint compressed to base64
- TC-TD-008-02: AcoustID accepts compressed format

---

## Approval and Sign-Off

**Specification Status:** Ready for Planning
**Next Step:** Run `/plan` to generate implementation plan

**Estimated Total Effort:**
- Sprint 1 (Critical): 12-16 hours
- Sprint 2 (High): 19-27 hours
- Sprint 3 (Medium): 10-14 hours
- **Total: 41-57 hours** (5-7 days of focused work)
