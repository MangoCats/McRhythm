# Increment 1: Testing Module Foundation

**Increment:** 1 of 9
**Phase:** Foundation
**Estimated Effort:** 2-3 hours
**Confidence:** HIGH (±20%)
**Prerequisites:** None

## Objective

Create the `wkmp-ai/src/testing/` module structure with core types and exports.

## Deliverables

1. `wkmp-ai/src/testing/mod.rs` - Module root with exports
2. `wkmp-ai/src/testing/types.rs` - Core types (Classification, FailureCategory, etc.)
3. `wkmp-ai/src/lib.rs` - Add `pub mod testing;`

## Requirements Covered

- None directly (foundation for all requirements)

## Tests to Pass

- Module compiles without errors
- Types are exported correctly

## Acceptance Criteria

- [ ] `cargo build -p wkmp-ai` succeeds
- [ ] `use wkmp_ai::testing::*` works
- [ ] Core enums (Classification, FailureCategory, ResetMode) defined
- [ ] Placeholder structs for TestEvaluator, BatchConfig defined

## Implementation Notes

```rust
// testing/types.rs
pub enum Classification {
    TruePositive,
    FalsePositive,
    TrueNegative,
    FalseNegative,
}

pub enum FailureCategory {
    WrongRecording,
    WrongAlbum,
    WrongArtist,
    MissedIdentification,
    OverconfidentWrong,
    UnderconfidentRight,
    AcoustIdMismatch,
    MetadataSearchFailed,
    ID3TagsUnreliable,
    MultipleValidMatches,
    NoGroundTruth,
    Timeout,
    ApiError,
}

pub enum ResetMode {
    Full,
    TestDataOnly,
    BatchOnly { batch_id: String },
    Incremental,
}

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub batch_size: usize,
    pub failure_threshold: usize,
    pub accuracy_threshold: f64,
    pub parallel_workers: usize,
    pub timeout_secs: u64,
}

impl Default for BatchConfig { ... }
```

## Dependencies

- Requires: None
- Enables: I2, I3, I4, I5, I6, I7, I8
