# Test Specifications for PLAN028

## Test Index

| Test ID | Type | Requirement | Description |
|---------|------|-------------|-------------|
| TC-U-001-01 | Unit | REQ-PERF-001 | ProgressManager stores updates in memory |
| TC-U-001-02 | Unit | REQ-PERF-001 | ProgressManager marks dirty on update |
| TC-U-001-03 | Unit | REQ-PERF-001 | Sync task runs every 10 seconds |
| TC-U-001-04 | Unit | REQ-PERF-001 | Sync persists to database when dirty |
| TC-U-002-01 | Unit | REQ-PERF-002 | SSE broadcast without database write |
| TC-U-002-02 | Unit | REQ-PERF-002 | SSE sends immediately on update |
| TC-U-003-01 | Unit | REQ-PERF-003 | WriteQueue enqueues operations |
| TC-U-003-02 | Unit | REQ-PERF-003 | WriteQueue processes serially |
| TC-U-003-03 | Unit | REQ-PERF-003 | WriteQueue handles backpressure |
| TC-I-001-01 | Integration | REQ-PERF-001,002 | Progress updates flow end-to-end |
| TC-I-002-01 | Integration | REQ-PERF-003,007 | Batch writes executed in order |
| TC-I-003-01 | Integration | REQ-PERF-004 | Verify O(1) database writes |

## Detailed Test Specifications

### TC-U-001-01: ProgressManager stores updates in memory
- **Given:** New ProgressManager instance
- **When:** update_progress(100, 1000, "Processing") called
- **Then:** Internal state reflects: processed=100, total=1000, operation="Processing"
- **Verify:** No database calls made

### TC-U-001-02: ProgressManager marks dirty on update
- **Given:** ProgressManager with clean state
- **When:** Any update method called
- **Then:** dirty flag = true
- **Verify:** Flag resets to false after sync

### TC-U-001-03: Sync task runs every 10 seconds
- **Given:** ProgressManager with sync task spawned
- **When:** Wait 11 seconds
- **Then:** Sync function called at least once
- **Verify:** Timer interval is 10±0.5 seconds

### TC-U-001-04: Sync persists when dirty
- **Given:** ProgressManager with dirty=true
- **When:** sync_to_database() called
- **Then:** Database updated with current state
- **Verify:** dirty flag = false after successful sync

### TC-U-002-01: SSE broadcast without database
- **Given:** ProgressManager with EventBus
- **When:** update_progress() called
- **Then:** SSE event emitted immediately
- **Verify:** No database operations in call stack

### TC-U-002-02: SSE sends immediately
- **Given:** ProgressManager receiving rapid updates
- **When:** 10 updates in 100ms
- **Then:** 10 SSE events sent
- **Verify:** <10ms latency per event

### TC-U-003-01: WriteQueue enqueues operations
- **Given:** WriteQueue instance
- **When:** enqueue(SaveSession), enqueue(RecordPassages)
- **Then:** Queue contains 2 operations in order
- **Verify:** Operations retrievable in FIFO order

### TC-U-003-02: WriteQueue processes serially
- **Given:** WriteQueue with 5 operations
- **When:** Processing starts
- **Then:** Operations execute one at a time
- **Verify:** No concurrent database connections

### TC-U-003-03: WriteQueue handles backpressure
- **Given:** WriteQueue with 1000 item limit
- **When:** Attempt to enqueue 1001st item
- **Then:** Enqueue blocks or returns error
- **Verify:** Queue size never exceeds 1000

### TC-I-001-01: End-to-end progress flow
- **Given:** Import session with 100 files
- **When:** Process all files
- **Then:** SSE events sent for each file, database updated periodically
- **Verify:** Final database state matches progress

### TC-I-002-01: Batch writes in order
- **Given:** 10 passages to record
- **When:** Submitted to WriteQueue
- **Then:** Single transaction with all 10 passages
- **Verify:** Transaction commits successfully

### TC-I-003-01: O(1) database writes
- **Given:** Import of 100 files
- **When:** Complete import
- **Then:** <10 database write operations total
- **Verify:** Previously would be 100+ writes