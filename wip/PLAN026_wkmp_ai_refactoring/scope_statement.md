# PLAN026: wkmp-ai Refactoring - Scope Statement

**Created:** 2025-11-26
**Source:** wkmp-ai/refactor1126.md

---

## In Scope

### Core Pipeline (Must Implement)
- Step 1: Folder scan with magic byte analysis and file classification
- Step 2: Known files check against existing database entries
- Step 3: SHA-256 hash computation and duplicate detection
- Step 4: Metadata extraction (ID3, Vorbis, MP4, APEv2, BWF, WMA)
- Step 5: Audio decoding with NO AUDIO detection (< 100ms above threshold)
- Step 6: Content type determination per SPEC_content_type_determination.md
- Completed duplicate handling with AtomicBool signaling

### Database Schema (Must Implement)
- Files table extensions (content_type, match_confidence, matched_release_mbid, etc.)
- Passages table with SPEC017 tick-based timing and SPEC002 six timing points
- Songs table with indexed recording_mbid
- Works table for musical compositions
- Artists table with artist_mbid
- Albums table with album_mbid
- Relationship tables: passage_songs, song_artists, passage_albums

### Algorithm Integration (Must Implement)
- am28 album matching algorithm (Stages 2-5)
- Duration-based triage routing
- Quick silence scan for segment estimation
- Single-song path: Chromaprint + AcoustID
- Album path: MusicBrainz search + edition testing
- Content type classification (6 outcomes)

### Architecture (Must Preserve)
- Single-threaded db_access pattern
- Read-only file handling ("played where they lie")
- Network-free post-import (local data caching)
- SSE progress reporting for UI

---

## Out of Scope (Deferred or Not Included)

### Explicitly Deferred
- Folder-level album detection (GAP-01 documented but complex; defer to future increment)
- Musical flavor extraction via Essentia (requires separate integration work)
- Lyric fetching and storage
- Cover art extraction and storage

### Not Part of This Refactoring
- wkmp-ap (Audio Player) changes
- wkmp-pd (Program Director) changes
- wkmp-ui changes beyond SSE consumption
- Database migration tooling (assume manual migration for now)
- Multi-user support
- Cloud/remote database support

### External Dependencies (Not Implemented Here)
- Essentia library integration (placeholder only)
- AcoustID API key management (use existing config)
- MusicBrainz rate limiting infrastructure (reuse am28 implementation)

---

## Assumptions

### Technical Assumptions
1. **am28 code is stable:** The examples/am28/ implementation is tested and ready for integration
2. **Database exists:** SQLite database with base schema already exists from previous wkmp work
3. **Dependencies available:** symphonia, lofty, sha2 crates already in Cargo.toml
4. **API keys configured:** AcoustID and MusicBrainz API keys configured in settings

### Environment Assumptions
1. **Single machine:** Import runs on same machine as database
2. **Local files:** Audio files are accessible via local filesystem
3. **Network available:** Internet connectivity for MusicBrainz/AcoustID lookups during import
4. **Sufficient storage:** Database can grow to accommodate imported file metadata

### Process Assumptions
1. **Incremental import:** Import can be interrupted and resumed
2. **One import at a time:** No concurrent import sessions from multiple sources
3. **User-initiated:** Import started explicitly by user, not automatic

---

## Constraints

### Technical Constraints
- **SQLite single-writer:** All database writes through db_access thread
- **SPEC017 compliance:** All timing in ticks (1 tick = 1/28,224,000 second)
- **SPEC002 compliance:** Passage timing uses six timing points model
- **Rust stable:** No nightly features required

### Performance Constraints
- **Memory:** Hold file scan results in memory (not database until processed)
- **API rate limits:** MusicBrainz 1 req/sec, AcoustID 3 req/sec
- **File I/O:** Read files sequentially (not parallel file access)

### Quality Constraints
- **High confidence target:** ≥80% confidence for successful identification
- **Match tolerance:** 3.0 seconds for track duration matching
- **Artist verification:** 50% Jaro-Winkler similarity threshold

---

## Dependencies

### Existing Code to Reuse
| Component | Location | Status |
|-----------|----------|--------|
| File scanner | wkmp-ai/src/scanner.rs | Keep |
| Magic byte reader | wkmp-ai/src/magic.rs | Keep |
| SSE progress | wkmp-ai/src/sse.rs | Keep |
| db_access pattern | wkmp-ai/src/db_access.rs | Keep |
| Settings/zero-conf | wkmp-common | Keep |

### New Code to Integrate
| Component | Source | Destination |
|-----------|--------|-------------|
| Album matching | wkmp-ai/examples/am28/ | wkmp-ai/src/matching/ |
| Silence detection | am28/silence.rs | wkmp-ai/src/analysis/ |
| Edition testing | am28/edition_tester.rs | wkmp-ai/src/matching/ |
| MusicBrainz client | am28/musicbrainz.rs | wkmp-ai/src/external/ |

### External Dependencies
| Dependency | Purpose | Version |
|------------|---------|---------|
| symphonia | Audio decoding | 0.5.x |
| lofty | Metadata extraction | 0.18.x |
| sha2 | Hash computation | 0.10.x |
| reqwest | HTTP client (MB/AcoustID) | 0.11.x |
| serde_json | JSON serialization | 1.x |
| tokio | Async runtime | 1.x |

---

## Risks (Summary)

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| am28 integration complexity | Medium | High | Incremental integration, test each module |
| Database schema migration | Low | Medium | Document migration steps, test on copy |
| API rate limit issues | Medium | Low | Reuse am28 rate limiting code |
| Memory usage with large libraries | Low | Medium | Process files sequentially |

---

## Success Criteria

### Functional Success
- [ ] All 6 pipeline steps execute correctly
- [ ] Files classified into correct content types
- [ ] Passages created with correct timing
- [ ] Songs/Artists/Albums records created from MusicBrainz data
- [ ] Duplicate detection works bidirectionally
- [ ] Import can be interrupted and resumed

### Quality Success
- [ ] ≥80% of identifiable files get high-confidence match
- [ ] Artist verification prevents wrong-artist matches
- [ ] Track timing within 3s tolerance
- [ ] No data loss on import interruption

### Architecture Success
- [ ] Single-threaded db_access pattern maintained
- [ ] SSE progress updates functional
- [ ] Files remain untouched (read-only)
- [ ] Post-import network-free operation confirmed
