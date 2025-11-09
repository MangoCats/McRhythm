# PLAN023 Focused Work Plan - P1 & P2 Items

**Date:** 2025-01-09
**Scope:** Complete all P1 (High Priority) and P2 (Medium Priority) technical debt
**Target:** Production-ready with real audio file processing + comprehensive testing
**Estimated Total Effort:** 40-60 hours

---

## Executive Summary

This work plan provides a **strategic roadmap** for completing PLAN023 implementation to full production readiness. It organizes 22 items (5 P1 + 17 P2) into **6 focused sprints** using parallel tracks and dependency management.

**Key Strategy:**
- **Parallel Development:** Multiple independent items worked simultaneously
- **Dependency-First:** Critical path items prioritized to unblock others
- **Incremental Integration:** Test each component before moving to next
- **Quality Gates:** Each sprint ends with verification milestone

**Total Estimate:** 40-60 hours (5-8 full development days)

---

## Sprint Overview

| Sprint | Focus | Duration | Items | Dependencies |
|--------|-------|----------|-------|--------------|
| **1** | Foundation (Parallel) | 8-10h | P1-2, P1-4 | None - can start immediately |
| **2** | Audio Integration | 4-6h | P1-1 | Sprint 1 (P1-2) |
| **3** | Workflow Core | 12-16h | P1-5 | Sprint 1, 2 (all Tier 1) |
| **4** | Testing & Validation | 10-15h | P2-2, P2-5, P2-6 | Sprint 3 |
| **5** | Code Quality | 3-6h | P3 items | Any time (parallel) |
| **6** | Optional Enhancements | 16-24h | P2-4 (Essentia) | Sprint 3 |

**Critical Path:** Sprint 1 → Sprint 2 → Sprint 3 → Sprint 4
**Parallel Tracks:** Sprint 5 can run anytime, Sprint 6 is optional

---

## Detailed Sprint Plans

### Sprint 1: Foundation - Audio & Metadata Extraction (8-10 hours)

**Objective:** Complete core Tier 1 extractors (resampling + ID3) to unblock downstream work

**Items:**
- ✅ P1-3: Genre mapper (COMPLETE)
- ⚠️ **P1-2:** Implement audio resampling with rubato (3-4h)
- ⚠️ **P1-4:** Complete ID3 extractor with lofty (4-5h)

**Why These First:**
- P1-2 and P1-4 are **independent** - can be developed in parallel
- P1-2 blocks P1-1 (chromaprint needs 44.1 kHz input)
- P1-4 provides metadata for integration testing
- Both are well-scoped with clear APIs

**Parallel Track A: Audio Resampling (P1-2) - 3-4 hours**

**Steps:**
1. **Add rubato dependency** (15 min)
   ```toml
   # Cargo.toml
   rubato = "0.15"
   ```

2. **Implement resampling in AudioLoader** (2-2.5h)
   ```rust
   // wkmp-ai/src/import_v2/tier1/audio_loader.rs

   use rubato::{
       Resampler, SincFixedIn, InterpolationType,
       InterpolationParameters, WindowFunction
   };

   impl AudioLoader {
       fn resample_if_needed(&self, samples: Vec<f32>, native_rate: u32) -> Result<Vec<f32>> {
           if native_rate == self.target_sample_rate {
               return Ok(samples);
           }

           let params = InterpolationParameters {
               sinc_len: 256,
               f_cutoff: 0.95,
               interpolation: InterpolationType::Linear,
               oversampling_factor: 256,
               window: WindowFunction::BlackmanHarris2,
           };

           let mut resampler = SincFixedIn::<f32>::new(
               self.target_sample_rate as f64 / native_rate as f64,
               2.0, // max resample ratio
               params,
               samples.len() / 2, // frame count
               2, // stereo channels
           )?;

           // De-interleave stereo
           let left: Vec<f32> = samples.iter().step_by(2).copied().collect();
           let right: Vec<f32> = samples.iter().skip(1).step_by(2).copied().collect();

           // Resample
           let output = resampler.process(&[left, right], None)?;

           // Re-interleave
           let resampled: Vec<f32> = output[0]
               .iter()
               .zip(&output[1])
               .flat_map(|(l, r)| vec![*l, *r])
               .collect();

           Ok(resampled)
       }
   }
   ```

3. **Add unit tests** (1-1.5h)
   ```rust
   #[test]
   fn test_no_resampling_44100() {
       // 44.1 kHz → 44.1 kHz (pass through)
   }

   #[test]
   fn test_resample_48000_to_44100() {
       // 48 kHz → 44.1 kHz (common CD rip scenario)
   }

   #[test]
   fn test_resample_96000_to_44100() {
       // 96 kHz → 44.1 kHz (high-res audio)
   }
   ```

**Acceptance Criteria:**
- ✅ All native sample rates converted to 44.1 kHz
- ✅ Quality preserved (no aliasing/artifacts)
- ✅ 3 unit tests passing
- ✅ AudioSegment.sample_rate always 44100

---

**Parallel Track B: ID3 Extraction (P1-4) - 4-5 hours**

**Steps:**
1. **Research lofty API** (30 min)
   - Read lofty docs and examples
   - Identify TXXX frame access pattern
   - Check MBID extraction methods

2. **Implement ID3Extractor** (2.5-3h)
   ```rust
   // wkmp-ai/src/import_v2/tier1/id3_extractor.rs

   use lofty::{Accessor, AudioFile, Probe, Tag, TaggedFileExt};
   use lofty::id3::v2::{Id3v2Tag, FrameId};

   impl ID3Extractor {
       pub fn extract(&self, file_path: &Path) -> ImportResult<ID3Metadata> {
           // Probe file
           let tagged_file = Probe::open(file_path)
               .context("Failed to open audio file")?
               .read()
               .context("Failed to read audio file tags")?;

           // Get primary tag (ID3v2 preferred)
           let tag = match tagged_file.primary_tag()
               .or_else(|| tagged_file.first_tag())
           {
               Some(t) => t,
               None => return Ok(Self::placeholder_metadata()),
           };

           // Extract standard fields
           let title = tag.title().map(|s| s.to_string());
           let artist = tag.artist().map(|s| s.to_string());
           let album = tag.album().map(|s| s.to_string());
           let year = tag.year();
           let genre = tag.genre().map(|s| s.to_string());

           // Extract MBID from TXXX frame
           let recording_mbid = if let Some(id3v2_tag) =
               tagged_file.tag(lofty::TagType::Id3v2)
           {
               Self::extract_mbid_from_id3v2(id3v2_tag)
           } else {
               None
           };

           // Calculate confidence based on completeness
           let field_count = [
               &title, &artist, &album,
               &year.as_ref().map(|_| &()), &genre
           ].iter().filter(|f| f.is_some()).count();

           let confidence = (field_count as f64 / 5.0) * 0.5; // Max 0.5

           Ok(ID3Metadata {
               recording_mbid,
               title,
               artist,
               album,
               year,
               genre,
               confidence,
           })
       }

       fn extract_mbid_from_id3v2(tag: &dyn Tag) -> Option<Uuid> {
           // Look for TXXX:MusicBrainz Recording Id
           // Implementation depends on lofty frame access API
           None // Placeholder if complex
       }

       fn placeholder_metadata() -> ID3Metadata {
           ID3Metadata {
               recording_mbid: None,
               title: None,
               artist: None,
               album: None,
               year: None,
               genre: None,
               confidence: 0.0,
           }
       }
   }
   ```

3. **Add unit tests** (1-1.5h)
   ```rust
   #[test]
   fn test_extract_from_valid_id3() {
       // Requires test audio file with ID3 tags
   }

   #[test]
   fn test_extract_from_no_tags() {
       // Should return placeholder, not error
   }

   #[test]
   fn test_confidence_calculation() {
       // Verify confidence matches completeness
   }
   ```

**Acceptance Criteria:**
- ✅ Extracts title, artist, album, year, genre
- ✅ Handles missing tags gracefully
- ✅ Confidence reflects completeness
- ✅ 3+ unit tests passing
- ✅ MBID extraction (best effort, OK if skipped initially)

---

**Sprint 1 Milestone:** ✅ Audio resampling + ID3 extraction working, all tests passing

**Estimated Time:** 8-10 hours total (can be done in 1-2 days if working in parallel)

---

### Sprint 2: Audio Fingerprinting (4-6 hours)

**Objective:** Complete chromaprint integration for AcoustID lookups

**Dependencies:** Sprint 1 (P1-2 resampling must be complete)

**Items:**
- ⚠️ **P1-1:** Complete Chromaprint integration (4-6h)

**Why Now:**
- Depends on P1-2 (needs 44.1 kHz input)
- Required for P1-5 (workflow integration)
- Self-contained, can be developed and tested independently

**Steps:**

1. **Research chromaprint-rust API** (45 min)
   - Review chromaprint-rust examples on GitHub
   - Check chromaprint-sys-next documentation
   - Verify Windows static linking configuration

2. **Implement ChromaprintAnalyzer** (2.5-3h)
   ```rust
   // wkmp-ai/src/import_v2/tier1/chromaprint_analyzer.rs

   use chromaprint_rust::Chromaprint;
   use base64::{Engine as _, engine::general_purpose};

   impl ChromaprintAnalyzer {
       pub fn analyze(&self, audio: &AudioSegment) -> ImportResult<ChromaprintResult> {
           // Verify sample rate (must be 44.1 kHz)
           if audio.sample_rate != 44100 {
               return Err(ImportError::ExtractionFailed(
                   format!("Chromaprint requires 44.1 kHz, got {} Hz", audio.sample_rate)
               ));
           }

           // Create chromaprint context
           let mut chromaprint = Chromaprint::new()
               .map_err(|e| ImportError::ExtractionFailed(
                   format!("Chromaprint init failed: {}", e)
               ))?;

           // Start fingerprinting (44.1 kHz, stereo)
           chromaprint.start(44100, 2)
               .map_err(|e| ImportError::ExtractionFailed(
                   format!("Chromaprint start failed: {}", e)
               ))?;

           // Convert f32 samples to i16 for chromaprint
           let samples_i16: Vec<i16> = audio.samples
               .iter()
               .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
               .collect();

           // Feed audio data
           chromaprint.feed(&samples_i16)
               .map_err(|e| ImportError::ExtractionFailed(
                   format!("Chromaprint feed failed: {}", e)
               ))?;

           // Finish and get fingerprint
           chromaprint.finish()
               .map_err(|e| ImportError::ExtractionFailed(
                   format!("Chromaprint finish failed: {}", e)
               ))?;

           // Get compressed fingerprint (base64-encoded)
           let raw_fingerprint = chromaprint.get_fingerprint()
               .map_err(|e| ImportError::ExtractionFailed(
                   format!("Get fingerprint failed: {}", e)
               ))?;

           let fingerprint_base64 = general_purpose::STANDARD.encode(&raw_fingerprint);

           // Calculate duration in seconds
           let duration_seconds = (audio.samples.len() / 2) as f64
               / audio.sample_rate as f64;

           Ok(ChromaprintResult {
               fingerprint: fingerprint_base64,
               duration_seconds,
               algorithm_version: chromaprint.get_algorithm() as u32,
           })
       }
   }
   ```

3. **Add unit tests** (1-1.5h)
   ```rust
   #[test]
   fn test_chromaprint_with_silence() {
       // Generate silent audio segment, verify fingerprint generation
   }

   #[test]
   fn test_chromaprint_with_sine_wave() {
       // Generate test tone, verify fingerprint is consistent
   }

   #[test]
   fn test_sample_rate_validation() {
       // Should reject non-44.1 kHz input
   }
   ```

**Acceptance Criteria:**
- ✅ Generates valid base64-encoded fingerprint
- ✅ Works with 44.1 kHz stereo audio
- ✅ Duration calculated correctly
- ✅ 3+ unit tests passing
- ✅ No crashes on Windows/Linux/Mac

**Sprint 2 Milestone:** ✅ Chromaprint working, ready for AcoustID integration

**Estimated Time:** 4-6 hours

---

### Sprint 3: Workflow Integration (12-16 hours)

**Objective:** Connect all Tier 1 extractors and complete end-to-end workflow

**Dependencies:** Sprint 1 (P1-2, P1-4) + Sprint 2 (P1-1)

**Items:**
- ⚠️ **P1-5:** Integrate SongWorkflowEngine with real extractors (12-16h)

**Why Now:**
- All Tier 1 extractors complete
- Critical path item blocking production
- Most complex integration task

**Steps:**

**Phase 1: API Client Initialization (2h)**

```rust
// wkmp-ai/src/import_v2/song_workflow_engine.rs

impl SongWorkflowEngine {
    pub fn new() -> Self {
        // Load configuration
        let config = Self::load_config_or_defaults();

        // Initialize MusicBrainz client
        let musicbrainz_client = Some(MusicBrainzClient::new(
            config.musicbrainz_api_url
                .unwrap_or_else(|| "https://musicbrainz.org/ws/2".to_string()),
            config.user_agent
                .unwrap_or_else(|| "WKMP/0.1.0".to_string()),
        ));

        // Initialize AcoustID client (optional - requires API key)
        let acoustid_client = config.acoustid_api_key.map(|key| {
            AcoustIDClient::new(key)
        });

        Self {
            id3_extractor: ID3Extractor::new(),
            chromaprint_analyzer: ChromaprintAnalyzer::new(),
            audio_loader: AudioLoader::new(44100),
            musicbrainz_client,
            acoustid_client,
            audio_features: AudioFeatureExtractor::new(),
            // ... rest
        }
    }

    fn load_config_or_defaults() -> WorkflowConfig {
        // Try to load from environment variables or config file
        WorkflowConfig {
            musicbrainz_api_url: std::env::var("MUSICBRAINZ_API_URL").ok(),
            acoustid_api_key: std::env::var("ACOUSTID_API_KEY").ok(),
            user_agent: std::env::var("WKMP_USER_AGENT").ok(),
        }
    }
}
```

**Phase 2: Audio Segment Extraction (1-2h)**

```rust
// In process_passage()

// Extract audio segment for this passage
let audio_segment = self.audio_loader.load_segment(
    audio_file_path,
    boundary.start_ticks,
    boundary.end_ticks,
).context("Failed to load audio segment")?;

tracing::debug!(
    "Extracted audio segment: {} samples @ {} Hz",
    audio_segment.samples.len(),
    audio_segment.sample_rate
);
```

**Phase 3: Tier 1 Parallel Extraction (3-4h)**

```rust
// Tier 1: Parallel extraction from all sources
tracing::info!("Tier 1: Running parallel extraction");

let (id3_result, chromaprint_result, audio_features_result) = tokio::join!(
    // ID3 tags from file
    async {
        match self.id3_extractor.extract(audio_file_path) {
            Ok(data) => Some(data),
            Err(e) => {
                tracing::warn!("ID3 extraction failed: {}", e);
                None
            }
        }
    },

    // Chromaprint fingerprint from audio
    async {
        match self.chromaprint_analyzer.analyze(&audio_segment) {
            Ok(data) => Some(data),
            Err(e) => {
                tracing::warn!("Chromaprint failed: {}", e);
                None
            }
        }
    },

    // Audio features from signal
    async {
        match self.audio_features.extract(&audio_segment) {
            Ok(data) => Some(data),
            Err(e) => {
                tracing::warn!("Audio features failed: {}", e);
                None
            }
        }
    },
);

// Query AcoustID if we have fingerprint
let acoustid_result = if let (Some(ref chromaprint), Some(ref client)) =
    (&chromaprint_result, &self.acoustid_client)
{
    match client.query(&chromaprint.fingerprint, chromaprint.duration_seconds).await {
        Ok(data) => Some(data),
        Err(e) => {
            tracing::warn!("AcoustID query failed: {}", e);
            None
        }
    }
} else {
    tracing::debug!("Skipping AcoustID (no fingerprint or no API key)");
    None
};

// Determine MBID for MusicBrainz query
let mb_mbid = acoustid_result.as_ref()
    .and_then(|r| r.recording_mbid)
    .or_else(|| id3_result.as_ref().and_then(|r| r.recording_mbid));

// Query MusicBrainz if we have MBID
let mb_result = if let (Some(mbid), Some(ref client)) = (mb_mbid, &self.musicbrainz_client) {
    match client.query_recording(&mbid).await {
        Ok(data) => Some(data),
        Err(e) => {
            tracing::warn!("MusicBrainz query failed: {}", e);
            None
        }
    }
} else {
    tracing::debug!("Skipping MusicBrainz (no MBID)");
    None
};
```

**Phase 4: Tier 2 Fusion (3-4h)**

```rust
// Tier 2: Fusion
tracing::info!("Tier 2: Running fusion");

// Build MBID candidate lists
let mut mbid_candidate_lists = Vec::new();

if let Some(ref acoustid) = acoustid_result {
    if let Some(mbid) = acoustid.recording_mbid {
        mbid_candidate_lists.push(ExtractorResult {
            data: vec![MBIDCandidate {
                mbid,
                confidence: acoustid.confidence,
                sources: vec![ExtractionSource::AcoustID],
            }],
            confidence: acoustid.confidence,
            source: ExtractionSource::AcoustID,
        });
    }
}

if let Some(ref mb) = mb_result {
    mbid_candidate_lists.push(ExtractorResult {
        data: vec![MBIDCandidate {
            mbid: mb.mbid,
            confidence: 0.9,
            sources: vec![ExtractionSource::MusicBrainz],
        }],
        confidence: 0.9,
        source: ExtractionSource::MusicBrainz,
    });
}

if let Some(ref id3) = id3_result {
    if let Some(mbid) = id3.recording_mbid {
        mbid_candidate_lists.push(ExtractorResult {
            data: vec![MBIDCandidate {
                mbid,
                confidence: id3.confidence,
                sources: vec![ExtractionSource::ID3Metadata],
            }],
            confidence: id3.confidence,
            source: ExtractionSource::ID3Metadata,
        });
    }
}

// Resolve identity
let resolved_identity = self.identity_resolver.resolve(mbid_candidate_lists)?;

// Build metadata bundles
let mut metadata_bundles = Vec::new();

if let Some(ref id3) = id3_result {
    metadata_bundles.push(ExtractorResult {
        data: Self::id3_to_metadata_bundle(id3),
        confidence: id3.confidence,
        source: ExtractionSource::ID3Metadata,
    });
}

if let Some(ref mb) = mb_result {
    metadata_bundles.push(ExtractorResult {
        data: Self::mb_to_metadata_bundle(mb),
        confidence: 0.9,
        source: ExtractionSource::MusicBrainz,
    });
}

// Fuse metadata
let fused_metadata = self.metadata_fuser.fuse(metadata_bundles)?;

// Build flavor extractions
let mut flavor_extractions = Vec::new();

if let Some(ref features) = audio_features_result {
    flavor_extractions.push(FlavorExtraction {
        flavor: features.flavor.clone(),
        confidence: features.confidence,
        source: ExtractionSource::AudioDerived,
    });
}

if let Some(ref genre) = id3_result.as_ref().and_then(|id3| id3.genre.as_ref()) {
    if let Some(genre_flavor) = self.genre_mapper.map_genre(genre) {
        flavor_extractions.push(FlavorExtraction {
            flavor: genre_flavor.data,
            confidence: genre_flavor.confidence,
            source: genre_flavor.source,
        });
    }
}

// Synthesize flavor
let synthesized_flavor = self.flavor_synthesizer.synthesize(flavor_extractions)?;
```

**Phase 5: Tier 3 Validation & Database (2-3h)**

```rust
// Tier 3: Validation (already working from Session 1)
let validation = self.validate_passage(
    &resolved_identity,
    &fused_metadata,
    &synthesized_flavor,
)?;

// Build ProcessedPassage
let processed = ProcessedPassage {
    boundary: boundary.clone(),
    identity: resolved_identity,
    metadata: fused_metadata,
    flavor: synthesized_flavor,
    validation,
    import_duration_ms: start_time.elapsed().as_millis() as u64,
    import_timestamp: chrono::Utc::now().to_rfc3339(),
    import_version: "PLAN023-v1.0.0".to_string(),
};

// Save to database
let passage_id = self.db_repository.save_processed_passage(
    &file_id,
    &processed,
    &import_session_id,
).await?;
```

**Phase 6: Integration Tests (2-3h)**

```rust
// tests/integration_workflow.rs

#[tokio::test]
async fn test_end_to_end_with_real_file() {
    // Requires test audio file
    let test_file = Path::new("tests/fixtures/test_song.flac");

    if !test_file.exists() {
        eprintln!("Skipping test: test audio file not found");
        return;
    }

    let engine = SongWorkflowEngine::new();

    let boundaries = vec![PassageBoundary {
        start_ticks: 0,
        end_ticks: 28_224_000 * 30, // 30 seconds
        confidence: 1.0,
        source: ExtractionSource::ManualAnnotation,
    }];

    let results = engine.process_file(test_file, boundaries).await.unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());

    let processed = results[0].as_ref().unwrap();
    assert!(processed.metadata.title.is_some());
    // MBID may be None if no AcoustID key configured
    assert!(!processed.flavor.flavor.characteristics.is_empty());
}
```

**Acceptance Criteria:**
- ✅ All API clients initialized from config/env
- ✅ Audio segment extraction working
- ✅ All Tier 1 extractors connected and running in parallel
- ✅ Multi-source identity resolution working
- ✅ Multi-source metadata fusion working
- ✅ Multi-source flavor synthesis working
- ✅ Tier 3 validation working
- ✅ Database persistence working
- ✅ SSE events emitted correctly
- ✅ Error isolation working (one extractor failure doesn't abort)
- ✅ End-to-end integration test passing with real audio

**Sprint 3 Milestone:** ✅ Complete end-to-end workflow functional

**Estimated Time:** 12-16 hours (2-3 full days)

---

### Sprint 4: Comprehensive Testing (10-15 hours)

**Objective:** Add system tests, error isolation tests, and validation tests

**Dependencies:** Sprint 3 (P1-5 must be complete)

**Items:**
- ⚠️ **P2-2:** System tests (4 tests, 6-8h)
- ⚠️ **P2-5:** Error isolation tests (3-4h)
- ⚠️ **P2-6:** Remaining test TODOs (3-4h)

**System Tests (P2-2) - 6-8 hours:**

**TC-S-010-01: Complete File Import Workflow (2h)**
```rust
#[tokio::test]
async fn tc_s_010_01_complete_file_import() {
    // End-to-end test with real audio file
    // Verifies: ID3 extraction → Chromaprint → AcoustID → MusicBrainz → DB
}
```

**TC-S-012-01: Multi-Song File Processing (2h)**
```rust
#[tokio::test]
async fn tc_s_012_01_multi_song_processing() {
    // Process file with 3 songs, verify each processed independently
    // Verifies: Per-song isolation, sequential processing
}
```

**TC-S-071-01: SSE Event Streaming (1-2h)**
```rust
#[tokio::test]
async fn tc_s_071_01_sse_event_streaming() {
    // Spawn HTTP server, connect SSE client, verify events received
    // Verifies: Real-time progress updates
}
```

**TC-S-NF-011-01: Performance Benchmarks (2h)**
```rust
#[tokio::test]
async fn tc_s_nf_011_01_performance_benchmark() {
    // Process 5 test songs, measure time per song
    // Verifies: <2 minutes per song requirement
}
```

**Error Isolation Tests (P2-5) - 3-4 hours:**

```rust
#[tokio::test]
async fn test_id3_failure_doesnt_abort() {
    // Corrupt ID3 tags, verify workflow continues with other sources
}

#[tokio::test]
async fn test_acoustid_failure_doesnt_abort() {
    // Invalid API key, verify workflow continues without AcoustID
}

#[tokio::test]
async fn test_passage_failure_doesnt_abort_import() {
    // One passage has corrupt audio, verify other passages processed
}
```

**Remaining TODOs (P2-6) - 3-4 hours:**

- ConsistencyChecker validation improvements
- Validation threshold tests
- Integration test expansions
- API response handling edge cases

**Sprint 4 Milestone:** ✅ Comprehensive test suite complete

**Estimated Time:** 10-15 hours (1.5-2 full days)

---

### Sprint 5: Code Quality (3-6 hours) - PARALLEL TRACK

**Objective:** Clean up compiler warnings and clippy issues

**Dependencies:** None - can be done anytime in parallel

**Items:**
- ⚠️ **P3-1:** Fix compiler warnings (1-2h)
- ⚠️ **P3-2:** Fix clippy warnings (2-3h)
- ⚠️ **P3-3:** Fix doctests (1h)

**Why Parallel:**
- Independent of feature development
- Can be done while waiting for tests to run
- Low risk, high maintainability benefit

**Quick Fixes:**

1. **Unused Imports (30 min)**
   ```bash
   cargo fix --lib -p wkmp-ai --allow-dirty
   ```

2. **Unused Variables (30 min)**
   ```rust
   // Prefix with underscore
   let Some(ref _selected_title) = metadata.title else { ... }
   ```

3. **Dead Code (30 min)**
   - Remove unused functions or mark with #[allow(dead_code)] if needed later

4. **Clippy Issues (2-3h)**
   ```bash
   cargo clippy --fix -p wkmp-ai --allow-dirty
   # Review and fix remaining manual issues
   ```

**Sprint 5 Milestone:** ✅ Zero warnings, clean clippy run

**Estimated Time:** 3-6 hours (can be spread across other sprints)

---

### Sprint 6: Optional Enhancements (16-24 hours) - OPTIONAL

**Objective:** Add Essentia integration for high-quality flavor extraction

**Dependencies:** Sprint 3 (workflow must be working)

**Items:**
- ⚠️ **P2-4:** Essentia integration (16-24h)

**Why Optional:**
- System is production-ready without it
- Complex integration with C++ library
- Significant effort for incremental improvement
- Can be added post-deployment

**If Pursuing:**

1. **Research Essentia Rust bindings** (2-3h)
2. **Implement essentia_analyzer module** (6-8h)
3. **Integrate with workflow** (2-3h)
4. **Add tests** (3-4h)
5. **Benchmark quality improvement** (3-4h)

**Sprint 6 Milestone:** ✅ Essentia flavor extraction working (optional)

**Estimated Time:** 16-24 hours (2-3 full days)

---

## Risk Management

### High-Risk Items

**P1-1 (Chromaprint):**
- **Risk:** API complexity, Windows linking
- **Mitigation:** Static linking configured, examples available
- **Fallback:** Use placeholder fingerprint, rely on ID3 MBIDs
- **Risk Level:** MEDIUM

**P1-5 (Workflow Integration):**
- **Risk:** Complex integration, many moving parts
- **Mitigation:** Incremental integration, thorough testing
- **Fallback:** None - critical path
- **Risk Level:** MEDIUM-HIGH

**P2-4 (Essentia):**
- **Risk:** C++ bindings, cross-platform builds
- **Mitigation:** Optional, can skip
- **Fallback:** Use audio features + genre mapping
- **Risk Level:** HIGH (but optional)

### Medium-Risk Items

**P1-2 (Resampling):**
- **Risk:** rubato API complexity
- **Mitigation:** Well-documented crate
- **Fallback:** Assume 44.1 kHz files
- **Risk Level:** LOW-MEDIUM

**P1-4 (ID3):**
- **Risk:** lofty API, TXXX complexity
- **Mitigation:** Mature crate
- **Fallback:** Extract basic tags only
- **Risk Level:** LOW-MEDIUM

### Risk Mitigation Strategy

1. **Parallel Development:** P1-2 and P1-4 reduce risk through redundancy
2. **Incremental Testing:** Test each component before integrating
3. **Graceful Degradation:** All extractors optional (system degrades gracefully)
4. **Fallback Plans:** Clear fallbacks for each high-risk item

---

## Quality Gates

### Sprint 1 Gate
- ✅ Audio resampling tests passing (3/3)
- ✅ ID3 extraction tests passing (3/3)
- ✅ No new compiler warnings
- ✅ Documentation updated

### Sprint 2 Gate
- ✅ Chromaprint tests passing (3/3)
- ✅ Integration with AudioLoader verified
- ✅ No crashes on target platforms

### Sprint 3 Gate
- ✅ End-to-end integration test passing
- ✅ All Tier 1 extractors connected
- ✅ Database persistence working
- ✅ Error isolation verified

### Sprint 4 Gate
- ✅ All 4 system tests passing
- ✅ Error isolation tests passing (3/3)
- ✅ Performance benchmarks meet <2min requirement
- ✅ Test coverage ≥ 170% of original requirement

### Sprint 5 Gate
- ✅ Zero compiler warnings
- ✅ Zero clippy warnings
- ✅ All doctests passing

### Sprint 6 Gate (Optional)
- ✅ Essentia tests passing
- ✅ Quality improvement verified vs audio features alone

---

## Timeline & Resource Allocation

### Optimistic Timeline (Experienced Developer, No Blockers)
- **Sprint 1:** 8 hours (1 day)
- **Sprint 2:** 4 hours (half day)
- **Sprint 3:** 12 hours (1.5 days)
- **Sprint 4:** 10 hours (1.5 days)
- **Sprint 5:** 3 hours (parallel)
- **Total:** 40 hours (5 days)

### Realistic Timeline (Includes Research, Debugging, Testing)
- **Sprint 1:** 10 hours (1.5 days)
- **Sprint 2:** 6 hours (1 day)
- **Sprint 3:** 16 hours (2 days)
- **Sprint 4:** 15 hours (2 days)
- **Sprint 5:** 6 hours (parallel)
- **Total:** 53 hours (6.5 days)

### Conservative Timeline (Includes Unknowns, Blockers, Rework)
- **Sprint 1:** 12 hours (2 days)
- **Sprint 2:** 8 hours (1 day)
- **Sprint 3:** 20 hours (2.5 days)
- **Sprint 4:** 18 hours (2.5 days)
- **Sprint 5:** 8 hours (parallel)
- **Total:** 66 hours (8 days)

**Recommended Schedule:** Plan for **60 hours (7-8 full development days)** to allow buffer for unknowns.

---

## Progress Tracking

### Current Status (Session 4)

| Item | Status | Time Spent | Remaining |
|------|--------|------------|-----------|
| P1-3 | ✅ Complete | 2h | 0h |
| P1-2 | ⚠️ Pending | 0h | 3-4h |
| P1-4 | ⚠️ Pending | 0h | 4-5h |
| P1-1 | ⚠️ Pending | 0h | 4-6h |
| P1-5 | ⚠️ Pending | 0h | 12-16h |
| P2-2 | ⚠️ Pending | 0h | 6-8h |
| P2-5 | ⚠️ Pending | 0h | 3-4h |
| P2-6 | ⚠️ Pending | 0h | 3-4h |
| P3-all | ⚠️ Pending | 0h | 3-6h |
| P2-4 | ⚠️ Optional | 0h | 16-24h |

**Progress:** 1/10 items complete (10%)
**Time Invested:** 2 hours
**Time Remaining:** 40-60 hours (without P2-4 optional)

---

## Success Metrics

### Completion Criteria

**Critical Path (Production-Ready):**
- ✅ P1-3 Complete (Genre mapper)
- ⬜ P1-2 Complete (Resampling)
- ⬜ P1-4 Complete (ID3)
- ⬜ P1-1 Complete (Chromaprint)
- ⬜ P1-5 Complete (Workflow)
- ⬜ P2-2 Complete (System tests)

**Quality Metrics:**
- All tests passing (target: 260+ tests, currently 250)
- Performance: <2 minutes per song
- Zero critical bugs
- Zero compiler/clippy warnings

**Production Readiness:**
- End-to-end workflow with real audio files ✅
- Multi-source fusion verified ✅
- Error handling comprehensive ✅
- Database persistence working ✅
- Documentation complete ✅

---

## Next Actions

**Immediate Next Steps (Start Sprint 1):**

1. **Parallel Track A - Start P1-2 (Resampling)**
   ```bash
   # Add rubato dependency
   # Implement AudioLoader::resample_if_needed()
   # Add 3 unit tests
   ```

2. **Parallel Track B - Start P1-4 (ID3)**
   ```bash
   # Research lofty API
   # Implement ID3Extractor::extract()
   # Add 3 unit tests
   ```

3. **Documentation**
   - Update progress tracking
   - Document configuration requirements (API keys)
   - Update TECHNICAL_DEBT_REPORT.md as items complete

**Recommended Approach:**
- Work Sprint 1 tracks in parallel (if 2 developers)
- Or sequential: P1-2 first (unblocks P1-1), then P1-4
- Run Sprint 5 (code quality) in parallel during Sprint 3-4 testing waits

---

**Work Plan Created:** 2025-01-09
**Next Update:** After Sprint 1 completion
**Contact:** See [P1_PROGRESS_REPORT.md](P1_PROGRESS_REPORT.md) and [TECHNICAL_DEBT_REPORT.md](TECHNICAL_DEBT_REPORT.md)
