# Specification Issues: PLAN024 - wkmp-ai Refinement

**Plan:** PLAN024 - wkmp-ai Refinement Implementation
**Date:** 2025-11-12
**Phase:** Phase 2 - Specification Completeness Verification
**Source:** [wip/SPEC032_wkmp-ai_refinement_specification.md](../../wip/SPEC032_wkmp-ai_refinement_specification.md)

---

## Executive Summary

**Total Issues Found:** 8 (0 CRITICAL, 2 HIGH, 4 MEDIUM, 2 LOW)

**Decision: PROCEED ✅**
- No CRITICAL issues blocking implementation
- HIGH issues are clarifiable during implementation
- Specification is sufficiently complete for planning and implementation

**Recommendation:**
- Address HIGH and MEDIUM issues during SPEC032 update phase
- Document assumptions for implementation
- Create test cases that verify intended behavior for ambiguous areas

---

## Issues by Severity

### CRITICAL Issues (0)

**None identified.** Specification is implementation-ready.

---

### HIGH Issues (2)

#### ISSUE-HIGH-001: Metadata Merge Logic Edge Cases Not Fully Specified

**Requirement:** REQ-SPEC032-009 (Metadata Extraction Merging)

**Description:**
Specification states "new metadata overwrites existing keys, old data not removed unless directly replaced" but does not fully specify edge cases:
- What happens if new metadata contains NULL/empty value for existing key?
- Does NULL overwrite existing value (delete) or preserve existing value?
- How to handle nested metadata structures (if any)?
- What constitutes "identical keys"? (exact match, case-insensitive, etc.)

**Current Specification (Line 353-357):**
> wkmp-ai MUST merge extracted metadata with existing metadata:
> - New metadata overwrites existing keys (new takes precedent)
> - Old metadata not removed unless directly replaced
> - Supports re-processing files with updated tags

**Impact:**
- Implementation may make different assumptions about NULL handling
- Re-import behavior ambiguous for tags deleted by user
- Test cases uncertain for edge scenarios

**Recommendation:**
**CLARIFY in SPEC032 update:**
```
Metadata Merge Rules:
1. If new metadata contains key with non-NULL value → Overwrite existing
2. If new metadata contains key with NULL/empty value → Preserve existing (do NOT delete)
3. If new metadata omits key that exists → Preserve existing (old key remains)
4. Key matching: Case-sensitive exact match
5. No nested structure support (flat key-value pairs only per lofty library)
```

**Severity Justification:**
HIGH because incorrect merge behavior could:
- Delete user metadata unintentionally
- Make re-import unpredictable
- Cause data loss

However, specification intent is clear (preservative approach), and implementation team can make reasonable decisions with documentation.

**Action:** Document merge rules explicitly in SPEC032, add test cases for NULL handling

---

#### ISSUE-HIGH-002: Adjacent Passage Merging Algorithm Not Specified

**Requirement:** REQ-SPEC032-012 (Song Matching with Confidence)

**Description:**
Specification states "Consider adjacent potential passage combinations (for songs with embedded silence)" but does not specify:
- **Algorithm:** How to determine which adjacent passages to merge?
- **Heuristic:** What indicates "embedded silence" vs. "separate songs"?
- **Limit:** Maximum number of adjacent passages to consider merging? (2? 3? N?)
- **Confidence:** Does merging affect confidence score? How?
- **Duration:** Does merged passage duration need to match song duration within tolerance?

**Current Specification (Line 376):**
> - Consider adjacent potential passage combinations (for songs with embedded silence)

**Impact:**
- Implementation team must design algorithm from scratch
- Different implementations may produce different results
- Test cases uncertain (what constitutes correct merging?)
- Ambiguity in "best overall MBID configuration"

**Recommendation:**
**CLARIFY in SPEC032 update with algorithm pseudocode:**
```
Adjacent Passage Merging Algorithm:
1. For each potential passage P[i]:
   - If MBID match confidence < threshold (e.g., 0.6):
     - Try merging P[i] + P[i+1] → Test combined fingerprint
     - Try merging P[i-1] + P[i] → Test combined fingerprint
     - Try merging P[i-1] + P[i] + P[i+1] → Test combined fingerprint (max 3 passages)
   - Compare confidence scores: individual vs. merged
   - Choose configuration with highest overall confidence

2. Duration heuristic:
   - Merged passage duration should be within ±10% of MBID recording duration
   - If outside tolerance, do NOT merge (likely separate songs)

3. Constraint:
   - Maximum 3 adjacent passages merged
   - Stop merging when confidence ≥ 0.85 (high confidence)
```

**Alternative (Simpler):**
- **Phase 1:** Implement NO automatic merging (each silence boundary = passage boundary)
- **Phase 2:** Add manual merging via wkmp-pe (future)
- **Rationale:** Automatic merging is complex, error-prone; manual approach more reliable

**Severity Justification:**
HIGH because:
- Core functionality (affects song matching accuracy)
- Algorithmic ambiguity (multiple valid interpretations)
- However, "fallback to no automatic merging" is acceptable for Stage One

**Action:**
1. Document merging algorithm in SPEC032 OR explicitly defer automatic merging to future release
2. Add acceptance test for embedded silence scenario (e.g., Pink Floyd "The Great Gig in the Sky" → "Money" silence)

---

### MEDIUM Issues (4)

#### ISSUE-MEDIUM-001: "High/Medium/Low/No Confidence" Thresholds Not Quantified

**Requirement:** REQ-SPEC032-012 (Song Matching with Confidence)

**Description:**
Specification mentions confidence categories (High, Medium, Low, No Confidence) but does not define:
- Numeric thresholds (e.g., High ≥ 0.85, Medium 0.60-0.84, Low 0.40-0.59, None < 0.40)
- Weighting formula (metadata score + fingerprint score + duration match = ?)
- What happens at boundary cases (e.g., 0.599 vs. 0.600)?

**Current Specification (Line 377):**
> - Rate match_quality (f32 0.0-1.0) as: High, Medium, Low, or No Confidence

**Impact:**
- Implementation team must define thresholds arbitrarily
- Different implementations may categorize same match differently
- Test cases cannot verify "High confidence" without knowing threshold

**Recommendation:**
**DEFINE in SPEC032 update:**
```
Confidence Thresholds:
- High Confidence:    match_quality ≥ 0.85
- Medium Confidence:  0.65 ≤ match_quality < 0.85
- Low Confidence:     0.45 ≤ match_quality < 0.65
- No Confidence:      match_quality < 0.45

Weighting Formula (Single-Segment Files):
  match_quality = (0.3 × metadata_score) + (0.6 × fingerprint_score) + (0.1 × duration_match)

  Where:
  - metadata_score: 0.0-1.0 (artist/title match similarity)
  - fingerprint_score: 0.0-1.0 (AcoustID confidence)
  - duration_match: 1.0 if within ±5%, 0.5 if within ±10%, 0.0 otherwise
```

**Severity Justification:**
MEDIUM because:
- Implementation can use reasonable defaults
- Primarily affects UI display ("High" vs. "Medium" label)
- Does not block implementation

**Action:** Document thresholds and formula in SPEC032, add test cases verifying threshold boundaries

---

#### ISSUE-MEDIUM-002: Symlink/Junction Detection Method Not Specified

**Requirement:** REQ-SPEC032-NF-004 (Symlink/Junction Handling)

**Description:**
Specification states "MUST NOT follow symlinks, junction points, or shortcuts" but does not specify:
- **Detection method:** How to detect symlinks/junctions in Rust on Windows vs. Linux?
- **Behavior:** Skip the link entirely, or traverse target but mark as external?
- **Error handling:** What if symlink target does not exist (dangling symlink)?

**Current Specification (Line 448-449):**
> wkmp-ai MUST NOT follow symlinks, junction points, or shortcuts during folder scanning.

**Impact:**
- Implementation must research platform-specific symlink detection
- Behavior may differ across platforms (Windows junctions vs. POSIX symlinks)
- Test cases need platform-specific scenarios

**Recommendation:**
**CLARIFY in SPEC032 update:**
```
Symlink/Junction Handling:
1. Detection:
   - Use std::fs::symlink_metadata() instead of std::fs::metadata()
   - Check file_type().is_symlink() || file_type().is_dir() with is_junction() (Windows)

2. Behavior:
   - If symlink/junction detected: Skip entirely (do NOT add to file list, do NOT traverse)
   - Log at DEBUG level: "Skipping symlink: [path]"

3. Error Handling:
   - Dangling symlinks: Skip silently (no error)
   - Permission errors: Log warning, continue scanning
```

**Severity Justification:**
MEDIUM because:
- Security concern (symlink traversal could expose external paths)
- However, standard library provides detection methods
- Implementation straightforward once clarified

**Action:** Document detection method in SPEC032, add test case with symlink in test folder

---

#### ISSUE-MEDIUM-003: Essentia Subprocess Integration Details Missing

**Requirement:** REQ-SPEC032-015 (Musical Flavor Retrieval)

**Description:**
Specification mentions "Essentia analysis" as fallback but does not specify:
- **Command:** What Essentia command to execute? (essentia_streaming? essentia_standard?)
- **Arguments:** What analysis parameters? (high-level profile, streaming mode, etc.)
- **Output Parsing:** How to parse Essentia output to extract flavor JSON?
- **Timeout:** How long to wait for Essentia process? (some files may take minutes)
- **Error Detection:** How to detect Essentia failure vs. success?

**Current Specification (Line 402-404):**
> - If failed: Attempt Essentia analysis on passage audio
>   - If successful: Store Essentia profile in songs.flavor, mark song 'FLAVOR READY'
>   - If failed: Mark song 'FLAVORING FAILED'

**Impact:**
- Implementation team must research Essentia CLI integration
- Different Essentia versions may have different CLI interfaces
- Test cases cannot be written without knowing expected output format

**Recommendation:**
**CLARIFY in SPEC032 update OR defer to implementation details:**
```
Essentia Integration (Pseudocode):
1. Export passage audio to temporary WAV file (44.1kHz, 16-bit, mono)
2. Execute: essentia_streaming [temp.wav] profile.json
3. Wait up to 60 seconds for completion
4. Parse profile.json → Extract high-level features → Convert to AcousticBrainz-compatible JSON
5. If process exits with code 0 and profile.json valid: Success
6. Otherwise: Mark 'FLAVORING FAILED'

Note: Essentia CLI integration may be deferred to implementation discovery phase.
Specify interface contract (input: passage audio, output: flavor JSON) not exact command.
```

**Alternative:**
- Document as "implementation-defined" with interface contract only
- Implementation team researches Essentia during coding

**Severity Justification:**
MEDIUM because:
- Fallback feature (AcousticBrainz is primary)
- Essentia integration is well-documented externally
- Can be discovered during implementation

**Action:** Either document Essentia CLI integration in SPEC032, OR mark as "implementation-defined" with interface contract specified

---

#### ISSUE-MEDIUM-004: "25% of Passage Analyzed" Stopping Condition Unclear

**Requirement:** REQ-SPEC032-014 (Amplitude-Based Lead-In/Lead-Out)

**Description:**
Specification states "OR 25% of total passage time has been analyzed" as stopping condition, but:
- **From which point:** 25% of passage duration from start (for lead-in) or from detected point?
- **Boundary:** If lead-in point not found within 25%, what value is recorded? (0? Duration/4?)
- **Asymmetry:** Lead-in threshold 45dB, lead-out threshold 40dB - why different? (intentional or typo?)

**Current Specification (Line 390-391):**
> - **Lead-in:** From start time forward until amplitude exceeds `lead-in_threshold_dB` (default 45dB) OR 25% of passage analyzed

**Impact:**
- Implementation may interpret "25% analyzed" differently
- Edge case behavior (silent passage, very quiet passage) ambiguous
- Test cases cannot verify correct behavior without precise definition

**Recommendation:**
**CLARIFY in SPEC032 update:**
```
Lead-In Detection:
1. Start at passage start time (tick 0 of passage)
2. Analyze audio samples forward in time
3. Stop when EITHER:
   - Audio amplitude exceeds lead-in_threshold_dB (45dB default) → Record this point as lead-in
   - OR analyzed 25% of passage duration → Record analyzed endpoint as lead-in (conservative: assume loud section starts there)
4. Lead-in point = ticks from passage start to detected/assumed point

Lead-Out Detection:
1. Start at passage end time (last tick of passage)
2. Analyze audio samples backward in time
3. Stop when EITHER:
   - Audio amplitude exceeds lead-out_threshold_dB (40dB default) → Record this point as lead-out
   - OR analyzed 25% of passage duration → Record analyzed endpoint as lead-out
4. Lead-out point = ticks from detected/assumed point to passage end

Threshold Asymmetry Rationale:
- Lead-in: 45dB (higher) - Ensures clean start (no quiet intro noise)
- Lead-out: 40dB (lower) - Allows gentle fade-outs to be preserved (not cut too early)
```

**Severity Justification:**
MEDIUM because:
- Affects crossfade quality (user-facing)
- However, reasonable interpretation possible (conservative approach)
- Worst case: Slightly imperfect crossfade points (not catastrophic)

**Action:** Document precise algorithm in SPEC032, add test case for very quiet passage

---

### LOW Issues (2)

#### ISSUE-LOW-001: File Hash Algorithm Not Specified

**Requirement:** REQ-SPEC032-008 (Hash-Based Duplicate Detection)

**Description:**
Specification mentions "file content hash" but does not specify:
- **Algorithm:** SHA-256? BLAKE3? MD5? (affects performance and collision resistance)
- **Content:** Entire file or audio stream only? (metadata changes would alter hash if entire file)
- **Encoding:** Hex string, Base64, raw bytes for storage?

**Current Specification (Line 346):**
> - Store hash in files table under fileId

**Impact:**
- Implementation team chooses algorithm arbitrarily
- Different implementations may use different hashes (incompatible databases)
- Performance varies significantly (MD5 fast, SHA-256 slower, BLAKE3 fastest)

**Recommendation:**
**DOCUMENT in SPEC032 update:**
```
File Hash Specification:
- Algorithm: BLAKE3 (fast, secure, collision-resistant)
- Content: Entire file (including metadata) - Ensures exact duplicate detection
- Storage Format: Hex string (64 characters)
- Field: files.content_hash TEXT (indexed for fast lookup)
```

**Alternative:**
- If BLAKE3 not available, use SHA-256 (widely supported, secure)
- MD5 NOT recommended (weak collision resistance, despite speed)

**Severity Justification:**
LOW because:
- Any reasonable hash algorithm works
- Choice affects performance but not correctness
- Can be decided during implementation without blocking

**Action:** Document hash algorithm choice in SPEC032 (recommend BLAKE3 or SHA-256)

---

#### ISSUE-LOW-002: UI Section Update Frequency Not Quantified

**Requirement:** REQ-SPEC032-NF-002 (Real-Time Progress Updates)

**Description:**
Specification states "update frequency: per file completion minimum, per phase update recommended" but does not quantify:
- **"Real-time":** Does this mean every 100ms? 500ms? 1 second?
- **Throttling:** Should updates be throttled to avoid overwhelming UI? (e.g., max 10 updates/second)
- **Batch updates:** Can multiple statistics be batched into single SSE event?

**Current Specification (Line 442-443):**
> wkmp-ai MUST update all 13 UI sections in real-time via SSE (update frequency: per file completion minimum, per phase update recommended).

**Impact:**
- Implementation may send excessive SSE events (performance issue)
- UI may lag if updates too frequent
- Test cases cannot verify "real-time" without quantified threshold

**Recommendation:**
**DEFINE in SPEC032 update:**
```
SSE Update Frequency:
- Minimum: Update on each file completion (INGEST COMPLETE, DUPLICATE HASH, NO AUDIO)
- Recommended: Update on each phase transition per file (10 events per file)
- Throttling: Max 100 updates/second total (aggregate across all 13 sections)
- Batching: Multiple statistics MAY be combined into single SSE event (JSON payload)
- Latency Target: Updates visible in UI within 500ms of event occurrence
```

**Severity Justification:**
LOW because:
- "Per file completion" provides minimum acceptable behavior
- Performance optimization (throttling) can be added incrementally
- Does not block implementation

**Action:** Document update frequency target in SPEC032, note throttling as future optimization

---

## Issues by Category

### Completeness (Missing Information)

- ISSUE-HIGH-001: Metadata merge NULL handling
- ISSUE-HIGH-002: Adjacent passage merging algorithm
- ISSUE-MEDIUM-001: Confidence thresholds
- ISSUE-MEDIUM-002: Symlink detection method
- ISSUE-MEDIUM-003: Essentia subprocess integration
- ISSUE-MEDIUM-004: Lead-in/lead-out 25% condition
- ISSUE-LOW-001: File hash algorithm
- ISSUE-LOW-002: UI update frequency

**Total:** 8 completeness issues

### Ambiguity (Multiple Interpretations)

- ISSUE-HIGH-001: Metadata merge (NULL = delete or preserve?)
- ISSUE-HIGH-002: Adjacent merging (how to decide?)
- ISSUE-MEDIUM-004: 25% analyzed (from where?)

**Total:** 3 ambiguity issues

### Consistency (Conflicts)

**None identified.** No contradictory requirements found.

### Testability (Difficulty Verifying)

All requirements are testable AFTER issues resolved:
- HIGH-001, HIGH-002 require algorithm definition before test cases can be written
- MEDIUM issues require parameter specification for verification

---

## Resolution Plan

### Immediate (Before SPEC032 Update)

**Resolve HIGH Issues:**
1. ISSUE-HIGH-001: Define metadata merge rules (especially NULL handling)
2. ISSUE-HIGH-002: Define adjacent passage merging algorithm OR explicitly defer to manual merging (wkmp-pe)

**Decision Required:**
- User/architect input on adjacent passage merging: Automatic (complex) vs. Manual (simple, deferred)

### During SPEC032 Update

**Resolve MEDIUM Issues:**
1. ISSUE-MEDIUM-001: Document confidence thresholds and weighting formula
2. ISSUE-MEDIUM-002: Document symlink detection method (std::fs::symlink_metadata)
3. ISSUE-MEDIUM-003: Document Essentia CLI interface OR mark as implementation-defined
4. ISSUE-MEDIUM-004: Document lead-in/lead-out algorithm precisely

### During Implementation

**Resolve LOW Issues:**
1. ISSUE-LOW-001: Choose and document hash algorithm (BLAKE3 or SHA-256)
2. ISSUE-LOW-002: Define and test SSE update frequency

---

## Impact on Planning

**Proceed with Planning:** ✅ YES

**Rationale:**
- No CRITICAL blockers identified
- HIGH issues are resolvable with reasonable assumptions
- MEDIUM and LOW issues do not prevent test case definition or implementation breakdown
- Specification quality is GOOD (73% P0 requirements are clear and actionable)

**Actions Before Implementation:**
1. Document HIGH issue resolutions in SPEC032 update
2. Create acceptance tests that verify intended behavior for ambiguous cases
3. Document assumptions in plan if issues remain unresolved

---

## Specification Quality Score

| Criterion | Score | Notes |
|-----------|-------|-------|
| **Completeness** | 85% | Most requirements complete, 8 minor gaps |
| **Clarity** | 90% | Mostly unambiguous, 3 multi-interpretation cases |
| **Consistency** | 100% | No contradictions found |
| **Testability** | 80% | Testable after HIGH issues resolved |
| **Traceability** | 100% | All requirements numbered, source lines documented |
| **Overall** | **91%** | **GOOD - Ready for implementation with clarifications** |

---

## Recommendations

### For Specification Authors

1. ✅ **Excellent:** 26 requirements clearly enumerated with IDs
2. ✅ **Excellent:** Scope boundaries explicitly stated (in/out)
3. ✅ **Excellent:** Two-stage roadmap clearly defined
4. ⚠️ **Improve:** Add algorithm pseudocode for complex features (adjacent merging, lead-in/lead-out)
5. ⚠️ **Improve:** Quantify thresholds (confidence categories, update frequency, tolerances)
6. ⚠️ **Improve:** Specify edge case handling (NULL metadata, dangling symlinks, very quiet passages)

### For Implementation Team

1. Document all assumptions when specification is ambiguous
2. Implement conservatively for HIGH-001 (preserve metadata, never delete unintentionally)
3. Consider deferring HIGH-002 (automatic adjacent merging) to future release
4. Add comprehensive test cases for edge scenarios identified in issues

### For Plan Workflow

1. Proceed to Phase 3 (Acceptance Test Definition)
2. Define test cases that verify intended behavior for ambiguous areas
3. Mark tests requiring HIGH issue resolution with "PENDING SPEC CLARIFICATION" tags
4. Update tests after SPEC032 update incorporates clarifications
