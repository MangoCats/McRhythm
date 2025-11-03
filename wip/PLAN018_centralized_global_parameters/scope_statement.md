# Scope Statement - PLAN018: Centralized Global Parameters

## In Scope

**✅ WILL be implemented:**

1. **GlobalParams Singleton Creation**
   - Create `wkmp-common/src/params.rs` module
   - Define `GlobalParams` struct with RwLock fields for all 15 parameters
   - Implement `Default` trait with SPEC016 default values
   - Export via `wkmp_common::params::PARAMS` static

2. **Database Initialization**
   - `GlobalParams::init_from_database()` method
   - Load all 15 parameters from `settings` table at wkmp-ap startup
   - Graceful fallback to defaults if database entries missing
   - Log warnings for missing/defaulted parameters

3. **Parameter Migration (All 15 Parameters)**
   - Migrate in risk-based order (Tier 1 → Tier 2 → Tier 3)
   - Per-parameter process:
     - Add to GlobalParams struct
     - Find all hardcoded references
     - Replace hardcoded values with PARAMS access
     - Remove Arc<RwLock> fields from structs
     - Update initialization code
     - Full test suite verification
     - Single commit per parameter

4. **Validation Methods**
   - Setter methods with range validation for each parameter
   - Type-safe parameter updates
   - Clear error messages for invalid values

5. **Testing**
   - Unit tests for parameter access patterns
   - Integration tests for database initialization
   - Regression tests to detect hardcoded values
   - Manual smoke tests for timing-critical parameters

6. **Documentation Updates**
   - Code documentation (doc comments)
   - Migration tracking in commit messages
   - Parameter purpose and traceability (DBD-PARAM-XXX tags)

## Out of Scope

**❌ Will NOT be implemented:**

1. **Hot-Reloadable Parameters**
   - Runtime parameter updates from database
   - API endpoints for parameter changes
   - **Deferred to:** Future enhancement (post-PLAN018)

2. **Parameter History/Audit Trail**
   - Tracking parameter value changes over time
   - **Rationale:** Not required for initial implementation

3. **Per-User Parameter Overrides**
   - User-specific parameter values
   - **Rationale:** Multi-user coordination is read-only for parameters

4. **GUI Configuration Interface**
   - Web UI for editing parameters
   - **Rationale:** Database `settings` table is configuration source

5. **Parameter Validation at Database Load**
   - Complex cross-parameter validation rules
   - **Rationale:** Load-time validation is basic (type + range only)

6. **New Parameters**
   - Adding parameters beyond the 15 identified in SPEC016
   - **Rationale:** Scope limited to existing parameters

7. **Changes to Database Schema**
   - No modifications to `settings` table structure
   - **Rationale:** Use existing schema as-is

## Assumptions

1. **Database Availability:** SQLite database is available and writable at wkmp-ap startup
2. **Single-Writer Model:** Only wkmp-ap writes to GlobalParams during initialization
3. **No Lock Contention:** RwLock read access is uncontended in audio hot path
4. **Startup-Only Init:** Parameters loaded once at startup, not dynamically reloaded
5. **Lock Poisoning is Fatal:** If RwLock is poisoned, system should panic (indicates critical failure)
6. **Test Environment:** Full test suite runs successfully on current codebase
7. **SPEC016 Completeness:** All database-backed parameters are documented in SPEC016
8. **Default Values Correct:** SPEC016 default values are validated and production-ready

## Constraints

### Technical Constraints

1. **No Breaking API Changes**
   - Public APIs remain unchanged during migration
   - Existing code continues to compile without modifications to callers

2. **Performance Requirements**
   - RwLock::read() overhead < 10ns (uncontended)
   - No measurable performance regression in audio callback
   - Zero allocation on parameter read

3. **Safety Requirements**
   - No panics in hot path (audio callback, mixer loop)
   - Thread-safe access from all microservices
   - Type-safe parameter values (compile-time checks)

4. **Rust Ecosystem**
   - Use `std::sync::RwLock` (not external crate)
   - Use `once_cell::sync::Lazy` for singleton initialization
   - Follow Rust idioms and conventions

### Process Constraints

1. **Incremental Migration**
   - One parameter at a time (15 separate migrations)
   - Full test suite must pass after each parameter
   - Manual verification for timing-critical parameters

2. **Commit Discipline**
   - Use manual `git commit` (NOT `/commit` workflow)
   - One commit per parameter migration
   - Structured commit message format (see Migration Plan Step 7)

3. **Risk-Based Ordering**
   - Low-risk parameters first (Tier 1)
   - High-risk timing parameters last (Tier 3)
   - Immediate rollback if regression detected

### Timeline Constraints

1. **Estimated Duration:** 8-12 hours total (15 parameters × 30-45 min each)
2. **Critical Path:** working_sample_rate migration (highest risk, thorough testing required)
3. **No Hard Deadline:** Quality over speed (catch regressions immediately)

## Dependencies

### Existing Modules (Read-Only)

- `wkmp-ap/src/playback/engine.rs` - Main consumer of parameters
- `wkmp-ap/src/audio_output/mod.rs` - Uses audio_buffer_size, working_sample_rate
- `wkmp-ap/src/playback/mixer.rs` - Uses mixer_* and pause_* parameters
- `wkmp-ap/src/decode/` - Uses decoder_* parameters
- `docs/SPEC016-mixer_and_buffers.md` - Parameter definitions and defaults
- `docs/IMPL001-database_schema.md` - Settings table schema

### Code to Modify

- `wkmp-common/src/lib.rs` - Add `pub mod params;`
- `wkmp-common/src/params.rs` - New file (create)
- `wkmp-ap/src/main.rs` - Add GlobalParams initialization call
- Various files with hardcoded values (identified per-parameter)
- Various structs with Arc<RwLock<T>> fields (identified per-parameter)

### External Dependencies

- `once_cell` crate - Already in Cargo.toml (used for Lazy static)
- `std::sync::RwLock` - Standard library (no new dependency)
- `sqlx` - Already in use for database queries

### Integration Points

1. **Database Settings Table**
   - Query: `SELECT key, value_type, value_text, value_int, value_real FROM settings WHERE key IN (...)`
   - Expected keys: 15 parameter names matching DBD-PARAM-XXX definitions

2. **wkmp-ap Startup Sequence**
   - After database pool creation
   - Before audio output initialization
   - Before engine instantiation

3. **Test Suite**
   - Unit tests: `cargo test --workspace`
   - Integration tests: `cargo test --test '*' --workspace`
   - Manual tests: Run wkmp-ap, play audio, verify timing

## Success Metrics

### Quantitative Metrics

1. **Zero Hardcoded Values:** No numeric literals matching parameter defaults in production code
2. **All Tests Pass:** 100% test suite success rate after each migration
3. **Position Timing Accuracy:** Within 100ms of stopwatch measurement (60s test)
4. **Performance:** No measurable increase in audio callback latency
5. **Code Reduction:** Remove 15 Arc<RwLock<T>> fields from various structs

### Qualitative Metrics

1. **Single Source of Truth:** All parameters accessed via `wkmp_common::params::PARAMS`
2. **Maintainability:** Clear documentation for each parameter
3. **Developer Experience:** Easy to add new parameters in future
4. **Code Quality:** Consistent access pattern throughout codebase
5. **Confidence:** Full test coverage and manual verification at each step

## Risks and Mitigations

See SPEC document lines 523-548 for detailed risk assessment.

**Summary:**
- **High-Risk Parameters:** working_sample_rate, audio_buffer_size, mixer_check_interval_ms
- **Mitigation Strategy:** Migrate last, maximum testing, stopwatch verification, immediate rollback capability

## Document Status

**Status:** Phase 1 Complete
**Created:** 2025-11-02
**Scope Approved:** Pending user review
