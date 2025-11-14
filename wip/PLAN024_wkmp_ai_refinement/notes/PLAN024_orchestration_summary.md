# PLAN024 Pipeline Orchestration Summary

**Date:** 2025-11-13
**Status:** Orchestration layer added - Integration work remaining
**Branch:** ai-trial2

---

## Summary

Added pipeline orchestration method to WorkflowOrchestrator that wires all 10 PLAN024 phases together. The orchestration layer is structurally complete but requires audio decoding integration to be fully functional.

---

## What Was Added

### New Method: `process_file_plan024()`

**Location:** [wkmp-ai/src/services/workflow_orchestrator/mod.rs:2141-2365](../wkmp-ai/src/services/workflow_orchestrator/mod.rs#L2141-L2365)

**Signature:**
```rust
pub async fn process_file_plan024(
    &self,
    file_path: &std::path::Path,
    root_folder: &std::path::Path,
    samples: &[f32],          // Decoded PCM audio samples (mono)
    sample_rate: usize,       // Sample rate in Hz
) -> Result<()>
```

**Functionality:**
- Wires all 10 phases sequentially with proper data flow
- Early exit logic for AlreadyProcessed, Duplicate, NoAudio
- Type-safe error handling throughout
- Comprehensive debug/info logging

---

## Pipeline Flow

```
1. Filename Matching
   ├─ Calculate relative path from root folder
   ├─ Check if file exists in database
   └─ Create new record OR reuse existing OR skip if already processed

2. Hash Deduplication
   ├─ Calculate SHA-256 hash of file content
   ├─ Check for duplicate hashes
   └─ Skip pipeline if duplicate found

3. Metadata Extraction & Merging
   ├─ Extract ID3/FLAC tags via lofty
   ├─ Merge with existing metadata (new overwrites old)
   └─ Update files table (artist, title, album, etc.)

4. Passage Segmentation
   ├─ Load settings from database (silence thresholds)
   ├─ Detect silence regions via SilenceDetector
   ├─ Calculate passage boundaries (non-silence regions)
   └─ Skip pipeline if NO AUDIO detected

5. Per-Passage Fingerprinting
   ├─ Get AcoustID API key from settings
   ├─ Generate Chromaprint fingerprints per passage
   ├─ Query AcoustID API for MBID candidates
   └─ Skip if passage < 10 seconds (Chromaprint minimum)

6. Song Matching
   ├─ Combine metadata + fingerprint evidence
   ├─ Calculate confidence scores (High/Medium/Low/None)
   ├─ Merge adjacent zero-song passages
   └─ Return matches with statistics

7. Recording
   ├─ Begin database transaction
   ├─ Get-or-create songs by MBID
   ├─ Create passage records (start/end ticks, song_id)
   ├─ Commit transaction
   └─ Return passage records for next phase

8. Amplitude Analysis
   ├─ Load lead-in/lead-out thresholds from settings
   ├─ Analyze RMS amplitude per passage
   ├─ Detect lead-in/lead-out timing (crossfade points)
   ├─ Update passages table (lead_in_start_ticks, lead_out_start_ticks)
   └─ Set passage status = 'INGEST COMPLETE'

9. Flavoring
   ├─ Collect unique song IDs from passages
   ├─ Query AcousticBrainz API for flavor vectors
   ├─ Fallback to Essentia if AcousticBrainz fails
   ├─ Update songs table (flavor_vector, flavor_source_blend)
   └─ Set song status = 'FLAVOR READY'

10. Finalization
    ├─ Validate all passages: status = 'INGEST COMPLETE'
    ├─ Validate all songs: status = 'FLAVOR READY'
    ├─ If validation passes: Update files.status = 'INGEST COMPLETE'
    └─ If validation fails: Return errors, leave file status unchanged
```

---

## Early Exit Conditions

**Phase 1: Filename Matching**
- **Condition:** `MatchResult::AlreadyProcessed(guid)`
- **Action:** Log info, return `Ok(())`
- **Reason:** File already ingested, no need to reprocess

**Phase 2: Hash Deduplication**
- **Condition:** `HashResult::Duplicate { hash, original_file_id }`
- **Action:** Log info with hash and original file ID, return `Ok(())`
- **Reason:** Duplicate content detected, skip ingest

**Phase 4: Passage Segmentation**
- **Condition:** `SegmentResult::NoAudio`
- **Action:** Log info, return `Ok(())`
- **Reason:** File contains <100ms of non-silence audio

---

## Integration Requirements (TODO)

### Audio Decoding Infrastructure

**Current Issue:** The method signature requires pre-decoded PCM samples, but no decoding helper exists in the orchestrator.

**What's Needed:**
1. **Audio Decoding Helper**
   - Use symphonia to decode audio files to mono f32 PCM
   - Handle multiple formats (MP3, FLAC, AAC, etc.)
   - Resample to consistent sample rate if needed
   - Error handling for corrupt/unsupported files

2. **Wrapper Method**
   Create a higher-level method that handles decoding:
   ```rust
   pub async fn process_file_plan024_with_decoding(
       &self,
       file_path: &std::path::Path,
       root_folder: &std::path::Path,
   ) -> Result<()> {
       // Decode audio using symphonia
       let (samples, sample_rate) = decode_audio(file_path)?;

       // Call existing method with decoded samples
       self.process_file_plan024(file_path, root_folder, &samples, sample_rate).await
   }
   ```

3. **Usage in UI**
   - wkmp-ai import wizard calls `process_file_plan024_with_decoding()`
   - Progress reporting via SSE
   - Cancel/pause functionality via tokio::CancellationToken

**Estimated Work:** 2-3 hours
- 1 hour: Audio decoding helper
- 1 hour: Wrapper method + error handling
- 1 hour: UI integration + testing

---

## Error Handling

**Pattern Used Throughout:**
- All phases return `Result<T>` using `wkmp_common::Error`
- anyhow::Result for orchestration layer
- `?` operator for error propagation
- Detailed error context in tracing logs

**Error Types:**
- `Error::Database` - SQLite errors
- `Error::Io` - File I/O errors
- `Error::InvalidInput` - Invalid UTF-8, path not under root folder
- `Error::Internal` - Service initialization failures, JSON serialization

**Failure Behavior:**
- If any phase fails, entire pipeline fails
- Database transactions rolled back automatically
- File status remains in last known state
- Error logged with full context (file_id, file_path, phase)

---

## Logging Strategy

**Debug Level:**
- Phase entry: `"Phase N: [Phase Name]"`
- Phase completion: Statistics (passages, matches, songs, etc.)
- Internal state: File IDs, hashes, counts

**Info Level:**
- Pipeline start: `"Starting PLAN024 10-phase per-file pipeline"`
- Early exits: Detailed reason (AlreadyProcessed, Duplicate, NoAudio)
- Pipeline complete: `"PLAN024 pipeline complete - File ingested successfully"`

**Error Level:**
- Phase failures: Full error context
- Finalization validation failures: List of validation errors

---

## Code Metrics

**Lines Added:** 247 lines
**Method Length:** 225 lines (2141-2365)
**Complexity:** Sequential pipeline (low cyclomatic complexity)

**Structure:**
- 10 phase blocks (one per phase)
- 3 early exit points
- 20+ tracing::debug statements
- 3 tracing::info statements (start, exits, complete)
- 1 tracing::error statement (finalization failure)

---

## Testing Status

**Unit Tests:** None added (orchestration method)
**Integration Tests:** Not yet implemented

**Testing Strategy:**
1. **Unit Tests (per phase):** Already exist (47 tests, all passing)
2. **Integration Tests (pipeline end-to-end):** Future work
   - Create test audio fixture
   - Run through full pipeline
   - Verify database records created correctly
   - Test early exit scenarios

**Estimated Testing Work:** 4-6 hours
- 2 hours: End-to-end test with real audio
- 2 hours: Early exit scenario tests
- 1 hour: Error handling tests
- 1 hour: Multi-file workflow tests

---

## Next Steps

### Immediate (2-3 hours)
1. **Add audio decoding helper**
   - Create `decode_audio()` function using symphonia
   - Handle format detection, decoding, mono conversion
   - Add error handling for unsupported formats

2. **Create wrapper method**
   - `process_file_plan024_with_decoding()` that calls decoding + pipeline
   - Document usage examples

3. **Manual testing**
   - Test with MP3, FLAC, AAC files
   - Verify database records
   - Check early exit paths

### Future (4-6 hours)
1. **UI Integration**
   - Connect to wkmp-ai import wizard
   - Add progress reporting via SSE
   - Implement cancel/pause functionality

2. **Integration Tests**
   - End-to-end pipeline tests
   - Error handling scenarios
   - Multi-file workflows

3. **Performance Optimization**
   - Parallel file processing
   - Batch fingerprinting
   - Connection pooling

---

## Dependencies

**Internal Services Used:**
- `FilenameMatcher` (Phase 1)
- `HashDeduplicator` (Phase 2)
- `MetadataMerger` (Phase 3)
- `PassageSegmenter` (Phase 4)
- `PassageFingerprinter` (Phase 5)
- `PassageSongMatcher` (Phase 6)
- `PassageRecorder` (Phase 7)
- `PassageAmplitudeAnalyzer` (Phase 8)
- `PassageFlavorFetcher` (Phase 9)
- `PassageFinalizer` (Phase 10)

**External Dependencies:**
- sqlx (database queries)
- uuid (file IDs)
- tracing (logging)
- anyhow (error handling)

**Optional External APIs:**
- AcoustID API (fingerprinting - Phase 5)
- AcousticBrainz API (flavoring - Phase 9)
- Essentia binary (fallback flavoring - Phase 9)

---

## Traceability

**Requirement:** [REQ-SPEC032-007] Per-File Import Pipeline
**Phases Implemented:**
- [REQ-SPEC032-008] Filename Matching (Phase 1)
- [REQ-SPEC032-009] Hash Deduplication (Phase 2)
- [REQ-SPEC032-010] Metadata Extraction (Phase 3)
- [REQ-SPEC032-011] Passage Segmentation (Phase 4)
- [REQ-SPEC032-012] Per-Passage Fingerprinting (Phase 5)
- [REQ-SPEC032-013] Song Matching (Phase 6)
- [REQ-SPEC032-014] Recording (Phase 7)
- [REQ-SPEC032-015] Amplitude Analysis (Phase 8)
- [REQ-SPEC032-016] Flavoring (Phase 9)
- [REQ-SPEC032-017] Finalization (Phase 10)

---

## Commit History

**Commit:** `bb627ed` - Add PLAN024 pipeline orchestration to WorkflowOrchestrator
**Files Modified:** 1 (workflow_orchestrator/mod.rs)
**Lines Changed:** +247
**Build Status:** ✅ Compiles cleanly (warnings only)

---

## Status Summary

✅ **Orchestration layer complete**
✅ **All 10 phases wired together**
✅ **Early exit logic implemented**
✅ **Error handling comprehensive**
✅ **Logging strategy implemented**
⏳ **Audio decoding integration pending**
⏳ **UI integration pending**
⏳ **Integration tests pending**

**Overall Completion:** ~75% (Phases 1-10 complete, integration work remaining)
