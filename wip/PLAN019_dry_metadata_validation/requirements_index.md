# PLAN019: Requirements Index

**Purpose:** Quick reference table of all requirements for DRY metadata validation implementation

**Total Requirements:** 10 (3 Critical/High functional, 4 High refactoring, 3 Medium)

---

## Requirements Table

| Req ID | Type | Priority | Brief Description | Impact | Source Line # |
|--------|------|----------|-------------------|--------|---------------|
| REQ-DRY-010 | Functional | High | Create ParamMetadata struct with all 15 parameter definitions | +150 LOC | Analysis doc |
| REQ-DRY-020 | Functional | High | Implement GlobalParams::metadata() static accessor method | +20 LOC | Analysis doc |
| REQ-DRY-030 | Functional | High | Add validation closure to ParamMetadata for each parameter | Incl. in 010 | Analysis doc |
| REQ-DRY-040 | Refactoring | High | Refactor init_from_database() to use metadata validators (eliminate duplication) | ~0 (replace) | Analysis doc |
| REQ-DRY-050 | Refactoring | High | Refactor 15 setter methods to delegate to metadata validators (DRY) | ~-80 LOC | Analysis doc |
| REQ-DRY-060 | Functional | **Critical** | Add server-side validation to bulk_update_settings() API handler | +30 LOC | Analysis doc |
| REQ-DRY-070 | Quality | **Critical** | Prevent invalid database writes (enforce validation before DB write) | Part of 060 | Analysis doc |
| REQ-DRY-080 | Refactoring | Medium | Refactor wkmp-ap get_volume/set_volume to use metadata validators | ~-10 LOC | Analysis doc |
| REQ-DRY-090 | Testing | High | Maintain 100% test coverage for all refactored code (24 existing + 10 new) | +50 LOC tests | Analysis doc |
| REQ-DRY-100 | Documentation | Medium | Document metadata-based validation pattern (module + struct + API) | +30 LOC docs | Analysis doc |

---

## Requirements by Priority

### Critical (2)
- **REQ-DRY-060:** Add API validation to prevent database corruption
- **REQ-DRY-070:** Enforce validation before database write

### High (6)
- **REQ-DRY-010:** Create ParamMetadata struct
- **REQ-DRY-020:** Implement metadata accessor
- **REQ-DRY-030:** Add validation closures
- **REQ-DRY-040:** Refactor database loading
- **REQ-DRY-050:** Refactor setter methods
- **REQ-DRY-090:** Maintain test coverage

### Medium (2)
- **REQ-DRY-080:** Refactor volume functions
- **REQ-DRY-100:** Documentation

---

## Requirements by Category

### Metadata Infrastructure (3)
- REQ-DRY-010: ParamMetadata struct
- REQ-DRY-020: metadata() accessor
- REQ-DRY-030: Validation closures

### Refactoring (3)
- REQ-DRY-040: Refactor database loading
- REQ-DRY-050: Refactor setters
- REQ-DRY-080: Refactor volume functions

### API Validation (2)
- REQ-DRY-060: Add API validation
- REQ-DRY-070: Prevent invalid writes

### Quality Assurance (2)
- REQ-DRY-090: Test coverage
- REQ-DRY-100: Documentation

---

## Detailed Requirements

### REQ-DRY-010: Create ParamMetadata Struct

**Priority:** High
**Type:** Functional
**Inputs:** None (static data structure)
**Outputs:** ParamMetadata struct with 6 fields

**Description:**
Create a `ParamMetadata` struct that encapsulates all metadata for a single GlobalParam parameter:
- `key: &'static str` - Parameter name (e.g., "volume_level")
- `data_type: &'static str` - Rust type as string (e.g., "f32")
- `default_value: &'static str` - Default as string (e.g., "0.5")
- `description: &'static str` - Human-readable description
- `validation_range: &'static str` - Valid range as string (e.g., "0.0-1.0")
- `validator: fn(&str) -> Result<(), String>` - Validation closure

**Must define metadata for all 15 GlobalParams parameters:**
1. volume_level
2. working_sample_rate
3. output_ringbuffer_size
4. maximum_decode_streams
5. decode_work_period
6. chunk_duration_ms
7. playout_ringbuffer_size
8. playout_ringbuffer_headroom
9. decoder_resume_hysteresis_samples
10. mixer_min_start_level
11. pause_decay_factor
12. pause_decay_floor
13. audio_buffer_size
14. mixer_check_interval_ms
15. (Note: Only 14 parameters currently, not 15 - verify exact count)

**Acceptance Criteria:**
- ParamMetadata struct compiles with all 6 fields
- All 15 parameter definitions created
- No compilation errors

**Dependencies:** None

---

### REQ-DRY-020: Implement metadata() Accessor

**Priority:** High
**Type:** Functional
**Inputs:** None (static method call)
**Outputs:** `&'static [ParamMetadata]` slice

**Description:**
Implement a static method `GlobalParams::metadata()` that returns a reference to a static array of ParamMetadata entries. This provides the single source of truth for all parameter metadata.

**Signature:**
```rust
impl GlobalParams {
    pub fn metadata() -> &'static [ParamMetadata] {
        // Return static array
    }
}
```

**Acceptance Criteria:**
- Method returns `&'static [ParamMetadata]`
- Same pointer returned on multiple calls (true static)
- Non-empty slice
- Compiles without errors

**Dependencies:** REQ-DRY-010 (needs ParamMetadata struct)

---

### REQ-DRY-030: Add Validation Closures

**Priority:** High
**Type:** Functional
**Inputs:** String value to validate
**Outputs:** `Result<(), String>` (Ok or error message)

**Description:**
For each of the 15 parameters, implement a validation closure with signature `fn(&str) -> Result<(), String>`. Each validator must:
1. Parse input string to expected type (f32, u32, u64, usize, f64)
2. Check value is within valid range
3. Return Ok(()) if valid
4. Return Err(msg) if invalid, with clear error message

**Error Message Format (Resolution for HIGH-001):**
Standard format: `"{param_name}: {specific_reason}"`

Example:
```
"volume_level: value 2.0 out of range [0.0, 1.0]"
"working_sample_rate: must be one of: 44100, 48000, 88200, 96000"
```

**Acceptance Criteria:**
- All 15 validators accept their default values
- All 15 validators reject out-of-range values
- Error messages include parameter name and reason
- Parse errors handled gracefully

**Dependencies:** Part of REQ-DRY-010

---

### REQ-DRY-040: Refactor init_from_database()

**Priority:** High
**Type:** Refactoring
**Inputs:** sqlx::SqlitePool reference
**Outputs:** Result<(), Box<dyn Error>>

**Description:**
Refactor `GlobalParams::init_from_database()` to use metadata validators instead of duplicated validation logic. Current implementation has ~80 lines of duplicated range checks. New implementation should:
1. Load value from database (existing helper functions)
2. Look up metadata for parameter
3. Call metadata validator
4. Apply via existing setter method (which now delegates to validator)

**Pattern:**
```rust
// Load from DB
match load_f32_param(db_pool, "volume_level").await {
    Ok(Some(value)) => {
        // Validate via metadata (instead of duplicated logic)
        let meta = GlobalParams::metadata().iter()
            .find(|m| m.key == "volume_level")
            .unwrap();

        match (meta.validator)(&value.to_string()) {
            Ok(_) => {
                if let Err(e) = PARAMS.set_volume_level(value) {
                    warn!("{}, using default", e);
                }
            }
            Err(e) => warn!("{}, using default", e),
        }
    }
    Ok(None) => warn!("volume_level not found, using default"),
    Err(e) => warn!("Failed to load volume_level: {}, using default", e),
}
```

**Acceptance Criteria:**
- All 24 existing tests pass
- Invalid database values rejected via metadata validators
- Code size reduced (~80 lines eliminated)
- Maintains existing error handling (warn + default)

**Dependencies:** REQ-DRY-010, REQ-DRY-020 (needs metadata)

---

### REQ-DRY-050: Refactor Setter Methods

**Priority:** High
**Type:** Refactoring
**Inputs:** Typed value (f32, u32, u64, usize, f64)
**Outputs:** `Result<(), String>`

**Description:**
Refactor all 15 setter methods to delegate to metadata validators instead of duplicating range checks. Current setters have hardcoded validation logic. New setters should:
1. Convert typed value to string (use `.to_string()` - Resolution for MEDIUM-001)
2. Look up metadata for parameter
3. Call metadata validator
4. Apply value if validation passes

**Pattern:**
```rust
pub fn set_working_sample_rate(&self, value: u32) -> Result<(), String> {
    // Find metadata
    let meta = Self::metadata().iter()
        .find(|m| m.key == "working_sample_rate")
        .unwrap();

    // Validate via metadata
    (meta.validator)(&value.to_string())?;

    // Apply if valid
    *self.working_sample_rate.write().unwrap() = value;
    Ok(())
}
```

**Edge Cases (MEDIUM-001):**
- f64::INFINITY → `.to_string()` produces "inf", validator detects as parse error
- f64::NaN → `.to_string()` produces "NaN", validator detects as parse error
- Both result in validation error (correct behavior)

**Acceptance Criteria:**
- All 15 setters delegate to metadata validators
- No duplicated range checks in setter code
- Error messages match metadata validator format
- All existing setter tests pass

**Dependencies:** REQ-DRY-010 (needs validator)

---

### REQ-DRY-060: Add API Validation

**Priority:** **Critical**
**Type:** Functional
**Inputs:** HashMap<String, String> from request body
**Outputs:** 200 OK (success) or 400 Bad Request (validation failed)

**Description:**
Add server-side validation to `bulk_update_settings()` API handler to prevent database corruption from invalid values. Currently, handler writes values directly to database without validation, resulting in silent failures at startup.

**New behavior:**
1. For each setting in request:
   - Look up metadata by key
   - Call metadata validator with value
   - Collect error if validation fails
2. If any validation errors:
   - Return 400 Bad Request
   - Include all errors in response (Resolution for HIGH-002)
   - Do NOT write to database
3. If all valid:
   - Write all to database
   - Schedule graceful shutdown (existing behavior)

**Pattern:**
```rust
pub async fn bulk_update_settings(
    State(ctx): State<AppContext>,
    Json(req): Json<BulkUpdateSettingsRequest>,
) -> Result<Json<BulkUpdateSettingsResponse>, (StatusCode, Json<StatusResponse>)> {
    let metadata_map: HashMap<&str, &ParamMetadata> =
        GlobalParams::metadata().iter().map(|m| (m.key, m)).collect();

    let mut errors = Vec::new();

    // Step 1: Validate all settings
    for (key, value) in &req.settings {
        if let Some(meta) = metadata_map.get(key.as_str()) {
            if let Err(e) = (meta.validator)(value) {
                errors.push(format!("{}: {}", key, e));
            }
        } else {
            errors.push(format!("{}: unknown parameter", key));
        }
    }

    // Step 2: If errors, return 400 (do NOT write to DB)
    if !errors.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(StatusResponse {
                status: format!("Validation failed: {}", errors.join(", ")),
            }),
        ));
    }

    // Step 3: All valid, write to database
    // ... (existing write logic)
}
```

**Acceptance Criteria:**
- Invalid values rejected with 400 status
- Error message lists all validation failures (not just first)
- Database unchanged after validation failure
- Valid values processed successfully (200 status)

**Dependencies:** REQ-DRY-020 (needs metadata accessor)

---

### REQ-DRY-070: Prevent Invalid Database Writes

**Priority:** **Critical**
**Type:** Quality
**Inputs:** Same as REQ-DRY-060
**Outputs:** Database contains only valid values

**Description:**
Enforce validation BEFORE database write to prevent invalid values from being persisted. This is the enforcement mechanism for REQ-DRY-060.

**Behavior:**
- Validation failures → No database write
- Partial validation failures → No database write (atomic, all-or-nothing)
- Only write to database after ALL values validated

**Acceptance Criteria:**
- Database unchanged after validation failure
- No partial writes (if 1 of 5 settings invalid, none written)
- Database integrity maintained

**Dependencies:** REQ-DRY-060 (enforcement mechanism)

---

### REQ-DRY-080: Refactor Volume Functions

**Priority:** Medium
**Type:** Refactoring
**Inputs:** Database pool + volume value
**Outputs:** Result<f32> or Result<()>

**Description:**
Refactor `get_volume/set_volume` functions in `wkmp-ap/src/db/settings.rs` to use metadata validators instead of hardcoded `.clamp(0.0, 1.0)` logic. This eliminates duplication with `GlobalParams::set_volume_level()`.

**Current Code (lines 15-35):**
```rust
pub async fn get_volume(db: &Pool<Sqlite>) -> Result<f32> {
    match get_setting::<f32>(db, "volume_level").await? {
        Some(vol) => Ok(vol.clamp(0.0, 1.0)), // DUPLICATED VALIDATION
        None => {
            set_volume(db, 0.5).await?;
            Ok(0.5)
        }
    }
}

pub async fn set_volume(db: &Pool<Sqlite>, volume: f32) -> Result<()> {
    let clamped = volume.clamp(0.0, 1.0); // DUPLICATED VALIDATION
    set_setting(db, "volume_level", clamped).await
}
```

**New Code:**
```rust
pub async fn get_volume(db: &Pool<Sqlite>) -> Result<f32> {
    match get_setting::<f32>(db, "volume_level").await? {
        Some(vol) => {
            // Validate using metadata (not duplicated clamp)
            let meta = GlobalParams::metadata().iter()
                .find(|m| m.key == "volume_level")
                .unwrap();

            match (meta.validator)(&vol.to_string()) {
                Ok(_) => Ok(vol),
                Err(_) => {
                    warn!("Invalid volume in database: {}, using default 0.5", vol);
                    set_volume(db, 0.5).await?;
                    Ok(0.5)
                }
            }
        }
        None => {
            set_volume(db, 0.5).await?;
            Ok(0.5)
        }
    }
}

pub async fn set_volume(db: &Pool<Sqlite>, volume: f32) -> Result<()> {
    // Validate using metadata (not duplicated clamp)
    let meta = GlobalParams::metadata().iter()
        .find(|m| m.key == "volume_level")
        .unwrap();

    (meta.validator)(&volume.to_string())
        .map_err(|e| anyhow::anyhow!("Volume validation failed: {}", e))?;

    set_setting(db, "volume_level", volume).await
}
```

**Acceptance Criteria:**
- Volume validation via metadata (no hardcoded `.clamp()`)
- Existing volume tests pass
- Consistent error handling with other settings

**Dependencies:** REQ-DRY-020 (needs metadata)

---

### REQ-DRY-090: Maintain 100% Test Coverage

**Priority:** High
**Type:** Testing
**Inputs:** Refactored code
**Outputs:** All existing tests pass + new tests for validation paths

**Description:**
Ensure all refactoring maintains existing test coverage (24 tests) and adds new tests for metadata-based validation system.

**Existing Tests (24):**
- Must continue passing after all refactoring
- Located in `wkmp-common/src/params.rs` tests module

**New Tests (10 - Resolution for MEDIUM-002):**
1. TC-U-010-01: ParamMetadata struct definition
2. TC-U-010-02: All 15 parameters in metadata
3. TC-U-020-01: metadata() accessor returns static reference
4. TC-U-030-01: Volume level validator (example)
5. TC-U-030-02: All 15 validators tested
6. TC-I-040-01: Database loading uses metadata validators
7. TC-U-050-01: Setter methods delegate to validators
8. TC-I-060-01/02: API validation (single + batch errors)
9. TC-I-070-01: Database integrity after failed validation
10. TC-I-080-01: Volume functions use metadata

**Acceptance Criteria:**
- All 24 existing tests pass
- All 10 new tests pass
- Total: 34/34 tests passing (100% coverage)

**Dependencies:** All REQ-DRY-xxx (tests verify implementation)

---

### REQ-DRY-100: Documentation

**Priority:** Medium
**Type:** Documentation
**Inputs:** Implemented code
**Outputs:** Rustdoc comments, code examples

**Description:**
Document the metadata-based validation pattern to help future developers understand and extend the system.

**Required Documentation (Resolution for HIGH-003):**

**1. Module-level documentation** (`wkmp-common/src/params.rs`):
```rust
//! # GlobalParams - Centralized Parameter Management
//!
//! This module provides centralized management for all database-backed parameters.
//!
//! ## Metadata-Based Validation Pattern
//!
//! All parameter validation logic is centralized in `ParamMetadata` structures
//! accessible via `GlobalParams::metadata()`. This eliminates duplication across:
//! - Database loading (`init_from_database()`)
//! - Setter methods (`set_volume_level()`, etc.)
//! - API validation (`bulk_update_settings()`)
//!
//! ## Example: Accessing Metadata
//!
//! ```rust
//! let metadata = GlobalParams::metadata();
//! let volume_meta = metadata.iter().find(|m| m.key == "volume_level").unwrap();
//! let result = (volume_meta.validator)("0.5"); // Ok(())
//! let result = (volume_meta.validator)("2.0"); // Err("volume_level: value 2.0 out of range [0.0, 1.0]")
//! ```
```

**2. Struct-level documentation** (`ParamMetadata`):
```rust
/// Metadata for a single GlobalParam parameter
///
/// Encapsulates all information about a parameter including its validation logic.
///
/// # Fields
///
/// - `key`: Parameter name (e.g., "volume_level")
/// - `data_type`: Rust type as string (e.g., "f32")
/// - `default_value`: Default value as string (e.g., "0.5")
/// - `description`: Human-readable description
/// - `validation_range`: Valid range as string (e.g., "0.0-1.0")
/// - `validator`: Closure that validates string input
///
/// # Example
///
/// ```rust
/// let meta = ParamMetadata {
///     key: "volume_level",
///     data_type: "f32",
///     default_value: "0.5",
///     description: "[DBD-PARAM-010] Audio output volume",
///     validation_range: "0.0-1.0",
///     validator: |s| {
///         let v: f32 = s.parse().map_err(|_| "Invalid number")?;
///         if v < 0.0 || v > 1.0 {
///             return Err(format!("volume_level: value {} out of range [0.0, 1.0]", v));
///         }
///         Ok(())
///     },
/// };
/// ```
pub struct ParamMetadata { ... }
```

**3. API handler comments** (`wkmp-ap/src/api/handlers.rs`):
```rust
/// POST /settings/bulk_update - Update multiple settings and trigger graceful shutdown
///
/// **Server-Side Validation:** This handler validates all settings using
/// `GlobalParams::metadata()` BEFORE writing to database. Invalid values
/// are rejected with 400 Bad Request, preventing database corruption.
///
/// ## Validation Flow
///
/// 1. For each setting, look up metadata by key
/// 2. Call `validator` closure to validate value
/// 3. Collect all errors (batch validation)
/// 4. If any errors: Return 400 Bad Request, do NOT write to database
/// 5. If all valid: Write all to database, schedule graceful shutdown
///
/// ## Example Error Response
///
/// ```json
/// {
///   "status": "Validation failed: volume_level: value 2.0 out of range [0.0, 1.0], audio_buffer_size: value 100000 out of range [512, 8192]"
/// }
/// ```
```

**Acceptance Criteria:**
- Module-level docs present and explain pattern
- Struct-level docs present with field descriptions
- API handler comments explain validation flow
- Rustdoc generates without errors (`cargo doc --no-deps`)

**Dependencies:** All implementation complete

---

## Summary Statistics

**Total Requirements:** 10
**By Priority:**
- Critical: 2 (20%)
- High: 6 (60%)
- Medium: 2 (20%)

**By Type:**
- Functional: 4
- Refactoring: 3
- Quality: 1
- Testing: 1
- Documentation: 1

**Estimated Impact:**
- Lines Added: ~200 (metadata + validation + tests + docs)
- Lines Removed: ~160 (duplicated validation/metadata)
- Net Change: +40 lines
- **DRY Benefit:** 3 sources of truth → 1 source of truth
