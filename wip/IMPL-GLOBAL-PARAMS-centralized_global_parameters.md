# Centralized Global Parameters Implementation

**⚙️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Refactor all SPEC016 database-backed global parameters into a centralized singleton in `wkmp-common`, eliminating hardcoded values and providing consistent read-frequently/write-rarely access patterns.

> **Related:** [SPEC016-mixer_and_buffers.md](../docs/SPEC016-mixer_and_buffers.md) | [Database Schema](../docs/IMPL001-database_schema.md)

---

## Executive Summary

### Problem Statement

**Current state:**
- Global parameters scattered across module-specific fields
- Hardcoded values in 8+ locations (e.g., `44100` sample rate)
- No single source of truth
- Parameter updates require code changes in multiple places
- Bug-prone: Today's 10% timing error caused by hardcoded sample rate

**Critical bug example (2025-11-02):**
```rust
// mixer.rs (WRONG - hardcoded)
let tick_increment = samples_to_ticks(frames_read, 44100);

// Should be:
let sample_rate = *PARAMS.working_sample_rate.read().unwrap();
let tick_increment = samples_to_ticks(frames_read, sample_rate);
```

**Impact:** Position counter advanced 48000/44100 = 1.088x too fast on Windows (48kHz devices).

### Proposed Solution

**Centralized parameter singleton in `wkmp-common`:**
```rust
pub mod params {
    pub static PARAMS: Lazy<GlobalParams> = Lazy::new(GlobalParams::default);

    pub struct GlobalParams {
        pub working_sample_rate: RwLock<u32>,
        pub mixer_min_start_level: RwLock<usize>,
        // ... all SPEC016 parameters
    }
}
```

**Access pattern:**
```rust
// Read (fast, uncontended)
let rate = *wkmp_common::params::PARAMS.working_sample_rate.read().unwrap();

// Write (rare, initialization only)
*wkmp_common::params::PARAMS.working_sample_rate.write().unwrap() = 48000;
```

### Migration Strategy

**Incremental, one parameter at a time:**
1. Add parameter to `GlobalParams` struct
2. Initialize from database on startup
3. Replace all hardcoded references
4. Replace Arc<RwLock> field with PARAMS access
5. Run full test suite
6. Commit before next parameter

**Risk mitigation:** Each parameter migrated independently with full testing.

---

## Requirements

### Functional Requirements

**FR-001: Centralized Parameter Storage**
- All SPEC016 parameters stored in `wkmp-common::params::GlobalParams`
- Single source of truth for global configuration
- Parameters loaded from database `settings` table on startup

**FR-002: Consistent Access Pattern**
- `RwLock<T>` for read-frequently/write-rarely pattern
- Low-contention reader access (readers don't block each other)
- Thread-safe across all microservices

**FR-003: No Hardcoded Values**
- Zero tolerance for hardcoded parameter values in code
- All accesses go through `PARAMS` singleton
- Compiler enforces correct usage

**FR-004: Database Synchronization**
- Parameters initialized from `settings` table
- Future: Support runtime updates from database
- Default values if database entry missing

**FR-005: Backward Compatibility**
- Existing database schema unchanged
- Existing `settings` table entries used as-is
- No API breaking changes during migration

### Non-Functional Requirements

**NFR-001: Performance**
- `RwLock::read()` overhead < 10ns (uncontended)
- No performance regression vs current Arc<RwLock> pattern
- Zero allocation on parameter read

**NFR-002: Safety**
- No panics in parameter access (use `.unwrap()` only where lock poisoning = fatal)
- Type-safe parameter values
- Validated ranges on write

**NFR-003: Testability**
- Each parameter migration verified by full test suite
- Unit tests for parameter access patterns
- Integration tests for database initialization

**NFR-004: Maintainability**
- Clear documentation of each parameter's purpose
- Traceability to SPEC016 requirement IDs
- Migration path documented for each parameter

---

## Parameter Inventory

### Phase 1: Identify All Parameters

**Sources to scan:**
1. `docs/SPEC016-mixer_and_buffers.md` - Primary parameter definitions
2. `docs/SPEC001-architecture.md` - System-wide parameters
3. `docs/SPEC002-crossfade.md` - Crossfade timing parameters
4. `wkmp-ap/src/api/handlers.rs:1241-1292` - Current parameter list
5. All files with `[DBD-PARAM-*]` tags

**Known parameters (initial list):**

| Parameter | Type | SPEC016 Tag | Default | Description |
|-----------|------|-------------|---------|-------------|
| `volume_level` | `f32` | DBD-PARAM-010 | 0.5 | Audio output volume (0.0-1.0) |
| `working_sample_rate` | `u32` | DBD-PARAM-020 | 44100 | Sample rate for decoded audio (Hz) |
| `output_ringbuffer_size` | `usize` | DBD-PARAM-030 | 88200 | Output ring buffer max samples |
| `output_refill_period` | `u64` | DBD-PARAM-040 | 90 | Milliseconds between mixer checks |
| `maximum_decode_streams` | `usize` | DBD-PARAM-050 | 12 | Max parallel decoder chains |
| `decode_work_period` | `u64` | DBD-PARAM-060 | 5000 | Decode priority evaluation period (ms) |
| `decode_chunk_size` | `usize` | DBD-PARAM-065 | 25000 | Samples per decode chunk |
| `playout_ringbuffer_size` | `usize` | DBD-PARAM-070 | 661941 | Decoded audio buffer size (samples) |
| `playout_ringbuffer_headroom` | `usize` | DBD-PARAM-080 | 4410 | Buffer headroom for late samples |
| `decoder_resume_hysteresis_samples` | `u64` | DBD-PARAM-085 | 44100 | Hysteresis for decoder pause/resume |
| `mixer_min_start_level` | `usize` | DBD-PARAM-088 | 22050 | Min samples before mixer starts |
| `pause_decay_factor` | `f64` | DBD-PARAM-090 | 0.95 | Exponential decay in pause mode |
| `pause_decay_floor` | `f64` | DBD-PARAM-100 | 0.0001778 | Min level before zero output |
| `audio_buffer_size` | `u32` | DBD-PARAM-110 | 2208 | Audio output buffer (frames/callback) |
| `mixer_check_interval_ms` | `u64` | DBD-PARAM-111 | 10 | Mixer thread check interval |

**Additional parameters to discover:**
- Search all SPEC files for `settings` table references
- Search for `[DBD-*]` tags in all code
- Search for numeric literals matching defaults (hardcoded values)

---

## Implementation Design

### Module Structure

**Location:** `wkmp-common/src/params.rs`

```rust
//! Global parameter management
//!
//! Centralized singleton for all SPEC016 database-backed parameters.
//! Read-frequently, write-rarely access pattern using RwLock.

use std::sync::RwLock;
use once_cell::sync::Lazy;

/// Global parameters singleton
///
/// Initialized once from database, accessed everywhere.
/// Read-frequently (hot path), write-rarely (startup/config change).
pub static PARAMS: Lazy<GlobalParams> = Lazy::new(GlobalParams::default);

/// Global parameter storage
///
/// All parameters stored with RwLock for thread-safe access.
/// Readers don't block each other (shared read lock).
pub struct GlobalParams {
    // [DBD-PARAM-010] Audio output volume
    pub volume_level: RwLock<f32>,

    // [DBD-PARAM-020] Working sample rate (CRITICAL - timing accuracy)
    pub working_sample_rate: RwLock<u32>,

    // ... all other parameters
}

impl Default for GlobalParams {
    fn default() -> Self {
        Self {
            volume_level: RwLock::new(0.5),
            working_sample_rate: RwLock::new(44100),
            // ... defaults from SPEC016
        }
    }
}

impl GlobalParams {
    /// Initialize all parameters from database
    ///
    /// Called once at wkmp-ap startup. Loads values from settings table.
    /// Falls back to defaults if database entry missing.
    pub async fn init_from_database(db_pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        // Load each parameter from settings table
        // Update PARAMS singleton with loaded values
        // Log any parameters using defaults

        todo!("Phase 1 implementation")
    }

    /// Validate and update a parameter
    ///
    /// Applies range validation before writing.
    /// Returns error if value out of valid range.
    pub fn set_working_sample_rate(&self, value: u32) -> Result<(), String> {
        // Validate: must be in [8000, 192000] and supported by TICK_RATE
        if value < 8000 || value > 192000 {
            return Err(format!("Sample rate {} out of range [8000, 192000]", value));
        }

        *self.working_sample_rate.write().unwrap() = value;
        Ok(())
    }

    // ... setter methods for other parameters with validation
}
```

### Access Patterns

**Hot path (read - frequently):**
```rust
// In audio callback / mixer / decoder
let sample_rate = *wkmp_common::params::PARAMS.working_sample_rate.read().unwrap();
let tick_increment = samples_to_ticks(frames_read, sample_rate);
```

**Cold path (write - rarely):**
```rust
// At startup / configuration change
wkmp_common::params::PARAMS.set_working_sample_rate(48000)?;
```

**Error handling:**
```rust
// Lock poisoning = fatal (another thread panicked while holding write lock)
// Using .unwrap() is acceptable - if poisoned, system is already broken
let value = *PARAMS.working_sample_rate.read().unwrap();
```

### Database Schema

**No changes required** - use existing `settings` table:

```sql
-- Parameters already stored in settings table
SELECT key, value_type, value_text
FROM settings
WHERE key LIKE 'working_sample_rate'
   OR key LIKE 'mixer_%'
   OR key LIKE 'decoder_%'
   OR key LIKE 'playout_%';
```

**Initialization query:**
```sql
-- Load all parameters at startup
SELECT key, value_type, value_text, value_int, value_real
FROM settings
WHERE key IN (
    'volume_level',
    'working_sample_rate',
    'output_ringbuffer_size',
    -- ... all parameter keys
);
```

---

## Migration Plan

### Commit Strategy

**Use manual `git commit` - NOT `/commit` workflow**

**Rationale:**
- `/commit` workflow includes automatic change_history.md updates and archive sync
- Designed for significant features/bug fixes, not incremental refactoring
- 15 parameters × 1 commit each = 15 commits total (manageable)
- Using `/commit` would create 15+ entries in change_history.md (spam)
- Manual commits are more efficient for systematic refactoring work

**Approach:**
- Each parameter = 1 commit after full verification (Steps 1-6 complete)
- Concise commit messages with structured format (see Step 7)
- Example: "Migrate mixer_min_start_level to GlobalParams [DBD-PARAM-088]"
- Final commit: "Complete GlobalParams migration - all 15 parameters"

### Migration Order (Risk-Based)

**Tier 1 (Low Risk - Non-Timing):**
1. `maximum_decode_streams` - Affects resource usage only
2. `decode_work_period` - Background task timing
3. `pause_decay_factor` - Audio quality (gradual)
4. `pause_decay_floor` - Audio quality (gradual)
5. `volume_level` - User-visible but non-critical

**Tier 2 (Medium Risk - Buffer Sizes):**
6. `output_ringbuffer_size` - Affects latency
7. `playout_ringbuffer_size` - Affects buffering
8. `playout_ringbuffer_headroom` - Affects safety margin
9. `mixer_min_start_level` - Affects startup behavior

**Tier 3 (High Risk - Critical Timing):**
10. `working_sample_rate` - CRITICAL (affects all timing)
11. `audio_buffer_size` - CRITICAL (audio callback)
12. `mixer_check_interval_ms` - CRITICAL (mixer loop)
13. `decode_chunk_size` - Affects decoder timing
14. `output_refill_period` - Affects mixer wake timing
15. `decoder_resume_hysteresis_samples` - Affects backpressure

**Rationale:** Low-risk parameters first to validate approach, timing-critical parameters last with maximum caution.

### Per-Parameter Migration Process

**For EACH parameter, follow this sequence:**

**COMMIT STRATEGY:** Use manual `git commit` with concise messages. Each parameter = 1 commit after full verification. Do NOT use `/commit` workflow (would spam commit log with 100+ repetitive messages).

#### Step 1: Add to GlobalParams
```rust
// wkmp-common/src/params.rs
pub struct GlobalParams {
    // ... existing parameters

    /// [DBD-PARAM-XXX] Description
    pub new_parameter: RwLock<Type>,
}

impl Default for GlobalParams {
    fn default() -> Self {
        Self {
            // ... existing
            new_parameter: RwLock::new(default_value),
        }
    }
}
```

#### Step 2: Find All Hardcoded References
```bash
# Search for hardcoded values
rg "hardcoded_value" --type rust

# Search for parameter name in comments
rg "DBD-PARAM-XXX" --type rust

# Search for old field name
rg "old_field_name" --type rust
```

**Document:** List all locations in implementation notes

#### Step 3: Replace Hardcoded Values
```rust
// BEFORE (hardcoded)
let threshold = 22050;

// AFTER (parameter)
let threshold = *wkmp_common::params::PARAMS.mixer_min_start_level.read().unwrap();
```

**Test:** Run `cargo test` - all tests must pass

#### Step 4: Remove Arc<RwLock> Field (if exists)
```rust
// BEFORE
pub struct PlaybackEngine {
    mixer_min_start_level: Arc<RwLock<usize>>,
}

// AFTER
pub struct PlaybackEngine {
    // Removed - use wkmp_common::params::PARAMS.mixer_min_start_level
}
```

**Test:** Run `cargo test` - all tests must pass

#### Step 5: Update Initialization Code
```rust
// BEFORE
let param_value = load_param_from_db(&db_pool).await?;
self.param_field = Arc::new(RwLock::new(param_value));

// AFTER
let param_value = load_param_from_db(&db_pool).await?;
*wkmp_common::params::PARAMS.new_parameter.write().unwrap() = param_value;
```

**Test:** Run integration tests

#### Step 6: Full Test Suite
```bash
# Unit tests
cargo test --workspace

# Integration tests (if available)
cargo test --test '*' --workspace

# Manual smoke test
cargo run -p wkmp-ap
# Play audio, verify timing, check logs
```

**Verify:**
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Manual testing confirms no regression
- [ ] Position timing accurate (stopwatch test for timing params)
- [ ] Audio playback smooth

#### Step 7: Single Commit for Parameter

**After Steps 1-6 complete and verified:**

```bash
git add -A
git commit -m "Migrate [param_name] to GlobalParams

- Add [param_name] to wkmp-common::params::GlobalParams
- Replace N hardcoded references
- Remove Arc<RwLock> field from [struct_names]
- Initialize from database settings table
- All tests passing

[DBD-PARAM-XXX]"
```

**Commit message format:**
- Line 1: "Migrate [param_name] to GlobalParams"
- Line 2: blank
- Lines 3-N: Bullet points of changes
- Last line: SPEC016 tag

**Example:**
```
Migrate mixer_min_start_level to GlobalParams

- Add mixer_min_start_level to wkmp-common::params::GlobalParams
- Replace 3 hardcoded references (22050)
- Remove Arc<RwLock> field from PlaybackEngine
- Initialize from database settings table
- All tests passing

[DBD-PARAM-088]
```

### Testing Strategy

**Per-parameter tests:**
1. **Unit tests** - Parameter access patterns
2. **Integration tests** - Database initialization
3. **Manual tests** - Audio playback verification

**Regression prevention:**
```rust
#[test]
fn test_no_hardcoded_sample_rate() {
    // Parse all source files
    // Search for literal "44100" outside of tests/defaults
    // FAIL if found
}
```

**Critical timing verification:**
```rust
#[test]
fn test_position_accuracy() {
    // Play 60s of audio
    // Verify position counter matches elapsed time within 100ms
}
```

---

## Acceptance Criteria

### Per-Parameter Migration

- [ ] Parameter added to `GlobalParams` struct
- [ ] Default value matches SPEC016
- [ ] All hardcoded references replaced
- [ ] All Arc<RwLock> fields removed
- [ ] Database initialization implemented
- [ ] Full test suite passes
- [ ] Manual testing confirms no regression
- [ ] Documentation updated

### Overall Project Completion

- [ ] All SPEC016 parameters migrated
- [ ] Zero hardcoded parameter values in codebase
- [ ] Single source of truth (`PARAMS` singleton)
- [ ] Database initialization working
- [ ] All tests passing
- [ ] Position timing accurate (stopwatch verified)
- [ ] Audio playback quality unchanged
- [ ] Performance baseline maintained

---

## Risk Assessment

### High-Risk Parameters

**`working_sample_rate` (DBD-PARAM-020):**
- **Risk:** 10% timing error already occurred
- **Impact:** All position tracking, crossfade timing, marker events
- **Mitigation:** Migrate last, maximum testing, stopwatch verification

**`audio_buffer_size` (DBD-PARAM-110):**
- **Risk:** Audio glitches/dropouts
- **Impact:** Real-time audio callback timing
- **Mitigation:** Extensive manual testing, multiple buffer sizes

**`mixer_check_interval_ms` (DBD-PARAM-111):**
- **Risk:** Underruns or excess CPU
- **Impact:** Mixer wake timing
- **Mitigation:** Monitor CPU usage, verify no underruns

### Mitigation Strategies

1. **Incremental migration** - One parameter at a time
2. **Test after each step** - Catch regressions immediately
3. **Low-risk first** - Build confidence before timing-critical
4. **Rollback plan** - Each parameter is separate commit
5. **Manual verification** - Stopwatch test for timing parameters

---

## Open Questions

**Q1: Should parameters be hot-reloadable from database?**
- Current plan: Initialize once at startup
- Future enhancement: Runtime updates via API
- Decision: Start with startup-only, add hot reload later

**Q2: Should we validate parameter ranges on write?**
- Proposal: Add setter methods with validation
- Example: `set_working_sample_rate(u32) -> Result<(), String>`
- Decision: Yes, prevent invalid values

**Q3: What happens if database missing parameter entry?**
- Proposal: Use default from `GlobalParams::default()`
- Log warning but continue
- Decision: Graceful fallback to defaults

**Q4: Should tests use separate parameter values?**
- Proposal: Tests create test-specific `GlobalParams` instance
- Avoid singleton for test isolation
- Decision: Add `GlobalParams::new_for_test()` constructor

---

## Success Metrics

**Correctness:**
- ✅ Zero hardcoded parameter values in production code
- ✅ Position timing accurate within 100ms (stopwatch verified)
- ✅ All unit tests passing
- ✅ All integration tests passing

**Performance:**
- ✅ No measurable performance regression
- ✅ Parameter read overhead < 10ns
- ✅ Audio callback latency unchanged

**Maintainability:**
- ✅ Single source of truth for parameters
- ✅ Clear documentation for each parameter
- ✅ Easy to add new parameters in future

---

## References

- [SPEC016 - Mixer and Buffers](../docs/SPEC016-mixer_and_buffers.md)
- [IMPL001 - Database Schema](../docs/IMPL001-database_schema.md)
- [Bug Report 2025-11-02 - 10% Timing Error](../project_management/change_history.md)

---

**Document Status:** Ready for `/plan` workflow
**Author:** Claude + User
**Created:** 2025-11-02
**Last Updated:** 2025-11-02
