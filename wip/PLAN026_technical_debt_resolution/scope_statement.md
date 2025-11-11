# Scope Statement - PLAN026: Technical Debt Resolution

## In Scope

### Sprint 1 (Critical) - MUST IMPLEMENT
1. **Boundary Detection Integration** (REQ-TD-001)
   - Replace SessionOrchestrator stub with SilenceDetector calls
   - Enable multi-passage detection within single audio files
   - Configure silence threshold and minimum duration
   - Return actual boundary list (not hardcoded single passage)

2. **Audio Segment Extraction** (REQ-TD-002)
   - Implement `extract_segment()` in AudioLoader
   - Accept time range (start_ticks, end_ticks) parameters
   - Decode only requested time range using symphonia
   - Convert stereo to mono via channel averaging
   - Resample to target sample rate (44.1kHz)

3. **Amplitude Analysis Endpoint** (REQ-TD-003)
   - **Decision:** Remove stub endpoint (Option A recommended)
   - Delete `/analyze/amplitude` route
   - Remove request/response models
   - Update API documentation

### Sprint 2 (High) - SHOULD IMPLEMENT
4. **MBID Extraction** (REQ-TD-004)
   - Attempt UFID frame reading via lofty
   - Fallback to `id3` crate for MP3 files
   - Parse MusicBrainz UFID identifier
   - Document limitations for non-MP3 formats

5. **Consistency Checker** (REQ-TD-005)
   - Modify MetadataFuser to preserve all candidates
   - Implement conflict detection for title/artist/album
   - Use string similarity threshold (strsim < 0.85 = conflict)
   - Return ValidationResult with conflict descriptions

6. **Event Bridge session_id** (REQ-TD-006)
   - Add session_id field to 8 ImportEvent variants
   - Replace `Uuid::nil()` placeholders with actual IDs
   - Update event conversion logic
   - Enable UI event correlation

7. **Flavor Synthesis** (REQ-TD-007)
   - Convert audio-derived flavor to FlavorExtraction
   - Pass all flavor sources to FlavorSynthesizer
   - Implement weighted combination algorithm
   - Calculate synthesis confidence

8. **Chromaprint Compression** (REQ-TD-008)
   - Check if chromaprint-rust exposes compression API
   - Implement base64 encoding if needed
   - Update database schema for compressed storage
   - Use compressed format for AcoustID queries

### Sprint 3 (Medium) - DEFERRED
9-12. **Polish Features** (REQ-TD-009 through REQ-TD-012)
   - Explicitly out of scope for this plan
   - Will be addressed in future release
   - Dependencies on Sprint 2 completion

---

## Out of Scope

### Explicitly Excluded
1. **Waveform Rendering** (REQ-TD-009) - UI polish feature, not critical
2. **Duration Tracking** (REQ-TD-010) - Nice-to-have statistics
3. **Flavor Confidence** (REQ-TD-011) - Superseded by REQ-TD-007
4. **Flavor Persistence** (REQ-TD-012) - Blocked by REQ-TD-007

### Related Work NOT Addressed
- Performance optimization (not identified as technical debt)
- Additional test coverage beyond acceptance criteria
- UI/UX improvements
- Documentation updates (beyond inline code comments)
- Error message improvements
- Logging enhancements

### Architectural Changes NOT Required
- No database schema changes (except REQ-TD-008 fingerprint field)
- No new microservices
- No event bus redesign
- No API breaking changes
- PLAN023 architecture remains unchanged

---

## Dependencies

### External Libraries (Already Available)
- `symphonia` v0.5 - Audio decoding
- `rubato` v0.15 - Resampling
- `lofty` v0.21 - Metadata extraction
- `strsim` v0.11 - String similarity
- `chromaprint-rust` - Audio fingerprinting

### May Need to Add
- `id3` v1.x - Fallback for UFID extraction (REQ-TD-004)

### Internal Components (No Changes)
- `SilenceDetector` - Already fully implemented
- `BoundaryFuser` - Working correctly
- `FlavorSynthesizer` - Exists but needs integration
- Database migrations framework - Established

---

## Success Criteria

### Sprint 1 Deliverable
✅ User can import multi-track album files
✅ Each track detected as separate passage
✅ No stub endpoints returning fake data
✅ All Sprint 1 tests passing (100% coverage)

### Sprint 2 Deliverable
✅ MBID extraction working for MP3 files
✅ Metadata conflicts visible during import
✅ Event streams properly correlated by session_id
✅ Musical flavor synthesis functional
✅ Chromaprint fingerprints in standard format
✅ All Sprint 2 tests passing (100% coverage)

### Overall Success
✅ All 8 requirements implemented (3 CRITICAL + 5 HIGH)
✅ Zero regression in existing functionality
✅ Performance within acceptable bounds:
  - Boundary detection: <200ms per file
  - Segment extraction: <100ms per passage
✅ Technical debt reduced from 45+ markers to <20

---

## Constraints

### Technical Constraints
- Must maintain PLAN023 3-tier architecture
- Must preserve existing database schema compatibility
- Must use existing microservices (no new services)
- Must follow Legible Software principles

### Timeline Constraints
- Sprint 1: 1 week (12-16 hours)
- Sprint 2: 1-2 weeks (19-27 hours)
- Total: 2-3 weeks for all critical and high priority work

### Resource Constraints
- Single developer implementation
- No external API dependencies (except AcoustID)
- Existing test infrastructure (no new frameworks)

---

## Risk Acceptance

### Accepted Risks
1. **MBID Extraction Limited Scope:** May only work for MP3 files
   - Mitigation: AcoustID fingerprinting remains fallback
   - Residual: Low-Medium

2. **Symphonia API Complexity:** Segment extraction may have edge cases
   - Mitigation: Comprehensive test coverage
   - Residual: Low

3. **Sprint 3 Deferred:** Medium priority items not addressed
   - Acceptance: Explicitly deferred to future release
   - Impact: Non-critical features delayed

### Unacceptable Risks
❌ Breaking existing functionality - Will be caught by regression tests
❌ Data corruption - Database transactions protect integrity
❌ Performance regression - Benchmarks will verify acceptable performance
