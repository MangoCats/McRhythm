# Analysis Results: wkmp-ai Refactoring Specification Review

**Analysis Date:** 2025-11-26
**Document Analyzed:** wkmp-ai/refactor1126.md
**Analysis Method:** 8-Phase Multi-Agent Workflow (/think command)
**Related Documents:** REQ002-entity_definitions.md, IMPL001-database_schema.md, SPEC_content_type_determination.md, am28 example

---

## Executive Summary

The refactor1126.md specification provides a solid foundation for migrating am28 into wkmp-ai, but several gaps, ambiguities, and conflicts must be resolved before implementation planning.

**Critical Findings:**
1. **GAP-01 RESOLVED:** Folder-level album detection - am28 adaptation documented (pre-segmented files, order determination, set-based matching)
2. **MISSING TABLE:** Works table not defined despite REQ002 requirements
3. **STATUS CONFLICT:** Two incompatible status models in specification
4. **SCHEMA CONFLICT:** Recording MBID uniqueness violates entity definition semantics
5. **AMBIGUOUS:** Lead-in/lead-out determination algorithm undefined

**Quick Navigation:**
- [Gaps Identified](#1-gaps-identified): 9 gaps requiring specification additions
- [Ambiguities](#2-ambiguities-identified): 7 ambiguities requiring clarification
- [Conflicts](#3-conflicts-identified): 6 conflicts requiring resolution
- [Proposed Resolutions](#4-proposed-resolutions): Recommendations for each issue

---

## 1. GAPS IDENTIFIED

### GAP-01: Folder-Level Album Detection (CRITICAL)

**Problem:** User requirement explicitly includes "individual song recording files - often collected in folders which collect all tracks from an album edition." Neither refactor1126.md nor SPEC_content_type_determination.md addresses this scenario.

**Current State:**
- am28: Processes single concatenated album files
- Single-song path: Processes individual files via Chromaprint
- No algorithm for: "These 12 MP3 files in this folder are album X"

**Required:**
- Phase to detect folder contains album tracks
- Algorithm to correlate multiple files to single MusicBrainz Release
- Handling for missing tracks (folder has 10 of 12 expected tracks)
- Handling for extra files (folder has album + bonus tracks + sound checks)

**Impact:** Without this, a significant portion of user's library cannot be correctly processed.

---

### GAP-02: Works Table Missing

**Problem:** REQ002 ENT-MB-030 defines Work entity with MBID. Songs table references `work_id` foreign key, but no Works table schema exists.

**Required Schema:**
```
works:
  guid           | TEXT      | PRIMARY KEY - UUID
  work_mbid      | TEXT      | NOT NULL UNIQUE - MusicBrainz Work ID
  title          | TEXT      | Work title
  composer       | TEXT      | Composer/author name(s)
  created_at     | TIMESTAMP | Record creation time
  updated_at     | TIMESTAMP | Record last update time
```

**Relationship:** Songs.work_id → Works.guid (Many-to-one per ENT-CARD-045)

---

### GAP-03: Sample Rate Storage Missing

**Problem:** SPEC017 tick conversion requires `sample_rate` but files table Additional columns list omits it.

**Required:** Add `sample_rate INTEGER` to files table (already mentioned in specification but in wrong section - needs to be in main table definition).

---

### GAP-04: Missing Tracks Persistence

**Problem:** am28 logs missing tracks but doesn't persist structured data:
```rust
info!("[A{}]   MusicBrainz tracks not found in file ({} missing):", ...);
```

For partial albums (75-94% match), there's no record of WHICH tracks are missing.

**Required:** Either:
- A. Add `missing_tracks` JSON field to passages import_metadata
- B. Create explicit `expected_tracks` table with match status per track

---

### GAP-05: Recording Entity vs Song Entity Conflation

**Problem:** REQ002 distinguishes:
- Recording (MusicBrainz entity, has MBID) - ENT-MB-020
- Song (WKMP entity = Recording + weighted artists + optional works) - ENT-MP-010

Current songs table conflates these. A Recording can appear in multiple Songs (same recording, different artist weights - ENT-CNST-030).

**Example:** Live version of "Song X" performed by:
- Original Artist (weight 1.0) → Song A
- Original Artist (0.7) + Guest Artist (0.3) → Song B

Both reference same Recording MBID but are different Songs.

**Required:** Either:
- A. Add recordings table as intermediate (Recording MBID → Recording record → Song references Recording)
- B. Clarify that songs.recording_mbid is NOT unique (current `NOT NULL` but needs `INDEX` not `UNIQUE`)

---

### GAP-06: AcousticBrainz Deprecation

**Problem:** refactor1126.md references "AcousticBrainz high level characterizations" but AcousticBrainz was [deprecated and shut down](https://blog.metabrainz.org/2022/02/16/acousticbrainz-shutdown/) in 2022.

**Required:** Update to reference Essentia or alternative musical feature extraction. The musical_flavor_vector field should reference the actual data source to be used.

---

### GAP-07: Ticks per Sample Field

**Problem:** Passages store timing in ticks. To convert am28 results (seconds) to ticks, need sample_rate. But passages don't store sample_rate - they inherit from parent file.

**Calculation:** `ticks = seconds × sample_rate × (28,224,000 / sample_rate) = seconds × 28,224,000`

**Clarification Needed:** Document that conversion is sample-rate-independent (seconds → ticks is always `× 28,224,000`).

---

### GAP-08: Passage Ordering Within File

**Problem:** For multi-track albums, passages need ordering to represent track sequence. Current schema has `track_number` but no explicit ordering constraint.

**Required:** Clarify that `track_number` provides ordering. Add constraint: `UNIQUE(file_id, track_number)` for files with content_type = FULL_ALBUM or PARTIAL_ALBUM.

---

### GAP-09: Edition Selection Audit Trail

**Problem:** am28 tests multiple MusicBrainz editions. Only winning edition is stored. No audit trail of rejected editions for debugging/review.

**Suggested:** Store rejected editions in import_metadata JSON:
```json
{
  "rejected_editions": [
    {"mbid": "...", "match_pct": 65.0, "reason": "artist_mismatch"},
    {"mbid": "...", "match_pct": 80.0, "reason": "lower_than_winner"}
  ]
}
```

---

## 2. AMBIGUITIES IDENTIFIED

### AMB-01: "High Confidence" Threshold Undefined

**Problem:** User states goal of "high confidence MBID identified passages" but threshold not defined.

**Current Definitions (inconsistent):**
- SPEC_content_type: FULL_ALBUM requires ≥95% match
- am28 confidence levels: Excellent (≥80%), Good (60-79%), Fair (40-59%), Poor (<40%)

**Question:** What is the minimum confidence for a passage to be considered "successfully identified"?

**Options:**
- A. ≥95% only (very conservative)
- B. ≥80% ("Excellent" from am28)
- C. ≥60% ("Good" and above)
- D. Any match with Recording MBID (store all, flag low-confidence)

---

### AMB-02: Content Type vs Status Overlap

**Problem:** Files table has both:
- `status`: "HASH COMPUTED", "METADATA EXTRACTED", "AUDIO DECODED", "INGEST COMPLETE", etc.
- `content_type`: "SINGLE_SONG", "FULL_ALBUM", "PARTIAL_ALBUM", etc.

SPEC_content_type_determination lists outcomes as "status" but these are clearly content classifications, not processing statuses.

**Required Clarification:**
- `status` = processing phase (Step 1-6 progression)
- `content_type` = classification result (from Step 6)

Rename SPEC outcomes to avoid confusion.

---

### AMB-03: Confidence Numeric Mapping

**Problem:** am28 produces string confidence ("Excellent", "Good", "Fair", "Poor"). Database has `identity_confidence REAL`.

**Mapping Undefined:** What numeric values correspond to string levels?

**Suggested Mapping:**
- Excellent: 0.95+
- Good: 0.75-0.94
- Fair: 0.50-0.74
- Poor: 0.0-0.49

---

### AMB-04: Lead-In/Lead-Out Algorithm

**Problem:** refactor1126.md states: "Lead-in point: where audio rises above threshold from start" but:
- What threshold (dB)?
- What analysis window size?
- Is this RMS-based or peak-based?

**Required:** Reference to algorithm or specify parameters:
```rust
const LEAD_THRESHOLD_DB: f64 = -12.0;  // Audio considered "started" above this
const LEAD_RMS_WINDOW_MS: u64 = 100;   // Analysis window
```

---

### AMB-05: Artist Mismatch Action

**Problem:** am28 sets `artist_mismatch = true` when similarity < 50%. But what happens to the match?

**Current am28 Behavior:**
- If match% < 95% AND artist_mismatch → "Artist Mismatch - Likely Incorrect"
- If match% ≥ 95% AND artist_mismatch → "Artist Mismatch - Review Required"

**Database Action Unclear:** Does artist_mismatch result in:
- A. Passage created but flagged for review?
- B. Passage not created (fails "high confidence" threshold)?
- C. User prompted before creation?

---

### AMB-06: Passage → Song Timing Semantics

**Problem:** passage_songs table has `start_time_ticks` and `end_time_ticks` described as "Song start/end within passage."

For typical case (one song per passage), these equal passage start/end.
For multi-song passage, these define song boundaries within passage.

**Question:** For 1:1 case, should timing be:
- A. Duplicated (start=0, end=passage_duration)?
- B. NULL (inherit from passage)?
- C. Same absolute ticks as passage?

---

### AMB-07: MULTIPLE_SONGS Classification Handling

**Problem:** SPEC_content_type_determination defines MULTIPLE_SONGS status but refactor1126.md doesn't address passage creation for this case.

**Question:** For a file with multiple unrelated songs:
- Create one passage per detected segment?
- Create one passage spanning entire file?
- Something else?

---

## 3. CONFLICTS IDENTIFIED

### CON-01: Status Value Incompatibility

**Conflict:**
- refactor1126.md Step statuses: "HASH COMPUTED", "METADATA EXTRACTED", "AUDIO DECODED"
- SPEC_content_type statuses: "SINGLE_SONG", "FULL_ALBUM", "PARTIAL_ALBUM"

These are different status dimensions. Using same field creates ambiguity.

**Resolution Required:** Two-field approach:
- `processing_status`: Tracks import phase ("PENDING", "SCANNING", "EXTRACTING", "DECODING", "CLASSIFYING", "COMPLETE", "FAILED")
- `content_type`: Classification result ("SINGLE_SONG", "FULL_ALBUM", etc.)

---

### CON-02: File Identifier Inconsistency

**Conflict:**
- refactor1126.md Files table: `file_id` (untyped "unique identifier")
- IMPL001-database_schema.md: `guid TEXT PRIMARY KEY` (UUID)

**Resolution:** Use `guid` (UUID) consistently per IMPL001. Update refactor1126.md.

---

### CON-03: Recording MBID Uniqueness Violation

**Conflict:**
- Songs table: `recording_mbid TEXT NOT NULL` (implies unique per context)
- ENT-CNST-030: "Same recording... by different artists = different songs"

If recording_mbid were UNIQUE, two Songs from same Recording would be impossible.

**Resolution:** recording_mbid should be indexed but NOT unique:
```sql
CREATE INDEX idx_songs_recording_mbid ON songs(recording_mbid);
```

---

### CON-04: Timing Units at Integration Boundary

**Conflict:**
- am28 types: `detected_duration: f64` (seconds)
- Database passages: `start_time_ticks: INTEGER` (ticks)

**Resolution:** Integration layer must convert:
```rust
fn seconds_to_ticks(secs: f64) -> i64 {
    (secs * 28_224_000.0).round() as i64
}
```

Document this conversion in integration section of refactor1126.md.

---

### CON-05: Match Tolerance Inconsistency

**Conflict:**
- am28 constants.rs: `MATCH_TOLERANCE_SECS: f64 = 3.0` (3 seconds)
- SPEC_content_type: `const MATCH_TOLERANCE_SECS: f64 = 10.0` (10 seconds)

**Resolution:** Use am28 value (3.0s) as it's empirically tuned through Run 28. Update SPEC_content_type to match.

---

### CON-06: Artist Verification Threshold

**Conflict:**
- am28 constants: `ARTIST_MISMATCH_THRESHOLD: f64 = 0.5` (50% Jaro-Winkler)
- SPEC_content_type: No threshold defined
- refactor1126.md: References verify_artist_match but no threshold

**Resolution:** Document threshold explicitly:
```rust
const ARTIST_SIMILARITY_ACCEPT: f64 = 0.50;  // Accept match if similarity ≥ 50%
```

---

## 4. PROPOSED RESOLUTIONS

### Resolution Summary Table

| Issue | Type | Priority | Proposed Resolution | Status |
|-------|------|----------|---------------------|--------|
| GAP-01 | Gap | CRITICAL | am28 adaptation with order determination (see detailed resolution below) | **RESOLVED** |
| GAP-02 | Gap | HIGH | Add Works table schema | **RESOLVED** in refactor1126.md |
| GAP-03 | Gap | MEDIUM | Add sample_rate to files table definition | **RESOLVED** (already in Additional columns) |
| GAP-04 | Gap | MEDIUM | Add missing_tracks to import_metadata JSON | **RESOLVED** in refactor1126.md |
| GAP-05 | Gap | HIGH | Remove UNIQUE from recording_mbid; add index | **RESOLVED** in refactor1126.md |
| GAP-06 | Gap | LOW | Update AcousticBrainz → Essentia reference | **RESOLVED** in refactor1126.md |
| GAP-07 | Gap | LOW | Document seconds→ticks conversion | **RESOLVED** in refactor1126.md |
| GAP-08 | Gap | MEDIUM | Add UNIQUE(file_id, track_number) constraint | **RESOLVED** in refactor1126.md |
| GAP-09 | Gap | LOW | Add rejected_editions to import_metadata | **RESOLVED** in refactor1126.md |
| AMB-01 | Ambiguity | HIGH | Define "high confidence" = ≥80% match | **RESOLVED** in refactor1126.md |
| AMB-02 | Ambiguity | HIGH | Separate processing_status from content_type | **RESOLVED** via CON-01 |
| AMB-03 | Ambiguity | MEDIUM | Define confidence string→numeric mapping | **RESOLVED** in refactor1126.md |
| AMB-04 | Ambiguity | MEDIUM | Specify lead-in/lead-out parameters | **RESOLVED** in refactor1126.md |
| AMB-05 | Ambiguity | MEDIUM | Define artist_mismatch action: create but flag | **RESOLVED** in refactor1126.md |
| AMB-06 | Ambiguity | LOW | Use NULL timing for 1:1 passage:song | **RESOLVED** in refactor1126.md |
| AMB-07 | Ambiguity | MEDIUM | Create one passage per segment for MULTIPLE_SONGS | **RESOLVED** in refactor1126.md |
| CON-01 | Conflict | HIGH | Status values are labels of convenience | **RESOLVED** |
| CON-02 | Conflict | HIGH | Use guid consistently | **RESOLVED** in refactor1126.md |
| CON-03 | Conflict | HIGH | Index not UNIQUE for recording_mbid | **RESOLVED** in refactor1126.md |
| CON-04 | Conflict | MEDIUM | Document seconds→ticks conversion | **RESOLVED** in refactor1126.md |
| CON-05 | Conflict | MEDIUM | Use MATCH_TOLERANCE_SECS = 3.0 | **RESOLVED** in refactor1126.md |
| CON-06 | Conflict | MEDIUM | Document ARTIST_SIMILARITY_ACCEPT = 0.50 | **RESOLVED** in refactor1126.md |

---

### Detailed Resolution: GAP-01 (Folder-Level Album Detection)

**Key Insight:** Separate audio files in a folder are **pre-segmented** - unlike concatenated album files where am28 must detect track boundaries via silence analysis. This eliminates the segmentation problem but introduces a **track ordering** problem.

**Adaptation of am28 for Folder-Based Albums:**

| Aspect | Album-in-File (am28) | Album-in-Folder (adaptation) |
|--------|---------------------|------------------------------|
| Track boundaries | Unknown (detect via silence) | Known (file boundaries) |
| Track order | Known (sequential in file) | May be unknown |
| Track durations | Detected via silence gaps | Decoded from each file |
| MusicBrainz matching | Duration sequence → Edition | Duration set → Edition (order-flexible) |

**Add new Step 2.5: Folder-Level Album Detection**

After Step 2 (Known Files Check), before Step 3 (Hash Computation):

**Phase 1: Folder Grouping and Metadata Extraction**
1. Group files by parent directory
2. For directories with 3+ audio files:
   - Extract ID3 metadata from all files (artist, album, track number)
   - Decode each file to get duration
   - Collect: `{filename, duration_secs, id3_track_num, id3_artist, id3_album}`

**Phase 2: Track Ordering Determination**

Three ordering strategies, in priority order:

| Strategy | Condition | Confidence |
|----------|-----------|------------|
| **ID3 Track Numbers** | ≥80% of files have TRCK tag with valid numbers | HIGH |
| **Filename Sort** | Filenames contain leading digits that sort correctly | MEDIUM |
| **Unordered** | Neither above applies | Requires order-agnostic matching |

**ID3 Track Number Ordering:**
```
Files: [song_a.mp3 (TRCK=3), song_b.mp3 (TRCK=1), song_c.mp3 (TRCK=2)]
Ordered: [song_b.mp3, song_c.mp3, song_a.mp3] → durations [d1, d2, d3]
```

**Filename Sort Ordering:**
```
Files: ["01 - Intro.mp3", "02 - Main.mp3", "03 - Outro.mp3"]
Regex: /^(\d+)[\s_.-]/  → extract leading number
Ordered by extracted number → durations [d1, d2, d3]
```

**Unordered (Order-Agnostic Matching):**
When ordering cannot be determined, use **set-based matching** instead of sequence-based:
- Collect duration set: {180s, 240s, 195s, ...}
- For each MusicBrainz edition, compare duration sets (not sequences)
- Match if durations pair within tolerance, regardless of order
- Lower confidence than ordered matching (cannot verify sequence)

**Phase 3: MusicBrainz Edition Matching (Adapted from am28)**

**With Known Order:** Apply am28 Stage 2 algorithm directly:
- Duration sequence: [d1, d2, d3, ..., dn]
- Compare to each Edition's expected durations
- Match percentage = tracks within tolerance / expected tracks
- No parameter sweep needed (boundaries already known)

**Without Known Order:** New order-agnostic algorithm:
```
for each Edition:
    expected = edition.durations (sorted)
    detected = folder_durations (sorted)

    # Greedy bipartite matching
    matched = 0
    used_expected = set()
    for d in detected:
        for i, e in enumerate(expected):
            if i not in used_expected and |d - e| <= tolerance:
                matched += 1
                used_expected.add(i)
                break

    match_pct = matched / len(expected) * 100
```

**Phase 4: Handle Missing and Extra Files**

| Scenario | Detection | Action |
|----------|-----------|--------|
| **Missing tracks** | Expected 12, found 10 matching | Log missing track indices, proceed with 83% match |
| **Extra files** | More files than expected tracks | Mark extras as EXTRA_TRACK, may be bonus content |
| **Non-album files** | Duration far outside expected range | Exclude from album matching, process separately |

**Phase 5: Create Database Records**

For successfully matched folder albums:
1. Create/lookup Album record (from winning Edition MBID)
2. For each file in order:
   - Create Passage (file_id, start=0, end=file_duration)
   - Create/lookup Song (from Edition.recording_mbids[track_idx])
   - Link via passage_songs, passage_albums
3. Set file.content_type = 'FOLDER_ALBUM_TRACK'
4. Create folder_album entry linking all files

**New content_type Value:**
```
'FOLDER_ALBUM_TRACK' - Individual file that is part of a folder-based album
```

**New Table:**
```
folder_albums:
  folder_id           | TEXT      | PRIMARY KEY - UUID
  folder_path         | TEXT      | NOT NULL - Path to folder
  detected_album      | TEXT      | Album name from ID3 tags
  detected_artist     | TEXT      | Artist from ID3 tags
  file_count          | INTEGER   | Audio files in folder
  ordering_method     | TEXT      | 'ID3_TRACK', 'FILENAME_SORT', 'UNORDERED'
  ordering_confidence | TEXT      | 'HIGH', 'MEDIUM', 'LOW'
  matched_release     | TEXT      | FK albums(guid) if matched
  match_percentage    | REAL      | Match quality (0.0-100.0)
  missing_tracks      | TEXT      | JSON array of missing track indices
  extra_files         | TEXT      | JSON array of unmatched file paths
  status              | TEXT      | 'PENDING', 'MATCHED', 'PARTIAL', 'UNMATCHED'
  created_at          | TIMESTAMP |
  updated_at          | TIMESTAMP |

folder_album_files:
  folder_id           | TEXT      | FK folder_albums(folder_id)
  file_id             | TEXT      | FK files(guid)
  track_position      | INTEGER   | Position in album (1-based, NULL if unordered)
  ordering_source     | TEXT      | 'ID3', 'FILENAME', 'MATCHED', NULL
  created_at          | TIMESTAMP |
  PRIMARY KEY (folder_id, file_id)
```

**Algorithm Constants:**
```rust
const FOLDER_MIN_FILES: usize = 3;           // Min files to consider as album folder
const FOLDER_ID3_ORDERING_THRESHOLD: f64 = 0.80;  // 80% files need TRCK for ID3 ordering
const FOLDER_MATCH_TOLERANCE_SECS: f64 = 3.0;     // Same as am28
const FOLDER_ALBUM_MIN_MATCH_PCT: f64 = 75.0;     // Min match for FOLDER_ALBUM status
```

---

### Detailed Resolution: CON-01 (Status Model)

**Resolution:** Existing status values in refactor1126.md and SPEC_content_type_determination.md are **labels of convenience**, not required values. Final implementation shall define status values appropriate to the actual implementation backend.

**Key Distinction (to be preserved):**
- `status` field: Tracks processing phase progression
- `content_type` field: Records classification result (orthogonal to processing phase)

**Implementation Guidance:**
- Status values should reflect actual processing pipeline stages
- Content type values should reflect classification outcomes
- Specific string values determined during implementation based on:
  - Database schema conventions
  - API response requirements
  - UI display needs
  - Consistency with existing wkmp-ai patterns

**Example (illustrative, not prescriptive):**
```
Processing progression: file enters system → hash computed → decoded → classified → complete
Classification outcomes: single song, full album, partial album, folder album track, unidentified
```

The two-field approach remains valid - just don't treat the specific string literals as mandatory.

---

## 5. Decisions Required

~~Before proceeding to /plan implementation, stakeholder decisions needed on:~~

~~1. **GAP-01:** Confirm folder-level album detection is in scope for initial implementation~~
~~2. **AMB-01:** Confirm "high confidence" threshold (recommend ≥80%)~~
~~3. **AMB-05:** Confirm artist_mismatch action (recommend: create passage, flag for review)~~
~~4. **GAP-05:** Confirm removing UNIQUE constraint on recording_mbid is acceptable~~

**Update 2025-11-26:** All decisions have been resolved and incorporated into refactor1126.md:
- GAP-01: Folder-level album detection documented with full algorithm
- AMB-01: High confidence threshold set to ≥80%
- AMB-05: Artist mismatch action = create passage, flag for review
- GAP-05: recording_mbid uses INDEX (not UNIQUE)

---

## Next Steps

~~This analysis is complete. Implementation planning requires explicit user authorization.~~

**Update 2025-11-26:** All identified gaps, ambiguities, and conflicts have been resolved in refactor1126.md.

**Specification is ready for /plan:**
- All 9 gaps resolved
- All 7 ambiguities clarified
- All 6 conflicts resolved
- refactor1126.md updated with comprehensive schema, constants, and algorithm definitions

**To proceed:** Run `/plan wkmp-ai/refactor1126.md` to create detailed implementation plan.

---

**Analysis Complete**
**Document Status:** All issues resolved - Ready for implementation planning
