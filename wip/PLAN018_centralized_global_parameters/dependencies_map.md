# Dependencies Map - PLAN018: Centralized Global Parameters

## Existing Code (Read-Only References)

### Specifications and Documentation

| Document | Path | Purpose | Status |
|----------|------|---------|--------|
| SPEC016 | docs/SPEC016-mixer_and_buffers.md | Parameter definitions, defaults, DBD-PARAM tags | ✅ Current |
| IMPL001 | docs/IMPL001-database_schema.md | Settings table schema | ✅ Current |
| GOV002 | docs/GOV002-requirements_enumeration.md | Requirement ID format | ✅ Current |

### Source Code Files to Analyze

**Files with hardcoded parameter values (identified during migration):**

| Parameter | Likely Files to Check | Search Pattern |
|-----------|----------------------|----------------|
| working_sample_rate | mixer.rs, audio_output/mod.rs, decoder/*, main.rs | `44100` |
| audio_buffer_size | audio_output/mod.rs | `2208` |
| mixer_min_start_level | mixer.rs, engine.rs | `22050` |
| playout_ringbuffer_size | decoder/*, engine.rs | `661941` |
| output_ringbuffer_size | audio_output/mod.rs | `88200` |
| pause_decay_factor | mixer.rs | `0.95` |
| pause_decay_floor | mixer.rs | `0.0001778` |

**Note:** Exact locations determined per-parameter during Step 2 of migration process.

### Database Schema

**Settings Table (No Changes Required):**

```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value_type TEXT NOT NULL,  -- 'text', 'int', 'real', 'bool'
    value_text TEXT,
    value_int INTEGER,
    value_real REAL
);
```

**Expected parameter keys:**
- `volume_level` (real)
- `working_sample_rate` (int)
- `output_ringbuffer_size` (int)
- `output_refill_period` (int)
- `maximum_decode_streams` (int)
- `decode_work_period` (int)
- `decode_chunk_size` (int)
- `playout_ringbuffer_size` (int)
- `playout_ringbuffer_headroom` (int)
- `decoder_resume_hysteresis_samples` (int)
- `mixer_min_start_level` (int)
- `pause_decay_factor` (real)
- `pause_decay_floor` (real)
- `audio_buffer_size` (int)
- `mixer_check_interval_ms` (int)

## Code to Create

### New Files

**1. wkmp-common/src/params.rs (PRIMARY DELIVERABLE)**

```rust
//! Global parameter management
//!
//! [DBD-PARAM-*] Centralized singleton for all SPEC016 database-backed parameters

use std::sync::RwLock;
use once_cell::sync::Lazy;
use sqlx::SqlitePool;

pub static PARAMS: Lazy<GlobalParams> = Lazy::new(GlobalParams::default);

pub struct GlobalParams {
    // 15 RwLock<T> fields (see implementation design)
}

impl Default for GlobalParams { /* ... */ }

impl GlobalParams {
    pub async fn init_from_database(db_pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> { /* ... */ }

    // Setter methods with validation
    pub fn set_working_sample_rate(&self, value: u32) -> Result<(), String> { /* ... */ }
    // ... (15 setter methods total)
}
```

**Estimated Size:** 400-600 lines

### Files to Modify

**2. wkmp-common/src/lib.rs**

```rust
// Add module declaration
pub mod params;
```

**Change:** +1 line

**3. wkmp-ap/src/main.rs**

```rust
// After database pool creation, before engine init
wkmp_common::params::PARAMS.init_from_database(&db_pool).await?;
info!("Global parameters initialized from database");
```

**Change:** +2 lines

**4. Various files per parameter (identified during migration)**

Example for `working_sample_rate`:
- Remove: `sample_rate: Arc<RwLock<u32>>` field
- Remove: Initialization of Arc<RwLock>
- Replace: `*self.sample_rate.read().unwrap()` → `*wkmp_common::params::PARAMS.working_sample_rate.read().unwrap()`

**Changes:** Varies per file, identified in Step 2-4 of per-parameter migration

## External Dependencies

### Rust Crates

| Crate | Version | Purpose | Status |
|-------|---------|---------|--------|
| once_cell | 1.19.0 | Lazy static initialization | ✅ Already in Cargo.toml |
| std::sync | (stdlib) | RwLock for thread-safe access | ✅ Built-in |
| sqlx | 0.7.x | Database queries | ✅ Already in use |
| tokio | 1.x | Async runtime (for init) | ✅ Already in use |

**No new dependencies required.**

### System Dependencies

- SQLite 3.x with JSON1 extension (already required)
- No additional system libraries needed

## Integration Points

### 1. Database Initialization (wkmp-ap startup)

**Location:** `wkmp-ap/src/main.rs` (after database pool creation)

**Sequence:**
```rust
// 1. Create database pool
let db_pool = SqlitePool::connect(&db_url).await?;

// 2. Initialize global parameters (NEW)
wkmp_common::params::PARAMS.init_from_database(&db_pool).await?;

// 3. Continue with audio output and engine initialization
let audio_output = AudioOutput::new(...)?;
let engine = PlaybackEngine::new(...)?;
```

**Error Handling:**
- If database query fails: Log error, use defaults, continue startup
- If parameter value invalid: Log warning, use default, continue startup
- Fatal errors: Database completely inaccessible (already handled by pool creation)

### 2. Parameter Access (Hot Path)

**Audio Callback Example:**
```rust
// BEFORE
let sample_rate = *self.sample_rate.read().unwrap();

// AFTER
let sample_rate = *wkmp_common::params::PARAMS.working_sample_rate.read().unwrap();
```

**Performance Requirement:** RwLock::read() < 10ns uncontended

### 3. Test Suite Integration

**Unit Tests:**
```rust
#[cfg(test)]
mod tests {
    use wkmp_common::params::PARAMS;

    #[test]
    fn test_default_working_sample_rate() {
        assert_eq!(*PARAMS.working_sample_rate.read().unwrap(), 44100);
    }
}
```

**Integration Tests:**
```rust
#[tokio::test]
async fn test_params_load_from_database() {
    let db_pool = create_test_database().await;
    PARAMS.init_from_database(&db_pool).await.unwrap();
    // Verify loaded values
}
```

## Dependency Status Summary

| Dependency Type | Count | Status | Notes |
|----------------|-------|--------|-------|
| Specifications | 3 | ✅ Complete | SPEC016, IMPL001, GOV002 |
| Existing Code | ~10 files | ✅ Available | Identified during migration |
| External Crates | 0 new | ✅ Ready | All dependencies already present |
| Database Schema | 1 table | ✅ Ready | settings table unchanged |
| Integration Points | 3 | ✅ Defined | Startup, hot path, tests |

**All dependencies satisfied - ready to proceed with implementation.**

## Risk Dependencies

### High-Risk Code Locations

**1. Audio Callback (wkmp-ap/src/audio_output/mod.rs)**
- **Risk:** Panic in audio callback = audio glitch
- **Mitigation:** Verify RwLock::read() never panics (only poisoned if another thread panicked)
- **Testing:** Extensive manual testing with multiple buffer sizes

**2. Mixer Loop (wkmp-ap/src/playback/mixer.rs)**
- **Risk:** Timing error if sample rate incorrect
- **Mitigation:** Stopwatch test for 60s playback, verify ±100ms accuracy
- **Testing:** Multiple test files, different sample rates (44.1kHz, 48kHz)

**3. Position Tracking (wkmp-ap/src/playback/engine.rs)**
- **Risk:** 10% timing error already occurred (hardcoded 44100 vs actual 48000)
- **Mitigation:** Migrate working_sample_rate LAST, maximum caution
- **Testing:** Stopwatch test, verify position counter accuracy

## Document Status

**Status:** Phase 1 Complete
**Created:** 2025-11-02
**Dependencies Verified:** All satisfied
