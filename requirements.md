# McRhythm Requirements

## Overview
McRhythm is a music player that selects tracks to play based on user preferences for style at various times of day.

## Core Features
- Plays tracks from local files (.mp3 and similar)
  - Identifies one or multiple track start / stop points and crossfade points within each music file
- Records when tracks are played to avoid repetition
- Cross references tracks to the MusicBrainz database for:
  - identification of the song(s) contained in the track (see Definitions for definition of song in this context)
  - identification of other relationships that may influence selection of tracks for enqueueing
- Cross references tracks to the AcousticBrainz database when possible, identifying musical character of each track
  - In absence of AcousticBrainz data will use AcousticBrainz algorithms locally to generate musical character values for the track
- Interface to ListenBrainz to inform future listening choices based on past likes and dislikes
- Local copies of all relevant subsets of database information enabling offline operation
- Web-based UI
  - Primary mode of operation is automatic, without user intervention
    - Auto-start on boot (of Linux / Windows / OS-X systems)
      - systemd service on Linux
      - Task scheduler launched service on Windows
      - launchd on OS-X
    - Auto-selection of tracks to play
      - Automatically select tracks until at least 2 tracks beyond the currently playing track and 
        at least 15 minutes of total track playing time is enqueued
      - Enter "Pause" mode when no tracks are available to enqueue, either because the library is empty
        or because all tracks in the library are currently in 0 probability cooldown.
  - Shows track currently playing
    - Shows associated album art or other still image(s) associated with the track when available
    - Shows track lyrics when available
  - Shows tracks queued for playing next
  - Manual user controls allow the user to control and configure the system when they want to
  - Access manual controls from phone and desktop browsers
- Audio output to OS standard output channels
  - Analog
  - HDMI
  - Bluetooth
- Multiple users may interact with the WebUI
  - Real-time ui updates via Server Sent Events keep all users views in sync
  - Single track queue for all users
  - Edge case definitions:
    - When one skip click is received, any other skip clicks received within the next 5 seconds are ignored
    - Queue operations are limited to: add and remove track
      - whenever an add track command is received, that track is added to the end of the queue
      - if two or more remove track commands are received for one track, the later commands are ignored
    - Lyric editing includes a "Submit" button, whenever a new lyric text is submitted it is recorded - overwriting previously submitted lyric texts.
    
## Technical Requirements
- Target platforms:
  - Primary target: Raspberry Pi Zero2W
  - Generic Linux
  - Windows
  - MacOS
  - Later targets (will use different Flutter instead of Tauri)
    - Android
    - iOS
- Technical Stack:
  - Rust
  - GStreamer
  - SQLite
  - Tauri
  
## Architecture
- TBD

## Non-functional Requirements
- Tracks should play continuously (when not paused by user)
- Cross fade between tracks
- Errors logged to developer interface, otherwise gracefully ignored and continue playing as best as able

## Definitions
- Track: a specific recording on a particular release.  Has a MBID (MusicBrainz unique identifier).
- Recording: the unique distinct piece of audio underlying a track. Has a MBID.
- Work: one or more recordings can exist of each work. Has a MBID.
- Performing Artist: the artist(s) credited with creating a recording. Has a MBID.
- Song: A combination of a recording and primary performing artist.
  Each track is identifiable as a song by the MBIDs of its associated recording and primary performing artist.

## Track Identification & Library Management

### Initial File Discovery
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
  - Embedded cover art (extract and store as blob)
- Fallback to filename parsing if tags missing
- Store all metadata in SQLite with timestamps

### Audio Fingerprinting
- Generate AcoustID fingerprint for each track using Chromaprint
- Store fingerprints in database for MusicBrainz lookup
- Query AcoustID API for MusicBrainz Recording IDs
- Cache API responses locally (store indefinitely, delete oldest when necessary for storage space constraints)
- Handle rate limiting (max 3 requests/second)

### Multi-track file handling
- Each file may contain one or more tracks
  - on initial import, user is asked if a file contains one or multiple tracks
    - one track files default to: start, fade-in, lead-in times at the start of the file and
                                  lead-out, fade-out, end times at the end of the file.
  - when a file is identified as multi-track, an initial segmentation of the file is attempted based on automatic silence detection
  - users may manually edit start, fade-in, lead-in, lead-out, fade-out and end times for all tracks in a file
  - users may add or delete track definitions from a file

### Crossfade handling
- Tracks may have a fade-in point identified at some time after the track start time, or the fade-in point may be identical to the start time - resulting in a fade-in time of 0.
- Tracks may have a lead-in point identified at some time at or after the fade-in point.  If the fade-in time is greater than 0, the earliest possible lead-in point is the fade-in point.  The lead-in time is the difference between the lead-in point and the start time.
- Tracks may have a fade-out point identified at some time before the track end time, or the fade-out point may be identical to the end time - resulting in a fade-out time of 0.
- Tracks may have a lead-out point identified at some time at or before the fade-out point.  If the fade-out time is greater than 0, the latest possible lead-out point is the fade-out point.  The lead-out time is the difference between the end time and the lead-out point.
- When fade-in time is greater than 0, the recorded audio level in the file is faded in, starting at 0 at the track start time and ending at the full current volume level at the fade-in point
- When fade-out time is greater than 0, the recorded audio level in the file is faded out, starting at the full current volume level at the fade-out point and ending 0 at the end of track time
- fade-in / fade-out profiles are set globally to be one of:
  - Exponential fade in / Logarithmic fade out
  - Cosine (S-Curve) fade in and fade out
  - Linear fade in and fade out
- When a playing track with a lead-out time greater than 0 is followed by a track with a longer lead-in time, the following track begins playing at its start time simultaneously with the playing track when the playing track reaches its lead-out point.
- When a playing track with a lead-out time greater than 0 is followed by a track with a shorter lead-in time, the following track begins playing simultaneously at its start time when the playing track has the same time remaining before its end time as the lead-in time of the following track.
- fade-in / fade-out behavior is unchanged by simultaneous playing of crossfading tracks
- default lead-in, lead-out, fade-in, fade-out times are 0, they may be edited by the user
- lead-in, lead-out, fade-in, fade-out times are stored in the track's database table

### MusicBrainz Integration (Phase 1)
- Store MusicBrainz IDs: Recording ID, Release ID, Artist ID
- Fetch and cache basic metadata:
  - Canonical artist name
  - Release title and date
  - Primary genre/tags (top 3)
- Offline mode: continue without external lookups if network unavailable

## Minimal Player Functionality

### Core Playback Engine
- Initialize GStreamer pipeline with:
  - File source
  - Decoder (auto-detect format)
  - Volume control element
  - Audio sink (auto-select: ALSA/PulseAudio/CoreAudio/WASAPI)
- Play single track from file path
- Report playback position every 500ms
- Handle state transitions: Stopped → Playing → Paused
  - Resume from pause with a 0.5 second fade-in (pause-resume fade-in system is independent of track crossfade
    system, the pause fade-in is applied on top of any track fade-in / fade-out / crossfade)

### Manual Controls
- **Play**: Start playback of current track
- **Pause**: Pause playback (maintain position)
- **Skip**: Move to next track in queue
- **Volume**: Set 0-100% volume level
- **Seek**: Jump to specific position in current track
- **Like**: Record a like associated with the track at the time like was indicated by the user
- **Dislike**: Record a dislike associated with the track at the time dislike was indicated by the user
- **Remove**: Remove a track from the queue
- **Select**: Select a track to enqueue
- **Import**: Rescan designated music folders for new / changed music files, add them to the local database
- **Output**: Select audio sink.  Default choice is PulseAudio (or the most common sink for the OS/environment), user may override and select other sinks and either let the OS control or manually specify output device.

- WebUI provided on HTTP port 5720 with no authentication

#### Lyrics input / storage
- WebUI provides page to input / edit lyrics associated with a track
  - split window allows web search for lyrics to facilitate easy copy-paste
- Lyrics are stored in track database table as plain UTF-8 text

### Automatic track Selection
- tracks are selected to be played based on multiple criteria:
  - When the track was last played
  - When the artist(s) associated with the track were last played
  - A user configured preference for the kind of tracks to be played at different times of day

- One or more tracks are associated with each song
- Each song has an individual frequency profile
  - minimum cooldown time required to pass between last play of the song before it is eligible to be enqueued for playing again.  During this time the song's probability of being enqueued for play is 0.
    - minimum song cooldown time defaults to 7 days
  - ramping cooldown time during which any track associated with the song is less likely to be selected for playing again - starting at the end of the minimum cooldown time when the song's probability to be selected is 0, ramping up linearly throughout the ramping cooldown time until the song is restored to 100% (1.0) of its base probability to be selected.
    - ramping song cooldown time defaults to 14 days
    
- Each primary performing artist has an individual frequency profile
  - minimum cooldown time required to pass between last play of any track by the primary performing artist, before any track with the same primary performing artist is eligible to be enqueued for playing again.
    - minimum primary performing artist cooldown time defaults to 2 hours
  - ramping cooldown time during which the any track by the primary performing artist is less likely to be selected for playing again - starting at the end of the minimum primary performing artist cooldown time when tracks by the artist probability to be selected is 0, ramping up linearly throughout the ramping cooldown time until tracks by the artist are restored to 100% (1.0) of their base probability to be selected.
    - ramping primary performing artist cooldown time defaults to 4 hours
    - track and primary performing artist ramping cooldown times "stack" meaning: the net probability for a track in ramping cooldown time which is associated with an artist also in ramping cooldown time is the product of the two ramping values (on a 0.0 - 1.0 scale)
    - when a track is associated with a primary performing artist and one or more featured artists, the artist with the lowest cooldown probability is used for computation of the track's net probability to be enqueued.

#### Base probabilities
- each song starts with a base probability of selection = 1.0
- each artist starts with a base probability of selection = 1.0
- users may edit song / artist probabilities
  - valid range from 0.0 to 1000.0, presented as logarithmic scale slider with option for numeric input
- track base probability is the product of the track's song base probability multiplied by the track's primary performing artist base probability

### Simple Queue Management
- Add tracks to queue (append)
- Automatically advance to next track on completion

### Play History Tracking
- Record in SQLite for each play:
  - Track ID
  - Timestamp (start time)
  - Duration played (for skip detection)
  - Completion status (played fully vs skipped)

### Web UI
**Status Display:**
- Current track: Title, Artist, Album
- Playback state: Playing/Paused/Stopped
- Progress bar: Current position / Total duration
- Album art placeholder (static image if none available)

**Control Panel:**
- Play/Pause toggle button
- Skip button
- Volume slider
- Current volume percentage display

**Next Up Queue:**
- List next 5 tracks in queue
- Show: Title - Artist

**API Endpoints (REST):**
- `GET /api/status` - Current playback state
- `POST /api/play` - Start playback
- `POST /api/pause` - Pause playback
- `POST /api/skip` - Skip to next
- `POST /api/volume` - Set volume (body: `{level: 0-100}`)
- `GET /api/queue` - Get upcoming tracks
- `POST /api/like` - like the track
- `POST /api/dislike` - dislike the track
- `POST /api/remove` - remove track from queue
- `POST /api/enqueue` - enqueue track
- `POST /api/seek` - skip to playback point in track
- `POST /api/import` - import new audio files
- `POST /api/output` - audio output device selection
- `GET /api/lyrics/:track_id`
- `PUT /api/lyrics/:track_id`
- `GET /api/events` (SSE endpoint for real-time updates)

### State Persistence
- Save on exit/load on startup:
  - Last played track and position
  - Volume level
  - Queue contents
- Store in SQLite settings table

### Error Handling
- Log errors to stdout/stderr with timestamps (this is one aspect of the developer interface)
  - Use tracing crate for log output, configure to identify filename and line number of each log message 
- On playback error: Skip to next track automatically
- On missing file: Remove from queue, continue
- On database error: Attempt retry once, then log and continue

### Network Error Handling
- When any network access fails, wait 5 seconds and retry.
  - Retry up to a maximum of 20 consecutive failures.
  - Notify user on UI of network problems

### Database Schema
See: database_schema.md

#### Album art
- Tracks are usually associated with an album
- Albums usually have two images stored as album art "front" and "back"
- Album art is stored locally as medium resolution (maximum 1024 pixels width and maximum 1024 pixels height) images, scaled down preserving aspect ratio when the source is larger, otherwise stored in original resolution

## Implementation Order
1. File scanner + metadata extraction
2. Basic playback engine (single file)
3. Database schema + storage
4. Queue management + auto-advance
5. Web UI + REST API
6. Audio fingerprinting + MusicBrainz lookup

