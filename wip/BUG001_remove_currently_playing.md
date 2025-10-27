# BUG001: Remove Currently Playing Passage Causes Playback State Corruption

**Severity:** High
**Component:** Audio Player (wkmp-ap)
**Status:** Confirmed
**Reported:** 2025-10-27

---

## Summary

Removing the currently playing passage from the queue causes playback state corruption. When a new passage is subsequently enqueued, the removed passage resumes playing instead of the new passage starting, and the new passage vanishes from the queue after the old passage finishes.

---

## Reproduction Steps

1. **Initial State:** Start wkmp-ap with empty queue
2. **Enqueue passage A:** Add passage to queue, playback starts
3. **Remove passage A:** Click remove button while passage A is playing
   - **Expected:** Passage A stops, queue empty, playback stops
   - **Actual:** Passage A stops, queue empty, playback stops ✓
4. **Enqueue passage B:** Add new passage to queue
   - **Expected:** Passage B starts playing immediately
   - **Actual:** Passage A resumes from where it stopped ✗
5. **Observe Buffer Chain Monitor:**
   - Passage B is buffered in decoder chain 1
   - Passage A continues playing (but not shown in queue)
6. **Wait for passage A to finish:**
   - **Expected:** Passage B starts playing
   - **Actual:** Passage B vanishes from queue, no playback ✗

---

## Root Cause Analysis

### Code Flow

**File:** `wkmp-ap/src/api/handlers.rs:371-383`
```rust
pub async fn remove_from_queue(...) {
    // Remove from database
    crate::db::queue::remove_from_queue(&ctx.db_pool, queue_entry_id).await;

    // Remove from in-memory queue
    ctx.engine.remove_queue_entry(queue_entry_id).await;
    // ^^^ PROBLEM: Doesn't handle currently playing passage
}
```

**File:** `wkmp-ap/src/playback/engine.rs:1465-1479`
```rust
pub async fn remove_queue_entry(&self, queue_entry_id: Uuid) -> bool {
    let removed = self.queue.write().await.remove(queue_entry_id);
    // ^^^ Delegates to QueueManager::remove()
    // ^^^ PROBLEM: Doesn't clear decoder chain or mixer state
}
```

**File:** `wkmp-ap/src/playback/queue_manager.rs:307-314`
```rust
pub fn remove(&mut self, queue_entry_id: Uuid) -> bool {
    if let Some(ref current) = self.current {
        if current.queue_entry_id == queue_entry_id {
            self.advance();  // Moves to next passage in queue structure
            return true;
            // ^^^ PROBLEM: Queue state updated but decoder/mixer unchanged
        }
    }
}
```

### The Bug

When removing the currently playing passage:

1. **Queue state updated:** `QueueManager::advance()` moves `current` → `next`, `next` → `queued[0]`
2. **Decoder chain NOT cleared:** The decoder chain still has the removed passage's PCM buffer cached
3. **Mixer state NOT updated:** Mixer continues reading from the old decoder chain
4. **Chain assignment stale:** When new passage enqueued, it gets a different chain assignment
5. **State inconsistency:** Queue thinks new passage is `current`, but mixer plays old passage

### Why Resume Happens

When passage A is removed and queue becomes empty:
- Mixer drains remaining PCM samples from passage A's buffer
- Playback stops when buffer exhausted
- **BUT:** Decoder chain retains passage A's decode state (file handle, position)

When passage B is enqueued:
- Queue assigns passage B as `current`
- Decoder worker picks passage B for buffering (gets new chain)
- **BUT:** Mixer still points to old chain with passage A
- Passage A's buffer refills (decode continues from saved position)
- Mixer plays passage A instead of passage B

---

## Expected Behavior

When the currently playing passage is removed:

1. **Immediate stop:** Mixer stops outputting audio from that passage
2. **Clear decoder chain:** Release decoder chain resources (file handle, buffers)
3. **Clear mixer state:** Reset mixer to idle/stopped state
4. **Update queue:** Remove from queue structure (already working)
5. **Start next passage:** If queue non-empty after removal, start next passage immediately

---

## Affected Requirements

- **REQ-QUE-070:** Queue management operations (remove from queue)
- **REQ-PB-010:** Playback control (stop playback when queue empty)
- **REQ-PB-040:** Queue advancement (skip to next passage)

---

## Technical Impact

### Subsystems Affected

1. **Queue Management (`queue_manager.rs`):**
   - `remove()` method handles queue structure but not playback state

2. **Playback Engine (`engine.rs`):**
   - `remove_queue_entry()` delegates to queue but doesn't coordinate with decoder/mixer

3. **Decoder Worker (`decoder_worker.rs`):**
   - Decoder chain holds stale passage reference after removal
   - Continues buffering removed passage when triggered

4. **Mixer (`pipeline/mixer.rs`):**
   - Mixer continues reading from stale decoder chain
   - No signal to stop/clear current passage

### Data Consistency Issues

- **Queue state:** Reflects removal correctly
- **Decoder chain assignment:** Stale mapping (removed passage still assigned)
- **Mixer state:** Playing removed passage
- **SSE events:** Queue appears empty but audio still playing

---

## Similar Issues

This is related to general "currently playing passage lifecycle" management:

- What happens when currently playing passage reaches natural end? (Working correctly)
- What happens when user clicks "next"? (Needs verification)
- What happens when queue is cleared? (Needs verification)

---

## Test Cases Needed

### Test 1: Remove Currently Playing Passage (Empty Queue After)
```
GIVEN: Passage A playing, queue = [A]
WHEN:  Remove passage A
THEN:
  - Playback stops immediately
  - Queue empty
  - Decoder chain for A released
  - No audio output
```

### Test 2: Remove Currently Playing Passage (Queue Non-Empty After)
```
GIVEN: Passage A playing, queue = [A, B, C]
WHEN:  Remove passage A
THEN:
  - Passage A stops immediately
  - Passage B starts playing
  - Queue = [B, C]
  - Decoder chain for A released
  - Decoder chain for B active
```

### Test 3: Remove Currently Playing, Enqueue New
```
GIVEN: Passage A playing, queue = [A]
WHEN:  Remove passage A, wait 1s, enqueue passage B
THEN:
  - Passage A stops immediately on removal
  - Queue empty for 1s
  - Passage B starts when enqueued
  - Passage A does NOT resume
```

### Test 4: Remove Next Passage (Not Currently Playing)
```
GIVEN: Passage A playing, queue = [A, B]
WHEN:  Remove passage B
THEN:
  - Passage A continues playing
  - Queue = [A]
  - No disruption to current playback
```

---

## Proposed Fix Overview

**Approach:** Add "currently playing passage lifecycle management" to `remove_queue_entry()`.

**Key Changes:**

1. **Detect removal of current passage:**
   - Check if `queue_entry_id` matches currently playing passage

2. **Clear decoder chain:**
   - Send command to decoder worker to stop and release chain
   - Clear chain assignment mapping

3. **Clear mixer state:**
   - Signal mixer to stop current passage
   - Drain remaining samples or flush buffer

4. **Start next passage:**
   - If queue non-empty after removal, trigger playback of new `current`
   - If queue empty, enter stopped state

**Detailed implementation plan:** See PLAN002_fix_remove_currently_playing.md

---

## References

- **Code:** `wkmp-ap/src/api/handlers.rs:371` (remove_from_queue handler)
- **Code:** `wkmp-ap/src/playback/engine.rs:1465` (remove_queue_entry)
- **Code:** `wkmp-ap/src/playback/queue_manager.rs:307` (QueueManager::remove)
- **Spec:** REQ-QUE-070 (Queue management)
- **Spec:** REQ-PB-040 (Skip/advance playback)

---

## Notes

- Bug discovered during authentication implementation testing (2025-10-27)
- Affects all queue removal scenarios where currently playing passage is target
- May also affect "Clear Queue" functionality (needs verification)
- May also affect "Skip Next" functionality (needs verification)
