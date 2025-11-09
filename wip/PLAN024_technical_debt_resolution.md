# PLAN024: Technical Debt Resolution & Architecture Migration

**Date:** 2025-01-09
**Status:** Draft
**Priority:** Critical
**Estimated Effort:** 60-90 hours (7-11 development days)

---

## Executive Summary

This plan addresses **critical technical debt** identified in the wkmp-ai codebase, focusing on completing the migration from the old `WorkflowOrchestrator` to the new `import_v2` architecture. The primary blocker is the **dual implementation pattern** where two parallel import systems exist, creating maintenance overhead and preventing feature completion.

**Key Objectives:**
1. Complete import_v2 integration with API routes
2. Deprecate and remove old WorkflowOrchestrator system
3. Complete unfinished integrations (MusicBrainz, AcoustID, amplitude analysis)
4. Remove dead code and #[allow(dead_code)] markers
5. Improve error handling and documentation

**Impact:** Unblocks feature development, reduces maintenance burden, improves code quality.

---

## Problem Statement

### Critical Issue: Dual Implementation Pattern

**Current State:**
- **Old System:** `services/workflow_orchestrator.rs` (1451 lines) - original implementation
- **New System:** `import_v2/song_workflow_engine.rs` - PLAN023 recode with superior architecture
- **API Routes:** Currently use old system OR have incomplete integration
- **Result:** Maintenance overhead, feature duplication, confusion about which system to use

**Technical Debt Summary:**
- 130 issues identified (deduplicated: 180-260 hours effort)
- 1 Critical issue (dual implementation)
- 19 High priority issues
- 96 Medium priority issues
- Primary categories: Architecture (28), Error Handling (37), TODO/FIXME (19), Dead Code (13)

**Blocker:** Cannot add new features effectively until migration is complete.

---

## Sprint Breakdown

### Sprint 1: Architecture Migration Foundation (16-20h) - CRITICAL

**Objective:** Complete core import_v2 features to enable API migration

#### 1.1: Complete Audio Segment Extraction (8-12h) - CRITICAL
**Current:** song_workflow_engine.rs:205 has placeholder comment
**Blocker:** Passage processing non-functional without this

**Implementation:**
```rust
// wkmp-ai/src/import_v2/song_workflow_engine.rs

async fn extract_all_sources(
    &self,
    file_path: &Path,
    boundary: &PassageBoundary,
) -> ImportResult<(ExtractorResult<MetadataBundle>, ...)> {
    // Phase 1: Load audio segment (ALREADY IMPLEMENTED - lines 422-428)
    let audio_segment = self.audio_loader.load_segment(
        file_path,
        boundary.start_ticks,
        boundary.end_ticks,
    )?;

    // Phase 2-5: Already implemented
    // ...existing code...
}
```

**Note:** Audio segment extraction is **ALREADY COMPLETE** per lines 422-428. This TODO is outdated.

**Action:** Update technical debt report - this is not a blocker.

#### 1.2: Complete Config-Based Client Initialization (4-6h) - HIGH
**Current:** MusicBrainz/AcoustID clients hardcoded to `None`
**Location:** song_workflow_engine.rs:120-121

**Implementation:**
```rust
// wkmp-ai/src/import_v2/song_workflow_engine.rs

impl SongWorkflowEngine {
    pub fn with_config(
        acoustid_api_key: Option<String>,
        musicbrainz_user_agent: String,
        musicbrainz_token: Option<String>,
    ) -> Self {
        let musicbrainz_client = Some(MusicBrainzClient::new(musicbrainz_user_agent));
        let acoustid_client = acoustid_api_key.map(|key| AcoustIDClient::new(key));

        Self {
            audio_loader: AudioLoader::default(),
            id3_extractor: ID3Extractor::default(),
            chromaprint_analyzer: ChromaprintAnalyzer::default(),
            musicbrainz_client,
            acoustid_client,
            audio_features: AudioFeatureExtractor::default(),
            identity_resolver: IdentityResolver::default(),
            metadata_fuser: MetadataFuser::default(),
            flavor_synthesizer: FlavorSynthesizer::default(),
            consistency_checker: ConsistencyChecker::default(),
            completeness_scorer: CompletenessScorer::default(),
            conflict_detector: ConflictDetector::default(),
            sse_broadcaster: None,
            extraction_timeout: Duration::from_secs(30),
        }
    }
}
```

**Integration Points:**
1. Update `main.rs` to pass config to `SongWorkflowEngine`
2. Wire up `WKMP_MUSICBRAINZ_TOKEN` environment variable
3. Update API handlers to use `with_config()` constructor

**Tests:**
- Unit test for config initialization
- Integration test with real MusicBrainz/AcoustID clients

**Acceptance Criteria:**
- ✅ No #[allow(dead_code)] on musicbrainz_client/acoustid_client
- ✅ Clients initialized from config/environment
- ✅ Tests verify client creation

#### 1.3: Complete MusicBrainz/AcoustID Integration (6-8h) - HIGH
**Current:** MBID candidates always empty (line 486)
**Blocker:** Prevents identity resolution from working fully

**Implementation:**
```rust
// wkmp-ai/src/import_v2/song_workflow_engine.rs

async fn extract_all_sources(...) -> ImportResult<(...)> {
    // ...existing code...

    // Phase 6: Assemble MBID candidates
    let mut mbid_candidates = vec![];

    // 6a. Extract MBID from ID3 UFID frame (if present)
    if let Some(mbid) = extract_mbid_from_id3(&id3_result.data) {
        mbid_candidates.push(ExtractorResult {
            data: vec![MBIDCandidate {
                mbid,
                score: 1.0,  // High confidence - from file metadata
                sources: vec!["ID3_UFID".to_string()],
            }],
            confidence: 0.95,
            source: ExtractionSource::ID3,
        });
    }

    // 6b. Query AcoustID API with fingerprint (if client available)
    if let Some(client) = &self.acoustid_client {
        match client.lookup(&fingerprint_result.data).await {
            Ok(candidates) => {
                mbid_candidates.push(candidates);
            }
            Err(e) => {
                tracing::warn!("AcoustID lookup failed (non-fatal): {}", e);
                // Continue processing - error isolation
            }
        }
    }

    Ok((id3_result, mbid_candidates, audio_flavor))
}

fn extract_mbid_from_id3(metadata: &MetadataBundle) -> Option<Uuid> {
    // Check for UFID frame with MusicBrainz identifier
    // Format: "http://musicbrainz.org" + UUID
    // Implementation depends on lofty API for UFID access
    None  // Placeholder - implement when lofty supports UFID
}
```

**Acceptance Criteria:**
- ✅ AcoustID API integration functional (if API key configured)
- ✅ MBID extraction from ID3 UFID (best effort)
- ✅ Error isolation verified (API failures don't abort workflow)
- ✅ Integration test with mock AcoustID responses

**Sprint 1 Milestone:** ✅ import_v2 system feature-complete, ready for API integration

---

### Sprint 2: API Route Migration (12-16h) - CRITICAL

**Objective:** Migrate all API routes from old WorkflowOrchestrator to new SongWorkflowEngine

#### 2.1: Audit Current API Route Usage (2h)

**Files to Audit:**
- `src/api/import_workflow.rs` - Main import API
- `src/api/ui.rs` - UI routes
- `src/api/amplitude_analysis.rs` - Amplitude API
- `src/main.rs` - Route registration

**Document:**
- Which routes use WorkflowOrchestrator?
- Which routes use import_v2?
- Which routes are stubs?
- Dependencies and integration points

**Deliverable:** Migration checklist with impact analysis

#### 2.2: Migrate Import Workflow API (6-8h) - CRITICAL

**Current:** `api/import_workflow.rs` likely uses old WorkflowOrchestrator
**Target:** Use `SongWorkflowEngine` with SSE broadcasting

**Implementation:**
```rust
// wkmp-ai/src/api/import_workflow.rs

use crate::import_v2::song_workflow_engine::SongWorkflowEngine;

// POST /import/start
pub async fn start_import(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ImportRequest>,
) -> Result<Json<ImportSession>, StatusCode> {
    // 1. Create import session in database
    let session = create_session(&state.db, &request.file_path).await?;

    // 2. Create SSE broadcaster for this session
    let (event_tx, _) = broadcast::channel(1000);
    state.sse_channels.insert(session.id, event_tx.clone());

    // 3. Detect passage boundaries (existing file_scanner logic)
    let boundaries = detect_boundaries(&request.file_path).await?;

    // 4. Spawn workflow processing task
    let db = state.db.clone();
    let file_path = request.file_path.clone();

    tokio::spawn(async move {
        // Create SongWorkflowEngine with config
        let mut engine = SongWorkflowEngine::with_config(
            state.acoustid_api_key.clone(),
            "WKMP/0.1.0 (https://github.com/wkmp)".to_string(),
            state.musicbrainz_token.clone(),
        ).with_sse(event_tx, 100);  // 100ms throttle

        // Process file
        let summary = engine.process_file(&file_path, &boundaries).await;

        // Update session in database
        update_session_complete(&db, session.id, &summary).await;
    });

    Ok(Json(session))
}

// GET /import/events/:session_id - SSE endpoint
pub async fn import_events(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.sse_channels
        .get(&session_id)
        .expect("Session not found")
        .subscribe();

    let stream = BroadcastStream::new(rx)
        .map(|event| {
            Ok(Event::default()
                .json_data(event)
                .unwrap())
        });

    Sse::new(stream)
}
```

**Migration Steps:**
1. Update `AppState` to store SSE channels and config
2. Modify import endpoints to use `SongWorkflowEngine`
3. Remove old WorkflowOrchestrator usage
4. Update import session database schema if needed
5. Test end-to-end import workflow

**Acceptance Criteria:**
- ✅ POST /import/start uses SongWorkflowEngine
- ✅ GET /import/events streams SSE correctly
- ✅ Database persistence working
- ✅ Error handling comprehensive
- ✅ Integration test passes

#### 2.3: Update UI Routes (2-4h)

**Files:** `src/api/ui.rs`

**Tasks:**
- Update import status endpoints to query new database schema
- Update session management
- Remove old WorkflowOrchestrator dependencies

**Acceptance Criteria:**
- ✅ UI displays import progress correctly
- ✅ SSE events render in UI
- ✅ No references to old system

#### 2.4: Integration Testing (2-4h)

**Tests:**
- End-to-end import via API
- SSE event streaming
- Multi-passage processing
- Error scenarios

**Acceptance Criteria:**
- ✅ All API integration tests pass
- ✅ Manual UI testing successful
- ✅ Performance acceptable (<2min per song)

**Sprint 2 Milestone:** ✅ All API routes use import_v2, old system no longer referenced

---

### Sprint 3: Legacy Code Removal (8-12h) - HIGH

**Objective:** Remove old WorkflowOrchestrator and deprecated code

#### 3.1: Deprecate Old WorkflowOrchestrator (4-6h)

**Files to Remove:**
- `src/services/workflow_orchestrator.rs` (1451 lines)
- Old test files that only test WorkflowOrchestrator

**Process:**
1. Verify no API routes reference WorkflowOrchestrator
2. Search for imports: `use crate::services::workflow_orchestrator`
3. Remove file and update `src/services/mod.rs`
4. Run full test suite to catch breakage
5. Update documentation references

**Acceptance Criteria:**
- ✅ workflow_orchestrator.rs deleted
- ✅ All tests still pass
- ✅ No compilation errors
- ✅ Git commit documents removal rationale

#### 3.2: Remove Duplicate Client Implementations (2-4h)

**Old Implementations (services/):**
- `services/acoustid_client.rs` - Replaced by `import_v2/tier1/acoustid_client.rs`
- `services/musicbrainz_client.rs` - Replaced by `import_v2/tier1/musicbrainz_client.rs`

**Process:**
1. Verify import_v2 versions are feature-complete
2. Update any remaining references to use import_v2 versions
3. Delete old files
4. Update mod.rs exports

**Acceptance Criteria:**
- ✅ Only import_v2/tier1 clients remain
- ✅ All functionality preserved
- ✅ Tests pass

#### 3.3: Clean Up Dead Code Markers (2h)

**Targets (13 instances):**
- Remove #[allow(dead_code)] from song_workflow_engine.rs (4 instances)
- Remove #[allow(dead_code)] from other import_v2 modules (9 instances)

**Process:**
1. For each marker, verify code is now used
2. Remove marker
3. If code is truly unused, delete it
4. Run `cargo check` to verify

**Acceptance Criteria:**
- ✅ Zero #[allow(dead_code)] in import_v2
- ✅ Code that was dead is either used or removed
- ✅ Compilation clean

**Sprint 3 Milestone:** ✅ Legacy code removed, codebase simplified

---

### Sprint 4: Feature Completion (12-16h) - MEDIUM/HIGH

**Objective:** Complete unfinished features identified in technical debt

#### 4.1: Implement Amplitude Analysis API Integration (8-12h) - MEDIUM

**Current:** `api/amplitude_analysis.rs` returns stub data
**Service:** `services/amplitude_analyzer.rs` - Full implementation exists (Sprint 6 complete!)

**Implementation:**
```rust
// wkmp-ai/src/api/amplitude_analysis.rs

use crate::services::amplitude_analyzer::AmplitudeAnalyzer;

pub async fn analyze_amplitude(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AmplitudeAnalysisRequest>,
) -> Result<Json<AmplitudeAnalysisResponse>, StatusCode> {
    // Create analyzer with user-provided params or defaults
    let params = request.parameters.unwrap_or_default();
    let analyzer = AmplitudeAnalyzer::new(params);

    // Perform analysis
    let result = analyzer.analyze_file(
        Path::new(&request.file_path),
        request.start_time,
        request.end_time.unwrap_or(f64::MAX),
    ).await.map_err(|e| {
        tracing::error!("Amplitude analysis failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Convert to response
    Ok(Json(AmplitudeAnalysisResponse {
        file_path: request.file_path,
        peak_rms: result.peak_rms,
        lead_in_duration: result.lead_in_duration,
        lead_out_duration: result.lead_out_duration,
        quick_ramp_up: result.quick_ramp_up,
        quick_ramp_down: result.quick_ramp_down,
        rms_profile: result.rms_profile.unwrap_or_default()
            .into_iter()
            .map(|v| v as f64)
            .collect(),
        analyzed_at: Utc::now(),
    }))
}
```

**Tasks:**
1. Replace stub implementation
2. Add error handling
3. Add integration test with real audio file
4. Update OpenAPI documentation

**Acceptance Criteria:**
- ✅ Real amplitude analysis working
- ✅ API returns actual RMS profile data
- ✅ Integration test passes
- ✅ Performance acceptable (<1s for 5min audio)

#### 4.2: Complete Metadata Fusion (4-6h) - MEDIUM

**Current:** Only ID3 metadata fused (line 269)
**Missing:** MusicBrainz metadata integration

**Implementation:**
```rust
// wkmp-ai/src/import_v2/song_workflow_engine.rs

async fn fuse_metadata(
    &self,
    id3_data: ExtractorResult<MetadataBundle>,
    mbid_candidates: Vec<ExtractorResult<Vec<MBIDCandidate>>>,
) -> ImportResult<FusedMetadata> {
    // Existing: Create MetadataExtraction from ID3
    let mut extractions = vec![
        MetadataExtraction {
            title: id3_data.data.title.clone(),
            artist: id3_data.data.artist.clone(),
            album: id3_data.data.album.clone(),
            source: ExtractionSource::ID3,
            confidence: id3_data.confidence,
        }
    ];

    // NEW: Add MusicBrainz metadata (if MBID resolved)
    if let Some(best_candidate) = mbid_candidates
        .iter()
        .flat_map(|c| &c.data)
        .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
    {
        if let Some(mb_client) = &self.musicbrainz_client {
            match mb_client.lookup(best_candidate.mbid).await {
                Ok(mb_data) => {
                    extractions.push(MetadataExtraction {
                        title: mb_data.data.title.clone(),
                        artist: mb_data.data.artist.clone(),
                        album: mb_data.data.album.clone(),
                        source: ExtractionSource::MusicBrainz,
                        confidence: mb_data.confidence,
                    });
                }
                Err(e) => {
                    tracing::warn!("MusicBrainz lookup failed (non-fatal): {}", e);
                }
            }
        }
    }

    // Fuse all sources
    let fused = self.metadata_fuser.fuse(extractions)?;
    Ok(fused)
}
```

**Acceptance Criteria:**
- ✅ MusicBrainz metadata included in fusion
- ✅ Error isolation maintained
- ✅ Tests verify multi-source fusion
- ✅ Confidence scores properly weighted

#### 4.3: Complete Flavor Synthesis Integration (2-4h) - MEDIUM

**Current:** FlavorSynthesizer marked dead_code, not used
**Missing:** Conversion from ExtractorResult<MusicalFlavor> to FusedFlavor

**Implementation:**
```rust
// wkmp-ai/src/import_v2/song_workflow_engine.rs

async fn synthesize_flavor(
    &self,
    audio_flavor: ExtractorResult<MusicalFlavor>,
    // Future: Add AcousticBrainz flavor when available
) -> ImportResult<FusedFlavor> {
    // Convert ExtractorResult<MusicalFlavor> to FlavorExtraction
    let flavor_extraction = FlavorExtraction {
        characteristics: audio_flavor.data.characteristics.clone(),
        source: audio_flavor.source,
        confidence: audio_flavor.confidence,
    };

    // Synthesize (currently just passes through, but extensible for multi-source)
    let fused = self.flavor_synthesizer.synthesize(vec![flavor_extraction])?;
    Ok(fused)
}
```

**Acceptance Criteria:**
- ✅ FlavorSynthesizer integrated
- ✅ No dead_code markers
- ✅ Tests pass

**Sprint 4 Milestone:** ✅ All major features complete and integrated

---

### Sprint 5: Code Quality & Documentation (8-12h) - MEDIUM

**Objective:** Fix remaining technical debt, improve documentation

#### 5.1: Fix Error Handling in Tests (4-6h) - MEDIUM

**Issue:** 155 instances of .unwrap()/.expect() in test code
**Impact:** Poor error messages when tests fail

**Implementation Pattern:**
```rust
// Before:
#[tokio::test]
async fn test_import_workflow() {
    let db = create_test_db().await.unwrap();
    let result = import_file(&db, "test.wav").await.unwrap();
    assert_eq!(result.successes, 1);
}

// After:
#[tokio::test]
async fn test_import_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let db = create_test_db().await?;
    let result = import_file(&db, "test.wav").await?;
    assert_eq!(result.successes, 1);
    Ok(())
}
```

**Scope:** Convert high-value tests (integration, system tests)
**Skip:** Low-value unit tests can remain with .unwrap()

**Acceptance Criteria:**
- ✅ All integration/system tests use ? operator
- ✅ Better error messages on failure
- ✅ Tests still pass

#### 5.2: Fix Compiler Warnings (1h) - LOW

**Issue:** 8 warnings (unused imports, unused mut, etc.)
**Solution:** Run `cargo fix --lib -p wkmp-ai --tests`

**Acceptance Criteria:**
- ✅ Zero compiler warnings
- ✅ All tests pass

#### 5.3: Add Module Documentation (3-5h) - MEDIUM

**Missing Documentation:**
- `import_v2/mod.rs` - Architecture overview
- `import_v2/tier1/mod.rs` - Extractor documentation
- `import_v2/tier2/mod.rs` - Fusion strategy
- `import_v2/tier3/mod.rs` - Validation criteria
- `services/amplitude_analyzer.rs` - Usage examples

**Template:**
```rust
//! # Module Name
//!
//! **Purpose:** Brief description
//!
//! **Architecture:** How it fits in the system
//!
//! **Usage:**
//! ```rust,no_run
//! // Example code
//! ```
//!
//! **Related:** Links to other modules, specs, docs
```

**Acceptance Criteria:**
- ✅ All major modules documented
- ✅ Examples compile (no_run for async)
- ✅ cargo doc generates clean output

#### 5.4: Document AcousticBrainz Deprecation (1h) - MEDIUM

**Issue:** AcousticBrainz ceased operations 2022, code should be marked legacy

**Tasks:**
1. Add deprecation notice to `services/acousticbrainz_client.rs`
2. Update architecture docs to note read-only status
3. Document Essentia migration path (future)

**Acceptance Criteria:**
- ✅ Clear deprecation warnings
- ✅ Users understand limitations

**Sprint 5 Milestone:** ✅ Professional code quality, comprehensive documentation

---

### Sprint 6: Testing & Validation (8-12h) - MEDIUM

**Objective:** Ensure migration is complete and stable

#### 6.1: Integration Test Suite (4-6h)

**New Tests Needed:**
- End-to-end import via new API routes
- MusicBrainz metadata fusion
- AcoustID integration (with mock server)
- Amplitude analysis API
- Error isolation with new workflow engine

**Acceptance Criteria:**
- ✅ 10+ new integration tests
- ✅ All tests pass
- ✅ Coverage >80% for import_v2

#### 6.2: Performance Validation (2-3h)

**Tests:**
- Benchmark 10-song import
- Memory profiling (check for leaks)
- SSE event throughput

**Acceptance Criteria:**
- ✅ <2min per song (REQ-AI-NF-011)
- ✅ No memory leaks
- ✅ SSE events delivered <100ms

#### 6.3: Manual Testing (2-3h)

**Scenarios:**
- Import single-song file
- Import multi-song file
- Import with missing ID3 tags
- Import with invalid audio
- UI workflow end-to-end

**Acceptance Criteria:**
- ✅ All scenarios work correctly
- ✅ Error messages user-friendly
- ✅ UI responsive

**Sprint 6 Milestone:** ✅ System thoroughly tested and validated

---

## Success Metrics

### Completion Criteria

**Critical (Must Complete):**
- ✅ Dual implementation pattern resolved (single import_v2 system)
- ✅ All API routes use new SongWorkflowEngine
- ✅ Old WorkflowOrchestrator deleted
- ✅ MusicBrainz/AcoustID clients integrated and functional
- ✅ Zero #[allow(dead_code)] in import_v2
- ✅ All integration tests pass

**High Priority (Should Complete):**
- ✅ Amplitude analysis API functional
- ✅ Metadata fusion includes MusicBrainz
- ✅ Flavor synthesis integrated
- ✅ Legacy client implementations removed
- ✅ Comprehensive documentation

**Medium Priority (Nice to Have):**
- ✅ Error handling improved in tests
- ✅ Compiler warnings fixed
- ✅ Performance validated
- ✅ Manual testing complete

### Quality Gates

**Per Sprint:**
- All tests pass (no regressions)
- Zero compilation errors
- Code review completed
- Documentation updated

**Final Acceptance:**
- Technical debt reduced by 70% (critical/high items)
- Single import architecture (import_v2 only)
- API routes fully migrated
- Production-ready for deployment

---

## Risk Management

### High Risks

**Risk: API migration breaks existing functionality**
- Mitigation: Comprehensive integration tests before migration
- Mitigation: Feature flags for gradual rollout
- Mitigation: Parallel testing (old vs new system)

**Risk: MusicBrainz/AcoustID integration reveals API issues**
- Mitigation: Mock servers for testing
- Mitigation: Error isolation already in place
- Mitigation: Graceful degradation if APIs unavailable

**Risk: Performance regression with new system**
- Mitigation: Benchmark before and after migration
- Mitigation: Profile critical paths
- Mitigation: Optimize hot spots if needed

### Medium Risks

**Risk: Legacy code removal breaks edge cases**
- Mitigation: Thorough test coverage
- Mitigation: Gradual removal with verification

**Risk: Documentation effort underestimated**
- Mitigation: Focus on critical modules first
- Mitigation: Use templates for consistency

---

## Dependencies & Prerequisites

### Before Starting Sprint 1:
- ✅ PLAN023 Sprints 1-5 complete (production-ready status achieved)
- ✅ All MUST and SHOULD items from PLAN023 done
- ✅ Technical debt report reviewed and approved

### Between Sprints:
- Sprint 1 → 2: import_v2 feature-complete
- Sprint 2 → 3: API routes migrated
- Sprint 3 → 4: Legacy code removed
- Sprint 4 → 5: Features complete
- Sprint 5 → 6: Documentation complete

---

## Timeline Estimate

### Optimistic (60 hours / 7.5 days):
- Sprint 1: 16h
- Sprint 2: 12h
- Sprint 3: 8h
- Sprint 4: 12h
- Sprint 5: 8h
- Sprint 6: 8h

### Realistic (75 hours / 9.4 days):
- Sprint 1: 20h
- Sprint 2: 14h
- Sprint 3: 10h
- Sprint 4: 14h
- Sprint 5: 10h
- Sprint 6: 10h

### Pessimistic (90 hours / 11.3 days):
- Sprint 1: 24h
- Sprint 2: 16h
- Sprint 3: 12h
- Sprint 4: 16h
- Sprint 5: 12h
- Sprint 6: 12h

**Recommended:** Plan for **75-80 hours** to allow buffer for unknowns.

---

## Rollout Strategy

### Phase 1: Development (Sprints 1-3)
- Complete import_v2 integration
- Migrate API routes with feature flag
- Remove legacy code (behind flag)

### Phase 2: Testing (Sprints 4-5)
- Enable new system by default
- Complete feature set
- Fix bugs, improve quality

### Phase 3: Validation (Sprint 6)
- Performance testing
- Manual QA
- Production readiness review

### Phase 4: Deployment
- Remove feature flags
- Deploy to production
- Monitor for issues

---

## Post-Migration Cleanup

### Immediate (Within 1 week):
- Remove feature flags
- Delete commented-out old code
- Archive old system documentation

### Short-term (Within 1 month):
- Implement remaining COULD items from technical debt
- Add advanced features (segment editor UI, waveform rendering)
- Optimize performance hot spots

### Long-term (Within 3 months):
- Implement Essentia integration (if decided)
- Add advanced boundary detection
- Implement conflict resolution UI

---

## Appendix A: File Deletion Checklist

### Sprint 3 Deletions:

**Primary Target:**
- [ ] `src/services/workflow_orchestrator.rs` (1451 lines)

**Duplicate Clients:**
- [ ] `src/services/acoustid_client.rs` (replaced by import_v2/tier1/acoustid_client.rs)
- [ ] `src/services/musicbrainz_client.rs` (replaced by import_v2/tier1/musicbrainz_client.rs)

**Test Files (if only test old system):**
- [ ] Any integration tests that exclusively test WorkflowOrchestrator

**Documentation:**
- [ ] Update references to old system in docs/
- [ ] Update IMPL* documents to reflect new architecture

---

## Appendix B: Integration Points

### API Routes (Sprint 2):
- `POST /import/start` - Start import workflow
- `GET /import/events/:id` - SSE event stream
- `GET /import/status/:id` - Query import status
- `POST /import/cancel/:id` - Cancel import
- `POST /analyze/amplitude` - Amplitude analysis

### Database Schema:
- `import_sessions` - Session tracking
- `import_results` - Per-passage results
- `songs` - Imported songs
- `passages` - Passage metadata
- `audio_fingerprints` - Chromaprint data

### Configuration:
- `WKMP_ACOUSTID_API_KEY` - AcoustID API key
- `WKMP_MUSICBRAINZ_TOKEN` - MusicBrainz auth token (optional)
- `WKMP_ROOT_FOLDER` - Root data folder

---

**Plan Status:** Draft
**Created:** 2025-01-09
**Next Review:** After Sprint 1 completion
**Owner:** Technical Lead
