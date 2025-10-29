# WKMP Audio Ingest Architecture

**🎵 TIER 2 - DESIGN SPECIFICATION**

Defines architecture for wkmp-ai (Audio Ingest microservice) to guide users through music library import. Derived from [Requirements](REQ001-requirements.md). See [Document Hierarchy](GOV001-document_hierarchy.md).

> **Related Documentation:** [Requirements](REQ001-requirements.md) | [Library Management](SPEC008-library_management.md) | [Amplitude Analysis](SPEC025-amplitude_analysis.md) | [Audio File Segmentation](IMPL005-audio_file_segmentation.md) | [Database Schema](IMPL001-database_schema.md) | [API Design](IMPL008-audio_ingest_api.md)

---

## Overview

**[AIA-OV-010]** wkmp-ai is the Audio Ingest microservice responsible for importing user music collections into the WKMP database with accurate MusicBrainz identification and optimal passage timing.

**Module Identity:**
- **Name:** wkmp-ai (Audio Ingest)
- **Port:** 5723
- **Version:** Full only (not in Lite/Minimal)
- **Technology:** Rust, Tokio (async), Axum (HTTP + SSE)

**Purpose:** Guide new users through:
1. File discovery and metadata extraction
2. Audio fingerprinting and MusicBrainz identification
3. Passage boundary detection (silence-based or manual)
4. Amplitude-based lead-in/lead-out detection
5. AcousticBrainz musical flavor data retrieval

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

**[AIA-UI-030]** After import completion:
- wkmp-ai displays "Import Complete" with link back to wkmp-ui
- User returns to wkmp-ui (http://localhost:5720) to use library
- wkmp-ui detects new files via database watch or SSE event from wkmp-ai

**See:** [On-Demand Microservices](../CLAUDE.md#on-demand-microservices) for architectural pattern

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

---

## Component Architecture

### High-Level Structure

```
wkmp-ai/
├── src/
│   ├── main.rs                    # HTTP server (Axum), port 5723
│   ├── api/                       # HTTP route handlers
│   │   ├── import_workflow.rs     # /import/* endpoints
│   │   ├── amplitude_analysis.rs  # /analyze/* endpoints
│   │   ├── parameters.rs          # /parameters/* endpoints
│   │   └── metadata.rs            # /metadata/* endpoints
│   ├── services/                  # Business logic
│   │   ├── file_scanner.rs        # Directory traversal, file discovery
│   │   ├── metadata_extractor.rs  # Tag parsing (lofty crate)
│   │   ├── fingerprinter.rs       # Chromaprint integration
│   │   ├── musicbrainz_client.rs  # MusicBrainz API client
│   │   ├── acousticbrainz_client.rs # AcousticBrainz API client
│   │   ├── amplitude_analyzer.rs  # RMS analysis, lead-in/lead-out detection (NEW)
│   │   ├── silence_detector.rs    # Silence-based boundary detection
│   │   ├── essentia_runner.rs     # Essentia subprocess integration
│   │   └── parameter_manager.rs   # Parameter loading/saving (NEW)
│   ├── models/                    # Data structures
│   │   ├── import_session.rs      # Import workflow state machine
│   │   ├── amplitude_profile.rs   # Amplitude envelope data structure
│   │   ├── parameters.rs          # Parameter definitions
│   │   └── import_result.rs       # Import operation results
│   └── db/                        # Database access
│       └── queries.rs             # SQL queries for import operations
```

### Component Responsibilities

**[AIA-COMP-010]** Component responsibility matrix:

| Component | Responsibility | Input | Output |
|-----------|---------------|-------|--------|
| **file_scanner** | Discover audio files in directory tree | Root folder path | List of file paths |
| **metadata_extractor** | Parse ID3/Vorbis/MP4 tags | File path | Title, artist, album, duration |
| **fingerprinter** | Generate Chromaprint fingerprints | Audio PCM data | Base64 fingerprint string |
| **musicbrainz_client** | Query MusicBrainz API | Recording MBID | Recording, artist, work, album metadata |
| **acousticbrainz_client** | Query AcousticBrainz API | Recording MBID | Musical flavor vector (JSON) |
| **amplitude_analyzer** | Detect lead-in/lead-out points | Audio PCM data, parameters | Lead-in duration, lead-out duration |
| **silence_detector** | Detect passage boundaries | Audio PCM data, threshold | List of (start, end) time pairs |
| **essentia_runner** | Run Essentia analysis (fallback) | Audio file path | Musical flavor vector (JSON) |
| **parameter_manager** | Load/save import parameters | Parameter name | Parameter value |

---

## Import Workflow State Machine

### Workflow States

**[AIA-WF-010]** Import session progresses through defined states:

```
                    POST /import/start
                           │
                           ▼
                   ┌───────────────┐
                   │   SCANNING    │  (Directory traversal, file discovery)
                   └───────┬───────┘
                           │
                           ▼
                   ┌───────────────┐
                   │   EXTRACTING  │  (Metadata extraction, hash calculation)
                   └───────┬───────┘
                           │
                           ▼
                   ┌───────────────┐
                   │ FINGERPRINTING│  (Chromaprint → AcoustID → MusicBrainz)
                   └───────┬───────┘
                           │
                           ▼
                   ┌───────────────┐
                   │  SEGMENTING   │  (Silence detection, passage boundaries)
                   └───────┬───────┘
                           │
                           ▼
                   ┌───────────────┐
                   │   ANALYZING   │  (Amplitude analysis, lead-in/lead-out)
                   └───────┬───────┘
                           │
                           ▼
                   ┌───────────────┐
                   │  FLAVORING    │  (AcousticBrainz or Essentia)
                   └───────┬───────┘
                           │
                           ▼
                   ┌───────────────┐
                   │   COMPLETED   │
                   └───────────────┘

                   Cancel available at any state → CANCELLED
                   Error in any state → FAILED (with error details)
```

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

**[AIA-ASYNC-020]** File processing parallelization:

**Strategy:** Process multiple files concurrently (configurable parallelism)

```
Single-threaded sequential:  [File1] → [File2] → [File3] → [File4] → ...
                             ↓
Parallel (4 workers):        [File1] [File2] [File3] [File4]
                                ↓      ↓      ↓      ↓
                             [File5] [File6] [File7] [File8]
                                ...
```

**Implementation:** `futures::stream::iter(...).buffer_unordered(MAX_PARALLEL)`

**Default Parallelism:** 4 concurrent file operations
- Balances CPU utilization vs. I/O throughput
- User-configurable via `import_parallelism` parameter

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
  "state": "ANALYZING",
  "progress": {
    "current": 250,
    "total": 1000,
    "percentage": 25.0
  },
  "current_operation": "Amplitude analysis: track_05.flac",
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

**[AIA-INT-030]** wkmp-ai writes passages with tick-based timing:
- Convert detected times (seconds) → ticks (INTEGER)
- Formula: `ticks = seconds * 28_224_000`
- Store in `start_time_ticks`, `lead_in_start_ticks`, etc.

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

**[AIA-PERF-010]** Expected import performance:

| Library Size | Expected Duration | Assumptions |
|--------------|-------------------|-------------|
| 100 files | 2-5 minutes | Average 3-minute songs, MusicBrainz lookup |
| 1,000 files | 20-40 minutes | Parallel processing (4 workers) |
| 10,000 files | 3-6 hours | Rate limiting (MusicBrainz, AcoustID) |

**Bottlenecks:**
- MusicBrainz rate limit: 1 req/s (primary bottleneck)
- AcoustID rate limit: 3 req/s
- Amplitude analysis: CPU-bound (~2-5s per 3-minute passage)

### Optimization Strategies

**[AIA-PERF-020]** Performance optimizations:

1. **Caching:** Check `acoustid_cache`, `musicbrainz_cache`, `acousticbrainz_cache` before API queries
2. **Batch Operations:** Insert passages in batches (100 at a time)
3. **Parallel Processing:** 4 concurrent file operations (configurable)
4. **Resumability:** Track processed files, allow resume after interruption (future enhancement)

---

## Security Considerations

### Input Validation

**[AIA-SEC-010]** Validate all user inputs:
- Root folder path: Must exist, readable, no symlink loops
- File paths: Must be within root folder (prevent directory traversal)
- Parameters: Range validation (e.g., thresholds -100dB to 0dB)

### API Key Management

**[AIA-SEC-020]** External API keys stored securely:
- AcoustID API key: Environment variable or config file
- Not hardcoded in source
- Not exposed in API responses or logs

---

## Testing Strategy

### Unit Tests

**[AIA-TEST-010]** Unit test coverage for:
- Parameter validation logic
- State machine transitions
- Tick conversion calculations
- Error handling paths

### Integration Tests

**[AIA-TEST-020]** Integration tests with:
- Mock MusicBrainz/AcoustID API responses
- Sample audio files (various formats, corrupted files)
- Database operations (in-memory SQLite)

### End-to-End Tests

**[AIA-TEST-030]** E2E tests:
- Import small library (10 files)
- Verify passages created correctly
- Check musical flavor data populated
- Validate timing point accuracy (within 10ms)

---

## Future Enhancements

**[AIA-FUTURE-010]** Potential enhancements (not in current scope):

1. **Resume After Interruption**
   - Persist session state to database
   - Resume from last processed file

2. **Incremental Import**
   - Detect new/modified files only
   - Skip already-imported files

3. **Conflict Resolution**
   - Handle duplicate files (same hash)
   - Merge passages from multiple imports

4. **Advanced Metadata**
   - Genre classification (ML model)
   - BPM detection (tempo analysis)
   - Key detection (musical key)

---

**Document Version:** 1.0
**Last Updated:** 2025-10-27
**Status:** Design specification (implementation pending)

---

End of document - Audio Ingest Architecture
