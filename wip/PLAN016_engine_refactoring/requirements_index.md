# Requirements Index: PLAN016 Engine Refactoring

**Source:** wip/SPEC024-wkmp_ap_technical_debt_remediation.md
**Section:** DEBT-QUALITY-002 Large Monolithic Files
**Extracted:** 2025-11-01

---

## Requirements Summary

**Total Requirements:** 3 (all P1 - High Priority)

| Req ID | Type | Brief Description | Source Line | Priority | Quantified |
|--------|------|-------------------|-------------|----------|------------|
| REQ-DEBT-QUALITY-002-010 | Structural | Split engine.rs into 3 modules: engine_core, engine_queue, engine_diagnostics | 553 | P1 | Yes (3 modules) |
| REQ-DEBT-QUALITY-002-020 | Quality | Each refactored module SHALL be <1500 lines | 555 | P1 | Yes (<1500) |
| REQ-DEBT-QUALITY-002-030 | API Stability | Public API SHALL remain unchanged (internal refactoring only) | 557 | P1 | Yes (no API changes) |

---

## Requirement Details

### REQ-DEBT-QUALITY-002-010: Module Split

**Source:** SPEC024 line 553

**Full Text:**
> playback/engine.rs SHALL be split into 3 modules: engine_core, engine_queue, engine_diagnostics

**Interpretation:**
- Current: Single file `wkmp-ap/src/playback/engine.rs` (4,251 lines as of 2025-11-01)
- Target: 3 separate modules under `wkmp-ap/src/playback/engine/`
  - `mod.rs` - Re-exports public API
  - `core.rs` - State management, lifecycle
  - `queue.rs` - Queue operations, enqueue/skip
  - `diagnostics.rs` - Status queries, telemetry

**Acceptance Criteria:**
- Directory structure matches design (SPEC024:562-567)
- All functionality preserved
- No code duplication across modules

---

### REQ-DEBT-QUALITY-002-020: Line Count Limit

**Source:** SPEC024 line 555

**Full Text:**
> Each refactored module SHALL be <1500 lines

**Interpretation:**
- Maximum file size: 1499 lines per module
- Applies to: `mod.rs`, `core.rs`, `queue.rs`, `diagnostics.rs`
- Current state: Single 4,251-line file MUST decompose to 4 files, each <1500 lines

**Quantified Target:**
- `mod.rs`: <1500 lines (likely <100 lines - just re-exports)
- `core.rs`: <1500 lines
- `queue.rs`: <1500 lines
- `diagnostics.rs`: <1500 lines

**Acceptance Criteria:**
- Run `wc -l` on each file
- All files < 1500 lines
- Total lines approximately equal to original (allowing for small overhead)

---

### REQ-DEBT-QUALITY-002-030: API Stability

**Source:** SPEC024 line 557

**Full Text:**
> Public API SHALL remain unchanged (internal refactoring only)

**Interpretation:**
- External callers (handlers, tests, other modules) use PlaybackEngine without code changes
- Public struct fields, public methods, function signatures unchanged
- Internal implementation may change completely
- Re-exports in `mod.rs` make refactoring transparent

**Acceptance Criteria:**
- All existing tests compile without modification
- API handlers compile without modification
- Public interface matches original exactly

---

## Requirements Not Included

**Out of Scope for PLAN016:**
- REQ-DEBT-SEC-001-xxx (Authentication bypass) - Different plan
- REQ-DEBT-FUNC-001-xxx through REQ-DEBT-FUNC-005-xxx (Functional debt) - Different plan
- REQ-DEBT-QUALITY-001-xxx (.unwrap() reduction) - Different plan
- REQ-DEBT-QUALITY-003-xxx (Compiler warnings) - Different plan
- REQ-DEBT-QUALITY-004-xxx (Duplicate config files) - Different plan
- REQ-DEBT-QUALITY-005-xxx (Backup files) - Different plan

**Rationale:** Focused plan for single refactoring task. Other debt items require separate planning and implementation.

---

## Traceability

| Requirement | Test IDs | Implementation Files | Status |
|-------------|----------|---------------------|--------|
| REQ-DEBT-QUALITY-002-010 | TBD | engine/mod.rs, core.rs, queue.rs, diagnostics.rs | Pending |
| REQ-DEBT-QUALITY-002-020 | TBD | (all 4 modules) | Pending |
| REQ-DEBT-QUALITY-002-030 | TBD | engine/mod.rs (public API) | Pending |

Test IDs will be assigned in Phase 3.

---

**Requirements Index Complete**
**Phase 1.2 Status:** ✓ All requirements extracted and cataloged
