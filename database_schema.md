# McRhythm Database Schema

**🗄️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines data structures and schema. Derived from Tier 2 design documents. See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md)

---

## Overview

McRhythm uses SQLite as its database engine. The schema is designed to support:
- Music file and passage management
- MusicBrainz entity relationships (songs, artists, works, albums)
- Musical flavor characterization vectors
- Playback history and cooldown tracking
- User preferences and time-based flavor targets
- Queue state persistence

## Schema Versioning

### `schema_version`

Tracks database schema version for migration management.

| Column | Type | Constraints | Description |
|---|---|---|---|
| version | INTEGER | PRIMARY KEY | Current schema version number |
| applied_at | TIMESTAMP | NOT NULL | When this version was applied |

### `users`

Stores user account information.

| Column | Type | Constraints | Description |
|---|---|---|---|
| guid | TEXT | PRIMARY KEY | Unique user identifier (UUID) |
| username | TEXT | NOT NULL UNIQUE | User's chosen username |
| password_hash | TEXT | NOT NULL | Salted hash of the user's password for authentication |
| password_salt | TEXT | NOT NULL | The random salt used for hashing the password |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Notes:**
- See [User Identity and Authentication](user_identity.md) for details on password hashing and account management.

## Core Entities

### `files`

Audio files discovered by the library scanner.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique file identifier (UUID) |
| path | TEXT | NOT NULL UNIQUE | Absolute file path |
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
| start_time | REAL | | Passage start time in seconds (NULL = file start) |
| fade_in_point | REAL | | Fade-in point in seconds (NULL = use global Crossfade Time) |
| lead_in_point | REAL | | Lead-in point in seconds (NULL = use global Crossfade Time) |
| lead_out_point | REAL | | Lead-out point in seconds (NULL = use global Crossfade Time) |
| fade_out_point | REAL | | Fade-out point in seconds (NULL = use global Crossfade Time) |
| end_time | REAL | | Passage end time in seconds (NULL = file end) |
| fade_in_curve | TEXT | | Fade-in curve: 'exponential', 'cosine', 'linear' (NULL = use global default) |
| fade_out_curve | TEXT | | Fade-out curve: 'logarithmic', 'cosine', 'linear' (NULL = use global default) |
| title | TEXT | | Title from file tags |
| user_title | TEXT | | User-defined passage title (overrides tag title) |
| artist | TEXT | | Artist from file tags |
| album | TEXT | | Album from file tags |
| lyrics | TEXT | | Passage lyrics (plain UTF-8 text) |
| musical_flavor_vector | TEXT | | JSON blob of AcousticBrainz characterization values (see Musical Flavor Vector Storage) |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Constraints:**
- CHECK: `start_time IS NULL OR start_time >= 0`
- CHECK: `end_time IS NULL OR (start_time IS NULL OR end_time > start_time)`
- CHECK: `fade_in_point IS NULL OR ((start_time IS NULL OR fade_in_point >= start_time) AND (end_time IS NULL OR fade_in_point <= end_time))`
- CHECK: `lead_in_point IS NULL OR ((start_time IS NULL OR lead_in_point >= start_time) AND (end_time IS NULL OR lead_in_point <= end_time))`
- CHECK: `lead_out_point IS NULL OR ((start_time IS NULL OR lead_out_point >= start_time) AND (end_time IS NULL OR lead_out_point <= end_time))`
- CHECK: `fade_out_point IS NULL OR ((start_time IS NULL OR fade_out_point >= start_time) AND (end_time IS NULL OR fade_out_point <= end_time))`
- CHECK: `fade_in_point IS NULL OR fade_out_point IS NULL OR fade_in_point <= fade_out_point`
- CHECK: `lead_in_point IS NULL OR lead_out_point IS NULL OR lead_in_point <= lead_out_point`
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
- `user_title` always takes precedence over `title` when set

### `songs`

Songs are unique combinations of a recording and a weighted set of artists.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique song identifier (UUID) |
| recording_mbid | TEXT | NOT NULL | MusicBrainz Recording ID (UUID) |
| work_id | TEXT | | Foreign key to the works table (UUID) |
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

### `images`

Images associated with various entities (songs, passages, albums, artists, works).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique image identifier (UUID) |
| file_path | TEXT | NOT NULL | Absolute path to image file |
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
- `logo`: McRhythm logo (entity_id = 'mcrhythm')

**Notes:**
- All entity_id values reference internal guid primary keys for consistency
- Priority allows multiple images of same type; UI displays highest priority (lowest number) first
- Logo image is bundled with application; one row with entity_id='mcrhythm' for consistency

## Relationship Tables (Many-to-Many)

### `passage_songs`

Associates passages with the songs they contain.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| passage_id | TEXT | NOT NULL REFERENCES passages(guid) ON DELETE CASCADE | Passage identifier |
| song_id | TEXT | NOT NULL REFERENCES songs(guid) ON DELETE CASCADE | Song identifier |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |

**Constraints:**
- PRIMARY KEY: `(passage_id, song_id)`

**Indexes:**
- `idx_passage_songs_passage` on `passage_id`
- `idx_passage_songs_song` on `song_id`

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
  - Compilation albums where passage spans tracks from different source albums

### `song_works`

Associates songs with the works they are recordings of.

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

## Playback & History

### `play_history`

Records of passage playback events.

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

### `likes_dislikes`

**⚠️ PHASE 2 FEATURE** - Table structure defined but feature implementation deferred.

User feedback on recordings.

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
- This table stores the outcome of the logic described in `like_dislike.md`. A single user action on a passage may result in multiple rows in this table, one for each affected recording.
- The `user_id` is essential for building a persistent taste profile, as described in [User Identity and Authentication](user_identity.md).
- The exact mechanism for how this data will be used by the Program Director to influence passage selection is TBD.

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
- `play_order` determines playback sequence; lowest value plays first
- Gaps in play_order values are allowed and expected (e.g., 10, 20, 30...)
- This design avoids expensive renumbering when inserting/removing entries
- To get next passage: `SELECT * FROM queue ORDER BY play_order LIMIT 1`
- Position 0 is not special; currently playing passage may be tracked in settings table

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

### `settings`

Application settings and user preferences (key-value store).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| key | TEXT | PRIMARY KEY | Setting key |
| value | TEXT | NOT NULL | Setting value (JSON format for complex values) |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When setting was last modified |

**Common keys:**
- `last_played_passage_id`: ID of last played passage
- `last_played_position`: Position in last played passage (seconds)
- `volume_level`: User volume level (0-100)
- `audio_sink`: Selected audio output sink
- `temporary_flavor_override`: JSON with target flavor and expiration
- `music_directories`: JSON array of directories to scan
- `global_crossfade_time`: Global crossfade time in seconds
- `global_fade_curve`: Global fade curve pair (default: 'exponential_logarithmic', options: 'linear_linear', 'cosine_cosine')
- `currently_playing_passage_id`: ID of passage currently playing (alternative to queue position 0)

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

1. Schema version is tracked in `schema_version` table
2. Migration scripts are numbered sequentially (001_initial.sql, 002_add_works.sql, etc.)
3. On startup, application checks current version and applies pending migrations
4. Each migration is wrapped in a transaction
5. Database is backed up before applying migrations

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