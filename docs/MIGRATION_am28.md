# am28 Migration Guide

This document describes the migration of album matching functionality from
the `examples/am28` prototype to the `wkmp-ai` library.

## Overview

The am28 prototype has been refactored into the library's `matching` module
with the following improvements:

- Cleaner module structure with separation of concerns
- Better error handling with typed errors
- Async/await integration with tokio
- Configuration via `AlbumMatcherConfig` instead of hardcoded constants
- Parallel decode + MusicBrainz lookup using `tokio::join!`
- Proper library API with public exports

## Module Mapping

| am28 File | Library Module |
|-----------|----------------|
| `types.rs` | `matching/types.rs` |
| `constants.rs` | `matching/constants.rs` |
| `metadata.rs` | `matching/metadata.rs` |
| `silence_detection.rs` | `matching/silence_detection.rs` |
| `single_track_discriminator.rs` | `matching/single_track.rs` |
| `stages/stage2.rs` | `matching/stages/stage2.rs` |
| `stages/stage3.rs` | `matching/stages/stage3.rs` |
| `stages/stage4.rs` | `matching/stages/stage4.rs` |
| `stages/stage5.rs` | `matching/stages/stage5.rs` |
| `orchestration.rs` | `matching/orchestrator.rs` |
| `editions.rs` | `matching/editions.rs` |
| `musicbrainz/` | `services/musicbrainz_client.rs` |
| `main.rs` (core logic) | `matching/album_matcher.rs` |

## API Changes

### Before (am28)

```rust
// Direct function calls in main loop
let silence_cache = precompute_silence_cache(&samples, sample_rate, &THRESHOLD_VALUES, &MIN_DURATION_VALUES);
let result = test_single_edition(edition_idx, &edition, &silence_cache, ...);
```

### After (Library)

```rust
use wkmp_ai::matching::{AlbumMatcher, AlbumMatcherConfig};

// Configured service with proper API
let config = AlbumMatcherConfig {
    match_tolerance_secs: 5.0,  // Customizable
    enable_early_exit: true,
    ..Default::default()
};
let matcher = AlbumMatcher::with_config(config)?;

let result = matcher.match_album(
    &file_path,
    Some("Artist Name"),  // Optional hints
    Some("Album Name"),
).await?;

println!("Matched: {}", result.matched);
println!("Stage: {:?}", result.matching_stage);
```

## Key API Differences

| Feature | am28 | Library |
|---------|------|---------|
| Construction | N/A | `AlbumMatcher::new()` or `with_config()` |
| Configuration | Constants | `AlbumMatcherConfig` struct |
| Execution | Sync with rayon | Async with tokio |
| Error handling | Strings/panics | `AlbumMatchError` enum |
| Result type | Custom struct | `AlbumMatchResult` |
| Parallelism | rayon::spawn | tokio::join! for decode+MB |

## Configuration Migration

### Constants (am28)

```rust
// constants.rs
pub const MATCH_TOLERANCE_SECS: f64 = 10.0;
pub const MIN_ARTIST_SIMILARITY: f64 = 0.50;
pub const EARLY_EXIT_GRACE_PERIOD_SECS: u64 = 20;
```

### Configuration (Library)

```rust
let config = AlbumMatcherConfig {
    match_tolerance_secs: 10.0,
    min_artist_similarity: 0.50,
    enable_early_exit: true,
    early_exit_grace_secs: 20,
    enable_stage3: true,
    enable_stage4: true,
    enable_stage5: true,
    stage4_penalty_percent: 25.0,
    threshold_values: None,  // Uses defaults
    min_duration_values: None,  // Uses defaults
    ..Default::default()
};
```

## Result Type Changes

### am28 Result

```rust
// Various structs across files
struct CandidateTestResult { ... }
struct EditionTestResult { ... }
```

### Library Result

```rust
pub struct AlbumMatchResult {
    pub matched: bool,
    pub release_mbid: Option<String>,
    pub matched_artist: Option<String>,
    pub matched_album: Option<String>,
    pub matching_stage: Option<MatchingStage>,
    pub match_percentage: f64,
    pub mean_error_seconds: f64,
    pub matched_track_count: usize,
    pub expected_track_count: usize,
    pub detected_track_count: usize,
    pub tracks: Vec<MatchedTrack>,
    pub confidence: String,
    pub artist_verified: bool,
    pub artist_similarity: f64,
    pub best_threshold_db: Option<f64>,
    pub best_min_duration_secs: Option<f64>,
    pub status: String,
}
```

## Features Not Migrated

The following am28 features are NOT migrated to the library:

1. **CLI interface** - Library only; CLI can wrap if needed
2. **Batch processing loop** - Caller handles iteration
3. **Progress reporting to stdout** - Use tracing instead
4. **AcoustID lookup** - Deferred to future enhancement
5. **JSON result files** - Caller handles serialization

## Keeping am28 Example

The `examples/am28` directory is retained for:

- Reference implementation and testing
- Demonstration of library internals
- Direct command-line usage for development

To run the am28 example:

```bash
cargo run --example am28 -- /path/to/albums
```

## Migration Checklist

For code using am28 directly:

- [ ] Replace direct function calls with `AlbumMatcher` service
- [ ] Update imports from `am28::*` to `wkmp_ai::matching::*`
- [ ] Convert sync code to async with `tokio::main`
- [ ] Replace hardcoded constants with `AlbumMatcherConfig`
- [ ] Update error handling from strings to `AlbumMatchError`
- [ ] Update result handling to use `AlbumMatchResult` fields

## Performance Notes

The library version maintains am28's performance characteristics:

- **Parallel decode + MusicBrainz**: Uses `tokio::join!` (was separate in am28)
- **180-parameter grid search**: Unchanged
- **Early-exit optimization**: Configurable via `enable_early_exit`
- **Grace period**: Configurable via `early_exit_grace_secs`

## Testing

Run library tests:

```bash
cargo test --lib matching::
```

Run integration tests:

```bash
cargo test --test album_matcher_integration
```

Run with audio fixtures (when available):

```bash
cargo test --test album_matcher_integration -- --ignored
```
