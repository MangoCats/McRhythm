# Approach Selection: Single Song Import Improvements

**Plan:** PLAN026
**Decision Date:** 2025-12-13

---

## Approaches Evaluated

### Approach A: New Standalone Service

**Description:** Create a completely new `SingleSongClassifier` service separate from existing `ContentTypeClassifier`.

**Pros:**
- Clean separation of concerns
- No risk of breaking album import path
- Easier to test in isolation

**Cons:**
- Code duplication (rate limiting, caching patterns)
- Two code paths to maintain
- Integration complexity with workflow orchestrator

**Risk Assessment:**
- Failure mode: Integration bugs between two classifiers
- Probability: Medium
- Impact: Medium
- Residual risk: Medium

### Approach B: Extend ContentTypeClassifier (RECOMMENDED)

**Description:** Add recording matcher capability to existing `ContentTypeClassifier`, integrating with current `classify_single_song()` method.

**Pros:**
- Reuses existing infrastructure (rate limiting, caching, error handling)
- Single code path for classification
- Already integrated with workflow orchestrator
- Follows existing patterns

**Cons:**
- Must not break existing functionality
- Slightly more complex single-song path

**Risk Assessment:**
- Failure mode: Regression in existing functionality
- Probability: Low (mitigated by tests)
- Impact: Medium
- Mitigation: Unit tests, integration tests, existing test suite
- Residual risk: Low

### Approach C: Decorator Pattern

**Description:** Wrap existing `ContentTypeClassifier` with decorator that adds recording search fallback.

**Pros:**
- No modification to existing code
- Easy to enable/disable

**Cons:**
- Adds indirection
- Complex interaction with async code
- Doesn't leverage existing internal state

**Risk Assessment:**
- Failure mode: Complexity leads to bugs
- Probability: Medium
- Impact: Low
- Residual risk: Low-Medium

---

## Decision

**RECOMMENDATION: Approach B (Extend ContentTypeClassifier)**

**RISK-BASED JUSTIFICATION:**

Approach B has lowest residual risk (Low) after mitigation:
- Failure modes identified: Regression in album path
- Mitigations: Comprehensive test suite, incremental changes, existing tests as safety net
- Residual risk: Low

Approach A has higher risk (Medium):
- Integration between two classifiers introduces coupling bugs
- Code duplication increases maintenance burden

Approach C has moderate risk (Low-Medium):
- Async decorator pattern adds complexity
- Indirection obscures control flow

Quality characteristics equivalent between approaches (all achieve requirements).

Per CLAUDE.md Decision-Making Framework: Risk (primary) → Quality (secondary) → Effort (tertiary).
Approach B selected based on lowest residual risk.

---

## Architecture Decision Record (ADR)

**ADR-026-001: Extend ContentTypeClassifier for Recording Search**

**Status:** Accepted

**Date:** 2025-12-13

**Context:**
The single-song import path needs MusicBrainz recording search as fallback when AcoustID fails. Three approaches were considered: new standalone service, extend existing classifier, or decorator pattern.

**Decision:**
Extend the existing `ContentTypeClassifier` to add recording matcher capability, integrated into the `classify_single_song()` method.

**Consequences:**

*Positive:*
- Reuses proven infrastructure (rate limiting, error handling)
- Single integration point with workflow orchestrator
- Follows established code patterns in wkmp-ai

*Negative:*
- ContentTypeClassifier grows in complexity
- Must ensure no regression in album path (mitigated by tests)

*Neutral:*
- Requires porting validation functions from am29 examples

---

## Implementation Strategy

Based on Approach B, the implementation will:

1. **Port string utilities from am29:**
   - `normalize_for_comparison()` → `utils/string_similarity.rs`
   - `best_jaro_winkler_ratio()` → `utils/string_similarity.rs`

2. **Create RecordingMatcher:**
   - New file: `services/recording_matcher.rs`
   - Uses existing `MusicBrainzClient.search_recordings()`
   - Applies Jaro-Winkler scoring to candidates

3. **Extend ContentTypeClassifier:**
   - Modify `classify_single_song()` to call RecordingMatcher as fallback
   - Integrate with IdentityResolver for multi-source fusion

4. **Add recording cache:**
   - New file: `db/recording_cache.rs`
   - Cache key: normalized (artist, title)
   - TTL: 7 days (configurable)
