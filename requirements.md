# McRhythm Requirements

**📜 TIER 1 - AUTHORITATIVE SOURCE DOCUMENT**

This document is the **top-level specification** defining WHAT McRhythm must do. Other documents are designed to satisfy these requirements.

**Update Policy:** ✅ Product decisions only | ❌ NOT derived from design/implementation

> See [Document Hierarchy](document_hierarchy.md) for complete update policies and change control process.

> **Related Documentation:** [Architecture](architecture.md) | [Crossfade Design](crossfade.md) | [Musical Flavor](musical_flavor.md) | [Event System](event_system.md) | [Requirements Enumeration](requirements_enumeration.md)

---

## Overview
McRhythm is a music player that selects passages to play based on user preferences for [musical flavor](musical_flavor.md#quantitative-definition) at various times of day.

## Core Features
- Plays passages from local files (.mp3 and similar)
  - Identifies one or multiple passage start / stop points and crossfade points within each music file
- Records when passages are played to avoid repetition
- Cross references passages to the MusicBrainz database for:
  - identification of the song(s) contained in the passage (see Definitions for definition of song in this context)
  - identification of other relationships that may influence selection of passages for enqueueing
- Cross references passages to the AcousticBrainz database when possible, identifying musical character of each passage
  - In absence of AcousticBrainz data will use AcousticBrainz algorithms (Essentia) locally to generate musical flavor values for the passage (Full version only)
- Local copies of all relevant subsets of database information enabling offline operation
- Web-based UI
  - Primary mode of operation is automatic, without user intervention
    - Auto-start on boot (of Linux / Windows / OS-X systems)
      - systemd service on Linux
      - Task scheduler launched service on Windows
      - launchd on OS-X
    - Auto-selection of passages to play
      - Automatically select passages until at least three passages are either currently playing, paused
        or waiting in the queue and  at least 15 minutes of total passage playing time is remaining 
        in the queue.
      - Enter "Pause" mode (same behavior as if the user clicked on the Pause control) when no passages
        with associated songs are available to enqueue, because the library lacks passages with songs
        which are not currently in 0 probability cooldown.
  - Shows passage currently playing
    - Shows associated album art or other still image(s) associated with the passage when available
    - Shows passage lyrics when available
  - Shows passages queued for playing next
  - Manual user controls allow the user to control and configure the system when they want to
  - Access manual controls from phone and desktop browsers
- Audio output to OS standard output channels
  - Analog
  - HDMI
  - Bluetooth
- Multiple users may interact with the WebUI
  - Real-time ui updates via Server Sent Events keep all users views in sync
  - Single passage queue for all users
  - Edge case definitions:
    - When one skip click is received, any other skip clicks received within the next 5 seconds are ignored
    - Queue operations are limited to: add and remove passage
      - whenever an add passage command is received, that passage is added to the end of the queue
      - if two or more remove passage commands are received for one passage, the later commands are ignored
    - Lyric editing includes a "Submit" button, whenever a new lyric text is submitted it is recorded - overwriting previously submitted lyric texts.
- Passages play continuously (when not paused by user)
    
## Additional Features
Planned for later development:
- Interface to ListenBrainz to inform future listening choices based on past likes and dislikes
- Mobile (Android, iOS) versions

## Three Versions

**Default Behavior:**
When not otherwise specified, requirements apply to all versions

### Full Version
**Target Platforms:** Linux desktop, Windows, macOS

**Features:**
- Player and Program Director (passage selector)
- All database building and maintenance
- File scanning and library management
- Essentia local analysis for musical flavor
- Preference editing (timeslots, base probabilities)
- MusicBrainz/AcousticBrainz integration
- For future development:
  - ListenBrainz integration

**Resource Profile:**
- CPU: Higher (Essentia analysis during import)
- Disk I/O: Higher (file scanning)
- Memory: ~512MB typical
- Network: Required for initial setup, optional for ongoing use

### Lite Version
**Target Platforms:** Raspberry Pi Zero2W, Linux desktop, Windows, macOS

**Features:**
- Player and Program Director (passage selector)
- Preference editing (timeslots, base probabilities)
- Uses pre-built static database from Full version
- Read-only external data (MusicBrainz/AcousticBrainz cached)
- No file scanning, no Essentia
- For future development:
  - ListenBrainz integration for feedback sync

**Resource Profile:**
- CPU: Moderate (playback + selection only)
- Disk I/O: Low (no scanning, read-only database)
- Memory: ~256MB typical
- Network: Optional (for ListenBrainz sync)

### Minimal Version
**Target Platforms:** Raspberry Pi Zero2W, embedded systems, resource-constrained devices

**Features:**
- Player and Program Director only
- Read-only database and preferences
- No editing capabilities
  - No like/dislike
  - No changing of flavor profiles
- Pre-configured timeslots and probabilities
- Smallest memory footprint
- No network access

**Resource Profile:**
- CPU: Minimal (playback + basic selection)
- Disk I/O: Minimal (read-only database)
- Memory: <256MB typical
- Network: None required

### Build Strategy

See [Implementation Order - Version Builds](implementation_order.md#27-version-builds-fulliteminimal) for Rust feature flags and conditional compilation approach.

**Database Deployment:**
- Full version exports complete database snapshot
- Lite/Minimal versions import read-only database
- Migration tools for version upgrades and cross-platform deployment

## Technical Requirements
- Target platforms:
  - Primary target: Raspberry Pi Zero2W (Lite and Minimal versions)
  - Generic Linux
  - Windows
  - MacOS
  - Later targets (will use different technical stack, e.g. Flutter instead of Tauri)
    - Android
    - iOS
- Technical Stack:
  - Rust
  - GStreamer
  - SQLite
  - Tauri
  
## Definitions

The terms track, recording, work, artist, song and passage have specific definitions found in [Entity Definitions](entity_definitions.md).
  
### Musical Flavor

Each passage is characterized to quantify its musical flavor.  Details of how musical flavor is determined and used are found in [Musical Flavor](musical_flavor.md).

#### Musical Flavor Target by time of day

- A 24 hour schedule defines the Musical Flavor Target for each timeslot during the day/night.
  - Users may adjust timeslots, adding / removing timeslots
    - one or more timeslots must always be defined
    - every time of day must be covered by one and only one defined timeslot
  - Each timeslot definition includes one or more passages defining the musical flavor of that timeslot
- Users may temporarily override the timeslot defined musical flavor by manually selecting a musical flavor
  to use for a coming time window (e.g. 1 or 2 hours)
- Musical Flavor Target impacts selection of songs being enqueued, anticipated play start time of the passage
  to be enqueued based on current queued passages' play time compared to the anticipated musical flavor target
  at that time, either by schedule or user override.
  - Passages in the queue are not impacted by changes to schedule based musical flavor targets
    - Passages are not interrupted by any time based transition of musical flavor targets
  - When a user issues a temporary override for musical flavor target, a new passage is selected based on the new
    target, then once that passage is enqueued and ready to play all previous passages awaiting play are removed
    from the queue and any remaining play time on the currently playing song is skipped so play of the newly enqueue
    passage starts as soon as possible.  The queue is then refilled based on the new user selected musical 
    flavor target.
   
## Non-functional Requirements
- Follow defined [coding conventions](coding_conventions.md).
- Errors logged to developer interface, otherwise gracefully ignored and continue playing as best as able
  - developer interface is stdout/stderr

## Passage Identification & Library Management

### Initial File Discovery
- Full version only, lite and minimal versions use existing database and file collections
- Recursively scan specified directory paths for audio files
- Support formats: MP3, FLAC, OGG, M4A, WAV
- Store file paths, modification times, and file hashes (SHA-256) in SQLite
- Skip hidden files and system directories
- Detect file changes (modified/deleted/added) on subsequent scans

### Metadata Extraction
- Extract from file tags (ID3v2, Vorbis Comments, MP4 tags):
  - Title, Artist, Album, Album Artist
  - Track number, Disc number
  - Year/Date, Genre
  - Duration
  - Embedded cover art (extract and save to file in same directory as audio file)
- Fallback to filename parsing if tags missing
- Store all metadata in SQLite with timestamps
- Default assumes each file contains one passage, unless otherwise identified by user
  - Files must have at least one passage defined to be included for selection to enqueue

### Audio Fingerprinting
- Generate AcoustID fingerprint for each passage using Chromaprint
- Store fingerprints in database for MusicBrainz lookup
- Query AcoustID API for MusicBrainz Recording IDs
- Cache API responses locally (store indefinitely, delete oldest when necessary for storage space constraints)
- Handle rate limiting (max 3 requests/second)

### Multi-passage File Handling

**Requirement:** Each audio file may contain one or more passages.

**Requirement:** Users must be able to:
- Define multiple passages within a single audio file
- Manually edit passage boundaries and timing points
- Add or delete passage definitions
- Associate each passage with MusicBrainz entities (tracks, recordings, artists, works)

**Requirement:** On initial import, the system must assist users by:
- Asking whether a file contains one or multiple passages
- Automatically detecting passage boundaries using silence detection (multi-passage files)
- Providing sensible defaults for single-passage files

> **See [Crossfade Design](crossfade.md#default-configuration) for default timing point values.**

### MusicBrainz Integration
- Store MusicBrainz IDs: Recording ID, Release ID, Artist ID, Work ID
- Fetch and cache basic metadata:
  - Canonical artist name
  - Release title and date
  - Primary genre/tags (limit to the top 10 when more than 10 are defined)
- Offline mode: when local data is available, do not perform network lookups

#### Lyrics input / storage (Full Version only)
- WebUI provides page to input / edit lyrics associated with a passage
  - split window allows web search for lyrics to facilitate easy copy-paste
- Lyrics are stored in passage database table as plain UTF-8 text

## Playback behaviors

### Crossfade Handling

**Requirement:** Passages must crossfade smoothly into each other without gaps or abrupt volume changes. Users must be able to configure crossfade timing for each passage individually or use global defaults.

**Requirement:** When resuming from Pause, audio must fade in smoothly to prevent audible "pops" or jarring transitions.

> **See [Crossfade Design](crossfade.md) for complete crossfade timing system, fade curves, and implementation details.**

### Automatic Passage Selection

**Requirement:** Passages are automatically selected based on:
- Musical flavor distance from current time-of-day target
- Cooldown periods preventing too-frequent replay of songs, artists, and works
- User-configured base probabilities for songs, artists, and works
- A passage must contain one or more songs to be considered for automatic selection

**Requirement:** Cooldown System
- Each song, artist, and work has configurable minimum and ramping cooldown periods
- During minimum cooldown, probability is zero (passage cannot be selected)
- During ramping cooldown, probability increases linearly from zero to base probability
- Cooldown probabilities multiply together (song × artist × work)
- When multiple artists are associated, use the lowest cooldown probability

**Requirement:** Base Probability Editing
- Users may adjust base probabilities for individual songs, artists, and works
- Passage base probability = song probability × artist probability × work probability

> **See [Musical Flavor](musical_flavor.md#usage-of-musical-flavor) for selection algorithm details.**
> **See [Architecture](architecture.md#2-program-director) for default cooldown values and probability calculation.**

**Requirement:** Base Probability User Interface
- Valid range: 0.0 to 1000.0, presented as logarithmic scale slider with option for numeric input
- Default values: All songs, artists, and works start at 1.0

### User Queue additions
- User may select any passage for enqueueing, including those with no songs contained

### Simple Queue Management
- Add passages to queue (append)
- Automatically advance to next passage on completion

### Play History
- Record in SQLite for each play:
  - Passage ID
  - Timestamp (start time)
  - Duration played (for skip detection)
  - Completion status (played fully vs skipped)

#### Album art and Image Management
- Passages are usually associated with an album
- Albums may have multiple images: "front", "back", and optionally "liner" notes
- Album art is stored as image files in the same directory as the audio files
- When extracted from embedded tags, saved as `{filename}.cover.{ext}` (e.g., `song.mp3.cover.jpg`)
- When fetched from external sources (MusicBrainz/Cover Art Archive), saved as:
  - `{album_mbid}.front.{ext}` - Album front cover
  - `{album_mbid}.back.{ext}` - Album back cover
  - `{album_mbid}.liner.{ext}` - Album liner notes (optional)
- Images are resized to maximum 1024x1024 pixels (preserving aspect ratio) when larger than this size
- Database stores file path references to album art images, not the image data itself

**Additional Image Types:**
- **Song-specific images**: User-uploaded only (Full version only) - for special performances, live versions, remixes
- **Passage-specific images**: User-uploaded only (Full version only) - for compilation tracks, medleys, custom edits
- **Artist images**: Fetched from MusicBrainz (artist photos) or user-uploaded (Full version only)
- **Work images**: User-uploaded only (Full version only) - for sheet music covers, opera/ballet production stills

> **See [Database Schema - images](database_schema.md#images) for unified image storage.**

## Player Functionality

### Manual Controls
- **Play**: Start playback of current passage
- **Pause**: Pause playback (maintain position)
- **Skip**: Move to next passage in queue
- **Volume**: Set 0-100% volume level
- **Seek**: Jump to specific position in current passage
- **Like**: Record a like associated with the passage at the time like was indicated by the user  (Full and Lite versions only)
- **Dislike**: Record a dislike associated with the passage at the time dislike was indicated by the user (Full and Lite versions only)
- **Remove**: Remove a passage from the queue
- **Select**: Select a passage to enqueue
- **Import**: Rescan designated music folders for new / changed music files, add them to the local database (Full version only)
- **Output**: Select audio sink.  Default choice is PulseAudio (or the most common sink for the OS/environment), user may override and select other sinks and either let the OS control or manually specify output device.

### Web UI
- WebUI provided on HTTP port 5720 with no authentication

**Status Display:**
- Passage Title (only when different from current song title and album title)
  - When passage contains one song: Display that song's information
  - When passage contains multiple songs: Display the song currently playing based on playback position within the passage
  - When passage contains zero songs: Display passage information only
- Current song:
  - Title
  - Artist(s)
  - Album
  - Play History 
    - Time since last play (of this song, in any passage)
      - Displays human-readable format: "2 hours ago", "3 days ago", "2 weeks ago"
    - Number of plays in the past: (Full and Lite versions only)
      - week
      - month
      - year
      - all time
  - Lyrics (when available) (Full and Lite versions only)

> **See [Database Schema - song_play_counts](database_schema.md#song_play_counts-view) for play count data storage.**

- Playback state: Playing/Paused
- Progress bar: Current position / Total duration
- Artwork: 2 images when available
  - Available image priority list:
    - Song specific image(s)
    - Passage specific image(s)
    - Album Front image
    - Album Rear image
    - Album liner image(s)
    - Artist specific image(s)
    - Work specific image(s)
    - McRhythm Logo image (always available, only displayed when no other artwork is available)
  - Highest priority available image displays to the left
  - Lower priority image(s) appear to the right
    - When more than one lower priority image (not including McRhythm logo) is available, right side displayed image rotates every 15 seconds
  - When only Logo or one image is available for display, that single image is displayed centered with blank space to the left and right.

> **See [Database Schema - images](database_schema.md#images) for image association storage.**

**Control Panel:**
- Play/Pause toggle button
- Skip button
- Volume slider
  - Current master volume percentage display

**Next Up Queue:**
- List next passages in queue
- Show: Passage Title - Primary Artist

**API Endpoints (REST):**
- `GET /api/status` - Current playback state
- `POST /api/play` - Start playback
- `POST /api/pause` - Pause playback
- `POST /api/skip` - Skip to next
- `POST /api/volume` - Set volume (body: `{level: 0-100}`)
- `GET /api/queue` - Get upcoming passages
- `POST /api/like` - like the passage (Full and Lite versions only)
- `POST /api/dislike` - dislike the passage (Full and Lite versions only)
- `POST /api/remove` - remove passage from queue
- `POST /api/enqueue` - enqueue passage
- `POST /api/seek` - skip to playback point in passage
- `POST /api/import` - import new audio files
- `POST /api/output` - audio output device selection
- `GET /api/lyrics/:passage_id` 
- `PUT /api/lyrics/:passage_id` (Full version only)
- `GET /api/events` (SSE endpoint for real-time updates)

### State Persistence
- Save on exit/load on startup:
  - Last played passage and position
  - Volume level
  - Queue contents
- Store in SQLite settings table

### Error Handling
- Log errors to stdout/stderr with timestamps (this is one aspect of the developer interface)
  - Use tracing crate for log output, configure to identify filename and line number of each log message 
- On playback error: Skip to next passage automatically
- On missing file: Remove from queue, continue
- On database error: Attempt retry once, then log and continue

### Network Error Handling
- When any network access fails, wait 5 seconds and retry.
  - Retry up to a maximum of 20 consecutive failures.
  - Notify user on UI of network problems

----
End of document - McRhythm Requirements
