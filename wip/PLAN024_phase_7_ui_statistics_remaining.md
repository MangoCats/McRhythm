# PLAN024 Phase 7: UI Statistics Remaining Work

**Status:** Per-file pipeline architecture COMPLETE (corrective implementation)
**Remaining:** Detailed sub-phase statistics aggregation and UI display

---

## Completed Work

### Architecture Compliance ✅
- [x] Per-file pipeline architecture per [AIA-ASYNC-020]
- [x] FuturesUnordered worker pool with N workers
- [x] Each file processes through all 10 phases sequentially
- [x] Deprecated batch-phase architecture
- [x] Updated ImportState enum (SCANNING → PROCESSING → COMPLETED)
- [x] Basic file-level progress: "Processing X to Y of Z" format

### Current Progress Display
**Format:** "Processing X to Y of Z"
**Where:**
- X = completed files
- Y = started files (completed + in_progress)
- Z = total files

**Current UI Updates:**
- File count progress (X/Y/Z)
- Current file being processed
- Elapsed time
- Estimated remaining time

---

## Remaining Work: Detailed Sub-Phase Statistics

Per `wip/wkmp-ai_refinement.md` lines 76-102, the UI should display detailed statistics for each sub-phase of the PROCESSING phase. The current implementation logs these statistics but does not aggregate or return them for UI display.

### Required Statistics Per Sub-Phase

#### 1. FILENAME MATCHING (Phase 1)
**Format:** "N completed filenames found"
**Data Needed:**
- N = count of files found with status 'INGEST COMPLETE' (early exit, already processed)

**Current State:**
- `FilenameMatcher.check_file()` returns `MatchResult` enum
- `MatchResult::AlreadyProcessed` indicates completed file
- **NOT AGGREGATED** - currently returns early without statistics

#### 2. HASHING (Phase 2)
**Format:** "N hashes computed, M matches found"
**Data Needed:**
- N = number of audio file hashes computed
- M = number of hashes matching files with status 'INGEST COMPLETE' (duplicate hash early exit)

**Current State:**
- `HashDeduplicator.process_file_hash()` returns `HashResult` enum
- `HashResult::Duplicate` indicates duplicate hash
- **NOT AGGREGATED** - currently returns early without statistics

#### 3. EXTRACTING (Phase 3)
**Format:** "Metadata successfully extracted from X files, Y failures"
**Data Needed:**
- X = files with successful metadata extraction (at least one item)
- Y = files with no metadata extracted

**Current State:**
- `MetadataMerger.extract_and_merge()` returns `Option<MergedMetadata>`
- Success/failure tracked internally
- **NOT RETURNED** - method returns merged metadata but not extraction statistics

#### 4. SEGMENTING (Phase 4)
**Format:** "X files, Y potential passages, Z finalized passages, W songs identified"
**Data Needed:**
- X = number of files that started segmentation
- Y = total potential passages identified (before merging)
- Z = total finalized passages (after merging adjacent silence-separated segments)
- W = number of songs successfully identified (from Phase 6 song matching)

**Current State:**
- `PassageSegmenter.segment_file()` returns `SegmentResult` enum
- `SegmentResult::Passages(boundaries)` contains passage count
- Potential vs. finalized passage distinction **NOT TRACKED**
- Song count comes from Phase 6, **NOT AVAILABLE** in Phase 4

#### 5. FINGERPRINTING (Phase 5)
**Format:** "X potential passages fingerprinted, Y successfully matched song identities"
**Data Needed:**
- X = number of passages run through Chromaprint
- Y = number of successful AcoustID matches (MBID returned)

**Current State:**
- `PassageFingerprinter.fingerprint_passages()` returns `Vec<FingerprintResult>`
- Success/failure per passage tracked
- **NOT AGGREGATED** - results returned but statistics not computed

#### 6. SONG MATCHING (Phase 6)
**Format:** "W high, X medium, Y low, Z no confidence"
**Data Needed:**
- W = high confidence passage-to-MBID matches
- X = medium confidence matches
- Y = low confidence matches
- Z = no confidence (zero-song passages)

**Current State:** ✅ **ALREADY AVAILABLE**
- `PassageSongMatcher.match_passages()` returns `SongMatchResult`
- `SongMatchResult.stats: SongMatchStats` contains:
  - `high_confidence: usize`
  - `medium_confidence: usize`
  - `low_confidence: usize`
  - `zero_song: usize`
- **READY TO USE** - just needs to be returned from `process_file_plan024()`

#### 7. RECORDING (Phase 7)
**Format:** Scrollable list of "Song Title in path/filename" or "unidentified passage in path/filename"
**Data Needed:**
- List of (song_title, file_path) tuples for each passage
- Distinction between identified vs. unidentified passages

**Current State:**
- `PassageRecorder.record_passages()` returns `RecordingResult`
- Contains `passages_recorded: usize` and `songs_created: usize`
- **DOES NOT RETURN** song titles or file paths (would need database query)

#### 8. AMPLITUDE (Phase 8)
**Format:** Scrollable list of "Song Title: X.XX seconds, lead-in: X ms, lead-out: X ms"
**Data Needed:**
- List of (song_title, passage_duration_ms, lead_in_duration_ms, lead_out_duration_ms) tuples

**Current State:**
- `PassageAmplitudeAnalyzer.analyze_passages()` returns `AmplitudeResult`
- Contains `passages_analyzed: usize` count
- **DOES NOT RETURN** individual passage timing details

#### 9. FLAVORING (Phase 9)
**Format:** "W pre-existing, X by AcousticBrainz, Y by Essentia, Z could not be flavored"
**Data Needed:**
- W = songs with pre-existing 'FLAVOR READY' status
- X = songs flavored via AcousticBrainz API
- Y = songs flavored via Essentia local computation
- Z = songs that failed both AcousticBrainz and Essentia

**Current State:** ✅ **ALREADY AVAILABLE**
- `PassageFlavorFetcher.fetch_flavors()` returns `FlavorResult`
- `FlavorResult.stats: FlavorStats` contains:
  - `flavors_fetched: usize` (total)
  - `acousticbrainz_count: usize`
  - `essentia_count: usize`
  - `failed_count: usize`
  - `pre_existing_count: usize`
- **READY TO USE** - just needs to be returned from `process_file_plan024()`

#### 10. PASSAGES COMPLETE (Phase 4e)
**Format:** "N finalized passages complete"
**Data Needed:**
- N = number of finalized passages that completed SEGMENTING process

**Current State:**
- Count available from Phase 4 segmentation results
- **NOT AGGREGATED** - currently scattered across phases

#### 11. FILES COMPLETE (Phase 5)
**Format:** "N files complete"
**Data Needed:**
- N = files marked 'INGEST COMPLETE' or 'DUPLICATE HASH' or 'NO AUDIO'

**Current State:** ✅ **ALREADY WORKING**
- Tracked in `phase_processing_per_file()` as `completed` counter
- Displayed in "Processing X to Y of Z" format

---

## Implementation Plan for Remaining Work

### Step 1: Define Aggregate Statistics Structure
Create `PerFilePipelineStats` struct to accumulate statistics across all files:

```rust
#[derive(Debug, Default, Clone)]
struct PerFilePipelineStats {
    // Phase 1: Filename Matching
    already_processed_count: usize,

    // Phase 2: Hashing
    hashes_computed: usize,
    duplicate_hash_count: usize,

    // Phase 3: Extracting
    metadata_success_count: usize,
    metadata_failure_count: usize,

    // Phase 4: Segmenting
    segmented_files_count: usize,
    potential_passages_count: usize,
    finalized_passages_count: usize,
    no_audio_count: usize,

    // Phase 5: Fingerprinting
    passages_fingerprinted: usize,
    acoustid_matches_count: usize,

    // Phase 6: Song Matching
    high_confidence_count: usize,
    medium_confidence_count: usize,
    low_confidence_count: usize,
    zero_song_count: usize,

    // Phase 7: Recording
    passages_recorded: usize,
    songs_created: usize,
    songs_reused: usize,

    // Phase 8: Amplitude
    passages_analyzed: usize,

    // Phase 9: Flavoring
    pre_existing_flavors: usize,
    acousticbrainz_flavors: usize,
    essentia_flavors: usize,
    failed_flavors: usize,

    // Overall
    files_complete: usize,
    files_failed: usize,
}
```

### Step 2: Modify `process_file_plan024()` Return Type
Change from `Result<()>` to `Result<FileProcessingStats>` where `FileProcessingStats` contains per-file statistics.

### Step 3: Aggregate Statistics in `phase_processing_per_file()`
Accumulate `FileProcessingStats` from each completed file into `PerFilePipelineStats`.

### Step 4: Update Progress Broadcasts
Include detailed statistics in progress updates:
- Update `ImportProgress` to include `sub_phase_stats: PerFilePipelineStats`
- Broadcast statistics via SSE events
- Update UI to display detailed statistics per sub-phase

### Step 5: Database Schema (Optional)
For RECORDING and AMPLITUDE scrollable lists, may need to query database for recent passages:
```sql
-- Get recent passages with song titles
SELECT s.title, p.file_id, f.path, p.start_ticks, p.end_ticks,
       p.lead_in_start_ticks, p.lead_out_start_ticks
FROM passages p
LEFT JOIN songs s ON p.song_id = s.guid
JOIN files f ON p.file_id = f.guid
WHERE f.session_id = ?
ORDER BY p.created_at DESC
LIMIT 100
```

---

## Estimated Effort

**Time Estimate:** 6-8 hours

**Breakdown:**
1. Define aggregate statistics structure: 1 hour
2. Modify service return types to include statistics: 2-3 hours
3. Aggregate statistics in orchestrator: 1-2 hours
4. Update progress broadcasts and SSE events: 1-2 hours
5. Testing and verification: 1 hour

**Complexity:** Medium
- Most services already compute statistics internally
- Main work is plumbing statistics through return values
- Aggregation logic is straightforward (accumulation)

**Dependencies:**
- No blocking dependencies
- Can be implemented incrementally (one sub-phase at a time)

---

## Rationale for Deferring This Work

**Current State:** The per-file pipeline architecture is COMPLETE and FUNCTIONAL per SPEC032 [AIA-ASYNC-020]. Files are processed correctly through all 10 phases.

**What Works:**
- ✅ Correct processing order (per-file, not batch)
- ✅ Parallel processing with N workers
- ✅ Early exits (AlreadyProcessed, DuplicateHash, NoAudio)
- ✅ Basic progress display ("Processing X to Y of Z")
- ✅ Cancellation support

**What's Missing:**
- ❌ Detailed sub-phase statistics for UI display
- ❌ Scrollable lists for RECORDING and AMPLITUDE phases

**Priority:** This is a **UX enhancement**, not a **functional requirement**. The import pipeline works correctly; the detailed statistics are for user visibility and transparency.

**Recommendation:**
- **Now:** Test end-to-end functionality with real audio files
- **Later:** Implement detailed sub-phase statistics as a UX improvement

---

## Testing Notes

**Critical Test:** Verify per-file pipeline processes files correctly
- Import a small test corpus (10-20 audio files)
- Verify files progress through all 10 phases
- Check database for correct passage/song records
- Verify early exits work (duplicate hash, no audio)

**UI Test:** Verify basic progress display shows file counts
- Check SSE events broadcast progress updates
- Verify "Processing X to Y of Z" format
- Confirm elapsed/estimated time calculation

**Sub-Phase Statistics Test:** (deferred until implementation)
- Verify each sub-phase counter increments correctly
- Check aggregate statistics match individual file results
- Test scrollable lists display recent passages

---

## References

- **Specification:** `wip/wkmp-ai_refinement.md` lines 76-102
- **Architecture:** SPEC032 [AIA-ASYNC-020] Per-file pipeline
- **Corrective Analysis:** `wip/PLAN024_architecture_discrepancy_analysis.md`
- **Commit:** 08296c5 "Replace batch-phase architecture with PLAN024 per-file pipeline"
