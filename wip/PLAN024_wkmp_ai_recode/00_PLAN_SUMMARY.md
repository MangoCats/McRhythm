# PLAN024: WKMP-AI Audio Import System Recode - Summary

**Plan Number:** PLAN024
**Created:** 2025-11-09
**Source Specification:** wip/SPEC_wkmp_ai_recode.md (1375 lines, 77 requirements)
**Status:** ✅ **Phases 1-3 Complete** (Week 1 Deliverable)

---

## Executive Summary

**Objective:** Create systematic implementation plan for WKMP-AI audio import system ground-up recode with 3-tier hybrid fusion architecture.

**Approach:** Test-first, specification-driven planning per /plan workflow

**Key Achievements:**
- ✅ 7/7 CRITICAL specification issues resolved
- ✅ 77 requirements analyzed (72 original + 5 amendments)
- ✅ 100% requirement → test coverage achieved
- ✅ DRY principle maintained (SSOT in amendments document)
- ✅ Entity-precise terminology per REQ002

**Status:** Ready for user review and approval before proceeding to implementation planning (Phases 4-8)

---

## Phase 1: Input Validation and Scope Definition

**Status:** ✅ Complete

**Deliverables:**
1. **requirements_index.md** - 72 requirements cataloged in tabular format
2. **scope_statement.md** - In/out scope, assumptions, constraints, success metrics
3. **dependencies_map.md** - Existing components, external libraries, integration points

**Key Findings:**
- 72 requirements total (61 functional, 11 non-functional)
- 91.7% high priority requirements
- Ground-up recode with no legacy code copying
- 3-tier hybrid fusion architecture (7 Tier 1 extractors, 4 Tier 2 fusers, 3 Tier 3 validators)
- Per-song sequential processing workflow
- 12-14 week estimated implementation duration

**Critical Dependencies Identified:**
- SPEC031 SchemaSync (verify availability in wkmp-common)
- SPEC017 tick conversion utilities (verify availability)
- Chromaprint Rust binding (identify crate or FFI approach)
- Essentia integration method (command execution or FFI)

---

## Phase 2: Specification Completeness Verification

**Status:** ✅ Complete

**Deliverables:**
1. **01_specification_issues.md** - 37 issues identified (7 CRITICAL, 10 HIGH, 13 MEDIUM, 7 LOW)
2. **02_specification_amendments.md** - SSOT for all resolutions (DRY compliance)

**CRITICAL Issues Resolved (7/7):**

**ISSUE-001: AcousticBrainz API availability**
- ✅ RESOLVED: Service operational (read-only), 29M+ recordings available
- Amendment 1: Line 33 status clarification
- Amendment 4: REQ-AI-041-03 (availability handling)

**ISSUE-002: Essentia installation detection**
- ✅ RESOLVED: Command execution check via `essentia_streaming --version`
- Amendment 3: REQ-AI-041-02 (detection mechanism)

**ISSUE-003: Audio segment extraction format**
- ✅ RESOLVED: PCM f32, original sample rate/channels, normalized [-1.0, 1.0]
- Amendment 2: REQ-AI-012-01 (format specification)

**ISSUE-003b: Workflow sequence conflict**
- ✅ RESOLVED: Hybrid approach with entity-precise terminology
- Amendment 7: REQ-AI-010 revision (Phase 0-6 workflow)

**ISSUE-004: Expected characteristics count**
- ✅ RESOLVED: Default 50, deferred refinement during implementation
- Amendment 5: REQ-AI-045-01 (completeness scoring)

**ISSUE-005: Passage boundary output format**
- ✅ RESOLVED: i64 ticks per SPEC017 (already specified)

**ISSUE-006: Missing API throttling limits**
- ✅ RESOLVED: AcoustID 400ms (3/sec + 20%), MusicBrainz 1200ms (1/sec + 20%)
- PARAM-AI-001, PARAM-AI-002 (database parameters)

**ISSUE-007: Chromaprint fingerprint format**
- ✅ RESOLVED: Chromaprint 1.6.0, base64 compressed, ALGORITHM_TEST2
- Amendment 6: REQ-AI-021-01 (Chromaprint specification)

**Research Completed:**
- AcousticBrainz: Operational, read-only, 29M+ recordings (accessed 2025-11-09)
- AcoustID: 3 req/sec rate limit, API v2, requires key
- MusicBrainz: 1 req/sec rate limit, HTTP 503 enforcement
- Chromaprint: v1.6.0 (latest stable), base64 output, 120s duration default

**Specification Amendments:**
- 7 amendments to SPEC_wkmp_ai_recode.md (to be executed after plan approval)
- 2 new IMPL documents to create (IMPL012, IMPL013)
- 4 database parameters for IMPL010 updates

**DRY Compliance:** ✅ All specifications in 02_specification_amendments.md (SSOT), plan documents reference only

---

## Phase 3: Acceptance Test Definition

**Status:** ✅ Complete

**Deliverables:**
1. **03_acceptance_tests.md** - 77 requirements with Given/When/Then tests

**Test Coverage:**
- Requirements tested: 77/77 (100%)
- Traceability matrix: Verified 100% coverage
- Test data specifications: 8 audio files, 3 database fixtures, 7 API mocks defined

**Test Categories:**
1. **Per-Song Import Workflow** (5 tests)
2. **Identity Resolution** (5 tests)
3. **Musical Flavor Synthesis** (7 tests)
4. **Passage Boundary Detection** (2 tests)
5. **Quality Validation** (2 tests)
6. **SSE Event Streaming** (2 tests)
7. **UI Progress Reporting** (1 composite test)
8. **Database Initialization** (2 tests)
9. **Database Schema** (2 tests)
10. **Time Representation** (1 test)
11. **Non-Functional Requirements** (6 tests)

**Test Approach:**
- Unit tests: Per-module (>90% coverage target)
- Integration tests: Per-phase (Phase 0-6)
- System tests: End-to-end (happy path + error scenarios)
- Acceptance tests: Requirements verification (this document)

**Test Data Requirements:**
- Audio files: 8 files (single-track, multi-track, corrupted, ambiguous, etc.)
- Database fixtures: 3 states (empty, old schema, populated)
- Mock API responses: 7 scenarios (success, error, rate limit, 404, etc.)

---

## Key Decisions and Approvals

**User Approvals (2025-11-09):**

1. ✅ **Audio segment format:** PCM f32, original sample rate/channels
2. ✅ **Essentia detection:** Command execution check (`essentia_streaming --version`)
3. ✅ **Workflow sequence:** Hybrid approach with entity-precise terminology per REQ002
4. ✅ **DRY compliance:** Option C (amendments document as SSOT)

**Entity Terminology:**
- Audio File [ENT-MP-020]: Physical file on disk
- Passage [ENT-MP-030]: Defined span within Audio File with timing metadata
- Recording [ENT-MB-020]: MusicBrainz unique audio entity (MBID)
- Work [ENT-MB-030]: Musical composition (0-many per Song)
- Artist [ENT-MB-040]: Performer/creator (0-many per Song with weights)
- Song [ENT-MP-010]: WKMP entity = Recording + Works + Artists

**Workflow Summary:**
```
Audio File [ENT-MP-020]
  ↓ Phase 0: Scanning + Metadata Extraction
  ↓ Phase 1: Boundary Detection
Passage(s) [ENT-MP-030] (initially zero Songs)
  ↓ Phase 2: Chromaprint Fingerprinting
Recording [ENT-MB-020] MBIDs identified
  ↓ Phase 3: Identity Resolution (MusicBrainz query)
Recording [ENT-MB-020] + Works [ENT-MB-030] + Artists [ENT-MB-040]
  ↓ Phase 4: Song Creation
Song(s) [ENT-MP-010] linked to Passage [ENT-MP-030]
  ↓ Phase 5: Musical Flavor Synthesis
  ↓ Phase 6: Quality Validation & Boundary Refinement
Final Passage(s) [ENT-MP-030] with Musical Flavor
```

---

## Documentation Deliverables

**PLAN024 Folder Contents:**

1. **00_PLAN_SUMMARY.md** (this file) - Executive summary
2. **requirements_index.md** - 77 requirements cataloged
3. **scope_statement.md** - Scope, assumptions, constraints
4. **dependencies_map.md** - Dependencies and integration points
5. **01_specification_issues.md** - Phase 2 analysis and resolutions
6. **02_specification_amendments.md** - SSOT for all specification changes
7. **03_acceptance_tests.md** - Given/When/Then tests (100% coverage)

**Total:** 7 planning documents (Phases 1-3 complete)

**SSOT Compliance:**
- Specifications: 02_specification_amendments.md (single source of truth)
- Requirements: wip/SPEC_wkmp_ai_recode.md (to be updated after approval)
- Entity definitions: docs/REQ002-entity_definitions.md (referenced, not duplicated)
- Plan documents: Reference SSOT, no specification duplication

---

## Next Steps (Phases 4-8)

**Pending User Approval:**
1. Review Phase 1-3 deliverables
2. Approve 7 specification amendments (02_specification_amendments.md)
3. Approve acceptance test approach (03_acceptance_tests.md)
4. Authorize proceeding to Phases 4-8 (implementation planning)

**After Approval:**

**Phase 4: Approach Selection (Week 2)**
- Module architecture design (Tier 1/2/3 separation)
- Technology selection (Chromaprint crate, Essentia integration method)
- Database strategy (SPEC031 compliance verification)

**Phase 5: Implementation Breakdown (Week 2)**
- Module-level task breakdown
- Dependency ordering (what must be built first)
- Interface contracts between modules

**Phase 6: Effort and Schedule Estimation (Week 2)**
- Per-module effort estimates
- Critical path identification
- Milestone definitions

**Phase 7: Risk Assessment and Mitigation Planning (Week 3)**
- Technical risks (API rate limits, Essentia integration, Bayesian fusion correctness)
- Schedule risks (12-14 week estimate validation)
- Mitigation strategies

**Phase 8: Plan Documentation and Approval (Week 3)**
- Final implementation plan document
- Resource allocation
- Go/no-go decision gate

**Estimated Timeline:**
- Phases 1-3: Week 1 ✅ **COMPLETE**
- Phases 4-6: Week 2 (pending approval)
- Phases 7-8: Week 3 (pending approval)
- Implementation: Weeks 4-17 (12-14 week estimate)

---

## Risk Summary

**High Risk Areas (Require Attention):**

1. **API Rate Limiting**
   - Risk: AcoustID/MusicBrainz throttling slows imports
   - Mitigation: 20% safety margins, exponential backoff, fallback strategies

2. **Bayesian Fusion Complexity**
   - Risk: Mathematical incorrectness in identity resolution
   - Mitigation: Unit tests with hand-verified calculations, reference implementation validation

3. **SPEC031 Dependency**
   - Risk: SchemaSync not implemented in wkmp-common
   - Mitigation: Verify availability in Phase 4, implement if missing (add to scope)

4. **Essentia Integration**
   - Risk: FFI binding complexity or runtime detection failure
   - Mitigation: Command execution check (approved), fallback to AudioDerived always available

**Medium Risk Areas:**

5. **Chromaprint Rust Binding**
   - Risk: No pure Rust crate available
   - Mitigation: FFI to C library via chromaprint-sys or custom bindings

6. **AcousticBrainz Dataset Limitation**
   - Risk: Only 29M recordings (not comprehensive), no new additions
   - Mitigation: Essentia fallback for missing recordings (approved)

7. **Test Data Acquisition**
   - Risk: Need specific test files (known MBIDs, specific durations, etc.)
   - Mitigation: Use public domain recordings, create synthetic test data

---

## Success Metrics

**Phase 1-3 Success (Week 1):**
- ✅ 100% requirement coverage achieved (77/77)
- ✅ All CRITICAL issues resolved (7/7)
- ✅ DRY compliance verified (SSOT maintained)
- ✅ Entity precision enforced (REQ002 compliance)
- ✅ User approvals obtained (all decisions)

**Implementation Success (Future):**
- 100% acceptance tests passing
- >90% code coverage (per REQ-AI-NF-032)
- Zero-configuration startup working (SPEC031)
- All 77 requirements implemented and verified
- Performance targets met (import time, throughput)

---

## Open Questions (For Phase 4)

**Technical:**
1. SPEC031 availability in wkmp-common? (VERIFY in Phase 4)
2. SPEC017 tick conversion utilities available? (VERIFY in Phase 4)
3. Which Chromaprint Rust crate to use? (DECIDE in Phase 4)
4. Essentia binary: `essentia_streaming` or `essentia_extractor`? (RECOMMEND: streaming)

**Process:**
5. When to execute specification amendments? (After plan approval)
6. Create IMPL012, IMPL013 before or during implementation? (RECOMMEND: before)
7. Integration with wkmp-ui for SSE events? (COORDINATE in Phase 5)

---

## Recommendations

**For User Review:**
1. **Review 02_specification_amendments.md thoroughly**
   - 7 amendments to SPEC_wkmp_ai_recode.md
   - 2 new IMPL documents (IMPL012, IMPL013)
   - 4 database parameters (PARAM-AI-001 through 004)

2. **Verify entity-precise workflow (Amendment 7)**
   - Phase 0-6 sequence with ENT-### identifiers
   - Zero-Song and Multi-Song Passage handling

3. **Approve acceptance test approach (03_acceptance_tests.md)**
   - 100% requirement coverage
   - Test data specifications
   - Test execution strategy

**For Implementation:**
1. **Execute amendments first (before implementation)**
   - Update SPEC_wkmp_ai_recode.md
   - Create IMPL012, IMPL013
   - Update IMPL010 with 4 parameters

2. **Verify dependencies (Phase 4)**
   - SPEC031 SchemaSync availability
   - SPEC017 tick utilities
   - Chromaprint binding selection

3. **Follow test-first approach**
   - Write acceptance tests first
   - Implement to pass tests
   - Achieve >90% coverage target

---

## Conclusion

**Phases 1-3 Complete:** ✅

All Week 1 deliverables produced:
- Requirements analysis (77 requirements)
- Specification completeness verification (7 CRITICAL issues resolved)
- Acceptance test definitions (100% coverage)

**DRY Compliance:** ✅ SSOT maintained in 02_specification_amendments.md

**Entity Precision:** ✅ REQ002 compliance throughout

**Recommendation:** **Approve proceeding to Phases 4-8** (implementation planning)

**Estimated Timeline:**
- Phase 4-6: Week 2 (approach selection, breakdown, estimates)
- Phase 7-8: Week 3 (risk assessment, final plan)
- Implementation: Weeks 4-17 (12-14 weeks)

---

**Document Version:** 1.0
**Last Updated:** 2025-11-09
**Phase 1-3 Status:** ✅ COMPLETE - Awaiting user review and approval
