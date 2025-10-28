# Recommendations & Deliverables

**Navigation:** [← Back to Summary](00_SUMMARY.md)

---

## Consolidated Recommendations

Based on risk-first analysis (CLAUDE.md framework), recommended approaches for wkmp-ai specification:

### 1. Amplitude Analysis
**Recommendation:** RMS with A-weighting
- **Reason:** Lowest risk (Low), adequate accuracy
- **Alternative considered:** EBU R128 LUFS (higher complexity, Low-Medium risk)

### 2. Parameter Storage
**Recommendation:** Hybrid (Global JSON + Per-Passage Overrides)
- **Reason:** Lowest risk (Low), matches `musical_flavor_vector` pattern
- **Implementation:**
  - Global defaults in `settings` table
  - Per-passage overrides in `passages.import_metadata` column
- **Alternative considered:** Dedicated table (Low-Medium risk, requires migrations)

### 3. Essentia Integration
**Recommendation:** Subprocess calls to binary
- **Reason:** Lowest risk (Low), clean separation
- **Implementation:** Call `essentia_streaming_extractor_music`, parse JSON output
- **Alternative considered:** FFI (Medium-High risk, memory safety issues)

### 4. Workflow Model
**Recommendation:** Async background jobs with SSE progress updates
- **Reason:** Lowest risk (Low), best UX, matches WKMP SSE architecture
- **Implementation:**
  - Tokio async tasks for import jobs
  - SSE endpoint for real-time progress
  - Polling fallback if SSE unavailable
- **Alternative considered:** Synchronous HTTP (High risk, timeouts)

### 5. UI Complexity
**Recommendation:** Progressive disclosure (Simple wizard → Advanced parameters)
- **Reason:** Lowest risk (Low), matches WKMP philosophy
- **Implementation:**
  - Default: Simple wizard with auto-detection
  - "Advanced" button reveals parameter tuning
- **Alternative considered:** Simple-only (Medium risk, can't fix edge cases)

---

## Specification Deliverables Required

### New Documents (Tier 2 - Design)

**1. SPEC###-audio_ingest_architecture.md** (~300 lines)
- Module architecture overview
- Integration with wkmp-ui, wkmp-pd
- Event system (SSE) integration
- HTTP API overview
- Workflow state machine

**2. SPEC###-amplitude_analysis.md** (~400 lines)
- Lead-in detection algorithm (RMS-based)
- Lead-out detection algorithm
- Quick ramp-up/ramp-down detection
- Parameter definitions:
  - `rms_window_ms` (default: 100)
  - `lead_in_threshold_db` (default: -12)
  - `lead_out_threshold_db` (default: -12)
  - `quick_ramp_threshold` (default: 0.75)
  - `quick_ramp_duration_ms` (default: 1000)
  - `max_lead_in_duration_s` (default: 5.0)
  - `max_lead_out_duration_s` (default: 5.0)
- RMS envelope calculation
- A-weighting filter specification
- Tick conversion for database storage

### New Documents (Tier 3 - Implementation)

**3. IMPL###-audio_ingest_api.md** (~300 lines)
- Complete HTTP API specification
- Endpoints:
  - `POST /import/start`
  - `GET /import/status/{session_id}`
  - `POST /import/cancel/{session_id}`
  - `POST /analyze/amplitude`
  - `GET /parameters/global`
  - `POST /parameters/global`
  - `GET /metadata/{passage_id}`
  - `POST /metadata/{passage_id}`
  - `GET /events` (SSE endpoint)
- Request/response schemas (JSON)
- Error codes and handling
- Rate limiting

**4. IMPL###-amplitude_analyzer_implementation.md** (~400 lines)
- Rust implementation details
- Library choices:
  - `dasp` for RMS calculation
  - `symphonia` for audio decoding
  - `rubato` for resampling if needed
- Code structure:
  - `struct AmplitudeAnalyzer`
  - `fn analyze_lead_in() -> Duration`
  - `fn analyze_lead_out() -> Duration`
  - `fn calculate_rms_envelope() -> Vec<f32>`
- Testing strategy:
  - Unit tests (synthetic audio)
  - Integration tests (real audio files)
  - Reference audio files for validation
- Performance optimization
- Error handling

**5. IMPL###-parameter_management.md** (~250 lines)
- Database schema:
  ```sql
  -- Add to settings table
  INSERT INTO settings (key, value_type, value_text)
  VALUES ('import_parameters', 'json', '{...}');

  -- Add to passages table
  ALTER TABLE passages ADD COLUMN import_metadata TEXT;
  ```
- JSON schema for parameters
- Default values
- Validation rules (ranges, types)
- Precedence: per-passage overrides global
- UI integration guidance

### Documentation Updates

**6. REQ001-requirements.md** (~50 lines changes)

Add requirements:
```
[REQ-PI-061] Automatic lead-in detection based on amplitude analysis
  - Detect slow amplitude ramps at passage start
  - Threshold: 1/4 perceived audible intensity (RMS-based)
  - Maximum lead-in duration: 5 seconds
  - Quick ramp-up (<1s to 3/4 intensity): zero lead-in

[REQ-PI-062] Automatic lead-out detection based on amplitude analysis
  - Detect slow amplitude ramps at passage end
  - Threshold: 1/4 perceived audible intensity (RMS-based)
  - Maximum lead-out duration: 5 seconds
  - Quick ramp-down (<1s to 3/4 intensity): zero lead-out

[REQ-PI-063] User-adjustable algorithm parameters
  - All amplitude analysis thresholds configurable
  - Global defaults + per-passage overrides
  - UI provides tuning interface

[REQ-PI-064] Extensible metadata framework
  - Support arbitrary 0.0-1.0 numeric parameters
  - Examples: seasonal_holiday, profanity_level
  - May be automatic, manual, or both
```

**7. SPEC008-library_management.md** (~30 lines changes)

Add section:
```
## Amplitude-Based Timing Point Detection

For automatic detection of passage lead-in and lead-out points,
see [Amplitude Analysis](SPEC###-amplitude_analysis.md).

This complements silence-based segmentation (for finding passage
boundaries) with amplitude-based analysis (for finding optimal
crossfade timing points within passages).
```

**8. IMPL001-database_schema.md** (~50 lines changes)

Update `passages` table:
```sql
ALTER TABLE passages ADD COLUMN import_metadata TEXT;
```

Add documentation:
```
| import_metadata | TEXT | | JSON blob for import-time analysis data (optional) |

**Import Metadata Format:**
{
  "rms_profile_summary": {
    "peak_rms": 0.85,
    "lead_in_detected": 2.35,
    "lead_out_detected": 3.12
  },
  "parameters_used": {
    "rms_window_ms": 100,
    "lead_in_threshold_db": -12,
    ...
  }
}

Provides audit trail of how passage timing was determined.
```

Update `passages` table (add additional_metadata):
```sql
ALTER TABLE passages ADD COLUMN additional_metadata TEXT;
```

Add documentation:
```
| additional_metadata | TEXT | | JSON blob for extensible metadata parameters |

**Additional Metadata Format:**
{
  "seasonal_holiday": 0.0,   // 0.0 = not seasonal, 1.0 = Christmas song
  "profanity_level": 0.0,    // 0.0 = clean, 1.0 = explicit
  "energy_level": 0.85,      // Automatic or manual
  "danceability": 0.72       // Automatic (Essentia) or manual
}

Allows extensible parameters without schema changes.
```

---

## Effort Estimates

**Specification Documents (No Coding):**
- SPEC###-audio_ingest_architecture.md: 2-3 hours
- SPEC###-amplitude_analysis.md: 3-4 hours
- IMPL###-audio_ingest_api.md: 2 hours
- IMPL###-amplitude_analyzer_implementation.md: 2-3 hours
- IMPL###-parameter_management.md: 1-2 hours
- Documentation updates (3 files): 1-2 hours
- **Total specification effort: 8-12 hours**

**Implementation (After Specifications Complete):**
- Amplitude analyzer module: 5-7 days
- HTTP API + routes: 3-4 days
- Parameter management: 2-3 days
- Import workflow state machine: 4-5 days
- UI wizard + advanced mode: 7-10 days
- Integration testing: 3-5 days
- **Total implementation: 3-4 weeks**

---

## Next Steps (User Decides)

**Option 1: Proceed with Specification**
1. Review recommendations above
2. Approve or modify architectural choices
3. Begin creating specification documents (8-12 hours)
4. After specs complete, run `/plan` for implementation plan

**Option 2: Proceed Directly to Implementation Planning**
1. Accept specifications "as designed in analysis"
2. Run `/plan wip/_user_story_analysis/00_SUMMARY.md`
3. `/plan` generates:
   - Requirements traceability
   - Test specifications
   - Increment breakdown
4. Begin coding after plan review

**Option 3: Defer/Revise**
1. Provide feedback on analysis
2. Request additional research or alternatives
3. Revisit architectural choices

---

## Risk Summary

**Overall Project Risk:** Low

**Individual Risk Assessments:**
- Amplitude analysis accuracy: Low (RMS adequate, user-adjustable)
- Performance (large libraries): Low (async processing, progress persistence)
- Parameter complexity: Low (progressive disclosure, presets)
- Essentia integration: Low-Medium (subprocess approach mitigates)
- User experience: Low (SSE provides real-time feedback)

**Highest Risk Item:** Essentia integration (Low-Medium)
- **Mitigation:** Subprocess approach avoids FFI memory safety issues
- **Fallback:** Passages without Essentia data marked "no flavor" (can still play)

---

**Navigation:** [← Back to Summary](00_SUMMARY.md)
