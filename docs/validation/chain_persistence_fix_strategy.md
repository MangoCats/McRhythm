# Chain Persistence Fix - Implementation Strategy

**Issue:** Decoder-buffer chains not maintaining stable passage assignments throughout lifecycle
**Root Cause:** `get_buffer_chains()` in engine.rs dynamically assigns slot_index based on queue position, violating [DBD-OV-080]
**Date:** 2025-10-20

---

## Problem Statement

**Observed Behavior:**
1. Enqueue passage A → appears in chain 0 (position 0 = "now playing")
2. Enqueue passage B → appears briefly in chain 1 (position 1 = "up next")
3. Passage B **immediately replaces** passage A in chain 0 and starts playing
4. Passage A disappears from buffer chain monitor

**Expected Behavior per [DBD-OV-080]:**
> "Each decoder-buffer chain is assigned to a passage in the queue and **remains associated with that passage** as the passage advances toward the now playing queue position."

1. Enqueue passage A → assigned to first available chain (e.g., chain 0), stays with chain 0 through entire lifecycle
2. Enqueue passage B → assigned to next available chain (e.g., chain 1), stays with chain 1 through entire lifecycle
3. Enqueue passage C → assigned to next available chain (e.g., chain 2), stays with chain 2 through entire lifecycle
4. When passage A completes playback → chain 0 becomes available for reassignment
5. Enqueue passage D → assigned to chain 0 (now available)

---

## Root Cause Analysis

### Current Implementation (INCORRECT)

**File:** `wkmp-ap/src/playback/engine.rs:908`

```rust
// Create BufferChainInfo for each passage (up to maximum_decode_streams)
for (slot_index, entry) in all_entries.iter().take(maximum_decode_streams).enumerate() {
    let queue_position = slot_index; // 0-indexed per [SPEC020-MONITOR-050]
    // ...
}
```

**Problem:** `slot_index` is derived from `enumerate()` over queue-ordered entries. When a passage completes and is removed from queue, all subsequent passages shift to lower slot indices.

**Example:**
```
Initial state:
  Queue: [A, B, C]
  slot_index mapping: A→0, B→1, C→2

After A completes:
  Queue: [B, C]
  slot_index mapping: B→0, C→1  // WRONG! B and C jumped chains
```

### Required Implementation (CORRECT)

**Chains must be tracked by passage identity (queue_entry_id), not queue position.**

**Example:**
```
Initial state:
  Queue: [A(qe_123), B(qe_456), C(qe_789)]
  Chain assignment: qe_123→chain_0, qe_456→chain_1, qe_789→chain_2

After A completes:
  Queue: [B(qe_456), C(qe_789)]
  Chain assignment: qe_456→chain_1, qe_789→chain_2  // CORRECT! Same chains
  Available chains: [chain_0]

Enqueue D:
  Queue: [B(qe_456), C(qe_789), D(qe_101)]
  Chain assignment: qe_456→chain_1, qe_789→chain_2, qe_101→chain_0  // Uses freed chain
```

---

## SPEC016 Documentation Assessment

### Clear Requirements

✅ **[DBD-OV-080]:** "Each decoder-buffer chain is assigned to a passage in the queue and remains associated with that passage"
✅ **[DBD-OV-050]:** "1:1 assignment to a passage in the queue"
✅ **[DBD-OV-060]:** "First position in queue = 'now playing' passage"
✅ **[DBD-OV-070]:** "Next position in queue = 'playing next' passage"

### Missing Specifications (Documentation Gaps)

❌ **Chain Assignment Timing:** When is a chain assigned to a passage?
❌ **Chain Lifecycle:** When is a chain freed/available for reassignment?
❌ **Chain Allocation Strategy:** FIFO? Lowest available index? Specific algorithm?
❌ **Chain Tracking Mechanism:** How is passage→chain mapping maintained?

### Proposed SPEC016 Additions

Add new section **[DBD-LIFECYCLE-XXX]** Chain Assignment Lifecycle:

**[DBD-LIFECYCLE-010]** Chain assignment occurs when:
1. A passage is enqueued AND a chain is available (immediate assignment)
2. A chain becomes available AND passages are waiting (deferred assignment)

**[DBD-LIFECYCLE-020]** Chain release occurs when:
1. Passage completes playback (mixer signals completion)
2. Passage is removed from queue before playback starts (user skip)

**[DBD-LIFECYCLE-030]** Chain allocation strategy:
- Use lowest-numbered available chain (0, 1, 2, ...)
- Maintains visual consistency in developer UI

**[DBD-LIFECYCLE-040]** Chain tracking:
- PlaybackEngine maintains HashMap<QueueEntryId, ChainIndex>
- BufferManager uses QueueEntryId as primary key (already implemented)
- `get_buffer_chains()` reports stable chain indices based on HashMap

---

## Implementation Plan - 5 Phases

### Phase 1: Add Chain Tracking State (Engine)

**File:** `wkmp-ap/src/playback/engine.rs`

**Changes:**
1. Add field to `PlaybackEngine`:
   ```rust
   // Implements [DBD-LIFECYCLE-040]: Chain assignment tracking
   chain_assignments: Arc<RwLock<HashMap<Uuid, usize>>>, // queue_entry_id → chain_index
   ```

2. Add field to track available chains:
   ```rust
   // Implements [DBD-LIFECYCLE-030]: Available chain pool
   available_chains: Arc<RwLock<BinaryHeap<Reverse<usize>>>>, // Min-heap for lowest-first allocation
   ```

3. Initialize in `PlaybackEngine::new()`:
   ```rust
   // Implements [DBD-PARAM-050]: maximum_decode_streams chains (default 12)
   let mut available = BinaryHeap::new();
   for i in 0..maximum_decode_streams {
       available.push(Reverse(i));
   }
   ```

**Requirement Mappings per [CO-104]:**
- `// Implements [DBD-LIFECYCLE-040]: Passage-to-chain mapping persistence`
- `// Implements [DBD-LIFECYCLE-030]: Lowest-numbered chain allocation`
- `// Implements [DBD-PARAM-050]: maximum_decode_streams chain pool`

### Phase 2: Implement Chain Assignment Logic

**File:** `wkmp-ap/src/playback/engine.rs`

**New Methods:**

```rust
// Implements [DBD-LIFECYCLE-010]: Chain assignment on enqueue
async fn assign_chain(&self, queue_entry_id: Uuid) -> Option<usize> {
    let mut available = self.available_chains.write().await;
    if let Some(Reverse(chain_index)) = available.pop() {
        let mut assignments = self.chain_assignments.write().await;
        assignments.insert(queue_entry_id, chain_index);
        tracing::debug!(
            queue_entry_id = %queue_entry_id,
            chain_index = chain_index,
            "Assigned chain to passage"
        );
        Some(chain_index)
    } else {
        tracing::warn!(
            queue_entry_id = %queue_entry_id,
            "No available chains for assignment"
        );
        None
    }
}

// Implements [DBD-LIFECYCLE-020]: Chain release on completion
async fn release_chain(&self, queue_entry_id: Uuid) {
    let mut assignments = self.chain_assignments.write().await;
    if let Some(chain_index) = assignments.remove(&queue_entry_id) {
        let mut available = self.available_chains.write().await;
        available.push(Reverse(chain_index));
        tracing::debug!(
            queue_entry_id = %queue_entry_id,
            chain_index = chain_index,
            "Released chain from passage"
        );
    }
}
```

### Phase 3: Fix `get_buffer_chains()` to Use Chain Assignments

**File:** `wkmp-ap/src/playback/engine.rs:908`

**Current (WRONG):**
```rust
for (slot_index, entry) in all_entries.iter().take(maximum_decode_streams).enumerate() {
    let queue_position = slot_index; // Derived from enumerate - VIOLATES [DBD-OV-080]
```

**Fixed (CORRECT):**
```rust
// Implements [DBD-OV-080]: Chains remain associated with passages
let assignments = self.chain_assignments.read().await;
let mut chain_infos = Vec::with_capacity(maximum_decode_streams);

// Build BufferChainInfo for all assigned chains
for entry in all_entries.iter() {
    if let Some(&chain_index) = assignments.get(&entry.queue_entry_id) {
        // Implements [DBD-OV-050]: 1:1 passage-to-chain assignment
        let queue_position = /* calculate from queue position, NOT chain_index */;

        // ... rest of BufferChainInfo construction using chain_index (not slot_index)
        chain_infos.push((chain_index, buffer_chain_info));
    }
}

// Fill idle chains
// Implements [DBD-LIFECYCLE-030]: Report all chains 0..(maximum_decode_streams-1)
for chain_idx in 0..maximum_decode_streams {
    if !chain_infos.iter().any(|(idx, _)| *idx == chain_idx) {
        chain_infos.push((chain_idx, BufferChainInfo::idle(chain_idx)));
    }
}

// Sort by chain_index for consistent display order
chain_infos.sort_by_key(|(idx, _)| *idx);
chain_infos.into_iter().map(|(_, info)| info).collect()
```

### Phase 4: Integrate with Enqueue/Dequeue Operations

**File:** `wkmp-ap/src/playback/engine.rs`

**Update `enqueue_file()` method:**
```rust
pub async fn enqueue_file(&self, file_path: PathBuf) -> Result<Uuid> {
    // ... existing queue insertion logic ...

    // Implements [DBD-LIFECYCLE-010]: Assign chain on enqueue if available
    self.assign_chain(queue_entry_id).await;

    Ok(queue_entry_id)
}
```

**Update completion handling:**
```rust
// In playback completion handler
async fn handle_passage_completion(&self, queue_entry_id: Uuid) {
    // ... existing completion logic ...

    // Implements [DBD-LIFECYCLE-020]: Release chain on completion
    self.release_chain(queue_entry_id).await;
}
```

### Phase 5: Add Unit Tests

**File:** `wkmp-ap/tests/chain_persistence_tests.rs` (NEW)

**Test Coverage:**

1. `test_chain_assignment_on_enqueue()`
   - Enqueue 3 passages → verify chains 0, 1, 2 assigned

2. `test_chain_persistence_across_queue_advance()`
   - Enqueue A, B, C → verify A→chain_0, B→chain_1, C→chain_2
   - Advance queue (A completes) → verify B still chain_1, C still chain_2

3. `test_chain_reuse_after_completion()`
   - Enqueue A, B, C (fill chains 0, 1, 2)
   - Complete A → verify chain 0 available
   - Enqueue D → verify D assigned to chain 0 (reused)

4. `test_chain_allocation_lowest_first()`
   - Enqueue 12 passages → fill all chains
   - Complete passages at chains 5, 2, 8 → verify freed
   - Enqueue 3 more → verify assigned to chains 2, 5, 8 (ascending order)

5. `test_no_chain_when_all_allocated()`
   - Enqueue 12 passages → all chains assigned
   - Enqueue 13th passage → verify no chain assigned (queued without chain)
   - Complete one passage → verify 13th passage gets freed chain

**Requirement Mappings in Tests per [CO-104]:**
```rust
/// Tests [DBD-OV-080]: Chains remain associated with passages
#[tokio::test]
async fn test_chain_persistence_across_queue_advance() {
    // Test implementation
}

/// Tests [DBD-LIFECYCLE-020]: Chain release on completion
#[tokio::test]
async fn test_chain_reuse_after_completion() {
    // Test implementation
}
```

---

## Multi-Agent Deployment Strategy

### Agent 1: Documentation Agent
**Task:** Update SPEC016 with [DBD-LIFECYCLE-XXX] requirements
**Deliverable:** Updated SPEC016-decoder_buffer_design.md

### Agent 2: Backend Implementation Agent
**Task:** Implement Phases 1-4 (chain tracking state + logic)
**Deliverable:** Updated engine.rs with chain persistence

### Agent 3: Test Agent
**Task:** Implement Phase 5 (unit tests)
**Deliverable:** New chain_persistence_tests.rs with 5 tests

### Agent 4: Integration Agent
**Task:** Verify existing buffer_chain_monitoring_tests still pass
**Deliverable:** Validation report

### Agent 5: QA Agent
**Task:** Manual testing with developer UI
**Deliverable:** Visual verification of stable chain assignments

---

## Success Criteria

✅ All existing tests pass (37 tests)
✅ New chain persistence tests pass (5 tests)
✅ Manual verification: Enqueue 3 passages → each stays in its assigned chain
✅ Manual verification: Complete passage 1 → chain 0 frees, passages 2-3 stay in chains 1-2
✅ SPEC016 updated with lifecycle requirements
✅ All code has requirement traceability comments per [CO-104]

---

## Estimated Effort

- Phase 1 (State): 30 minutes
- Phase 2 (Assignment Logic): 45 minutes
- Phase 3 (Fix get_buffer_chains): 1 hour
- Phase 4 (Integration): 30 minutes
- Phase 5 (Tests): 1 hour
- Documentation: 30 minutes

**Total:** 4.25 hours

---

**Document Status:** Implementation Strategy
**Next Action:** Execute Phase 1 - Add Chain Tracking State
