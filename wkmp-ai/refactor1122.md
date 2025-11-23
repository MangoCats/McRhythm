# wkmp-ai Major Refactoring 1122

The results from album_matcher_19.rs have been very good so far.  wkmp-ai/examples/album_matcher_20.rs is a cleaned up version of 19, ready to test to confirm equivalent performance.

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
- retrieve and store AcousticBrainz high level characterizations of songs found in the passages found in the audio files

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

## Later steps

start, end, lead-in and lead-out point identification

storing all this in the database tables

Chromaprint / AcoustID song identification

Obtaining/recording AcousticBrainz high level descriptions for songs

## Completed duplicate handling

As mentioned in Step 3 Hash Computation, when a file's status is set to any of the completed statuses (like: INGEST COMPLETE, DUPLICATE, NO AUDIO) then another check is made of the database for any other files with a matching hash value.  If any other files are found with a matching hash and an incomplete status, their status is set to DUPLICATE and their duplicate_id field is set to the file_id of this just finalized file.

This also triggers a mechanism whereby the file, just marked DUPLICATE, is searched for within the list of files either in process or waiting to be processed (by path/filename).  If found, then that file object is signaled (through an AtomicBool) that it has been marked duplicate.  All database write processes, and other convenient / appropriate locations in the file processing code shall check this signal flag and if it is set the file shall abort any further processing - it's already completed, status DUPLICATE.  This AtomicBool prevents the need to check status in the database frequently.

Once a file has completed duplicate handling, it is not processed any more.


-----

## Database schema notes

The database includes:

### Files table

Entries in the Files table include (but are not limited to, and any key may have a value of NULL at any time):

Column name / Key   | Value description
--------------------+----------------------------------------------------------------------------------------
file_id             | a unique identifier
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
duplicate_id        | Usually NULL, but when status is DUPLICATE this contains the file_id of a file with a completed status and identical hash value to this one
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
				   