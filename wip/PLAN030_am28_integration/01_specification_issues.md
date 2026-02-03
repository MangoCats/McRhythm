# PLAN030: Specification Issues (Phase 2)

## Analysis Summary

| Severity | Count | Action |
|----------|-------|--------|
| CRITICAL | 0 | - |
| HIGH | 2 | Resolve before implementation |
| MEDIUM | 4 | Track during implementation |
| LOW | 3 | Document, no action needed |

**Decision:** ✅ PROCEED - No blocking issues

---

## HIGH Priority Issues

### HIGH-001: Type Duplication Between am28 and Library

**Location:** `examples/am28/types.rs` vs `src/matching/types.rs`

**Issue:** Both define `Edition` and related types with slightly different structures:
- am28 `Edition` has `name_distance_rank`, `name_distance_score` fields
- Library `Edition` has `artist_credit`, `country`, `status` fields

**Impact:** Merge conflict during Increment 1

**Resolution:**
1. Create unified `Edition` type with all fields
2. Use `Option<>` for fields not always available
3. Document which fields come from which source

---

### HIGH-002: Async vs Sync Execution Model

**Location:** `examples/am28/main.rs:70-80`

**Issue:** am28 uses mixed async/sync:
- `process_single_album()` is async (MusicBrainz API calls)
- CPU-bound work (decoding, silence detection) runs sync in rayon

**Impact:** Need to decide execution model for library

**Resolution:**
1. Keep API async for MusicBrainz calls
2. Use `tokio::task::spawn_blocking()` for CPU-bound work
3. Document expected runtime (tokio)

---

## MEDIUM Priority Issues

### MEDIUM-001: Hardcoded Parameter Grids

**Location:** `examples/am28/constants.rs:50-100`

**Issue:** Parameter grid (12 thresholds × 15 durations = 180) is hardcoded. May not be optimal for all audio.

**Resolution:** Make configurable via `AlbumMatcherConfig` but keep defaults.

---

### MEDIUM-002: MusicBrainz Client Duplication

**Location:** `examples/am28/musicbrainz/api.rs` vs `services/musicbrainz_client.rs`

**Issue:** am28 has its own MusicBrainz client with caching. Library has existing client.

**Resolution:**
1. Extend existing `MusicBrainzClient` with caching
2. Add `comprehensive_search()` method
3. Reuse rate limiting infrastructure

---

### MEDIUM-003: Single-Track Discriminator Thresholds

**Location:** `examples/am28/single_track_discriminator.rs:1-50`

**Issue:** Thresholds for single-track detection are empirically derived but undocumented.

**Resolution:** Document thresholds with rationale in code comments.

---

### MEDIUM-004: Stage 4 Penalty Rationale

**Location:** `examples/am28/constants.rs` (STAGE4_PENALTY_PERCENT = 25.0)

**Issue:** 25% penalty for quiet spot detection results is arbitrary.

**Resolution:** Document in constants.rs that this prevents Stage 4 from claiming 100% matches due to lower reliability.

---

## LOW Priority Issues

### LOW-001: AcoustID Integration Placeholder

**Location:** `examples/am28/main.rs:80`

**Issue:** `_acoustid_api_key` parameter is reserved but unused.

**Resolution:** Keep placeholder, document as future enhancement.

---

### LOW-002: Deprecated Import States

**Location:** am28 doesn't have this issue

**Issue:** Library has deprecated `ImportState` variants used elsewhere.

**Resolution:** Not relevant to this plan (pre-existing issue).

---

### LOW-003: Missing Error Types

**Location:** am28 uses `anyhow::Result` throughout

**Issue:** No custom error types for matching-specific errors.

**Resolution:** Add `AlbumMatchError` enum (already exists in library).

---

## Verification Checklist

### Completeness

- [x] All 12 functional requirements have source code
- [x] All 4 non-functional requirements achievable
- [x] Type definitions complete
- [x] Constants documented

### Ambiguity

- [x] Parameter grid search documented (180 combinations)
- [x] Stage flow documented (2→3→4→5)
- [x] Early-exit behavior documented (20s grace period)
- [ ] Single-track thresholds need documentation (MEDIUM-003)

### Testability

- [x] Stage outputs can be verified
- [x] Match percentages are quantifiable
- [x] Artist similarity is measurable
- [x] Cache hit/miss trackable

### Consistency

- [x] Type names consistent
- [ ] Edition types need merge (HIGH-001)
- [x] Confidence levels consistent with existing

---

## Recommendations

1. **Resolve HIGH-001 first** (Increment 1) - Type merge is foundation
2. **Address HIGH-002 in architecture** - Document async model
3. **Track MEDIUM issues** during implementation
4. **LOW issues** can be deferred or ignored
