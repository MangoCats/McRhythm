# Analysis Results: SPEC017 Tick-Based Timing Compliance Review

**Analysis Date:** 2025-11-02
**Document Analyzed:** [wip/spec017_compliance_review.md](spec017_compliance_review.md)
**Analysis Method:** 8-Phase Multi-Agent Workflow (/think command)
**Analyst:** Claude Code (Sonnet 4.5)
**Modules Reviewed:** 7 (wkmp-common, wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le, wkmp-dr)

---

## Executive Summary

### Quick Navigation
- **Modules Reviewed:** 7
- **Issues Found:** 6 (1 HIGH, 2 MEDIUM, 3 LOW)
- **Recommendation:** Fix wkmp-dr HIGH priority issue; accept pragmatic API deviations
- **Overall Status:** ⚠️ Mostly Compliant with pragmatic deviations

### Key Findings

1. **✅ Tick Infrastructure is Excellent** - wkmp-common provides comprehensive tick-based timing system with zero-error conversions
2. **❌ wkmp-dr Non-Compliant** - Violates SRC-LAYER-011 by displaying only ticks (missing seconds)
3. **⚠️ API Layer Deviation** - HTTP APIs use milliseconds/seconds instead of raw ticks (pragmatic but non-compliant with SRC-API-010)
4. **⚠️ wkmp-ai Schema Issue** - File duration uses f64 seconds instead of i64 ticks
5. **✅ Database Schema Fully Compliant** - All passage timing fields use INTEGER ticks per SPEC017

### Critical Issue

**wkmp-dr Database Review UI** violates [SRC-LAYER-011](../docs/SPEC017-sample_rate_conversion.md#developer-facing-layers-use-ticks):
- **Requirement:** "Developer UI displays both ticks AND seconds for developer inspection"
- **Current State:** Displays raw ticks only (e.g., `141120000`)
- **Required State:** Display both (e.g., `141120000 ticks (5.00s)`)

### Recommendation

**Fix HIGH priority issue #1 (wkmp-dr)** - Add seconds conversion in table display

**Accept pragmatic API deviations** - Milliseconds/seconds provide adequate precision for HTTP ergonomics while preserving sample-accuracy internally via tick-based database

### Decisions Required

1. API layer philosophy: Enforce raw ticks vs. accept milliseconds/seconds
2. wkmp-dr display format: Dual display vs. tooltip vs. separate column
3. File duration migration: Migrate to ticks vs. keep f64 seconds

---

## Detailed Analysis

### 1. Current State Assessment

#### Tick Infrastructure (wkmp-common)

**File:** `wkmp-common/src/timing.rs` (560+ lines)

**TICK_RATE:** 28,224,000 Hz
- Least Common Multiple (LCM) of 11 supported audio sample rates (8kHz - 192kHz)
- Ensures zero rounding error in sample-to-tick conversions
- One tick ≈ 35.4 nanoseconds

**Conversion Functions (8 total):**
- `ms_to_ticks(i64) -> i64` - API → Database (lossless)
- `ticks_to_ms(i64) -> i64` - Database → API (max error ~0.035ms)
- `ticks_to_samples(i64, u32) -> usize` - Database → Playback (exact)
- `samples_to_ticks(usize, u32) -> i64` - Playback → Database (exact)
- `ticks_to_seconds(i64) -> f64` - Display/logging
- `seconds_to_ticks(f64) -> i64` - User input
- `ticks_per_sample(u32) -> i64` - Lookup helper
- Additional validation functions

**Type System:**
- `i64` for ticks (database storage)
- `u64` for milliseconds (API layer)
- `usize` for samples (playback engine)
- `f64` for seconds (display only)

**Test Coverage:** 40+ test cases covering all 11 sample rates

**Assessment:** ✅ **Excellent** - Comprehensive, well-tested, requirement-traced

---

#### Database Schema Compliance

**Passages Table** (`wkmp-common/src/db/init.rs:323-375`):

All 6 timing fields use `INTEGER` type (SQLite i64):
- `start_time_ticks` - Passage start (ticks from file start)
- `fade_in_start_ticks` - Fade-in begins (nullable)
- `lead_in_start_ticks` - Lead-in begins (nullable)
- `lead_out_start_ticks` - Lead-out begins (nullable)
- `fade_out_start_ticks` - Fade-out begins (nullable)
- `end_time_ticks` - Passage end

**Cross-Reference:**
- ✅ [SRC-DB-011] start_time_ticks - COMPLIANT
- ✅ [SRC-DB-012] end_time_ticks - COMPLIANT
- ✅ [SRC-DB-013] fade_in_start_ticks - COMPLIANT
- ✅ [SRC-DB-014] fade_out_start_ticks - COMPLIANT
- ✅ [SRC-DB-015] lead_in_start_ticks - COMPLIANT
- ✅ [SRC-DB-016] lead_out_start_ticks - COMPLIANT

**Assessment:** ✅ **Fully Compliant** with SPEC017 database storage requirements

---

#### Module-by-Module Compliance

##### wkmp-common (Core Timing Infrastructure)

**Database Models:**
- `PassageTimingMs` struct - API representation (u64 milliseconds)
- `PassageTimingTicks` struct - Database representation (i64 ticks)
- Automatic conversion via `From` trait implementations

**Variable Naming:**
- Consistent `*_ticks` suffix for tick values
- Consistent `*_ms` suffix for millisecond values
- Clear separation of concerns

**Status:** ✅ **Fully Compliant** - Exemplary implementation

---

##### wkmp-ap (Audio Player)

**Database Operations:**
- Reads timing fields as `i64` ticks from passages table
- Converts using `wkmp_common::timing::seconds_to_ticks()`
- Validates and clamps timing points

**API Endpoints:**
- `GET /playback/position` - Returns `PositionResponse` with `position_ms: u64`, `duration_ms: u64`
- `POST /playback/seek` - Accepts `SeekRequest` with `position_ms: u64`

**Playback Engine:**
- Internal timing uses ticks (`start_time_ticks: i64`, `position_ticks: i64`)
- Converts to samples via `ticks_to_samples()` for audio processing
- Fader applies fade curves using tick-based position tracking

**Variable Naming:**
- Good: `position_ms`, `duration_ms`, `start_time_ticks`, `crossfade_duration_ms`
- Ambiguous: Some variables named `start_time`, `duration` without unit suffix

**Issues Identified:**
1. ⚠️ **API uses milliseconds** instead of ticks per [SRC-API-010](../docs/SPEC017-sample_rate_conversion.md#api-representation)
2. ⚠️ **Variable naming** could be clearer in some contexts

**Status:** ⚠️ **Functional Deviations** - Works correctly but deviates from API specification

---

##### wkmp-ui (User Interface)

**Current State:** Placeholder module (`println!` only)

**Infrastructure Available:**
- `ticks_to_seconds()` function ready for display conversion
- `seconds_to_ticks()` function ready for user input conversion

**Expected Implementation:**
- Fetch timing from API (milliseconds or ticks)
- Convert to seconds using `ticks_to_seconds()` for display
- Apply decimal formatting (1-2 places typical, up to 6 for precision)
- Convert user inputs from seconds to ticks using `seconds_to_ticks()`

**Status:** 🔍 **Not Implemented** - Awaiting UI development

---

##### wkmp-pd (Program Director)

**Current State:** Placeholder module only

**Status:** 🔍 **Not Implemented**

---

##### wkmp-ai (Audio Ingest)

**Database Operations:**
- Reads/writes timing fields as `i64` ticks
- `Passage` struct uses `start_time_ticks: i64`, `end_time_ticks: i64`, etc.
- Converts via `seconds_to_ticks()` when creating passages

**API Endpoints:**
- `POST /analyze/amplitude` - Request uses `start_time: f64` (seconds), `end_time: Option<f64>` (seconds)
- Response uses `lead_in_duration: f64` (seconds), `lead_out_duration: f64` (seconds)

**File Duration:**
- `AudioFile.duration: Option<f64>` - Stored as REAL (seconds), not ticks
- Inconsistent with passage timing representation

**Silence Detection:**
- `SilenceRegion` uses `start_seconds: f32`, `end_seconds: f32` (32-bit floats)
- Precision loss risk for very long files

**Issues Identified:**
1. ⚠️ **API uses seconds** instead of milliseconds (inconsistent with wkmp-ap)
2. ⚠️ **File duration uses f64 seconds** instead of i64 ticks
3. ⚠️ **Silence detection uses f32** (precision loss risk)

**Status:** ⚠️ **Functional Deviations** - Works correctly but has schema and API inconsistencies

---

##### wkmp-le (Lyric Editor)

**Current State:** Placeholder module only

**Status:** 🔍 **Not Implemented**

---

##### wkmp-dr (Database Review)

**Database Operations:**
- Read-only access to passages table
- Exposes raw timing fields: `start_time_ticks`, `end_time_ticks`, `fade_in_start_ticks`, etc.

**UI Display:**
- **File:** `wkmp-dr/src/ui/app.js` (lines 346-386)
- **Current Implementation:** Displays raw tick values without conversion
- **Example:** Shows `141120000` (not `141120000 ticks (5.00s)`)

**Column Headers:**
- Display field names: `start_time_ticks`, `end_time_ticks`, etc.
- No unit labels in table headers
- Unit information available in "Table Semantics" modal

**SRC-LAYER-011 Requirement:**
> "Developer UI displays both ticks AND seconds for developer inspection"

**Issues Identified:**
1. ❌ **Missing seconds display** - Violates SRC-LAYER-011

**Status:** ❌ **Non-Compliant** - Functional but violates developer UI requirement

---

### 2. Compliance Matrix

| Module | Database Reads | Database Writes | API Endpoints | Variable Naming | UI Display | Overall Status |
|--------|---------------|-----------------|---------------|-----------------|------------|----------------|
| **wkmp-common** | ✅ `i64` ticks | ✅ INTEGER ticks | N/A | ✅ Consistent | N/A | ✅ **COMPLIANT** |
| **wkmp-ap** | ✅ `i64` ticks | ✅ Converts correctly | ⚠️ `u64` ms (not ticks) | ⚠️ Some ambiguous | N/A | ⚠️ **ISSUES FOUND** |
| **wkmp-ui** | N/A | N/A | N/A | N/A | 🔍 Ready | 🔍 **NEEDS IMPL** |
| **wkmp-pd** | N/A | N/A | N/A | N/A | N/A | 🔍 **PLACEHOLDER** |
| **wkmp-ai** | ✅ `i64` ticks | ✅ Converts correctly | ⚠️ `f64` seconds | ⚠️ `f64`/`f32` | N/A | ⚠️ **ISSUES FOUND** |
| **wkmp-le** | N/A | N/A | N/A | N/A | N/A | 🔍 **PLACEHOLDER** |
| **wkmp-dr** | ✅ `i64` ticks | N/A (read-only) | ✅ Returns ticks | ✅ Clear names | ❌ Ticks only | ❌ **NON-COMPLIANT** |

---

### 3. Issues List

#### Issue #1: wkmp-dr Missing Seconds Display (HIGH PRIORITY)

**Location:** `wkmp-dr/src/ui/app.js:369-379`

**Issue Type:** Missing conversion - SRC-LAYER-011 violation

**Current State:**
```javascript
// Line 369: Column headers
html += `<th${className}>${col}</th>`;

// Line 379: Data cells
html += `<td${className}>${value}</td>`;
```

Displays: `141120000` (raw ticks only)

**Expected State:**
Per [SRC-LAYER-011](../docs/SPEC017-sample_rate_conversion.md#developer-facing-layers-use-ticks):
> "Developer UI displays both ticks AND seconds for developer inspection"

Should display: `141120000 ticks (5.00s)` or separate columns

**Impact:** Medium
- Functional but reduces debugging efficiency
- Developers must manually calculate seconds from ticks
- Slows down database inspection workflow

**Recommended Fix:**

**Option A: Dual Display (Recommended)**
```javascript
const TIMING_COLUMNS = ['start_time_ticks', 'end_time_ticks',
                        'fade_in_start_ticks', 'fade_out_start_ticks',
                        'lead_in_start_ticks', 'lead_out_start_ticks'];
const TICK_RATE = 28224000;

if (TIMING_COLUMNS.includes(data.columns[index]) && cell !== null) {
    const ticks = parseInt(cell);
    const seconds = (ticks / TICK_RATE).toFixed(6);
    html += `<td${className}>${value} (${seconds}s)</td>`;
} else {
    html += `<td${className}>${value}</td>`;
}
```

**Option B: Hover Tooltip**
```javascript
if (TIMING_COLUMNS.includes(data.columns[index]) && cell !== null) {
    const ticks = parseInt(cell);
    const seconds = (ticks / TICK_RATE).toFixed(6);
    html += `<td${className} title="${seconds} seconds">${value}</td>`;
} else {
    html += `<td${className}>${value}</td>`;
}
```

**Option C: Separate Computed Column**
- Add virtual columns `start_time_s`, `end_time_s`, etc.
- Compute server-side or client-side

**Recommendation:** Option A (dual display) - Most information-dense for developer workflow

---

#### Issue #2: wkmp-ap API Uses Milliseconds Instead of Ticks (MEDIUM PRIORITY)

**Location:** `wkmp-ap/src/api/handlers.rs:118-126`

**Issue Type:** API layer deviation from SRC-API-010

**Current State:**
```rust
#[derive(Debug, Serialize)]
pub struct PositionResponse {
    passage_id: Option<Uuid>,
    position_ms: u64,      // milliseconds
    duration_ms: u64,      // milliseconds
    state: String,
}
```

**Expected State:**
Per [SRC-API-010](../docs/SPEC017-sample_rate_conversion.md#api-representation):
> "The REST API uses ticks (64-bit signed integers) for sample-accurate precision"

Should use:
```rust
pub struct PositionResponse {
    passage_id: Option<Uuid>,
    position_ticks: i64,   // ticks
    duration_ticks: i64,   // ticks
    state: String,
}
```

**Impact:** Low
- Functional: Milliseconds provide ~0.035ms precision (adequate for UI updates)
- Sample-accuracy preserved internally via tick-based database
- HTTP API ergonomics benefit from milliseconds vs. 9-digit tick values

**Recommended Fix:**

**Option A (Breaking Change):** Change API to use ticks
- Aligns with SRC-API-010 specification
- Requires client code updates

**Option B (Backward Compatible):** Add both fields
```rust
pub struct PositionResponse {
    position_ms: u64,         // Deprecated, for compatibility
    position_ticks: i64,      // Preferred, sample-accurate
    duration_ms: u64,         // Deprecated
    duration_ticks: i64,      // Preferred
    ...
}
```

**Option C (Document Deviation):** Accept milliseconds as pragmatic for HTTP APIs
- Document that internal precision is preserved via tick-based database
- Update SRC-API-010 to acknowledge pragmatic API layer deviation

**Recommendation:** Option C - Current approach balances usability and precision

---

#### Issue #3: wkmp-ai File Duration Uses Seconds, Not Ticks (MEDIUM PRIORITY)

**Location:** `wkmp-ai/src/db/files.rs:19`

**Issue Type:** Database schema deviation

**Current State:**
```rust
pub struct AudioFile {
    pub guid: Uuid,
    pub path: String,
    pub duration: Option<f64>,  // seconds (floating-point)
    ...
}
```

Stored as: `duration REAL` (SQLite floating-point)

**Expected State:**
Duration should use ticks for consistency with passage timing:
```rust
pub struct AudioFile {
    pub guid: Uuid,
    pub path: String,
    pub duration_ticks: Option<i64>,  // ticks
    ...
}
```

**Impact:** Medium
- Floating-point precision loss risk for very long files
- Inconsistent with passage timing representation (all use ticks)
- Complicates duration calculations (requires conversions)

**Recommended Fix:**

1. **Create migration:** Convert `duration REAL` → `duration_ticks INTEGER`
```sql
ALTER TABLE files ADD COLUMN duration_ticks INTEGER;
UPDATE files SET duration_ticks = CAST(duration * 28224000 AS INTEGER);
-- After migration complete:
-- ALTER TABLE files DROP COLUMN duration;
```

2. **Update AudioFile struct:**
```rust
pub struct AudioFile {
    pub duration_ticks: Option<i64>,  // ticks, not seconds
    ...
}
```

3. **Update import logic:**
```rust
// When importing files:
let duration_ticks = wkmp_common::timing::seconds_to_ticks(duration_seconds);
audio_file.duration_ticks = Some(duration_ticks);
```

**Recommendation:** Implement migration for consistency with passage timing

---

#### Issue #4: wkmp-ai API Uses Seconds Instead of Milliseconds (LOW PRIORITY)

**Location:** `wkmp-ai/src/api/amplitude_analysis.rs:10-24, 28-52`

**Issue Type:** API layer inconsistency

**Current State:**
```rust
pub struct AmplitudeAnalysisRequest {
    pub start_time: f64,        // seconds
    pub end_time: Option<f64>,  // seconds
    ...
}

pub struct AmplitudeAnalysisResponse {
    pub lead_in_duration: f64,   // seconds
    pub lead_out_duration: f64,  // seconds
    ...
}
```

**Expected State:**
For consistency with wkmp-ap, use milliseconds:
```rust
pub struct AmplitudeAnalysisRequest {
    pub start_time_ms: u64,
    pub end_time_ms: Option<u64>,
    ...
}
```

**Impact:** Low
- Functional: Works correctly with conversions
- Inconsistent API conventions across microservices (wkmp-ap uses ms, wkmp-ai uses seconds)
- May confuse API consumers

**Recommended Fix:**

Standardize on milliseconds for all HTTP API timing parameters:
- Update wkmp-ai API to use `*_ms: u64` fields
- Maintain internal conversions to/from ticks

**Recommendation:** Low priority - Consider during next API version update

---

#### Issue #5: wkmp-ai Silence Detection Uses f32 (LOW PRIORITY)

**Location:** `wkmp-ai/src/services/silence_detector.rs:20-23, 41-47`

**Issue Type:** Precision loss risk

**Current State:**
```rust
pub struct SilenceRegion {
    pub start_seconds: f32,  // 32-bit float (~7 decimal digits)
    pub end_seconds: f32,
}

// Parameters
min_duration_sec: f32,
```

**Expected State:**
Use `f64` for consistency with other timing code:
```rust
pub struct SilenceRegion {
    pub start_seconds: f64,  // 64-bit float (~15 decimal digits)
    pub end_seconds: f64,
}
```

Or use exact representation (samples or ticks).

**Impact:** Low
- f32 provides ~7 decimal digits precision (adequate for silence detection in typical audio files)
- Example: 1 hour = 3600 seconds → f32 precision ~0.0003 seconds (~300 microseconds)
- Unlikely to cause issues in practice

**Recommended Fix:**

**Option A:** Change to `f64` for consistency
**Option B:** Change to `usize` samples for exact representation
**Option C:** Document rationale for `f32` choice (memory/performance trade-off)

**Recommendation:** Option A (change to f64) - Consistency outweighs memory savings

---

#### Issue #6: Variable Naming Clarity in wkmp-ap (LOW PRIORITY)

**Location:** `wkmp-ap/src/playback/pipeline/timing.rs` (various)

**Issue Type:** Undocumented units

**Examples:**
```rust
let start_time = passage.start_time;  // What unit? (Actually: u64 ms)
let duration = 5000;                  // What unit? (Actually: u32 ms)
let position = get_position();         // What unit? (Actually: usize frames)
```

**Expected State:**
Variables should have unit suffix or inline comment:
```rust
// Good: Unit in name
let start_time_ms = passage.start_time_ms;
let duration_ms: u32 = 5000;

// Good: Unit in comment
let position = get_position();  // usize frames at working sample rate
```

**Impact:** Low
- Code is functional and correct
- Clarity would improve maintainability
- Important for timing-critical audio code

**Recommended Fix:**

Add inline comments documenting units for ambiguous variables:
```rust
let start_time = passage.start_time;  // u64 ms from passage start
let duration = calculate_duration();   // u32 ms crossfade duration
let position = mixer.position();       // usize frames at 48kHz
```

Or refactor to use unit suffixes in variable names.

**Recommendation:** Add inline unit comments during next refactoring pass

---

### 4. wkmp-dr UI Enhancement Specification

**Objective:** Satisfy [SRC-LAYER-011](../docs/SPEC017-sample_rate_conversion.md#developer-facing-layers-use-ticks) by displaying both ticks and seconds

#### Recommended Approach: Dual Display

**Column Display Format:**
- Keep existing column names: `start_time_ticks`, `end_time_ticks`, etc.
- Add computed seconds inline: `141120000 (5.000000s)`

**Implementation:**

**JavaScript Constants:**
```javascript
const TICK_RATE = 28224000;
const TIMING_COLUMNS = [
    'start_time_ticks',
    'end_time_ticks',
    'fade_in_start_ticks',
    'fade_out_start_ticks',
    'lead_in_start_ticks',
    'lead_out_start_ticks'
];
```

**Rendering Logic (app.js:369-379):**
```javascript
function renderTable(data) {
    // ... existing setup ...

    data.rows.forEach(row => {
        html += '<tr>';
        row.forEach((cell, index) => {
            const colName = data.columns[index];
            const isDereferenced = dereferencedCols.has(colName);
            const className = isDereferenced ? ' class="dereferenced"' : '';

            // Format timing columns
            if (TIMING_COLUMNS.includes(colName) && cell !== null) {
                const ticks = parseInt(cell);
                const seconds = (ticks / TICK_RATE).toFixed(6);
                html += `<td${className}>${cell} (${seconds}s)</td>`;
            } else {
                const value = cell === null ? '<em>null</em>' : String(cell);
                html += `<td${className}>${value}</td>`;
            }
        });
        html += '</tr>';
    });

    // ... rest of function ...
}
```

**Decimal Precision:**
- Use 6 decimal places (microsecond precision)
- Example: `141120000 (5.000000s)`
- Rationale: Developer tool benefits from high precision

**Tooltip Enhancement (Optional):**
Add hover text with conversion formula:
```javascript
html += `<td${className} title="Ticks ÷ 28,224,000 = ${seconds}s">${cell} (${seconds}s)</td>`;
```

**Column Header Enhancement (Optional):**
Update column headers to indicate dual display:
```javascript
const headerMap = {
    'start_time_ticks': 'start_time_ticks (ticks / seconds)',
    'end_time_ticks': 'end_time_ticks (ticks / seconds)',
    // ... etc
};
```

**CSS Styling (Optional):**
```css
.timing-cell {
    font-family: 'Courier New', monospace;
    white-space: nowrap;
}

.timing-cell .seconds {
    color: #666;
    font-size: 0.9em;
}
```

With markup:
```javascript
html += `<td${className} class="timing-cell">${cell} <span class="seconds">(${seconds}s)</span></td>`;
```

---

### 5. Common Patterns Document

#### Recommended Variable Naming Conventions

**Ticks (i64) - Database/Internal:**
```rust
// GOOD: Suffix clearly indicates unit
let start_time_ticks: i64 = passage.start_time_ticks;
let position_ticks: i64 = mixer.current_position();

// ACCEPTABLE: Comment documents unit
let start_time: i64 = passage.start_time;  // ticks, SPEC017 tick-based time
```

**Milliseconds (u64) - API Layer:**
```rust
// GOOD: Suffix clearly indicates unit
let position_ms: u64 = 5000;
let duration_ms: u64 = calculate_duration_ms();

// ACCEPTABLE: Type makes it clear
let crossfade_duration: u32 = 1000;  // u32 milliseconds for crossfade
```

**Samples (usize) - Playback Engine:**
```rust
// GOOD: Suffix clearly indicates unit
let fade_duration_samples: usize = 44100;
let buffer_position_samples: usize = 0;

// ACCEPTABLE: Context makes it clear
let sample_count: usize = buffer.len();
```

**Seconds (f64) - Display Only:**
```rust
// GOOD: Suffix clearly indicates unit
let position_seconds: f64 = ticks_to_seconds(position_ticks);

// ACCEPTABLE: Function name makes it clear
let seconds = ticks_to_seconds(ticks);
```

#### Inline Comment Format

**When variable name is ambiguous:**
```rust
let position: i64 = 0;  // ticks, SPEC017 tick-based time
let duration: u32 = 1000;  // milliseconds, API layer
let offset: usize = 44100;  // samples at working sample rate
```

**For function parameters:**
```rust
/// Seeks to the specified position in the current passage
///
/// # Parameters
/// - `position_ms`: Seek position in milliseconds from passage start
pub fn seek(&mut self, position_ms: u64) -> Result<()> {
    // ...
}
```

#### Type Aliases (Optional)

**For additional clarity:**
```rust
pub type Ticks = i64;        // SPEC017 tick-based time (28,224,000 Hz)
pub type Milliseconds = u64;  // Milliseconds for API layer
pub type Samples = usize;     // Sample count at working sample rate

// Usage:
let position: Ticks = mixer.position_ticks();
let duration: Milliseconds = 5000;
let buffer_size: Samples = 4096;
```

#### Conversion Function Usage

**Always use wkmp_common::timing functions:**
```rust
use wkmp_common::timing::*;

// API → Database
let ticks = ms_to_ticks(position_ms as i64);

// Database → Playback
let samples = ticks_to_samples(start_time_ticks, sample_rate);

// Display
let seconds = ticks_to_seconds(position_ticks);

// User Input
let ticks = seconds_to_ticks(user_input_seconds);
```

**Never implement ad-hoc conversions:**
```rust
// BAD: Manual conversion
let ticks = (ms * 28224) as i64;  // Use ms_to_ticks() instead

// BAD: Magic numbers
let seconds = ticks as f64 / 28224000.0;  // Use ticks_to_seconds() instead
```

---

## Recommendation

### Overall Assessment

WKMP's tick-based timing system is **well-architected and mostly compliant** with SPEC017. The core infrastructure (wkmp-common) is excellent with comprehensive conversion functions, strong test coverage, and clear requirement traceability.

**Strengths:**
1. Zero-error tick-to-sample conversions for all 11 supported sample rates
2. Consistent database schema using INTEGER ticks
3. Well-documented conversion functions with requirement IDs
4. Comprehensive test coverage (40+ test cases)

**Weaknesses:**
1. wkmp-dr violates SRC-LAYER-011 (missing seconds display)
2. API layer deviates from SRC-API-010 (uses milliseconds/seconds, not ticks)
3. Some variable naming ambiguity in timing-critical code

### Immediate Action Required

**Fix HIGH priority Issue #1:**
- **What:** Add seconds display to wkmp-dr timing columns
- **Why:** Violates SRC-LAYER-011 requirement
- **Impact:** Improves developer debugging efficiency
- **Effort:** Low (simple JavaScript enhancement)
- **Risk:** Low (display-only change, no logic changes)

### Consider for Next Iteration

**MEDIUM priority issues:**
1. Standardize API timing units (milliseconds vs. seconds) across microservices
2. Migrate wkmp-ai file duration to i64 ticks for consistency

**LOW priority issues:**
3. Add inline unit documentation for ambiguous timing variables
4. Change wkmp-ai silence detection from f32 to f64

### Accept as Pragmatic

**API layer using milliseconds/seconds:**
- Balances HTTP ergonomics with precision requirements
- Sample-accuracy preserved internally via tick-based database
- Consider documenting this as intentional deviation from SRC-API-010

**Rationale:** Raw tick values (e.g., `141120000`) are less ergonomic for HTTP APIs than milliseconds (`5000`) or seconds (`5.0`). The current approach provides adequate precision for API consumers while maintaining sample-accurate internal timing.

---

## Decisions Required

Stakeholder must decide:

### 1. API Layer Philosophy

**Question:** Should HTTP APIs strictly use raw ticks per SRC-API-010, or accept milliseconds/seconds as pragmatic?

**Option A: Strict Compliance**
- Change all APIs to use i64 ticks
- Aligns with SRC-API-010 specification
- Breaking change for API clients

**Option B: Pragmatic Deviation (Recommended)**
- Accept milliseconds/seconds as ergonomic for HTTP
- Document deviation from SRC-API-010 with rationale
- Maintain sample-accurate internal timing

**Recommendation:** Option B - Current approach balances usability and precision

---

### 2. wkmp-dr Display Format

**Question:** How should Developer UI display timing values?

**Option A: Dual Display (Recommended)**
- Format: `141120000 (5.000000s)`
- Most information-dense
- No additional columns needed

**Option B: Hover Tooltip**
- Format: `141120000` with tooltip showing seconds
- Cleaner visual appearance
- Requires hover interaction

**Option C: Separate Computed Column**
- Add virtual `*_seconds` columns
- Clean separation of representations
- Wider table, more scrolling

**Recommendation:** Option A - Dual display provides immediate visibility

---

### 3. File Duration Migration

**Question:** Should wkmp-ai file duration migrate from f64 seconds to i64 ticks?

**Option A: Migrate to Ticks (Recommended)**
- Consistent with passage timing representation
- Eliminates floating-point precision loss
- Requires database migration

**Option B: Keep f64 Seconds**
- Maintains backward compatibility
- Adequate precision for file durations
- Inconsistent with passage timing

**Recommendation:** Option A if feasible - Consistency outweighs migration effort

---

## Next Steps

This analysis is complete. Implementation planning requires explicit user authorization.

**To proceed with implementation:**

1. Review analysis findings and prioritize issues
2. Make decisions on API layer philosophy and display formats
3. Run `/plan [specification_document]` to create detailed implementation plan

**For HIGH priority issue #1 (wkmp-dr seconds display):**
- Can be implemented independently
- Relatively simple JavaScript enhancement
- Immediate developer experience improvement
- Estimated effort: 1-2 hours

**User retains full authority over:**
- Whether to implement any recommendations
- Which issues to address (all, high only, selected)
- API layer standardization approach
- Timeline for fixes
