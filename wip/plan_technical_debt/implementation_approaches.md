# Implementation Approaches: Technical Debt Reduction

**Project:** WKMP Technical Debt Reduction
**Phase:** /plan Phase 4 - Approach Selection
**Generated:** 2025-11-10

---

## Approach Selection Methodology

For each requirement, evaluate approaches based on:
1. **Risk** (Primary): Lowest risk of breaking existing functionality
2. **Effort** (Secondary): Implementation time and complexity
3. **Testability** (Tertiary): Ease of verifying correctness

**Decision Framework:**
- If multiple approaches have equivalent risk, choose lowest effort
- If risk differs, ALWAYS choose lowest risk regardless of effort
- Document rejected alternatives with rationale

---

## Phase 1: Quick Wins - Approaches

### REQ-TD1-001: Replace Blocking Sleep in Async Context

**Requirement:** Replace `std::thread::sleep` with `tokio::time::sleep` in async test

**Approach Selected:** Direct replacement + verification

**Implementation Steps:**
1. Locate `std::thread::sleep` call in `wkmp-common/src/time.rs:37`
2. Replace with `tokio::time::sleep(Duration::from_millis(10)).await`
3. Update imports: `use tokio::time::sleep;`
4. Run test: `cargo test test_now_successive_calls_advance`
5. Verify test still passes

**Rationale:**
- **Risk:** LOW - Trivial change, test provides immediate verification
- **Effort:** MINIMAL - Single line change
- **Testability:** EXCELLENT - Existing test verifies correctness

**Rejected Alternatives:**
- None - this is the canonical fix for this issue

---

### REQ-TD1-002: Delete Dead Code (song_processor.rs)

**Requirement:** Remove unused `song_processor.rs` file and commented module declaration

**Approach Selected:** Delete file + verify no references + test

**Implementation Steps:**
1. Search codebase for all references to `song_processor`:
   ```bash
   grep -rn "song_processor" wkmp-ai/src/
   ```
2. Remove comment from `wkmp-ai/src/workflow/mod.rs:11`
3. Delete file: `git rm wkmp-ai/src/workflow/song_processor.rs`
4. Build: `cargo build -p wkmp-ai`
5. Test: `cargo test -p wkmp-ai`
6. Commit: `git commit -m "Remove dead code: song_processor.rs"`

**Rationale:**
- **Risk:** LOW - File is unused (commented out), no references expected
- **Effort:** MINIMAL - Delete file and comment
- **Testability:** EXCELLENT - Build + test suite verify no breakage

**Rejected Alternatives:**
- Keep file "just in case" - REJECTED (violates dead code removal goal)

---

### REQ-TD1-003: Fix Compiler Warnings

**Requirement:** Eliminate 11 compiler warnings

**Approach Selected:** Fix warnings individually using table + verify

**Implementation Steps:**
1. Run `cargo build -p wkmp-ai -p wkmp-common 2>&1 | tee /tmp/warnings.txt`
2. For each warning in specification table (lines 175-186):
   - **Unused imports:** Delete import line
   - **Unused variables:** Prefix with `_` (e.g., `event_bus` → `_event_bus`)
   - **Dead code:** Remove or add `#[allow(dead_code)]` with comment justification
3. Build after each fix to verify warning eliminated
4. Run full test suite after all fixes

**Rationale:**
- **Risk:** LOW - Standard warning fixes, well-understood patterns
- **Effort:** LOW - 11 warnings, most are simple deletions/renames
- **Testability:** EXCELLENT - Build output shows warning count

**Rejected Alternatives:**
- Use `#[allow(unused)]` globally - REJECTED (silences warnings, doesn't fix issues)
- Fix only some warnings - REJECTED (requirement is "eliminate ALL warnings")

---

### REQ-TD1-004: Fix Clippy Lints

**Requirement:** Address 22 clippy warnings using auto-fix + manual fixes

**Approach Selected:** cargo clippy --fix + manual review + test

**Implementation Steps:**
1. Run auto-fix:
   ```bash
   cargo clippy --fix --lib -p wkmp-ai --allow-dirty
   cargo clippy --fix --lib -p wkmp-common --allow-dirty
   ```
2. Review auto-fixes: `git diff`
3. Commit auto-fixes if all tests pass
4. Run clippy again to find remaining warnings:
   ```bash
   cargo clippy -p wkmp-ai -p wkmp-common
   ```
5. Manually fix remaining warnings
6. Verify: `cargo clippy -p wkmp-ai -p wkmp-common -- -D warnings`

**Rationale:**
- **Risk:** LOW-MEDIUM - Auto-fixes are generally safe but should be reviewed
- **Effort:** LOW - Auto-fix handles most cases
- **Testability:** EXCELLENT - Clippy output + test suite verify correctness

**Rejected Alternatives:**
- Manual fixes only - REJECTED (higher effort, more error-prone)
- Skip review of auto-fixes - REJECTED (increases risk)

---

### REQ-TD1-005: Fix Panic Statements in Production Code

**Requirement:** Replace 2 panic! macros with proper error handling

**Approach Selected:** Replace panic with Result + error message + test

**Implementation Steps:**
1. **Fix events.rs:1564:**
   ```rust
   // BEFORE
   _ => panic!("Wrong event type deserialized"),

   // AFTER
   _ => return Err(serde::de::Error::custom(
       format!("Invalid event type: expected {}, got {}", expected, actual)
   )),
   ```
2. **Fix extractors/mod.rs:77:**
   - Locate `unimplemented!()` call
   - Determine if function is used (check references)
   - If unused: Delete function
   - If used: Implement or return error

3. Verify no other panics: `grep -rn "panic!\|unimplemented!" wkmp-ai/src/ wkmp-common/src/`
4. Run tests: `cargo test -p wkmp-ai -p wkmp-common`

**Rationale:**
- **Risk:** LOW - Changes are localized, serde error is standard pattern
- **Effort:** LOW - 2 specific fixes
- **Testability:** GOOD - Tests verify error paths work

**Rejected Alternatives:**
- Keep panic! with better message - REJECTED (still crashes on invalid input)

---

## Phase 2: File Organization - Approaches

### REQ-TD2-001: Refactor Workflow Orchestrator into Modules

**Requirement:** Split 2,253-line file into 7-8 modules

**Approach Selected:** Extract method pattern + incremental refactoring

**Implementation Steps:**
1. **Create directory structure:**
   ```bash
   mkdir wkmp-ai/src/services/workflow_orchestrator
   ```

2. **Extract modules one by one (order matters for minimal breakage):**
   - **Step 1:** Create `mod.rs` with current `WorkflowOrchestrator` struct (no methods yet)
   - **Step 2:** Extract `phase_scanning.rs`:
     - Move `phase_1_scan_files()` method and helpers
     - Update `mod.rs` to use extracted module
     - Test: `cargo test -p wkmp-ai`
   - **Step 3:** Extract `phase_extraction.rs` (ID3 methods)
   - **Step 4:** Extract `phase_fingerprinting.rs` (Chromaprint/AcoustID)
   - **Step 5:** Extract `phase_segmenting.rs` (silence detection)
   - **Step 6:** Extract `phase_analyzing.rs` (amplitude analysis)
   - **Step 7:** Extract `phase_flavoring.rs` (AcousticBrainz/Essentia)
   - **Step 8:** Extract `entity_linking.rs` (song/artist/album linking)

3. **For each extraction:**
   - Move functions to new module file
   - Add `pub(super)` visibility for internal functions
   - Import new module in `mod.rs`: `mod phase_scanning;`
   - Update method calls to use module path: `self::phase_scanning::scan_files(...)`
   - Run tests after each extraction
   - Commit if tests pass

4. **Update `mod.rs` re-exports:**
   ```rust
   pub use self::WorkflowOrchestrator;
   // Phase modules are private (pub(crate) or no pub)
   ```

5. **Verify backward compatibility:**
   - External imports unchanged: `use wkmp_ai::services::workflow_orchestrator::WorkflowOrchestrator;`
   - All public methods accessible
   - Tests pass

**Rationale:**
- **Risk:** MEDIUM - Large refactoring, but incremental approach reduces risk
- **Effort:** HIGH - 2,253 lines to reorganize
- **Testability:** EXCELLENT - 216 tests verify no behavioral changes after each step

**Critical Considerations:**
- **Public API Preservation:** Only `WorkflowOrchestrator` struct and its public methods exported from `mod.rs`
- **Phase Modules Internal:** All phase modules are `pub(crate)` or private
- **Incremental Testing:** Test after EACH phase extraction, not at the end

**Rejected Alternatives:**
- "Big bang" refactoring (move all at once) - REJECTED (high risk, hard to debug failures)
- Keep as single file - REJECTED (violates requirement)
- Split alphabetically instead of by phase - REJECTED (loses logical grouping)

---

### REQ-TD2-002: Split events.rs by Category

**Requirement:** Split 1,711-line file into 3-4 category modules

**Approach Selected:** Move types by category + wildcard re-exports

**Implementation Steps:**
1. **Create directory:**
   ```bash
   mkdir wkmp-common/src/events
   ```

2. **Extract modules (order: least dependent first):**
   - **Step 1:** Create `system_events.rs`
     - Move system/config event types
     - No dependencies on other events
   - **Step 2:** Create `playback_events.rs`
     - Move audio playback event types
   - **Step 3:** Create `import_events.rs`
     - Move import workflow event types
   - **Step 4:** Create `sse_formatting.rs`
     - Move SSE serialization functions
     - May reference event types from other modules

3. **Create `mod.rs` with re-exports:**
   ```rust
   mod import_events;
   mod playback_events;
   mod system_events;
   mod sse_formatting;

   pub use self::import_events::*;
   pub use self::playback_events::*;
   pub use self::system_events::*;
   pub use self::sse_formatting::*;
   ```

4. **Update imports throughout codebase:**
   - Before: `use wkmp_common::events::ImportProgressEvent;`
   - After: `use wkmp_common::events::ImportProgressEvent;` (UNCHANGED!)
   - Re-exports make external imports transparent

5. **Test after each module extraction:**
   ```bash
   cargo test -p wkmp-common
   ```

**Rationale:**
- **Risk:** LOW - Pure data types, no behavior changes
- **Effort:** MEDIUM - 1,711 lines, but straightforward moves
- **Testability:** EXCELLENT - Tests verify serialization unchanged

**Rejected Alternatives:**
- Explicit re-exports - REJECTED (verbose, many types)
- Keep as single file - REJECTED (violates requirement)

---

### REQ-TD2-003: Split params.rs by Parameter Group

**Requirement:** Split 1,450-line file into 4-5 parameter group modules

**Approach Selected:** Same as events.rs (move types + re-export)

**Implementation Steps:**
1. Create `wkmp-common/src/params/` directory
2. Extract modules:
   - `crossfade_params.rs` (CrossfadeParams, FadeCurve, etc.)
   - `selector_params.rs` (SelectorParams, cooldown settings)
   - `timing_params.rs` (TimingParams, duration settings)
   - `flavor_params.rs` (FlavorParams, musical flavor weights)
   - `system_params.rs` (SystemParams, general config)
3. Create `mod.rs` with wildcard re-exports: `pub use self::crossfade_params::*;`
4. Test after each extraction

**Rationale:**
- **Risk:** LOW - Same pattern as events.rs
- **Effort:** MEDIUM - Similar size to events.rs
- **Testability:** EXCELLENT - Database serialization tests verify correctness

**Rejected Alternatives:**
- (Same as REQ-TD2-002)

---

### REQ-TD2-004: Reorganize api/ui.rs into Page Modules

**Requirement:** Split 1,308-line UI file into 5-6 page modules

**Approach Selected:** Extract page handlers + shared components

**Implementation Steps:**
1. Create `wkmp-ai/src/api/ui/` directory
2. Extract page modules:
   - `dashboard_page.rs` (main dashboard handler)
   - `settings_page.rs` (settings form handler)
   - `library_page.rs` (library view handler)
   - `import_page.rs` (import wizard handler)
   - `components.rs` (shared HTML generation functions)
3. Create `mod.rs` with route registration:
   ```rust
   mod dashboard_page;
   mod settings_page;
   mod library_page;
   mod import_page;
   mod components;

   pub fn router() -> Router {
       Router::new()
           .route("/", get(dashboard_page::handler))
           .route("/settings", get(settings_page::handler))
           .route("/library", get(library_page::handler))
           .route("/import", get(import_page::handler))
   }
   ```
4. Test HTTP endpoints return correct HTML

**Rationale:**
- **Risk:** LOW-MEDIUM - UI refactoring, but routes unchanged
- **Effort:** MEDIUM - HTML generation code, needs careful extraction
- **Testability:** GOOD - HTTP tests verify pages render correctly

**Rejected Alternatives:**
- Split by component type (forms, tables, etc.) - REJECTED (doesn't match user mental model of "pages")

---

## Phase 3: Error Handling - Approaches

### REQ-TD3-001: Audit unwrap() Usage

**Requirement:** Audit and classify 506 unwrap/expect calls

**Approach Selected:** Automated search + manual classification + document

**Implementation Steps:**
1. **Generate list of all unwrap/expect calls:**
   ```bash
   grep -rn "\.unwrap()\|\.expect(" wkmp-ai/src/ wkmp-common/src/ | \
       grep -v "#\[cfg(test)\]" | \
       grep -v "/tests/" | \
       grep -v "test_" > /tmp/unwrap_raw.txt
   ```

2. **Create audit document:** `wip/unwrap_audit.md`
   ```markdown
   # Unwrap Audit

   | File | Line | Context | Classification | Priority | Justification |
   |------|------|---------|----------------|----------|---------------|
   | ... | ... | ... | ... | ... | ... |
   ```

3. **Classify each call (criteria from spec):**
   - **KEEP:** Startup config, FFI libraries, compile-time invariants
   - **CONVERT:** User input, file I/O, network, database
   - **REMOVE:** Default values, fallback logic, optional chaining

4. **Assign priority to convertible calls:**
   - **HIGH:** HTTP handlers, file import, database
   - **MEDIUM:** Internal APIs, background tasks
   - **LOW:** Initialization, test utilities

5. **Document justifications:**
   - KEEP: "Required FFI library for Chromaprint" (acoustid_client.rs:45)
   - CONVERT: "User file path input" (file_scanner.rs:120)
   - REMOVE: "Can use unwrap_or_default()" (metadata_fuser.rs:200)

6. **Commit audit document:**
   ```bash
   git add wip/unwrap_audit.md
   git commit -m "Add unwrap audit document (506 calls classified)"
   ```

**Rationale:**
- **Risk:** LOW - Audit only, no code changes yet
- **Effort:** HIGH - 506 calls to review manually
- **Testability:** N/A - Documentation deliverable

**Time Estimate:** 4-6 hours (manual review required)

**Rejected Alternatives:**
- Automated classification - REJECTED (requires human judgment for context)
- Skip audit, just fix high-priority - REJECTED (requirement mandates complete audit)

---

### REQ-TD3-002: Convert User-Facing unwrap() Calls

**Requirement:** Convert 200+ high-priority unwrap calls to proper error handling

**Approach Selected:** Incremental conversion by module + anyhow::Context

**Implementation Steps:**
1. **Start with highest priority modules (from audit):**
   - **Priority 1:** `api/` (HTTP handlers)
   - **Priority 2:** `services/file_scanner.rs`
   - **Priority 3:** `workflow/workflow_orchestrator.rs`
   - **Priority 4:** `workflow/storage.rs`

2. **Conversion pattern:**
   ```rust
   // BEFORE
   let data = load_file(path).unwrap();

   // AFTER
   let data = load_file(path)
       .with_context(|| format!("Failed to load file: {}", path.display()))?;
   ```

3. **For each module:**
   - Review audit document for CONVERT calls in that module
   - Convert each unwrap/expect to Result + context
   - Propagate errors up call chain (add `?` operator)
   - Update function signatures to return Result if needed
   - Test after each module completion

4. **Track progress in audit document:**
   - Add "Status" column: PENDING | IN_PROGRESS | DONE
   - Update as conversions complete

**Rationale:**
- **Risk:** MEDIUM - Changes error handling, but tests verify correctness
- **Effort:** HIGH - 200+ calls to convert
- **Testability:** EXCELLENT - Tests verify error paths work

**Critical Considerations:**
- **Function Signature Changes:** May need to update function signatures to return Result
- **Error Propagation:** Use `?` operator for clean propagation
- **Error Context:** Always add context with `.with_context()` or `.context()`

**Rejected Alternatives:**
- Convert all at once - REJECTED (high risk, hard to debug)
- Skip context addition - REJECTED (reduces error message quality)

---

### REQ-TD3-003: Add Error Context with anyhow

**Requirement:** Add descriptive error context to all error propagation sites

**Approach Selected:** Add context to existing ? operators (bundled with REQ-TD3-002)

**Implementation Steps:**
1. This is integrated with REQ-TD3-002 conversions
2. For existing `?` operators without context:
   ```bash
   grep -rn "?" wkmp-ai/src/services/ wkmp-ai/src/workflow/ | \
       grep -v "with_context\|context(" > /tmp/missing_context.txt
   ```
3. Add context to each bare `?` operator in critical paths
4. Context message format:
   ```rust
   operation()?;  // BEFORE

   operation()
       .context("Failed to [operation]: [additional info]")?;  // AFTER
   ```

**Rationale:**
- **Risk:** LOW - Additive change, no behavior impact
- **Effort:** MEDIUM - Many ? operators to enhance
- **Testability:** GOOD - Error messages visible in tests/logs

---

## Phase 4: Documentation - Approaches

### REQ-TD4-001: Enable missing_docs Lint

**Requirement:** Add `#![warn(missing_docs)]` to lib.rs

**Approach Selected:** Enable lint + capture baseline warnings

**Implementation Steps:**
1. Add lint to `wkmp-ai/src/lib.rs`:
   ```rust
   #![warn(missing_docs)]
   ```
2. Add lint to `wkmp-common/src/lib.rs`:
   ```rust
   #![warn(missing_docs)]
   ```
3. Build and capture warnings:
   ```bash
   cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation" | tee /tmp/missing_docs_baseline.txt
   ```
4. Commit lint enablement

**Rationale:**
- **Risk:** NONE - Warning only, doesn't break build
- **Effort:** MINIMAL - Two line additions
- **Testability:** EXCELLENT - Build output shows warnings

---

### REQ-TD4-002: Document Public Modules

**Requirement:** Add module-level docs to all public modules

**Approach Selected:** Template-based documentation + incremental addition

**Implementation Steps:**
1. **Create documentation template:**
   ```rust
   //! Brief one-line description
   //!
   //! Longer explanation of module purpose and contents.
   //!
   //! # Examples
   //! ```rust,ignore
   //! // Example usage
   //! ```
   //!
   //! # Architecture
   //! [How this module fits into larger system]
   ```

2. **List all modules needing docs:**
   ```bash
   cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation for a module"
   ```

3. **Document modules incrementally:**
   - Start with public API modules (types.rs, extractors/mod.rs)
   - Then HTTP API (api/)
   - Then internal modules

4. **For each module:**
   - Add `//!` doc comment at top of file
   - Include purpose, usage, examples (if applicable)
   - Build to verify warning eliminated
   - Commit

**Rationale:**
- **Risk:** NONE - Documentation only, no code changes
- **Effort:** MEDIUM - Multiple modules, but template speeds up
- **Testability:** EXCELLENT - Build warnings + generated docs

---

### REQ-TD4-003: Document Public Functions

**Requirement:** Add function docs to all 294 public functions

**Approach Selected:** Template-based documentation + batching

**Implementation Steps:**
1. **Create documentation template:**
   ```rust
   /// Brief one-line description
   ///
   /// Longer explanation of function behavior.
   ///
   /// # Arguments
   /// * `param1` - Description of param1
   /// * `param2` - Description of param2
   ///
   /// # Returns
   /// Description of return value
   ///
   /// # Errors
   /// When this function returns an error and why
   ///
   /// # Examples
   /// ```rust,ignore
   /// // Example usage
   /// ```
   pub fn function_name(...) -> Result<...> { }
   ```

2. **List undocumented functions:**
   ```bash
   cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation for" | grep -v "module"
   ```

3. **Document in batches of 20-30 functions:**
   - Group by module
   - Add docs to all functions in module
   - Build to verify
   - Commit batch

4. **Prioritize modules:**
   - Public API first (types.rs, trait definitions)
   - HTTP API second
   - Internal modules last

**Rationale:**
- **Risk:** NONE - Documentation only
- **Effort:** HIGH - 294 functions
- **Testability:** EXCELLENT - Build warnings verify completeness

---

## Phase 5: Code Quality - Approaches

### REQ-TD5-001: Extract Rate Limiter Utility

**Requirement:** Extract 4 duplicate rate limiter implementations to shared utility

**Approach Selected:** Create shared utility + refactor clients incrementally

**Implementation Steps:**
1. **Create utility in wkmp-common:**
   File: `wkmp-common/src/rate_limiter.rs`
   ```rust
   use std::sync::Arc;
   use std::time::Duration;
   use tokio::sync::Mutex;
   use tokio::time::{sleep, Instant};

   /// Rate limiter for API clients (e.g., 1 request/second)
   #[derive(Clone)]
   pub struct RateLimiter {
       last_request: Arc<Mutex<Option<Instant>>>,
       interval: Duration,
   }

   impl RateLimiter {
       /// Create rate limiter with specified interval between requests
       pub fn new(interval: Duration) -> Self {
           Self {
               last_request: Arc::new(Mutex::new(None)),
               interval,
           }
       }

       /// Acquire permit (sleeps if needed to maintain rate limit)
       pub async fn acquire(&self) {
           let mut last = self.last_request.lock().await;

           if let Some(last_time) = *last {
               let elapsed = last_time.elapsed();
               if elapsed < self.interval {
                   sleep(self.interval - elapsed).await;
               }
           }

           *last = Some(Instant::now());
       }
   }
   ```

2. **Add unit tests:**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_rate_limiting() {
           let limiter = RateLimiter::new(Duration::from_secs(1));

           let start = Instant::now();
           limiter.acquire().await;  // Immediate
           limiter.acquire().await;  // Waits ~1s

           assert!(start.elapsed() >= Duration::from_millis(900));
       }
   }
   ```

3. **Export from wkmp-common:**
   ```rust
   // wkmp-common/src/lib.rs
   pub mod rate_limiter;
   ```

4. **Refactor each client (one at a time):**
   - **Client 1:** acoustid_client.rs
     - Replace custom rate limiter with `use wkmp_common::rate_limiter::RateLimiter;`
     - Update struct: `rate_limiter: RateLimiter`
     - Update usage: `self.rate_limiter.acquire().await;`
     - Remove old implementation
     - Test: `cargo test -p wkmp-ai acoustid`
   - **Client 2:** services/musicbrainz_client.rs
   - **Client 3:** extractors/musicbrainz_client.rs
   - **Client 4:** acousticbrainz_client.rs

**Rationale:**
- **Risk:** LOW - Well-tested utility, incremental refactoring
- **Effort:** MEDIUM - Create utility + refactor 4 clients
- **Testability:** EXCELLENT - Unit tests + integration tests

**Rejected Alternatives:**
- Refactor all clients at once - REJECTED (higher risk)
- Keep duplicates - REJECTED (violates DRY principle)

---

### REQ-TD5-002: Break Up Long Functions

**Requirement:** Refactor functions >200 lines using extract-method

**Approach Selected:** Identify long functions + extract sub-steps

**Implementation Steps:**
1. **Find functions >200 lines:**
   ```bash
   # Manual review or use code analysis tool
   # Focus on:
   # - workflow_orchestrator/ (orchestration functions)
   # - api/ui/ (HTML generation functions)
   # - workflow/storage.rs (test helpers)
   ```

2. **For each long function:**
   - Identify logical sub-steps
   - Extract each sub-step to helper function
   - Name helpers clearly: `validate_input()`, `extract_data()`, `store_results()`
   - Update main function to call helpers
   - Test after extraction

3. **Refactoring pattern:**
   ```rust
   // BEFORE (300 lines)
   pub fn process_passage(...) -> Result<...> {
       // 50 lines: validation
       // 100 lines: extraction
       // 100 lines: fusion
       // 50 lines: storage
   }

   // AFTER (multiple focused functions)
   pub fn process_passage(...) -> Result<...> {
       validate_passage_input(...)?;
       let extracted = extract_passage_data(...)?;
       let fused = fuse_passage_metadata(extracted)?;
       store_passage_results(fused)?;
       Ok(())
   }

   fn validate_passage_input(...) -> Result<()> { /* 50 lines */ }
   fn extract_passage_data(...) -> Result<...> { /* 100 lines */ }
   fn fuse_passage_metadata(...) -> Result<...> { /* 100 lines */ }
   fn store_passage_results(...) -> Result<()> { /* 50 lines */ }
   ```

**Rationale:**
- **Risk:** MEDIUM - Large refactoring, but tests verify correctness
- **Effort:** HIGH - Many functions to refactor
- **Testability:** EXCELLENT - Tests verify behavior unchanged

**Rejected Alternatives:**
- Leave long functions - REJECTED (violates requirement)
- Extract arbitrarily to meet line count - REJECTED (harms readability)

---

### REQ-TD5-003: Consolidate Configuration Structs

**Requirement:** Unify PipelineConfig, SongProcessorConfig, ImportParameters into WorkflowConfig

**Approach Selected:** Create unified struct + builder + migrate usage

**Implementation Steps:**
1. **Create unified config:**
   File: `wkmp-common/src/config/workflow.rs`
   ```rust
   pub struct WorkflowConfig {
       pub acoustid_api_key: Option<String>,
       pub enable_musicbrainz: bool,
       pub enable_essentia: bool,
       pub enable_audio_derived: bool,
       pub min_quality_threshold: f32,
       pub import_params: ImportParameters,
   }

   impl WorkflowConfig {
       pub fn builder() -> WorkflowConfigBuilder {
           WorkflowConfigBuilder::default()
       }
   }

   #[derive(Default)]
   pub struct WorkflowConfigBuilder {
       // fields...
   }

   impl WorkflowConfigBuilder {
       pub fn acoustid_api_key(mut self, key: String) -> Self { /* ... */ }
       pub fn build(self) -> WorkflowConfig { /* ... */ }
   }
   ```

2. **Migrate PipelineConfig usage:**
   - Find all uses of PipelineConfig
   - Replace with WorkflowConfig
   - Test

3. **Remove SongProcessorConfig** (already dead code, deleted in Phase 1)

4. **Update ImportParameters usage:**
   - Embed in WorkflowConfig
   - Update construction sites

**Rationale:**
- **Risk:** MEDIUM - Config changes affect many callsites
- **Effort:** MEDIUM - Create new struct + migrate usage
- **Testability:** GOOD - Tests verify config still works

---

### REQ-TD5-004: Remove Magic Numbers

**Requirement:** Replace magic numbers with named constants

**Approach Selected:** Identify magic numbers + replace with constants

**Implementation Steps:**
1. **Find magic numbers:**
   ```bash
   grep -rn "Duration::from_secs([0-9]" wkmp-ai/src/ wkmp-common/src/ | grep -v "from_secs(0)"
   ```

2. **Define constants at module level:**
   ```rust
   // BEFORE
   tokio::time::sleep(Duration::from_secs(15)).await;

   // AFTER
   /// SSE keepalive interval (15 seconds)
   const SSE_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

   tokio::time::sleep(SSE_KEEPALIVE_INTERVAL).await;
   ```

3. **Categories to address:**
   - Status update intervals
   - SSE keepalive timeouts
   - Buffer sizes
   - Threshold values

4. **Document constants:**
   - Include units in doc comment
   - Explain purpose/rationale

**Rationale:**
- **Risk:** NONE - Simple rename, behavior unchanged
- **Effort:** LOW - Straightforward replacements
- **Testability:** EXCELLENT - Tests verify values unchanged

---

## Implementation Sequence

**Phase Order:** 1 → 2 → 3 → 4 → 5 (sequential recommended)

**Within Each Phase:**
- Complete requirements in priority order (CRITICAL → HIGH → MEDIUM → LOW)
- Test after each requirement completion
- Commit after tests pass
- Run cross-phase tests (AT-ALL-001, AT-ALL-002) after each phase

**Critical Path:**
1. Phase 1 (Quick Wins) - Reduces noise for later phases
2. Phase 2 (File Organization) - Improves navigability for Phase 3
3. Phase 3 (Error Handling) - Cleaner error paths for documentation
4. Phase 4 (Documentation) - Document before final refactoring
5. Phase 5 (Code Quality) - Last refinements

**Estimated Duration:** 4-6 weeks (per specification)
