# Gap Analysis: Current Implementation vs. Amended Specifications

**Document Type:** Gap Analysis (Pre-Implementation)
**Created:** 2025-10-21
**Purpose:** Identify precise implementation gaps between current code and drafted specification amendments
**Status:** ANALYSIS ONLY - NO CODE CHANGES

---

## Executive Summary

This document analyzes the current implementation in `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/` against five drafted specification amendments:

1. **[DBD-DEC-090]** File Duration Probing for NULL Endpoints
2. **[DBD-DEC-095]** Endpoint Discovery and Return
3. **[DBD-BUF-065]** Buffer Capacity Validation
4. **[DBD-FADE-065]** Pre-Buffer Fade Application
5. **[DBD-COMP-015]** Completion State Machine Integration

**Key Finding:** All five requirements are either **Not Implemented** or **Partially Implemented**. Ring buffer architecture is in place but decoder/fade integration is incomplete.

---

## Section 1: Decoder-Related Requirements

### [DBD-DEC-090] File Duration Probing for NULL Endpoints

**Requirement Summary:** When `end_time_ticks=None` (file end), decoder must probe file metadata to discover actual duration before decode, then convert to ticks and return discovered endpoint.

**Status:** ❌ **Not Implemented**

**What Exists:**
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/decoder_pool.rs:325-330`
- Code converts `None` endpoint to `end_ms=0` for "decode until EOF" behavior
  ```rust
  let end_ms = if request.full_decode {
      passage.end_time_ticks
          .map(|t| wkmp_common::timing::ticks_to_ms(t) as u64)
          .unwrap_or(0) // 0 = file end in decoder
  } else { /* ... */ };
  ```
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/audio/decoder.rs` (SimpleDecoder)
- Decoder successfully reads to EOF and returns samples
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/decoder_pool.rs:268`
- Total samples counted after decode completion:
  ```rust
  let total_samples = rt_handle.block_on(async {
      if let Some(buffer_arc) = buffer_manager.get_buffer(passage_id).await {
          let buffer = buffer_arc.lock().await;
          (buffer.stats().total_written * 2) as usize // Convert frames to samples
      } else {
          0
      }
  });
  ```

**What's Missing:**
1. **No pre-decode metadata probing** - Decoder doesn't query symphonia metadata API before decode starts
2. **No endpoint discovery** - Actual file duration never extracted from metadata
3. **No tick conversion** - Discovered sample count not converted back to ticks
4. **No return mechanism** - `decode_passage_internal()` returns `Result<()>`, not endpoint data
5. **No database update** - Discovered endpoints not written back to passages table

**Impact:**
- ✅ Decode completes successfully (decoder reads to EOF)
- ❌ Crossfade timing calculations cannot use actual endpoint (falls back to 10s default)
- ❌ Fade-out applied at wrong position (no lead_out_point data)
- ❌ Database remains inconsistent (NULL endpoints never populated)
- ❌ [DBD-COMP-015] completion detection may fail (no total_frames sentinel)

**Implementation Complexity:** **High**
- Requires symphonia metadata API integration (probe file before decode)
- Need to add endpoint return value to `decode_passage_internal()` function signature
- Must propagate discovered endpoints through worker loop to engine/queue manager
- Database write-back logic needed to persist discovered endpoints
- Error handling for files without duration metadata (corrupted/streaming files)

---

### [DBD-DEC-095] Endpoint Discovery and Return

**Requirement Summary:** Decoder must return discovered endpoint (in ticks) to caller so it can be used for crossfade timing and persisted to database.

**Status:** ❌ **Not Implemented**

**What Exists:**
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/decoder_pool.rs:248-254`
- Worker loop calls `decode_passage_internal()` and gets `Result<()>`
  ```rust
  match Self::decode_passage_internal(
      &request,
      Arc::clone(&buffer_manager),
      &state,
      &rt_handle,
      resume_from_sample,
  ) {
      Ok(()) => { /* finalize buffer */ }
      Err(e) => { /* error handling */ }
  }
  ```
- No mechanism to return discovered endpoint data

**What's Missing:**
1. **Modified return type** - Need `Result<DiscoveredEndpoint>` instead of `Result<()>`
2. **DiscoveredEndpoint struct** - New type to hold discovered endpoint data:
   ```rust
   struct DiscoveredEndpoint {
       end_time_ticks: Option<i64>,
       total_samples: usize,
       duration_ms: Option<u64>,
   }
   ```
3. **Worker loop handling** - Worker must extract endpoint from return value
4. **Queue manager notification** - Worker must notify engine/queue manager of discovered endpoint
5. **Database persistence** - Queue manager must write discovered endpoint to passages table

**Impact:**
- ❌ Crossfade timing calculations use stale/incorrect endpoints
- ❌ Database inconsistency persists (NULL endpoints never updated)
- ❌ User manual edits to passage timing lost on next decode
- ❌ Completion detection unreliable without accurate endpoint data

**Implementation Complexity:** **Medium**
- Change function signature (breaking change to internal API)
- Add new struct definition
- Update worker loop to handle new return type
- Add async notification path from worker to engine
- Database write logic (already exists in queue manager, just needs call site)

---

## Section 2: Buffer-Related Requirements

### [DBD-BUF-065] Buffer Capacity Validation

**Requirement Summary:** BufferManager must validate requested capacity against configured playout_ringbuffer_size before creating ring buffer. Reject operations if capacity exceeds limit.

**Status:** ⚠️ **Partially Implemented**

**What Exists:**
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/buffer_manager.rs:104-132`
- BufferManager creates PlayoutRingBuffer with default capacity:
  ```rust
  let buffer_arc = Arc::new(Mutex::new(PlayoutRingBuffer::new(
      None, // Use default capacity (661,941)
      None, // Use default headroom (441)
      Some(hysteresis), // Use configured resume hysteresis
      Some(queue_entry_id),
  )));
  ```
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/playout_ring_buffer.rs:46-51`
- PlayoutRingBuffer has default constants:
  ```rust
  const DEFAULT_CAPACITY: usize = 661_941;
  const DEFAULT_HEADROOM: usize = 441;
  ```
- TODO comment exists: "// TODO: Read capacity and headroom from settings database" (line 114)

**What's Missing:**
1. **Settings database read** - No code to query `playout_ringbuffer_size` from settings table
2. **Capacity validation** - No check that requested capacity ≤ configured max
3. **Error handling** - No rejection mechanism for over-capacity requests
4. **Configuration loading** - No startup initialization of capacity parameter
5. **Dynamic reconfiguration** - No support for changing capacity without restart

**Impact:**
- ✅ Ring buffers work correctly with hardcoded defaults
- ⚠️ Cannot configure capacity via database (hardcoded to 661,941)
- ❌ No validation against user-configured limits
- ❌ System behavior doesn't match [DBD-PARAM-070] if settings are changed
- ⚠️ Not a critical bug, but inconsistent with design spec

**Implementation Complexity:** **Low**
- Add settings query to BufferManager::new() (database read)
- Store capacity as `Arc<RwLock<usize>>` field in BufferManager
- Pass capacity to PlayoutRingBuffer::new() instead of `None`
- Add validation check in allocate_buffer() (simple comparison)
- Return error if capacity exceeded

---

### [DBD-BUF-050] Decoder Pause/Resume with Hysteresis

**Requirement Summary:** Decoder must pause when buffer fill_level >= (capacity - headroom), resume when fill_level < (capacity - headroom * 2). Hysteresis prevents oscillation.

**Status:** ✅ **Implemented Correctly**

**What Exists:**
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/buffer_manager.rs:351-361`
- `should_decoder_pause()` checks buffer pause condition:
  ```rust
  pub async fn should_decoder_pause(&self, queue_entry_id: Uuid) -> Result<bool, String> {
      let buffers = self.buffers.read().await;
      let managed = buffers.get(&queue_entry_id)
          .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;
      let buffer_arc = Arc::clone(&managed.buffer);
      drop(buffers);
      let buffer = buffer_arc.lock().await;
      Ok(buffer.should_decoder_pause())
  }
  ```
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/buffer_manager.rs:378-389`
- `can_decoder_resume()` checks resume condition with hysteresis:
  ```rust
  pub async fn can_decoder_resume(&self, queue_entry_id: Uuid) -> Option<bool> {
      let buffers = self.buffers.read().await;
      let managed = buffers.get(&queue_entry_id)?;
      let buffer_arc = Arc::clone(&managed.buffer);
      drop(buffers);
      let buffer = buffer_arc.lock().await;
      Some(buffer.can_decoder_resume()) // Uses configurable hysteresis
  }
  ```
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/decoder_pool.rs:444-463`
- Worker loop checks pause condition after each chunk:
  ```rust
  let should_pause = rt_handle.block_on(async {
      buffer_manager.should_decoder_pause(passage_id).await.unwrap_or(false)
  });
  if should_pause {
      let samples_processed = end;
      let mut paused = state.paused_jobs.lock().unwrap();
      paused.insert(passage_id, (request.clone(), samples_processed));
      // ... logging ...
      return Ok(());
  }
  ```
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/decoder_pool.rs:620-690`
- `check_paused_jobs_for_resume()` implements hysteresis-aware resume:
  ```rust
  let can_resume = rt_handle.block_on(async {
      buffer_manager.can_decoder_resume(*passage_id).await.unwrap_or(false)
  });
  ```

**What's Complete:**
- ✅ Pause detection after each 1-second chunk
- ✅ Hysteresis-aware resume check
- ✅ Priority-preserving resume (highest priority resumes first)
- ✅ Paused job tracking via HashMap<Uuid, (DecodeRequest, usize)>
- ✅ Buffer existence validation before resume

**What's Missing:**
- Nothing major - implementation is functionally complete

**Impact:**
- ✅ Decoder pause/resume works correctly
- ✅ Prevents buffer overflow
- ✅ Hysteresis prevents oscillation
- ✅ No critical gaps identified

**Implementation Complexity:** N/A (already implemented)

---

## Section 3: Fade-Related Requirements

### [DBD-FADE-065] Pre-Buffer Fade Application

**Requirement Summary:** Fade curves (fade-in/fade-out) must be applied to samples BEFORE they are pushed to ring buffer, not during mixer playback. Fade unit is part of decode chain, not mixer.

**Status:** ❌ **Not Implemented**

**What Exists:**
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/decoder_pool.rs:415-481`
- Decoder converts samples to stereo, then pushes to ring buffer directly:
  ```rust
  // Convert to stereo if mono
  let stereo_samples = if channels == 1 {
      let mut stereo = Vec::with_capacity(final_samples.len() * 2);
      for sample in final_samples {
          stereo.push(sample);
          stereo.push(sample);
      }
      stereo
  } else { /* ... */ };

  // Append samples in chunks WITH PAUSE CHECKS
  const CHUNK_SIZE: usize = 88200; // 1 second @ 44.1kHz
  for chunk_idx in start_chunk..total_chunks {
      let chunk = &stereo_samples[start..end];
      let frames_pushed = rt_handle.block_on(async {
          buffer_manager.push_samples(passage_id, chunk).await
      });
      // ... pause check ...
  }
  ```
- **No fade application** - Samples pushed to buffer without any amplitude modification

**Where Fades Are Currently Applied:**
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/mixer.rs` (mixer component)
- Fades currently applied DURING PLAYBACK by mixer, not during decode
- This violates SPEC016 design (fades should be pre-applied in decode chain)

**What's Missing:**
1. **Fade calculation logic in decoder** - No code to compute fade-in/fade-out curves
2. **Passage timing data access** - Decoder doesn't access fade_in_point_ticks, fade_out_point_ticks
3. **Per-chunk fade application** - No multiplication of samples by fade envelope
4. **Fade curve implementation** - Need exponential/logarithmic/linear/cosine curves
5. **Position tracking for fades** - Need to know sample position relative to fade regions

**Implementation Requirements:**
```rust
// Pseudocode for missing fade application
fn apply_passage_fades(
    samples: &mut [f32],
    passage_timing: &PassageWithTiming,
    current_position_ticks: i64,
) {
    let fade_in_start = passage_timing.fade_in_point_ticks;
    let fade_in_end = passage_timing.fade_in_point_ticks + fade_in_duration;
    let fade_out_start = passage_timing.fade_out_point_ticks.unwrap_or(0);
    let fade_out_end = passage_timing.end_time_ticks.unwrap_or(0);

    for (i, sample) in samples.iter_mut().enumerate() {
        let sample_position = current_position_ticks + (i as i64 * tick_resolution);

        if sample_position >= fade_in_start && sample_position < fade_in_end {
            // Apply fade-in curve
            let fade_in_progress = (sample_position - fade_in_start) as f32 / fade_in_duration as f32;
            let fade_mult = passage_timing.fade_in_curve.calculate_fade_in(fade_in_progress);
            *sample *= fade_mult;
        } else if sample_position >= fade_out_start && sample_position < fade_out_end {
            // Apply fade-out curve
            let fade_out_progress = (sample_position - fade_out_start) as f32 / fade_out_duration as f32;
            let fade_mult = passage_timing.fade_out_curve.calculate_fade_out(fade_out_progress);
            *sample *= fade_mult;
        }
        // else: pass through (no fade applied)
    }
}
```

**Impact:**
- ⚠️ System works but violates SPEC016 architecture
- ⚠️ Fades applied in mixer (wrong location per spec)
- ❌ Cannot pre-compute faded buffers
- ❌ Mixer must re-apply fades every playback (inefficient)
- ❌ Ring buffer draining concept incomplete (buffers not "ready-to-play")
- ⚠️ Not a critical bug, but significant architectural deviation

**Implementation Complexity:** **High**
- Need to refactor fade logic from mixer to decoder
- Must handle chunk-based processing (fades span multiple chunks)
- Position tracking across pause/resume cycles
- Fade curve API integration (FadeCurve enum already exists in wkmp_common)
- Requires coordination between decoder and passage timing data
- Need to preserve existing mixer fade logic temporarily during migration

**Migration Risk:** **High**
- Moving fades from mixer to decoder is breaking change
- May affect crossfade behavior (mixer expects faded samples)
- Requires careful testing to ensure no audio artifacts
- Recommend phased approach: implement pre-buffer fades first, test, then remove mixer fades

---

## Section 4: Completion State Integration

### [DBD-COMP-015] Completion State Machine Integration

**Requirement Summary:** Buffer exhaustion detection and passage completion must integrate with buffer state machine. Buffer state transitions must trigger mixer state updates.

**Status:** ⚠️ **Partially Implemented**

**What Exists:**
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/buffer_manager.rs:394-435`
- `finalize_buffer()` transitions to Finished state:
  ```rust
  pub async fn finalize_buffer(&self, queue_entry_id: Uuid, total_samples: usize) -> Result<(), String> {
      let mut buffers = self.buffers.write().await;
      let managed = buffers.get_mut(&queue_entry_id)
          .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;
      let old_state = managed.metadata.state;

      // Set total samples
      managed.metadata.total_samples = Some(total_samples);

      // Transition to Finished
      if old_state != BufferState::Finished {
          managed.metadata.state = BufferState::Finished;
          self.emit_event(BufferEvent::StateChanged {
              queue_entry_id,
              old_state,
              new_state: BufferState::Finished,
              samples_buffered: total_samples,
          }).await;
          self.emit_event(BufferEvent::Finished {
              queue_entry_id,
              total_samples,
          }).await;
      }

      // Mark decode complete on ring buffer
      let buffer = Arc::clone(&managed.buffer);
      drop(buffers);
      let mut buf = buffer.lock().await;
      buf.mark_decode_complete(); // Sets decode_complete flag
      Ok(())
  }
  ```
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/buffer_manager.rs:707-716`
- `is_buffer_exhausted()` checks both decode complete AND buffer drained:
  ```rust
  pub async fn is_buffer_exhausted(&self, queue_entry_id: Uuid) -> Option<bool> {
      let buffers = self.buffers.read().await;
      let managed = buffers.get(&queue_entry_id)?;
      let buffer_arc = Arc::clone(&managed.buffer);
      drop(buffers);
      let buffer = buffer_arc.lock().await;
      Some(buffer.is_exhausted()) // Checks: decode_complete && occupied() == 0
  }
  ```
- **File:** `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/playout_ring_buffer.rs`
- Ring buffer has `mark_decode_complete()` and `is_exhausted()` methods

**What's Missing:**
1. **Mixer integration** - Mixer doesn't actively monitor buffer state changes
2. **Event subscription** - Mixer doesn't subscribe to BufferEvent::Finished
3. **Automatic completion detection** - Mixer relies on manual checks, not event-driven
4. **Crossfade trigger on exhaustion** - No automatic crossfade when buffer exhausted
5. **Total_frames sentinel** - PassageBuffer legacy type still used, no total_frames field
6. **Position comparison** - Mixer doesn't compare current position to total_frames

**What Works:**
- ✅ Buffer state machine tracks Finished state correctly
- ✅ BufferEvent::Finished emitted when decode completes
- ✅ is_buffer_exhausted() correctly checks decode_complete + occupied == 0
- ✅ Ring buffer properly signals exhaustion

**What Doesn't Work:**
- ❌ Mixer doesn't react to BufferEvent::Finished (events ignored)
- ❌ Completion detection relies on polling, not events
- ❌ Crossfade may start late (no proactive exhaustion notification)

**Impact:**
- ⚠️ System works but isn't event-driven (polls instead of reacts)
- ⚠️ Completion detection has ~90ms latency (output_refill_period)
- ❌ BufferEvent infrastructure underutilized
- ⚠️ Not a critical bug, but suboptimal architecture

**Implementation Complexity:** **Medium**
- Need to wire BufferEvent channel to mixer
- Mixer must subscribe to Finished events
- Add event handler in mixer to trigger crossfade or completion
- May require refactoring mixer state machine to accept external events
- Test interaction between event-driven and polling paths

---

## Section 5: Quick Wins

These are **low-complexity, high-impact** fixes that should be prioritized first:

### 1. [DBD-BUF-065] Add Settings Database Read for Buffer Capacity
**Complexity:** Low
**Impact:** High (enables configurable buffer sizes per spec)
**Files:** `buffer_manager.rs:114`
**Effort:** 1-2 hours

**Implementation:**
```rust
// In BufferManager::new()
pub async fn new(db_pool: Pool<Sqlite>) -> Self {
    // Read capacity from settings
    let capacity = sqlx::query!("SELECT value FROM settings WHERE key = 'playout_ringbuffer_size'")
        .fetch_optional(&db_pool)
        .await
        .ok()
        .flatten()
        .and_then(|row| row.value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_CAPACITY);

    Self {
        buffers: Arc::new(RwLock::new(HashMap::new())),
        capacity: Arc::new(RwLock::new(capacity)),
        // ... rest of fields ...
    }
}
```

### 2. [DBD-COMP-015] Wire BufferEvent::Finished to Mixer
**Complexity:** Low
**Impact:** Medium (improves responsiveness)
**Files:** `mixer.rs`, `engine.rs`
**Effort:** 2-3 hours

**Implementation:**
- Add BufferEvent channel receiver to CrossfadeMixer
- Subscribe to Finished events in mixer initialization
- Add event handler to trigger crossfade when Finished received
- Test crossfade timing with event-driven vs polling

### 3. Remove Broken Change Detection in BufferChainStatus Emitter
**Complexity:** Trivial
**Impact:** High (fixes frozen buffer display issue)
**Files:** `engine.rs:2294-2297`
**Effort:** 15 minutes

**Implementation:**
- Delete lines 2279, 2293-2297 (change detection logic)
- Always emit BufferChainStatus every 1 second
- Update comment to reflect unconditional emission

**Reference:** See `/home/sw/Dev/McRhythm/docs/validation/buffer_fill_display_issue_analysis.md` for full analysis of this issue.

---

## Section 6: Critical Path Dependencies

These requirements **block** other work and must be implemented in order:

### Phase 1: Endpoint Discovery (Foundational)
**Requirements:** [DBD-DEC-090], [DBD-DEC-095]
**Why Critical:** Crossfade timing, completion detection, and fade positioning all depend on accurate endpoints
**Blocks:** Fade application ([DBD-FADE-065]), completion detection refinement

### Phase 2: Pre-Buffer Fade Application
**Requirements:** [DBD-FADE-065]
**Why Critical:** Ring buffer architecture incomplete without pre-applied fades
**Blocks:** Mixer drain refactoring (can't drain if fades applied during playback)

### Phase 3: Completion State Integration
**Requirements:** [DBD-COMP-015]
**Why Critical:** Enables event-driven playback flow
**Blocks:** None (enhancement to existing functionality)

### Phase 4: Buffer Capacity Configuration
**Requirements:** [DBD-BUF-065]
**Why Critical:** Low priority (system works with defaults)
**Blocks:** None

---

## Section 7: Risk Areas

These changes have **high risk of breaking existing functionality**:

### 1. Moving Fades from Mixer to Decoder
**Risk Level:** **HIGH**
**Reason:** Crossfading assumes mixer applies fades; pre-applied fades may interact incorrectly
**Mitigation:**
- Implement fade-in-decoder with feature flag
- Run parallel fade systems temporarily (decoder + mixer)
- Verify audio output identical before removing mixer fades
- Extensive A/B testing with different fade curves

### 2. Changing decode_passage_internal() Return Type
**Risk Level:** **MEDIUM**
**Reason:** Breaking change to internal API, affects worker loop and error handling
**Mitigation:**
- Update all call sites atomically
- Add integration tests for endpoint return
- Verify error propagation still works

### 3. Wiring Events to Mixer
**Risk Level:** **MEDIUM**
**Reason:** Mixer state machine tightly coupled to polling; events may cause race conditions
**Mitigation:**
- Add event handling as parallel path (don't remove polling initially)
- Test event timing vs polling timing
- Verify no duplicate crossfades triggered

---

## Section 8: Implementation Priority

Recommended order based on dependencies, complexity, and impact:

### Immediate (Week 1)
1. **[QUICK WIN]** Remove BufferChainStatus change detection (15 min)
2. **[QUICK WIN]** Add settings read for buffer capacity (2 hours)
3. **[CRITICAL PATH]** Implement [DBD-DEC-090] endpoint probing (8-12 hours)

### Short-term (Week 2-3)
4. **[CRITICAL PATH]** Implement [DBD-DEC-095] endpoint return (6-8 hours)
5. **[QUICK WIN]** Wire BufferEvent::Finished to mixer (3 hours)
6. **[CRITICAL PATH]** Implement [DBD-FADE-065] pre-buffer fades (16-20 hours)

### Medium-term (Week 4+)
7. **[ENHANCEMENT]** Complete [DBD-COMP-015] event-driven completion (6-8 hours)
8. **[CLEANUP]** Remove mixer fade logic (4 hours)
9. **[TESTING]** Comprehensive integration tests (8-12 hours)

**Total Estimated Effort:** 53-83 hours (6-10 working days)

---

## Section 9: Testing Strategy

Each gap closure requires specific testing:

### [DBD-DEC-090] Endpoint Discovery Testing
- **Unit:** Mock symphonia probe, verify tick conversion
- **Integration:** Decode file with unknown endpoint, verify discovered value
- **Regression:** Decode file with known endpoint, verify unchanged behavior

### [DBD-DEC-095] Endpoint Return Testing
- **Unit:** Verify DiscoveredEndpoint struct serialization
- **Integration:** Decode file, verify engine receives endpoint
- **Database:** Verify discovered endpoint written to passages table

### [DBD-BUF-065] Capacity Validation Testing
- **Unit:** Test capacity validation logic (over/under limit)
- **Integration:** Configure capacity via settings, verify buffer creation
- **Error:** Request over-capacity buffer, verify rejection

### [DBD-FADE-065] Pre-Buffer Fade Testing
- **Unit:** Test fade curve calculation for each curve type
- **Audio:** Compare pre-faded vs mixer-faded output (should be identical)
- **Regression:** Verify crossfades still work with pre-applied fades

### [DBD-COMP-015] Completion State Testing
- **Unit:** Test event emission timing
- **Integration:** Verify mixer reacts to Finished events
- **Regression:** Verify polling path still works

---

## Section 10: Acceptance Criteria

Implementation is complete when:

### [DBD-DEC-090] File Duration Probing
- [ ] Decoder calls symphonia metadata probe before decode
- [ ] Discovered duration converted to ticks correctly
- [ ] NULL endpoints replaced with discovered values in passages table
- [ ] Error handling for files without metadata
- [ ] Integration test: decode file with NULL endpoint, verify database updated

### [DBD-DEC-095] Endpoint Discovery Return
- [ ] DiscoveredEndpoint struct defined and documented
- [ ] decode_passage_internal() returns Result<DiscoveredEndpoint>
- [ ] Worker loop extracts and logs discovered endpoints
- [ ] Engine/queue manager receives endpoint notifications
- [ ] Integration test: verify endpoint flows from decoder to database

### [DBD-BUF-065] Buffer Capacity Validation
- [ ] BufferManager reads playout_ringbuffer_size from settings on startup
- [ ] allocate_buffer() validates capacity against configured limit
- [ ] Over-capacity requests return error, not panic
- [ ] Unit test: verify validation logic
- [ ] Integration test: change setting, restart, verify new capacity used

### [DBD-FADE-065] Pre-Buffer Fade Application
- [ ] Fade calculation logic exists in decoder_pool.rs
- [ ] Fades applied to chunks before push_samples()
- [ ] All 5 fade curves (linear, exponential, logarithmic, cosine, constant) implemented
- [ ] Position tracking across pause/resume cycles
- [ ] Audio test: pre-faded output identical to mixer-faded output
- [ ] Mixer fade logic removed (after verification)

### [DBD-COMP-015] Completion State Integration
- [ ] Mixer subscribes to BufferEvent::Finished
- [ ] Finished event triggers crossfade or completion
- [ ] Polling path still works (parallel implementation)
- [ ] Event-driven path tested under load
- [ ] Latency improvement measured (should be < 90ms)

---

## Section 11: Known Limitations

### Current System Limitations (Not Gaps)
These are intentional design decisions or accepted limitations:

1. **Re-decode on resume** - Paused jobs re-decode entire file, skip already-processed samples
   - Not a bug: documented as "optimize later" in pause/resume design
   - Impact: ~10-50ms overhead per resume (negligible)

2. **Hardcoded chunk size** - 1-second chunks (88,200 samples) not configurable
   - Not a gap: [DBD-PARAM-060] defines decode_work_period (5s), not chunk size
   - Impact: None (1s chunks work well for pause detection)

3. **PassageBuffer type still exists** - Legacy Vec<f32> buffer code not removed
   - Not a gap: kept for reference during migration
   - Impact: None (unused, can be deleted in cleanup phase)

4. **Mixer reads by drain, not by index** - Current mixer may still use position tracking
   - Not a gap if drain-based design not yet approved
   - Impact: See mixer_drain_refactoring_design.md for full analysis

---

## Section 12: Out of Scope

These are **NOT** implementation gaps (either working correctly or not required):

### ✅ Working Correctly
- Decoder pause/resume logic ([DBD-BUF-050]) - **IMPLEMENTED**
- Ring buffer push/pop operations ([DBD-BUF-010]) - **IMPLEMENTED**
- Buffer state machine ([DBD-BUF-020] through [DBD-BUF-060]) - **IMPLEMENTED**
- Hysteresis-aware resume ([DBD-BUF-050]) - **IMPLEMENTED**
- Priority-based decode scheduling ([DBD-DEC-040]) - **IMPLEMENTED**

### ❌ Not Required for Current Phase
- Mixer drain refactoring - **DESIGN ONLY** (not approved for implementation)
- Segment-based memory freeing - **FUTURE ENHANCEMENT**
- Incremental decoder state preservation - **FUTURE OPTIMIZATION**
- Preemptive priority switching - **FUTURE ENHANCEMENT**

---

## Appendix A: File-by-File Gap Summary

### `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/decoder_pool.rs`
**Lines:** 312-484 (decode_passage_internal)
**Gaps:**
- Missing endpoint discovery ([DBD-DEC-090])
- Missing endpoint return ([DBD-DEC-095])
- Missing fade application ([DBD-FADE-065])
**Complexity:** High (requires symphonia integration + fade logic)

### `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/buffer_manager.rs`
**Lines:** 104-132 (allocate_buffer)
**Gaps:**
- Missing settings database read ([DBD-BUF-065])
- Missing capacity validation ([DBD-BUF-065])
**Complexity:** Low (simple database query + validation)

### `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/pipeline/mixer.rs`
**Lines:** Multiple (event subscription, completion detection)
**Gaps:**
- Missing BufferEvent subscription ([DBD-COMP-015])
- Missing event-driven completion ([DBD-COMP-015])
**Complexity:** Medium (requires event channel wiring)

### `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/engine.rs`
**Lines:** 2294-2297 (buffer_chain_status_emitter)
**Gaps:**
- Broken f32 change detection (buffer display freezing issue)
**Complexity:** Trivial (delete 4 lines)

### `/home/sw/Dev/McRhythm/wkmp-ap/src/playback/playout_ring_buffer.rs`
**Lines:** N/A
**Gaps:** None (ring buffer implementation complete)

---

## Appendix B: Requirement Traceability Matrix

| Requirement ID | Specification Section | Implementation Status | File | Line Range | Gap Severity |
|----------------|----------------------|----------------------|------|------------|--------------|
| DBD-DEC-090 | Endpoint Probing | ❌ Not Implemented | decoder_pool.rs | 325-330 | High |
| DBD-DEC-095 | Endpoint Return | ❌ Not Implemented | decoder_pool.rs | 248-254 | High |
| DBD-BUF-065 | Capacity Validation | ⚠️ Partial | buffer_manager.rs | 104-132 | Medium |
| DBD-BUF-050 | Pause/Resume | ✅ Implemented | decoder_pool.rs | 444-463, 620-690 | None |
| DBD-FADE-065 | Pre-Buffer Fades | ❌ Not Implemented | decoder_pool.rs | 415-481 | High |
| DBD-COMP-015 | Completion Events | ⚠️ Partial | mixer.rs, buffer_manager.rs | Multiple | Medium |
| DBD-PARAM-070 | Buffer Capacity | ⚠️ Hardcoded | buffer_manager.rs | 114 | Low |

**Legend:**
- ✅ Implemented: Fully functional, meets specification
- ⚠️ Partial: Core logic exists but missing pieces
- ❌ Not Implemented: No code exists for this requirement

---

## Appendix C: Related Documentation

**Specifications:**
- `/home/sw/Dev/McRhythm/docs/SPEC016-decoder_buffer_design.md` - Main decoder/buffer spec
- `/home/sw/Dev/McRhythm/docs/validation/decoder_pause_resume_design.md` - Pause/resume design
- `/home/sw/Dev/McRhythm/docs/validation/ring_buffer_refactoring_plan.md` - Ring buffer migration
- `/home/sw/Dev/McRhythm/docs/validation/mixer_drain_refactoring_design.md` - Mixer drain design (not approved)

**Implementation Plans:**
- `/home/sw/Dev/McRhythm/docs/validation/test_migration_plan.md` - Test migration strategy

**Issue Analysis:**
- `/home/sw/Dev/McRhythm/docs/validation/buffer_fill_display_issue_analysis.md` - Display freezing issue

---

**Document Status:** Gap Analysis Complete - Ready for Technical Review
**Next Steps:**
1. Review gap analysis with technical lead
2. Approve priority order (Section 8)
3. Begin Phase 1 implementation (Quick Wins + Critical Path)
4. Create detailed implementation tasks for [DBD-DEC-090] (highest priority)

**Estimated Total Effort:** 53-83 hours (7-10 working days for full implementation)
