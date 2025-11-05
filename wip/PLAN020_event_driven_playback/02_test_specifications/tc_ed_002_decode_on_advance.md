# TC-ED-002: Decode Triggered on Queue Advance

**Test ID:** TC-ED-002 | **Category:** Event-Driven Decode | **Priority:** P0
**Requirements:** FR-001, NFR-001 | **Source:** §5.3.1 (lines 688-695)

---

## Test Objective

Verify decode requests issued immediately (<1ms) when queue advances, triggering decode for newly promoted passages.

---

## BDD Specification

**Given:**
- Queue with current, next, and 3 queued passages
- All passages have buffers assigned
- Current passage playing

**When:**
- Current passage completes (PassageComplete event)
- Queue advances (next → current, queued[0] → next)

**Then:**
- Decode request issued for newly promoted next passage (was queued[0])
- Latency <1ms from queue advance to decode request
- Priority is `DecodePriority::Next`
- No watchdog intervention

---

## Implementation Sketch

```rust
#[tokio::test]
async fn test_decode_on_queue_advance() -> Result<()> {
    let engine = TestEngine::new().await?;
    let spy = DecoderWorkerSpy::new();
    engine.install_decoder_spy(spy.clone()).await;

    // Enqueue 5 passages (current, next, 3 queued)
    let ids = engine.enqueue_multiple(5).await?;

    // Simulate current passage completion
    engine.simulate_passage_complete(ids[0]).await?;

    // Verify decode triggered for newly promoted next (was queued[0])
    spy.verify_decode_request(ids[2], DecodePriority::Next, 1)?;

    Ok(())
}
```

---

## Success Criteria

✓ Decode request <1ms after queue advance
✓ Priority correct (Next for promoted passage)
✓ No watchdog intervention

---

## Traceability

**Requirements:** FR-001 (Event-Driven Decode), NFR-001 (Responsiveness)
**Specification:** §3.1 FR-001, §4.3.4 (queue advance implementation), §5.3.1 TC-ED-002
