# Implementation Plan - PLAN026: Technical Debt Resolution

**Plan Status:** Ready for Execution
**Total Effort:** 31-43 hours (Sprint 1 + Sprint 2)
**Sprints:** 2 (Sprint 3 deferred)

---

## Sprint 1: Critical Path (Week 1)

**Goal:** Unblock production use - fix boundary detection, enable segment extraction, remove stub endpoints

**Total Effort:** 12-16 hours
**Test Count:** 11 tests
**Deliverable:** Users can import multi-track albums successfully

---

### Increment 1.1: Boundary Detection Integration (REQ-TD-001)

**Effort:** 4-6 hours
**Dependencies:** None (SilenceDetector already implemented)
**Files Modified:** 1
**Tests:** 4 (2 unit, 2 integration)

#### Step 1.1.1: Read Existing SilenceDetector Implementation
**Files:** [wkmp-ai/src/services/silence_detector.rs](wkmp-ai/src/services/silence_detector.rs)
**Action:** Review API, configuration options, test coverage
**Deliverable:** Understanding of `SilenceDetector::detect()` interface

#### Step 1.1.2: Modify SessionOrchestrator Phase 3
**File:** [wkmp-ai/src/import_v2/session_orchestrator.rs](wkmp-ai/src/import_v2/session_orchestrator.rs)
**Lines:** 232-239 (replace stub)

**Current Code:**
```rust
// Strategy 1: Silence-based detection
// For now, create single passage per file spanning entire duration
let file_boundaries = vec![PassageBoundary {
    start_ticks: 0,
    end_ticks: (duration_secs * 28_224_000.0) as i64,
    confidence: 0.8,
    detection_method: BoundaryDetectionMethod::SilenceDetection,
}];
```

**New Code:**
```rust
// Phase 3: Boundary Detection - Use SilenceDetector for multi-passage detection
tracing::debug!("Phase 3: Detecting passage boundaries for file: {}", audio_file.display());

// Load audio samples for silence analysis
let audio_loader = AudioLoader::new();
let samples = audio_loader.load_file(&audio_file).await?;

// Get silence detection configuration from database
let silence_threshold_db = wkmp_common::config::get_float_setting(
    &self.db,
    "import.boundary_detection.silence_threshold_db",
    -60.0
).await?;

let min_silence_duration_sec = wkmp_common::config::get_float_setting(
    &self.db,
    "import.boundary_detection.min_silence_duration_sec",
    0.5
).await?;

// Initialize detector with configuration
let silence_detector = SilenceDetector::new(
    silence_threshold_db as f32,
    min_silence_duration_sec as f32
);

// Detect silence regions
let silence_regions = silence_detector.detect(&samples, file_metadata.sample_rate).map_err(|e| {
    ImportError::ExtractionFailed(format!("Silence detection failed: {}", e))
})?;

// Convert silence regions to passage boundaries
let file_boundaries = if silence_regions.is_empty() {
    // No silence detected - entire file is one passage
    vec![PassageBoundary {
        start_ticks: 0,
        end_ticks: (duration_secs * 28_224_000.0) as i64,
        confidence: 0.8,
        detection_method: BoundaryDetectionMethod::SilenceDetection,
    }]
} else {
    // Create passages between silence regions
    let mut boundaries = Vec::new();
    let mut last_end_sample = 0;

    for silence in &silence_regions {
        // Passage before this silence
        if silence.start_sample > last_end_sample {
            let start_ticks = (last_end_sample as f64 / file_metadata.sample_rate as f64 * 28_224_000.0) as i64;
            let end_ticks = (silence.start_sample as f64 / file_metadata.sample_rate as f64 * 28_224_000.0) as i64;

            boundaries.push(PassageBoundary {
                start_ticks,
                end_ticks,
                confidence: 0.8,
                detection_method: BoundaryDetectionMethod::SilenceDetection,
            });
        }
        last_end_sample = silence.end_sample;
    }

    // Final passage after last silence
    if last_end_sample < samples.len() {
        let start_ticks = (last_end_sample as f64 / file_metadata.sample_rate as f64 * 28_224_000.0) as i64;
        let end_ticks = (duration_secs * 28_224_000.0) as i64;

        boundaries.push(PassageBoundary {
            start_ticks,
            end_ticks,
            confidence: 0.8,
            detection_method: BoundaryDetectionMethod::SilenceDetection,
        });
    }

    boundaries
};

tracing::info!(
    "Detected {} passages in file {}",
    file_boundaries.len(),
    audio_file.display()
);
```

**Deliverable:** Multi-passage detection functional

#### Step 1.1.3: Add Database Settings
**File:** [wkmp-ai/src/import_v2/mod.rs](wkmp-ai/src/import_v2/mod.rs) (initialization)

**Code:**
```rust
// Initialize boundary detection settings if not present
sqlx::query(
    r#"
    INSERT OR IGNORE INTO settings (key, value, description)
    VALUES
        ('import.boundary_detection.silence_threshold_db', '-60.0', 'Silence detection threshold in dB (more negative = quieter)'),
        ('import.boundary_detection.min_silence_duration_sec', '0.5', 'Minimum silence duration to detect passage boundary (seconds)')
    "#
)
.execute(db)
.await?;
```

**Deliverable:** Configuration values available in database

#### Step 1.1.4: Write Unit Tests
**File:** [wkmp-ai/tests/boundary_detection.rs](wkmp-ai/tests/boundary_detection.rs) (new)

**Tests:**
- TC-TD-001-03: Short silence ignored
- TC-TD-001-04: Configurable threshold

**Deliverable:** 2 unit tests passing

#### Step 1.1.5: Write Integration Tests
**File:** [wkmp-ai/tests/integration_workflow.rs](wkmp-ai/tests/integration_workflow.rs) (extend)

**Tests:**
- TC-TD-001-01: Multi-track album detection
- TC-TD-001-02: Single track detection

**Deliverable:** 2 integration tests passing

#### Step 1.1.6: Manual Verification
**Action:** Import actual multi-track album FLAC file
**Expected:** Log shows "Detected N passages" where N > 1
**Deliverable:** Real-world validation complete

---

### Increment 1.2: Audio Segment Extraction (REQ-TD-002)

**Effort:** 6-8 hours
**Dependencies:** None (independent of 1.1)
**Files Modified:** 1
**Tests:** 5 (4 unit, 1 error)

#### Step 1.2.1: Research Symphonia Time-Range API
**Documentation:** https://docs.rs/symphonia/latest/symphonia/
**Focus:** `FormatReader::seek()`, `Decoder::decode()`
**Deliverable:** Understanding of seek mechanics and sample-accurate positioning

#### Step 1.2.2: Implement `AudioLoader::extract_segment()`
**File:** [wkmp-ai/src/import_v2/tier1/audio_loader.rs](wkmp-ai/src/import_v2/tier1/audio_loader.rs)

**Add Method:**
```rust
impl AudioLoader {
    /// Extract audio segment within specified time range
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    /// * `start_ticks` - Start position in ticks (28,224,000 Hz)
    /// * `end_ticks` - End position in ticks (28,224,000 Hz)
    /// * `target_sample_rate` - Output sample rate (typically 44100 Hz)
    ///
    /// # Returns
    /// Mono PCM samples for requested time range at target sample rate
    ///
    /// # Errors
    /// - `ImportError::FileNotFound` - File does not exist
    /// - `ImportError::DecodeFailed` - Decode failure
    /// - `ImportError::InvalidTimeRange` - start_ticks > end_ticks
    /// - `ImportError::TimeRangeOutOfBounds` - Range exceeds file duration
    pub async fn extract_segment(
        &self,
        file_path: &Path,
        start_ticks: i64,
        end_ticks: i64,
        target_sample_rate: usize,
    ) -> ImportResult<Vec<f32>> {
        // Validation
        if start_ticks > end_ticks {
            return Err(ImportError::InvalidTimeRange(format!(
                "start_ticks ({}) > end_ticks ({})",
                start_ticks, end_ticks
            )));
        }

        // Open file
        let file = std::fs::File::open(file_path).map_err(|e| {
            ImportError::FileNotFound(format!("{}: {}", file_path.display(), e))
        })?;

        let media_source = Box::new(file);
        let mut format = symphonia::default::get_probe()
            .format(
                &Default::default(),
                symphonia::core::io::MediaSourceStream::new(media_source, Default::default()),
                &Default::default(),
                &Default::default(),
            )
            .map_err(|e| ImportError::DecodeFailed(format!("Probe failed: {}", e)))?
            .format;

        // Get default track
        let track = format
            .default_track()
            .ok_or_else(|| ImportError::DecodeFailed("No default track".to_string()))?;

        let codec_params = track.codec_params.clone();
        let track_id = track.id;

        // Validate time range against file duration
        if let Some(n_frames) = codec_params.n_frames {
            let sample_rate = codec_params.sample_rate.unwrap_or(44100) as f64;
            let duration_ticks = (n_frames as f64 / sample_rate * 28_224_000.0) as i64;

            if end_ticks > duration_ticks {
                return Err(ImportError::TimeRangeOutOfBounds(format!(
                    "end_ticks ({}) exceeds file duration ({})",
                    end_ticks, duration_ticks
                )));
            }
        }

        // Create decoder
        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &Default::default())
            .map_err(|e| ImportError::DecodeFailed(format!("Decoder creation failed: {}", e)))?;

        // Convert ticks to samples
        let sample_rate = codec_params.sample_rate.unwrap_or(44100) as f64;
        let start_sample = (start_ticks as f64 / 28_224_000.0 * sample_rate) as u64;
        let end_sample = (end_ticks as f64 / 28_224_000.0 * sample_rate) as u64;

        // Seek to start position
        if start_sample > 0 {
            let seek_to = symphonia::core::formats::SeekTo::TimeStamp { ts: start_sample, track_id };
            format.seek(symphonia::core::formats::SeekMode::Accurate, seek_to)
                .map_err(|e| ImportError::DecodeFailed(format!("Seek failed: {}", e)))?;
        }

        // Decode samples in range
        let mut samples_f32 = Vec::new();
        let mut current_sample = start_sample;

        while current_sample < end_sample {
            let packet = match format.next_packet() {
                Ok(pkt) => pkt,
                Err(symphonia::core::errors::Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(ImportError::DecodeFailed(format!("Packet read failed: {}", e))),
            };

            if packet.track_id() != track_id {
                continue;
            }

            let decoded = decoder.decode(&packet)
                .map_err(|e| ImportError::DecodeFailed(format!("Decode failed: {}", e)))?;

            // Convert to f32 and handle multi-channel (average to mono)
            let spec = decoded.spec();
            let channels = spec.channels.count();

            for frame_idx in 0..decoded.frames() {
                if current_sample + frame_idx as u64 >= end_sample {
                    break;
                }

                // Average all channels to mono
                let mut sample_sum = 0.0f32;
                for ch in 0..channels {
                    let sample = match decoded.chan(ch) {
                        symphonia::core::audio::AudioBufferRef::F32(buf) => buf[frame_idx],
                        symphonia::core::audio::AudioBufferRef::U8(buf) => (buf[frame_idx] as f32 / 128.0) - 1.0,
                        symphonia::core::audio::AudioBufferRef::U16(buf) => (buf[frame_idx] as f32 / 32768.0) - 1.0,
                        symphonia::core::audio::AudioBufferRef::U24(buf) => (buf[frame_idx].into_i32() as f32 / 8388608.0),
                        symphonia::core::audio::AudioBufferRef::U32(buf) => (buf[frame_idx] as f32 / 2147483648.0) - 1.0,
                        symphonia::core::audio::AudioBufferRef::S8(buf) => buf[frame_idx] as f32 / 128.0,
                        symphonia::core::audio::AudioBufferRef::S16(buf) => buf[frame_idx] as f32 / 32768.0,
                        symphonia::core::audio::AudioBufferRef::S24(buf) => buf[frame_idx].into_i32() as f32 / 8388608.0,
                        symphonia::core::audio::AudioBufferRef::S32(buf) => buf[frame_idx] as f32 / 2147483648.0,
                        symphonia::core::audio::AudioBufferRef::F64(buf) => buf[frame_idx] as f32,
                    };
                    sample_sum += sample;
                }
                samples_f32.push(sample_sum / channels as f32);
            }

            current_sample += decoded.frames() as u64;
        }

        // Resample if needed
        if sample_rate as usize != target_sample_rate {
            samples_f32 = self.resample(samples_f32, sample_rate as usize, target_sample_rate)?;
        }

        Ok(samples_f32)
    }

    /// Resample audio to target sample rate using rubato
    fn resample(&self, samples: Vec<f32>, from_rate: usize, to_rate: usize) -> ImportResult<Vec<f32>> {
        use rubato::{FftFixedInOut, Resampler};

        let resampler = FftFixedInOut::<f32>::new(from_rate, to_rate, samples.len(), 1)
            .map_err(|e| ImportError::DecodeFailed(format!("Resampler creation failed: {}", e)))?;

        let resampled = resampler.process(&[samples], None)
            .map_err(|e| ImportError::DecodeFailed(format!("Resampling failed: {}", e)))?;

        Ok(resampled[0].clone())
    }
}
```

**Deliverable:** `extract_segment()` method functional

#### Step 1.2.3: Update SongWorkflowEngine
**File:** [wkmp-ai/src/import_v2/song_workflow_engine.rs](wkmp-ai/src/import_v2/song_workflow_engine.rs)
**Lines:** 252-253 (replace TODO)

**Current:**
```rust
// Phase 1: Extract audio segment (placeholder - actual implementation TBD)
// TODO: Implement audio segment extraction using symphonia
```

**New:**
```rust
// Phase 1: Extract audio segment for passage
tracing::debug!("Phase 1: Extracting audio segment for passage");

let segment_samples = audio_loader.extract_segment(
    &file_path,
    boundary.start_ticks,
    boundary.end_ticks,
    44100,  // Target sample rate for fingerprinting
).await.map_err(|e| {
    ImportError::ExtractionFailed(format!("Segment extraction failed: {}", e))
})?;

tracing::debug!(
    "Extracted {} samples for fingerprinting",
    segment_samples.len()
);
```

**Deliverable:** Workflow uses segment extraction for fingerprinting

#### Step 1.2.4: Write Unit Tests
**File:** [wkmp-ai/tests/segment_extraction.rs](wkmp-ai/tests/segment_extraction.rs) (new)

**Tests:**
- TC-TD-002-01: Exact duration extraction
- TC-TD-002-02: Precise tick positioning
- TC-TD-002-03: Stereo to mono conversion
- TC-TD-002-04: Resampling

**Deliverable:** 4 unit tests passing

#### Step 1.2.5: Write Error Test
**File:** [wkmp-ai/tests/segment_extraction.rs](wkmp-ai/tests/segment_extraction.rs)

**Test:**
- TC-TD-002-05: Time range out of bounds

**Deliverable:** Error handling validated

---

### Increment 1.3: Remove Amplitude Analysis Stub (REQ-TD-003)

**Effort:** 2 hours
**Dependencies:** None
**Files Modified:** 3
**Tests:** 2 (1 integration, 1 compilation)

#### Step 1.3.1: Remove Endpoint from API Routes
**File:** [wkmp-ai/src/api/routes.rs](wkmp-ai/src/api/routes.rs)

**Remove:**
```rust
.route("/analyze/amplitude", post(amplitude_analysis::analyze_amplitude))
```

**Deliverable:** Route registration removed

#### Step 1.3.2: Delete Module
**File:** [wkmp-ai/src/api/amplitude_analysis.rs](wkmp-ai/src/api/amplitude_analysis.rs)

**Action:** Delete entire file

**Deliverable:** Module removed from codebase

#### Step 1.3.3: Remove from mod.rs
**File:** [wkmp-ai/src/api/mod.rs](wkmp-ai/src/api/mod.rs)

**Remove:**
```rust
pub mod amplitude_analysis;
```

**Deliverable:** Module declaration removed

#### Step 1.3.4: Add TODO Comment for Future Implementation
**File:** [wkmp-ai/src/api/mod.rs](wkmp-ai/src/api/mod.rs)

**Add:**
```rust
// TODO: Amplitude analysis deferred to future release
// Will be implemented when use case is clarified (PLAN026 REQ-TD-003)
```

**Deliverable:** Documentation of deferral

#### Step 1.3.5: Verify Compilation
**Command:** `cargo build -p wkmp-ai`

**Expected:** No errors related to amplitude_analysis

**Deliverable:** Clean build

#### Step 1.3.6: Write Integration Test
**File:** [wkmp-ai/tests/api_endpoints.rs](wkmp-ai/tests/api_endpoints.rs)

**Test:** TC-TD-003-01: Endpoint removed (returns 404)

**Deliverable:** Test confirms removal

---

## Sprint 2: High Priority (Week 2-3)

**Goal:** Improve metadata quality and event correlation

**Total Effort:** 19-27 hours
**Test Count:** 14 tests
**Deliverable:** Metadata quality improved, events properly correlated

---

### Increment 2.1: MBID Extraction from ID3 Tags (REQ-TD-004)

**Effort:** 4-6 hours
**Dependencies:** May require adding `id3` crate
**Files Modified:** 2
**Tests:** 3 (2 unit, 1 error)

#### Step 2.1.1: Investigate lofty UFID Support
**Action:** Read lofty crate documentation and source code
**Check:** Can `lofty::Tag` access UFID frames?
**Deliverable:** Decision on whether fallback needed

#### Step 2.1.2: Add id3 Crate (If Needed)
**File:** [wkmp-ai/Cargo.toml](wkmp-ai/Cargo.toml)

**Add:**
```toml
id3 = "1.0"  # Fallback for UFID extraction if lofty insufficient
```

**Deliverable:** Dependency available if needed

#### Step 2.1.3: Implement MBID Extraction
**File:** [wkmp-ai/src/import_v2/tier1/id3_extractor.rs](wkmp-ai/src/import_v2/tier1/id3_extractor.rs)
**Lines:** 208-209 (replace TODO)

**Current:**
```rust
// TODO: Extract MusicBrainz ID from UFID frame (lofty doesn't expose UFID yet)
mbid: None,
```

**New (Option A - lofty supports UFID):**
```rust
// Extract MusicBrainz ID from UFID frame
mbid: tag
    .get_string(&ItemKey::Unknown("UFID"))  // Adjust based on lofty API
    .and_then(|s| Uuid::parse_str(s).ok()),
```

**New (Option B - fallback to id3 crate):**
```rust
// Extract MusicBrainz ID from UFID frame
// Note: lofty doesn't expose UFID, use id3 crate for MP3 files
mbid: if file_path.extension().and_then(|s| s.to_str()) == Some("mp3") {
    extract_mbid_from_ufid_mp3(file_path)?
} else {
    None  // UFID extraction only supported for MP3 files
},
```

**Helper Function (Option B):**
```rust
/// Extract MBID from MP3 UFID frame using id3 crate
fn extract_mbid_from_ufid_mp3(file_path: &Path) -> Option<Uuid> {
    use id3::Tag;

    let tag = Tag::read_from_path(file_path).ok()?;

    // Search for MusicBrainz UFID frame
    for frame in tag.frames() {
        if let id3::Content::UniqueFileIdentifier(ufid) = frame.content() {
            if ufid.owner == "http://musicbrainz.org" {
                // Parse MBID from identifier bytes
                let mbid_str = String::from_utf8_lossy(&ufid.identifier);
                if let Ok(uuid) = Uuid::parse_str(&mbid_str) {
                    tracing::debug!("Extracted MBID from UFID: {}", uuid);
                    return Some(uuid);
                }
            }
        }
    }

    None
}
```

**Deliverable:** MBID extraction functional for MP3 files

#### Step 2.1.4: Write Unit Tests
**File:** [wkmp-ai/tests/mbid_extraction.rs](wkmp-ai/tests/mbid_extraction.rs) (new)

**Tests:**
- TC-TD-004-01: Extract MBID from MP3 UFID
- TC-TD-004-02: File without MBID returns None

**Deliverable:** 2 unit tests passing

#### Step 2.1.5: Write Error Test
**File:** [wkmp-ai/tests/mbid_extraction.rs](wkmp-ai/tests/mbid_extraction.rs)

**Test:** TC-TD-004-03: Malformed MBID handling

**Deliverable:** Error handling validated

---

### Increment 2.2: Consistency Checker Implementation (REQ-TD-005)

**Effort:** 6-8 hours
**Dependencies:** Requires MetadataFuser modification
**Files Modified:** 2
**Tests:** 3 (3 unit)

#### Step 2.2.1: Define String Similarity Thresholds
**File:** [wkmp-ai/src/import_v2/tier3/consistency_checker.rs](wkmp-ai/src/import_v2/tier3/consistency_checker.rs)

**Add Constants:**
```rust
const CONFLICT_THRESHOLD: f64 = 0.85;  // strsim < 0.85 = Conflict
const WARNING_THRESHOLD: f64 = 0.95;   // 0.85 ≤ strsim < 0.95 = Warning
// strsim ≥ 0.95 = Pass
```

**Deliverable:** Thresholds documented

#### Step 2.2.2: Modify MetadataFuser to Preserve Candidates
**File:** [wkmp-ai/src/import_v2/tier2/metadata_fuser.rs](wkmp-ai/src/import_v2/tier2/metadata_fuser.rs)

**Current API:**
```rust
pub fn fuse(&self, bundle: MetadataBundle) -> FusedMetadata { ... }
```

**New API:**
```rust
pub struct FusionResult {
    pub fused: FusedMetadata,
    pub candidates: MetadataBundle,  // Preserve all candidates for validation
}

pub fn fuse(&self, bundle: MetadataBundle) -> FusionResult {
    let fused = self.fuse_internal(&bundle);  // Existing logic
    FusionResult {
        fused,
        candidates: bundle,  // Preserve original candidates
    }
}
```

**Deliverable:** All metadata candidates available for validation

#### Step 2.2.3: Implement Validation Logic
**File:** [wkmp-ai/src/import_v2/tier3/consistency_checker.rs](wkmp-ai/src/import_v2/tier3/consistency_checker.rs)
**Lines:** 51-70 (replace stub)

**Current:**
```rust
pub fn validate_title(&self, metadata: &FusedMetadata) -> ValidationResult {
    ValidationResult::Pass  // Stub
}
```

**New:**
```rust
pub fn validate_title(&self, candidates: &MetadataBundle) -> ValidationResult {
    use strsim::normalized_levenshtein;

    let title_candidates = &candidates.title;

    if title_candidates.len() <= 1 {
        return ValidationResult::Pass;  // No conflict possible
    }

    // Compare all pairs of candidates
    for i in 0..title_candidates.len() {
        for j in (i + 1)..title_candidates.len() {
            let title1 = &title_candidates[i].value;
            let title2 = &title_candidates[j].value;
            let similarity = normalized_levenshtein(title1, title2);

            if similarity < CONFLICT_THRESHOLD {
                return ValidationResult::Conflict(format!(
                    "Title mismatch: '{}' (source: {:?}) vs '{}' (source: {:?})",
                    title1, title_candidates[i].source,
                    title2, title_candidates[j].source
                ));
            } else if similarity < WARNING_THRESHOLD {
                return ValidationResult::Warning(format!(
                    "Title minor difference: '{}' vs '{}' (similarity: {:.2})",
                    title1, title2, similarity
                ));
            }
        }
    }

    ValidationResult::Pass
}

// Implement same logic for validate_artist() and validate_album()
pub fn validate_artist(&self, candidates: &MetadataBundle) -> ValidationResult {
    // Same logic as validate_title() but for artist field
    // ...
}

pub fn validate_album(&self, candidates: &MetadataBundle) -> ValidationResult {
    // Same logic as validate_title() but for album field
    // ...
}
```

**Deliverable:** Conflict detection functional

#### Step 2.2.4: Update Workflow to Use Candidates
**File:** [wkmp-ai/src/import_v2/song_workflow_engine.rs](wkmp-ai/src/import_v2/song_workflow_engine.rs)

**Update Phase 6 (Validation):**
```rust
// Phase 6: Consistency validation
let fusion_result = metadata_fuser.fuse(metadata_bundle);

let validation_report = consistency_checker.validate_all(&fusion_result.candidates);

// Use fusion_result.fused for final metadata
```

**Deliverable:** Workflow uses candidate-based validation

#### Step 2.2.5: Write Unit Tests
**File:** [wkmp-ai/tests/consistency_checker.rs](wkmp-ai/tests/consistency_checker.rs) (new)

**Tests:**
- TC-TD-005-01: Conflicting candidates detected
- TC-TD-005-02: Similar candidates produce warning
- TC-TD-005-03: Identical candidates pass

**Deliverable:** 3 unit tests passing

---

### Increment 2.3: Event Bridge session_id Fields (REQ-TD-006)

**Effort:** 2-3 hours
**Dependencies:** None
**Files Modified:** 1
**Tests:** 2 (1 unit, 1 integration)

#### Step 2.3.1: Add session_id Fields to ImportEvent
**File:** [wkmp-ai/src/event_bridge.rs](wkmp-ai/src/event_bridge.rs)
**Lines:** 110, 128, 147, etc.

**Current:**
```rust
PassagesDiscovered {
    file_id: Uuid,
    count: usize,
},
```

**New:**
```rust
PassagesDiscovered {
    session_id: Uuid,  // Add session_id
    file_id: Uuid,
    count: usize,
},
```

**Apply to All Variants:**
- PassagesDiscovered
- SongStarted
- SongProgress
- SongComplete
- SongFailed
- SessionProgress
- SessionComplete
- SessionFailed

**Deliverable:** All event variants have session_id field

#### Step 2.3.2: Update Event Conversion Logic
**File:** [wkmp-ai/src/event_bridge.rs](wkmp-ai/src/event_bridge.rs)

**Update `convert()` method to extract session_id:**
```rust
match import_event {
    ImportEvent::PassagesDiscovered { session_id, file_id, count } => {
        WkmpEvent {
            event_type: "passages_discovered".to_string(),
            data: json!({
                "session_id": session_id,  // Include in payload
                "file_id": file_id,
                "count": count
            }),
            ..Default::default()
        }
    },
    // ... update all variants
}
```

**Deliverable:** Converted events include session_id in payload

#### Step 2.3.3: Update Event Emission Sites
**Files:**
- [wkmp-ai/src/import_v2/session_orchestrator.rs](wkmp-ai/src/import_v2/session_orchestrator.rs)
- [wkmp-ai/src/import_v2/song_workflow_engine.rs](wkmp-ai/src/import_v2/song_workflow_engine.rs)

**Replace all `Uuid::nil()` with actual session_id:**
```rust
// OLD:
self.emit_event(ImportEvent::SongStarted {
    session_id: Uuid::nil(),  // WRONG
    passage_id,
});

// NEW:
self.emit_event(ImportEvent::SongStarted {
    session_id: self.session_id,  // Use actual ID
    passage_id,
});
```

**Deliverable:** All events use valid session_id

#### Step 2.3.4: Write Unit Test
**File:** [wkmp-ai/tests/event_bridge.rs](wkmp-ai/tests/event_bridge.rs) (new)

**Test:** TC-TD-006-01: All events include valid session_id

**Deliverable:** Unit test validates session_id presence

#### Step 2.3.5: Write Integration Test
**File:** [wkmp-ai/tests/event_correlation.rs](wkmp-ai/tests/event_correlation.rs) (new)

**Test:** TC-TD-006-02: UI event correlation

**Deliverable:** End-to-end validation of event correlation

---

### Increment 2.4: Flavor Synthesis Implementation (REQ-TD-007)

**Effort:** 4-6 hours
**Dependencies:** None (FlavorSynthesizer exists)
**Files Modified:** 1
**Tests:** 3 (3 unit)

#### Step 2.4.1: Read FlavorSynthesizer API
**File:** [wkmp-ai/src/import_v2/tier2/flavor_synthesizer.rs](wkmp-ai/src/import_v2/tier2/flavor_synthesizer.rs)

**Action:** Review `synthesize()` method, input/output types

**Deliverable:** Understanding of synthesis algorithm

#### Step 2.4.2: Convert Audio-Derived Flavor to FlavorExtraction
**File:** [wkmp-ai/src/import_v2/song_workflow_engine.rs](wkmp-ai/src/import_v2/song_workflow_engine.rs)
**Lines:** 369-370 (replace TODO)

**Current:**
```rust
// TODO: Convert ExtractorResult<MusicalFlavor> to FlavorExtraction for synthesis
```

**New:**
```rust
// Phase 5: Flavor synthesis - Combine multiple flavor sources
tracing::debug!("Phase 5: Synthesizing musical flavor from all sources");

let mut flavor_sources = Vec::new();

// Add audio-derived flavor
if let Ok(audio_flavor) = audio_derived_flavor {
    flavor_sources.push(FlavorExtraction {
        flavor: audio_flavor.data,
        confidence: audio_flavor.confidence,
        source: audio_flavor.source,
    });
}

// Add AcousticBrainz flavor (if available)
// ... (implementation depends on whether AcousticBrainz integration exists)

// Synthesize combined flavor
let synthesized = if !flavor_sources.is_empty() {
    flavor_synthesizer.synthesize(flavor_sources).map_err(|e| {
        ImportError::ExtractionFailed(format!("Flavor synthesis failed: {}", e))
    })?
} else {
    // No flavor sources available - use default
    SynthesizedFlavor {
        flavor: MusicalFlavor::default(),
        flavor_confidence: 0.1,
        flavor_completeness: 0.0,
        sources_used: vec![],
    }
};

tracing::info!(
    "Flavor synthesis complete: confidence={:.2}, completeness={:.2}",
    synthesized.flavor_confidence,
    synthesized.flavor_completeness
);
```

**Deliverable:** Workflow uses flavor synthesis

#### Step 2.4.3: Write Unit Tests
**File:** [wkmp-ai/tests/flavor_synthesis.rs](wkmp-ai/tests/flavor_synthesis.rs) (new)

**Tests:**
- TC-TD-007-01: Multiple sources combined
- TC-TD-007-02: Agreeing sources high confidence
- TC-TD-007-03: Conflicting sources lower confidence

**Deliverable:** 3 unit tests passing

---

### Increment 2.5: Chromaprint Compressed Fingerprint (REQ-TD-008)

**Effort:** 3-4 hours
**Dependencies:** May need compression implementation
**Files Modified:** 2 (analyzer + migration)
**Tests:** 3 (1 unit, 1 integration, 1 error)

#### Step 2.5.1: Check chromaprint-rust Compression API
**Documentation:** Check if `chromaprint` crate exposes compression

**Options:**
- API available → Use native compression
- API not available → Implement base64 encoding

**Deliverable:** Implementation strategy chosen

#### Step 2.5.2: Implement Compression (If Needed)
**File:** [wkmp-ai/src/import_v2/tier1/chromaprint_analyzer.rs](wkmp-ai/src/import_v2/tier1/chromaprint_analyzer.rs)
**Lines:** 93-94 (replace TODO)

**Current:**
```rust
// TODO: Use Chromaprint's compressed fingerprint format (for AcoustID compatibility)
fingerprint_compressed: None,
```

**New (Option A - Native):**
```rust
fingerprint_compressed: Some(chromaprint.compress(&fingerprint)),
```

**New (Option B - Manual Base64):**
```rust
fingerprint_compressed: Some(compress_fingerprint(&fingerprint)),
```

**Helper Function (Option B):**
```rust
/// Compress fingerprint to AcoustID-compatible base64 format
fn compress_fingerprint(raw: &[u32]) -> String {
    use base64::{Engine as _, engine::general_purpose};

    // Convert u32 array to bytes
    let bytes: Vec<u8> = raw
        .iter()
        .flat_map(|&val| val.to_le_bytes())
        .collect();

    // Encode as base64
    general_purpose::STANDARD.encode(bytes)
}
```

**Deliverable:** Compressed fingerprint generation functional

#### Step 2.5.3: Create Database Migration
**File:** [wkmp-ai/migrations/V009_chromaprint_compression.sql](wkmp-ai/migrations/V009_chromaprint_compression.sql) (new)

**SQL:**
```sql
-- Migration V009: Add compressed fingerprint column
-- Requirement: REQ-TD-008
-- Date: 2025-11-10

ALTER TABLE fingerprints ADD COLUMN fingerprint_compressed TEXT;

-- Optional: Populate from existing raw fingerprints
-- (Can be done lazily during next import)
```

**Deliverable:** Migration file created

#### Step 2.5.4: Write Unit Test
**File:** [wkmp-ai/tests/chromaprint_compression.rs](wkmp-ai/tests/chromaprint_compression.rs) (new)

**Test:** TC-TD-008-01: Fingerprint compressed to base64

**Deliverable:** Unit test validates compression

#### Step 2.5.5: Write Integration Test
**File:** [wkmp-ai/tests/chromaprint_acoustid.rs](wkmp-ai/tests/chromaprint_acoustid.rs) (new)

**Test:** TC-TD-008-02: AcoustID accepts compressed format

**Deliverable:** API compatibility validated

#### Step 2.5.6: Write Error Test
**File:** [wkmp-ai/tests/chromaprint_compression.rs](wkmp-ai/tests/chromaprint_compression.rs)

**Test:** TC-TD-008-03: Compression error handling

**Deliverable:** Error handling validated

---

## Sprint 3: Medium Priority (DEFERRED)

**Status:** Explicitly out of scope for this implementation plan

**Requirements:**
- REQ-TD-009: Waveform Rendering
- REQ-TD-010: Duration Tracking in File Stats
- REQ-TD-011: Flavor Confidence Calculation (superseded by REQ-TD-007)
- REQ-TD-012: Flavor Data Persistence (blocked by REQ-TD-007)

**Future Plan:** Will be addressed in separate plan when Sprint 1 and Sprint 2 complete.

---

## Implementation Order Summary

### Week 1 (Sprint 1)
Day 1-2: Increment 1.1 (Boundary Detection)
Day 3-4: Increment 1.2 (Segment Extraction)
Day 5: Increment 1.3 (Remove Amplitude Analysis)

### Week 2 (Sprint 2)
Day 1-2: Increment 2.1 (MBID Extraction) + Increment 2.3 (Event Bridge)
Day 3-4: Increment 2.2 (Consistency Checker)
Day 5: Increment 2.4 (Flavor Synthesis)

### Week 3 (Sprint 2 Continued)
Day 1: Increment 2.5 (Chromaprint Compression)
Day 2-3: Integration testing, bug fixes
Day 4-5: Performance benchmarks, documentation

---

## Testing Strategy

### Per-Increment Testing
- Write tests BEFORE implementation (TDD approach)
- Run `cargo test` after each code change
- Achieve 100% test coverage per increment before moving to next

### Sprint-Level Testing
- Run full test suite at end of each sprint
- Include regression tests from previous work
- Performance benchmarks for Sprint 1

### Pre-Merge Checklist
✅ All 25 acceptance tests passing
✅ No new compiler warnings
✅ Performance benchmarks meet targets (<200ms boundary detection, <100ms segment extraction)
✅ Integration test with real multi-track album successful
✅ Code review completed

---

## Risk Mitigation

### Highest Risk: REQ-TD-002 (Segment Extraction)
**Mitigation:**
- Start with simple WAV files for initial implementation
- Add format support incrementally (WAV → FLAC → MP3 → AAC)
- Comprehensive error handling at each layer
- Test with various sample rates and bit depths

### Medium Risk: REQ-TD-004 (MBID Extraction)
**Mitigation:**
- Investigate lofty API before committing to fallback
- If fallback needed, limit to MP3 files (most common use case)
- Document limitations clearly
- AcoustID fingerprinting remains available as alternative

---

## Success Metrics

### Sprint 1 Success Criteria
✅ User imports 10-track album → 10 passages detected
✅ Each passage fingerprinted independently
✅ No stub endpoints returning fake data
✅ All 11 Sprint 1 tests passing
✅ Performance: Boundary detection <200ms per file

### Sprint 2 Success Criteria
✅ MBID extracted from MP3 files with UFID frames
✅ Metadata conflicts detected and reported
✅ All events correlated by session_id
✅ Musical flavor synthesis functional
✅ Chromaprint fingerprints in AcoustID-compatible format
✅ All 14 Sprint 2 tests passing

### Overall Success
✅ Technical debt reduced from 45+ markers to <20
✅ Zero regression in existing functionality
✅ Production-ready multi-track album import
✅ Improved metadata quality and event correlation
