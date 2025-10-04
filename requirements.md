# McRhythm Requirements

> **Related Documentation:** [Architecture](architecture.md) | [Database Schema](database_schema.md) | [Implementation Order](implementation_order.md)

## Overview
McRhythm is a music player that selects passages to play based on user preferences for musical flavor at various times of day.

## Core Features
- Plays passages from local files (.mp3 and similar)
  - Identifies one or multiple passage start / stop points and crossfade points within each music file
- Records when passages are played to avoid repetition
- Cross references passages to the MusicBrainz database for:
  - identification of the song(s) contained in the passage (see Definitions for definition of song in this context)
  - identification of other relationships that may influence selection of passages for enqueueing
- Cross references passages to the AcousticBrainz database when possible, identifying musical character of each passage
  - In absence of AcousticBrainz data will use AcousticBrainz algorithms (Essentia) locally to generate musical character values for the passage (Full version only)
- Interface to ListenBrainz to inform future listening choices based on past likes and dislikes
- Local copies of all relevant subsets of database information enabling offline operation
- Web-based UI
  - Primary mode of operation is automatic, without user intervention
    - Auto-start on boot (of Linux / Windows / OS-X systems)
      - systemd service on Linux
      - Task scheduler launched service on Windows
      - launchd on OS-X
    - Auto-selection of passages to play
      - Automatically select passages until at least 2 passages beyond the currently playing passage and 
        at least 15 minutes of total passage playing time is enqueued
      - Enter "Pause" mode when no passages are available to enqueue, either because the library is empty
        or because all passages in the library are currently in 0 probability cooldown.
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
    
## Three Versions

### Full Version
**Target Platforms:** Linux desktop, Windows, macOS

**Features:**
- Player and Program Director (passage selector)
- All database building and maintenance
- File scanning and library management
- Essentia local analysis for musical flavor
- MusicBrainz/AcousticBrainz/ListenBrainz integration
- Preference editing (timeslots, probabilities)

**Resource Profile:**
- CPU: Higher (Essentia analysis during import)
- Disk I/O: Higher (file scanning)
- Memory: ~512MB typical
- Network: Required for initial setup, optional for ongoing use

### Lite Version
**Target Platforms:** Raspberry Pi Zero2W, Linux desktop, Windows, macOS

**Features:**
- Player and Program Director (passage selector)
- Preference editing (timeslots, probabilities, base probabilities)
- Uses pre-built static database from Full version
- Read-only external data (MusicBrainz/AcousticBrainz cached)
- ListenBrainz integration for feedback sync
- No file scanning, no Essentia

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

**Default Behavior:**
When not otherwise specified, requirements apply to all versions

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
  
## Architecture
See architecture.md

## Definitions
- Track: a specific recording on a particular release.  Has a MBID (MusicBrainz unique identifier).
- Recording: the unique distinct piece of audio underlying a track. Has a MBID.
- Work: one or more recordings can exist of each work. Has a MBID.
- Performing Artist: the artist(s) credited with creating a recording. Has a MBID.
- Song: A combination of a recording and primary performing artist.
  - each song may appear in one or more passages.
- Passage: A span of audio.
  - In McRhythm a passage is a defined part of an audio file with start, fade-in, lead-in, 
    lead-out, fade-out, end points in time defined, as described in Crossfade Handling.
  - A passage may contain zero or more songs.
  
### Musical Flavor
The AcousticBrainz high level characterization of tracks (identified by MBID) provides values of:

- danceability, danceable: 0-1
- gender, female: 0-1
- genre_dortmund, alternative: 0-1
- genre_dortmund, blues: 0-1
- genre_dortmund, electronic: 0-1
- genre_dortmund, folkcountry: 0-1
- genre_dortmund, funksoulrnb: 0-1
- genre_dortmund, jazz: 0-1
- genre_dortmund, pop: 0-1
- genre_dortmund, raphiphop: 0-1
- genre_dortmund, rock: 0-1
- genre_electronic, ambient: 0-1
- genre_electronic, dnb: 0-1
- genre_electronic, house: 0-1
- genre_electronic, techno: 0-1
- genre_electronic, trance: 0-1
- genre_rosamerica, cla: 0-1
- genre_rosamerica, dan: 0-1
- genre_rosamerica, hip: 0-1
- genre_rosamerica, jaz: 0-1
- genre_rosamerica, pop: 0-1
- genre_rosamerica, rhy: 0-1
- genre_rosamerica, roc: 0-1
- genre_rosamerica, spe: 0-1
- genre_tzanetakis, blu: 0-1
- genre_tzanetakis, cla: 0-1
- genre_tzanetakis, cou: 0-1
- genre_tzanetakis, dis: 0-1
- genre_tzanetakis, hip: 0-1
- genre_tzanetakis, jaz: 0-1
- genre_tzanetakis, met: 0-1
- genre_tzanetakis, pop: 0-1
- genre_tzanetakis, reg: 0-1
- genre_tzanetakis, roc: 0-1
- ismir04_rhythm, ChaChaCha: 0-1
- ismir04_rhythm, Jive: 0-1
- ismir04_rhythm, Quickstep: 0-1
- ismir04_rhythm, Rumba-American: 0-1
- ismir04_rhythm, Rumba-International: 0-1
- ismir04_rhythm, Rumba-Misc: 0-1
- ismir04_rhythm, Samba: 0-1
- ismir04_rhythm, Tango: 0-1
- ismir04_rhythm, VienneseWalts: 0-1
- ismir04_rhythm, Waltz: 0-1
- mood_acoustic, acoustic: 0-1
- mood_aggressive, aggressive: 0-1
- mood_electronic, electronic: 0-1 
- and many more...

These values break down into two categories: binary classifications with two dimensions that add up to 1.0, 
and higher dimensional characterizations with more than two dimensions which all add up to 1.0.

The musical flavor of a track is defined by its multi-dimensional position defined by these AcousticBrainz
high level characterization values.  Musical flavor is used by calculating the square of the Euclidian 
distance between two tracks' positions - lower distance means the tracks are more similar, higher 
distance means they are less similar.

Square of distance is calculated first based on all of the binary classifications.

Then a square of distance is calculated for each of the higher dimensional characterizations, and all of
those squares of distances are arithmetically averaged (Σ(diff²)/N) to come up with a single higher 
dimensional distance value.

Finally, a total distance is calculated as the sum of the binary classifications square of distance plus 
the average of all higher dimensional squares of distances.  The absolute number is not important, the
relative value between comparisons is what matters.

- Each song corresponds to a single AcousticBrainz "position" / Musical Flavor.

- A passage may contain zero or more songs
  - When a passage has no songs contained in it, it has no Musical Flavor and cannot be selected for enqueing based on Musical Flavor criteria
  - When a passage has one song, that song's position is used directly as the passage's musical flavor.
  - When a passage has more than one song, each dimensional value of each song is arithmetically averaged 
    to compute the passage's "position" / Musical Flavor.
    - This average position is stored in the passage's database entry rather than re-computing it every time needed.

#### Usage of Musical Flavor
When a user defines a preferred musical flavor, they select one or more passages which contain one or more songs.

The defined musical flavor is the average position of all passages defined by the user.

When passages are being considered for enqueuing:
1. zero probability passages (in song, or artist, or work, or other cooldown criteria) are ignored
2. all passages with non-zero probability of selection have their squared distance from the
   target musical flavor computed
3. candidate passages are sorted by distance from the target, with closest passages considered first.
4. the 100 closest (or all, when less than 100 non-zero probability candidates are available) are then considered
   based on their current computed probability, the product of their base probability and any applicable cooldown
   modifiers and other probability modifiers
5. a random number between zero and the sum of all considered passages' computed probabilities is selected
6. the list of candidates is iterated and each candidate's computed probability is subtracted from the random number
   the first candidate to reduce the random number to zero or below is the chosen passage for enqueuing.
   
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
- Passages should play continuously (when not paused by user)
- Errors logged to developer interface, otherwise gracefully ignored and continue playing as best as able


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

### Audio Fingerprinting
- Generate AcoustID fingerprint for each passage using Chromaprint
- Store fingerprints in database for MusicBrainz lookup
- Query AcoustID API for MusicBrainz Recording IDs
- Cache API responses locally (store indefinitely, delete oldest when necessary for storage space constraints)
- Handle rate limiting (max 3 requests/second)

### Multi-passage file handling
- Each file may contain one or more passages
  - on initial import, user is asked if a file is expected to contain one or multiple passages
    - one passage files default to: start, fade-in, lead-in times at the start of the file and
                                  lead-out, fade-out, end times at the end of the file.
  - when a file is identified as multi-passage, an initial segmentation of the file is attempted based on automatic silence detection
  - users may manually edit start, fade-in, lead-in, lead-out, fade-out and end times for all passages in a file
  - users may add or delete passage definitions associated with a file
  - each passage is associated with zero or more MusicBrainz tracks, recordings, artists, and works

### MusicBrainz Integration
- Store MusicBrainz IDs: Recording ID, Release ID, Artist ID
- Fetch and cache basic metadata:
  - Canonical artist name
  - Release title and date
  - Primary genre/tags (limit to the top 10 when more than 10 are defined)
- Offline mode: when local data is available, do not perform network lookups

#### Lyrics input / storage
- WebUI provides page to input / edit lyrics associated with a passage
  - split window allows web search for lyrics to facilitate easy copy-paste
- Lyrics are stored in passage database table as plain UTF-8 text

## Playback behaviors

### Crossfade Handling
- Passages have a fade-in point identified at some time at or after the passage start time, at or before the fade-out point.
  - The fade-in time is the difference between the start time and fade-in point.
  - The fade-in point may be identical to the start time - resulting in a fade-in time of 0.
  - As default fade-in points are set to the start time.
- Passages have a lead-in point identified at some time at or after the start time, at or before the lead-out point.
  - The lead-in time is the difference between the start time and lead-in point.
  - The lead-in point may be identical to the start time - resulting in a lead-in time of 0.
  - As default lead-in points are set to the start time.
- Passages have a fade-out point identified at some time at or before the passage end time, at or after the fade-in point.
  - The fade-out time is the difference between the fade-out point and the end time.
  - The fade-out point may be identical to the end time - resulting in a fade-out time of 0.
  - As default fade-out points are set to the end time.
- Passages have a lead-out point identified at some time at or before the passage end time, at or after the lead-in point.
  - The lead-out time is the difference between the lead-out point and the end time.
  - The lead-out point may be identical to the end time - resulting in a lead-out time of 0.
  - As default lead-out points are set to the end time.
- When fade-in time is greater than 0, the recorded audio level in the file is faded in, starting at 0 at the passage start time and ending at the full current volume level at the fade-in point
- When fade-out time is greater than 0, the recorded audio level in the file is faded out, starting at the full current volume level at the fade-out point and ending 0 at the end of passage time
- fade-in / fade-out profiles are set per-passage to be one of:
  - Exponential fade in / Logarithmic fade out
  - Cosine (S-Curve) fade in and fade out
  - Linear fade in and fade out
- When a playing passage with a lead-out time greater than 0 is followed by a passage with a longer lead-in time, the following passage begins playing at its start time simultaneously with the playing passage when the playing passage reaches its lead-out point.
- When a playing passage with a lead-out time greater than 0 is followed by a passage with a shorter lead-in time, the following passage begins playing simultaneously at its start time when the playing passage has the same time remaining before its end time as the lead-in time of the following passage.
- fade-in / fade-out behavior is unchanged by simultaneous playing of crossfading passages
- default lead-in, lead-out, fade-in, fade-out times are 0, they may be edited by the user
- lead-in, lead-out, fade-in, fade-out times are stored in the passage's database table

### Fade-in when resuming from Pause
- When resuming play after Pause, the audio level ramps up exponentially across a time of 0.5 seconds
- This level ramping up is applied after any crossfade behavior that may be happening simultaneously

### Automatic passage Selection
- passages are selected to be played based on multiple criteria:
  - When the song(s) associated with the passage were last played
  - When the artist(s) associated with the passage were last played
  - When the work(s) associated with the passage were last played
  - A user configured preference for the flavor of passages to be played at different times of day
  - A passage must contain one or more songs to be considered for automatic selection

- One or more passages are associated with each song
- Each song has an individual frequency profile
  - minimum cooldown time required to pass between last play of the song before it is eligible to be enqueued for playing again.  During this time the song's probability of being enqueued for play is 0.
    - minimum song cooldown time defaults to 7 days
  - ramping cooldown time during which any passage associated with the song is less likely to be selected for playing again - starting at the end of the minimum cooldown time when the song's probability to be selected is 0, ramping up linearly throughout the ramping cooldown time until the song is restored to 100% (1.0) of its base probability to be selected.
    - ramping song cooldown time defaults to 14 days
    
- Each primary performing artist has an individual frequency profile
  - minimum cooldown time required to pass between last play of any passage by the primary performing artist, before any passage with the same primary performing artist is eligible to be enqueued for playing again.
    - minimum primary performing artist cooldown time defaults to 2 hours
  - ramping cooldown time during which the any passage by the primary performing artist is less likely to be selected for playing again - starting at the end of the minimum primary performing artist cooldown time when passages by the artist probability to be selected is 0, ramping up linearly throughout the ramping cooldown time until passages by the artist are restored to 100% (1.0) of their base probability to be selected.
    - ramping primary performing artist cooldown time defaults to 4 hours
    - passage and primary performing artist ramping cooldown times "stack" meaning: the net probability for a passage in ramping cooldown time which is associated with an artist also in ramping cooldown time is the product of the two ramping values (on a 0.0 - 1.0 scale)
    - when a passage is associated with a primary performing artist and one or more featured artists, the artist with the lowest cooldown probability is used for computation of the passage's net probability to be enqueued.

#### Base probabilities
- each song starts with a base probability of selection = 1.0
- each artist starts with a base probability of selection = 1.0
- users may edit song / artist probabilities
  - valid range from 0.0 to 1000.0, presented as logarithmic scale slider with option for numeric input
- passage base probability is the product of the passage's song base probability multiplied by the passage's primary performing artist base probability

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

### Database Schema
See: database_schema.md

#### Album art
- Passages are usually associated with an album
- Albums usually have two images stored as album art "front" and "back"
- Album art is stored as image files in the same directory as the audio files
- When extracted from embedded tags, saved as `{filename}.cover.{ext}` (e.g., `song.mp3.cover.jpg`)
- When fetched from external sources (MusicBrainz/Cover Art Archive), saved as `{album_mbid}.front.{ext}` and `{album_mbid}.back.{ext}` in the audio file directory
- Images are resized to maximum 1024x1024 pixels (preserving aspect ratio) when larger than this size
- Database stores file path references to album art images, not the image data itself

## Player Functionality

### Manual Controls
- **Play**: Start playback of current passage
- **Pause**: Pause playback (maintain position)
- **Skip**: Move to next passage in queue
- **Volume**: Set 0-100% volume level
- **Seek**: Jump to specific position in current passage
- **Like**: Record a like associated with the passage at the time like was indicated by the user
- **Dislike**: Record a dislike associated with the passage at the time dislike was indicated by the user
- **Remove**: Remove a passage from the queue
- **Select**: Select a passage to enqueue
- **Import**: Rescan designated music folders for new / changed music files, add them to the local database (Full version only)
- **Output**: Select audio sink.  Default choice is PulseAudio (or the most common sink for the OS/environment), user may override and select other sinks and either let the OS control or manually specify output device.

### Web UI
- WebUI provided on HTTP port 5720 with no authentication

**Status Display:**
- Current passage: Title, Artist, Album
- Playback state: Playing/Paused/Stopped
- Progress bar: Current position / Total duration
- Album art placeholder (static image if none available)

**Control Panel:**
- Play/Pause toggle button
- Skip button
- Volume slider
- Current volume percentage display

**Next Up Queue:**
- List next 5 passages in queue
- Show: Title - Artist

**API Endpoints (REST):**
- `GET /api/status` - Current playback state
- `POST /api/play` - Start playback
- `POST /api/pause` - Pause playback
- `POST /api/skip` - Skip to next
- `POST /api/volume` - Set volume (body: `{level: 0-100}`)
- `GET /api/queue` - Get upcoming passages
- `POST /api/like` - like the passage
- `POST /api/dislike` - dislike the passage
- `POST /api/remove` - remove passage from queue
- `POST /api/enqueue` - enqueue passage
- `POST /api/seek` - skip to playback point in passage
- `POST /api/import` - import new audio files
- `POST /api/output` - audio output device selection
- `GET /api/lyrics/:passage_id`
- `PUT /api/lyrics/:passage_id`
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

