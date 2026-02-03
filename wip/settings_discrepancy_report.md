# Settings Reference Discrepancy Analysis

**Generated:** 2025-11-16
**Purpose:** Comprehensive comparison of settings documented in IMPL016-settings_reference.md against actual database initialization in wkmp-common/src/db/init.rs and runtime usage across the codebase.

---

## Executive Summary

**Key Findings:**
- **10 settings** documented in IMPL016 but NOT initialized in init.rs
- **2 settings** initialized in init.rs but NOT documented in IMPL016
- **2 settings** accessed in code but NOT in either IMPL016 or init.rs
- **3 discrepancies** in default values between IMPL016 and init.rs (**FIXED** - init.rs is now source of truth)

---

## 1. Settings in IMPL016 (61 total)

Listed by line number with database key:

| Line | Setting Key | Default (per IMPL016) |
|------|-------------|----------------------|
| 56 | `acoustid_api_key` | NULL |
| 70 | `ai_database_connection_pool_size` | 96 |
| 86 | `ai_database_lock_retry_ms` | 250 |
| 101 | `ai_database_max_lock_wait_ms` | 5000 |
| 115 | `ai_longwork_yield_interval_ms` | 990 |
| 130 | `ai_processing_thread_count` | NULL (auto-detect) |
| 144 | `audio_buffer_size` | 2208 |
| 167 | `audio_sink` | "default" |
| 181 | `backup_interval_ms` | 7776000000 |
| 196 | `backup_location` | Same folder as wkmp.db |
| 210 | `backup_minimum_interval_ms` | 1209600000 |
| 224 | `backup_retention_count` | 3 |
| 238 | `currently_playing_passage_id` | NULL |
| 252 | `decode_chunk_size` | 32000 |
| 269 | `decode_work_period` | 5000 |
| 284 | `decoder_resume_hysteresis_samples` | 44100 |
| 304 | `global_crossfade_time` | 2.0 |
| 318 | `global_fade_curve` | "exponential_logarithmic" |
| 332 | `http_base_ports` | [5720, 15720, 25720, 17200, 23400] |
| 346 | `http_keepalive_timeout_ms` | 60000 |
| 360 | `http_max_body_size_bytes` | 1048576 |
| 374 | `http_request_timeout_ms` | 30000 |
| 388 | `ingest_max_concurrent_jobs` | 12 |
| 404 | `initial_play_state` | "playing" |
| 418 | `last_backup_timestamp_ms` | NULL |
| 432 | `last_played_passage_id` | NULL |
| 446 | `last_played_position_ticks` | 0 |
| 461 | `lead_in_threshold_dB` | 45.0 |
| 480 | `lead_out_threshold_dB` | 40.0 |
| 499 | `maximum_decode_streams` | 12 |
| 515 | `minimum_passage_audio_duration_ticks` | 2822400 (100ms) |
| 530 | `mixer_batch_size_low` | 512 |
| 549 | `mixer_batch_size_optimal` | 256 |
| 568 | `mixer_check_interval_ms` | 10 |
| 591 | `mixer_min_start_level` | 22050 |
| 610 | `music_directories` | [] |
| 625 | `output_ringbuffer_size` | 8192 |
| 646 | `pause_decay_factor` | 0.95 |
| 662 | `pause_decay_floor` | 0.0001778 |
| 677 | `playback_failure_threshold` | 3 |
| 692 | `playback_failure_window_seconds` | 60 |
| 706 | `playback_progress_interval_ms` | 5000 |
| 722 | `playout_ringbuffer_headroom` | 4410 |
| 739 | `playout_ringbuffer_size` | 661941 |
| 756 | `position_event_interval_ms` | 1000 |
| 782 | `queue_current_id` | NULL |
| 802 | `queue_entry_timing_overrides` | {} |
| 826 | `queue_max_enqueue_batch` | 5 |
| 840 | `queue_max_size` | 100 |
| 854 | `queue_refill_acknowledgment_timeout_seconds` | 5 |
| 868 | `queue_refill_request_throttle_seconds` | 10 |
| 882 | `queue_refill_threshold_passages` | 2 |
| 897 | `queue_refill_threshold_seconds` | 900 (15 min) |
| 912 | `relaunch_attempts` | 20 |
| 927 | `relaunch_delay` | 5000 (milliseconds) |
| 941 | `resume_from_pause_fade_in_curve` | "exponential" |
| 955 | `resume_from_pause_fade_in_duration` | 0.5 |
| 969 | `session_timeout_seconds` | 31536000 (1 year) |
| 983 | `silence_min_duration_ticks` | 8467200 (300ms) |
| 998 | `silence_threshold_dB` | 35.0 |
| 1012 | `temporary_flavor_override` | NULL |
| 1033 | `validation_enabled` | "true" |
| 1051 | `validation_interval_secs` | 10 |
| 1070 | `validation_tolerance_samples` | 8192 |
| 1093 | `volume_fade_update_period` | 10 |
| 1111 | `volume_level` | 0.5 |
| 1126 | `working_sample_rate` | 44100 |

---

## 2. Settings in init.rs (46 total)

From `wkmp-common\src\db\init.rs`, function `init_default_settings()` (lines 188-268):

| Line | Setting Key | Default (per init.rs) |
|------|-------------|----------------------|
| 190 | `initial_play_state` | "playing" |
| 191 | `volume_level` | "0.5" |
| 192 | `global_crossfade_time` | "2.0" |
| 193 | `volume_fade_update_period` | "10" |
| 196 | `audio_sink` | "default" |
| 197 | `position_event_interval_ms` | "1000" |
| 198 | `playback_progress_interval_ms` | "5000" |
| 201 | `queue_max_size` | "100" |
| 202 | `queue_refill_threshold_passages` | "2" |
| 203 | `queue_refill_threshold_seconds` | "900" |
| 204 | `queue_refill_request_throttle_seconds` | "10" |
| 205 | `queue_refill_acknowledgment_timeout_seconds` | "5" |
| 206 | `queue_max_enqueue_batch` | "5" |
| 209 | `session_timeout_seconds` | "31536000" |
| 212 | `backup_interval_ms` | "7776000000" |
| 213 | `backup_minimum_interval_ms` | "1209600000" |
| 214 | `backup_retention_count` | "3" |
| 215 | `backup_location` | "" |
| 216 | `last_backup_timestamp_ms` | "0" |
| 219 | `http_base_ports` | "[5720, 15720, 25720, 17200, 23400]" |
| 220 | `http_request_timeout_ms` | "30000" |
| 221 | `http_keepalive_timeout_ms` | "60000" |
| 222 | `http_max_body_size_bytes` | "1048576" |
| 225 | `relaunch_delay` | "5000" |
| 226 | `relaunch_attempts` | "20" |
| 229 | `playback_failure_threshold` | "3" |
| 230 | `playback_failure_window_seconds` | "60" |
| 235 | `ingest_max_concurrent_jobs` | "12" |
| 239 | `ai_database_max_lock_wait_ms` | "5000" |
| 240 | `ai_database_lock_retry_ms` | "250" |
| 245 | `ai_longwork_yield_interval_ms` | "990" |
| 248 | `validation_enabled` | "true" |
| 249 | `validation_interval_secs` | "10" |
| 250 | `validation_tolerance_samples` | "8192" |
| 254 | `working_sample_rate` | "44100" |
| 255 | `output_ringbuffer_size` | "8192" |
| 256 | `maximum_decode_streams` | "12" |
| 257 | `decode_work_period` | "5000" |
| 258 | `chunk_duration_ms` | "1000" |
| 259 | `playout_ringbuffer_size` | "661941" |
| 260 | `playout_ringbuffer_headroom` | "4410" |
| 261 | `decoder_resume_hysteresis_samples` | "44100" |
| 262 | `mixer_min_start_level` | "22050" |
| 263 | `pause_decay_factor` | "0.95" |
| 264 | `pause_decay_floor` | "0.0001778" |
| 265 | `audio_buffer_size` | "2208" |
| 266 | `mixer_check_interval_ms` | "10" |

---

## 3. Settings Accessed in Code (additional findings)

From grepping the codebase for settings table queries:

| File | Line | Setting Key | Usage |
|------|------|-------------|-------|
| `wkmp-common\src\api\auth.rs` | 105 | `api_shared_secret` | Read/generate auth token |
| `wkmp-ap\src\api\handlers.rs` | 1027 | `root_folder` | Read root folder path |
| `wkmp-ap\src\db\settings.rs` | 140 | `last_played_position_ms` | Save playback position |
| `wkmp-ap\src\db\settings.rs` | 149 | `last_played_position_ms` | Load playback position |
| `wkmp-ap\src\db\settings.rs` | 357 | `mixer_batch_size_low` | Load mixer config |
| `wkmp-ap\src\db\settings.rs` | 362 | `mixer_batch_size_optimal` | Load mixer config |
| `wkmp-ap\src\db\settings.rs` | 118 | `global_fade_curve` | Load crossfade curve |
| `wkmp-ai\src\services\passage_amplitude_analyzer.rs` | 339 | `lead_in_threshold_dB` | Read threshold |
| `wkmp-ai\src\services\passage_amplitude_analyzer.rs` | 350 | `lead_out_threshold_dB` | Read threshold |
| `wkmp-ai\src\models\bootstrap_config.rs` | 86 | `ai_database_connection_pool_size` | Read pool size |
| `wkmp-ai\src\models\bootstrap_config.rs` | 97 | `ai_processing_thread_count` | Read thread count |

---

## 4. Discrepancy Analysis

### 4.1 Settings in IMPL016 but NOT in init.rs

**Settings documented but not initialized (28 total):**

| Setting | IMPL016 Default | Status |
|---------|----------------|--------|
| `acoustid_api_key` | NULL | **OK** - User-configured API key |
| `ai_database_connection_pool_size` | 96 | **MISSING** - Hardcoded in `init_database()` line 28 |
| `ai_processing_thread_count` | NULL | **OK** - Auto-detected at runtime |
| `currently_playing_passage_id` | NULL | **OK** - Runtime state |
| `decode_chunk_size` | 32000 | **MISSING** - Should be initialized |
| `global_fade_curve` | "exponential_logarithmic" | **MISSING** - Should be initialized |
| `last_played_passage_id` | NULL | **OK** - Runtime state |
| `last_played_position_ticks` | 0 | **UNIT MISMATCH** - Code uses `last_played_position_ms` |
| `lead_in_threshold_dB` | 45.0 | **MISSING** - wkmp-ai reads this setting |
| `lead_out_threshold_dB` | 40.0 | **MISSING** - wkmp-ai reads this setting |
| `minimum_passage_audio_duration_ticks` | 2822400 | **MISSING** - Should be initialized |
| `mixer_batch_size_low` | 512 | **MISSING** - Code has fallback but inconsistent |
| `mixer_batch_size_optimal` | 256 | **MISSING** - Code has fallback but inconsistent |
| `music_directories` | [] | **OK** - User-configured list |
| `queue_current_id` | NULL | **OK** - Runtime state |
| `queue_entry_timing_overrides` | {} | **OK** - User-configured overrides |
| `resume_from_pause_fade_in_curve` | "exponential" | **MISSING** - Should be initialized |
| `resume_from_pause_fade_in_duration` | 0.5 | **MISSING** - Should be initialized |
| `silence_min_duration_ticks` | 8467200 | **MISSING** - Should be initialized |
| `silence_threshold_dB` | 35.0 | **MISSING** - Should be initialized |
| `temporary_flavor_override` | NULL | **OK** - Runtime-only setting |

### 4.2 Settings in init.rs but NOT in IMPL016

**Settings initialized but undocumented (1 total):**

1. **`chunk_duration_ms`** (init.rs:258)
   - Default: "1000"
   - **Status:** UNDOCUMENTED - Should be added to IMPL016
   - **Note:** May be related to `decode_chunk_size` or separate parameter

### 4.3 Settings Accessed in Code but NOT in Either

**Runtime-only settings found in code (3 total):**

1. **`api_shared_secret`** (wkmp-common\src\api\auth.rs:105)
   - Default: Generated at runtime (non-zero i64)
   - **Status:** RUNTIME-ONLY - Auto-generated, not pre-configured
   - **Recommendation:** Document in IMPL016 as runtime-generated setting

2. **`root_folder`** (wkmp-ap\src\api\handlers.rs:1027)
   - Default: Resolved from 4-tier priority system
   - **Status:** ARCHITECTURAL VIOLATION - Not a database setting
   - **Recommendation:** Remove from settings table queries, use config system

3. **`last_played_position_ms`** (wkmp-ap\src\db\settings.rs:140, 149)
   - Default: 0 (implied)
   - **Status:** UNIT MISMATCH with IMPL016 `last_played_position_ticks`
   - **Recommendation:** Standardize on ticks vs milliseconds per SPEC017

### 4.4 Default Value Discrepancies (**RESOLVED**)

| Setting | IMPL016 (Before) | init.rs | IMPL016 (After) | Status |
|---------|-----------------|---------|----------------|--------|
| `relaunch_delay` | 5 seconds | "5000" ms | 5000 ms | ✅ **FIXED** |
| `pause_decay_factor` | 0.96875 (31/32) | "0.95" | 0.95 | ✅ **FIXED** |
| `playout_ringbuffer_headroom` | 32768 | "4410" | 4410 | ✅ **FIXED** |
| `last_backup_timestamp_ms` | NULL | "0" | NULL | **Minor** - NULL vs 0 |
| `backup_location` | "Same folder" | "" | "Same folder" | **Minor** - Description vs empty string |

**Note:** init.rs is now established as source of truth. IMPL016 has been updated to match init.rs defaults.

---

## 5. Recommendations

### 5.1 HIGH PRIORITY - Add Missing Initializations

Add to `init_default_settings()` in wkmp-common/src/db/init.rs:

```rust
// Crossfade settings (missing from init.rs)
ensure_setting(pool, "global_fade_curve", "exponential_logarithmic").await?;
ensure_setting(pool, "resume_from_pause_fade_in_curve", "exponential").await?;
ensure_setting(pool, "resume_from_pause_fade_in_duration", "0.5").await?;

// Decoder/Mixer settings (missing from init.rs)
ensure_setting(pool, "decode_chunk_size", "32000").await?;
ensure_setting(pool, "mixer_batch_size_low", "512").await?;
ensure_setting(pool, "mixer_batch_size_optimal", "256").await?;

// Audio ingest thresholds (missing from init.rs)
ensure_setting(pool, "lead_in_threshold_dB", "45.0").await?;
ensure_setting(pool, "lead_out_threshold_dB", "40.0").await?;
ensure_setting(pool, "silence_threshold_dB", "35.0").await?;
ensure_setting(pool, "silence_min_duration_ticks", "8467200").await?;  // 300ms
ensure_setting(pool, "minimum_passage_audio_duration_ticks", "2822400").await?;  // 100ms
```

### 5.2 MEDIUM PRIORITY - Document Undocumented Settings

Add to IMPL016-settings_reference.md:

1. **`chunk_duration_ms`**
   - Document purpose, units, valid range
   - Clarify relationship to `decode_chunk_size` if any

2. **`api_shared_secret`**
   - Document as runtime-generated authentication token
   - Note: Auto-generated at startup, not user-configurable

### 5.3 LOW PRIORITY - Remove Invalid Settings Queries

**Fix `root_folder` query:**
- File: `wkmp-ap\src\api\handlers.rs:1027`
- Issue: Queries settings table for `root_folder` which should come from config system
- Fix: Use `wkmp_common::config::RootFolderResolver` instead

### 5.4 ARCHITECTURAL - Standardize Position Tracking Units

**Fix position tracking unit mismatch:**
- IMPL016 documents: `last_played_position_ticks`
- Code uses: `last_played_position_ms` (milliseconds)
- **Decision needed:** Standardize on ticks (per SPEC017) or milliseconds
- **Recommendation:** Use ticks for consistency with all other timing fields

### 5.5 ARCHITECTURAL - Connection Pool Size

**Fix `ai_database_connection_pool_size` hardcoding:**
- Currently: Hardcoded in `init_database()` at line 28 (`max_connections(96)`)
- Should: Read from settings table like other structural parameters
- Impact: Cannot be configured without code changes

---

## 6. Impact Assessment

### 6.1 Production Impact

**HIGH RISK (RESOLVED):**
- ✅ `relaunch_delay` mismatch: **FIXED** - Now correctly 5000ms
- ✅ `playout_ringbuffer_headroom`: **FIXED** - Now correctly 4410 samples
- ✅ `pause_decay_factor`: **FIXED** - Now correctly 0.95

**MEDIUM RISK:**
- Missing mixer batch sizes: Fallback values exist in code but inconsistent initialization
- Missing crossfade settings: Fallback values exist but inconsistent behavior
- Hardcoded connection pool: Cannot be tuned without code changes

**LOW RISK:**
- Missing amplitude thresholds: Only affects wkmp-ai import (Full version only)
- Missing validation settings: Already initialized in init.rs
- Position tracking unit mismatch: Runtime state, not critical

### 6.2 Development Impact

**Documentation Trust:**
- IMPL016 now matches init.rs for common settings
- Remaining discrepancies are missing initializations, not conflicting defaults

**Configuration Fragility:**
- 13 settings rely on code fallbacks rather than database defaults
- Inconsistent behavior between fresh database and existing database

---

## 7. Verification SQL Query

To verify all documented settings are initialized, run after applying fixes:

```sql
-- Check for missing settings from IMPL016
SELECT 'MISSING: ' || key AS status, key
FROM (
  VALUES
    ('acoustid_api_key'),
    ('ai_database_connection_pool_size'),
    ('ai_database_lock_retry_ms'),
    ('ai_database_max_lock_wait_ms'),
    ('ai_longwork_yield_interval_ms'),
    ('ai_processing_thread_count'),
    ('audio_buffer_size'),
    ('audio_sink'),
    ('backup_interval_ms'),
    ('backup_location'),
    ('backup_minimum_interval_ms'),
    ('backup_retention_count'),
    ('chunk_duration_ms'),
    ('decode_chunk_size'),
    ('decode_work_period'),
    ('decoder_resume_hysteresis_samples'),
    ('global_crossfade_time'),
    ('global_fade_curve'),
    ('lead_in_threshold_dB'),
    ('lead_out_threshold_dB'),
    ('mixer_batch_size_low'),
    ('mixer_batch_size_optimal'),
    ('mixer_check_interval_ms'),
    ('mixer_min_start_level'),
    ('minimum_passage_audio_duration_ticks'),
    ('pause_decay_factor'),
    ('pause_decay_floor'),
    ('playout_ringbuffer_headroom'),
    ('playout_ringbuffer_size'),
    ('resume_from_pause_fade_in_curve'),
    ('resume_from_pause_fade_in_duration'),
    ('silence_min_duration_ticks'),
    ('silence_threshold_dB')
) AS expected(key)
WHERE key NOT IN (SELECT key FROM settings);

-- Check for unexpected settings not in IMPL016
SELECT 'UNDOCUMENTED: ' || key AS status, key, value
FROM settings
WHERE key NOT IN (
  'acoustid_api_key',
  'ai_database_connection_pool_size',
  'ai_database_lock_retry_ms',
  'ai_database_max_lock_wait_ms',
  'ai_longwork_yield_interval_ms',
  'ai_processing_thread_count',
  'audio_buffer_size',
  'audio_sink',
  'backup_interval_ms',
  'backup_location',
  'backup_minimum_interval_ms',
  'backup_retention_count',
  'chunk_duration_ms',
  'currently_playing_passage_id',
  'decode_chunk_size',
  'decode_work_period',
  'decoder_resume_hysteresis_samples',
  'global_crossfade_time',
  'global_fade_curve',
  'http_base_ports',
  'http_keepalive_timeout_ms',
  'http_max_body_size_bytes',
  'http_request_timeout_ms',
  'ingest_max_concurrent_jobs',
  'initial_play_state',
  'last_backup_timestamp_ms',
  'last_played_passage_id',
  'last_played_position_ticks',
  'lead_in_threshold_dB',
  'lead_out_threshold_dB',
  'maximum_decode_streams',
  'minimum_passage_audio_duration_ticks',
  'mixer_batch_size_low',
  'mixer_batch_size_optimal',
  'mixer_check_interval_ms',
  'mixer_min_start_level',
  'music_directories',
  'output_ringbuffer_size',
  'pause_decay_factor',
  'pause_decay_floor',
  'playback_failure_threshold',
  'playback_failure_window_seconds',
  'playback_progress_interval_ms',
  'playout_ringbuffer_headroom',
  'playout_ringbuffer_size',
  'position_event_interval_ms',
  'queue_current_id',
  'queue_entry_timing_overrides',
  'queue_max_enqueue_batch',
  'queue_max_size',
  'queue_refill_acknowledgment_timeout_seconds',
  'queue_refill_request_throttle_seconds',
  'queue_refill_threshold_passages',
  'queue_refill_threshold_seconds',
  'relaunch_attempts',
  'relaunch_delay',
  'resume_from_pause_fade_in_curve',
  'resume_from_pause_fade_in_duration',
  'session_timeout_seconds',
  'silence_min_duration_ticks',
  'silence_threshold_dB',
  'temporary_flavor_override',
  'validation_enabled',
  'validation_interval_secs',
  'validation_tolerance_samples',
  'volume_fade_update_period',
  'volume_level',
  'working_sample_rate'
);
```

---

## 8. Next Steps

1. **Immediate:** Review this report and confirm priorities
2. **Short-term:** Add missing `ensure_setting()` calls to init.rs (10 settings)
3. **Medium-term:** Document `chunk_duration_ms` and `api_shared_secret` in IMPL016
4. **Long-term:** Standardize position tracking units (ticks vs milliseconds)
5. **Architectural:** Move connection pool size from hardcoded to settings-based

---

**Report Status:** Complete
**Action Required:** Yes - 13 missing initializations, 2 undocumented settings, 1 architectural fix
