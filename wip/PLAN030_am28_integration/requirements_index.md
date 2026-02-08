# PLAN030: Requirements Index

## Source: am28 Example Code Analysis

| Req ID | Type | Description | Source File | Priority |
|--------|------|-------------|-------------|----------|
| REQ-AM-001 | Functional | Decode audio file to PCM samples | utils/audio.rs | P0 |
| REQ-AM-002 | Functional | Extract metadata (ID3 tags + filename parsing) | metadata.rs:1-581 | P0 |
| REQ-AM-003 | Functional | Search MusicBrainz for candidate releases | musicbrainz/api.rs | P0 |
| REQ-AM-004 | Functional | Group releases into editions by track pattern | matching/edition.rs | P0 |
| REQ-AM-005 | Functional | Stage 2: Parameter grid search (180 combinations) | stages/stage2.rs | P0 |
| REQ-AM-006 | Functional | Stage 3: Over-segmentation assembly (DP) | stages/stage3.rs | P1 |
| REQ-AM-007 | Functional | Stage 4: Quiet spot detection (RMS) | stages/stage4.rs | P1 |
| REQ-AM-008 | Functional | Stage 5: Extra track merging | stages/stage5.rs | P2 |
| REQ-AM-009 | Functional | Artist verification (Jaro-Winkler) | matching/validation.rs | P0 |
| REQ-AM-010 | Functional | Early-exit optimization on 100% match | utils/early_exit.rs | P1 |
| REQ-AM-011 | Functional | MusicBrainz response caching | musicbrainz/cache.rs | P1 |
| REQ-AM-012 | Functional | Single-track discriminator | single_track_discriminator.rs | P0 |
| REQ-NF-001 | Non-Func | Async/await compatible (tokio) | N/A | P0 |
| REQ-NF-002 | Non-Func | Library API (no CLI) | N/A | P0 |
| REQ-NF-003 | Non-Func | Integrate with existing MB client | N/A | P1 |
| REQ-NF-004 | Non-Func | Memory efficient audio streaming | N/A | P2 |

## Existing Types (from PLAN026)

Already defined in `wkmp-ai/src/matching/types.rs`:

| Type | Purpose | Lines |
|------|---------|-------|
| `MatchingStage` | Stage identifier (Stage2-5) | 13-46 |
| `MatchedTrack` | Per-track match details | 54-73 |
| `Edition` | MusicBrainz edition metadata | 76-98 |
| `AlbumMatchResult` | Complete match result | 100-137 |

## am28 Types to Migrate

From `wkmp-ai/examples/am28/types.rs`:

| Type | Purpose | Lines | Action |
|------|---------|-------|--------|
| `CacheMode` | Cache configuration | 19-27 | NEW |
| `CacheConfig` | Cache settings | 30-34 | NEW |
| `CachedSearch` | Cached API response | 37-45 | NEW |
| `CachedRelease` | Cached release details | 48-56 | NEW |
| `CacheStats` | Hit/miss tracking | 74-120 | NEW |
| `MBSearchResponse` | API response wrapper | 127-131 | NEW |
| `MBRelease` | Release from search | 137-153 | MERGE with existing |
| `MBArtistCredit` | Artist credit | 156-160 | NEW |
| `MBArtist` | Artist info | 163-167 | NEW |
| `MBReleaseDetails` | Release with tracks | 173-182 | NEW |
| `MBMedia` | Medium (disc) | 185-191 | NEW |
| `MBRecording` | Recording info | 194-200 | NEW |
| `MBTrack` | Track info | ~201+ | NEW |
| `Edition` | Edition metadata | ~300+ | MERGE with existing |
| `SilenceCache` | Pre-computed silence | ~400+ | NEW |
| `CandidateTestResult` | Single test result | ~500+ | NEW |
| `EditionTestResult` | Edition test result | ~600+ | NEW |
| `ValidationResult` | Final result | ~700+ | MERGE with AlbumMatchResult |

## Constants to Migrate

From `wkmp-ai/examples/am28/constants.rs`:

| Constant | Value | Purpose |
|----------|-------|---------|
| `DEFAULT_THRESHOLD_DB` | -48.0 | Silence threshold |
| `DEFAULT_MIN_DURATION_SECS` | 0.5 | Min silence duration |
| `MATCH_TOLERANCE_SECS` | 3.0 | Track match tolerance |
| `MIN_ARTIST_SIMILARITY` | 0.50 | Jaro-Winkler threshold |
| `THRESHOLD_VALUES` | [-42...-66 dB] | Grid search thresholds (12) |
| `MIN_DURATION_VALUES` | [0.1...2.0s] | Grid search durations (15) |
| `STAGE4_PENALTY_PERCENT` | 25.0 | Quiet spot penalty |
| `EARLY_EXIT_GRACE_PERIOD_SECS` | 20 | Grace period after 100% |
| `MB_RATE_LIMIT_MS` | 1000 | MusicBrainz rate limit |
