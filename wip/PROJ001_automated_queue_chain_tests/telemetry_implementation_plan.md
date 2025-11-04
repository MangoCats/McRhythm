# Telemetry Implementation Plan for Priority Tests

**Goal:** Add minimal telemetry to enable upgrading Tests 6-7 from infrastructure to functional tests

**Estimated Effort:** 4-6 hours

---

## Current State Analysis

### Existing Infrastructure ✅

**Generation Counter System:**
- `WorkerState.chain_assignments_generation` - Tracks chain assignment changes
- `WorkerState.last_observed_generation` - Tracks decoder's observed state
- Incremented on: chain assign, chain release, buffer ready
- Used for: Re-evaluation trigger detection

**Priority Selection:**
- `select_highest_priority_chain()` - Selects based on play_order
- Already logs with `debug!()` which buffer selected
- Checks `can_decoder_resume()` for each candidate
- Returns `Option<Uuid>` of selected chain

**Decoder Worker Loop:**
- Tracks `current_target` - Which chain decoder is filling
- Has `last_work_start` - Timestamp for work period tracking
- Implements re-evaluation logic in `should_reevaluate_priority()`

### What Tests Need

**Test 6: Buffer Priority by Queue Position**
- **Requires:** Know which buffer decoder selected to fill
- **Solution:** Expose `current_target` from worker state

**Test 7: Re-evaluation Trigger on Chain Change**
- **Requires:** Detect when re-evaluation occurred
- **Solution:** Expose generation counter or add event

**Test 8: Buffer Fill Level Selection** (Already functional via infrastructure)
- Can query `test_get_buffer_fill_percent()` ✅

**Test 9: Work Period Re-evaluation** (Requires timing control)
- Needs playback environment - defer for now

---

## Proposed Solution: Minimal Test Helpers

### Approach: Extend PlaybackEngine Test Helpers

Add 2-3 new test helper methods to `PlaybackEngine` following existing pattern:

```rust
// In wkmp-ap/src/playback/engine/core.rs (test helpers section)

#[doc(hidden)]
pub async fn test_get_decoder_target(&self) -> Option<Uuid> {
    // Return current decoder target (which buffer being filled)
}

#[doc(hidden)]
pub async fn test_get_generation_counter(&self) -> (u64, u64) {
    // Return (current_generation, last_observed_generation)
}

#[doc(hidden)]
pub async fn test_wait_for_generation_change(&self, timeout_ms: u64) -> bool {
    // Wait for generation counter to change (re-evaluation occurred)
    // Returns true if changed, false if timeout
}
```

### Implementation Details

**1. Add helper to DecoderWorker:**
```rust
// In decoder_worker.rs
impl DecoderWorker {
    #[doc(hidden)]
    pub async fn test_get_current_target(&self) -> Option<Uuid> {
        let state = self.state.lock().await;
        state.current_target
    }

    #[doc(hidden)]
    pub async fn test_get_generation(&self) -> (u64, u64) {
        let state = self.state.lock().await;
        (state.chain_assignments_generation, state.last_observed_generation)
    }
}
```

**2. Expose through PlaybackEngine:**
```rust
// In engine/core.rs
impl PlaybackEngine {
    #[doc(hidden)]
    pub async fn test_get_decoder_target(&self) -> Option<Uuid> {
        self.decoder_worker.test_get_current_target().await
    }

    #[doc(hidden)]
    pub async fn test_get_generation_counter(&self) -> (u64, u64) {
        self.decoder_worker.test_get_generation().await
    }

    #[doc(hidden)]
    pub async fn test_wait_for_generation_change(&self, timeout_ms: u64) -> bool {
        let (initial_gen, _) = self.test_get_generation_counter().await;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);

        loop {
            tokio::time::sleep(Duration::from_millis(10)).await;

            let (current_gen, _) = self.test_get_generation_counter().await;
            if current_gen != initial_gen {
                return true; // Generation changed (re-evaluation occurred)
            }

            if tokio::time::Instant::now() >= deadline {
                return false; // Timeout
            }
        }
    }
}
```

**3. Add to TestEngine wrapper:**
```rust
// In tests/test_engine.rs
impl TestEngine {
    pub async fn get_decoder_target(&self) -> Option<Uuid> {
        self.engine.test_get_decoder_target().await
    }

    pub async fn get_generation_counter(&self) -> (u64, u64) {
        self.engine.test_get_generation_counter().await
    }

    pub async fn wait_for_generation_change(&self, timeout_ms: u64) -> bool {
        self.engine.test_wait_for_generation_change(timeout_ms).await
    }
}
```

---

## Upgrade Path for Tests

### Test 6: Buffer Priority by Queue Position

**Current (Infrastructure):**
- Enqueues 3 passages
- Verifies monitoring capability exists
- Cannot test actual priority

**Upgrade to Functional:**
```rust
#[tokio::test]
async fn test_buffer_priority_by_queue_position() -> anyhow::Result<()> {
    let engine = TestEngine::new(12).await?;
    let temp_dir = TempDir::new()?;

    // Enqueue 3 passages
    let mut ids = Vec::new();
    for i in 0..3 {
        let path = create_test_audio_file_in_dir(temp_dir.path(), i)?;
        let id = engine.enqueue_file(path).await?;
        ids.push(id);
    }

    // Start playback to activate decoder
    engine.play().await?;

    // Wait for decoder to select a target
    sleep(Duration::from_millis(100)).await;

    // Verify decoder selected position 0 (highest priority)
    let target = engine.get_decoder_target().await;
    assert_eq!(target, Some(ids[0]), "Decoder should prioritize position 0 first");

    // Wait for position 0 buffer to fill
    // (Need to detect when buffer full - use buffer fill percent)
    loop {
        let fill = engine.engine.test_get_buffer_fill_percent(ids[0]).await;
        if fill.unwrap_or(0.0) > 0.8 {
            break; // Buffer nearly full
        }
        sleep(Duration::from_millis(50)).await;
    }

    // Verify decoder moved to position 1
    let target = engine.get_decoder_target().await;
    assert_eq!(target, Some(ids[1]), "Decoder should move to position 1 after 0 fills");

    Ok(())
}
```

### Test 7: Re-evaluation Trigger on Chain Change

**Current (Infrastructure):**
- Enqueues 3, removes 1
- Verifies chain assignments update
- Cannot test re-evaluation timing

**Upgrade to Functional:**
```rust
#[tokio::test]
async fn test_reevaluation_on_chain_assignment_change() -> anyhow::Result<()> {
    let engine = TestEngine::new(12).await?;
    let temp_dir = TempDir::new()?;

    // Enqueue 3 passages
    let mut ids = Vec::new();
    for i in 0..3 {
        let path = create_test_audio_file_in_dir(temp_dir.path(), i)?;
        let id = engine.enqueue_file(path).await?;
        ids.push(id);
    }

    // Start playback
    engine.play().await?;
    sleep(Duration::from_millis(100)).await;

    // Get initial generation counter
    let (gen_before, _) = engine.get_generation_counter().await;

    // Remove middle passage (chain assignment change)
    engine.remove_queue_entry(ids[1]).await?;

    // Verify generation counter incremented (re-evaluation triggered)
    let (gen_after, _) = engine.get_generation_counter().await;
    assert!(
        gen_after > gen_before,
        "Generation counter should increment on chain removal"
    );

    // Verify decoder re-evaluated (wait for generation change to be observed)
    let reevaluated = engine.wait_for_generation_change(1000).await;
    assert!(reevaluated, "Decoder should re-evaluate after chain removal");

    Ok(())
}
```

---

## Implementation Steps

### Phase 1: Add Test Helpers (2 hours)
1. Add `test_get_current_target()` to DecoderWorker
2. Add `test_get_generation()` to DecoderWorker
3. Expose through PlaybackEngine test helpers
4. Add methods to TestEngine wrapper
5. Verify compilation

### Phase 2: Upgrade Test 6 (1-2 hours)
1. Remove `#[ignore]` attribute
2. Implement functional test logic
3. Run test, debug issues
4. Verify test passes consistently

### Phase 3: Upgrade Test 7 (1-2 hours)
1. Remove `#[ignore]` attribute
2. Implement functional test logic
3. Run test, debug issues
4. Verify test passes consistently

### Phase 4: Documentation (30 minutes)
1. Update FINAL_TEST_SUITE_STATUS.md
2. Create session 7 summary
3. Update README.md metrics

---

## Risk Assessment

### Low Risk ✅
- Test helpers follow existing pattern (`#[doc(hidden)]`)
- No changes to production logic
- Only read-only state inspection
- Minimal new code (<100 lines)

### Medium Risk ⚠️
- Tests may be flaky if timing assumptions wrong
- Decoder behavior may vary under test vs production
- May need tuning of sleep/timeout values

**Mitigation:**
- Use generous timeouts (1000ms+)
- Add retry logic if needed
- Document timing assumptions

---

## Success Criteria

- [ ] Test helpers compile without errors
- [ ] Test 6 upgraded to functional and passing
- [ ] Test 7 upgraded to functional and passing
- [ ] No flakiness in 10 consecutive test runs
- [ ] Execution time remains <5s
- [ ] Documentation updated

---

## Alternative: Event-Based Approach (Deferred)

Could implement event stream for priority selection decisions:

```rust
enum DecoderEvent {
    PrioritySelected { queue_entry_id: Uuid, play_order: i64 },
    ReEvaluationTriggered { reason: String },
    BufferFull { queue_entry_id: Uuid },
}
```

**Pros:**
- More detailed telemetry
- Better for production monitoring
- Clearer test assertions

**Cons:**
- More implementation work (8+ hours)
- Requires event channel infrastructure
- May impact production performance

**Decision:** Defer to future enhancement. Current test helper approach sufficient for validating requirements.

---

## Next Steps After This Implementation

**Remaining Work:**
- Test 8: Already functional via infrastructure ✅
- Test 9: Requires timing control (playback environment) - defer
- Test 10: Edge case debug (low priority) - defer

**Expected Outcome:**
- 7 of 10 tests functional (70% functional coverage)
- 90% overall test implementation
- 2 tests remain infrastructure-only
- 1 known edge case documented
