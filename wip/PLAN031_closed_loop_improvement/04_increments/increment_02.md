# Increment 2: Ground Truth Schema & Store

**Increment:** 2 of 9
**Phase:** Foundation
**Estimated Effort:** 2-3 hours
**Confidence:** HIGH (±20%)
**Prerequisites:** Increment 1

## Objective

Create ground truth database schema and data access layer.

## Deliverables

1. `migrations/YYYYMMDDHHMMSS_ground_truth.sql` - Schema migration
2. `wkmp-ai/src/testing/ground_truth.rs` - Data access functions

## Requirements Covered

- SPEC031-GT-010 (Ground truth management) - partial
- SPEC031-GT-020 (Ground truth schema) - complete

## Tests to Pass

- TC-U-GT-004: Ground truth schema validation

## Acceptance Criteria

- [ ] Migration creates `ground_truth` table
- [ ] `GroundTruth` struct with all fields from spec
- [ ] `get_by_hash(pool, hash)` function works
- [ ] `load_from_json(pool, path)` function works
- [ ] `load_from_embedded_tags(pool, files)` function works

## Implementation Notes

```rust
// testing/ground_truth.rs
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GroundTruth {
    pub id: i64,
    pub file_hash: String,
    pub file_path: String,
    pub expected_mbid: Option<String>,
    pub expected_album_mbid: Option<String>,
    pub verification_method: String,
    pub verified_at: String,
    pub confidence: f64,
    pub notes: Option<String>,
}

pub async fn get_by_hash(pool: &SqlitePool, hash: &str) -> Result<Option<GroundTruth>>;
pub async fn insert(pool: &SqlitePool, gt: &GroundTruth) -> Result<()>;
pub async fn load_from_json(pool: &SqlitePool, path: &Path) -> Result<usize>;
pub async fn load_from_embedded_tags(pool: &SqlitePool, files: &[PathBuf]) -> Result<usize>;
```

## Dependencies

- Requires: I1 (types)
- Enables: I3 (evaluation), I7 (reset)
