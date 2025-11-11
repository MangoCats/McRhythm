# Requirements Index - PLAN026: Technical Debt Resolution

**Total Requirements:** 12 (3 CRITICAL, 8 HIGH, 4 MEDIUM)

## CRITICAL Requirements (Sprint 1)

### REQ-TD-001: Functional Boundary Detection
- **Priority:** CRITICAL
- **Effort:** 4-6 hours
- **Location:** session_orchestrator.rs:232-239
- **Goal:** Replace stub with actual SilenceDetector integration
- **Impact:** Unblocks multi-track album imports

### REQ-TD-002: Audio Segment Extraction
- **Priority:** CRITICAL
- **Effort:** 6-8 hours
- **Location:** song_workflow_engine.rs:252-253
- **Goal:** Implement time-range audio extraction using symphonia
- **Impact:** Enables per-passage fingerprinting

### REQ-TD-003: Remove or Implement Amplitude Analysis
- **Priority:** CRITICAL (User-Facing API)
- **Effort:** 2 hours (remove) OR 8-12 hours (implement)
- **Location:** api/amplitude_analysis.rs:24-35
- **Goal:** Remove stub endpoint returning fake data
- **Impact:** Eliminates misleading API response

---

## HIGH Priority Requirements (Sprint 2)

### REQ-TD-004: MBID Extraction from ID3 Tags
- **Priority:** HIGH
- **Effort:** 4-6 hours
- **Location:** tier1/id3_extractor.rs:208-209
- **Goal:** Extract MusicBrainz ID from UFID frames
- **Impact:** Reduces AcoustID API calls

### REQ-TD-005: Consistency Checker Implementation
- **Priority:** HIGH
- **Effort:** 6-8 hours
- **Location:** tier3/consistency_checker.rs:51-70
- **Goal:** Detect metadata conflicts between sources
- **Impact:** Improves data quality visibility

### REQ-TD-006: Event Bridge session_id Fields
- **Priority:** HIGH
- **Effort:** 2-3 hours
- **Location:** event_bridge.rs:110, 128, 147
- **Goal:** Add session_id to all ImportEvent variants
- **Impact:** Enables proper event correlation in UI

### REQ-TD-007: Flavor Synthesis Implementation
- **Priority:** HIGH
- **Effort:** 4-6 hours
- **Location:** song_workflow_engine.rs:369-370
- **Goal:** Combine multiple flavor sources with weighting
- **Impact:** More accurate musical flavor vectors

### REQ-TD-008: Chromaprint Compressed Fingerprint
- **Priority:** HIGH
- **Effort:** 3-4 hours
- **Location:** tier1/chromaprint_analyzer.rs:93-94
- **Goal:** Use AcoustID-compatible compressed format
- **Impact:** Smaller storage, standard API format

---

## MEDIUM Priority Requirements (Sprint 3 - Deferred)

### REQ-TD-009: Waveform Rendering
- **Priority:** MEDIUM
- **Effort:** 6-8 hours
- **Location:** api/ui.rs:864
- **Status:** Deferred to future release

### REQ-TD-010: Duration Tracking in File Stats
- **Priority:** MEDIUM
- **Effort:** 1-2 hours
- **Location:** session_orchestrator.rs:430
- **Status:** Deferred - nice-to-have feature

### REQ-TD-011: Flavor Confidence Calculation
- **Priority:** MEDIUM
- **Effort:** 2-3 hours
- **Location:** session_orchestrator.rs:521
- **Status:** Deferred - addressed by REQ-TD-007

### REQ-TD-012: Flavor Data Persistence
- **Priority:** MEDIUM
- **Effort:** 2-3 hours
- **Location:** db_repository.rs:173
- **Status:** Deferred - blocked by REQ-TD-007

---

## Implementation Order

**Sprint 1 (Critical):** REQ-TD-001 → REQ-TD-002 → REQ-TD-003
- Total: 12-16 hours
- Deliverable: Multi-track album imports working

**Sprint 2 (High):** REQ-TD-004 → REQ-TD-005 → REQ-TD-006 → REQ-TD-007 → REQ-TD-008
- Total: 19-27 hours
- Deliverable: Metadata quality + event correlation

**Sprint 3 (Medium):** REQ-TD-009, REQ-TD-010, REQ-TD-011, REQ-TD-012
- Total: 10-14 hours
- Status: Deferred to future release
