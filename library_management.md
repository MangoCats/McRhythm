# McRhythm Library Management

**📚 TIER 2 - DESIGN SPECIFICATION**

Defines file scanning, metadata extraction, and MusicBrainz integration workflows. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md) | [Database Schema](database_schema.md) | [Entity Definitions](entity_definitions.md)

---

## Overview

Library Management encompasses the full workflow from discovering audio files on disk to associating them with MusicBrainz entities and extracting musical characterization data.

**Scope:** Full version only. Lite and Minimal versions use pre-built databases exported from Full version.

**Components:**
- File Scanner: Discovers audio files on disk
- Metadata Extractor: Reads embedded tags
- Fingerprint Generator: Creates AcoustID fingerprints
- MusicBrainz Client: Identifies recordings and artists
- AcousticBrainz Client: Fetches musical flavor data
- Passage Editor: User interface for multi-passage files

## File Discovery Workflow

### Initial Library Scan

**Trigger:** User initiates import via WebUI or CLI

**Process:**

1. **Directory Traversal**
   - Recursively walk specified directory paths
   - Follow symlinks (with cycle detection)
   - Skip hidden files (names starting with `.`)
   - Skip system directories (`.git`, `node_modules`, etc.)

2. **File Filtering**
   - Accept extensions: `.mp3`, `.flac`, `.ogg`, `.oga`, `.m4a`, `.mp4`, `.wav`
   - Case-insensitive extension matching
   - Reject non-audio files, corrupt files (detected during metadata extraction)

3. **Hash Calculation**
   - Compute SHA-256 hash of file contents
   - Used for duplicate detection and change detection
   - Store hash in `files` table

4. **Database Recording**
   - Insert file record with path, hash, modification time
   - Initially `duration` is NULL (populated during metadata extraction)
   - Create default single passage per file (user can split later)

**Performance Considerations:**
- Process files in batches of 100 for database insertions
- Hash computation may be I/O bound on HDDs
- Progress reporting via UI (files scanned, errors encountered)

### Incremental Scan (Subsequent Scans)

**Trigger:** User initiates rescan, or periodic background scan (configurable)

**Change Detection:**

1. **Deleted Files**
   - Query all file paths from database
   - Check filesystem existence
   - If file missing: Delete file record (cascades to passages via foreign key)

2. **Modified Files**
   - Compare modification time and file size
   - If changed: Recompute hash
   - If hash changed: Mark for metadata re-extraction
   - Preserve user-edited passage data if possible

3. **New Files**
   - Same process as initial scan for new files
   - No action for unchanged files

**Optimization:** Use filesystem modification time as first-pass filter before hashing

## Metadata Extraction

### Tag Parsing

**Supported Formats:**
- **MP3**: ID3v2.3, ID3v2.4 tags (via `id3` or `lofty` Rust crate)
- **FLAC**: Vorbis Comments (via `metaflac` or `lofty`)
- **OGG/OGA**: Vorbis Comments (via `lewton` + manual parsing or `lofty`)
- **M4A/MP4**: iTunes-style MP4 tags (via `mp4ameta` or `lofty`)
- **WAV**: RIFF INFO tags (via `hound` + custom parser or `lofty`)

**Recommended Library:** `lofty` crate (unified interface for all formats)

**Extracted Fields:**

| Tag Field | Database Column | Fallback |
|-----------|----------------|----------|
| Title | `passages.title` | Filename (without extension) |
| Artist | `passages.artist` | "Unknown Artist" |
| Album | `passages.album` | "Unknown Album" |
| Album Artist | Used for MusicBrainz lookup | Falls back to Artist |
| Track Number | Not stored (used for sorting during import) | - |
| Disc Number | Not stored | - |
| Year/Date | Not stored directly | May be fetched from MusicBrainz |
| Genre | Not stored (MusicBrainz tags preferred) | - |
| Duration | `files.duration` | Computed via audio decoding |
| Embedded Cover Art | Extracted to image file | - |

**Fallback to Filename Parsing:**

If tags are completely missing:

```
Artist - Album - 01 - Title.mp3  →  Artist: "Artist", Album: "Album", Title: "Title"
01 - Title.mp3                   →  Artist: "Unknown Artist", Title: "Title"
Title.mp3                        →  Artist: "Unknown Artist", Title: "Title"
```

Regex patterns for common naming conventions (flexible, best-effort)

### Cover Art Extraction

**Process:**

1. **Check for embedded art**
   - ID3v2: APIC frame
   - Vorbis: METADATA_BLOCK_PICTURE
   - MP4: `covr` atom

2. **Extract image data**
   - Decode image format (JPEG, PNG, GIF, BMP)
   - Validate image (non-corrupt, reasonable dimensions)

3. **Save to disk**
   - Filename: `{audio_filename}.cover.{ext}`
   - Example: `song.mp3` → `song.mp3.cover.jpg`
   - Same directory as audio file
   - If file exists, check hash before overwriting

4. **Resize if needed**
   - If width > 1024 or height > 1024:
     - Resize to max 1024x1024 (preserve aspect ratio)
     - Use high-quality resampling (Lanczos3)
   - Library: `image` crate

5. **Record in database**
   - Insert into `images` table
   - `image_type` = 'album_front' (assumption for embedded art)
   - `entity_id` = album guid (if album identified, else passage guid)
   - `file_path` = absolute path to saved image

**Note:** External cover art (from MusicBrainz) handled separately (see MusicBrainz Integration below)

## Audio Fingerprinting

### Chromaprint Integration

**Library:** `chromaprint` Rust bindings (or FFI to C library)

**Process:**

1. **Decode audio to PCM**
   - Use GStreamer pipeline: `filesrc → decodebin → audioconvert → audioresample`
   - Target format: 16-bit signed integer, mono, 11025 Hz (Chromaprint requirement)
   - Duration: Full passage (or first 120 seconds for very long passages)

2. **Generate fingerprint**
   - Feed PCM data to Chromaprint algorithm
   - Produces base64-encoded fingerprint string
   - Typically 20-200 characters depending on duration

3. **Store fingerprint**
   - Insert into `acoustid_cache` table (before API query)
   - `fingerprint` column stores the base64 string
   - Associate with passage (may need intermediate table)

### AcoustID API Query

**Endpoint:** `https://api.acoustid.org/v2/lookup`

**Request:**
```http
POST /v2/lookup
Content-Type: application/x-www-form-urlencoded

client=<API_KEY>&duration=<seconds>&fingerprint=<base64>&meta=recordings+releasegroups
```

**Parameters:**
- `client`: AcoustID API key (from environment variable)
- `duration`: Passage duration in seconds (integer)
- `fingerprint`: Chromaprint base64 string
- `meta`: Request MusicBrainz Recording IDs and Release Group IDs

**Response:**
```json
{
  "status": "ok",
  "results": [
    {
      "id": "acoustid-uuid",
      "score": 0.95,
      "recordings": [
        {
          "id": "mbid-recording-uuid",
          "title": "Song Title",
          "artists": [{"id": "mbid-artist-uuid", "name": "Artist Name"}],
          "releasegroups": [{"id": "mbid-releasegroup-uuid", "title": "Album Title"}]
        }
      ]
    }
  ]
}
```

**Rate Limiting:**
- Max 3 requests per second (per AcoustID terms)
- Implement token bucket or leaky bucket rate limiter
- Queue fingerprints for batch processing

**Caching:**
- Store response in `acoustid_cache.recording_mbid`
- Store confidence score in `acoustid_cache.confidence`
- Cache indefinitely (delete oldest entries only if storage constrained)

**Error Handling:**
- No match (empty results): User must manually identify passage
- Network error: Retry with exponential backoff (see [Requirements - Network Error Handling](requirements.md#network-error-handling))
- Rate limit error: Wait and retry

## MusicBrainz Integration

### Recording Lookup

**Input:** MusicBrainz Recording ID from AcoustID

**Endpoint:** `https://musicbrainz.org/ws/2/recording/{mbid}?inc=artists+releases+tags`

**Request:**
```http
GET /ws/2/recording/{mbid}?inc=artists+releases+tags
Accept: application/json
User-Agent: McRhythm/1.0.0 ( contact@example.com )
```

**Response:** (Simplified)
```json
{
  "id": "mbid-recording-uuid",
  "title": "Song Title",
  "artist-credit": [
    {"artist": {"id": "mbid-artist1-uuid", "name": "Artist 1"}},
    {"artist": {"id": "mbid-artist2-uuid", "name": "Artist 2"}}
  ],
  "releases": [
    {
      "id": "mbid-release-uuid",
      "title": "Album Title",
      "date": "2020-05-15"
    }
  ],
  "tags": [
    {"name": "rock", "count": 5},
    {"name": "alternative", "count": 3}
  ]
}
```

**Data Extraction:**

1. **Create/Update Song**
   - `recording_mbid` = Recording ID
   - Create a weighted artist set from the `artist-credit` list. By default, all artists can be given equal weight, but this should be configurable by the user later.
   - Check for existing song (recording + weighted artist set), create if needed

2. **Create/Update Artists**
   - For each artist in artist-credit:
     - `artist_mbid` = Artist ID
     - `name` = Artist name
     - Insert or update in `artists` table

3. **Create/Update Albums**
   - For each release:
     - `album_mbid` = Release ID
     - `title` = Release title
     - `release_date` = Date (ISO 8601 format)
     - Insert or update in `albums` table
     - Create `passage_albums` relationship

4. **Store Tags**
   - Take top 10 tags by count
   - Store in `musicbrainz_cache.metadata` as JSON
   - Not currently used for selection (future enhancement)

### Work Lookup

**Endpoint:** `https://musicbrainz.org/ws/2/work/{mbid}`

**When:** If Recording has associated Work (found in Recording lookup response)

**Process:**
- Fetch Work metadata
- Create `works` table entry
- Create `song_works` relationship

**Note:** Many recordings don't have associated Works (especially popular music)

### Cover Art Fetch

**Source:** Cover Art Archive (https://coverartarchive.org/)

**Endpoint:** `https://coverartarchive.org/release/{release-mbid}`

**Response:**
```json
{
  "images": [
    {
      "types": ["Front"],
      "front": true,
      "image": "https://coverartarchive.org/release/{mbid}/12345.jpg",
      "thumbnails": {
        "small": "https://...",
        "large": "https://..."
      }
    },
    {
      "types": ["Back"],
      "back": true,
      "image": "https://coverartarchive.org/release/{mbid}/67890.jpg"
    }
  ]
}
```

**Download Process:**

1. **Fetch JSON** to get image URLs
2. **Download images** (Front, Back, optionally Liner/Booklet)
3. **Save to disk:**
   - `{album_mbid}.front.jpg`
   - `{album_mbid}.back.jpg`
   - `{album_mbid}.liner.{ext}` (if available)
   - Same directory as audio files (or user-designated art directory)
4. **Resize** if > 1024x1024 (same as embedded art)
5. **Record in database:**
   - `images` table with `image_type` = 'album_front', 'album_back', 'album_liner'
   - `entity_id` = album guid

**Caching:**
- Store fetched art URLs in `musicbrainz_cache`
- Check for existing files before re-downloading

**Fallback:**
- If Cover Art Archive has no images, keep embedded art
- If neither exists, use placeholder or no image

### Rate Limiting

**MusicBrainz Terms:**
- Max 1 request per second
- Respect `X-RateLimit-*` headers if present

**Implementation:**
- Use `governor` crate or manual rate limiter
- Queue requests and process sequentially

**User-Agent Requirement:**
- Must include application name and contact info
- Example: `McRhythm/1.0.0 ( https://github.com/user/mcrhythm )`

## Multi-Passage Files

**[LIB-MPF-010]** McRhythm supports the segmentation of a single audio file (e.g., a full album rip) into multiple, distinct Passages. The detailed workflow for this process, including automatic silence detection, MusicBrainz release matching, and manual user review, is specified in the [Audio File Segmentation](audio_file_segmentation.md) document.

## AcousticBrainz Integration

### Fetching Musical Flavor Data

**Input:** MusicBrainz Recording ID

**Endpoint:** `https://acousticbrainz.org/api/v1/{recording-mbid}/high-level`

**Request:**
```http
GET /api/v1/{recording-mbid}/high-level
Accept: application/json
```

**Response:** See [sample_highlevel.json](sample_highlevel.json) for complete example

**Abbreviated Response:**
```json
{
  "highlevel": {
    "danceability": {
      "all": {
        "danceable": 0.72,
        "not_danceable": 0.28
      }
    },
    "genre_dortmund": {
      "all": {
        "alternative": 0.1,
        "electronic": 0.3,
        "pop": 0.25,
        "rock": 0.35
      }
    }
  }
}
```

**Data Storage:**

1. **Cache full response**
   - Store in `acousticbrainz_cache.high_level_data` (JSON text column)
   - Never expires (AcousticBrainz project is discontinued, data is static)

2. **Extract flavor vector**
   - Parse all `highlevel.*` dimensions
   - Store in `passages.musical_flavor_vector` (JSON column)
   - Format: See [Musical Flavor](musical_flavor.md#quantitative-definition)

**Error Handling:**
- 404 Not Found: Recording has no AcousticBrainz data
  - Fallback to local Essentia analysis (Full version only)
  - Or mark passage as "no flavor data" (cannot be auto-selected)

### Local Essentia Analysis (Full Version Only)

**When:** AcousticBrainz data unavailable (404 response)

**Library:** `essentia` (C++ library with Python or Rust bindings)

**Process:**

1. **Decode passage audio** to WAV format

2. **Run Essentia high-level extractor**
   - Command: `essentia_streaming_extractor_music input.wav output.json`
   - Produces same JSON structure as AcousticBrainz

3. **Parse output** and store in database (same as AcousticBrainz)

**Performance:**
- CPU-intensive (~30 seconds per 3-minute passage on desktop)
- Show progress in UI ("Analyzing passage 5 of 127...")
- Run in background thread (non-blocking)

**Raspberry Pi Optimization:**
- May take 2-3 minutes per passage on Pi Zero2W
- Consider pre-computing on desktop (Full version), then export database to Lite version

## Lyrics Input

### WebUI Lyrics Editor

**Page:** `/lyrics/:passage_id`

**Layout:**

```
┌─────────────────────────────────────────────────┐
│  Editing Lyrics for: {Passage Title}            │
├─────────────────────────────────────────────────┤
│                                                  │
│  ┌──────────────────┐  ┌──────────────────────┐ │
│  │ Lyrics Editor    │  │ Web Search Helper    │ │
│  │                  │  │                      │ │
│  │ ┌──────────────┐ │  │ Search: [        ] │ │
│  │ │              │ │  │                      │ │
│  │ │  [textarea]  │ │  │ <iframe with search  │ │
│  │ │              │ │  │  results from        │ │
│  │ │              │ │  │  lyrics site>        │ │
│  │ └──────────────┘ │  │                      │ │
│  │                  │  │                      │ │
│  │  [Submit] [Cancel]  │                      │ │
│  └──────────────────┘  └──────────────────────┘ │
└─────────────────────────────────────────────────┘
```

**Features:**
- **Left pane:** Textarea for lyrics input (plain UTF-8 text, multi-line)
- **Right pane:** Embedded iframe for web search (e.g., Google search for "{Song Title} {Artist} lyrics")
  - Facilitates copy-paste from lyrics websites
  - User manually searches and copies
- **Submit button:** Saves lyrics to database
  - Overwrites existing lyrics (last write wins)
  - No version history (future enhancement)

**API Call:**
```http
PUT /api/lyrics/{passage_id}
Content-Type: application/json

{
  "lyrics": "First line\nSecond line\nChorus..."
}
```

**Storage:**
- `passages.lyrics` column (TEXT, plain UTF-8)
- No formatting (user may include newlines, whitespace)

## Progress Reporting and Events

### UI Progress Updates

**During Import:**
- Emit `LibraryScanProgress` events (custom event type, not in event_system.md yet)
  - Files scanned: 523/1024
  - Current file: `/path/to/file.mp3`
  - Errors encountered: 3

**On Completion:**
- Emit `LibraryScanCompleted` event (see [Event System](event_system.md))
  - `files_added`: 127
  - `files_updated`: 15
  - `files_removed`: 3
  - `duration`: 45.3 seconds

**Error Reporting:**
- Log errors to stderr (developer interface)
- Show summary in UI: "Import completed with 3 errors. See log for details."

## Database Schema Integration

See [Database Schema](database_schema.md) for complete table definitions:
- `files` table: File paths, hashes, modification times
- `passages` table: Passage boundaries, metadata
- `songs`, `artists`, `works`, `albums` tables: MusicBrainz entities
- `passage_songs`, `passage_albums`, `song_works`: Relationships
- `acoustid_cache`, `musicbrainz_cache`, `acousticbrainz_cache`: API response caching

## Testing Considerations

**Test Cases:**
- File scanning with various directory structures (nested, symlinks, hidden files)
- Metadata extraction for all supported formats (MP3, FLAC, OGG, M4A, WAV)
- Missing/corrupt tags (fallback to filename parsing)
- Duplicate detection (same hash, different paths)
- Multi-passage file segmentation (silence detection edge cases)
- AcoustID/MusicBrainz API error handling (network failures, rate limits, 404s)
- Cover art extraction and resizing
- Concurrent import operations (if supported)

**Mock External APIs:**
- Use `mockito` or similar to mock AcoustID/MusicBrainz responses
- Test offline mode (cached data only)

----
End of document - McRhythm Library Management
