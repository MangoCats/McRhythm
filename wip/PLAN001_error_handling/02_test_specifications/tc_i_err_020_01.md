# TC-I-ERR-020-01: Buffer Underrun Emergency Refill

**Test ID:** TC-I-ERR-020-01
**Test Type:** Integration Test
**Requirement:** REQ-AP-ERR-020 (Buffer underrun emergency refill with 2000ms timeout)
**Specification:** SPEC021 ERH-BUF-010 (Buffer Underrun)
**Priority:** High
**Estimated Effort:** 30 minutes

---

## Test Objective

Verify end-to-end buffer underrun recovery:
1. Underrun detected when buffer exhausted
2. Mixer pauses and inserts silence
3. Emergency refill triggered with priority decode
4. Refill completes within configured timeout (2000ms)
5. BufferUnderrun event emitted
6. Playback resumes when buffer refilled
7. BufferUnderrunRecovered event emitted

---

## Scope

**Components Under Test:**
- PlayoutRingBuffer (underrun detection)
- Mixer (underrun handling, silence insertion)
- DecoderWorker (emergency refill)
- Event system (underrun events)

**Integration Points:**
- Buffer → Mixer (underrun signal)
- Mixer → Decoder (emergency refill request)
- Decoder → Buffer (priority decode)
- Event emission across components

---

## Test Setup

**Given:**
```rust
// Set up full playback stack
let db_pool = setup_test_database().await;
let buffer_manager = Arc::new(BufferManager::new());
let decoder = Arc::new(DecoderWorker::new(Arc::clone(&buffer_manager)));
let engine = Arc::new(Engine::new(db_pool, buffer_manager, decoder));

// Configure aggressive buffer underrun scenario
sqlx::query("UPDATE settings SET value = '1000' WHERE key = 'buffer_capacity_ms'")
    .execute(&engine.db_pool).await.unwrap();
sqlx::query("UPDATE settings SET value = '2000' WHERE key = 'buffer_underrun_recovery_timeout_ms'")
    .execute(&engine.db_pool).await.unwrap();

// Set up event capture
let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
engine.set_event_channel(event_tx);

// Enqueue real audio file with slow decoder to cause underrun
let test_file = find_test_audio_file("large_file.mp3");
let passage_id = engine.enqueue_file(test_file).await.unwrap();

// Start playback
engine.play().await.unwrap();

// Artificially delay decoder to force underrun
decoder.inject_decode_delay(Duration::from_secs(3)).await;  // Longer than buffer capacity
```

---

## Test Execution

**When:**
```rust
// Wait for buffer to drain (should take ~1 second given 1000ms capacity)
tokio::time::sleep(Duration::from_millis(1500)).await;

// Buffer should now be underrun
```

---

## Test Verification

**Then:**

### 1. BufferUnderrun Event Emitted
```rust
let event1 = tokio::time::timeout(Duration::from_secs(1), event_rx.recv())
    .await.expect("Should receive BufferUnderrun event");

match event1.unwrap() {
    WkmpEvent::BufferUnderrun {
        passage_id: event_pid,
        buffer_fill_percent,
        timestamp,
    } => {
        assert_eq!(event_pid, passage_id);
        assert!(buffer_fill_percent < 0.1, "Buffer should be nearly empty: {}", buffer_fill_percent);
        assert!(timestamp <= Utc::now());
    }
    other => panic!("Expected BufferUnderrun, got {:?}", other),
}
```

### 2. Mixer Paused and Silence Inserted
```rust
// Check mixer state
let mixer_state = engine.get_mixer_state().await;
assert_eq!(mixer_state.status, MixerStatus::Paused, "Mixer should pause on underrun");

// Check that silence was inserted (no audio glitches)
// This would be verified by audio output monitoring in real scenario
// For test: verify mixer.underrun_silence_inserted flag
assert!(mixer_state.underrun_silence_inserted, "Silence should be inserted");
```

### 3. Emergency Refill Triggered
```rust
// Verify decoder received priority request
let decode_stats = decoder.get_stats().await;
assert!(
    decode_stats.emergency_refill_requests > 0,
    "Emergency refill should be requested"
);
```

### 4. Buffer Refills Within Timeout
```rust
// Wait for refill (should complete within 2000ms timeout)
tokio::time::sleep(Duration::from_millis(2500)).await;

// Check buffer fill level
let buffer_status = engine.get_buffer_status().await;
let passage_buffer = buffer_status.buffers.iter()
    .find(|b| b.passage_id == passage_id)
    .expect("Passage buffer should exist");

assert!(
    passage_buffer.fill_percent > 0.5,
    "Buffer should be refilled: {} (expected >50%)",
    passage_buffer.fill_percent
);
```

### 5. BufferUnderrunRecovered Event Emitted
```rust
let event2 = tokio::time::timeout(Duration::from_secs(1), event_rx.recv())
    .await.expect("Should receive BufferUnderrunRecovered event");

match event2.unwrap() {
    WkmpEvent::BufferUnderrunRecovered {
        passage_id: event_pid,
        recovery_time_ms,
        timestamp,
    } => {
        assert_eq!(event_pid, passage_id);
        assert!(recovery_time_ms <= 2000, "Recovery should complete within timeout");
        assert!(timestamp <= Utc::now());
    }
    other => panic!("Expected BufferUnderrunRecovered, got {:?}", other),
}
```

### 6. Playback Resumed
```rust
// Check mixer resumed
let mixer_state_after = engine.get_mixer_state().await;
assert_eq!(mixer_state_after.status, MixerStatus::Playing, "Mixer should resume after refill");

// Verify audio is actually playing (position advancing)
let pos1 = engine.get_position().await.position_ms;
tokio::time::sleep(Duration::from_millis(500)).await;
let pos2 = engine.get_position().await.position_ms;
assert!(pos2 > pos1, "Position should advance after recovery");
```

---

## Pass Criteria

**Test passes if ALL of the following are true:**
- ✓ BufferUnderrun event emitted when buffer exhausted
- ✓ Mixer paused immediately on underrun
- ✓ Silence inserted (no audio glitch)
- ✓ Emergency refill requested with priority
- ✓ Buffer refilled within configured timeout (2000ms)
- ✓ BufferUnderrunRecovered event emitted
- ✓ Playback resumed automatically
- ✓ Position continues advancing after recovery

---

## Fail Criteria

**Test fails if ANY of the following occur:**
- ✗ No BufferUnderrun event emitted
- ✗ Mixer did not pause
- ✗ No silence inserted (potential audio glitch)
- ✗ Emergency refill not requested
- ✗ Refill exceeded timeout
- ✗ No BufferUnderrunRecovered event
- ✗ Playback did not resume
- ✗ Position not advancing (playback stuck)

---

## Edge Cases

**Also test:**
- Multiple underruns in sequence
- Underrun during crossfade
- Underrun with queue depth = 1

---

## Performance Requirements

- Recovery time < 2000ms (configured timeout)
- No audio glitches (silence inserted smoothly)
- Position accuracy maintained (<100 samples drift)

---

**Test Status:** Defined
**Implementation Status:** Pending
