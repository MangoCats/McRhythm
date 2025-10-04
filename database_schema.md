# McRhythm Database Schema

## Overview

This document describes the complete SQLite database schema for McRhythm. The schema supports:
- Multi-track audio files with configurable crossfade points
- MusicBrainz metadata integration
- Cooldown-based intelligent track selection
- Time-of-day preferences
- Audio feature analysis (AcousticBrainz/local)
- Play history and user feedback (likes/dislikes)
- API response caching

## Schema Diagram (Entity Relationships)

```
files (1) ──→ (N) tracks
                │
                ├──→ (1) audio_features
                ├──→ (N) track_artists ──→ (1) mb_artists
                ├──→ (N) play_history
                ├──→ (N) track_feedback
                ├──→ (N) queue
                └──→ (1) mb_recordings ──→ (N) recording_artists ──→ (1) mb_artists
                         │                 └──→ (N) recording_works ──→ (1) mb_works
                         └──→ (N) songs ──→ (1) mb_artists (primary)

mb_releases (1) ──→ (N) mb_release_artists ──→ (1) mb_artists
            └──→ (N) album_art

mb_tags (N) ──→ (1) entity (artist/recording/release/work)
```

## Table Definitions

### Core Library Tables

#### `files`
Represents physical audio files on disk. Each file may contain one or more tracks.

```sql
CREATE TABLE files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL UNIQUE,
    file_hash TEXT NOT NULL, -- SHA-256
    file_size INTEGER NOT NULL,
    format TEXT NOT NULL, -- MP3, FLAC, OGG, M4A, WAV
    modified_time INTEGER NOT NULL, -- Unix timestamp
    added_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_scanned INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    scan_error TEXT, -- Last error encountered during scan
    CONSTRAINT valid_format CHECK (format IN ('MP3', 'FLAC', 'OGG', 'M4A', 'WAV'))
);

CREATE INDEX idx_files_path ON files(file_path);
CREATE INDEX idx_files_hash ON files(file_hash);
CREATE INDEX idx_files_last_scanned ON files(last_scanned);
```

**Design Notes:**
- `file_hash` enables detection of duplicate files
- `scan_error` stores last error for troubleshooting
- Supports multi-track files (one file → many tracks)

---

#### `tracks`
Individual tracks within files. Core table for playback and metadata.

```sql
CREATE TABLE tracks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,

    -- Position within file (seconds, relative to file start)
    start_time_in_file REAL NOT NULL DEFAULT 0.0,
    end_time_in_file REAL NOT NULL, -- Duration of track in file

    -- Crossfade timings (seconds)
    fade_in_time REAL NOT NULL DEFAULT 0.0,
    lead_in_time REAL NOT NULL DEFAULT 0.0,
    fade_out_time REAL NOT NULL DEFAULT 0.0,
    lead_out_time REAL NOT NULL DEFAULT 0.0,

    -- Metadata from file tags
    title TEXT,
    artist TEXT, -- From file tags (may differ from MusicBrainz)
    album TEXT,
    album_artist TEXT,
    track_number INTEGER,
    disc_number INTEGER,
    year INTEGER,
    genre TEXT,
    duration REAL NOT NULL, -- Track duration in seconds

    -- Lyrics
    lyrics TEXT, -- Plain UTF-8 text

    -- Audio fingerprinting
    acoustid_fingerprint TEXT, -- Chromaprint fingerprint
    acoustid_id TEXT, -- AcoustID identifier

    -- MusicBrainz identifiers
    mb_track_id TEXT, -- MusicBrainz Track MBID
    mb_recording_id TEXT, -- MusicBrainz Recording MBID
    mb_release_id TEXT, -- MusicBrainz Release MBID

    -- Timestamps
    added_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_scanned INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    metadata_updated INTEGER, -- Last time MusicBrainz data was updated

    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
    CONSTRAINT valid_times CHECK (
        start_time_in_file >= 0 AND
        end_time_in_file > start_time_in_file AND
        fade_in_time >= 0 AND
        lead_in_time >= 0 AND
        fade_out_time >= 0 AND
        lead_out_time >= 0 AND
        duration > 0
    )
);

CREATE INDEX idx_tracks_file_id ON tracks(file_id);
CREATE INDEX idx_tracks_mb_recording ON tracks(mb_recording_id);
CREATE INDEX idx_tracks_mb_release ON tracks(mb_release_id);
CREATE INDEX idx_tracks_acoustid ON tracks(acoustid_id);
CREATE INDEX idx_tracks_title ON tracks(title);
CREATE INDEX idx_tracks_artist ON tracks(artist);
```

**Design Notes:**
- Crossfade timing fields: `fade_in_time`, `lead_in_time`, `fade_out_time`, `lead_out_time`
- Default values are 0 (no crossfade)
- `start_time_in_file` and `end_time_in_file` support multi-track files
- File tag metadata separate from MusicBrainz metadata (may differ)
- Lyrics stored as plain UTF-8 text

---

### MusicBrainz Entity Cache

#### `mb_artists`
MusicBrainz artist entities with cooldown configuration.

```sql
CREATE TABLE mb_artists (
    mbid TEXT PRIMARY KEY, -- MusicBrainz ID
    name TEXT NOT NULL,
    sort_name TEXT,
    disambiguation TEXT,
    artist_type TEXT, -- Person, Group, Orchestra, etc.

    -- Cooldown configuration (per artist)
    min_cooldown_seconds INTEGER NOT NULL DEFAULT 7200, -- 2 hours
    ramping_cooldown_seconds INTEGER NOT NULL DEFAULT 14400, -- 4 hours

    -- Last play tracking (for cooldown calculation)
    last_played_at INTEGER, -- Unix timestamp of last play

    -- Metadata
    fetched_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX idx_mb_artists_name ON mb_artists(name);
CREATE INDEX idx_mb_artists_last_played ON mb_artists(last_played_at);
```

**Design Notes:**
- Cooldown defaults: 2 hours minimum, 4 hours ramping
- User-configurable per artist
- `last_played_at` updated when any track by this artist plays

---

#### `mb_recordings`
MusicBrainz recording entities.

```sql
CREATE TABLE mb_recordings (
    mbid TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    length INTEGER, -- Duration in milliseconds
    disambiguation TEXT,
    video BOOLEAN DEFAULT 0,

    fetched_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX idx_mb_recordings_title ON mb_recordings(title);
```

---

#### `mb_works`
MusicBrainz work entities (compositions).

```sql
CREATE TABLE mb_works (
    mbid TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    work_type TEXT, -- Song, Symphony, Opera, etc.
    disambiguation TEXT,

    fetched_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX idx_mb_works_title ON mb_works(title);
```

---

#### `mb_releases`
MusicBrainz release entities (albums).

```sql
CREATE TABLE mb_releases (
    mbid TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    release_date TEXT, -- YYYY-MM-DD or partial
    release_status TEXT, -- Official, Promotion, Bootleg, etc.
    release_group_mbid TEXT,
    barcode TEXT,

    fetched_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX idx_mb_releases_title ON mb_releases(title);
CREATE INDEX idx_mb_releases_date ON mb_releases(release_date);
```

---

#### `mb_release_artists`
Junction table for release → artists relationships.

```sql
CREATE TABLE mb_release_artists (
    release_mbid TEXT NOT NULL,
    artist_mbid TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (release_mbid, artist_mbid),
    FOREIGN KEY (release_mbid) REFERENCES mb_releases(mbid) ON DELETE CASCADE,
    FOREIGN KEY (artist_mbid) REFERENCES mb_artists(mbid) ON DELETE CASCADE
);
```

---

#### `mb_tags`
Genre/style tags from MusicBrainz for any entity type.

```sql
CREATE TABLE mb_tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL, -- 'artist', 'recording', 'release', 'work'
    entity_mbid TEXT NOT NULL,
    tag_name TEXT NOT NULL,
    tag_count INTEGER NOT NULL DEFAULT 0, -- Vote count from MusicBrainz

    UNIQUE (entity_type, entity_mbid, tag_name),
    CONSTRAINT valid_entity_type CHECK (entity_type IN ('artist', 'recording', 'release', 'work'))
);

CREATE INDEX idx_mb_tags_entity ON mb_tags(entity_type, entity_mbid);
CREATE INDEX idx_mb_tags_name ON mb_tags(tag_name);
```

**Design Notes:**
- Generic table for tags on any MusicBrainz entity
- `tag_count` represents community vote count (higher = more agreement)

---

### Relationships

#### `track_artists`
Junction table: tracks → artists (supports multiple artists per track).

```sql
CREATE TABLE track_artists (
    track_id INTEGER NOT NULL,
    artist_mbid TEXT NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT 0, -- Primary performing artist
    position INTEGER NOT NULL DEFAULT 0, -- Order in credits

    PRIMARY KEY (track_id, artist_mbid),
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    FOREIGN KEY (artist_mbid) REFERENCES mb_artists(mbid) ON DELETE CASCADE
);

CREATE INDEX idx_track_artists_track ON track_artists(track_id);
CREATE INDEX idx_track_artists_artist ON track_artists(artist_mbid);
CREATE INDEX idx_track_artists_primary ON track_artists(is_primary) WHERE is_primary = 1;
```

**Design Notes:**
- `is_primary` identifies the primary performing artist (used for song identification)
- `position` preserves credit order

---

#### `recording_works`
Junction table: recordings → works.

```sql
CREATE TABLE recording_works (
    recording_mbid TEXT NOT NULL,
    work_mbid TEXT NOT NULL,

    PRIMARY KEY (recording_mbid, work_mbid),
    FOREIGN KEY (recording_mbid) REFERENCES mb_recordings(mbid) ON DELETE CASCADE,
    FOREIGN KEY (work_mbid) REFERENCES mb_works(mbid) ON DELETE CASCADE
);
```

---

#### `recording_artists`
Junction table: recordings → artists.

```sql
CREATE TABLE recording_artists (
    recording_mbid TEXT NOT NULL,
    artist_mbid TEXT NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT 0,
    position INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (recording_mbid, artist_mbid),
    FOREIGN KEY (recording_mbid) REFERENCES mb_recordings(mbid) ON DELETE CASCADE,
    FOREIGN KEY (artist_mbid) REFERENCES mb_artists(mbid) ON DELETE CASCADE
);

CREATE INDEX idx_recording_artists_primary ON recording_artists(is_primary) WHERE is_primary = 1;
```

---

### Songs

#### `songs`
Combination of recording + primary artist. Core entity for cooldown tracking.

```sql
CREATE TABLE songs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recording_mbid TEXT NOT NULL,
    primary_artist_mbid TEXT NOT NULL,

    -- Cooldown configuration (per song)
    min_cooldown_seconds INTEGER NOT NULL DEFAULT 604800, -- 7 days
    ramping_cooldown_seconds INTEGER NOT NULL DEFAULT 1209600, -- 14 days

    -- Last play tracking
    last_played_at INTEGER, -- Unix timestamp

    -- Computed/cached data
    play_count INTEGER NOT NULL DEFAULT 0,
    like_count INTEGER NOT NULL DEFAULT 0,
    dislike_count INTEGER NOT NULL DEFAULT 0,

    created_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    UNIQUE (recording_mbid, primary_artist_mbid),
    FOREIGN KEY (recording_mbid) REFERENCES mb_recordings(mbid) ON DELETE CASCADE,
    FOREIGN KEY (primary_artist_mbid) REFERENCES mb_artists(mbid) ON DELETE CASCADE
);

CREATE INDEX idx_songs_recording ON songs(recording_mbid);
CREATE INDEX idx_songs_artist ON songs(primary_artist_mbid);
CREATE INDEX idx_songs_last_played ON songs(last_played_at);
```

**Design Notes:**
- As per requirements: Song = Recording + Primary Performing Artist
- Cooldown defaults: 7 days minimum, 14 days ramping
- `last_played_at` updated when any track of this song plays
- Cached counts for performance (updated via triggers or application logic)

---

### Playback & User Interaction

#### `play_history`
Historical record of all track plays.

```sql
CREATE TABLE play_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER NOT NULL,
    song_id INTEGER, -- May be NULL if not yet linked to MusicBrainz

    -- Timing
    played_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')), -- Start time
    duration_played REAL NOT NULL, -- Seconds actually played
    completed BOOLEAN NOT NULL DEFAULT 0, -- True if played to end

    -- Position in queue
    queue_position INTEGER,

    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    FOREIGN KEY (song_id) REFERENCES songs(id) ON DELETE SET NULL
);

CREATE INDEX idx_play_history_track ON play_history(track_id);
CREATE INDEX idx_play_history_song ON play_history(song_id);
CREATE INDEX idx_play_history_time ON play_history(played_at DESC);
CREATE INDEX idx_play_history_completed ON play_history(completed);
```

**Design Notes:**
- `completed = 0` indicates user skipped
- `duration_played` allows detection of partial plays
- Index on `played_at DESC` for recent history queries

---

#### `track_feedback`
User likes and dislikes with ListenBrainz sync tracking.

```sql
CREATE TABLE track_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER NOT NULL,
    song_id INTEGER,

    feedback_type TEXT NOT NULL, -- 'like' or 'dislike'
    position_in_track REAL NOT NULL, -- Seconds into track when feedback given
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    -- ListenBrainz sync tracking
    synced_to_listenbrainz BOOLEAN NOT NULL DEFAULT 0,
    sync_attempted_at INTEGER,

    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    FOREIGN KEY (song_id) REFERENCES songs(id) ON DELETE SET NULL,
    CONSTRAINT valid_feedback CHECK (feedback_type IN ('like', 'dislike'))
);

CREATE INDEX idx_feedback_track ON track_feedback(track_id);
CREATE INDEX idx_feedback_song ON track_feedback(song_id);
CREATE INDEX idx_feedback_type ON track_feedback(feedback_type);
CREATE INDEX idx_feedback_sync ON track_feedback(synced_to_listenbrainz) WHERE synced_to_listenbrainz = 0;
```

**Design Notes:**
- `position_in_track` captures when user gave feedback (may indicate specific section)
- Partial index on `synced_to_listenbrainz = 0` for efficient sync queue queries

---

#### `queue`
Current playback queue.

```sql
CREATE TABLE queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER NOT NULL,
    position INTEGER NOT NULL, -- Order in queue (0 = currently playing)

    -- Auto-selection metadata
    auto_selected BOOLEAN NOT NULL DEFAULT 0,
    selection_probability REAL, -- Probability score when selected
    selected_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    UNIQUE (position),
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

CREATE INDEX idx_queue_position ON queue(position);
CREATE INDEX idx_queue_track ON queue(track_id);
```

**Design Notes:**
- `position = 0` is currently playing track
- `UNIQUE (position)` ensures no gaps in queue
- `selection_probability` stores computed probability for debugging/analysis

---

### Audio Analysis

#### `audio_features`
AcousticBrainz or locally computed audio features.

```sql
CREATE TABLE audio_features (
    track_id INTEGER PRIMARY KEY,

    -- Low-level features
    bpm REAL, -- Beats per minute
    key TEXT, -- Musical key (C, C#, D, etc.)
    scale TEXT, -- Major or Minor

    -- Timbre
    brightness REAL, -- 0.0 to 1.0

    -- Dynamics
    loudness REAL, -- dB
    dynamic_range REAL, -- dB

    -- Rhythm
    danceability REAL, -- 0.0 to 1.0
    rhythm_complexity REAL, -- 0.0 to 1.0

    -- Tonal
    tonality REAL, -- 0.0 to 1.0
    harmonicity REAL, -- 0.0 to 1.0

    -- Energy
    energy REAL, -- 0.0 to 1.0

    -- Voice
    vocal_instrumental REAL, -- 0.0 (instrumental) to 1.0 (vocal)
    speech_ratio REAL, -- 0.0 to 1.0

    -- Mood (derived features)
    valence REAL, -- 0.0 (sad) to 1.0 (happy)
    arousal REAL, -- 0.0 (calm) to 1.0 (energetic)

    -- Source and metadata
    source TEXT NOT NULL, -- 'acousticbrainz' or 'local_analysis'
    analyzed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    analysis_version TEXT, -- Version of analysis algorithm

    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    CONSTRAINT valid_source CHECK (source IN ('acousticbrainz', 'local_analysis'))
);

CREATE INDEX idx_audio_features_bpm ON audio_features(bpm);
CREATE INDEX idx_audio_features_energy ON audio_features(energy);
CREATE INDEX idx_audio_features_danceability ON audio_features(danceability);
CREATE INDEX idx_audio_features_valence ON audio_features(valence);
```

**Design Notes:**
- Features based on AcousticBrainz API (now defunct, use local algorithms)
- Indexes on common filter criteria (bpm, energy, etc.)
- `analysis_version` tracks algorithm changes over time

---

### Album Art

#### `album_art`
Album cover images (front/back).

```sql
CREATE TABLE album_art (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    release_mbid TEXT, -- NULL for non-MusicBrainz albums

    -- For non-MB albums, use hash of album name + artist
    album_identifier TEXT NOT NULL, -- MBID or hash

    image_type TEXT NOT NULL, -- 'front' or 'back'
    image_data BLOB NOT NULL,
    mime_type TEXT NOT NULL, -- image/jpeg, image/png

    -- Image dimensions
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,

    -- Source tracking
    source TEXT NOT NULL, -- 'embedded', 'coverartarchive', 'manual'
    fetched_date INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    UNIQUE (album_identifier, image_type),
    FOREIGN KEY (release_mbid) REFERENCES mb_releases(mbid) ON DELETE CASCADE,
    CONSTRAINT valid_image_type CHECK (image_type IN ('front', 'back')),
    CONSTRAINT valid_source CHECK (source IN ('embedded', 'coverartarchive', 'manual')),
    CONSTRAINT valid_dimensions CHECK (width > 0 AND width <= 1024 AND height > 0 AND height <= 1024)
);

CREATE INDEX idx_album_art_identifier ON album_art(album_identifier);
CREATE INDEX idx_album_art_release ON album_art(release_mbid);
```

**Design Notes:**
- Maximum 1024x1024 pixels (per requirements)
- `album_identifier` handles both MusicBrainz and non-MusicBrainz albums
- `UNIQUE (album_identifier, image_type)` ensures one front, one back per album

---

### Time-Based Preferences

#### `time_preferences`
Configurable time-of-day track selection preferences.

```sql
CREATE TABLE time_preferences (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Time window (24-hour format)
    start_hour INTEGER NOT NULL, -- 0-23
    start_minute INTEGER NOT NULL DEFAULT 0, -- 0-59
    end_hour INTEGER NOT NULL, -- 0-23
    end_minute INTEGER NOT NULL DEFAULT 0, -- 0-59

    -- Days of week (comma-separated: 0=Sunday, 6=Saturday)
    -- NULL or empty means all days
    days_of_week TEXT,

    -- Preference criteria (JSON or multiple related tables)
    -- For now, simplified as tags
    preferred_tags TEXT, -- Comma-separated tag names
    preferred_energy_min REAL, -- 0.0 to 1.0
    preferred_energy_max REAL, -- 0.0 to 1.0
    preferred_valence_min REAL, -- 0.0 to 1.0
    preferred_valence_max REAL, -- 0.0 to 1.0
    preferred_bpm_min REAL,
    preferred_bpm_max REAL,

    -- Weight multiplier for matching tracks
    weight_multiplier REAL NOT NULL DEFAULT 1.0, -- Probability multiplier

    -- Metadata
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    CONSTRAINT valid_hours CHECK (
        start_hour >= 0 AND start_hour <= 23 AND
        end_hour >= 0 AND end_hour <= 23
    ),
    CONSTRAINT valid_minutes CHECK (
        start_minute >= 0 AND start_minute <= 59 AND
        end_minute >= 0 AND end_minute <= 59
    ),
    CONSTRAINT valid_ranges CHECK (
        (preferred_energy_min IS NULL OR (preferred_energy_min >= 0.0 AND preferred_energy_min <= 1.0)) AND
        (preferred_energy_max IS NULL OR (preferred_energy_max >= 0.0 AND preferred_energy_max <= 1.0)) AND
        (preferred_valence_min IS NULL OR (preferred_valence_min >= 0.0 AND preferred_valence_min <= 1.0)) AND
        (preferred_valence_max IS NULL OR (preferred_valence_max >= 0.0 AND preferred_valence_max <= 1.0)) AND
        (preferred_bpm_min IS NULL OR preferred_bpm_min > 0) AND
        (preferred_bpm_max IS NULL OR preferred_bpm_max > 0)
    )
);

CREATE INDEX idx_time_prefs_hours ON time_preferences(start_hour, end_hour);
```

**Design Notes:**
- Supports overlapping time windows
- `weight_multiplier` applied to track probability when time matches
- NULL preference values = no filter on that dimension

---

### API Cache

#### `api_cache`
Generic cache for external API responses.

```sql
CREATE TABLE api_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    api_name TEXT NOT NULL, -- 'acoustid', 'musicbrainz', 'listenbrainz'
    request_key TEXT NOT NULL, -- Hash of request parameters
    response_data TEXT NOT NULL, -- JSON response

    fetched_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    access_count INTEGER NOT NULL DEFAULT 1,
    last_accessed INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    UNIQUE (api_name, request_key)
);

CREATE INDEX idx_api_cache_name ON api_cache(api_name);
CREATE INDEX idx_api_cache_accessed ON api_cache(last_accessed);
```

**Design Notes:**
- `request_key` is hash of request parameters (enables deduplication)
- `last_accessed` enables LRU eviction when cache grows too large
- Indefinite storage per requirements (line 88), deleted oldest when space constrained

---

### Configuration & State

#### `settings`
Application configuration and state (key-value store).

```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    value_type TEXT NOT NULL DEFAULT 'string', -- 'string', 'integer', 'real', 'boolean', 'json'
    description TEXT,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    CONSTRAINT valid_value_type CHECK (value_type IN ('string', 'integer', 'real', 'boolean', 'json'))
);
```

**Common Settings:**
- `current_track_id` (integer) - Currently playing track
- `current_position` (real) - Playback position in seconds
- `volume_level` (integer) - Volume 0-100
- `audio_sink` (string) - Selected audio output
- `fade_curve_profile` (string) - 'exponential', 'cosine', or 'linear'
- `last_scan_time` (integer) - Unix timestamp of last library scan
- `network_failure_count` (integer) - Consecutive network failures
- `queue_state` (json) - Serialized queue state for persistence

---

#### `music_directories`
Configured music library directories.

```sql
CREATE TABLE music_directories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    directory_path TEXT NOT NULL UNIQUE,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    last_scanned INTEGER,
    scan_recursive BOOLEAN NOT NULL DEFAULT 1,
    added_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX idx_music_dirs_enabled ON music_directories(enabled) WHERE enabled = 1;
```

**Design Notes:**
- Separate table (easier management than JSON in settings)
- `enabled` allows temporarily disabling directories
- Partial index on enabled directories for scan queries

---

## Views for Common Queries

### `v_tracks_full`
Tracks with complete metadata (files + MusicBrainz).

```sql
CREATE VIEW v_tracks_full AS
SELECT
    t.id,
    t.title,
    t.artist,
    t.album,
    t.duration,
    t.lyrics,
    f.file_path,
    t.start_time_in_file,
    t.end_time_in_file,
    t.fade_in_time,
    t.lead_in_time,
    t.fade_out_time,
    t.lead_out_time,
    t.mb_recording_id,
    t.mb_release_id,
    r.title AS mb_recording_title,
    rel.title AS mb_release_title,
    GROUP_CONCAT(a.name, ', ') AS mb_artists
FROM tracks t
LEFT JOIN files f ON t.file_id = f.id
LEFT JOIN mb_recordings r ON t.mb_recording_id = r.mbid
LEFT JOIN mb_releases rel ON t.mb_release_id = rel.mbid
LEFT JOIN track_artists ta ON t.id = ta.track_id
LEFT JOIN mb_artists a ON ta.artist_mbid = a.mbid
GROUP BY t.id;
```

---

### `v_songs_with_cooldown`
Songs with real-time cooldown probability calculation.

```sql
CREATE VIEW v_songs_with_cooldown AS
SELECT
    s.id,
    s.recording_mbid,
    s.primary_artist_mbid,
    s.last_played_at,
    s.min_cooldown_seconds,
    s.ramping_cooldown_seconds,
    s.play_count,
    s.like_count,
    s.dislike_count,
    r.title AS recording_title,
    a.name AS artist_name,
    -- Cooldown probability calculation
    CASE
        WHEN s.last_played_at IS NULL THEN 1.0
        WHEN (strftime('%s', 'now') - s.last_played_at) < s.min_cooldown_seconds THEN 0.0
        WHEN (strftime('%s', 'now') - s.last_played_at) < (s.min_cooldown_seconds + s.ramping_cooldown_seconds) THEN
            CAST((strftime('%s', 'now') - s.last_played_at - s.min_cooldown_seconds) AS REAL) / s.ramping_cooldown_seconds
        ELSE 1.0
    END AS song_cooldown_probability,
    -- Artist cooldown
    a.last_played_at AS artist_last_played_at,
    CASE
        WHEN a.last_played_at IS NULL THEN 1.0
        WHEN (strftime('%s', 'now') - a.last_played_at) < a.min_cooldown_seconds THEN 0.0
        WHEN (strftime('%s', 'now') - a.last_played_at) < (a.min_cooldown_seconds + a.ramping_cooldown_seconds) THEN
            CAST((strftime('%s', 'now') - a.last_played_at - a.min_cooldown_seconds) AS REAL) / a.ramping_cooldown_seconds
        ELSE 1.0
    END AS artist_cooldown_probability
FROM songs s
LEFT JOIN mb_recordings r ON s.recording_mbid = r.mbid
LEFT JOIN mb_artists a ON s.primary_artist_mbid = a.mbid;
```

**Design Notes:**
- Computes cooldown probabilities in SQL (0.0 during minimum cooldown, linear ramp after)
- Both song and artist cooldown probabilities available
- Can be multiplied together for final probability (as per requirements line 178)

---

### `v_queue_details`
Queue with track metadata.

```sql
CREATE VIEW v_queue_details AS
SELECT
    q.position,
    q.track_id,
    q.auto_selected,
    q.selection_probability,
    t.title,
    t.artist,
    t.album,
    t.duration,
    f.file_path
FROM queue q
JOIN tracks t ON q.track_id = t.id
JOIN files f ON t.file_id = f.id
ORDER BY q.position;
```

---

## Migration Notes

### Initial Schema Creation

Run the complete schema SQL in order:

1. Core library tables (`files`, `tracks`)
2. MusicBrainz entity tables (`mb_artists`, `mb_recordings`, etc.)
3. Relationship tables (`track_artists`, `recording_artists`, etc.)
4. Songs table
5. Playback tables (`play_history`, `track_feedback`, `queue`)
6. Audio features table
7. Album art table
8. Time preferences table
9. API cache table
10. Configuration tables (`settings`, `music_directories`)
11. Views

### Future Schema Changes

Use SQLite's limited ALTER TABLE support:
- Adding columns: `ALTER TABLE table_name ADD COLUMN column_name TYPE`
- Other changes: Create new table, copy data, drop old, rename new

### Data Integrity

Triggers can be added for:
- Auto-updating `songs.play_count` when `play_history` inserted
- Auto-updating `songs.last_played_at` when track of that song plays
- Auto-updating `mb_artists.last_played_at` when artist's track plays
- Cascading queue position updates when track removed

Example trigger:

```sql
CREATE TRIGGER update_song_play_count
AFTER INSERT ON play_history
WHEN NEW.completed = 1 AND NEW.song_id IS NOT NULL
BEGIN
    UPDATE songs
    SET play_count = play_count + 1,
        last_played_at = NEW.played_at
    WHERE id = NEW.song_id;
END;
```

---

## Performance Considerations

### Critical Indexes

Already defined in schema:
- `idx_play_history_time` - Fast recent play queries
- `idx_mb_artists_last_played` - Artist cooldown queries
- `idx_songs_last_played` - Song cooldown queries
- `idx_audio_features_*` - Time-of-day preference filtering
- Partial index on `synced_to_listenbrainz = 0` - Sync queue

### Query Optimization

For track selection queries (most performance-critical):

```sql
-- Example: Select eligible tracks based on cooldown
SELECT t.id, t.title, s.song_cooldown_probability, s.artist_cooldown_probability
FROM tracks t
JOIN v_songs_with_cooldown s ON t.mb_recording_id = s.recording_mbid
WHERE s.song_cooldown_probability > 0
  AND s.artist_cooldown_probability > 0
ORDER BY RANDOM()
LIMIT 100;
```

### Database Size Estimates

For 10,000 tracks:
- `tracks`: ~3 MB (300 bytes/track)
- `play_history`: ~1 MB/year (assumes 20 plays/day)
- `audio_features`: ~1.5 MB (150 bytes/track)
- `album_art`: ~200 MB (20 KB avg/album × 1000 albums)
- `api_cache`: ~5-10 MB (varies by API usage)
- **Total**: ~250-300 MB for typical library

---

## Outstanding Requirements

The following requirements need further specification before implementation:

1. **Base probability calculation** - How are tracks initially scored before cooldowns applied?
2. **Like/dislike weighting** - How do likes/dislikes affect selection probability?
3. **Time preference application** - Exact formula for applying time-of-day preferences
4. **ListenBrainz integration** - API endpoints, sync frequency, data format
5. **MusicBrainz relationships** (line 12) - Which specific relationships affect selection?

---

## Complete Schema SQL

See appendix or separate SQL file for complete executable schema.
