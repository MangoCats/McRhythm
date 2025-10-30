# SPEC024: wkmp-ap Technical Debt Remediation

**🛠️ TIER 2 - DESIGN SPECIFICATION**

**Status:** Draft
**Created:** 2025-10-29
**Related Documents:**
- Source: [wkmp-ap_technical_debt_report.md](wkmp-ap_technical_debt_report.md)
- References: [REQ-NF-020] Error handling, [IMPL002] Coding conventions

---

## Executive Summary

This specification addresses technical debt identified in the wkmp-ap (Audio Player) microservice audit. The audit found 13 explicit TODOs, 376 .unwrap() calls, 21 compiler warnings, and 1 CRITICAL security issue (POST/PUT authentication bypass).

**Priority Classification:**
- **CRITICAL:** 1 issue (security) - Blocks multi-user deployment
- **HIGH:** 5 issues - Impact functionality/debuggability
- **MEDIUM:** 7 issues - Code health and maintainability

**Remediation Timeline:** 3 sprints (security → functionality → quality)

---

## 1. Critical Security Issue

### [DEBT-SEC-001] POST/PUT Authentication Bypass

**Severity:** CRITICAL
**File:** `wkmp-ap/src/api/auth_middleware.rs:832`
**Impact:** Any client can modify playback state without authentication

**Current Behavior:**
```rust
Method::POST | Method::PUT => {
    tracing::warn!("POST/PUT request bypassing authentication - not yet implemented");
    return Ok(Authenticated);  // ⚠️ ALLOWS ALL REQUESTS
}
```

**SHALL Requirements:**

**[REQ-DEBT-SEC-001-010]** The system SHALL validate authentication for ALL POST and PUT requests

**[REQ-DEBT-SEC-001-020]** POST/PUT authentication SHALL use the same shared_secret mechanism as GET requests

**[REQ-DEBT-SEC-001-030]** Authentication credentials for POST/PUT SHALL be extracted from request body JSON field `shared_secret`

**[REQ-DEBT-SEC-001-040]** If authentication fails, the system SHALL return HTTP 401 with JSON error response

**Design Approach:**

Extract `shared_secret` from JSON body before handler processes request:

1. **For POST with JSON body:**
   - Extract JSON body
   - Look for `shared_secret` field (optional)
   - If present: Validate against server secret
   - If absent: Reject with 401 (authentication required)

2. **For PUT with JSON body:**
   - Same as POST

3. **For POST/PUT without JSON body:**
   - Allow through (may be used for operations without params)
   - Handler validation catches issues

**Implementation Location:** `auth_middleware.rs:825-835`

**Acceptance Test:**
```rust
#[tokio::test]
async fn test_post_requires_authentication() {
    // POST without shared_secret → 401
    let response = client.post("/api/queue")
        .json(&json!({"passage_id": "test"}))
        .send().await;
    assert_eq!(response.status(), 401);

    // POST with invalid secret → 401
    let response = client.post("/api/queue")
        .json(&json!({"passage_id": "test", "shared_secret": "wrong"}))
        .send().await;
    assert_eq!(response.status(), 401);

    // POST with valid secret → 200
    let response = client.post("/api/queue")
        .json(&json!({"passage_id": "test", "shared_secret": valid_secret}))
        .send().await;
    assert_eq!(response.status(), 200);
}
```

---

## 2. High-Priority Functionality Issues

### [DEBT-FUNC-001] Missing File Path in Decoder Errors

**Severity:** HIGH
**Files:** `wkmp-ap/src/audio/decode.rs:161, 176`
**Impact:** Debugging decoder errors requires guessing which file failed

**SHALL Requirements:**

**[REQ-DEBT-FUNC-001-010]** Decoder error messages SHALL include the absolute file path of the audio file being decoded

**[REQ-DEBT-FUNC-001-020]** The ChunkedDecoder struct SHALL store the file_path as a field

**[REQ-DEBT-FUNC-001-030]** Error construction SHALL use the stored file_path instead of PathBuf::from("unknown")

**Design Changes:**

```rust
pub struct ChunkedDecoder {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    file_path: PathBuf,  // NEW: Store for error reporting
}

impl ChunkedDecoder {
    pub fn new(
        format_reader: Box<dyn FormatReader>,
        decoder: Box<dyn symphonia::core::codecs::Decoder>,
        track_id: u32,
        file_path: PathBuf,  // NEW: Accept in constructor
    ) -> Self {
        Self {
            format_reader,
            decoder,
            track_id,
            file_path,  // NEW: Store
        }
    }
}
```

**Acceptance Test:**
```rust
#[tokio::test]
async fn test_decoder_error_includes_file_path() {
    let corrupt_file = Path::new("/tmp/corrupt.mp3");
    create_corrupt_audio_file(corrupt_file);

    let result = ChunkedDecoder::new(..., corrupt_file.to_path_buf());

    match result.decode_chunk() {
        Err(AudioPlayerError::Decoder(DecoderError::DecoderPanic { file_path, .. })) => {
            assert_eq!(file_path, corrupt_file);
        }
        _ => panic!("Expected decoder error with file path"),
    }
}
```

---

### [DEBT-FUNC-002] Buffer Configuration Not Reading from Database

**Severity:** HIGH
**File:** `wkmp-ap/src/playback/buffer_manager.rs:122`
**Impact:** Tuning tool settings ignored, hardcoded defaults used

**SHALL Requirements:**

**[REQ-DEBT-FUNC-002-010]** BufferManager SHALL read buffer_capacity_samples from settings database on initialization

**[REQ-DEBT-FUNC-002-020]** BufferManager SHALL read buffer_headroom_samples from settings database on initialization

**[REQ-DEBT-FUNC-002-030]** If settings are NULL/missing, BufferManager SHALL use compiled defaults (661,941 capacity, 4,410 headroom)

**[REQ-DEBT-FUNC-002-040]** Settings SHALL be loaded once at BufferManager creation, not per-buffer allocation

**Design:**

```rust
impl BufferManager {
    pub async fn new(db: SqlitePool) -> Result<Self> {
        // NEW: Query settings from database
        let capacity = get_setting_i64(&db, "buffer_capacity_samples").await?
            .unwrap_or(661_941);  // Default if NULL

        let headroom = get_setting_i64(&db, "buffer_headroom_samples").await?
            .unwrap_or(4_410);  // Default if NULL

        let hysteresis = get_setting_i64(&db, "buffer_resume_hysteresis").await?
            .unwrap_or(2_205);  // Default if NULL

        Ok(Self {
            buffers: Arc::new(RwLock::new(HashMap::new())),
            buffer_capacity: capacity,  // NEW: Store
            buffer_headroom: headroom,  // NEW: Store
            resume_hysteresis: Arc::new(RwLock::new(hysteresis)),
        })
    }

    pub async fn get_or_create_buffer(&self, queue_entry_id: Uuid) -> Arc<PlayoutRingBuffer> {
        // ...existing lookup logic...

        let buffer_arc = Arc::new(PlayoutRingBuffer::new(
            Some(self.buffer_capacity),  // NEW: Use stored config
            Some(self.buffer_headroom),  // NEW: Use stored config
            Some(hysteresis),
            Some(queue_entry_id),
        ));

        // ...rest of method...
    }
}
```

**Acceptance Test:**
```rust
#[tokio::test]
async fn test_buffer_manager_reads_settings() {
    let db = setup_test_db().await;

    // Set custom buffer settings
    set_setting(&db, "buffer_capacity_samples", "500000").await;
    set_setting(&db, "buffer_headroom_samples", "3000").await;

    let manager = BufferManager::new(db).await.unwrap();
    let buffer = manager.get_or_create_buffer(Uuid::new_v4()).await;

    // Verify buffer created with custom settings
    assert_eq!(buffer.capacity(), 500000);
    assert_eq!(buffer.headroom(), 3000);
}
```

---

### [DEBT-FUNC-003] Incomplete Buffer Chain Diagnostics

**Severity:** HIGH
**File:** `wkmp-ap/src/playback/engine.rs:1203-1228`
**Impact:** Developer UI missing telemetry (decoder state, sample rate, fade stage, start time)

**SHALL Requirements:**

**[REQ-DEBT-FUNC-003-010]** BufferChainInfo SHALL include decoder_state populated from decoder worker status

**[REQ-DEBT-FUNC-003-020]** BufferChainInfo SHALL include source_sample_rate populated from decoder metadata

**[REQ-DEBT-FUNC-003-030]** BufferChainInfo SHALL include fade_stage populated from decoder worker fade state

**[REQ-DEBT-FUNC-003-040]** BufferChainInfo SHALL include started_at timestamp populated when chain begins mixing

**Design:**

Add telemetry to `DecoderWorker`:

```rust
pub struct DecoderWorker {
    // ...existing fields...
    telemetry: Arc<RwLock<DecoderTelemetry>>,  // NEW
}

#[derive(Clone)]
pub struct DecoderTelemetry {
    pub decoder_state: String,
    pub source_sample_rate: Option<u32>,
    pub fade_stage: Option<String>,
}

impl DecoderWorker {
    pub async fn get_telemetry(&self) -> DecoderTelemetry {
        self.telemetry.read().await.clone()
    }
}
```

Populate in `PlaybackEngine::get_buffer_chain_state()`:

```rust
// Query decoder telemetry if chain has decoder assigned
let telemetry = if let Some(worker) = self.decoder_pool.get_worker_for_chain(chain_index).await {
    Some(worker.get_telemetry().await)
} else {
    None
};

let info = BufferChainInfo {
    // ...existing fields...
    decoder_state: telemetry.as_ref().map(|t| t.decoder_state.clone()),
    source_sample_rate: telemetry.as_ref().and_then(|t| t.source_sample_rate),
    fade_stage: telemetry.as_ref().and_then(|t| t.fade_stage.clone()),
    started_at: mixer.get_chain_start_time(chain_index),  // NEW: Query mixer
};
```

**Acceptance Test:**
```rust
#[tokio::test]
async fn test_buffer_chain_diagnostics_complete() {
    let engine = setup_engine().await;
    engine.enqueue_passage(test_passage()).await;

    tokio::time::sleep(Duration::from_millis(100)).await;  // Let decoder start

    let state = engine.get_buffer_chain_state().await;
    let chain = &state[0];

    // Verify all diagnostics populated
    assert!(chain.decoder_state.is_some());
    assert!(chain.source_sample_rate.is_some());
    assert!(chain.fade_stage.is_some());
    assert!(chain.started_at.is_some());
}
```

---

### [DEBT-FUNC-004] Missing Song Album UUIDs

**Severity:** HIGH
**Files:** `wkmp-ap/src/playback/engine.rs:1840, 2396, 2687`
**Impact:** Passage metadata incomplete in PassageStarted/PassageComplete events

**SHALL Requirements:**

**[REQ-DEBT-FUNC-004-010]** PassageStarted events SHALL include song_albums field populated with album UUIDs

**[REQ-DEBT-FUNC-004-020]** PassageComplete events SHALL include song_albums field populated with album UUIDs

**[REQ-DEBT-FUNC-004-030]** Album UUIDs SHALL be queried from database joining passages → passage_albums → albums

**Design:**

Add database query function:

```rust
// In wkmp-ap/src/db/passages.rs
pub async fn get_passage_album_uuids(
    pool: &SqlitePool,
    passage_id: Uuid,
) -> Result<Vec<Uuid>> {
    let rows = sqlx::query(
        "SELECT albums.guid
         FROM passage_albums
         JOIN albums ON passage_albums.album_id = albums.guid
         WHERE passage_albums.passage_id = ?"
    )
    .bind(passage_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut album_ids = Vec::new();
    for row in rows {
        let guid_str: String = row.get("guid");
        album_ids.push(Uuid::parse_str(&guid_str)?);
    }

    Ok(album_ids)
}
```

Use in engine event emission:

```rust
// Query albums once when passage enqueued/started
let song_albums = db::passages::get_passage_album_uuids(&self.db, passage_id).await
    .unwrap_or_else(|e| {
        warn!("Failed to query passage albums: {}", e);
        Vec::new()
    });

// Use in event emission
self.event_bus.publish(PlaybackEvent::PassageStarted {
    // ...other fields...
    song_albums,  // Now populated
});
```

**Acceptance Test:**
```rust
#[tokio::test]
async fn test_passage_events_include_albums() {
    let db = setup_test_db().await;
    let passage_id = Uuid::new_v4();
    let album_id_1 = Uuid::new_v4();
    let album_id_2 = Uuid::new_v4();

    // Create passage with 2 albums
    create_test_passage(&db, passage_id).await;
    link_passage_to_album(&db, passage_id, album_id_1).await;
    link_passage_to_album(&db, passage_id, album_id_2).await;

    let engine = PlaybackEngine::new(db, ...).await;
    let mut events = engine.subscribe_events();

    engine.enqueue_passage(passage_id).await;

    let event = events.recv().await.unwrap();
    match event {
        PlaybackEvent::PassageStarted { song_albums, .. } => {
            assert_eq!(song_albums.len(), 2);
            assert!(song_albums.contains(&album_id_1));
            assert!(song_albums.contains(&album_id_2));
        }
        _ => panic!("Expected PassageStarted"),
    }
}
```

---

### [DEBT-FUNC-005] Duration Played Calculation Stubbed

**Severity:** HIGH
**Files:** `wkmp-ap/src/playback/engine.rs:2018, 2103`
**Impact:** PassageComplete events report duration_played as 0.0 seconds (incorrect)

**SHALL Requirements:**

**[REQ-DEBT-FUNC-005-010]** The system SHALL track passage playback start time when mixing begins

**[REQ-DEBT-FUNC-005-020]** PassageComplete events SHALL calculate duration_played as elapsed time from start to completion

**[REQ-DEBT-FUNC-005-030]** Duration SHALL be calculated in seconds with millisecond precision

**[REQ-DEBT-FUNC-005-040]** If start time not available (error condition), duration_played SHALL be 0.0

**Design:**

Add start time tracking to mixer:

```rust
// In CrossfadeMixer
pub struct CrossfadeMixer {
    // ...existing fields...
    passage_start_times: HashMap<usize, Instant>,  // NEW: chain_index → start time
}

impl CrossfadeMixer {
    pub fn on_chain_started(&mut self, chain_index: usize) {
        self.passage_start_times.insert(chain_index, Instant::now());
    }

    pub fn get_passage_duration(&self, chain_index: usize) -> Option<f64> {
        self.passage_start_times.get(&chain_index)
            .map(|start| start.elapsed().as_secs_f64())
    }

    pub fn on_chain_completed(&mut self, chain_index: usize) -> Option<f64> {
        self.passage_start_times.remove(&chain_index)
            .map(|start| start.elapsed().as_secs_f64())
    }
}
```

Use in event emission:

```rust
let duration_played = self.mixer.lock().await.on_chain_completed(chain_index)
    .unwrap_or(0.0);

self.event_bus.publish(PlaybackEvent::PassageComplete {
    // ...other fields...
    duration_played,  // Now calculated
});
```

**Acceptance Test:**
```rust
#[tokio::test]
async fn test_passage_duration_calculated() {
    let engine = setup_engine().await;
    let mut events = engine.subscribe_events();

    // Enqueue short passage (3 seconds)
    let passage = create_test_passage_3s();
    engine.enqueue_passage(passage).await;

    // Wait for completion
    let event = wait_for_event(&mut events, |e| {
        matches!(e, PlaybackEvent::PassageComplete { .. })
    }).await;

    match event {
        PlaybackEvent::PassageComplete { duration_played, .. } => {
            // Should be ~3.0 seconds (±100ms tolerance)
            assert!((duration_played - 3.0).abs() < 0.1);
        }
        _ => panic!("Expected PassageComplete"),
    }
}
```

---

## 3. Code Quality Issues

### [DEBT-QUALITY-001] Excessive .unwrap() Usage

**Severity:** MEDIUM
**Count:** 376 instances
**Risk:** Potential panics in production

**SHALL Requirements:**

**[REQ-DEBT-QUALITY-001-010]** Critical audio thread code SHALL NOT use .unwrap() for mutex locks

**[REQ-DEBT-QUALITY-001-020]** Mutex lock failures SHALL be handled with proper error propagation

**[REQ-DEBT-QUALITY-001-030]** Event channel operations SHALL use match/if-let instead of .unwrap()

**Priority Locations:**
1. `audio/buffer.rs` - Mutex locks (11 instances)
2. `events.rs` - Channel operations (3 instances)
3. Audio output callback paths

**Design Pattern:**

```rust
// BEFORE (panic on poisoned mutex)
let guard = mutex.lock().unwrap();

// AFTER (propagate error)
let guard = mutex.lock().map_err(|e| {
    Error::MutexPoisoned(format!("Audio buffer mutex poisoned: {}", e))
})?;
```

**Acceptance Test:**
```rust
#[tokio::test]
async fn test_mutex_poison_handling() {
    let buffer = AudioBuffer::new(1000);

    // Simulate mutex poisoning
    poison_mutex(&buffer.producer);

    // Should return error, not panic
    let result = buffer.push(&[0.0, 0.0]);
    assert!(matches!(result, Err(Error::MutexPoisoned(_))));
}
```

**Implementation Scope:** Focus on hot paths first (audio thread, decoder workers), defer test code unwraps.

---

### [DEBT-QUALITY-002] Large Monolithic Files

**Severity:** MEDIUM
**Files:** `engine.rs` (3,573 lines), `mixer.rs` (1,933 lines), `handlers.rs` (1,305 lines)

**SHALL Requirements:**

**[REQ-DEBT-QUALITY-002-010]** playback/engine.rs SHALL be split into 3 modules: engine_core, engine_queue, engine_diagnostics

**[REQ-DEBT-QUALITY-002-020]** Each refactored module SHALL be <1500 lines

**[REQ-DEBT-QUALITY-002-030]** Public API SHALL remain unchanged (internal refactoring only)

**Design:**

```
wkmp-ap/src/playback/
├── engine/
│   ├── mod.rs               # Re-exports public API
│   ├── core.rs              # State management, lifecycle
│   ├── queue.rs             # Queue operations, enqueue/skip
│   └── diagnostics.rs       # Status queries, telemetry
├── pipeline/
│   ├── mixer/
│   │   ├── mod.rs
│   │   ├── crossfade.rs     # Crossfade logic
│   │   └── buffer_mgmt.rs   # Buffer operations
```

**Acceptance Test:**
```rust
#[test]
fn test_public_api_unchanged() {
    // Verify all public functions still accessible
    let engine = PlaybackEngine::new(...);
    engine.enqueue_passage(...);
    engine.skip_current();
    engine.get_status();
    // If compiles, API unchanged
}
```

---

### [DEBT-QUALITY-003] Compiler Warnings

**Severity:** MEDIUM
**Count:** 21 warnings

**SHALL Requirements:**

**[REQ-DEBT-QUALITY-003-010]** All unused imports SHALL be removed

**[REQ-DEBT-QUALITY-003-020]** Dead code that is never called SHALL be removed

**[REQ-DEBT-QUALITY-003-030]** Dead code used indirectly via Axum routing SHALL be marked with #[allow(dead_code)]

**Actions:**
1. Run `cargo fix --lib -p wkmp-ap` (auto-fixes 4 warnings)
2. Manually review remaining 17 warnings
3. Remove genuinely unused code
4. Add `#[allow(dead_code)]` for indirectly-used functions (e.g., `developer_ui()` called via routing)

**Acceptance Test:**
```bash
cargo build -p wkmp-ap 2>&1 | grep "warning:" | wc -l
# Should output: 0
```

---

### [DEBT-QUALITY-004] Duplicate Configuration Files

**Severity:** MEDIUM
**Files:** `config.rs` (6,994 bytes), `config_new.rs` (14,804 bytes)

**SHALL Requirements:**

**[REQ-DEBT-QUALITY-004-010]** Only one config module SHALL exist in wkmp-ap

**[REQ-DEBT-QUALITY-004-020]** The active config system SHALL be identified by checking main.rs imports

**[REQ-DEBT-QUALITY-004-030]** The obsolete config file SHALL be deleted

**Actions:**
1. Check `main.rs`: Which config is imported?
2. If `config_new`: Rename `config_new.rs` → `config.rs`, delete old `config.rs`
3. If `config`: Delete `config_new.rs`
4. Verify no compilation errors

**Acceptance Test:**
```bash
ls wkmp-ap/src/config*.rs | wc -l
# Should output: 1
```

---

### [DEBT-QUALITY-005] Backup Files in Repository

**Severity:** LOW
**Files:** `events.rs.backup`, `events.rs.backup2`

**SHALL Requirements:**

**[REQ-DEBT-QUALITY-005-010]** Backup files SHALL NOT be committed to repository

**[REQ-DEBT-QUALITY-005-020]** Version control history SHALL serve as backup mechanism

**Actions:**
1. Delete `wkmp-ap/src/events.rs.backup`
2. Delete `wkmp-ap/src/events.rs.backup2`
3. Add `*.backup*` to `.gitignore` if not present

**Acceptance Test:**
```bash
find wkmp-ap/src -name "*.backup*" | wc -l
# Should output: 0
```

---

## 4. Medium-Priority TODOs (Future Enhancements)

### [DEBT-FUTURE-001] Phase 5 Features

**Files:** `playback/pipeline/mixer.rs:893-960`
**Status:** Future roadmap, not blocking

**Features:**
- Seeking support (`set_position`)
- Drain-based buffer management updates
- Ring buffer underrun detection improvements

**Defer Until:** Phase 5 implementation (per project roadmap)

---

### [DEBT-FUTURE-002] Outdated TODO Comment

**File:** `api/handlers.rs:1302`
**Issue:** Comment says "need to implement dynamic shared_secret embedding" but feature already complete in `server.rs:77`

**Action:** Delete outdated TODO comment

---

### [DEBT-FUTURE-003] Clipping Warning Logging

**File:** `playback/pipeline/mixer.rs:534`

**SHALL Requirement:**

**[REQ-DEBT-FUTURE-003-010]** The mixer SHALL log a warning when audio samples exceed ±1.0 (clipping detected)

**Design:**
```rust
if mixed_left.abs() > 1.0 || mixed_right.abs() > 1.0 {
    warn!("Audio clipping detected at frame {}: L={:.2}, R={:.2}",
          frame_pos, mixed_left, mixed_right);
}
```

---

## 5. Implementation Phases

### Sprint 1: Security & Critical (Week 1)

**Focus:** Eliminate security vulnerability and high-impact bugs

**Deliverables:**
1. ✅ [DEBT-SEC-001] POST/PUT authentication (CRITICAL)
2. ✅ [DEBT-FUNC-001] File paths in decoder errors
3. ✅ [DEBT-FUNC-002] Buffer config from database

**Success Criteria:**
- All POST/PUT endpoints require authentication
- Decoder errors include file paths
- Tuning tool settings respected
- Security tests pass

---

### Sprint 2: Functionality & Diagnostics (Week 2)

**Focus:** Complete missing telemetry and metadata

**Deliverables:**
4. ✅ [DEBT-FUNC-003] Buffer chain diagnostics
5. ✅ [DEBT-FUNC-004] Song album UUIDs
6. ✅ [DEBT-FUNC-005] Duration played calculation
7. ✅ [DEBT-QUALITY-004] Resolve config duplication
8. ✅ [DEBT-QUALITY-005] Delete backup files

**Success Criteria:**
- Developer UI shows complete telemetry
- Passage events include album metadata
- Duration played accurate
- One config file only
- No backup files in repo

---

### Sprint 3: Code Health (Week 3)

**Focus:** Improve maintainability and code quality

**Deliverables:**
9. ✅ [DEBT-QUALITY-001] Audit/fix high-risk .unwrap() calls (audio thread priority)
10. ✅ [DEBT-QUALITY-003] Fix all compiler warnings
11. ✅ [DEBT-QUALITY-002] Refactor engine.rs
12. ✅ [DEBT-FUTURE-003] Add clipping warning log
13. ✅ [DEBT-FUTURE-002] Remove outdated TODO

**Success Criteria:**
- Zero compiler warnings
- No .unwrap() in audio hot paths
- engine.rs split into 3 modules
- All tests pass

---

## 6. Acceptance Criteria (Overall)

**Sprint 1 Complete When:**
- [ ] POST/PUT authentication enforced for all endpoints
- [ ] Security tests pass (TC-SEC-001-01 through TC-SEC-001-04)
- [ ] Decoder errors include file paths
- [ ] Buffer settings read from database

**Sprint 2 Complete When:**
- [ ] Developer UI buffer chain diagnostics complete
- [ ] Passage events include album UUIDs
- [ ] Duration played calculated accurately
- [ ] Single config file only
- [ ] No backup files

**Sprint 3 Complete When:**
- [ ] Zero compiler warnings (`cargo build -p wkmp-ap`)
- [ ] No .unwrap() in audio thread code paths
- [ ] engine.rs <1500 lines (split complete)
- [ ] All integration tests pass

**Full Remediation Complete When:**
- [ ] All 13 TODOs resolved
- [ ] Security issue eliminated
- [ ] All acceptance tests pass
- [ ] Code review approved

---

## 7. Risk Assessment

**Sprint 1 Risks:**
- **Authentication changes break existing clients:** Mitigate with backward-compatible grace period (warn but allow)
- **Buffer config queries slow initialization:** Mitigate with cached values

**Sprint 2 Risks:**
- **Album queries impact performance:** Mitigate with query once at enqueue time, cache in queue entry
- **Duration tracking overhead:** Minimal - HashMap lookup, negligible

**Sprint 3 Risks:**
- **Refactoring introduces bugs:** Mitigate with comprehensive test suite before/after
- **.unwrap() replacement adds verbosity:** Acceptable for safety

---

## 8. Testing Strategy

**Unit Tests:**
- Each issue gets dedicated test suite
- Verify fix works as specified
- Verify edge cases handled

**Integration Tests:**
- Authentication flow (POST/PUT with/without credentials)
- End-to-end playback with duration tracking
- Developer UI telemetry accuracy

**Regression Tests:**
- Existing test suite must pass after each sprint
- No functionality broken by refactoring

---

## Document Version

**Version:** 1.0
**Status:** Draft - Ready for /plan
**Author:** WKMP Development Team
**Date:** 2025-10-29
