# SSE Replay/History Enhancement for Reconnecting Clients

**Context:** Increment 7 implementation (2025-10-29)

**Issue:** Current SSE implementation does not provide event replay when clients reconnect after disconnection.

**Impact:**
- If a client disconnects during import (network issues, browser refresh, etc.), they miss progress updates
- Client must poll `/import/status/:id` endpoint to catch up on missed progress
- Not a seamless real-time experience

**Enhancement Proposal:**

## Option 1: Last-Event-ID Support (SSE Standard)
- SSE spec supports `Last-Event-ID` header for reconnection
- Server maintains recent event history (e.g., last 100 events)
- On reconnection with `Last-Event-ID`, replay missed events
- Pros: Standard SSE feature, well-supported by browsers
- Cons: Requires event ID tracking and history buffer

## Option 2: Initial State Snapshot
- Similar to wkmp-ap's `InitialState` event pattern
- On SSE connection, send current import session state
- Includes: current phase, progress, elapsed time
- Pros: Simple, always provides complete current state
- Cons: Doesn't provide full history of what happened

## Option 3: Hybrid Approach
- Send `InitialState` on connection (current status)
- Support `Last-Event-ID` for replay if available
- Fallback to polling if history not available
- Pros: Best user experience, handles all cases
- Cons: Most complex implementation

## Recommendation: Option 2 (Initial State)
- Simpler implementation
- Matches existing pattern in wkmp-ap
- Sufficient for most use cases (user mainly cares about current progress)
- Can upgrade to Option 3 later if needed

## Implementation Notes:
- Add `ImportInitialState` event type to `WkmpEvent` enum
- Query current session state on SSE connection
- Emit initial state before streaming ongoing events
- Similar to wkmp-ap's `/events` endpoint pattern (wkmp-ap/src/api/sse.rs:28-96)

**Priority:** Medium (nice-to-have, not critical for MVP)
**Effort:** 2-4 hours
**Risk:** Low
