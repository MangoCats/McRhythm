# PLAN027: Technical Debt Maintenance & Quality Improvements

**Status:** Ready for Review
**Created:** 2025-11-10
**Source:** [TECHNICAL_DEBT_REPORT.md](../TECHNICAL_DEBT_REPORT.md)
**Dependencies:** PLAN026 (completed)

---

## Executive Summary

**Goal:** Address all MEDIUM and LOW priority technical debt identified in post-PLAN026 analysis

**Scope:** 22 technical debt items across 7 categories
- 5 MEDIUM priority items (53-78 hours)
- 17 LOW priority items (71-106 hours)

**Total Effort:** 124-184 hours (4-6 sprints)

**Approach:** Incremental sprints, each delivering production value
- Sprint 1 (IMMEDIATE): Critical fixes affecting CI/CD (2-3h)
- Sprint 2 (ARCHITECTURAL): Event system unification (24-32h)
- Sprint 3 (QUALITY): Test coverage & error handling (20-30h)
- Sprint 4 (FEATURES): API client completion (26-40h)
- Sprint 5 (DEFERRED): Sprint 3 items from PLAN026 (40-60h)
- Sprint 6 (POLISH): Documentation & cleanup (12-19h)

**Priority Philosophy:**
1. Fix CI reliability first (flaky tests block deployment)
2. Architectural improvements next (reduce long-term maintenance)
3. Quality improvements (test coverage, error handling)
4. Feature completion (API clients)
5. Deferred enhancements (waveform, audio features)

---

## Sprint 1: IMMEDIATE Fixes (2-3 hours)

**Goal:** Fix CI reliability and remove misleading comments
**Deliverable:** 100% passing CI builds, clean codebase

### Item 1.1: Fix Flaky Performance Test

**Priority:** CRITICAL (blocks CI)
**Effort:** 1-2 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 4.1](../TECHNICAL_DEBT_REPORT.md)

**Problem:**
`tc_s_nf_011_01_performance_benchmark` fails intermittently due to 2.0x timing variance threshold being too strict for CI environments (JIT compilation, system load, caching).

**File:** [tests/system_tests.rs:734](../../wkmp-ai/tests/system_tests.rs#L734)

**Current Code:**
```rust
assert!(
    ratio >= 0.5 && ratio <= 2.0,
    "Performance degradation detected: first={:.3}s, last={:.3}s (ratio: {:.2})",
    first_duration / 1000.0,
    last_duration / 1000.0,
    ratio
);
```

**Solution:** Increase threshold to 3.0x (accounts for CI noise)

**New Code:**
```rust
// Allow 3x variation (accounts for CI noise, JIT, caching)
// Test verifies no major performance regression, not exact timing
assert!(
    ratio >= 0.33 && ratio <= 3.0,
    "Performance degradation detected: first={:.3}s, last={:.3}s (ratio: {:.2})",
    first_duration / 1000.0,
    last_duration / 1000.0,
    ratio
);
```

**Acceptance Test:**
- Run test 10 times locally: all pass
- Run test in CI environment: no failures over 20 builds
- Performance still detects 4x+ regressions

**Alternative (if threshold still too strict):**
Mark test as `#[ignore]`, run manually for performance validation

---

### Item 1.2: Remove Outdated TODO Comment

**Priority:** LOW (cleanup)
**Effort:** 1 minute
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 2.3](../TECHNICAL_DEBT_REPORT.md)

**File:** [session_orchestrator.rs:657](../../wkmp-ai/src/import_v2/session_orchestrator.rs#L657)

**Current:**
```rust
// TODO: When workflow uses FlavorSynthesizer, replace with direct result
```

**Issue:** FlavorSynthesizer already integrated (REQ-TD-007), comment is outdated.

**Solution:** Delete comment entirely

**Acceptance Test:** Grep for "FlavorSynthesizer.*TODO" returns no results

---

**Sprint 1 Deliverable:** Clean CI builds, no flaky tests, no misleading comments

**Estimated Completion:** 1 day (includes testing)

---

## Sprint 2: ARCHITECTURAL Improvements (24-32 hours)

**Goal:** Unify event system, remove temporary scaffolding
**Deliverable:** Single event system, clean architecture

### Item 2.1: Event System Unification

**Priority:** MEDIUM
**Effort:** 16-24 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 1.1 & 6.1](../TECHNICAL_DEBT_REPORT.md)

**Problem:**
Dual event systems in use:
- `import_v2::ImportEvent` (new, import-specific)
- `wkmp_common::WkmpEvent` (legacy, system-wide)

Event bridge translates between them (temporary scaffolding).

**Impact:**
- Maintenance burden (keep both in sync)
- Performance overhead (translation layer)
- Developer confusion (which event type to use?)

**Files Affected:**
- `wkmp-ai/src/event_bridge.rs` (DELETE)
- `wkmp-ai/src/import_v2/types.rs` (move ImportEvent to wkmp-common)
- `wkmp-common/src/events.rs` (merge ImportEvent variants)
- `wkmp-ai/src/import_v2/sse_broadcaster.rs` (update SSE format)
- `wkmp-ui/src/sse_client.rs` (update SSE handlers - OTHER REPO)
- `wkmp-pd/src/event_subscriber.rs` (update event handlers - OTHER REPO)

**Solution:**

#### Phase 1: Merge Event Types (8-12h)

**Step 1:** Move ImportEvent variants to wkmp-common::WkmpEvent
```rust
// wkmp-common/src/events.rs

pub enum WkmpEvent {
    // ... existing variants ...

    // Import workflow events (migrated from import_v2::ImportEvent)
    PassagesDiscovered {
        session_id: Uuid,
        file_path: String,
        count: usize,
    },
    SongStarted {
        session_id: Uuid,
        song_index: usize,
        total_songs: usize,
    },
    ExtractionComplete {
        session_id: Uuid,
        song_index: usize,
        confidence: f64,
    },
    FusionComplete {
        session_id: Uuid,
        song_index: usize,
        confidence: f64,
    },
    ValidationComplete {
        session_id: Uuid,
        song_index: usize,
        warnings: Vec<String>,
        conflicts: Vec<(String, ConflictSeverity)>,
    },
    SongComplete {
        session_id: Uuid,
        song_index: usize,
        success: bool,
    },
    SessionComplete {
        session_id: Uuid,
        total_songs: usize,
        successful: usize,
        failed: usize,
    },
    SessionFailed {
        session_id: Uuid,
        error: String,
    },
}
```

**Step 2:** Update all event emissions in wkmp-ai
- `session_orchestrator.rs`: 5 emissions
- `song_workflow_engine.rs`: 9 emissions
- Delete `import_v2/types.rs` ImportEvent enum

**Step 3:** Delete event_bridge.rs module
- Remove from `lib.rs` imports
- Remove from router setup
- Delete file

**Acceptance Tests:**
- All 274 tests still passing
- Grep for "ImportEvent" in wkmp-ai returns 0 results
- SSE stream emits WkmpEvent format only

#### Phase 2: Update Dependent Modules (8-12h)

**wkmp-ui Changes:**
```rust
// Before (dual event handling)
match event {
    ImportEvent::SongStarted { .. } => { /* handle */ }
    WkmpEvent::ImportProgressUpdate { .. } => { /* handle */ }
}

// After (unified)
match event {
    WkmpEvent::SongStarted { .. } => { /* handle */ }
    WkmpEvent::ImportProgressUpdate { .. } => { /* handle */ }
}
```

**wkmp-pd Changes:**
- Update event subscription to WkmpEvent only
- Remove ImportEvent handling code
- Test: PD still responds to import completion events

**Acceptance Tests:**
- wkmp-ui displays import progress correctly
- wkmp-pd detects import completion
- SSE stream format validated with integration test

**Rollback Plan:**
If migration fails, keep event_bridge.rs and revert changes. Event bridge provides compatibility layer for safe rollback.

---

**Sprint 2 Deliverable:** Single unified event system, event_bridge.rs deleted, all modules use WkmpEvent

**Estimated Completion:** 3-4 days

---

## Sprint 3: QUALITY Improvements (20-30 hours)

**Goal:** Improve test coverage, audit error handling
**Deliverable:** Integration tests with real audio, safer error handling

### Item 3.1: Integration Tests with Real Audio Files

**Priority:** MEDIUM
**Effort:** 8-12 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 2.4](../TECHNICAL_DEBT_REPORT.md)

**Problem:**
Test coverage gaps identified:
- No end-to-end tests with real FLAC/MP3 files
- Error isolation not tested (failing passage handling)
- Validation thresholds not tested with known conflicts

**Current State:**
Unit tests use generated sine waves. Integration tests use minimal fixtures.

**Solution:**

#### Step 1: Create Test Fixtures (2-3h)

**Directory Structure:**
```
wkmp-ai/tests/fixtures/audio/
├── multi_track_album.flac     # 3 tracks with 2sec silence between
├── picard_tagged.mp3          # MP3 with MusicBrainz UFID frame
├── conflicting_metadata.mp3   # ID3 tags: "Beatles" vs "The Beatles"
├── corrupt_passage.flac       # Intentionally corrupted mid-file
└── minimal_valid.flac         # 3-second minimum for chromaprint
```

**Fixture Generation:**
```bash
# Use hound crate to generate test FLAC files
# Use id3 crate to inject UFID frames for testing
# Use hex editor to corrupt passages for error isolation tests
```

**Test Data Documentation:**
```markdown
# tests/fixtures/audio/README.md

## Test Fixtures

### multi_track_album.flac
- Duration: 15 seconds (3 tracks × 5 seconds)
- Silence: 2 seconds between tracks (at 5s and 10s)
- Expected: 3 passages detected
- Sample rate: 44.1kHz, 16-bit stereo

### picard_tagged.mp3
- Contains UFID frame with owner "http://musicbrainz.org"
- MBID: 12345678-1234-1234-1234-123456789abc
- Expected: MBID extracted with 0.95 confidence

### conflicting_metadata.mp3
- ID3v2.3 tag: artist = "Beatles"
- ID3v2.4 tag: artist = "The Beatles"
- Expected: Consistency warning (similarity 0.82)
```

#### Step 2: Write Integration Tests (6-9h)

**Test 1: Full Import Workflow**
```rust
#[tokio::test]
async fn test_full_import_multi_track_album() {
    // Load multi_track_album.flac
    let audio_path = fixture_path("multi_track_album.flac");

    // Import via SessionOrchestrator
    let session_id = orchestrator.import_file(&audio_path).await.unwrap();

    // Verify: 3 passages detected
    let passages = db.get_passages_for_file(&audio_path).await.unwrap();
    assert_eq!(passages.len(), 3);

    // Verify: Passage boundaries match expected (silence at 5s, 10s)
    assert_eq!(passages[0].end_ticks, 5 * 28_224_000);
    assert_eq!(passages[1].start_ticks, 5 * 28_224_000);
    assert_eq!(passages[1].end_ticks, 10 * 28_224_000);

    // Verify: All passages fingerprinted
    for passage in &passages {
        assert!(passage.chromaprint_fingerprint.is_some());
    }

    // Verify: Session completed successfully
    let session = db.get_session(session_id).await.unwrap();
    assert_eq!(session.status, "completed");
    assert_eq!(session.successful_passages, 3);
}
```

**Test 2: Error Isolation**
```rust
#[tokio::test]
async fn test_error_isolation_corrupt_passage() {
    let audio_path = fixture_path("corrupt_passage.flac");

    // Import file with corrupted middle passage
    let session_id = orchestrator.import_file(&audio_path).await.unwrap();

    // Verify: Session completed (not aborted)
    let session = db.get_session(session_id).await.unwrap();
    assert_eq!(session.status, "completed");

    // Verify: Other passages imported successfully
    assert!(session.successful_passages > 0);
    assert!(session.failed_passages > 0);

    // Verify: Error logged for corrupt passage
    let events = get_session_events(session_id);
    assert!(events.iter().any(|e| matches!(e, WkmpEvent::SongComplete { success: false, .. })));
}
```

**Test 3: Validation Thresholds**
```rust
#[tokio::test]
async fn test_consistency_validation_thresholds() {
    let audio_path = fixture_path("conflicting_metadata.mp3");

    // Import file with conflicting artist tags
    let session_id = orchestrator.import_file(&audio_path).await.unwrap();

    // Verify: Validation warning generated
    let events = get_session_events(session_id);
    let validation_event = events.iter().find(|e| {
        matches!(e, WkmpEvent::ValidationComplete { warnings, .. } if !warnings.is_empty())
    }).expect("Expected validation warnings");

    if let WkmpEvent::ValidationComplete { warnings, .. } = validation_event {
        assert!(warnings[0].contains("Beatles"));
        assert!(warnings[0].contains("similarity"));
    }
}
```

**Test 4: MBID Extraction**
```rust
#[tokio::test]
async fn test_mbid_extraction_from_ufid() {
    let audio_path = fixture_path("picard_tagged.mp3");

    // Import MP3 with UFID frame
    let session_id = orchestrator.import_file(&audio_path).await.unwrap();

    // Verify: MBID extracted from UFID
    let passages = db.get_passages_for_file(&audio_path).await.unwrap();
    let identity = passages[0].identity.as_ref().unwrap();

    assert_eq!(
        identity.mbid,
        Some(Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap())
    );
    assert_eq!(identity.mbid_confidence, 0.95);
}
```

**Acceptance Tests:**
- All 4 integration tests pass
- Total test count: 274 → 278+ tests
- Test execution time: <5 seconds (fixtures are small)

---

### Item 3.2: Error Handling Audit

**Priority:** MEDIUM
**Effort:** 4-6 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 3.1](../TECHNICAL_DEBT_REPORT.md)

**Problem:**
146 `.unwrap()/.expect()` calls across codebase. Most are in test code (acceptable), but ~30-40 in production code need review.

**Solution:**

#### Step 1: Audit Production .unwrap() Calls (2-3h)

**Script:**
```bash
# Find all .unwrap() in production code (exclude tests)
grep -rn "\.unwrap()" wkmp-ai/src --include="*.rs" | grep -v "tests::" | grep -v "#\[cfg(test)\]"
```

**Classification:**
For each `.unwrap()`:
1. **SAFE (Validated Invariant):** Document with `.expect("reason")`
2. **RISKY (User Input):** Convert to `?` operator or `unwrap_or_default()`
3. **TEST CODE:** Ignore (test panics are acceptable)

**Example Audit Results:**
```rust
// SAFE - Validated invariant
pub fn process_uuid(uuid_str: &str) -> Uuid {
    // Caller contract: uuid_str must be valid UUID (documented in fn docs)
    Uuid::parse_str(uuid_str)
        .expect("UUID validation failed - caller contract violation")
}

// RISKY - User input, should handle error
pub fn load_config(path: &Path) -> Config {
    let contents = fs::read_to_string(path).unwrap(); // RISKY!
    toml::from_str(&contents).unwrap()                 // RISKY!
}

// FIXED - Proper error handling
pub fn load_config(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)?;
    let config = toml::from_str(&contents)?;
    Ok(config)
}
```

#### Step 2: Convert Risky .unwrap() to Proper Error Handling (2-3h)

**Priority Files (highest .unwrap() count):**
1. `tier2/metadata_fuser.rs`: 18 occurrences
2. `tier2/boundary_fuser.rs`: 11 occurrences
3. `tier2/identity_resolver.rs`: 11 occurrences
4. `tier2/flavor_synthesizer.rs`: 10 occurrences

**Conversion Pattern:**
```rust
// Before
let metadata = extractor.extract(file).unwrap();

// After (if error should propagate)
let metadata = extractor.extract(file)?;

// After (if fallback acceptable)
let metadata = extractor.extract(file).unwrap_or_default();

// After (if logging needed)
let metadata = extractor.extract(file).unwrap_or_else(|e| {
    tracing::warn!("Metadata extraction failed: {}, using defaults", e);
    Metadata::default()
});
```

**Acceptance Tests:**
- Grep for production `.unwrap()` shows only documented invariants
- All `.expect()` calls include clear reason strings
- All tests still pass (no behavioral changes)

---

### Item 3.3: Test Code Panic Messages

**Priority:** LOW
**Effort:** 2-3 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 4.2](../TECHNICAL_DEBT_REPORT.md)

**Problem:**
9 `panic!()` calls in test code have unclear messages.

**Solution:**
Replace with descriptive panic messages showing expected vs actual.

**Example:**
```rust
// Before
_ => panic!("Wrong event type"),

// After
_ => panic!("Expected ImportProgressUpdate, got {:?}", event),
```

**Files:**
- `file_scanner.rs`: 2 occurrences
- `event_bridge.rs`: 5 occurrences
- `consistency_checker.rs`: 2 occurrences

**Acceptance Test:** All panic messages include expected vs actual context

---

**Sprint 3 Deliverable:** Integration tests with real audio, safer error handling, clear test failures

**Estimated Completion:** 3-4 days

---

## Sprint 4: FEATURE Completion (26-40 hours)

**Goal:** Complete MusicBrainz and AcoustID API clients
**Deliverable:** Functional metadata enrichment, automatic MBID lookup

### Item 4.1: MusicBrainz Client Implementation

**Priority:** MEDIUM
**Effort:** 12-16 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 1.2](../TECHNICAL_DEBT_REPORT.md)

**Problem:**
MusicBrainz client has 5 `#[allow(dead_code)]` suppressions. API rate limiting configured but not used.

**Dead Code:**
- `lookup_recording()` - Recording lookup by MBID
- `search_recordings()` - Text search for recordings
- `lookup_artist()` - Artist lookup by MBID
- `lookup_release()` - Release lookup by MBID
- `query_by_isrc()` - Recording lookup by ISRC code

**Solution:**

#### Phase 1: Implement API Methods (6-8h)

**Method 1: lookup_recording()**
```rust
/// Look up recording by MBID
///
/// Rate limit: 1 request/second (MusicBrainz policy)
/// Caches results in-memory for 1 hour
pub async fn lookup_recording(&self, mbid: Uuid) -> Result<RecordingMetadata> {
    // Check rate limit
    self.rate_limiter.check().await?;

    // API call
    let url = format!(
        "https://musicbrainz.org/ws/2/recording/{}?inc=artists+releases&fmt=json",
        mbid
    );

    let response = self.client.get(&url)
        .header("User-Agent", &self.user_agent)
        .send()
        .await?;

    // Parse response
    let recording: MBRecording = response.json().await?;

    Ok(RecordingMetadata {
        title: recording.title,
        artists: recording.artist_credit.iter().map(|a| a.name.clone()).collect(),
        release: recording.releases.first().map(|r| r.title.clone()),
        duration_ms: recording.length,
    })
}
```

**Method 2: search_recordings()**
```rust
/// Search for recordings by title + artist
///
/// Returns top 5 matches sorted by score
pub async fn search_recordings(
    &self,
    title: &str,
    artist: Option<&str>,
) -> Result<Vec<RecordingMatch>> {
    self.rate_limiter.check().await?;

    let query = if let Some(artist) = artist {
        format!("recording:\"{}\" AND artist:\"{}\"", title, artist)
    } else {
        format!("recording:\"{}\"", title)
    };

    let url = format!(
        "https://musicbrainz.org/ws/2/recording/?query={}&fmt=json&limit=5",
        urlencoding::encode(&query)
    );

    let response = self.client.get(&url)
        .header("User-Agent", &self.user_agent)
        .send()
        .await?;

    let results: MBSearchResults = response.json().await?;

    Ok(results.recordings.into_iter().map(|r| RecordingMatch {
        mbid: r.id,
        title: r.title,
        artists: r.artist_credit.iter().map(|a| a.name.clone()).collect(),
        score: r.score,
    }).collect())
}
```

**Method 3: lookup_artist(), lookup_release(), query_by_isrc()**
Similar implementation pattern with appropriate endpoints.

#### Phase 2: Integration with Identity Resolver (4-6h)

**Add to Tier 2 identity_resolver.rs:**
```rust
impl IdentityResolver {
    async fn enrich_metadata(&self, identity: &mut ResolvedIdentity) -> Result<()> {
        // If we have MBID, validate and enrich from MusicBrainz
        if let Some(mbid) = identity.mbid {
            match self.mb_client.lookup_recording(mbid).await {
                Ok(metadata) => {
                    tracing::info!("Enriched metadata from MusicBrainz for MBID {}", mbid);
                    identity.title = Some(metadata.title);
                    identity.artists = metadata.artists;
                    identity.enrichment_source = Some("MusicBrainz");
                }
                Err(e) => {
                    tracing::warn!("MusicBrainz lookup failed for {}: {}", mbid, e);
                }
            }
        }

        Ok(())
    }
}
```

#### Phase 3: Test Rate Limiting (2-3h)

**Test: Rate limit enforcement**
```rust
#[tokio::test]
async fn test_musicbrainz_rate_limiting() {
    let client = MusicBrainzClient::new("test-agent");

    let start = Instant::now();

    // Make 3 requests (should take ~2 seconds with 1 req/sec limit)
    for _ in 0..3 {
        client.lookup_recording(test_mbid()).await.ok();
    }

    let duration = start.elapsed();
    assert!(duration >= Duration::from_secs(2), "Rate limiting not enforced");
}
```

**Acceptance Tests:**
- All 5 methods implemented and remove `#[allow(dead_code)]`
- Rate limiting enforced (1 req/sec)
- Integration test with real MusicBrainz API (using test MBID)
- Metadata enrichment works in identity resolver

---

### Item 4.2: AcoustID Client Implementation

**Priority:** LOW-MEDIUM
**Effort:** 6-8 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 1.3](../TECHNICAL_DEBT_REPORT.md)

**Problem:**
AcoustID client marked as `#[allow(dead_code)]`, not integrated. Chromaprint fingerprints generated but not submitted to AcoustID.

**Solution:**

#### Phase 1: Implement lookup() Method (3-4h)

**Register for API Key:**
```
https://acoustid.org/new-application
Application: WKMP Music Player
Description: Open source DJ music player
```

**Implementation:**
```rust
/// Look up recording MBID by chromaprint fingerprint
///
/// Returns list of possible matches sorted by score
pub async fn lookup(
    &self,
    fingerprint: &str,
    duration_secs: u32,
) -> Result<Vec<AcoustIDMatch>> {
    // Rate limit: 3 req/sec (AcoustID policy)
    self.rate_limiter.check().await?;

    let params = [
        ("client", self.api_key.as_str()),
        ("duration", &duration_secs.to_string()),
        ("fingerprint", fingerprint),
        ("meta", "recordings"),
    ];

    let response = self.client
        .post("https://api.acoustid.org/v2/lookup")
        .form(&params)
        .send()
        .await?;

    let result: AcoustIDResponse = response.json().await?;

    if result.status != "ok" {
        return Err(Error::AcoustIDError(result.error.unwrap_or_default()));
    }

    Ok(result.results.into_iter().filter_map(|r| {
        r.recordings.map(|recordings| {
            recordings.into_iter().map(|rec| AcoustIDMatch {
                mbid: rec.id,
                score: r.score,
                sources: rec.sources.unwrap_or(1),
            }).collect()
        })
    }).flatten().collect())
}
```

#### Phase 2: Integration with Identity Resolver (2-3h)

**Add as fallback source after UFID:**
```rust
impl IdentityResolver {
    async fn resolve_mbid(&self, fingerprint: &str, duration_ms: u32) -> Option<Uuid> {
        // Try AcoustID lookup as fallback if UFID not present
        let duration_secs = duration_ms / 1000;

        match self.acoustid_client.lookup(fingerprint, duration_secs).await {
            Ok(matches) if !matches.is_empty() => {
                // Take highest scoring match with >2 sources (more reliable)
                let best_match = matches.into_iter()
                    .filter(|m| m.sources >= 2)
                    .max_by_key(|m| (m.score * 100.0) as u32)?;

                tracing::info!(
                    "AcoustID match found: MBID {} (score: {:.2}, sources: {})",
                    best_match.mbid, best_match.score, best_match.sources
                );

                Some(best_match.mbid)
            }
            Ok(_) => {
                tracing::debug!("No high-confidence AcoustID matches found");
                None
            }
            Err(e) => {
                tracing::warn!("AcoustID lookup failed: {}", e);
                None
            }
        }
    }
}
```

**Acceptance Tests:**
- `lookup()` method implemented, `#[allow(dead_code)]` removed
- Rate limiting enforced (3 req/sec)
- Integration test with real AcoustID API (using known fingerprint)
- Identity resolver uses AcoustID as fallback source

---

### Item 4.3: Audio Features Extractor (DEFERRED)

**Priority:** LOW
**Effort:** 20-30 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 1.4](../TECHNICAL_DEBT_REPORT.md)

**Status:** Blocked by Essentia integration decision

**Defer to:** Future sprint after audio analysis library evaluation

**Rationale:** Requires DSP expertise or library integration. Current workflow functional without this.

---

**Sprint 4 Deliverable:** MusicBrainz and AcoustID clients fully functional, metadata enrichment working

**Estimated Completion:** 4-5 days

---

## Sprint 5: DEFERRED Enhancements (40-60 hours)

**Goal:** Complete Sprint 3 items from PLAN026
**Deliverable:** Waveform rendering, duration tracking, audio features

**Note:** This sprint addresses items deferred from PLAN026 Sprint 3.

### Item 5.1: Waveform Rendering (REQ-TD-009)

**Priority:** LOW
**Effort:** 16-24 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 2.1](../TECHNICAL_DEBT_REPORT.md)

**Problem:**
No waveform visualization for passage boundaries in import UI.

**Solution:**

#### Phase 1: Waveform Data Generation (8-12h)

**Add to audio_loader.rs:**
```rust
/// Generate waveform data for visualization
///
/// Returns peak/RMS values for each time bucket (configurable resolution)
pub fn generate_waveform(
    &self,
    file_path: &Path,
    resolution: usize, // pixels width
) -> Result<WaveformData> {
    let samples = self.load_full(file_path)?;

    let samples_per_bucket = samples.len() / resolution;
    let mut peaks = Vec::with_capacity(resolution);

    for chunk in samples.chunks(samples_per_bucket) {
        let peak = chunk.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        peaks.push(WaveformBucket { peak, rms });
    }

    Ok(WaveformData {
        buckets: peaks,
        duration_ms: (samples.len() as f64 / 44100.0 * 1000.0) as u32,
    })
}
```

#### Phase 2: API Endpoint (4-6h)

**Add to api/ui.rs:**
```rust
#[get("/api/waveform/{file_id}")]
async fn get_waveform(
    file_id: web::Path<Uuid>,
    query: web::Query<WaveformParams>,
) -> Result<Json<WaveformData>> {
    let file_path = db.get_file_path(file_id).await?;
    let loader = AudioLoader::new();

    let waveform = loader.generate_waveform(
        &file_path,
        query.width.unwrap_or(1000) // default 1000px
    )?;

    Ok(Json(waveform))
}
```

#### Phase 3: UI Integration (4-6h)

**Frontend (wkmp-ai UI - not wkmp-ui):**
```javascript
// Render waveform on canvas
async function renderWaveform(fileId, canvasId) {
    const response = await fetch(`/api/waveform/${fileId}?width=1000`);
    const waveform = await response.json();

    const canvas = document.getElementById(canvasId);
    const ctx = canvas.getContext('2d');

    // Draw waveform peaks
    waveform.buckets.forEach((bucket, i) => {
        const x = (i / waveform.buckets.length) * canvas.width;
        const height = bucket.peak * canvas.height;
        ctx.fillRect(x, canvas.height / 2 - height / 2, 2, height);
    });

    // Draw boundary markers
    boundaries.forEach(boundary => {
        const x = (boundary.start_ticks / totalTicks) * canvas.width;
        ctx.strokeStyle = 'red';
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, canvas.height);
        ctx.stroke();
    });
}
```

**Acceptance Tests:**
- Waveform generation produces valid data (peak/RMS buckets)
- API endpoint returns waveform for file ID
- UI renders waveform with boundary markers
- Performance: <500ms for 5-minute file at 1000px resolution

---

### Item 5.2: Duration Tracking (REQ-TD-010)

**Priority:** LOW
**Effort:** 4-6 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 2.2](../TECHNICAL_DEBT_REPORT.md)

**Problem:**
File statistics missing total duration, preventing time-based progress %.

**Solution:**

**Step 1: Calculate duration during audio load**
```rust
// session_orchestrator.rs

let duration_ms = (samples.len() as f64 / sample_rate as f64 * 1000.0) as u32;

FileStats {
    total_passages: boundaries.len(),
    total_duration_ms: duration_ms, // REQ-TD-010: Track file-level duration
    processed_passages: 0,
}
```

**Step 2: Add to SSE events**
```rust
WkmpEvent::PassagesDiscovered {
    session_id,
    file_path,
    count,
    duration_ms, // NEW: Total file duration
}
```

**Step 3: UI progress calculation**
```javascript
// Calculate time-based progress percentage
const progressPercent = (processedDurationMs / totalDurationMs) * 100;
```

**Acceptance Tests:**
- FileStats includes total_duration_ms
- PassagesDiscovered event includes duration
- UI displays time-based progress (e.g., "45% (2:30 / 5:00)")

---

### Item 5.3: Audio Features Extractor Implementation

**Priority:** LOW
**Effort:** 20-30 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 1.4](../TECHNICAL_DEBT_REPORT.md)

**Defer to:** After library evaluation (aubio-rs vs essentia-rs vs manual DSP)

**Reason:** Requires significant DSP expertise, low ROI without Essentia integration.

---

**Sprint 5 Deliverable:** Waveform rendering, duration tracking, visual boundary editing

**Estimated Completion:** 5-7 days

---

## Sprint 6: POLISH & Documentation (12-19 hours)

**Goal:** Clean up dead code, improve documentation
**Deliverable:** Zero `#[allow(dead_code)]`, API docs with examples

### Item 6.1: Dead Code Cleanup

**Priority:** LOW
**Effort:** 2-4 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 3.2](../TECHNICAL_DEBT_REPORT.md)

**Files:**
- `song_workflow_engine.rs`: Remove unused helper methods (2)
- `import_workflow.rs`: Remove unused helper struct (1)
- `flavor_synthesizer.rs`: Remove or use unused field (1)
- `silence_detector.rs`: Remove unused method (1)

**Action:** Delete truly dead code, document incomplete features.

**Acceptance Test:** Grep for `#[allow(dead_code)]` returns only documented incomplete features.

---

### Item 6.2: API Documentation

**Priority:** LOW
**Effort:** 8-12 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 7.1](../TECHNICAL_DEBT_REPORT.md)

**Add `/// # Examples` to public APIs:**

**Example:**
```rust
/// Generate fingerprint for audio passage
///
/// # Arguments
/// * `samples` - Mono PCM audio samples (f32, normalized to [-1.0, 1.0])
/// * `duration_ms` - Duration of audio in milliseconds
///
/// # Returns
/// Base64-encoded Chromaprint fingerprint with confidence 0.7
///
/// # Errors
/// Returns `ImportError::AudioProcessingFailed` if:
/// - Sample buffer is empty
/// - Duration < 3 seconds (Chromaprint requirement)
/// - Chromaprint processing fails
///
/// # Performance
/// O(n) where n = sample count. Approximately 50-200ms for 5-second passage.
///
/// # Examples
/// ```rust
/// let analyzer = ChromaprintAnalyzer::default();
/// let samples = generate_sine_wave(440.0, 5.0, 44100);
/// let duration_ms = 5000;
///
/// let result = analyzer.analyze(&samples, duration_ms)?;
/// assert_eq!(result.confidence, 0.7);
/// assert!(!result.data.is_empty());
/// ```
pub fn analyze(&self, samples: &[f32], duration_ms: u32) -> ImportResult<ExtractorResult<String>> {
    // ...
}
```

**Priority Files:**
- Tier 1 extractors (public APIs)
- Tier 2 fusers (public APIs)
- Tier 3 validators (public APIs)

**Acceptance Test:** `cargo doc --open` shows examples for all public APIs.

---

### Item 6.3: Configuration Caching (DEFERRED)

**Priority:** LOW
**Effort:** 8-12 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 6.2](../TECHNICAL_DEBT_REPORT.md)

**Defer to:** Future performance optimization sprint

**Reason:** Current database query overhead acceptable for import workflow (not performance-critical path).

---

### Item 6.4: Architecture Decision Records

**Priority:** LOW
**Effort:** 2-3 hours
**Source:** [TECHNICAL_DEBT_REPORT.md - Category 7.2](../TECHNICAL_DEBT_REPORT.md)

**Create `docs/adr/` directory:**
```
docs/adr/
├── 001-event-system-unification.md
├── 002-musicbrainz-rate-limiting.md
├── 003-waveform-canvas-vs-svg.md
└── template.md
```

**ADR Template:**
```markdown
# ADR-NNN: Title

**Status:** Accepted | Rejected | Superseded
**Date:** YYYY-MM-DD
**Deciders:** Name(s)

## Context
What is the issue we're trying to solve?

## Decision
What did we decide?

## Consequences
What are the results (positive and negative)?

## Alternatives Considered
What other options did we evaluate?
```

**Acceptance Test:** 3 ADRs documented (event system, rate limiting, waveform rendering).

---

**Sprint 6 Deliverable:** Clean codebase, comprehensive API docs, ADR repository

**Estimated Completion:** 2-3 days

---

## Risk Assessment

### Sprint 1 Risks: LOW
- **Performance test threshold:** May need further adjustment if 3.0x still fails
- **Mitigation:** Mark as `#[ignore]` if threshold tuning insufficient

### Sprint 2 Risks: MEDIUM
- **Event system migration:** Changes affect 3 modules (wkmp-ai, wkmp-ui, wkmp-pd)
- **Mitigation:** Keep event_bridge.rs until all modules migrated (rollback safety)
- **Testing:** Integration tests verify SSE format compatibility

### Sprint 3 Risks: LOW
- **Test fixtures:** Audio file generation may be complex
- **Mitigation:** Use hound crate (already in dev-dependencies) for FLAC/WAV generation

### Sprint 4 Risks: MEDIUM
- **API rate limiting:** MusicBrainz/AcoustID may block excessive testing
- **Mitigation:** Use mock servers for unit tests, real API only for integration tests
- **API key:** AcoustID requires registration (may take 1-2 days for approval)
- **Mitigation:** Apply for API key in Sprint 1, have ready by Sprint 4

### Sprint 5 Risks: LOW-MEDIUM
- **Waveform performance:** Large files may cause UI lag
- **Mitigation:** Configurable resolution (default 1000px), cache waveform data

### Sprint 6 Risks: LOW
- **Documentation effort:** May take longer than estimated
- **Mitigation:** Document incrementally during code review

---

## Success Metrics

### Sprint 1:
- ✅ CI builds pass 100% (no flaky tests)
- ✅ Zero TODO comments referencing completed work

### Sprint 2:
- ✅ event_bridge.rs deleted
- ✅ Grep for "ImportEvent" in wkmp-ai returns 0 results
- ✅ All 274+ tests passing
- ✅ SSE stream format validated

### Sprint 3:
- ✅ 4 new integration tests with real audio files
- ✅ Total test count: 278+ passing
- ✅ Production `.unwrap()` count reduced by 50%+
- ✅ All `.expect()` include clear reason strings

### Sprint 4:
- ✅ Zero `#[allow(dead_code)]` for MusicBrainz/AcoustID clients
- ✅ MusicBrainz metadata enrichment working
- ✅ AcoustID MBID lookup working
- ✅ Rate limiting enforced (1 req/sec MusicBrainz, 3 req/sec AcoustID)

### Sprint 5:
- ✅ Waveform API endpoint functional
- ✅ UI renders waveform with boundary markers
- ✅ Duration tracking in all import events

### Sprint 6:
- ✅ Zero unjustified `#[allow(dead_code)]`
- ✅ API docs include examples for all public methods
- ✅ 3 ADRs documented

---

## Dependencies

### External:
- **AcoustID API Key:** Required for Sprint 4 (apply in Sprint 1)
- **Test Audio Files:** Required for Sprint 3 (generate with hound crate)

### Internal:
- **Sprint 2 blocks Sprint 4:** Event system must be unified before adding new event types
- **Sprint 3 provides fixtures for Sprint 5:** Integration tests reuse audio fixtures for waveform testing

### Module Coordination:
- **wkmp-ui updates:** Required for Sprint 2 (event system migration)
- **wkmp-pd updates:** Required for Sprint 2 (event system migration)

---

## Rollback Plans

### Sprint 1:
- Revert performance test threshold change
- Restore TODO comment (no functional impact)

### Sprint 2:
- **Rollback strategy:** Keep event_bridge.rs until all modules migrated
- **Validation:** Integration tests verify SSE compatibility before deleting bridge
- **Fallback:** Re-enable event_bridge.rs if wkmp-ui/wkmp-pd have issues

### Sprint 3:
- Delete test fixtures if not needed
- Revert error handling changes if behavioral regressions detected

### Sprint 4:
- **API failures:** Mock servers for testing if real API unavailable
- **Rate limiting issues:** Disable API integration, keep as stub until resolved

### Sprint 5:
- Waveform rendering optional (feature flag)
- Duration tracking backward compatible (events work without duration field)

### Sprint 6:
- Documentation changes non-functional (no rollback needed)

---

## Estimated Timeline

**Total Duration:** 16-24 weeks (part-time development)

**Sprint Schedule:**
- Sprint 1 (IMMEDIATE): Week 1 (2-3 hours)
- Sprint 2 (ARCHITECTURAL): Weeks 2-4 (24-32 hours)
- Sprint 3 (QUALITY): Weeks 5-7 (20-30 hours)
- Sprint 4 (FEATURES): Weeks 8-11 (26-40 hours)
- Sprint 5 (DEFERRED): Weeks 12-16 (40-60 hours)
- Sprint 6 (POLISH): Weeks 17-18 (12-19 hours)

**Full-Time Equivalent:** 15-23 days (assuming 8-hour workdays)

---

## Acceptance Criteria

**Plan is complete when:**
- ✅ All 5 MEDIUM priority items resolved
- ✅ All 17 LOW priority items resolved or explicitly deferred with rationale
- ✅ Zero flaky tests in CI
- ✅ Zero `#[allow(dead_code)]` without documentation
- ✅ API documentation complete (examples for all public methods)
- ✅ MusicBrainz and AcoustID clients fully functional
- ✅ Waveform rendering and duration tracking working
- ✅ Event system unified (single WkmpEvent type)
- ✅ Test coverage includes integration tests with real audio files

**Quality Gates:**
- All sprints maintain 100% test pass rate
- No regressions in existing functionality
- Code review approval for architectural changes (Sprint 2)
- Performance validation for waveform rendering (Sprint 5)

---

## Appendix: Test Data Generation

### Generating Test FLAC Files

**Using hound crate:**
```rust
use hound::{WavSpec, WavWriter};

fn generate_multi_track_album() {
    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create("multi_track_album.wav", spec).unwrap();

    // Track 1: 5 seconds at 440Hz
    write_sine_wave(&mut writer, 440.0, 5.0, 44100);

    // Silence: 2 seconds
    write_silence(&mut writer, 2.0, 44100);

    // Track 2: 5 seconds at 523Hz (C5)
    write_sine_wave(&mut writer, 523.25, 5.0, 44100);

    // Silence: 2 seconds
    write_silence(&mut writer, 2.0, 44100);

    // Track 3: 5 seconds at 659Hz (E5)
    write_sine_wave(&mut writer, 659.25, 5.0, 44100);

    writer.finalize().unwrap();

    // Convert WAV to FLAC using flac command-line tool
    std::process::Command::new("flac")
        .arg("multi_track_album.wav")
        .output()
        .unwrap();
}

fn write_sine_wave(writer: &mut WavWriter<std::io::BufWriter<std::fs::File>>, freq: f32, duration: f32, sample_rate: u32) {
    let num_samples = (duration * sample_rate as f32) as usize;
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * freq * t).sin();
        let amplitude = (sample * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).unwrap(); // Left channel
        writer.write_sample(amplitude).unwrap(); // Right channel
    }
}

fn write_silence(writer: &mut WavWriter<std::io::BufWriter<std::fs::File>>, duration: f32, sample_rate: u32) {
    let num_samples = (duration * sample_rate as f32) as usize;
    for _ in 0..num_samples {
        writer.write_sample(0i16).unwrap(); // Left channel
        writer.write_sample(0i16).unwrap(); // Right channel
    }
}
```

### Injecting UFID Frames

**Using id3 crate:**
```rust
use id3::{Tag, TagLike, frame::{Frame, Content}};

fn inject_mbid_ufid(mp3_path: &str, mbid: &str) {
    let mut tag = Tag::read_from_path(mp3_path).unwrap_or_default();

    // Create UFID frame
    let owner = "http://musicbrainz.org";
    let identifier = mbid.as_bytes();

    // Construct frame data: owner\0identifier
    let mut data = owner.as_bytes().to_vec();
    data.push(0); // Null terminator
    data.extend_from_slice(identifier);

    let frame = Frame::with_content("UFID", Content::Unknown(id3::frame::Unknown {
        data,
        ..Default::default()
    }));

    tag.add_frame(frame);
    tag.write_to_path(mp3_path, id3::Version::Id3v24).unwrap();
}
```

---

**Plan Status:** Ready for Sprint 1 execution
**Next Action:** Review and approve plan, then begin Sprint 1 (fix flaky test)
**Estimated Start Date:** 2025-11-11
**Estimated Completion:** 2026-05-01 (16-24 weeks)
