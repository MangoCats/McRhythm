# TC-U-ERR-010-01: File Not Found Decode Failure

**Test ID:** TC-U-ERR-010-01
**Test Type:** Unit Test
**Requirement:** REQ-AP-ERR-010 (Decode errors skip passage, continue with next)
**Specification:** SPEC021 ERH-DEC-010 (File Read Failures)
**Priority:** High
**Estimated Effort:** 15 minutes

---

## Test Objective

Verify that when decoder encounters file-not-found error, it:
1. Logs error at ERROR level with correct context
2. Emits PassageDecodeFailed event with correct fields
3. Removes passage from queue
4. Releases decoder chain
5. Does NOT crash or panic

---

## Scope

**Component Under Test:** `DecoderWorker` (wkmp-ap/src/audio/decode.rs)

**Interfaces:**
- Input: Decode request with non-existent file path
- Output: Error logged, event emitted, chain released

**Dependencies:**
- symphonia decoder (will return IoError)
- Event system (wkmp-common events)
- Logging system (tracing)

---

## Test Setup

**Given:**
```rust
// Create decoder worker
let buffer_manager = Arc::new(BufferManager::new());
let decoder = Arc::new(DecoderWorker::new(Arc::clone(&buffer_manager)));

// Create test passage with non-existent file
let passage = PassageWithTiming {
    passage_id: Some(Uuid::new_v4()),
    file_path: PathBuf::from("/nonexistent/file.mp3"),  // File does NOT exist
    start_time_ticks: 0,
    end_time_ticks: Some(10000),
    // ... other fields
};

// Set up event capture
let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
decoder.set_event_channel(event_tx);

// Set up log capture
let log_subscriber = tracing_subscriber::fmt()
    .with_max_level(tracing::Level::ERROR)
    .with_test_writer()
    .finish();
```

---

## Test Execution

**When:**
```rust
// Submit decode request
let passage_id = passage.passage_id.unwrap();
let result = decoder.submit(
    passage_id,
    passage.clone(),
    DecodePriority::Immediate,
    true,  // full_decode
).await;

// Wait for decode attempt to complete
tokio::time::sleep(Duration::from_millis(500)).await;
```

---

## Test Verification

**Then:**

### 1. Error Logged at ERROR Level
```rust
// Verify log captured
let logs = log_capture.get_logs();
assert!(
    logs.iter().any(|log| {
        log.level == Level::ERROR &&
        log.target.contains("Decoder") &&
        log.message.contains("File read error") &&
        log.fields.contains_key("passage_id") &&
        log.fields.contains_key("file_path") &&
        log.fields["file_path"].contains("/nonexistent/file.mp3")
    }),
    "ERROR log should be emitted with correct fields"
);
```

### 2. PassageDecodeFailed Event Emitted
```rust
// Check event was emitted
let event = tokio::time::timeout(
    Duration::from_secs(2),
    event_rx.recv()
).await.expect("Should receive event within 2 seconds");

match event.unwrap() {
    WkmpEvent::PassageDecodeFailed {
        passage_id: event_pid,
        error_type,
        error_message,
        file_path,
        timestamp,
    } => {
        assert_eq!(event_pid, passage_id, "Event passage_id should match");
        assert_eq!(error_type, "file_read_error", "Error type should be file_read_error");
        assert!(error_message.contains("not found") || error_message.contains("No such file"),
                "Error message should indicate file not found");
        assert_eq!(file_path, "/nonexistent/file.mp3", "File path should match");
        assert!(timestamp <= Utc::now(), "Timestamp should be valid");
    }
    other => panic!("Expected PassageDecodeFailed event, got {:?}", other),
}
```

### 3. Decoder Chain Released
```rust
// Verify chain is available for reuse
let chain_status = buffer_manager.get_chain_status().await;
assert_eq!(
    chain_status.available_chains,
    chain_status.total_chains,
    "All chains should be available after error (chain released)"
);
```

### 4. No Panic or Crash
```rust
// Decoder worker should still be functional
assert!(
    decoder.is_running().await,
    "Decoder worker should still be running (no panic)"
);

// Should accept new decode requests
let new_passage_id = Uuid::new_v4();
let result = decoder.submit(
    new_passage_id,
    create_valid_test_passage(),  // This time with valid file
    DecodePriority::Immediate,
    true,
).await;
assert!(result.is_ok(), "Decoder should accept new requests after error");
```

---

## Pass Criteria

**Test passes if ALL of the following are true:**
- ✓ ERROR-level log emitted with passage_id, file_path, and error message
- ✓ PassageDecodeFailed event emitted with all required fields
- ✓ Decoder chain released (available for next passage)
- ✓ No panic occurred (decoder worker still running)
- ✓ Decoder accepts new requests after error

---

## Fail Criteria

**Test fails if ANY of the following occur:**
- ✗ No log emitted
- ✗ Wrong log level (not ERROR)
- ✗ Missing log fields (passage_id, file_path)
- ✗ No event emitted
- ✗ Wrong event type
- ✗ Missing event fields
- ✗ Decoder chain NOT released (leak)
- ✗ Decoder worker panicked or crashed
- ✗ Decoder refuses new requests after error

---

## Edge Cases Tested

- Non-existent file path (primary scenario)
- File path with special characters
- Null/empty file path (should be caught earlier, but test defensive code)

---

## Implementation Notes

**Error Injection:**
Use real file system - just provide non-existent path. No mocking needed.

**Timing:**
Allow 500ms for decode attempt to fail (symphonia should fail immediately on file open).

**Cleanup:**
Ensure decoder worker is shut down properly in test teardown.

---

**Test Status:** Defined
**Implementation Status:** Pending
**Last Verified:** N/A (not yet implemented)
