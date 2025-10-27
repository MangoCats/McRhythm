# Scope Statement - Fix Remove Currently Playing Passage Bug

**Plan ID:** PLAN002
**Date:** 2025-10-27
**Status:** Planning Phase

---

## In Scope

### Core Fix
1. **Detect removal of currently playing passage**
   - Add check in `remove_queue_entry()` to identify if removed entry is current

2. **Clear decoder chain resources**
   - Stop decoding for removed passage
   - Release file handle and buffers
   - Clear chain assignment mapping

3. **Clear mixer state**
   - Stop mixer from reading removed passage's buffer
   - Clear playback position
   - Reset to idle/stopped state

4. **Handle queue state transitions**
   - If queue empty after removal: Enter stopped state
   - If queue non-empty: Start next passage immediately

5. **Prevent removed passage resume**
   - Ensure decoder doesn't refill removed passage's buffer
   - Ensure mixer doesn't read from stale chain

### Test Coverage
- Test removal of currently playing passage (empty queue after)
- Test removal of currently playing passage (queue non-empty after)
- Test removal then enqueue new passage (the reported bug scenario)
- Test removal of non-current passage (should not disrupt playback)

###Documentation
- Update code comments with fix explanation
- Reference BUG001 in commit message

---

## Out of Scope

### Not Included in This Fix
1. **"Clear Queue" functionality verification**
   - Mentioned as "needs verification" in bug report
   - Should be separate investigation/fix if broken

2. **"Skip Next" functionality verification**
   - Mentioned as "needs verification" in bug report
   - Should be separate investigation/fix if broken

3. **General playback lifecycle refactoring**
   - This is a targeted bug fix, not architectural redesign
   - Broader lifecycle management improvements are future work

4. **Performance optimization**
   - Focus is correctness, not performance
   - Performance work can be done separately if needed

5. **UI changes**
   - No changes to developer UI or API contracts
   - Fix is internal to playback engine

---

## Assumptions

1. **Queue structure management is correct**
   - `QueueManager::remove()` correctly updates queue data structure
   - Issue is only with decoder/mixer coordination

2. **Natural playback end works correctly**
   - When passage finishes naturally, next passage starts properly
   - This confirms the "start next passage" mechanism exists and works

3. **Decoder worker command pattern exists**
   - Decoder worker has some mechanism to receive commands
   - May need to add new command type for "stop chain"

4. **Mixer can be signaled**
   - Mixer has some way to be told to stop/clear
   - May be through shared state or channel

5. **Single-threaded or properly synchronized**
   - All state updates are properly synchronized with async/await
   - No additional locking needed beyond existing patterns

6. **Test infrastructure exists**
   - Can create integration tests for playback scenarios
   - Test files and test harness available

---

## Constraints

### Technical
- **Must maintain existing API contracts**
  - No changes to HTTP API endpoints
  - No changes to SSE event formats

- **Must preserve existing functionality**
  - Removing non-current passages must still work
  - Natural passage end must still work
  - Manual skip must still work (if implemented)

- **Must follow Rust async patterns**
  - Use tokio channels for signaling
  - Proper `.await` usage
  - No blocking operations in async context

### Project
- **Bug fix only - no feature additions**
  - Solve the reported problem
  - Don't add new capabilities

- **Minimal code changes**
  - Touch only necessary files
  - Preserve existing architecture where possible

### Time
- **High priority bug**
  - Affects core playback functionality
  - Blocks normal usage (can't safely remove passages)

---

## Dependencies

### Code Dependencies

**Existing Code (will modify):**
- `wkmp-ap/src/playback/engine.rs:1465` - `remove_queue_entry()` function
- `wkmp-ap/src/playback/queue_manager.rs:307` - `QueueManager::remove()` function
- `wkmp-ap/src/playback/decoder_worker.rs` - decoder command handling (if exists)
- `wkmp-ap/src/playback/pipeline/mixer.rs` - mixer state management (if accessible)

**Existing Code (will reference):**
- `wkmp-ap/src/api/handlers.rs:371` - `remove_from_queue` HTTP handler (unchanged)
- `wkmp-ap/src/db/queue.rs` - database queue operations (unchanged)
- Existing playback start/stop mechanisms

**External Dependencies:**
- tokio (async runtime)
- Standard Rust async/await patterns
- Existing WKMP event system (for SSE notifications)

### Specification Dependencies
- REQ-QUE-070: Queue management operations
- REQ-PB-010: Playback control (stop)
- REQ-PB-040: Queue advancement (skip/next)

### No New External Dependencies
- No new crates required
- No new system libraries required
- Uses existing playback infrastructure

---

## Success Criteria

### Functional Success
1. ✓ Removing currently playing passage stops playback immediately
2. ✓ Decoder resources released (no file handle leak)
3. ✓ Mixer stops outputting audio from removed passage
4. ✓ Queue state reflects removal correctly
5. ✓ New passage starts immediately when enqueued after removal
6. ✓ Removed passage does NOT resume playback
7. ✓ Removing non-current passage doesn't disrupt current playback

### Test Success
- All 4 test scenarios pass (from bug report)
- No regression in existing playback scenarios
- Manual testing confirms fix

### Code Quality
- Changes follow existing code patterns
- Proper error handling
- Code comments explain the fix
- Commit message references BUG001

---

## Risk Assessment (High-Level)

### Primary Risks
1. **Decoder/mixer signaling mechanism unclear**
   - Mitigation: Investigate existing code first, find precedent

2. **State synchronization issues**
   - Mitigation: Use existing async patterns, proper locking

3. **Regression in working scenarios**
   - Mitigation: Comprehensive test coverage

### Risk Acceptance
- This is a critical bug affecting core functionality
- Risk of fix causing issues is acceptable vs. risk of leaving bug unfixed
- Changes will be tested before deployment

---

## References

- **Bug Report:** wip/BUG001_remove_currently_playing.md
- **Architecture:** docs/SPEC001-architecture.md (single-stream audio design)
- **Requirements:** docs/REQ001-requirements.md (playback requirements)
- **Implementation:** docs/IMPL003-project_structure.md (wkmp-ap structure)
