# SPEC017 Tick-Based Timing Compliance Review

**Document Type:** Analysis Request for `/think` workflow
**Created:** 2025-11-02
**Purpose:** Comprehensive review of all modules for SPEC017 compliance and time unit documentation

---

## Objective

Perform a systematic review of all WKMP microservices to ensure:

1. **SPEC017 Compliance:** All passage timing uses tick-based representation per [SPEC017-sample_rate_conversion.md](../docs/SPEC017-sample_rate_conversion.md)
2. **Time Unit Clarity:** Every time-related variable declaration and usage is explicitly documented with its unit
3. **wkmp-dr Labeling:** Database review UI includes time units in all column headers and semantic labels

---

## Scope

### Modules to Review

All six WKMP microservices:
- **wkmp-ap** (Audio Player) - Core playback engine with tick-based timing
- **wkmp-ui** (User Interface) - API interactions and display conversions
- **wkmp-pd** (Program Director) - Passage selection and timing calculations
- **wkmp-ai** (Audio Ingest) - File scanning and passage timing extraction
- **wkmp-le** (Lyric Editor) - Timing-related lyric synchronization features
- **wkmp-dr** (Database Review) - Read-only database inspection with timing display
- **wkmp-common** - Shared models, database interactions, and timing utilities

### Timing Categories to Inspect

Per [SPEC023-timing_terminology.md](../docs/SPEC023-timing_terminology.md), WKMP distinguishes four timing types:

1. **Calendar Time** (system timestamps) - Unix milliseconds
2. **Monotonic Elapsed Time** (intervals) - `std::time::Instant`, `Duration`
3. **Audio Timeline Time** (ticks) - `i64` values per SPEC017
4. **Callback Time** (audio samples/frames) - PCM sample counts

### Specific Timing Points to Verify

Per [SPEC017 SRC-DB-011 through SRC-DB-016](../docs/SPEC017-sample_rate_conversion.md#database-storage):

**Passage Timing Fields (must be ticks):**
- `start_time` - Passage start boundary
- `end_time` - Passage end boundary
- `fade_in_point` - Fade-in completion point
- `fade_out_point` - Fade-out start point
- `lead_in_point` - Lead-in end point
- `lead_out_point` - Lead-out start point

**Related Timing Calculations:**
- Crossfade overlap duration calculations
- Playback position tracking
- Passage duration computations
- Time display conversions (ticks ↔ seconds)

---

## Questions to Address

### 1. Database Interactions

**For each module:**
- Does it read/write passage timing fields from/to the database?
- Are all timing fields handled as `i64` ticks (not floats, not seconds)?
- Are tick values converted correctly to/from other representations?
- Are NULL handling semantics correct per [SRC-DB-020](../docs/SPEC017-sample_rate_conversion.md#database-storage)?

### 2. Variable Naming and Documentation

**For each time-related variable:**
- Is the variable name sufficiently clear about its unit?
- Is there an inline comment documenting the unit if ambiguous?
- Does the code distinguish between:
  - Ticks (`i64` per SPEC017)
  - Frames/samples (`u64` or `usize`)
  - Milliseconds (`u64` or `Duration`)
  - Microseconds (`u64` or `Duration`)
  - Seconds (`f64` for display only)

**Examples of Good Variable Naming:**
```rust
// GOOD: Unit clear from name
let start_time_ticks: i64 = passage.start_time;
let duration_ms: u64 = 5000;
let sample_count: usize = 44100;

// GOOD: Unit documented in comment
let position: i64 = 0;  // ticks, SPEC017 tick-based time

// BAD: Ambiguous
let start_time = passage.start_time;  // What unit?
let duration = 5000;  // Ticks? ms? samples?
```

### 3. API Layer Compliance

**REST API Endpoints:**
- Do all API endpoints use `i64` ticks for passage timing per [SRC-API-010](../docs/SPEC017-sample_rate_conversion.md#api-representation)?
- Are JSON serialization/deserialization types correct?
- Do error messages reference ticks appropriately?

### 4. UI Layer Conversions

**Developer UI (wkmp-dr):**
- Does wkmp-dr display both ticks AND seconds per [SRC-LAYER-011](../docs/SPEC017-sample_rate_conversion.md#developer-facing-layers-use-ticks)?
- Are column headers labeled with units? (e.g., "Start Time (ticks)", "Duration (seconds)")
- Are semantic labels clear about units?

**End-User UI (wkmp-ui):**
- Does the UI convert ticks to seconds for display per [SRC-LAYER-012](../docs/SPEC017-sample_rate_conversion.md#user-facing-layers-use-seconds)?
- Is appropriate decimal precision used (1-2 decimal places typically)?
- Are user inputs converted correctly from seconds to ticks per [SRC-LAYER-030](../docs/SPEC017-sample_rate_conversion.md#developer-facing-layers-use-ticks)?

### 5. Tick Conversion Utilities

**wkmp-common:**
- Are there reusable conversion utilities for ticks ↔ seconds?
- Are conversion functions documented with formulae per [SRC-CONV-030](../docs/SPEC017-sample_rate_conversion.md#sample-to-tick-conversion) and [SRC-TIME-010](../docs/SPEC017-sample_rate_conversion.md#tick-to-duration-conversion)?
- Are conversions used consistently across modules?

### 6. Test Coverage

**For timing-related code:**
- Are tick conversions tested with known values?
- Do tests verify sample-accuracy for common sample rates (44.1kHz, 48kHz)?
- Are edge cases tested (zero-duration intervals per [XFD-OV-020](../docs/SPEC002-crossfade.md#overview))?

---

## Problems to Identify

### Compliance Issues

1. **Incorrect Data Types:**
   - Floating-point types (`f64`, `f32`) used for storage/API where ticks should be used
   - Unsigned integers where signed `i64` is required

2. **Missing Conversions:**
   - Direct use of tick values in user-facing displays without conversion to seconds
   - Missing tick-to-sample conversions in audio processing

3. **Lossy Conversions:**
   - Premature conversion to floating-point seconds in developer-facing layers
   - Unnecessary round-trips through floating-point

4. **Undocumented Units:**
   - Variables named `time`, `duration`, `position` without unit clarification
   - Function parameters without unit documentation

### wkmp-dr Specific Issues

1. **Column Headers:**
   - Time columns without unit labels
   - Mixed units in same display context
   - Ambiguous field names (e.g., "Start" vs. "Start Time (ticks)")

2. **Display Formatting:**
   - Tick values displayed without corresponding seconds
   - Seconds displayed without corresponding ticks
   - Inconsistent decimal precision

---

## Expected Outputs

### 1. Compliance Matrix

For each module, document:

| Module | Database Reads | Database Writes | API Endpoints | Variable Naming | UI Display | Status |
|--------|---------------|-----------------|---------------|-----------------|------------|--------|
| wkmp-ap | ... | ... | ... | ... | N/A | ... |
| wkmp-ui | ... | ... | ... | ... | ... | ... |
| wkmp-pd | ... | ... | ... | ... | N/A | ... |
| wkmp-ai | ... | ... | ... | ... | ... | ... |
| wkmp-le | ... | ... | ... | ... | ... | ... |
| wkmp-dr | ... | N/A (read-only) | ... | ... | ... | ... |
| wkmp-common | ... | ... | N/A | ... | N/A | ... |

**Status Values:** ✅ Compliant | ⚠️ Issues Found | ❌ Non-Compliant | 🔍 Needs Review

### 2. Issues List

For each non-compliant area:
- **Location:** File path and line number (e.g., `wkmp-ap/src/engine.rs:247`)
- **Issue Type:** Data type mismatch, missing conversion, undocumented unit, etc.
- **Current State:** What the code does now
- **Expected State:** What SPEC017 requires
- **Impact:** High (breaks sample-accuracy), Medium (confusing but functional), Low (documentation only)
- **Recommended Fix:** Specific code change or documentation addition

### 3. wkmp-dr UI Enhancement Specification

Detailed recommendations for wkmp-dr time field display:

- Column header format (e.g., "Start (ticks)", "Start (s)")
- Display both ticks and computed seconds for timing fields
- Appropriate decimal precision for seconds display
- Tooltip/hover behavior for additional precision
- Consistency with developer UI requirements per [SRC-LAYER-011](../docs/SPEC017-sample_rate_conversion.md#developer-facing-layers-use-ticks)

### 4. Common Patterns Document

Document recommended patterns for:
- Variable naming conventions for time values
- Inline comment format for unit documentation
- Conversion function usage
- Type aliases for clarity (e.g., `type Ticks = i64;`)

---

## Analysis Method

### Phase 1: Codebase Exploration

For each module:
1. Identify all files that interact with timing (database queries, API handlers, calculations)
2. Catalog all time-related variable declarations
3. Map data flow for passage timing fields (database → API → UI)

### Phase 2: SPEC017 Cross-Reference

For each timing usage:
1. Verify data type matches SPEC017 requirements
2. Check for appropriate conversions at layer boundaries
3. Validate against specific requirement IDs (SRC-DB-*, SRC-API-*, SRC-LAYER-*)

### Phase 3: wkmp-dr UI Review

1. Examine current column header implementation
2. Identify time-related fields in database tables displayed
3. Assess current unit labeling and documentation
4. Propose specific UI improvements

### Phase 4: Pattern Analysis

1. Identify common compliance patterns (good practices to replicate)
2. Identify common violations (anti-patterns to fix)
3. Document reusable solutions

---

## Success Criteria

The analysis is successful when:

1. ✅ All modules inspected for SPEC017 compliance
2. ✅ All timing-related variables documented with units
3. ✅ Compliance matrix completed for all 7 modules
4. ✅ Issues list provides actionable fixes with location/impact/priority
5. ✅ wkmp-dr UI specification ready for implementation
6. ✅ Common patterns documented for future development

---

## Constraints

### Must NOT Do

- ❌ Modify any code (analysis only, no implementation)
- ❌ Create implementation plans (use `/plan` workflow for that)
- ❌ Write test specifications (defer to `/plan` phase)

### Must Do

- ✅ Review all relevant source code systematically
- ✅ Cross-reference SPEC017, SPEC023, SPEC002 requirements
- ✅ Document findings with file/line number precision
- ✅ Provide concrete examples of issues and fixes
- ✅ Follow `/think` workflow phases (no implementation planning)

---

## Related Documentation

- [SPEC017-sample_rate_conversion.md](../docs/SPEC017-sample_rate_conversion.md) - Tick-based timing specification
- [SPEC023-timing_terminology.md](../docs/SPEC023-timing_terminology.md) - Four timing types defined
- [SPEC002-crossfade.md](../docs/SPEC002-crossfade.md) - Crossfade timing points (must use ticks)
- [IMPL001-database_schema.md](../docs/IMPL001-database_schema.md) - Database schema (timing fields)
- [IMPL002-coding_conventions.md](../docs/IMPL002-coding_conventions.md) - Coding standards

---

## Notes

**Thoroughness Required:** This is a comprehensive compliance review affecting sample-accurate audio playback. Missing violations risk audio glitches and precision loss.

**Developer UI Priority:** wkmp-dr improvements are user-facing for developers inspecting the database. Clear unit labeling directly impacts debugging efficiency.

**Pattern Documentation Value:** Establishing clear patterns now prevents future violations as codebase grows.

---

## After Analysis

**Analysis Date:** 2025-11-02
**Analysis Method:** `/think` Multi-Agent Workflow (8-Phase Analysis)
**Analysis Output:** [spec017_compliance_review_analysis_results.md](spec017_compliance_review_analysis_results.md)

### Quick Summary

**Overall Assessment:** ⚠️ **Mostly Compliant** - Excellent tick infrastructure with pragmatic deviations

**Critical Finding:**
wkmp-dr Database Review UI violates SRC-LAYER-011 by displaying only ticks (missing seconds conversion). Developer UI requirement explicitly states "displays both ticks AND seconds for developer inspection."

**Modules Reviewed:** 7 (wkmp-common, wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le, wkmp-dr)
**Issues Found:** 6 (1 HIGH, 2 MEDIUM, 3 LOW)

**Options Analyzed:** 3
1. Strict API compliance (use raw ticks in HTTP APIs) - Specification-aligned but less ergonomic
2. Pragmatic API deviation (use milliseconds/seconds) - Current approach, balances usability and precision
3. Accept database schema deviation (file duration as f64 seconds) - Functional but inconsistent

**Recommendation:** **Fix HIGH priority issue (wkmp-dr)**, accept pragmatic API deviations
- wkmp-dr: Add seconds display via dual format `141120000 (5.000000s)`
- API layer: Document milliseconds/seconds as intentional ergonomic choice
- File duration: Consider migration to ticks for consistency

**Key Findings:**
- ✅ wkmp-common tick infrastructure is excellent (comprehensive, well-tested)
- ✅ Database schema fully compliant (all passage timing uses INTEGER ticks)
- ❌ wkmp-dr missing seconds display (violates SRC-LAYER-011)
- ⚠️ API layer uses milliseconds/seconds instead of ticks (pragmatic but non-compliant)
- ⚠️ wkmp-ai file duration uses f64 seconds (inconsistent with passage timing)

**See full analysis document for:**
- Detailed compliance matrix for all 7 modules
- 6 issues with location, impact, and recommended fixes
- wkmp-dr UI enhancement specification (dual display implementation)
- Common patterns document (variable naming, conversion function usage)

**Next Step:** Review findings and decide: (1) API layer philosophy (strict vs. pragmatic), (2) wkmp-dr display format, (3) file duration migration
