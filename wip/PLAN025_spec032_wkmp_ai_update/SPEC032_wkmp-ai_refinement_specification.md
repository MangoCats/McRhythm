# SPEC032 wkmp-ai Refinement Specification

**Document Type:** Implementation Planning Specification
**Purpose:** Define requirements for updating SPEC032 and wkmp-ai implementation to match refined workflow
**Source:** [wip/wkmp-ai_refinement.md](wkmp-ai_refinement.md)
**Target Documents:** [docs/SPEC032-audio_ingest_architecture.md](../docs/SPEC032-audio_ingest_architecture.md)
**Target Code:** wkmp-ai microservice
**Date:** 2025-11-12
**Status:** Ready for /plan workflow

---

## Executive Summary

This specification defines changes required to align SPEC032 and wkmp-ai implementation with the refined import workflow documented in [wkmp-ai_refinement.md](wkmp-ai_refinement.md). The refinement focuses wkmp-ai exclusively on automatic audio file ingest, removing quality control and manual editing features (to be handled by future wkmp-qa and wkmp-pe microservices).

**Key Changes:**
1. **Scope Reduction:** Remove out-of-scope features (quality control, manual passage editing)
2. **Two-Stage Roadmap:** Stage One (root folder only), Stage Two (external folder import with file movement)
3. **Refined Workflow:** 5-step sequential process (API key validation → folder selection → scanning → processing → completion)
4. **Enhanced Processing:** 10-phase per-file pipeline with improved duplicate detection, hash-based deduplication, and flexible song matching
5. **Comprehensive UI Progress Display:** 13 real-time SSE-driven status sections
6. **Database-Driven Settings:** All thresholds and parameters stored in database settings table

**Effort Estimate:** Medium-Large (15-25 hours)
- SPEC032 update: 4-6 hours
- Code refactoring: 8-12 hours
- UI progress display: 3-5 hours
- Testing and validation: 2-4 hours

---

## Document Information

**Stakeholders:**
- wkmp-ai development team
- System architects
- End users (music library import)

**Dependencies:**
- [SPEC032-audio_ingest_architecture.md](../docs/SPEC032-audio_ingest_architecture.md) - Architecture specification to be updated
- [REQ001-requirements.md](../docs/REQ001-requirements.md) - May require requirement updates
- [SPEC002-crossfade.md](../docs/SPEC002-crossfade.md) - Passage timing definitions
- [SPEC017-sample_rate_conversion.md](../docs/SPEC017-sample_rate_conversion.md) - Tick time units
- [IMPL001-database_schema.md](../docs/IMPL001-database_schema.md) - Database schema (files, passages, songs tables)

**Timeline:**
- Specification review: Immediate
- SPEC032 update: 1-2 days
- Implementation: 3-5 days
- Testing: 1-2 days

---

## Problems to Solve

### Problem 1: Scope Creep and Unclear Boundaries

**Current State:**
- SPEC032 includes quality control features (detecting skips, gaps, audio quality issues)
- SPEC032 includes manual passage editing features (user-directed fade point definition, metadata revision)
- Unclear which features belong in wkmp-ai vs. future microservices

**Desired State:**
- wkmp-ai focuses exclusively on automatic audio file ingest
- Quality control explicitly moved to future wkmp-qa microservice
- Manual passage editing explicitly moved to future wkmp-pe (Passage Editor) microservice
- Clear architectural boundaries documented

**Impact:**
- Development focus improved (no ambiguity about what to implement)
- Architectural clarity for future microservice development
- User expectations properly managed

### Problem 2: Missing Two-Stage Development Roadmap

**Current State:**
- SPEC032 does not distinguish between initial release (root folder only) and future enhancements
- Import from external folders with file movement is mixed with basic import features

**Desired State:**
- **Stage One:** Root folder and subfolder import only (initial release)
- **Stage Two:** External folder scanning with file movement to root folder (future enhancement)
- Clear distinction in both specification and implementation

**Impact:**
- Enables incremental delivery (Stage One complete and shippable)
- Prevents scope creep during initial development
- Clear roadmap for users and stakeholders

### Problem 3: Workflow Steps Not Clearly Enumerated

**Current State:**
- SPEC032 describes workflow as "state machine" with batch phases
- High-level states (SCANNING, PROCESSING, COMPLETED) lack detailed substep enumeration
- User-facing workflow steps (API key validation, folder selection) not explicitly documented

**Desired State:**
- 5 top-level workflow steps clearly enumerated:
  1. AcoustID API key validation
  2. Folder selection (root folder or subfolder)
  3. Scanning (file discovery)
  4. Processing (per-file pipeline)
  5. Completion
- Each step's purpose, inputs, outputs, and error handling documented

**Impact:**
- Implementation clarity (exact sequence of operations)
- UI design guidance (which steps require user interaction)
- Testing guidance (step boundaries define test cases)

### Problem 4: Per-File Processing Pipeline Not Fully Specified

**Current State:**
- SPEC032 describes components (file_scanner, metadata_extractor, fingerprinter, etc.)
- Processing phases documented at high level (EXTRACTING, FINGERPRINTING, SEGMENTING, etc.)
- Exact sequence of operations per file not enumerated as distinct phases

**Desired State:**
- 10-phase per-file processing pipeline clearly enumerated:
  1. **FILENAME MATCHING:** Detect previously completed files by path/name/metadata
  2. **HASHING:** Compute content hash for duplicate detection
  3. **EXTRACTING:** ID3 metadata extraction
  4. **SEGMENTING:** Silence-based audio segmentation
  5. **FINGERPRINTING:** Chromaprint analysis per potential passage
  6. **SONG MATCHING:** MBID assignment with confidence scoring
  7. **RECORDING:** Write passages and songs to database
  8. **AMPLITUDE:** Lead-in/lead-out point detection
  9. **FLAVORING:** AcousticBrainz or Essentia musical flavor retrieval
  10. **PASSAGES COMPLETE:** Mark file as 'INGEST COMPLETE'
- Each phase's inputs, outputs, database interactions, and error states documented

**Impact:**
- Implementation guidance (exact operation sequence)
- Progress reporting clarity (13 UI status sections map to phases)
- Debugging support (phase boundaries isolate issues)

### Problem 5: Duplicate Detection Strategy Incomplete

**Current State:**
- SPEC032 mentions hash-based duplicate detection
- Strategy for handling duplicates not fully specified
- Relationship between filename matching and hash matching unclear

**Desired State:**
- **Two-tier duplicate detection:**
  1. **Filename matching:** Check for existing file entry with matching path, filename, created/modified metadata
     - If found with status 'INGEST COMPLETE': Skip file (already complete)
     - If found without 'INGEST COMPLETE': Reuse existing fileId, continue processing
     - If not found: Assign new fileId
  2. **Hash matching:** Compute file content hash, check for matching hashes
     - If matching hash found with 'INGEST COMPLETE' status: Mark file as 'DUPLICATE HASH', link to original
     - Otherwise: Continue processing
- `matching_hashes` field stores list of fileIds with identical content hashes
- Bidirectional linking: If file A has hash matching file B, both files' `matching_hashes` lists updated

**Impact:**
- Efficient handling of duplicate files (avoid reprocessing)
- Support for reorganized libraries (same content, different paths)
- Database integrity (relationships preserved)

### Problem 6: Song Matching Flexibility Not Specified

**Current State:**
- SPEC032 describes confidence-based MBID matching
- Single-song-per-passage assumption implicit
- Zero-song passages mentioned but handling not fully specified

**Desired State:**
- **wkmp-ai assigns zero or one song per passage** (not multiple)
  - High/medium/low/no confidence matching
  - No-confidence passages create passage entries with no associated song
- **Future microservices (wkmp-pe) may assign multiple songs per passage**
  - wkmp-ai implementation must not preclude this
  - Database schema supports many-to-many passage-song relationships
- **Adjacent passage merging:** When song metadata indicates embedded silence, adjacent potential passages merged to single passage

**Impact:**
- Handles complex audio files (songs with embedded silence)
- Supports future multi-song passages (medleys, live recordings)
- Clear architectural boundary (wkmp-ai does automatic best-effort, wkmp-pe allows manual refinement)

### Problem 7: Settings Management Strategy Not Documented

**Current State:**
- SPEC032 mentions parameters (silence_threshold_dB, minimum_passage_duration, etc.)
- Storage mechanism unclear (TOML vs. database settings table)
- Default values scattered across specification

**Desired State:**
- **All import parameters stored in database settings table**
  - `silence_threshold_dB` (default: 35dB RMS)
  - `silence_min_duration_ticks` (default: 300ms in SPEC017 ticks)
  - `minimum_passage_audio_duration_ticks` (default: 100ms in SPEC017 ticks)
  - `lead-in_threshold_dB` (default: 45dB)
  - `lead-out_threshold_dB` (default: 40dB)
  - `ai_processing_thread_count` (default: NULL, auto-initialized to CPU_core_count + 1 on first import)
- **Read from database with fallback to compiled defaults**
- **Auto-initialization for thread count:** If NULL, compute based on CPU core count, store result
- **UI allows parameter editing** (settings page, including thread count tuning)
- **Per-import-session parameter snapshot** (ensures consistency if settings change mid-import)

**Impact:**
- User-configurable thresholds without recompilation
- Database-first configuration (aligns with WKMP architecture)
- Auditability (settings values recorded with each import)
- Performance tuning (users can adjust thread count for import speed vs. system availability balance)

### Problem 8: UI Progress Display Specification Missing

**Current State:**
- SPEC032 mentions SSE for real-time progress updates
- Specific UI sections and their content not enumerated
- Mapping between workflow phases and UI display unclear

**Desired State:**
- **13 real-time UI progress sections** (SSE-driven):
  1. **SCANNING:** "scanning" → "N potential files found"
  2. **PROCESSING:** "Processing X to Y of Z" (X completed, Y started, Z total)
  3. **FILENAME MATCHING:** "N completed filenames found"
  4. **HASHING:** "N hashes computed, M matches found"
  5. **EXTRACTING:** "Metadata successfully extracted from X files, Y failures"
  6. **SEGMENTING:** "X files, Y potential passages, Z finalized passages, W songs identified"
  7. **FINGERPRINTING:** "X potential passages fingerprinted, Y successfully matched song identities"
  8. **SONG MATCHING:** "W high, X medium, Y low, Z no confidence"
  9. **RECORDING:** Scrollable list of "Song Title in path/file" or "unidentified passage in path/file"
  10. **AMPLITUDE:** Scrollable list of "Song Title | Duration | lead-in Nms | lead-out Nms"
  11. **FLAVORING:** "W pre-existing, X by AcousticBrainz, Y by Essentia, Z could not be flavored"
  12. **PASSAGES COMPLETE:** "N finalized passages completed"
  13. **FILES COMPLETE:** "N files completed PROCESSING"
- Each section updates in real-time as processing progresses
- Vertically scrollable sections where appropriate (RECORDING, AMPLITUDE)

**Impact:**
- User visibility into import progress (detailed status)
- Debugging support (section values reveal bottlenecks)
- Professional UX (comprehensive real-time feedback)

### Problem 9: Status Field Values Not Enumerated

**Current State:**
- SPEC032 mentions status fields (files.status, passages.status, songs.status)
- Possible status values not fully enumerated
- Status transition logic unclear

**Desired State:**
- **files.status enumeration:**
  - `'PENDING'` (initial state, assigned fileId but not processed)
  - `'PROCESSING'` (file in processing pipeline)
  - `'INGEST COMPLETE'` (all passages extracted, flavored, and recorded)
  - `'DUPLICATE HASH'` (hash matches existing 'INGEST COMPLETE' file)
  - `'NO AUDIO'` (less than minimum_passage_audio_duration_ticks of non-silence)
- **passages.status enumeration:**
  - `'PENDING'` (passage created, awaiting amplitude analysis)
  - `'INGEST COMPLETE'` (lead-in/lead-out detected, ready for playback)
- **songs.status enumeration:**
  - `'PENDING'` (MBID assigned, awaiting flavor retrieval)
  - `'FLAVOR READY'` (high-level AcousticBrainz or Essentia profile stored)
  - `'FLAVORING FAILED'` (both AcousticBrainz and Essentia failed)

**Impact:**
- Database query clarity (filter by status for specific states)
- Status tracking (detect incomplete imports)
- Error handling (distinguish between pending and failed states)

---

## Comparison of Current vs. Desired Architecture

### Workflow Structure

| Aspect | Current SPEC032 | Refined Workflow |
|--------|----------------|------------------|
| **Top-Level States** | SCANNING → PROCESSING → COMPLETED | 5 steps: API key validation → folder selection → SCANNING → PROCESSING → completion |
| **User Interaction** | Implicit (not specified) | Explicit: API key prompt, folder selection prompt |
| **Per-File Processing** | High-level phases (EXTRACTING, FINGERPRINTING, etc.) | 10 enumerated phases with explicit sequence |
| **Duplicate Handling** | Hash-based (underspecified) | Two-tier: Filename matching first, then hash matching |
| **Error States** | FAILED (generic) | Status-specific: 'DUPLICATE HASH', 'NO AUDIO', 'FLAVORING FAILED' |

### Scope and Features

| Feature | Current SPEC032 | Refined Scope |
|---------|----------------|---------------|
| **Audio Quality Control** | Included (detecting skips, gaps, quality issues) | **Removed** (future wkmp-qa microservice) |
| **Manual Passage Editing** | Included (user-directed fade points, metadata revision) | **Removed** (future wkmp-pe microservice) |
| **External Folder Import** | Implied as standard feature | **Stage Two only** (future enhancement) |
| **Multi-Song Passages** | Possible but unclear | **wkmp-ai assigns 0 or 1 songs only**; wkmp-pe may assign multiple |
| **Settings Storage** | Unclear (TOML vs. database) | **Database settings table** (all parameters) |

### UI Progress Display

| Aspect | Current SPEC032 | Refined UI Specification |
|--------|----------------|--------------------------|
| **Progress Granularity** | High-level (files processed count) | **13 detailed sections** (per-phase statistics) |
| **Real-Time Updates** | SSE mentioned | **SSE-driven** with explicit section enumeration |
| **Status Details** | Generic progress bar | **Detailed statistics:** confidence levels, method breakdown, scrollable lists |
| **User Guidance** | Not specified | Explicit feedback per workflow step |

---

## Requirements Analysis

### Functional Requirements

**REQ-SPEC032-001: Scope Definition**
SPEC032 MUST explicitly define wkmp-ai scope as automatic audio file ingest only, excluding quality control (wkmp-qa) and manual passage editing (wkmp-pe).

**REQ-SPEC032-002: Two-Stage Roadmap**
SPEC032 MUST distinguish between Stage One (root folder/subfolder import) and Stage Two (external folder import with file movement).

**REQ-SPEC032-003: Five-Step Workflow**
SPEC032 MUST document the 5-step top-level workflow: API key validation → folder selection → scanning → processing → completion.

**REQ-SPEC032-004: AcoustID API Key Validation**
wkmp-ai MUST validate the stored AcoustID API key at workflow start:
- If invalid: Prompt user to enter valid key or acknowledge lack
- If valid: Continue silently to next step (DEBUG logging only)
- Choice remembered for import session; re-prompted on next session if still invalid

**REQ-SPEC032-005: Folder Selection**
wkmp-ai MUST prompt user to select folder to scan:
- Default: Root folder (initially presented)
- Stage One: Only root folder or subfolders allowed
- Stage Two: External folders allowed (with file movement post-identification)

**REQ-SPEC032-006: Ten-Phase Per-File Pipeline**
SPEC032 MUST document the 10-phase per-file processing pipeline:
1. FILENAME MATCHING
2. HASHING
3. EXTRACTING
4. SEGMENTING
5. FINGERPRINTING
6. SONG MATCHING
7. RECORDING
8. AMPLITUDE
9. FLAVORING
10. PASSAGES COMPLETE

**REQ-SPEC032-007: Filename Matching Logic**
wkmp-ai MUST check for existing file entries by path, filename, created/modified metadata:
- If found with status 'INGEST COMPLETE': Skip file (no further processing)
- If found without 'INGEST COMPLETE': Reuse fileId, continue processing
- If not found: Assign new fileId, continue processing

**REQ-SPEC032-008: Hash-Based Duplicate Detection**
wkmp-ai MUST compute file content hash and check for duplicates:
- Store hash in files table under fileId
- Search for files with matching hashes
- If match found with 'INGEST COMPLETE' status: Mark current file as 'DUPLICATE HASH', add matching fileId to front of matching_hashes list
- Update matching_hashes bidirectionally (both files reference each other)
- If no 'INGEST COMPLETE' match: Continue processing

**REQ-SPEC032-009: Metadata Extraction Merging**
wkmp-ai MUST merge extracted metadata with existing metadata:
- New metadata overwrites existing keys (new takes precedent)
- Old metadata not removed unless directly replaced
- Supports re-processing files with updated tags

**REQ-SPEC032-010: Silence-Based Segmentation**
wkmp-ai MUST detect passage boundaries using silence analysis:
- Silence threshold: `silence_threshold_dB` (default 35dB RMS) from settings table
- Silence minimum duration: `silence_min_duration_ticks` (default 300ms) from settings table
- Any audio segment continuously below threshold for minimum duration = silence
- Each non-silence segment between silences = potential passage
- Files with <`minimum_passage_audio_duration_ticks` (default 100ms) non-silence marked 'NO AUDIO', no further processing

**REQ-SPEC032-011: Fingerprinting Per Potential Passage**
wkmp-ai MUST fingerprint each potential passage individually using Chromaprint:
- Submit each potential passage fingerprint to AcoustID API
- Retrieve candidate MBIDs per potential passage
- Skip AcoustID if API key invalid and acknowledged

**REQ-SPEC032-012: Song Matching with Confidence**
wkmp-ai MUST assign MBIDs to passages based on combined evidence:
- Combine MusicBrainz metadata lookup (from ID3/filename/folder context) with AcoustID fingerprint results
- Consider adjacent potential passage combinations (for songs with embedded silence)
- Rate match_quality (f32 0.0-1.0) as: High, Medium, Low, or No Confidence
- Assign best overall MBID configuration for all passages in file
- No-confidence passages: Create passage entry with no associated song

**REQ-SPEC032-013: Passage Recording**
wkmp-ai MUST record finalized passages in database:
- Create new passage rows with start/end times (SPEC017 ticks)
- Associate passages with fileId
- Create new song entries for MBIDs not yet in songs table
- Add passageId to existing song entries' passage lists

**REQ-SPEC032-014: Amplitude-Based Lead-In/Lead-Out**
wkmp-ai MUST detect lead-in and lead-out points for each finalized passage:
- **Lead-in:** From start time forward until amplitude exceeds `lead-in_threshold_dB` (default 45dB) OR 25% of passage analyzed
- **Lead-out:** From end time backward until amplitude exceeds `lead-out_threshold_dB` (default 40dB) OR 25% of passage analyzed
- Record lead-in/lead-out points in passage table
- Mark passage status as 'INGEST COMPLETE'
- wkmp-ai leaves fade-in/fade-out points empty (future wkmp-pe responsibility)

**REQ-SPEC032-015: Musical Flavor Retrieval**
wkmp-ai MUST retrieve musical flavor data for passages with songs:
- If passage has no song: Skip flavoring
- If song status = 'FLAVOR READY': Skip flavoring
- Otherwise: Attempt AcousticBrainz high-level profile retrieval using MBID
  - If successful: Store JSON profile in songs.flavor, mark song 'FLAVOR READY'
  - If failed: Attempt Essentia analysis on passage audio
    - If successful: Store Essentia profile in songs.flavor, mark song 'FLAVOR READY'
    - If failed: Mark song 'FLAVORING FAILED'

**REQ-SPEC032-016: File Completion**
wkmp-ai MUST mark file status as 'INGEST COMPLETE' when all passages and associated songs completed (recorded, amplitude analyzed, flavored).

**REQ-SPEC032-017: Session Completion**
wkmp-ai MUST complete import session when all scanned files dispositioned as 'INGEST COMPLETE', 'DUPLICATE HASH', or 'NO AUDIO'.

**REQ-SPEC032-018: Database Settings Table**
wkmp-ai MUST read all import parameters from database settings table:
- `silence_threshold_dB` (f32, default: 35.0)
- `silence_min_duration_ticks` (i64, default: 300ms converted to ticks per SPEC017)
- `minimum_passage_audio_duration_ticks` (i64, default: 100ms converted to ticks per SPEC017)
- `lead-in_threshold_dB` (f32, default: 45.0)
- `lead-out_threshold_dB` (f32, default: 40.0)
- `acoustid_api_key` (string)
- `ai_processing_thread_count` (i64, default: NULL, auto-initialized on first import)

**REQ-SPEC032-019: Processing Thread Count Auto-Initialization**
wkmp-ai MUST determine optimal parallel processing thread count:
- Read `ai_processing_thread_count` from database settings table
- If NULL (undefined): Auto-initialize based on CPU core count
  - Algorithm: `ai_processing_thread_count = CPU_core_count + 1`
  - Write computed value to database settings table for future use
- If defined: Use stored value (allows user tuning)
- Advanced users may manually adjust value to balance import speed vs. system availability

**REQ-SPEC032-020: Thirteen UI Progress Sections**
wkmp-ai MUST display 13 real-time SSE-driven UI progress sections as enumerated in Problem 8.

**REQ-SPEC032-021: Status Field Enumerations**
wkmp-ai MUST use status field values as enumerated in Problem 9 for files, passages, and songs tables.

### Non-Functional Requirements

**REQ-SPEC032-NF-001: Parallel Processing**
wkmp-ai MUST process multiple files in parallel using thread count from `ai_processing_thread_count` setting (per REQ-SPEC032-019).

**REQ-SPEC032-NF-002: Real-Time Progress Updates**
wkmp-ai MUST update all 13 UI sections in real-time via SSE (update frequency: per file completion minimum, per phase update recommended).

**REQ-SPEC032-NF-003: Sample-Accurate Timing**
All passage timing points (start, end, lead-in, lead-out) MUST be sample-accurate and converted to SPEC017 ticks for storage.

**REQ-SPEC032-NF-004: Symlink/Junction Handling**
wkmp-ai MUST NOT follow symlinks, junction points, or shortcuts during folder scanning.

**REQ-SPEC032-NF-005: Metadata Preservation**
wkmp-ai MUST preserve all extracted metadata (even if not used for matching) for future analysis.

---

## Expected Changes to SPEC032

### Sections to Add

1. **Scope Definition Section**
   - "In Scope" subsection: Automatic audio ingest features (enumerate from refinement doc)
   - "Out of Scope" subsection: Quality control (→ wkmp-qa), Manual editing (→ wkmp-pe)

2. **Two-Stage Roadmap Section**
   - Stage One: Root folder/subfolder import (initial release)
   - Stage Two: External folder import with file movement (future)

3. **Five-Step Workflow Section**
   - Step 1: AcoustID API key validation
   - Step 2: Folder selection
   - Step 3: Scanning
   - Step 4: Processing
   - Step 5: Completion

4. **Ten-Phase Per-File Pipeline Section**
   - Enumerate all 10 phases with inputs/outputs/database interactions
   - Replace current "batch phase" description

5. **Duplicate Detection Strategy Section**
   - Filename matching logic (two outcomes: reuse fileId or new fileId)
   - Hash matching logic (three outcomes: skip, mark duplicate, or continue)
   - Bidirectional linking via matching_hashes

6. **Settings Management Section**
   - All parameters stored in database settings table
   - Default values enumerated
   - Parameter snapshot per import session

7. **UI Progress Display Specification Section**
   - 13 sections enumerated with exact display text format
   - SSE event structure for each section
   - Scrollable section identification

8. **Status Field Enumeration Section**
   - files.status values and transitions
   - passages.status values and transitions
   - songs.status values and transitions

### Sections to Update

1. **Overview Section**
   - Remove quality control features from purpose statement
   - Remove manual passage editing from purpose statement
   - Add "Stage One" scope qualifier

2. **Component Architecture Section**
   - Update component responsibility matrix to reflect 10-phase pipeline
   - Remove quality control components
   - Remove manual editing components

3. **Import Workflow State Machine Section**
   - Add API key validation and folder selection steps before SCANNING
   - Update PROCESSING state description to reference 10-phase pipeline
   - Add status value enumerations

4. **Database Integration Section**
   - Add settings table read operations
   - Document status field values
   - Document matching_hashes field and bidirectional linking

### Sections to Remove

1. **Quality Control Features**
   - Remove any references to skip/gap/quality issue detection
   - Move to future wkmp-qa specification reference

2. **Manual Passage Editing Features**
   - Remove user-directed fade point definition
   - Remove manual MBID revision features
   - Move to future wkmp-pe specification reference

---

## Expected Changes to wkmp-ai Code

### New Components Required

1. **API Key Validator** (`services/api_key_validator.rs`)
   - Validate AcoustID API key at workflow start
   - Prompt user if invalid
   - Remember choice for session

2. **Folder Selector UI** (`api/ui/folder_selector.rs`)
   - Display folder selection UI with root folder default
   - Validate selected folder (Stage One: must be under root)

3. **Filename Matcher** (`services/filename_matcher.rs`)
   - Query files table for matching path/name/metadata
   - Return existing fileId or indicate new file

4. **Hash-Based Deduplicator** (`services/hash_deduplicator.rs`)
   - Compute file content hash
   - Query files table for matching hashes
   - Update matching_hashes bidirectionally

5. **Settings Manager** (`services/settings_manager.rs`)
   - Read parameters from database settings table
   - Provide defaults if settings missing
   - Auto-initialize thread count (CPU_core_count + 1) when NULL, persist to database
   - Snapshot settings per import session

6. **Progress Tracker** (`models/progress_tracker.rs`)
   - Track statistics for all 13 UI sections
   - Emit SSE events on updates
   - Thread-safe concurrent access

7. **Status Manager** (`db/status_manager.rs`)
   - Enforce status field value enumerations
   - Handle status transitions
   - Validate state machine invariants

### Components to Update

1. **Import Workflow Orchestrator** (`services/workflow_orchestrator/mod.rs`)
   - Add API key validation step before scanning
   - Add folder selection step
   - Update per-file pipeline to 10 phases
   - Integrate progress tracker

2. **Metadata Extractor** (`services/metadata_extractor.rs`)
   - Implement merge logic (new overwrites existing, old preserved)

3. **Silence Detector** (`services/silence_detector.rs`)
   - Read thresholds from settings manager
   - Detect 'NO AUDIO' files (<100ms non-silence)

4. **Song Matching Logic** (currently in `services/confidence_assessor.rs`)
   - Support zero-song passages (no-confidence matches)
   - Support adjacent passage merging (songs with embedded silence)
   - Prevent multi-song assignments (wkmp-ai assigns 0 or 1 songs only)

5. **Amplitude Analyzer** (`services/amplitude_analyzer.rs`)
   - Read lead-in/lead-out thresholds from settings manager
   - Leave fade-in/fade-out points empty (wkmp-pe responsibility)

6. **Database Queries** (`db/files.rs`, `db/passages.rs`, `db/songs.rs`)
   - Add status field queries
   - Add matching_hashes field handling
   - Add settings table queries

7. **SSE Event Emitter** (`api/sse.rs`)
   - Emit 13 distinct event types (one per UI section)
   - Include appropriate statistics per event

8. **UI Progress Page** (`api/ui/import_progress.rs`)
   - Display 13 progress sections
   - Subscribe to 13 SSE event types
   - Implement scrollable sections (RECORDING, AMPLITUDE)

### Components to Remove

1. **Quality Control Features**
   - Remove any skip/gap/quality detection code
   - Remove related UI components

2. **Manual Editing Features**
   - Remove user-directed fade point UI
   - Remove manual MBID revision UI
   - Remove segment boundary adjustment UI (keep for future wkmp-pe reference)

### Database Schema Changes Required

**files table:**
- Confirm `matching_hashes` field exists (JSON array of fileIds)
- Confirm `status` field exists with proper enumeration

**passages table:**
- Confirm `status` field exists
- Confirm `fade_in` and `fade_out` fields exist (nullable, wkmp-ai leaves NULL)

**songs table:**
- Confirm `status` field exists
- Confirm `flavor` field exists (JSON)

**settings table:**
- Confirm exists with (key TEXT PRIMARY KEY, value TEXT) structure
- Add required settings if missing (with defaults)

---

## Acceptance Criteria

**AC-001: SPEC032 Updated**
- SPEC032 document includes all 8 new sections enumerated above
- All 4 sections updated as specified
- All out-of-scope features removed or moved to future microservice references
- Document passes markdown linting
- All internal references valid

**AC-002: wkmp-ai Implements Five-Step Workflow**
- Step 1 (API key validation): Valid keys proceed silently, invalid keys prompt user
- Step 2 (folder selection): UI displays folder selector, validates Stage One constraint
- Step 3 (scanning): File discovery works as specified
- Step 4 (processing): 10-phase pipeline executes per file
- Step 5 (completion): Session completes when all files dispositioned

**AC-003: Ten-Phase Pipeline Implemented**
- All 10 phases execute in documented sequence
- Filename matching correctly handles 3 cases (completed/reuse/new)
- Hash matching correctly detects duplicates and updates matching_hashes bidirectionally
- Metadata extraction merges correctly
- Silence detection handles 'NO AUDIO' files
- Song matching assigns 0 or 1 songs per passage
- Recording creates passages and songs correctly
- Amplitude analysis leaves fade-in/fade-out NULL
- Flavoring tries AcousticBrainz then Essentia, marks status correctly
- File completion marks status 'INGEST COMPLETE'

**AC-004: Duplicate Detection Works**
- Filename matching: Previously completed file skipped
- Hash matching: Duplicate file marked 'DUPLICATE HASH', original referenced
- matching_hashes: Bidirectional links created
- Test with: Same file, renamed file (same content), reorganized file (same content, different path)

**AC-005: Settings Management Works**
- All 6 parameters readable from database settings table (including ai_processing_thread_count)
- Defaults applied when settings missing
- Thread count auto-initializes to CPU_core_count + 1 when NULL
- Auto-initialized thread count persists to database
- UI settings page allows editing (including thread count)
- Settings changes persist across restarts

**AC-006: Thirteen UI Sections Display Correctly**
- All 13 sections visible on import progress page
- SSE updates each section in real-time
- RECORDING section scrollable, displays song titles or "unidentified passage"
- AMPLITUDE section scrollable, displays durations and lead-in/lead-out values
- Statistics accurate (manually verified against sample import)

**AC-007: Status Fields Work Correctly**
- files.status transitions: PENDING → PROCESSING → (INGEST COMPLETE | DUPLICATE HASH | NO AUDIO)
- passages.status transitions: PENDING → INGEST COMPLETE
- songs.status transitions: PENDING → (FLAVOR READY | FLAVORING FAILED)
- Database queries filter by status correctly

**AC-008: Zero-Song Passages Supported**
- No-confidence matches create passage entries with no song association
- Passage displays as "unidentified passage" in UI
- Passage playable (if user adds to queue manually)

**AC-009: Stage One Folder Constraint Enforced**
- Folder selector allows root folder and subfolders
- Folder selector rejects external folders (outside root)
- Error message displayed for external folders

**AC-010: API Key Validation Works**
- Invalid key: User prompted to enter valid key or acknowledge lack
- Valid key: Workflow proceeds silently (DEBUG log confirms validation)
- AcoustID calls skipped if invalid key acknowledged
- Next import session re-validates key

---

## Test Strategy

### Unit Tests

**Filename Matcher:**
- Input: Path/name/metadata matching existing 'INGEST COMPLETE' file → Output: Skip signal
- Input: Path/name/metadata matching existing 'PENDING' file → Output: Existing fileId
- Input: Path/name/metadata not matching any file → Output: New fileId

**Hash Deduplicator:**
- Input: Hash matching existing 'INGEST COMPLETE' file → Output: 'DUPLICATE HASH' status, matching_hashes updated
- Input: Hash matching existing 'PENDING' file → Output: Continue processing, matching_hashes updated
- Input: Hash not matching any file → Output: Continue processing

**Metadata Extractor Merge:**
- Input: New metadata {"artist": "New"}, Existing {"artist": "Old", "album": "Keep"} → Output: {"artist": "New", "album": "Keep"}

**Silence Detector:**
- Input: File with <100ms non-silence → Output: 'NO AUDIO' status
- Input: File with 2 songs separated by 3s silence → Output: 2 potential passages

**Song Matcher:**
- Input: High-confidence MBID match → Output: Passage with song assigned, confidence "High"
- Input: No-confidence match → Output: Passage with no song, confidence "No confidence"

**Amplitude Analyzer:**
- Input: Passage with 2s lead-in before amplitude exceeds 45dB → Output: lead-in = 2000ms
- Input: Passage with fade-in (should be NULL) → Output: fade-in = NULL (wkmp-ai never sets this)

**Settings Manager:**
- Input: Database settings table has `silence_threshold_dB = 40.0` → Output: 40.0
- Input: Database settings table missing `silence_threshold_dB` → Output: 35.0 (default)

**Thread Count Auto-Initialization:**
- Input: Database settings table has `ai_processing_thread_count = NULL`, CPU has 8 cores → Output: 9 (8+1), value written to database
- Input: Database settings table has `ai_processing_thread_count = 6` → Output: 6 (use stored value)
- Input: User edits `ai_processing_thread_count` to 4 via UI → Output: Next import uses 4 threads

### Integration Tests

**End-to-End Import:**
- Input: Folder with 10 audio files (various formats) → Output: All files processed, passages created, songs identified
- Verify: Filename matching skips re-imported files
- Verify: Hash matching detects duplicates
- Verify: UI displays 13 progress sections with correct statistics

**API Key Validation:**
- Input: Invalid API key → Output: User prompted, AcoustID calls skipped
- Input: Valid API key → Output: Silent proceed, AcoustID calls succeed

**Folder Selection:**
- Input: Root folder selected → Output: Import proceeds
- Input: Subfolder selected → Output: Import proceeds
- Input: External folder selected (Stage One) → Output: Error message, import blocked

**Duplicate Scenarios:**
- Scenario 1: Same file imported twice → First import processes, second import skips (filename match)
- Scenario 2: File renamed and re-imported → First import processes, second import marks 'DUPLICATE HASH' (hash match)
- Scenario 3: matching_hashes bidirectional → File A and File B with same hash both reference each other

### System Tests

**Large Library Import:**
- Input: 1000+ audio files
- Verify: All files dispositioned correctly
- Verify: UI progress sections update smoothly (no lag)
- Verify: Database integrity (no orphaned passages, all songs have valid MBIDs or NULL)

**Error Recovery:**
- Scenario: AcousticBrainz API down → Fallback to Essentia works
- Scenario: Essentia fails → Song marked 'FLAVORING FAILED', import continues
- Scenario: Malformed audio file → File marked with error status, import continues

---

## Risk Assessment

### Risk 1: SPEC032 Update Scope Creep

**Failure Mode:** SPEC032 update expands beyond refinement scope, adding new features not in refinement doc
**Probability:** Medium
**Impact:** Medium (delays /plan execution, adds unvalidated requirements)
**Mitigation:** Strict adherence to refinement doc content, no feature additions without explicit user approval
**Residual Risk:** Low

### Risk 2: Code Refactoring Breaks Existing Functionality

**Failure Mode:** Updating workflow orchestrator to 10-phase pipeline introduces regressions in working features
**Probability:** Medium
**Impact:** High (existing import functionality broken)
**Mitigation:** Comprehensive unit and integration tests before and after refactoring, staged rollout per phase
**Residual Risk:** Low-Medium

### Risk 3: UI Progress Display Performance Degradation

**Failure Mode:** 13 SSE sections with real-time updates cause UI lag or excessive server load
**Probability:** Low
**Impact:** Medium (poor UX, but functionality intact)
**Mitigation:** Throttle SSE updates (e.g., max 10 updates/second), test with large imports (1000+ files)
**Residual Risk:** Low

### Risk 4: Settings Migration for Existing Databases

**Failure Mode:** Existing wkmp.db files lack settings table or required settings entries
**Probability:** High (existing installations)
**Impact:** Medium (import fails due to missing settings)
**Mitigation:** Settings manager provides defaults, database schema sync creates missing table/settings
**Residual Risk:** Low

### Risk 5: Duplicate Detection Edge Cases

**Failure Mode:** Complex rename/reorganization scenarios create orphaned or incorrectly linked duplicates
**Probability:** Low
**Impact:** Medium (user confusion, database clutter)
**Mitigation:** Comprehensive duplicate scenario testing, bidirectional link validation in unit tests
**Residual Risk:** Low

### Risk 6: Stage One Folder Constraint Bypass

**Failure Mode:** User finds way to import from external folders in Stage One (e.g., symlink)
**Probability:** Low
**Impact:** Low (Stage Two feature exposed early, but functional)
**Mitigation:** Symlink detection (REQ-SPEC032-NF-004), path validation before scanning
**Residual Risk:** Low

**Overall Residual Risk:** Low-Medium (primarily Risk 2: code refactoring regressions)

**Recommendation:** Proceed with implementation. Mitigate Risk 2 via staged rollout (implement phases incrementally, test after each phase).

---

## Implementation Notes

### Development Sequence Recommendation

1. **SPEC032 Update First** (4-6 hours)
   - Ensures specification-driven development
   - Provides implementation blueprint
   - Allows architectural review before coding

2. **Settings Management** (2-3 hours)
   - Foundational for all phases (thresholds used throughout)
   - Low risk, high value
   - Enables parameter experimentation

3. **Status Field Enumerations** (1-2 hours)
   - Database schema updates
   - Status manager implementation
   - Required for all phases

4. **Duplicate Detection** (3-4 hours)
   - Filename matcher
   - Hash deduplicator
   - matching_hashes bidirectional linking
   - High complexity, early delivery prevents rework

5. **Ten-Phase Pipeline Refactoring** (5-7 hours)
   - Update workflow orchestrator
   - Integrate filename matcher, hash deduplicator
   - Stage incrementally (test after each phase)

6. **UI Progress Display** (3-5 hours)
   - Progress tracker model
   - SSE event emitters
   - UI page updates
   - Depends on pipeline refactoring

7. **API Key Validation & Folder Selection** (2-3 hours)
   - Low complexity, user-facing
   - Final polish before testing

8. **Integration & System Testing** (2-4 hours)
   - End-to-end scenarios
   - Large library testing
   - Error recovery testing

**Total Estimated Effort:** 22-34 hours (Medium-Large)

### Code Review Focus Areas

1. **Status Field Transitions:** Ensure state machine invariants enforced (no invalid transitions)
2. **Bidirectional Linking:** Verify matching_hashes updated symmetrically
3. **Metadata Merge Logic:** Confirm new overwrites existing, old preserved
4. **Zero-Song Passages:** Verify passages without songs display/function correctly
5. **SSE Event Throttling:** Check for performance issues with high-frequency updates

### Documentation Requirements

- Update SPEC032 as specified (before implementation)
- Update IMPL008-audio_ingest_api.md (if API endpoints change)
- Update user-facing documentation (workflow steps, settings page)
- Add inline code comments referencing SPEC032 requirement IDs

---

## Glossary

**Stage One:** Initial wkmp-ai release supporting root folder and subfolder import only
**Stage Two:** Future wkmp-ai enhancement supporting external folder import with file movement to root
**wkmp-qa:** Future microservice for audio quality control (detecting skips, gaps, quality issues)
**wkmp-pe:** Future microservice for manual passage editing (user-directed fade points, metadata revision)
**Potential Passage:** Segment detected by silence analysis, may be merged before finalization
**Finalized Passage:** Passage with MBID assigned (or no-confidence match), ready for recording
**Zero-Song Passage:** Passage with no associated song (no-confidence match)
**matching_hashes:** JSON array field in files table storing fileIds with identical content hashes
**Sample-Accurate:** Timing precision at audio sample level (~0.02ms at 48kHz)
**SPEC017 Ticks:** Time unit used throughout WKMP (see SPEC017-sample_rate_conversion.md)
**SSE:** Server-Sent Events (HTTP streaming for real-time updates)

---

## References

- [wip/wkmp-ai_refinement.md](wkmp-ai_refinement.md) - Source refinement document
- [docs/SPEC032-audio_ingest_architecture.md](../docs/SPEC032-audio_ingest_architecture.md) - Current specification
- [docs/SPEC002-crossfade.md](../docs/SPEC002-crossfade.md) - Passage timing definitions
- [docs/SPEC017-sample_rate_conversion.md](../docs/SPEC017-sample_rate_conversion.md) - Tick time units
- [docs/IMPL001-database_schema.md](../docs/IMPL001-database_schema.md) - Database schema
- [docs/GOV001-document_hierarchy.md](../docs/GOV001-document_hierarchy.md) - Documentation governance

---

**END OF SPECIFICATION**

**Status:** Ready for /plan workflow
**Next Step:** Run `/plan wip/SPEC032_wkmp-ai_refinement_specification.md` to generate detailed implementation plan with test specifications and increment breakdown.
