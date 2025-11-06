# Skip Forward Feature Design

**Feature:** 10-Second Skip Forward Button for wkmp-ap UI

**Author:** Claude Code
**Date:** 2025-06-11
**Status:** Draft Design Document

---

## 1. Executive Summary

This document specifies a new "skip forward 10 seconds" feature for the wkmp-ap audio player UI. The feature adds a playback control button that advances playback by exactly 10 seconds within the currently playing passage, subject to buffer availability constraints.

**Key Design Principles:**
- **Safety-First:** Only skip if ≥11 seconds of audio frames available in buffer (prevents underrun)
- **Sample-Accurate:** Uses existing tick-based positioning system (preserves WKMP's 0.02ms precision)
- **Risk-First Framework:** Leverages proven `seek()` architecture to minimize implementation risk
- **UI/UX Consistency:** Follows existing playback control patterns (play/pause/skip_next)

---

## 2. Requirements

### 2.1 Functional Requirements

**[REQ-SF-010]** **Skip Forward Button**
- UI MUST display a "skip forward" button in the playback controls section
- Button MUST be visually consistent with existing play/pause/skip buttons
- Button position: between "play/pause" and "skip next" buttons

**[REQ-SF-020]** **10-Second Skip Logic**
- When clicked, button MUST advance playback position by exactly 10.0 seconds
- Skip MUST be sample-accurate (uses tick-based positioning, not millisecond rounding)
- Skip MUST respect passage boundaries (cannot skip past end of current passage)

**[REQ-SF-030]** **Buffer Availability Validation** *(CRITICAL - Safety Requirement)*
- Backend MUST validate that ≥11 seconds (11000ms) of audio frames are available in the current passage's playout ring buffer
- If insufficient buffer available, API MUST return HTTP 400 error with descriptive message
- Frontend MUST disable button when buffer insufficient (proactive UX)
- Frontend MUST display error toast if skip fails due to buffer constraint

**[REQ-SF-040]** **Playback State Requirements**
- Skip forward MUST work in both Playing and Paused states
- Skip forward MUST NOT work when no passage is playing (return HTTP 400)
- After skip, playback state MUST remain unchanged (Playing→Playing, Paused→Paused)

**[REQ-SF-050]** **Event Emission**
- Backend MUST emit `PlaybackProgress` SSE event with updated position after skip
- Event MUST include accurate position_ms, duration_ms, passage_id

### 2.2 Non-Functional Requirements

**[REQ-SF-NF-010]** **Performance**
- API response time: <50ms (skip operation is lightweight - just position update)
- UI button response: <100ms (matches existing playback controls)
- No audio gaps or glitches during skip

**[REQ-SF-NF-020]** **Reliability**
- Buffer validation MUST be atomic (check-then-act race condition prevented)
- Skip operation MUST be idempotent (multiple rapid clicks handled gracefully)

**[REQ-SF-NF-030]** **Maintainability**
- Reuse existing `seek()` backend logic (minimize code duplication)
- Follow WKMP architectural patterns (tick-based timing, event-driven updates)

---

## 3. Technical Design

### 3.1 Architecture Overview

```text
┌─────────────────────────────────────────────────────────────┐
│ Frontend (wkmp-ui)                                          │
│                                                             │
│  Playback Controls:                                         │
│  [ ◀◀ Prev ] [ ▶️/⏸️ Play/Pause ] [ ⏭️+10s Skip ] [ ⏭️ Next ]│
│                                    ^                         │
│                                    │ onClick()               │
│                                    ↓                         │
│  POST /playback/skip-forward → API Handler                  │
└─────────────────────────────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ Backend (wkmp-ap)                                           │
│                                                             │
│  API Handler: skip_forward()                                │
│  │                                                          │
│  ├─ 1. Get current queue entry                             │
│  ├─ 2. Get playout ring buffer for passage                 │
│  ├─ 3. Validate buffer availability:                        │
│  │     if buffer.occupied() < 11s_in_frames                │
│  │        return HTTP 400 (insufficient buffer)             │
│  ├─ 4. Calculate new position:                             │
│  │     new_pos = current_tick + (10s * sample_rate)        │
│  ├─ 5. Call engine.seek(new_pos_ms)                        │
│  │     (reuses existing seek logic)                         │
│  └─ 6. Return HTTP 200                                     │
│                                                             │
│  PlaybackEngine::seek()                                     │
│  │                                                          │
│  ├─ Validate passage/buffer exists                         │
│  ├─ Convert ms → frames (sample_rate)                      │
│  ├─ Clamp to buffer bounds (max = total_written)           │
│  ├─ Update mixer.set_current_passage(passage_id, seek_tick)│
│  └─ Emit PlaybackProgress SSE event                        │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Backend Implementation

#### 3.2.1 API Endpoint

**Route:** `POST /playback/skip-forward`
**Handler:** `handlers::skip_forward()`
**Request Body:** None (always skips 10 seconds)
**Response:**
- **200 OK:** Skip successful
  ```json
  {
    "status": "skipped forward 10 seconds",
    "new_position_ms": 45320
  }
  ```
- **400 Bad Request:** Cannot skip (insufficient buffer, no passage playing)
  ```json
  {
    "status": "error: insufficient buffer - only 8.5s available (need 11s)"
  }
  ```
- **500 Internal Server Error:** Unexpected error

#### 3.2.2 Handler Implementation

**Location:** `wkmp-ap/src/api/handlers.rs`

```rust
/// POST /playback/skip-forward - Skip forward 10 seconds
///
/// **Traceability:** [REQ-SF-010] through [REQ-SF-050]
/// **Requirements:**
/// - [REQ-SF-020] 10-second skip with sample accuracy
/// - [REQ-SF-030] Buffer availability validation (≥11s required)
/// - [REQ-SF-040] Works in Playing/Paused states
/// - [REQ-SF-050] Emits PlaybackProgress event
///
/// # Safety
/// This endpoint validates buffer availability BEFORE seeking to prevent underruns.
/// The 11-second requirement (1 second margin beyond 10s skip) ensures audio pipeline
/// has sufficient data during position transition.
pub async fn skip_forward(
    State(ctx): State<AppContext>,
) -> Result<Json<SkipForwardResponse>, (StatusCode, Json<StatusResponse>)> {
    info!("Skip forward request (10 seconds)");

    // Step 1: Get current queue entry
    let queue = ctx.engine.queue.read().await;
    let current = queue.current().cloned();
    drop(queue);

    let current = match current {
        Some(c) => c,
        None => {
            warn!("Skip forward failed: no passage currently playing");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(StatusResponse {
                    status: "error: no passage currently playing".to_string(),
                }),
            ));
        }
    };

    // Step 2: Get buffer for current passage
    let buffer_ref = ctx.engine.buffer_manager.get_buffer(current.queue_entry_id).await;
    let buffer = match buffer_ref {
        Some(b) => b,
        None => {
            warn!("Skip forward failed: buffer not available for queue_entry_id={}", current.queue_entry_id);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StatusResponse {
                    status: "error: buffer not available".to_string(),
                }),
            ));
        }
    };

    // Step 3: Validate buffer availability
    // [REQ-SF-030] Require ≥11 seconds of buffered audio (10s skip + 1s safety margin)
    let sample_rate = *ctx.engine.working_sample_rate.read().unwrap();
    let required_frames = ((11000 * sample_rate as u64) / 1000) as usize; // 11 seconds in frames
    let available_frames = buffer.occupied();

    if available_frames < required_frames {
        let available_seconds = (available_frames as f64) / (sample_rate as f64);
        let msg = format!(
            "error: insufficient buffer - only {:.1}s available (need 11s)",
            available_seconds
        );
        warn!("Skip forward failed: {}", msg);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(StatusResponse { status: msg }),
        ));
    }

    // Step 4: Get current playback position
    let mixer = ctx.engine.mixer.read().await;
    let current_tick = mixer.get_current_tick();
    drop(mixer);

    // Step 5: Calculate new position (10 seconds forward)
    // Convert ticks → ms → add 10s → pass to seek()
    let current_position_ms = wkmp_common::timing::ticks_to_ms(current_tick, sample_rate);
    let new_position_ms = current_position_ms + 10000; // Add 10 seconds

    // Step 6: Call existing seek() logic (reuse validation, clamping, events)
    match ctx.engine.seek(new_position_ms).await {
        Ok(_) => {
            info!("Skip forward successful: {}ms → {}ms", current_position_ms, new_position_ms);
            Ok(Json(SkipForwardResponse {
                status: "skipped forward 10 seconds".to_string(),
                new_position_ms,
            }))
        }
        Err(e) => {
            error!("Skip forward seek failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StatusResponse {
                    status: format!("error: {}", e),
                }),
            ))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SkipForwardResponse {
    status: String,
    new_position_ms: u64,
}
```

#### 3.2.3 Route Registration

**Location:** `wkmp-ap/src/api/server.rs`

```rust
// In build_router():
.route("/playback/skip-forward", post(handlers::skip_forward))
```

**Placement:** After `/playback/seek`, before `/playback/next`

### 3.3 Frontend Implementation

#### 3.3.1 UI Button Component

**Location:** `wkmp-ui/src/components/PlaybackControls.tsx` (or equivalent)

**Button Specifications:**
- **Icon:** `⏭️+10s` or custom SVG (forward arrow with "10s" text)
- **Position:** Between "Play/Pause" and "Skip Next" buttons
- **Tooltip:** "Skip forward 10 seconds"
- **Disabled State:** When buffer < 11s or no passage playing
- **Loading State:** Show spinner during API call

**Example JSX:**
```jsx
<button
  onClick={handleSkipForward}
  disabled={!canSkipForward}
  className="playback-control-button"
  title="Skip forward 10 seconds"
  aria-label="Skip forward 10 seconds"
>
  {isSkipping ? <Spinner /> : <SkipForwardIcon />}
  <span className="skip-label">+10s</span>
</button>
```

#### 3.3.2 Button State Logic

**Button should be disabled when:**
1. No passage is currently playing (`current_passage_id === null`)
2. Buffer availability < 11 seconds (checked via SSE `BufferChainStatus` events)
3. Currently processing skip request (`isSkipping === true`)

**Buffer validation logic:**
```typescript
// Subscribe to SSE BufferChainStatus events
const canSkipForward = useMemo(() => {
  if (!currentPassageId) return false;
  if (isSkipping) return false;

  // Find current passage's buffer chain
  const chain = bufferChains.find(c => c.passage_id === currentPassageId);
  if (!chain) return false;

  // Check if ≥11s buffered (buffer_duration_ms from BufferChainInfo)
  return (chain.buffer_duration_ms || 0) >= 11000;
}, [currentPassageId, bufferChains, isSkipping]);
```

#### 3.3.3 API Integration

**Click Handler:**
```typescript
const handleSkipForward = async () => {
  setIsSkipping(true);

  try {
    const response = await fetch('http://localhost:5721/playback/skip-forward', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
    });

    if (!response.ok) {
      const error = await response.json();
      showToast('error', error.status);
      return;
    }

    const result = await response.json();
    showToast('success', `Skipped to ${formatTime(result.new_position_ms)}`);
  } catch (error) {
    console.error('Skip forward failed:', error);
    showToast('error', 'Failed to skip forward');
  } finally {
    setIsSkipping(false);
  }
};
```

---

## 4. Risk Analysis

### 4.1 Primary Risks

**Risk 1: Buffer Underrun During Skip**
- **Failure Mode:** User skips forward when buffer nearly exhausted → audio gaps/stutters
- **Probability:** Low (11-second requirement provides 1s safety margin)
- **Impact:** Medium (audible glitch, poor UX)
- **Mitigation:**
  - **[REQ-SF-030]** Enforce ≥11s buffer validation in backend (MANDATORY)
  - Frontend disables button when buffer < 11s (proactive UX)
  - Reuse `seek()` clamping logic (prevents out-of-bounds access)
- **Residual Risk:** Low

**Risk 2: Race Condition (Buffer Check vs. Skip)**
- **Failure Mode:** Buffer check passes → decoder pauses → skip executes on depleted buffer
- **Probability:** Very Low (11s margin + hysteresis makes simultaneous exhaustion unlikely)
- **Impact:** Medium (potential underrun)
- **Mitigation:**
  - Use `buffer.occupied()` which reads atomic fill_level counter (no TOCTOU gap)
  - Mixer handles underruns gracefully (returns last valid frame per [DBD-BUF-030])
  - 11-second margin provides time buffer (decoder refills during transition)
- **Residual Risk:** Low

**Risk 3: UI Button State Desync**
- **Failure Mode:** Button enabled when it shouldn't be (buffer depleted, no passage)
- **Probability:** Low (SSE events update state real-time)
- **Impact:** Low (API returns 400, user sees error toast)
- **Mitigation:**
  - Frontend subscribes to `BufferChainStatus` SSE events (1s update rate)
  - Backend validation is authoritative (frontend state is advisory)
  - Error toasts provide clear feedback when skip fails
- **Residual Risk:** Low

### 4.2 Secondary Risks

**Risk 4: Excessive Skip Requests (User Spam Clicks)**
- **Failure Mode:** User rapidly clicks button → queue of concurrent API requests
- **Probability:** Medium (no rate limiting on client)
- **Impact:** Low (idempotent operation, last request wins)
- **Mitigation:**
  - `isSkipping` state prevents concurrent requests
  - Backend `seek()` is idempotent (position updates are stateless)
- **Residual Risk:** Very Low

**Risk 5: Skip Past Passage End**
- **Failure Mode:** Skip would exceed passage duration → undefined behavior
- **Probability:** Very Low (seek() clamps to buffer bounds)
- **Impact:** Low (clamped to passage end, no crash)
- **Mitigation:**
  - Reuse `seek()` clamping logic:
    ```rust
    let clamped_position = position_frames.min(max_frames.saturating_sub(1));
    ```
  - If skip lands at passage end, mixer detects EOF and advances to next
- **Residual Risk:** Very Low

---

## 5. Implementation Plan

### 5.1 Phase 1: Backend API (2-3 hours)

1. **Add response type** to `handlers.rs` (5 min)
   - `SkipForwardResponse { status, new_position_ms }`

2. **Implement `skip_forward()` handler** in `handlers.rs` (1 hour)
   - Get current queue entry
   - Get buffer and validate ≥11s availability
   - Calculate new position (current_tick + 10s)
   - Call `ctx.engine.seek(new_pos_ms)`
   - Return success/error response

3. **Register route** in `server.rs` (5 min)
   - `.route("/playback/skip-forward", post(handlers::skip_forward))`

4. **Testing** (1 hour)
   - Unit test: Handler logic (mock dependencies)
   - Integration test: Full flow (real buffer, seek validation)
   - Edge cases: No passage, insufficient buffer, skip past end

### 5.2 Phase 2: Frontend UI (2-3 hours)

1. **Create button component** (30 min)
   - Icon/label design
   - Styling (match existing controls)
   - Accessibility (aria-label, keyboard support)

2. **Implement state logic** (1 hour)
   - Subscribe to BufferChainStatus SSE
   - Calculate `canSkipForward` (buffer ≥11s, passage playing)
   - Handle loading state (`isSkipping`)

3. **Integrate API call** (30 min)
   - `handleSkipForward()` click handler
   - Error handling (400/500 responses)
   - Success toast (show new position)

4. **Testing** (1 hour)
   - Manual testing: Click button, verify skip
   - Edge cases: Rapid clicks, buffer depletion, no passage
   - Cross-browser compatibility

### 5.3 Phase 3: Documentation & Polish (1 hour)

1. **Update API documentation** (30 min)
   - Add `/playback/skip-forward` endpoint to SPEC007 (API Design)
   - Document request/response schema
   - Add buffer validation requirement to docs

2. **User-facing documentation** (30 min)
   - Update user manual (if applicable)
   - Add keyboard shortcut (future enhancement: Shift+Right)

---

## 6. Testing Strategy

### 6.1 Backend Tests

**Test 1: Successful Skip (Happy Path)**
- Setup: Enqueue passage, fill buffer to 20s
- Action: POST `/playback/skip-forward`
- Assert:
  - HTTP 200 response
  - `new_position_ms = old_position_ms + 10000`
  - PlaybackProgress SSE event emitted

**Test 2: Insufficient Buffer**
- Setup: Enqueue passage, fill buffer to 5s
- Action: POST `/playback/skip-forward`
- Assert:
  - HTTP 400 response
  - Error message includes "insufficient buffer"

**Test 3: No Passage Playing**
- Setup: Empty queue
- Action: POST `/playback/skip-forward`
- Assert:
  - HTTP 400 response
  - Error message: "no passage currently playing"

**Test 4: Skip Near End of Passage**
- Setup: Passage with 30s duration, current position = 25s
- Action: POST `/playback/skip-forward`
- Assert:
  - HTTP 200 (clamped to passage end)
  - Position does not exceed passage duration

### 6.2 Frontend Tests

**Test 5: Button Disabled States**
- Assert button disabled when:
  1. No passage playing
  2. Buffer < 11s
  3. `isSkipping === true`

**Test 6: Button Enabled State**
- Setup: Passage playing, buffer = 15s
- Assert: Button enabled and clickable

**Test 7: API Error Handling**
- Mock API to return 400
- Click button
- Assert: Error toast displayed, button re-enabled

---

## 7. Future Enhancements

These are out-of-scope for initial implementation but documented for future reference:

1. **Configurable Skip Duration**
   - Allow user to set skip duration (5s, 10s, 15s, 30s)
   - Stored in database settings table

2. **Keyboard Shortcut**
   - Shift+Right Arrow = skip forward 10s
   - Matches media player conventions (VLC, YouTube)

3. **Skip Backward Feature**
   - Mirror implementation: skip backward 10s
   - More complex (may require decoder re-seek if buffer flushed)

4. **Visual Progress Indicator**
   - Show "ghosted" position preview on seek bar during skip
   - Animate position change (smooth transition)

5. **Adaptive Buffer Requirement**
   - Reduce 11s requirement to 6s when buffer refill rate is high
   - Monitor decoder throughput and adjust dynamically

---

## 8. Traceability Matrix

| Requirement | Implementation | Test |
|-------------|----------------|------|
| [REQ-SF-010] Button in UI | Frontend: `PlaybackControls` component | Manual: UI renders button |
| [REQ-SF-020] 10s skip logic | Backend: `skip_forward()` calculates new_pos | Test 1: Verify +10000ms |
| [REQ-SF-030] Buffer validation | Backend: Check `buffer.occupied() >= 11s` | Test 2: Insufficient buffer |
| [REQ-SF-040] State requirements | Backend: Check `current.is_some()` | Test 3: No passage playing |
| [REQ-SF-050] Event emission | Backend: `seek()` emits PlaybackProgress | Test 1: Verify SSE event |
| [REQ-SF-NF-010] Performance | Handler delegates to `seek()` (fast) | Manual: Response time <50ms |
| [REQ-SF-NF-020] Reliability | Atomic buffer read, idempotent seek | Test 4: Rapid clicks handled |
| [REQ-SF-NF-030] Maintainability | Reuse `seek()` logic | Code review: No duplication |

---

## 9. Open Questions

**Q1:** Should the button show remaining skip-able time?
**A1:** Deferred to future enhancement. Initial version just shows "+10s" label.

**Q2:** What if skip lands exactly at crossfade start point?
**A2:** Mixer handles this automatically (crossfade logic is position-driven). No special handling needed.

**Q3:** Should we persist last skip time to prevent accidental double-skips?
**A3:** No. `isSkipping` state prevents concurrent requests, which is sufficient. Persisting skip time adds complexity without clear benefit.

---

## 10. Approval & Sign-off

**Design Reviewed By:** [Pending User Approval]
**Implementation Start Date:** [TBD]
**Target Completion:** [TBD]

---

## Appendices

### Appendix A: Relevant Existing Code References

**Buffer validation pattern:**
- [playout_ring_buffer.rs:498](c:\Users\Mango Cat\Dev\McRhythm\wkmp-ap\src\playback\playout_ring_buffer.rs#L498) - `occupied()` method
- [playout_ring_buffer.rs:493](c:\Users\Mango Cat\Dev\McRhythm\wkmp-ap\src\playback\playout_ring_buffer.rs#L493) - `capacity()` method

**Seek implementation:**
- [playback.rs:155-200](c:\Users\Mango Cat\Dev\McRhythm\wkmp-ap\src\playback\engine\playback.rs#L155-L200) - `seek()` method
- Clamping logic at line 185: `clamped_position = position_frames.min(max_frames.saturating_sub(1))`

**Mixer position tracking:**
- [mixer.rs:447-449](c:\Users\Mango Cat\Dev\McRhythm\wkmp-ap\src\playback\mixer.rs#L447-L449) - `get_current_tick()`
- [mixer.rs:425-429](c:\Users\Mango Cat\Dev\McRhythm\wkmp-ap\src\playback\mixer.rs#L425-L429) - `set_current_passage()`

**API handler patterns:**
- [handlers.rs:648-686](c:\Users\Mango Cat\Dev\McRhythm\wkmp-ap\src\api\handlers.rs#L648-L686) - `play()` handler
- [handlers.rs:833-854](c:\Users\Mango Cat\Dev\McRhythm\wkmp-ap\src\api\handlers.rs#L833-L854) - `skip_next()` handler

### Appendix B: Timing Conversions Reference

**Sample Rate:** 44100 Hz (default working_sample_rate)

**10 seconds:**
- Milliseconds: 10,000 ms
- Frames: 441,000 frames (stereo)
- Ticks: 441,000 ticks (1:1 with frames per `wkmp_common::timing`)

**11 seconds (validation threshold):**
- Milliseconds: 11,000 ms
- Frames: 485,100 frames (stereo)

**Conversion formulas** (from `wkmp_common::timing`):
```rust
fn ms_to_ticks(ms: u64, sample_rate: u32) -> i64 {
    ((ms as i64) * (sample_rate as i64)) / 1000
}

fn ticks_to_ms(ticks: i64, sample_rate: u32) -> u64 {
    ((ticks as u64) * 1000) / (sample_rate as u64)
}
```
