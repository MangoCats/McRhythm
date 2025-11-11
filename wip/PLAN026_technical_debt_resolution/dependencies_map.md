# Dependencies Map - PLAN026: Technical Debt Resolution

## Requirement Dependencies (Implementation Order)

```
Sprint 1 (Critical - No Dependencies)
├── REQ-TD-001: Boundary Detection (INDEPENDENT)
├── REQ-TD-002: Segment Extraction (INDEPENDENT)
└── REQ-TD-003: Amplitude Analysis (INDEPENDENT)

Sprint 2 (High - Some Dependencies)
├── REQ-TD-004: MBID Extraction (INDEPENDENT)
├── REQ-TD-005: Consistency Checker (INDEPENDENT)
├── REQ-TD-006: Event Bridge (INDEPENDENT)
├── REQ-TD-007: Flavor Synthesis (INDEPENDENT)
└── REQ-TD-008: Chromaprint Compression (INDEPENDENT)

Sprint 3 (Deferred)
├── REQ-TD-009: Waveform Rendering (INDEPENDENT but deferred)
├── REQ-TD-010: Duration Tracking (INDEPENDENT but deferred)
├── REQ-TD-011: Flavor Confidence ──┐
└── REQ-TD-012: Flavor Persistence  └─> BLOCKED BY REQ-TD-007
```

**Key Insight:** All Sprint 1 and Sprint 2 requirements are independent - can be implemented in parallel if desired.

---

## External Library Dependencies

### Already in Cargo.toml (wkmp-ai)
```toml
[dependencies]
symphonia = { version = "0.5", features = ["all"] }  # REQ-TD-002
rubato = "0.15"                                       # REQ-TD-002
lofty = "0.21"                                        # REQ-TD-004
chromaprint = { package = "chromaprint-rust" }       # REQ-TD-008
strsim = "0.11"                                       # REQ-TD-005
uuid = { version = "1.0", features = ["v4"] }        # REQ-TD-006
sqlx = { version = "0.8", features = ["sqlite"] }    # All DB access
tokio = { version = "1", features = ["full"] }       # Async runtime
tracing = "0.1"                                       # Logging
```

### May Need to Add
```toml
id3 = "1.0"  # REQ-TD-004 - Fallback for UFID extraction if lofty insufficient
```

---

## Internal Component Dependencies

### Components Used (Already Implemented)

#### REQ-TD-001: Boundary Detection
**Uses:**
- [services/silence_detector.rs](../src/services/silence_detector.rs) - `SilenceDetector::detect()`
- [import_v2/types.rs](../src/import_v2/types.rs) - `PassageBoundary`, `BoundaryDetectionMethod`

**Modifies:**
- [session_orchestrator.rs](../src/import_v2/session_orchestrator.rs) - `process_session()` Phase 3

#### REQ-TD-002: Audio Segment Extraction
**Uses:**
- [tier1/audio_loader.rs](../src/import_v2/tier1/audio_loader.rs) - Extends with `extract_segment()`
- `symphonia::core::formats::FormatReader` - Seek and decode
- `rubato::FftFixedInOut` - Resampling

**Used By:**
- [song_workflow_engine.rs](../src/import_v2/song_workflow_engine.rs) - `run_song_workflow()` Phase 3

#### REQ-TD-003: Amplitude Analysis
**Removes:**
- [api/amplitude_analysis.rs](../src/api/amplitude_analysis.rs) - Entire module
- [api/mod.rs](../src/api/mod.rs) - Route registration
- [api/routes.rs](../src/api/routes.rs) - API endpoint

#### REQ-TD-004: MBID Extraction
**Modifies:**
- [tier1/id3_extractor.rs](../src/import_v2/tier1/id3_extractor.rs) - `extract()` method
- May add `id3` crate usage for UFID frames

#### REQ-TD-005: Consistency Checker
**Uses:**
- [tier3/consistency_checker.rs](../src/import_v2/tier3/consistency_checker.rs) - `validate_*()` methods
- [tier2/metadata_fuser.rs](../src/import_v2/tier2/metadata_fuser.rs) - Must preserve candidates
- `strsim::normalized_levenshtein()` - String similarity

#### REQ-TD-006: Event Bridge
**Modifies:**
- [event_bridge.rs](../src/event_bridge.rs) - `ImportEvent` enum variants
- [event_bridge.rs](../src/event_bridge.rs) - `convert()` method

**Affects:**
- All event emission sites (session_orchestrator, song_workflow_engine)

#### REQ-TD-007: Flavor Synthesis
**Uses:**
- [tier2/flavor_synthesizer.rs](../src/import_v2/tier2/flavor_synthesizer.rs) - `synthesize()` method
- [import_v2/types.rs](../src/import_v2/types.rs) - `FlavorExtraction`, `SynthesizedFlavor`

**Modifies:**
- [song_workflow_engine.rs](../src/import_v2/song_workflow_engine.rs) - `run_song_workflow()` Phase 5

#### REQ-TD-008: Chromaprint Compression
**Uses:**
- [tier1/chromaprint_analyzer.rs](../src/import_v2/tier1/chromaprint_analyzer.rs) - `analyze()` method
- `chromaprint-rust` compression API (if available)

**May Need:**
- Base64 encoding implementation if compression not exposed

---

## Database Schema Dependencies

### Existing Schema (No Changes for Sprint 1)
- `files` table - No changes needed
- `passages` table - Already has boundary fields

### Changes Required (Sprint 2)

#### REQ-TD-008: Chromaprint Compression
**Migration Required:** Add `fingerprint_compressed` column
```sql
ALTER TABLE fingerprints ADD COLUMN fingerprint_compressed TEXT;
```

**Backward Compatibility:**
- Keep existing `fingerprint_raw` column
- Add new `fingerprint_compressed` column
- Migration populates compressed from raw

---

## Configuration Dependencies

### Database Settings (May Need New Settings)

#### REQ-TD-001: Boundary Detection
```sql
INSERT OR IGNORE INTO settings (key, value, description) VALUES
('import.boundary_detection.silence_threshold_db', '-60.0', 'Silence detection threshold in dB'),
('import.boundary_detection.min_silence_duration_sec', '0.5', 'Minimum silence duration to detect boundary');
```

### No TOML Config Changes
All configuration database-driven per WKMP architecture.

---

## Test Dependencies

### Existing Test Infrastructure
- `serial_test` crate - Already used for database tests
- `tempfile` crate - Temporary test files
- `hound` crate - WAV file generation for tests

### Test Data Needed

#### REQ-TD-001: Boundary Detection
- Multi-track album WAV file (10 tracks, 2-second gaps)
- Single-track WAV file (no boundaries)

#### REQ-TD-002: Segment Extraction
- 3-minute test WAV file (extract 30-second segment)
- Test various formats (FLAC, MP3, AAC)

#### REQ-TD-004: MBID Extraction
- MP3 file with MusicBrainz UFID frame
- Test file without MBID

#### REQ-TD-005: Consistency Checker
- Mock MetadataBundle with conflicting candidates
- Test cases: identical, similar (typo), conflicting

---

## Cross-Module Dependencies

### Microservices Affected
- **wkmp-ai:** All changes isolated to this module
- **wkmp-ui:** No code changes (may display new event fields)
- **wkmp-common:** No changes to shared library

### API Contract Changes
- REQ-TD-003: **BREAKING** - Removes `/analyze/amplitude` endpoint
  - Impact: If wkmp-ui calls this endpoint, must remove that code
  - Mitigation: Grep for endpoint usage before removal

### Event Schema Changes
- REQ-TD-006: **NON-BREAKING** - Adds session_id field
  - Old code ignores new field (backward compatible)
  - New UI code can use field for correlation

---

## Risk Dependencies

### Dependency Chain Failures

**If REQ-TD-001 fails:**
- No impact on other requirements
- Multi-track albums remain broken (status quo)

**If REQ-TD-002 fails:**
- Blocks per-passage fingerprinting
- Fallback: Use full-file fingerprints (less accurate)

**If REQ-TD-004 fails:**
- Increased AcoustID API usage (more requests)
- Slower imports (additional network calls)

**If REQ-TD-007 fails:**
- REQ-TD-011 and REQ-TD-012 remain blocked
- Musical flavor vectors less accurate

---

## Parallel Implementation Feasibility

### Can Be Implemented Simultaneously
✅ All Sprint 1 requirements (REQ-TD-001, 002, 003)
✅ All Sprint 2 requirements (REQ-TD-004 through 008)

### Merge Conflicts Unlikely
- Requirements touch different files
- Only potential conflict: session_orchestrator.rs (REQ-TD-001 vs others)
- Mitigation: Implement REQ-TD-001 first (highest impact)

### Recommended Sequence (if serial)
1. REQ-TD-001 (boundary detection) - Highest user impact
2. REQ-TD-002 (segment extraction) - Enables fingerprinting
3. REQ-TD-003 (remove stub) - Quick win
4. REQ-TD-005 (consistency checker) - Data quality
5. REQ-TD-007 (flavor synthesis) - Unblocks deferred work
6. REQ-TD-006 (event bridge) - UI correlation
7. REQ-TD-004 (MBID extraction) - Optimization
8. REQ-TD-008 (chromaprint) - Storage optimization
