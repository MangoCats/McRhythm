# TC-S-RECOVERY-001: Multiple Concurrent Errors

**Test ID:** TC-S-RECOVERY-001
**Test Type:** System Test (End-to-End)
**Requirements:** All REQ-AP-ERR-### requirements
**Specification:** SPEC021 (Complete error handling strategy)
**Priority:** High
**Estimated Effort:** 60 minutes

---

## Test Objective

Verify system robustness under multiple concurrent error conditions:
- Decode errors + buffer underrun + device configuration issues
- System continues operating through cascading failures
- All error events emitted correctly
- All errors logged appropriately
- Queue integrity maintained
- User controls remain functional
- System eventually recovers to normal operation

---

## Scope

**System-Level Integration:**
- Complete wkmp-ap playback stack
- All error handling paths
- Event emission across all components
- Logging system
- Database persistence
- Audio output device management

**Real-World Scenario:**
Simulates stressed production environment with:
- Poor quality audio files (some truncated, some unsupported)
- Slow I/O (network filesystem)
- Resource constraints (limited memory)
- Audio device issues (configuration problems)

---

## Test Setup

**Given:**
```rust
// Set up complete system
let db_pool = setup_test_database().await;
let config = TestConfig {
    buffer_capacity_ms: 1000,  // Small buffer (stress test)
    buffer_underrun_recovery_timeout_ms: 2000,
    maximum_decode_streams: 3,  // Limited chains
    // ... other settings
};
let system = setup_full_wkmp_ap_system(db_pool, config).await;

// Set up comprehensive event and log capture
let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
let log_capture = setup_log_capture();
system.set_event_channel(event_tx);

// Enqueue mixed-quality passages
let passages = vec![
    ("valid1.mp3", PassageQuality::Valid),
    ("truncated_30pct.mp3", PassageQuality::Truncated30),  // Will fail <50%
    ("valid2.flac", PassageQuality::Valid),
    ("unsupported_codec.xyz", PassageQuality::UnsupportedCodec),  // Will fail
    ("valid3.opus", PassageQuality::Valid),
    ("truncated_60pct.mp3", PassageQuality::Truncated60),  // Will play (≥50%)
    ("valid4.wav", PassageQuality::Valid),
];

for (file, quality) in &passages {
    enqueue_test_passage(&system, file, quality).await;
}

// Configure device to have initial configuration issues
system.set_audio_device("problematic_device").await;

// Inject I/O delay to cause buffer underruns
system.inject_io_delay(Duration::from_millis(500)).await;
```

---

## Test Execution

**When:**
```rust
// Start playback and let system run for 2 minutes
system.play().await.unwrap();

// Monitor system for 120 seconds
let test_duration = Duration::from_secs(120);
let start_time = Instant::now();

// Collect events during test
let mut events_collected = Vec::new();
while start_time.elapsed() < test_duration {
    if let Ok(event) = tokio::time::timeout(
        Duration::from_millis(100),
        event_rx.recv()
    ).await {
        if let Some(evt) = event {
            events_collected.push(evt);
        }
    }
}
```

---

## Test Verification

**Then:**

### 1. All Error Types Detected and Handled
```rust
// Verify decode errors were detected
let decode_failed_events: Vec<_> = events_collected.iter()
    .filter(|e| matches!(e, WkmpEvent::PassageDecodeFailed { .. }))
    .collect();
assert!(decode_failed_events.len() >= 2, "Should detect truncated and missing files");

// Verify codec errors
let codec_events: Vec<_> = events_collected.iter()
    .filter(|e| matches!(e, WkmpEvent::PassageUnsupportedCodec { .. }))
    .collect();
assert!(codec_events.len() >= 1, "Should detect unsupported codec");

// Verify buffer underruns
let underrun_events: Vec<_> = events_collected.iter()
    .filter(|e| matches!(e, WkmpEvent::BufferUnderrun { .. }))
    .collect();
assert!(underrun_events.len() >= 1, "Should experience buffer underruns (small buffer + I/O delay)");

// Verify device config issues
let device_events: Vec<_> = events_collected.iter()
    .filter(|e| matches!(e, WkmpEvent::AudioDeviceConfigError { .. } | WkmpEvent::AudioDeviceConfigFallback { .. }))
    .collect();
assert!(device_events.len() >= 1, "Should encounter device config issues");
```

### 2. System Continued Operating
```rust
// Verify system is still running
assert!(system.is_running().await, "System should still be running");

// Verify playback reached the end (or near end) of queue
let final_position = system.get_position().await;
assert!(
    final_position.position_ms > 30000,  // At least 30 seconds played
    "System should have made progress through queue"
);
```

### 3. Queue Integrity Maintained
```rust
let final_queue = system.get_queue().await;

// Failed passages should be removed
assert!(
    !final_queue.iter().any(|e| e.file_path.to_str().unwrap().contains("truncated_30pct")),
    "Failed passage should be removed"
);
assert!(
    !final_queue.iter().any(|e| e.file_path.to_str().unwrap().contains("unsupported_codec")),
    "Unsupported codec passage should be removed"
);

// Valid passages should remain
assert!(
    final_queue.iter().any(|e| e.file_path.to_str().unwrap().contains("valid")),
    "Valid passages should remain in queue"
);

// Truncated 60% should have played (≥50% threshold)
// Check if it was completed
let passage_completed_events: Vec<_> = events_collected.iter()
    .filter(|e| matches!(e, WkmpEvent::PassageCompleted { .. }))
    .collect();
assert!(passage_completed_events.len() >= 1, "At least one passage should complete");
```

### 4. All Errors Logged
```rust
let logs = log_capture.get_logs();

// Verify ERROR logs for fatal errors
let error_logs: Vec<_> = logs.iter().filter(|l| l.level == Level::ERROR).collect();
assert!(error_logs.len() >= 3, "Should have ERROR logs for decode failures");

// Verify WARNING logs for recoverable errors
let warning_logs: Vec<_> = logs.iter().filter(|l| l.level == Level::WARNING).collect();
assert!(warning_logs.len() >= 2, "Should have WARNING logs for underruns, partial decodes");

// Verify structured logging (all logs have required fields)
for log in error_logs {
    assert!(log.fields.contains_key("timestamp"), "Log should have timestamp");
    assert!(log.fields.contains_key("component"), "Log should have component");
    // passage_id may not always be present (e.g., device errors)
}
```

### 5. User Controls Functional Throughout
```rust
// Test pause/resume during errors
system.pause().await.unwrap();
tokio::time::sleep(Duration::from_millis(100)).await;
assert_eq!(system.get_state().await, PlaybackState::Paused);

system.play().await.unwrap();
tokio::time::sleep(Duration::from_millis(100)).await;
assert_eq!(system.get_state().await, PlaybackState::Playing);

// Test volume control
system.set_volume(0.5).await.unwrap();
assert_eq!(system.get_volume().await, 0.5);

// Test skip
let position_before_skip = system.get_position().await.passage_id;
system.skip_forward().await.unwrap();
tokio::time::sleep(Duration::from_millis(500)).await;
let position_after_skip = system.get_position().await.passage_id;
assert_ne!(position_before_skip, position_after_skip, "Skip should change passage");
```

### 6. System Recovers to Normal Operation
```rust
// Remove I/O delay and problematic device
system.clear_io_delay().await;
system.set_audio_device("default").await;

// Enqueue new valid passages
let new_passage_id = enqueue_test_passage(&system, "recovery_test.mp3", PassageQuality::Valid).await;

// Wait for new passage to start
tokio::time::sleep(Duration::from_secs(5)).await;

// Verify no more errors occurring
let events_after_recovery: Vec<_> = collect_events_for_duration(&mut event_rx, Duration::from_secs(10)).await;
let error_events_after: Vec<_> = events_after_recovery.iter()
    .filter(|e| matches!(e,
        WkmpEvent::PassageDecodeFailed { .. } |
        WkmpEvent::BufferUnderrun { .. } |
        WkmpEvent::AudioDeviceConfigError { .. }
    ))
    .collect();

assert_eq!(error_events_after.len(), 0, "No errors should occur after recovery");
```

---

## Pass Criteria

**Test passes if ALL of the following are true:**
- ✓ All error types detected (decode, buffer, device)
- ✓ All errors emitted correct events
- ✓ All errors logged at appropriate levels
- ✓ System continued operating through errors
- ✓ Failed passages removed from queue
- ✓ Valid passages played successfully
- ✓ Queue integrity maintained
- ✓ User controls functional throughout
- ✓ System recovered to normal operation
- ✓ No panics or crashes

---

## Fail Criteria

**Test fails if ANY of the following occur:**
- ✗ System crashed or panicked
- ✗ System stopped responding (deadlock)
- ✗ Queue corrupted (invalid state)
- ✗ User controls not functional
- ✗ Missing error events
- ✗ Missing error logs
- ✗ System did not recover after stress removed
- ✗ Memory leak (growing memory usage)
- ✗ File handle leak (exhausted handles)

---

## Performance Metrics

**Measure during test:**
- Event emission latency (<100ms from error to event)
- Error recovery time (per error type)
- Memory usage (should remain stable)
- CPU usage (should remain reasonable)
- Audio output continuity (minimize gaps)

---

## Success Thresholds

- System uptime: 100% (no crashes)
- Event delivery: 100% (all errors emit events)
- Log completeness: 100% (all errors logged)
- Queue integrity: 100% (no corruption)
- User control availability: 100% (always responsive)

---

**Test Status:** Defined
**Implementation Status:** Pending
**Execution Time:** ~5 minutes (2 min test + 3 min setup/verification)
