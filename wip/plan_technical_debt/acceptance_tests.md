# Acceptance Tests: Technical Debt Reduction

**Project:** WKMP Technical Debt Reduction
**Specification:** SPEC_technical_debt_reduction.md
**Generated:** 2025-11-10
**Total Tests:** 31 acceptance tests for 26 requirements

---

## Test Index

| Test ID | Requirement | Phase | Priority | Type |
|---------|-------------|-------|----------|------|
| AT-TD1-001 | REQ-TD1-001 | 1 | CRITICAL | Unit |
| AT-TD1-002 | REQ-TD1-002 | 1 | CRITICAL | Integration |
| AT-TD1-003a | REQ-TD1-003 | 1 | HIGH | Build |
| AT-TD1-003b | REQ-TD1-003 | 1 | HIGH | Verification |
| AT-TD1-004 | REQ-TD1-004 | 1 | HIGH | Lint |
| AT-TD1-005 | REQ-TD1-005 | 1 | HIGH | Unit |
| AT-TD2-001a | REQ-TD2-001 | 2 | CRITICAL | Integration |
| AT-TD2-001b | REQ-TD2-001 | 2 | CRITICAL | API |
| AT-TD2-002 | REQ-TD2-002 | 2 | HIGH | Integration |
| AT-TD2-003 | REQ-TD2-003 | 2 | HIGH | Integration |
| AT-TD2-004 | REQ-TD2-004 | 2 | MEDIUM | Integration |
| AT-TD3-001a | REQ-TD3-001 | 3 | HIGH | Document |
| AT-TD3-001b | REQ-TD3-001 | 3 | HIGH | Verification |
| AT-TD3-002a | REQ-TD3-002 | 3 | HIGH | Integration |
| AT-TD3-002b | REQ-TD3-002 | 3 | HIGH | Metric |
| AT-TD3-003 | REQ-TD3-003 | 3 | MEDIUM | Verification |
| AT-TD4-001 | REQ-TD4-001 | 4 | HIGH | Build |
| AT-TD4-002a | REQ-TD4-002 | 4 | HIGH | Documentation |
| AT-TD4-002b | REQ-TD4-002 | 4 | HIGH | Verification |
| AT-TD4-003a | REQ-TD4-003 | 4 | MEDIUM | Documentation |
| AT-TD4-003b | REQ-TD4-003 | 4 | MEDIUM | Verification |
| AT-TD5-001a | REQ-TD5-001 | 5 | MEDIUM | Unit |
| AT-TD5-001b | REQ-TD5-001 | 5 | MEDIUM | Integration |
| AT-TD5-002 | REQ-TD5-002 | 5 | MEDIUM | Metric |
| AT-TD5-003 | REQ-TD5-003 | 5 | MEDIUM | Integration |
| AT-TD5-004 | REQ-TD5-004 | 5 | LOW | Verification |
| AT-ALL-001 | REQ-TD-ALL-001 | ALL | CRITICAL | Integration |
| AT-ALL-002a | REQ-TD-ALL-002 | ALL | CRITICAL | API |
| AT-ALL-002b | REQ-TD-ALL-002 | ALL | CRITICAL | Verification |
| AT-ALL-003 | REQ-TD-ALL-003 | ALL | HIGH | Process |
| AT-ALL-004 | REQ-TD-ALL-004 | ALL | MEDIUM | Documentation |

---

## Phase 1: Quick Wins - Acceptance Tests

### AT-TD1-001: Replace Blocking Sleep in Async Context

**Requirement:** REQ-TD1-001
**Priority:** CRITICAL

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- File exists: `wkmp-common/src/time.rs`

**Test Steps:**
1. Open `wkmp-common/src/time.rs`
2. Navigate to line 37 (or search for `std::thread::sleep` in test code)
3. Verify `std::thread::sleep` has been replaced with `tokio::time::sleep`
4. Verify `.await` is present after sleep call
5. Run test: `cargo test -p wkmp-common test_now_successive_calls_advance`

**Expected Results:**
- ✅ No `std::thread::sleep` calls in async contexts
- ✅ `tokio::time::sleep().await` present in test
- ✅ Test `test_now_successive_calls_advance` passes
- ✅ Test execution time unchanged (±10%)

**Pass Criteria:**
```bash
# Search for blocking sleep in async context
grep -rn "std::thread::sleep" wkmp-common/src/*.rs | grep -E "async fn|#\[tokio::test\]"
# Expected: No results

# Verify tokio sleep usage
grep -rn "tokio::time::sleep" wkmp-common/src/time.rs
# Expected: At least one result in async test

# Run specific test
cargo test -p wkmp-common test_now_successive_calls_advance --quiet
# Expected: test result: ok
```

---

### AT-TD1-002: Delete Dead Code (song_processor.rs)

**Requirement:** REQ-TD1-002
**Priority:** CRITICAL

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- File currently exists: `wkmp-ai/src/workflow/song_processor.rs`

**Test Steps:**
1. Verify file does NOT exist: `wkmp-ai/src/workflow/song_processor.rs`
2. Open `wkmp-ai/src/workflow/mod.rs`
3. Verify no comment or declaration of `song_processor` module (line 11 or anywhere)
4. Search codebase for references to `song_processor`
5. Run full test suite

**Expected Results:**
- ✅ File `song_processor.rs` deleted (-368 lines)
- ✅ No `pub mod song_processor;` or `// pub mod song_processor;` in mod.rs
- ✅ No imports referencing song_processor anywhere in codebase
- ✅ All 216 tests pass
- ✅ Project compiles without errors

**Pass Criteria:**
```bash
# Verify file deleted
test ! -f wkmp-ai/src/workflow/song_processor.rs && echo "PASS: File deleted" || echo "FAIL: File exists"

# Verify no references in mod.rs
grep -n "song_processor" wkmp-ai/src/workflow/mod.rs
# Expected: No results

# Verify no references in codebase
grep -rn "song_processor" wkmp-ai/src/
# Expected: No results

# Build and test
cargo build -p wkmp-ai && cargo test -p wkmp-ai --quiet
# Expected: Build success, test result: ok
```

---

### AT-TD1-003a: Fix Compiler Warnings - Build Clean

**Requirement:** REQ-TD1-003
**Priority:** HIGH

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 1 changes committed

**Test Steps:**
1. Clean build: `cargo clean`
2. Build both crates: `cargo build -p wkmp-ai -p wkmp-common`
3. Capture warning count from output

**Expected Results:**
- ✅ Zero compiler warnings
- ✅ Build succeeds
- ✅ No unused import warnings
- ✅ No unused variable warnings
- ✅ No dead code warnings

**Pass Criteria:**
```bash
# Clean build
cargo clean

# Build with warning capture
cargo build -p wkmp-ai -p wkmp-common 2>&1 | tee /tmp/build_output.txt

# Count warnings
WARNINGS=$(grep "warning:" /tmp/build_output.txt | wc -l)

if [ "$WARNINGS" -eq 0 ]; then
    echo "PASS: Zero warnings"
else
    echo "FAIL: $WARNINGS warnings found"
    grep "warning:" /tmp/build_output.txt
fi
```

---

### AT-TD1-003b: Fix Compiler Warnings - Specific Fixes Verification

**Requirement:** REQ-TD1-003
**Priority:** HIGH

**Preconditions:**
- AT-TD1-003a passed (zero warnings)

**Test Steps:**
1. Verify specific warning fixes from specification table:
   - Unused import `AudioFile` removed from `id3_extractor.rs:19`
   - Unused import `warn` removed from `essentia_analyzer.rs:41`
   - Unused import `warn` removed from `id3_genre_mapper.rs:23`
   - Unused import `ExtractionError` removed from `chromaprint_analyzer.rs:35`
   - Unused variable `event_bus` → `_event_bus` in `workflow_orchestrator.rs:1749`
   - Unused variable `sample_rate` → `_sample_rate` in `audio_derived_extractor.rs:224`
   - Dead fields in `acoustid_client.rs:335` removed or justified
   - Dead fields/methods in `metadata_fuser.rs` removed or justified

**Expected Results:**
- ✅ All specified warnings resolved
- ✅ Unused items removed or prefixed with `_`
- ✅ Dead code removed or marked `#[allow(dead_code)]` with comment justification

**Pass Criteria:**
```bash
# Verify specific fixes (sample checks)
! grep "use.*AudioFile" wkmp-ai/src/extractors/id3_extractor.rs | grep -v "//" && echo "PASS: AudioFile import removed"

! grep "use.*warn" wkmp-ai/src/extractors/essentia_analyzer.rs | grep -v "//" && echo "PASS: warn import removed from essentia"

grep "_event_bus" wkmp-ai/src/services/workflow_orchestrator.rs && echo "PASS: event_bus prefixed"

# All tests pass
cargo test -p wkmp-ai -p wkmp-common --quiet
# Expected: test result: ok
```

---

### AT-TD1-004: Fix Clippy Lints

**Requirement:** REQ-TD1-004
**Priority:** HIGH

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 1 changes committed

**Test Steps:**
1. Run clippy on both crates: `cargo clippy -p wkmp-ai -p wkmp-common -- -D warnings`
2. Verify zero clippy warnings

**Expected Results:**
- ✅ Zero clippy warnings
- ✅ No `from_*` method warnings
- ✅ No manual RangeInclusive::contains warnings
- ✅ No unnecessary map warnings
- ✅ No clone on Copy type warnings
- ✅ All 216 tests still pass

**Pass Criteria:**
```bash
# Run clippy (fail on warnings)
cargo clippy -p wkmp-ai -p wkmp-common -- -D warnings 2>&1 | tee /tmp/clippy_output.txt

# Check for warnings
CLIPPY_WARNINGS=$(grep "warning:" /tmp/clippy_output.txt | wc -l)

if [ "$CLIPPY_WARNINGS" -eq 0 ]; then
    echo "PASS: Zero clippy warnings"
else
    echo "FAIL: $CLIPPY_WARNINGS clippy warnings found"
    grep "warning:" /tmp/clippy_output.txt
fi

# Tests still pass
cargo test -p wkmp-ai -p wkmp-common --quiet
# Expected: test result: ok. 216 passed
```

---

### AT-TD1-005: Fix Panic Statements in Production Code

**Requirement:** REQ-TD1-005
**Priority:** HIGH

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`

**Test Steps:**
1. Search for `panic!` in production code (exclude tests)
2. Search for `unimplemented!` in production code
3. Verify events.rs:1564 panic replaced with proper error
4. Verify extractors/mod.rs:77 unimplemented handled
5. Run full test suite

**Expected Results:**
- ✅ No `panic!` calls in production code
- ✅ No `unimplemented!` calls in production code
- ✅ Error paths return Result types
- ✅ Error messages are descriptive
- ✅ All 216 tests pass

**Pass Criteria:**
```bash
# Find panic! in non-test code
grep -rn "panic!" wkmp-ai/src/ wkmp-common/src/ | grep -v "#\[cfg(test)\]" | grep -v "/tests/" | grep -v "test_"

# Expected: Only false positives (inside test functions)
# Manually verify results are in test code only

# Find unimplemented! in production code
grep -rn "unimplemented!" wkmp-ai/src/ wkmp-common/src/ | grep -v "#\[cfg(test)\]" | grep -v "/tests/"

# Expected: No results

# Verify events.rs fix
grep -A5 "Wrong event type" wkmp-common/src/events.rs
# Expected: Should see serde::de::Error::custom, not panic!

# Tests pass
cargo test -p wkmp-ai -p wkmp-common --quiet
# Expected: test result: ok. 216 passed
```

---

## Phase 2: File Organization - Acceptance Tests

### AT-TD2-001a: Refactor Workflow Orchestrator - Module Structure

**Requirement:** REQ-TD2-001
**Priority:** CRITICAL

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 1 complete

**Test Steps:**
1. Verify directory exists: `wkmp-ai/src/services/workflow_orchestrator/`
2. Verify all target files exist:
   - `mod.rs`
   - `phase_scanning.rs`
   - `phase_extraction.rs`
   - `phase_fingerprinting.rs`
   - `phase_segmenting.rs`
   - `phase_analyzing.rs`
   - `phase_flavoring.rs`
   - `entity_linking.rs`
3. Verify line counts for each file (<650 lines for largest)
4. Verify no file >800 lines
5. Run full test suite

**Expected Results:**
- ✅ workflow_orchestrator/ directory exists
- ✅ 8 module files present
- ✅ mod.rs coordinates phases (~200 lines)
- ✅ Largest file <650 lines (71% reduction from 2,253)
- ✅ Each phase module self-contained
- ✅ All 216 tests pass

**Pass Criteria:**
```bash
# Verify directory structure
test -d wkmp-ai/src/services/workflow_orchestrator && echo "PASS: Directory exists"

# Verify all files exist
for file in mod.rs phase_scanning.rs phase_extraction.rs phase_fingerprinting.rs phase_segmenting.rs phase_analyzing.rs phase_flavoring.rs entity_linking.rs; do
    test -f "wkmp-ai/src/services/workflow_orchestrator/$file" && echo "PASS: $file exists" || echo "FAIL: $file missing"
done

# Check line counts
echo "Line counts:"
wc -l wkmp-ai/src/services/workflow_orchestrator/*.rs

# Verify largest file <650 lines
MAX_LINES=$(wc -l wkmp-ai/src/services/workflow_orchestrator/*.rs | sort -rn | head -1 | awk '{print $1}')
if [ "$MAX_LINES" -lt 650 ]; then
    echo "PASS: Largest file is $MAX_LINES lines (<650 target)"
else
    echo "FAIL: Largest file is $MAX_LINES lines (>650 target)"
fi

# Tests pass
cargo test -p wkmp-ai --quiet
# Expected: test result: ok
```

---

### AT-TD2-001b: Refactor Workflow Orchestrator - Public API Preservation

**Requirement:** REQ-TD2-001
**Priority:** CRITICAL

**Preconditions:**
- AT-TD2-001a passed (module structure exists)

**Test Steps:**
1. Verify `WorkflowOrchestrator` struct is re-exported from `mod.rs`
2. Verify public methods remain accessible
3. Verify phase modules are not publicly exported
4. Attempt import from external code (wkmp-ui perspective)
5. Run integration tests

**Expected Results:**
- ✅ `WorkflowOrchestrator` publicly accessible
- ✅ Public API unchanged (backward compatible)
- ✅ Phase modules are internal implementation details
- ✅ External crates can still import and use WorkflowOrchestrator
- ✅ All 216 tests pass

**Pass Criteria:**
```bash
# Verify re-export in mod.rs
grep "pub use" wkmp-ai/src/services/workflow_orchestrator/mod.rs | grep "WorkflowOrchestrator"
# Expected: pub use ... WorkflowOrchestrator

# Verify phase modules not publicly exported
! grep "pub mod phase_" wkmp-ai/src/services/workflow_orchestrator/mod.rs && echo "PASS: Phase modules private"

# Check public API (from lib.rs or services/mod.rs)
grep -rn "pub.*workflow_orchestrator" wkmp-ai/src/services/mod.rs wkmp-ai/src/lib.rs

# Tests pass
cargo test -p wkmp-ai --quiet
# Expected: test result: ok. 216 passed
```

---

### AT-TD2-002: Split events.rs by Category

**Requirement:** REQ-TD2-002
**Priority:** HIGH

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 1 complete

**Test Steps:**
1. Verify directory exists: `wkmp-common/src/events/`
2. Verify all target files exist:
   - `mod.rs`
   - `import_events.rs`
   - `playback_events.rs`
   - `system_events.rs`
   - `sse_formatting.rs`
3. Verify line counts (<600 lines for largest)
4. Verify re-exports in mod.rs maintain public API
5. Run full test suite

**Expected Results:**
- ✅ events/ directory exists
- ✅ 5 module files present
- ✅ mod.rs re-exports all event types
- ✅ Largest file <600 lines
- ✅ SSE serialization preserved
- ✅ All 216 tests pass

**Pass Criteria:**
```bash
# Verify directory structure
test -d wkmp-common/src/events && echo "PASS: events/ directory exists"

# Verify files
for file in mod.rs import_events.rs playback_events.rs system_events.rs sse_formatting.rs; do
    test -f "wkmp-common/src/events/$file" && echo "PASS: $file exists"
done

# Check line counts
wc -l wkmp-common/src/events/*.rs

# Verify largest <600 lines
MAX_LINES=$(wc -l wkmp-common/src/events/*.rs | sort -rn | head -1 | awk '{print $1}')
[ "$MAX_LINES" -lt 600 ] && echo "PASS: Largest $MAX_LINES lines" || echo "FAIL: Largest $MAX_LINES lines"

# Verify re-exports
grep "pub use" wkmp-common/src/events/mod.rs | wc -l
# Expected: Multiple re-exports

# Tests pass
cargo test -p wkmp-common --quiet
```

---

### AT-TD2-003: Split params.rs by Parameter Group

**Requirement:** REQ-TD2-003
**Priority:** HIGH

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 1 complete

**Test Steps:**
1. Verify directory exists: `wkmp-common/src/params/`
2. Verify all target files exist:
   - `mod.rs`
   - `crossfade_params.rs`
   - `selector_params.rs`
   - `timing_params.rs`
   - `flavor_params.rs`
   - `system_params.rs`
3. Verify line counts (<400 lines for largest)
4. Verify re-exports maintain public API
5. Verify database serialization unchanged
6. Run full test suite

**Expected Results:**
- ✅ params/ directory exists
- ✅ 6 module files present
- ✅ Parameters grouped by functional area
- ✅ Largest file <400 lines
- ✅ Default implementations preserved
- ✅ All 216 tests pass

**Pass Criteria:**
```bash
# Verify directory and files
test -d wkmp-common/src/params && echo "PASS: params/ directory exists"

for file in mod.rs crossfade_params.rs selector_params.rs timing_params.rs flavor_params.rs system_params.rs; do
    test -f "wkmp-common/src/params/$file" && echo "PASS: $file exists"
done

# Line counts
wc -l wkmp-common/src/params/*.rs

# Verify largest <400 lines
MAX_LINES=$(wc -l wkmp-common/src/params/*.rs | sort -rn | head -1 | awk '{print $1}')
[ "$MAX_LINES" -lt 400 ] && echo "PASS: Largest $MAX_LINES lines"

# Tests pass (especially database serialization tests)
cargo test -p wkmp-common --quiet
```

---

### AT-TD2-004: Reorganize api/ui.rs into Page Modules

**Requirement:** REQ-TD2-004
**Priority:** MEDIUM

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 1 complete

**Test Steps:**
1. Verify directory exists: `wkmp-ai/src/api/ui/`
2. Verify all target files exist:
   - `mod.rs`
   - `dashboard_page.rs`
   - `settings_page.rs`
   - `library_page.rs`
   - `import_page.rs`
   - `components.rs`
3. Verify line counts (<300 lines for largest)
4. Verify routes preserved (URLs unchanged)
5. Test HTTP endpoints return correct HTML
6. Run full test suite

**Expected Results:**
- ✅ ui/ directory exists
- ✅ 6 module files present
- ✅ Each page in separate file
- ✅ Shared components extracted
- ✅ Largest file <300 lines
- ✅ Routes preserved (URLs unchanged)
- ✅ All 216 tests pass

**Pass Criteria:**
```bash
# Verify directory and files
test -d wkmp-ai/src/api/ui && echo "PASS: ui/ directory exists"

for file in mod.rs dashboard_page.rs settings_page.rs library_page.rs import_page.rs components.rs; do
    test -f "wkmp-ai/src/api/ui/$file" && echo "PASS: $file exists"
done

# Line counts
wc -l wkmp-ai/src/api/ui/*.rs

# Verify largest <300 lines
MAX_LINES=$(wc -l wkmp-ai/src/api/ui/*.rs | sort -rn | head -1 | awk '{print $1}')
[ "$MAX_LINES" -lt 300 ] && echo "PASS: Largest $MAX_LINES lines"

# Tests pass
cargo test -p wkmp-ai --quiet
```

---

## Phase 3: Error Handling - Acceptance Tests

### AT-TD3-001a: Audit unwrap() Usage - Document Creation

**Requirement:** REQ-TD3-001
**Priority:** HIGH

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 2 complete

**Test Steps:**
1. Verify audit document exists: `wip/unwrap_audit.md`
2. Verify document contains table with columns:
   - File
   - Line
   - Context
   - Classification (KEEP/CONVERT/REMOVE)
   - Priority (HIGH/MEDIUM/LOW)
   - Justification
3. Count table entries (should be ~506)
4. Verify all classifications have justifications

**Expected Results:**
- ✅ `wip/unwrap_audit.md` exists
- ✅ Table format with all required columns
- ✅ All 506 unwrap/expect calls classified
- ✅ Each classification has justification
- ✅ Priority assigned to all convertible calls
- ✅ Document checked into git

**Pass Criteria:**
```bash
# Verify file exists
test -f wip/unwrap_audit.md && echo "PASS: Audit document exists"

# Count table rows (excluding header)
AUDIT_ROWS=$(grep "^|" wip/unwrap_audit.md | grep -v "^| File" | grep -v "^|--" | wc -l)

echo "Audit rows: $AUDIT_ROWS (expected ~506)"

# Verify columns present
grep "| File | Line | Context | Classification | Priority | Justification |" wip/unwrap_audit.md && echo "PASS: Table format correct"

# Verify classifications used
grep "KEEP\|CONVERT\|REMOVE" wip/unwrap_audit.md | wc -l
# Expected: >0

# Verify in git
git ls-files wip/unwrap_audit.md && echo "PASS: Document in git"
```

---

### AT-TD3-001b: Audit unwrap() Usage - Verification

**Requirement:** REQ-TD3-001
**Priority:** HIGH

**Preconditions:**
- AT-TD3-001a passed (audit document exists)

**Test Steps:**
1. Search codebase for unwrap() calls (excluding tests)
2. Compare count with audit document
3. Spot-check 10 random entries for accuracy
4. Verify classification criteria applied consistently

**Expected Results:**
- ✅ Audit count matches actual unwrap() count
- ✅ Spot checks are accurate (file/line/context match)
- ✅ Classifications follow criteria:
  - KEEP: Startup config, FFI, compile-time invariants
  - CONVERT: User input, file I/O, network, database
  - REMOVE: Default values, fallback logic
- ✅ Priorities reflect user impact

**Pass Criteria:**
```bash
# Count unwrap/expect in production code
ACTUAL_COUNT=$(grep -rn "\.unwrap()\|\.expect(" wkmp-ai/src/ wkmp-common/src/ | \
    grep -v "#\[cfg(test)\]" | \
    grep -v "/tests/" | \
    grep -v "test_" | \
    wc -l)

echo "Actual unwrap count: $ACTUAL_COUNT"

# Compare with audit document count
AUDIT_COUNT=$(grep -c "^|" wip/unwrap_audit.md | tail -1)

echo "Audit count: $AUDIT_COUNT"

# Manual spot check (review 10 random entries)
echo "Spot check: Manually verify 10 random entries from audit match codebase"
```

---

### AT-TD3-002a: Convert User-Facing unwrap() Calls - Implementation

**Requirement:** REQ-TD3-002
**Priority:** HIGH

**Preconditions:**
- AT-TD3-001b passed (audit complete)
- Phase 2 complete

**Test Steps:**
1. Search for unwrap() in high-priority locations:
   - api/ (HTTP handlers)
   - services/file_scanner.rs
   - workflow/workflow_orchestrator.rs
   - workflow/storage.rs
2. Verify conversions use Result + anyhow::Context
3. Verify error messages are descriptive
4. Run full test suite
5. Test error paths return proper errors

**Expected Results:**
- ✅ User-facing paths return Result<T, E>
- ✅ Error messages include context (what/why)
- ✅ Error propagation uses ? operator
- ✅ No unwrap() in HTTP handlers
- ✅ No unwrap() in file import workflow
- ✅ All 216 tests pass
- ✅ New tests verify error paths

**Pass Criteria:**
```bash
# Check HTTP handlers for unwrap (should be zero)
UNWRAPS_API=$(grep -rn "\.unwrap()\|\.expect(" wkmp-ai/src/api/ | \
    grep -v "#\[cfg(test)\]" | \
    grep -v "/tests/" | \
    wc -l)

echo "unwrap() in api/: $UNWRAPS_API (expected: 0)"

# Check file_scanner for unwrap
UNWRAPS_SCANNER=$(grep -rn "\.unwrap()\|\.expect(" wkmp-ai/src/services/file_scanner.rs | \
    grep -v "#\[cfg(test)\]" | \
    wc -l)

echo "unwrap() in file_scanner.rs: $UNWRAPS_SCANNER (expected: 0 or low)"

# Verify anyhow::Context usage
grep -rn "\.context(\|\.with_context(" wkmp-ai/src/ | wc -l
# Expected: Many uses (>50)

# Tests pass
cargo test -p wkmp-ai -p wkmp-common --quiet
```

---

### AT-TD3-002b: Convert User-Facing unwrap() Calls - Metric

**Requirement:** REQ-TD3-002
**Priority:** HIGH

**Preconditions:**
- AT-TD3-002a passed (conversions implemented)

**Test Steps:**
1. Count remaining unwrap() calls in production code
2. Verify count <50 (target achieved)
3. Verify remaining unwrap() calls are all classified as KEEP with justifications

**Expected Results:**
- ✅ <50 unwrap() calls remaining in production code
- ✅ All remaining calls are justified (KEEP classification)
- ✅ 90%+ reduction from 506 → <50

**Pass Criteria:**
```bash
# Count remaining unwrap calls
REMAINING=$(grep -rn "\.unwrap()\|\.expect(" wkmp-ai/src/ wkmp-common/src/ | \
    grep -v "#\[cfg(test)\]" | \
    grep -v "/tests/" | \
    grep -v "test_" | \
    wc -l)

echo "Remaining unwrap() calls: $REMAINING (target: <50)"

if [ "$REMAINING" -lt 50 ]; then
    echo "PASS: Target achieved (<50)"
    REDUCTION=$(echo "scale=1; (506 - $REMAINING) / 506 * 100" | bc)
    echo "Reduction: ${REDUCTION}% from original 506"
else
    echo "FAIL: $REMAINING remaining (target: <50)"
fi

# Verify remaining are justified
echo "Manual check: Verify remaining unwrap() calls have justification comments"
```

---

### AT-TD3-003: Add Error Context with anyhow

**Requirement:** REQ-TD3-003
**Priority:** MEDIUM

**Preconditions:**
- AT-TD3-002a passed (conversions implemented)

**Test Steps:**
1. Search for ? operators without context in critical paths:
   - Import workflow
   - Database operations
   - Network API calls
   - File system operations
2. Verify context added using `.context()` or `.with_context()`
3. Verify error messages include operation attempted
4. Verify error messages include relevant identifiers (paths, UUIDs)
5. Test error messages are helpful

**Expected Results:**
- ✅ All ? operators in critical paths have context
- ✅ Error messages include operation attempted
- ✅ Error messages include relevant identifiers
- ✅ Context added without changing error types
- ✅ All 216 tests pass

**Pass Criteria:**
```bash
# Count uses of anyhow::Context
CONTEXT_USES=$(grep -rn "\.context(\|\.with_context(" wkmp-ai/src/services/ wkmp-ai/src/workflow/ | wc -l)

echo "Context uses: $CONTEXT_USES (expected: >50)"

# Spot check: Verify error messages include context
grep -A1 "\.with_context(" wkmp-ai/src/services/file_scanner.rs | head -20
# Manual review: Error messages should include "Failed to..." with details

# Tests pass
cargo test -p wkmp-ai --quiet
```

---

## Phase 4: Documentation - Acceptance Tests

### AT-TD4-001: Enable missing_docs Lint

**Requirement:** REQ-TD4-001
**Priority:** HIGH

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 3 complete

**Test Steps:**
1. Verify `#![warn(missing_docs)]` in `wkmp-ai/src/lib.rs`
2. Verify `#![warn(missing_docs)]` in `wkmp-common/src/lib.rs`
3. Build and capture warnings
4. Verify warnings generated for undocumented items

**Expected Results:**
- ✅ Lint enabled in both lib.rs files
- ✅ Generates warnings for undocumented public items
- ✅ Does not block compilation (warn, not deny)
- ✅ Baseline warnings captured

**Pass Criteria:**
```bash
# Verify lint enabled
grep "#!\[warn(missing_docs)\]" wkmp-ai/src/lib.rs && echo "PASS: wkmp-ai lint enabled"
grep "#!\[warn(missing_docs)\]" wkmp-common/src/lib.rs && echo "PASS: wkmp-common lint enabled"

# Build and check for missing_docs warnings (before documentation phase)
cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation" | wc -l
# Expected: Some warnings (will be eliminated in AT-TD4-002/003)

# Verify compilation succeeds
cargo build -p wkmp-ai -p wkmp-common && echo "PASS: Compilation succeeds"
```

---

### AT-TD4-002a: Document Public Modules - Implementation

**Requirement:** REQ-TD4-002
**Priority:** HIGH

**Preconditions:**
- AT-TD4-001 passed (lint enabled)

**Test Steps:**
1. List all public modules in wkmp-ai and wkmp-common
2. Verify each has module-level documentation (`//!` doc comments)
3. Verify documentation includes:
   - Brief one-line description
   - Longer explanation of purpose
   - Examples (where applicable)
   - Architecture notes (where applicable)
4. Build and verify no missing module doc warnings

**Expected Results:**
- ✅ All public modules have `//!` documentation
- ✅ Documentation includes purpose, usage, examples
- ✅ Documentation is accurate
- ✅ All 216 tests pass
- ✅ `cargo doc` generates complete documentation

**Pass Criteria:**
```bash
# Build and check for missing module docs
cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation for a module"
# Expected: No results

# Generate docs
cargo doc -p wkmp-ai -p wkmp-common --no-deps --quiet && echo "PASS: Docs generated"

# Manual spot check
echo "Manual check: Review generated docs at target/doc/wkmp_ai/index.html"
echo "Verify module documentation present for all modules"
```

---

### AT-TD4-002b: Document Public Modules - Verification

**Requirement:** REQ-TD4-002
**Priority:** HIGH

**Preconditions:**
- AT-TD4-002a passed (documentation added)

**Test Steps:**
1. Open generated documentation: `target/doc/wkmp_ai/index.html`
2. Verify all modules listed have documentation
3. Spot-check 10 modules for quality:
   - Description is clear
   - Purpose is stated
   - Examples are present (where applicable)
4. Verify no "missing documentation" warnings

**Expected Results:**
- ✅ 100% of public modules documented
- ✅ Documentation quality is high
- ✅ Zero missing module documentation warnings

**Pass Criteria:**
```bash
# Generate docs and open
cargo doc -p wkmp-ai -p wkmp-common --no-deps --open

# Check for warnings
cargo doc -p wkmp-ai -p wkmp-common --no-deps 2>&1 | grep "missing documentation for a module" | wc -l
# Expected: 0

echo "Manual verification: Review documentation quality for all modules"
```

---

### AT-TD4-003a: Document Public Functions - Implementation

**Requirement:** REQ-TD4-003
**Priority:** MEDIUM

**Preconditions:**
- AT-TD4-002b passed (modules documented)

**Test Steps:**
1. List all public functions in wkmp-ai and wkmp-common (294 total)
2. Verify each has function documentation (`///` doc comments)
3. Verify documentation includes:
   - Brief one-line description
   - Arguments documented
   - Returns documented
   - Errors documented (for Result types)
   - Examples (for complex functions)
4. Build and verify no missing function doc warnings

**Expected Results:**
- ✅ All 294 public functions have documentation
- ✅ Parameters documented with types and purpose
- ✅ Return values explained
- ✅ Error conditions documented
- ✅ Examples provided for complex functions
- ✅ All 216 tests pass
- ✅ `cargo doc` generates complete API docs

**Pass Criteria:**
```bash
# Build and check for missing function docs
cargo build -p wkmp-ai -p wkmp-common 2>&1 | grep "missing documentation for" | grep -v "module"
# Expected: No results (or very few)

# Generate docs
cargo doc -p wkmp-ai -p wkmp-common --no-deps --quiet && echo "PASS: Docs generated"

# Count documented functions (manual audit)
echo "Manual check: Verify all public functions documented"
```

---

### AT-TD4-003b: Document Public Functions - Verification

**Requirement:** REQ-TD4-003
**Priority:** MEDIUM

**Preconditions:**
- AT-TD4-003a passed (documentation added)

**Test Steps:**
1. Open generated documentation
2. Navigate to each module
3. Spot-check 20 functions for quality:
   - Description is clear
   - Arguments explained
   - Return value explained
   - Errors documented (if Result type)
4. Verify zero missing function documentation warnings

**Expected Results:**
- ✅ 100% of public functions documented (294/294)
- ✅ Documentation quality is high
- ✅ Zero missing function documentation warnings

**Pass Criteria:**
```bash
# Check for warnings
cargo doc -p wkmp-ai -p wkmp-common --no-deps 2>&1 | grep "missing documentation for" | wc -l
# Expected: 0

# Generate final docs
cargo doc -p wkmp-ai -p wkmp-common --no-deps && echo "PASS: Complete documentation generated"

echo "Manual verification: Review documentation quality for sample of functions"
```

---

## Phase 5: Code Quality - Acceptance Tests

### AT-TD5-001a: Extract Rate Limiter Utility - Implementation

**Requirement:** REQ-TD5-001
**Priority:** MEDIUM

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 4 complete

**Test Steps:**
1. Verify file exists: `wkmp-common/src/rate_limiter.rs`
2. Verify RateLimiter struct matches specification:
   - `new(interval: Duration)` constructor
   - `acquire() -> ()` async method
   - Clone trait derived
   - Send + Sync traits (via Arc<Mutex>)
3. Verify module exported from wkmp-common/src/lib.rs
4. Run rate limiter unit tests

**Expected Results:**
- ✅ `wkmp-common/src/rate_limiter.rs` exists
- ✅ RateLimiter struct implemented per spec
- ✅ Unit tests verify rate limiting behavior
- ✅ Tests pass

**Pass Criteria:**
```bash
# Verify file exists
test -f wkmp-common/src/rate_limiter.rs && echo "PASS: Rate limiter utility exists"

# Verify struct definition
grep "pub struct RateLimiter" wkmp-common/src/rate_limiter.rs && echo "PASS: Struct defined"

# Verify methods
grep "pub fn new" wkmp-common/src/rate_limiter.rs && echo "PASS: new() present"
grep "pub async fn acquire" wkmp-common/src/rate_limiter.rs && echo "PASS: acquire() present"

# Run tests
cargo test -p wkmp-common rate_limiter --quiet
# Expected: test result: ok
```

---

### AT-TD5-001b: Extract Rate Limiter Utility - Integration

**Requirement:** REQ-TD5-001
**Priority:** MEDIUM

**Preconditions:**
- AT-TD5-001a passed (utility implemented)

**Test Steps:**
1. Verify all 4 clients refactored to use shared utility:
   - `wkmp-ai/src/services/acoustid_client.rs`
   - `wkmp-ai/src/services/musicbrainz_client.rs`
   - `wkmp-ai/src/extractors/musicbrainz_client.rs`
   - `wkmp-ai/src/services/acousticbrainz_client.rs`
2. Verify each client imports RateLimiter from wkmp-common
3. Verify duplicate implementations removed
4. Verify rate limiting behavior preserved (1 req/sec for MusicBrainz, etc.)
5. Run integration tests for all clients

**Expected Results:**
- ✅ All 4 clients use shared RateLimiter
- ✅ Duplicate implementations removed
- ✅ Rate limiting behavior preserved
- ✅ All 216 tests pass
- ✅ Integration tests verify rate limiting

**Pass Criteria:**
```bash
# Verify imports
for client in services/acoustid_client.rs services/musicbrainz_client.rs extractors/musicbrainz_client.rs services/acousticbrainz_client.rs; do
    grep "use wkmp_common::rate_limiter::RateLimiter" "wkmp-ai/src/$client" && echo "PASS: $client uses shared utility"
done

# Verify duplicate logic removed (no Arc<Mutex<Option<Instant>>> in clients)
! grep "Arc<Mutex<Option<Instant>>>" wkmp-ai/src/services/acoustid_client.rs && echo "PASS: Duplicate removed from acoustid"

# Tests pass
cargo test -p wkmp-ai --quiet
# Expected: test result: ok
```

---

### AT-TD5-002: Break Up Long Functions

**Requirement:** REQ-TD5-002
**Priority:** MEDIUM

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 4 complete

**Test Steps:**
1. Search for functions >200 lines in wkmp-ai and wkmp-common
2. Verify count is zero (all functions <200 lines)
3. Spot-check 10 previously long functions:
   - Verify extracted into smaller functions
   - Verify single responsibility principle
   - Verify function names are descriptive
4. Run full test suite

**Expected Results:**
- ✅ Zero functions >200 lines
- ✅ Extracted functions have single responsibility
- ✅ Function names clearly describe purpose
- ✅ All 216 tests pass
- ✅ Logic preserved (no behavior changes)

**Pass Criteria:**
```bash
# Find functions >200 lines (requires custom script or manual review)
echo "Manual check: Review function lengths"

# Automated approach: Use tokei or similar
tokei wkmp-ai/src/ wkmp-common/src/ --output json | jq '.Rust.children[] | select(.stats.code > 200)'
# Expected: No results

# Alternative: Manual sampling
# Review workflow_orchestrator/, api/ui/, workflow/storage.rs

# Tests pass
cargo test -p wkmp-ai -p wkmp-common --quiet
# Expected: test result: ok. 216 passed
```

---

### AT-TD5-003: Consolidate Configuration Structs

**Requirement:** REQ-TD5-003
**Priority:** MEDIUM

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 4 complete

**Test Steps:**
1. Verify file exists: `wkmp-common/src/config/workflow.rs`
2. Verify unified `WorkflowConfig` struct exists
3. Verify struct includes:
   - API keys (acoustid_api_key)
   - Feature flags (enable_musicbrainz, enable_essentia, enable_audio_derived)
   - Quality thresholds (min_quality_threshold)
   - Import parameters
4. Verify builder pattern implemented
5. Verify duplicate structs removed/refactored:
   - PipelineConfig → WorkflowConfig
   - SongProcessorConfig → removed (dead code)
6. Update all usages to new WorkflowConfig
7. Run full test suite

**Expected Results:**
- ✅ Single WorkflowConfig struct
- ✅ Builder pattern for optional fields
- ✅ All existing usages updated
- ✅ All 216 tests pass
- ✅ No behavior changes

**Pass Criteria:**
```bash
# Verify file exists
test -f wkmp-common/src/config/workflow.rs && echo "PASS: WorkflowConfig exists"

# Verify struct definition
grep "pub struct WorkflowConfig" wkmp-common/src/config/workflow.rs && echo "PASS: Struct defined"

# Verify builder
grep "pub fn builder" wkmp-common/src/config/workflow.rs && echo "PASS: Builder pattern present"

# Verify PipelineConfig removed/replaced
! grep "PipelineConfig" wkmp-ai/src/workflow/pipeline.rs | grep "struct" && echo "PASS: PipelineConfig replaced"

# Tests pass
cargo test -p wkmp-ai -p wkmp-common --quiet
```

---

### AT-TD5-004: Remove Magic Numbers

**Requirement:** REQ-TD5-004
**Priority:** LOW

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase 5 in progress

**Test Steps:**
1. Search codebase for common magic numbers:
   - Status update intervals (15, 30, 60 seconds)
   - SSE keepalive timeouts
   - Buffer sizes
   - Threshold values
2. Verify replaced with named constants
3. Verify constants documented with units/meaning
4. Run full test suite

**Expected Results:**
- ✅ All magic numbers >1 replaced with constants
- ✅ Constants have descriptive names
- ✅ Constants documented with units/meaning
- ✅ All 216 tests pass

**Pass Criteria:**
```bash
# Search for Duration::from_secs with literal numbers (sample check)
grep -rn "Duration::from_secs([0-9]" wkmp-ai/src/ wkmp-common/src/ | grep -v "Duration::from_secs(0)" | head -10
# Review: Should see named constants instead

# Verify constant definitions
grep -rn "^const.*Duration = " wkmp-ai/src/ wkmp-common/src/ | wc -l
# Expected: Multiple constants defined

# Manual spot check
echo "Manual check: Verify magic numbers replaced with named constants"

# Tests pass
cargo test -p wkmp-ai -p wkmp-common --quiet
```

---

## Cross-Phase Requirements - Acceptance Tests

### AT-ALL-001: Test Preservation

**Requirement:** REQ-TD-ALL-001
**Priority:** CRITICAL
**Run:** After EVERY increment in ALL phases

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Any code change committed

**Test Steps:**
1. Run full test suite: `cargo test -p wkmp-ai -p wkmp-common --all-features`
2. Verify all 216 tests pass
3. Verify no tests skipped or ignored
4. Measure test execution time
5. Compare with baseline (should not increase >10%)

**Expected Results:**
- ✅ All 216 tests pass
- ✅ No tests skipped or disabled
- ✅ No test behavior changes
- ✅ Test execution time unchanged (±10%)

**Pass Criteria:**
```bash
# Run full test suite
cargo test -p wkmp-ai -p wkmp-common --all-features --quiet -- --test-threads=1

# Expected output format:
# test result: ok. 216 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

# Automated check
TEST_OUTPUT=$(cargo test -p wkmp-ai -p wkmp-common --all-features --quiet 2>&1)

PASSED=$(echo "$TEST_OUTPUT" | grep "test result: ok" | grep -oP '\d+ passed' | grep -oP '\d+')

if [ "$PASSED" -eq 216 ]; then
    echo "PASS: All 216 tests passing"
else
    echo "FAIL: Only $PASSED tests passing (expected 216)"
fi

# Measure execution time (baseline: record first, then compare)
# time cargo test -p wkmp-ai -p wkmp-common --all-features --quiet
```

---

### AT-ALL-002a: Backward Compatibility - API Contract Preservation

**Requirement:** REQ-TD-ALL-002
**Priority:** CRITICAL
**Run:** After each phase completion

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase N complete

**Test Steps:**
1. List all public types in wkmp-common before refactoring (baseline)
2. List all public types in wkmp-common after refactoring
3. Compare (should be identical or additive only)
4. List all public functions in wkmp-common before/after
5. Compare function signatures (should be unchanged)
6. Run cargo semver checks (if available)

**Expected Results:**
- ✅ All public function signatures unchanged
- ✅ All public types unchanged
- ✅ All public constants unchanged
- ✅ Only additive changes allowed (new private functions OK)
- ✅ No breaking changes

**Pass Criteria:**
```bash
# List public API items (sample check)
# Before refactoring (baseline): Save to /tmp/api_before.txt
# After refactoring: Save to /tmp/api_after.txt

# Check wkmp-common public API
rustdoc -Z unstable-options --output-format json -p wkmp-common > /tmp/api_after.json

# Manual comparison
echo "Manual check: Compare public API before/after refactoring"
echo "Verify no breaking changes (removed items, signature changes)"

# Alternative: cargo-public-api (if available)
# cargo install cargo-public-api
# cargo public-api -p wkmp-common > /tmp/api.txt
```

---

### AT-ALL-002b: Backward Compatibility - Semantic Versioning

**Requirement:** REQ-TD-ALL-002
**Priority:** CRITICAL
**Run:** After project completion

**Preconditions:**
- All phases complete

**Test Steps:**
1. Review CHANGELOG.md for all changes
2. Classify changes:
   - Breaking changes → major version bump (FORBIDDEN)
   - New features → minor version bump (NONE expected)
   - Bug fixes / refactoring → patch version bump (EXPECTED)
3. Verify version bump plan is patch-only
4. Verify Cargo.toml version updated correctly

**Expected Results:**
- ✅ No breaking changes (no major version bump required)
- ✅ Only refactoring changes (patch version bump)
- ✅ Semantic versioning preserved

**Pass Criteria:**
```bash
# Check Cargo.toml versions
grep "^version" wkmp-common/Cargo.toml
grep "^version" wkmp-ai/Cargo.toml

# Verify versions bumped by patch only (0.X.Y -> 0.X.Y+1)

echo "Manual check: Review CHANGELOG.md"
echo "Verify all changes are non-breaking refactoring (patch bump)"
```

---

### AT-ALL-003: Incremental Delivery

**Requirement:** REQ-TD-ALL-003
**Priority:** HIGH
**Run:** After each phase completion

**Preconditions:**
- Working directory: `/home/sw/Dev/McRhythm`
- Phase N complete

**Test Steps:**
1. Verify all tests pass (AT-ALL-001)
2. Verify clean git status (no uncommitted changes)
3. Verify git commits exist for each increment
4. Verify phase tag exists (e.g., `technical-debt-phase1-complete`)
5. Verify CI pipeline green (if applicable)

**Expected Results:**
- ✅ Each phase has clear entry/exit criteria
- ✅ Phases completed without blocking
- ✅ Codebase not in broken state
- ✅ Git commits after each increment with tests passing

**Pass Criteria:**
```bash
# Verify clean git status
git status --porcelain
# Expected: No output (clean working directory)

# Verify recent commits
git log --oneline -10
# Expected: Commits for recent increments

# Verify phase tag
git tag | grep "technical-debt-phase"
# Expected: Tags for completed phases

# All tests pass
cargo test -p wkmp-ai -p wkmp-common --quiet
# Expected: test result: ok. 216 passed
```

---

### AT-ALL-004: Documentation Updates

**Requirement:** REQ-TD-ALL-004
**Priority:** MEDIUM
**Run:** After each phase completion

**Preconditions:**
- Phase N complete

**Test Steps:**
1. Verify CHANGELOG.md updated with phase summary
2. Verify module documentation updated for refactored modules
3. Verify README.md updated if public API changed (unlikely)
4. Verify all documentation changes committed

**Expected Results:**
- ✅ Module moves update import documentation
- ✅ Extracted functions have documentation
- ✅ README updated if needed
- ✅ CHANGELOG entries for each phase

**Pass Criteria:**
```bash
# Verify CHANGELOG.md updated
grep "Phase [1-5]" CHANGELOG.md
# Expected: Entries for each completed phase

# Verify format matches specification
grep -A5 "### Phase 1:" CHANGELOG.md
# Expected:
# ### Phase 1: Quick Wins (YYYY-MM-DD)
# - Fixed blocking sleep in async context
# - Removed dead code (song_processor.rs)
# ...

# Check git for documentation changes
git log --oneline --grep="Phase [1-5]" | head -5
# Expected: Commits with phase documentation
```

---

## Test Execution Summary

**Total Tests:** 31 acceptance tests
**Critical Tests:** 9 (MUST PASS for project success)
**High Priority Tests:** 12
**Medium Priority Tests:** 9
**Low Priority Tests:** 1

**Testing Strategy:**
1. Run AT-ALL-001 (test preservation) after EVERY increment
2. Run phase-specific tests after phase completion
3. Run AT-ALL-002 (backward compatibility) after each phase
4. Run AT-ALL-003 (incremental delivery) after each phase
5. Run AT-ALL-004 (documentation updates) after each phase

**Critical Path Tests (Cannot Proceed Without):**
- AT-TD1-001: Blocking sleep fix (prevents runtime issues)
- AT-TD1-002: Dead code removal (reduces confusion)
- AT-TD2-001a/b: Workflow orchestrator refactoring (maintainability)
- AT-TD3-001a: Unwrap audit (risk assessment)
- AT-ALL-001: Test preservation (regression prevention)
- AT-ALL-002a/b: Backward compatibility (no breaking changes)

---

## Notes on Test Automation

**Automated Tests:** Most tests can be automated using bash scripts:
- Build/lint checks (AT-TD1-003, AT-TD1-004, AT-TD4-001)
- File existence checks (AT-TD1-002, AT-TD2-001a, etc.)
- Metric verification (line counts, unwrap counts)
- Test execution (AT-ALL-001)

**Manual Tests:** Some tests require human judgment:
- Code quality spot checks (AT-TD3-001b)
- Documentation quality review (AT-TD4-002b, AT-TD4-003b)
- API contract comparison (AT-ALL-002a)
- CHANGELOG review (AT-ALL-004)

**Recommended Approach:**
1. Automate what you can (bash scripts in `scripts/acceptance_tests/`)
2. Manual review for subjective criteria
3. Run automated tests in CI pipeline
4. Manual review as gate before proceeding to next phase
