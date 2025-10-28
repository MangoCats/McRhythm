# Workflow State Machine Tests - wkmp-ai

**Requirements:** AIA-WF-010, AIA-WF-020, AIA-ASYNC-010, AIA-ASYNC-020
**Priority:** P0 (Critical), P1 (High)
**Test Count:** 12

---

## TEST-009: Start Import Session Creates Valid Session ID

**Requirement:** AIA-ASYNC-010
**Type:** Integration
**Priority:** P0

**Given:**
- wkmp-ai server running
- Empty database
- Test root folder with 5 MP3 files

**When:**
- POST /import/start
  ```json
  {
    "root_folder": "/tmp/test_library",
    "parameters": {
      "rms_window_ms": 100,
      "lead_in_threshold_db": -12.0,
      "import_parallelism": 4
    }
  }
  ```

**Then:**
- Response status: 200 OK
- Response body contains:
  ```json
  {
    "session_id": "<valid UUID>",
    "status": "started",
    "message": "Import session started"
  }
  ```

**Acceptance Criteria:**
- ✅ session_id is valid UUID v4
- ✅ Background task spawned (check task count)
- ✅ Session initial state is SCANNING

---

## TEST-010: State Machine Transitions Through All States

**Requirement:** AIA-WF-010
**Type:** End-to-End
**Priority:** P0

**Given:**
- Import session started with 1 MP3 file (simple library)
- File has valid audio, known MusicBrainz recording

**When:**
- Poll GET /import/status/{session_id} every 500ms

**Then:**
- State transitions observed in sequence:
  1. SCANNING (file discovery)
  2. EXTRACTING (metadata extraction)
  3. FINGERPRINTING (Chromaprint → AcoustID → MusicBrainz)
  4. SEGMENTING (silence detection)
  5. ANALYZING (amplitude analysis)
  6. FLAVORING (AcousticBrainz)
  7. COMPLETED

**Acceptance Criteria:**
- ✅ All 7 states reached
- ✅ States in correct order (no skips)
- ✅ No invalid state transitions
- ✅ Total duration < 30 seconds for 1 file

---

## TEST-011: Import Session State Persisted In-Memory Only

**Requirement:** AIA-WF-020
**Type:** Unit
**Priority:** P1

**Given:**
- Import session running (EXTRACTING state)

**When:**
- Query session state via GET /import/status/{session_id}
- Check database for session state table/rows

**Then:**
- Session state returned from in-memory structure
- No `import_sessions` table exists in database
- No session state rows in any database table

**Acceptance Criteria:**
- ✅ State accessible via API (in-memory)
- ✅ No database session state storage
- ✅ Session lost after server restart (expected)

---

## TEST-012: Multiple Files Processed Concurrently

**Requirement:** AIA-ASYNC-020
**Type:** Performance
**Priority:** P1

**Given:**
- Import session with 8 MP3 files
- Parameters: `import_parallelism: 4`

**When:**
- Monitor import progress and log timestamps

**Then:**
- Observe 4 files processing simultaneously (checked via concurrent operation logs)
- Processing time ~2x faster than sequential (8 files in ~2x time of 4 files)

**Acceptance Criteria:**
- ✅ Max 4 concurrent file operations at any time
- ✅ Total time < sequential time × 0.6 (40% speedup)
- ✅ All 8 files complete successfully

---

## TEST-013: Import Session Cancelled Successfully

**Requirement:** AIA-WF-010
**Type:** Integration
**Priority:** P0

**Given:**
- Import session in progress (FINGERPRINTING state, 50% complete)

**When:**
- POST /import/cancel/{session_id}

**Then:**
- Response status: 200 OK
- Session state changes to CANCELLED
- Background task terminates within 2 seconds
- No new database writes after cancellation

**Acceptance Criteria:**
- ✅ Cancellation request succeeds
- ✅ State = CANCELLED in status endpoint
- ✅ Background task cleaned up
- ✅ No partial/corrupted database rows

---

## TEST-014: Import Session Error Transitions to FAILED

**Requirement:** AIA-WF-010
**Type:** Integration
**Priority:** P0

**Given:**
- Import session with corrupt audio file (triggers DECODE_ERROR)

**When:**
- Import processes corrupt file

**Then:**
- Session state transitions to FAILED
- Status endpoint shows error details:
  ```json
  {
    "state": "FAILED",
    "error": {
      "code": "DECODE_ERROR",
      "message": "Failed to decode audio",
      "file_path": "/tmp/corrupt.mp3"
    }
  }
  ```

**Acceptance Criteria:**
- ✅ State = FAILED after critical error
- ✅ Error details captured
- ✅ No further file processing

---

## TEST-015: Background Task Spawned Immediately

**Requirement:** AIA-ASYNC-010
**Type:** Unit
**Priority:** P0

**Given:**
- wkmp-ai server running

**When:**
- POST /import/start (with 100 files)
- Measure response time

**Then:**
- Response returns immediately (< 50ms)
- Background task spawned (tokio::spawn)
- HTTP request does NOT wait for import completion

**Acceptance Criteria:**
- ✅ Response time < 50ms
- ✅ Response does not block on import
- ✅ Background task started (check logs)

---

## TEST-016: Concurrent Import Sessions Rejected

**Requirement:** AIA-WF-020 (derived from scope)
**Type:** Integration
**Priority:** P0

**Given:**
- Import session already running (session_1)

**When:**
- POST /import/start (attempt to start session_2)

**Then:**
- Response status: 409 Conflict
- Response body:
  ```json
  {
    "error": "Import already in progress",
    "active_session_id": "<session_1_uuid>"
  }
  ```

**Acceptance Criteria:**
- ✅ Second import rejected
- ✅ First import continues unaffected
- ✅ Clear error message

---

## TEST-017: Session Progress Tracking Accuracy

**Requirement:** AIA-WF-010
**Type:** Integration
**Priority:** P0

**Given:**
- Import session with 10 files

**When:**
- Poll GET /import/status/{session_id} during import

**Then:**
- Progress updates show:
  - current: incrementing from 0 to 10
  - total: always 10
  - percentage: 0% → 10% → 20% → ... → 100%
  - current_operation: describes current file

**Acceptance Criteria:**
- ✅ Progress never exceeds 100%
- ✅ current ≤ total always true
- ✅ Percentage calculation accurate: (current / total) * 100

---

## TEST-018: Empty Root Folder Completes Immediately

**Requirement:** AIA-WF-010
**Type:** Integration
**Priority:** P0

**Given:**
- Empty root folder (no audio files)

**When:**
- POST /import/start with empty folder

**Then:**
- State transitions: SCANNING → COMPLETED
- Status shows:
  - files_processed: 0
  - passages_created: 0
  - duration: < 1 second

**Acceptance Criteria:**
- ✅ No errors or failures
- ✅ State = COMPLETED
- ✅ No database writes (except maybe log entry)

---

## TEST-019: Large Library Import (1000 Files)

**Requirement:** AIA-ASYNC-020
**Type:** Performance
**Priority:** P1

**Given:**
- Root folder with 1000 MP3 files (average 3 minutes each)
- Parameters: `import_parallelism: 4`

**When:**
- POST /import/start

**Then:**
- Import completes within 40 minutes (per PERF-010)
- All 1000 files processed
- No timeouts or crashes

**Acceptance Criteria:**
- ✅ Duration: 20-40 minutes
- ✅ All files processed successfully
- ✅ Memory usage stable (<2GB)

---

## TEST-020: Session State Access After Completion

**Requirement:** AIA-WF-020
**Type:** Integration
**Priority:** P1

**Given:**
- Import session completed (state = COMPLETED)

**When:**
- GET /import/status/{session_id} 5 minutes after completion

**Then:**
- Status still accessible
- Final statistics preserved:
  - files_processed, files_failed, passages_created, duration_seconds

**Acceptance Criteria:**
- ✅ Session state retained for 1 hour post-completion
- ✅ After 1 hour, return 404 Not Found
- ✅ Memory cleanup of old sessions

---

## Test Implementation Notes

**Framework:** `cargo test --test workflow_tests -p wkmp-ai`

**Test Helpers:**
```rust
// Wait for session to reach specific state
async fn wait_for_state(
    client: &reqwest::Client,
    session_id: Uuid,
    target_state: ImportState,
    timeout: Duration,
) -> Result<(), TestError> {
    let start = Instant::now();
    loop {
        if start.elapsed() > timeout {
            return Err(TestError::Timeout);
        }
        let status = get_import_status(client, session_id).await?;
        if status.state == target_state {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

// Create test library with N files
fn create_test_library(path: &Path, num_files: usize) {
    for i in 0..num_files {
        let file_path = path.join(format!("test_{:04}.mp3", i));
        std::fs::copy("fixtures/sample.mp3", file_path).unwrap();
    }
}
```

---

End of workflow tests
