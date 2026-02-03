# Album Matcher 28 - Modular Implementation

**Status:** Phase 4 - Structure Setup Complete

This folder contains the refactored modular version of `album_matcher_28.rs`.

---

## Module Structure

```
am28/
├── main.rs                    # Entry point, CLI parsing, orchestration
├── mod.rs                     # Module declarations
├── types.rs                   # Shared data structures
├── constants.rs               # Configuration constants (STAGE2 arrays)
├── silence_detection.rs       # Silence detection, WindowDbProfile
├── musicbrainz/              # MusicBrainz API client and caching
│   ├── mod.rs
│   ├── api.rs                # HTTP client, rate limiting
│   ├── cache.rs              # File-based cache (3 modes)
│   └── types.rs              # MB-specific types
├── stages/                   # Matching stages (Stages 2-5)
│   ├── mod.rs
│   ├── stage2.rs             # 180-parameter sweep with early-exit
│   ├── stage3.rs             # Dynamic programming assembly
│   ├── stage4.rs             # Edition-guided quiet spot detection (RMS)
│   └── stage5.rs             # Extra track merging
├── matching/                 # Candidate testing and edition selection
│   ├── mod.rs
│   ├── candidate.rs          # Match quality calculation
│   ├── edition.rs            # Edition processing, winner selection
│   └── validation.rs         # Single-track detection, name validation
├── utils/                    # Utility functions
│   ├── mod.rs
│   ├── audio.rs              # Audio decoding (symphonia)
│   ├── fingerprint.rs        # AcoustID fingerprinting
│   └── timing.rs             # Timing, heartbeat, stagger logic
└── README.md                 # This file
```

---

## Module Responsibilities

### Top-Level Modules

- **main.rs** - Entry point, CLI argument parsing, album processing loop, error handling
- **types.rs** - Shared data structures used across modules
- **constants.rs** - Global constants including STAGE2 parameter arrays
- **silence_detection.rs** - WindowDbProfile and silence detection algorithms

### musicbrainz/

**Purpose:** MusicBrainz API integration with caching

- **api.rs** - HTTP client with rate limiting (2-second interval between requests)
- **cache.rs** - File-based cache with 3 modes (Disabled, ReadWrite, ReadOnly)
- **types.rs** - MusicBrainz-specific data structures for API responses

### stages/

**Purpose:** Five-stage album matching pipeline

- **stage2.rs** - 180-parameter silence detection sweep (12 thresholds × 15 min_durations) with early-exit on 100% match
- **stage3.rs** - Dynamic programming assembly for over-segmented tracks
- **stage4.rs** - Edition-guided quiet spot detection using RMS profiling
- **stage5.rs** - Extra track merging to improve match quality

### matching/

**Purpose:** Candidate testing and edition selection logic

- **candidate.rs** - Test segmentation against expected track durations, calculate match percentage
- **edition.rs** - Process multiple MusicBrainz editions, select winner with track count penalty
- **validation.rs** - Single-track discriminator, artist/album name validation

### utils/

**Purpose:** Utility functions for audio processing and timing

- **audio.rs** - MP3 decoding using symphonia, sample extraction
- **fingerprint.rs** - AcoustID fingerprinting (if used)
- **timing.rs** - Timing measurement, heartbeat thread, progress reporting

---

## Dependencies

### Module Dependency Graph

```
main.rs
├── types.rs (foundation - used by all)
├── constants.rs (used by stages, matching)
├── silence_detection.rs
│   └── types.rs
├── musicbrainz/
│   ├── api.rs (HTTP, rate limiting)
│   ├── cache.rs (JSON serialization)
│   └── types.rs
├── stages/
│   ├── stage2.rs → silence_detection, types, constants
│   ├── stage3.rs → types, matching/candidate
│   ├── stage4.rs → utils/audio, types
│   └── stage5.rs → types, matching/candidate
├── matching/
│   ├── candidate.rs → types, constants
│   ├── edition.rs → stages, matching/candidate, types
│   └── validation.rs → types
└── utils/
    ├── audio.rs (symphonia)
    ├── fingerprint.rs (chromaprint)
    └── timing.rs (tokio)
```

**Key Principles:**
- `types.rs` is the foundation (shared by all modules)
- `constants.rs` used by stages and matching
- No circular dependencies (bottom-up design)
- Higher-level modules orchestrate lower-level modules

---

## Getting Started

### Compilation

**Wrapper entry point:**
```bash
cargo run --example album_matcher_28 -- --help
```

**Direct module access (during development):**
```rust
// examples/album_matcher_28.rs
mod am28;

fn main() -> anyhow::Result<()> {
    am28::main()
}
```

### Development Workflow

**Current Status:** Phase 4 (Structure Setup) - stub files created

**Next Steps:**
1. **Phase 5:** Extract types and constants from album_matcher_28.rs
2. **Phase 6:** Extract core modules (silence_detection, utils, musicbrainz)
3. **Phase 7:** Extract stage modules (stage2-5)
4. **Phase 8:** Extract matching modules and orchestration
5. **Phase 9:** Final verification (200-album dataset)

**Testing at Each Phase:**
```bash
# Quick verification (10 albums)
cargo run --release --example album_matcher_28 -- \
    --training-set training_set.txt \
    --limit 10 \
    --output test_output.txt

# Compare with baseline
python analyze_best_params.py test_output.txt
```

---

## Implementation Patterns

### Error Handling

Use `anyhow::Result<T>` consistently:
```rust
pub fn run_stage2(params: Params) -> Result<Output> {
    let result = compute(params)
        .context("Stage 2 failed")?;
    Ok(result)
}
```

### State Management

Pass state as parameters (no global mutable state):
```rust
fn process_albums(success_count: &mut usize) {
    // ...
}
```

### Async/Sync Boundaries

- **Async:** main(), MusicBrainz API calls
- **Sync:** stages (CPU-bound), audio decoding (blocking I/O)

### Module Visibility

- `am28::main()` - `pub` (called by wrapper)
- Internal modules - `pub(crate)` (shared within am28/ only)
- Helper functions - `pub(crate)` or private

---

## References

- **Specification:** wip/SPEC_album_matcher_28_refactoring.md
- **Plan:** wip/PLAN024_album_matcher_28_refactoring/
- **Source:** album_matcher_28.rs (monolithic version being refactored)
- **Baseline:** album_matcher_output_run27.txt (verification baseline)

---

## Document Control

**Version:** 1.0 (Phase 4 - Structure Setup)
**Status:** Stub files created, ready for extraction
**Last Updated:** 2025-11-25
