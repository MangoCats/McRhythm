# Technical Debt Remediation - Acceptance Test Specifications

**Source Specification:** SPEC_technical_debt_remediation.md
**Generated:** 2025-11-05
**Plan Workflow Phase:** Phase 3 - Acceptance Test Definition

---

## Overview

This document defines acceptance tests for all 10 requirements (6 functional + 4 non-functional) using Given/When/Then format. Each test is traceable to specific requirements and success criteria.

**Test Execution Strategy:** Incremental validation after each code change

**Test Framework:** Rust cargo test (unit + integration), cargo clippy, cargo bench

---

## Functional Requirements - Acceptance Tests

### FR-001: Code Organization

**Requirement:** Refactor core.rs (3,156 LOC) into modules <1,000 LOC each with clear boundaries

---

**AT-FR-001-001: Core.rs File Split**

**Given:**
- playback/engine/core.rs exists at 3,156 LOC
- Existing modules: queue.rs (716 LOC), diagnostics.rs (1,023 LOC)

**When:**
- core.rs is refactored into 4-5 component-based modules

**Then:**
- ✅ Each resulting file is <1,000 LOC
- ✅ Files created:
  - core.rs (retained, ~800 LOC) - PlaybackEngine struct, lifecycle
  - playback.rs (~600-800 LOC) - Playback control methods
  - chains.rs (~600-800 LOC) - Buffer chain management
  - events.rs (~400-600 LOC) - Event emission logic
  - lifecycle.rs (optional, ~200-400 LOC) - Initialization/shutdown
- ✅ Total LOC across all files ≈ 2,600-3,400 LOC (accounting for overhead)

**Validation:**
```bash
wc -l wkmp-ap/src/playback/engine/*.rs
# Verify each file <1,000 LOC
```

---

**AT-FR-001-002: Module Boundaries Clear**

**Given:**
- core.rs has been split into multiple modules

**When:**
- Analyzing module dependencies and imports

**Then:**
- ✅ No circular dependencies between modules
- ✅ Each module has single, clear responsibility
- ✅ Internal module communication uses well-defined interfaces
- ✅ No excessive cross-module coupling (avoid "god object" anti-pattern)

**Validation:**
```bash
# Check for circular dependencies
cargo build --package wkmp-ap 2>&1 | grep -i "cyclic"

# Review module structure
grep "^use crate::playback::engine::" wkmp-ap/src/playback/engine/*.rs
```

---

**AT-FR-001-003: Public API Preserved**

**Given:**
- PlaybackEngine had public API before refactoring

**When:**
- Refactoring is complete

**Then:**
- ✅ All public methods still accessible
- ✅ Method signatures unchanged (no breaking changes)
- ✅ Public types still exported from mod.rs
- ✅ External callers (tests, other modules) compile without changes

**Validation:**
```bash
# Ensure workspace compiles
cargo build --workspace

# Ensure all tests pass (no API breakage)
cargo test --workspace
```

---

**AT-FR-001-004: Tests Still Pass**

**Given:**
- Baseline test suite established (Increment 1)
- core.rs refactored

**When:**
- Running full test suite

**Then:**
- ✅ All unit tests pass
- ✅ All integration tests pass
- ✅ No new test failures introduced
- ✅ Test count unchanged (no tests accidentally removed)

**Validation:**
```bash
cargo test --package wkmp-ap --lib
cargo test --package wkmp-ap --test '*'

# Compare to baseline
diff baseline_tests.log current_tests.log
```

---

### FR-002: Code Cleanup

**Requirement:** Remove deprecated code, consolidate duplicates, remove obsolete files

---

**AT-FR-002-001: Deprecated Auth Middleware Removed**

**Given:**
- api/auth_middleware.rs contains 530+ LOC deprecated code (lines 250-800)

**When:**
- Deprecated section removed

**Then:**
- ✅ Lines 250-800 (deprecated V1 auth) deleted
- ✅ File size reduced by ~530 LOC
- ✅ Only V2 auth code remains
- ✅ No references to removed code remain in codebase

**Validation:**
```bash
# Check file size
wc -l wkmp-ap/src/api/auth_middleware.rs
# Should be ~395 LOC (925 - 530)

# Check for deprecated markers
grep -i "deprecated" wkmp-ap/src/api/auth_middleware.rs
# Should find zero occurrences in code (only in comments if needed)

# Check for V1 references
grep -r "V1.*auth" wkmp-ap/src/
# Should find zero occurrences
```

---

**AT-FR-002-002: Diagnostics Properly Separated (CORRECTED)**

**Given:**
- playback/diagnostics.rs (512 LOC) - Type definitions
- playback/engine/diagnostics.rs (859 LOC) - Implementation methods
- Phase 2 analysis confirms NO duplication

**When:**
- Reviewing diagnostics modules

**Then:**
- ✅ Both files retained (proper separation confirmed)
- ✅ playback/diagnostics.rs exports types (PassageMetrics, PipelineMetrics, ValidationResult, ValidationError)
- ✅ playback/engine/diagnostics.rs contains impl PlaybackEngine methods
- ✅ No consolidation needed (TD-M-001 removed from scope)

**Validation:**
```bash
# Verify both files exist
ls -la wkmp-ap/src/playback/diagnostics.rs
ls -la wkmp-ap/src/playback/engine/diagnostics.rs

# Verify proper exports
grep "^pub (struct|enum)" wkmp-ap/src/playback/diagnostics.rs
grep "^impl PlaybackEngine" wkmp-ap/src/playback/engine/diagnostics.rs
```

---

**AT-FR-002-003: Obsolete Files Removed**

**Given:**
- Legacy decoder files may exist (decoder_pool, serial_decoder)

**When:**
- Searching for obsolete modules

**Then:**
- ✅ No decoder_pool.rs in src/ tree
- ✅ No serial_decoder.rs in src/ tree
- ✅ Comments in mod.rs note removals ("Removed: decoder_pool")
- ✅ No imports referencing removed modules

**Validation:**
```bash
# Search for obsolete files
find wkmp-ap/src -name "decoder_pool.rs" -o -name "serial_decoder.rs"
# Should return zero results

# Check mod.rs documents removals
grep "Removed:" wkmp-ap/src/playback/mod.rs

# Check for stale imports
grep "decoder_pool\|serial_decoder" wkmp-ap/src/**/*.rs
# Should only find in comments
```

---

**AT-FR-002-004: Config Struct Removed**

**Given:**
- Config struct used only for single port parameter
- Phase 2 confirms safe to remove

**When:**
- Config struct removed and replaced with u16 parameter

**Then:**
- ✅ api/server.rs uses `port: u16` parameter instead of `config: Config`
- ✅ main.rs passes port directly, no Config struct construction
- ✅ config.rs file deleted OR marked #[deprecated]
- ✅ No imports of `crate::config::Config` remain (except in tests if deprecated)

**Validation:**
```bash
# Check server.rs signature
grep "pub async fn run" wkmp-ap/src/api/server.rs
# Should show: run(port: u16, ...)

# Check main.rs
grep "Config {" wkmp-ap/src/main.rs
# Should return zero results

# Check if config.rs exists
ls wkmp-ap/src/config.rs
# Should be deleted OR contain #[deprecated] if kept

# Verify compilation
cargo build --package wkmp-ap
```

---

### FR-003: Feature Completion

**Requirement:** Implement DEBT-007, REQ-DEBT-FUNC-002, REQ-DEBT-FUNC-003, DEBT-002 (optional)

---

**AT-FR-003-001: DEBT-007 Source Sample Rate Implemented**

**Given:**
- BufferChainMetadata.source_sample_rate field exists but not set
- decoder_worker.rs has marker comment at line ~409

**When:**
- Decoder reads source sample rate from audio file metadata
- Sets BufferChainMetadata.source_sample_rate = Some(rate)

**Then:**
- ✅ decoder_worker.rs extracts rate from symphonia Track metadata
- ✅ BufferChainMetadata.source_sample_rate contains actual value (not None)
- ✅ Source rate propagates to buffer_manager events
- ✅ Diagnostics endpoint shows source rates for active chains
- ✅ Value matches actual audio file sample rate (44100, 48000, 96000, etc.)

**Validation:**
```bash
# Search for implementation
grep -A 5 "source_sample_rate = Some" wkmp-ap/src/playback/decoder_worker.rs

# Run integration test with known sample rate file
cargo test test_source_sample_rate_telemetry

# Check diagnostics output
curl http://localhost:5721/playback/buffer_chains | jq '.chains[].source_sample_rate'
# Should show actual rates, not null
```

**Test Case:**
```rust
#[tokio::test]
async fn test_debt_007_source_sample_rate() {
    // Given: Audio file with known 48000 Hz sample rate
    let passage_id = enqueue_test_passage("test_48khz.flac").await;

    // When: File is decoded
    wait_for_decode_complete(passage_id).await;

    // Then: Source rate is tracked
    let metadata = get_buffer_metadata(passage_id).await;
    assert_eq!(metadata.source_sample_rate, Some(48000));
}
```

---

**AT-FR-003-002: FUNC-002 Duration Played Implemented**

**Given:**
- PassageCompleted events lack duration_played field
- Frame counters tracked in PlaybackPosition

**When:**
- PassageCompleted event emitted
- Duration calculated: (end_frame - start_frame) / sample_rate * 1000.0

**Then:**
- ✅ PassageCompleted event includes `duration_played_ms: u64` field
- ✅ Calculation uses actual playback frames (not passage total duration)
- ✅ Duration accounts for seek operations (if implemented)
- ✅ Value in milliseconds matches actual playback time

**Validation:**
```bash
# Check event definition in wkmp-common
grep "duration_played_ms" wkmp-common/src/events.rs

# Run integration test
cargo test test_duration_played_calculation
```

**Test Case:**
```rust
#[tokio::test]
async fn test_func_002_duration_played() {
    // Given: Passage of known duration
    let passage_id = enqueue_test_passage("5_second_clip.flac").await;

    // When: Passage plays completely
    let events = collect_events_until_completed(passage_id).await;

    // Then: Duration matches expected playback time
    let completed = events.iter()
        .find(|e| matches!(e, PlaybackEvent::PassageCompleted { .. }))
        .unwrap();

    if let PlaybackEvent::PassageCompleted { duration_played_ms, .. } = completed {
        assert!(duration_played_ms >= 4900 && duration_played_ms <= 5100); // ±100ms tolerance
    }
}
```

---

**AT-FR-003-003: FUNC-003 Album Metadata Implemented**

**Given:**
- Function `db::passages::get_passage_album_uuids` exists but unused
- PassageStarted/Completed events lack album_uuids field

**When:**
- PassageStarted event emitted
- PassageCompleted event emitted
- Album UUIDs fetched from database: passages → recordings → releases

**Then:**
- ✅ Both events include `album_uuids: Vec<Uuid>` field
- ✅ UUIDs fetched via `get_passage_album_uuids(&db_pool, passage_id).await`
- ✅ Multiple albums supported (Vec, not single UUID)
- ✅ Empty vec if no album metadata available (graceful fallback)

**Validation:**
```bash
# Check event definitions
grep "album_uuids" wkmp-common/src/events.rs

# Check function is called
grep "get_passage_album_uuids" wkmp-ap/src/playback/engine/*.rs

# Run integration test
cargo test test_album_metadata_in_events
```

**Test Case:**
```rust
#[tokio::test]
async fn test_func_003_album_uuids() {
    // Given: Passage linked to known album
    let passage_id = create_test_passage_with_album().await;

    // When: Passage starts playing
    let events = collect_events_until_started(passage_id).await;

    // Then: Album UUIDs present in event
    let started = events.iter()
        .find(|e| matches!(e, PlaybackEvent::PassageStarted { .. }))
        .unwrap();

    if let PlaybackEvent::PassageStarted { album_uuids, .. } = started {
        assert!(!album_uuids.is_empty());
        // Verify UUID matches expected album
    }
}
```

---

**AT-FR-003-004: DEBT-002 Audio Clipping Detection (OPTIONAL)**

**Given:**
- DEBT-002 marked optional LOW priority
- May be deferred to future work

**When:**
- IF implemented: Mixer detects samples exceeding ±1.0 range

**Then:**
- ✅ Clipping events emitted OR logged
- ✅ Threshold configurable (default ±1.0)
- ✅ Clipping count tracked per passage
- ❌ IF NOT implemented: Skip this test, document as deferred

**Validation:**
```bash
# Check if implemented
grep -r "clipping" wkmp-ap/src/playback/mixer.rs

# If implemented, run test
cargo test test_clipping_detection || echo "Deferred to future work"
```

---

### FR-004: Code Quality

**Requirement:** Address clippy warnings, fix doctests, resolve dead code warnings

---

**AT-FR-004-001: Clippy Warnings Resolved**

**Given:**
- Baseline: 19 clippy warnings in wkmp-ap, 5 in wkmp-common

**When:**
- Running cargo clippy with strict flags

**Then:**
- ✅ Zero clippy warnings in wkmp-ap
- ✅ Zero clippy warnings in wkmp-common
- ✅ All warnings either fixed or explicitly approved
- ✅ No new warnings introduced by refactoring

**Validation:**
```bash
cargo clippy --package wkmp-ap -- -W clippy::all -D warnings
cargo clippy --package wkmp-common -- -W clippy::all -D warnings

# Should exit with code 0 (no warnings)
echo $?  # Should be 0
```

---

**AT-FR-004-002: Doctest Failures Fixed**

**Given:**
- Baseline: 1 doctest failure in api/handlers.rs

**When:**
- Running cargo test --doc

**Then:**
- ✅ Zero doctest failures
- ✅ All code examples in doc comments compile and run
- ✅ Doctests updated to match current API

**Validation:**
```bash
cargo test --doc --package wkmp-ap

# Check specific file
cargo test --doc --package wkmp-ap -- handlers
```

---

**AT-FR-004-003: Dead Code Warnings Resolved**

**Given:**
- Baseline: 76 dead_code warnings after pragma removal

**When:**
- Reviewing dead code warnings

**Then:**
- ✅ Dead code either:
  - Used in tests (add #[cfg(test)] visibility)
  - Actually unused and removed
  - Reserved for "Phase 4" (marked with comment + #[allow(dead_code)] with rationale)
- ✅ All #[allow(dead_code)] pragmas have accompanying comment explaining why

**Validation:**
```bash
cargo build --package wkmp-ap 2>&1 | grep "warning: .*dead_code"

# Check any remaining allows have rationale
grep -B 1 "#\[allow(dead_code)\]" wkmp-ap/src/**/*.rs
# Each should have comment above explaining
```

---

### FR-005: Documentation Completeness

**Requirement:** Create IMPL008, document buffer tuning, update IMPL003, update module docs, verify API docs

---

**AT-FR-005-001: IMPL008 Created**

**Given:**
- DecoderWorker implementation lacks dedicated documentation

**When:**
- IMPL008-decoder_worker_implementation.md created

**Then:**
- ✅ Document exists at `docs/IMPL008-decoder_worker_implementation.md`
- ✅ Length: 500-1000 lines
- ✅ Contains all required sections:
  - Architecture Overview
  - Component Structure
  - Key Algorithms
  - Performance Characteristics
  - Integration Points
  - Testing Approach
  - Traceability
- ✅ Registered in workflows/REG001_number_registry.md
- ✅ Linked from IMPL003 (decoder_worker.rs description)

**Validation:**
```bash
# Verify file exists
ls -la docs/IMPL008-decoder_worker_implementation.md

# Check length
wc -l docs/IMPL008-decoder_worker_implementation.md
# Should be 500-1000 lines

# Verify required sections
grep "^## " docs/IMPL008-decoder_worker_implementation.md | wc -l
# Should have 7+ major sections

# Check registration
grep "IMPL008" workflows/REG001_number_registry.md
```

---

**AT-FR-005-002: Buffer Tuning Workflow Documented**

**Given:**
- tune-buffers binary lacks operator documentation

**When:**
- Buffer tuning guide created (IMPL009 recommended)

**Then:**
- ✅ Document exists at `docs/IMPL009-buffer_tuning_guide.md`
- ✅ Length: 300-500 lines
- ✅ Contains all required sections:
  - When to Tune
  - Tuning Process
  - Parameter Recommendations
  - Applying Results
  - Troubleshooting
  - Examples
- ✅ Non-technical operator can follow guide
- ✅ Linked from wkmp-ap/README.md

**Validation:**
```bash
# Verify file
ls -la docs/IMPL009-buffer_tuning_guide.md

# Check length
wc -l docs/IMPL009-buffer_tuning_guide.md
# Should be 300-500 lines

# Check README link
grep "IMPL009\|buffer.*tuning" wkmp-ap/README.md
```

---

**AT-FR-005-003: IMPL003 Updated**

**Given:**
- IMPL003-project_structure.md outdated (references decoder_pool, etc.)

**When:**
- IMPL003 updated to reflect current structure

**Then:**
- ✅ wkmp-ap module structure section updated
- ✅ Shows current src/ organization (engine/, pipeline/, tuning/, bin/)
- ✅ Obsolete module references removed (decoder_pool, serial_decoder)
- ✅ Key files list updated (reflects core.rs refactoring if completed)
- ✅ "Last Updated" date refreshed to 2025-11-05 or later

**Validation:**
```bash
# Check for obsolete references
grep -i "decoder_pool\|serial_decoder" docs/IMPL003-project_structure.md
# Should only appear in "historical" context if at all

# Check for current modules
grep "tuning/\|decoder_worker" docs/IMPL003-project_structure.md
# Should be present

# Check last updated
grep "Last Updated" docs/IMPL003-project_structure.md
```

---

**AT-FR-005-004: Module-Level Docs Updated**

**Given:**
- core.rs refactored into multiple modules

**When:**
- Each new module file reviewed for documentation

**Then:**
- ✅ Each module has comprehensive `//!` doc comment at top
- ✅ Module docs explain:
  - Module's role in system
  - Public API contracts
  - Links to relevant SPEC documents
  - Usage examples for complex APIs (if applicable)
- ✅ No "TODO" or placeholder comments in module docs

**Validation:**
```bash
# Check all engine modules have module docs
for file in wkmp-ap/src/playback/engine/*.rs; do
    head -20 "$file" | grep "^//!" || echo "Missing: $file"
done

# Check for TODOs
grep "TODO" wkmp-ap/src/playback/engine/*.rs
# Should be zero in module-level docs
```

---

**AT-FR-005-005: Public API Documentation Complete**

**Given:**
- Public API may have missing or incomplete doc comments

**When:**
- Running cargo doc and reviewing output

**Then:**
- ✅ All public functions have doc comments
- ✅ All public structs have doc comments
- ✅ All public enums have doc comments
- ✅ No broken doc links
- ✅ cargo doc generates without warnings

**Validation:**
```bash
cargo doc --package wkmp-ap --package wkmp-common --no-deps 2>&1 | tee doc_output.log

# Check for warnings
grep "warning:" doc_output.log
# Should be zero

# Check for broken links
grep "unresolved link" doc_output.log
# Should be zero
```

---

### FR-006: Documentation Accuracy

**Requirement:** Validate code-to-doc references, remove obsolete docs, update READMEs

---

**AT-FR-006-001: Code-to-Doc References Valid**

**Given:**
- Code contains traceability comments ([REQ-*], [SPEC-*], [DEBT-*])

**When:**
- Grepping for traceability markers

**Then:**
- ✅ All [REQ-*] references point to valid requirements
- ✅ All [SPEC-*] references point to valid specifications
- ✅ All [DEBT-*] markers for completed features removed
- ✅ File:line references in docs updated if code moved

**Validation:**
```bash
# Extract all requirement references
grep -r "\[REQ-" wkmp-ap/src/ | cut -d: -f2 | sort -u > req_refs.txt

# Verify each exists in REQ001 or other specs
while read ref; do
    grep -q "$ref" docs/REQ*.md || echo "Invalid: $ref"
done < req_refs.txt

# Check for resolved DEBT markers still in code
grep "\[DEBT-007\]\|FUNC-002\|FUNC-003" wkmp-ap/src/
# Should not appear in comments after implementation
```

---

**AT-FR-006-002: README Files Updated**

**Given:**
- wkmp-ap/README.md may be outdated or missing

**When:**
- README reviewed

**Then:**
- ✅ wkmp-ap/README.md exists
- ✅ Contains:
  - Module overview
  - Architecture summary (single-stream design)
  - Key components (engine, mixer, decoder worker)
  - Build/test instructions
  - Link to docs/ specifications
- ✅ References to removed modules deleted
- ✅ References to new modules added (tuning, buffer guide)

**Validation:**
```bash
# Verify file exists
ls -la wkmp-ap/README.md

# Check for required sections
grep -i "architecture\|build\|test" wkmp-ap/README.md

# Check for obsolete references
grep -i "decoder_pool" wkmp-ap/README.md
# Should be zero
```

---

**AT-FR-006-003: CHANGELOG Entry Created**

**Given:**
- Technical debt remediation work completed

**When:**
- CHANGELOG entry created

**Then:**
- ✅ Entry in CHANGELOG.md or project_management/change_history.md
- ✅ Summarizes:
  - Technical debt items addressed
  - Major code changes (core.rs refactoring)
  - Features completed (DEBT markers)
  - Documentation added/updated
  - Breaking changes: NONE
- ✅ Dated 2025-11-05 or later

**Validation:**
```bash
# Check for entry
grep -i "technical debt\|remediation" project_management/change_history.md

# Or check CHANGELOG
grep -i "technical debt" CHANGELOG.md

# Verify date
grep "2025-11-0[5-9]" project_management/change_history.md
```

---

## Non-Functional Requirements - Acceptance Tests

### NFR-001: Test Coverage Preservation

**Requirement:** All existing tests pass, no coverage reduction, 90%+ for new code

---

**AT-NFR-001-001: Baseline Tests Pass**

**Given:**
- Baseline established in Increment 1
- Documented in wip/baseline_validation_results.md

**When:**
- Running full test suite after ANY code change

**Then:**
- ✅ All baseline tests still pass
- ✅ Test count unchanged (no tests lost)
- ✅ Test duration within ±20% of baseline
- ✅ No new test failures introduced

**Validation:**
```bash
cargo test --workspace --no-fail-fast 2>&1 | tee current_tests.log

# Compare to baseline
diff baseline_tests.log current_tests.log

# Count tests
grep "test result:" current_tests.log
# Should match baseline count
```

---

**AT-NFR-001-002: New Tests for New Features**

**Given:**
- DEBT-007, FUNC-002, FUNC-003 implemented

**When:**
- Reviewing test coverage for new code

**Then:**
- ✅ Unit tests for source_sample_rate extraction
- ✅ Integration test for duration_played calculation
- ✅ Integration test for album_uuids in events
- ✅ New tests achieve 90%+ line coverage

**Validation:**
```bash
# Run tests with coverage (requires cargo-tarpaulin or cargo-llvm-cov)
cargo tarpaulin --package wkmp-ap --out Html

# Review coverage report
open tarpaulin-report.html

# Check specific modules
grep "decoder_worker\|engine/events" tarpaulin-report.html
# Should show 90%+ coverage
```

---

**AT-NFR-001-003: No Coverage Regression**

**Given:**
- Baseline coverage documented

**When:**
- Comparing current coverage to baseline

**Then:**
- ✅ Overall coverage maintained or improved
- ✅ No modules with significant coverage drop (>5%)
- ✅ Refactored code maintains coverage

**Validation:**
```bash
# Compare coverage reports
diff baseline_coverage.txt current_coverage.txt

# Check for regressions
# Manual review of coverage delta
```

---

### NFR-002: Performance Maintenance

**Requirement:** No regressions in critical paths, benchmarks within ±5%

---

**AT-NFR-002-001: Benchmark Baseline Preserved**

**Given:**
- Baseline benchmarks documented in Increment 1

**When:**
- Running benchmarks after core.rs refactoring

**Then:**
- ✅ All benchmarks within ±5% of baseline
- ✅ No single benchmark >10% slower
- ✅ Audio callback latency unchanged
- ✅ Critical path performance maintained

**Validation:**
```bash
cargo bench --package wkmp-ap 2>&1 | tee current_benchmarks.log

# Extract timing data
grep "time:" current_benchmarks.log > current_times.txt

# Compare to baseline
diff baseline_times.txt current_times.txt

# Calculate percentage changes
python3 scripts/compare_benchmarks.py baseline_times.txt current_times.txt
# All should be within ±5%
```

---

**AT-NFR-002-002: No Memory Regressions**

**Given:**
- Memory usage baseline established

**When:**
- Monitoring memory usage during playback

**Then:**
- ✅ Heap allocation unchanged
- ✅ No memory leaks introduced
- ✅ Buffer sizes unchanged (unless intentionally tuned)
- ✅ Peak memory usage within ±10% of baseline

**Validation:**
```bash
# Run with memory profiler (valgrind or heaptrack)
valgrind --tool=massif target/debug/wkmp-ap

# Compare memory profiles
ms_print massif.out.baseline > baseline_mem.txt
ms_print massif.out.current > current_mem.txt
diff baseline_mem.txt current_mem.txt
```

---

### NFR-003: API Compatibility

**Requirement:** No breaking changes to wkmp-common, use #[deprecated], follow semver

---

**AT-NFR-003-001: wkmp-common API Unchanged**

**Given:**
- wkmp-common public API documented

**When:**
- Reviewing wkmp-common changes

**Then:**
- ✅ No function signatures changed (breaking)
- ✅ No struct fields removed (breaking)
- ✅ No public types removed (breaking)
- ✅ All downstream modules compile without changes

**Validation:**
```bash
# Compile all downstream modules
cargo build --package wkmp-ui
cargo build --package wkmp-pd

# Should succeed without errors
echo $?  # Should be 0
```

---

**AT-NFR-003-002: Deprecation Properly Marked**

**Given:**
- If any items deprecated during remediation

**When:**
- Reviewing deprecated items

**Then:**
- ✅ All deprecated items use #[deprecated(since = "version", note = "guidance")]
- ✅ Note provides migration path
- ✅ Deprecated items still compile and work
- ✅ Tests for deprecated items still pass

**Validation:**
```bash
# Find all deprecated items
grep -r "#\[deprecated" wkmp-common/src/ wkmp-ap/src/

# Verify format
grep -A 1 "#\[deprecated" wkmp-common/src/ | grep "since\|note"
# All should have both attributes
```

---

**AT-NFR-003-003: Semantic Versioning Followed**

**Given:**
- wkmp-common and wkmp-ap have version numbers

**When:**
- Remediation complete

**Then:**
- ✅ Version bump follows semver:
  - MAJOR: Breaking changes (should be NONE)
  - MINOR: New features (DEBT implementations = YES)
  - PATCH: Bug fixes
- ✅ Cargo.toml version updated appropriately
- ✅ CHANGELOG notes version bump rationale

**Validation:**
```bash
# Check versions
grep "^version" wkmp-common/Cargo.toml
grep "^version" wkmp-ap/Cargo.toml

# Should be MINOR bump (e.g., 0.1.0 → 0.2.0)
# NOT major bump (no breaking changes)
```

---

### NFR-004: Documentation Quality

**Requirement:** All public APIs documented, module docs updated, traceability preserved

---

**AT-NFR-004-001: Public API Documentation Complete**

**Given:**
- Public API requires documentation

**When:**
- Running cargo doc

**Then:**
- ✅ All `pub fn` have doc comments
- ✅ All `pub struct` have doc comments
- ✅ All `pub enum` have doc comments
- ✅ Doc comments are substantive (not just "TODO" or function name)
- ✅ Examples provided for complex APIs

**Validation:**
```bash
cargo doc --package wkmp-ap --package wkmp-common --no-deps

# Check for missing docs warning
cargo rustdoc --package wkmp-ap -- -D missing_docs 2>&1 | grep "warning:"
# Should be zero warnings
```

---

**AT-NFR-004-002: Module Documentation Updated**

**Given:**
- Modules refactored or added

**When:**
- Reviewing module-level docs

**Then:**
- ✅ All modules have `//!` doc comments
- ✅ Module docs explain purpose and responsibilities
- ✅ Links to SPEC documents included where relevant
- ✅ No outdated information in module docs

**Validation:**
```bash
# Check all modules have docs
for file in wkmp-ap/src/**/*.rs; do
    head -5 "$file" | grep "^//!" || echo "Missing: $file"
done
```

---

**AT-NFR-004-003: Traceability Preserved**

**Given:**
- Code has traceability comments

**When:**
- Reviewing traceability after refactoring

**Then:**
- ✅ All traceability comments moved with code
- ✅ New code has appropriate traceability comments
- ✅ No orphaned traceability references (file moved but comment references old location)
- ✅ DEBT markers removed after implementation (but traceability to requirement remains)

**Validation:**
```bash
# Check for orphaned references
grep -r "\[REQ-\|\[SPEC-\|\[DEBT-" wkmp-ap/src/ | \
    while read line; do
        # Verify reference is valid
        # (Manual review or script)
    done
```

---

## Test Execution Summary

**Total Acceptance Tests Defined:** 31

**Breakdown by Requirement:**
- FR-001: 4 tests (Code Organization)
- FR-002: 4 tests (Code Cleanup)
- FR-003: 4 tests (Feature Completion)
- FR-004: 3 tests (Code Quality)
- FR-005: 5 tests (Documentation Completeness)
- FR-006: 3 tests (Documentation Accuracy)
- NFR-001: 3 tests (Test Coverage)
- NFR-002: 2 tests (Performance)
- NFR-003: 3 tests (API Compatibility)
- NFR-004: 3 tests (Documentation Quality)

**Test Execution Strategy:**
- Baseline establishment: Increment 1 (before any changes)
- After each increment: Run full test suite
- STOP if any test fails
- Document results in increment completion checklist

---

## Traceability Matrix

| Requirement | Test IDs | Coverage |
|-------------|----------|----------|
| FR-001 | AT-FR-001-001 through AT-FR-001-004 | 100% |
| FR-002 | AT-FR-002-001 through AT-FR-002-004 | 100% |
| FR-003 | AT-FR-003-001 through AT-FR-003-004 | 100% |
| FR-004 | AT-FR-004-001 through AT-FR-004-003 | 100% |
| FR-005 | AT-FR-005-001 through AT-FR-005-005 | 100% |
| FR-006 | AT-FR-006-001 through AT-FR-006-003 | 100% |
| NFR-001 | AT-NFR-001-001 through AT-NFR-001-003 | 100% |
| NFR-002 | AT-NFR-002-001 through AT-NFR-002-002 | 100% |
| NFR-003 | AT-NFR-003-001 through AT-NFR-003-003 | 100% |
| NFR-004 | AT-NFR-004-001 through AT-NFR-004-003 | 100% |

**100% requirement coverage achieved** ✅

---

## Next Steps

**Phase 3 Complete** - All acceptance tests defined

**Ready for Implementation:**
- All requirements have testable acceptance criteria
- Test execution strategy documented
- Traceability matrix complete

**Recommended Next Steps:**
1. User review and approval of Phase 1-3 deliverables
2. Establish baseline (Increment 1)
3. Begin implementation following incremental approach
4. Execute acceptance tests after each increment

---

*End of Acceptance Test Specifications*
