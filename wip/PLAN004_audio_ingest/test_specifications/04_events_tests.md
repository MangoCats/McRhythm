# Events Tests (SSE and Polling) - wkmp-ai

**Requirements:** AIA-SSE-010, AIA-POLL-010
**Priority:** P0 (Critical), P1 (High)
**Test Count:** 10

---

## TEST-038: SSE Connection Established

**Requirement:** AIA-SSE-010
**Type:** Integration
**Priority:** P0

**Given:**
- wkmp-ai server running
- Import session created (session_id available)

**When:**
- Client connects to `GET /events?session_id={uuid}`

**Then:**
- HTTP 200 OK response
- Content-Type: text/event-stream
- Connection remains open
- Keepalive comments sent every 30 seconds

**Acceptance Criteria:**
- ✅ SSE connection established
- ✅ Client receives "connected" event
- ✅ Connection persists (no timeout)
- ✅ Keepalive prevents idle disconnect

---

## TEST-039: SSE State Changed Event

**Requirement:** AIA-SSE-010
**Type:** Integration
**Priority:** P0

**Given:**
- SSE connection active for session
- Import transitions: SCANNING → EXTRACTING

**When:**
- State change occurs

**Then:**
- Event received:
  ```json
  {
    "type": "state_changed",
    "session_id": "uuid",
    "old_state": "SCANNING",
    "new_state": "EXTRACTING",
    "timestamp": "2025-10-27T12:34:56Z"
  }
  ```

**Acceptance Criteria:**
- ✅ Event type = "state_changed"
- ✅ old_state and new_state correct
- ✅ Timestamp in ISO 8601 format
- ✅ Event received within 100ms of state change

---

## TEST-040: SSE Progress Event

**Requirement:** AIA-SSE-010
**Type:** Integration
**Priority:** P0

**Given:**
- SSE connection active
- Import processing files (50/100 complete)

**When:**
- File processing completes

**Then:**
- Event received:
  ```json
  {
    "type": "progress",
    "session_id": "uuid",
    "current": 50,
    "total": 100,
    "operation": "Fingerprinting: artist_album_track.mp3",
    "timestamp": "2025-10-27T12:34:57Z"
  }
  ```

**Acceptance Criteria:**
- ✅ Event type = "progress"
- ✅ current ≤ total always
- ✅ Percentage = (current / total) × 100
- ✅ operation describes current file

---

## TEST-041: SSE Error Event

**Requirement:** AIA-SSE-010
**Type:** Integration
**Priority:** P0

**Given:**
- SSE connection active
- Import encounters corrupt audio file

**When:**
- Error occurs (decode failure)

**Then:**
- Event received:
  ```json
  {
    "type": "error",
    "session_id": "uuid",
    "file_path": "corrupt_file.mp3",
    "error_code": "DECODE_ERROR",
    "error_message": "Failed to decode audio",
    "timestamp": "2025-10-27T12:34:58Z"
  }
  ```

**Acceptance Criteria:**
- ✅ Event type = "error"
- ✅ error_code from standard list
- ✅ error_message human-readable
- ✅ file_path included when file-specific

---

## TEST-042: SSE Completed Event

**Requirement:** AIA-SSE-010
**Type:** Integration
**Priority:** P0

**Given:**
- SSE connection active
- Import completes successfully

**When:**
- Final state = COMPLETED

**Then:**
- Event received:
  ```json
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
- SSE connection closes after event

**Acceptance Criteria:**
- ✅ Event type = "completed"
- ✅ Statistics accurate (files, passages, duration)
- ✅ Connection closes gracefully
- ✅ No further events sent

---

## TEST-043: SSE Reconnection Handling

**Requirement:** AIA-SSE-010
**Type:** Integration
**Priority:** P0

**Given:**
- SSE connection established, then disconnected (network issue)
- Import still in progress

**When:**
- Client reconnects with last-event-id header

**Then:**
- Missed events replayed (if buffered)
- OR current state sent immediately
- Stream continues from current point

**Acceptance Criteria:**
- ✅ Reconnection succeeds
- ✅ No duplicate events
- ✅ Client receives current state
- ✅ Progress continues smoothly

---

## TEST-044: Polling Status Endpoint

**Requirement:** AIA-POLL-010
**Type:** Integration
**Priority:** P1

**Given:**
- Import session in progress (FINGERPRINTING state)

**When:**
- Client polls `GET /import/status/{session_id}`

**Then:**
- Response:
  ```json
  {
    "session_id": "uuid",
    "state": "FINGERPRINTING",
    "progress": {
      "current": 250,
      "total": 1000,
      "percentage": 25.0
    },
    "current_operation": "Chromaprint: track_05.flac",
    "errors": [],
    "started_at": "2025-10-27T12:30:00Z",
    "elapsed_seconds": 270
  }
  ```

**Acceptance Criteria:**
- ✅ Current state accurate
- ✅ Progress percentage calculation correct
- ✅ Elapsed time updates on each poll
- ✅ Error list includes all errors encountered

---

## TEST-045: Polling Interval Compliance

**Requirement:** AIA-POLL-010
**Type:** Performance
**Priority:** P1

**Given:**
- Client polling every 1 second (1000ms)

**When:**
- Monitor server load during polling

**Then:**
- Server responds within 50ms
- No resource exhaustion
- No rate limiting triggered

**Acceptance Criteria:**
- ✅ Response time <50ms at 1Hz polling
- ✅ Memory stable (<10MB growth per hour)
- ✅ CPU usage minimal (<5%)

---

## TEST-046: Polling Shows Progress Updates

**Requirement:** AIA-POLL-010
**Type:** Integration
**Priority:** P1

**Given:**
- Import at 10% complete
- Client polls every 2 seconds

**When:**
- Import progresses to 50% complete

**Then:**
- Poll 1: percentage = 10%
- Poll 2: percentage = 20% (or higher)
- Poll 3: percentage = 30% (or higher)
- ...
- Final poll: percentage = 50%

**Acceptance Criteria:**
- ✅ Percentage never decreases
- ✅ Percentage updates reflect actual progress
- ✅ current and total values consistent

---

## TEST-047: Polling Aggregated Errors

**Requirement:** AIA-POLL-010
**Type:** Integration
**Priority:** P1

**Given:**
- Import with 3 errors encountered:
  1. DECODE_ERROR on file1.mp3
  2. MBID_LOOKUP_FAILED on file2.flac
  3. DECODE_ERROR on file3.ogg

**When:**
- Client polls `/import/status/{session_id}`

**Then:**
- errors array contains all 3 errors:
  ```json
  {
    "errors": [
      {
        "file_path": "file1.mp3",
        "error_code": "DECODE_ERROR",
        "error_message": "Failed to decode audio"
      },
      {
        "file_path": "file2.flac",
        "error_code": "MBID_LOOKUP_FAILED",
        "error_message": "No MusicBrainz match found"
      },
      {
        "file_path": "file3.ogg",
        "error_code": "DECODE_ERROR",
        "error_message": "Failed to decode audio"
      }
    ]
  }
  ```

**Acceptance Criteria:**
- ✅ All errors included in array
- ✅ Errors ordered chronologically
- ✅ Error details preserved (file, code, message)
- ✅ Duplicate error codes allowed (file1, file3)

---

## Test Implementation Notes

**Framework:** `cargo test --test events_tests -p wkmp-ai`

**SSE Testing Helper:**
```rust
use futures::StreamExt;

#[tokio::test]
async fn test_sse_connection() {
    let client = reqwest::Client::new();
    let mut event_stream = client
        .get("http://localhost:5723/events?session_id=test-uuid")
        .send()
        .await
        .unwrap()
        .bytes_stream();

    // Read events
    while let Some(chunk) = event_stream.next().await {
        let chunk = chunk.unwrap();
        let event: Event = parse_sse_event(&chunk).unwrap();

        match event.event_type.as_str() {
            "state_changed" => { /* assert */ },
            "progress" => { /* assert */ },
            "completed" => break,
            _ => {}
        }
    }
}
```

**Polling Helper:**
```rust
async fn poll_until_complete(
    client: &reqwest::Client,
    session_id: Uuid,
) -> ImportStatus {
    loop {
        let status: ImportStatus = client
            .get(format!("http://localhost:5723/import/status/{}", session_id))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        if status.state == "COMPLETED" || status.state == "FAILED" {
            return status;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
```

---

End of events tests
