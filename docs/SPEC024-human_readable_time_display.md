# SPEC024: Human-Readable Time Display Standards

**Status:** Draft
**Created:** 2025-11-02
**Category:** User Interface Specification

---

## Purpose

Define consistent human-readable time formatting standards for WKMP user interfaces, ensuring times are displayed in the most appropriate format based on magnitude.

---

## Scope

This specification applies to:
- wkmp-dr (Database Review UI) - Developer-facing time displays
- wkmp-ui (User Interface) - End-user facing time displays
- Any other UI components displaying durations or time intervals

This specification does **NOT** apply to:
- Audio timeline time (ticks) - See SPEC017
- System timestamps (use ISO 8601 or RFC 3339)
- API representations (use seconds or milliseconds with unit suffixes)

---

## Time Display Standards

### [SPEC024-FMT-010] Format Selection by Magnitude

Time format selection SHALL be based on the **typical magnitude** of the value being displayed:

| Typical Range | Format | Decimal Places | Example |
|---------------|--------|----------------|---------|
| < 100 seconds | `X.XXs` | 2 decimal places | `5.00s`, `45.23s`, `99.99s` |
| 100 seconds to 100 minutes | `M:SS.Xs` | 1 decimal place | `1:40.5s`, `5:00.0s`, `99:59.9s` |
| 100 minutes to 25 hours | `H:MM:SS` | No decimals | `1:40:00`, `5:00:00`, `24:59:59` |
| ≥ 25 hours | `X.XXd` or `H:MM:SS` | Up to 2 decimals (days) or none (hours) | `7d`, `7.1d`, `7.57d` or `23:45:30` |

### [SPEC024-FMT-020] Format Selection Algorithm

1. Determine the **typical maximum value** for the field being displayed
2. Select format based on typical maximum (not current value)
3. Apply format consistently to all values in that field/column

**Rationale:** Consistent formatting prevents format changes as values grow, improving readability.

---

## Format Specifications

### [SPEC024-FMT-030] Short Duration Format (< 100s)

**Format:** `X.XXs`
- Range: 0.00s to 99.99s
- Decimal places: Exactly 2
- Unit: Always display `s` suffix
- Alignment: Right-aligned for tabular display

**Examples:**
```
0.00s
5.00s
45.23s
99.99s
```

**Special Cases:**
- Zero: Display as `0.00s` (not `0s`)
- NULL: Display as `null` or `—` (en dash)

---

### [SPEC024-FMT-040] Medium Duration Format (100s to 100m)

**Format:** `M:SS.Xs`
- Range: 1:40.0s to 99:59.9s
- Minutes: 1-2 digits, no leading zero for single-digit minutes
- Seconds: Always 2 digits with leading zero
- Decimal: Exactly 1 decimal place
- Unit: Always display `s` suffix
- Alignment: Right-aligned for tabular display

**Examples:**
```
1:40.0s
5:00.0s
12:34.5s
99:59.9s
```

**Special Cases:**
- Exactly 100 seconds: Display as `1:40.0s` (not `100.00s`)
- NULL: Display as `null` or `—`

---

### [SPEC024-FMT-050] Long Duration Format (100m to 25h)

**Format:** `H:MM:SS`
- Range: 1:40:00 to 24:59:59
- Hours: 1-2 digits, no leading zero for single-digit hours
- Minutes: Always 2 digits with leading zero
- Seconds: Always 2 digits with leading zero
- No decimal places
- No unit suffix
- Alignment: Right-aligned for tabular display

**Examples:**
```
1:40:00
5:00:00
12:34:56
24:59:59
```

**Special Cases:**
- Exactly 100 minutes: Display as `1:40:00` (not `100:00.0s`)
- NULL: Display as `null` or `—`

---

### [SPEC024-FMT-060] Extended Duration Format (≥ 25h typical max)

**Format Selection:** Based on **actual value** (not typical max):
- **Value < 25 hours:** Use `H:MM:SS` format (no decimals)
- **Value ≥ 25 hours (≥ 90000 seconds):** Use `X.XXd` format (up to 2 decimal places)

#### Sub-format A: Hours (< 25h actual value)
**Format:** `H:MM:SS`
- Hours: 1-2 digits (0-24), no leading zero
- Minutes: Always 2 digits with leading zero
- Seconds: Always 2 digits with leading zero
- No decimal places

**Examples:**
```
1:00:00    (1 hour)
12:30:45   (12.5 hours)
23:59:59   (just under 24 hours)
```

#### Sub-format B: Days (≥ 25h actual value)
**Format:** `X.XXd`
- Days: 1+ digits (integer part)
- Decimal places: Up to 2, trailing zeros omitted
- Unit: Always display `d` suffix
- Alignment: Right-aligned for tabular display

**Rounding Rules:**
- 0 decimal places if exact day value (e.g., 7.000 days → `7d`)
- 1 decimal place if hundredths round to zero (e.g., 7.104 days → `7.1d`)
- 2 decimal places otherwise (e.g., 7.573 days → `7.57d`)

**Examples:**
```
1.04d      (25 hours = 90000s)
7d         (7.000 days = 604800s)
7.1d       (7.104 days = 613786s)
7.57d      (7.573 days = 654307s)
14d        (14.000 days = 1209600s)
365.25d    (leap year)
```

**Special Cases:**
- Exactly 25 hours (90000s): Display as `1.04d` (not `25:00:00`)
- Values 24h59m59s and below: Use `H:MM:SS` format (e.g., `24:59:59`)
- NULL: Display as `null` or `—`

---

## Application Guidelines

### [SPEC024-APP-010] Field Type Classification

Classify each time field by its **typical maximum value**:

#### Passage Timing (< 100s typically)
- Fade durations: 0-10 seconds → Short format (`5.00s`)
- Lead-in/out durations: 0-30 seconds → Short format (`12.50s`)
- Crossfade overlap: 0-15 seconds → Short format (`8.75s`)

#### File Durations (100s to 100m typically)
- Audio file length: 30 seconds to 10 minutes → Medium format (`5:30.0s`)
- Passage duration: 2-8 minutes → Medium format (`4:32.5s`)

#### Cooldown Periods (100m to 25h typically)
- Artist cooldown: 2-4 hours → Long format (`2:30:00`)
- Work cooldown: 3-7 days → Extended format (`3d-12:00:00`)
- Song cooldown: 7-14 days → Extended format (`7d-0:00:00`)

#### Session Durations (100m to 25h typically)
- Import session elapsed time: 5 minutes to 2 hours → Long format (`1:45:23`)
- Playback session time: 30 minutes to 8 hours → Long format (`4:15:00`)

---

## Database Review (wkmp-dr) Implementation

### [SPEC024-DR-010] Column-Specific Formats

| Table | Column | Typical Max | Format |
|-------|--------|-------------|--------|
| passages | `start_time_ticks`, `end_time_ticks` | Varies (ticks) | Dual: `{ticks} ({seconds})` per SPEC017 |
| passages | `fade_in_duration`, `fade_out_duration` | 10s | Short: `5.00s` |
| passages | `lead_in_duration`, `lead_out_duration` | 30s | Short: `12.50s` |
| songs | `min_cooldown` | 7 days | Extended: `7d-0:00:00` |
| songs | `ramping_cooldown` | 14 days | Extended: `14d-0:00:00` |
| artists | `min_cooldown` | 2 hours | Long: `2:00:00` |
| artists | `ramping_cooldown` | 4 hours | Long: `4:00:00` |
| works | `min_cooldown` | 3 days | Extended: `3d-0:00:00` |
| works | `ramping_cooldown` | 7 days | Extended: `7d-0:00:00` |

---

## User Interface (wkmp-ui) Implementation

### [SPEC024-UI-010] Display Requirements

- Passage timing displays: Use short format (per SPEC017 SRC-LAYER-012, show seconds only)
- Cooldown settings: Use format matching database review
- Progress indicators: Show elapsed/remaining time in appropriate format

---

## Edge Cases

### [SPEC024-EDGE-010] Boundary Values

When a value falls exactly on a format boundary, use the **larger format**:
- 100.00 seconds → Medium format: `1:40.0s` (not short)
- 100:00 minutes → Long format: `1:40:00` (not medium)
- 25:00:00 hours (90000s) → Days format: `1.04d` (not H:MM:SS)

**Rationale:** Prevents visual ambiguity and maintains consistent digit width.

### [SPEC024-EDGE-020] NULL and Missing Values

- Database NULL: Display as `null` in wkmp-dr
- Missing/unknown: Display as `—` (en dash U+2014) in wkmp-ui
- Never display empty string or `0` for missing values

### [SPEC024-EDGE-030] Negative Values

If negative durations occur (error condition):
- Prepend minus sign: `-5.00s`, `-1:40.0s`, `-2:30:00`, `-7d`
- Log warning (negative durations are invalid)

---

## Implementation Functions

### JavaScript (wkmp-dr)

```javascript
/**
 * Format seconds as human-readable time per SPEC024.
 * @param {number} seconds - Duration in seconds
 * @param {number} typicalMax - Typical maximum value for this field (seconds)
 * @returns {string} Formatted time string
 */
function formatHumanTime(seconds, typicalMax) {
    if (seconds === null || seconds === undefined) return 'null';

    // [SPEC024-FMT-010] Select format by typical maximum
    if (typicalMax < 100) {
        // Short format: X.XXs
        return `${seconds.toFixed(2)}s`;
    } else if (typicalMax < 6000) {  // 100 minutes
        // Medium format: M:SS.Xs
        const minutes = Math.floor(seconds / 60);
        const secs = seconds % 60;
        return `${minutes}:${secs.toFixed(1).padStart(4, '0')}s`;
    } else if (typicalMax < 90000) {  // 25 hours
        // Long format: H:MM:SS
        const hours = Math.floor(seconds / 3600);
        const mins = Math.floor((seconds % 3600) / 60);
        const secs = Math.floor(seconds % 60);
        return `${hours}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    } else {
        // Extended format: Dual sub-format based on actual value
        if (seconds < 90000) {  // < 25 hours
            // Sub-format A: H:MM:SS
            const hours = Math.floor(seconds / 3600);
            const mins = Math.floor((seconds % 3600) / 60);
            const secs = Math.floor(seconds % 60);
            return `${hours}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
        } else {
            // Sub-format B: X.XXd (>= 25 hours)
            const days = seconds / 86400.0;
            const rounded2dp = Math.round(days * 100) / 100;
            const rounded1dp = Math.round(days * 10) / 10;

            if (Math.abs(rounded2dp - Math.floor(rounded2dp)) < 0.001) {
                return `${Math.floor(rounded2dp)}d`;
            } else if (Math.abs(rounded2dp * 10 - Math.floor(rounded2dp * 10)) < 0.001) {
                return `${rounded1dp.toFixed(1)}d`;
            } else {
                return `${rounded2dp.toFixed(2)}d`;
            }
        }
    }
}
```

### Rust (wkmp-common)

**Canonical implementation available in `wkmp-common::human_time` module.**

```rust
use wkmp_common::human_time::format_human_time;

// Format with explicit typical_max (recommended for consistency)
let formatted = format_human_time(604800, 1209600);  // "7d"
let formatted = format_human_time(7200, 1209600);     // "2:00:00" (< 25h)

// Format Option<i64> (NULL-safe)
use wkmp_common::human_time::format_human_time_opt;
let formatted = format_human_time_opt(Some(7200), 1209600);  // "2:00:00"
let formatted = format_human_time_opt(None, 1209600);        // "null"

// Auto-inference (convenience, less consistent in tables)
use wkmp_common::human_time::format_human_time_auto;
let formatted = format_human_time_auto(330);  // "5:30.0s"
```

**Module provides:**
- `format_human_time(seconds: i64, typical_max: i64) -> String` - Main function
- `format_human_time_opt(seconds_opt: Option<i64>, typical_max: i64) -> String` - NULL-safe
- `format_human_time_auto(seconds: i64) -> String` - Auto-inference (20% headroom)

**Full test coverage:** See `wkmp-common/src/human_time.rs` for 9 test functions.

---

## Implementation Pattern

### For Rust Modules

**Step 1:** Add dependency in `Cargo.toml`:
```toml
[dependencies]
wkmp-common = { path = "../wkmp-common" }
```

**Step 2:** Import and use:
```rust
use wkmp_common::human_time::format_human_time;

// Define typical max constants for your fields
const SONG_COOLDOWN_TYPICAL_MAX: i64 = 1209600;  // 14 days
const ARTIST_COOLDOWN_TYPICAL_MAX: i64 = 14400;  // 4 hours

// Format in display logic
let formatted = format_human_time(cooldown_seconds, SONG_COOLDOWN_TYPICAL_MAX);
```

### For JavaScript/Frontend

**Current:** wkmp-dr uses inline `formatHumanTime()` function (lines 391-424 in app.js)

**Future Options:**
1. **Backend formatting:** Add HTTP endpoint to format time values (recommended for consistency)
2. **Shared library:** Extract to shared JavaScript module if multiple frontends need it
3. **Direct port:** Copy implementation to other JavaScript codebases (ensure synchronization)

**Recommendation:** For wkmp-ui and future frontends, use backend formatting endpoint to maintain single source of truth in Rust.

---

## Traceability

| Requirement | Source | Priority |
|-------------|--------|----------|
| SPEC024-FMT-010 | User feedback (songs table cooldowns) | HIGH |
| SPEC024-FMT-020 | Consistency principle | MEDIUM |
| SPEC024-FMT-030 | Standard practice | MEDIUM |
| SPEC024-FMT-040 | Standard practice | MEDIUM |
| SPEC024-FMT-050 | Standard practice | MEDIUM |
| SPEC024-FMT-060 | Extended duration support | LOW |

---

## Related Specifications

- **SPEC017:** Tick-based timing (audio timeline precision)
- **SPEC023:** Timing terminology (four timing types)
- **IMPL001:** Database schema (cooldown column types)

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 0.1 | 2025-11-02 | Claude Code | Initial draft based on user feedback |
| 0.2 | 2025-11-02 | Claude Code | Added Rust implementation and integration patterns |
| 0.3 | 2025-11-02 | Claude Code | Revised extended format to use dual sub-format (days for ≥25h, H:MM:SS for <25h) |
