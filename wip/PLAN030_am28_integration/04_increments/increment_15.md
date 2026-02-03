# Increment 15: Documentation & Cleanup

**Estimated Effort:** 3 hours
**Dependencies:** Increments 1-14
**Deliverables:** API docs, migration guide, cleanup

---

## Objective

Complete documentation, clean up deprecated code, and prepare for release.

---

## Tasks

### 15.1 API Documentation

Update module-level documentation in `src/matching/mod.rs`:

```rust
//! Album Matching Module
//!
//! Multi-stage algorithm for identifying full-album audio files against
//! MusicBrainz releases.
//!
//! # Overview
//!
//! The album matcher uses a four-stage approach to match concatenated album
//! recordings to their source releases:
//!
//! - **Stage 2**: Parameter grid search - tests 180 combinations of silence
//!   detection parameters against candidate editions
//! - **Stage 3**: Over-segmentation assembly - uses dynamic programming to
//!   merge over-detected track boundaries
//! - **Stage 4**: Quiet spot detection - RMS-based boundary detection for
//!   albums without clear silences
//! - **Stage 5**: Extra track merging - handles bonus/hidden tracks by
//!   merging final segments
//!
//! # Usage
//!
//! ```rust,no_run
//! use wkmp_ai::services::{AlbumMatcher, AlbumMatcherConfig};
//! use wkmp_ai::services::MusicBrainzClient;
//! use std::path::Path;
//!
//! async fn match_album() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = AlbumMatcherConfig::default();
//!     let mb_client = MusicBrainzClient::new("MyApp/1.0");
//!     let matcher = AlbumMatcher::new(config, mb_client);
//!
//!     let result = matcher.match_album(Path::new("album.mp3")).await?;
//!
//!     println!("Content type: {:?}", result.content_type);
//!     println!("Match percentage: {:?}", result.match_percentage);
//!     println!("Artist verified: {}", result.artist_verified);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Configuration
//!
//! Key configuration options in [`AlbumMatcherConfig`]:
//!
//! | Parameter | Default | Description |
//! |-----------|---------|-------------|
//! | `min_match_percentage` | 80.0 | Minimum % of tracks to match |
//! | `tolerance_secs` | 3.0 | Track duration tolerance |
//! | `min_artist_similarity` | 0.50 | Jaro-Winkler threshold |
//! | `max_editions` | 20 | Maximum editions to test |
//! | `early_exit` | true | Stop on 100% match |
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     AlbumMatcher                        │
//! ├─────────────────────────────────────────────────────────┤
//! │  1. Extract metadata (ID3/filename)                     │
//! │  2. Pre-decode single-track check                       │
//! │  3. Decode audio to PCM                                 │
//! │  4. Post-decode single-track check                      │
//! │  5. Search MusicBrainz                                  │
//! │  6. Group releases into editions                        │
//! │  7. Run multi-stage matching                            │
//! │  8. Verify artist match                                 │
//! │  9. Return ClassificationResult                         │
//! └─────────────────────────────────────────────────────────┘
//! ```

pub mod constants;
pub mod types;
pub mod metadata;
pub mod silence_detection;
pub mod single_track;
pub mod editions;
pub mod stages;
pub mod orchestrator;
pub mod validation;

pub use types::*;
pub use constants::*;
```

### 15.2 Migration Guide

Create `docs/MIGRATION_am28.md`:

```markdown
# am28 Migration Guide

This document describes the migration of album matching functionality from
the `examples/am28` prototype to the `wkmp-ai` library.

## Overview

The am28 prototype (~8,800 lines) has been refactored into the library's
`matching` module (~3,500 lines) with the following improvements:

- Cleaner module structure
- Better error handling
- Async/await integration
- Configuration via struct instead of constants
- Proper library API

## Module Mapping

| am28 File | Library Module |
|-----------|----------------|
| `types.rs` | `matching/types.rs` |
| `constants.rs` | `matching/constants.rs` |
| `metadata.rs` | `matching/metadata.rs` |
| `silence_detection.rs` | `matching/silence_detection.rs` |
| `single_track_discriminator.rs` | `matching/single_track.rs` |
| `matching/edition.rs` | `matching/editions/` |
| `stages/stage2.rs` | `matching/stages/stage2.rs` |
| `stages/stage3.rs` | `matching/stages/stage3.rs` |
| `stages/stage4.rs` | `matching/stages/stage4.rs` |
| `stages/stage5.rs` | `matching/stages/stage5.rs` |
| `orchestration.rs` | `matching/orchestrator.rs` |
| `musicbrainz/` | `services/musicbrainz/` (extended) |
| `main.rs` | `services/album_matcher.rs` |

## API Changes

### Before (am28)

```rust
// Direct function calls with hardcoded constants
let result = process_single_album(&file_path, &mb_api_key, &acoustid_key).await?;
```

### After (Library)

```rust
// Configured service with proper API
let config = AlbumMatcherConfig {
    min_match_percentage: 90.0,  // Customizable
    ..Default::default()
};
let matcher = AlbumMatcher::new(config, mb_client);
let result = matcher.match_album(&file_path).await?;
```

## Deprecated am28 Features

The following am28 features are NOT migrated:

1. **CLI interface** - Library only, CLI can wrap if needed
2. **Batch processing loop** - Caller handles iteration
3. **Progress reporting to stdout** - Use tracing instead
4. **AcoustID placeholder** - Deferred to future enhancement

## Keeping am28 Example

The `examples/am28` directory is retained for:
- Reference implementation
- Testing against library version
- Demonstration of library usage

To run the example:
```bash
cargo run --example am28 -- /path/to/albums
```
```

### 15.3 Cleanup Tasks

```rust
// Checklist for cleanup

// [ ] Remove duplicate type definitions from am28/types.rs that are now in library
// [ ] Update am28/main.rs to use library types where possible
// [ ] Add deprecation notices to am28 modules that are fully migrated
// [ ] Run cargo clippy and fix warnings
// [ ] Run cargo fmt
// [ ] Verify all tests pass
// [ ] Update CHANGELOG.md
```

### 15.4 Final Verification

```bash
# Full verification script
#!/bin/bash

echo "=== Verification ==="

# 1. Compile check
cargo check --all-targets
echo "✓ Compilation successful"

# 2. All tests
cargo test --lib
echo "✓ Library tests pass"

# 3. Documentation
cargo doc --no-deps
echo "✓ Documentation builds"

# 4. Clippy
cargo clippy -- -D warnings
echo "✓ No clippy warnings"

# 5. Format check
cargo fmt --check
echo "✓ Code formatted"

echo "=== Verification Complete ==="
```

---

## Deliverables Checklist

| Item | Location | Status |
|------|----------|--------|
| Module documentation | `src/matching/mod.rs` | [ ] |
| API documentation | All public items | [ ] |
| Migration guide | `docs/MIGRATION_am28.md` | [ ] |
| Deprecated code cleanup | Various | [ ] |
| CHANGELOG update | `CHANGELOG.md` | [ ] |
| Final verification | Pass all checks | [ ] |

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-S-015-01 | Documentation builds | `cargo doc` succeeds |
| TC-S-015-02 | All tests pass | `cargo test` green |
| TC-S-015-03 | No clippy warnings | `cargo clippy` clean |

---

## Acceptance Criteria

- [ ] API documentation complete
- [ ] Migration guide written
- [ ] Deprecated code cleaned up
- [ ] All verification checks pass
- [ ] All 3 tests pass
