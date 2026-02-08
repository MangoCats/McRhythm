# WKMP Audio Ingest API

**🔌 TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines HTTP API for wkmp-ai (Audio Ingest microservice). Derived from [SPEC032](SPEC032-audio_ingest_architecture.md). See [Document Hierarchy](GOV001-document_hierarchy.md).

> **Related:** [Architecture](SPEC032-audio_ingest_architecture.md) | [Amplitude Analysis](SPEC025-amplitude_analysis.md) | [Library Management](SPEC008-library_management.md)

---

## Overview

**Module:** wkmp-ai
**Port:** 5723
**Protocol:** HTTP/1.1, Server-Sent Events (SSE)
**Format:** JSON request/response bodies

---

## Import Workflow Endpoints

### POST /import/start

**Description:** Begin import session

**Request:**
```json
{
  "root_folder": "/home/user/Music",
  "parameters": {
    "scan_subdirectories": true,
    "file_extensions": [".mp3", ".flac", ".ogg", ".m4a", ".wav"],
    "skip_hidden_files": true,
    "parallelism": 4
  }
}
```

**Response (202 Accepted):**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "state": "SCANNING",
  "started_at": "2025-10-27T12:00:00Z"
}
```

**Errors:**
- 400: Invalid root_folder (doesn't exist, not readable)
- 409: Import session already running
- 500: Internal server error

---

### GET /import/status/{session_id}

**Description:** Poll import progress

**Response (200 OK):**
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
      "file_path": "corrupt.mp3",
      "error_code": "DECODE_ERROR",
      "error_message": "Failed to decode audio"
    }
  ],
  "started_at": "2025-10-27T12:00:00Z",
  "elapsed_seconds": 270,
  "estimated_remaining_seconds": 810
}
```

**Errors:**
- 404: Session not found

---

### POST /import/cancel/{session_id}

**Description:** Cancel running import

**Response (200 OK):**
```json
{
  "session_id": "uuid",
  "state": "CANCELLED",
  "files_processed": 150,
  "files_skipped": 850,
  "cancelled_at": "2025-10-27T12:05:00Z"
}
```

---

## File Classification Endpoint

### GET /api/import/file-classification

**Description:** Get file classification report after SCANNING phase completes

**Query Parameters:**
- `category` (optional): Filter by category (`audio`, `image`, `other`). If omitted, returns all categories.
- `offset` (optional): Pagination offset (default: 0)
- `limit` (optional): Maximum files per category (default: 500, max: 1000)

**Response (200 OK) - All Categories:**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "scanned_folder": "/home/user/Music",
  "scan_completed_at": "2025-11-16T12:05:23Z",
  "audio_files": {
    "count": 1247,
    "total_size_bytes": 8812634112,
    "files": [
      {
        "path": "/home/user/Music/Album1/track01.flac",
        "size_bytes": 44040192,
        "modified_at": "2024-01-15T14:23:00Z"
      },
      {
        "path": "/home/user/Music/Album1/track02.flac",
        "size_bytes": 39845120,
        "modified_at": "2024-01-15T14:25:00Z"
      }
      // ... (up to 500 files per category, use pagination for more)
    ]
  },
  "image_files": {
    "count": 342,
    "total_size_bytes": 45678912,
    "files": [
      {
        "path": "/home/user/Music/Album1/cover.jpg",
        "size_bytes": 145678,
        "modified_at": "2024-01-10T10:00:00Z"
      }
      // ... (up to 500 files)
    ]
  },
  "other_files": {
    "count": 15,
    "total_size_bytes": 1234567,
    "files": [
      {
        "path": "/home/user/Music/playlist.m3u",
        "size_bytes": 2048,
        "modified_at": "2024-02-01T08:30:00Z"
      }
      // ... (up to 500 files)
    ]
  }
}
```

**Response (200 OK) - Single Category with Pagination:**

Request: `GET /api/import/file-classification?category=audio&offset=500&limit=500`

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "scanned_folder": "/home/user/Music",
  "scan_completed_at": "2025-11-16T12:05:23Z",
  "category": "audio",
  "pagination": {
    "offset": 500,
    "limit": 500,
    "total_count": 1247,
    "has_more": true
  },
  "files": [
    {
      "path": "/home/user/Music/Album42/song_501.mp3",
      "size_bytes": 5242880,
      "modified_at": "2023-08-12T16:45:00Z"
    }
    // ... (next 500 files, or fewer if near end)
  ]
}
```

**Errors:**
- `404 Not Found` - Session not found or SCANNING phase not yet completed
- `409 Conflict` - Import session still in SCANNING phase (report not ready)
- `400 Bad Request` - Invalid category parameter (must be `audio`, `image`, or `other`)

**Notes:**
- File classification data available after SCANNING phase completes
- Classification persists in session state until session ends (COMPLETED or CANCELLED)
- Files sorted alphabetically by path (case-insensitive)
- `size_bytes` is actual file size on disk (not compressed size)
- `modified_at` is ISO 8601 timestamp from file metadata (last modification time)

---

## Amplitude Analysis Endpoints

### POST /analyze/amplitude

**Description:** Analyze single file amplitude

**Request:**
```json
{
  "file_path": "/home/user/Music/track.flac",
  "start_time": 0.0,
  "end_time": 180.5,
  "parameters": {
    "rms_window_ms": 100,
    "lead_in_threshold_db": -12.0,
    "lead_out_threshold_db": -12.0
  }
}
```

**Response (200 OK):**
```json
{
  "file_path": "/home/user/Music/track.flac",
  "peak_rms": 0.95,
  "lead_in_duration": 2.35,
  "lead_out_duration": 3.12,
  "quick_ramp_up": false,
  "quick_ramp_down": false,
  "rms_profile": [0.02, 0.15, 0.45, 0.82, 0.95, 0.93, ...],
  "analyzed_at": "2025-10-27T12:34:56Z"
}
```

---

## Parameter Endpoints

### GET /parameters/global

**Description:** Get global import parameters

**Response (200 OK):**
```json
{
  "rms_window_ms": 100,
  "lead_in_threshold_db": -12.0,
  "lead_out_threshold_db": -12.0,
  "quick_ramp_threshold": 0.75,
  "quick_ramp_duration_s": 1.0,
  "max_lead_in_duration_s": 5.0,
  "max_lead_out_duration_s": 5.0,
  "apply_a_weighting": true
}
```

### POST /parameters/global

**Description:** Update global parameters

**Request:**
```json
{
  "lead_in_threshold_db": -10.0,
  "max_lead_in_duration_s": 6.0
}
```

**Response (200 OK):**
```json
{
  "status": "updated",
  "parameters": { /* full updated parameters */ }
}
```

---

## SSE Endpoint

### GET /events

**Description:** Subscribe to import events (Server-Sent Events)

**Query Parameters:**
- `session_id` (optional): Filter events for specific session

**Event Stream:**
```
event: state_changed
data: {"session_id": "uuid", "old_state": "SCANNING", "new_state": "EXTRACTING", ...}

event: progress
data: {"session_id": "uuid", "current": 250, "total": 1000, ...}

event: error
data: {"session_id": "uuid", "file_path": "corrupt.mp3", ...}

event: completed
data: {"session_id": "uuid", "files_processed": 982, ...}
```

---

**Document Version:** 1.1
**Last Updated:** 2025-11-16
**Changes:**
- **v1.1 (2025-11-16):**
  - Added File Classification Endpoint (`GET /api/import/file-classification`)
  - Added support for pagination via `category`, `offset`, `limit` query parameters
  - Added response schemas for all-categories and single-category views
  - Added error codes for classification endpoint (404, 409, 400)
- **v1.0 (2025-10-27):**
  - Initial API specification for wkmp-ai import workflow
  - Import workflow endpoints, amplitude analysis, parameter management, SSE events
