# Requirements Index - Centralized Global Parameters

## Functional Requirements

### FR-001: Centralized Parameter Storage
**Priority:** HIGH
**Description:** All SPEC016 parameters stored in `wkmp-common::params::GlobalParams`
**Details:**
- Single source of truth for global configuration
- Parameters loaded from database `settings` table on startup
- 15 parameters total (see Parameter Inventory)

**Specification Reference:** Lines 76-79

### FR-002: Consistent Access Pattern
**Priority:** HIGH
**Description:** `RwLock<T>` for read-frequently/write-rarely pattern
**Details:**
- Low-contention reader access (readers don't block each other)
- Thread-safe across all microservices
- Standard access pattern throughout codebase

**Specification Reference:** Lines 81-84

### FR-003: No Hardcoded Values
**Priority:** CRITICAL
**Description:** Zero tolerance for hardcoded parameter values in code
**Details:**
- All accesses go through `PARAMS` singleton
- Compiler enforces correct usage
- Prevents timing bugs (e.g., 44100 hardcoded sample rate issue)

**Specification Reference:** Lines 86-89

### FR-004: Database Synchronization
**Priority:** HIGH
**Description:** Parameters initialized from `settings` table
**Details:**
- Load all parameters at startup
- Future: Support runtime updates from database
- Default values if database entry missing

**Specification Reference:** Lines 91-94

### FR-005: Backward Compatibility
**Priority:** MEDIUM
**Description:** Existing database schema unchanged
**Details:**
- Existing `settings` table entries used as-is
- No API breaking changes during migration
- Seamless transition from current implementation

**Specification Reference:** Lines 96-99

## Non-Functional Requirements

### NFR-001: Performance
**Priority:** HIGH
**Description:** Minimal overhead for parameter access
**Details:**
- `RwLock::read()` overhead < 10ns (uncontended)
- No performance regression vs current Arc<RwLock> pattern
- Zero allocation on parameter read
- Critical for audio hot path

**Specification Reference:** Lines 103-106

### NFR-002: Safety
**Priority:** HIGH
**Description:** No panics in parameter access
**Details:**
- Use `.unwrap()` only where lock poisoning = fatal
- Type-safe parameter values
- Validated ranges on write
- Graceful error handling

**Specification Reference:** Lines 108-111

### NFR-003: Testability
**Priority:** HIGH
**Description:** Comprehensive testing at each migration step
**Details:**
- Each parameter migration verified by full test suite
- Unit tests for parameter access patterns
- Integration tests for database initialization
- Regression tests for hardcoded values

**Specification Reference:** Lines 113-116

### NFR-004: Maintainability
**Priority:** MEDIUM
**Description:** Clear documentation and traceability
**Details:**
- Clear documentation of each parameter's purpose
- Traceability to SPEC016 requirement IDs
- Migration path documented for each parameter
- Code comments reference DBD-PARAM-XXX tags

**Specification Reference:** Lines 118-121

## Requirements Summary

**Total Requirements:** 9 (5 functional, 4 non-functional)
**Critical Priority:** 1 (FR-003)
**High Priority:** 6 (FR-001, FR-002, FR-004, NFR-001, NFR-002, NFR-003)
**Medium Priority:** 2 (FR-005, NFR-004)

## Parameter Inventory (15 Parameters)

| ID | Parameter Name | Type | Default | Risk Tier |
|----|---------------|------|---------|-----------|
| DBD-PARAM-010 | volume_level | f32 | 0.5 | Tier 1 (Low) |
| DBD-PARAM-020 | working_sample_rate | u32 | 44100 | Tier 3 (High) |
| DBD-PARAM-030 | output_ringbuffer_size | usize | 88200 | Tier 2 (Medium) |
| DBD-PARAM-040 | output_refill_period | u64 | 90 | Tier 3 (High) |
| DBD-PARAM-050 | maximum_decode_streams | usize | 12 | Tier 1 (Low) |
| DBD-PARAM-060 | decode_work_period | u64 | 5000 | Tier 1 (Low) |
| DBD-PARAM-065 | decode_chunk_size | usize | 25000 | Tier 3 (High) |
| DBD-PARAM-070 | playout_ringbuffer_size | usize | 661941 | Tier 2 (Medium) |
| DBD-PARAM-080 | playout_ringbuffer_headroom | usize | 4410 | Tier 2 (Medium) |
| DBD-PARAM-085 | decoder_resume_hysteresis_samples | u64 | 44100 | Tier 3 (High) |
| DBD-PARAM-088 | mixer_min_start_level | usize | 22050 | Tier 2 (Medium) |
| DBD-PARAM-090 | pause_decay_factor | f64 | 0.95 | Tier 1 (Low) |
| DBD-PARAM-100 | pause_decay_floor | f64 | 0.0001778 | Tier 1 (Low) |
| DBD-PARAM-110 | audio_buffer_size | u32 | 2208 | Tier 3 (High) |
| DBD-PARAM-111 | mixer_check_interval_ms | u64 | 10 | Tier 3 (High) |
