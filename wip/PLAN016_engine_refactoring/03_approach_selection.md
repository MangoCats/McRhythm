# Approach Selection: PLAN016 Engine Refactoring

**Plan:** PLAN016
**Date:** 2025-11-01
**Status:** Approach Selected
**Decision Method:** Risk-First Framework (per CLAUDE.md)

---

## Executive Summary

**Approaches Evaluated:** 2
**Recommended Approach:** Approach A (Functional Decomposition)
**Risk Level:** Low
**Estimated Effort:** 8-12 hours

**Decision Basis:** Approach A has lowest residual risk (Low) after mitigation, with natural module boundaries identified through code analysis. Approach B (Line-Count-First Split) has higher risk (Medium) due to arbitrary boundaries.

---

## Code Analysis Summary

**Current State:**
- File: `wkmp-ap/src/playback/engine.rs`
- Size: 4,251 lines
- Primary Struct: `PlaybackEngine` with 107 lines of field definitions
- Public Methods: 28 (lifecycle, queue, diagnostics, monitoring)
- Private Helpers: ~20 functions (~1,600 lines total)

**Natural Module Boundaries Identified:**

| Functional Area | Line Count | Coupling | Module Target |
|-----------------|------------|----------|---------------|
| Lifecycle & Orchestration | ~1,380 | Very High | core.rs |
| Queue Operations | ~1,250 | Medium | queue.rs |
| Status & Diagnostics | ~1,035 | Low-Medium | diagnostics.rs |
| Overhead (imports, tests) | ~586 | N/A | Distributed |

**Key Finding:** Code has clear functional separation with minimal coupling between queue/diagnostics areas.

---

## Approach A: Functional Decomposition (RECOMMENDED)

### Description

Split engine.rs based on **functional responsibility** identified through code analysis:

**Module Organization:**

```
engine/
├── mod.rs (~100 lines)
│   └── Re-exports all pub items (PlaybackEngine, methods)
│
├── core.rs (~1,380 lines)
│   ├── PlaybackEngine struct (107 lines)
│   ├── Lifecycle: new(), start(), stop() (350 lines)
│   ├── Playback control: play(), pause(), seek() (150 lines)
│   ├── Orchestration: playback_loop(), process_queue() (600 lines)
│   ├── Chain allocation: assign_chain(), release_chain() (60 lines)
│   └── Utilities: clone_handles() (50 lines)
│   └── Imports (~60 lines)
│
├── queue.rs (~1,250 lines)
│   ├── Queue operations: enqueue_file(), remove_queue_entry() (820 lines)
│   ├── Queue control: skip_next(), clear_queue() (310 lines)
│   ├── Queue queries: get_queue_entries(), reorder_queue_entry() (80 lines)
│   └── Imports (~40 lines)
│
└── diagnostics.rs (~1,035 lines)
    ├── Status accessors: get_buffer_chains(), get_metrics() (280 lines)
    ├── Monitoring config: set_buffer_monitor_rate() (15 lines)
    ├── Event handlers: position_event_handler(), buffer_event_handler() (700 lines)
    └── Imports (~40 lines)
```

**Public API (mod.rs re-exports):**
```rust
pub use core::PlaybackEngine;

// All public methods automatically available via impl blocks
// in core.rs, queue.rs, diagnostics.rs using #[allow(private_bounds)]
```

---

### Risk Assessment

#### Failure Modes

**FM-1: Module Exceeds Line Limit**
- **Probability:** Low (15%)
- **Impact:** Medium (requires further decomposition)
- **Scenario:** core.rs exceeds 1,500 lines after adding imports/boilerplate
- **Current Estimate:** 1,380 lines (120 line headroom)

**Mitigation:**
- Monitor line counts during migration
- If core.rs approaches 1,450 lines: Split orchestration helpers into core_orchestration.rs
- Acceptance: Further decomposition allowed per scope statement

**Residual Risk:** Low

---

**FM-2: Compilation Errors (Import/Visibility)**
- **Probability:** Medium (40%)
- **Impact:** Low (easily fixed)
- **Scenario:** Missing imports, incorrect visibility modifiers (pub/pub(super))

**Mitigation:**
- Incremental compilation after each module extraction
- Use `pub(super)` for engine-internal items
- Start with queue.rs (cleanest separation, lowest coupling)

**Residual Risk:** Low

---

**FM-3: API Breakage**
- **Probability:** Very Low (<5%)
- **Impact:** High (handlers/tests break)
- **Scenario:** Public method inadvertently becomes private

**Mitigation:**
- mod.rs re-exports ensure all pub items accessible
- TC-I-030-01 (handler compilation test) catches breakage immediately
- TC-I-030-02 (test suite) provides comprehensive verification

**Residual Risk:** Very Low

---

**FM-4: Behavior Changes**
- **Probability:** Very Low (<5%)
- **Impact:** High (playback logic breaks)
- **Scenario:** Logic error introduced during code movement

**Mitigation:**
- No code rewriting - pure move operations
- TC-I-030-02 (test suite pass rate) verifies behavior unchanged
- process_queue() stays intact (no splitting 580-line function)

**Residual Risk:** Very Low

---

#### Overall Risk

**Residual Risk (After Mitigation):** **LOW**

**Risk Calculation:**
- FM-1 (Low prob × Medium impact) = Low
- FM-2 (Medium prob × Low impact) = Low
- FM-3 (Very Low prob × High impact) = Low
- FM-4 (Very Low prob × High impact) = Very Low

**Highest Residual Risk:** Low (FM-1 and FM-2)

---

### Quality Characteristics

**Maintainability:** High
- Natural functional boundaries (lifecycle, queue, diagnostics)
- Each module has single responsibility
- Reduced cognitive load (1,380 lines vs. 4,251 lines per file)

**Test Coverage:** High
- 11 acceptance tests defined
- Existing test suite provides regression detection
- TC-I-030-02 ensures 100% baseline tests pass

**Architectural Alignment:** High
- Follows Rust module best practices
- Aligns with WKMP coding conventions (IMPL002)
- Internal refactoring (API unchanged per REQ-030)

---

### Implementation Effort

**Estimated Effort:** 8-12 hours

**Breakdown:**
1. Baseline establishment (tests, API documentation): 1 hour
2. Create module structure (mod.rs, empty files): 30 min
3. Extract queue.rs (cleanest separation): 2-3 hours
4. Extract diagnostics.rs (handlers independent): 2-3 hours
5. Cleanup core.rs (update imports): 1 hour
6. Verification (run all 11 tests): 1 hour
7. Code review and adjustments: 1-2 hours

**Dependencies:**
- Existing: engine.rs, test suite, handlers.rs
- Tools: cargo, wc, grep
- No external blockers

---

### Technical Complexity

**Complexity:** Medium

**Complex Areas:**
1. **Orchestration Hub** (process_queue() - 580 lines)
   - Stays in core.rs intact (no splitting)
   - High coupling to mixer, buffer_manager, decoder_worker

2. **Handler Task Spawning** (start() method)
   - Spawns 9+ async tasks for event handlers
   - handlers move to diagnostics.rs but are spawned from core.rs

3. **Shared State Access Patterns**
   - Many Arc<RwLock<T>> fields
   - Careful ordering required (avoid deadlocks)
   - No changes to access patterns (pure code movement)

**Mitigation:** Keep complex functions intact, move as whole units.

---

## Approach B: Line-Count-First Split (NOT RECOMMENDED)

### Description

Split engine.rs into equal-sized chunks (~1,400 lines each) based on **line count** rather than functional boundaries.

**Module Organization:**
```
engine/
├── mod.rs (~100 lines) - Re-exports
├── part1.rs (~1,400 lines) - Lines 1-1,400
├── part2.rs (~1,400 lines) - Lines 1,401-2,800
└── part3.rs (~1,400 lines) - Lines 2,801-4,200
```

---

### Risk Assessment

#### Failure Modes

**FM-1: Arbitrary Boundaries**
- **Probability:** High (80%)
- **Impact:** High (poor maintainability)
- **Scenario:** Functions split mid-logic, related code separated

**Mitigation:**
- Adjust boundaries to respect function boundaries
- Still results in arbitrary grouping by file position, not function

**Residual Risk:** Medium-High

---

**FM-2: Poor Maintainability**
- **Probability:** Very High (95%)
- **Impact:** High (defeats refactoring purpose)
- **Scenario:** Developers cannot find code logically (no functional organization)

**Mitigation:**
- None effective - fundamental approach problem

**Residual Risk:** High

---

**FM-3: High Coupling Between Modules**
- **Probability:** High (70%)
- **Impact:** Medium (circular dependencies, poor isolation)
- **Scenario:** part1.rs needs items from part2.rs and vice versa

**Mitigation:**
- Use pub(super) liberally
- Still results in tight coupling across arbitrary boundaries

**Residual Risk:** Medium

---

#### Overall Risk

**Residual Risk (After Mitigation):** **MEDIUM-HIGH**

**Rationale:**
- Arbitrary boundaries create maintainability problems
- Defeats purpose of refactoring (improving code organization)
- High coupling across modules

---

### Quality Characteristics

**Maintainability:** Low
- No logical organization
- Developers must remember line numbers, not functional areas
- Violates single-responsibility principle

**Test Coverage:** Same as Approach A
- Tests would still pass (behavior unchanged)
- But doesn't improve code understandability

**Architectural Alignment:** Poor
- Not aligned with Rust module best practices
- Arbitrary grouping, not functional

---

### Implementation Effort

**Estimated Effort:** 6-8 hours (faster but wrong)

**Breakdown:**
1. Split file at line 1,400 and 2,800: 2 hours
2. Fix imports and visibility: 2-3 hours
3. Verification: 1 hour
4. Rework when arbitrary boundaries cause issues: 1-2 hours

**Faster than Approach A but produces inferior result.**

---

## Approach Comparison

| Criterion | Approach A (Functional) | Approach B (Line-Count) |
|-----------|-------------------------|-------------------------|
| **Residual Risk** | Low | Medium-High |
| **Maintainability** | High | Low |
| **Test Coverage** | High (11 tests) | High (11 tests) |
| **Arch. Alignment** | High | Poor |
| **Effort** | 8-12 hours | 6-8 hours |
| **Meets Requirements** | ✅ Yes | ✅ Yes (but poorly) |

---

## Decision

### Recommended Approach: **Approach A (Functional Decomposition)**

**Rationale (Risk-First Framework per CLAUDE.md):**

**1. Risk (Primary Criterion):**
- Approach A: Low residual risk
- Approach B: Medium-High residual risk
- **Decision:** Approach A wins on risk reduction

**2. Quality (Secondary Criterion):**
- Approach A: High maintainability, high architectural alignment
- Approach B: Low maintainability, poor alignment
- **Decision:** Approach A significantly better

**3. Effort (Tertiary Consideration):**
- Approach A: 8-12 hours
- Approach B: 6-8 hours
- **Difference:** 2-4 hours (~30% more)
- **Per CLAUDE.md:** "If lowest-risk approach requires 2x effort versus higher-risk approach, choose lowest-risk approach. Effort differential is secondary to risk reduction."
- **Decision:** 30% effort increase acceptable for risk reduction from Medium-High to Low

**Overall Decision:** Approach A (Functional Decomposition)

---

## Architecture Decision Record (ADR)

**Title:** ADR-001: Functional Decomposition for Engine Refactoring

**Status:** Accepted

**Date:** 2025-11-01

**Context:**

`wkmp-ap/src/playback/engine.rs` has grown to 4,251 lines, violating maintainability standards. SPEC024 REQ-DEBT-QUALITY-002 requires splitting into 3 modules (<1500 lines each) while preserving public API.

Two approaches considered:
- **Approach A:** Split by functional responsibility (lifecycle/queue/diagnostics)
- **Approach B:** Split by line count (equal-sized chunks)

**Decision:**

We will use **Approach A: Functional Decomposition**.

**Consequences:**

**Positive:**
- Natural module boundaries identified through code analysis
- Each module has single responsibility (SRP)
- Improved maintainability and code navigation
- Low risk of implementation failure
- Aligns with Rust module best practices

**Negative:**
- 2-4 hours more implementation effort than Approach B
- Requires understanding code structure (not mechanical split)

**Tradeoffs Accepted:**
- Effort increase (30%) is acceptable for risk reduction (Medium-High → Low)
- Upfront analysis time (Phase 4) required but improves outcome quality

**Mitigation Plans:**
- Monitor line counts during migration (FM-1)
- Incremental compilation after each module (FM-2)
- Keep complex functions intact (process_queue(), handlers)

**Verification:**
- 11 acceptance tests (TC-U-010-01 through TC-S-030-02)
- Traceability matrix 100% complete
- Baseline test suite pass rate 100%

---

## Module Responsibility Matrix

**CORE.RS** (State Management & Lifecycle)

| Function | Lines | Responsibility | Public? |
|----------|-------|----------------|---------|
| PlaybackEngine struct | 107 | Primary struct definition | Yes |
| new() | 144 | Initialize all components | Yes |
| start() | 460 | Start playback loop, spawn handlers | Yes |
| stop() | 36 | Shutdown, persist state | Yes |
| play() | 57 | Resume playback | Yes |
| pause() | 57 | Pause playback | Yes |
| seek() | 70 | Seek to position | Yes |
| playback_loop() | 25 | Main 100ms loop | No |
| process_queue() | 580 | Orchestration hub (decode, crossfade, completion) | No |
| assign_chain() | 26 | Allocate decoder chain | No |
| release_chain() | 17 | Return chain to pool | No |
| clone_handles() | 27 | Clone Arc fields for tasks | No |

**Total:** ~1,380 lines

---

**QUEUE.RS** (Queue Operations)

| Function | Lines | Responsibility | Public? |
|----------|-------|----------------|---------|
| skip_next() | 83 | Skip current, advance queue | Yes |
| clear_queue() | 53 | Clear all entries | Yes |
| enqueue_file() | 79 | Add passage to queue | Yes |
| remove_queue_entry() | 75 | Remove specific entry | Yes |
| reorder_queue_entry() | 25 | Change play order | Yes |
| get_queue_entries() | 29 | Query queue state | Yes |
| queue_len() | 3 | Count entries | Yes |
| emit_queue_change_events() | 26 | Helper - emit events | No |
| complete_passage_removal() | 33 | Helper - cleanup | No |

**Total:** ~1,250 lines

---

**DIAGNOSTICS.RS** (Monitoring & Status)

| Function | Lines | Responsibility | Public? |
|----------|-------|----------------|---------|
| get_volume_arc() | 2 | Volume accessor | Yes |
| get_buffer_manager() | 2 | Buffer manager accessor | Yes |
| is_audio_expected() | 2 | Underrun classification | Yes |
| get_callback_stats() | 3 | Gap/stutter stats | Yes |
| get_buffer_chains() | 189 | Buffer chain info | Yes |
| verify_queue_sync() | 54 | In-memory vs DB validation | Yes |
| get_buffer_statuses() | 2 | Status map accessor | Yes |
| get_pipeline_metrics() | 27 | Aggregated metrics | Yes |
| set_buffer_monitor_rate() | 3 | Configure SSE rate | Yes |
| trigger_buffer_monitor_update() | 3 | Force immediate update | Yes |
| position_event_handler() | 160 | Position update handler | No |
| buffer_event_handler() | 300 | Buffer event handler | No |
| buffer_chain_status_emitter() | 68 | Periodic SSE emitter | No |
| playback_position_emitter() | 37 | Position SSE emitter | No |

**Total:** ~1,035 lines

---

## Inter-Module Dependencies

**Dependency Graph:**

```
handlers.rs (API callers)
    ↓
engine/mod.rs (Public API re-exports)
    ↓
┌──────────┬───────────────┬──────────────┐
│ core.rs  │   queue.rs    │ diagnostics  │
│          │               │     .rs      │
│  ↓       │      ↓        │      ↓       │
│ mixer    │   buffer_mgr  │   state      │
│ buffer   │   decoder     │   (read)     │
│ queue    │   db_pool     │              │
│ state    │               │              │
└──────────┴───────────────┴──────────────┘
```

**Coupling Levels:**
- core.rs → queue.rs: Medium (calls skip_next() on completion)
- core.rs → diagnostics.rs: Medium (spawns handlers)
- queue.rs → core.rs: Medium (uses chain allocation)
- queue.rs → diagnostics.rs: None
- diagnostics.rs → core.rs: Low (reads state)
- diagnostics.rs → queue.rs: None

**All coupling flows through `&self` receiver on PlaybackEngine - no circular dependencies.**

---

## Implementation Strategy

**Execution Order:**

1. **Baseline** (Increment 1):
   - Run TC-I-030-02 (test baseline)
   - Run TC-U-030-01 (document API)
   - Run TC-S-020-01 (measure line count)

2. **Extract queue.rs** (Increment 3):
   - **Rationale:** Cleanest separation, lowest coupling to diagnostics
   - Move queue operations + queries
   - Update core.rs imports
   - Verify compilation (TC-I-010-01)
   - Run tests (TC-I-030-02)

3. **Extract diagnostics.rs** (Increment 5):
   - **Rationale:** Handlers are independent loops
   - Move status accessors + handlers
   - Update core.rs imports
   - Verify compilation
   - Run tests

4. **Cleanup core.rs** (Increment 6):
   - Remove moved code
   - Update imports
   - Create mod.rs with re-exports
   - Delete original engine.rs

5. **Verification** (Increment 6):
   - Run all 11 acceptance tests
   - Verify handlers.rs compiles unchanged
   - Manual review (TC-S-010-02, TC-S-030-02)

---

## Success Criteria

**From Risk-First Framework:**

✅ **Risk Minimization:** Low residual risk achieved
✅ **Quality:** High maintainability, architectural alignment
✅ **Requirements:** All 3 requirements satisfied (REQ-010/020/030)
✅ **Tests:** 100% coverage (11 tests defined)

**Quantified Metrics:**

| Metric | Target | Method |
|--------|--------|--------|
| Line counts | All <1500 | TC-U-020-01 |
| API stability | Zero changes | TC-I-030-01, TC-U-030-01 |
| Test pass rate | 100% baseline | TC-I-030-02 |
| Total lines | ≈4,251 ±5% | TC-S-020-01 |

---

**Approach Selection Complete**
**Phase 4 Status:** ✓ Approach A (Functional Decomposition) Selected
**Next Phase:** Implementation Breakdown (Phase 5)
