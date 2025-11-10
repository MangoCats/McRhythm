# Implementation Plan: Technical Debt Reduction

**Project:** WKMP Technical Debt Reduction
**Status:** PLAN COMPLETE - Ready for Implementation
**Generated:** 2025-11-10
**Total Estimated Effort:** 4-6 weeks

---

## Executive Summary

**Objective:** Systematically eliminate technical debt in wkmp-ai and wkmp-common modules through 5 phased increments.

**Scope:**
- 26 requirements across 5 phases
- 506 unwrap() calls to audit and reduce to <50
- 2,253-line monolithic file to split into 7-8 modules
- Zero compiler/clippy warnings target
- 100% public API documentation

**Key Constraints:**
- ✅ All 216 tests MUST pass after each increment
- ✅ Backward compatibility REQUIRED (no breaking changes)
- ✅ Incremental delivery (each phase independently valuable)

**Success Metrics:**
- Zero compiler warnings (from 11)
- Zero clippy warnings (from 22)
- <50 unwrap() calls in production (from 506, >90% reduction)
- All files <800 lines (largest currently 2,253 lines)
- 100% public API documented (currently ~55%)

---

## Table of Contents

1. [Phase 1: Quick Wins](#phase-1-quick-wins)
2. [Phase 2: File Organization](#phase-2-file-organization)
3. [Phase 3: Error Handling](#phase-3-error-handling)
4. [Phase 4: Documentation](#phase-4-documentation)
5. [Phase 5: Code Quality](#phase-5-code-quality)
6. [Effort Estimates](#effort-estimates)
7. [Risk Analysis](#risk-analysis)
8. [Implementation Checklist](#implementation-checklist)
9. [Approval and Sign-Off](#approval-and-sign-off)

---

## Phase 1: Quick Wins

**Objective:** Eliminate critical issues and auto-fixable warnings
**Duration:** 1-2 days
**Priority:** CRITICAL

### Increment 1.1: Replace Blocking Sleep (REQ-TD1-001)

**Effort:** 15 minutes

**Steps:**
1. Open `wkmp-common/src/time.rs`
2. Locate `std::thread::sleep` call (line ~37)
3. Replace with:
   ```rust
   tokio::time::sleep(Duration::from_millis(10)).await
   ```
4. Update imports: `use tokio::time::sleep;`
5. Test: `cargo test -p wkmp-common test_now_successive_calls_advance`
6. Commit: "Fix blocking sleep in async test context"

**Acceptance Test:** AT-TD1-001

---

### Increment 1.2: Delete Dead Code (REQ-TD1-002)

**Effort:** 15 minutes

**Steps:**
1. Verify no references: `grep -rn "song_processor" wkmp-ai/src/`
2. Remove comment from `wkmp-ai/src/workflow/mod.rs:11`
3. Delete file: `git rm wkmp-ai/src/workflow/song_processor.rs`
4. Build: `cargo build -p wkmp-ai`
5. Test: `cargo test -p wkmp-ai`
6. Commit: "Remove dead code: song_processor.rs (-368 lines)"

**Acceptance Test:** AT-TD1-002

---

### Increment 1.3: Fix Compiler Warnings (REQ-TD1-003)

**Effort:** 1-2 hours

**Steps:**
1. Build and capture warnings:
   ```bash
   cargo build -p wkmp-ai -p wkmp-common 2>&1 | tee /tmp/warnings.txt
   ```
2. Fix each warning per specification table:
   - Unused imports: Delete
   - Unused variables: Prefix with `_`
   - Dead code fields: Remove or add `#[allow(dead_code)]` with justification
3. Build after each fix to verify
4. Final verification:
   ```bash
   cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "warning:" | wc -l
   # Expected: 0
   ```
5. Commit: "Fix all compiler warnings (11 warnings eliminated)"

**Acceptance Tests:** AT-TD1-003a, AT-TD1-003b

---

### Increment 1.4: Fix Clippy Lints (REQ-TD1-004)

**Effort:** 1-2 hours

**Steps:**
1. Run auto-fix:
   ```bash
   cargo clippy --fix --lib -p wkmp-ai --allow-dirty
   cargo clippy --fix --lib -p wkmp-common --allow-dirty
   ```
2. Review auto-fixes: `git diff`
3. Test: `cargo test -p wkmp-ai -p wkmp-common`
4. Commit auto-fixes if tests pass
5. Run clippy again for remaining warnings:
   ```bash
   cargo clippy -p wkmp-ai -p wkmp-common
   ```
6. Manually fix remaining warnings
7. Final verification:
   ```bash
   cargo clippy -p wkmp-ai -p wkmp-common -- -D warnings
   # Expected: Success (no warnings)
   ```
8. Commit: "Fix all clippy lints (22 warnings eliminated)"

**Acceptance Test:** AT-TD1-004

---

### Increment 1.5: Fix Panic Statements (REQ-TD1-005)

**Effort:** 30-45 minutes

**Steps:**
1. Fix `wkmp-common/src/events.rs:1564`:
   ```rust
   // Replace panic! with:
   _ => return Err(serde::de::Error::custom(
       format!("Invalid event type: expected {}, got {}", expected, actual)
   )),
   ```
2. Fix `wkmp-ai/src/extractors/mod.rs:77`:
   - Check if function is used
   - If unused: Delete
   - If used: Implement or return error
3. Verify no other panics:
   ```bash
   grep -rn "panic!\|unimplemented!" wkmp-ai/src/ wkmp-common/src/ | \
       grep -v "#\[cfg(test)\]" | grep -v "/tests/"
   # Expected: No results (or only test code)
   ```
4. Test: `cargo test -p wkmp-ai -p wkmp-common`
5. Commit: "Replace panic! statements with proper error handling"

**Acceptance Test:** AT-TD1-005

---

### Phase 1 Completion Checklist

- [  ] All 216 tests pass
- [ ] Zero compiler warnings
- [  ] Zero clippy warnings
- [  ] No panic! in production code
- [  ] Dead code removed (-368 lines)
- [  ] Blocking sleep replaced
- [  ] CHANGELOG.md updated with Phase 1 summary
- [  ] User approval received

**Phase 1 Deliverable:** Clean build with zero warnings, no panics, no dead code

---

## Phase 2: File Organization

**Objective:** Split monolithic files into modular structures
**Duration:** 1-2 weeks
**Priority:** CRITICAL (orchestrator), HIGH (events, params), MEDIUM (ui)

### Increment 2.1: Refactor Workflow Orchestrator (REQ-TD2-001)

**Effort:** 3-4 days

**Steps:**
1. **Setup (30 min):**
   ```bash
   mkdir wkmp-ai/src/services/workflow_orchestrator
   ```

2. **Extract modules incrementally (test after each):**

   **2.1a: Create mod.rs stub (1 hour)**
   - Create `mod.rs` with `WorkflowOrchestrator` struct definition
   - No methods yet, just struct and fields
   - Test: `cargo build -p wkmp-ai`

   **2.1b: Extract phase_scanning.rs (4 hours)**
   - Move `phase_1_scan_files()` and helpers
   - Add `mod phase_scanning;` to mod.rs
   - Update calls: `self::phase_scanning::scan_files(...)`
   - Test: `cargo test -p wkmp-ai`
   - Commit: "Extract phase_scanning module from orchestrator"

   **2.1c: Extract phase_extraction.rs (4 hours)**
   - Move ID3 extraction methods
   - Test and commit

   **2.1d: Extract phase_fingerprinting.rs (4 hours)**
   - Move Chromaprint/AcoustID methods
   - Test and commit

   **2.1e: Extract phase_segmenting.rs (3 hours)**
   - Move silence detection methods
   - Test and commit

   **2.1f: Extract phase_analyzing.rs (3 hours)**
   - Move amplitude analysis methods
   - Test and commit

   **2.1g: Extract phase_flavoring.rs (3 hours)**
   - Move AcousticBrainz/Essentia methods
   - Test and commit

   **2.1h: Extract entity_linking.rs (3 hours)**
   - Move song/artist/album linking methods
   - Test and commit

   **2.1i: Finalize mod.rs (1 hour)**
   - Add public re-exports: `pub use self::WorkflowOrchestrator;`
   - Verify phase modules are private/pub(crate)
   - Final test: `cargo test -p wkmp-ai`
   - Commit: "Complete workflow_orchestrator refactoring (2,253 → 7 modules)"

3. **Verify line counts:**
   ```bash
   wc -l wkmp-ai/src/services/workflow_orchestrator/*.rs
   # Verify largest <650 lines
   ```

4. **Verify backward compatibility:**
   - External imports unchanged
   - All public methods accessible
   - Tests pass

**Acceptance Tests:** AT-TD2-001a, AT-TD2-001b

---

### Increment 2.2: Split events.rs (REQ-TD2-002)

**Effort:** 1-2 days

**Steps:**
1. **Setup (15 min):**
   ```bash
   mkdir wkmp-common/src/events
   ```

2. **Extract modules (test after each):**

   **2.2a: Extract system_events.rs (2 hours)**
   - Move system/config event types
   - Test: `cargo test -p wkmp-common`
   - Commit

   **2.2b: Extract playback_events.rs (2 hours)**
   - Move playback event types
   - Test and commit

   **2.2c: Extract import_events.rs (3 hours)**
   - Move import workflow event types
   - Test and commit

   **2.2d: Extract sse_formatting.rs (3 hours)**
   - Move SSE serialization functions
   - Test and commit

   **2.2e: Create mod.rs with re-exports (30 min)**
   ```rust
   pub use self::import_events::*;
   pub use self::playback_events::*;
   pub use self::system_events::*;
   pub use self::sse_formatting::*;
   ```
   - Test: `cargo test -p wkmp-common`
   - Commit: "Complete events.rs split (1,711 → 4 modules)"

**Acceptance Test:** AT-TD2-002

---

### Increment 2.3: Split params.rs (REQ-TD2-003)

**Effort:** 1-2 days

**Steps:**
1. Create `wkmp-common/src/params/` directory
2. Extract modules (same pattern as events.rs):
   - `crossfade_params.rs` (3 hours)
   - `selector_params.rs` (3 hours)
   - `timing_params.rs` (2 hours)
   - `flavor_params.rs` (3 hours)
   - `system_params.rs` (2 hours)
3. Create `mod.rs` with wildcard re-exports
4. Test after each extraction
5. Commit: "Complete params.rs split (1,450 → 5 modules)"

**Acceptance Test:** AT-TD2-003

---

### Increment 2.4: Reorganize api/ui.rs (REQ-TD2-004)

**Effort:** 2-3 days

**Steps:**
1. Create `wkmp-ai/src/api/ui/` directory
2. Extract page modules:
   - `dashboard_page.rs` (3 hours)
   - `settings_page.rs` (3 hours)
   - `library_page.rs` (4 hours)
   - `import_page.rs` (3 hours)
   - `components.rs` (shared HTML, 2 hours)
3. Create `mod.rs` with route registration
4. Test HTTP endpoints
5. Commit: "Complete ui.rs split (1,308 → 6 modules)"

**Acceptance Test:** AT-TD2-004

---

### Phase 2 Completion Checklist

- [  ] All 216 tests pass
- [  ] workflow_orchestrator/ split complete (largest file <650 lines)
- [  ] events/ split complete (largest file <600 lines)
- [  ] params/ split complete (largest file <400 lines)
- [  ] api/ui/ split complete (largest file <300 lines)
- [  ] No files >800 lines in wkmp-ai or wkmp-common
- [  ] Backward compatibility preserved
- [  ] CHANGELOG.md updated with Phase 2 summary
- [  ] User approval received

**Phase 2 Deliverable:** Modular file structure with all files <800 lines

---

## Phase 3: Error Handling

**Objective:** Audit unwrap() calls and convert to proper error handling
**Duration:** 2-3 days
**Priority:** HIGH

### Increment 3.1: Audit unwrap() Usage (REQ-TD3-001)

**Effort:** 4-6 hours

**Steps:**
1. **Generate list (15 min):**
   ```bash
   grep -rn "\.unwrap()\|\.expect(" wkmp-ai/src/ wkmp-common/src/ | \
       grep -v "#\[cfg(test)\]" | grep -v "/tests/" | grep -v "test_" > /tmp/unwrap_raw.txt
   ```

2. **Create audit document (4-5 hours):**
   - Create `wip/unwrap_audit.md`
   - Table format:
     ```markdown
     | File | Line | Context | Classification | Priority | Justification |
     ```
   - Review each of ~506 calls
   - Classify as KEEP/CONVERT/REMOVE
   - Assign priority (HIGH/MEDIUM/LOW)
   - Add justification for each

3. **Classification criteria:**
   - **KEEP:** Startup config, FFI, invariants
   - **CONVERT:** User input, I/O, network, database
   - **REMOVE:** Defaults, fallbacks

4. **Commit audit document:**
   ```bash
   git add wip/unwrap_audit.md
   git commit -m "Add unwrap audit (506 calls classified)"
   ```

**Acceptance Tests:** AT-TD3-001a, AT-TD3-001b

**Critical Gap Resolution:** This increment resolves GAP-CRITICAL-1 (audit output format)

---

### Increment 3.2: Convert User-Facing unwrap() (REQ-TD3-002)

**Effort:** 3-4 days

**Steps:**
1. **Convert by module priority:**

   **3.2a: HTTP API handlers (1 day)**
   - Review audit for CONVERT calls in `api/`
   - Convert each unwrap to Result + context
   - Test: `cargo test -p wkmp-ai`
   - Commit: "Convert unwrap() in HTTP handlers"

   **3.2b: File scanner (4 hours)**
   - Convert unwrap() in `services/file_scanner.rs`
   - Test and commit

   **3.2c: Workflow orchestrator (1 day)**
   - Convert unwrap() in `workflow/workflow_orchestrator/`
   - Test and commit

   **3.2d: Database operations (1 day)**
   - Convert unwrap() in `workflow/storage.rs`
   - Test and commit

2. **Conversion pattern:**
   ```rust
   // BEFORE
   let data = load_file(path).unwrap();

   // AFTER
   let data = load_file(path)
       .with_context(|| format!("Failed to load file: {}", path.display()))?;
   ```

3. **Track progress:**
   - Update audit document with Status column
   - Mark each conversion as DONE

**Acceptance Tests:** AT-TD3-002a, AT-TD3-002b

---

### Increment 3.3: Add Error Context (REQ-TD3-003)

**Effort:** 1 day (integrated with 3.2)

**Steps:**
1. During unwrap() conversions, add context to all error propagation
2. For existing bare `?` operators:
   ```bash
   grep -rn "?" wkmp-ai/src/services/ wkmp-ai/src/workflow/ | \
       grep -v "with_context\|context(" > /tmp/missing_context.txt
   ```
3. Add context to critical paths:
   - Import workflow
   - Database operations
   - Network APIs
   - File system operations
4. Context message format:
   ```rust
   .with_context(|| format!("Failed to [operation]: [details]"))?
   ```

**Acceptance Test:** AT-TD3-003

---

### Phase 3 Completion Checklist

- [  ] All 216 tests pass
- [  ] Unwrap audit complete (506 calls classified)
- [  ] <50 unwrap() calls remaining in production code
- [  ] >90% reduction achieved (506 → <50)
- [  ] All user-facing paths use Result types
- [  ] Error messages include context
- [  ] CHANGELOG.md updated with Phase 3 summary
- [  ] User approval received

**Phase 3 Deliverable:** Complete unwrap audit + >90% unwrap reduction + error context

---

## Phase 4: Documentation

**Objective:** Document all public APIs and modules
**Duration:** 1 week
**Priority:** HIGH (modules), MEDIUM (functions)

### Increment 4.1: Enable missing_docs Lint (REQ-TD4-001)

**Effort:** 15 minutes

**Steps:**
1. Add to `wkmp-ai/src/lib.rs`: `#![warn(missing_docs)]`
2. Add to `wkmp-common/src/lib.rs`: `#![warn(missing_docs)]`
3. Build and capture baseline warnings:
   ```bash
   cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation" | tee /tmp/missing_docs_baseline.txt
   ```
4. Commit: "Enable missing_docs lint"

**Acceptance Test:** AT-TD4-001

---

### Increment 4.2: Document Public Modules (REQ-TD4-002)

**Effort:** 2-3 days

**Steps:**
1. **List modules needing docs (15 min):**
   ```bash
   cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation for a module"
   ```

2. **Document modules incrementally:**

   **4.2a: Public API modules (1 day)**
   - types.rs
   - extractors/mod.rs (trait definitions)
   - fusion/mod.rs
   - validators/mod.rs

   **4.2b: HTTP API modules (1 day)**
   - api/ui/
   - api/routes.rs

   **4.2c: Internal modules (1 day)**
   - services/
   - workflow/

3. **Documentation template:**
   ```rust
   //! Brief one-line description
   //!
   //! Longer explanation of module purpose and contents.
   //!
   //! # Examples
   //! ```rust,ignore
   //! // Example usage
   //! ```
   ```

4. **Verify after each batch:**
   ```bash
   cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation for a module"
   # Verify count decreases
   ```

5. **Final verification:**
   ```bash
   cargo doc -p wkmp-ai -p wkmp-common --no-deps
   # Verify all modules documented
   ```

**Acceptance Tests:** AT-TD4-002a, AT-TD4-002b

---

### Increment 4.3: Document Public Functions (REQ-TD4-003)

**Effort:** 3-4 days

**Steps:**
1. **List undocumented functions (15 min):**
   ```bash
   cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation for" | grep -v "module"
   ```

2. **Document in batches of 20-30 functions:**

   **4.3a: Public API types (1 day)**
   - types.rs (ConfidenceValue, MetadataExtraction, etc.)
   - Trait definitions (SourceExtractor, etc.)

   **4.3b: HTTP API (1 day)**
   - Route handlers
   - UI generation functions

   **4.3c: Internal APIs (2 days)**
   - Services (file_scanner, API clients)
   - Workflow (orchestrator, pipeline, storage)

3. **Documentation template:**
   ```rust
   /// Brief one-line description
   ///
   /// # Arguments
   /// * `param1` - Description
   ///
   /// # Returns
   /// Description of return value
   ///
   /// # Errors
   /// When this returns an error
   ```

4. **Verify after each batch:**
   ```bash
   cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation for" | grep -v "module" | wc -l
   ```

5. **Final verification:**
   ```bash
   cargo doc -p wkmp-ai -p wkmp-common --no-deps -- -D missing_docs
   # Should succeed with zero warnings
   ```

**Acceptance Tests:** AT-TD4-003a, AT-TD4-003b

---

### Phase 4 Completion Checklist

- [  ] All 216 tests pass
- [  ] missing_docs lint enabled
- [  ] 100% module documentation
- [  ] 100% public function documentation (294 functions)
- [  ] Zero missing documentation warnings
- [  ] cargo doc generates complete docs
- [  ] CHANGELOG.md updated with Phase 4 summary
- [  ] User approval received

**Phase 4 Deliverable:** Complete public API documentation (100% coverage)

---

## Phase 5: Code Quality

**Objective:** Extract utilities, refactor long functions, consolidate config
**Duration:** 1-2 weeks
**Priority:** MEDIUM-LOW

### Increment 5.1: Extract Rate Limiter Utility (REQ-TD5-001)

**Effort:** 1-2 days

**Steps:**
1. **Create shared utility (3 hours):**
   - Create `wkmp-common/src/rate_limiter.rs`
   - Implement RateLimiter struct with Clone
   - Add unit tests
   - Export from lib.rs
   - Commit: "Add shared RateLimiter utility"

2. **Refactor clients incrementally:**

   **5.1a: acoustid_client.rs (2 hours)**
   - Replace custom rate limiter with shared utility
   - Test: `cargo test -p wkmp-ai acoustid`
   - Commit: "Refactor acoustid_client to use shared RateLimiter"

   **5.1b: services/musicbrainz_client.rs (2 hours)**
   - Refactor and test

   **5.1c: extractors/musicbrainz_client.rs (2 hours)**
   - Refactor and test

   **5.1d: acousticbrainz_client.rs (2 hours)**
   - Refactor and test

3. **Verify 4 duplicates eliminated:**
   ```bash
   ! grep "Arc<Mutex<Option<Instant>>>" wkmp-ai/src/services/*.rs
   # Expected: No results
   ```

**Acceptance Tests:** AT-TD5-001a, AT-TD5-001b

---

### Increment 5.2: Break Up Long Functions (REQ-TD5-002)

**Effort:** 3-4 days

**Steps:**
1. **Identify functions >200 lines (1 hour):**
   - Manual review or use code analysis tool
   - Focus on orchestrator, ui, storage

2. **Refactor functions incrementally:**

   **5.2a: Orchestrator functions (1-2 days)**
   - Identify 5-10 longest functions
   - Extract logical sub-steps to helpers
   - Name helpers clearly
   - Test after each refactoring

   **5.2b: UI generation functions (1 day)**
   - Extract HTML generation helpers
   - Test HTTP endpoints

   **5.2c: Storage helpers (1 day)**
   - Refactor long test helpers
   - Test

3. **Pattern:**
   ```rust
   // BEFORE (300 lines)
   pub fn process(...) -> Result<...> { /* all logic */ }

   // AFTER (4 focused functions)
   pub fn process(...) -> Result<...> {
       validate(...)?;
       let data = extract(...)?;
       let result = transform(data)?;
       store(result)?;
       Ok(())
   }
   ```

4. **Verify:**
   ```bash
   # Manual review or tool
   # Verify no functions >200 lines
   ```

**Acceptance Test:** AT-TD5-002

---

### Increment 5.3: Consolidate Configuration (REQ-TD5-003)

**Effort:** 2-3 days

**Steps:**
1. **Create unified config (1 day):**
   - Create `wkmp-common/src/config/workflow.rs`
   - Define WorkflowConfig struct
   - Implement builder pattern
   - Add tests
   - Commit: "Add unified WorkflowConfig"

2. **Migrate PipelineConfig usage (1 day):**
   - Find all PipelineConfig uses
   - Replace with WorkflowConfig
   - Test after each migration
   - Commit: "Migrate PipelineConfig to WorkflowConfig"

3. **Update ImportParameters usage (1 day):**
   - Embed in WorkflowConfig
   - Update construction sites
   - Test
   - Commit: "Complete WorkflowConfig consolidation"

**Acceptance Test:** AT-TD5-003

---

### Increment 5.4: Remove Magic Numbers (REQ-TD5-004)

**Effort:** 1 day

**Steps:**
1. **Find magic numbers (1 hour):**
   ```bash
   grep -rn "Duration::from_secs([0-9]" wkmp-ai/src/ wkmp-common/src/ | grep -v "from_secs(0)"
   ```

2. **Replace with constants (4-6 hours):**
   - Define constants at module level
   - Document with units/purpose
   - Replace literals with constants
   - Test after each module

3. **Pattern:**
   ```rust
   /// SSE keepalive interval (15 seconds)
   const SSE_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
   ```

4. **Commit:** "Replace magic numbers with named constants"

**Acceptance Test:** AT-TD5-004

---

### Phase 5 Completion Checklist

- [  ] All 216 tests pass
- [  ] Rate limiter utility extracted (4 duplicates → 1)
- [  ] All functions <200 lines
- [  ] Single WorkflowConfig struct
- [  ] Magic numbers replaced with constants
- [  ] CHANGELOG.md updated with Phase 5 summary
- [  ] User approval received

**Phase 5 Deliverable:** Extracted utilities, refactored functions, consolidated config

---

## Effort Estimates

### Per-Phase Estimates

| Phase | Requirements | Increments | Effort (Days) | Effort (Hours) |
|-------|--------------|------------|---------------|----------------|
| Phase 1: Quick Wins | 5 | 5 | 1-2 | 8-16 |
| Phase 2: File Organization | 4 | 4 | 7-10 | 56-80 |
| Phase 3: Error Handling | 3 | 3 | 4-5 | 32-40 |
| Phase 4: Documentation | 3 | 3 | 5-7 | 40-56 |
| Phase 5: Code Quality | 4 | 4 | 5-8 | 40-64 |
| **TOTAL** | **19** | **19** | **22-32** | **176-256** |

**Note:** Total includes 22 phase-specific requirements (4 cross-phase requirements apply to all phases)

### Per-Increment Estimates (Detailed)

#### Phase 1: Quick Wins
- 1.1 Blocking sleep: 15 min
- 1.2 Dead code: 15 min
- 1.3 Compiler warnings: 1-2 hours
- 1.4 Clippy lints: 1-2 hours
- 1.5 Panic statements: 30-45 min
- **Phase 1 Total: 4-6 hours**

#### Phase 2: File Organization
- 2.1 Workflow orchestrator: 3-4 days (24-32 hours)
- 2.2 Events.rs: 1-2 days (8-16 hours)
- 2.3 Params.rs: 1-2 days (8-16 hours)
- 2.4 API/UI: 2-3 days (16-24 hours)
- **Phase 2 Total: 7-10 days (56-80 hours)**

#### Phase 3: Error Handling
- 3.1 Unwrap audit: 4-6 hours
- 3.2 Convert unwrap: 3-4 days (24-32 hours)
- 3.3 Error context: 1 day (8 hours, integrated with 3.2)
- **Phase 3 Total: 4-5 days (32-40 hours)**

#### Phase 4: Documentation
- 4.1 Enable lint: 15 min
- 4.2 Module docs: 2-3 days (16-24 hours)
- 4.3 Function docs: 3-4 days (24-32 hours)
- **Phase 4 Total: 5-7 days (40-56 hours)**

#### Phase 5: Code Quality
- 5.1 Rate limiter: 1-2 days (8-16 hours)
- 5.2 Long functions: 3-4 days (24-32 hours)
- 5.3 Config consolidation: 2-3 days (16-24 hours)
- 5.4 Magic numbers: 1 day (8 hours)
- **Phase 5 Total: 7-10 days (56-80 hours)**

### Assumptions
- Developer familiarity with Rust
- Developer familiarity with WKMP codebase
- No blockers or unexpected issues
- Continuous 8-hour work days (actual calendar time will be longer)

### Calendar Time Estimates
- **Minimum:** 4 weeks (with full-time dedicated work)
- **Expected:** 5 weeks (with typical interruptions)
- **Maximum:** 6 weeks (with unexpected issues or part-time work)

---

## Risk Analysis

### Phase 1 Risks

**Risk 1.1: Auto-fix introduces bugs**
- **Probability:** LOW
- **Impact:** MEDIUM
- **Mitigation:** Review all auto-fixes, test after each, commit only if tests pass

**Risk 1.2: Warning table is stale**
- **Probability:** MEDIUM
- **Impact:** LOW
- **Mitigation:** Regenerate warning list at start of Phase 1, use as checklist

---

### Phase 2 Risks

**Risk 2.1: Workflow orchestrator refactoring breaks functionality**
- **Probability:** MEDIUM
- **Impact:** HIGH
- **Mitigation:** Incremental extraction (test after EACH module), commit frequently

**Risk 2.2: Public API accidentally broken**
- **Probability:** LOW-MEDIUM
- **Impact:** CRITICAL
- **Mitigation:** Verify re-exports, test backward compatibility, run AT-ALL-002

**Risk 2.3: Circular module dependencies**
- **Probability:** LOW
- **Impact:** MEDIUM
- **Mitigation:** Extract least-dependent modules first, avoid cross-references

---

### Phase 3 Risks

**Risk 3.1: Unwrap conversions introduce new error paths**
- **Probability:** HIGH
- **Impact:** MEDIUM
- **Mitigation:** Thorough testing, error path tests, verify error messages

**Risk 3.2: Function signature changes cascade**
- **Probability:** MEDIUM
- **Impact:** MEDIUM
- **Mitigation:** Update signatures incrementally, compile after each change

**Risk 3.3: Audit is incomplete or inaccurate**
- **Probability:** MEDIUM
- **Impact:** LOW
- **Mitigation:** Automated search + manual review, spot-check accuracy

---

### Phase 4 Risks

**Risk 4.1: Documentation is low quality or inaccurate**
- **Probability:** MEDIUM
- **Impact:** LOW
- **Mitigation:** Review docs for accuracy, update based on code inspection

**Risk 4.2: Documentation effort underestimated**
- **Probability:** MEDIUM
- **Impact:** LOW
- **Mitigation:** Buffer time in estimates, prioritize critical modules

---

### Phase 5 Risks

**Risk 5.1: Rate limiter refactoring breaks rate limiting behavior**
- **Probability:** LOW
- **Impact:** HIGH (violates API terms of service)
- **Mitigation:** Thorough unit tests, integration tests, verify timing behavior

**Risk 5.2: Function extraction harms readability**
- **Probability:** MEDIUM
- **Impact:** LOW
- **Mitigation:** Extract only logical sub-steps, use clear names, avoid over-extraction

**Risk 5.3: Config consolidation breaks existing usage**
- **Probability:** LOW-MEDIUM
- **Impact:** MEDIUM
- **Mitigation:** Incremental migration, test after each callsite update

---

### Cross-Phase Risks

**Risk ALL-1: Test suite has gaps (doesn't catch regressions)**
- **Probability:** LOW-MEDIUM
- **Impact:** HIGH
- **Mitigation:** Assume 216 tests are adequate (validated by PLAN024 work), manual smoke testing

**Risk ALL-2: Scope creep (functional changes introduced)**
- **Probability:** MEDIUM
- **Impact:** MEDIUM
- **Mitigation:** Strict adherence to scope statement, defer all feature work

**Risk ALL-3: Merge conflicts (concurrent work)**
- **Probability:** LOW (assuming dedicated work)
- **Impact:** MEDIUM
- **Mitigation:** Coordinate with team, frequent commits, rebase if needed

---

## Implementation Checklist

### Pre-Implementation
- [  ] Review complete plan (this document)
- [  ] Resolve critical gaps (GAP-CRITICAL-1, GAP-CRITICAL-2)
- [  ] User approval of plan
- [  ] Clean working directory (`git status` clean)
- [  ] All 216 tests currently passing
- [  ] Create tracking branch: `git checkout -b technical-debt-reduction`

### During Implementation
- [  ] Follow phase order (1 → 2 → 3 → 4 → 5)
- [  ] Test after EVERY increment
- [  ] Commit after tests pass
- [  ] Run AT-ALL-001 (test preservation) after each commit
- [  ] Run AT-ALL-002 (backward compatibility) after each phase
- [  ] Update CHANGELOG.md after each phase
- [  ] Request user review after each phase

### Post-Implementation
- [  ] All 26 requirements complete
- [  ] All 31 acceptance tests pass
- [  ] Final backward compatibility check
- [  ] Final documentation review
- [  ] Update version numbers (patch bump)
- [  ] Merge to main branch
- [  ] Tag release: `git tag -a technical-debt-complete -m "Technical debt reduction complete"`

---

## Approval and Sign-Off

### Specification Approval

**Status:** ✅ APPROVED FOR IMPLEMENTATION

**Specification Source:** `wip/SPEC_technical_debt_reduction.md`
**Technical Debt Report:** `wip/technical_debt_report.md`

**Plan Generated By:** /plan workflow (Phases 1-8 complete)
**Plan Generated Date:** 2025-11-10

### Plan Completeness

**Phase 1 (Input Validation):** ✅ COMPLETE
- requirements_index.md (26 requirements extracted)
- scope_statement.md (in/out scope, constraints)
- dependencies_map.md (internal/external dependencies)

**Phase 2 (Completeness Verification):** ✅ COMPLETE
- completeness_report.md (92% complete, 2 critical gaps identified)

**Phase 3 (Acceptance Tests):** ✅ COMPLETE
- acceptance_tests.md (31 tests for 26 requirements)

**Phase 4 (Approaches):** ✅ COMPLETE
- implementation_approaches.md (approaches for all requirements)

**Phase 5-8 (Implementation Plan):** ✅ COMPLETE
- This document (increments, estimates, risks, checklist)

### Critical Gap Resolutions

**GAP-CRITICAL-1: Unwrap Audit Output Format**
- ✅ RESOLVED in Increment 3.1
- Format: `wip/unwrap_audit.md` with table structure
- Columns: File | Line | Context | Classification | Priority | Justification

**GAP-CRITICAL-2: Public API Preservation**
- ✅ RESOLVED in Increment 2.1i
- Strategy: WorkflowOrchestrator re-exported from mod.rs
- Phase modules are private/pub(crate)
- Backward compatibility tests defined (AT-TD2-001b)

### Implementation Authorization

**Ready for Implementation:** ✅ YES

**Prerequisites:**
- [  ] User reviews and approves this plan
- [  ] Critical gaps acknowledged as resolved
- [  ] 216 tests currently passing
- [  ] Clean working directory

**Next Step:** Begin Phase 1 (Quick Wins) per implementation checklist above

---

**END OF IMPLEMENTATION PLAN**
