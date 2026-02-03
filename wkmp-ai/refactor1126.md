# wkmp-ai Major Refactoring 1126

**Updated 2025-11-26:** Expanded with detailed database schema definitions for passage timing, songs table, and am28 integration notes.

The results from album_matcher have been very good through run 28. The am28 modular example (`wkmp-ai/examples/am28/`) provides a clean, tested implementation ready for integration into wkmp-ai proper.

The current project is to do a major refactoring of wkmp-ai to incorporate the algorithm, perhaps most of the code directly, into wkmp-ai.

What's good to keep from the current wkmp-ai implementation:

- The file scanner and magic byte reader.

- The basic structure of the user interface with live updates via SSE showing the user how analysis is progressing.

- Maybe the chromaprint / AcoustID code - it has worked well in the past, but is struggling now, probably due to difficulty in identifying passages to ID.

- The concept of the single threaded database interface service.  I think the implementation is suffering from too much complex integration with overly complex passage identification code that doesn't work well.

- The concepts of the zero-conf database and especially its settings table with default values.

- Concept / parameter / enum and other designations in the documentation, where applicable to this implementation, should be carried over as default - though they should also be reviewed for self-consistency and changed when appropriate for the new implementation.

The rest of the existing wkmp-ai implementation and documentation should be set aside for archival reference, but mostly ignored when the new implementation is ready to start.  New documentation shall be developed for wkmp-ai and shall be concise and complete on its own without reliance on existing wkmp-ai specific documentation.  General documentation that applies across wkmp, like SPEC017, shall be referred to as-is - or identified as a source of inconsistency / conflict to be resolved.

-----

## Guiding Principles

- Imported audio files are "played where they lie" - they are not moved, renamed, touched, or modified in any way.  They are only read.  All information gathered about the imported files is recorded in the database.
- This import process gathers data from remote databases, via internet and potentially other connections.  When this process is done, any information needed for future operations with the files shall all be locally stored in the database, this process' intent is to make the rest of the system free of network dependencies.

-----

## Foundational Elements

- The main wkmp SQLite database is a central repository which all wkmp microservices use.
- While wkmp-ai is a multi-threaded implementation to speed importing of large libraries of files, the SQLite database has a single threaded access design.  wkmp-ai devotes one specific db_access thread to handle database access operations by proxy for all other wkmp-ai threads.  The read and write operations are communicated to the db_access thread which enqueues them, and processes the database operation in FIFO order.  This enables direct monitoring of bottleneck statistics (how deep is the queue, what's the longest wait time, which operations are involved in major queue backups) and prevents ambiguities about what may or may not be a database access time issue.

-----

## Objectives

The purpose of the wkmp-ai audio import process is to:

- identify passages in audio files
  - at what point in time (in the file) do they start?
  - at what point in time (in the file) do they end?
  - at what point in time (in the file) should they deny other tracks permission to continue playing over their beginning (lead-in point)?
  - at what point in time (in the file) should they allow other tracks permission to start playing over their ending (lead-out point)?
  - what is the song or other MusicBrainz MBID most appropriate for this passage, if any?
- identify related MusicBrainz data and capture it in the local database
  - album / release identities and data
  - song identities and data
  - artist identities and data
  - much more...
- retrieve and store musical characterizations of songs found in the passages found in the audio files (via Essentia or similar audio analysis - note: AcousticBrainz was deprecated in 2022)

While passages in general may contain zero, one, or more songs with MBIDs, the wkmp-ai process only identifies single song or zero song passages.
  
-----

## Important related documents

- REQ002 entity definitions
- SPEC017 regarding time units of ticks for passage table references to time offsets in audio files
- SPEC002 crossfade definitions
- SPEC031 data driven schema maintenance: zero conf system design
- REQ001 system level requirements

- SPEC032 audio ingest architecture is a historical reference, it may help fill in some blanks in the new design, but is not in any way authoritative vs the new wkmp-ai design

-----

## Operational realities

The audio files wkmp-ai works with:
- may be in one of many formats (see: REQ001 REQ-CF-010)
- may contain one song, a whole album of music, or other kinds of audio both musical, spoken, etc.
- are occasionally corrupted, or the product of a glitchy extraction
- while file names, folder names, ID3 tags and other sources of identity about the audio files are generally reliable, they are not 100% reliable and should always be cross checked / validated to the extent possible automatically
- the automatic identification process should recognize when it has a low reliability conclusion about an audio file's contents' identity and bring this to the user's attention

-----

## Step 1 - Folder Scan

The "new vision" is to start an import of audio files in a folder beginning with a scan / magic byte analysis which creates an in-memory mapping of:

- The list of all files in the folder and its sub-folders
- Classification of each file as:
  1. audio by magic byte with    recognized audio filename extension
  2. audio by magic byte without recognized audio filename extension
  3. image by magic byte with    recognized image filename extension
  4. image by magic byte without recognized image filename extension
  5. other by magic byte without recognized audio or image filename extension
  6. other by magic byte with    recognized audio filename extension
  7. other by magic byte with    recognized image filename extension
  
- other means: not recognized by magic byte analysis as an audio or image file
- Files of types 1., 3. and 5. "have proper extensions".
- Files of other types "have improper extensions", and are shown to the user in a list/table showing their path/name/extension and describing the issue with each.

- Files of types 3. and 5. are simply ignored by later wkmp-ai processes
- wkmp-ai only works on files of type 1.
- The in-memory list of verified audio files' paths is passed to Step 2.

The folder scan is a very fast process, all files in the folder are scanned at-once to provide information to estimate time to complete the import process.  After the folder scan is complete, files of type 1 are worked on one by one, bringing each file to completion in part so that if the import process is interrupted at least some of the work of importing is in a complete, ready to use state in the database and future import work can "pick up where the previous import left off."

## Step 2 - Known Files Check

Proceeding file by file through the folder scan list:

Determine if the file is already in the database or not.

Lookup each file by path and name in the files table of the database.

If the path/filename is present in the files table of the database: compare the file's filesystem metadata (size in bytes, last modified date) to the copy of the metadata for that path/filename in the database.  If the metadata is an exact match and the file's status is one of the complete statuses, then this file will not be processed any further.
  
Other files, whether not in the files table of the database or in the files table of the database with a not complete status, proceed to Step 3.

## Step 3 - Hash computation

Each file passed from step 2 to step 3 has the hash (SHA-256) of all its data computed (this is the currently processing file).  The files table is searched for any matches to this hash.

If a hash match is found and that match has the currently processing file's path and filename, then it must have different filesystem metadata than the file currently has, otherwise this file would have stopped processing in step 2 - so: update the filesystem metadata in this file's entry in the database to match the current filesystem metadata of this file, update the "filesystem metadata last updated" record for this file, and then continue processing this file based on its entry's status in the database.

If a hash match is found and that match has a different path/filename, if that hash match file's status is DUPLICATE, then it should point to the file it is a DUPLICATE of, the currently processing's status is also marked DUPLICATE and set to point at the "root" file that the other DUPLICATE file points at.  Once this is done, the currently processing file will not be processed any further.

If the hash matched file's status is one of the other complete statuses, then this file's status is marked DUPLICATE, a pointer to the file-id of the hash matched file is recorded in this file's record, and this file will not be processed any further.

If the hash matched file's status is not a complete status, it is ignored for the moment - when the currently processing file's status becomes one of the complete statuses it will search for matching hashes and update any which are not complete to be DUPLICATE pointing at the currently processing file (see: Completed duplicate handling)

After handling any hash matches, if the currently processing file's path is found in the database, then that entry (with the matching path) is updated with this file's hash, metadata, metadata update time, and a status of HASH COMPUTED (a not complete status), and this file continues processing in Step 4.

When the currently processing file's path is not found in the database, then a new entry is created with a new fileId, the currently processing file's path, this file's hash, metadata, metadata update time, and a status of HASH COMPUTED (a not complete status), and this file continues processing in Step 4.

## Step 4 - Metadata extraction

The currently processing file has its internal metadata (ID3 tags and others like: Vorbis Comments, MP4/iTunes Tags, APEv2, BWF, and WMA), if any, extracted and stored in the files table internalMetadata json object.

timeId3LastUpdated in the internalMetadata object is populated with the time when the ID3 tags were extracted, whether any ID3 data was found or not.

status is updated to METADATA EXTRACTED when this process is complete

## Step 5 - Audio decoding

All audio in the file is decoded to uncompressed frames.

The total runtime of the file is recorded in the files table

If less than 100 milliseconds of audio higher than the no audio threshold is found in the entire file, the file's status is updated to NO AUDIO and processing jumps to Completed duplicate handling.

status is updated to AUDIO DECODED when this process is complete and continues to Step 6

## Step 6 - Content type determination

At this point, we know there is some sound in the audio file, we want to categorize it further as:

- a single song (with a MusicBrainz Recording MBID)
- a full album (with a MusicBrainz Release MBID)
- multiple songs with MBIDs but not associated with a particular album release
- audio not described in MusicBrainz

**Algorithm summary:**

1. **Duration-based triage:** Files < 12 min → single-song path; 12-25 min → dual-path; > 25 min → album path
2. **Quick silence scan:** Relaxed parameters (-45 dB, 2s gaps) estimate segment count before MusicBrainz lookup
3. **Single-song path:** Chromaprint → AcoustID lookup → Recording MBID (confidence ≥ 0.8 = confirmed)
4. **Album path:** ID3/path metadata → MusicBrainz Release search → edition matching via silence detection + track duration comparison (adapted from album_matcher_20.rs stages 2-5)
5. **Fallback classification:** Poor album match (< 50%) with multiple segments → try per-segment AcoustID → classify as MULTIPLE_SONGS or NOT_IN_MUSICBRAINZ

**Status outcomes:** SINGLE_SONG, FULL_ALBUM, PARTIAL_ALBUM, MULTIPLE_SONGS, NOT_IN_MUSICBRAINZ, IDENTIFICATION_FAILED

See [wkmp-ai/SPEC_content_type_determination.md](SPEC_content_type_determination.md) for full algorithm details, decision trees, and confidence thresholds.

Statuses of NOT_IN_MUSICBRAINZ and IDENTIFICATION_FAILED are completed statuses which send the file to Completed duplicate handling.

Statuses of FULL_ALBUM, PARTIAL_ALBUM, MULTIPLE_SONGS proceed to their own specific handlers which segment the file into multiple passages which will each be handled individually.  SINGLE_SONG becomes a single passage to be handled like one of the passages from the segmented files.

**MULTIPLE_SONGS Handling (AMB-07):**

When a file contains multiple unrelated songs (different artists/releases per segment):
1. Create **one passage per detected segment** (based on silence detection boundaries)
2. Each passage is independently identified via Chromaprint/AcoustID
3. Each passage gets its own Song association (if identified) or remains a zero-song passage
4. Passages are linked to file via `file_id` but NOT linked via `passage_albums` (no single album)
5. Track numbers are assigned sequentially (1, 2, 3...) but do not imply album ordering

This handles compilation files, mixed DJ sets, and files containing unrelated songs.

## Later steps

start, end, lead-in and lead-out point identification

storing all this in the database tables

Chromaprint / AcoustID song identification - not always available, not always correct, but when it is available it can serve as assurance that track identification has been done correctly, and it can serve to identify more likely alternate editions that may not have aligned their silence gaps with track durations as well as the current candidate which has AcoustID conflicts, but they have AcoustID confirmations.

Obtaining/recording musical feature descriptions for songs (via Essentia or similar audio analysis)

## Completed duplicate handling

As mentioned in Step 3 Hash Computation, when a file's status is set to any of the completed statuses (like: INGEST COMPLETE, DUPLICATE, NO AUDIO) then another check is made of the database for any other files with a matching hash value.  If any other files are found with a matching hash and an incomplete status, their status is set to DUPLICATE and their duplicate_id field is set to the file_id of this just finalized file.

This also triggers a mechanism whereby the file, just marked DUPLICATE, is searched for within the list of files either in process or waiting to be processed (by path/filename).  If found, then that file object is signaled (through an AtomicBool) that it has been marked duplicate.  All database write processes, and other convenient / appropriate locations in the file processing code shall check this signal flag and if it is set the file shall abort any further processing - it's already completed, status DUPLICATE.  This AtomicBool prevents the need to check status in the database frequently.

Once a file has completed duplicate handling, it is not processed any more.


-----

## Algorithm Constants and Thresholds

These constants define the key thresholds used throughout wkmp-ai processing. Values are derived from am28 empirical tuning and specification requirements.

### Confidence Thresholds (AMB-01, AMB-03)

```rust
// "High confidence" definition for successful identification
const HIGH_CONFIDENCE_THRESHOLD: f64 = 0.80;  // ≥80% = high confidence

// String-to-numeric confidence mapping
// Match % → Numeric Confidence → String Level
//   95%+  →  0.95-1.00  →  "Excellent"
//   75-94% →  0.75-0.94  →  "Good"
//   50-74% →  0.50-0.74  →  "Fair"
//   <50%   →  0.00-0.49  →  "Poor"

const CONFIDENCE_EXCELLENT_MIN: f64 = 0.95;
const CONFIDENCE_GOOD_MIN: f64 = 0.75;
const CONFIDENCE_FAIR_MIN: f64 = 0.50;
```

### Album Matching Thresholds (CON-05)

```rust
// Track duration matching tolerance (seconds)
// Note: am28 uses 3.0s based on empirical tuning through Run 28
const MATCH_TOLERANCE_SECS: f64 = 3.0;

// Match percentage thresholds for classification
const FULL_ALBUM_MIN_MATCH_PCT: f64 = 95.0;
const PARTIAL_ALBUM_MIN_MATCH_PCT: f64 = 75.0;
```

### Artist Verification (CON-06, AMB-05)

```rust
// Artist name similarity threshold (Jaro-Winkler distance)
const ARTIST_SIMILARITY_ACCEPT: f64 = 0.50;  // Accept if ≥50% similar

// Artist mismatch action: CREATE passage but FLAG for review
// When artist_mismatch = true AND match_pct < 95%: flag as "Review Required"
// When artist_mismatch = true AND match_pct >= 95%: accept but note mismatch
```

### Lead-In/Lead-Out Detection (AMB-04)

```rust
// Amplitude analysis parameters for determining lead-in/lead-out points
const LEAD_THRESHOLD_DB: f64 = -12.0;      // Audio "started" when above this level
const LEAD_RMS_WINDOW_MS: u64 = 100;       // RMS analysis window size
const LEAD_OUT_THRESHOLD_DB: f64 = -12.0;  // Audio "ending" when below this level
```

### Time Unit Conversion (GAP-07, CON-04)

```rust
// Seconds to ticks conversion (sample-rate independent)
// 1 tick = 1/28,224,000 second (per SPEC017)
fn seconds_to_ticks(secs: f64) -> i64 {
    (secs * 28_224_000.0).round() as i64
}

fn ticks_to_seconds(ticks: i64) -> f64 {
    ticks as f64 / 28_224_000.0
}
```

-----

## Database schema notes

The database includes the tables defined below. These schemas align with IMPL001-database_schema.md with extensions needed for am28 album matching integration.

**Reference Documents:**
- IMPL001-database_schema.md - Main schema definition (authoritative)
- SPEC017-sample_rate_conversion.md - Tick-based timing (1 tick = 1/28,224,000 second)
- SPEC002-crossfade.md - Six timing points definition

-----

### Passages table

Each passage represents a playable segment within an audio file. For files containing full albums, each track becomes one passage. All timing values are stored as **INTEGER ticks** per SPEC017.

**Core Timing Fields (per SPEC002 crossfade definitions):**

Column name          | Type    | Description
---------------------+---------+------------------------------------------------------------------
guid                 | TEXT    | PRIMARY KEY - UUID for this passage
file_id              | TEXT    | FOREIGN KEY to files(guid) - parent audio file
start_time_ticks     | INTEGER | NOT NULL - Passage start boundary (ticks from file start, 0 = file start)
end_time_ticks       | INTEGER | NOT NULL - Passage end boundary (ticks from file start)
fade_in_start_ticks  | INTEGER | NULL - When fade-in begins (NULL = use global Crossfade Time)
lead_in_start_ticks  | INTEGER | NULL - Latest time previous passage may overlap (NULL = use global)
lead_out_start_ticks | INTEGER | NULL - Earliest time next passage may start (NULL = use global)
fade_out_start_ticks | INTEGER | NULL - When fade-out begins (NULL = use global)
fade_in_curve        | TEXT    | NULL - Curve type: 'exponential', 'cosine', 'linear' (NULL = use global)
fade_out_curve       | TEXT    | NULL - Curve type: 'logarithmic', 'cosine', 'linear' (NULL = use global)

**Metadata and Identification Fields:**

Column name                  | Type      | Description
-----------------------------+-----------+------------------------------------------------------------
title                        | TEXT      | Track/passage title from file tags or MusicBrainz
user_title                   | TEXT      | User-defined title override
artist                       | TEXT      | Artist from tags (denormalized cache)
album                        | TEXT      | Album from tags (denormalized cache)
track_number                 | INTEGER   | Track position within album (1-based, from am28 matching)
disc_number                  | INTEGER   | Disc number for multi-disc releases (from am28)
musical_flavor_vector        | TEXT      | JSON blob of musical characterization values (via Essentia or similar)

**Import Provenance Fields (from am28 matching results):**

Column name                  | Type      | Description
-----------------------------+-----------+------------------------------------------------------------
import_metadata              | TEXT      | JSON: silence detection params, RMS profile, match confidence
identity_confidence          | REAL      | Overall match confidence from am28 (0.0-1.0)
matching_stage               | TEXT      | Which am28 stage produced this match (stage2, stage3, etc.)
mean_error_seconds           | REAL      | Mean timing error vs MusicBrainz expected durations
match_percentage             | REAL      | Percentage of tracks matched within tolerance

**Status Tracking:**

Column name     | Type      | Description
----------------+-----------+------------------------------------------------------------
status          | TEXT      | 'PENDING', 'INGEST COMPLETE' - import phase tracking
decode_status   | TEXT      | 'pending', 'successful', 'unsupported_codec', 'failed'
created_at      | TIMESTAMP | Record creation time
updated_at      | TIMESTAMP | Record last update time

**Constraints (per SPEC002):**
- start_time_ticks >= 0
- end_time_ticks > start_time_ticks
- Fade points within passage bounds
- Lead points within passage bounds
- No cross-constraint between fade and lead (independent chains)

**Passage Ordering Constraint (GAP-08):**
- For album files (content_type = FULL_ALBUM or PARTIAL_ALBUM): `UNIQUE(file_id, track_number)`
- For MULTIPLE_SONGS files: track_number is sequential but NOT guaranteed to match any album sequence
- For SINGLE_SONG files: track_number = 1 or NULL

-----

### Songs table

A Song is a unique combination of a MusicBrainz Recording and weighted artist set (per REQ002 ENT-CNST-030).

**Important:** `recording_mbid` is **indexed but NOT unique**. The same Recording MBID can appear in multiple Song records because Song identity includes the weighted artist set. Example: same recording performed by different artists (or same artists with different weights) = different songs.

Column name         | Type      | Constraints    | Description
--------------------+-----------+----------------+--------------------------------------------------
guid                | TEXT      | PRIMARY KEY    | UUID for this song
recording_mbid      | TEXT      | NOT NULL INDEX | MusicBrainz Recording ID (UUID) - **from am28 Edition.recording_mbids** (indexed, NOT unique)
title               | TEXT      |                | Recording title from MusicBrainz
work_id             | TEXT      | FK works(guid) | Musical work this is a recording of
related_songs       | TEXT      |                | JSON array of related song GUIDs (other recordings of same work)
lyrics              | TEXT      |                | Lyrics (plain UTF-8 text)
base_probability    | REAL      | DEFAULT 1.0    | Selection probability (0.0-1000.0)
min_cooldown        | INTEGER   | DEFAULT 604800 | Minimum cooldown seconds (default 7 days)
ramping_cooldown    | INTEGER   | DEFAULT 1209600| Ramping cooldown seconds (default 14 days)
last_played_at      | TIMESTAMP |                | Last playback timestamp
status              | TEXT      | DEFAULT 'PENDING' | 'PENDING', 'FLAVOR READY', 'FLAVORING FAILED'
created_at          | TIMESTAMP | NOT NULL       | Record creation time
updated_at          | TIMESTAMP | NOT NULL       | Record last update time

**Key Insight from am28:** The `Edition.recording_mbids` array contains the Recording MBID for each track. When creating passages from album matching:
1. Each detected track boundary → one passage entry
2. Each `recording_mbids[track_idx]` → lookup/create song entry with that MBID
3. Link via `passage_songs` table

-----

### Works table

A Work represents a unique musical composition (per REQ002 ENT-MB-030). Multiple recordings can exist of each work.

Column name         | Type      | Constraints    | Description
--------------------+-----------+----------------+--------------------------------------------------
guid                | TEXT      | PRIMARY KEY    | UUID for this work
work_mbid           | TEXT      | NOT NULL UNIQUE| MusicBrainz Work ID (UUID)
title               | TEXT      |                | Work title from MusicBrainz
composer            | TEXT      |                | Composer/author name(s)
iswc                | TEXT      |                | International Standard Musical Work Code
work_type           | TEXT      |                | 'Song', 'Symphony', 'Opera', etc.
created_at          | TIMESTAMP | NOT NULL       | Record creation time
updated_at          | TIMESTAMP | NOT NULL       | Record last update time

**Relationship:** Songs.work_id → Works.guid (Many-to-one per ENT-CARD-045)

**Note:** Work association is optional. Zero works = improvisations, sound effects; Multiple works = mashups, medleys (handled via separate table if needed).

-----

### Artists table

Column name         | Type      | Constraints    | Description
--------------------+-----------+----------------+--------------------------------------------------
guid                | TEXT      | PRIMARY KEY    | UUID for this artist
artist_mbid         | TEXT      | NOT NULL UNIQUE| MusicBrainz Artist ID (UUID)
name                | TEXT      | NOT NULL       | Canonical artist name
base_probability    | REAL      | DEFAULT 1.0    | Selection probability (0.0-1000.0)
min_cooldown        | INTEGER   | DEFAULT 7200   | Minimum cooldown seconds (default 2 hours)
ramping_cooldown    | INTEGER   | DEFAULT 14400  | Ramping cooldown seconds (default 4 hours)
last_played_at      | TIMESTAMP |                | Last playback timestamp
created_at          | TIMESTAMP | NOT NULL       | Record creation time
updated_at          | TIMESTAMP | NOT NULL       | Record last update time

**From am28:** Artist info comes from `MBArtistCredit` in release details. Need to extract and dedupe artists per release.

-----

### Albums table

Column name    | Type      | Constraints    | Description
---------------+-----------+----------------+--------------------------------------------------
guid           | TEXT      | PRIMARY KEY    | UUID for this album
album_mbid     | TEXT      | NOT NULL UNIQUE| MusicBrainz Release ID (UUID) - **from am28 Edition.mbids[].mbid**
title          | TEXT      | NOT NULL       | Album title from MusicBrainz
release_date   | TEXT      |                | Release date (ISO 8601 format)
artist_credit  | TEXT      |                | Combined artist credit string
country        | TEXT      |                | Release country (from am28 EditionMBID.country)
status         | TEXT      |                | Release status: 'Official', 'Bootleg', etc.
created_at     | TIMESTAMP | NOT NULL       | Record creation time
updated_at     | TIMESTAMP | NOT NULL       | Record last update time

**From am28:** Best MBID selected via `select_best_mbid()` function, preferring Official + CD releases.

-----

### Relationship Tables

**passage_songs** - Links passages to songs with timing info

Column name       | Type    | Description
------------------+---------+------------------------------------------------------------
passage_id        | TEXT    | FK passages(guid)
song_id           | TEXT    | FK songs(guid)
start_time_ticks  | INTEGER | Song start within passage (ticks from passage start), NULL for 1:1
end_time_ticks    | INTEGER | Song end within passage (ticks from passage start), NULL for 1:1
created_at        | TIMESTAMP| Record creation time
PRIMARY KEY (passage_id, song_id)

**Timing Semantics (AMB-06):**
- **1:1 case** (one song per passage): `start_time_ticks` and `end_time_ticks` are **NULL** - timing inherited from passage
- **Multi-song passage**: Values specify song boundaries within the passage (relative to passage start)
- NULL handling: When reading, NULL start = 0, NULL end = passage duration

**song_artists** - Links songs to artists with weights

Column name | Type | Description
------------+------+------------------------------------------------------------
song_id     | TEXT | FK songs(guid)
artist_id   | TEXT | FK artists(guid)
weight      | REAL | Artist contribution weight (0.0-1.0, sum must equal 1.0)
created_at  | TIMESTAMP | Record creation time
PRIMARY KEY (song_id, artist_id)

**passage_albums** - Links passages to albums

Column name | Type | Description
------------+------+------------------------------------------------------------
passage_id  | TEXT | FK passages(guid)
album_id    | TEXT | FK albums(guid)
created_at  | TIMESTAMP | Record creation time
PRIMARY KEY (passage_id, album_id)

-----

### Files table

Entries in the Files table include (but are not limited to, and any key may have a value of NULL at any time).

**Note:** Per IMPL001-database_schema.md, primary keys use `guid TEXT` (UUID format) for consistency across all tables.

Column name / Key   | Value description
--------------------+----------------------------------------------------------------------------------------
guid                | PRIMARY KEY - UUID for this file (used as `file_id` in foreign key references)
--------------------+----------------------------------------------------------------------------------------
status              | an enum which includes: INGEST COMPLETE, DUPLICATE, NO AUDIO, HASH COMPUTED, AUDIO DECODED and others...
--------------------+----------------------------------------------------------------------------------------
path                | this is the path, relative to the root folder, and filename of the file including extension
--------------------+----------------------------------------------------------------------------------------
filesystem_metadata | a json object which may include:
                    | Key                     | Value description
				    |-------------------------+--------------------------------------------------------------
				    | timeMetadataLastUpdated | i64 Unix time (0 at Jan 1 1970) in microseconds when this metadata was last copied here from the filesystem
				    | size                    | Number of bytes in the file (not size on disk)
				    | timeLastModified        | i64 Unix time (0 at Jan 1 1970) in microseconds, converted from the filesystem's time units
--------------------+----------------------------------------------------------------------------------------
hash                | BLOB(32) of the SHA-256 hash result
--------------------+----------------------------------------------------------------------------------------
duplicate_id        | Usually NULL, but when status is DUPLICATE this contains the guid of a file with a completed status and identical hash value to this one
--------------------+----------------------------------------------------------------------------------------
total_runtime_ticks | total audio runtime of the file in integer SPEC017 ticks
--------------------+----------------------------------------------------------------------------------------
internal_metadata   | a json object which may include tag info extracted from the file's data:
                    | Key                     | Value description
	 			    |-------------------------+--------------------------------------------------------------
	 			    | timeTagsLastUpdated     | i64 Unix time (0 at Jan 1 1970) in microseconds when metadata was last extracted
	 			    | format                  | string identifying the tag format found: "id3v2", "id3v1", "vorbis", "mp4", "ape", "bwf", "wma", or null if none
				    |-------------------------+--------------------------------------------------------------
				    | id3                     | (MP3 files) nested json object with ID3v2/ID3v1 frame IDs as keys:
				    |                         | Key                     | Value description
				    |                         |-------------------------+------------------------------------
				    |                         | TIT2                    | Title (song name)
				    |                         | TPE1                    | Artist (lead performer)
				    |                         | TALB                    | Album title
				    |                         | TRCK                    | Track number (often "X/Y" format)
				    |                         | TYER / TDRC             | Year / Recording date
				    |                         | TCON                    | Genre
				    |                         | TPOS                    | Disc number (often "X/Y" format)
				    |                         | TPE2                    | Album artist
				    |                         | TXXX                    | User-defined text (array of {description, value})
				    |                         | ...                     | other ID3 frames as found
				    |-------------------------+--------------------------------------------------------------
				    | vorbis                  | (OGG, FLAC, Opus files) nested json object with Vorbis Comment field names as keys:
				    |                         | Key                     | Value description
				    |                         |-------------------------+------------------------------------
				    |                         | TITLE                   | Track title
				    |                         | ARTIST                  | Artist name
				    |                         | ALBUM                   | Album title
				    |                         | TRACKNUMBER             | Track number (may be "X" or "X/Y")
				    |                         | DISCNUMBER              | Disc number
				    |                         | DATE                    | Release date (often just year)
				    |                         | GENRE                   | Genre
				    |                         | ALBUMARTIST             | Album artist (for compilations)
				    |                         | COMMENT                 | Free-form comment
				    |                         | MUSICBRAINZ_TRACKID     | MusicBrainz Recording MBID
				    |                         | MUSICBRAINZ_ALBUMID     | MusicBrainz Release MBID
				    |                         | ...                     | other Vorbis comments as found (case-insensitive by spec)
				    |-------------------------+--------------------------------------------------------------
				    | mp4                     | (M4A, AAC, MP4 files) nested json object with iTunes/MP4 atom names as keys:
				    |                         | Key                     | Value description
				    |                         |-------------------------+------------------------------------
				    |                         | ©nam                    | Title
				    |                         | ©ART                    | Artist
				    |                         | ©alb                    | Album title
				    |                         | aART                    | Album artist
				    |                         | trkn                    | Track number (tuple: [track, total])
				    |                         | disk                    | Disc number (tuple: [disc, total])
				    |                         | ©day                    | Release date/year
				    |                         | ©gen / gnre             | Genre (text or numeric ID)
				    |                         | cpil                    | Compilation flag (boolean)
				    |                         | ©wrt                    | Composer
				    |                         | ----                    | iTunes-specific atoms stored with "----" prefix
				    |                         | ...                     | other MP4 atoms as found
				    |-------------------------+--------------------------------------------------------------
				    | ape                     | (APE, Musepack, WavPack files) nested json object with APEv2 tag keys:
				    |                         | Key                     | Value description
				    |                         |-------------------------+------------------------------------
				    |                         | Title                   | Track title
				    |                         | Artist                  | Artist name
				    |                         | Album                   | Album title
				    |                         | Track                   | Track number
				    |                         | Year                    | Release year
				    |                         | Genre                   | Genre
				    |                         | Album Artist            | Album artist
				    |                         | Disc                    | Disc number
				    |                         | Comment                 | Comment
				    |                         | ...                     | other APEv2 tags as found (case-insensitive keys)
				    |-------------------------+--------------------------------------------------------------
				    | bwf                     | (Broadcast WAV files) nested json object with BWF/BEXT chunk fields:
				    |                         | Key                     | Value description
				    |                         |-------------------------+------------------------------------
				    |                         | Description             | Free-form description (256 chars max)
				    |                         | Originator              | Creator/originator name
				    |                         | OriginatorReference     | Unique reference (e.g., facility code)
				    |                         | OriginationDate         | Creation date (YYYY-MM-DD)
				    |                         | OriginationTime         | Creation time (HH:MM:SS)
				    |                         | TimeReference           | Sample count since midnight (for SMPTE sync)
				    |                         | Version                 | BWF version number
				    |                         | UMID                    | Unique Material Identifier (64 bytes hex)
				    |                         | LoudnessValue           | Integrated loudness (EBU R 128)
				    |                         | LoudnessRange           | Loudness range (EBU R 128)
				    |                         | CodingHistory           | Signal chain/encoding history
				    |                         | ...                     | other BEXT fields as found
				    |-------------------------+--------------------------------------------------------------
				    | wma                     | (WMA, ASF files) nested json object with ASF metadata attribute names:
				    |                         | Key                     | Value description
				    |                         |-------------------------+------------------------------------
				    |                         | Title                   | Track title
				    |                         | Author                  | Artist/author
				    |                         | WM/AlbumTitle           | Album title
				    |                         | WM/AlbumArtist          | Album artist
				    |                         | WM/TrackNumber          | Track number
				    |                         | WM/PartOfSet            | Disc number
				    |                         | WM/Year                 | Release year
				    |                         | WM/Genre                | Genre
				    |                         | WM/Composer             | Composer
				    |                         | Description             | Description/comment
				    |                         | Copyright               | Copyright notice
				    |                         | WM/Publisher            | Publisher/label
				    |                         | WM/UniqueFileIdentifier | MusicBrainz Recording MBID (if present)
				    |                         | ...                     | other ASF attributes as found
--------------------+----------------------------------------------------------------------------------------

**Additional Files table columns (from IMPL001 and am28 requirements):**

Column name          | Type      | Description
---------------------+-----------+------------------------------------------------------------------
format               | TEXT      | Audio format: 'FLAC', 'MP3', 'AAC', 'WAV', 'Opus', etc. (via lofty)
sample_rate          | INTEGER   | Sample rate in Hz (e.g., 44100, 48000, 96000)
channels             | INTEGER   | Number of audio channels (1=mono, 2=stereo)
file_size_bytes      | INTEGER   | File size in bytes
content_type         | TEXT      | Classification from Step 6: 'SINGLE_SONG', 'FULL_ALBUM', 'PARTIAL_ALBUM', 'MULTIPLE_SONGS', 'NOT_IN_MUSICBRAINZ', 'IDENTIFICATION_FAILED'
matched_release_mbid | TEXT      | Best MusicBrainz Release MBID from am28 matching (NULL if unmatched)
match_confidence     | TEXT      | am28 confidence level: 'Excellent', 'Good', 'Fair', 'Poor'
match_percentage     | REAL      | Percentage of tracks matched within tolerance (0.0-100.0)
artist_verified      | INTEGER   | Boolean: 1 if artist name verification passed, 0 if mismatch detected
matching_hashes      | TEXT      | JSON array of file UUIDs with matching content hash (bidirectional duplicate links)

-----

## am28 Integration: Mapping Types to Database Records

This section describes how am28 data structures map to database records.

### ValidationResult → File + Passages

When am28 produces a `ValidationResult` for an album file:

```
ValidationResult {
    album_path          → files.path
    mbid                → files.matched_release_mbid, albums.album_mbid
    match_percentage    → files.match_percentage
    confidence          → files.match_confidence
    matched_artist      → albums.artist_credit
    matched_album       → albums.title
    artist_mismatch     → files.artist_verified (inverted)
    matching_stage      → import_metadata JSON in passages
    track_matches[]     → passages[] (one per track)
}

**Artist Mismatch Handling (AMB-05):**

When `artist_mismatch = true` (artist name similarity < 50%):

| match_pct | Action |
|-----------|--------|
| ≥ 95% | **CREATE** passage, set `artist_verified = 0`, add note in `import_metadata` |
| < 95% | **CREATE** passage, set `artist_verified = 0`, flag as "Review Required" in UI |

**Rationale:** High match percentage (≥95%) with artist mismatch may indicate:
- Different spelling of same artist (e.g., "The Beatles" vs "Beatles")
- Compilation album with various artists
- MusicBrainz data quality issue

Always create the passage (don't discard the match) but flag for user review when confidence is lower.

### TrackMatch → Passage Timing

Each `TrackMatch` from am28 provides track boundaries:

```
TrackMatch {
    detected_duration   → (end_time - start_time) / 28,224,000 seconds
    expected_duration   → cross-reference with MusicBrainz
    error               → stored in import_metadata JSON
    matches             → contributes to overall confidence
}
```

The passage timing is computed as:
- `start_time_ticks` = cumulative sum of prior track durations × 28,224,000
- `end_time_ticks` = start_time_ticks + (detected_duration × 28,224,000)
- `lead_in_start_ticks` = NULL initially (determined by amplitude analysis phase)
- `lead_out_start_ticks` = NULL initially (determined by amplitude analysis phase)

### Edition → Album + Songs

The winning `Edition` from am28 matching provides:

```
Edition {
    track_count         → number of passages to create
    durations[]         → expected durations for validation
    recording_mbids[]   → songs.recording_mbid for each track
    mbids[]             → albums.album_mbid (select best via select_best_mbid())
    artist              → albums.artist_credit
    album               → albums.title
}
```

### Recording MBID Flow

```
am28 MBReleaseDetails.media[].tracks[].recording.id
  ↓
Edition.recording_mbids[track_idx]
  ↓
songs.recording_mbid (create or lookup song)
  ↓
passage_songs.song_id (link passage to song)
```

-----

## Lead-In / Lead-Out Point Determination

While am28 determines **track boundaries** (start/end), it does not determine lead-in/lead-out points. These are determined by a subsequent **amplitude analysis phase**:

### Phase: Amplitude Analysis (Post-am28)

For each passage created from am28:
1. Decode the passage's audio segment
2. Calculate RMS envelope at configurable window size
3. Detect lead-in point: where audio rises above threshold from start
4. Detect lead-out point: where audio falls below threshold before end
5. Store in `lead_in_start_ticks` and `lead_out_start_ticks`

**Default behavior (from SPEC002 XFD-OV-020):**
- If lead-in/lead-out points equal start/end (zero-duration intervals), passage plays with no crossfade overlap
- Fade-in/fade-out points default to same as start/end (pass-through mode, no volume envelope)

**Import Metadata JSON structure:**

```json
{
  "amplitude_analysis": {
    "peak_rms": 0.95,
    "lead_in_detected_s": 2.3,
    "lead_out_detected_s": 3.2,
    "parameters_used": {
      "rms_window_ms": 100,
      "lead_in_threshold_db": -12.0,
      "lead_out_threshold_db": -12.0
    }
  },
  "am28_match": {
    "stage": "album_extractor_2_optimization",
    "threshold_db": -54.0,
    "min_duration_secs": 0.5,
    "mean_error_s": 1.23,
    "match_pct": 100.0,
    "missing_tracks": [5, 12],
    "missing_track_titles": ["Hidden Track", "Bonus Track"],
    "rejected_editions": [
      {"mbid": "abc-123", "match_pct": 65.0, "reason": "artist_mismatch"},
      {"mbid": "def-456", "match_pct": 80.0, "reason": "lower_than_winner"}
    ]
  }
}
```

**Missing Tracks (GAP-04):** For partial albums (75-94% match), `missing_tracks` records which expected track indices were not found in the audio file. `missing_track_titles` provides human-readable names from MusicBrainz.

**Rejected Editions (GAP-09):** For debugging and review, `rejected_editions` records candidate MusicBrainz editions that were tested but not selected, along with reason for rejection.

-----

## MusicBrainz Cache Integration

am28 uses a file-based cache in `./cache/musicbrainz/`. For wkmp-ai integration:

**Option A: Migrate to SQLite tables**
- Use existing `musicbrainz_cache` table from IMPL001
- Store release details as JSON blob
- Benefits: unified storage, ACID transactions, query capability

**Option B: Keep file-based cache + SQLite metadata**
- am28 cache directory structure works well for development
- Add `musicbrainz_cache_metadata` table for tracking cache freshness
- Benefits: easier debugging, external tool compatibility

**Recommendation:** Option A for production, Option B acceptable for initial migration.

-----

## Suggested New Database Tables/Views

### musicbrainz_releases_cache (expanded from IMPL001)

Column name   | Type      | Description
--------------+-----------+--------------------------------------------------
release_mbid  | TEXT      | PRIMARY KEY - MusicBrainz Release ID
artist_credit | TEXT      | Artist credit string
title         | TEXT      | Release title
track_count   | INTEGER   | Number of tracks
total_duration| INTEGER   | Sum of track durations (seconds)
country       | TEXT      | Release country
status        | TEXT      | Official/Bootleg/etc.
media_format  | TEXT      | CD/Digital/Vinyl
release_json  | TEXT      | Full JSON blob from MusicBrainz API
cached_at     | TIMESTAMP | When response was cached

### file_import_sessions

Track batch import operations:

Column name       | Type      | Description
------------------+-----------+--------------------------------------------------
session_id        | TEXT      | PRIMARY KEY - UUID
started_at        | TIMESTAMP | Session start time
completed_at      | TIMESTAMP | Session completion time
source_folder     | TEXT      | Root folder being imported
total_files       | INTEGER   | Files discovered in scan
processed_files   | INTEGER   | Files processed so far
successful_files  | INTEGER   | Files successfully identified
failed_files      | INTEGER   | Files that failed identification
status            | TEXT      | 'RUNNING', 'COMPLETED', 'CANCELLED', 'FAILED'

-----

## Migration Checklist: am28 → wkmp-ai

1. **Code migration:**
   - [ ] Copy `wkmp-ai/examples/am28/` modules to `wkmp-ai/src/`
   - [ ] Replace file-based cache with SQLite cache tables
   - [ ] Integrate with wkmp-ai's db_access thread pattern
   - [ ] Wire up SSE progress reporting

2. **Database migration:**
   - [ ] Add new columns to files table (content_type, match_confidence, etc.)
   - [ ] Add new columns to passages table (track_number, disc_number, matching_stage)
   - [ ] Create musicbrainz_releases_cache table
   - [ ] Create file_import_sessions table

3. **Pipeline integration:**
   - [ ] Step 6 (Content Type Determination) → calls am28 album path
   - [ ] Passage creation from TrackMatch results
   - [ ] Song/Artist/Album record creation from Edition data
   - [ ] passage_songs, song_artists, passage_albums linking

4. **Post-processing integration:**
   - [ ] Amplitude analysis phase for lead-in/lead-out
   - [ ] AcousticBrainz/Essentia flavor retrieval
   - [ ] Final status updates

-----

## After Analysis

**Analysis Date:** 2025-11-26
**Analysis Method:** `/think` Multi-Agent Workflow (8-Phase Analysis)
**Analysis Output:** [refactor1126_analysis.md](refactor1126_analysis.md)

### Quick Summary

**Specification Completeness Review:**
Identified 9 gaps, 7 ambiguities, and 6 conflicts - **all now resolved**.

**Resolutions Applied:**
1. **GAP-01 RESOLVED:** Folder-level album detection - am28 adaptation with order determination
2. **GAP-02 RESOLVED:** Works table schema added
3. **GAP-05 RESOLVED:** Recording MBID changed to INDEX (not UNIQUE)
4. **GAP-06 RESOLVED:** AcousticBrainz → Essentia references updated
5. **All AMB items RESOLVED:** Constants section added with thresholds
6. **All CON items RESOLVED:** Consistent guid usage, match tolerance 3.0s, artist similarity 0.50

**Key Additions to This Document:**
- Algorithm Constants and Thresholds section (lines 201-267)
- Works table schema (lines 298-315)
- Recording MBID index clarification (lines 273-280)
- MULTIPLE_SONGS handling (lines 180-189)
- Artist mismatch handling (lines 643-657)
- Passage ordering constraint (lines 350-353)
- Passage:song timing semantics (lines 451-454)
- Import metadata with missing_tracks and rejected_editions (lines 712-725)

**Next Step:** Run `/plan wkmp-ai/refactor1126.md` to create detailed implementation plan

-----

End of refactor1126.md