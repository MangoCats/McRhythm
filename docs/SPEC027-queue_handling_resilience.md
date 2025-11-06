# Queue Handling Resilience

**📝 TIER 2 - DESIGN SPECIFICATION**

Defines resilience patterns for queue operations, event handling, and resource cleanup to ensure reliable passage transitions under edge cases and concurrent operations.

> **Related Documentation:** [SPEC028 Playback Orchestration](SPEC028-playback_orchestration.md) | [SPEC016 Decoder Buffer Design](SPEC016-decoder_buffer_design.md) | [Requirements](REQ001-requirements.md)

---

## Overview

Queue handling involves coordinating multiple subsystems (database, memory, mixer, decoder chains, buffers) during passage transitions. This specification defines resilience patterns to handle edge cases:

- **Duplicate events** from multiple completion sources (PassageComplete marker, EOF, early EOF)
- **Retry operations** where database or memory state may already reflect prior attempts
- **Concurrent operations** where multiple threads access queue state simultaneously

**Key Principle:** Operations must be **idempotent** (safe to retry) and **robust** to duplicate/out-of-order events.

---

## Design Decisions

### 1. Idempotent Database Operations

**Decision:** Database removal operations return success/failure indication rather than erroring on "not found."

**Rationale:**
- Queue removal can be triggered from multiple sources (user action, passage completion, error conditions)
- Second removal attempt is a success case (already removed), not an error
- Eliminates ERROR logs for normal duplicate-event scenarios
- Follows distributed systems pattern: idempotent operations enable safe retries

**Pattern:**
```rust
pub async fn remove_from_queue(db: &Pool<Sqlite>, queue_entry_id: Uuid) -> Result<bool>
```
- Returns `Ok(true)` if entry removed by this call
- Returns `Ok(false)` if entry not found (idempotent no-op)
- Returns `Err(_)` only for database errors (connection failure, etc.)

**Alternative Considered:** Error on second removal
- **Rejected:** Requires callers to distinguish "expected miss" from "unexpected miss"
- **Rejected:** Pollutes logs with ERROR messages for normal race conditions

**Requirements:**
- **[SPEC027-IDEMP-010]** All queue database operations MUST be idempotent
- **[SPEC027-IDEMP-020]** Return value MUST distinguish "action taken" from "already done"

---

### 2. Event Deduplication

**Decision:** Deduplicate PassageComplete events using time-bounded cache (5-second window).

**Rationale:**
- PassageComplete emitted from 3 sources: normal marker, EOF, early EOF
- All three can fire for same passage (e.g., early EOF also triggers PassageComplete marker)
- Time window chosen: 5 seconds >> longest passage transition (~1-2 seconds)
- Automatic cleanup prevents unbounded memory growth

**Design:**
- Use `HashMap<Uuid, Instant>` to track recently processed queue_entry_ids
- Check cache before processing event (skip if seen within 5 seconds)
- Clean up entries older than 5 seconds to prevent memory leak
- Thread-safe via `Arc<RwLock<HashMap>>` for concurrent access

**Deduplication Window Rationale:**
- **1 second:** Too short, misses delayed events in slow systems
- **5 seconds:** Conservative safety margin, covers all realistic delays
- **60 seconds:** Unnecessarily long, wastes memory on stale entries
- **Choice:** 5 seconds balances safety vs memory efficiency

**Alternative Considered:** Event source consolidation (single PassageComplete)
- **Rejected:** Higher risk - requires complex EOF detection logic in mixer
- **Rejected:** Harder to test all EOF edge cases (early EOF, missing markers)
- **Deferred:** May revisit in future for cleaner architecture

**Requirements:**
- **[SPEC027-DEDUP-010]** PassageComplete events MUST be deduplicated per queue_entry_id
- **[SPEC027-DEDUP-020]** Deduplication window MUST be 5 seconds minimum
- **[SPEC027-DEDUP-030]** Stale cache entries MUST be automatically cleaned up
- **[SPEC027-DEDUP-040]** Deduplication state MUST be thread-safe

---

### 3. Resource Cleanup Ordering

**Decision:** Define canonical cleanup sequence for queue entry removal.

**Cleanup Order:**
1. **Release decoder-buffer chain** - Free resources first
2. **Clear mixer state** - Stop audio output, clear markers
3. **Remove from database** - Persist removal (idempotent)
4. **Remove from memory queue** - Update in-memory state
5. **Release buffer memory** - Free decoded audio samples
6. **Emit events** - Notify subscribers (UI, telemetry)
7. **Reassign freed chain** - Allocate to waiting passages

**Rationale:**
- **Resources freed before reassignment** - Prevents resource exhaustion
- **Database before memory** - Persistence trumps memory (crash recovery)
- **Events last** - Subscribers see consistent post-cleanup state
- **Chain reassignment last** - New decode only after full cleanup

**Alternative Considered:** Ad-hoc cleanup in each caller
- **Rejected:** Code duplication (3+ copies of cleanup logic)
- **Rejected:** Cleanup order can diverge, introducing bugs
- **Rejected:** Harder to maintain correct ordering as system evolves

**Requirements:**
- **[SPEC027-CLEANUP-010]** Cleanup operations MUST follow canonical order
- **[SPEC027-CLEANUP-020]** Single cleanup implementation (DRY principle)
- **[SPEC027-CLEANUP-030]** All cleanup callers MUST use helper method

---

### 4. Concurrent Access Safety

**Decision:** Use RwLock for queue state, atomic operations for counters.

**Access Patterns:**
- **Frequent reads:** Queue inspection (current, next, queued positions)
- **Infrequent writes:** Queue modifications (add, remove, promote)
- **RwLock optimization:** Multiple concurrent readers, exclusive writer

**Thread Safety Requirements:**
- **Deduplication cache:** `Arc<RwLock<HashMap<Uuid, Instant>>>`
- **Queue state:** `Arc<RwLock<QueueManager>>`
- **Atomic counters:** Statistics (watchdog interventions, etc.)

**Requirements:**
- **[SPEC027-THREAD-010]** All shared queue state MUST be thread-safe
- **[SPEC027-THREAD-020]** Use RwLock for read-heavy access patterns
- **[SPEC027-THREAD-030]** Use atomic operations for counters

---

## Architectural Integration

### Integration with SPEC028 (Playback Orchestration)

**Event-Driven Model:**
- SPEC028 defines event-driven playback orchestration
- SPEC027 adds resilience to event processing (deduplication, idempotency)
- Events remain primary coordination mechanism
- Deduplication is transparent layer, doesn't change event semantics

**Watchdog Coordination:**
- SPEC028 defines watchdog as detection-only safety net
- SPEC027 ensures watchdog can safely retry operations (idempotency)
- Multiple watchdog checks won't cause duplicate removals

### Integration with SPEC016 (Decoder Buffer Design)

**Chain Lifecycle:**
- SPEC016 defines chain assignment, decoding, playback, release
- SPEC027 ensures cleanup respects chain lifecycle (release before reassignment)
- Cleanup helper encapsulates chain release timing

**Buffer Management:**
- SPEC016 defines per-chain buffer lifecycle
- SPEC027 ensures buffers released in correct cleanup phase
- Idempotent buffer removal (no error if already released)

---

## Performance Characteristics

### Deduplication Overhead

**Cache Operations:**
- Read check: O(1) HashMap lookup (fast path)
- Write insert: O(1) HashMap insert (event processing)
- Cleanup: O(n) iteration over cache entries (background, infrequent)

**Memory Impact:**
- Typical: <100 entries (passages/minute × 5-second window)
- Maximum: ~300 entries (60 passages/minute × 5 seconds)
- Per entry: ~40 bytes (Uuid + Instant)
- **Total: <12KB peak memory** (negligible)

**Cleanup Strategy:**
- Cleanup every 10 seconds (background task)
- Or: Per-event spawn (simpler, slightly higher overhead)
- **Choice:** Per-event spawn for simplicity (PLAN decision, not SPEC)

### Idempotency Overhead

**Database Check:**
- `rows_affected() > 0` check: ~0 overhead (already performed)
- No additional queries required

**Caller Impact:**
- Callers check boolean return instead of catching errors
- Slightly cleaner code, no performance difference

---

## Error Handling

### Expected Edge Cases (Not Errors)

**Idempotent No-Ops:**
- Second removal attempt → `Ok(false)` (not error)
- Logged at DEBUG level: "already removed"

**Duplicate Events:**
- Duplicate PassageComplete → Skip silently
- Logged at DEBUG level: "duplicate event (Xms ago)"

**Missing Resources:**
- Buffer not found during cleanup → Ignore (already released)
- Chain not assigned → Ignore (never assigned or already released)

### Actual Errors

**Database Failures:**
- Connection timeout → `Err(DatabaseError)`
- Transaction conflict → Retry or fail gracefully
- Logged at ERROR level

**Cleanup Failures:**
- Mixer clear fails → Log ERROR, continue cleanup (best effort)
- Event emission fails → Log WARN, continue (non-critical)

---

## Testing Strategy

### Test Coverage Requirements

**Idempotency Tests:**
- First removal succeeds (normal case)
- Second removal returns false (idempotent no-op)
- Remove non-existent entry returns false

**Deduplication Tests:**
- First event processed (normal case)
- Duplicate event within 5 seconds ignored
- Multiple distinct events processed independently
- Stale cache entries cleaned up after 5 seconds

**Cleanup Ordering Tests:**
- Verify cleanup steps execute in canonical order
- Verify cleanup is idempotent (safe to retry)
- Verify all cleanup callers use helper

**Concurrency Tests:**
- Concurrent event processing (no race conditions)
- Concurrent queue modifications (thread-safe)
- Stress test: rapid skip operations, duplicate events

---

## Migration Considerations

### Backward Compatibility

**Database Schema:** No changes required (idempotency is return value change only)

**API Changes:**
- `remove_from_queue()` signature changes: `Result<()>` → `Result<bool>`
- **Breaking change** for callers (requires update)
- **Mitigation:** All callers internal to wkmp-ap (no external API impact)

**Event Semantics:**
- PassageComplete events unchanged
- Deduplication is transparent to event subscribers
- No breaking changes for event consumers

### Rollout Strategy

**Test-Driven Implementation:**
1. Write unit tests for idempotency (fail initially)
2. Implement idempotent operations (tests pass)
3. Write integration tests for deduplication
4. Implement deduplication logic
5. Refactor cleanup methods to use helper
6. Verify no regression in existing tests

**Verification:**
- Monitor logs for ERROR→DEBUG transition (duplicate events)
- Verify no new ERROR logs introduced
- Confirm cleanup happens exactly once per passage

---

## Future Enhancements

**Out of scope for initial implementation:**

### Configurable Deduplication Window

**Current:** 5-second hardcoded constant
**Enhancement:** Database setting `deduplication_window_ms`
**Benefit:** Tune for different hardware/workloads
**Risk:** Low (configuration complexity)

### Telemetry for Duplicate Events

**Current:** DEBUG logging only
**Enhancement:** SSE event `WkmpEvent::DuplicateEventDetected`
**Benefit:** Developer UI visibility into event system health
**Risk:** Low (new event type)

### Event Source Consolidation

**Current:** 3 sources emit PassageComplete (marker, EOF, early EOF)
**Enhancement:** Single PassageComplete emission from mixer
**Benefit:** Cleaner architecture, no deduplication needed
**Risk:** Medium-High (complex EOF detection logic, harder to test)
**Status:** Deferred pending further analysis

---

## Traceability

### Requirements Satisfied

This specification satisfies the following requirements from REQ001:
- **[REQ-PLAY-030]** Automatic passage advancement (resilient queue handling)
- **[REQ-PLAY-040]** Skip to next passage (idempotent removal)
- **[REQ-NF-020]** System reliability (robust to duplicate events)

### Design Influences

This specification is informed by:
- **[SPEC028]** Event-driven orchestration (deduplication layer for events)
- **[SPEC016]** Decoder buffer design (cleanup respects chain lifecycle)
- **Analysis:** Queue handling mechanism analysis (identified edge cases)

### Implementation Guidance

Implementation details captured in:
- **PLAN022:** Queue handling resilience implementation plan
- **Test specs:** wkmp-ap/tests/queue_*_tests.rs

---

## Summary

**Problem:** Queue operations fail on duplicate events due to non-idempotent operations and lack of event deduplication.

**Solution:**
1. **Idempotent database operations** - Safe to retry, no ERROR on second removal
2. **Event deduplication** - 5-second cache prevents duplicate processing
3. **Canonical cleanup order** - DRY helper ensures correct resource release sequence

**Benefits:**
- Zero ERROR logs for normal duplicate-event scenarios
- Robust to event ordering/timing variations
- Maintainable single cleanup implementation (DRY)
- Minimal overhead (<12KB memory, O(1) cache lookups)

**Status:** Active design specification, implemented in PLAN022

---

**Maintained By:** Technical lead
**Last Updated:** 2025-11-06
**Version:** 1.0
