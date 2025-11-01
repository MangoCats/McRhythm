# Dependencies Map: PLAN016 Engine Refactoring

**Plan:** PLAN016
**Feature:** Engine Refactoring
**Date:** 2025-11-01

---

## Dependency Categories

### 1. Existing Code Dependencies (Read-Only)

These components exist and MUST NOT be modified (only analyzed):

| Component | Path | Lines | Status | Usage |
|-----------|------|-------|--------|-------|
| engine.rs (current) | wkmp-ap/src/playback/engine.rs | 4,251 | Implemented | Source code to be refactored |
| mixer.rs | wkmp-ap/src/playback/mixer.rs | 866 | Refactored (PLAN014) | Called by engine |
| buffer_manager.rs | wkmp-ap/src/playback/buffer_manager.rs | ~1000 | Implemented | Managed by engine |
| queue_manager.rs | wkmp-ap/src/playback/queue_manager.rs | ~500 | Implemented | Managed by engine |
| decoder_pool.rs | wkmp-ap/src/playback/pipeline/decoder_pool.rs | ~800 | Implemented | Managed by engine |
| handlers.rs | wkmp-ap/src/api/handlers.rs | 1,305 | Implemented | Calls PlaybackEngine API |

**Action Required:**
- Analyze engine.rs structure before refactoring
- Identify public API surface (used by handlers)
- Map internal dependencies (mixer, buffer_manager, queue_manager)

---

### 2. Test Dependencies

Test files that define API contract:

| Test File | Type | Purpose | Impact |
|-----------|------|---------|--------|
| integration_tests.rs | Integration | Full playback scenarios | MUST pass unchanged |
| mixer_tests/ | Unit | Mixer functionality | Should be unaffected |
| decoder_pool_tests.rs | Unit | Decoder functionality | Should be unaffected |
| API tests (if any) | Integration | HTTP handlers | MUST pass unchanged |

**Verification Plan:**
1. Run `cargo test -p wkmp-ap` BEFORE refactoring → Establish baseline
2. Run `cargo test -p wkmp-ap` AFTER refactoring → Verify no regressions
3. Document which tests exercise PlaybackEngine API

---

### 3. External Libraries (No Changes Expected)

| Crate | Version | Usage in Engine | Status |
|-------|---------|-----------------|--------|
| tokio | 1.35 | Async runtime, RwLock, broadcast channels | Stable |
| sqlx | 0.7 | Database queries (passages, queue) | Stable |
| uuid | 1.6 | Entity IDs (passage_id, queue_entry_id) | Stable |
| tracing | 0.1 | Logging and diagnostics | Stable |
| anyhow | 1.0 | Error handling | Stable |

**No new crates will be added.**

---

### 4. Internal Module Dependencies

Modules that engine.rs interacts with:

#### Outbound Dependencies (Engine → Other Modules)

| Module | Interaction | Nature | Refactoring Impact |
|--------|-------------|--------|-------------------|
| mixer.rs | `Arc<RwLock<Mixer>>` | Engine owns mixer instance | Low (internal field, not public API) |
| buffer_manager.rs | `Arc<BufferManager>` | Engine manages buffers | Low (internal coordination) |
| queue_manager.rs | `Arc<RwLock<QueueManager>>` | Engine manages queue | Low (internal coordination) |
| decoder_pool.rs | `Arc<DecoderPool>` | Engine coordinates decoding | Low (internal coordination) |
| events.rs | Emits `PlaybackEvent` | Engine emits events via broadcast | Low (internal implementation) |

**Key Insight:** All internal dependencies are via `Arc<RwLock<T>>` - refactoring engine internals should not affect these.

#### Inbound Dependencies (Other Modules → Engine)

| Caller | Methods Called | API Surface | Refactoring Impact |
|--------|----------------|-------------|-------------------|
| handlers.rs | `enqueue_passage()`, `skip_current()`, `get_status()`, etc. | Public API | HIGH (MUST preserve) |
| Tests | All public methods | Public API | HIGH (MUST preserve) |
| main.rs | `PlaybackEngine::new()` | Constructor | HIGH (MUST preserve) |

**Critical:** Public API MUST remain unchanged → REQ-DEBT-QUALITY-002-030

---

### 5. Database Dependencies

Engine interacts with SQLite database:

| Query Type | Tables Accessed | Usage | Impact |
|------------|-----------------|-------|--------|
| Passage queries | passages, passage_timing, passage_albums | Load passage metadata | No schema changes |
| Queue queries | queue, queue_entries | Manage playback queue | No schema changes |
| Settings queries | settings | Load configuration | No schema changes |

**No database schema changes required.**

---

### 6. Configuration Dependencies

Engine loads settings from database:

| Setting | Table | Usage | Impact |
|---------|-------|-------|--------|
| master_volume | settings | Audio output volume | Loaded in engine, no change |
| position_update_interval_ms | settings | SSE update frequency | Loaded in engine, no change |
| working_sample_rate | settings | Audio pipeline config | Loaded in engine, no change |

**No configuration changes required.**

---

## Dependency Analysis Summary

### Dependencies WITH Impact (Require Attention)

1. **handlers.rs → PlaybackEngine (HIGH IMPACT)**
   - Public API MUST be preserved exactly
   - All handler call sites must compile without changes
   - Action: Extract public API surface, document in `mod.rs`

2. **Tests → PlaybackEngine (HIGH IMPACT)**
   - Test compilation defines API contract
   - All tests must pass without modification
   - Action: Run full test suite before/after refactoring

3. **engine.rs Internal Structure (MEDIUM IMPACT)**
   - Current organization unknown (requires analysis)
   - May have tight coupling between functions
   - Action: Analyze code sections, plan module boundaries

### Dependencies WITHOUT Impact (Transparent)

1. **Internal modules (mixer, buffer_manager, queue_manager)**
   - Engine refactoring is internal implementation
   - No changes to how engine interacts with these modules
   - Status: Safe to ignore during refactoring

2. **External crates (tokio, sqlx, uuid, etc.)**
   - No version changes
   - No API changes
   - Status: Transparent to refactoring

3. **Database schema**
   - No table changes
   - No query changes
   - Status: Transparent to refactoring

---

## Dependency Graph

```
handlers.rs (EXTERNAL - MUST NOT CHANGE CALLS)
    ↓
engine/mod.rs (PUBLIC API - re-exports)
    ↓
engine/core.rs (State, lifecycle)
    ↓
engine/queue.rs (Queue operations)
    ↓
engine/diagnostics.rs (Status, telemetry)
    ↓
Internal modules (mixer, buffer_manager, etc.) - Unchanged
```

**Key Principle:** Public API surface defined by `mod.rs` re-exports. Internal modules can call each other via `pub(super)` visibility.

---

## Critical Path

**Dependency Resolution Order:**

1. ✅ **Phase 1 (Current):** Identify dependencies
2. **Phase 2:** Analyze engine.rs structure → Identify functional sections
3. **Phase 3:** Define acceptance tests → Public API contract
4. **Phase 4:** Plan module boundaries → Minimize inter-module coupling
5. **Phase 5:** Implement refactoring → Move code to modules
6. **Phase 6:** Verify tests → Ensure API unchanged

**Blocker Resolution:**

| Blocker | Impact | Resolution |
|---------|--------|------------|
| High coupling in engine.rs | Cannot cleanly separate | Introduce helper modules or shared utilities |
| Unknown public API surface | May break callers | Extract API from handlers.rs call sites |
| Insufficient test coverage | Cannot verify correctness | Document risk, proceed with manual testing |

---

**Dependencies Map Complete**
**Phase 1.4 Status:** ✓ All dependencies identified and cataloged
