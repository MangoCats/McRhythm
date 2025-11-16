# WKMP Settings Table Reference

**⚙️ TIER 3 - IMPLEMENTATION SPECIFICATION**

**Document ID:** IMPL016-settings_reference.md
**Status:** Production Ready
**Created:** 2025-11-15
**Purpose:** Single authoritative reference for all settings table parameters

> **Related Documentation:** [Database Schema](IMPL001-database_schema.md) | [Decoder Buffer Design](SPEC016-decoder_buffer_design.md) | [Parameter Management](IMPL010-parameter_management.md) | [Amplitude Analysis](SPEC025-amplitude_analysis.md)

---

## Executive Summary

This document serves as the **single authoritative reference** for all parameters stored in the `settings` table. Each parameter is documented with consistent metadata: database key, type, default value, units, valid range, modification impact, owning module(s), and cross-references to defining specifications.

**Purpose:**
- Eliminate scattered parameter documentation across multiple specifications
- Provide standardized metadata for all settings (units, ranges, defaults, modification impact)
- Enable quick lookup for developers, operators, and automated tools
- Document parameter interdependencies and conversion formulas

**Scope:** All parameters in the `settings` table across all six WKMP microservices.

---

## Parameter Classification

### Modification Impact Categories

Parameters are classified by their modification behavior:

| Category | Description | Effect Timeline | Examples |
|----------|-------------|-----------------|----------|
| **IMMEDIATE** | Changes take effect during operation without restart | Immediate | `volume_level`, `audio_sink` |
| **RESTART_REQUIRED** | Structural parameters read once at startup | Next restart | `working_sample_rate`, `audio_buffer_size`, `output_ringbuffer_size` |
| **REIMPORT_REQUIRED** | Changes require re-running file import workflow | Next import | `silence_threshold_dB`, `lead_in_threshold_dB` |
| **STATE_PERSISTENCE** | Runtime state tracking, not user-configurable | N/A (automatic) | `queue_current_id`, `currently_playing_passage_id` |

### Module Ownership

| Module | Abbreviation | Port | Version Availability |
|--------|--------------|------|---------------------|
| Audio Player | wkmp-ap | 5721 | All |
| User Interface | wkmp-ui | 5720 | All |
| Program Director | wkmp-pd | 5722 | Full, Lite |
| Audio Ingest | wkmp-ai | 5723 | Full |
| Lyric Editor | wkmp-le | 5724 | Full |
| Database Review | wkmp-dr | 5725 | Full |

---

## Parameter Index (Alphabetical)

### acoustid_api_key

- **Database Key:** `acoustid_api_key`
- **Type:** TEXT (nullable)
- **Default:** `NULL`
- **Units:** N/A (API key string)
- **Valid Range:** Valid AcoustID API key format (alphanumeric string)
- **Modification Impact:** IMMEDIATE (validated at workflow start)
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1035](IMPL001-database_schema.md#settings), [IMPL010:365-372](IMPL010-parameter_management.md)
- **Description:** AcoustID API key for audio fingerprinting service (Phase 5). Required for automatic MusicBrainz recording identification. Obtain from https://acoustid.org/api-key.

---

### ai_database_connection_pool_size

- **Database Key:** `ai_database_connection_pool_size`
- **Type:** INTEGER
- **Default:** `96`
- **Units:** connections
- **Valid Range:** 1-500 (recommended: 8 × max_concurrent_jobs)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1026](IMPL001-database_schema.md#settings), [ARCH-PERF-020]
- **Description:** SQLite connection pool size for audio ingest pipeline. Sized for 12 workers × 8 connections per worker. Higher values support more concurrent import jobs but increase memory usage (~1 MB per connection).
- **Interdependencies:** Should be ≥ `ingest_max_concurrent_jobs` × 8 for optimal performance
- **Performance Notes:** Each connection holds ~1 MB of SQLite cache. 96 connections = ~96 MB baseline memory.

---

### ai_database_lock_retry_ms

- **Database Key:** `ai_database_lock_retry_ms`
- **Type:** INTEGER
- **Default:** `250`
- **Units:** milliseconds
- **Valid Range:** 50-5000
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1029](IMPL001-database_schema.md#settings)
- **Description:** SQLite `busy_timeout` - time to wait for database lock before returning error to retry logic. Lower values fail faster, higher values reduce retry overhead.
- **Interdependencies:** Works with `ai_database_max_lock_wait_ms` for total retry budget

---

### ai_database_max_lock_wait_ms

- **Database Key:** `ai_database_max_lock_wait_ms`
- **Type:** INTEGER
- **Default:** `5000`
- **Units:** milliseconds
- **Valid Range:** 500-30000
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1028](IMPL001-database_schema.md#settings)
- **Description:** Maximum total time for retry logic to attempt database operations before giving up. Prevents infinite retry loops during severe lock contention.

---

### ai_processing_thread_count

- **Database Key:** `ai_processing_thread_count`
- **Type:** INTEGER (nullable)
- **Default:** `NULL` (auto-initialized to CPU core count + 1)
- **Units:** threads
- **Valid Range:** 1-64 (recommended: CPU cores × 1.5)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1036](IMPL001-database_schema.md#settings)
- **Description:** Thread pool size for parallel audio processing (decoding, fingerprinting). NULL triggers automatic detection at startup.

---

### audio_buffer_size

- **Database Key:** `audio_buffer_size`
- **Type:** INTEGER
- **Default:** `2208`
- **Units:** frames (stereo samples)
- **Valid Range:** 64-65536 (powers of 2 recommended for efficiency)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-110](SPEC016-decoder_buffer_design.md#audio_buffer_size), [GUIDE003:858](GUIDE003_audio_pipeline_diagrams.md), [IMPL001:989](IMPL001-database_schema.md#settings)
- **Description:** Audio device callback buffer size. Controls latency vs. stability tradeoff.
- **Time Equivalent:** 50.1 ms @ 44.1kHz
- **Tuning:**
  - **Smaller (64-704 frames):** Lower latency, higher CPU usage, less stable
  - **Default (2208 frames):** Balanced - 50ms latency, VeryHigh stability confidence
  - **Larger (4096+ frames):** Higher latency, more stable on slower systems
- **Interdependencies:**
  - Callback frequency = `working_sample_rate` / `audio_buffer_size`
  - At 44.1kHz with 2208 frames: ~21.5 callbacks/second
  - Mixer batch sizes ([DBD-PARAM-112/113](#mixer_batch_size_low)) must sustain fill rate

---

### audio_sink

- **Database Key:** `audio_sink`
- **Type:** TEXT
- **Default:** `"default"`
- **Units:** N/A (device identifier string)
- **Valid Range:** Valid cpal audio device identifier or "default"
- **Modification Impact:** IMMEDIATE (requires stop/restart playback pipeline)
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:990](IMPL001-database_schema.md#settings)
- **Description:** Selected audio output device. Use "default" for system default device, or specific device ID from cpal enumeration.

---

### backup_interval_ms

- **Database Key:** `backup_interval_ms`
- **Type:** INTEGER
- **Default:** `7776000000` (90 days)
- **Units:** milliseconds
- **Valid Range:** 86400000-31536000000 (1 day to 1 year)
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ui (All versions)
- **Defined In:** [IMPL001:1000](IMPL001-database_schema.md#settings)
- **Description:** Periodic automatic backup interval. Backup occurs when (current_time - `last_backup_timestamp_ms`) > `backup_interval_ms`.
- **Conversion:** 1 day = 86,400,000 ms; 90 days = 7,776,000,000 ms

---

### backup_location

- **Database Key:** `backup_location`
- **Type:** TEXT
- **Default:** Same folder as `wkmp.db`
- **Units:** N/A (filesystem path)
- **Valid Range:** Valid writable directory path
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ui (All versions)
- **Defined In:** [IMPL001:999](IMPL001-database_schema.md#settings)
- **Description:** Directory path for automatic database backups. Backups are named `wkmp_backup_YYYYMMDD_HHMMSS.db`.

---

### backup_minimum_interval_ms

- **Database Key:** `backup_minimum_interval_ms`
- **Type:** INTEGER
- **Default:** `1209600000` (14 days)
- **Units:** milliseconds
- **Valid Range:** 3600000-7776000000 (1 hour to 90 days)
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ui (All versions)
- **Defined In:** [IMPL001:1001](IMPL001-database_schema.md#settings)
- **Description:** Minimum time between startup backups. Prevents backup spam when restarting frequently during development/testing.

---

### backup_retention_count

- **Database Key:** `backup_retention_count`
- **Type:** INTEGER
- **Default:** `3`
- **Units:** backups
- **Valid Range:** 1-100
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ui (All versions)
- **Defined In:** [IMPL001:1002](IMPL001-database_schema.md#settings)
- **Description:** Number of timestamped backups to retain. Oldest backups are deleted automatically when count exceeds this limit.

---

### currently_playing_passage_id

- **Database Key:** `currently_playing_passage_id`
- **Type:** TEXT (UUID, nullable)
- **Default:** `NULL`
- **Units:** N/A (UUID)
- **Valid Range:** Valid UUID or NULL
- **Modification Impact:** STATE_PERSISTENCE (automatic)
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:985](IMPL001-database_schema.md#settings)
- **Description:** UUID of passage currently playing. Automatically updated by playback engine, not user-configurable. NULL when stopped/paused.

---

### decode_chunk_size

- **Database Key:** `decode_chunk_size`
- **Type:** INTEGER
- **Default:** `32000`
- **Units:** samples (output from resampler per chunk)
- **Valid Range:** 4410-88200 (0.1-2.0 seconds @ 44.1kHz)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-065](SPEC016-decoder_buffer_design.md#decode_chunk_size), [GUIDE003:851](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Resampler output chunk size. Balances decode granularity vs. overhead.
- **Time Equivalent:** ~0.73 seconds @ 44.1kHz
- **Behavior:** When file sample rate differs from `working_sample_rate`, decoder may send larger/smaller chunks to resampler before downsampling to this chunk size.
- **Calculation:** `decoder_output_max_chunk = decode_chunk_size × (file_sample_rate / working_sample_rate)`

---

### decode_work_period

- **Database Key:** `decode_work_period`
- **Type:** INTEGER
- **Default:** `5000`
- **Units:** milliseconds
- **Valid Range:** 1000-30000
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-060](SPEC016-decoder_buffer_design.md#decode_work_period), [GUIDE003:850](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Interval between decode job priority queue evaluations. Prevents low-priority long decodes (30-minute file) from starving high-priority decodes ("now playing" buffer).
- **Behavior:** Decoder checks priority queue at chunk boundaries (typically ~1s intervals). If higher-priority job exists, current decoder pauses and yields.

---

### decoder_resume_hysteresis_samples

- **Database Key:** `decoder_resume_hysteresis_samples`
- **Type:** INTEGER
- **Default:** `44100`
- **Units:** samples (stereo)
- **Valid Range:** 882-88200 (0.02-2.0 seconds @ 44.1kHz)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-085](SPEC016-decoder_buffer_design.md#decoder_resume_hysteresis_samples), [GUIDE003:854](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Hysteresis gap between decoder pause and resume thresholds. Prevents rapid oscillation.
- **Time Equivalent:** 1.0 second @ 44.1kHz
- **Behavior:**
  - Pause threshold: `free_space ≤ playout_ringbuffer_headroom` (4410 samples)
  - Resume threshold: `free_space ≥ decoder_resume_hysteresis_samples + playout_ringbuffer_headroom` (48510 samples)
  - Actual hysteresis gap = 44100 samples (1.0s)
- **Interdependencies:** Works with [playout_ringbuffer_headroom](#playout_ringbuffer_headroom)

---

### global_crossfade_time

- **Database Key:** `global_crossfade_time`
- **Type:** REAL
- **Default:** `2.0`
- **Units:** seconds
- **Valid Range:** 0.0-10.0
- **Modification Impact:** IMMEDIATE (applies to next crossfade)
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:1005](IMPL001-database_schema.md#settings), [SPEC002-crossfade.md](SPEC002-crossfade.md)
- **Description:** Default crossfade duration when passages don't specify custom crossfade timing. Applied to both fade-out and fade-in curves.

---

### global_fade_curve

- **Database Key:** `global_fade_curve`
- **Type:** TEXT
- **Default:** `"exponential_logarithmic"`
- **Units:** N/A (curve type name)
- **Valid Range:** `"exponential_logarithmic"`, `"linear_linear"`, `"cosine_cosine"`
- **Modification Impact:** IMMEDIATE (applies to next crossfade)
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:1006](IMPL001-database_schema.md#settings), [SPEC002-crossfade.md](SPEC002-crossfade.md)
- **Description:** Fade curve pair for crossfades. Format: `"fade_out_curve_fade_in_curve"`. Exponential-logarithmic provides constant power crossfade (perceptually smooth).

---

### http_base_ports

- **Database Key:** `http_base_ports`
- **Type:** TEXT (JSON array)
- **Default:** `[5720, 15720, 25720, 17200, 23400]`
- **Units:** N/A (port numbers)
- **Valid Range:** JSON array of integers 1024-65535
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** All modules (All versions)
- **Defined In:** [IMPL001:1041](IMPL001-database_schema.md#settings)
- **Description:** HTTP server base port candidates. Modules try each port in order until one binds successfully. Supports multiple WKMP instances on same machine.

---

### http_keepalive_timeout_ms

- **Database Key:** `http_keepalive_timeout_ms`
- **Type:** INTEGER
- **Default:** `60000`
- **Units:** milliseconds
- **Valid Range:** 1000-300000 (1 second to 5 minutes)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** All modules (All versions)
- **Defined In:** [IMPL001:1043](IMPL001-database_schema.md#settings)
- **Description:** HTTP keepalive connection timeout. Longer values reduce connection overhead, shorter values free resources faster.

---

### http_max_body_size_bytes

- **Database Key:** `http_max_body_size_bytes`
- **Type:** INTEGER
- **Default:** `1048576` (1 MB)
- **Units:** bytes
- **Valid Range:** 1024-104857600 (1 KB to 100 MB)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** All modules (All versions)
- **Defined In:** [IMPL001:1044](IMPL001-database_schema.md#settings)
- **Description:** Maximum HTTP request body size. Prevents memory exhaustion from large malicious requests.

---

### http_request_timeout_ms

- **Database Key:** `http_request_timeout_ms`
- **Type:** INTEGER
- **Default:** `30000`
- **Units:** milliseconds
- **Valid Range:** 1000-300000 (1 second to 5 minutes)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** All modules (All versions)
- **Defined In:** [IMPL001:1042](IMPL001-database_schema.md#settings)
- **Description:** HTTP request timeout for inter-module communication. Long-running operations (import, analysis) use separate timeout mechanisms.

---

### ingest_max_concurrent_jobs

- **Database Key:** `ingest_max_concurrent_jobs`
- **Type:** INTEGER
- **Default:** `12`
- **Units:** jobs (concurrent import workers)
- **Valid Range:** 1-64 (recommended: CPU cores × 1.5)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1027](IMPL001-database_schema.md#settings), [ARCH-ASYNC-020]
- **Description:** Maximum concurrent audio file import worker threads. Balanced for modern multi-core CPUs.
- **Interdependencies:** `ai_database_connection_pool_size` should be ≥ 8 × this value for optimal performance
- **Performance Notes:** Each worker consumes ~50-100 MB memory during active decoding

---

### initial_play_state

- **Database Key:** `initial_play_state`
- **Type:** TEXT
- **Default:** `"playing"`
- **Units:** N/A (state name)
- **Valid Range:** `"playing"`, `"paused"`
- **Modification Impact:** RESTART_REQUIRED (affects next startup)
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:984](IMPL001-database_schema.md#settings), [REQ-PB-050]
- **Description:** Playback state on application launch. "playing" = auto-resume playback, "paused" = wait for user play command.

---

### last_backup_timestamp_ms

- **Database Key:** `last_backup_timestamp_ms`
- **Type:** INTEGER (nullable)
- **Default:** `NULL`
- **Units:** milliseconds (Unix epoch time)
- **Valid Range:** 0-9223372036854775807 (64-bit signed integer max)
- **Modification Impact:** STATE_PERSISTENCE (automatic)
- **Used By:** wkmp-ui (All versions)
- **Defined In:** [IMPL001:1003](IMPL001-database_schema.md#settings)
- **Description:** Unix timestamp (milliseconds) of last successful database backup. Automatically updated by backup service, not user-configurable.

---

### last_played_passage_id

- **Database Key:** `last_played_passage_id`
- **Type:** TEXT (UUID, nullable)
- **Default:** `NULL`
- **Units:** N/A (UUID)
- **Valid Range:** Valid UUID or NULL
- **Modification Impact:** STATE_PERSISTENCE (automatic)
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:986](IMPL001-database_schema.md#settings)
- **Description:** UUID of last played passage. Used for playback history tracking. Automatically updated by playback engine.

---

### last_played_position_ticks

- **Database Key:** `last_played_position_ticks`
- **Type:** INTEGER
- **Default:** `0`
- **Units:** ticks (WKMP internal time unit: 28,224,000 ticks/second)
- **Valid Range:** 0-9223372036854775807
- **Modification Impact:** STATE_PERSISTENCE (automatic)
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:987](IMPL001-database_schema.md#settings)
- **Description:** Playback position in ticks at last clean shutdown. Reset to 0 on queue change. Used for "resume where you left off" functionality.
- **Conversion:** `seconds = ticks / 28224000`

---

### lead_in_threshold_dB

- **Database Key:** `lead_in_threshold_dB`
- **Type:** REAL
- **Default:** `45.0`
- **Units:** dB (absolute RMS amplitude level, per [SPEC025:AMP-THR-010](SPEC025-amplitude_analysis.md#threshold-definitions))
- **Valid Range:** 0.0-100.0
- **Modification Impact:** REIMPORT_REQUIRED (affects Phase 8 amplitude analysis)
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1033](IMPL001-database_schema.md#settings), [SPEC025:AMP-THR-010,AMP-PARAM-010](SPEC025-amplitude_analysis.md), [SPEC032:301](SPEC032-audio_ingest_architecture.md)
- **Description:** Absolute dB threshold defining minimum RMS amplitude level considered "audible content beginning". Audio below this threshold is lead-in ambience. Per [SPEC025:AMP-THR-010](SPEC025-amplitude_analysis.md).
- **Presets:** See [Import Genre Presets](#import-genre-presets) below
  - Classical: 50.0 dB (more sensitive to quiet lead-ins)
  - Rock/Pop: 40.0 dB (less sensitive, expects sudden attacks)
  - Electronic/Ambient: 55.0 dB (very sensitive to long fade-ins)
  - Default: 45.0 dB (balanced for mixed library)

---

### lead_out_threshold_dB

- **Database Key:** `lead_out_threshold_dB`
- **Type:** REAL
- **Default:** `40.0`
- **Units:** dB (absolute RMS amplitude level, per [SPEC025:AMP-THR-020](SPEC025-amplitude_analysis.md#threshold-definitions))
- **Valid Range:** 0.0-100.0
- **Modification Impact:** REIMPORT_REQUIRED (affects Phase 8 amplitude analysis)
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1034](IMPL001-database_schema.md#settings), [SPEC025:AMP-THR-020,AMP-PARAM-010](SPEC025-amplitude_analysis.md), [SPEC032:302](SPEC032-audio_ingest_architecture.md)
- **Description:** Absolute dB threshold defining minimum RMS amplitude level considered "audible content ending". Audio below this threshold is lead-out ambience. Per [SPEC025:AMP-THR-020](SPEC025-amplitude_analysis.md).
- **Presets:** See [Import Genre Presets](#import-genre-presets) below
  - Classical: 45.0 dB (more sensitive to quiet lead-outs)
  - Rock/Pop: 35.0 dB (less sensitive, expects abrupt endings)
  - Electronic/Ambient: 50.0 dB (very sensitive to long fade-outs)
  - Default: 40.0 dB (balanced for mixed library)

---

### maximum_decode_streams

- **Database Key:** `maximum_decode_streams`
- **Type:** INTEGER
- **Default:** `12`
- **Units:** decoders (concurrent decode chains)
- **Valid Range:** 1-64
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-050](SPEC016-decoder_buffer_design.md#maximum_decode_streams), [GUIDE003:849](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Maximum number of concurrent audio decoders. Limits CPU and memory usage for queue pre-buffering.
- **Memory Impact:** Each decoder chain allocates `playout_ringbuffer_size` buffer (~5.3 MB). Total: 12 × 5.3 MB ≈ 64 MB for playout buffers.
- **Behavior:** Queue passages beyond this count wait until earlier passages finish playing before decoding begins.

---

### minimum_passage_audio_duration_ticks

- **Database Key:** `minimum_passage_audio_duration_ticks`
- **Type:** INTEGER
- **Default:** `2822400` (100ms)
- **Units:** ticks (WKMP internal time: 28,224,000 ticks/second)
- **Valid Range:** 0-28224000 (0-1 second)
- **Modification Impact:** REIMPORT_REQUIRED (affects Phase 4 segmentation)
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1032](IMPL001-database_schema.md#settings)
- **Description:** Minimum non-silence duration for valid audio. Files with <100ms non-silence flagged as "NO AUDIO" (likely data tracks, silence).
- **Conversion:** `100ms × 28224000 ticks/sec = 2822400 ticks`

---

### mixer_batch_size_low

- **Database Key:** `mixer_batch_size_low`
- **Type:** INTEGER
- **Default:** `512`
- **Units:** frames (stereo samples)
- **Valid Range:** 16-1024
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-112](SPEC016-decoder_buffer_design.md#mixer_batch_size_low), [GUIDE003:860](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Frames mixer fills per wake-up when output ring buffer <50% capacity. Aggressive recovery mode.
- **Performance:**
  - With `audio_buffer_size=2208` @ 44.1kHz and `mixer_check_interval_ms=10ms`
  - Audio callback drains ~441 frames per 10ms interval
  - 512 frames provides 16% margin for catch-up recovery
- **Interdependencies:** Must exceed drain rate to prevent underruns: `mixer_batch_size_low > (audio_buffer_size / (1000 / mixer_check_interval_ms))`

---

### mixer_batch_size_optimal

- **Database Key:** `mixer_batch_size_optimal`
- **Type:** INTEGER
- **Default:** `256`
- **Units:** frames (stereo samples)
- **Valid Range:** 16-512
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-113](SPEC016-decoder_buffer_design.md#mixer_batch_size_optimal), [GUIDE003:861](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Frames mixer fills per wake-up when output ring buffer 50-75% capacity. Steady-state operation.
- **Performance:**
  - 256 frames provides ~58% of drain rate (256 ÷ 441)
  - Intentionally lower to allow gradual buffer depletion from 75% toward 50%
  - Maintains buffer in optimal range without oscillation
- **Interdependencies:** Should be approximately half of `mixer_batch_size_low` for smooth threshold transitions

---

### mixer_check_interval_ms

- **Database Key:** `mixer_check_interval_ms`
- **Type:** INTEGER
- **Default:** `10`
- **Units:** milliseconds
- **Valid Range:** 1-100
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-111](SPEC016-decoder_buffer_design.md#mixer_check_interval_ms), [GUIDE003:859](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Mixer thread wake interval. Controls frequency of output ring buffer refills.
- **Tuning:**
  - **Smaller (1-5ms):** More responsive, higher CPU usage (frequent async/tokio overhead)
  - **Default (10ms):** Balanced CPU usage, VeryHigh stability
  - **Larger (20-100ms):** Lower CPU, requires larger batch sizes
- **Empirical Data (Intel i7-8650U @ 1.90GHz):**
  - 5ms interval: Stable with 704+ frame audio buffer (2x safety)
  - 10ms interval: Stable with 2208+ frame audio buffer (3x safety, SELECTED)
  - 20ms+ intervals: Unstable on test system
- **Interdependencies:** Works with `mixer_batch_size_low` and `mixer_batch_size_optimal` to maintain buffer fill

---

### mixer_min_start_level

- **Database Key:** `mixer_min_start_level`
- **Type:** INTEGER
- **Default:** `22050`
- **Units:** samples (stereo)
- **Valid Range:** 4410-220500 (0.1-5.0 seconds @ 44.1kHz)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-088](SPEC016-decoder_buffer_design.md#mixer_min_start_level), [GUIDE003:855](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Minimum samples required in chain buffer before mixer starts playback. Protects against buffer underruns during startup.
- **Time Equivalent:** 0.5 seconds @ 44.1kHz
- **Behavior:**
  - Mixer waits until buffer contains ≥22050 samples before starting playback
  - Once started, may draw all available samples (underruns logged as errors)
  - At passage end, buffer drains completely (expected behavior)

---

### music_directories

- **Database Key:** `music_directories`
- **Type:** TEXT (JSON array)
- **Default:** `[]`
- **Units:** N/A (filesystem paths)
- **Valid Range:** JSON array of valid readable directory paths
- **Modification Impact:** IMMEDIATE (affects next import scan)
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1038](IMPL001-database_schema.md#settings)
- **Description:** Directories to scan for audio files during import. Scanned recursively for supported formats (FLAC, MP3, M4A, OGG, OPUS, WAV).
- **Example:** `["/home/user/Music", "/mnt/nas/albums"]`

---

### output_ringbuffer_size

- **Database Key:** `output_ringbuffer_size`
- **Type:** INTEGER
- **Default:** `8192`
- **Units:** frames (stereo samples)
- **Valid Range:** 2048-262144 (46ms to 5.9s @ 44.1kHz)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-030](SPEC016-decoder_buffer_design.md#output_ringbuffer_size), [GUIDE003:847](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Lock-free SPSC ring buffer capacity between mixer thread and audio callback.
- **Time Equivalent:** 186ms @ 44.1kHz
- **Architecture:** Single-producer (mixer) single-consumer (audio callback) lock-free ring buffer for real-time audio delivery.
- **Tuning:**
  - **Smaller (2048-4096):** Lower latency, higher underrun risk
  - **Default (8192):** 186ms buffer for VeryHigh stability confidence
  - **Larger (16384+):** More stable, higher latency
- **History:** Originally 88200 samples (2.0s) in SPEC016, reduced to 8192 frames (186ms) for optimal balance.

---

### pause_decay_factor

- **Database Key:** `pause_decay_factor`
- **Type:** REAL
- **Default:** `0.96875` (31/32)
- **Units:** ratio (multiplication factor)
- **Valid Range:** 0.0-1.0
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-090](SPEC016-decoder_buffer_design.md#pause_decay_factor), [GUIDE003:856](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Exponential decay factor applied to samples during pause mode. Creates smooth fade to silence, reducing audible "pop" from sudden stop.
- **Behavior:** Each sample output during pause = `previous_sample × 0.96875`
- **Time to Silence:** ~200 samples @ 44.1kHz to reach `pause_decay_floor`

---

### pause_decay_floor

- **Database Key:** `pause_decay_floor`
- **Type:** REAL
- **Default:** `0.0001778`
- **Units:** level (amplitude)
- **Valid Range:** 0.0-1.0
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-100](SPEC016-decoder_buffer_design.md#pause_decay_floor), [GUIDE003:857](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Threshold below which pause mode stops exponential decay and outputs pure silence (0.0). Performance optimization.
- **Behavior:** When `|sample| < 0.0001778`, output 0.0 instead of multiplying

---

### playback_failure_threshold

- **Database Key:** `playback_failure_threshold`
- **Type:** INTEGER
- **Default:** `3`
- **Units:** failures
- **Valid Range:** 1-100
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-pd (Full, Lite versions)
- **Defined In:** [IMPL001:1046](IMPL001-database_schema.md#settings)
- **Description:** Number of consecutive playback failures before Program Director stops automatic passage selection.
- **Interdependencies:** Works with `playback_failure_window_seconds` for time-windowed failure counting

---

### playback_failure_window_seconds

- **Database Key:** `playback_failure_window_seconds`
- **Type:** INTEGER
- **Default:** `60`
- **Units:** seconds
- **Valid Range:** 10-3600
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-pd (Full, Lite versions)
- **Defined In:** [IMPL001:1047](IMPL001-database_schema.md#settings)
- **Description:** Time window for counting playback failures. Failures older than this window are ignored.

---

### playback_progress_interval_ms

- **Database Key:** `playback_progress_interval_ms`
- **Type:** INTEGER
- **Default:** `5000`
- **Units:** milliseconds (playback time, not wall-clock time)
- **Valid Range:** 1000-10000
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:995](IMPL001-database_schema.md#settings), [DB-SET-220](IMPL001-database_schema.md#db-set-220)
- **Description:** Interval for emitting `PlaybackProgress` SSE events to UI clients. Controls UI progress bar update frequency.
- **Behavior:** Based on playback time (audio samples consumed), not wall-clock time. Paused playback stops event emission.
- **Distinction:** External SSE events (UI updates), distinct from internal `position_event_interval_ms`

---

### playout_ringbuffer_headroom

- **Database Key:** `playout_ringbuffer_headroom`
- **Type:** INTEGER
- **Default:** `32768`
- **Units:** samples (stereo)
- **Valid Range:** 882-88200 (0.02-2.0 seconds @ 44.1kHz)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-080](SPEC016-decoder_buffer_design.md#playout_ringbuffer_headroom), [GUIDE003:853](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Reserved space in playout buffer for in-flight resampler samples after decoder pause.
- **Time Equivalent:** 0.74 seconds @ 44.1kHz
- **Behavior:** Decoder pauses when `free_space ≤ 32768` to prevent overflow from resampler output still in pipeline.
- **Interdependencies:** Pause threshold for `decoder_resume_hysteresis_samples` calculation

---

### playout_ringbuffer_size

- **Database Key:** `playout_ringbuffer_size`
- **Type:** INTEGER
- **Default:** `661941`
- **Units:** samples (stereo)
- **Valid Range:** 44100-8820000 (1-200 seconds @ 44.1kHz)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-070](SPEC016-decoder_buffer_design.md#playout_ringbuffer_size), [GUIDE003:852](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Decoded/resampled audio buffer capacity per passage.
- **Time Equivalent:** 15.01 seconds @ 44.1kHz
- **Memory:** 8 bytes per sample (f32 stereo) × 661941 = ~5.3 MB per buffer
- **Total Memory:** With `maximum_decode_streams=12`: 12 × 5.3 MB = ~64 MB for playout buffers

---

### position_event_interval_ms

- **Database Key:** `position_event_interval_ms`
- **Type:** INTEGER
- **Default:** `1000`
- **Units:** milliseconds (audio time)
- **Valid Range:** 100-5000
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:994](IMPL001-database_schema.md#settings), [DB-SET-210](IMPL001-database_schema.md#db-set-210)
- **Description:** Interval for mixer to emit internal `PositionUpdate` events. Controls song boundary detection accuracy.
- **Behavior:**
  - Mixer checks frame counter every `get_next_frame()` call
  - Event emitted when frame count reaches `(position_event_interval_ms / 1000.0) × sample_rate`
  - At 44.1kHz with 1000ms: Event every 44,100 frames
- **Affects:**
  - Song boundary detection latency (lower = faster detection)
  - CPU usage (lower = more frequent event processing)
- **Trade-offs:**
  - 100ms: Very responsive, ~10× CPU overhead, <100ms boundary detection
  - 1000ms (default): Balanced, minimal CPU, ~1s boundary detection
  - 5000ms: Low CPU, delayed detection up to 5s
- **Distinction:** Internal events (song boundaries), distinct from external `playback_progress_interval_ms` (UI updates)

---

### queue_current_id

- **Database Key:** `queue_current_id`
- **Type:** TEXT (UUID, nullable)
- **Default:** `NULL`
- **Units:** N/A (UUID)
- **Valid Range:** Valid UUID or NULL
- **Modification Impact:** STATE_PERSISTENCE (automatic)
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-125](SPEC016-decoder_buffer_design.md#queue_current_id)
- **Description:** UUID of currently playing queue entry. Enables "resume where you left off" functionality across restarts.
- **Behavior:**
  - Automatically set on passage playback start
  - Updated on each passage transition
  - Deleted when stopped/idle
  - Loaded on startup for potential queue restoration
- **Note:** Runtime state data, not user-configurable (documented here for completeness)

---

### queue_entry_timing_overrides

- **Database Key:** `queue_entry_timing_overrides`
- **Type:** TEXT (JSON object)
- **Default:** `{}`
- **Units:** N/A (JSON map)
- **Valid Range:** JSON object mapping queue entry GUID → timing override object
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:1013](IMPL001-database_schema.md#settings)
- **Description:** Per-queue-entry crossfade timing overrides. Allows custom crossfade durations for specific queue entries.
- **Format:**
```json
{
  "queue_entry_guid": {
    "crossfade_duration_s": 3.5,
    "fade_out_curve": "exponential",
    "fade_in_curve": "logarithmic"
  }
}
```

---

### queue_max_enqueue_batch

- **Database Key:** `queue_max_enqueue_batch`
- **Type:** INTEGER
- **Default:** `5`
- **Units:** passages
- **Valid Range:** 1-50
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-pd (Full, Lite versions)
- **Defined In:** [IMPL001:1019](IMPL001-database_schema.md#settings)
- **Description:** Maximum passages Program Director enqueues at once in response to queue refill request.

---

### queue_max_size

- **Database Key:** `queue_max_size`
- **Type:** INTEGER
- **Default:** `100`
- **Units:** passages
- **Valid Range:** 1-10000
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:1018](IMPL001-database_schema.md#settings)
- **Description:** Maximum queue size in passages. Prevents unbounded memory growth from excessive enqueueing.

---

### queue_refill_acknowledgment_timeout_seconds

- **Database Key:** `queue_refill_acknowledgment_timeout_seconds`
- **Type:** INTEGER
- **Default:** `5`
- **Units:** seconds
- **Valid Range:** 1-60
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ap (Full, Lite versions)
- **Defined In:** [IMPL001:1017](IMPL001-database_schema.md#settings)
- **Description:** Timeout waiting for Program Director to acknowledge queue refill request. If no response, Audio Player attempts to relaunch wkmp-pd.

---

### queue_refill_request_throttle_seconds

- **Database Key:** `queue_refill_request_throttle_seconds`
- **Type:** INTEGER
- **Default:** `10`
- **Units:** seconds
- **Valid Range:** 1-300
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ap (Full, Lite versions)
- **Defined In:** [IMPL001:1016](IMPL001-database_schema.md#settings)
- **Description:** Minimum interval between queue refill requests to Program Director. Prevents request spam when queue is underfilled.

---

### queue_refill_threshold_passages

- **Database Key:** `queue_refill_threshold_passages`
- **Type:** INTEGER
- **Default:** `2`
- **Units:** passages
- **Valid Range:** 0-100
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ap (Full, Lite versions)
- **Defined In:** [IMPL001:1014](IMPL001-database_schema.md#settings)
- **Description:** Minimum passages remaining before triggering automatic queue refill request to Program Director.
- **Interdependencies:** Works with `queue_refill_threshold_seconds` (both conditions must be true to trigger refill)

---

### queue_refill_threshold_seconds

- **Database Key:** `queue_refill_threshold_seconds`
- **Type:** INTEGER
- **Default:** `900` (15 minutes)
- **Units:** seconds
- **Valid Range:** 60-7200 (1 minute to 2 hours)
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ap (Full, Lite versions)
- **Defined In:** [IMPL001:1015](IMPL001-database_schema.md#settings)
- **Description:** Minimum time remaining in queue before triggering automatic refill request.
- **Interdependencies:** Works with `queue_refill_threshold_passages` (both conditions must be true)

---

### relaunch_attempts

- **Database Key:** `relaunch_attempts`
- **Type:** INTEGER
- **Default:** `20`
- **Units:** attempts
- **Valid Range:** 1-100
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ui (All versions)
- **Defined In:** [IMPL001:1022](IMPL001-database_schema.md#settings)
- **Description:** Maximum module relaunch attempts before giving up. Prevents infinite relaunch loops when module crashes on startup.
- **Interdependencies:** Works with `relaunch_delay` for throttled retry behavior

---

### relaunch_delay

- **Database Key:** `relaunch_delay`
- **Type:** INTEGER
- **Default:** `5`
- **Units:** seconds
- **Valid Range:** 1-60
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ui (All versions)
- **Defined In:** [IMPL001:1021](IMPL001-database_schema.md#settings)
- **Description:** Wait time between module relaunch attempts. Prevents rapid relaunch spam when module fails immediately.

---

### resume_from_pause_fade_in_curve

- **Database Key:** `resume_from_pause_fade_in_curve`
- **Type:** TEXT
- **Default:** `"exponential"`
- **Units:** N/A (curve type name)
- **Valid Range:** `"linear"`, `"exponential"`, `"cosine"`
- **Modification Impact:** IMMEDIATE (applies to next resume)
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:1009](IMPL001-database_schema.md#settings)
- **Description:** Fade-in curve type when resuming from pause. Exponential provides smooth perceptual fade-in.

---

### resume_from_pause_fade_in_duration

- **Database Key:** `resume_from_pause_fade_in_duration`
- **Type:** REAL
- **Default:** `0.5`
- **Units:** seconds
- **Valid Range:** 0.0-5.0
- **Modification Impact:** IMMEDIATE (applies to next resume)
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:1008](IMPL001-database_schema.md#settings)
- **Description:** Fade-in duration when resuming playback from pause. Prevents audible "pop" from sudden level change.

---

### session_timeout_seconds

- **Database Key:** `session_timeout_seconds`
- **Type:** INTEGER
- **Default:** `31536000` (1 year)
- **Units:** seconds
- **Valid Range:** 60-31536000 (1 minute to 1 year)
- **Modification Impact:** IMMEDIATE (applies to new sessions)
- **Used By:** wkmp-ui (All versions)
- **Defined In:** [IMPL001:1024](IMPL001-database_schema.md#settings)
- **Description:** Session timeout for user authentication. Default 1 year provides "remember me" behavior for local installations.

---

### silence_min_duration_ticks

- **Database Key:** `silence_min_duration_ticks`
- **Type:** INTEGER
- **Default:** `8467200` (300ms)
- **Units:** ticks (WKMP internal time: 28,224,000 ticks/second)
- **Valid Range:** 0-28224000 (0-1 second)
- **Modification Impact:** REIMPORT_REQUIRED (affects Phase 4 segmentation)
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1031](IMPL001-database_schema.md#settings)
- **Description:** Minimum silence duration to detect passage boundary during audio file segmentation.
- **Conversion:** `300ms × 28224000 ticks/sec = 8467200 ticks`

---

### silence_threshold_dB

- **Database Key:** `silence_threshold_dB`
- **Type:** REAL
- **Default:** `35.0`
- **Units:** dB (below maximum amplitude)
- **Valid Range:** 20.0-80.0
- **Modification Impact:** REIMPORT_REQUIRED (affects Phase 4 segmentation)
- **Used By:** wkmp-ai (Full version only)
- **Defined In:** [IMPL001:1030](IMPL001-database_schema.md#settings)
- **Description:** Amplitude threshold for silence detection during passage segmentation. Higher values = stricter silence detection (more sensitive).

---

### temporary_flavor_override

- **Database Key:** `temporary_flavor_override`
- **Type:** TEXT (JSON object, nullable)
- **Default:** `NULL`
- **Units:** N/A (JSON)
- **Valid Range:** JSON object with `target_flavor` (vector) and `expiration` (timestamp), or NULL
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-pd (Full, Lite versions)
- **Defined In:** [IMPL001:1039](IMPL001-database_schema.md#settings)
- **Description:** Temporary musical flavor override with expiration timestamp. Allows time-limited playlist customization (e.g., "party mode for 2 hours").
- **Format:**
```json
{
  "target_flavor": [0.5, 0.8, ...],
  "expiration": "2025-11-15T22:00:00Z"
}
```

---

### validation_enabled

- **Database Key:** `validation_enabled`
- **Type:** TEXT (boolean as "true"/"false")
- **Default:** `"true"`
- **Units:** N/A (boolean)
- **Valid Range:** `"true"`, `"false"`
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-130](SPEC016-decoder_buffer_design.md#validation_enabled)
- **Description:** Master switch for automatic validation service. Enables periodic pipeline integrity checks (sample conservation verification).
- **Use Cases:**
  - Production: Enable for monitoring
  - Testing: Enable to verify sample conservation
  - Performance-critical: Disable to eliminate validation overhead

---

### validation_interval_secs

- **Database Key:** `validation_interval_secs`
- **Type:** INTEGER
- **Default:** `10`
- **Units:** seconds
- **Valid Range:** 1-3600 (recommended: 5-60)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-131](SPEC016-decoder_buffer_design.md#validation_interval_secs)
- **Description:** Time interval between validation service checks during playback.
- **Tuning:**
  - Shorter: More frequent checks, higher overhead, faster issue detection
  - Longer: Lower overhead, delayed issue detection
  - Default (10s): Good balance for production monitoring
- **Interdependencies:** Works with `validation_tolerance_samples` for pass/fail determination

---

### validation_tolerance_samples

- **Database Key:** `validation_tolerance_samples`
- **Type:** INTEGER
- **Default:** `8192`
- **Units:** samples (stereo @ working_sample_rate)
- **Valid Range:** 0-88200 (recommended: 4096-16384)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-132](SPEC016-decoder_buffer_design.md#validation_tolerance_samples)
- **Description:** Allowable sample count discrepancy before flagging validation failure.
- **Time Equivalent:** ~186ms @ 44.1kHz
- **Behavior:** Compares decoder output = buffer written = buffer read = mixer output. Fails if discrepancy exceeds tolerance.
- **Rationale:**
  - Zero tolerance too strict (async timing, buffer chunk boundaries)
  - Default 8192 samples matches `output_ringbuffer_size` for consistency
- **Tuning:**
  - Stricter (lower): More sensitive, higher false-positive risk
  - Looser (higher): Less sensitive, may miss subtle problems
  - Zero: Only for exact sample-perfect validation testing

---

### volume_fade_update_period

- **Database Key:** `volume_fade_update_period`
- **Type:** INTEGER
- **Default:** `10`
- **Units:** milliseconds
- **Valid Range:** 1-100
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:1011](IMPL001-database_schema.md#settings)
- **Description:** Update period for volume fade operations (e.g., resume fade-in). Controls smoothness vs. CPU usage.
- **Tuning:**
  - Lower (1-5ms): Smoother fades, higher CPU
  - Default (10ms): Good balance (imperceptible steps @ 44.1kHz)
  - Higher (50-100ms): Audible steps, lower CPU

---

### volume_level

- **Database Key:** `volume_level`
- **Type:** REAL
- **Default:** `0.5`
- **Units:** linear amplitude (0.0-1.0)
- **Valid Range:** 0.0-1.0
- **Modification Impact:** IMMEDIATE
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [IMPL001:989](IMPL001-database_schema.md#settings)
- **Description:** Master volume level. HTTP API uses 0.0-1.0. UI displays 0-100 with conversion: `display = round(volume × 100.0)`.
- **Note:** Linear amplitude scale, not logarithmic dB. 0.5 ≈ -6 dB.

---

### working_sample_rate

- **Database Key:** `working_sample_rate`
- **Type:** INTEGER
- **Default:** `44100`
- **Units:** Hz (samples per second)
- **Valid Range:** 8000-192000 (recommended: 44100 or 48000)
- **Modification Impact:** RESTART_REQUIRED
- **Used By:** wkmp-ap (All versions)
- **Defined In:** [SPEC016:DBD-PARAM-020](SPEC016-decoder_buffer_design.md#working_sample_rate), [GUIDE003:846](GUIDE003_audio_pipeline_diagrams.md)
- **Description:** Target sample rate for all decoded audio. All audio is resampled to this rate before buffering/mixing.
- **Negotiation:** At startup, AudioOutput attempts to use this preferred rate. If device doesn't support it, device's native rate is used instead. Effective rate = device native rate.
- **Bypass:** When decoder outputs audio at effective `working_sample_rate`, resampling is bypassed (passthrough mode).
- **Common Values:**
  - 44100 Hz: CD quality, default
  - 48000 Hz: Professional audio, typical Windows device native rate
  - 96000 Hz: High-res audio (not recommended for compatibility)

---

## Parameter Interdependencies

### Audio Pipeline Buffer Chain

```
working_sample_rate [DBD-PARAM-020]
  ↓ (affects time conversions for all parameters below)

decode_chunk_size [DBD-PARAM-065]
  → Resampler output chunk size (~0.73s @ 44.1kHz)

playout_ringbuffer_size [DBD-PARAM-070]
  → Decoded audio buffer (15.01s @ 44.1kHz)
  → Memory: 661,941 samples × 8 bytes = ~5.3 MB per buffer

  playout_ringbuffer_headroom [DBD-PARAM-080]
    → Decoder pause threshold (0.74s @ 44.1kHz)

  decoder_resume_hysteresis_samples [DBD-PARAM-085]
    → Resume threshold gap (1.0s @ 44.1kHz)
    → Resume when free_space ≥ (DBD-080 + DBD-085)

  mixer_min_start_level [DBD-PARAM-088]
    → Buffer ready threshold (0.5s @ 44.1kHz)

output_ringbuffer_size [DBD-PARAM-030]
  → Lock-free buffer mixer→callback (186ms @ 44.1kHz)

audio_buffer_size [DBD-PARAM-110]
  → Audio device callback size (50.1ms @ 44.1kHz)
  → Callback frequency = working_sample_rate / audio_buffer_size
```

### Mixer Timing Chain

```
mixer_check_interval_ms [DBD-PARAM-111]
  → How often mixer wakes to refill output buffer (10ms default)

  mixer_batch_size_low [DBD-PARAM-112]
    → Frames filled when buffer <50% (512 frames)
    → Must exceed drain rate: 512 > (2208 / (1000/10)) ≈ 441

  mixer_batch_size_optimal [DBD-PARAM-113]
    → Frames filled when buffer 50-75% (256 frames)
    → Intentionally ~58% of drain rate for gradual depletion
```

**Critical Constraint:** `mixer_batch_size_low` must exceed audio callback drain rate to prevent underruns:

```
drain_rate = audio_buffer_size / (1000 / mixer_check_interval_ms)
mixer_batch_size_low > drain_rate

Example: 2208 frames / (1000ms / 10ms) = 441 frames/interval
         512 frames > 441 frames ✓ (16% margin)
```

### Queue Refill Logic

```
queue_refill_threshold_passages [1014] AND
queue_refill_threshold_seconds [1015]
  → Both must be true to trigger refill request

queue_refill_request_throttle_seconds [1016]
  → Minimum interval between requests (10s)

queue_refill_acknowledgment_timeout_seconds [1017]
  → Timeout waiting for PD response (5s)
  → After timeout, attempt to relaunch wkmp-pd

  relaunch_delay [1021]
    → Wait 5s between relaunch attempts

  relaunch_attempts [1022]
    → Max 20 attempts before giving up
```

### Import Pipeline Parameters

```
silence_threshold_dB [1030]
  → Amplitude threshold for silence detection

silence_min_duration_ticks [1031]
  → Minimum silence duration for passage boundary (300ms)

minimum_passage_audio_duration_ticks [1032]
  → Minimum non-silence for valid audio (100ms)

lead_in_threshold_dB [1033]
  → 1/4 intensity point for fade-in detection

lead_out_threshold_dB [1034]
  → 1/4 intensity point for fade-out detection
```

All reimport-required parameters: Changing any value requires re-running import workflow for changes to take effect.

### Database Connection Pool Sizing

```
ingest_max_concurrent_jobs [1027]
  → Maximum concurrent import workers (12)

ai_database_connection_pool_size [1026]
  → Should be ≥ 8 × ingest_max_concurrent_jobs
  → Default: 96 = 12 workers × 8 connections per worker

ai_database_lock_retry_ms [1029]
  → SQLite busy_timeout (250ms)

ai_database_max_lock_wait_ms [1028]
  → Total retry budget (5000ms = 20 retries @ 250ms)
```

---

## Unit Conversion Reference

### Time Units

| From | To | Formula |
|------|-----|---------|
| **Ticks** | Seconds | `seconds = ticks / 28_224_000` |
| **Ticks** | Milliseconds | `ms = ticks / 28_224` |
| **Seconds** | Ticks | `ticks = seconds × 28_224_000` |
| **Milliseconds** | Ticks | `ticks = ms × 28_224` |
| **Samples** | Seconds @ 44.1kHz | `seconds = samples / 44_100` |
| **Samples** | Milliseconds @ 44.1kHz | `ms = samples / 44.1` |
| **Frames** | Samples (stereo) | `samples = frames × 2` |
| **Frames** | Seconds @ 44.1kHz | `seconds = frames / 44_100` |

**WKMP Internal Time:** 28,224,000 ticks/second (chosen for integer division by common sample rates)

**Examples:**
- 100ms = 100 × 28,224 = 2,822,400 ticks
- 300ms = 300 × 28,224 = 8,467,200 ticks
- 1 second = 28,224,000 ticks
- 44,100 samples @ 44.1kHz = 1 second = 28,224,000 ticks

### Sample Rate Conversions

| Rate (Hz) | 1 Second (samples) | 100ms (samples) | Common Usage |
|-----------|-------------------|-----------------|--------------|
| 44,100 | 44,100 | 4,410 | CD quality (default) |
| 48,000 | 48,000 | 4,800 | Professional audio |
| 88,200 | 88,200 | 8,820 | High-res (2× CD) |
| 96,000 | 96,000 | 9,600 | High-res (2× pro) |

---

## Parameter Presets

### Audio Player Performance Profiles

**Conservative (Default - VeryHigh Stability):**
```json
{
  "audio_buffer_size": 2208,
  "mixer_check_interval_ms": 10,
  "mixer_batch_size_low": 512,
  "mixer_batch_size_optimal": 256,
  "output_ringbuffer_size": 8192,
  "latency_ms": 50.1
}
```

**Balanced (High Stability):**
```json
{
  "audio_buffer_size": 1472,
  "mixer_check_interval_ms": 10,
  "mixer_batch_size_low": 384,
  "mixer_batch_size_optimal": 192,
  "output_ringbuffer_size": 8192,
  "latency_ms": 33.4
}
```

**Aggressive (Minimum Latency):**
```json
{
  "audio_buffer_size": 704,
  "mixer_check_interval_ms": 5,
  "mixer_batch_size_low": 256,
  "mixer_batch_size_optimal": 128,
  "output_ringbuffer_size": 4096,
  "latency_ms": 16.0
}
```

### Import Genre Presets

**Classical Music:**
```json
{
  "lead_in_threshold_dB": 50.0,
  "lead_out_threshold_dB": 45.0,
  "silence_threshold_dB": 40.0
}
```
**Rationale:** Classical music often has gradual dynamic changes requiring more sensitive detection (higher absolute dB thresholds detect quieter content). Per [SPEC025:327-330](SPEC025-amplitude_analysis.md).

**Rock/Pop:**
```json
{
  "lead_in_threshold_dB": 40.0,
  "lead_out_threshold_dB": 35.0,
  "silence_threshold_dB": 35.0
}
```
**Rationale:** Rock/pop typically has sharp transients and abrupt endings requiring less sensitive detection (lower thresholds expect louder content onset). Per [SPEC025:332-335](SPEC025-amplitude_analysis.md).

**Electronic/Ambient:**
```json
{
  "lead_in_threshold_dB": 55.0,
  "lead_out_threshold_dB": 50.0,
  "silence_threshold_dB": 45.0
}
```
**Rationale:** Electronic music often uses extended fades requiring very sensitive detection (highest thresholds to capture long quiet ramps). Per [SPEC025:337-340](SPEC025-amplitude_analysis.md).

**Default (General Purpose):**
```json
{
  "lead_in_threshold_dB": 45.0,
  "lead_out_threshold_dB": 40.0,
  "silence_threshold_dB": 35.0
}
```
**Rationale:** Balanced for mixed library with various musical styles. Per [SPEC025:342-345](SPEC025-amplitude_analysis.md), [SPEC032](SPEC032-audio_ingest_architecture.md).

**NOTE:** Hardcoded algorithm parameters (not in settings table): `rms_window_ms: 100`, `quick_ramp_threshold: 0.75`, `quick_ramp_duration_s: 1.0`, `max_lead_in_duration_s: 10.0`, `max_lead_out_duration_s: 10.0`, `apply_a_weighting: false`. See [SPEC025:310-315](SPEC025-amplitude_analysis.md) for algorithm parameter details.

---

## Document Maintenance

**Update Triggers:**
- New parameter added to `settings` table → Add entry to Parameter Index
- Parameter default value changed in code → Update Default column
- Valid range modified → Update Valid Range column
- Modification impact changed → Update Modification Impact column

**Cross-References:**
- When modifying SPEC*/IMPL* documents, check if parameter definitions changed
- Update this document to maintain single source of truth
- Add amendment notes in original specification documents pointing to IMPL011

**Verification:**
```sql
-- Query to verify all settings documented
SELECT key FROM settings
WHERE key NOT IN (
  -- List from Parameter Index above
  'acoustid_api_key', 'ai_database_connection_pool_size', ...
);
```

---

**Document Version:** 1.0
**Last Updated:** 2025-11-15
**Change History:**
- v1.0 (2025-11-15): Initial comprehensive settings reference created
