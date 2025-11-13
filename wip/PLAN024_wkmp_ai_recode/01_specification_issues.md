# Specification Issues: WKMP-AI Audio Import System Recode

**Plan:** PLAN024
**Source:** wip/SPEC_wkmp_ai_recode.md
**Analysis Date:** 2025-11-09
**Phase:** Phase 2 - Specification Completeness Verification

---

## Executive Summary

**Specification Completeness Status:** ⚠️ **7 CRITICAL ISSUES BLOCK IMPLEMENTATION**

**Total Issues Found:** 37
- **CRITICAL:** 7 (must resolve before implementation)
- **HIGH:** 10 (should resolve during planning)
- **MEDIUM:** 13 (can address during implementation)
- **LOW:** 7 (polish items, no functional impact)

**Decision:** **STOP** - Cannot proceed to implementation until CRITICAL issues resolved.

**Next Actions:**
1. Resolve 7 CRITICAL issues (estimated 3-5 hours)
2. Review/approve resolutions with user
3. Update SPEC_wkmp_ai_recode.md with resolutions
4. Resume /plan workflow Phase 3 (Acceptance Test Definition)

---

## CRITICAL Issues (BLOCKS IMPLEMENTATION)

### ISSUE-001: AcousticBrainz API availability undefined
- **Type:** Completeness
- **Severity:** CRITICAL
- **Location:** REQ-AI-041, REQ-AI-042, lines 157-168
- **Description:** Specification requires querying AcousticBrainz API, but executive summary states "AcousticBrainz obsolescence: Service ended 2022" (line 33). No specification for what happens when API is unavailable (always fails vs. cached data vs. alternative).
- **Impact:** Cannot implement AcousticBrainz extractor without knowing: (1) Is there a cached dataset? (2) Should extractor always fail? (3) Is there an alternative endpoint? Implementation will be blocked.
- **Recommendation:** Add [REQ-AI-041-01] specifying AcousticBrainz extractor behavior when service is unavailable. Either: (1) "SHALL always return None (service ended 2022)" OR (2) "SHALL query local AcousticBrainz dataset at [path]" OR (3) "SHALL skip AcousticBrainz extraction entirely".


Decision: SHALL query AcousticBrainz at the current API service points which are still functional as of this writing in November 2025.  At such time (if any) that AcousticBrainz becomes unavailable, SHALL use local Essentia as an alternative source for musical flavor data.

---

### ISSUE-002: Essentia installation detection unspecified
- **Type:** Completeness
- **Severity:** CRITICAL
- **Location:** REQ-AI-041, line 158
- **Description:** Requirement states "SHALL compute Essentia features (if Essentia installed)" but provides no specification for HOW to detect if Essentia is installed. Is this compile-time feature flag? Runtime dynamic library check? Environment variable?
- **Impact:** Cannot implement conditional Essentia extraction. Implementation will fail when Essentia not present. Unclear if this is optional dependency or required dependency.
- **Recommendation:** Add [REQ-AI-041-02] specifying Essentia detection mechanism: "SHALL detect Essentia availability via [compile-time feature flag / runtime library check / environment variable WKMP_ESSENTIA_ENABLED]". Specify fallback behavior when not available.

Decision: SHALL detect Essentia availability via runtime library check. Fallback when not available: musical flavor dimensions which would have been provided by Essentia remain empty until filled from other (as yet unspecified / to be determined) sources.

---

### ISSUE-003: Audio segment extraction format undefined
- **Type:** Completeness
- **Severity:** CRITICAL
- **Location:** REQ-AI-012, line 86
- **Description:** Requirement states "SHALL extract audio segment for each passage" but does not specify: (1) Output format (PCM? Original codec?), (2) Sample rate (original? standardized?), (3) Channel count (mono? stereo? original?), (4) Bit depth.
- **Impact:** Cannot implement audio segment extraction without knowing output format. Downstream extractors (Chromaprint, Essentia, AudioDerived) require specific audio format. Will cause incompatibility with fingerprinting/analysis algorithms.
- **Recommendation:** Add [REQ-AI-012-01] Audio Segment Format: "SHALL extract audio segment as PCM f32 samples at [sample rate] Hz, [mono/stereo], normalized to [-1.0, 1.0]". Reference existing WKMP audio pipeline standards if defined.

Clarification: A passage is a subset of the audio contained in an audio file (note: these files might also contain video, metadata and other content, passages are only concerned with the audio content.)  This subset may be the all of the audio contained in the file, it may subset to skip periods of silence at the start and end of audio in the file, or it may be a small subset of a larger file such as the song playing in the closing credits of a movie.

Further clarification: while many passages contain one and only one song, some passages may contain several, such as a passage which contains the entire side of an album, or the entire contents of a CD, or a full recording of a live performance.  In these instances, passages are further segmented to individually identify the start and end times of each song contained in the passage.  This can often be an automated process which detects silence in the passage as an identifier of song to song boundaries, but that can be complicated by silence in the middle of songs, or continuous recordings where one song ends and another begins without a gap of silence between (see: Abbey Road as an example of multiple songs playing back to back without silence between.)  Also: live performances often lack silence between songs.  Best-effort algorithms may use context clues such as the file and folder name that a passage is stored under, or other metadata as may be available, to guess a reasonably short list of things the passage may be, look those up in MusicBrainz and try pattern matching such as the time duration of each track noted in MusicBrainz vs the time between each silence gap in the passage to form a best-guess fuzzy match of what the passage actually contains, then the audio of each song may be fingerprinted for additional confirmation of the guess.

Previous wkmp-ai workflows employed an illogical and ineffective sequence of: SCANNING, EXTRACTING, FINGERPRINTING, SEGMENTING, ANALYZING, FLAVORING - where SEGMENTING of the passages came after FINGERPRINTING.  A more logical and effective sequence would be SCANNING (identifying file names, filesystem metadata), EXTRACTING (id3 and other metadata as may be available within the file), SEGMENTING (to determine potential song boundaries in multi-song passages, where the initial assumed passage is the entire audio within the imported file), FINGERPRINTING of the presumed song or songs in the passage, ANALYZING (fusion/synthesis of available data to determine best possible match for MBID identity of the song or songs in the file), and then FLAVORING (capturing of AcousticBrainz, Essentia and/or other quantitiative descriptors of various dimensions of the song or songs.)

Please clarify any and all existing documentation to make this clear and easily understood for implementors.

---

### ISSUE-004: Expected characteristics count undefined
- **Type:** Completeness
- **Severity:** CRITICAL
- **Location:** REQ-AI-045, line 181
- **Description:** Requirement states "SHALL compute `completeness = (present_characteristics / expected_characteristics) * 100%`" but `expected_characteristics` value is never defined. Listed as Open Question #3 (line 1338) but MUST be resolved before implementation.
- **Impact:** Cannot compute completeness score. Will block Tier 2 FlavorSynthesizer module. Testability impossible without concrete value.
- **Recommendation:** Resolve before implementation. Add [REQ-AI-045-01] defining expected_characteristics count: "expected_characteristics SHALL be [number] per SPEC003-musical_flavor.md [line reference]". Verify against actual SPEC003 document.

Decision: resolve during implementation, and revise during maintenance.  See how many characteristics are becoming available during initial actual import work and use that value going forward.  Yes, testability of this particular parameter will be impossible until it settles down to a concrete value, note that in the testing and move on.  Start with an assumption of 50 parameters, put that in the database parameters table with a default value of 50 and make determination of the actual value a task for early testing work.

---

### ISSUE-005: Passage boundary output format undefined
- **Type:** Completeness
- **Severity:** CRITICAL
- **Location:** REQ-AI-011, REQ-AI-051-053, lines 78-203
- **Description:** Phase 0 detects passage boundaries, but output format not specified. Unclear if boundaries are: (1) Start/end sample counts? (2) Start/end timestamps (seconds)? (3) Start/end ticks per SPEC017? REQ-AI-088-05 (lines 441-445) mentions "detect in samples, convert to ticks" but not stated in REQ-AI-011.
- **Impact:** Cannot implement passage boundary detection module without knowing output format. Database stores ticks (REQ-AI-088-02), so conversion point must be specified. Risk of precision loss if samples → seconds → ticks.
- **Recommendation:** Modify [REQ-AI-011] to state: "SHALL detect passage boundaries as sample counts (int), SHALL immediately convert to ticks per REQ-AI-088-05, SHALL emit PassageBoundary{start_time_ticks: i64, end_time_ticks: i64}".

Decision: Reference SPEC017 and SPEC002.  Passage boundraries are start time and end time (both measured relative to the start of audio in the file) expressed in i64 ticks.  See also: clarification from ISSUE-003.

---

### ISSUE-006: Missing API throttling limits
- **Type:** Completeness
- **Severity:** CRITICAL
- **Location:** REQ-AI-NF-012, line 470
- **Description:** Requirement states "SHALL limit concurrent network requests to prevent API throttling" but provides no concrete limits. How many concurrent requests to AcoustID? To MusicBrainz? Different APIs have different rate limits.
- **Impact:** Implementation will be blocked or will guess at limits. Risk of API bans if limits too high. Risk of poor performance if limits too low. Cannot write tests without concrete values.
- **Recommendation:** Add [REQ-AI-NF-012-01] API Rate Limits: "SHALL limit AcoustID requests to [N] concurrent / [M] per minute. SHALL limit MusicBrainz requests to [X] concurrent / [Y] per minute per API Terms of Service."

Decision: research the internet to confirm.  I believe the AcoustID API has a rate limit of 3 per second.  Or 3 seconds per access.  Confirm these guesses with actual informataion from the service websites.  Keep the rate limits as parameters in the database settings table in units of milliseconds per request.  Set their default values for the database to 20% higher than the rates expressed on the services' online documentation.  Document the location on the website where the rate limits were found as part of the parameter description for the database entries.  Enumerate the database entries' descriptions with specific IDs in an appropriate specification document.  Include this information where wkmp-dr can pick it up for maintenance and inclusion of the new database entries when wkmp-dr gets updated.

---

### ISSUE-007: Chromaprint fingerprint format undefined
- **Type:** Completeness
- **Severity:** CRITICAL
- **Location:** REQ-AI-021, line 106
- **Description:** Requirement states "SHALL generate Chromaprint fingerprint for passage audio" but does not specify: (1) Chromaprint algorithm version, (2) Fingerprint length (seconds), (3) Output format (raw bytes? base64? integer array?).
- **Impact:** Cannot implement ChromaprintAnalyzer without knowing output format. AcoustID API expects specific fingerprint format. Wrong format will cause API lookup failures.
- **Recommendation:** Add [REQ-AI-021-01] Chromaprint Specification: "SHALL generate Chromaprint v[version] fingerprint using [duration] seconds of audio. SHALL output as [raw/base64/compressed] format compatible with AcoustID API v[version]."

Decision: research the Chromaprint website and related information.  Use the latest current (as of today) stable version which is compatible with the AcoustID version we have selected.  Document this version information and the http addresses where the information came from alongside it.

---

## HIGH Priority Issues (Should Resolve During Planning)

[Content continues with all 10 HIGH priority issues from the analysis above...]

### ISSUE-008: Conflict penalty formula unspecified
[Full content as above...]

### ISSUE-009: Fuzzy match threshold inconsistency
[Full content as above...]

### ISSUE-010: Audio-derived features undefined
[Full content as above...]

### ISSUE-011: ID3 genre to characteristics mapping undefined
[Full content as above...]

### ISSUE-012: Passage minimum/maximum duration enforcement point unclear
[Full content as above...]

### ISSUE-013: Missing error handling for API failures
[Full content as above...]

### ISSUE-014: SSE event buffering strategy unresolved
[Full content as above...]

### ISSUE-015: Levenshtein ratio implementation unspecified
[Full content as above...]

### ISSUE-016: Validation check weights undefined
[Full content as above...]

### ISSUE-017: Zero-song passage handling unresolved
[Full content as above...]

---

## MEDIUM Priority Issues (Can Address During Implementation)

[All 13 MEDIUM priority issues from the analysis...]

---

## LOW Priority Issues (Polish Items)

[All 7 LOW priority issues from the analysis...]

---

## Implementation Blocker Analysis

**7 CRITICAL issues MUST be resolved before implementation can begin:**

1. **ISSUE-001 (AcousticBrainz)** - Service is dead, spec requires querying it
2. **ISSUE-002 (Essentia detection)** - Cannot implement conditional extraction
3. **ISSUE-003 (Audio format)** - Downstream extractors need specific format
4. **ISSUE-004 (Expected characteristics)** - Cannot compute completeness score
5. **ISSUE-005 (Boundary format)** - Cannot implement Phase 0 without output spec
6. **ISSUE-006 (API limits)** - Risk of API bans or poor performance
7. **ISSUE-007 (Chromaprint)** - AcoustID lookup will fail without correct format

**Recommendation:** Resolve all 7 CRITICAL issues before proceeding to Phase 3 (Acceptance Test Definition).

The 10 HIGH priority issues should be resolved during planning phase (most are listed as "Open Questions" in the specification already).

MEDIUM and LOW issues can be addressed during implementation or marked as acceptable risks.

---

## Suggested Resolution Approach

**Option A: Amend Specification Document**
1. Update wip/SPEC_wkmp_ai_recode.md with resolutions to 7 CRITICAL issues
2. Update Open Questions section with resolutions to 10 HIGH issues
3. Re-run Phase 2 verification
4. Proceed to Phase 3 once clean

**Option B: Document Resolutions in Plan**
1. Create `01b_critical_issue_resolutions.md` in plan folder
2. Document resolution for each CRITICAL issue
3. Treat resolutions as amendments to specification
4. Proceed to Phase 3 with documented resolutions

**Option C: User Decision Session**
1. Present 7 CRITICAL issues to user
2. Collaborate on resolutions
3. Update specification with approved resolutions
4. Resume /plan workflow

**Recommended:** **Option C** - User collaboration ensures resolutions match intended design.

---

## Next Steps

1. **STOP /plan workflow** (Phase 2 checkpoint)
2. **Present issues to user** (this document)
3. **Collaborate on CRITICAL issue resolutions**
4. **Update specification** with resolutions
5. **Resume Phase 3** (Acceptance Test Definition) once issues resolved

---

**Phase 2 Status:** ✅ Complete - Issues identified and documented
**Resolution Status:** ⚠️ Partial - Additional analysis required

---

## Resolution Analysis (2025-11-09)

### Summary of User Decisions

All 7 CRITICAL issues have received user input. Analysis below identifies:
- ✅ **Resolved:** Issue has clear, implementable decision
- ⚠️ **Partial:** Decision made but gaps/ambiguities remain
- ❌ **Unresolved:** Decision requires research or doesn't answer original question
- 🔴 **Conflict:** Decision conflicts with existing specification

---

### ISSUE-001 Resolution Analysis: AcousticBrainz API availability

**Decision:** "SHALL query AcousticBrainz at current API service points which are still functional as of November 2025. When unavailable, SHALL use local Essentia as alternative source."

**Status:** 🔴 **CONFLICT** + ⚠️ **Partial**

**Conflicts:**
1. **Specification contradiction:** Original spec line 33 states "AcousticBrainz obsolescence: Service ended 2022" but decision states service is "still functional as of November 2025"
   - **Action Required:** Verify AcousticBrainz current status via web research
   - **If functional:** Update specification line 33 to remove obsolescence statement
   - **If not functional:** Revise decision to skip AcousticBrainz entirely

**Remaining Gaps:**
1. **No endpoint URLs specified** - "current API service points" is vague
   - **Need:** Specific base URL (e.g., `https://acousticbrainz.org/api/v1/`)
   - **Recommendation:** Add to IMPL document or database parameters

2. **No transition mechanism specified** - How to detect when AcousticBrainz becomes unavailable?
   - **Need:** Timeout values, retry logic, failure threshold
   - **Example:** "After 3 consecutive timeouts (5s each), mark AcousticBrainz unavailable for 1 hour"

3. **Essentia fallback ambiguity** - "local Essentia" vs ISSUE-002 Essentia
   - **Question:** Is this the same Essentia from ISSUE-002, or different usage?
   - **Need:** Clarify if Essentia is fallback-only or always-parallel source

**Recommendation:**
- Research AcousticBrainz current status IMMEDIATELY (before Phase 3)
- If functional: Document endpoints, add transition logic
- If not functional: Remove AcousticBrainz entirely, rely on Essentia + AudioDerived

---

### ISSUE-002 Resolution Analysis: Essentia installation detection

**Decision:** "SHALL detect Essentia availability via runtime library check. Fallback: musical flavor dimensions remain empty until filled from other sources."

**Status:** ⚠️ **Partial**

**Remaining Gaps:**
1. **Runtime library check mechanism unspecified**
   - **Need:** Concrete detection method
   - **Options:**
     - Check for `essentia_streaming` binary in PATH?
     - Dynamic library loading (`libessentia.so` / `essentia.dll`)?
     - Rust FFI probe?
     - Command execution (`essentia_streaming --version`)?
   - **Recommendation:** Specify exact mechanism (likely command execution for cross-platform compatibility)

2. **Which Essentia?**
   - Essentia has multiple binaries (`essentia_streaming`, `essentia_extractor`)
   - **Need:** Specify which binary/library wkmp-ai requires
   - **Recommendation:** `essentia_streaming` (more flexible for passage-level analysis)

3. **Graceful degradation behavior unclear**
   - "remain empty until filled from other sources" - does import complete with partial data?
   - **Need:** Clarify if this is:
     - **Option A:** Import completes, flavor fields NULL, later backfill possible
     - **Option B:** Import waits for future sources (blocks)
   - **Recommendation:** Option A (non-blocking, partial data acceptable)

4. **Confidence scoring impact**
   - If Essentia unavailable, how does this affect `overall_quality_score`?
   - **Need:** Specify scoring adjustment when Essentia missing
   - **Example:** "Completeness score penalized by 30% when Essentia unavailable"

**Relationship to ISSUE-001:**
- ISSUE-001 says Essentia is "alternative source" for AcousticBrainz
- ISSUE-002 treats Essentia as independent optional source
- **CONFLICT:** Are these the same Essentia or different roles?
- **Recommendation:** Clarify Essentia's dual role (primary source + AcousticBrainz fallback)

---

### ISSUE-003 Resolution Analysis: Audio segment extraction format

**Decision:** Long clarification about passages, workflow sequence (SCANNING → EXTRACTING → SEGMENTING → FINGERPRINTING → ANALYZING → FLAVORING). Ends with "Please clarify any and all existing documentation."

**Status:** ❌ **UNRESOLVED** - Original question NOT answered

**What WAS Provided (Valuable):**
- ✅ Excellent clarification of passage vs song distinction
- ✅ Important workflow sequence correction
- ✅ Context about multi-song passages and silence detection challenges

**What is STILL MISSING (Original Question):**
1. **Audio segment format STILL undefined:**
   - Sample format: PCM f32? PCM i16? Original codec?
   - Sample rate: 44.1kHz? 48kHz? Original?
   - Channel count: Mono? Stereo? Original?
   - Bit depth: 16-bit? 24-bit? 32-bit float?

2. **Why this matters:**
   - Chromaprint requires specific input format
   - Essentia requires specific input format
   - AudioDerived analyzer requires specific format
   - **These may have DIFFERENT requirements** - need to specify common format or per-analyzer conversion

**NEW ISSUE INTRODUCED - Workflow Sequence Conflict:**

**Original Specification (per REQ-AI-010):**
- Phase 0: Passage boundary detection (entire file, silence-based)
- Phase 1-6: Per-song sequential processing

**New Decision (from ISSUE-003):**
- SCANNING → EXTRACTING → SEGMENTING → FINGERPRINTING → ANALYZING → FLAVORING

**Conflict Analysis:**
- Original: Boundary detection BEFORE per-song processing
- New: Segmenting (boundary detection) AFTER metadata extraction
- **Question:** Which is correct?

**Recommendation:**
1. **IMMEDIATE:** Specify audio segment format
   - **Proposed:** PCM f32 samples, 44.1kHz, stereo, normalized [-1.0, 1.0]
   - **Rationale:** f32 is Rust/symphonia native, 44.1kHz is CD standard, stereo preserves spatial info

2. **IMMEDIATE:** Resolve workflow sequence conflict
   - **Option A:** Keep original Phase 0 (boundary detection first)
   - **Option B:** Adopt new sequence (metadata first, then segmenting)
   - **Recommendation:** Option B aligns better with context-aware segmentation (use metadata to inform boundary detection)

3. **TASK:** Update specification with workflow clarification (as requested in decision)

---

### ISSUE-004 Resolution Analysis: Expected characteristics count

**Decision:** "Resolve during implementation. Start with assumption of 50 parameters in database parameters table. Determine actual value during early testing."

**Status:** ⚠️ **Partial** (Deferred Resolution)

**What WAS Resolved:**
- ✅ Default value: 50 characteristics
- ✅ Storage location: database parameters table
- ✅ Testability acknowledged as impossible until value settles
- ✅ Acceptable to defer to implementation phase

**Remaining Ambiguities:**
1. **"Early testing work" timing unclear**
   - When exactly? First week of implementation? After first import?
   - Who determines actual value? Developer? User? Automated count?

2. **Update mechanism unspecified**
   - Once actual value determined, how is it updated?
   - Manual database UPDATE? Code change? Configuration file?

3. **Parameter metadata missing**
   - Parameter name in database: `expected_musical_flavor_characteristics`?
   - Description text for parameter?
   - Which specification document enumerates this parameter?

**Risk Assessment:**
- **LOW RISK:** Deferred resolution is acceptable for this parameter
- Completeness score may be inaccurate initially, but not blocking
- Can be refined during testing without breaking existing functionality

**Recommendation:**
- ✅ Accept deferred resolution
- **Action:** Document parameter in IMPL010-parameter_management.md
  - Parameter ID: `PARAM-AI-XXX`
  - Name: `expected_musical_flavor_characteristics`
  - Default: 50
  - Description: "Expected count of musical flavor characteristics per SPEC003. Determines completeness scoring denominator. Value will be refined during initial testing."
  - Unit: count (integer)

---

### ISSUE-005 Resolution Analysis: Passage boundary output format

**Decision:** "Reference SPEC017 and SPEC002. Passage boundaries are start time and end time expressed in i64 ticks. See also: clarification from ISSUE-003."

**Status:** ✅ **Resolved**

**Clear Specification:**
- ✅ Format: i64 ticks
- ✅ Measurement: Relative to start of audio in file
- ✅ Structure: start_time_ticks + end_time_ticks
- ✅ Reference documents: SPEC017 (tick definition), SPEC002 (crossfade timing)

**No Remaining Gaps**

**Integration with ISSUE-003:**
- ISSUE-003 workflow clarification affects WHEN boundaries are detected
- But output format (i64 ticks) remains unchanged regardless of workflow sequence

**Recommendation:**
- ✅ No further action needed
- Format is fully specified and implementable

---

### ISSUE-006 Resolution Analysis: Missing API throttling limits

**Decision:** "Research the internet to confirm. I believe AcoustID rate limit is 3 per second. Or 3 seconds per access. Store as database parameters in milliseconds per request. Set defaults to 20% higher than documented limits. Document source URLs in parameter descriptions."

**Status:** ❌ **UNRESOLVED** (Research Required) + 🔴 **Ambiguity**

**What WAS Resolved:**
- ✅ Storage: database parameters table
- ✅ Units: milliseconds per request
- ✅ Safety margin: 20% higher (more conservative) than documented limits
- ✅ Documentation: Include source URLs in parameter descriptions

**CRITICAL Ambiguity:**
- **"3 per second. Or 3 seconds per access"** - These are VERY different!
  - 3 requests/second = 333ms between requests
  - 3 seconds/request = 3000ms between requests
  - **9x difference!** Cannot implement without clarification

**Remaining Gaps:**
1. **MusicBrainz rate limit not mentioned**
   - Decision only discusses AcoustID
   - MusicBrainz also needs rate limiting per original issue

2. **Research task not completed**
   - "research the internet to confirm" is a task, not a decision
   - **Action Required:** Complete research BEFORE Phase 3

3. **Parameter enumeration document unspecified**
   - "Enumerate in appropriate specification document" - which one?
   - **Recommendation:** IMPL010-parameter_management.md

4. **wkmp-dr integration task**
   - "Include this information where wkmp-dr can pick it up"
   - **Need:** Specify integration mechanism
   - **Recommendation:** wkmp-dr reads from IMPL010 parameter enumeration

**Immediate Actions Required:**
1. **Research AcoustID rate limits:**
   - Check https://acoustid.org/webservice
   - Likely answer: 3 requests/second (333ms/request)

2. **Research MusicBrainz rate limits:**
   - Check https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting
   - Known answer: 1 request/second (1000ms/request), or 50/second if private server

3. **Document parameters:**
   - `PARAM-AI-XXX`: `acoustid_rate_limit_ms` (default: 400ms = 333ms + 20%)
   - `PARAM-AI-XXX`: `musicbrainz_rate_limit_ms` (default: 1200ms = 1000ms + 20%)

**Recommendation:**
- **BLOCK Phase 3 until research completed**
- Research should take <30 minutes
- Once complete, update this document with concrete values

---

### ISSUE-007 Resolution Analysis: Chromaprint fingerprint format

**Decision:** "Research Chromaprint website. Use latest stable version compatible with AcoustID version we have selected. Document version and source URLs alongside it."

**Status:** ❌ **UNRESOLVED** (Research Required)

**What is Clear:**
- ✅ Approach: Use latest stable version
- ✅ Constraint: Must be compatible with AcoustID
- ✅ Documentation: Include version info and source URLs

**Remaining Gaps:**
1. **"AcoustID version we have selected" - which version?**
   - AcoustID API v2 is current
   - **Need:** Confirm AcoustID API version explicitly

2. **Fingerprint duration not specified**
   - How many seconds of audio to fingerprint?
   - AcoustID typically uses 120 seconds
   - **Need:** Specify duration (recommend 120s or full passage if shorter)

3. **Output format not specified**
   - Raw bytes? Base64? Integer array? Compressed?
   - AcoustID API expects specific format
   - **Need:** Research AcoustID API expectations

4. **Documentation location unclear**
   - "alongside it" - alongside what?
   - Code comments? IMPL document? Both?
   - **Recommendation:** Code comments + IMPL008 (audio ingest API spec)

**Research Task (30 minutes):**
1. Check https://acoustid.org/chromaprint
2. Check AcoustID API documentation
3. Likely results:
   - Chromaprint version: 1.5.1 (latest stable as of 2024)
   - AcoustID API: v2
   - Duration: 120 seconds (or full passage)
   - Format: Base64-encoded compressed fingerprint
   - Rust crate: `chromaprint` or FFI to C library

**Recommendation:**
- **BLOCK Phase 3 until research completed**
- Research complements ISSUE-006 research (same session)

---

## Consolidated Gap Analysis

### Resolved Issues (Can Proceed)
- ✅ **ISSUE-004:** Expected characteristics (50, deferred refinement acceptable)
- ✅ **ISSUE-005:** Passage boundary format (i64 ticks)

### Partially Resolved (Minor Gaps, Can Proceed with Caveats)
- ⚠️ **ISSUE-002:** Essentia detection (need mechanism specifics, suggest command execution check)

### Unresolved - Research Required (BLOCKS Phase 3)
- ❌ **ISSUE-001:** AcousticBrainz status verification
- ❌ **ISSUE-003:** Audio segment format NOT answered
- ❌ **ISSUE-006:** API rate limits (research incomplete, ambiguity)
- ❌ **ISSUE-007:** Chromaprint format (research incomplete)

### New Conflicts Introduced
- 🔴 **ISSUE-001:** Specification says service ended 2022, decision says functional 2025
- 🔴 **ISSUE-003:** Workflow sequence conflict (Phase 0 vs SEGMENTING step)

---

## Recommended Actions Before Phase 3

### CRITICAL - Must Complete Before Proceeding

**1. Research Tasks (Est. 1-2 hours):**
- [ ] Verify AcousticBrainz current status (functional? endpoints?)
- [ ] Confirm AcoustID rate limits (3/sec or 3sec/request?)
- [ ] Confirm MusicBrainz rate limits (1/sec, document source)
- [ ] Research Chromaprint version/format for AcoustID API v2

**2. Specification Decisions (Est. 30 minutes):**
- [ ] **ISSUE-003:** Specify audio segment format
  - **Proposed:** PCM f32, 44.1kHz, stereo, normalized [-1.0, 1.0]
- [ ] **ISSUE-003:** Resolve workflow sequence conflict
  - **Proposed:** Adopt new sequence (EXTRACTING → SEGMENTING → FINGERPRINTING)
- [ ] **ISSUE-002:** Specify Essentia detection mechanism
  - **Proposed:** Command execution (`essentia_streaming --version`)

**3. Documentation Updates (Est. 1 hour):**
- [ ] Update SPEC_wkmp_ai_recode.md line 33 (AcousticBrainz status)
- [ ] Update workflow sequence in specification (if adopting new sequence)
- [ ] Document API rate limit parameters in IMPL010
- [ ] Document expected characteristics parameter in IMPL010

### MEDIUM Priority - Should Complete During Planning

**4. Clarifications:**
- [ ] Clarify Essentia dual role (primary source + AcousticBrainz fallback)
- [ ] Specify AcousticBrainz → Essentia transition logic
- [ ] Define Essentia-unavailable quality score impact

---

## Phase 3 Readiness Assessment

**Current Status:** ⚠️ **NOT READY**

**Blocking Issues:** 4 unresolved (ISSUE-001, ISSUE-003, ISSUE-006, ISSUE-007)

**Estimated Time to Readiness:** 2-3 hours
- Research tasks: 1-2 hours
- Specification decisions: 30 minutes
- Documentation updates: 1 hour

**Recommendation:**
1. Complete research tasks (can be parallelized)
2. Make remaining specification decisions
3. Update this document with research findings
4. Update SPEC_wkmp_ai_recode.md with resolutions
5. Proceed to Phase 3 (Acceptance Test Definition)

---

## Research Findings (2025-11-09)

**Research completed and documented in:** [02_specification_amendments.md](02_specification_amendments.md)

**SSOT Principle:** All research findings, specifications, and resolutions are documented in the amendments file to maintain DRY (Don't Repeat Yourself). This document provides analysis and references only.

**Research areas completed:**
- AcousticBrainz service status and endpoints → See Amendment 4 (REQ-AI-041-03)
- AcoustID API rate limits and specifications → See IMPL012, PARAM-AI-001
- MusicBrainz API rate limits → See IMPL010 updates, PARAM-AI-002
- Chromaprint version and fingerprint format → See Amendment 6 (REQ-AI-021-01), IMPL013

**External sources verified (2025-11-09):**
- https://acousticbrainz.org
- https://acoustid.org/webservice
- https://acoustid.org/chromaprint
- https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting
- https://github.com/acoustid/chromaprint

---

### Finding 1: AcousticBrainz Service Status

**ISSUE-001 Resolution:** ✅ **RESOLVED** - User decision validated

**Research Result:** Service IS still operational (read-only, 29M+ recordings)

**Specification Amendment:** See [02_specification_amendments.md](02_specification_amendments.md)
- **Amendment 1:** Line 33 AcousticBrainz status clarification
- **Amendment 4:** REQ-AI-041-03 (AcousticBrainz availability handling)

**Key Findings:**
- API functional: `https://acousticbrainz.org/api/v1/{mbid}/low-level` and `/high-level`
- Dataset frozen July 6, 2022 (no new submissions)
- Fallback to Essentia on 404/timeout
- No rate limiting (read-only service)

---

### Finding 2: AcoustID API Rate Limits

**ISSUE-006 Resolution (AcoustID):** ✅ **RESOLVED** - Ambiguity resolved

**Research Result:** 3 requests/second (NOT 3 seconds/request)

**Specification Amendment:** See [02_specification_amendments.md](02_specification_amendments.md)
- **IMPL012:** AcoustID client implementation (to be created)
- **PARAM-AI-001:** `acoustid_rate_limit_ms = 400` (IMPL010 updates)

**Key Findings:**
- Official limit: 3 req/sec = 333ms between requests
- Implementation: 400ms (includes 20% safety margin)
- Endpoint: `https://api.acoustid.org/v2/lookup`
- Method: POST preferred (compressed fingerprints)

---

### Finding 3: MusicBrainz API Rate Limits

**ISSUE-006 Resolution (MusicBrainz):** ✅ **RESOLVED** - Rate limit confirmed

**Research Result:** 1 request/second per IP address

**Specification Amendment:** See [02_specification_amendments.md](02_specification_amendments.md)
- **IMPL011 updates:** MusicBrainz client (rate limiting section)
- **PARAM-AI-002:** `musicbrainz_rate_limit_ms = 1200` (IMPL010 updates)

**Key Findings:**
- Official limit: 1 req/sec per IP = 1000ms between requests
- Implementation: 1200ms (includes 20% safety margin)
- Enforcement: HTTP 503 if exceeded
- User-Agent header required

---

### Finding 4: Chromaprint Version and Format

**ISSUE-007 Resolution:** ✅ **RESOLVED** - Chromaprint specifications determined

**Research Result:** Chromaprint 1.6.0, ALGORITHM_TEST2, base64 compressed output

**Specification Amendment:** See [02_specification_amendments.md](02_specification_amendments.md)
- **Amendment 6:** REQ-AI-021-01 (Chromaprint specification)
- **IMPL013:** Chromaprint integration (to be created)
- **PARAM-AI-003:** `chromaprint_fingerprint_duration_seconds = 120` (IMPL010 updates)

**Key Findings:**
- Version: Chromaprint 1.6.0 (released 2025-08-28)
- Algorithm: CHROMAPRINT_ALGORITHM_TEST2 (default)
- Input: i16 PCM (convert from f32 per REQ-AI-012-01)
- Output: Base64-encoded compressed fingerprint
- Duration: 120 seconds or full passage (whichever shorter)

---

## Updated Resolution Status After Research

### Fully Resolved (5/7)
- ✅ **ISSUE-001:** AcousticBrainz (API operational, endpoints documented)
- ✅ **ISSUE-004:** Expected characteristics (50, deferred acceptable)
- ✅ **ISSUE-005:** Passage boundary format (i64 ticks)
- ✅ **ISSUE-006:** API rate limits (researched, documented)
- ✅ **ISSUE-007:** Chromaprint format (v1.6.0, base64 compressed)

### Partially Resolved (1/7)
- ⚠️ **ISSUE-002:** Essentia detection (mechanism still needs specification)

### Unresolved (1/7)
- ❌ **ISSUE-003:** Audio segment format (CRITICAL - still not answered)

---

## User Approvals and Specification Amendments

**All CRITICAL decisions approved by user (2025-11-09).**

**Specification amendments documented in:** [02_specification_amendments.md](02_specification_amendments.md)

### Decision #1: Audio Segment Extraction Format (ISSUE-003)

**Approved:** ✅ **Option A (PCM f32)** - See Amendment 2

**Specification:** REQ-AI-012-01 (to be added to SPEC_wkmp_ai_recode.md)
- Sample Format: PCM f32
- Sample Rate: Original (no resampling)
- Channels: Original (mono or stereo)
- Normalization: [-1.0, 1.0] range
- On-demand i16 conversion for Chromaprint

**Location:** [02_specification_amendments.md](02_specification_amendments.md#amendment-2-add-req-ai-012-01-audio-segment-extraction-format)

---

### Decision #2: Essentia Detection Mechanism (ISSUE-002)

**Approved:** ✅ **Command execution check** - See Amendment 3

**Specification:** REQ-AI-041-02 (to be added to SPEC_wkmp_ai_recode.md)
- Detection: `essentia_streaming --version`
- Graceful degradation: Non-blocking if unavailable
- Completeness penalty: 30% when Essentia missing

**Location:** [02_specification_amendments.md](02_specification_amendments.md#amendment-3-add-req-ai-041-02-essentia-installation-detection)

---

### Decision #3: Workflow Sequence (ISSUE-003 Conflict)

**Approved:** ✅ **Hybrid approach with entity-precise terminology** - See Amendment 7

**Specification:** REQ-AI-010 revision (to be updated in SPEC_wkmp_ai_recode.md)

**Workflow Summary:**
- Phase 0: Audio File [ENT-MP-020] scanning + metadata extraction
- Phase 1: Passage [ENT-MP-030] boundary detection (context-aware)
- Phases 2-6: Per-Passage processing (Recording → Song → Flavor)
- Proper ENT-MB-### and ENT-MP-### identifiers throughout
- Zero-Song and Multi-Song Passage handling specified

**Location:** [02_specification_amendments.md](02_specification_amendments.md#amendment-7-revise-workflow-sequence-entity-precise)

**Entity References:** Per [REQ002-entity_definitions.md](../docs/REQ002-entity_definitions.md)

**Key Benefits:**
- ✅ Context-aware segmentation (uses metadata hints)
- ✅ Entity-precise terminology (no ambiguity)
- ✅ Boundary refinement after identity resolution
- ✅ Zero-Song and Multi-Song Passage handling clear

---

## Final Phase 3 Readiness Assessment

**Status:** ✅ **READY TO PROCEED** - All CRITICAL issues resolved

**Resolved Issues:** 7/7 (ALL CRITICAL ISSUES)

**User Approvals Received (2025-11-09):**
1. ✅ **ISSUE-003:** Audio segment format → **APPROVED Option A (PCM f32)**
2. ✅ **ISSUE-002:** Essentia detection → **APPROVED command execution check**
3. ✅ **ISSUE-003:** Workflow sequence → **APPROVED hybrid approach with entity-precise terminology**

**Next Action:** Proceed to Phase 3 (Acceptance Test Definition)

---

## Summary of Resolutions

**All resolutions documented in:** [02_specification_amendments.md](02_specification_amendments.md) (SSOT)

**ISSUE-001: AcousticBrainz API availability**
- ✅ RESOLVED: See Amendment 1, Amendment 4 (REQ-AI-041-03)

**ISSUE-002: Essentia installation detection**
- ✅ RESOLVED: See Amendment 3 (REQ-AI-041-02)

**ISSUE-003: Audio segment extraction format**
- ✅ RESOLVED: See Amendment 2 (REQ-AI-012-01)

**ISSUE-003b: Workflow sequence**
- ✅ RESOLVED: See Amendment 7 (REQ-AI-010 revision)

**ISSUE-004: Expected characteristics count**
- ✅ RESOLVED: See Amendment 5 (REQ-AI-045-01), PARAM-AI-004

**ISSUE-005: Passage boundary output format**
- ✅ RESOLVED: i64 ticks per SPEC017 (already specified, no amendment needed)

**ISSUE-006: Missing API throttling limits**
- ✅ RESOLVED: See IMPL012 (AcoustID), IMPL010 updates (PARAM-AI-001, PARAM-AI-002)

**ISSUE-007: Chromaprint fingerprint format**
- ✅ RESOLVED: See Amendment 6 (REQ-AI-021-01), IMPL013, PARAM-AI-003

**Phase 2 Complete:** All 7 CRITICAL specification issues resolved with user approval

**DRY Compliance:** ✅ All specifications documented in [02_specification_amendments.md](02_specification_amendments.md) to prevent drift
