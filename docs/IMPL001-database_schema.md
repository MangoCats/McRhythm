# WKMP Database Schema

**🗄️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines data structures and schema. Derived from Tier 2 design documents. See [Document Hierarchy](GOV001-document_hierarchy.md).

> **Related Documentation:** [Requirements](REQ001-requirements.md) | [Architecture](SPEC001-architecture.md)

---

## Overview

WKMP uses SQLite as its database engine. The schema is designed to support:
- Music file and passage management
- MusicBrainz entity relationships (songs, artists, works, albums)
- Musical flavor characterization vectors
- Playback history and cooldown tracking
- User preferences and time-based flavor targets
- Queue state persistence

## Global vs User-Scoped Data Design

**WKMP functions like a shared hi-fi system**, not a personal music player:

**Global Data (System-Wide):**
- **Playback settings**: Volume, audio sink, crossfade settings
- **Playback state**: Currently playing passage, queue, play history
- **System configuration**: Module network addresses, timeslots, flavor targets
- **Rationale**:
  - Multiple users may be listening simultaneously (e.g., family in living room)
  - System may run with zero users logged in (background music)
  - User-specific playback settings would be inappropriate for shared listening
  - **Play history is global**: All users see the same play history; cooldowns apply to all users collectively (system assumes all listeners hear all songs as they are played)

**User-Scoped Data:**
- **Likes/Dislikes**: Individual taste preferences tracked per user
- **User accounts**: Authentication credentials and session tokens
- **Rationale**:
  - Personal preferences should not affect shared playback
  - Program Director may (or may not) incorporate user taste into selection algorithm
  - Anonymous users share a common taste profile

**Tables by Scope:**
- **Global**: `settings`, `queue`, `play_history`, `timeslots`, `module_config`, all music metadata
- **User-scoped**: `likes_dislikes`, `users`

## File System Organization

**Root Folder Structure:**
- All audio files, artwork, and other referenced files are organized under a single **root folder**
- The SQLite database file is located in the root folder
- All file paths stored in the database are **relative to the root folder**

**Benefits:**
- **Portability**: The entire collection (database + files) can be moved to a new location by moving the root folder
- **Migration**: Easy to migrate to different systems or backup/restore the complete library
- **Flexibility**: Root folder can be on any storage device or network location

**Path Storage:**
- All `path` and `file_path` columns store paths relative to the root folder
- Example: If root folder is `/home/user/music/` and audio file is `/home/user/music/albums/song.mp3`, the database stores `albums/song.mp3`
- Path separator: Use forward slash (`/`) on all platforms for consistency
  - **Windows Platform Note**: When deployed on Windows, modules must translate forward slashes to backslashes before passing paths to Windows APIs that require backslashes
- The application determines the root folder at runtime from configuration

**Database Location:**
- Database file: `wkmp.db` (or user-specified name) in the root folder
- Application reads root folder location from configuration file or environment variable
- See [Deployment](IMPL004-deployment.md) for configuration details

## Schema Versioning

### `schema_version`

Tracks database schema version for migration management.

| Column | Type | Constraints | Description |
|---|---|---|---|
| version | INTEGER | PRIMARY KEY | Current schema version number |
| applied_at | TIMESTAMP | NOT NULL | When this version was applied |

<a id="users"></a>
### `users`

Stores user account information for both Anonymous and registered users.

| Column | Type | Constraints | Description |
|---|---|---|---|
| guid | TEXT | PRIMARY KEY | Unique user identifier (UUID) |
| username | TEXT | NOT NULL UNIQUE | User's chosen username |
| password_hash | TEXT | NOT NULL | Salted hash of the user's password for authentication (empty string for Anonymous) |
| password_salt | TEXT | NOT NULL | The random salt used for hashing the password (empty string for Anonymous) |
| config_interface_access | BOOLEAN | NOT NULL DEFAULT 1 | Whether user has access to configuration interfaces (1 = enabled, 0 = disabled) |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Pre-populated Records:**
- The database includes one pre-created record for the Anonymous user:
  - `guid`: `00000000-0000-0000-0000-000000000001` (or another fixed UUID)
  - `username`: `Anonymous`
  - `password_hash`: `''` (empty string - no password)
  - `password_salt`: `''` (empty string - no salt needed)
  - `config_interface_access`: `1` (enabled by default)

**Configuration Interface Access:**
- The `config_interface_access` column controls whether a user can view and edit configuration settings via any microservice module's configuration interface
- **Default**: `1` (enabled) for all users
- **Anonymous user default**: Enabled (`1`)
- **New user accounts**: Inherit the Anonymous user's current `config_interface_access` value at account creation time
- **NULL handling**: If value is NULL when read, it is reset to `1` (enabled) and stored
- **Rationale**: Accessibility-first design; prevents accidental lockouts; command-line password reset tool provides recovery path
- See [Architecture - Configuration Interface Access Control](SPEC001-architecture.md) for complete access control specification

**Notes:**
- All users (anonymous and registered) must have a record in this table
- See [User Identity and Authentication](SPEC010-user_identity.md) for password hashing and account management
- The Anonymous user record is created during database initialization and cannot be deleted

## Core Entities

### `files`

Audio files discovered by the library scanner.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique file identifier (UUID) |
| path | TEXT | NOT NULL UNIQUE | File path relative to root folder |
| hash | TEXT | NOT NULL | SHA-256 hash of file contents |
| duration | REAL | | File duration in seconds (NULL = not yet scanned) |
| modification_time | TIMESTAMP | NOT NULL | File last modified timestamp |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Constraints:**
- CHECK: `duration IS NULL OR duration > 0`

**Indexes:**
- `idx_files_path` on `path`
- `idx_files_hash` on `hash`

### `passages`

Audio passages (playable segments) extracted from files.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique passage identifier (UUID) |
| file_id | TEXT | NOT NULL REFERENCES files(guid) ON DELETE CASCADE | Parent audio file |
| start_time_ticks | INTEGER | NOT NULL | Passage start ([SRC-DB-011], ticks from file start, 0 = file start) |
| fade_in_start_ticks | INTEGER | | Fade-in start ([SRC-DB-012], ticks from file start, NULL = use global Crossfade Time) |
| lead_in_start_ticks | INTEGER | | Lead-in start ([SRC-DB-013], ticks from file start, NULL = use global Crossfade Time) |
| lead_out_start_ticks | INTEGER | | Lead-out start ([SRC-DB-014], ticks from file start, NULL = use global Crossfade Time) |
| fade_out_start_ticks | INTEGER | | Fade-out start ([SRC-DB-015], ticks from file start, NULL = use global Crossfade Time) |
| end_time_ticks | INTEGER | NOT NULL | Passage end ([SRC-DB-016], ticks from file start) |

**Tick-Based Timing:**
- All timing fields use INTEGER ticks instead of REAL seconds for lossless precision
- Conversion: ticks = seconds * 28,224,000 ([SPEC017 SRC-TICK-020])
- One tick ≈ 35.4 nanoseconds ([SPEC017 SRC-TICK-030])
- See [SPEC017 Database Storage](SPEC017-sample_rate_conversion.md#database-storage) for complete tick storage specification

| fade_in_curve | TEXT | | Fade-in curve type (see [SPEC002 XFD-CURV-020]: exponential, cosine, linear) (NULL = use global default) |
| fade_out_curve | TEXT | | Fade-out curve type (see [SPEC002 XFD-CURV-030]: logarithmic, cosine, linear) (NULL = use global default) |

> See [SPEC002 Fade Curves](SPEC002-crossfade.md#fade-curves) for curve definitions.
| title | TEXT | | Title from file tags |
| user_title | TEXT | | User-defined passage title (overrides tag title) |
| artist | TEXT | | Artist from file tags |
| album | TEXT | | Album from file tags |
| musical_flavor_vector | TEXT | | JSON blob of AcousticBrainz characterization values (see Musical Flavor Vector Storage) |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Constraints:**
- CHECK: `start_time_ticks >= 0`
- CHECK: `end_time_ticks > start_time_ticks`
- CHECK: `fade_in_start_ticks IS NULL OR (fade_in_start_ticks >= start_time_ticks AND fade_in_start_ticks <= end_time_ticks)`
- CHECK: `lead_in_start_ticks IS NULL OR (lead_in_start_ticks >= start_time_ticks AND lead_in_start_ticks <= end_time_ticks)`
- CHECK: `lead_out_start_ticks IS NULL OR (lead_out_start_ticks >= start_time_ticks AND lead_out_start_ticks <= end_time_ticks)`
- CHECK: `fade_out_start_ticks IS NULL OR (fade_out_start_ticks >= start_time_ticks AND fade_out_start_ticks <= end_time_ticks)`
- CHECK: `fade_in_start_ticks IS NULL OR fade_out_start_ticks IS NULL OR fade_in_start_ticks <= fade_out_start_ticks`
- CHECK: `lead_in_start_ticks IS NULL OR lead_out_start_ticks IS NULL OR lead_in_start_ticks <= lead_out_start_ticks`
- CHECK: `fade_in_curve IS NULL OR fade_in_curve IN ('exponential', 'cosine', 'linear')`
- CHECK: `fade_out_curve IS NULL OR fade_out_curve IN ('logarithmic', 'cosine', 'linear')`

**Indexes:**
- `idx_passages_file_id` on `file_id`
- `idx_passages_title` on `title`

**Notes on denormalized fields:**
- `title`, `artist`, `album` fields are denormalized caches from file tags
- Used for display before MusicBrainz lookup completes or when lookup fails
- Source of truth is `passage_songs` and `passage_albums` relationships
- These fields may become stale if MusicBrainz data is updated
- `user_title` always takes precedence over `title` when set for display purposes
- Both `user_title` and `title` are subject to the "only when different" display logic (see [UI Specification](SPEC009-ui_specification.md#passage-title-display))

### `songs`

Songs are unique combinations of a recording and a weighted set of artists. Each Song has exactly one Recording, but may be closely related to other Songs of the same Work (either by the same artist or other artists).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique song identifier (UUID) |
| recording_mbid | TEXT | NOT NULL | MusicBrainz Recording ID (UUID) |
| work_id | TEXT | | Foreign key to the works table (UUID) |
| related_songs | TEXT | | JSON array of related song GUIDs, ordered from most to least closely related |
| lyrics | TEXT | | Lyrics for this recording (plain UTF-8 text) |
| base_probability | REAL | NOT NULL DEFAULT 1.0 | Base selection probability (0.0-1000.0) |
| min_cooldown | INTEGER | NOT NULL DEFAULT 604800 | Minimum cooldown seconds (default 7 days) |
| ramping_cooldown | INTEGER | NOT NULL DEFAULT 1209600 | Ramping cooldown seconds (default 14 days) |
| last_played_at | TIMESTAMP | | Last time any passage with this song was played |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Constraints:**
- CHECK: `base_probability >= 0.0 AND base_probability <= 1000.0`
- CHECK: `min_cooldown >= 0`
- CHECK: `ramping_cooldown >= 0`

**Indexes:**
- `idx_songs_recording_mbid` on `recording_mbid`
- `idx_songs_last_played` on `last_played_at`

**Related Songs:**
- The `related_songs` field contains a JSON array of song GUIDs, ordered from most to least closely related
- Related songs are typically other recordings of the same Work, either by the same artist or other artists
- This list is populated by Audio Ingest (wkmp-ai) during file ingestion based on MusicBrainz relationships
- Users can edit the related songs list via wkmp-ai's HTTP interface (Full version only)
- Example: `["song-guid-1", "song-guid-2", "song-guid-3"]`

**Lyrics Display Behavior:**
- When displaying lyrics, User Interface (wkmp-ui) follows a fallback chain:
  1. Check the current Song's `lyrics` field - if non-empty, display these lyrics
  2. If empty, iterate through `related_songs` in order (most to least closely related)
  3. Display lyrics from the first related Song with a non-empty `lyrics` field
  4. If no Song in the chain has lyrics, leave the lyrics display area empty

### `artists`

Performing artists from MusicBrainz.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique artist identifier (UUID) |
| artist_mbid | TEXT | NOT NULL UNIQUE | MusicBrainz Artist ID (UUID) |
| name | TEXT | NOT NULL | Canonical artist name |
| base_probability | REAL | NOT NULL DEFAULT 1.0 | Base selection probability (0.0-1000.0) |
| min_cooldown | INTEGER | NOT NULL DEFAULT 7200 | Minimum cooldown seconds (default 2 hours) |
| ramping_cooldown | INTEGER | NOT NULL DEFAULT 14400 | Ramping cooldown seconds (default 4 hours) |
| last_played_at | TIMESTAMP | | Last time any passage by this artist was played |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Constraints:**
- CHECK: `base_probability >= 0.0 AND base_probability <= 1000.0`
- CHECK: `min_cooldown >= 0`
- CHECK: `ramping_cooldown >= 0`

**Indexes:**
- `idx_artists_mbid` on `artist_mbid`
- `idx_artists_last_played` on `last_played_at`

### `works`

Musical works from MusicBrainz (compositions that can have multiple recordings).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique work identifier (UUID) |
| work_mbid | TEXT | NOT NULL UNIQUE | MusicBrainz Work ID (UUID) |
| title | TEXT | NOT NULL | Work title |
| base_probability | REAL | NOT NULL DEFAULT 1.0 | Base selection probability (0.0-1000.0) |
| min_cooldown | INTEGER | NOT NULL DEFAULT 259200 | Minimum cooldown seconds (default 3 days) |
| ramping_cooldown | INTEGER | NOT NULL DEFAULT 604800 | Ramping cooldown seconds (default 7 days) |
| last_played_at | TIMESTAMP | | Last time any passage of this work was played |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Constraints:**
- CHECK: `base_probability >= 0.0 AND base_probability <= 1000.0`
- CHECK: `min_cooldown >= 0`
- CHECK: `ramping_cooldown >= 0`

**Indexes:**
- `idx_works_mbid` on `work_mbid`
- `idx_works_last_played` on `last_played_at`

### `albums`

Albums/releases from MusicBrainz.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique album identifier (UUID) |
| album_mbid | TEXT | NOT NULL UNIQUE | MusicBrainz Release ID (UUID) |
| title | TEXT | NOT NULL | Album title |
| release_date | TEXT | | Release date (ISO 8601 format) |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Indexes:**
- `idx_albums_mbid` on `album_mbid`

**Note:** Album art is stored in the `images` table with `image_type` IN ('album_front', 'album_back', 'album_liner') and `entity_id` = album guid.

<a id="images"></a>
### `images`

Images associated with various entities (songs, passages, albums, artists, works).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique image identifier (UUID) |
| file_path | TEXT | NOT NULL | File path relative to root folder |
| image_type | TEXT | NOT NULL | Type of image (see below) |
| entity_id | TEXT | NOT NULL | UUID of associated entity |
| priority | INTEGER | NOT NULL DEFAULT 100 | Display priority (lower = higher priority) |
| width | INTEGER | | Image width in pixels |
| height | INTEGER | | Image height in pixels |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Constraints:**
- CHECK: `image_type IN ('album_front', 'album_back', 'album_liner', 'song', 'passage', 'artist', 'work', 'logo')`
- CHECK: `priority >= 0`

**Indexes:**
- `idx_images_entity` on `(entity_id, image_type, priority)`
- `idx_images_type` on `image_type`

**Image Types:**
- `album_front`: Album front cover art (entity_id = album guid)
- `album_back`: Album back cover art (entity_id = album guid)
- `album_liner`: Album liner notes images (entity_id = album guid)
- `song`: Song-specific image (entity_id = song guid)
- `passage`: Passage-specific image (entity_id = passage guid)
- `artist`: Artist photo/image (entity_id = artist guid)
- `work`: Work-related image (entity_id = work guid)
- `logo`: WKMP logo (entity_id = 'wkmp')

**Notes:**
- All entity_id values reference internal guid primary keys for consistency
- Priority allows multiple images of same type; UI displays highest priority (lowest number) first
- Logo image is bundled with application; one row with entity_id='wkmp' for consistency

**Image File Storage:**
- Images extracted from audio files are stored in the filesystem alongside the audio files (within the root folder tree)
- Storage location: Same folder as the audio file the artwork relates to
- If multiple audio files in different folders relate to the same artwork, store in the same folder as the first related audio file (rare case)
- Naming convention: Same filename as the source file the art is extracted from
- Filename conflict resolution: Append current time in ISO8601 format before file extension
  - Example: `cover.jpg` → `cover_2025-10-09T12:34:56Z.jpg`
- The `file_path` column stores the path relative to the root folder
  - Example: If audio file is `albums/artist/song.mp3`, artwork might be stored as `albums/artist/song.jpg`
- See [Library Management](SPEC008-library_management.md) for complete artwork extraction and storage workflows

## Relationship Tables (Many-to-Many)

### `passage_songs`

Associates passages with the songs they contain, including timing information for song boundary detection.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| passage_id | TEXT | NOT NULL REFERENCES passages(guid) ON DELETE CASCADE | Passage identifier |
| song_id | TEXT | NOT NULL REFERENCES songs(guid) ON DELETE CASCADE | Song identifier |
| start_time_ms | INTEGER | NOT NULL | Song start time within passage (milliseconds from passage start) |
| end_time_ms | INTEGER | NOT NULL | Song end time within passage (milliseconds from passage start) |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |

**Constraints:**
- PRIMARY KEY: `(passage_id, song_id)`
- CHECK: `start_time_ms >= 0`
- CHECK: `end_time_ms > start_time_ms`

**Indexes:**
- `idx_passage_songs_passage` on `passage_id`
- `idx_passage_songs_song` on `song_id`
- `idx_passage_songs_timing` on `(passage_id, start_time_ms)`

**Notes:**
- Times are relative to passage start (passage.start_time = 0ms for this table)
- Used by Audio Player to detect song boundary crossings during playback
- Enables CurrentSongChanged event emission at correct positions
- Multiple songs may exist within a passage with gaps between them

### `song_artists`

Associates songs with the artists who performed them, and their respective weights.

| Column | Type | Constraints | Description |
|---|---|---|---|
| `song_id` | TEXT | NOT NULL REFERENCES songs(guid) ON DELETE CASCADE | Song identifier |
| `artist_id` | TEXT | NOT NULL REFERENCES artists(guid) ON DELETE CASCADE | Artist identifier |
| `weight` | REAL | NOT NULL | The weight of the artist's contribution (0.0-1.0) |
| `created_at` | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |

**Constraints:**
- PRIMARY KEY: `(song_id, artist_id)`
- CHECK: `weight > 0.0 AND weight <= 1.0`

**Indexes:**
- `idx_song_artists_song` on `song_id`
- `idx_song_artists_artist` on `artist_id`

### `passage_albums`

Associates passages with albums.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| passage_id | TEXT | NOT NULL REFERENCES passages(guid) ON DELETE CASCADE | Passage identifier |
| album_id | TEXT | NOT NULL REFERENCES albums(guid) ON DELETE CASCADE | Album identifier |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |

**Constraints:**
- PRIMARY KEY: `(passage_id, album_id)`

**Indexes:**
- `idx_passage_albums_passage` on `passage_id`
- `idx_passage_albums_album` on `album_id`

**Notes:**
- While passage→album relationship can be derived through passage→song→recording→release,
  this table provides a direct link for performance and handles edge cases:
  - Multiple songs in a passage may come from different albums
  - Passage metadata needs album association before song identification completes
  - Compilation albums where passage spans songs from different source albums
- **Album art selection for multi-album passages**:
  - UI displays artwork based on the currently playing song within the passage
  - When playback position is between songs (in a gap), nearest song determines artwork
  - If a song is associated with multiple albums, all albums' art rotates every 15 seconds
  - See [UI Specification - Album Artwork Display](SPEC009-ui_specification.md#album-artwork-display) for complete display logic

### `song_works`

Associates songs with the works they are recordings of. Many-to-many relationship.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| song_id | TEXT | NOT NULL REFERENCES songs(guid) ON DELETE CASCADE | Song identifier |
| work_id | TEXT | NOT NULL REFERENCES works(guid) ON DELETE CASCADE | Work identifier |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |

**Constraints:**
- PRIMARY KEY: `(song_id, work_id)`

**Indexes:**
- `idx_song_works_song` on `song_id`
- `idx_song_works_work` on `work_id`

**Notes:**
- **Cardinality**: Song → Work is many-to-many (see [ENT-CARD-045](REQ002-entity_definitions.md#ent-card-045))
- **Common cases**:
  - **One work per song**: Standard case for original compositions
  - **Zero works**: Song has no entries in this table (improvisations, sound effects, non-musical passages)
  - **Multiple works per song**: Mashups, medleys, or songs combining multiple source works
- **Mashup example**: Artist creates new song combining two existing works
  - Song associates with new mashup work (if created as a distinct work)
  - Song also associates with both source works that were combined
  - Total: 3 rows in `song_works` for this one song
- **Work probability calculation**: When a song has multiple works, the work cooldown multiplier is the product of all associated works' cooldown multipliers (all must be out of cooldown for song to be selectable)

## Playback & History

### `play_history`

Records of passage playback events (global/system-wide).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique play event identifier (UUID) |
| passage_id | TEXT | NOT NULL REFERENCES passages(guid) ON DELETE CASCADE | Passage that was played |
| timestamp | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When playback started |
| duration_played | REAL | NOT NULL | How long the passage was played (seconds) |
| completed | BOOLEAN | NOT NULL DEFAULT 0 | 1 if played fully, 0 if skipped |

**Indexes:**
- `idx_play_history_passage` on `passage_id`
- `idx_play_history_timestamp` on `timestamp`
- `idx_play_history_passage_time` on `(passage_id, timestamp)` (optimizes cooldown time-range queries)

**Notes:**
- **Global (system-wide)**: Play history is not user-specific. All users see the same play history.
- **Cooldowns apply collectively**: The system assumes all listeners hear all songs as they are played, so cooldowns affect passage selection for everyone.
- `ON DELETE CASCADE` policy: When a passage is deleted from the database, its play history is also deleted
- This maintains database integrity but loses historical statistics for deleted content
- Acceptable trade-off for library management (deleted files should not leave orphaned data)

### `song_play_counts` (View)

Optimized view for efficient play count queries by time period.

**View Definition:**
```sql
CREATE VIEW song_play_counts AS
SELECT
    s.guid as song_id,
    s.recording_mbid,
    COUNT(ph.guid) FILTER (
        WHERE ph.timestamp > datetime('now', '-7 days')
        AND ph.completed = 1
    ) as plays_week,
    COUNT(ph.guid) FILTER (
        WHERE ph.timestamp > datetime('now', '-30 days')
        AND ph.completed = 1
    ) as plays_month,
    COUNT(ph.guid) FILTER (
        WHERE ph.timestamp > datetime('now', '-365 days')
        AND ph.completed = 1
    ) as plays_year,
    COUNT(ph.guid) FILTER (WHERE ph.completed = 1) as plays_all_time,
    MAX(ph.timestamp) as last_played_at
FROM songs s
LEFT JOIN passage_songs ps ON s.guid = ps.song_id
LEFT JOIN play_history ph ON ps.passage_id = ph.passage_id
GROUP BY s.guid;
```

**Columns:**
- `song_id`: Song GUID
- `recording_mbid`: MusicBrainz Recording ID
- `plays_week`: Number of completed plays in past 7 days
- `plays_month`: Number of completed plays in past 30 days
- `plays_year`: Number of completed plays in past 365 days
- `plays_all_time`: Total number of completed plays
- `last_played_at`: Timestamp of most recent play

**Usage:**
- Implements UI requirement for play history display (week/month/year/all-time counts)
- Only counts completed plays (skipped plays excluded)
- Updates automatically as play_history changes
- Efficient for status display queries

### `song_play_history`

Records each time a song is played. Used primarily for cooldown calculations by Program Director.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique play record (UUID) |
| song_id | TEXT | NOT NULL REFERENCES songs(guid) ON DELETE CASCADE | Song played |
| passage_id | TEXT | REFERENCES passages(guid) ON DELETE SET NULL | Passage context (may be NULL if passage deleted) |
| start_timestamp_ms | INTEGER | NOT NULL | Unix milliseconds UTC when song started |
| stop_timestamp_ms | INTEGER | NOT NULL | Unix milliseconds UTC when song stopped |
| audio_played_ms | INTEGER | NOT NULL | Milliseconds of audio actually played |
| pause_duration_ms | INTEGER | NOT NULL DEFAULT 0 | Milliseconds spent in Pause state during this play |

**Indexes:**
- `idx_song_play_history_song` on `song_id`
- `idx_song_play_history_start` on `start_timestamp_ms`

**Notes:**
- One record per song per play (passage with 3 songs → 3 records)
- **Primary use**: Program Director cooldown calculations (per-song timing)
- **Secondary uses**:
  - Skip detection: `audio_played_ms` < expected song duration
  - Rewind detection: `audio_played_ms` > (stop_timestamp_ms - start_timestamp_ms - pause_duration_ms)
  - Clock drift analysis: Compare wall time to audio time accounting for pause
- All timestamps are signed 64-bit integers (Unix milliseconds UTC)
- `passage_id` may be NULL if passage definition deleted after play
- Single table for all songs (not one table per song)

### `likes_dislikes`

**Phase 1 Feature** (Full and Lite versions only)

**User-scoped** feedback on songs, tracked per user identity. This is an example of user-specific data (contrast with global `settings` table).

| Column | Type | Constraints | Description |
|---|---|---|---|
| guid | TEXT | PRIMARY KEY | Unique feedback identifier (UUID) |
| user_id | TEXT | NOT NULL | Identifier for the user providing the feedback |
| song_id | TEXT | NOT NULL REFERENCES songs(guid) ON DELETE CASCADE | Song being rated |
| type | TEXT | NOT NULL | 'like' or 'dislike' |
| weight | REAL | NOT NULL DEFAULT 1.0 | The weight of the Like/Dislike action |
| timestamp | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When feedback was given |

**Constraints:**
- CHECK: `type IN ('like', 'dislike')`
- CHECK: `weight > 0.0`

**Indexes:**
- `idx_likes_dislikes_user_recording` on `user_id`, `recording_id`
- `idx_likes_dislikes_timestamp` on `timestamp`

**Notes:**
- This table stores the outcome of the logic described in `like_dislike.md`. A single user action on a passage may result in multiple rows in this table, one for each affected song.
- The `user_id` references the `users.guid` column, enabling per-user taste profiles
- Anonymous users share the same `user_id`, resulting in a shared taste profile for all anonymous sessions
- Registered users have individual taste profiles based on their unique `user_id`
- **User-scoped vs Global**: Likes/Dislikes are user-specific because they represent individual taste preferences. In contrast, playback settings (volume, audio sink, etc.) are global because WKMP functions like a shared hi-fi system where multiple users may be listening simultaneously.
- **Program Director integration**: The Program Director may (or may not) incorporate user taste profiles into passage selection, depending on configuration and future development
- See [Likes and Dislikes](SPEC006-like_dislike.md) for weight distribution algorithm
- See [Musical Taste](SPEC004-musical_taste.md) for how likes/dislikes build taste profiles

### `queue`

Current playback queue.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique queue entry identifier (UUID) |
| passage_id | TEXT | NOT NULL REFERENCES passages(guid) ON DELETE CASCADE | Passage in queue |
| play_order | INTEGER | NOT NULL | Playback order (gaps allowed for easier reordering) |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When added to queue |

**Indexes:**
- `idx_queue_order` on `play_order`
- `idx_queue_passage` on `passage_id`

**Notes:**
- `play_order`: Integer ordering enables fast SQL retrieval (lowest value plays first)
- Gaps encouraged to avoid expensive renumbering on insertion (e.g., 10, 20, 30...)
- New passages appended with `play_order = last_play_order + 10`
- When inserting between existing entries with no gap, renumber tail: `UPDATE queue SET play_order = play_order + 10 WHERE play_order >= insertion_point`
- API accepts UUID-based insert positions ("after guid-123") which translates to play_order internally
- To get next passage: `SELECT * FROM queue ORDER BY play_order LIMIT 1`
- **Persistence**: Written immediately on every enqueue/dequeue operation
- **Recovery**: Fully recoverable after crash (playback position is not)
- Currently playing tracked via `currently_playing_passage_id` in settings
- Completed passages removed immediately (queue is forward-looking only)
- **Typical queue depth**: 5-10 passages (graceful degradation up to 1000+)
- **Overflow protection**: At 3 min/passage average with +10 increment, 1,225 years until 32-bit integer overflow. Automatic renumbering triggered if play_order exceeds 2,000,000,000.

**Queue Entry Timing Overrides:**
- Queue entries may override passage timing via enqueue API
- Overrides stored as JSON in `settings.queue_entry_timing_overrides`
- Keyed by queue entry guid
- If no override: Use passage timing from passages table

## Time-based Flavor System

### `timeslots`

24-hour schedule defining musical flavor targets.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique timeslot identifier (UUID) |
| start_hour | INTEGER | NOT NULL | Hour when timeslot begins (0-23) |
| start_minute | INTEGER | NOT NULL | Minute when timeslot begins (0-59) |
| name | TEXT | NOT NULL | User-friendly name for this timeslot |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Constraints:**
- CHECK: `start_hour >= 0 AND start_hour <= 23`
- CHECK: `start_minute >= 0 AND start_minute <= 59`
- UNIQUE: `(start_hour, start_minute)`

**Indexes:**
- `idx_timeslots_start` on `(start_hour, start_minute)`

### `timeslot_passages`

Passages that define the musical flavor target for each timeslot.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| timeslot_id | TEXT | NOT NULL REFERENCES timeslots(guid) ON DELETE CASCADE | Timeslot identifier |
| passage_id | TEXT | NOT NULL REFERENCES passages(guid) ON DELETE CASCADE | Passage defining flavor |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |

**Constraints:**
- PRIMARY KEY: `(timeslot_id, passage_id)`

**Indexes:**
- `idx_timeslot_passages_timeslot` on `timeslot_id`
- `idx_timeslot_passages_passage` on `passage_id`

## Configuration & Settings

<a id="module_config"></a>
### `module_config`

Module network configuration for inter-module communication. Each module reads this table on startup to discover other modules' addresses and ports.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| module_name | TEXT | PRIMARY KEY | Module identifier |
| host | TEXT | NOT NULL | Hostname or IP address |
| port | INTEGER | NOT NULL | TCP port number |
| enabled | BOOLEAN | NOT NULL DEFAULT 1 | Whether module is enabled (1) or disabled (0) |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When configuration was last modified |

**Constraints:**
- CHECK: `module_name IN ('audio_player', 'user_interface', 'program_director', 'audio_ingest', 'lyric_editor')`
- CHECK: `port > 0 AND port <= 65535`

**Indexes:**
- `idx_module_config_name` on `module_name`

**Default Values:**

The database is initialized with default module configurations on first run:

| module_name | host | port | enabled |
|-------------|------|------|---------|
| user_interface | 127.0.0.1 | 5720 | 1 |
| audio_player | 127.0.0.1 | 5721 | 1 |
| program_director | 127.0.0.1 | 5722 | 1 |
| audio_ingest | 0.0.0.0 | 5723 | 1 |
| lyric_editor | 0.0.0.0 | 5724 | 1 |

**Initialization:**
- **User Interface (wkmp-ui)** initializes the entire database on first run, including `module_config` table with all default values
- Other modules initialize their required tables if missing on startup
- Each module verifies its entry in `module_config` exists, adding it with defaults if not found

**Notes:**
- All modules read from this table on startup to determine how to connect to other modules
- The `enabled` flag allows modules to be logically disabled without removing configuration
- Host can be `127.0.0.1` for localhost-only, or `0.0.0.0` to bind to all interfaces
- Default bind addresses reflect security and accessibility needs:
  - Internal services (audio_player, program_director): `127.0.0.1` (localhost-only)
  - User-facing web UIs (user_interface, audio_ingest, lyric_editor): `0.0.0.0` for network access, but user_interface defaults to `127.0.0.1` for security (user can change to `0.0.0.0` if needed)
- For distributed deployments, update `host` values to actual IP addresses or hostnames
- Each module binds to its own configured port and uses other modules' configurations to make HTTP requests
- See [Deployment - Module Discovery](IMPL004-deployment.md#module-discovery-via-database) for startup sequence

<a id="settings"></a>
### `settings`

**Global** application settings (key-value store). All settings are system-wide, not per-user.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| key | TEXT | PRIMARY KEY | Setting key |
| value | TEXT | NOT NULL | Setting value (JSON format for complex values) |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When setting was last modified |

**Design Philosophy:**
- **WKMP functions like a shared hi-fi system**: Settings apply to the entire system, not individual users
- **Multiple users may be listening simultaneously**: User-specific settings would be inappropriate
- **System may run with zero users logged in**: Settings must be independent of authentication state
- **User-specific data belongs elsewhere**: See `likes_dislikes` table for user-scoped preferences

### Settings Keys Reference

All runtime configuration is stored in the `settings` table using a key-value pattern. Settings are typed in application code but stored as TEXT in the database.

**Configuration Philosophy:**
- **Database-first**: All runtime settings in `settings` table (per architecture.md)
- **TOML files**: Bootstrap only (root folder path, logging)
- **NULL values**: When a setting is NULL or missing, the application initializes it with the built-in default value and writes that default to the database
- **Built-in defaults**: All default values are defined in application code, NOT in TOML config files

| Key | Type | Default | Purpose | Module | Version |
|-----|------|---------|---------|--------|---------|
| **Playback State** |
| `initial_play_state` | TEXT | `"playing"` | Playback state on app launch ("playing" or "paused") | wkmp-ap | All |
| `currently_playing_passage_id` | TEXT (UUID) | NULL | UUID of passage currently playing | wkmp-ap | All |
| `last_played_passage_id` | TEXT (UUID) | NULL | UUID of last played passage | wkmp-ap | All |
| `last_played_position` | INTEGER (ms) | 0 | Position in milliseconds (updated only on clean shutdown, reset to 0 on queue change) | wkmp-ap | All |
| **Audio Configuration** |
| `volume_level` | REAL | 0.5 | Volume as double 0.0-1.0 (HTTP API also uses 0.0-1.0; UI displays 0-100 with conversion: `display = round(volume * 100.0)`) | wkmp-ap | All |
| `audio_sink` | TEXT | `"default"` | Selected audio output sink identifier | wkmp-ap | All |

> See [SPEC016 Operating Parameters](SPEC016-decoder_buffer_design.md#operating-parameters) for complete audio player operating parameter definitions ([DBD-PARAM-010] through [DBD-PARAM-100]).
| **Event Timing Configuration** |
| `position_event_interval_ms` | INTEGER | 1000 | **Internal event**: Interval for mixer to emit PositionUpdate internal events (milliseconds). Controls song boundary detection accuracy and CPU usage. Lower values = more frequent boundary checks but higher CPU. Range: 100-5000ms. | wkmp-ap | All |
| `playback_progress_interval_ms` | INTEGER | 5000 | **External event**: Interval for emitting PlaybackProgress SSE events to UI clients (milliseconds). Controls UI progress bar update frequency. Based on playback time, not wall clock time. Range: 1000-10000ms. | wkmp-ap | All |

> Note: These event intervals are distinct from [SPEC016 DBD-PARAM-040] output_refill_period (90ms) which controls mixer-to-output buffer refills. See [SPEC016 Operating Parameters](SPEC016-decoder_buffer_design.md#operating-parameters).
| **Database Backup** |
| `backup_location` | TEXT | (same folder as wkmp.db) | Path to backup directory | wkmp-ui | All |
| `backup_interval_ms` | INTEGER | 7776000000 | Periodic backup interval (default: 90 days) | wkmp-ui | All |
| `backup_minimum_interval_ms` | INTEGER | 1209600000 | Minimum time between startup backups (default: 14 days) | wkmp-ui | All |
| `backup_retention_count` | INTEGER | 3 | Number of timestamped backups to keep | wkmp-ui | All |
| `last_backup_timestamp_ms` | INTEGER | NULL | Unix milliseconds of last successful backup | wkmp-ui | All |
| **Crossfade** |
| `global_crossfade_time` | REAL | 2.0 | Global crossfade time in seconds | wkmp-ap | All |
| `global_fade_curve` | TEXT | `"exponential_logarithmic"` | Fade curve pair (options: 'exponential_logarithmic', 'linear_linear', 'cosine_cosine') | wkmp-ap | All |
| **Pause/Resume** |
| `resume_from_pause_fade_in_duration` | REAL | 0.5 | Resume fade-in duration in seconds (range: 0.0-5.0) | wkmp-ap | All |
| `resume_from_pause_fade_in_curve` | TEXT | `"exponential"` | Resume fade-in curve type (options: 'linear', 'exponential', 'cosine') | wkmp-ap | All |
| **Volume Fade Updates** |
| `volume_fade_update_period` | INTEGER | 10 | Volume fade update period in milliseconds (range: 1-100) | wkmp-ap | All |
| **Queue Management** |
| `queue_entry_timing_overrides` | TEXT (JSON) | `{}` | JSON object mapping queue entry guid → timing overrides (see schema below) | wkmp-ap | All |
| `queue_refill_threshold_passages` | INTEGER | 2 | Min passages before refill | wkmp-ap | Full, Lite |
| `queue_refill_threshold_seconds` | INTEGER | 900 | Min seconds before refill (15 minutes) | wkmp-ap | Full, Lite |
| `queue_refill_request_throttle_seconds` | INTEGER | 10 | Min interval between refill requests | wkmp-ap | Full, Lite |
| `queue_refill_acknowledgment_timeout_seconds` | INTEGER | 5 | Timeout for PD acknowledgment | wkmp-ap | Full, Lite |
| `queue_max_size` | INTEGER | 100 | Maximum queue size in passages | wkmp-ap | All |
| `queue_max_enqueue_batch` | INTEGER | 5 | Maximum passages to enqueue at once by Program Director | wkmp-pd | Full, Lite |
| **Module Management** |
| `relaunch_delay` | INTEGER | 5 | Seconds between module relaunch attempts | wkmp-ui | All |
| `relaunch_attempts` | INTEGER | 20 | Max relaunch attempts before giving up | wkmp-ui | All |
| **Session Management** |
| `session_timeout_seconds` | INTEGER | 31536000 | Session timeout duration (default: 1 year) | wkmp-ui | All |
| **File Ingest** |
| `ingest_max_concurrent_jobs` | INTEGER | 4 | Maximum concurrent file processing jobs | wkmp-ai | Full |
| **Library** |
| `music_directories` | TEXT (JSON) | `[]` | JSON array of directories to scan | wkmp-ai | Full |
| `temporary_flavor_override` | TEXT (JSON) | NULL | JSON with target flavor and expiration | wkmp-pd | Full, Lite |
| **HTTP Server Configuration** |
| `http_base_ports` | TEXT (JSON) | `[5720, 15720, 25720, 17200, 23400]` | JSON array of base port numbers | All modules | All |
| `http_request_timeout_ms` | INTEGER | 30000 | Request timeout in milliseconds | All modules | All |
| `http_keepalive_timeout_ms` | INTEGER | 60000 | Keepalive timeout in milliseconds | All modules | All |
| `http_max_body_size_bytes` | INTEGER | 1048576 | Maximum request body size (1 MB) | All modules | All |
| **Program Director** |
| `playback_failure_threshold` | INTEGER | 3 | Failures before stopping automatic selection | wkmp-pd | Full, Lite |
| `playback_failure_window_seconds` | INTEGER | 60 | Time window for failure counting | wkmp-pd | Full, Lite |

**Notes:**
- All settings are stored as TEXT in the database but represent different types in application code
- NULL or missing values are automatically initialized with built-in defaults and written to the database
- Version column indicates which WKMP versions use each setting (All = Full, Lite, Minimal)
- See [Requirements](REQ001-requirements.md) for requirement references (e.g., [REQ-PB-050] for `initial_play_state`)
- See [Crossfade Design](SPEC002-crossfade.md) for complete crossfade system specifications
- See [Architecture](SPEC001-architecture.md) for module responsibilities and configuration patterns

### Event Timing Intervals - Detailed Explanation

WKMP uses an **event-driven architecture** for position tracking with two configurable time intervals that control different aspects of the system:

#### 1. Position Event Interval (`position_event_interval_ms`)

**Purpose:** Controls how often the mixer emits **internal** `PositionUpdate` events

**Stored in:** `settings` table, `position_event_interval_ms` column
**Default:** `1000` milliseconds (1 second)
**Range:** 100-5000ms
**Used by:** wkmp-ap (Audio Player module)

**Application Points:**
- **Mixer thread** (`playback/pipeline/mixer.rs`): Checks frame counter every `get_next_frame()` call
- **Event emission**: When frame count reaches `(position_event_interval_ms / 1000.0) * sample_rate` frames
- At 44.1kHz with 1000ms interval: Event emitted every 44,100 frames

**Affects:**
- **Song boundary detection accuracy**: Lower values = boundaries detected faster
- **CPU usage**: Lower values = more frequent event processing
- **CurrentSongChanged event latency**: Lower values = faster detection of song transitions

**Trade-offs:**
- **100ms**: Very responsive, ~10x CPU overhead, <100ms boundary detection
- **1000ms** (default): Balanced, minimal CPU, ~1s boundary detection
- **5000ms**: Low CPU, delayed boundary detection up to 5s

**Internal Event Flow:**
```
Mixer (every position_event_interval_ms of audio)
  └─> Emit PositionUpdate event
      └─> MPSC channel (capacity: 100 events)
          └─> Position Event Handler receives event
              └─> Checks song timeline for boundaries
```

#### 2. PlaybackProgress Event Interval (`playback_progress_interval_ms`)

**Purpose:** Controls how often **external** `PlaybackProgress` SSE events are sent to UI clients

**Stored in:** `settings` table, `playback_progress_interval_ms` column
**Default:** `5000` milliseconds (5 seconds)
**Range:** 1000-10000ms
**Used by:** wkmp-ap (Audio Player module)

**Application Points:**
- **Position event handler** (`playback/engine.rs`): Tracks elapsed playback time
- **Emission logic**: When `current_position_ms - last_progress_position_ms >= playback_progress_interval_ms`
- Based on **playback time**, not wall clock time (paused = no events)

**Affects:**
- **UI progress bar update frequency**: Lower values = smoother progress bar
- **Network traffic**: Lower values = more SSE events sent to clients
- **UI responsiveness**: Balance between smoothness and efficiency

**Trade-offs:**
- **1000ms**: Very smooth progress bar, higher network traffic
- **5000ms** (default): Balanced, standard 5-second UI updates
- **10000ms**: Minimal traffic, jerky progress bar updates

**External Event Flow:**
```
Position Event Handler
  └─> Receives PositionUpdate (has current position_ms)
  └─> Checks: position_ms - last_emitted >= playback_progress_interval_ms?
      └─> YES: Emit PlaybackProgress SSE event to all clients
```

#### Interval Relationship and Independence

These two intervals are **independent and serve different purposes**:

| Aspect | Position Event Interval | PlaybackProgress Event Interval |
|--------|-------------------------|--------------------------------|
| **Scope** | Internal (wkmp-ap only) | External (SSE to UI clients) |
| **Event Type** | `PlaybackEvent::PositionUpdate` | `WkmpEvent::PlaybackProgress` |
| **Transport** | MPSC channel | HTTP Server-Sent Events |
| **Purpose** | Song boundary detection | UI progress bar updates |
| **Trigger** | Frame count | Playback time elapsed |
| **Typical Ratio** | 1:5 (1s : 5s) | Default configuration |

**Example Scenario (Default Settings):**
```
Time    Position Event (1000ms)    PlaybackProgress Event (5000ms)
0ms     ✓ (check boundaries)       ✓ (emit to UI)
1000ms  ✓ (check boundaries)
2000ms  ✓ (check boundaries)
3000ms  ✓ (check boundaries)
4000ms  ✓ (check boundaries)
5000ms  ✓ (check boundaries)       ✓ (emit to UI)
6000ms  ✓ (check boundaries)
...
```

**Configuration Recommendations:**

- **Standard playback**: position_event_interval_ms=1000, playback_progress_interval_ms=5000
- **Rapid song changes**: position_event_interval_ms=500, playback_progress_interval_ms=2000
- **Low-power devices**: position_event_interval_ms=2000, playback_progress_interval_ms=10000
- **High-precision lyrics**: position_event_interval_ms=100, playback_progress_interval_ms=1000

**Important:** Both intervals are measured in **playback time**, not wall clock time:
- During playback: Intervals advance with audio frames
- When paused: No events emitted (no frame generation)
- After seek: Immediate position event, then regular intervals resume

**Queue Entry Timing Overrides JSON Schema:**

The `queue_entry_timing_overrides` setting stores per-queue-entry timing overrides as a JSON object. Each key is a queue entry GUID, and each value is an object containing override fields. All fields are optional; only overridden values are included.

**Structure:**
```json
{
  "queue-entry-uuid-1": {
    "start_time_ms": 1000,
    "end_time_ms": 180000,
    "lead_in_point_ms": 2000,
    "lead_out_point_ms": 175000,
    "fade_in_point_ms": 500,
    "fade_out_point_ms": 179500,
    "fade_in_curve": "linear",
    "fade_out_curve": "cosine"
  },
  "queue-entry-uuid-2": {
    "end_time_ms": 120000,
    "fade_in_curve": "exponential"
  }
}
```

**Field Definitions:**
- `start_time_ms` (integer, optional): Override passage start time (milliseconds from file start)
- `end_time_ms` (integer, optional): Override passage end time (milliseconds from file start)
- `lead_in_point_ms` (integer, optional): Override lead-in point (milliseconds from passage start)
- `lead_out_point_ms` (integer, optional): Override lead-out point (milliseconds from passage start)
- `fade_in_point_ms` (integer, optional): Override fade-in point (milliseconds from passage start)
- `fade_out_point_ms` (integer, optional): Override fade-out point (milliseconds from passage start)
- `fade_in_curve` (string, optional): Override fade-in curve ("linear", "exponential", "cosine")
- `fade_out_curve` (string, optional): Override fade-out curve ("linear", "logarithmic", "cosine")

**Notes:**
- Empty object `{}` means no overrides (use passage defaults from `passages` table)
- Missing fields mean "use passage default for this field"
- Partial overrides are supported (e.g., override only `end_time_ms` and `fade_in_curve`)
- When queue entry is removed, its override entry should be deleted from this JSON object
- Timing points relative to passage start (not file start), except `start_time_ms` and `end_time_ms`
- See [api_design.md - POST /playback/enqueue](SPEC007-api_design.md#post-playbackenqueue) for override semantics during enqueue

> See [Deployment - HTTP Server Configuration](IMPL004-deployment.md#13-http-server-configuration) for details on port selection algorithm, duplicate instance detection, and bind address configuration.

## External API Caching

### `acoustid_cache`

Cached responses from AcoustID API.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| fingerprint | TEXT | PRIMARY KEY | Chromaprint fingerprint |
| recording_mbid | TEXT | NOT NULL | MusicBrainz Recording ID returned |
| confidence | REAL | | Confidence score from AcoustID |
| cached_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When response was cached |

**Indexes:**
- `idx_acoustid_fingerprint` on `fingerprint`

### `musicbrainz_cache`

Cached metadata from MusicBrainz API.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| mbid | TEXT | PRIMARY KEY | MusicBrainz entity ID (UUID) |
| entity_type | TEXT | NOT NULL | 'recording', 'release', 'artist', or 'work' |
| metadata | TEXT | NOT NULL | JSON blob of fetched metadata |
| cached_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When response was cached |

**Constraints:**
- CHECK: `entity_type IN ('recording', 'release', 'artist', 'work')`

**Indexes:**
- `idx_musicbrainz_mbid` on `mbid`
- `idx_musicbrainz_type` on `entity_type`

### `acousticbrainz_cache`

Cached musical characterization data from AcousticBrainz.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| recording_mbid | TEXT | PRIMARY KEY | MusicBrainz Recording ID (UUID) |
| high_level_data | TEXT | NOT NULL | JSON blob of high-level characterization |
| cached_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When response was cached |

**Indexes:**
- `idx_acousticbrainz_mbid` on `recording_mbid`

## Triggers

### Update Timestamps

Automatically update `updated_at` columns on row modification:

```sql
CREATE TRIGGER update_files_timestamp
AFTER UPDATE ON files
BEGIN
    UPDATE files SET updated_at = CURRENT_TIMESTAMP WHERE guid = NEW.guid;
END;

CREATE TRIGGER update_passages_timestamp
AFTER UPDATE ON passages
BEGIN
    UPDATE passages SET updated_at = CURRENT_TIMESTAMP WHERE guid = NEW.guid;
END;

CREATE TRIGGER update_songs_timestamp
AFTER UPDATE ON songs
BEGIN
    UPDATE songs SET updated_at = CURRENT_TIMESTAMP WHERE guid = NEW.guid;
END;

CREATE TRIGGER update_artists_timestamp
AFTER UPDATE ON artists
BEGIN
    UPDATE artists SET updated_at = CURRENT_TIMESTAMP WHERE guid = NEW.guid;
END;

CREATE TRIGGER update_works_timestamp
AFTER UPDATE ON works
BEGIN
    UPDATE works SET updated_at = CURRENT_TIMESTAMP WHERE guid = NEW.guid;
END;

CREATE TRIGGER update_albums_timestamp
AFTER UPDATE ON albums
BEGIN
    UPDATE albums SET updated_at = CURRENT_TIMESTAMP WHERE guid = NEW.guid;
END;

CREATE TRIGGER update_timeslots_timestamp
AFTER UPDATE ON timeslots
BEGIN
    UPDATE timeslots SET updated_at = CURRENT_TIMESTAMP WHERE guid = NEW.guid;
END;

CREATE TRIGGER update_settings_timestamp
AFTER UPDATE ON settings
BEGIN
    UPDATE settings SET updated_at = CURRENT_TIMESTAMP WHERE key = NEW.key;
END;

CREATE TRIGGER update_module_config_timestamp
AFTER UPDATE ON module_config
BEGIN
    UPDATE module_config SET updated_at = CURRENT_TIMESTAMP WHERE module_name = NEW.module_name;
END;

CREATE TRIGGER update_images_timestamp
AFTER UPDATE ON images
BEGIN
    UPDATE images SET updated_at = CURRENT_TIMESTAMP WHERE guid = NEW.guid;
END;
```

### Cooldown Updates

Automatically update `last_played_at` timestamps when passages are played:

```sql
CREATE TRIGGER update_song_last_played
AFTER INSERT ON play_history
BEGIN
    UPDATE songs
    SET last_played_at = NEW.timestamp
    WHERE guid IN (
        SELECT song_id
        FROM passage_songs
        WHERE passage_id = NEW.passage_id
    );
END;

CREATE TRIGGER update_artist_last_played
AFTER INSERT ON play_history
BEGIN
    UPDATE artists
    SET last_played_at = NEW.timestamp
    WHERE guid IN (
        SELECT sa.artist_id
        FROM passage_songs ps
        JOIN song_artists sa ON ps.song_id = sa.song_id
        WHERE ps.passage_id = NEW.passage_id
    );
END;

CREATE TRIGGER update_work_last_played
AFTER INSERT ON play_history
BEGIN
    UPDATE works
    SET last_played_at = NEW.timestamp
    WHERE guid IN (
        SELECT sw.work_id
        FROM passage_songs ps
        JOIN song_works sw ON ps.song_id = sw.song_id
        WHERE ps.passage_id = NEW.passage_id
    );
END;
```

## Notes

### Data Types

- **TEXT**: UTF-8 text strings, used for UUIDs and general text
- **INTEGER**: SQLite's native integer type, used for counts and positions
- **REAL**: Floating point numbers for time values and probabilities (IEEE 754 double precision, ~15-17 decimal digits)
- **TIMESTAMP**: Stored as TEXT in ISO 8601 format (`YYYY-MM-DD HH:MM:SS`)
  - All timestamps stored in **UTC timezone**
  - Application converts to local time for display
  - SQLite datetime functions (`datetime('now')`, `CURRENT_TIMESTAMP`, etc.) produce UTC by default
  - No timezone suffix stored (implicit UTC)
- **BOOLEAN**: Stored as INTEGER (0 = false, 1 = true)

### UUID Primary Keys

All entity tables use TEXT-based UUID (guid) primary keys instead of auto-incrementing integers:
- Globally unique across all databases
- Enables easier database merging (Full → Lite/Minimal)
- Generated using UUID v4 (random) in application code
- Stored as lowercase hyphenated format: `550e8400-e29b-41d4-a716-446655440000`

### Musical Flavor Vector Storage

The `passages.musical_flavor_vector` field stores a JSON blob containing all AcousticBrainz high-level characterization values. Example structure:

```json
{
  "danceability": {"danceable": 0.7, "not_danceable": 0.3},
  "gender": {"female": 0.4, "male": 0.6},
  "genre_dortmund": {
    "alternative": 0.1,
    "blues": 0.05,
    "electronic": 0.2,
    "folkcountry": 0.05,
    "funksoulrnb": 0.1,
    "jazz": 0.15,
    "pop": 0.2,
    "raphiphop": 0.05,
    "rock": 0.1
  },
  "mood_acoustic": {"acoustic": 0.6, "not_acoustic": 0.4}
}
```

### Migration Strategy

**Current Schema Version:** `0.1` (Development)

**Development Phase:**
- During development, the database schema version is established as `0.1`
- Any changes to the database schema during development are addressed by deletion of existing databases and rebuild from scratch with default values
- No migration scripts are maintained during the development phase

**Post-Release Strategy:**
1. Schema version is tracked in `schema_version` table
2. Migration scripts are numbered sequentially (001_initial.sql, 002_add_works.sql, etc.)
3. On startup, application checks current version and applies pending migrations
4. Each migration is wrapped in a transaction
5. Database is backed up before applying migrations

**Breaking Changes:**
- After initial release, breaking changes are to be avoided
- If a breaking change must be implemented for release, migration strategies shall be developed and implemented before release of the breaking change

**Version Upgrade Paths (Minimal → Lite → Full):**
- Minimal, Lite, and Full versions are implemented by launching different subsets of the modules (microservices)
- Each module creates database tables it requires if they are missing
- Each module initializes missing values it requires with default values encoded in the module
- See [architecture.md#module-launching-process](SPEC001-architecture.md#module-launching-process) for module initialization details

**Database Downgrades:**
- **Important:** Databases cannot be downgraded
- Once a schema migration is applied, reverting to an earlier version is not supported
- Users must backup their database before upgrading if they need to preserve the ability to use an older version

### Performance Considerations

- All foreign keys have corresponding indexes
- Last-played timestamps are indexed for fast cooldown calculations
- Musical flavor vectors are stored as JSON for flexibility (can normalize to columns if performance requires)
- Query plans should be analyzed with `EXPLAIN QUERY PLAN` for selection algorithm queries
- Consider periodic `VACUUM` and `ANALYZE` for optimal performance

### Version-Specific Tables

Some tables are only populated/used in specific versions:

- **Full version**: All tables
- **Lite version**: Read-only external data caches, editable user preferences
- **Minimal version**: Read-only everything, minimal settings

### Future Enhancements

Potential schema additions (not yet specified):

- `genres` table for normalized genre taxonomy
- `playlists` and `playlist_passages` for manual playlists
- `user_preferences_history` for tracking preference changes over time
- `listenbrainz_submissions` queue for pending uploads

**Recently Added:**
- ✅ `images` table for multi-type image storage (songs, passages, albums, artists, works)
- ✅ `song_play_counts` view for efficient time-range play count queries
- ✅ `passages.user_title` column for user-defined passage titles

----
End of document - WKMP Database Schema
