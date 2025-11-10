# SPEC: Technical Debt Reduction - wkmp-ai & wkmp-common

**Document Type:** Technical Specification
**Status:** Approved for Planning
**Created:** 2025-11-10
**Source Analysis:** `wip/technical_debt_report.md`
**Scope:** Complete technical debt remediation for wkmp-ai and wkmp-common

---

## 1. Executive Summary

### 1.1 Purpose

This specification defines requirements for systematic technical debt reduction in wkmp-ai and wkmp-common codebases. Technical debt was identified through comprehensive analysis (see `wip/technical_debt_report.md`) and classified into 5 remediation phases.

### 1.2 Problem Statement

Current technical debt assessment reveals:
- **2 Critical Issues:** Blocking async sleep, 2,253-line monolithic file
- **7 High Priority Issues:** Dead code, excessive unwrap() usage, missing documentation
- **12 Medium Priority Issues:** Code duplication, long functions, configuration sprawl
- **8 Low Priority Issues:** Magic numbers, logging inconsistencies

**Impact if unaddressed:**
- Maintainability decline (merge conflicts, cognitive load)
- Bug introduction risk increases
- Onboarding friction for new developers
- Performance degradation over time
- Crash risk from 506 unwrap() calls

**Estimated cost of deferral:** 2-3x effort after 6 months

### 1.3 Goals

**Primary Goals:**
1. Eliminate all critical and high-priority technical debt
2. Improve code maintainability and readability
3. Reduce crash risk through proper error handling
4. Establish documentation baseline for public APIs

**Success Metrics:**
- Zero compiler warnings
- Zero clippy warnings
- <50 unwrap() calls in production code
- 100% public API documentation coverage
- All files <800 lines (largest file reduction from 2,253 → 650 lines)
- All 216 tests continue passing

---

## 2. Scope and Boundaries

### 2.1 In Scope

**Modules:**
- `wkmp-ai/src/**/*.rs` (all Rust source files)
- `wkmp-common/src/**/*.rs` (all Rust source files)

**Work Categories:**
- File reorganization and modularization
- Error handling improvements
- Dead code removal
- Documentation additions
- Code quality improvements
- Compiler/clippy warning resolution

### 2.2 Out of Scope

**Explicitly Excluded:**
- Functional changes (behavior must remain identical)
- Performance optimizations (defer to separate effort)
- New features or capabilities
- Test additions (maintain existing 216 tests, ensure they pass)
- Other modules (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-le, wkmp-dr)
- Architectural redesign

### 2.3 Constraints

**Technical:**
- All 216 existing tests MUST continue passing
- Public API contracts MUST NOT change (backward compatibility)
- Rust stable channel only
- No new dependencies without approval

**Process:**
- Incremental refactoring (no "big bang" rewrites)
- Each phase independently deliverable
- Continuous integration (tests pass after each increment)

---

## 3. Phase 1: Quick Wins (Critical Fixes)

**Objective:** Eliminate critical issues and auto-fixable warnings with minimal risk

**Estimated Effort:** 1-2 days

---

### REQ-TD1-001: Replace Blocking Sleep in Async Context

**Priority:** CRITICAL
**Location:** `wkmp-common/src/time.rs:37`

**Requirement:**
The system SHALL replace `std::thread::sleep` with `tokio::time::sleep` in async test context.

**Rationale:**
- Blocking sleep prevents async runtime from executing other tasks
- Reduces concurrency (thread unavailable for duration)
- Violates async/await contract

**Acceptance Criteria:**
1. `std::thread::sleep` removed from async context
2. Replaced with `tokio::time::sleep().await`
3. Test `test_now_successive_calls_advance` still passes
4. No other async tests affected

**Impact:**
- **Before:** Thread blocked for 10ms during test
- **After:** Async runtime can execute other tasks during wait

---

### REQ-TD1-002: Delete Dead Code (song_processor.rs)

**Priority:** CRITICAL
**Location:** `wkmp-ai/src/workflow/song_processor.rs` (368 lines)

**Requirement:**
The system SHALL remove the unused `song_processor.rs` file and its commented-out module declaration.

**Rationale:**
- File is completely unused (commented out in mod.rs:11)
- Causes confusion ("Is this still needed?")
- 368 lines of maintenance burden
- False positives in code searches

**Acceptance Criteria:**
1. File `wkmp-ai/src/workflow/song_processor.rs` deleted
2. Comment `// pub mod song_processor;` removed from `wkmp-ai/src/workflow/mod.rs:11`
3. All imports referencing song_processor removed
4. All 216 tests still pass
5. Project compiles without errors

**Impact:**
- -368 lines of unused code
- Reduced confusion for developers

---

### REQ-TD1-003: Fix Compiler Warnings

**Priority:** HIGH
**Metrics:** 11 compiler warnings identified

**Requirement:**
The system SHALL eliminate all compiler warnings in wkmp-ai and wkmp-common.

**Warning Categories:**
1. Unused imports (4 warnings)
2. Unused variables (2 warnings)
3. Dead code fields/methods (5 warnings)

**Acceptance Criteria:**
1. `cargo build -p wkmp-ai -p wkmp-common` produces zero warnings
2. All unused imports removed
3. All unused variables removed or prefixed with `_`
4. Dead code removed or marked with `#[allow(dead_code)]` with justification comment
5. All 216 tests still pass

**Specific Fixes Required:**

| Warning | Location | Fix |
|---------|----------|-----|
| Unused import `AudioFile` | `id3_extractor.rs:19` | Remove import |
| Unused import `warn` | `essentia_analyzer.rs:41` | Remove import |
| Unused import `warn` | `id3_genre_mapper.rs:23` | Remove import |
| Unused import `ExtractionError` | `chromaprint_analyzer.rs:35` | Remove import |
| Unused variable `event_bus` | `workflow_orchestrator.rs:1749` | Prefix with `_event_bus` |
| Unused variable `sample_rate` | `audio_derived_extractor.rs:224` | Prefix with `_sample_rate` |
| Dead field `id` | `acoustid_client.rs:335` | Remove or justify |
| Dead fields | `metadata_fuser.rs` | Remove or justify |
| Dead method `string_similarity` | `metadata_fuser.rs:234` | Remove or justify |
| Dead function `levenshtein_distance` | `metadata_fuser.rs:317` | Remove or justify |

---

### REQ-TD1-004: Fix Clippy Lints

**Priority:** HIGH
**Metrics:** 22 clippy warnings identified

**Requirement:**
The system SHALL address all clippy warnings using auto-fix where possible, manual fixes otherwise.

**Acceptance Criteria:**
1. `cargo clippy -p wkmp-ai -p wkmp-common` produces zero warnings
2. Auto-fixable warnings resolved via `cargo clippy --fix`
3. Manual fixes applied with clear rationale
4. All 216 tests still pass

**Known Lints to Address:**
- Methods called `from_*` usually take no `self` (3 instances)
- Manual `!RangeInclusive::contains` implementation (4 instances)
- Unnecessary map of identity function
- Using `clone` on `Copy` type (Fingerprinter)
- Clamp-like pattern without using clamp function
- Use of `or_insert_with` to construct default value

**Approach:**
1. Run `cargo clippy --fix --lib -p wkmp-ai --allow-dirty`
2. Run `cargo clippy --fix --lib -p wkmp-common --allow-dirty`
3. Review auto-fixes, commit
4. Manually address remaining warnings
5. Verify tests pass

---

### REQ-TD1-005: Fix Panic Statements in Production Code

**Priority:** HIGH
**Locations:** 2 real panics (4 false positives in test code)

**Requirement:**
The system SHALL replace panic! macros in production code with proper error handling.

**Panics to Fix:**

**1. events.rs:1564**
```rust
// BEFORE
_ => panic!("Wrong event type deserialized"),

// AFTER
_ => return Err(serde::de::Error::custom(
    format!("Invalid event type: expected {}, got {}", expected, actual)
)),
```

**2. extractors/mod.rs:77**
```rust
// BEFORE
unimplemented!()

// AFTER
// Either implement or remove entirely if unused
```

**Acceptance Criteria:**
1. No `panic!` or `unimplemented!` calls in production code paths
2. All error paths return `Result<T, E>` types
3. Error messages are descriptive
4. Tests verify new error paths
5. All 216 tests still pass

**False Positives (No Action Required):**
- `file_scanner.rs:320,332` - Inside test helper functions
- `schema_sync.rs:545,586` - Inside test code

---

## 4. Phase 2: File Organization

**Objective:** Reduce file sizes to <800 lines for improved maintainability

**Estimated Effort:** 1-2 weeks

---

### REQ-TD2-001: Refactor Workflow Orchestrator into Modules

**Priority:** CRITICAL
**Current State:** 2,253 lines in single file (6.6% of codebase)
**Target State:** 7-8 module files, largest <650 lines

**Requirement:**
The system SHALL decompose `workflow_orchestrator.rs` into modular structure organized by workflow phase.

**Target Structure:**
```
wkmp-ai/src/services/workflow_orchestrator/
├── mod.rs                    (~200 lines - coordinator, exports)
├── phase_scanning.rs         (~300 lines - filesystem traversal)
├── phase_extraction.rs       (~350 lines - ID3 metadata)
├── phase_fingerprinting.rs   (~400 lines - Chromaprint + AcoustID)
├── phase_segmenting.rs       (~300 lines - silence detection)
├── phase_analyzing.rs        (~250 lines - amplitude analysis)
├── phase_flavoring.rs        (~250 lines - AcousticBrainz/Essentia)
└── entity_linking.rs         (~300 lines - song/artist/album linking)
```

**Acceptance Criteria:**
1. Each phase module is self-contained (<400 lines)
2. `mod.rs` coordinates phases (no phase implementation logic)
3. Public API unchanged (backward compatibility)
4. All 216 tests pass
5. No duplication between modules
6. Clear module-level documentation

**Approach:**
1. Create `workflow_orchestrator/` directory
2. Extract each `phase_*` method to separate file
3. Create `mod.rs` with coordinator logic
4. Update imports throughout codebase
5. Verify tests pass after each extraction

**Benefit:**
- Largest file reduced from 2,253 → 650 lines (71% reduction)
- Each phase independently maintainable
- Reduced merge conflict risk
- Improved code navigation

---

### REQ-TD2-002: Split events.rs by Category

**Priority:** HIGH
**Current State:** 1,711 lines in single file
**Target State:** 3-4 module files, largest <600 lines

**Requirement:**
The system SHALL decompose `wkmp-common/src/events.rs` into category-specific modules.

**Target Structure:**
```
wkmp-common/src/events/
├── mod.rs                  (~100 lines - re-exports)
├── import_events.rs        (~500 lines - import workflow events)
├── playback_events.rs      (~400 lines - audio playback events)
├── system_events.rs        (~300 lines - system/config events)
└── sse_formatting.rs       (~400 lines - SSE serialization)
```

**Acceptance Criteria:**
1. Each event category in separate file
2. `mod.rs` re-exports all types (public API unchanged)
3. No circular dependencies
4. All 216 tests pass
5. SSE event formatting preserved

---

### REQ-TD2-003: Split params.rs by Parameter Group

**Priority:** HIGH
**Current State:** 1,450 lines in single file
**Target State:** 4-5 module files, largest <400 lines

**Requirement:**
The system SHALL decompose `wkmp-common/src/params.rs` into parameter group modules.

**Target Structure:**
```
wkmp-common/src/params/
├── mod.rs                  (~100 lines - re-exports)
├── crossfade_params.rs     (~300 lines - crossfade settings)
├── selector_params.rs      (~350 lines - passage selection)
├── timing_params.rs        (~250 lines - timing/duration)
├── flavor_params.rs        (~300 lines - musical flavor)
└── system_params.rs        (~200 lines - system configuration)
```

**Acceptance Criteria:**
1. Parameters grouped by functional area
2. `mod.rs` re-exports all types
3. Default implementations preserved
4. All 216 tests pass
5. Database serialization unchanged

---

### REQ-TD2-004: Reorganize api/ui.rs into Page Modules

**Priority:** MEDIUM
**Current State:** 1,308 lines in single file
**Target State:** 5-6 module files, largest <300 lines

**Requirement:**
The system SHALL decompose `wkmp-ai/src/api/ui.rs` into page-specific modules.

**Target Structure:**
```
wkmp-ai/src/api/ui/
├── mod.rs                  (~100 lines - router, exports)
├── dashboard_page.rs       (~200 lines - main dashboard HTML)
├── settings_page.rs        (~250 lines - settings form)
├── library_page.rs         (~300 lines - library view)
├── import_page.rs          (~250 lines - import wizard)
└── components.rs           (~200 lines - shared HTML components)
```

**Acceptance Criteria:**
1. Each page in separate file
2. Shared components extracted to `components.rs`
3. Routes preserved (URLs unchanged)
4. All 216 tests pass
5. HTML output byte-for-byte identical

**Approach:**
1. Extract page handler functions to modules
2. Extract shared HTML generation to components
3. Keep route registration in mod.rs
4. Test each page independently

---

## 5. Phase 3: Error Handling Audit

**Objective:** Replace unwrap() calls with proper error handling

**Estimated Effort:** 2-3 days

---

### REQ-TD3-001: Audit and Classify unwrap() Usage

**Priority:** HIGH
**Current State:** 506 unwrap()/expect() calls (excluding tests)
**Target State:** <50 justified unwrap() calls, rest converted to error handling

**Requirement:**
The system SHALL audit all unwrap() and expect() calls and classify as:
1. **Justifiable:** Startup config, invariants, compile-time guarantees
2. **Convertible:** Can return Result, should use `?` operator
3. **Removable:** Can use safer alternatives (unwrap_or, unwrap_or_else)

**Acceptance Criteria:**
1. All 506 calls reviewed and classified
2. Classification documented in audit report
3. Priority list for conversion created
4. Justifiable unwraps have explanatory comments

**Classification Criteria:**

**Justifiable (Keep with Comment):**
- Config file required for startup: `config.parse().expect("Config required")`
- FFI library required: `ChromaprintContext::new().expect("Chromaprint required")`
- Compile-time verified invariants: `Some(value).unwrap()` in test assertions

**Convertible (Replace with `?`):**
- User input handling
- File I/O operations
- Network requests
- Database queries

**Removable (Use Safer Alternative):**
- Default values: Use `unwrap_or_default()`
- Fallback logic: Use `unwrap_or_else(|| ...)`
- Optional chaining: Use `and_then()`, `map()`

---

### REQ-TD3-002: Convert User-Facing unwrap() Calls

**Priority:** HIGH
**Target:** Convert 200+ high-priority unwrap() calls

**Requirement:**
The system SHALL convert unwrap() calls in user-facing code paths to proper error handling.

**High-Priority Locations:**
- HTTP request handlers
- File import logic
- Database operations
- Configuration parsing (user-provided configs)

**Conversion Pattern:**
```rust
// BEFORE
let client = Client::builder().build().expect("Failed to create HTTP client");

// AFTER
let client = Client::builder().build()
    .context("Failed to create HTTP client")?;
```

**Acceptance Criteria:**
1. All user-facing paths return `Result<T, E>`
2. Error messages are descriptive and actionable
3. Errors include context (what was attempted, why it failed)
4. Error propagation uses `?` operator
5. All 216 tests pass
6. New tests verify error paths

**Categories to Convert (Priority Order):**
1. HTTP API handlers (highest user impact)
2. File import workflow (data loss risk)
3. Database operations (corruption risk)
4. Configuration loading (startup failures)

---

### REQ-TD3-003: Add Error Context with anyhow

**Priority:** MEDIUM

**Requirement:**
The system SHALL add descriptive error context to all error propagation sites using `anyhow::Context`.

**Pattern:**
```rust
// BEFORE
let data = load_file(path)?;

// AFTER
let data = load_file(path)
    .with_context(|| format!("Failed to load passage data from {}", path.display()))?;
```

**Acceptance Criteria:**
1. All `?` operators in critical paths have context
2. Error messages include operation attempted
3. Error messages include relevant identifiers (file paths, UUIDs, etc.)
4. Context added without changing error types
5. All 216 tests pass

**Scope:**
- Import workflow error paths
- Database operation failures
- Network API errors
- File system operations

---

## 6. Phase 4: Documentation

**Objective:** Document all public APIs and modules

**Estimated Effort:** 1 week

---

### REQ-TD4-001: Enable missing_docs Lint

**Priority:** HIGH

**Requirement:**
The system SHALL enable `#![warn(missing_docs)]` in lib.rs for wkmp-ai and wkmp-common.

**Acceptance Criteria:**
1. Lint enabled in both crates
2. Generates warnings for undocumented public items
3. Does not block compilation (warn, not deny)
4. Baseline documented before enforcement

**Implementation:**
```rust
// Add to lib.rs
#![warn(missing_docs)]
```

---

### REQ-TD4-002: Document Public Modules

**Priority:** HIGH
**Target:** 100% of public modules have module-level documentation

**Requirement:**
The system SHALL add module-level documentation (`//!` doc comments) to all public modules.

**Module Documentation Standard:**
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
//! [Optional: How this module fits into larger system]
```

**Acceptance Criteria:**
1. All public modules have `//!` documentation
2. Documentation includes purpose, usage, examples (where applicable)
3. Documentation is accurate (verified against code)
4. All 216 tests pass
5. `cargo doc` generates complete documentation

**Scope:**
- All modules in wkmp-ai/src/
- All modules in wkmp-common/src/
- Focus on public API modules first

---

### REQ-TD4-003: Document Public Functions

**Priority:** MEDIUM
**Target:** 100% of public functions documented
**Current:** ~34-51% undocumented

**Requirement:**
The system SHALL add documentation comments (`///`) to all public functions.

**Function Documentation Standard:**
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
pub fn function_name(param1: Type1, param2: Type2) -> Result<ReturnType, ErrorType> {
    // implementation
}
```

**Acceptance Criteria:**
1. All public functions (294 total) have documentation
2. Parameters documented with types and purpose
3. Return values explained
4. Error conditions documented
5. Examples provided for complex functions
6. All 216 tests pass
7. `cargo doc` generates complete API docs

**Approach:**
1. Generate list of undocumented functions
2. Prioritize by module (core modules first)
3. Document in batches of 20-30 functions
4. Review for accuracy and completeness

---

## 7. Phase 5: Code Quality Improvements

**Objective:** Reduce code duplication and improve maintainability

**Estimated Effort:** 1-2 weeks

---

### REQ-TD5-001: Extract Rate Limiter Utility

**Priority:** MEDIUM
**Current:** 4 duplicate implementations

**Requirement:**
The system SHALL extract rate limiting logic to shared utility in wkmp-common.

**Current Duplications:**
1. `wkmp-ai/src/services/acoustid_client.rs` (90 lines)
2. `wkmp-ai/src/services/musicbrainz_client.rs` (107 lines)
3. `wkmp-ai/src/extractors/musicbrainz_client.rs` (114 lines)
4. `wkmp-ai/src/services/acousticbrainz_client.rs` (192 lines)

**Target Structure:**
```rust
// wkmp-common/src/rate_limiter.rs
pub struct RateLimiter {
    last_request: Arc<Mutex<Option<Instant>>>,
    interval: Duration,
}

impl RateLimiter {
    pub fn new(interval: Duration) -> Self { /* ... */ }
    pub async fn acquire(&self) { /* ... */ }
}
```

**Usage Pattern:**
```rust
// In API client
struct MusicBrainzClient {
    rate_limiter: RateLimiter,
    // ...
}

impl MusicBrainzClient {
    async fn query(&self) {
        self.rate_limiter.acquire().await;
        // Make request
    }
}
```

**Acceptance Criteria:**
1. `RateLimiter` utility created in wkmp-common
2. All 4 clients refactored to use shared utility
3. Rate limiting behavior preserved (1 req/sec)
4. All 216 tests pass
5. Tests verify rate limiting still works

**Benefit:**
- 4 implementations → 1 implementation
- Single source of truth for rate limiting
- Fix bugs once, applies to all clients

---

### REQ-TD5-002: Break Up Long Functions

**Priority:** MEDIUM
**Target:** All functions <200 lines
**Current:** 450+ functions >150 lines

**Requirement:**
The system SHALL refactor functions >200 lines using extract-method pattern.

**Priority Functions (>300 lines):**
1. `workflow_orchestrator.rs` functions (20+ long functions)
2. `api/ui.rs` HTML generation functions (200-700 lines)
3. `workflow/storage.rs` test helper functions (200-300 lines)

**Refactoring Pattern:**
```rust
// BEFORE: 300-line function
pub fn process_passage(...) -> Result<...> {
    // 50 lines: validation
    // 100 lines: extraction
    // 100 lines: fusion
    // 50 lines: storage
}

// AFTER: Multiple focused functions
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

**Acceptance Criteria:**
1. No functions >200 lines
2. Extracted functions have single responsibility
3. Function names clearly describe purpose
4. All 216 tests pass
5. Logic preserved (no behavior changes)

**Approach:**
1. Focus on orchestrator first (highest complexity)
2. Extract logical sub-steps to helper functions
3. Add documentation to extracted functions
4. Verify tests pass after each extraction

---

### REQ-TD5-003: Consolidate Configuration Structs

**Priority:** MEDIUM

**Requirement:**
The system SHALL consolidate duplicate configuration structs into unified `WorkflowConfig`.

**Current Duplication:**
- `PipelineConfig` (workflow/pipeline.rs)
- `SongProcessorConfig` (workflow/song_processor.rs - DEAD CODE, remove)
- `ImportParameters` (models/import_parameters.rs)

**Target Structure:**
```rust
// wkmp-common/src/config/workflow.rs
pub struct WorkflowConfig {
    // API keys
    pub acoustid_api_key: Option<String>,

    // Feature flags
    pub enable_musicbrainz: bool,
    pub enable_essentia: bool,
    pub enable_audio_derived: bool,

    // Quality thresholds
    pub min_quality_threshold: f32,

    // Import parameters
    pub import_params: ImportParameters,
}

impl WorkflowConfig {
    pub fn builder() -> WorkflowConfigBuilder { /* ... */ }
}
```

**Acceptance Criteria:**
1. Single `WorkflowConfig` struct replaces duplicates
2. Builder pattern for optional fields
3. All existing usages updated
4. All 216 tests pass
5. No behavior changes

---

### REQ-TD5-004: Remove Magic Numbers

**Priority:** LOW

**Requirement:**
The system SHALL replace magic numbers with named constants.

**Examples:**
```rust
// BEFORE
tokio::time::sleep(Duration::from_secs(15)).await;

// AFTER
const SSE_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
tokio::time::sleep(SSE_KEEPALIVE_INTERVAL).await;
```

**Scope:**
- Status update intervals
- SSE keepalive timeouts
- Buffer sizes
- Threshold values

**Acceptance Criteria:**
1. All magic numbers >1 replaced with constants
2. Constants have descriptive names
3. Constants documented with units/meaning
4. All 216 tests pass

---

## 8. Cross-Phase Requirements

### REQ-TD-ALL-001: Test Preservation

**Priority:** CRITICAL

**Requirement:**
All existing tests SHALL continue to pass throughout all phases of technical debt reduction.

**Acceptance Criteria:**
1. All 216 tests pass after each increment
2. No tests skipped or disabled
3. No test behavior changes (same assertions, same coverage)
4. Test execution time does not increase >10%

**Verification:**
```bash
# Run after each increment
cargo test -p wkmp-ai -p wkmp-common --all-features

# Expected output
test result: ok. 216 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

### REQ-TD-ALL-002: Backward Compatibility

**Priority:** CRITICAL

**Requirement:**
Public APIs SHALL NOT change in breaking ways.

**Acceptance Criteria:**
1. All public function signatures unchanged
2. All public types unchanged
3. All public constants unchanged
4. Semantic versioning preserved (patch version bumps only)
5. No breaking changes require major version bump

**Allowed Changes:**
- Adding new public functions (non-breaking)
- Adding private helper functions
- Reorganizing internal module structure
- Improving documentation

**Forbidden Changes:**
- Removing public functions
- Changing public function signatures
- Changing public type definitions
- Changing behavior of existing functions

---

### REQ-TD-ALL-003: Incremental Delivery

**Priority:** HIGH

**Requirement:**
Each phase SHALL be independently deliverable and verifiable.

**Acceptance Criteria:**
1. Each phase has clear entry/exit criteria
2. Phases can be completed in order without blocking
3. Partial completion does not leave codebase in broken state
4. Git commits after each increment (tests passing)

**Verification:**
- After each increment: Commit with tests passing
- After each phase: Tag release with phase complete
- CI pipeline green for all commits

---

### REQ-TD-ALL-004: Documentation Updates

**Priority:** MEDIUM

**Requirement:**
All refactoring SHALL include corresponding documentation updates.

**Acceptance Criteria:**
1. Module moves update import documentation
2. Extracted functions have documentation
3. README updated if public API changes
4. CHANGELOG entries for each phase

**Format:**
```markdown
## CHANGELOG

### Phase 1: Quick Wins (2025-11-11)
- Fixed blocking sleep in async context
- Removed dead code (song_processor.rs)
- Eliminated all compiler warnings
- Fixed clippy lints (22 warnings)
- Removed panic! calls in production code
```

---

## 9. Success Criteria

### 9.1 Quantitative Metrics

**After Complete Implementation:**

| Metric | Current | Target | Success |
|--------|---------|--------|---------|
| Largest File | 2,253 lines | <800 lines | Reduced to 650 lines |
| Compiler Warnings | 11 | 0 | Zero warnings |
| Clippy Warnings | 22 | 0 | Zero warnings |
| unwrap() Calls (prod) | 506 | <50 | <50 justified calls |
| Dead Code Files | 1 (368 lines) | 0 | Zero dead code |
| Public API Docs | ~55% | 100% | 100% documented |
| Files >1000 Lines | 5 | 2 | 3 files reduced |
| Test Pass Rate | 100% | 100% | 216/216 passing |

---

### 9.2 Qualitative Criteria

**Code Quality:**
- ✅ All functions have clear single responsibility
- ✅ Error handling is comprehensive and informative
- ✅ No panic! calls in production code paths
- ✅ Code is easily navigable (no 2000+ line files)

**Maintainability:**
- ✅ New developers can understand module structure
- ✅ Changes to one phase don't affect others (modularity)
- ✅ Error messages guide users to resolution
- ✅ Documentation explains "why" not just "what"

**Robustness:**
- ✅ Proper error propagation throughout
- ✅ No silent failures
- ✅ Async operations don't block runtime
- ✅ Resource cleanup guaranteed (Drop implementations)

---

## 10. Dependencies and Assumptions

### 10.1 Existing Dependencies

**Must Remain Stable:**
- Rust stable toolchain
- tokio async runtime
- axum web framework
- sqlx database library
- All existing dependencies in Cargo.toml

### 10.2 New Dependencies (Approved)

**Phase 3: Error Handling**
- `anyhow` crate (already in use, just expanded usage)

**No new external dependencies required for other phases.**

### 10.3 Assumptions

1. **Test Suite Correctness:** Existing 216 tests accurately verify behavior
2. **No Concurrent Changes:** No major feature work during refactoring
3. **Tooling Availability:** cargo, clippy, rustfmt available
4. **Time Allocation:** 4-6 weeks of dedicated effort available

---

## 11. Risks and Mitigations

### 11.1 Risk: Breaking Existing Functionality

**Probability:** Medium
**Impact:** High
**Mitigation:**
- Run tests after every increment
- Use type system to catch breaking changes
- Incremental refactoring (not big bang)
- Code review each phase

---

### 11.2 Risk: Scope Creep

**Probability:** Medium
**Impact:** Medium
**Mitigation:**
- Strict scope definition (no functional changes)
- Reject "improvements" that add features
- Focus on technical debt only
- Phase-based delivery (can stop after any phase)

---

### 11.3 Risk: Merge Conflicts

**Probability:** Low (assuming dedicated work)
**Impact:** Medium
**Mitigation:**
- Complete in 4-6 week window
- Communicate refactoring to team
- Frequent commits (after each passing increment)
- Rebase frequently if other work ongoing

---

### 11.4 Risk: Performance Regression

**Probability:** Low
**Impact:** Medium
**Mitigation:**
- No algorithmic changes (only structure)
- Benchmark critical paths before/after
- Profile if performance concerns arise
- Defer optimizations to separate effort

---

## 12. Out of Scope (Explicitly Deferred)

**Not Included in This Specification:**

1. **Performance Optimizations**
   - String allocation improvements
   - Clone reduction (strategic review needed)
   - Async function coloring analysis
   - Defer to separate performance initiative

2. **Test Additions**
   - New test coverage
   - Integration test expansion
   - Performance benchmarks
   - Maintain existing 216 tests only

3. **Architectural Changes**
   - Trait object vs enum_dispatch
   - Event bus subscription tracking
   - Database query builders
   - Major design decisions deferred

4. **Security Enhancements**
   - Security audits
   - Dependency vulnerability fixes
   - Trust boundary validation
   - Separate security review required

5. **New Features**
   - Any behavior changes
   - New capabilities
   - API expansions
   - This is refactoring only

---

## 13. References

**Source Documents:**
- `wip/technical_debt_report.md` - Complete technical debt analysis
- `CLAUDE.md` - Project coding standards and conventions
- `docs/GOV001-document_hierarchy.md` - Documentation governance
- `docs/GOV002-requirements_enumeration.md` - Requirement ID scheme

**Related Standards:**
- Rust API Guidelines
- Tokio best practices
- Error handling patterns (anyhow, thiserror)

---

## 14. Approval and Sign-Off

**Specification Status:** Ready for Planning

**Next Steps:**
1. Run `/plan wip/SPEC_technical_debt_reduction.md`
2. Generate implementation plan with acceptance tests
3. Review plan for completeness
4. Begin Phase 1 implementation

**Prepared By:** Claude Code Technical Debt Analysis
**Date:** 2025-11-10
**Version:** 1.0
