# Rust Multi-Crate Workflow

**Purpose:** Run comprehensive quality checks across all WKMP workspace crates with optimized reporting

**Task:** Execute formatting, linting, and testing across the entire WKMP workspace.

---

## Instructions

You are running comprehensive Rust quality checks for the WKMP multi-crate workspace. Execute all checks systematically and report results concisely.

---

## Workspace Overview

WKMP consists of 6 crates in a Cargo workspace:

**Microservices:**
- `wkmp-ap` - Audio Player (port 5721)
- `wkmp-ui` - User Interface (port 5720)
- `wkmp-pd` - Program Director (port 5722)
- `wkmp-ai` - Audio Ingest (port 5723)
- `wkmp-le` - Lyric Editor (port 5724)

**Shared Library:**
- `wkmp-common` - Common library (database models, events, utilities)

---

## Execution Steps

### Step 1: Format Check (cargo fmt)

Run formatting check across entire workspace:

```bash
cargo fmt --all -- --check
```

**Success:** "No formatting issues"
**Failure:** List files needing formatting, then auto-fix:
```bash
cargo fmt --all
```

**Report:**
- ✅ All files formatted correctly
- OR ⚠️ Auto-formatted X files: [list]

---

### Step 2: Clippy Lints (cargo clippy)

Run clippy across entire workspace with warnings as errors:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**Success:** "No clippy warnings"
**Failure:** Group warnings by:
- **Errors** (must fix)
- **Warnings** (should fix)
- **Info** (optional)

**Report:**
- ✅ No clippy issues
- OR ❌ Found X issues across Y crates:
  - Crate breakdown: wkmp-ap (2), wkmp-ui (1), etc.
  - Top 3 most common issue types
  - Link to full output if >10 issues

**Filtering:**
- Skip pedantic lints unless explicitly enabled
- Focus on correctness, performance, and style issues

---

### Step 3: Build All Crates (cargo build)

Build entire workspace in debug mode:

```bash
cargo build --workspace --all-features
```

**Success:** "All crates built successfully"
**Failure:** Report first compilation error with context

**Report:**
- ✅ All 6 crates built successfully (X.Xs)
- OR ❌ Build failed in [crate]: [error summary]

**Performance:**
- Show total build time
- Flag if build time >60s (possible incremental cache issue)

---

### Step 4: Test All Crates (cargo test)

Run all tests across workspace:

```bash
cargo test --workspace --all-features
```

**Success:** "All tests passed"
**Failure:** Report failures by crate

**Report:**
- ✅ All tests passed (X tests, Y.Ys)
  - Breakdown: wkmp-ap (45), wkmp-ui (12), wkmp-common (23), etc.
- OR ❌ Test failures:
  - Failed tests by crate
  - First 3 failure messages
  - Total: X/Y tests passed

**Parallel Execution:**
- Tests run in parallel by default
- If deadlocks occur, suggest: `cargo test -- --test-threads=1`

---

### Step 5: Documentation Check (cargo doc)

Build documentation for all crates:

```bash
cargo doc --workspace --all-features --no-deps
```

**Success:** "Documentation built successfully"
**Failure:** Report documentation errors/warnings

**Report:**
- ✅ Documentation built (X warnings)
- OR ❌ Documentation errors: [summary]

**Common Issues:**
- Broken intra-doc links
- Missing documentation on public items
- Invalid markdown in doc comments

---

## Optimized Execution Strategy

### Parallel Execution

Run independent checks in parallel for speed:

**Phase 1 (Parallel):**
- Format check
- Clippy
- Documentation build

**Phase 2 (Sequential):**
- Build (required for tests)
- Tests (depends on successful build)

### Early Exit Strategy

**Stop on critical failures:**
- If build fails → skip tests (can't run without successful build)
- If format check fails → auto-fix and continue
- If clippy fails → continue to build/test (warnings don't block)

### Smart Caching

Leverage Cargo's incremental compilation:
- Checks use existing build artifacts when possible
- Clean build only if explicitly requested

---

## Output Format

### Console Output (To User)

**Summary Report:**
```
🔍 WKMP Multi-Crate Quality Check

✅ Format:  All files formatted correctly
⚠️  Clippy:  3 warnings in wkmp-ap, 1 in wkmp-ui
✅ Build:   All 6 crates built (12.3s)
✅ Tests:   95 tests passed (3.7s)
✅ Docs:    Documentation built (2 warnings)

Overall: 4/5 passed, 1 with warnings
```

**Detailed Findings (if issues exist):**
```
⚠️ Clippy Warnings (4 total):

wkmp-ap/src/crossfade.rs:
  - Line 45: needless_borrow
  - Line 78: redundant_clone

wkmp-ui/src/handlers.rs:
  - Line 120: unused_variable

Run `cargo clippy --fix` to auto-fix where possible.
```

### Exit Status

Return actionable next steps:
- ✅ **All checks passed** → "Ready to commit"
- ⚠️ **Warnings only** → "Review warnings, consider fixing before commit"
- ❌ **Errors exist** → "Fix errors before committing: [specific actions]"

---

## Extended Options

### Quick Check (Faster)

For rapid iteration, skip doc build and run only:
- Format check
- Clippy
- Tests (skip build, cargo test builds automatically)

**Usage:** Mention "quick check" to user

### Full Check (Comprehensive)

For pre-commit validation:
- All standard checks
- Release build: `cargo build --release --workspace`
- Benchmark compilation: `cargo bench --no-run`

**Usage:** Mention "full check" to user

### Per-Crate Check

Focus on specific crate:
```bash
cargo clippy -p wkmp-ap -- -D warnings
cargo test -p wkmp-ap
```

**Usage:** If user specifies crate name

---

## Performance Benchmarks

**Expected times (incremental build):**
- Format check: <1s
- Clippy: 5-15s
- Build: 10-30s
- Tests: 3-10s
- Docs: 5-10s
- **Total: 23-66s**

**If times exceed 2x expected:**
- Suggest: `cargo clean` and rebuild
- Check for background processes consuming CPU
- Verify incremental compilation enabled

---

## Error Handling

### Common Issues

**"could not compile X due to Y previous errors"**
- Show first error with file:line context
- Suggest: Check recent changes in that crate

**"test failed, to rerun pass `--test ...`"**
- Extract test name and failure reason
- Provide rerun command for just that test

**"error: linker `cc` not found"**
- Report: Missing build tools
- Suggest: Install build essentials

### Recovery Actions

**If cargo.lock conflicts:**
```bash
cargo update
cargo build --workspace
```

**If incremental cache corrupted:**
```bash
cargo clean
cargo build --workspace
```

---

## Integration with Other Workflows

**Before /commit:**
- Run `/check-all` to verify no regressions
- Fix any errors before committing
- Warnings acceptable if documented

**During implementation:**
- Run quick check frequently
- Run full check before marking increment complete

**Before pull requests:**
- Run full check (including release build)
- Ensure 0 clippy warnings (use -D warnings)

---

## Success Criteria

✅ All crates formatted correctly
✅ Clippy passes with 0 errors (warnings acceptable)
✅ All crates build successfully
✅ All tests pass
✅ Documentation builds without errors
✅ Results reported concisely to user
✅ Actionable next steps provided

---

**Expected runtime:**
- Quick check: 10-20s
- Standard check: 25-60s
- Full check: 60-120s

**Frequency:** Run before every commit, after every significant change
