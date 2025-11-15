# SPEC032: WKMP Audio Ingest Architecture

**🎵 TIER 2 - DESIGN SPECIFICATION**

Defines architecture for wkmp-ai (Audio Ingest microservice) to guide users through music library import. Derived from [Requirements](REQ001-requirements.md). See [Document Hierarchy](GOV001-document_hierarchy.md).

> **Related Documentation:** [Requirements](REQ001-requirements.md) | [Library Management](SPEC008-library_management.md) | [Amplitude Analysis](SPEC025-amplitude_analysis.md) | [Audio File Segmentation](IMPL005-audio_file_segmentation.md) | [Sample Rate Conversion](SPEC017-sample_rate_conversion.md) | [Data-Driven Schema Maintenance](SPEC031-data_driven_schema_maintenance.md) | [Database Schema](IMPL001-database_schema.md) | [API Design](IMPL008-audio_ingest_api.md)

---

## Overview

**[AIA-OV-010]** wkmp-ai is the Audio Ingest microservice responsible for automatic audio file import and passage identification for music library construction.

**Module Identity:**
- **Name:** wkmp-ai (Audio Ingest)
- **Port:** 5723
- **Version:** Full only (not in Lite/Minimal)
- **Technology:** Rust, Tokio (async), Axum (HTTP + SSE)

**Purpose:** Automate music library construction through intelligent audio import workflow:
1. File discovery and audio format verification
2. Metadata extraction (ID3 tags, filename, folder context)
3. Hash-based duplicate detection with bidirectional linking
4. Silence-based passage boundary detection
5. Per-passage audio fingerprinting via AcoustID API
6. Song matching with confidence assessment (High/Medium/Low/None)
7. Amplitude-based lead-in/lead-out point detection (define valid overlap regions for crossfades)
8. Musical flavor data retrieval (AcousticBrainz → Essentia fallback)

**In Scope:** Automatic ingest workflow for music library construction

**Out of Scope:**
- Quality control and audio issue detection (future wkmp-qa microservice)
- Manual passage editing and MBID revision (future wkmp-pe microservice)

---

## Scope Definition

**[AIA-SCOPE-010]** wkmp-ai implements **automatic audio file ingest only**:

**In Scope (Automatic Ingest):**
- File discovery, metadata extraction, hash-based deduplication
- Silence-based segmentation and passage boundary detection
- Chromaprint fingerprinting per potential passage
- Song matching via AcoustID + MusicBrainz with confidence scoring
- Amplitude analysis for lead-in/lead-out detection
- Musical flavor retrieval (AcousticBrainz → Essentia fallback)
- Database population with files, passages, songs, and relationships

**Out of Scope (Future Microservices):**
- **Quality Control (wkmp-qa):** Skip/gap/quality issue detection and reporting
- **Passage Editing (wkmp-pe):** User-directed fade point definition, manual MBID revision

**Rationale:** Single-responsibility microservices enable focused development and independent deployment.

---

## Two-Stage Development Roadmap

**[AIA-ROADMAP-010]** wkmp-ai development follows a two-stage roadmap:

**Stage One: Root Folder Import (Current Scope)**
- **Constraint:** Import from root folder or subfolders only
- **Workflow:** Select folder → Scan → Process → Ingest complete
- **Target:** Initial library construction from primary music collection location

**Stage Two: External Folder Import (Future Enhancement)**
- **Feature:** Import from folders outside root folder
- **Workflow:** Select external folder → Identify files → Copy/move to root → Ingest
- **Target:** Incorporate music from external drives, downloads, CDs

**Current Implementation:** Stage One only (root folder constraint enforced)

---

## Architectural Context

### Microservices Integration

**[AIA-MS-010]** wkmp-ai integrates with existing WKMP microservices:

| Module | Port | wkmp-ai Integration | Communication |
|--------|------|---------------------|---------------|
| **wkmp-ui** | 5720 | Launch point for import wizard, progress monitoring | HTTP REST + SSE |
| **wkmp-pd** | 5722 | None (import is pre-playback) | - |
| **wkmp-ap** | 5721 | None (import is pre-playback) | - |
| **wkmp-le** | 5724 | None (lyrics added post-import) | - |

**Primary Integration:** wkmp-ui provides launch point for wkmp-ai import workflow; wkmp-ai owns import UX

### UI Architecture

**[AIA-UI-010]** wkmp-ai provides its own web-based UI for import workflows:
- **Access:** User navigates to http://localhost:5723 in browser
- **Launch:** wkmp-ui provides "Import Music" button that opens wkmp-ai in new tab/window
- **Technology:** HTML/CSS/JavaScript served by wkmp-ai Axum server
- **Pattern:** Similar to wkmp-le (on-demand specialized tool with dedicated UI)
- **Routes:**
  - `/` - Import wizard home page
  - `/import-progress` - Real-time progress display with SSE
  - `/segment-editor` - Waveform editor for passage boundaries ([IMPL005](IMPL005-audio_file_segmentation.md) Step 4)
  - `/api/*` - REST API endpoints for programmatic access

**[AIA-UI-020]** wkmp-ui integration:
- wkmp-ui checks if wkmp-ai is running (via `/health` endpoint)
- If running, "Import Music" button enabled (opens http://localhost:5723)
- If not running, button shows "Install Full Version to enable import"
- No embedded import UI in wkmp-ui (wkmp-ai owns all import UX)

**[AIA-UI-025]** Folder selection UI implementation (Step 2 of workflow):
- **Technology:** Server-side directory tree component (HTML/CSS/JavaScript)
  - No HTML5 File API (not supported for folder selection in browsers)
  - Server-side directory traversal with client-side tree rendering
- **UI Layout:**
  ```
  ┌─────────────────────────────────────────────┐
  │ Select Folder to Import                     │
  ├─────────────────────────────────────────────┤
  │ Root Folder: /home/user/Music              │
  │                                             │
  │ ▼ Music (root folder)                       │
  │   ▼ Albums                                  │
  │     ▶ The Beatles                           │
  │     ▶ Pink Floyd                            │
  │   ▼ Compilations                            │
  │     ▶ Best of 70s                           │
  │   ▶ Singles                                 │
  │                                             │
  │ [Select This Folder] [Cancel]               │
  └─────────────────────────────────────────────┘
  ```
- **Stage One Constraint Enforcement:**
  - Tree only displays root folder and descendants (subfolders any depth)
  - External folders not visible in tree (cannot be selected)
  - Stage Two (future): Add "Browse External Folders" button at top
- **User Interaction:**
  - Click folder name → Highlight folder
  - Click expand icon (▶/▼) → Expand/collapse subfolder tree
  - Click "Select This Folder" → Validate path (Stage One check), proceed to scanning
  - Validation error → Display modal: "Stage Two feature - coming soon"
- **API Endpoints:**
  - `GET /api/folders/tree?root={path}` - Get folder tree (Stage One: root and descendants only)
  - `POST /api/folders/validate` - Validate selected folder (Stage One constraint check)
- **Performance:**
  - Lazy loading: Only load subfolders when parent expanded
  - Cache folder tree in memory (invalidate on refresh button click)

**[AIA-UI-030]** After import completion:
- wkmp-ai displays "Import Complete" with link back to wkmp-ui
- User returns to wkmp-ui (http://localhost:5720) to use library
- wkmp-ui detects new files via database watch or SSE event from wkmp-ai

**See:** [On-Demand Microservices](../CLAUDE.md#on-demand-microservices) for architectural pattern

---

## Five-Step Workflow

**[AIA-WORKFLOW-010]** wkmp-ai import workflow consists of five high-level steps:

**Step 1: AcoustID API Key Validation**
- Validate stored `acoustid_api_key` from database settings table
- If invalid or missing: Prompt user for valid key OR acknowledge lack of key
- User choice: Provide valid key (validate before proceeding) OR skip fingerprinting for session
- Remember choice for current session, re-prompt next session if still invalid
- DEBUG logging for validation process

**Step 2: Folder Selection**
- UI to select folder to scan (default: root folder)
- **Stage One Constraint:** Only root folder or subfolders allowed
- Error message if external folder selected ("Stage Two feature - coming soon")
- Folder validation: exists, readable, no symlink loops

**Step 3: Scanning**
- Batch directory traversal (discover all audio files)
- Parallel magic byte verification (filter to audio files only)
- Symlink/junction detection (do not follow)
- Output: List of valid audio file paths

**Step 4: Processing**
- Per-file pipeline: Each file processed through 10-phase pipeline (see below)
- Parallel processing: Multiple files processed concurrently (thread count from settings)
- Real-time progress: SSE updates per phase with file counts
- Error handling: Log per-file errors, continue processing other files

**Step 5: Completion**
- Session completion determination: All files dispositioned (complete or failed)
- Summary: Files processed, passages created, songs identified, errors encountered
- UI: "Import Complete" with link back to wkmp-ui

**Workflow State Machine:** `IDLE → API_KEY_VALIDATION → FOLDER_SELECTION → SCANNING → PROCESSING → COMPLETED`

### Shared Database

**[AIA-DB-010]** wkmp-ai writes to shared SQLite database (`wkmp.db` in root folder):

**Tables Written:**
- `files` - Audio file metadata (path, hash, duration)
- `passages` - Passage definitions (timing points, fade curves)
- `songs`, `artists`, `works`, `albums` - MusicBrainz entities
- `passage_songs`, `passage_albums` - Relationships
- `acoustid_cache`, `musicbrainz_cache`, `acousticbrainz_cache` - API response caches

**Tables Read:**
- `settings` - Import parameters (global defaults)

**Concurrency:** Single-user import workflow (no concurrent import sessions from different users)

**[AIA-DB-020]** Zero-configuration database initialization per [SPEC031 Data-Driven Schema Maintenance](SPEC031-data_driven_schema_maintenance.md):

**Automatic Startup Behavior:**
1. **Database Creation:** If `wkmp.db` does not exist, create it automatically
2. **Table Creation:** Execute `CREATE TABLE IF NOT EXISTS` for all required tables
3. **Schema Sync:** Automatically detect and repair schema drift
   - Introspect actual database schema via `PRAGMA table_info`
   - Compare to expected schema from code definitions
   - Apply `ALTER TABLE` statements for missing columns
   - Log all schema changes comprehensively
4. **Migration Framework:** Run complex data migrations if schema version < current

**Zero-Configuration Guarantees ([SPEC031:REQ-NF-036, REQ-NF-037](SPEC031-data_driven_schema_maintenance.md#requirements-analysis)):**
- No manual database setup required
- No manual schema migrations for column additions
- Automatic recovery from schema drift
- Development-production parity (same schema maintenance in both)

**Implementation:** Uses `wkmp_common::db::schema_sync` module for automatic schema maintenance. See [SPEC031](SPEC031-data_driven_schema_maintenance.md) for complete specification.

---

## Duplicate Detection Strategy

**[AIA-DUPL-010]** wkmp-ai uses a two-tier duplicate detection strategy:

**Tier 1: Filename Matching (Phase 1)**
- Query `files` table by exact path and last modified time metadata match
- If found with completed passages: Skip file entirely (already processed)
- Rationale: Avoid re-processing already-imported files

**Tier 2: Hash-Based Matching (Phase 2)**
- Compute file content hash (SHA-256)
- Query `files.matching_hashes` JSON field for files with matching hash
- If match found with status: `'INGEST COMPLETE'`:
  - Create bidirectional link: `current.matching_hashes ← match.fileId` AND `match.matching_hashes ← current.fileId`
  - Mark current file status: `'DUPLICATE HASH'`
  - Skip remaining phases (duplicate content detected)
- Rationale: Detect renamed files, reorganized files, duplicate copies

**Bidirectional Linking:**
```sql
-- Example: File A (hash=abc123) matches File B (hash=abc123, already processed)
UPDATE files SET matching_hashes = json_insert(matching_hashes, '$[#]', 'fileB_uuid') WHERE file_id = 'fileA_uuid';
UPDATE files SET matching_hashes = json_insert(matching_hashes, '$[#]', 'fileA_uuid') WHERE file_id = 'fileB_uuid';
```

**Benefits:**
- **Fast skip:** Filename matching avoids expensive hash computation for already-processed files
- **Reorganization tolerance:** Hash matching detects moved/renamed files
- **Bidirectional links:** Users can discover all copies of same audio content
- **No false positives:** Hash collision probability negligible (2^-256 for SHA-256)

---

## Settings Management

**[AIA-SETTINGS-010]** Import parameters stored in database `settings` table:

**Settings Table Schema:**
```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT  -- JSON-encoded value
);
```

**Import Parameters (7 total):**

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `silence_threshold_dB` | REAL | 35.0 | Silence detection threshold (Phase 4) |
| `silence_min_duration_ticks` | INTEGER | 8467200 | Minimum silence duration (300ms in ticks) |
| `minimum_passage_audio_duration_ticks` | INTEGER | 2822400 | Minimum non-silence for valid audio (100ms in ticks) |
| `lead_in_threshold_dB` | REAL | 45.0 | Lead-in detection threshold (Phase 8) |
| `lead_out_threshold_dB` | REAL | 40.0 | Lead-out detection threshold (Phase 8) |
| `acoustid_api_key` | TEXT | NULL | AcoustID API key (Phase 5) |
| `ai_processing_thread_count` | INTEGER | NULL | Parallel processing thread count (auto-initialized) |

**Parameter Loading:**
- Read from `settings` table at workflow start
- If NULL or missing: Use compiled default
- Store default to database for future use

**Thread Count Auto-Initialization (MANDATORY):**
```rust
// At workflow start (REQUIRED behavior):
let thread_count = match read_setting("ai_processing_thread_count") {
    Some(value) => value,  // Use stored value (user tuning supported)
    None => {
        let cpu_count = num_cpus::get();
        let computed = cpu_count + 1;  // Algorithm: CPU_core_count + 1
        write_setting("ai_processing_thread_count", computed);  // REQUIRED: Persist for future use
        computed
    }
};
```

**Auto-Initialization Persistence Requirements:**
- **MUST persist computed value:** When `ai_processing_thread_count` is NULL, compute `num_cpus::get() + 1` and write to database BEFORE starting processing
- **MUST use persisted value:** Subsequent sessions use stored value (no re-computation unless user deletes setting)
- **MUST support user tuning:** Users can override auto-computed value via direct database update
- **Rationale:** Preserve auto-computed value for consistency across sessions, enable user performance tuning

**Benefits:**
- **Auto-initialization:** Sensible defaults without user configuration
- **User tuning:** Users can override auto-computed thread count via database update
- **Persistent:** Settings retained across sessions (no re-computation overhead)
- **Centralized:** All microservices share `settings` table (wkmp-wide configuration)

---

## UI Progress Display Specification

**[AIA-UI-PROGRESS-010]** wkmp-ai provides 13 real-time progress sections via SSE:

**13 Progress Sections:**
1. **SCANNING** - File discovery progress (files discovered)
2. **PROCESSING** - Overall processing progress (files completed / total)
3. **FILENAME MATCHING** - Phase 1 statistics (skipped, reused, new)
4. **HASHING** - Phase 2 statistics (duplicates detected)
5. **EXTRACTING** - Phase 3 statistics (metadata extracted)
6. **SEGMENTING** - Phase 4 statistics (passages detected, NO AUDIO files)
7. **FINGERPRINTING** - Phase 5 statistics (fingerprints generated, API calls)
8. **SONG MATCHING** - Phase 6 statistics (High/Medium/Low/None confidence counts)
9. **RECORDING** - Phase 7 statistics (passages written, scrollable detail)
10. **AMPLITUDE** - Phase 8 statistics (lead-in/lead-out detected, scrollable detail)
11. **FLAVORING** - Phase 9 statistics (flavor retrieved, fallback used, failed)
12. **PASSAGES COMPLETE** - Phase 10 statistics (files finalized)
13. **FILES COMPLETE** - Session completion statistics (total files, passages, songs, errors)

**SSE Event Format:**
```json
{
  "type": "progress_update",
  "section": "FINGERPRINTING",
  "data": {
    "fingerprints_generated": 1234,
    "api_calls_made": 1150,
    "api_calls_cached": 84,
    "files_processed": 1234,
    "total_files": 5736
  },
  "timestamp": "2025-11-12T10:34:56Z"
}
```

**UI Implementation:**
- **Scrollable Sections:** RECORDING, AMPLITUDE (detailed per-passage information)
- **Live Updates:** SSE pushes updates every 2 seconds during processing
- **Per-Phase Statistics:** Counters for each phase outcome (success, skip, error)

**Rationale:**
- **Transparency:** Users see exactly what import is doing
- **Debugging:** Phase-level statistics help diagnose issues
- **Engagement:** Real-time updates maintain user confidence during long imports

---

## Status Field Enumeration

**[AIA-STATUS-010]** Database tables use status fields with defined enumerations:

**files.status:**
- `'PENDING'` - File discovered, not yet processed
- `'PROCESSING'` - File currently in pipeline
- `'INGEST COMPLETE'` - All passages and songs complete (Phase 10)
- `'DUPLICATE HASH'` - Duplicate content detected (Phase 2)
- `'NO AUDIO'` - File has <100ms non-silence (Phase 4)

**passages.status:**
- `'PENDING'` - Passage detected, not yet processed
- `'INGEST COMPLETE'` - Amplitude analysis complete (Phase 8)

**songs.status:**
- `'PENDING'` - Song created, flavor not yet retrieved
- `'FLAVOR READY'` - Musical flavor data successfully retrieved (Phase 9)
- `'FLAVORING FAILED'` - Flavor retrieval failed (both AcousticBrainz and Essentia)

**Status Transitions:**

**files:** `PENDING → PROCESSING → [HASHING] → [NO AUDIO | DUPLICATE HASH | (continue)] → INGEST COMPLETE`

**passages:** `PENDING → INGEST COMPLETE`

**songs:** `PENDING → [FLAVOR READY | FLAVORING FAILED]`

**Database Queries:**
```sql
-- Find incomplete files (for resume functionality)
SELECT * FROM files WHERE status IN ('PENDING', 'PROCESSING');

-- Find files needing retry (flavoring failed)
SELECT DISTINCT f.* FROM files f
  JOIN passages p ON p.file_id = f.file_id
  JOIN passage_songs ps ON ps.passage_id = p.passage_id
  JOIN songs s ON s.song_id = ps.song_id
WHERE s.status = 'FLAVORING FAILED';
```

**Enforcement:**
- Database triggers enforce valid status values (optional)
- Application code validates status before writes
- Status transitions logged for debugging

---

## Lead-In/Lead-Out vs Fade-In/Fade-Out Distinction

**[AIA-TIMING-010]** wkmp-ai Phase 8 (Amplitude Analysis) detects **lead-in and lead-out points only**. These are fundamentally different concepts from fade-in/fade-out:

**Lead-In/Lead-Out Points ([SPEC002:XFD-PT-030, XFD-PT-040](SPEC002-crossfade.md#point-definitions)):**
- **Purpose:** Define valid overlap regions where passages MAY play simultaneously with adjacent passages
- **Definition (Lead-In Point):** Latest time the previous passage may still be playing ([SPEC002:XFD-PT-030](SPEC002-crossfade.md#point-definitions))
- **Definition (Lead-Out Point):** Earliest time the next passage may start playing ([SPEC002:XFD-PT-040](SPEC002-crossfade.md#point-definitions))
- **Nature:** Single absolute tick positions within the passage (not durations, not ranges)
- **Audio Processing:** No volume modification or fading occurs at these points
- **Use Case:** Enable crossfade scheduling (determine when passages can overlap without listener distraction)
- **Detection Method:** Amplitude-based (scan for audio exceeding threshold)
- **wkmp-ai Responsibility:** Detect and record these points automatically (Phase 8)

**Fade-In/Fade-Out Points ([SPEC002:XFD-PT-020, XFD-PT-050](SPEC002-crossfade.md#point-definitions)):**
- **Purpose:** Define volume envelope for passage playback (modify audible volume over time)
- **Definition (Fade-In Point):** When volume reaches 100% ([SPEC002:XFD-PT-020](SPEC002-crossfade.md#point-definitions))
- **Definition (Fade-Out Point):** When volume begins decreasing ([SPEC002:XFD-PT-050](SPEC002-crossfade.md#point-definitions))
- **Nature:** Two absolute tick positions per fade (start and end points for each fade curve)
- **Audio Processing:** Apply fade curves to modify volume (5 curve types per [SPEC002](SPEC002-crossfade.md))
- **Use Case:** Smooth passage starts/ends (e.g., passages extracted from middle of continuous music)
- **Detection Method:** Requires musical judgment (not automatable via amplitude analysis alone)
- **wkmp-ai Responsibility:** None (leave all fade fields NULL, manual definition deferred to wkmp-pe)

**Database Fields (Automatic vs Manual):**
```sql
-- Automatically detected by wkmp-ai Phase 8 (Amplitude Analysis):
lead_in_start_ticks   INTEGER  -- Single point: latest time previous passage may play
lead_out_start_ticks  INTEGER  -- Single point: earliest time next passage may start

-- Left NULL by wkmp-ai (manual definition in wkmp-pe):
fade_in_start_ticks   INTEGER  -- Fade curve start: passage start → fade-in point
fade_in_end_ticks     INTEGER  -- Fade curve end: volume reaches 100%
fade_out_start_ticks  INTEGER  -- Fade curve start: volume begins decreasing
-- (fade_out_end_ticks is always end_time_ticks per SPEC002)
```

**Key Distinction Summary:**
- **Lead-In/Lead-Out:** Define WHEN passages may overlap (scheduling boundaries, no volume change)
- **Fade-In/Fade-Out:** Define HOW passage volume changes over time (volume envelope, independent of overlap)
- **Independence:** Lead and fade points are independent per [SPEC002:XFD-CONS-020](SPEC002-crossfade.md#constraints)
  - Lead-In may be before, after, or equal to Fade-In
  - Lead-Out may be before, after, or equal to Fade-Out
  - All four points may be equal (no overlap, no fades)

**Example Passage (60-second track):**
```
Database Values (after wkmp-ai Phase 8):
  start_time_ticks      = 0 ticks (0.0s)
  lead_in_start_ticks   = 141120000 ticks (5.0s)  ← Detected by amplitude analysis
  fade_in_start_ticks   = NULL                    ← Not detected (requires manual definition)
  fade_in_end_ticks     = NULL                    ← Not detected (requires manual definition)
  fade_out_start_ticks  = NULL                    ← Not detected (requires manual definition)
  lead_out_start_ticks  = 1552320000 ticks (55.0s) ← Detected by amplitude analysis
  end_time_ticks        = 1693440000 ticks (60.0s)

Interpretation:
  - Passage may overlap with previous passage from 0.0s-5.0s (lead-in region)
  - Passage may overlap with next passage from 55.0s-60.0s (lead-out region)
  - No volume fades applied (fade fields NULL → constant volume 1.0 throughout)
  - User may later define fades via wkmp-pe if desired (e.g., fade-in from 0s-2s)
```

**Rationale for Automatic Lead-In/Lead-Out Only:**
- **Amplitude-based detection:** Reliable for determining "when music is present" (threshold-based)
- **Objective criteria:** Lead points have clear audio-level triggers (e.g., >45dB RMS)
- **Fade points require judgment:** Artistic decision about how aggressively to fade volume
  - Some users prefer gentle fades (fade-in over 5 seconds)
  - Others prefer abrupt fades (fade-in over 0.5 seconds)
  - No single "correct" answer determinable from audio amplitude alone
- **Separation of concerns:** wkmp-ai provides automatic analysis, wkmp-pe provides manual refinement

**See Also:**
- [SPEC002 Crossfade Design](SPEC002-crossfade.md) - Complete timing point specification
- [SPEC002:XFD-PT-070](SPEC002-crossfade.md#point-definitions) - Lead-In/Lead-Out definition
- [SPEC002:XFD-PT-080](SPEC002-crossfade.md#point-definitions) - Fade-In/Fade-Out definition (independent of simultaneous playback)
- [SPEC025 Amplitude Analysis](SPEC025-amplitude_analysis.md) - Lead-in/lead-out detection algorithm

---

## Component Architecture

### High-Level Structure

```
wkmp-ai/
├── src/
│   ├── main.rs                       # HTTP server (Axum), port 5723
│   │                                 # Zero-config DB initialization (SPEC031)
│   │                                 # API key validation (Step 1)
│   ├── api/                          # HTTP route handlers
│   │   ├── import_workflow.rs        # /import/* endpoints
│   │   ├── folder_selector.rs        # /select-folder/* endpoints (NEW - Step 2)
│   │   ├── import_progress.rs        # /import-progress UI (13 SSE sections)
│   │   ├── amplitude_analysis.rs     # /analyze/* endpoints
│   │   ├── parameters.rs             # /parameters/* endpoints
│   │   └── metadata.rs               # /metadata/* endpoints
│   ├── services/                     # Business logic
│   │   ├── api_key_validator.rs      # AcoustID API key validation (NEW - Phase 5 prereq)
│   │   ├── file_scanner.rs           # Directory traversal, symlink/junction detection
│   │   ├── filename_matcher.rs       # Filename matching logic (NEW - Phase 1)
│   │   ├── hash_deduplicator.rs      # Hash-based duplicate detection (NEW - Phase 2)
│   │   ├── metadata_extractor.rs     # Tag parsing with merge logic (Phase 3)
│   │   ├── silence_detector.rs       # Silence-based segmentation + NO AUDIO detection (Phase 4)
│   │   ├── fingerprinter.rs          # Per-passage Chromaprint fingerprinting (Phase 5)
│   │   ├── acoustid_client.rs        # AcoustID API client (Phase 5)
│   │   ├── confidence_assessor.rs    # Song matching with confidence (Phase 6)
│   │   ├── musicbrainz_client.rs     # MusicBrainz API client (Phase 6)
│   │   ├── amplitude_analyzer.rs     # Lead-in/lead-out detection (Phase 8)
│   │   ├── acousticbrainz_client.rs  # AcousticBrainz API client (Phase 9)
│   │   ├── essentia_runner.rs        # Essentia subprocess (Phase 9 fallback)
│   │   ├── settings_manager.rs       # Database settings table management (NEW)
│   │   └── workflow_orchestrator/    # 10-phase pipeline coordination
│   │       ├── mod.rs                # Per-file pipeline state machine
│   │       ├── phase_filename_matching.rs
│   │       ├── phase_hashing.rs
│   │       ├── phase_extracting.rs
│   │       ├── phase_segmenting.rs
│   │       ├── phase_fingerprinting.rs
│   │       ├── phase_song_matching.rs
│   │       ├── phase_recording.rs
│   │       ├── phase_amplitude.rs
│   │       ├── phase_flavoring.rs
│   │       └── phase_passages_complete.rs
│   ├── models/                       # Data structures
│   │   ├── import_session.rs         # Import workflow state machine (5 steps + 10 phases)
│   │   ├── progress_tracker.rs       # Progress statistics for 13 UI sections (NEW)
│   │   ├── amplitude_profile.rs      # Amplitude envelope data structure
│   │   ├── parameters.rs             # Parameter definitions (7 settings)
│   │   └── import_result.rs          # Import operation results
│   └── db/                           # Database access
│       ├── files.rs                  # Files table (status, matching_hashes)
│       ├── passages.rs               # Passages table (status)
│       ├── songs.rs                  # Songs table (status)
│       ├── settings.rs               # Settings table (NEW)
│       └── status_manager.rs         # Status field enumeration enforcement (NEW)
```

### Component Responsibilities

**[AIA-COMP-010]** Component responsibility matrix (10-phase pipeline):

| Component | Responsibility | Phase | Input | Output |
|-----------|---------------|-------|-------|--------|
| **api_key_validator** | Validate AcoustID API key, prompt user if invalid | Pre-workflow | API key string | Valid/Invalid + user acknowledgment |
| **file_scanner** | Discover audio files, skip symlinks/junctions | Step 3 | Root folder path | List of valid audio file paths |
| **filename_matcher** | Match files by path/filename/metadata | Phase 1 | File path | Skip/Reuse/New + fileId |
| **hash_deduplicator** | Compute hash, detect duplicates, create bidirectional links | Phase 2 | File content | Hash, duplicate status, matching_hashes links |
| **metadata_extractor** | Parse tags, merge with existing metadata | Phase 3 | File path | Title, artist, album, duration, merged metadata |
| **silence_detector** | Detect passage boundaries, detect NO AUDIO | Phase 4 | Audio PCM, thresholds from settings | Potential passage time ranges (ticks) or NO AUDIO status |
| **fingerprinter** | Generate Chromaprint fingerprints per passage | Phase 5 | Audio PCM per passage | Base64 fingerprint string per passage |
| **acoustid_client** | Query AcoustID API for MBID candidates | Phase 5 | Fingerprint string | List of (MBID, confidence score) |
| **confidence_assessor** | Combine metadata + fingerprint evidence | Phase 6 | Metadata + fingerprint scores | MBID with confidence (High/Medium/Low/None) per passage |
| **musicbrainz_client** | Query MusicBrainz API for recording details | Phase 6 | Recording MBID | Recording, artist, work, album metadata |
| **amplitude_analyzer** | Detect lead-in/lead-out points | Phase 8 | Audio PCM, thresholds from settings | Lead-in/lead-out absolute positions (ticks), fade fields NULL |
| **acousticbrainz_client** | Query AcousticBrainz API for musical flavor | Phase 9 | Recording MBID | Musical flavor vector (JSON) |
| **essentia_runner** | Run Essentia analysis (fallback for Phase 9) | Phase 9 | Audio file path | Musical flavor vector (JSON) |
| **settings_manager** | Read/write database settings table with defaults | All phases | Setting key | Setting value (with auto-initialization) |
| **workflow_orchestrator** | Coordinate 10-phase pipeline per file | Step 4 | File list, settings | Import results per file |

**Note:** The following components have been removed from the refined workflow specification (PLAN024):
- **Pattern Analyzer** - Structural pattern analysis for contextual matching (out of scope for automatic ingest)
- **Contextual Matcher** - MusicBrainz search using metadata + pattern clues (simplified to metadata + fingerprint only)

These features may be reconsidered for future quality-control or manual-editing microservices (wkmp-qa, wkmp-pe).

---

## Import Workflow State Machine

### Workflow States

**[AIA-WF-010]** Import session progresses through 5 high-level workflow steps:

```
                    POST /import/start
                           │
                           ▼
                ┌──────────────────────┐
                │ API_KEY_VALIDATION   │  (Step 1: Validate AcoustID API key, prompt if needed)
                └──────────┬───────────┘
                           │
                           ▼
                ┌──────────────────────┐
                │  FOLDER_SELECTION    │  (Step 2: Select folder, enforce Stage One constraint)
                └──────────┬───────────┘
                           │
                           ▼
                ┌──────────────────────┐
                │      SCANNING        │  (Step 3: Directory traversal, file discovery, symlink skip)
                └──────────┬───────────┘
                           │
                           ▼
                ┌──────────────────────┐
                │     PROCESSING       │  (Step 4: 10-phase per-file pipeline, N parallel workers)
                │                      │  Each file: FILENAME MATCHING → HASHING → EXTRACTING →
                │                      │  SEGMENTING → FINGERPRINTING → SONG MATCHING →
                │                      │  RECORDING → AMPLITUDE → FLAVORING → PASSAGES COMPLETE
                └──────────┬───────────┘
                           │
                           ▼
                ┌──────────────────────┐
                │      COMPLETED       │  (Step 5: Session complete, summary displayed)
                └──────────────────────┘

                   Cancel available at any state → CANCELLED
                   Error in any state → FAILED (with error details)
```

**State Semantics:**
- **API_KEY_VALIDATION:** Validate stored `acoustid_api_key`, prompt user if invalid/missing
- **FOLDER_SELECTION:** UI for folder selection, enforce Stage One constraint (root folder only)
- **SCANNING:** Batch file discovery (parallel magic byte verification, symlink/junction skip)
- **PROCESSING:** Per-file 10-phase pipeline with N parallel workers (thread count from settings)
  - Each worker processes one file through 10 phases sequentially
  - Progress reported as files completed (e.g., "2,581 / 5,736 files")
  - Workers pick next unprocessed file upon completion
  - Phase-level statistics broadcasted via SSE (13 progress sections)
- **COMPLETED:** All files dispositioned (complete or failed), summary displayed

**Legacy Phase States (Deprecated):**
The following fine-grained phase states (EXTRACTING, FINGERPRINTING, SEGMENTING, ANALYZING, FLAVORING) are deprecated in favor of the unified PROCESSING state with 10-phase per-file pipeline. These legacy states may appear in database schema or logs but represent obsolete batch-phase architecture.

### State Persistence

**[AIA-WF-020]** Import session state persisted in-memory (Tokio async tasks):
- Session ID (UUID)
- Current state
- Files processed / total files
- Current operation description
- Errors encountered
- Cancellation flag

**Note:** Session state NOT persisted to database (transient workflow only)

---

## Asynchronous Processing Model

### Background Jobs

**[AIA-ASYNC-010]** Import operations run as Tokio background tasks:

```rust
// Conceptual example (not implementation code)
async fn start_import(root_folder: PathBuf, params: ImportParameters) -> ImportSessionId {
    let session_id = Uuid::new_v4();
    let (tx, rx) = broadcast::channel(100);  // SSE event channel

    tokio::spawn(async move {
        import_workflow(session_id, root_folder, params, tx).await
    });

    session_id
}
```

**Benefits:**
- HTTP request returns immediately (no timeout issues)
- User can poll status or subscribe to SSE
- Graceful cancellation support
- Progress reporting during long operations

### Concurrent Processing

**[AIA-ASYNC-020]** Parallel per-file processing architecture:

**Strategy:** Process multiple files concurrently through 10-phase per-file pipeline (N workers processing different files simultaneously)

**Architecture:**
```
Step 3: SCANNING (Batch File Discovery)
  └─ Directory traversal, parallel magic byte verification
  └─ Skip symlinks/junctions
     Output: List of valid audio file paths

Step 4: PROCESSING (Parallel Worker Pool)
  ├─ Worker 1: File A → 10-Phase Pipeline → Complete
  ├─ Worker 2: File B → 10-Phase Pipeline → Complete
  ├─ Worker 3: File C → 10-Phase Pipeline → Complete
  └─ Worker N: File D → 10-Phase Pipeline → Complete

  Each Worker Processes One File Sequentially Through 10 Phases:
    Phase 1: FILENAME MATCHING  - Check if file exists (reuse fileId)
    Phase 2: HASHING            - Calculate hash, detect duplicates
    Phase 3: EXTRACTING         - Parse metadata tags (ID3/Vorbis/MP4)
    Phase 4: SEGMENTING         - Detect passage boundaries via silence
    Phase 5: FINGERPRINTING     - Generate Chromaprint per passage
    Phase 6: SONG MATCHING      - Combine metadata + fingerprint evidence
    Phase 7: RECORDING          - Write passages to database
    Phase 8: AMPLITUDE          - Detect lead-in/lead-out points
    Phase 9: FLAVORING          - Retrieve musical flavor (AcousticBrainz/Essentia)
    Phase 10: PASSAGES COMPLETE - Mark file as INGEST COMPLETE

  Parallelism Pattern:
    - N workers operate concurrently (N from ai_processing_thread_count setting)
    - Each worker processes one file through all 10 phases sequentially
    - Workers pick next unprocessed file upon completion
    - Files complete in any order (non-deterministic)
```

**Key Characteristics:**
- **Per-file sequential pipeline:** Each file goes through all 10 phases before next file starts in same worker
- **Adaptive parallelism:** N from `ai_processing_thread_count` setting (auto-initialized: `CPU_core_count + 1`)
- **Constant worker utilization:** FuturesUnordered maintains constant parallelism level
- **Balanced resource usage:** CPU-intensive (FINGERPRINTING), I/O-bound (EXTRACTING), and network-bound (SONG MATCHING) operations happen concurrently across workers
- **Fine-grained progress:** Report files completed (e.g., "2,581 / 5,736 files")

**Implementation:**
```rust
// Get thread count from settings (auto-initializes if NULL)
let parallelism_level = db::settings::get_or_init_processing_thread_count(&db).await?;

let mut file_iter = discovered_files.iter().enumerate();
let mut tasks = FuturesUnordered::new();

// Seed initial worker pool
for _ in 0..parallelism_level {
    if let Some((idx, file_path)) = file_iter.next() {
        tasks.push(process_file_through_10_phases(idx, file_path, db.clone()));
    }
}

// Process completions and spawn next file
while let Some((idx, path, result)) = tasks.next().await {
    // ... handle result, update progress ...

    // Maintain parallelism level
    if let Some((idx, file_path)) = file_iter.next() {
        tasks.push(process_file_through_10_phases(idx, file_path, db.clone()));
    }
}
```

**Default Parallelism:** CPU-adaptive via auto-initialization
- Algorithm: `CPU_core_count + 1` (persisted to database on first run)
- Example: 4-core system → 5 concurrent workers
- Example: 8-core system → 9 concurrent workers
- User-configurable via `settings.ai_processing_thread_count`

### Per-File Pipeline Benefits

**[AIA-ASYNC-030]** Rationale for per-file sequential pipeline architecture:

**1. Superior User Experience**
```
Progress Granularity:
  Batch phases:      [=========>         ] 45% (Phase 3 of 6 complete)
  Per-file pipeline: [=========>         ] 45% (2,581 of 5,736 files complete)

Cancellation:
  Batch phases:      Cancel at phase boundaries only (lose partial phase progress)
  Per-file pipeline: Cancel at file boundaries (natural checkpoints, resume trivial)
```

**2. Better Resource Utilization**
```
Batch Phase Processing (Deprecated):
  Phase 3 (Fingerprinting): [CPU: 95%] [I/O: 5%] [Network: 0%]
  Phase 6 (Flavoring):      [CPU: 5%]  [I/O: 5%] [Network: 90%]
  → Resources underutilized during different phases

Per-File Pipeline (Current Architecture):
  Worker 1: Fingerprinting   [CPU: 95%] [I/O: 5%] [Network: 0%]
  Worker 2: API call waiting [CPU: 5%]  [I/O: 5%] [Network: 90%]
  Worker 3: Analyzing        [CPU: 95%] [I/O: 5%] [Network: 0%]
  Worker 4: Extracting       [CPU: 15%] [I/O: 85%] [Network: 0%]
  → Balanced resource utilization across workers
```

**3. Simplified Error Recovery**
```
Batch phases:      If crash during Phase 4 → restart entire Phase 4
Per-file pipeline: If crash after file_2581 → resume from file_2582
                   Database query: SELECT files WHERE NOT EXISTS (passage)
```

**4. Memory Efficiency**
```
Batch Phase 3 (Fingerprinting):
  └─ Hold ALL fingerprints in memory: Vec<(usize, Option<String>)>
     5,736 files * 1KB fingerprint = ~6 MB working set

Per-File Pipeline:
  └─ Process one file at a time per worker
     4 workers * 1KB = 4 KB working set
```

**5. Natural Idempotency**
```
Per-file pipeline enables trivial resume logic:
  1. Query database for files without passages
  2. Process only those files
  3. Each file is atomic unit (either complete or not)
```

**Performance Impact:**
- **Expected speedup:** 1.4-1.6x faster overall (46% improvement)
- **Mechanism:** Overlap CPU-intensive and network-bound operations across workers
- **Trade-off:** Slightly more complex coordination logic vs. batch phases

**Implementation Complexity:**
- **Moderate:** Requires per-file state machine and worker coordination
- **Mitigation:** Use `futures::stream::buffer_unordered()` for work distribution
- **Benefit:** Better UX and performance justify additional complexity

### Per-File Pipeline Implementation Requirements

**[AIA-ASYNC-040]** Mandatory implementation architecture for per-file processing:

**1. Pipeline Function Signature**
```rust
async fn process_file_complete(
    file: &AudioFile,
    db: &SqlitePool,
    fingerprinter: &Fingerprinter,
    rate_limiter: &RateLimiter,
    // ... other services
) -> Result<ProcessedFileResult, ImportError>
```

**2. Pipeline Stages (Sequential per File)**
```
For each file, execute in order (10-phase pipeline):

  Phase 1: FILENAME MATCHING
    └─ Check if file path/name already exists in database
    └─ Output: Skip (already processed), Reuse (update metadata), or New (create fileId)

  Phase 2: HASHING
    └─ Calculate SHA-256 hash of file content
    └─ Check for duplicate hash in database
    └─ If duplicate: Create bidirectional link via matching_hashes JSON field, mark DUPLICATE HASH, stop
    └─ Output: Unique hash (continue) or DUPLICATE HASH (stop)

  Phase 3: EXTRACTING
    └─ Parse metadata tags (Lofty: ID3/Vorbis/MP4)
    └─ Merge with existing metadata using JSON object merge algorithm:
       a. Load existing metadata JSON from database (or empty object {} if NULL)
       b. For each key-value pair in newly extracted metadata:
          - If new value is non-NULL: Set merged[key] = new_value (overwrite existing)
          - If new value is NULL: Preserve existing[key] (no change)
       c. Keys present in existing but absent in new: Preserved unchanged
       d. Result: Union of old and new metadata, with new non-NULL values taking precedence
    └─ Output: Title, artist, album, duration, merged metadata JSON

  Phase 4: SEGMENTING
    └─ Decode audio PCM, detect silence using thresholds from settings table:
       - silence_threshold_dB (default: 35dB RMS)
       - silence_min_duration_ticks (default: 8467200 ticks = 300ms)
    └─ Identify potential passage boundaries (audio segments between silence)
    └─ Calculate total non-silence duration across all potential passages
    └─ NO AUDIO detection (file-level check):
       a. If total non-silence duration < minimum_passage_audio_duration_ticks (default: 2822400 ticks = 100ms):
          - Mark files.status = 'NO AUDIO'
          - STOP processing this file (skip remaining phases)
          - Log: "File has <100ms non-silence, marked NO AUDIO"
       b. Otherwise: Continue to fingerprinting (Phase 5)
    └─ Filter potential passages by minimum duration:
       - Each potential passage MUST be ≥ minimum_passage_audio_duration_ticks
       - Passages shorter than minimum: Discarded (not viable for playback)
    └─ Output: Potential passage time ranges (ticks) OR NO AUDIO status (stop)

  Phase 5: FINGERPRINTING
    └─ Generate Chromaprint fingerprint PER PASSAGE (tokio::task::spawn_blocking for CPU work)
    └─ Query AcoustID API per passage (rate-limited, async)
    └─ Output: List of (MBID, confidence score) per passage

  Phase 6: SONG MATCHING
    └─ Combine metadata + fingerprint evidence per passage
    └─ Assess confidence level for each potential passage:
       - High: Fingerprint match + metadata match (title/artist/duration aligned)
       - Medium: Fingerprint match OR strong metadata match
       - Low: Weak fingerprint or metadata evidence
       - None: No fingerprint match, no metadata match (zero-song passage)
    └─ Apply zero-song passage merging algorithm:
       a. Identify sequences of adjacent passages with None confidence
       b. Merge contiguous None-confidence passages into single passage:
          - New start_time_ticks = first passage start
          - New end_time_ticks = last passage end
          - Discard intermediate silence boundaries
       c. Exception: Preserve boundaries if silence duration >30 seconds
          (likely intentional track separation, not embedded silence)
       d. Rationale: Unidentifiable audio likely one continuous section
          (ambient, spoken word, sound effects, etc.)
    └─ Output: MBID with confidence (High/Medium/Low/None) per finalized passage

  Phase 7: RECORDING
    └─ Write passages to database (atomic transaction)
    └─ Convert all timing points to ticks per SPEC017 (ticks = seconds * 28,224,000)
    └─ Create songs, artists, works, albums, passage_songs relationships
    └─ Output: Persisted passages with passageId

  Phase 8: AMPLITUDE
    └─ Detect lead-in point (single absolute tick position):
       a. Scan forward from start_time_ticks
       b. Find first position where RMS amplitude > lead_in_threshold_dB (default: 45dB)
       c. Maximum scan distance: 25% of passage duration (fallback if threshold never exceeded)
       d. Record absolute tick position as lead_in_start_ticks
    └─ Detect lead-out point (single absolute tick position):
       a. Scan backward from end_time_ticks
       b. Find first position where RMS amplitude > lead_out_threshold_dB (default: 40dB)
       c. Maximum scan distance: 25% of passage duration (fallback if threshold never exceeded)
       d. Record absolute tick position as lead_out_start_ticks
    └─ Leave fade_in_start_ticks, fade_in_end_ticks, fade_out_start_ticks fields NULL (manual definition deferred to wkmp-pe)
    └─ Mark passages.status = 'INGEST COMPLETE'
    └─ Output: Lead-in/lead-out absolute positions persisted (NOT durations, NOT fades)

  Phase 9: FLAVORING
    └─ Check if passage has associated song (passage_songs table)
       └─ If no song: Skip flavoring (zero-song passage), continue to Phase 10
    └─ Check if song.status = 'FLAVOR READY' (pre-existing flavor from previous import)
       └─ If true: Skip flavor retrieval (increment 'pre-existing' counter), continue to Phase 10
    └─ Otherwise: Query AcousticBrainz API for musical flavor (rate-limited, async)
    └─ Fallback to Essentia if AcousticBrainz fails
    └─ Mark songs.status = 'FLAVOR READY' or 'FLAVORING FAILED'
    └─ Output: Musical flavor vector (JSON) or failure status

  Phase 10: PASSAGES COMPLETE
    └─ Mark files.status = 'INGEST COMPLETE'
    └─ Increment completion counter
    └─ Broadcast progress event via SSE
```

**Rationale for Sequence:**
- **Filename matching first** avoids redundant processing of existing files
- **Hashing before extraction** catches duplicate content early (skip expensive operations)
- **Segmentation before fingerprinting** provides passage boundaries for per-passage fingerprints
- **Per-passage fingerprints** more accurate than whole-file fingerprints for multi-track files
- **Evidence combination** (metadata + fingerprints) achieves high-confidence MBID matches
- **Recording before amplitude** ensures passages exist in database for amplitude updates
- **Flavor retrieval last** occurs only after confident identification (avoids wasted API calls)

**3. Parallel Execution**
```rust
use futures::stream::{self, StreamExt};

// Get thread count from settings (auto-initialized: CPU_core_count + 1)
let worker_count = db::settings::get_or_init_processing_thread_count(&db).await?;

let results: Vec<Result<ProcessedFileResult, ImportError>> =
    stream::iter(discovered_files)
        .map(|file_path| process_file_through_10_phases(file_path, db.clone()))
        .buffer_unordered(worker_count)  // N concurrent workers
        .collect()
        .await;
```

**4. Rate Limiter Coordination**
```rust
use governor::{Quota, RateLimiter};

// Shared across all workers
let mb_rate_limiter = Arc::new(RateLimiter::direct(
    Quota::per_second(nonzero!(1_u32))  // MusicBrainz: 1 req/s
));

// Within pipeline:
mb_rate_limiter.until_ready().await;  // Block until rate limit allows
let result = musicbrainz_api_call().await?;
```

**5. Database Transaction Strategy**
```
Option A (Simple): Commit after each file
  └─ Pros: Natural atomicity, trivial resume
  └─ Cons: Higher transaction overhead (5,736 commits)

Option B (Batched - RECOMMENDED): Per-worker batching
  └─ Each worker maintains queue of 10 files
  └─ Commit queue every 10 files or when queue full
  └─ Pros: Reduced transaction overhead (10x fewer commits)
  └─ Cons: Slightly more complex state management
```

**Note:** Schema maintenance handled automatically per [SPEC031](SPEC031-data_driven_schema_maintenance.md). Workers write data only; database creation, table creation, and schema drift repair occur automatically at startup. No manual `CREATE TABLE` or `ALTER TABLE` statements required in import pipeline.

**6. Progress Tracking**
```rust
let completed_count = Arc::new(AtomicUsize::new(0));

// Within each worker after file completion:
completed_count.fetch_add(1, Ordering::Relaxed);

// Monitoring task polls every 2 seconds:
let current = completed_count.load(Ordering::Relaxed);
broadcast_progress_event(current, total_files);
```

**7. Error Handling**
```
Per-file errors:
  ├─ Log error with file path
  ├─ Increment failure counter
  ├─ Continue processing other files
  └─ Include in final summary

Fatal errors:
  ├─ Cancel all workers
  ├─ Transition to FAILED state
  └─ Preserve partial progress in database
```

**8. Cancellation Support**
```rust
use tokio_util::sync::CancellationToken;

let cancel_token = CancellationToken::new();

// Check cancellation between pipeline stages:
if cancel_token.is_cancelled() {
    return Err(ImportError::Cancelled);
}
```

### CPU-Intensive Operation Yielding

**[AIA-ASYNC-050]** Prevention of async runtime starvation during CPU-intensive operations:

**Problem:** CPU-bound operations (SHA-256 hashing, Chromaprint fingerprinting, RMS amplitude analysis) can block Tokio's async runtime for extended periods, starving other tasks and causing worker threads to appear "stalled" despite consuming CPU.

**Root Cause:** Running synchronous CPU-intensive work directly on Tokio's async thread pool prevents the scheduler from executing other tasks, leading to:
- Database lock contention (workers holding connections during CPU work)
- SSE heartbeat delays (event broadcasting blocked)
- UI unresponsiveness (progress updates delayed)
- Apparent "stalling" despite high CPU usage

**Solution: Two-Tier Approach**

**1. Wrap CPU-Intensive Operations in `spawn_blocking`**
- Move synchronous blocking operations off async runtime onto dedicated blocking thread pool
- Applies to: Chromaprint FFI calls, synchronous audio decoding
- **Example:**
```rust
// INCORRECT: Blocks async runtime
let fingerprint = fingerprinter.fingerprint_segment(file_path, start_sec, end_sec)?;

// CORRECT: Uses blocking thread pool
let fingerprint = tokio::task::spawn_blocking(move || {
    fingerprinter.fingerprint_segment(&file_path_clone, start_sec, end_sec)
})
.await??;
```

**2. Periodic Yielding Within Long Operations**
- For operations that must run on async runtime or blocking pool, periodically yield control back to scheduler
- Prevents single operation from monopolizing thread for extended periods
- Controlled by `ai_longwork_yield_interval_ms` setting

**Setting: `ai_longwork_yield_interval_ms`**
- **Type:** Integer (milliseconds)
- **Default:** 990 (just under 1 second)
- **Purpose:** Interval for periodic yielding during CPU-intensive operations
- **Behavior:**
  - Value > 0: Yield every N milliseconds during long operations
  - Value = 0: Disable yielding (faster execution, risk of starvation)
- **Tradeability:** Users can trade responsiveness for raw performance

**Operations Using Yield Timers:**
1. **SHA-256 Hash Calculation (Phase 2):**
   - Yields every 990ms while processing large MP3 files (chunk-by-chunk hashing)
   - Uses `std::thread::yield_now()` in `spawn_blocking` context
2. **Amplitude Analysis (Phase 8):**
   - Yields every 990ms during RMS calculation on PCM buffers
   - Uses `tokio::task::yield_now().await` in async context
3. **Audio Decoding (Phases 4, 5, 8):**
   - Yields every 990ms during Symphonia packet decoding
   - Prevents stalls on very long audio files (multi-hour mixes)

**Implementation Pattern (Async Context):**
```rust
let mut last_yield = Instant::now();
let yield_enabled = yield_interval_ms > 0;

loop {
    // Yield periodically to Tokio scheduler
    if yield_enabled && last_yield.elapsed().as_millis() >= yield_interval_ms as u128 {
        tokio::task::yield_now().await;
        last_yield = Instant::now();
    }

    // ... CPU-intensive work (e.g., decode audio packet) ...
}
```

**Implementation Pattern (Blocking Context):**
```rust
let mut last_yield = Instant::now();
let yield_enabled = yield_interval_ms > 0;

loop {
    // Yield periodically to blocking thread pool
    if yield_enabled && last_yield.elapsed().as_millis() >= yield_interval_ms as u128 {
        std::thread::yield_now();
        last_yield = Instant::now();
    }

    // ... CPU-intensive work (e.g., SHA-256 hashing) ...
}
```

**Performance Impact:**
- **Overhead:** Minimal (~0.1% for 990ms interval on typical operations)
- **Benefit:** Prevents worker stalling, maintains SSE heartbeat, improves UI responsiveness
- **Trade-off:** Slightly slower raw CPU performance vs. much better system-wide responsiveness

**Visibility:** This setting is CRITICAL for proper async runtime behavior. It must be:
- Documented in all CPU-intensive service modules
- Passed to all long-running operations
- Visible in settings UI with clear description
- Tested with both enabled (990ms) and disabled (0ms) configurations

**Related Settings:**
- `ai_processing_thread_count` - Number of parallel workers (affects contention)
- `ai_database_max_lock_wait_ms` - Database lock timeout (related symptom of starvation)

---

## Real-Time Progress Updates

### Server-Sent Events (SSE)

**[AIA-SSE-010]** wkmp-ai provides SSE endpoint for real-time progress:

**Endpoint:** `GET /events?session_id={uuid}`

**Event Types:**
```json
// State change event
{
  "type": "state_changed",
  "session_id": "uuid",
  "old_state": "SCANNING",
  "new_state": "EXTRACTING",
  "timestamp": "2025-10-27T12:34:56Z"
}

// Progress update event
{
  "type": "progress",
  "session_id": "uuid",
  "current": 250,
  "total": 1000,
  "operation": "Fingerprinting: artist_album_track.mp3",
  "timestamp": "2025-10-27T12:34:57Z"
}

// Error event
{
  "type": "error",
  "session_id": "uuid",
  "file_path": "corrupt_file.mp3",
  "error_code": "DECODE_ERROR",
  "error_message": "Failed to decode audio",
  "timestamp": "2025-10-27T12:34:58Z"
}

// Completion event
{
  "type": "completed",
  "session_id": "uuid",
  "files_processed": 982,
  "files_failed": 18,
  "passages_created": 1024,
  "duration_seconds": 320,
  "timestamp": "2025-10-27T12:40:00Z"
}
```

**Reconnection:** Client may disconnect/reconnect, missed events available via `/import/status` polling

### Polling Fallback

**[AIA-POLL-010]** For clients without SSE support:

**Endpoint:** `GET /import/status/{session_id}`

**Response:**
```json
{
  "session_id": "uuid",
  "state": "PROCESSING",
  "progress": {
    "current": 250,
    "total": 1000,
    "percentage": 25.0
  },
  "current_operation": "Processing file 250/1000: track_05.flac (Phase 8: AMPLITUDE)",
  "errors": [
    {
      "file_path": "corrupt_file.mp3",
      "error_code": "DECODE_ERROR",
      "error_message": "Failed to decode audio"
    }
  ],
  "started_at": "2025-10-27T12:30:00Z",
  "elapsed_seconds": 270
}
```

**Polling Interval:** Client should poll every 1-2 seconds during active import

---

## Integration with Existing Workflows

### SPEC008 Integration (Library Management)

**[AIA-INT-010]** wkmp-ai implements workflows defined in SPEC008:
- File discovery (SPEC008:32-88)
- Metadata extraction (SPEC008:90-128)
- Cover art extraction (SPEC008:130-209)
- Chromaprint fingerprinting (SPEC008:210-285)
- MusicBrainz integration (SPEC008:287-431)
- AcousticBrainz integration (SPEC008:435-510)

**New Additions:**
- Amplitude-based lead-in/lead-out detection (SPEC025)
- Parameter management (IMPL010)

### IMPL005 Integration (Audio File Segmentation)

**[AIA-INT-020]** wkmp-ai implements segmentation workflow from IMPL005:
- Source media identification (CD/Vinyl/Cassette)
- Silence detection with user-adjustable thresholds
- MusicBrainz release matching
- User review and manual adjustment UI
- Multi-passage file ingestion

**Enhancement:** Add amplitude analysis step after segmentation

### IMPL001 Integration (Database Schema)

**[AIA-INT-030]** wkmp-ai writes passages with tick-based timing per [SPEC017 Sample Rate Conversion](SPEC017-sample_rate_conversion.md):

**Tick-Based Time Representation:**
- All passage timing points stored as INTEGER ticks (not floating-point seconds)
- **1 tick = 1/28,224,000 second** ≈ 35.4 nanoseconds ([SPEC017:SRC-TICK-030](SPEC017-sample_rate_conversion.md#tick-rate-calculation))
- Tick rate is LCM of all supported sample rates (8kHz-192kHz)
- **Sample-accurate precision:** Any sample boundary from any supported sample rate exactly representable as integer ticks
- **Zero rounding errors:** Integer arithmetic eliminates cumulative floating-point errors

**Conversion Formula:**
```rust
// Convert detected times (floating-point seconds) → INTEGER ticks
let ticks: i64 = (seconds * 28_224_000.0).round() as i64;
```

**Database Fields (all INTEGER, stored as absolute positions relative to file start):**
- `start_time_ticks` - Passage start point (absolute position in file)
- `lead_in_start_ticks` - Lead-in fade start (absolute position, must be >= start_time_ticks)
- `fade_in_start_ticks` - Fade-in start (absolute position)
- `fade_in_end_ticks` - Fade-in end (music at full volume, absolute position)
- `fade_out_start_ticks` - Fade-out start (begin crossfade, absolute position)
- `lead_out_start_ticks` - Lead-out start (absolute position, must be <= end_time_ticks)
- `end_time_ticks` - Passage end point (absolute position in file)

**Storage Convention:** All timing fields are absolute positions relative to file start (tick 0). Database CHECK constraints enforce: `start_time_ticks <= lead_in_start_ticks <= end_time_ticks` (and similar for lead_out_start_ticks). When relative positions (passage-relative) are needed for display or duration calculations, compute from absolute positions.

**Rationale:** Tick-based timing ensures sample-accurate crossfade points across all source sample rates, satisfying [REQ-CF-050] precision requirements. Absolute positioning simplifies crossfade calculations. See [SPEC017](SPEC017-sample_rate_conversion.md) for complete tick system specification and [SPEC025](SPEC025-amplitude_analysis.md) for amplitude timing details.

### Database Integration

**[AIA-INT-040]** wkmp-ai database schema additions for refined workflow:

**Settings Table (New):**
```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT  -- JSON-encoded value
);
```
- Stores 7 import parameters (see Settings Management section)
- Shared across all WKMP microservices
- Auto-initialized with defaults if missing
- Thread count auto-computed and persisted on first run

**files Table Additions:**
```sql
ALTER TABLE files ADD COLUMN status TEXT DEFAULT 'PENDING';
ALTER TABLE files ADD COLUMN matching_hashes TEXT;  -- JSON array of fileIds
```
- `status`: PENDING, PROCESSING, INGEST COMPLETE, DUPLICATE HASH, NO AUDIO
- `matching_hashes`: JSON array for bidirectional duplicate links

**passages Table Additions:**
```sql
ALTER TABLE passages ADD COLUMN status TEXT DEFAULT 'PENDING';
```
- `status`: PENDING, INGEST COMPLETE
- `fade_in`, `fade_out` fields remain NULL (future wkmp-pe responsibility)

**songs Table Additions:**
```sql
ALTER TABLE songs ADD COLUMN status TEXT DEFAULT 'PENDING';
```
- `status`: PENDING, FLAVOR READY, FLAVORING FAILED

**Automatic Schema Maintenance:**
- All schema changes via data-driven schema maintenance (SPEC031)
- No manual migrations required
- Columns added automatically at startup if missing
- Zero-configuration database initialization

---

## Error Handling Strategy

### Error Categories

**[AIA-ERR-010]** Import errors categorized by severity:

| Severity | Behavior | Examples |
|----------|----------|----------|
| **Warning** | Continue processing, log warning | Missing album art, no AcousticBrainz data |
| **Skip File** | Skip current file, continue with others | Corrupt audio file, unsupported format |
| **Critical** | Abort entire import session | Database write error, out of disk space |

### Error Reporting

**[AIA-ERR-020]** Errors reported via:
1. SSE error events (real-time)
2. `/import/status` endpoint (aggregated list)
3. Import completion summary (final report)

**Error Details:**
- File path (if file-specific)
- Error code (e.g., `DECODE_ERROR`, `MBID_LOOKUP_FAILED`)
- Human-readable error message
- Timestamp

---

## Performance Considerations

### Throughput Targets

**[AIA-PERF-010]** Expected import performance with per-file pipeline:

| Library Size | Expected Duration | Assumptions |
|--------------|-------------------|-------------|
| 100 files | 2-4 minutes | Average 3-minute songs, per-file pipeline with N workers (N from ai_processing_thread_count) |
| 1,000 files | 15-30 minutes | Per-file pipeline, overlapping CPU/network operations |
| 5,736 files | 90-120 minutes | Real-world library (1.5-2 hours, ~1.46x faster than batch phases) |
| 10,000 files | 2.5-4 hours | Rate limiting (MusicBrainz, AcoustID), improved from 3-6 hours |

**Performance Comparison:**
```
Batch Phase Architecture (Deprecated):
  5,736 files → 175 minutes (2.9 hours)
  └─ CPU idle during network-heavy phases (Flavoring)

Per-File Pipeline Architecture (Required):
  5,736 files → 120 minutes (2.0 hours)
  └─ CPU and network operations overlap across workers
  └─ 46% faster overall (1.46x speedup)
```

**Bottlenecks:**
- **MusicBrainz rate limit:** 1 req/s (primary bottleneck, mitigated by overlapping with CPU work)
- **AcoustID rate limit:** 3 req/s
- **Amplitude analysis:** CPU-bound (~2-5s per 3-minute passage, parallelized across workers)

**Why Per-File Pipeline is Faster:**
- While Worker 1 waits for MusicBrainz API (1 second), other workers perform CPU-intensive fingerprinting
- Heterogeneous workload balanced across workers (CPU + I/O + network concurrency)
- No idle phases waiting for entire batch to complete

### Optimization Strategies

**[AIA-PERF-020]** Performance optimizations in per-file pipeline:

1. **Caching:** Check `acoustid_cache`, `musicbrainz_cache`, `acousticbrainz_cache` before API queries
2. **Per-Worker Database Batching:** Each worker commits every 10 files (reduces transaction overhead)
3. **Parallel Per-File Processing:** N concurrent workers through full pipeline (CPU/network overlap, N from ai_processing_thread_count setting)
4. **Rate Limiter Coordination:** Shared rate limiter across workers (via `governor` crate)
5. **Natural Resumability:** Query database for files without passages, process only incomplete files

**[AIA-PERF-030]** File scanning parallelization:

**Strategy:** Two-phase scan with sequential directory traversal and parallel file verification

```
Phase 1 (Sequential): Directory traversal + symlink detection
  ├─ WalkDir iterator discovers all file paths
  ├─ Symlink loop detection (mutable HashSet, single-threaded)
  └─ Output: Vec<PathBuf> of candidate files

Phase 2 (Parallel): Magic byte verification
  ├─ Rayon parallel iterator processes candidate files
  ├─ Each thread reads first 12 bytes independently
  ├─ Filter to audio files only (FLAC/MP3/OGG/M4A/WAV signatures)
  └─ Output: Vec<PathBuf> of verified audio files
```

**Performance Impact:**
- SSD systems: 4-6x speedup (40,000-60,000 files/sec)
- HDD systems: 1.5-2.5x speedup (800-1,200 files/sec)
- No shared mutable state during parallel phase (thread-safe)
- Filesystem guarantees concurrent read safety

**Implementation:** Use `rayon::prelude::*` for parallel iterator in Phase 2

**[AIA-PERF-040]** Per-segment fingerprinting within per-file pipeline:

**Strategy:** Per-segment Chromaprint fingerprinting after segmentation, with parallel workers processing different files

```
Per-File Pipeline (within each worker, 10 phases sequentially):
  Phase 1: FILENAME MATCHING     - Check database for existing file (DB query)
  Phase 2: HASHING               - Calculate SHA-256 hash (CPU + I/O bound)
  Phase 3: EXTRACTING            - Parse metadata tags (I/O bound)
  Phase 4: SEGMENTING            - Decode PCM, silence detection (CPU + I/O bound)
  Phase 5: FINGERPRINTING        - For each passage:
     a. Extract passage PCM data (memory operation)
     b. Resample to 44.1kHz if needed (CPU bound)
     c. Generate Chromaprint fingerprint via FFI (CPU bound)
        └─ CHROMAPRINT_LOCK mutex serializes chromaprint_new()/chromaprint_free()
           (required for FFTW backend thread safety, negligible overhead ~1-2ms)
     d. Rate-limited AcoustID API lookup per passage (network I/O bound)
  Phase 6: SONG MATCHING         - Combine metadata + fingerprint scores (CPU bound)
  Phase 7: RECORDING             - Write passages to database (DB transaction)
     └─ Convert all passage timing points (seconds → INTEGER ticks per SPEC017)
  Phase 8: AMPLITUDE             - Detect lead-in/lead-out points (CPU bound)
  Phase 9: FLAVORING             - Rate-limited AcousticBrainz/Essentia lookup (network I/O bound)
  Phase 10: PASSAGES COMPLETE    - Mark file INGEST COMPLETE (DB update)

Parallel Execution (N workers, N from ai_processing_thread_count setting):
  Worker 1: File_001 → [10-Phase Pipeline] → Complete
  Worker 2: File_002 → [10-Phase Pipeline] → Complete
  Worker 3: File_003 → [10-Phase Pipeline] → Complete
  Worker N: File_004 → [10-Phase Pipeline] → Complete
```

**Performance Characteristics:**
- **Per-passage fingerprints** more accurate than whole-file fingerprints for multi-track files
- **Early exit on duplicates** (Phase 2 hashing) avoids expensive operations for duplicate content
- **Segmentation before fingerprinting** provides passage boundaries for per-passage fingerprints
- **CPU-bound operations** (Phase 4-5: SEGMENTING, FINGERPRINTING) overlap with **network-bound operations** (Phase 6, 9: SONG MATCHING, FLAVORING) across workers
- While Worker 1 waits for AcoustID API (Phase 5), Workers 2-N perform CPU-intensive operations
- Better resource utilization than batch phase approach (avoids idle CPU during API-heavy phases)

**Thread Safety:**
- Mutex-protected chromaprint context creation (safe for all FFT backends)
- Per-worker per-segment fingerprinting (no shared mutable state between workers)
- Deterministic results (order-independent processing)

**Implementation:**
- Use `tokio::task::spawn_blocking()` for CPU-intensive per-segment fingerprinting within async pipeline
- Use `once_cell::Lazy<Mutex<()>>` for CHROMAPRINT_LOCK
- Coordinate rate-limited API calls via `governor` crate (shared rate limiter across workers)
- Cache per-segment fingerprints and AcoustID results to avoid reprocessing on retry

**[AIA-PERF-050]** Progress reporting during per-file pipeline processing:

**Real-time progress updates:**
```
During PROCESSING State (Per-File Pipeline):
  ├─ Update progress every 2 seconds (time-based polling)
  ├─ Broadcast SSE progress events for UI
  ├─ Update session database every 2 seconds
  └─ Log progress messages (INFO level)
```

**Progress information provided:**
- **Files completed:** "2,581 / 5,736 files (45.0%)"
- **Processing rate:** files/second across all workers
- **Estimated time remaining:** Based on current completion rate
- **Success/failure counts:** Per-file success and failure tallies
- **Current operations:** List of files being processed by each worker (optional)

**Progress Granularity:**
```
Per-file pipeline progress is naturally fine-grained:
  ├─ Each completed file increments progress counter
  ├─ No waiting for entire phase to complete
  └─ User sees continuous advancement (not phase-based jumps)

Example progression:
  [00:00] Processing: 0 / 5,736 files (0.0%)
  [00:02] Processing: 47 / 5,736 files (0.8%) | Rate: 23.5 files/sec | ETA: 4m 02s
  [00:04] Processing: 95 / 5,736 files (1.7%) | Rate: 23.8 files/sec | ETA: 4m 00s
  ...continuous updates every 2 seconds...
  [03:58] Processing: 5,690 / 5,736 files (99.2%) | Rate: 23.9 files/sec | ETA: 2s
  [04:00] Completed: 5,736 / 5,736 files (100.0%)
```

**UI Integration:**
- SSE progress events trigger UI updates
- Progress bar shows percentage complete (file-based, not phase-based)
- Live file count and rate display
- Optional: Show which files each worker is currently processing

**Performance Impact:**
- Progress updates add negligible overhead (<1% CPU time)
- Database writes every 2 seconds (non-blocking best-effort)
- SSE broadcasts are non-blocking (fire-and-forget)

**Implementation:**
- Use `Arc<AtomicUsize>` for thread-safe progress counters (files completed)
- Spawn async monitoring task (tokio::spawn) that polls counters every 2 seconds
- Each worker increments atomic counter after completing a file
- Natural checkpoint-based progress (file boundaries)

---

## Security Considerations

### Input Validation

**[AIA-SEC-010]** Validate all user inputs:
- **Root folder path:** Must exist, readable, no symlink loops
- **Selected folder (Step 2):** Must be root folder or subfolder (Stage One constraint)
  - Validation algorithm:
    ```rust
    fn validate_stage_one_folder(selected: &Path, root: &Path) -> Result<(), ValidationError> {
        // Canonicalize paths (resolve symlinks, normalize separators)
        let canonical_selected = selected.canonicalize()
            .map_err(|e| ValidationError::PathNotFound { path: selected.to_path_buf(), source: e })?;
        let canonical_root = root.canonicalize()
            .map_err(|e| ValidationError::PathNotFound { path: root.to_path_buf(), source: e })?;

        // Check if selected is root or subfolder of root
        if canonical_selected.starts_with(&canonical_root) {
            Ok(())
        } else {
            Err(ValidationError::ExternalFolder {
                selected: canonical_selected,
                root: canonical_root,
                message: "Stage Two feature - coming soon. Please select root folder or a subfolder within root."
            })
        }
    }
    ```
  - Edge cases handled:
    - Symbolic links: Resolved via `canonicalize()` before comparison
    - Relative paths: Converted to absolute via `canonicalize()`
    - Windows junction points: Treated as symlinks (resolved by `canonicalize()`)
    - Case sensitivity: Platform-native comparison (case-insensitive on Windows)
  - Error message if external folder selected: "Stage Two feature - coming soon"
- **File paths during scanning:** Must be within selected folder (prevent directory traversal)
- **Symlinks/junctions:** Do NOT follow during scanning (prevent loop vulnerabilities)
- **Parameters:** Range validation (e.g., thresholds -100dB to 0dB, thread count ≥1)

### API Key Management

**[AIA-SEC-020]** External API keys stored securely:
- AcoustID API key: Stored in database `settings` table
- Not hardcoded in source code
- Not exposed in API responses or logs
- DEBUG-level logging only for validation process

**[AIA-SEC-030]** AcoustID API key validation (Step 1 of workflow):
- Validate stored key at workflow start before scanning
- If invalid/missing: Prompt user for valid key OR acknowledge lack
- User choice persisted for session (re-prompt next session if still invalid)
  - **Session-scoped state (in-memory only, not persisted to database):**
    ```rust
    struct ImportSession {
        session_id: Uuid,
        acoustid_skip_acknowledged: bool,  // True if user chose to skip fingerprinting
        // ... other session state
    }
    ```
  - **State lifecycle:**
    - Created when import workflow starts (Step 1)
    - `acoustid_skip_acknowledged` initialized to `false`
    - If user acknowledges lack of API key: Set to `true` for this session
    - Cleared when session ends (Step 5 or cancellation)
    - Next session starts fresh: Re-prompt for API key validation
  - **Rationale:** Session-scoped (not persistent) ensures users are reminded to provide API key on each import, maximizing identification accuracy
- If user acknowledges lack: Phase 5 (Fingerprinting) skipped, metadata-only matching used
- Log warning when fingerprinting skipped (reduced identification accuracy)
- See Five-Step Workflow section for complete validation logic

---

## Testing Strategy

### Unit Tests

**[AIA-TEST-010]** Unit test coverage for new/updated components:
- **Filename matching logic:** Skip/Reuse/New outcomes (Phase 1)
- **Hash deduplication:** Bidirectional linking, duplicate detection (Phase 2)
- **Metadata merging:** New overwrites, old preserved (Phase 3)
- **NO AUDIO detection:** <100ms non-silence threshold (Phase 4)
- **Confidence scoring:** High/Medium/Low/None classification (Phase 6)
- **Settings manager:** Read/write with defaults, auto-initialization
- **Status transitions:** Valid state machine transitions
- **Tick conversion calculations:** Sample-accurate precision
- **Thread count auto-initialization:** CPU_core_count + 1 algorithm

### Integration Tests

**[AIA-TEST-020]** Integration tests with updated workflow:
- **API key validation:** Invalid key prompting, user acknowledgment
- **Folder selection:** Stage One constraint enforcement (root folder only)
- **5-step workflow:** API key → Folder → Scanning → Processing → Completion
- **10-phase pipeline:** Per-file sequential processing
- **Hash-based duplicate detection:** Multiple files with same content
- **Status field updates:** Database status transitions for files/passages/songs
- **Settings table operations:** Read/write, defaults, auto-initialization
- Mock MusicBrainz/AcoustID API responses
- Sample audio files (various formats, corrupted files, NO AUDIO files)
- Database operations (in-memory SQLite with SPEC031 auto-maintenance)

### System Tests

**[AIA-TEST-030]** End-to-end system tests:
- **Small library import:** 10 files, verify 5-step workflow completion
- **Duplicate detection:** Import same file twice, verify DUPLICATE HASH status
- **NO AUDIO detection:** Import silent file, verify NO AUDIO status
- **Zero-song passages:** Import unidentifiable audio, verify None confidence
- **Flavor retrieval fallback:** Mock AcousticBrainz failure, verify Essentia fallback
- **13 UI progress sections:** Verify SSE events for all sections
- **Thread count auto-init:** Verify ai_processing_thread_count persisted
- **Timing accuracy:** Validate tick-based timing (sample-accurate)
- **Symlink handling:** Verify symlinks/junctions not followed during scanning

**Test Coverage Target:** 100% of 26 requirements (per PLAN024 traceability matrix)

---

## Future Enhancements

**[AIA-FUTURE-010]** Potential enhancements (not in current scope):

**Stage Two Features (Next Phase):**
1. **External Folder Import**
   - Allow importing from folders outside root folder
   - File movement/copying to root after identification
   - Multi-location library management

**Quality Control Microservice (wkmp-qa):**
2. **Audio Quality Assessment**
   - Skip/gap/quality issue detection
   - Audio quality scoring and reporting
   - Quality-based filtering and review UI

**Passage Editing Microservice (wkmp-pe):**
3. **Manual Passage Editing**
   - User-directed fade-in/fade-out point definition (currently NULL)
   - Manual MBID revision and override
   - Passage boundary adjustment UI
   - Metadata manual correction UI

**General Enhancements:**
4. **Advanced Musical Analysis**
   - Genre classification (ML model)
   - BPM detection (tempo analysis)
   - Musical key detection

**Implemented in PLAN024 (Previously Future):**
- ✅ Incremental Import (filename matching Phase 1: skip already-processed files)
- ✅ Duplicate Detection (hash-based Phase 2: bidirectional linking)
- ✅ Resume After Interruption (query for files without passages, resume processing)

---

**Document Version:** 2.1
**Last Updated:** 2025-11-13
**Status:** Design specification (PLAN024 refinement - implementation in progress)
**Changes:**
- **v2.1 (2025-11-13):**
  - Added comprehensive "Lead-In/Lead-Out vs Fade-In/Fade-Out Distinction" section ([AIA-TIMING-010])
  - Enhanced Phase 8 (AMPLITUDE) with detailed algorithm: 25% scan limit, absolute positions (not durations)
  - Enhanced Phase 9 (FLAVORING) with pre-existing flavor check logic
  - Enhanced Phase 3 (EXTRACTING) with metadata merge algorithm (JSON object merge)
  - Enhanced Phase 6 (SONG MATCHING) with zero-song passage merging algorithm (30-second threshold)
  - Enhanced Phase 4 (SEGMENTING) with NO AUDIO detection logic (file-level check)
  - Added Stage One folder validation algorithm to [AIA-SEC-010] (canonicalize + starts_with)
  - Added session-scoped API key acknowledgment details to [AIA-SEC-030] (in-memory state)
  - Added thread count auto-initialization persistence requirements (MANDATORY)
  - Added folder selection UI implementation details ([AIA-UI-025]) with tree component spec
- **v2.0 (2025-11-12):**
  - Added Scope Definition, Two-Stage Roadmap, Five-Step Workflow sections
  - Added Ten-Phase Per-File Pipeline specification
  - Added Duplicate Detection Strategy, Settings Management, UI Progress Display, Status Field Enumeration sections
  - Updated Component Architecture for 10-phase pipeline
  - Updated Import Workflow State Machine for 5-step workflow
  - Added Database Integration details (settings table, status fields, matching_hashes)
  - Updated Testing Strategy for 26 requirements (100% coverage target)
  - Removed deprecated pattern analyzer, contextual matcher components (out of scope)
  - Clarified out-of-scope features (quality control → wkmp-qa, manual editing → wkmp-pe)

---

End of document - Audio Ingest Architecture
