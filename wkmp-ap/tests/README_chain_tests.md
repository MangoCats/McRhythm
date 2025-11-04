# Chain Assignment and Priority Filling Tests

## Purpose

Automated test suite for decoder-buffer chain assignment lifecycle and buffer filling priority. Prevents regression of recurring issues:
- Chain collision when passages removed (chains not properly cleaned up)
- Unassigned passages not getting chains when available
- Buffer filling priority not respecting queue position
- Re-evaluation not triggered on chain assignment changes

## Test Coverage Matrix

| Test | Requirements | Historical Issue | Priority |
|------|--------------|------------------|----------|
| `test_chain_assignment_on_enqueue` | [DBD-LIFECYCLE-010] | N/A | P0 |
| `test_chain_exhaustion` | [DBD-PARAM-050] | N/A | P1 |
| `test_chain_release_on_removal` | [DBD-LIFECYCLE-020] | Chain collision | P0 |
| `test_unassigned_passage_gets_chain_on_availability` | [DBD-LIFECYCLE-030] | Unassigned passages ignored | P0 |
| `test_chain_reassignment_after_batch_removal` | [DBD-LIFECYCLE-020/030] | Chain collision + reassignment | P0 |
| `test_buffer_priority_by_queue_position` | [DBD-DEC-045] | Haphazard buffer filling order | P0 |
| `test_reevaluation_on_chain_assignment_change` | [DBD-DEC-045] | Re-evaluation not triggered | P1 |
| `test_buffer_fill_level_selection` | [DBD-DEC-045] | Overfilling buffers | P1 |
| `test_decode_work_period_reevaluation` | [DBD-DEC-045], [DBD-PARAM-060] | Stale priority selection | P1 |
| `test_no_chain_collision` | [DBD-LIFECYCLE-020] | Chain collision on reuse | P0 |

**Priority Levels:**
- **P0**: Core chain lifecycle and historical regression tests (MUST implement)
- **P1**: Re-evaluation and priority selection tests (SHOULD implement)

## Test Harness Requirements

### 1. TestEngine Wrapper

Wrapper around `PlaybackEngine` providing test-friendly interface:

```rust
struct TestEngine {
    engine: Arc<PlaybackEngine>,
    db_path: PathBuf,
    temp_dir: tempfile::TempDir,
}

impl TestEngine {
    async fn new(max_streams: usize) -> Result<Self>;
    async fn enqueue_file(&self, path: PathBuf) -> Result<Uuid>;
    async fn remove_queue_entry(&self, id: Uuid) -> Result<()>;
    async fn play(&self) -> Result<()>;
    async fn get_buffer_chains(&self) -> Vec<ChainInfo>;
    async fn get_queue(&self) -> Vec<QueueEntryInfo>;
    async fn get_chain_index(&self, id: Uuid) -> Option<usize>;
}
```

### 2. Test Database Setup

- In-memory SQLite database for isolation
- Minimal schema (passages, queue_entries, settings tables)
- Pre-populate test passages with stub metadata

### 3. Test Audio Files

Two approaches (choose one):

**Option A (Mock):** Generate silent PCM buffers in-memory, skip actual decoding
- Faster test execution
- No file I/O dependencies
- Requires mocking decoder creation

**Option B (Real):** Create minimal valid audio files (e.g., 100ms silence MP3)
- Tests actual decoder integration
- More realistic end-to-end testing
- Requires test asset files

**Recommendation:** Option A for unit tests (faster), Option B for integration tests (thorough)

### 4. State Inspection Helpers

Add methods to expose internal state for verification:

```rust
impl PlaybackEngine {
    #[cfg(test)]
    pub async fn test_get_chain_assignments(&self) -> HashMap<Uuid, usize>;

    #[cfg(test)]
    pub async fn test_get_available_chains(&self) -> Vec<usize>;

    #[cfg(test)]
    pub async fn test_get_decoder_state(&self) -> DecoderWorkerState;
}
```

### 5. Timing Control

Tests need deterministic timing:
- Replace `tokio::time::sleep` with `tokio::time::pause()`/`advance()` in tests
- Mock decode chunk processing (instant completion)
- Control re-evaluation trigger timing

## Implementation Guide

### Step 1: Create TestEngine Wrapper

File: `wkmp-ap/tests/test_engine.rs`

```rust
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;
use wkmp_ap::playback::engine::PlaybackEngine;
use wkmp_common::models::db::passage::PassageDbData;

pub struct TestEngine {
    engine: Arc<PlaybackEngine>,
    db_path: PathBuf,
    _temp_dir: TempDir,
}

impl TestEngine {
    pub async fn new(max_streams: usize) -> anyhow::Result<Self> {
        // Create temp directory
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        // Initialize database with schema
        let pool = wkmp_common::db::create_pool(&db_path).await?;
        wkmp_common::db::run_migrations(&pool).await?;

        // Set maximum_decode_streams setting
        sqlx::query("INSERT INTO settings (key, value) VALUES ('maximum_decode_streams', ?)")
            .bind(max_streams.to_string())
            .execute(&pool)
            .await?;

        // Create PlaybackEngine
        let engine = Arc::new(PlaybackEngine::new(pool).await?);

        Ok(Self {
            engine,
            db_path,
            _temp_dir: temp_dir,
        })
    }

    pub async fn enqueue_file(&self, path: PathBuf) -> anyhow::Result<Uuid> {
        // Create stub passage in database
        let passage_id = Uuid::new_v4();
        // ... insert passage and enqueue
        todo!("Implement passage creation and enqueue")
    }

    pub async fn get_buffer_chains(&self) -> Vec<ChainInfo> {
        // Call engine's test helper to get chain state
        todo!("Implement chain state inspection")
    }
}

pub struct ChainInfo {
    pub queue_entry_id: Option<Uuid>,
    pub slot_index: usize,
    pub queue_position: Option<u32>,
    pub buffer_fill_percent: f32,
}
```

### Step 2: Add Test Helpers to PlaybackEngine

File: `wkmp-ap/src/playback/engine/core.rs`

```rust
#[cfg(test)]
impl PlaybackEngine {
    pub async fn test_get_chain_assignments(&self) -> HashMap<Uuid, usize> {
        self.chain_assignments.read().await.clone()
    }

    pub async fn test_get_available_chains(&self) -> Vec<usize> {
        self.available_chains.read().await
            .iter()
            .map(|Reverse(idx)| *idx)
            .collect()
    }

    pub async fn test_get_decoder_state(&self) -> TestDecoderState {
        // Expose relevant decoder_worker state
        todo!("Implement decoder state inspection")
    }
}
```

### Step 3: Implement Mock Audio Files

File: `wkmp-ap/tests/test_audio.rs`

```rust
use std::path::PathBuf;

pub fn create_test_passage(db: &SqlitePool, index: usize) -> anyhow::Result<Uuid> {
    let passage_id = Uuid::new_v4();
    let passage = PassageDbData {
        passage_id,
        filepath: PathBuf::from(format!("test_{}.mp3", index)),
        start_frame: 0,
        end_frame: 44100, // 1 second @ 44.1kHz
        // ... other fields
    };
    // Insert into database
    todo!("Insert passage")
}
```

### Step 4: Enable First Test

In `chain_assignment_tests.rs`, remove `#[ignore]` from `test_chain_assignment_on_enqueue`:

```rust
#[tokio::test]
// #[ignore] // REMOVED
async fn test_chain_assignment_on_enqueue() {
    let engine = TestEngine::new(12).await.unwrap();

    // Enqueue 12 passages
    let mut queue_entry_ids = Vec::new();
    for i in 0..12 {
        let queue_entry_id = engine.enqueue_file(test_audio_file(i)).await.unwrap();
        queue_entry_ids.push(queue_entry_id);
    }

    // Verify: All 12 passages have assigned chains
    let chains = engine.get_buffer_chains().await;
    assert_eq!(chains.len(), 12, "All 12 passages should have chains");

    // Verify: Chain indexes are unique (0-11)
    let mut chain_indexes: Vec<usize> = chains.iter().map(|c| c.slot_index).collect();
    chain_indexes.sort();
    assert_eq!(chain_indexes, (0..12).collect::<Vec<_>>(), "Chain indexes should be 0-11");
}
```

### Step 5: Run Tests Incrementally

```bash
# Run single test
cargo test -p wkmp-ap test_chain_assignment_on_enqueue -- --nocapture

# Run all chain tests
cargo test -p wkmp-ap chain_assignment_tests -- --nocapture

# Run with logging
RUST_LOG=debug cargo test -p wkmp-ap test_chain_assignment_on_enqueue -- --nocapture
```

### Step 6: Iterate Through Remaining Tests

Enable tests in order of priority:
1. P0 tests: `test_chain_release_on_removal`, `test_unassigned_passage_gets_chain_on_availability`, `test_no_chain_collision`
2. P0 batch test: `test_chain_reassignment_after_batch_removal`
3. P1 priority tests: `test_buffer_priority_by_queue_position`, `test_reevaluation_on_chain_assignment_change`

## Test Execution Strategy

### Phase 1: Chain Lifecycle (P0)
Focus on core chain assignment/release/reassignment without buffer filling logic.

**Success Criteria:**
- Chains assigned up to maximum_decode_streams
- Chains properly released on removal (no collision)
- Freed chains reassigned to unassigned passages

### Phase 2: Buffer Priority (P0/P1)
Add buffer filling verification and priority selection tests.

**Success Criteria:**
- Position 0 buffer fills first
- Lower positions fill in order
- Re-evaluation triggers on events

### Phase 3: Edge Cases (P1)
Complete remaining re-evaluation and timing tests.

**Success Criteria:**
- decode_work_period triggers re-evaluation
- Buffers above resume threshold not selected
- Generation counter properly tracks changes

## Continuous Integration

Add to CI pipeline:
```yaml
- name: Run chain assignment tests
  run: cargo test -p wkmp-ap chain_assignment_tests
```

## Maintenance

**When to update tests:**
- Changing maximum_decode_streams default
- Modifying chain assignment logic
- Altering re-evaluation triggers
- Adjusting hysteresis thresholds

**Test failure triage:**
1. Check if requirement changed (update test)
2. Check if implementation regressed (fix code)
3. Check if test assumptions invalid (update test)

## References

- [SPEC016-decoder_buffer_design.md](../docs/SPEC016-decoder_buffer_design.md) - Requirements [DBD-LIFECYCLE-*] and [DBD-DEC-045]
- [decoder_worker.rs](../wkmp-ap/src/playback/decoder_worker.rs) - Implementation under test
- [engine/core.rs](../wkmp-ap/src/playback/engine/core.rs) - Chain assignment orchestration
