# McRhythm Requirements

**📜 TIER 1 - AUTHORITATIVE SOURCE DOCUMENT**

This document is the **top-level specification** defining WHAT McRhythm must do. Other documents are designed to satisfy these requirements.

**Update Policy:** ✅ Product decisions only | ❌ NOT derived from design/implementation

> See [Document Hierarchy](document_hierarchy.md) for complete update policies and change control process.

> **Related Documentation:** [Architecture](architecture.md) | [API Design](api_design.md) | [UI Specification](ui_specification.md) | [Library Management](library_management.md) | [Crossfade Design](crossfade.md) | [Musical Flavor](musical_flavor.md) | [Program Director](program_director.md) | [Event System](event_system.md) | [Requirements Enumeration](requirements_enumeration.md)

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
  - **Note:** AcousticBrainz project is discontinued but the website and API remain available with static data
  - Cached AcousticBrainz data never expires (no new data being added to AcousticBrainz)
  - In absence of AcousticBrainz data, uses AcousticBrainz algorithms (Essentia) locally to generate musical flavor values for the passage (Full version only)

**AcousticBrainz Status (as of 2024):**
- The AcousticBrainz project ceased accepting new submissions in 2022
- The existing database and API remain online in read-only mode
- Contains musical analysis data for millions of recordings submitted before discontinuation
- McRhythm uses AcousticBrainz data when available, falls back to local Essentia analysis when not

> **See [Library Management - AcousticBrainz Integration](library_management.md#acousticbrainz-integration) for cache expiration policy and fallback behavior.**
- Local copies of all relevant subsets of database information enabling offline operation
- Web-based UI
  - Primary mode of operation is automatic, without user intervention
    - Auto-start on boot
      - systemd service on Linux (Phase 1)
      - Task scheduler launched service on Windows (Phase 1)
      - launchd on macOS (Phase 2)
    - Auto-selection of passages to play
      - Automatically select passages until at least three passages are either currently playing, paused
        or waiting in the queue and  at least 15 minutes of total passage playing time is remaining 
        in the queue.
      - Stop automatic enqueueing when no passages with associated songs are available to enqueue,
        because the library lacks passages with songs which are not currently in 0 probability cooldown.
      - When automatic enqueueing is unable to add passages to the queue, the queue may eventually
        become empty as passages finish playing.
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
  - Each user has a persistent identity (UUID) established through authentication or anonymous access
  - Real-time UI updates via Server Sent Events keep all users' views in sync
  - Single shared passage queue for all users
  - User-specific data (likes, dislikes, taste profiles) tracked per user UUID
  - Concurrent user actions handled via specific strategies: skip throttling, idempotent queue removal, "last write wins" for lyrics

> **See [Multi-User Coordination](multi_user_coordination.md) for complete specifications on handling concurrent user actions.**
- Passages play continuously (when not paused by user)
    
## Additional Features
Planned for later development:  (Phase 2)
- Interface to ListenBrainz to inform future listening choices based on past likes and dislikes
- Mobile (Android, iOS) versions

## Three Versions

**Default Behavior:**
When not otherwise specified, requirements apply to all versions

### Full Version
**Target Platforms:** Linux desktop, Windows, macOS (Phase 2)

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
- Network: Two distinct network access types:
  - **Internet access**: Required for initial library setup (MusicBrainz, AcousticBrainz, etc.), optional for ongoing playback
  - **Local network access**: Required for remote WebUI access, not required for localhost or automatic playback

### Lite Version
**Target Platforms:** Raspberry Pi Zero2W, Linux desktop, Windows, macOS (Phase 2)

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
- Network:
  - Internet: Optional (Phase 2: ListenBrainz sync only)
  - Local network: Required for remote WebUI access, not required for localhost

### Minimal Version (Phase 2)
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
- Network:
  - Internet: None required
  - Local network: Required for remote WebUI access, not required for localhost or automatic playback

### Build Strategy

See [Implementation Order - Version Builds](implementation_order.md#27-version-builds-fulliteminimal) for Rust feature flags and conditional compilation approach.

**Database Deployment:**
- Full version exports complete database snapshot
- Lite/Minimal versions import read-only database
- Migration tools for version upgrades and cross-platform deployment

## Technical Requirements
- Target platforms:
  - Primary target: Raspberry Pi Zero2W (Lite and Minimal versions)
  - Generic Linux (Phase 1)
  - Windows (Phase 1)
  - macOS (Phase 2)
  - Later targets (will use different technical stack, e.g. Flutter instead of Tauri)
    - Android (Lite and Minimal versions) (Phase 2)
    - iOS (Lite and Minimal versions) (Phase 2)
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

**Requirement:** Full version scans local audio files, extracts metadata, and associates passages with MusicBrainz entities. Lite and minimal versions use pre-built databases.

**Requirement:** Support audio formats: MP3, FLAC, OGG, M4A, WAV

**Requirement:** Extract metadata from file tags and generate audio fingerprints for MusicBrainz lookup

**Requirement:** Each audio file may contain one or more passages

**Requirement:** Users must be able to:
- Define multiple passages within a single audio file
- Manually edit passage boundaries and timing points
- Add or delete passage definitions
- Associate each passage with MusicBrainz entities (tracks, recordings, artists, works)

**Requirement:** On initial import, the system must assist users by offering automatic passage boundary detection. The detailed workflow for this is specified in [Audio File Segmentation](audio_file_segmentation.md).

**Requirement:** Store MusicBrainz IDs and fetch basic metadata (artist names, release titles, genre tags)

**Requirement:** WebUI provides interface to input/edit lyrics associated with a passage (Full version only)
- Concurrent lyric editing uses "last write wins" strategy (no conflict resolution)
- Multiple users may edit lyrics simultaneously; last submitted text persists

> **See [Multi-User Coordination - Concurrent Lyric Editing](multi_user_coordination.md#3-concurrent-lyric-editing) for concurrent editing behavior.**

> **See [Library Management](library_management.md) for file scanning workflows, metadata extraction details, and MusicBrainz integration process.**
> **See [Crossfade Design](crossfade.md#default-configuration) for default passage timing point values.**

### Library Edge Cases

**Requirement:** Zero-song passage library handling
- When library contains only passages with zero songs:
  - Automatic passage selection cannot operate (no valid candidates)
  - Users may still manually enqueue passages
  - Queue may become empty when all manually enqueued passages finish
  - System remains in user-selected Play/Pause state when queue is empty
  - No automatic state changes occur

**Requirement:** Empty library handling
- When library contains no passages at all:
  - Automatic passage selection cannot operate
  - Manual enqueueing is not possible (no passages to select)
  - Queue is empty
  - System remains in user-selected Play/Pause state
  - UI should indicate library is empty and prompt user to import music (Full version) or load database (Lite/Minimal versions)

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
- Passages with zero songs can only be manually enqueued by users
- If library contains only zero-song passages, automatic selection cannot operate (manual enqueueing still works)

> **See [Program Director](program_director.md) for complete selection algorithm specification.**

**Requirement:** Cooldown System
- Each song, artist, and work has configurable minimum and ramping cooldown periods
- During minimum cooldown, probability is zero (passage cannot be selected)
- During ramping cooldown, probability increases linearly from zero to base probability

> **Technical Specification**: See [Program Director - Cooldown System](program_director.md#cooldown-system) for complete algorithm details, multiplier calculations, and multi-entity handling.

**Default Cooldown Periods:**
- **Song:** 7 days minimum, 14 days ramping
- **Artist:** 2 hours minimum, 4 hours ramping
- **Work:** 3 days minimum, 7 days ramping

**Requirement:** Base Probability Editing
- Users may adjust base probabilities for individual songs, artists, and works
- Passage base probability is calculated as: `song probability × artist probability × work probability`
  - For passages with a single song, the song probability is used directly.
  - For passages with multiple songs, the song probability is a weighted average of the individual song probabilities, based on the duration of each song within the passage. This is consistent with the musical flavor calculation described in [ENT-CONST-020](entity_definitions.md#ent-const-020-passage-with-multiple-songs).
  - When a song has multiple artists, each artist is assigned a weight, with the sum of all artist weights for that song equaling 1.0. The `artist probability` for the song is the sum of each associated artist's base probability multiplied by their respective weight.

> **See [Program Director](program_director.md) for selection algorithm, cooldown system, and probability calculation details.**

**Requirement:** Base Probability User Interface
- Valid range: 0.0 to 1000.0, presented as logarithmic scale slider with option for numeric input
- Default values: All songs, artists, and works start at 1.0

### User Queue additions
- User may select any passage for enqueueing, including those with no songs contained

### Simple Queue Management
- Add passages to queue (append)
- Automatically advance to next passage on completion

### Queue Empty Behavior
<a name="queue-empty-behavior"></a>

**Requirement:** When the queue becomes empty (no passages waiting to play):
- Audio playback stops naturally (no audio output)
- Play/Pause mode does NOT change automatically
- System remains in whatever Play/Pause state the user last set
- User maintains full control of Play/Pause mode via API regardless of queue state

**Requirement:** When a passage is enqueued while queue is empty:
- If system is in Play mode: Begin playing the newly enqueued passage immediately
- If system is in Pause mode: Passage enters queue but does not play until user selects Play mode

**Requirement:** Automatic selection only enqueues passages containing one or more songs
- Passages with zero songs cannot be automatically selected
- If library contains only passages with zero songs, automatic selection cannot operate
- Users may manually enqueue any passage (including zero-song passages) at any time
- Manual enqueueing works regardless of whether automatic selection can operate

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
- Images are resized to maximum 1024x1024 pixels (preserving aspect ratio) when larger than this size
- Database stores file path references to album art images, not the image data itself

**Additional Image Types:**
- **Song-specific images**: User-uploaded only (Full version only) - for special performances, live versions, remixes
- **Passage-specific images**: User-uploaded only (Full version only) - for compilation tracks, medleys, custom edits
- **Artist images**: Fetched from MusicBrainz (artist photos) or user-uploaded (Full version only)
- **Work images**: User-uploaded only (Full version only) - for sheet music covers, opera/ballet production stills

> **See [Database Schema - images](database_schema.md#images) for image storage details.**
> **See [Library Management - Cover Art](library_management.md#cover-art-extraction) for image file naming and extraction workflow.**

## Player Functionality

### Manual Controls
- **Play**: Start playback of current passage
- **Pause**: Pause playback (maintain position)
- **Skip**: Move to next passage in queue
- **Volume**: Set 0-100% volume level
- **Seek**: Jump to specific position in current passage
- **Like**: Record a like associated with the passage at the time like was indicated by the user (Full and Lite versions only)
  - Like is recorded against the user's UUID
  - Used to build user-specific taste profile
- **Dislike**: Record a dislike associated with the passage at the time dislike was indicated by the user (Full and Lite versions only)
  - Dislike is recorded against the user's UUID
  - Used to refine user-specific taste profile
- **Remove**: Remove a passage from the queue
- **Select**: Select a passage to enqueue
- **Import**: Rescan designated music folders for new / changed music files, add them to the local database (Full version only)
- **Output**: Select audio sink.  Default choice is PulseAudio (or the most common sink for the OS/environment), user may override and select other sinks and either let the OS control or manually specify output device.

### Network Status Indicators

**Requirement:** Internet connection status visibility (Full version only)
- Display internet connection status in library management / import UI
- Status states:
  - **Connected**: Internet accessible, all features available
  - **Retrying (N/20)**: Connection attempt N of 20 in progress
  - **Connection Failed**: 20 retries exhausted, user action required
- Status indicator should be small but clearly visible
- Does not obstruct primary UI elements

**Requirement:** Connection retry controls
- "Retry Connection" button visible when status is "Connection Failed"
- Clicking retry button resets counter and attempts 20 new connection attempts
- Any UI control requiring internet automatically triggers reconnection attempt

**Requirement:** Feature availability feedback
- When user attempts internet-dependent action while disconnected:
  - Display clear notification explaining internet requirement
  - Offer "Retry Connection" option
  - List which features are unavailable without internet:
    - Import new music files
    - Fetch MusicBrainz metadata
    - Retrieve AcousticBrainz flavor data
    - Download cover art
    - (Phase 2) ListenBrainz synchronization

**Note:** Lite and Minimal versions do not require internet access and do not display internet status indicators.

### Playback State

> **Technical Specification**: See [Event System - PlaybackState Enum](event_system.md#playbackstate-enum) for complete technical definition and event handling details.

**Requirement:** System has exactly two playback states:
- **Playing**: Audio plays when passages are available in queue
  - If queue has passages: Plays audio
  - If queue is empty: Silent, but plays immediately when passage enqueued
- **Paused**: User has paused playback
  - Audio paused at current position (if passage is playing)
  - Newly enqueued passages wait until user selects Play

**Requirement:** No "stopped" state
- Traditional media player "stopped" state does not exist
- System is always either Playing or Paused

**Requirement:** Initial state on app launch
- System always starts in Playing state
- If queue has passages, playback begins immediately
- If queue is empty, system waits in Playing state (ready to play when passage enqueued)

**Requirement:** State persistence
- Playback state is NOT persisted across app restarts
- Always resumes to Playing state on launch
- Queue contents and position ARE persisted (see State Persistence section)

### Authentication and User Identity
<a name="user-authentication"></a>

> **Technical Specification**: See [User Identity and Authentication](user_identity.md#authentication-modes) for complete design details, session management, and security considerations.

**Requirement:** System supports multiple concurrent users with persistent identities

**Requirement:** Three authentication modes:
- **Anonymous Mode**: Users access shared "Anonymous" account (username: "Anonymous", no password)
  - Suitable for casual use or public installations
  - All anonymous users share the same UUID
  - Anonymous user data (likes/dislikes) is shared across all anonymous sessions

- **Account Creation**: Users create personal account with unique username and password
  - Username must be unique, 1-63 UTF-8 characters, no invisible characters
  - Password must be 1-63 UTF-8 characters, no invisible characters
  - System generates unique UUID for the user
  - Password stored as salted hash (salt + UUID + password)

- **Account Login**: Users authenticate with existing username and password
  - Successful login provides user UUID to client

**Requirement:** Client-side session persistence
- Browser stores user UUID in localStorage
- Session expires after one year of inactivity
- Expiration resets to one year on each successful connection
- User automatically recognized on subsequent visits without re-authentication

**Requirement:** Concurrent sessions allowed
- Single user UUID may be authenticated from multiple browsers/devices simultaneously
- All sessions for all users receive same real-time event stream

> **See [User Identity and Authentication](user_identity.md) for complete authentication flow, password hashing, and account management specifications.**

### Web UI
- WebUI provided on HTTP port 5720
- Authentication system allows users to:
  - Proceed as the shared "Anonymous" user (no password required)
  - Create a personal account with username and password
  - Login to an existing account
- Authenticated browser sessions persist user UUID for up to one year
- Session automatically renews on each connection

**Status Display:**
- Passage Title (only when different from current song title and album title)
  - User-defined passage title (user_title) takes precedence over tag-based title when set
  - The "only when different" logic applies to whichever title is being used (user_title or tag title)
  - Passage title is shown in addition to song title and album title, not as a replacement
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
  - **Current song determination**: Artwork based on currently playing song within passage
    - When playback position is within a song: Use that song's images
    - When playback position is in a gap: Use nearest song (before or after) to determine images
  - **Image priority** (for currently playing song):
    - Song specific → Passage specific → Album Front → Album Rear → Album Liner → Artist → Work → Logo
  - **Display layout**:
    - Highest priority image on left
    - Lower priority images on right, rotating every 15 seconds when multiple available
  - **Multi-album songs**: Display all associated albums' art in rotation (15-second intervals)
  - **Single image**: Centered with blank space left/right when only one image available

> **See [UI Specification - Album Artwork Display](ui_specification.md#album-artwork-display) for complete artwork selection logic, rotation behavior, and gap handling.**

> **See [Database Schema - images](database_schema.md#images) for image association storage.**

**Control Panel:**
- Play/Pause toggle button
- Skip button
- Volume slider
  - Current master volume percentage display

**Next Up Queue:**
- List next passages in queue
- Show: Passage Title - Primary Artist

**API Endpoints:**
- REST API provides endpoints for all manual controls and status queries
- Server-Sent Events (SSE) endpoint for real-time UI updates across all connected clients

> **See [API Design](api_design.md) for complete endpoint specifications, request/response formats, and error handling.**

### State Persistence
- Save on exit/load on startup:
  - Last played passage and position
  - Volume level
  - Queue contents
- Store in SQLite settings table
- Playback state always resumes to "Playing" on app launch (not persisted)

### Error Handling
- Log errors to stdout/stderr with timestamps (this is one aspect of the developer interface)
  - Log messages must include filename and line number for developer troubleshooting
- On playback error: Skip to next passage automatically
- On missing file: Remove from queue, continue
- On database error: Attempt retry once, then log and continue

### Network Error Handling

McRhythm requires two distinct types of network access with different error handling:

#### Internet Access (External APIs)

**Used for:**
- MusicBrainz metadata lookup during library import
- AcousticBrainz musical flavor data retrieval
- Cover art fetching
- Future ListenBrainz integration (Phase 2)

**Error Handling:**
- When any internet access fails, wait 5 seconds and retry
- Retry up to a maximum of 20 consecutive failures
- After 20 failures, stop attempting until user triggers reconnection
- Reconnection triggers:
  - User clicks any UI control that requires internet (Import, metadata refresh, etc.)
  - User explicitly clicks "Retry Connection" button
  - Counter resets to 20 attempts on each user-triggered reconnection

**User Interface Requirements:**
- **Status indicator**: Small, clear indicator showing internet connection status
  - States: "Connected", "Retrying (N/20)", "Connection Failed - Retry"
  - Visible in library management / import UI (Full version only)
- **Control feedback**: When user attempts internet-dependent action while disconnected:
  - Show notification: "This feature requires internet connection. Please check your connection and retry."
  - Provide "Retry Connection" button to restart connection attempts
- **Degraded functionality**: System continues operating with cached/local data when internet unavailable

**Playback Impact:**
- **No impact on playback**: Music continues playing during internet outages
- Playback uses only local database and audio files (no internet required)

#### Local Network Access (WebUI Server)

**Used for:**
- Serving WebUI on `http://localhost:5720`
- Server-Sent Events (SSE) for real-time UI updates
- REST API endpoints for playback control

**Error Handling:**
- HTTP server binds to localhost:5720 on startup
- If port binding fails: Log error and exit (critical failure)
- Once running, server continues indefinitely

**Access Requirements:**
- **Localhost access**: Always available (no network required)
- **Remote access**: Requires local network connectivity
  - User responsible for network configuration (router, firewall, etc.)
  - No internet required (local network only)

**Playback Impact:**
- **Automatic playback**: Works without any network access
  - System auto-starts on boot
  - Auto-selects and plays passages
  - No WebUI access needed for basic operation
- **Manual control**: Requires WebUI access (localhost or remote)

### Offline Operation

**Requirement:** System operates fully without internet access

**Capabilities without internet:**
- Play all passages in local library
- Automatic passage selection and enqueueing
- Crossfade between passages
- WebUI access via localhost
- Like/Dislike functionality (Full and Lite versions)
- Queue management
- All playback controls

**Limitations without internet:**
- Cannot import new music files (Full version)
- Cannot fetch new MusicBrainz metadata (Full version)
- Cannot retrieve AcousticBrainz flavor data (Full version)
  - Falls back to local Essentia analysis (Full version only)
- Cannot download cover art (Full version)
- Cannot sync with ListenBrainz (Phase 2)

**Internet reconnection:**
- System continues using cached data during outages
- User can trigger reconnection via UI controls
- Automatic retry (20 attempts) when user requests internet-dependent features

----
End of document - McRhythm Requirements
