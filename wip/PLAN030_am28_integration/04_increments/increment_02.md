# Increment 2: Constants & Configuration

**Estimated Effort:** 2 hours
**Dependencies:** Increment 1 (types)
**Deliverables:** matching/constants.rs, enhanced AlbumMatcherConfig

---

## Objective

Create centralized constants module and extend AlbumMatcherConfig to make am28 parameters configurable.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| constants.rs | 408 | Adapt to library |

---

## Tasks

### 2.1 Create matching/constants.rs

```rust
//! Album Matching Constants
//!
//! Default parameters for the am28 album matching algorithm.

/// Default silence threshold (dB) for track boundary detection
pub const DEFAULT_THRESHOLD_DB: f64 = -48.0;

/// Default minimum silence duration (seconds)
pub const DEFAULT_MIN_DURATION_SECS: f64 = 0.5;

/// Track match tolerance (seconds)
pub const MATCH_TOLERANCE_SECS: f64 = 3.0;

/// Minimum artist similarity (Jaro-Winkler) to accept match
pub const MIN_ARTIST_SIMILARITY: f64 = 0.50;

/// Stage 4 penalty percentage (quiet spot detection less reliable)
pub const STAGE4_PENALTY_PERCENT: f64 = 25.0;

/// Early-exit grace period after 100% match found (seconds)
pub const EARLY_EXIT_GRACE_PERIOD_SECS: u64 = 20;

/// MusicBrainz API rate limit (milliseconds between requests)
pub const MB_RATE_LIMIT_MS: u64 = 1000;

/// Parameter grid: threshold values (dB)
/// 12 values from -42 to -66 in 2dB steps
pub const THRESHOLD_VALUES: [f64; 12] = [
    -42.0, -44.0, -46.0, -48.0, -50.0, -52.0,
    -54.0, -56.0, -58.0, -60.0, -62.0, -66.0,
];

/// Parameter grid: minimum duration values (seconds)
/// 15 values from 0.1 to 2.0
pub const MIN_DURATION_VALUES: [f64; 15] = [
    0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
    1.2, 1.4, 1.6, 1.8, 2.0,
];

/// Total parameter combinations (12 × 15 = 180)
pub const TOTAL_PARAM_COMBINATIONS: usize =
    THRESHOLD_VALUES.len() * MIN_DURATION_VALUES.len();
```

### 2.2 Extend AlbumMatcherConfig

Update `matching/album_matcher.rs`:

```rust
/// Configuration for album matching
#[derive(Debug, Clone)]
pub struct AlbumMatcherConfig {
    // Existing fields
    pub default_threshold_db: f64,
    pub default_min_duration_secs: f64,
    pub match_tolerance_secs: f64,
    pub min_artist_similarity: f64,
    pub enable_cache: bool,
    pub cache_dir: Option<PathBuf>,

    // NEW: Stage configuration
    pub enable_stage3: bool,          // Over-segmentation assembly
    pub enable_stage4: bool,          // Quiet spot detection
    pub enable_stage5: bool,          // Extra track merging
    pub stage4_penalty_percent: f64,  // Penalty for Stage 4 results

    // NEW: Early-exit configuration
    pub enable_early_exit: bool,
    pub early_exit_grace_secs: u64,

    // NEW: Parameter grid (optional override)
    pub threshold_values: Option<Vec<f64>>,
    pub min_duration_values: Option<Vec<f64>>,
}

impl Default for AlbumMatcherConfig {
    fn default() -> Self {
        Self {
            default_threshold_db: DEFAULT_THRESHOLD_DB,
            default_min_duration_secs: DEFAULT_MIN_DURATION_SECS,
            match_tolerance_secs: MATCH_TOLERANCE_SECS,
            min_artist_similarity: MIN_ARTIST_SIMILARITY,
            enable_cache: true,
            cache_dir: None,
            enable_stage3: true,
            enable_stage4: true,
            enable_stage5: true,
            stage4_penalty_percent: STAGE4_PENALTY_PERCENT,
            enable_early_exit: true,
            early_exit_grace_secs: EARLY_EXIT_GRACE_PERIOD_SECS,
            threshold_values: None,
            min_duration_values: None,
        }
    }
}

impl AlbumMatcherConfig {
    /// Get threshold values (custom or default)
    pub fn threshold_values(&self) -> &[f64] {
        self.threshold_values.as_deref()
            .unwrap_or(&THRESHOLD_VALUES)
    }

    /// Get min duration values (custom or default)
    pub fn min_duration_values(&self) -> &[f64] {
        self.min_duration_values.as_deref()
            .unwrap_or(&MIN_DURATION_VALUES)
    }

    /// Total parameter combinations
    pub fn total_combinations(&self) -> usize {
        self.threshold_values().len() * self.min_duration_values().len()
    }
}
```

### 2.3 Add Module to matching/mod.rs

```rust
pub mod constants;
pub use constants::*;
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-002-01 | Default threshold values in valid range | All between -70 and -30 dB |
| TC-U-002-02 | Parameter grid generates 180 combinations | 12 × 15 = 180 |
| TC-U-002-03 | AlbumMatcherConfig builder pattern | Custom values override defaults |

---

## Acceptance Criteria

- [ ] constants.rs created with all am28 constants
- [ ] AlbumMatcherConfig extended with new fields
- [ ] Default implementation uses constants
- [ ] `cargo check` passes
- [ ] All 3 tests pass
