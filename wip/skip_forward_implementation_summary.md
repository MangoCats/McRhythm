# Skip Forward Feature - Implementation Summary

**Date:** 2025-06-11
**Status:** Backend Complete ✅ | Frontend Pending

---

## Backend Implementation Complete

### Files Modified

1. **`wkmp-ap/src/api/handlers.rs`** (Lines 898-949)
   - Added `skip_forward()` API handler
   - Delegates to `PlaybackEngine::skip_forward()`
   - Returns `SkipForwardResponse` with new position
   - Error handling with appropriate HTTP status codes (400/500)

2. **`wkmp-ap/src/playback/engine/playback.rs`** (Lines 152-228)
   - Added `PlaybackEngine::skip_forward()` method
   - Implements 6-step validation and execution:
     1. Get current queue entry (fail if none)
     2. Get buffer for passage (fail if unavailable)
     3. **Validate buffer ≥11 seconds** (critical safety check)
     4. Get current playback tick position
     5. Calculate new position (+10,000ms)
     6. Delegate to existing `seek()` method

3. **`wkmp-ap/src/api/server.rs`** (Line 105)
   - Registered route: `POST /playback/skip-forward`
   - Mapped to `handlers::skip_forward`

### API Specification

**Endpoint:** `POST http://localhost:5721/playback/skip-forward`

**Request:**
- No body required

**Success Response (200 OK):**
```json
{
  "status": "skipped forward 10 seconds",
  "new_position_ms": 45320
}
```

**Error Responses:**

*400 Bad Request* - No passage playing:
```json
{
  "status": "error: no passage currently playing"
}
```

*400 Bad Request* - Insufficient buffer:
```json
{
  "status": "error: insufficient buffer - only 8.5s available (need 11s)"
}
```

*500 Internal Server Error* - Buffer not available:
```json
{
  "status": "error: buffer not available for queue_entry_id=..."
}
```

### Safety Features Implemented

✅ **[REQ-SF-030] Buffer Validation**
- Checks that ≥11 seconds (485,100 frames @ 44.1kHz) are buffered
- Prevents buffer underruns during position transition
- 1-second safety margin beyond the 10s skip

✅ **[REQ-SF-040] State Requirements**
- Works in both Playing and Paused states
- Playback state remains unchanged after skip
- Fails gracefully when no passage is playing

✅ **[REQ-SF-050] Event Emission**
- Delegates to `seek()` which emits `PlaybackProgress` SSE event
- UI receives real-time position update via existing SSE infrastructure

✅ **Sample-Accurate Positioning**
- Uses tick-based timing system (28,224,000 Hz tick rate)
- Conversion: `current_tick → ticks_to_ms() → +10000ms → seek()`
- Preserves WKMP's 0.02ms precision guarantee

### Architecture Decisions

**Risk-First Framework Applied:**

1. **Reuse `seek()` Logic** ✅
   - **Risk:** Code duplication → maintenance burden
   - **Mitigation:** `skip_forward()` delegates to `seek()`
   - **Result:** 6 lines of actual logic, rest is validation
   - **Residual Risk:** Low

2. **11-Second Buffer Requirement** ✅
   - **Risk:** Buffer underrun during skip
   - **Mitigation:** Require 11s (10s skip + 1s margin)
   - **Result:** Atomic check using `buffer.occupied()`
   - **Residual Risk:** Low

3. **Engine Method vs. Direct API Logic** ✅
   - **Risk:** Private field access from handler
   - **Mitigation:** Create `PlaybackEngine::skip_forward()` method
   - **Result:** Clean separation, testable, follows existing patterns
   - **Residual Risk:** Very Low

### Compilation Status

✅ **Compiles Successfully**
```bash
cargo build -p wkmp-ap
# Result: Success (6 warnings, 0 errors)
# Warnings are pre-existing (unused imports, dead code)
```

### Code Quality

- **Traceability:** All requirements tagged with `[REQ-SF-XXX]` comments
- **Documentation:** Comprehensive doc comments on all new functions
- **Error Handling:** Descriptive error messages for all failure modes
- **Logging:** Info-level logs for success, warn-level for failures
- **Patterns:** Follows existing WKMP conventions (lock early-release, error types)

---

## Manual Testing Instructions

### Prerequisites

1. Start `wkmp-ap` backend:
   ```bash
   cargo run -p wkmp-ap
   ```

2. Enqueue a passage with sufficient duration (>20s):
   ```bash
   curl -X POST http://localhost:5721/playback/enqueue \
     -H "Content-Type: application/json" \
     -d '{"file_path": "/path/to/long/audio/file.mp3"}'
   ```

3. Start playback:
   ```bash
   curl -X POST http://localhost:5721/playback/play
   ```

4. Wait for buffer to fill (check `/playback/buffer_chains` shows >11s buffered)

### Test Cases

**Test 1: Successful Skip (Happy Path)**
```bash
# Wait ~5 seconds into playback
curl -X POST http://localhost:5721/playback/skip-forward

# Expected:
# - HTTP 200 OK
# - Response: {"status": "skipped forward 10 seconds", "new_position_ms": ~15000}
# - Audio position jumps forward 10 seconds
# - No audio gaps or glitches
```

**Test 2: Insufficient Buffer**
```bash
# Immediately after enqueue (buffer still filling)
curl -X POST http://localhost:5721/playback/skip-forward

# Expected:
# - HTTP 400 Bad Request
# - Response: {"status": "error: insufficient buffer - only X.Xs available (need 11s)"}
```

**Test 3: No Passage Playing**
```bash
# With empty queue
curl -X POST http://localhost:5721/playback/skip-forward

# Expected:
# - HTTP 400 Bad Request
# - Response: {"status": "error: no passage currently playing"}
```

**Test 4: Skip While Paused**
```bash
# Pause playback
curl -X POST http://localhost:5721/playback/pause

# Skip forward
curl -X POST http://localhost:5721/playback/skip-forward

# Expected:
# - HTTP 200 OK
# - Position advances (check with GET /playback/position)
# - Playback remains paused
```

**Test 5: Skip Near End of Passage**
```bash
# For 30s passage, seek to 25s first
curl -X POST http://localhost:5721/playback/seek \
  -H "Content-Type: application/json" \
  -d '{"position_ms": 25000}'

# Then skip forward
curl -X POST http://localhost:5721/playback/skip-forward

# Expected:
# - HTTP 200 OK
# - Position clamped to passage end (or near end)
# - Mixer naturally advances to next passage if queued
```

---

## Frontend Implementation - TODO

### UI Button Component (Pending)

**Location:** `wkmp-ui/src/components/PlaybackControls.tsx` (or equivalent)

**Requirements:**
- Button icon: ⏭️ or custom SVG with "+10s" label
- Position: Between "Play/Pause" and "Skip Next"
- Tooltip: "Skip forward 10 seconds"
- Disabled when:
  - No passage playing
  - Buffer < 11s (check via SSE `BufferChainStatus`)
  - Request in progress (`isSkipping` state)

**API Integration:**
```typescript
const handleSkipForward = async () => {
  setIsSkipping(true);

  try {
    const response = await fetch('http://localhost:5721/playback/skip-forward', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        // TODO: Add authentication headers (timestamp + hash per SPEC007)
      },
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

**Buffer Availability Check:**
```typescript
// Subscribe to SSE BufferChainStatus events
const canSkipForward = useMemo(() => {
  if (!currentPassageId) return false;
  if (isSkipping) return false;

  // Find current passage's buffer chain
  const chain = bufferChains.find(c => c.passage_id === currentPassageId);
  if (!chain) return false;

  // Check if ≥11s buffered
  return (chain.buffer_duration_ms || 0) >= 11000;
}, [currentPassageId, bufferChains, isSkipping]);
```

---

## Next Steps

### Backend (Complete) ✅
- [x] Implement `skip_forward()` handler
- [x] Add `PlaybackEngine::skip_forward()` method
- [x] Register route in `server.rs`
- [x] Test compilation
- [ ] Write integration tests (optional - can be done post-MVP)

### Frontend (Pending)
- [ ] Create skip forward button component
- [ ] Implement state logic (canSkipForward check)
- [ ] Integrate API call with error handling
- [ ] Test button in developer UI
- [ ] Add keyboard shortcut (future enhancement)

### Documentation (Pending)
- [ ] Update SPEC007 (API Design) with `/playback/skip-forward` endpoint
- [ ] Add requirement IDs to REQ001 (if creating formal requirements)
- [ ] Update user manual (if applicable)

---

## Estimated Remaining Effort

**Frontend Implementation:** 2-3 hours
- Button component: 30 min
- State logic: 1 hour
- API integration: 30 min
- Testing: 1 hour

**Documentation:** 30 min
- API spec update: 15 min
- User-facing docs: 15 min

**Total Remaining:** ~3 hours to complete frontend + docs

---

## Risk Assessment (Final)

| Risk | Probability | Impact | Mitigation | Residual Risk |
|------|-------------|--------|------------|---------------|
| Buffer underrun | Low | Medium | 11s requirement + atomic check | Low |
| Race condition | Very Low | Medium | Atomic buffer read + mixer handling | Low |
| UI state desync | Low | Low | SSE updates + backend validation | Low |
| Spam clicks | Medium | Low | isSkipping state + idempotent seek | Very Low |
| Skip past end | Very Low | Low | seek() clamping | Very Low |

**Overall Risk:** Low ✅

---

## Approval Status

**Backend Implementation:** ✅ Complete & Compiling
**Frontend Implementation:** ⏳ Pending
**Feature Status:** 50% Complete (backend done)

**Ready for:** Manual testing, frontend development
**Blockers:** None
