# Specification Issues Analysis: PLAN016 Engine Refactoring

**Plan:** PLAN016
**Specification:** SPEC024-wkmp_ap_technical_debt_remediation.md
**Requirements Analyzed:** REQ-DEBT-QUALITY-002-010/020/030
**Analysis Date:** 2025-11-01

---

## Executive Summary

**Total Issues Found:** 8
- **CRITICAL:** 2 (blocks implementation without resolution)
- **HIGH:** 3 (high risk of implementation failure)
- **MEDIUM:** 2 (should resolve before implementation)
- **LOW:** 1 (minor, can address during implementation)

**Recommendation:** **PROCEED WITH CAUTION** - Critical issues require resolution during Phase 4 (Approach Selection). Issues are resolvable through code analysis and design decisions, not specification rewrites.

---

## Issues by Requirement

### REQ-DEBT-QUALITY-002-010: Module Split

**Requirement Text (SPEC024:553):**
> playback/engine.rs SHALL be split into 3 modules: engine_core, engine_queue, engine_diagnostics

#### ISSUE 001: Module Naming Inconsistency

**Category:** Ambiguity
**Severity:** MEDIUM

**Problem:**
- Requirement says "engine_core, engine_queue, engine_diagnostics" (underscore naming)
- Design diagram (SPEC024:562-567) shows "core.rs, queue.rs, diagnostics.rs" (no underscore)
- Rust convention: Module files use snake_case, which would be "core.rs" not "engine_core.rs"

**Impact:**
- Confusion about actual file names
- May create files with wrong names

**Resolution:**
Follow Rust convention + design diagram:
- Files: `core.rs`, `queue.rs`, `diagnostics.rs`
- Module names in code: `mod core;`, `mod queue;`, `mod diagnostics;`
- Requirement naming is conceptual, not literal file names

**Status:** CLARIFIED (proceed with design diagram naming)

---

#### ISSUE 002: "3 modules" Count Ambiguity

**Category:** Incompleteness
**Severity:** HIGH

**Problem:**
- Requirement says "split into 3 modules"
- Design shows 4 files: `mod.rs`, `core.rs`, `queue.rs`, `diagnostics.rs`
- Is `mod.rs` counted as a module, or just a re-export file?

**Impact:**
- Unclear if 4 files violates "3 modules" requirement
- Affects compliance verification

**Interpretation:**
- **3 functional modules:** core, queue, diagnostics
- **Plus 1 interface module:** mod.rs (re-exports only, not functional code)
- Total files: 4, but only 3 are "modules" in the requirement sense

**Resolution:**
Clarify in plan: "3 functional modules + 1 interface module (mod.rs)"

**Status:** RESOLVED BY INTERPRETATION

---

#### ISSUE 003: Module Responsibility Ambiguity

**Category:** Ambiguity
**Severity:** CRITICAL

**Problem:**
- Requirement specifies module names but not precise boundaries
- Design provides hints (SPEC024:565-567):
  - core.rs: "State management, lifecycle"
  - queue.rs: "Queue operations, enqueue/skip"
  - diagnostics.rs: "Status queries, telemetry"
- But engine.rs has 4,251 lines - unclear what goes where

**Specific Ambiguities:**
1. Where does `PlaybackEngine::new()` go? (Core or all modules need parts?)
2. Where do event emissions go? (Core or Diagnostics?)
3. Where does buffer coordination go? (Core or separate?)
4. Where do database queries go? (Each module or Core?)
5. Where do internal helper functions go? (Core or dedicated helpers module?)

**Impact:**
- Cannot plan refactoring without understanding code organization
- High risk of arbitrary/inconsistent module boundaries
- May require moving code multiple times if initial split wrong

**Resolution Required:**
**PHASE 4 (Approach Selection) MUST include:**
1. Analyze engine.rs structure → Identify functional sections
2. Map each section to target module (core/queue/diagnostics/helpers)
3. Define module responsibility matrix
4. Validate against line count constraints

**Status:** CRITICAL - BLOCKS detailed planning, resolvable in Phase 4

---

### REQ-DEBT-QUALITY-002-020: Line Count Limit

**Requirement Text (SPEC024:555):**
> Each refactored module SHALL be <1500 lines

#### ISSUE 004: "Each module" Scope Ambiguity

**Category:** Ambiguity
**Severity:** LOW

**Problem:**
- Does "each module" include `mod.rs`?
- `mod.rs` is typically <100 lines (just re-exports)
- Applying <1500 line limit to `mod.rs` is trivial

**Interpretation:**
- Limit applies to ALL 4 files: mod.rs, core.rs, queue.rs, diagnostics.rs
- `mod.rs` will naturally comply (<100 lines expected)
- Focus on core/queue/diagnostics staying under limit

**Resolution:**
Apply limit to all 4 files, but `mod.rs` is not a concern.

**Status:** RESOLVED BY INTERPRETATION

---

#### ISSUE 005: Feasibility Undefined

**Category:** Incompleteness
**Severity:** HIGH

**Problem:**
- Current engine.rs: 4,251 lines
- Target: 4 files, each <1500 lines
- Maximum capacity: 4 × 1499 = 5,996 lines
- Headroom: 5,996 - 4,251 = 1,745 lines (29% overhead allowed)

**Questions:**
1. What if code distribution is uneven? (e.g., core.rs needs 2,500 lines?)
2. What if refactoring ADDS lines (module boilerplate, visibility keywords)?
3. Is further decomposition allowed if needed? (e.g., 5+ modules?)

**Impact:**
- May discover during refactoring that 3 modules insufficient
- Requirement doesn't explicitly prohibit additional modules

**Resolution:**
**Assumption:** If any module exceeds 1499 lines, further decompose:
- Example: Split core.rs → core.rs + core_state.rs + core_lifecycle.rs
- Maintain "3 functional areas" concept, but allow sub-modules if needed
- Document deviation in implementation report

**Status:** MEDIUM - Risk identified, mitigation planned

---

### REQ-DEBT-QUALITY-002-030: API Stability

**Requirement Text (SPEC024:557):**
> Public API SHALL remain unchanged (internal refactoring only)

#### ISSUE 006: "Public API" Definition Ambiguous

**Category:** Ambiguity
**Severity:** CRITICAL

**Problem:**
- "Public API" not explicitly defined in specification
- Rust has multiple visibility levels: `pub`, `pub(crate)`, `pub(super)`, `pub(in path)`
- Does "public API" mean:
  - `pub` items only? (visible outside wkmp-ap crate)
  - `pub(crate)` items? (visible within wkmp-ap)
  - Items called by handlers.rs?

**Impact:**
- Cannot verify compliance without definition
- Risk of changing something that should be stable

**Resolution Required:**
**Define "Public API" as:**
1. All `pub` items in current engine.rs (exported from crate)
2. All items called by handlers.rs (HTTP API handlers)
3. All items called by tests (test API contract)

**Action in Phase 3:**
- Extract public API surface from current engine.rs
- Document all `pub fn`, `pub struct`, `pub enum`
- Cross-reference with handlers.rs call sites

**Status:** CRITICAL - Definition required for test specification

---

#### ISSUE 007: API Compatibility Test Inadequate

**Category:** Testability
**Severity:** HIGH

**Problem:**
Acceptance test in SPEC024:577-586:
```rust
#[test]
fn test_public_api_unchanged() {
    let engine = PlaybackEngine::new(...);
    engine.enqueue_passage(...);
    engine.skip_current();
    engine.get_status();
    // If compiles, API unchanged
}
```

**Issues with Test:**
1. Tests only 4 methods - engine has ~20+ public methods
2. "If compiles" is weak verification - doesn't test behavior
3. Doesn't verify struct fields unchanged
4. Doesn't verify return types unchanged
5. Doesn't test that ALL existing callers compile

**Impact:**
- Test may pass but API actually broken
- Incomplete verification of requirement

**Resolution:**
**Phase 3 MUST define comprehensive API test:**
- Test ALL public methods (not just 3-4)
- Verify existing integration tests compile without modification
- Verify handlers.rs compiles without changes
- Consider API snapshot testing (compare public interface before/after)

**Status:** HIGH - Test specification inadequate, must improve in Phase 3

---

## Cross-Requirement Issues

### ISSUE 008: Module Organization Not Specified

**Category:** Incompleteness
**Severity:** MEDIUM

**Problem:**
Requirements specify:
- WHAT to create (3 modules)
- HOW MANY lines (< 1500 each)
- STABILITY constraint (API unchanged)

Requirements DO NOT specify:
- Function-to-module mapping
- Inter-module communication patterns
- Shared code/helpers location
- Error handling strategy
- Testing strategy for internal modules

**Impact:**
- Leaves significant design decisions to implementer
- Risk of inconsistent or suboptimal module organization

**Resolution:**
**Phase 4 (Approach Selection) MUST address:**
1. Analyze current code structure
2. Define module responsibilities explicitly
3. Plan inter-module dependencies
4. Identify shared/helper code location
5. Document design rationale

**Status:** MEDIUM - Not a blocker, but requires design work in Phase 4

---

## Consistency Check

**Cross-Requirement Consistency:** ✅ PASS

- REQ-010 (3 modules) + REQ-020 (line limits) = Feasible (4,251 lines < 6,000 line capacity)
- REQ-010 (module split) + REQ-030 (API stable) = Compatible (internal refactoring)
- REQ-020 (line limits) + REQ-030 (API stable) = No conflict

**No contradictions found between requirements.**

---

## Testability Check

| Requirement | Testable? | Test Method | Issues |
|-------------|-----------|-------------|--------|
| REQ-010 (3 modules) | ✅ YES | Directory structure check, file count | None |
| REQ-020 (<1500 lines) | ✅ YES | `wc -l` for each file | None |
| REQ-030 (API unchanged) | ⚠️ PARTIAL | Compile test + integration tests | Test too weak (Issue 007) |

**Overall Testability:** GOOD (with Issue 007 resolved in Phase 3)

---

## Dependency Validation

**From dependencies_map.md:**

| Dependency | Exists? | Documented? | Stable? | Status |
|------------|---------|-------------|---------|--------|
| engine.rs (current) | ✅ YES | Needs analysis | In use | OK |
| handlers.rs | ✅ YES | ✅ YES | Stable | OK |
| Test suite | ✅ YES | Needs inventory | Stable | OK |
| External crates | ✅ YES | ✅ YES | Stable | OK |

**All dependencies exist and are stable.** No missing dependencies block implementation.

---

## Issues Prioritization

### CRITICAL Issues (MUST Resolve Before Implementation)

**CRITICAL-001 (Issue 003):** Module Responsibility Ambiguity
- **Impact:** Cannot plan refactoring without code analysis
- **Resolution:** Phase 4 - Analyze engine.rs, define module boundaries
- **Owner:** Phase 4 (Approach Selection)

**CRITICAL-002 (Issue 006):** Public API Definition Ambiguous
- **Impact:** Cannot verify API stability without definition
- **Resolution:** Phase 3 - Extract public API from code, document explicitly
- **Owner:** Phase 3 (Test Definition)

### HIGH Issues (High Risk Without Resolution)

**HIGH-001 (Issue 002):** "3 modules" vs. 4 files
- **Impact:** May fail compliance check
- **Resolution:** Clarify interpretation (3 functional + 1 interface)
- **Status:** RESOLVED BY INTERPRETATION

**HIGH-002 (Issue 005):** Feasibility if modules exceed limit
- **Impact:** May need more than 3 modules
- **Resolution:** Allow further decomposition if needed, document deviation
- **Status:** RISK IDENTIFIED, MITIGATION PLANNED

**HIGH-003 (Issue 007):** API Test Inadequate
- **Impact:** May not catch API breakage
- **Resolution:** Phase 3 - Define comprehensive API test suite
- **Owner:** Phase 3 (Test Definition)

### MEDIUM Issues (Should Resolve)

**MEDIUM-001 (Issue 001):** Module Naming
- **Resolution:** Use Rust conventions (core.rs not engine_core.rs)
- **Status:** CLARIFIED

**MEDIUM-002 (Issue 008):** Module Organization Not Specified
- **Resolution:** Phase 4 - Define explicitly during approach selection
- **Owner:** Phase 4

### LOW Issues (Minor)

**LOW-001 (Issue 004):** "Each module" includes mod.rs
- **Resolution:** Apply limit to all files, mod.rs trivially complies
- **Status:** RESOLVED BY INTERPRETATION

---

## Decision: Proceed or Stop?

**Analysis:**
- 2 CRITICAL issues identified
- Both are RESOLVABLE through normal planning process:
  - CRITICAL-001: Resolved in Phase 4 (code analysis)
  - CRITICAL-002: Resolved in Phase 3 (API extraction)
- Issues do NOT require specification rewrites
- Issues do NOT indicate fundamental problems with requirements

**Decision:** **PROCEED TO PHASE 3**

**Rationale:**
- Critical issues are analysis/design tasks, not specification defects
- Specification provides sufficient direction
- Detailed boundaries defined during implementation planning (expected)
- Phase 3 and Phase 4 will resolve all critical issues

---

## Next Steps

**Phase 3 Actions:**
1. Define comprehensive API stability tests (resolves CRITICAL-002, HIGH-003)
2. Extract public API surface from current engine.rs
3. Document all public methods, structs, fields
4. Create test specifications for module structure, line counts, API stability

**Phase 4 Actions:**
1. Analyze engine.rs code structure (resolves CRITICAL-001, MEDIUM-002)
2. Map functions to modules (core/queue/diagnostics)
3. Define module boundaries and responsibilities
4. Plan for potential additional modules if needed (addresses HIGH-002)

---

**Specification Issues Analysis Complete**
**Phase 2 Status:** ✓ All requirements analyzed
**Recommendation:** PROCEED TO PHASE 3
