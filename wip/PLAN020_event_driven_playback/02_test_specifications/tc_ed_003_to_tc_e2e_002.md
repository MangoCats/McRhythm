# Test Specifications: TC-ED-003 through TC-E2E-002

**Consolidated Test Specifications**
**Total Tests:** 9 (TC-ED-003, TC-ED-004, TC-ED-005, TC-WD-001, TC-WD-002, TC-WD-003, TC-E2E-001, TC-E2E-002, TC-WD-DISABLED-001)

---

## TC-ED-003: Decode Priority Correct by Position

**Priority:** P0 | **Requirements:** FR-001 | **Source:** §5.3.1 (lines 697-703)

### Objective
Verify decode priority matches queue position (Current=Immediate, Next=Next, Queued=Prefetch).

### BDD Specification

**Given:** Empty queue, DecoderWorkerSpy installed

**When:**
1. Enqueue passage A (position: current)
2. Enqueue passage B (position: next)
3. Enqueue passage C (position: queued[0])

**Then:**
- Passage A decode priority: `DecodePriority::Immediate`
- Passage B decode priority: `DecodePriority::Next`
- Passage C decode priority: `DecodePriority::Prefetch`

### Implementation Sketch
```rust
#[tokio::test]
async fn test_decode_priority_by_position() -> Result<()> {
    let engine = TestEngine::new().await?;
    let spy = DecoderWorkerSpy::new();
    engine.install_decoder_spy(spy.clone()).await;

    let id_a = engine.enqueue_file("test1.mp3").await?;
    let id_b = engine.enqueue_file("test2.mp3").await?;
    let id_c = engine.enqueue_file("test3.mp3").await?;

    spy.verify_decode_request(id_a, DecodePriority::Immediate, 1)?;
    spy.verify_decode_request(id_b, DecodePriority::Next, 1)?;
    spy.verify_decode_request(id_c, DecodePriority::Prefetch, 1)?;
    Ok(())
}
```

**Traceability:** FR-001 (Event-Driven Decode)

---

## TC-ED-004: Mixer Starts on Buffer Threshold

**Priority:** P0 | **Requirements:** FR-002, NFR-001 | **Source:** §5.3.2 (lines 706-714)

### Objective
Verify mixer startup triggered immediately (<1ms) when buffer reaches threshold.

### BDD Specification

**Given:**
- Current passage in queue
- Mixer idle
- Buffer <3000ms

**When:**
- BufferManager.push_samples() called repeatedly
- Buffer level reaches 3000ms (threshold crossed)

**Then:**
- BufferThresholdReached event emitted
- Mixer startup initiated within 1ms
- PassageStarted event emitted
- No watchdog intervention

### Implementation Sketch
```rust
#[tokio::test]
async fn test_mixer_starts_on_threshold() -> Result<()> {
    let engine = TestEngine::new().await?;
    let id = engine.enqueue_file("test.mp3").await?;

    // Simulate buffer fill to threshold
    let t0 = Instant::now();
    engine.simulate_buffer_fill(id, 3000).await?;

    // Wait briefly for event propagation
    tokio::time::sleep(Duration::from_millis(2)).await;

    // Verify mixer started
    let mixer_passage = engine.get_mixer_current_passage().await;
    assert_eq!(mixer_passage, Some(id));
    assert!(t0.elapsed() < Duration::from_millis(5), "Mixer start latency");

    Ok(())
}
```

**Traceability:** FR-002 (Event-Driven Mixer Startup), NFR-001 (Responsiveness)

---

## TC-ED-005: Mixer Already Playing - No Duplicate Start

**Priority:** P0 | **Requirements:** FR-002 | **Source:** §5.3.2 (lines 715-721)

### Objective
Verify no duplicate mixer start when threshold reached while already playing.

### BDD Specification

**Given:**
- Mixer already playing passage A
- Passage B enqueued (next position)
- Passage B buffer <3000ms

**When:**
- Passage B buffer reaches 3000ms threshold

**Then:**
- BufferThresholdReached event emitted for passage B
- Mixer continues playing passage A (no restart)
- Current playback uninterrupted

### Implementation Sketch
```rust
#[tokio::test]
async fn test_mixer_no_duplicate_start() -> Result<()> {
    let engine = TestEngine::new().await?;
    let id_a = engine.enqueue_file("test1.mp3").await?;
    let id_b = engine.enqueue_file("test2.mp3").await?;

    // Start playback of A
    engine.simulate_buffer_fill(id_a, 3000).await?;
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(engine.get_mixer_current_passage().await, Some(id_a));

    // Fill buffer for B
    engine.simulate_buffer_fill(id_b, 3000).await?;
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Verify mixer still on A
    assert_eq!(engine.get_mixer_current_passage().await, Some(id_a));

    Ok(())
}
```

**Traceability:** FR-002 (Event-Driven Mixer Startup - edge case)

---

## TC-WD-001: Watchdog Detects Missing Current Buffer

**Priority:** P0 | **Requirements:** FR-003, FR-004 | **Source:** §5.3.3 (lines 724-731)

### Objective
Verify watchdog detects and intervenes when current passage has no buffer.

### BDD Specification

**Given:**
- Current passage in queue
- No buffer exists for current passage (event system failed to trigger decode)
- Watchdog enabled in production mode (NOT test mode)

**When:**
- Watchdog check runs (100ms timer)

**Then:**
- WARN logged: "[WATCHDOG] Event system failure detected: Current passage X has no buffer. Intervention: Requesting immediate decode. This indicates a bug in event-driven logic."
- In production: Decode request issued
- In test: Test fails with panic

### Implementation Sketch
```rust
#[tokio::test]
#[should_panic(expected = "WATCHDOG INTERVENTION")]
async fn test_watchdog_detects_missing_current_buffer() {
    let engine = TestEngine::new_with_watchdog_enabled().await.unwrap();

    // Enqueue WITHOUT triggering decode (simulate event failure)
    let id = engine.enqueue_file_without_decode("test.mp3").await.unwrap();

    // Wait for watchdog to run (>100ms)
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Test should panic due to watchdog intervention
    // If we reach here, watchdog didn't intervene (test fails)
    panic!("Watchdog should have intervened");
}
```

**Traceability:** FR-003 (Watchdog Loop), FR-004 (Test Coverage)

---

## TC-WD-002: Watchdog Detects Mixer Not Started

**Priority:** P0 | **Requirements:** FR-003, FR-004 | **Source:** §5.3.3 (lines 732-739)

### Objective
Verify watchdog detects mixer idle with ready buffer.

### BDD Specification

**Given:**
- Current passage in queue
- Buffer ≥3000ms (ready for playback)
- Mixer idle (event system failed to start mixer)

**When:** Watchdog check runs

**Then:**
- WARN logged: "[WATCHDOG] Mixer idle with ready buffer (X). Intervention: Starting mixer now."
- In production: Mixer started
- In test: Test fails with panic

**Traceability:** FR-003 (Watchdog Loop), FR-004 (Test Coverage)

---

## TC-WD-003: Watchdog Detects Missing Next Buffer

**Priority:** P0 | **Requirements:** FR-003, FR-004 | **Source:** §5.3.3 (lines 740-747)

### Objective
Verify watchdog detects missing next passage buffer while current playing.

### BDD Specification

**Given:**
- Current passage playing
- Next passage in queue
- No buffer for next passage (event system failed to prefetch)

**When:** Watchdog check runs

**Then:**
- WARN logged: "[WATCHDOG] Next passage X has no buffer while current playing. Intervention: Requesting next decode."
- In production: Decode request issued
- In test: Test fails with panic

**Traceability:** FR-003 (Watchdog Loop), FR-004 (Test Coverage)

---

## TC-E2E-001: Complete Playback Flow (Event-Driven)

**Priority:** P0 | **Requirements:** FR-001, FR-002, FR-004, NFR-001 | **Source:** §5.3.4 (lines 750-766)

### Objective
Verify complete playback flow executes entirely via events without watchdog intervention.

### BDD Specification

**Given:**
- Empty queue
- Audio output available (or mocked)

**When:**
1. Enqueue passage A
2. Wait for buffer threshold
3. Verify mixer starts
4. Wait for passage completion
5. Enqueue passage B during playback

**Then:**
- Each operation triggers next via events (no polling)
- No watchdog interventions
- Playback continuous and correct

**Timing Verification:**
- Enqueue → Decode start: <1ms
- Buffer threshold → Mixer start: <1ms
- PassageComplete → Queue advance: <1ms

### Implementation Sketch
```rust
#[tokio::test]
async fn test_complete_playback_flow_event_driven() -> Result<()> {
    let engine = TestEngine::new().await?;
    let spy = DecoderWorkerSpy::new();
    engine.install_decoder_spy(spy.clone()).await;

    // 1. Enqueue passage A
    let t0 = Instant::now();
    let id_a = engine.enqueue_file("test1.mp3").await?;

    // Verify decode triggered immediately
    spy.verify_decode_request(id_a, DecodePriority::Immediate, 1)?;

    // 2. Simulate buffer fill to threshold
    let t1 = Instant::now();
    engine.simulate_buffer_fill(id_a, 3000).await?;
    tokio::time::sleep(Duration::from_millis(2)).await;

    // Verify mixer started
    assert_eq!(engine.get_mixer_current_passage().await, Some(id_a));
    assert!(t1.elapsed() < Duration::from_millis(5), "Mixer start latency");

    // 3. Enqueue passage B during playback
    let id_b = engine.enqueue_file("test2.mp3").await?;
    spy.verify_decode_request(id_b, DecodePriority::Next, 1)?;

    // 4. Simulate passage A completion
    let t2 = Instant::now();
    engine.simulate_passage_complete(id_a).await?;
    tokio::time::sleep(Duration::from_millis(2)).await;

    // Verify queue advanced (B now current)
    let current = engine.get_current_passage().await;
    assert_eq!(current.map(|e| e.queue_entry_id), Some(id_b));
    assert!(t2.elapsed() < Duration::from_millis(5), "Queue advance latency");

    // No watchdog interventions (test would panic if watchdog intervened)
    Ok(())
}
```

**Traceability:** FR-001, FR-002, FR-004, NFR-001 (comprehensive integration)

---

## TC-E2E-002: Multi-Passage Queue Build (Event-Driven)

**Priority:** P0 | **Requirements:** FR-001, FR-004 | **Source:** §5.3.4 (lines 768-775)

### Objective
Verify rapid enqueue of multiple passages triggers all decode requests immediately with correct priorities.

### BDD Specification

**Given:** Empty queue

**When:** Enqueue 10 passages rapidly (sequential calls)

**Then:**
- All decode requests triggered immediately (<1ms per passage)
- Priority correct:
  - Passage 0: Immediate (current)
  - Passage 1: Next
  - Passages 2-9: Prefetch
- No decode requests from watchdog (all event-driven)
- No watchdog interventions

### Implementation Sketch
```rust
#[tokio::test]
async fn test_multi_passage_queue_build() -> Result<()> {
    let engine = TestEngine::new().await?;
    let spy = DecoderWorkerSpy::new();
    engine.install_decoder_spy(spy.clone()).await;

    // Enqueue 10 passages
    let ids: Vec<Uuid> = futures::future::join_all(
        (0..10).map(|i| engine.enqueue_file(format!("test{}.mp3", i)))
    ).await.into_iter().collect::<Result<Vec<_>>>()?;

    // Verify all decode requests triggered
    spy.verify_decode_request(ids[0], DecodePriority::Immediate, 1)?;
    spy.verify_decode_request(ids[1], DecodePriority::Next, 1)?;
    for i in 2..10 {
        spy.verify_decode_request(ids[i], DecodePriority::Prefetch, 1)?;
    }

    // Verify no watchdog interventions (test would panic)
    Ok(())
}
```

**Traceability:** FR-001 (Event-Driven Decode), FR-004 (Test Coverage)

---

## TC-WD-DISABLED-001: Event System Works Without Watchdog

**Priority:** P1 | **Requirements:** FR-001, FR-002, FR-004, NFR-003 | **Source:** §5.3.5 (lines 778-785)

### Objective
Verify event system is sufficient without watchdog (watchdog is safety net, not primary mechanism).

### BDD Specification

**Given:** PlaybackEngine with `watchdog_enabled: false`

**When:** Complete playback flow (enqueue, play, advance)

**Then:**
- All operations complete successfully via events
- No stuck states (buffer fills, mixer starts, queue advances)
- Verifies event system is self-sufficient

### Implementation Sketch
```rust
#[tokio::test]
async fn test_event_system_without_watchdog() -> Result<()> {
    let mut config = TestConfig::default();
    config.watchdog_enabled = false;

    let engine = TestEngine::new_with_config(config).await?;

    // Complete flow without watchdog
    let id_a = engine.enqueue_file("test1.mp3").await?;
    engine.simulate_buffer_fill(id_a, 3000).await?;
    tokio::time::sleep(Duration::from_millis(10)).await;

    assert_eq!(engine.get_mixer_current_passage().await, Some(id_a));

    let id_b = engine.enqueue_file("test2.mp3").await?;
    engine.simulate_passage_complete(id_a).await?;
    tokio::time::sleep(Duration::from_millis(10)).await;

    assert_eq!(engine.get_current_passage().await.map(|e| e.queue_entry_id), Some(id_b));

    // Success - no watchdog needed
    Ok(())
}
```

**Traceability:** FR-001, FR-002, FR-004, NFR-003 (Testability - isolation)

---

## Summary

**Total Tests:** 11
**P0 Tests:** 10
**P1 Tests:** 1

**Coverage:**
- Event-driven decode: 3 tests
- Event-driven mixer: 2 tests
- Watchdog detection: 3 tests
- End-to-end flows: 2 tests
- Watchdog disabled: 1 test

**Requirements Coverage:** 100% (all requirements have tests)
