# Requirements Index: Technical Debt Reduction

**Source:** SPEC_technical_debt_reduction.md
**Generated:** 2025-11-10
**Total Requirements:** 26 (22 phase-specific + 4 cross-phase)

---

## Phase 1: Quick Wins (Critical Fixes)

### REQ-TD1-001: Replace Blocking Sleep in Async Context
- **Priority:** CRITICAL
- **Location:** `wkmp-common/src/time.rs:37`
- **Description:** Replace `std::thread::sleep` with `tokio::time::sleep` in async test context
- **Impact:** Prevents blocking of async runtime, improves concurrency

### REQ-TD1-002: Delete Dead Code
- **Priority:** CRITICAL
- **Location:** `wkmp-ai/src/workflow/song_processor.rs` (368 lines)
- **Description:** Remove unused song_processor.rs file and commented-out module declaration
- **Impact:** -368 lines, eliminates confusion

### REQ-TD1-003: Fix Compiler Warnings
- **Priority:** HIGH
- **Metrics:** 11 compiler warnings (4 unused imports, 2 unused variables, 5 dead code)
- **Description:** Eliminate all compiler warnings in wkmp-ai and wkmp-common
- **Impact:** Clean build output, improved code quality

### REQ-TD1-004: Fix Clippy Lints
- **Priority:** HIGH
- **Metrics:** 22 clippy warnings
- **Description:** Address all clippy warnings using auto-fix where possible
- **Impact:** Improved Rust idioms, better code quality

### REQ-TD1-005: Fix Panic Statements
- **Priority:** HIGH
- **Locations:** 2 real panics (events.rs:1564, extractors/mod.rs:77)
- **Description:** Replace panic! macros in production code with proper error handling
- **Impact:** Graceful error handling, no unexpected crashes

---

## Phase 2: File Organization

### REQ-TD2-001: Refactor Workflow Orchestrator
- **Priority:** CRITICAL
- **Current State:** 2,253 lines in single file (6.6% of codebase)
- **Target State:** 7-8 module files, largest <650 lines
- **Description:** Decompose workflow_orchestrator.rs into modular structure by phase
- **Impact:** 71% size reduction, improved maintainability

### REQ-TD2-002: Split events.rs
- **Priority:** HIGH
- **Current State:** 1,711 lines
- **Target State:** 3-4 modules, largest <600 lines
- **Description:** Decompose events.rs into category-specific modules
- **Impact:** Better organization, reduced merge conflicts

### REQ-TD2-003: Split params.rs
- **Priority:** HIGH
- **Current State:** 1,450 lines
- **Target State:** 4-5 modules, largest <400 lines
- **Description:** Decompose params.rs into parameter group modules
- **Impact:** Functional grouping, easier navigation

### REQ-TD2-004: Reorganize api/ui.rs
- **Priority:** MEDIUM
- **Current State:** 1,308 lines
- **Target State:** 5-6 modules, largest <300 lines
- **Description:** Decompose ui.rs into page-specific modules
- **Impact:** Page-level modularity, reusable components

---

## Phase 3: Error Handling Audit

### REQ-TD3-001: Audit unwrap() Usage
- **Priority:** HIGH
- **Current State:** 506 unwrap()/expect() calls (excluding tests)
- **Target State:** <50 justified unwrap() calls
- **Description:** Audit and classify all unwrap() calls as justifiable/convertible/removable
- **Impact:** Risk assessment, conversion roadmap

### REQ-TD3-002: Convert User-Facing unwrap()
- **Priority:** HIGH
- **Target:** Convert 200+ high-priority unwrap() calls
- **Description:** Convert unwrap() in user-facing paths to proper error handling
- **Impact:** Graceful error handling, better user experience

### REQ-TD3-003: Add Error Context
- **Priority:** MEDIUM
- **Description:** Add descriptive error context using `anyhow::Context`
- **Impact:** Actionable error messages, easier debugging

---

## Phase 4: Documentation

### REQ-TD4-001: Enable missing_docs Lint
- **Priority:** HIGH
- **Description:** Enable `#![warn(missing_docs)]` in lib.rs
- **Impact:** Enforcement mechanism for documentation completeness

### REQ-TD4-002: Document Public Modules
- **Priority:** HIGH
- **Target:** 100% of public modules
- **Description:** Add module-level documentation to all public modules
- **Impact:** Complete module documentation

### REQ-TD4-003: Document Public Functions
- **Priority:** MEDIUM
- **Target:** 100% of public functions (294 total)
- **Current:** ~34-51% undocumented
- **Description:** Add documentation comments to all public functions
- **Impact:** Complete API documentation

---

## Phase 5: Code Quality Improvements

### REQ-TD5-001: Extract Rate Limiter Utility
- **Priority:** MEDIUM
- **Current:** 4 duplicate implementations
- **Description:** Extract rate limiting logic to shared utility in wkmp-common
- **Impact:** DRY principle, single source of truth

### REQ-TD5-002: Break Up Long Functions
- **Priority:** MEDIUM
- **Target:** All functions <200 lines
- **Current:** 450+ functions >150 lines
- **Description:** Refactor functions >200 lines using extract-method pattern
- **Impact:** Improved readability, single responsibility

### REQ-TD5-003: Consolidate Configuration Structs
- **Priority:** MEDIUM
- **Description:** Consolidate duplicate configuration structs into unified WorkflowConfig
- **Impact:** Single source of truth for configuration

### REQ-TD5-004: Remove Magic Numbers
- **Priority:** LOW
- **Description:** Replace magic numbers with named constants
- **Impact:** Improved code readability and maintainability

---

## Cross-Phase Requirements

### REQ-TD-ALL-001: Test Preservation
- **Priority:** CRITICAL
- **Description:** All existing tests SHALL continue to pass throughout all phases
- **Metric:** 216 tests pass after each increment
- **Impact:** Ensures no regressions

### REQ-TD-ALL-002: Backward Compatibility
- **Priority:** CRITICAL
- **Description:** Public APIs SHALL NOT change in breaking ways
- **Impact:** Semantic versioning, no breaking changes

### REQ-TD-ALL-003: Incremental Delivery
- **Priority:** HIGH
- **Description:** Each phase SHALL be independently deliverable and verifiable
- **Impact:** Continuous integration, no broken states

### REQ-TD-ALL-004: Documentation Updates
- **Priority:** MEDIUM
- **Description:** All refactoring SHALL include corresponding documentation updates
- **Impact:** Documentation stays in sync with code

---

## Requirements Summary

| Phase | Requirements | Priority Breakdown |
|-------|--------------|-------------------|
| Phase 1 | 5 | 2 Critical, 3 High |
| Phase 2 | 4 | 1 Critical, 2 High, 1 Medium |
| Phase 3 | 3 | 2 High, 1 Medium |
| Phase 4 | 3 | 2 High, 1 Medium |
| Phase 5 | 4 | 3 Medium, 1 Low |
| Cross-Phase | 4 | 2 Critical, 1 High, 1 Medium |
| **Total** | **23** | **5 Critical, 8 High, 8 Medium, 1 Low** |

---

## Requirements Traceability

All requirements extracted from SPEC_technical_debt_reduction.md sections:
- Section 3: Phase 1 requirements (lines 93-262)
- Section 4: Phase 2 requirements (lines 264-407)
- Section 5: Phase 3 requirements (lines 409-525)
- Section 6: Phase 4 requirements (lines 527-642)
- Section 7: Phase 5 requirements (lines 644-838)
- Section 8: Cross-phase requirements (lines 840-934)

Each requirement SHALL have corresponding acceptance tests defined in Phase 3 of /plan workflow.
