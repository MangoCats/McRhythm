# SPEC032: WKMP Audio Ingest Architecture

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
                   │   SCANNING    │  (Batch: Directory traversal, file discovery)
                   └───────┬───────┘
                           │
                           ▼
                   ┌───────────────┐
                   │  PROCESSING   │  (Per-file pipeline: Extract → Fingerprint →
                   │               │   Segment → Analyze → Flavor)
                   │               │  (4 parallel workers, files processed to completion)
                   └───────┬───────┘
                           │
                           ▼
                   ┌───────────────┐
                   │   COMPLETED   │
                   └───────────────┘

                   Cancel available at any state → CANCELLED
                   Error in any state → FAILED (with error details)
```

**State Semantics:**
- **SCANNING:** Batch file discovery (parallel magic byte verification)
- **PROCESSING:** Per-file pipeline with 4 concurrent workers
  - Each worker processes one file through entire pipeline: Extract → Fingerprint → Segment → Analyze → Flavor
  - Progress reported as files completed (e.g., "2,581 / 5,736 files")
  - Workers pick next unprocessed file upon completion
- **COMPLETED:** All files successfully processed

**Legacy Phase States (Deprecated):**
The following fine-grained phase states (EXTRACTING, FINGERPRINTING, SEGMENTING, ANALYZING, FLAVORING) are deprecated in favor of the unified PROCESSING state. These legacy states may appear in database schema or logs but represent obsolete batch-phase architecture.

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

**Strategy:** Process each file sequentially through all pipeline stages with multiple concurrent workers

**Architecture:**
```
Phase 1: SCANNING (Batch)
  └─ Discover all audio files (parallel magic byte verification)
     Output: Vec<PathBuf> of discovered files

Phase 2-6: Per-File Pipeline (Parallel Workers)
  ├─ Worker 1: File_001 → [Extract → Fingerprint → Segment → Analyze → Flavor]
  ├─ Worker 2: File_002 → [Extract → Fingerprint → Segment → Analyze → Flavor]
  ├─ Worker 3: File_003 → [Extract → Fingerprint → Segment → Analyze → Flavor]
  └─ Worker 4: File_004 → [Extract → Fingerprint → Segment → Analyze → Flavor]

  When Worker 1 completes File_001:
    └─ Pick next unprocessed file (File_005) and repeat pipeline
```

**Key Characteristics:**
- **Hybrid approach:** Batch scanning (Phase 1) + sequential per-file processing (Phases 2-6)
- **Parallel workers:** 4 concurrent files processing through full pipeline
- **Heterogeneous work:** Different workers may be in different pipeline stages simultaneously
- **Better resource utilization:** CPU-intensive operations (fingerprinting, analysis) happen concurrently with I/O-bound operations (API calls, database writes)

**Implementation:**
```rust
// Conceptual design
futures::stream::iter(discovered_files)
    .map(|file| process_file_through_pipeline(file))  // Full pipeline per file
    .buffer_unordered(4)  // 4 concurrent workers
    .collect()
    .await
```

**Default Parallelism:** 4 concurrent file operations
- Balances CPU, I/O, and network utilization
- User-configurable via `import_parallelism` parameter
- Worker count should not exceed CPU core count for optimal performance

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
Batch Phase Processing (Current):
  Phase 3 (Fingerprinting): [CPU: 95%] [I/O: 5%] [Network: 0%]
  Phase 6 (Flavoring):      [CPU: 5%]  [I/O: 5%] [Network: 90%]
  → Resources underutilized during different phases

Per-File Pipeline (Proposed):
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
For each file, execute in order:
  1. Extract metadata (Lofty, ID3 tags)
  2. Calculate file hash (SHA-256)
  3. Generate Chromaprint fingerprint (tokio::task::spawn_blocking for CPU work)
  4. Lookup AcoustID API (rate-limited, async)
  5. Lookup MusicBrainz API (rate-limited, async)
  6. Detect passage boundaries (silence detection or manual)
  7. Analyze amplitude for each passage (lead-in/lead-out detection)
  8. Fetch AcousticBrainz musical flavor data (rate-limited, async)
  9. Write all data to database (atomic transaction)
  10. Increment completion counter
```

**3. Parallel Execution**
```rust
use futures::stream::{self, StreamExt};

let results: Vec<Result<ProcessedFileResult, ImportError>> =
    stream::iter(discovered_files)
        .map(|file| process_file_complete(file, db, fingerprinter, rate_limiter))
        .buffer_unordered(4)  // 4 concurrent workers
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

**[AIA-PERF-010]** Expected import performance with per-file pipeline:

| Library Size | Expected Duration | Assumptions |
|--------------|-------------------|-------------|
| 100 files | 2-4 minutes | Average 3-minute songs, per-file pipeline with 4 workers |
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
- While Worker 1 waits for MusicBrainz API (1 second), Workers 2-4 perform CPU-intensive fingerprinting
- Heterogeneous workload balanced across workers (CPU + I/O + network concurrency)
- No idle phases waiting for entire batch to complete

### Optimization Strategies

**[AIA-PERF-020]** Performance optimizations in per-file pipeline:

1. **Caching:** Check `acoustid_cache`, `musicbrainz_cache`, `acousticbrainz_cache` before API queries
2. **Per-Worker Database Batching:** Each worker commits every 10 files (reduces transaction overhead)
3. **Parallel Per-File Processing:** 4 concurrent workers through full pipeline (CPU/network overlap)
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

**[AIA-PERF-040]** Fingerprinting within per-file pipeline:

**Strategy:** Chromaprint fingerprinting as part of per-file sequential pipeline with parallel workers

```
Per-File Fingerprinting Pipeline (within each worker):
  1. Decode audio to PCM (I/O + CPU bound)
  2. Resample to 44.1kHz if needed (CPU bound)
  3. Generate Chromaprint fingerprint via FFI (CPU bound)
     └─ CHROMAPRINT_LOCK mutex serializes chromaprint_new()/chromaprint_free()
        (required for FFTW backend thread safety, negligible overhead ~1-2ms)
  4. Rate-limited AcoustID API lookup (network I/O bound)
  5. Rate-limited MusicBrainz API lookup (network I/O bound)
  6. Write results to database

Parallel Execution (4 workers):
  Worker 1: File_001 → [Decode → Resample → Fingerprint → API → DB]
  Worker 2: File_002 → [Decode → Resample → Fingerprint → API → DB]
  Worker 3: File_003 → [Decode → Resample → Fingerprint → API → DB]
  Worker 4: File_004 → [Decode → Resample → Fingerprint → API → DB]
```

**Performance Characteristics:**
- **CPU-bound operations** (decode, resample, fingerprint) overlap with **network-bound operations** (API calls) across workers
- While Worker 1 waits for MusicBrainz API (1 second), Workers 2-4 perform CPU-intensive fingerprinting
- Better resource utilization than batch phase approach (avoids idle CPU during API-heavy phases)

**Thread Safety:**
- Mutex-protected chromaprint context creation (safe for all FFT backends)
- Per-worker fingerprinting (no shared mutable state between workers)
- Deterministic results (order-independent processing)

**Implementation:**
- Use `tokio::task::spawn_blocking()` for CPU-intensive fingerprinting within async pipeline
- Use `once_cell::Lazy<Mutex<()>>` for CHROMAPRINT_LOCK
- Coordinate rate-limited API calls via `governor` crate (shared rate limiter across workers)

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
