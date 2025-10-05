# McRhythm Database Schema

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md) | [Implementation Order](implementation_order.md) | [Document Hierarchy](document_hierarchy.md)

---

**🗄️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines data structures and schema. Derived from Tier 2 design documents. See [Document Hierarchy](document_hierarchy.md).

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
|--------|------|-------------|-------------|
| version | INTEGER | PRIMARY KEY | Current schema version number |
| applied_at | TIMESTAMP | NOT NULL | When this version was applied |

## Core Entities

### `files`

Audio files discovered by the library scanner.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique file identifier (UUID) |
| path | TEXT | NOT NULL UNIQUE | Absolute file path |
| hash | TEXT | NOT NULL | SHA-256 hash of file contents |
| modification_time | TIMESTAMP | NOT NULL | File last modified timestamp |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Indexes:**
- `idx_files_path` on `path`
- `idx_files_hash` on `hash`

### `passages`

Audio passages (playable segments) extracted from files.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique passage identifier (UUID) |
| file_id | TEXT | NOT NULL REFERENCES files(guid) ON DELETE CASCADE | Parent audio file |
| start_time | REAL | NOT NULL | Passage start time in seconds |
| fade_in_time | REAL | NOT NULL DEFAULT 0 | Duration of fade-in in seconds |
| lead_in_time | REAL | NOT NULL DEFAULT 0 | Duration of lead-in in seconds |
| lead_out_time | REAL | NOT NULL DEFAULT 0 | Duration of lead-out in seconds |
| fade_out_time | REAL | NOT NULL DEFAULT 0 | Duration of fade-out in seconds |
| end_time | REAL | NOT NULL | Passage end time in seconds |
| fade_profile | TEXT | NOT NULL DEFAULT 'linear' | Fade profile: 'linear', 'exponential', 'cosine' |
| title | TEXT | | Title from file tags |
| artist | TEXT | | Artist from file tags |
| album | TEXT | | Album from file tags |
| musical_flavor_vector | TEXT | | JSON blob of AcousticBrainz characterization values |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Constraints:**
- CHECK: `start_time >= 0`
- CHECK: `end_time > start_time`
- CHECK: `fade_in_time >= 0`
- CHECK: `lead_in_time >= 0`
- CHECK: `lead_out_time >= 0`
- CHECK: `fade_out_time >= 0`
- CHECK: `fade_profile IN ('linear', 'exponential', 'cosine')`

**Indexes:**
- `idx_passages_file_id` on `file_id`
- `idx_passages_title` on `title`

### `songs`

Songs are unique combinations of a recording and primary performing artist (MusicBrainz concept).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique song identifier (UUID) |
| recording_mbid | TEXT | NOT NULL | MusicBrainz Recording ID (UUID) |
| primary_artist_mbid | TEXT | NOT NULL | MusicBrainz Artist ID (UUID) |
| base_probability | REAL | NOT NULL DEFAULT 1.0 | Base selection probability (0.0-1000.0) |
| min_cooldown | INTEGER | NOT NULL DEFAULT 604800 | Minimum cooldown seconds (default 7 days) |
| ramping_cooldown | INTEGER | NOT NULL DEFAULT 1209600 | Ramping cooldown seconds (default 14 days) |
| last_played_at | TIMESTAMP | | Last time any passage with this song was played |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Constraints:**
- UNIQUE: `(recording_mbid, primary_artist_mbid)`
- CHECK: `base_probability >= 0.0 AND base_probability <= 1000.0`
- CHECK: `min_cooldown >= 0`
- CHECK: `ramping_cooldown >= 0`

**Indexes:**
- `idx_songs_recording_mbid` on `recording_mbid`
- `idx_songs_artist_mbid` on `primary_artist_mbid`
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
| min_cooldown | INTEGER | | Minimum cooldown seconds (TBD - specification needed) |
| ramping_cooldown | INTEGER | | Ramping cooldown seconds (TBD - specification needed) |
| last_played_at | TIMESTAMP | | Last time any passage of this work was played |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

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
| front_art_path | TEXT | | File path to front cover art image |
| back_art_path | TEXT | | File path to back cover art image |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record creation time |
| updated_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | Record last update time |

**Indexes:**
- `idx_albums_mbid` on `album_mbid`

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

### `likes_dislikes`

User feedback on passages (TBD: specification needed for how this affects selection).

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| guid | TEXT | PRIMARY KEY | Unique feedback identifier (UUID) |
| passage_id | TEXT | NOT NULL REFERENCES passages(guid) ON DELETE CASCADE | Passage being rated |
| type | TEXT | NOT NULL | 'like' or 'dislike' |
| timestamp | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When feedback was given |

**Constraints:**
- CHECK: `type IN ('like', 'dislike')`

**Indexes:**
- `idx_likes_dislikes_passage` on `passage_id`
- `idx_likes_dislikes_timestamp` on `timestamp`

### `queue`

Current playback queue.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| position | INTEGER | PRIMARY KEY | Queue position (0 = currently playing) |
| passage_id | TEXT | NOT NULL REFERENCES passages(guid) ON DELETE CASCADE | Passage in queue |
| created_at | TIMESTAMP | NOT NULL DEFAULT CURRENT_TIMESTAMP | When added to queue |

**Indexes:**
- `idx_queue_position` on `position`

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
    WHERE artist_mbid IN (
        SELECT primary_artist_mbid
        FROM songs
        WHERE guid IN (
            SELECT song_id
            FROM passage_songs
            WHERE passage_id = NEW.passage_id
        )
    );
END;

CREATE TRIGGER update_work_last_played
AFTER INSERT ON play_history
BEGIN
    UPDATE works
    SET last_played_at = NEW.timestamp
    WHERE guid IN (
        SELECT work_id
        FROM song_works
        WHERE song_id IN (
            SELECT song_id
            FROM passage_songs
            WHERE passage_id = NEW.passage_id
        )
    );
END;
```

## Notes

### Data Types

- **TEXT**: UTF-8 text strings, used for UUIDs and general text
- **INTEGER**: SQLite's native integer type, used for counts and positions
- **REAL**: Floating point numbers for time values and probabilities
- **TIMESTAMP**: Stored as TEXT in ISO 8601 format (`YYYY-MM-DD HH:MM:SS`)
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

- `featured_artists` table for featured artist handling
- `genres` table for normalized genre taxonomy
- `playlists` and `playlist_passages` for manual playlists
- `user_preferences_history` for tracking preference changes over time
- `listenbrain_submissions` queue for pending uploads
