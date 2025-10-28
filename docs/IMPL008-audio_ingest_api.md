# WKMP Audio Ingest API

**🔌 TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines HTTP API for wkmp-ai (Audio Ingest microservice). Derived from [SPEC024](SPEC024-audio_ingest_architecture.md). See [Document Hierarchy](GOV001-document_hierarchy.md).

> **Related:** [Architecture](SPEC024-audio_ingest_architecture.md) | [Amplitude Analysis](SPEC025-amplitude_analysis.md) | [Library Management](SPEC008-library_management.md)

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

**Document Version:** 1.0
**Last Updated:** 2025-10-27
