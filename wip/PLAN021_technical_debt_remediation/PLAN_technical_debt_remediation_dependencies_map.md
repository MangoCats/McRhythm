# Technical Debt Remediation - Dependencies Map

**Source Specification:** SPEC_technical_debt_remediation.md
**Generated:** 2025-11-05
**Plan Workflow Phase:** Phase 1 - Input Validation and Scope Definition

---

## Overview

This document maps all dependencies for the technical debt remediation project, including:
- Existing code and components (what we're modifying)
- Required documentation (what we're referencing)
- External tools and systems (what we're using)
- Inter-requirement dependencies (what depends on what)

---

## Existing Code Components

### Core Components Being Modified

**wkmp-ap/src/playback/engine/core.rs** (3,156 LOC)
- **Current State:** Single monolithic file, violates <1,500 LOC guideline
- **Purpose:** Core playback orchestration, lifecycle, queue management
- **Dependencies:**
  - Uses: BufferManager, DecoderWorker, Mixer, SharedState
  - Referenced by: PlaybackEngine public API
  - Tested by: Unit tests, integration tests, benchmarks
- **Specification:** SPEC016-decoder_buffer_design.md, SPEC028-playback_orchestration.md
- **Modification:** Refactor into modules <1,000 LOC (FR-001, Increment 2)

**wkmp-ap/src/api/auth_middleware.rs** (925 LOC)
- **Current State:** Contains 530+ LOC deprecated code (lines 250-800)
- **Purpose:** HTTP authentication middleware (V1 deprecated, V2 active)
- **Dependencies:**
  - Uses: axum, tower, wkmp_common::api
  - Referenced by: api/server.rs
  - Tested by: Integration tests
- **Specification:** SPEC007-api_design.md (V2 auth)
- **Modification:** Remove deprecated V1 code (FR-002, Increment 3)

**wkmp-ap/src/playback/diagnostics.rs** (514 LOC)
- **Current State:** Duplicate of playback/engine/diagnostics.rs
- **Purpose:** Pipeline diagnostics and validation
- **Dependencies:**
  - DUPLICATE of: playback/engine/diagnostics.rs (860 LOC)
  - Used by: Unclear (requires analysis)
- **Modification:** Consolidate into single module (FR-002, Increment 3)
- **Decision Needed:** Which version is canonical? (likely engine/diagnostics.rs)

**wkmp-ap/src/playback/engine/diagnostics.rs** (860 LOC)
- **Current State:** Canonical diagnostics implementation
- **Purpose:** Pipeline validation, buffer monitoring, integrity checks
- **Dependencies:**
  - Uses: DecoderChain, BufferManager, ChainMonitor
  - Referenced by: PlaybackEngine, validation service
  - Tested by: Unit tests
- **Specification:** SPEC016, SPEC028 (validation requirements)
- **Modification:** Resolve duplication with playback/diagnostics.rs

**wkmp-ap/src/config.rs** (206 LOC)
- **Current State:** Legacy Config struct, marked "Phase 4" compatibility
- **Purpose:** Bootstrap configuration (superseded by wkmp_common::config)
- **Dependencies:**
  - Uses: wkmp_common::config::RootFolderResolver
  - Referenced by: main.rs (minimal usage)
  - Tested by: Unit tests
- **Modification:** Evaluate usage, remove if obsolete (FR-002, Increment 6)
- **Requires:** Grep for all usage in wkmp-ap and other modules

---

### Components Referenced by DEBT Markers

**DEBT-007: Source Sample Rate Telemetry**
- **Location:** TBD (likely decoder.rs, events.rs)
- **Purpose:** Track source file sample rates for telemetry
- **Dependencies:**
  - Reads: Audio file metadata (symphonia)
  - Emits: Events or stores in database
  - Accessed: Via API or diagnostics
- **Specification:** Underspecified (see Open Questions)
- **Modification:** Implement per specification (FR-003, Increment 4)

**REQ-DEBT-FUNC-002: Duration Played Calculation**
- **Location:** TBD (likely playback/engine/core.rs or events.rs)
- **Purpose:** Track cumulative playback time per passage
- **Dependencies:**
  - Reads: Frame position, sample rate
  - Stores: Cumulative duration
  - Emits: Via events
- **Specification:** Underspecified (calculation point unclear)
- **Modification:** Implement per specification (FR-003, Increment 4)

**REQ-DEBT-FUNC-003: Album Metadata for Events**
- **Location:** TBD (likely events.rs, db layer)
- **Purpose:** Include album UUID in playback events
- **Dependencies:**
  - Reads: Database (passages → recordings → releases)
  - Emits: Enriched events with album UUID
  - Downstream: wkmp-ui displays album info
- **Specification:** Underspecified (fetching logic location unclear)
- **Modification:** Implement per specification (FR-003, Increment 4)

**DEBT-002: Audio Clipping Detection**
- **Location:** TBD (likely mixer.rs or output.rs)
- **Purpose:** Detect audio samples exceeding ±1.0 range
- **Dependencies:**
  - Reads: Mixed audio samples
  - Emits: Clipping events or logs
- **Priority:** OPTIONAL (may defer to future work)
- **Modification:** Implement if time permits (FR-003, Increment 4)

---

### Components Tested But Not Modified

**wkmp-common/** (shared library)
- **Current State:** 5 clippy warnings, large error variant warnings
- **Purpose:** Shared database models, events, config utilities
- **Dependencies:**
  - Used by: ALL modules (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le, wkmp-dr)
  - Tested by: wkmp-common unit tests + all downstream modules
- **Modification:** Fix clippy warnings, NO API changes (FR-004, Increment 5)
- **Constraint:** API compatibility MUST be preserved (NFR-003)

**wkmp-ui/** (User Interface)
- **Modification:** NONE
- **Testing:** REQUIRED (validate no regressions from wkmp-common changes)

**wkmp-pd/** (Program Director)
- **Modification:** NONE
- **Testing:** REQUIRED (validate no regressions)

**wkmp-ai, wkmp-le, wkmp-dr** (optional modules)
- **Modification:** NONE
- **Testing:** REQUIRED if available (validate no regressions)

---

## Required Documentation

### Specification Documents (READ-ONLY)

**SPEC016-decoder_buffer_design.md**
- **Purpose:** Authoritative design for decoder-buffer chain architecture
- **Usage:** Guide core.rs refactoring (Increment 2)
- **Key Sections:**
  - Serial decoder strategy rationale
  - DecoderChain lifecycle
  - Buffer capacity management
- **Status:** AVAILABLE (read in previous conversation)

**SPEC028-playback_orchestration.md**
- **Purpose:** Event-driven playback engine design
- **Usage:** Guide core.rs refactoring (Increment 2)
- **Key Sections:**
  - Engine lifecycle states
  - Component interaction patterns
  - Event-driven coordination
- **Status:** AVAILABLE (read in previous conversation)

**SPEC002-crossfade.md**
- **Purpose:** Crossfade timing and algorithm design
- **Usage:** Reference for engine/core.rs refactoring
- **Key Sections:**
  - 6 timing points (fade_start, fade_mid, fade_end, etc.)
  - Sample-accurate positioning
- **Status:** AVAILABLE (read in previous conversation)

**SPEC007-api_design.md**
- **Purpose:** HTTP API and authentication design
- **Usage:** Reference for auth_middleware cleanup (Increment 3)
- **Key Sections:**
  - V2 authentication (shared_secret)
  - V1 deprecation (to be removed)
- **Status:** AVAILABLE

**REQ001-requirements.md**
- **Purpose:** Top-level requirements
- **Usage:** Validate DEBT implementations meet requirements
- **Status:** AVAILABLE

---

### Implementation Documents (MAY UPDATE)

**IMPL002-coding_conventions.md**
- **Purpose:** Coding standards and conventions
- **Usage:** Guide refactoring style (Increment 2)
- **Key Sections:**
  - File size limits (<1,500 LOC guideline)
  - Module organization patterns
  - Documentation standards
- **Status:** AVAILABLE

**IMPL003-project_structure.md**
- **Purpose:** Project structure documentation
- **Usage:** UPDATE after refactoring complete (Increment 7)
- **Modifications:**
  - Reflect current wkmp-ap src/ organization
  - Remove obsolete module references
  - Update key files list
- **Status:** AVAILABLE (requires update)

**IMPL008-decoder_worker_implementation.md**
- **Purpose:** DecoderWorker architecture documentation (NEW)
- **Usage:** CREATE in Increment 7
- **Content:**
  - Architecture overview
  - Component structure
  - Key algorithms
  - Performance characteristics
  - Integration points
  - Testing approach
- **Status:** DOES NOT EXIST (to be created)

**Buffer Tuning Workflow Documentation**
- **Purpose:** Operator guide for tune-buffers utility (NEW)
- **Usage:** CREATE in Increment 7
- **Content:**
  - When to tune
  - Tuning process
  - Parameter recommendations
  - Applying results
  - Troubleshooting
- **Status:** DOES NOT EXIST (to be created)
- **Location:** TBD (IMPL009, README section, or docs/operators/)

---

### Governance Documents (READ-ONLY)

**GOV001-document_hierarchy.md**
- **Purpose:** Documentation tier system and governance
- **Usage:** Ensure documentation updates follow tier rules
- **Key Sections:**
  - 5-tier hierarchy (Governance → Requirements → Design → Implementation → Execution)
  - Upward flow prohibited, downward flow normal
- **Status:** AVAILABLE

**GOV002-requirements_enumeration.md**
- **Purpose:** Requirement ID scheme
- **Usage:** Reference for traceability comments
- **Status:** AVAILABLE

---

## External Tools and Systems

### Development Tools (REQUIRED)

**Rust Toolchain (stable)**
- **Purpose:** Compile, test, benchmark code
- **Commands:**
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace -- -W clippy::all -D warnings`
  - `cargo bench --package wkmp-ap`
  - `cargo doc --workspace --no-deps`
- **Status:** ASSUMED AVAILABLE

**Git**
- **Purpose:** Version control, rollback capability
- **Commands:**
  - `git status`
  - `git diff`
  - `git add`, `git commit`
  - `git restore` (rollback if needed)
- **Status:** ASSUMED AVAILABLE

**Grep / Ripgrep**
- **Purpose:** Search for code patterns, DEBT markers, references
- **Commands:**
  - `rg "DEBT-007"` (find DEBT markers)
  - `rg "Config::"` (find Config struct usage)
  - `rg "\[REQ-"` (find requirement references)
- **Status:** ASSUMED AVAILABLE

---

### Test Infrastructure (REQUIRED)

**Cargo Test Framework**
- **Purpose:** Run unit and integration tests
- **Test Locations:**
  - wkmp-ap/src/**/*_test.rs (unit tests)
  - wkmp-ap/tests/**/*.rs (integration tests)
  - wkmp-common/src/**/*_test.rs (unit tests)
  - wkmp-common/tests/**/*.rs (integration tests)
- **Status:** AVAILABLE (219 tests in wkmp-ap)

**Cargo Bench Framework**
- **Purpose:** Performance benchmarks
- **Benchmark Locations:**
  - wkmp-ap/benches/**/*.rs (8 benchmarks)
- **Status:** AVAILABLE

**Test Audio Files**
- **Purpose:** Integration test fixtures
- **Location:** wkmp-ap/test_assets/
- **Status:** ASSUMED AVAILABLE

**System Audio Devices**
- **Purpose:** Audio output tests (cpal)
- **Status:** ASSUMED AVAILABLE (may need mock for headless CI)

---

### Database (REQUIRED)

**SQLite Database**
- **Purpose:** Shared state, settings, queue persistence
- **Location:** Determined by root folder resolution
- **Schema:** migrations/*.sql
- **Tables Used:**
  - settings (volume, root_folder, tuning parameters)
  - queue (persistence across restarts)
  - passages, recordings, artists, releases (DEBT-003 album fetching)
- **Status:** ASSUMED INITIALIZED

---

## Inter-Requirement Dependencies

### Dependency Graph

```
NFR-001 (Test Coverage Preservation)
    │ [GATES ALL WORK - No modifications without passing tests]
    ↓
FR-001 (Refactor core.rs)
    │ [Requires: SPEC016, SPEC028, IMPL002]
    │ [Produces: New module files]
    ├→ FR-005 (Update module docs) [Depends on: FR-001 complete]
    ├→ FR-006 (Validate references) [Depends on: FR-001 code moves]
    └→ NFR-002 (Performance validation) [Depends on: FR-001 complete]
    ↓
FR-002 (Code Cleanup)
    │ [Requires: Grep for Config usage]
    │ [Produces: Removed deprecated code, consolidated diagnostics]
    └→ FR-006 (Update docs) [Depends on: FR-002 removal complete]
    ↓
FR-003 (DEBT Completion)
    │ [Requires: DEBT specifications (may be incomplete)]
    │ [Produces: Implemented DEBT-007, FUNC-002, FUNC-003]
    └→ FR-006 (Remove DEBT markers) [Depends on: FR-003 complete]
    ↓
FR-004 (Code Quality)
    │ [Independent - can run anytime after code changes]
    │ [Produces: Clippy clean, doctest passing]
    ↓
FR-005, FR-006 (Documentation)
    │ [Requires: ALL code changes complete]
    │ [Produces: IMPL008, buffer tuning guide, updated IMPL003, updated READMEs]
    └→ FINAL ACCEPTANCE
```

---

### Critical Path Analysis

**Critical Path:** NFR-001 → FR-001 → FR-002 → FR-003 → FR-004 → FR-005/FR-006

**Total Duration:** 14-21.75 days

**Bottlenecks:**
1. **FR-001 (core.rs refactor):** 2-3 days - Largest effort, blocks downstream
2. **FR-003 (DEBT completion):** 2-3 days - Underspecified, may require research
3. **FR-005 (Documentation):** 4-6.75 days - Largest effort, but parallelizable

**Parallelization Opportunities:**
- FR-005 (IMPL008 creation) and FR-006 (reference validation) can partially overlap
- FR-004 (code quality) can run concurrently with documentation work

---

### Blocking Dependencies

**FR-001 BLOCKED BY:**
- ❌ No known blockers (ready to start after baseline)

**FR-002 BLOCKED BY:**
- ✅ FR-001 complete (safer to cleanup after refactoring)

**FR-003 BLOCKED BY:**
- ⚠️ Specification completeness (Phase 2 must resolve)
- ✅ FR-002 complete (cleaner codebase)

**FR-004 BLOCKED BY:**
- ⚠️ FR-001 complete (core.rs refactor may resolve some warnings)

**FR-005 BLOCKED BY:**
- ✅ FR-001 complete (need refactored structure to document)
- ✅ ALL code changes complete (doc updates come last)

**FR-006 BLOCKED BY:**
- ✅ FR-001 complete (code moves require reference updates)
- ✅ FR-003 complete (DEBT markers removed after implementation)

---

## Open Questions Requiring Resolution

These dependencies are INCOMPLETE and require Phase 2 specification verification:

### Q1: core.rs Refactoring Strategy (FR-001)

**Question:** What is the optimal module split for core.rs (3,156 LOC)?

**Options:**
- A: By feature (queue, chain_mgmt, lifecycle, events)
- B: By layer (orchestration, coordination, state)
- C: By component (engine_core, engine_queue, engine_chains)

**Blocking:** FR-001 implementation plan
**Resolution:** Phase 2 - Analyze SPEC016, SPEC028 guidance

---

### Q2: DEBT-007 Specification (FR-003)

**Question:** What exactly should DEBT-007 source sample rate telemetry track?

**Unknowns:**
- What sample rates to track? (source file rate? resampling target rate? both?)
- Where stored? (database? in-memory state? events only?)
- When emitted? (on decode start? continuously? on change?)
- API access? (SSE? REST endpoint? diagnostics only?)

**Blocking:** FR-003 DEBT-007 implementation
**Resolution:** Phase 2 - Search codebase for DEBT-007 markers, read context

---

### Q3: FUNC-002 and FUNC-003 Specifications (FR-003)

**Question:** Where should album UUID fetching and duration_played calculation reside?

**Unknowns:**
- **FUNC-002 (duration_played):** Calculate at event emission or store as state?
- **FUNC-003 (album UUID):** Fetch in db layer or event layer?
- Performance implications? (database query per event?)

**Blocking:** FR-003 implementation design
**Resolution:** Phase 2 - Search codebase, analyze event flow

---

### Q4: Config Struct Usage (FR-002)

**Question:** Is Config struct actually used anywhere, or can it be safely removed?

**Requires:**
- Grep for `Config::` usage in wkmp-ap
- Check for external dependencies (wkmp-ui, wkmp-pd)
- Analyze main.rs bootstrap usage

**Blocking:** FR-002 Increment 6 (Config cleanup)
**Resolution:** Phase 2 - Run grep analysis

---

### Q5: Diagnostics Duplication (FR-002)

**Question:** Which diagnostics module is canonical? What references exist?

**Requires:**
- Analyze usage of playback/diagnostics.rs (514 LOC)
- Analyze usage of playback/engine/diagnostics.rs (860 LOC)
- Determine merge strategy

**Blocking:** FR-002 Increment 3 (diagnostics consolidation)
**Resolution:** Phase 2 - Grep for imports, analyze differences

---

### Q6: Buffer Tuning Guide Location (FR-005)

**Question:** Where should buffer tuning workflow documentation reside?

**Options:**
- A: IMPL009-buffer_tuning_guide.md (new document)
- B: Section in wkmp-ap/README.md (operator guide)
- C: docs/operators/ (new operators directory)

**Blocking:** FR-005 documentation structure
**Resolution:** Phase 2 - Evaluate existing documentation structure

---

### Q7: Test Baseline Status (NFR-001)

**Question:** Are there pre-existing test failures or flaky tests?

**Requires:**
- Run `cargo test --workspace` for baseline
- Document any pre-existing failures
- Identify flaky tests to exclude from regression detection

**Blocking:** Increment 1 (baseline establishment)
**Resolution:** Increment 1 execution (immediate)

---

### Q8: Benchmark Baseline (NFR-002)

**Question:** What is current benchmark baseline for performance comparison?

**Requires:**
- Run `cargo bench --package wkmp-ap`
- Document baseline results
- Establish ±5% tolerance ranges

**Blocking:** FR-001 performance validation
**Resolution:** Increment 1 execution (immediate)

---

## Dependency Risks

### High-Risk Dependencies

**Risk: DEBT Specifications Underspecified**
- **Impact:** Cannot implement FR-003 without clear specifications
- **Probability:** MEDIUM (DEBT markers often lack detail)
- **Mitigation:** Phase 2 completeness verification, user consultation
- **Residual Risk:** MEDIUM

**Risk: Config Struct Has Hidden Dependencies**
- **Impact:** Cannot remove Config without breaking compilation
- **Probability:** LOW (marked "Phase 4" suggests obsolete)
- **Mitigation:** Thorough grep analysis in Phase 2
- **Residual Risk:** LOW

**Risk: Pre-existing Test Failures Block Work**
- **Impact:** Cannot establish clean baseline, blocks all work
- **Probability:** LOW (project well-maintained)
- **Mitigation:** Triage protocol, defer pre-existing issues
- **Residual Risk:** LOW

---

### Medium-Risk Dependencies

**Risk: core.rs Refactoring More Complex Than Expected**
- **Impact:** Schedule slip, Increment 2 extends beyond 3 days
- **Probability:** MEDIUM (3,156 LOC is substantial)
- **Mitigation:** Incremental approach allows early stopping
- **Residual Risk:** MEDIUM

**Risk: wkmp-common Changes Break Downstream Modules**
- **Impact:** Test failures in wkmp-ui, wkmp-pd
- **Probability:** LOW (scope prohibits API changes)
- **Mitigation:** Full workspace testing after each change
- **Residual Risk:** LOW

---

## Next Steps

**Phase 1 Status:** Dependencies mapped and analyzed.

**Next Phase:** Phase 2 - Specification Completeness Verification
- **Resolve Open Questions Q1-Q8**
- **Search codebase for DEBT marker context**
- **Analyze Config struct usage**
- **Evaluate diagnostics duplication**
- **Answer all specification gaps**

**Approval Required:** User should review dependencies map and confirm understanding before Phase 2.

---

*End of Dependencies Map*
