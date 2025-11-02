# SPEC017 Compliance Remediation Implementation

**Document Type:** Implementation Specification for `/plan` workflow
**Created:** 2025-11-02
**Based On:** [spec017_compliance_review_analysis_results.md](spec017_compliance_review_analysis_results.md)
**Purpose:** Implement fixes for SPEC017 tick-based timing compliance issues

---

## Document Information

**Stakeholders:** WKMP Development Team
**Priority:** HIGH (critical SRC-LAYER-011 violation)
**Timeline:** Single increment (estimated 4-8 hours)
**Breaking Changes:** YES (file duration migration requires database rebuild)

---

## Executive Summary

This specification defines implementation for three compliance fixes based on SPEC017 analysis:

1. **Fix wkmp-dr Developer UI** - Add seconds display per SRC-LAYER-011 (HIGH priority)
2. **Document API Layer Pragmatic Deviation** - Accept milliseconds/seconds in HTTP APIs (decision rationale)
3. **Migrate wkmp-ai File Duration to Ticks** - Change from f64 seconds to i64 ticks (MEDIUM priority, breaking)

**User Decisions Applied:**
- **API Layer Philosophy:** Option B - Accept milliseconds/seconds as pragmatic; document deviation and ensure clear unit labeling
- **wkmp-dr Display Format:** Option A - Dual display format `141120000 (5.000000s)`
- **File Duration Migration:** Option A - Migrate to i64 ticks (breaking change, requires database rebuild)

---

## Requirements

### Functional Requirements

#### REQ-F-001: wkmp-dr Dual Time Display

**Priority:** HIGH
**Source:** [SRC-LAYER-011](../docs/SPEC017-sample_rate_conversion.md#developer-facing-layers-use-ticks)

**Requirement:**
Developer UI SHALL display timing values in both ticks and seconds simultaneously.

**Acceptance Criteria:**
- Timing columns display format: `{ticks} ({seconds}s)`
- Example: `141120000 (5.000000s)`
- Applies to all 6 passage timing fields:
  - `start_time_ticks`
  - `end_time_ticks`
  - `fade_in_start_ticks`
  - `fade_out_start_ticks`
  - `lead_in_start_ticks`
  - `lead_out_start_ticks`
- Decimal precision: 6 places (microsecond precision)
- NULL values display as: `null` (no conversion)

**Rationale:**
Violates current requirement [SRC-LAYER-011]: "Developer UI displays both ticks AND seconds for developer inspection"

---

#### REQ-F-002: API Timing Unit Documentation

**Priority:** MEDIUM
**Source:** User decision (API Layer Philosophy Option B)

**Requirement:**
All API timing parameters SHALL be clearly documented with units in code comments and function documentation.

**Acceptance Criteria:**
- Every API request/response struct with timing fields has doc comments indicating units
- Example format:
  ```rust
  /// Position in milliseconds from passage start
  ///
  /// Note: WKMP uses tick-based timing internally (28,224,000 Hz) for sample-accuracy.
  /// API layer uses milliseconds for HTTP ergonomics. See SPEC017 for details.
  pub position_ms: u64,
  ```
- Function parameters with timing use unit suffixes (`_ms`, `_ticks`, `_seconds`)
- Error messages reference correct units
- SPEC017 updated with section documenting pragmatic API deviation

**Rationale:**
Accept milliseconds/seconds as pragmatic for HTTP APIs while ensuring clarity through documentation

---

#### REQ-F-003: File Duration Migration to Ticks

**Priority:** MEDIUM (Breaking Change)
**Source:** Consistency with passage timing representation

**Requirement:**
File duration SHALL be stored as i64 ticks, not f64 seconds.

**Acceptance Criteria:**
- `AudioFile.duration` field type changed from `Option<f64>` to `Option<i64>`
- Field renamed: `duration` → `duration_ticks`
- Database schema updated: `duration REAL` → `duration_ticks INTEGER`
- All duration calculations use tick-based values
- File import converts metadata duration to ticks via `seconds_to_ticks()`
- Breaking change documented in migration notes

**Rationale:**
Consistency with passage timing representation; eliminates floating-point precision loss

---

#### REQ-F-004: Variable Naming Clarity

**Priority:** LOW
**Source:** Code documentation review

**Requirement:**
All timing-related variables SHALL have clear unit indicators.

**Acceptance Criteria:**
- Variable names use unit suffixes: `position_ms`, `duration_ticks`, `sample_count`
- OR variables without suffixes have inline comments: `let position = 0; // ticks, SPEC017`
- Applies to:
  - `wkmp-ap/src/playback/pipeline/timing.rs`
  - `wkmp-ai/src/services/silence_detector.rs`
  - Any other timing-critical code

**Rationale:**
Improve code maintainability and prevent unit confusion

---

### Non-Functional Requirements

#### REQ-NF-001: Test Coverage

**Requirement:**
All changes SHALL have automated test coverage per traceability matrix.

**Acceptance Criteria:**
- wkmp-dr: UI rendering test verifies dual display format
- wkmp-ai: Database roundtrip test verifies tick storage
- wkmp-ai: File duration conversion test verifies `seconds_to_ticks()` accuracy
- Integration test: Create file → import → verify duration_ticks matches expected value

---

#### REQ-NF-002: Documentation Updates

**Requirement:**
All documentation SHALL be updated to reflect implementation changes.

**Acceptance Criteria:**
- SPEC017 updated with API layer deviation section
- IMPL001 database schema updated with `duration_ticks` field
- Code documentation follows variable naming standards
- Migration notes document breaking change

---

#### REQ-NF-003: Backward Compatibility

**Requirement:**
Breaking changes SHALL be clearly documented with migration path.

**Acceptance Criteria:**
- Database rebuild instructions provided
- Users informed that existing databases must be recreated
- No automated migration (acceptable per user decision)

---

## Technical Design

### Design Overview

Three independent changes with minimal interdependencies:

1. **wkmp-dr UI Enhancement** - Frontend JavaScript changes only
2. **API Documentation** - Code comments and SPEC017 updates
3. **File Duration Migration** - Database schema + model changes (breaking)

### Component Architecture

```
┌─────────────────────────────────────────────────┐
│ wkmp-dr UI (Change #1)                         │
│ ├─ app.js: renderTable() function             │
│ │  └─ Add tick-to-seconds conversion          │
│ └─ Tests: Verify dual display format           │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│ API Documentation (Change #2)                   │
│ ├─ wkmp-ap/src/api/handlers.rs                 │
│ │  └─ Add timing unit doc comments             │
│ ├─ wkmp-ai/src/api/amplitude_analysis.rs       │
│ │  └─ Add timing unit doc comments             │
│ └─ docs/SPEC017-sample_rate_conversion.md      │
│    └─ Add "API Layer Pragmatic Deviation" §    │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│ File Duration Migration (Change #3)             │
│ ├─ wkmp-ai/src/db/files.rs                     │
│ │  ├─ AudioFile.duration → duration_ticks      │
│ │  └─ Type: Option<f64> → Option<i64>          │
│ ├─ wkmp-common/src/db/init.rs                  │
│ │  └─ Schema: duration REAL → duration_ticks   │
│ ├─ wkmp-ai import workflow                     │
│ │  └─ Convert metadata duration via            │
│ │     seconds_to_ticks()                       │
│ └─ Tests: Verify tick storage/retrieval        │
└─────────────────────────────────────────────────┘
```

---

## Detailed Design

### Change #1: wkmp-dr Dual Time Display

#### Files Modified

**Primary:**
- `wkmp-dr/src/ui/app.js` (lines 346-386)

**Testing:**
- `wkmp-dr/tests/ui_display_tests.rs` (new file)

#### Implementation Details

**JavaScript Constants (add at top of file):**
```javascript
// SPEC017 tick-based timing constants
const TICK_RATE = 28224000;  // 28.224 MHz, see SPEC017 SRC-TICK-020

// Timing columns requiring dual display (ticks + seconds)
const TIMING_COLUMNS = [
    'start_time_ticks',
    'end_time_ticks',
    'fade_in_start_ticks',
    'fade_out_start_ticks',
    'lead_in_start_ticks',
    'lead_out_start_ticks'
];
```

**Conversion Helper Function:**
```javascript
/**
 * Convert ticks to seconds per SPEC017
 * @param {number} ticks - Tick value (28,224,000 Hz)
 * @returns {string} Formatted seconds with 6 decimal places
 */
function ticksToSeconds(ticks) {
    const seconds = ticks / TICK_RATE;
    return seconds.toFixed(6);
}
```

**Modified renderTable() Function (lines 369-379):**
```javascript
function renderTable(data) {
    const container = document.getElementById('tableContainer');

    // ... existing validation code ...

    // Render table headers
    let html = '<table><thead><tr>';
    data.columns.forEach(col => {
        const isDereferenced = dereferencedCols.has(col);
        const className = isDereferenced ? ' class="dereferenced"' : '';
        html += `<th${className}>${col}</th>`;
    });
    html += '</tr></thead><tbody>';

    // Render table rows with timing conversion
    data.rows.forEach(row => {
        html += '<tr>';
        row.forEach((cell, index) => {
            const colName = data.columns[index];
            const isDereferenced = dereferencedCols.has(colName);
            const className = isDereferenced ? ' class="dereferenced"' : '';

            // Apply dual display for timing columns [REQ-F-001]
            if (TIMING_COLUMNS.includes(colName) && cell !== null) {
                const ticks = parseInt(cell);
                const seconds = ticksToSeconds(ticks);
                html += `<td${className}>${cell} (${seconds}s)</td>`;
            } else {
                const value = cell === null ? '<em>null</em>' : String(cell);
                html += `<td${className}>${value}</td>`;
            }
        });
        html += '</tr>';
    });

    html += '</tbody></table>';
    container.innerHTML = html;
}
```

**CSS Enhancement (optional, for visual clarity):**
```css
/* Timing column styling */
td.timing-value {
    font-family: 'Courier New', monospace;
    white-space: nowrap;
}

td.timing-value .seconds {
    color: #666;
    font-size: 0.9em;
    margin-left: 0.3em;
}
```

With updated markup:
```javascript
html += `<td${className} class="timing-value">${cell} <span class="seconds">(${seconds}s)</span></td>`;
```

---

### Change #2: API Timing Unit Documentation

#### Files Modified

**Primary:**
- `wkmp-ap/src/api/handlers.rs` (doc comments)
- `wkmp-ai/src/api/amplitude_analysis.rs` (doc comments)
- `docs/SPEC017-sample_rate_conversion.md` (new section)

#### Implementation Details

**wkmp-ap API Documentation Example:**
```rust
/// Playback position response
///
/// Returns current playback state with timing in milliseconds.
///
/// Note: WKMP uses tick-based timing internally (28,224,000 Hz) per SPEC017
/// for sample-accurate precision. The API layer uses milliseconds for HTTP
/// ergonomics while preserving internal precision.
#[derive(Debug, Serialize)]
pub struct PositionResponse {
    /// Current passage being played (if any)
    pub passage_id: Option<Uuid>,

    /// Current playback position in milliseconds from passage start
    ///
    /// Converted from internal tick representation via ticks_to_ms().
    /// Precision: ~0.035ms (28,224 ticks per millisecond)
    pub position_ms: u64,

    /// Total passage duration in milliseconds
    ///
    /// Converted from internal tick representation via ticks_to_ms().
    pub duration_ms: u64,

    /// Playback state: "playing", "paused", or "stopped"
    pub state: String,
}
```

**wkmp-ai API Documentation Example:**
```rust
/// Amplitude analysis request parameters
///
/// Specifies the time range for amplitude analysis.
///
/// Note: WKMP uses tick-based timing internally (28,224,000 Hz) per SPEC017.
/// This API uses seconds for ergonomics. Values are converted to ticks
/// internally via seconds_to_ticks().
#[derive(Debug, Deserialize)]
pub struct AmplitudeAnalysisRequest {
    /// Analysis start time in seconds from file start
    ///
    /// Converted to ticks internally for sample-accurate processing.
    /// Default: 0.0 (file start)
    #[serde(default)]
    pub start_time: f64,  // seconds

    /// Analysis end time in seconds from file start
    ///
    /// If None, analyzes to file end.
    /// Converted to ticks internally for sample-accurate processing.
    pub end_time: Option<f64>,  // seconds

    // ... other fields
}
```

**SPEC017 New Section (add after existing SRC-API-030):**

```markdown
### API Layer Pragmatic Deviation

**[SRC-API-040]** WKMP HTTP APIs use **milliseconds** (wkmp-ap) or **seconds** (wkmp-ai)
instead of raw tick values for ergonomic reasons:

**Rationale:**
- Raw tick values (e.g., `141120000`) are less intuitive for HTTP API consumers
- Milliseconds (`5000`) and seconds (`5.0`) provide human-readable timing
- Internal tick precision is preserved through conversion layer
- API-to-database flow: `ms → ms_to_ticks() → i64 storage`
- Database-to-API flow: `i64 storage → ticks_to_ms() → ms`

**Precision:**
- Milliseconds: ~0.035ms precision (28,224 ticks/ms)
- Adequate for API layer while maintaining sample-accurate internal timing

**Conversion Guarantees:**
- `ms → ticks → ms` roundtrip: Exact for all millisecond values
- `seconds → ticks → seconds` roundtrip: Within 1 tick (~35ns error)

**Documentation Requirement:**
All API timing parameters MUST be documented with units in code comments.

**Related:**
- [SRC-API-010] Original specification (raw ticks)
- [SRC-API-020, 030] Conversion functions used
- [REQ-F-002] Documentation requirement
```

---

### Change #3: File Duration Migration to Ticks

#### Files Modified

**Primary:**
- `wkmp-ai/src/db/files.rs` (struct + queries)
- `wkmp-common/src/db/init.rs` (schema)
- `wkmp-ai/src/services/metadata_extractor.rs` (conversion)
- `docs/IMPL001-database_schema.md` (schema documentation)

**Testing:**
- `wkmp-ai/tests/file_duration_tests.rs` (new file)

#### Implementation Details

**AudioFile Struct Change:**

**Before:**
```rust
pub struct AudioFile {
    pub guid: Uuid,
    pub path: String,
    pub duration: Option<f64>,  // seconds
    pub sample_rate: Option<u32>,
    // ... other fields
}
```

**After:**
```rust
pub struct AudioFile {
    pub guid: Uuid,
    pub path: String,

    /// File duration in ticks (28,224,000 Hz)
    ///
    /// Stored as i64 ticks per SPEC017 for consistency with passage timing.
    /// Convert to seconds via: wkmp_common::timing::ticks_to_seconds()
    ///
    /// [REQ-F-003] Migration from f64 seconds to i64 ticks
    pub duration_ticks: Option<i64>,  // ticks, not seconds

    pub sample_rate: Option<u32>,
    // ... other fields
}
```

**Database Schema Change (wkmp-common/src/db/init.rs):**

**Before:**
```sql
CREATE TABLE files (
    guid TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    duration REAL,                    -- f64 seconds
    sample_rate INTEGER,
    -- ... other columns
)
```

**After:**
```sql
CREATE TABLE files (
    guid TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    duration_ticks INTEGER,           -- i64 ticks (28,224,000 Hz)
    sample_rate INTEGER,
    -- ... other columns
)
```

**Metadata Extractor Conversion:**

**File:** `wkmp-ai/src/services/metadata_extractor.rs`

**Before:**
```rust
let duration_sec = // extract from metadata
AudioFile {
    duration: Some(duration_sec),
    // ...
}
```

**After:**
```rust
use wkmp_common::timing::seconds_to_ticks;

let duration_sec = // extract from metadata
let duration_ticks = seconds_to_ticks(duration_sec);

AudioFile {
    duration_ticks: Some(duration_ticks),  // [REQ-F-003]
    // ...
}
```

**Database Query Updates:**

All SQL queries must use `duration_ticks` instead of `duration`:
- INSERT statements
- SELECT statements
- UPDATE statements (if any)

**Example:**
```rust
// Before
sqlx::query!("SELECT guid, path, duration FROM files WHERE guid = ?", guid)

// After
sqlx::query!("SELECT guid, path, duration_ticks FROM files WHERE guid = ?", guid)
```

---

### Change #4: Variable Naming Clarity (Low Priority)

#### Files Modified

**Primary:**
- `wkmp-ap/src/playback/pipeline/timing.rs`
- `wkmp-ai/src/services/silence_detector.rs`

#### Implementation Details

**Add inline comments for ambiguous variables:**

**Example (timing.rs):**
```rust
// Before
let start_time = passage.start_time;
let duration = 5000;

// After
let start_time = passage.start_time;  // u64 milliseconds from passage start
let duration = 5000;  // u32 milliseconds, crossfade overlap time
```

**Example (silence_detector.rs):**
```rust
// Before
pub struct SilenceRegion {
    pub start_seconds: f32,
    pub end_seconds: f32,
}

// After
pub struct SilenceRegion {
    /// Silence region start time in seconds (f32 for memory efficiency)
    ///
    /// Note: WKMP uses tick-based timing for passages. This f32 representation
    /// provides adequate precision (~300μs at 1 hour) for silence detection.
    pub start_seconds: f32,

    /// Silence region end time in seconds (f32 for memory efficiency)
    pub end_seconds: f32,
}
```

---

## Test Specifications

### Test Categories

Per `/plan` workflow requirements, tests are organized by:
- **Unit Tests:** Individual function/component testing
- **Integration Tests:** Multi-component interaction testing
- **Acceptance Tests:** End-to-end requirement validation

---

### Unit Tests

#### UT-001: wkmp-dr Tick-to-Seconds Conversion

**File:** `wkmp-dr/tests/ui_display_tests.rs` (new)

**Test Case:**
```rust
#[test]
fn test_ticks_to_seconds_display_format() {
    // Known conversions per SPEC017
    let test_cases = vec![
        (0_i64, "0.000000"),                    // Zero
        (28_224_000, "1.000000"),               // 1 second
        (141_120_000, "5.000000"),              // 5 seconds
        (2_822_400_000, "100.000000"),          // 100 seconds
    ];

    for (ticks, expected_seconds) in test_cases {
        let formatted = format_timing_value(ticks);
        let expected = format!("{} ({}s)", ticks, expected_seconds);
        assert_eq!(formatted, expected,
                   "Tick value {} should format to {}", ticks, expected);
    }
}
```

**Traceability:** [REQ-F-001], [SRC-LAYER-011]

---

#### UT-002: File Duration Roundtrip Test

**File:** `wkmp-ai/tests/file_duration_tests.rs` (new)

**Test Case:**
```rust
use wkmp_common::timing::{seconds_to_ticks, ticks_to_seconds};

#[test]
fn test_file_duration_ticks_roundtrip() {
    let test_durations_seconds = vec![
        0.0,      // Zero duration
        1.5,      // 1.5 seconds
        180.0,    // 3 minutes
        3600.0,   // 1 hour
        7200.5,   // 2 hours 0.5 seconds
    ];

    for original_seconds in test_durations_seconds {
        // Convert seconds → ticks (import path)
        let duration_ticks = seconds_to_ticks(original_seconds);

        // Convert ticks → seconds (display path)
        let recovered_seconds = ticks_to_seconds(duration_ticks);

        // Verify precision within 1 tick (~35ns)
        let error = (original_seconds - recovered_seconds).abs();
        assert!(error < 0.000001,
                "Roundtrip error {} exceeds tolerance for {}s",
                error, original_seconds);
    }
}
```

**Traceability:** [REQ-F-003], [SRC-TIME-010], [SRC-TIME-020]

---

### Integration Tests

#### IT-001: File Import with Tick Duration

**File:** `wkmp-ai/tests/import_integration_tests.rs` (add to existing)

**Test Case:**
```rust
#[tokio::test]
async fn test_file_import_stores_duration_as_ticks() {
    let pool = setup_test_database().await;

    // Create test audio file with known duration
    let test_file_path = create_test_audio_file(5.5); // 5.5 seconds

    // Import file
    let result = import_audio_file(&pool, &test_file_path).await;
    assert!(result.is_ok(), "Import should succeed");

    let file_id = result.unwrap();

    // Retrieve from database
    let file = load_audio_file(&pool, file_id).await.unwrap();

    // Verify duration stored as ticks
    assert!(file.duration_ticks.is_some(), "Duration should be stored");

    let expected_ticks = seconds_to_ticks(5.5);
    assert_eq!(file.duration_ticks.unwrap(), expected_ticks,
               "Duration should match expected tick value");

    // Verify conversion back to seconds
    let recovered_seconds = ticks_to_seconds(file.duration_ticks.unwrap());
    assert!((recovered_seconds - 5.5).abs() < 0.000001,
            "Recovered seconds should match original");
}
```

**Traceability:** [REQ-F-003], [REQ-NF-001]

---

#### IT-002: wkmp-dr Display Rendering

**File:** `wkmp-dr/tests/display_integration_tests.rs` (new)

**Test Case:**
```rust
#[tokio::test]
async fn test_passages_table_displays_dual_timing() {
    let pool = setup_test_database().await;

    // Create passage with known timing
    let passage = create_test_passage_with_timing(
        seconds_to_ticks(2.5),  // start: 2.5s
        seconds_to_ticks(7.5),  // end: 7.5s
    );
    save_passage(&pool, &passage).await.unwrap();

    // Fetch passages table via API
    let response = fetch_table_data(&pool, "passages").await.unwrap();

    // Verify timing columns include seconds
    let start_time_cell = response.rows[0]["start_time_ticks"];
    assert!(start_time_cell.contains("(2.500000s)"),
            "Start time should display seconds: {}", start_time_cell);

    let end_time_cell = response.rows[0]["end_time_ticks"];
    assert!(end_time_cell.contains("(7.500000s)"),
            "End time should display seconds: {}", end_time_cell);
}
```

**Traceability:** [REQ-F-001], [SRC-LAYER-011]

---

### Acceptance Tests

#### AT-001: Developer UI Compliance

**Requirement:** [REQ-F-001] - wkmp-dr displays both ticks and seconds

**Given:** Database contains passages with timing values
**When:** Developer opens wkmp-dr and views passages table
**Then:** All timing columns display format `{ticks} ({seconds}s)`

**Test Implementation:**
```rust
#[tokio::test]
async fn acceptance_developer_ui_dual_display() {
    // Setup
    let pool = setup_test_database().await;
    create_sample_passages(&pool).await;

    // Execute
    let html = render_passages_table(&pool).await.unwrap();

    // Verify
    assert!(html.contains("141120000 (5.000000s)"),
            "HTML should contain dual display format");
    assert!(html.contains("282240000 (10.000000s)"),
            "Multiple timing values should use dual format");
}
```

**Pass Criteria:**
- All 6 timing columns use dual display
- NULL values display as "null" without conversion
- Decimal precision is 6 places
- Format matches exactly: `{integer} ({decimal}s)`

---

#### AT-002: File Duration Storage Consistency

**Requirement:** [REQ-F-003] - File duration stored as i64 ticks

**Given:** Audio file with 5.5 second duration
**When:** File is imported via wkmp-ai
**Then:** Database stores duration as 155,232,000 ticks (exactly)

**Test Implementation:**
```rust
#[tokio::test]
async fn acceptance_file_duration_ticks_storage() {
    let pool = setup_test_database().await;

    // Import 5.5 second file
    let file_id = import_test_file(&pool, "test_5.5s.mp3").await.unwrap();

    // Query database directly
    let row = sqlx::query!("SELECT duration_ticks FROM files WHERE guid = ?", file_id)
        .fetch_one(&pool).await.unwrap();

    let expected_ticks = seconds_to_ticks(5.5);  // 155,232,000
    assert_eq!(row.duration_ticks.unwrap(), expected_ticks,
               "Database should store exact tick value");
}
```

**Pass Criteria:**
- Database column type is INTEGER (not REAL)
- Stored value matches `seconds_to_ticks()` exactly
- No floating-point precision loss
- Retrieval yields identical tick value

---

#### AT-003: API Documentation Completeness

**Requirement:** [REQ-F-002] - API timing parameters clearly documented

**Given:** Developer reviews API code
**When:** Reading function/struct documentation
**Then:** All timing parameters have unit indicators

**Manual Verification Checklist:**
- [ ] `PositionResponse` struct has timing field doc comments
- [ ] `SeekRequest` struct has timing field doc comments
- [ ] `AmplitudeAnalysisRequest` struct has timing field doc comments
- [ ] All timing parameters use unit suffixes (`_ms`, `_ticks`, `_seconds`)
- [ ] Doc comments reference SPEC017 for tick conversion details

**Pass Criteria:**
- 100% of API timing fields documented
- Unit suffixes present in variable names
- Doc comments explain internal tick representation
- SPEC017 cross-references included

---

## Traceability Matrix

| Requirement | Design Section | Test Cases | Status |
|-------------|----------------|------------|--------|
| REQ-F-001 (wkmp-dr dual display) | Change #1 | UT-001, IT-002, AT-001 | ✅ Complete |
| REQ-F-002 (API documentation) | Change #2 | AT-003 (manual) | ✅ Complete |
| REQ-F-003 (file duration ticks) | Change #3 | UT-002, IT-001, AT-002 | ✅ Complete |
| REQ-F-004 (variable naming) | Change #4 | (code review) | ✅ Complete |
| REQ-NF-001 (test coverage) | All changes | All test cases | ✅ Complete |
| REQ-NF-002 (documentation) | All changes | (review) | ✅ Complete |
| REQ-NF-003 (breaking changes) | Change #3 | Migration notes | ✅ Complete |

**Coverage:** 100% of functional requirements traced to tests

---

## Implementation Notes

### Breaking Changes

**File Duration Migration:**
- Requires database rebuild (existing databases incompatible)
- No automated migration provided (acceptable per user decision)
- Users must delete existing `wkmp.db` and re-import files
- Document in release notes

**Migration Instructions:**
```bash
# Backup existing database (optional)
cp ~/Music/wkmp.db ~/Music/wkmp.db.backup

# Delete database (breaking change)
rm ~/Music/wkmp.db

# Restart wkmp-ai
# Database will be recreated with new schema

# Re-import all audio files via wkmp-ai import wizard
```

---

### Testing Strategy

**Phase 1: Unit Tests**
- Implement UT-001, UT-002 first
- Verify tick conversion accuracy
- Fast feedback loop (<1 second per test)

**Phase 2: Integration Tests**
- Implement IT-001, IT-002
- Requires test database setup
- Slower feedback (~5 seconds per test)

**Phase 3: Acceptance Tests**
- Implement AT-001, AT-002, AT-003
- End-to-end validation
- Manual AT-003 checklist review

**Phase 4: Manual Verification**
- Build wkmp-dr UI and visually inspect timing display
- Import sample audio file and verify tick storage
- Review all API documentation changes

---

### Documentation Updates

**Files to Update:**

1. **SPEC017-sample_rate_conversion.md**
   - Add [SRC-API-040] section documenting pragmatic deviation
   - Update examples to show millisecond API usage

2. **IMPL001-database_schema.md**
   - Update `files` table schema with `duration_ticks INTEGER`
   - Remove `duration REAL` reference

3. **CHANGELOG.md** (if exists)
   - Document breaking change in file duration storage
   - Provide migration instructions

4. **README.md** (if exists)
   - Note database rebuild required for this version

---

## Risk Assessment

### Risk #1: Database Rebuild Friction

**Failure Mode:** Users frustrated by database rebuild requirement
**Probability:** Medium
**Impact:** Low (one-time inconvenience)
**Mitigation:**
- Clear migration instructions in release notes
- Automated database backup before deletion
- Import wizard UX improvements

**Residual Risk:** Low

---

### Risk #2: wkmp-dr Display Performance

**Failure Mode:** Tick-to-seconds conversion slows table rendering
**Probability:** Low
**Impact:** Low (JavaScript division is fast)
**Mitigation:**
- Conversion is simple division (1 operation per cell)
- Typical passages table <100 rows
- Rendering remains <100ms

**Residual Risk:** Very Low

---

### Risk #3: Floating-Point to Integer Precision Loss

**Failure Mode:** Existing file durations truncated during migration
**Probability:** N/A (no automated migration)
**Impact:** N/A
**Mitigation:**
- Fresh import from source files preserves precision
- Users re-import original audio files

**Residual Risk:** N/A

---

## Success Criteria

Implementation is successful when:

1. ✅ wkmp-dr displays all timing values in dual format `{ticks} ({seconds}s)`
2. ✅ All API timing parameters documented with units
3. ✅ SPEC017 updated with API deviation section
4. ✅ File duration stored as i64 ticks in database
5. ✅ All test cases pass (100% traceability)
6. ✅ Documentation updated (SPEC017, IMPL001)
7. ✅ Breaking change migration instructions provided
8. ✅ Code review confirms variable naming clarity improvements

---

## Open Questions

None - all decisions made by user.

---

## Related Documentation

- [SPEC017-sample_rate_conversion.md](../docs/SPEC017-sample_rate_conversion.md) - Tick-based timing specification
- [SPEC023-timing_terminology.md](../docs/SPEC023-timing_terminology.md) - Four timing types
- [IMPL001-database_schema.md](../docs/IMPL001-database_schema.md) - Database schema reference
- [spec017_compliance_review_analysis_results.md](spec017_compliance_review_analysis_results.md) - Original analysis
