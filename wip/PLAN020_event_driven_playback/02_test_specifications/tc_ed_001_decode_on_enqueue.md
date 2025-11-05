# TC-ED-001: Decode Triggered on Enqueue

**Test ID:** TC-ED-001
**Category:** Event-Driven Decode
**Priority:** P0 (Critical)
**Requirements:** FR-001, NFR-001
**Source:** Specification §5.3.1 (lines 680-686)

---

## Test Objective

Verify that decode requests are issued immediately (<1ms) when a passage is enqueued, without requiring watchdog intervention.

---

## BDD Specification

### Scenario: Enqueue passage triggers immediate decode request

**Given:**
- Empty queue (no current, next, or queued passages)
- PlaybackEngine initialized with test configuration
- DecoderWorkerSpy installed to track decode requests
- No decode activity in progress

**When:**
- `enqueue_file(test_audio.mp3)` is called
- Timestamp T0 captured at enqueue call

**Then:**
- Decode request issued for enqueued passage
- Decode request timestamp T1 within 1ms of T0 (T1 - T0 < 1ms)
- Decode priority is `DecodePriority::Immediate` (enqueued to current position)
- No watchdog intervention occurs (test would panic if it did)
- DecoderWorkerSpy.verify_decode_request() passes with max_latency_ms=1

---

## Test Implementation Pseudocode

```rust
#[tokio::test]
async fn test_decode_triggered_on_enqueue() -> Result<()> {
    // Setup
    let engine = TestEngine::new().await?;
    let spy = DecoderWorkerSpy::new();
    engine.install_decoder_spy(spy.clone()).await;

    // Record start time
    let t0 = Instant::now();

    // Execute
    let queue_entry_id = engine.enqueue_file("tests/test_assets/100ms_silence.mp3").await?;

    // Verify timing and priority
    spy.verify_decode_request(
        queue_entry_id,
        DecodePriority::Immediate,
        1 // max latency: 1ms
    )?;

    // Verify no watchdog intervention (test mode panics on intervention)
    // If we reach here, watchdog did not intervene
    assert!(t0.elapsed() < Duration::from_millis(10), "Test should complete quickly");

    Ok(())
}
```

---

## Success Criteria

- ✓ Test passes with <1ms latency (measured)
- ✓ DecodePriority::Immediate used for first enqueue
- ✓ No watchdog intervention (test does not panic)
- ✓ Test execution time <100ms

---

## Failure Modes

**Failure 1: Decode not requested**
- **Symptom:** DecoderWorkerSpy.verify_decode_request() returns error "No decode request for {id}"
- **Root Cause:** Event-driven enqueue not implemented or event handler not registered
- **Fix:** Implement event trigger in enqueue_file() per §4.3.1

**Failure 2: Latency >1ms**
- **Symptom:** verify_decode_request() returns error "Latency too high: Xms (max: 1ms)"
- **Root Cause:** Event propagation delay or blocking operations in handler
- **Fix:** Profile event handler, remove blocking operations

**Failure 3: Watchdog intervention**
- **Symptom:** Test panics with "WATCHDOG INTERVENTION IN TEST: Current passage has no buffer"
- **Root Cause:** Event system not working, watchdog triggered decode instead
- **Fix:** Debug event emission and handler registration

**Failure 4: Wrong priority**
- **Symptom:** verify_decode_request() returns error "Wrong priority: expected Immediate, got Next"
- **Root Cause:** Priority calculation incorrect in enqueue_file()
- **Fix:** Verify queue position detection logic per §4.3.1

---

## Dependencies

**Code References:**
- [queue.rs:enqueue_file()](../../../wkmp-ap/src/playback/engine/queue.rs) - Implementation target
- Specification §4.3.1 (lines 322-383) - Design reference

**Test Infrastructure:**
- DecoderWorkerSpy (§5.4 lines 789-826)
- TestEngine wrapper ([wkmp-ap/tests/test_engine.rs](../../../wkmp-ap/tests/test_engine.rs))

**Related Tests:**
- TC-ED-002 (decode on advance) - Similar pattern, different trigger
- TC-ED-003 (priority verification) - Extends priority testing
- TC-E2E-001 (complete flow) - Integration test including this behavior

---

## Traceability

**Requirements Verified:**
- FR-001: Event-Driven Decode Initiation (primary)
- NFR-001: Responsiveness (<1ms latency)

**Specification Sections:**
- §3.1 FR-001: Requirement definition
- §4.3.1: Event-driven enqueue implementation
- §5.3.1: Test case specification
- §5.4: Test spy infrastructure
