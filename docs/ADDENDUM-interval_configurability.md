# ADDENDUM: Event Timing Interval Configurability

**📋 TIER R - REVIEW & CHANGE CONTROL**

Temporary cross-document clarification enhancing event timing interval configuration documentation. Updatable until integrated into Tier 2-3 documents. See [Document Hierarchy](GOV001-document_hierarchy.md) for Tier R classification details.

**Authority:** Temporary authoritative - valid until content integrated into SPEC/IMPL documents

**Status:** Active (pending integration)
**Date:** 2025-10-18 (Revision 2)
**Related Documents:**
- CHANGELOG-event_driven_architecture.md
- REV002-event_driven_architecture_update.md
**Type:** Configuration Enhancement

**Integration Target:** Content should be merged into SPEC001-architecture.md, SPEC011-event_system.md, and IMPL001-database_schema.md

---

## Overview

This addendum clarifies and enhances the event timing interval configuration documentation. The initial event-driven architecture update (REV002) introduced position events but did not fully specify their configurability. This revision ensures all time intervals are:

1. **Clearly marked as configurable**
2. **Stored in the database Global settings table**
3. **Uniquely identified with purpose and application points**
4. **Consistently referenced across all documentation**

---

## Additional Database Settings

### New Setting Added

| Setting Name | Type | Default | Purpose | Module | Versions |
|--------------|------|---------|---------|--------|----------|
| `position_event_interval_ms` | INTEGER | 1000 | Internal event: Interval for mixer to emit PositionUpdate internal events. Controls song boundary detection accuracy and CPU usage. Range: 100-5000ms. | wkmp-ap | All |

**Location:** `settings` table (Global settings)

**Rationale:** Position event emission frequency was initially described as hardcoded (~1 second). Making this configurable allows users to:
- Tune song boundary detection responsiveness
- Optimize CPU usage for their environment
- Support high-precision use cases (e.g., lyric sync)

---

## Time Interval Taxonomy

### Complete List of Event-Related Time Intervals

| Interval Name | Database Setting | Default | Scope | Purpose | Configurable |
|---------------|------------------|---------|-------|---------|--------------|
| **Position Event Interval** | `position_event_interval_ms` | 1000ms | Internal (wkmp-ap) | How often mixer emits PositionUpdate events | ✅ Yes |
| **PlaybackProgress Event Interval** | `playback_progress_interval_ms` | 5000ms | External (SSE) | How often PlaybackProgress events sent to UI | ✅ Yes |

### Non-Event Time Intervals (For Comparison)

| Interval Name | Database Setting | Default | Purpose | Configurable |
|---------------|------------------|---------|---------|--------------|
| Crossfade Duration | `global_crossfade_time` | 2.0s | Audio crossfade length | ✅ Yes |
| Resume Fade Duration | `resume_from_pause_fade_in_duration` | 0.5s | Pause→Play fade-in | ✅ Yes |
| Queue Refill Throttle | `queue_refill_request_throttle_seconds` | 10s | Min interval between refill requests | ✅ Yes |
| Backup Interval | `backup_interval_ms` | 90 days | Periodic backup frequency | ✅ Yes |

---

## Documentation Updates (Revision 2)

### Files Modified in This Addendum

#### 1. IMPL001-database_schema.md

**Changes:**
- **Added:** New section "Event Timing Intervals - Detailed Explanation" (lines 800-905)
- **Added:** Database setting `position_event_interval_ms` (line 749)
- **Enhanced:** `playback_progress_interval_ms` description (line 750)
- **Created:** "Event Timing Configuration" subsection header (line 748)

**Content Added:**
- Comprehensive explanation of both time intervals
- Application points and emission logic
- Trade-off analysis for different interval values
- Interval relationship and independence
- Configuration recommendations for different use cases
- Example timing scenario table

#### 2. SPEC001-architecture.md

**Changes:**
- **Updated:** [ARCH-SNGC-030] CurrentSongChanged Emission (lines 354-367)
  - Added: "Configurable interval" specification
  - Added: Cross-reference to database schema
  - Added: Detection latency specification
- **Updated:** [ARCH-SNGC-060] Performance Considerations (lines 496-511)
  - Added: Position event frequency configurability
  - Added: CPU overhead scaling with interval
  - Added: Cross-reference to configuration guidance
- **Updated:** SSE Events list (line 137)
  - Changed: "~5 seconds" to "configurable interval, default: 5 seconds"

#### 3. SPEC011-event_system.md

**Changes:**
- **Updated:** `PlaybackEvent::PositionUpdate` documentation (lines 598-602)
  - Added: Database setting reference
  - Added: Configurability note
  - Added: Cross-reference to IMPL001
- **Updated:** Event Flow diagram (lines 638-652)
  - Changed: Hardcoded "44,100 frames" to "position_event_interval_ms of audio"
  - Changed: "Every 5 events" to interval-based logic with setting name
  - Added: Default values for clarity
- **Updated:** Design Rationale section (lines 662-665)
  - Added: Configurable intervals bullet point
  - Added: Both database settings listed
  - Added: Cross-reference to database schema

#### 4. GUIDE001-wkmp_ap_implementation_plan.md

**Changes:**
- **Updated:** Event Emission Timing section (lines 256-268)
  - Added: Database setting names for both intervals
  - Added: Configuration details for each event type
  - Added: Cross-reference to database schema
  - Clarified: Event-driven vs timer-based distinction

---

## Configuration Storage Location

**All event timing intervals are stored in:**

**Table:** `settings` (Global settings table)
**Schema:** See [IMPL001-database_schema.md](IMPL001-database_schema.md)
**Access:** Read-only during playback, updated via configuration API

**Database Design:**
```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT  -- All settings stored as TEXT, parsed by application
);

-- Example rows:
INSERT INTO settings (key, value) VALUES ('position_event_interval_ms', '1000');
INSERT INTO settings (key, value) VALUES ('playback_progress_interval_ms', '5000');
```

**Application Loading:**
```rust
// wkmp-ap/src/config.rs (pseudo-code)
struct PlaybackConfig {
    position_event_interval_ms: u32,  // Default: 1000
    playback_progress_interval_ms: u32,  // Default: 5000
}

fn load_from_database(pool: &SqlitePool) -> PlaybackConfig {
    let position_interval = query_setting("position_event_interval_ms")
        .unwrap_or(1000);  // Default if NULL/missing
    let progress_interval = query_setting("playback_progress_interval_ms")
        .unwrap_or(5000);  // Default if NULL/missing

    PlaybackConfig {
        position_event_interval_ms: position_interval,
        playback_progress_interval_ms: progress_interval,
    }
}
```

---

## Purpose and Application Point Clarification

### Position Event Interval (`position_event_interval_ms`)

**Purpose:**
Controls how frequently the mixer generates internal `PositionUpdate` events for song boundary detection.

**Application Points:**
1. **Mixer frame generation** (`wkmp-ap/src/playback/pipeline/mixer.rs`)
   - Method: `CrossfadeMixer::get_next_frame()`
   - Logic: Frame counter incremented every call
   - Emission: When `frame_count >= (interval_ms / 1000.0) * sample_rate`

2. **Song timeline checking** (`wkmp-ap/src/playback/engine.rs`)
   - Method: `position_event_handler()`
   - Logic: Receives `PositionUpdate` events
   - Action: Checks `SongTimeline::check_boundary(position_ms)`

**Affects:**
- Song boundary detection latency (0 to interval_ms)
- `CurrentSongChanged` event emission timing
- CPU usage (inversely proportional to interval)

---

### PlaybackProgress Event Interval (`playback_progress_interval_ms`)

**Purpose:**
Controls how frequently `PlaybackProgress` SSE events are sent to UI clients.

**Application Points:**
1. **Position event handler** (`wkmp-ap/src/playback/engine.rs`)
   - Method: `position_event_handler()`
   - Logic: Tracks `last_progress_position_ms`
   - Emission: When `current_position_ms - last_progress_position_ms >= interval_ms`

2. **SSE broadcaster** (`wkmp-ap/src/api/sse.rs`)
   - Receives `WkmpEvent::PlaybackProgress`
   - Serializes to JSON
   - Sends to all connected SSE clients

**Affects:**
- UI progress bar update smoothness
- Network traffic (inversely proportional to interval)
- Client responsiveness perception

---

## Interval Independence and Interaction

These intervals are **independent** but work together in the event pipeline:

```
Frame Generation
  └─> Every position_event_interval_ms (default: 1000ms)
      └─> Emit PositionUpdate (internal event)
          └─> Position Event Handler
              ├─> Check song boundaries → CurrentSongChanged (immediate if crossed)
              └─> Check progress interval → PlaybackProgress (if elapsed)
                  └─> Every playback_progress_interval_ms (default: 5000ms)
```

**Typical Relationship:**
- Position interval: 1000ms (1 second)
- Progress interval: 5000ms (5 seconds)
- Ratio: 1:5 (5 position events for each progress event)

**Configurable Independently:**
- Can set position to 500ms and progress to 10000ms
- Can set position to 2000ms and progress to 2000ms (1:1 ratio)
- No enforced relationship between the two

---

## Migration Impact

### Changes from REV002 (Initial Event-Driven Update)

| Aspect | REV002 (Initial) | REV002 + Addendum |
|--------|------------------|-------------------|
| Position event interval | Described as "~1 second", not explicitly configurable | Database setting `position_event_interval_ms` = 1000ms |
| PlaybackProgress logic | "Every 5 events" | "Based on playback time elapsed >= interval_ms" |
| Documentation detail | Basic event flow | Comprehensive configuration guidance |
| Cross-references | Minimal | Extensive linking across Tier 2/3 docs |

### No Code Changes Required (Yet)

This addendum is **documentation-only**. The implementation will need to:

1. **Read `position_event_interval_ms` from database**
2. **Convert to frame count:** `(interval_ms / 1000.0) * sample_rate`
3. **Check frame counter in `get_next_frame()`**
4. **Emit event when threshold reached**

---

## Consistency Verification

### Cross-Document References Added

| Source | Target | Link Type |
|--------|--------|-----------|
| SPEC001-architecture.md:360 | IMPL001-database_schema.md#event-timing-intervals | Configuration details |
| SPEC001-architecture.md:511 | IMPL001-database_schema.md#event-timing-intervals | Configuration guidance |
| SPEC011-event_system.md:602 | IMPL001-database_schema.md | Interval configuration |
| SPEC011-event_system.md:665 | IMPL001-database_schema.md#event-timing-intervals | Detailed guidance |
| GUIDE001-wkmp_ap_implementation_plan.md:268 | IMPL001-database_schema.md#event-timing-intervals | Configuration details |

### Terminology Consistency

| Term | Used In | Consistent? |
|------|---------|-------------|
| "position_event_interval_ms" | All documents | ✅ Yes |
| "playback_progress_interval_ms" | All documents | ✅ Yes |
| "event-driven" | All documents | ✅ Yes |
| "Internal event" vs "External event" | SPEC011, IMPL001 | ✅ Yes |
| "playback time, not wall clock time" | SPEC001, IMPL001, GUIDE001 | ✅ Yes |

---

## Summary

This addendum ensures comprehensive configurability documentation for event timing intervals:

✅ **Both intervals clearly marked as configurable**
✅ **Database storage location specified (settings table)**
✅ **Purpose and application points uniquely identified**
✅ **Consistent references across all Tier 2/3/4 documents**
✅ **Comprehensive configuration guidance provided**
✅ **Trade-offs and recommendations documented**

**Next Steps:**
1. Implement code to read `position_event_interval_ms` from database
2. Update mixer to use configurable interval instead of hardcoded frame count
3. Test with various interval configurations
4. Update API documentation if runtime configuration changes are supported

---

**This addendum completes the event timing configuration documentation suite.**
