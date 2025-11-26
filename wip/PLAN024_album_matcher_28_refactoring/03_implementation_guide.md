# Implementation Guide: Album Matcher 28 Refactoring

**Plan:** PLAN024
**Specification:** SPEC_album_matcher_28_refactoring.md
**Phase:** 3 - Implementation Patterns
**Date:** 2025-11-25
**Status:** Phase 3 - Implementation Guide

---

## Purpose

This guide documents **implementation patterns and conventions** to address the 4 MEDIUM-priority gaps identified in Phase 2 completeness analysis.

---

## 1. Error Handling Strategy

**Gap Addressed:** GAP-FUNC-01 (Error Handling Strategy)

### Standard Pattern

**All modules SHALL use `anyhow::Result<T>` for error handling.**

```rust
use anyhow::{Result, Context};

pub fn run_stage2(/* params */) -> Result<SingleEditionStage2Results> {
    // Function body
    let profile = compute_profile(samples)
        .context("Failed to compute WindowDbProfile")?;

    // ...

    Ok(results)
}
```

### Error Propagation

**Errors are propagated to main.rs orchestration level using `?` operator:**

```rust
// In module function
pub fn process_edition(/* params */) -> Result<CandidateTestResult> {
    let stage2_result = run_stage2(params)?;  // Propagate errors up
    let stage3_result = run_stage3(stage2_result)?;
    // ...
}
```

### Logging Errors

**Errors are logged at main.rs orchestration level, NOT within modules:**

```rust
// main.rs
for album in albums {
    match process_album(&album) {
        Ok(result) => {
            info!("[A{}] SUCCESS: Matched {} tracks", album.id, result.matched);
        }
        Err(e) => {
            error!("[A{}] FAILED: {:?}", album.id, e);
            // Continue processing next album
        }
    }
}
```

### Context Annotations

**Add context at module boundaries for better error messages:**

```rust
pub fn decode_audio(file_path: &Path) -> Result<Vec<f32>> {
    symphonia::decode(file_path)
        .context(format!("Failed to decode audio file: {}", file_path.display()))?;
    // ...
}
```

### Conventions

| Situation | Pattern | Example |
|-----------|---------|---------|
| I/O errors | Propagate with context | `.context("Failed to read cache file")?` |
| Algorithm failures | Propagate with context | `.context("DP assembly failed - no valid path")?` |
| User input errors | Return specific error | `anyhow::bail!("Invalid cache mode: {}", mode)` |
| Expected failures | Log warning, continue | `warn!("Album {} skipped - single track detected")` |

### Anti-Patterns (Avoid)

❌ **Don't:** Log errors within modules
```rust
// BAD: Logging in module
pub fn run_stage2() -> Result<_> {
    if error {
        error!("Stage 2 failed");  // NO - Let caller log
        return Err(/* ... */);
    }
}
```

✅ **Do:** Log errors at orchestration level
```rust
// GOOD: Logging in main.rs
match run_stage2(params) {
    Ok(result) => { /* ... */ }
    Err(e) => {
        error!("[A{}] Stage 2 failed: {:?}", album_id, e);  // YES
    }
}
```

---

## 2. CLI Wrapper Mechanism

**Gap Addressed:** GAP-FUNC-03 (CLI Wrapper Mechanism)

### Wrapper File Structure

**File:** `wkmp-ai/examples/album_matcher_28.rs` (entry point for Cargo)

```rust
//! Album Matcher 28 - Refactored Modular Version
//!
//! This is a minimal wrapper that delegates to the modular implementation
//! in the am28/ folder.
//!
//! Usage: cargo run --example album_matcher_28 -- [OPTIONS]

mod am28;

fn main() -> anyhow::Result<()> {
    am28::main()
}
```

**That's it.** The wrapper is minimal - just module declaration and delegation.

### Main Module Structure

**File:** `wkmp-ai/examples/am28/main.rs` (actual implementation)

```rust
//! # Album Matcher 28 - Main Entry Point
//!
//! CLI parsing, orchestration, and top-level error handling.

use anyhow::Result;
use clap::Parser;
use tracing::{info, error};

// Module declarations
mod types;
mod constants;
mod silence_detection;
mod musicbrainz;
mod stages;
mod matching;
mod utils;

/// Album Matcher CLI Arguments
#[derive(Parser, Debug)]
#[clap(name = "album_matcher_28")]
struct Args {
    /// Training set file path
    #[clap(long, value_name = "FILE")]
    training_set: String,

    /// Output file path
    #[clap(long, value_name = "FILE", default_value = "album_matcher_output.txt")]
    output: String,

    /// Limit number of albums processed
    #[clap(long, value_name = "N")]
    limit: Option<usize>,

    /// Cache mode: disabled, readonly, readwrite
    #[clap(long, value_name = "MODE", default_value = "readwrite")]
    cache_mode: String,

    // ... other arguments
}

pub fn main() -> Result<()> {
    // 1. Initialize tracing
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .init();

    // 2. Parse CLI arguments
    let args = Args::parse();

    // 3. Orchestrate album processing
    process_albums(args)?;

    Ok(())
}

fn process_albums(args: Args) -> Result<()> {
    // Album loop orchestration
    // ...
}
```

### Module Visibility

**Visibility rules for am28/ modules:**

| Module | Visibility | Rationale |
|--------|-----------|-----------|
| `am28::main()` | `pub` | Called by wrapper |
| `am28::types::*` | `pub(crate)` | Shared within am28/, not exposed externally |
| `am28::stages::*` | `pub(crate)` | Internal to am28/ |
| `am28::matching::*` | `pub(crate)` | Internal to am28/ |
| Helper functions | `pub(crate)` or private | Internal only |

**Rationale:** Minimize public API surface. Only `main()` needs to be `pub`.

---

## 3. Mutable State Handling

**Gap Addressed:** GAP-FUNC-04 (Mutable State Handling)

### Heartbeat Thread Pattern

**Heartbeat thread is launched in main.rs, channel passed to timing utilities:**

```rust
// main.rs
use tokio::sync::mpsc;

pub fn main() -> Result<()> {
    // ...

    // Launch heartbeat thread (if not disabled)
    let (heartbeat_tx, mut heartbeat_rx) = mpsc::channel(1);
    let heartbeat_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            info!("Heartbeat: Still processing...");
        }
    });

    // Pass heartbeat_tx to album processing if needed
    process_albums(args, heartbeat_tx)?;

    // Cleanup on exit
    drop(heartbeat_tx);
    heartbeat_handle.abort();

    Ok(())
}
```

### Progress Counters Pattern

**Progress counters passed as function parameters, NOT global state:**

```rust
// main.rs
fn process_albums(args: Args) -> Result<()> {
    let mut success_count = 0;
    let mut failure_count = 0;

    for (idx, album_path) in albums.iter().enumerate() {
        match process_album(album_path, idx + 1, albums.len()) {
            Ok(result) => {
                success_count += 1;
                info!("[A{}] SUCCESS ({}/{})", result.id, success_count, albums.len());
            }
            Err(e) => {
                failure_count += 1;
                error!("[A{}] FAILED ({}/{}): {:?}", album_id, failure_count, albums.len(), e);
            }
        }
    }

    // Final summary
    info!("Completed: {} successes, {} failures", success_count, failure_count);

    Ok(())
}
```

### Shared State Pattern

**For MusicBrainz API rate limiting (existing pattern from album_matcher_28.rs):**

```rust
// musicbrainz/api.rs
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct MusicBrainzClient {
    client: reqwest::Client,
    rate_limiter: Arc<Semaphore>,
    min_interval: Duration,
}

impl MusicBrainzClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            rate_limiter: Arc::new(Semaphore::new(1)),  // Only 1 request at a time
            min_interval: Duration::from_secs(2),       // 2-second interval
        }
    }

    pub async fn search(&self, query: &str) -> Result<MBSearchResponse> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await?;

        // Make API call
        let response = self.client.get(&url).send().await?;

        // Sleep to enforce minimum interval
        tokio::time::sleep(self.min_interval).await;

        // Parse and return
        Ok(response.json().await?)
    }
}
```

### State Management Principles

| State Type | Pattern | Example |
|------------|---------|---------|
| **Progress tracking** | Function parameters | `success_count`, `album_index` |
| **Configuration** | Struct fields | `MusicBrainzClient { min_interval }` |
| **Concurrency control** | `Arc<Semaphore>` | MB API rate limiting |
| **Thread communication** | `tokio::sync::mpsc` | Heartbeat thread |
| **Global constants** | `const` in constants.rs | `STAGE2_THRESHOLD_VALUES` |

### Anti-Patterns (Avoid)

❌ **Don't:** Use global mutable state
```rust
// BAD: Global mutable
static mut SUCCESS_COUNT: usize = 0;  // NO
```

✅ **Do:** Pass state as parameters
```rust
// GOOD: Explicit parameters
fn process_album(/* ... */, success_count: &mut usize) -> Result<_> {
    // ...
    *success_count += 1;
}
```

---

## 4. Async/Sync Boundary Documentation

**Gap Addressed:** GAP-INTF-03 (Async/Sync Boundary)

### Async Functions

**Functions that MUST be async (based on album_matcher_28.rs):**

| Function | Location | Reason |
|----------|----------|--------|
| `main()` | am28/main.rs | Tokio runtime (`#[tokio::main]`) |
| `search_releases()` | musicbrainz/api.rs | HTTP client (reqwest async) |
| `fetch_release_details()` | musicbrainz/api.rs | HTTP client (reqwest async) |
| Process album loop | main.rs | Calls async MB API functions |

**Pattern:**
```rust
// main.rs
#[tokio::main]
pub async fn main() -> Result<()> {
    // ...
    process_albums(args).await?;
    Ok(())
}

async fn process_albums(args: Args) -> Result<()> {
    for album in albums {
        // Async calls to MusicBrainz API
        let editions = mb_client.search(&query).await?;
        // ...
    }
    Ok(())
}
```

### Sync Functions

**Functions that are sync (no `async` keyword):**

| Function | Location | Reason |
|----------|----------|--------|
| `compute_profile()` | silence_detection.rs | CPU-bound, no I/O |
| `detect_silence()` | silence_detection.rs | CPU-bound algorithm |
| `run_stage2()` | stages/stage2.rs | CPU-bound (uses pre-computed profile) |
| `run_stage3()` | stages/stage3.rs | CPU-bound (DP assembly) |
| `run_stage4()` | stages/stage4.rs | CPU-bound (RMS profiling) |
| `run_stage5()` | stages/stage5.rs | CPU-bound (track merging) |
| `test_candidate()` | matching/candidate.rs | CPU-bound (percentage calculation) |
| `decode_audio()` | utils/audio.rs | Blocking I/O (symphonia sync API) |

**Pattern:**
```rust
// silence_detection.rs
pub fn compute_profile(samples: &[f32], window_size: usize) -> WindowDbProfile {
    // Sync implementation
    // ...
}
```

### Async/Sync Interaction

**Calling sync from async:**
```rust
// main.rs (async)
async fn process_album(album_path: &Path) -> Result<AlbumResult> {
    // 1. Decode audio (sync, blocking I/O)
    let samples = utils::audio::decode_audio(album_path)?;

    // 2. Fetch MusicBrainz data (async, HTTP I/O)
    let editions = mb_client.search(&query).await?;

    // 3. Run stages (sync, CPU-bound)
    let stage2_result = stages::run_stage2(&samples, &editions)?;
    let stage3_result = stages::run_stage3(&stage2_result)?;

    Ok(result)
}
```

**Blocking I/O in async context:**
If audio decoding takes too long, consider:
```rust
// Optional: Offload blocking I/O to thread pool
let samples = tokio::task::spawn_blocking(|| {
    utils::audio::decode_audio(album_path)
}).await??;
```

**However,** for this refactoring, preserve exact async/sync boundaries from album_matcher_28.rs (no optimization changes).

### Async Boundaries Summary

```
main() [ASYNC]
 ├─ process_albums() [ASYNC]
 │   ├─ decode_audio() [SYNC] - blocking I/O
 │   ├─ mb_client.search() [ASYNC] - HTTP I/O
 │   ├─ run_stage2() [SYNC] - CPU-bound
 │   ├─ run_stage3() [SYNC] - CPU-bound
 │   ├─ run_stage4() [SYNC] - CPU-bound
 │   └─ run_stage5() [SYNC] - CPU-bound
 └─ heartbeat_thread [ASYNC] - background task
```

---

## 5. Module Organization Conventions

### File Naming

**Rust module naming conventions:**
- Use snake_case for module names: `silence_detection.rs`
- Match folder name: `musicbrainz/` → `mod musicbrainz;`
- Use `mod.rs` for folder modules: `musicbrainz/mod.rs`

### Module Declarations

**In am28/mod.rs:**
```rust
//! # Album Matcher 28 - Modular Implementation
//!
//! Refactored version of album_matcher_28.rs with clear module boundaries.

pub mod main;  // Public (called by wrapper)

pub(crate) mod types;
pub(crate) mod constants;
pub(crate) mod silence_detection;
pub(crate) mod musicbrainz;
pub(crate) mod stages;
pub(crate) mod matching;
pub(crate) mod utils;

// Re-export main for convenience
pub use main::main;
```

**In am28/main.rs:**
```rust
use crate::types::*;
use crate::constants::*;
use crate::silence_detection;
use crate::musicbrainz;
use crate::stages;
use crate::matching;
use crate::utils;
```

### Subfolder Modules

**In am28/stages/mod.rs:**
```rust
//! # Matching Stages (Stage 2-5)
//!
//! Five-stage album matching pipeline.

pub(crate) mod stage2;
pub(crate) mod stage3;
pub(crate) mod stage4;
pub(crate) mod stage5;

// Re-export key functions
pub use stage2::run_stage2_single_edition_cached;
pub use stage3::run_stage3_single_edition;
pub use stage4::run_stage4_single_edition;
pub use stage5::run_stage5_single_edition;
```

**In am28/main.rs (using stages):**
```rust
use crate::stages::{
    run_stage2_single_edition_cached,
    run_stage3_single_edition,
    run_stage4_single_edition,
    run_stage5_single_edition,
};
```

---

## 6. Code Extraction Checklist

**When extracting code from album_matcher_28.rs to a module:**

### Pre-Extraction

- [ ] Identify function boundaries (start/end of function)
- [ ] Identify dependencies (what functions/types it uses)
- [ ] Note visibility (pub, pub(crate), or private)
- [ ] Check for state dependencies (global variables, mutable state)

### Extraction

- [ ] Copy function to target module
- [ ] Add necessary `use` statements
- [ ] Adjust visibility (`pub(crate)` for most internal functions)
- [ ] Preserve all comments (especially complex algorithm comments)
- [ ] Verify function signature unchanged

### Post-Extraction

- [ ] Update main.rs to use new module path
- [ ] Remove old function from album_matcher_28.rs (or comment out)
- [ ] Compile: `cargo check --example album_matcher_28`
- [ ] Verify no warnings from extracted module
- [ ] Run 10-album subset test to verify functionality

### Verification

- [ ] Function compiles without errors
- [ ] Function compiles without warnings
- [ ] Test output matches baseline (if testable in isolation)
- [ ] Document what was extracted in phase notes

---

## 7. Common Patterns from album_matcher_28.rs

### Pattern 1: Album ID Logging

**Preserve album ID prefix in all log messages:**

```rust
info!("[A{}] Decoding audio: {}", album_id, file_path.display());
info!("[A{}] MusicBrainz search: {} results", album_id, results.len());
info!("[A{}] Stage 2: Testing parameter {}/180", album_id, param_idx);
info!("[A{}] SUCCESS: Matched {}/{} tracks ({}%)", album_id, matched, total, percentage);
```

**Rationale:** Allows filtering logs by album, matches existing format.

### Pattern 2: Progress Reporting

**Report progress every N albums:**

```rust
if (album_idx + 1) % 10 == 0 {
    info!("Progress: Processed {}/{} albums ({} successes, {} failures)",
          album_idx + 1, total_albums, success_count, failure_count);
}
```

### Pattern 3: Parameter Iteration

**Preserve exact iteration order (critical for early-exit):**

```rust
// Iterate outer loop (thresholds) first, then inner loop (min_durations)
for threshold in STAGE2_THRESHOLD_VALUES.iter() {
    for min_duration in STAGE2_MIN_DURATION_VALUES.iter() {
        // Test this parameter combination
        let result = test_parameters(*threshold, *min_duration, &profile)?;

        // Early-exit on 100% match
        if result.percentage >= 100.0 {
            return Ok(result);
        }
    }
}
```

---

## 8. Testing During Extraction

### Subset Testing Script

**Create test script for quick verification:**

```powershell
# test_subset.ps1
# Run refactored version on first 10 albums

cargo run --release --example album_matcher_28 -- `
    --training-set training_set.txt `
    --limit 10 `
    --output test_subset_output.txt `
    --cache-mode readonly

# Compare with baseline
python analyze_best_params.py test_subset_output.txt
```

**Run after each extraction phase.**

### Diff Checking

**Compare outputs:**

```bash
# Extract structured data
python analyze_best_params.py album_matcher_output_run28.txt > baseline.json
python analyze_best_params.py test_subset_output.txt > refactored.json

# Compare JSON
diff baseline.json refactored.json
```

**Expected:** No differences for functional correctness.

---

## 9. Implementation Phases (Reminder)

**Reference from specification:**

| Phase | Milestone | Verification |
|-------|-----------|--------------|
| Phase 1 | Structure setup | TEST-STRUCT-001 |
| Phase 2 | Types and constants | TEST-FUNC-002a/b, TEST-STRUCT-006 |
| Phase 3 | Core modules | TEST-BUILD-001, TEST-STRUCT-004 |
| Phase 4 | Stage modules | TEST-FUNC-005a/b |
| Phase 5 | Matching and orchestration | TEST-FUNC-005c, TEST-STRUCT-002a/b |
| Phase 6 | Final verification | All remaining tests |

---

## 10. Quick Reference

### Import Conventions

```rust
// Standard library
use std::path::Path;
use std::time::{Duration, Instant};

// External crates
use anyhow::{Result, Context};
use tokio::sync::Semaphore;
use tracing::{info, debug, warn, error};

// Internal modules (crate-relative paths)
use crate::types::*;
use crate::constants::*;
use crate::silence_detection::WindowDbProfile;
```

### Error Handling Template

```rust
pub fn my_function(params: Params) -> Result<Output> {
    let step1 = compute_something(params)
        .context("Step 1 failed")?;

    let step2 = process_data(step1)
        .context("Step 2 failed")?;

    Ok(step2)
}
```

### Module Documentation Template

```rust
//! # Module Name
//!
//! Brief description of module purpose.
//!
//! ## Key Types
//! - `TypeName`: Description
//!
//! ## Key Functions
//! - `function_name()`: Description
//!
//! ## Related Modules
//! - `other::module`: How it relates
```

---

## Document Control

**Version:** 1.0
**Status:** Phase 3 - Implementation Guide
**Created:** 2025-11-25
**Last Updated:** 2025-11-25

**Dependencies:**
- Upstream: 01_PHASE2_COMPLETENESS_ANALYSIS.md (gaps identified)
- Downstream: Implementation (Phase 4+)
