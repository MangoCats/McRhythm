# WKMP User Interface Specification

**🎨 TIER 2 - DESIGN SPECIFICATION**

Defines the Web UI design, layout, and behavior. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Requirements](requirements.md) | [API Design](api_design.md) | [Event System](event_system.md) | [User Identity](user_identity.md) | [Multi-User Coordination](multi_user_coordination.md)

---

## Overview

WKMP's user interface is a web-based UI accessible via desktop and mobile browsers. The primary design philosophy is **automatic operation with manual control available when desired**. The UI provides real-time status updates, playback control, and library management capabilities.

**Key Principles:**
- **Automatic-first**: System operates without user intervention by default
- **Multi-user**: Multiple users can view and control simultaneously via Server-Sent Events
- **Real-time**: UI updates instantly across all connected clients
- **Responsive**: Works on desktop and mobile browsers
- **Minimal network**: Works via localhost (no internet required for playback)

**Access Methods:**
- **Localhost**: `http://localhost:5720` (no network required)
- **Local network**: `http://<machine-ip>:5720` (LAN access)
- **Remote**: User-configured port forwarding (not recommended)

---

## Authentication Flow

### Initial Connection

**[UI-AUTH-010]** When a user connects without a stored UUID (first visit or after one-year expiration), display an authentication screen with three options:

1. **Proceed Anonymously**
   - Button: "Continue as Guest"
   - Subtext: "Quick access, shared preferences with other guests"
   - No username/password required

2. **Create a New Account**
   - Button: "Create Account"
   - Form fields: Username, Password
   - Validation messages inline

3. **Login to Existing Account**
   - Button: "Login"
   - Form fields: Username, Password
   - "Forgot password?" text with explanation to use account maintenance tool

**[UI-AUTH-020]** Once authenticated, store UUID in browser localStorage with one-year rolling expiration.

**[UI-AUTH-030]** Subsequent visits automatically use stored UUID without showing authentication screen.

**[UI-AUTH-040]** Display current user information in UI:
- Small user indicator (e.g., "Logged in as: username" or "Guest")
- Logout button (clears localStorage UUID)
- Account settings button (registered users only)

> **See:** [User Identity and Authentication](user_identity.md) for complete authentication specification

---

## Main Playback View

### Layout Structure

**[UI-LAYOUT-010]** The main view is divided into three primary sections:

```
┌─────────────────────────────────────────────────────┐
│              Top Bar (User, Status)                 │
├─────────────────────────────────────────────────────┤
│                                                     │
│              Album Artwork (2 images)               │
│                                                     │
├─────────────────────────────────────────────────────┤
│         Now Playing Information                     │
│         - Passage Title                             │
│         - Song Title                                │
│         - Artist(s)                                 │
│         - Album                                     │
│         - Play History                              │
├─────────────────────────────────────────────────────┤
│         Progress Bar                                │
│         [========>               ]                  │
│         Current Position / Total Duration           │
├─────────────────────────────────────────────────────┤
│         Playback Controls                           │
│         [Play/Pause] [Skip] [Volume Slider]         │
│         [Like] [Dislike] (Full/Lite only)           │
├─────────────────────────────────────────────────────┤
│         Queue ("Next Up")                           │
│         - Passage 1: Title - Artist                 │
│         - Passage 2: Title - Artist                 │
│         - Passage 3: Title - Artist                 │
│         [Remove buttons]                            │
└─────────────────────────────────────────────────────┘
```

---

## Album Artwork Display

### Current Song Tracking

**[UI-ART-010]** The UI must track the currently playing song within a passage to determine which album art to display.

**[UI-ART-020]** Current song is determined by:
```
if position is within a song's time range:
    current_song = that_song
else:
    current_song = nearest_song(position)
```

Where `nearest_song()` returns the song with the closest start or end time to the current position.

### Artwork Priority and Selection

**[UI-ART-030]** For the currently playing song, select artwork using this priority:

1. Song-specific images (user-uploaded, Full version only)
2. Passage-specific images (user-uploaded, Full version only)
3. Album Front cover
4. Album Rear cover
5. Album Liner notes images
6. Artist images (photos)
7. Work images (sheet music, production stills)
8. WKMP Logo (fallback, always available)

**[UI-ART-040]** Display layout:
- **Left image**: Highest priority available image
- **Right image**: Next priority image(s), rotating every 15 seconds when multiple available
- **Single image mode**: Center the image with blank space left/right (when only logo or one image available)

### Album Art Rotation

**[UI-ART-050]** When the current song is associated with multiple albums:

1. Collect all albums associated with the song
2. For each album, collect available images in priority order: Front, Rear, Liner
3. Create rotation sequence from all collected images
4. Display each image for 15 seconds
5. Cycle through sequence continuously while song is playing

**[UI-ART-060]** Rotation timer behavior:
- Starts when song begins playing (timer = 0s)
- Pauses when playback is paused (maintains current image and timer position)
- Resumes from paused timer position when playback resumes (does not reset to 0s)
- Resets to 0s when crossing to a new song

**Example:** Song has 3 album images rotating every 15 seconds:
- At 8 seconds: Playback paused (showing image 1, timer at 8s)
- User pauses for 2 minutes
- Resume playback: Timer resumes at 8s, still showing image 1
- At 15 seconds: Rotate to image 2 (7 seconds after resume)
- At 30 seconds: Rotate to image 3
- At 45 seconds: Rotate back to image 1

**[UI-ART-070]** Song boundary transitions:
- When playback crosses from Song A to Song B:
  - Immediately update artwork to Song B's first image
  - Reset rotation timer
  - Begin rotation sequence for Song B

### Gap Handling

**[UI-ART-080]** When playback position is in a gap (not within any song):
- Calculate distance to previous song's end time
- Calculate distance to next song's start time
- Use the song with smaller distance as "current song"
- If no songs before or after, use passage-level images or logo

**[UI-ART-090]** Example scenario:
```
Passage structure:
- 0:00 - 0:05: Silence
- 0:05 - 3:30: Song A
- 3:30 - 3:35: Silence
- 3:35 - 6:45: Song B
- 6:45 - 6:50: Unidentified audio
- 6:50 - 10:00: Song C

Artwork selection:
- 0:00 - 0:02.5: Song A images (nearest to 0:00)
- 0:02.5 - 3:32.5: Song A images (within range + nearest)
- 3:32.5 - 6:47.5: Song B images (nearest to gap)
- 6:47.5 - 10:00: Song C images (nearest + within range)
```

**[UI-ART-100]** Multi-album song rotation example:
```
Song appears on 3 albums, each with front and rear art:
- 0-15s: Album A front
- 15-30s: Album B front
- 30-45s: Album C front
- 45-60s: Album A rear
- 60-75s: Album B rear
- 75-90s: Album C rear
- (cycle repeats)
```

**[UI-ART-110]** Implementation timing:
- Artwork selection logic runs every 500ms (synchronized with position updates)
- Ensures timely updates when crossing song boundaries
- Rotation timer separate from position updates (15-second intervals)

> **See:** [Event System - CurrentSongChanged](event_system.md) for event-driven artwork updates

---

## Now Playing Information

### Passage Title Display

**[UI-INFO-010]** Display passage title only when different from current song title AND album title.

**[UI-INFO-015]** Passage title determination:
- If `user_title` is set, use it as the passage title
- Otherwise, use `title` (from file tags)
- The "only when different" comparison uses whichever title is selected
- Passage title is displayed in addition to (not replacing) song title and album title

**[UI-INFO-016]** Display logic examples:

| Passage Title | Song Title | Album Title | Display Passage? | Reason |
|---------------|------------|-------------|------------------|--------|
| "Live at Wembley" | "Bohemian Rhapsody" | "Live at Wembley" | ❌ Hide | passage = album |
| "Bohemian Rhapsody" | "Bohemian Rhapsody" | "A Night at the Opera" | ❌ Hide | passage = song |
| "Medley" | "Bohemian Rhapsody" | "A Night at the Opera" | ✅ Show | passage ≠ both song and album |
| "Live at Wembley" | "Bohemian Rhapsody" | "Greatest Hits" | ✅ Show | passage ≠ both song and album |
| "Bohemian Rhapsody" | "Bohemian Rhapsody" | "Bohemian Rhapsody" | ❌ Hide | passage = both song and album |

**Rule:** Display passage title ONLY when it differs from BOTH the song title AND the album title. If passage title matches either one, hide it to avoid redundancy.

**[UI-INFO-020]** When passage contains:
- **One song**: Display that song's information (usually no passage title shown)
- **Multiple songs**: Display currently playing song based on playback position
- **Zero songs**: Display passage information only

### Current Song Information

**[UI-INFO-030]** Display for the currently playing song:

1. **Title**: Song title
2. **Artist(s)**:
   - Single artist: "Artist Name"
   - Multiple artists: "Artist 1, Artist 2, Artist 3"
   - No artist: "Unknown Artist"
3. **Album**: Album title (or "Unknown Album")
4. **Play History** (global/system-wide): (Full and Lite versions only)
   - Time since last play: "Last played: 2 hours ago" / "3 days ago" / "Never"
   - Play counts:
     - Past week: "5 plays this week"
     - Past month: "12 plays this month"
     - Past year: "48 plays this year"
     - All time: "127 plays total"
   - **Note**: Play history is global (all users see the same history)

**[UI-INFO-040]** Play history time formatting:
- < 1 hour: "X minutes ago"
- < 24 hours: "X hours ago"
- < 7 days: "X days ago"
- < 4 weeks: "X weeks ago"
- < 1 year: "X months ago"
- ≥ 1 year: "X years ago"
- Never played: "Never"

### Lyrics Display

**[UI-LYRICS-010]** **(Full and Lite versions only)** Display lyrics for current passage when available.

**[UI-LYRICS-020]** Lyrics display:
- Scrollable text area below album art and song info
- Plain UTF-8 text (preserve line breaks)
- Auto-scroll disabled by default (user can scroll manually)
- "No lyrics available" message when fallback chain exhausted (see below)

**[UI-LYRICS-025]** Lyrics fallback chain logic:

The system attempts to find lyrics using a fallback chain to maximize the chance of displaying relevant lyrics:

1. **Check current song's lyrics field**
   - If current song has `lyrics` field populated (non-empty), display those lyrics

2. **Check related songs (if current song lyrics empty)**
   - Iterate through `related_songs` array (ordered from most to least closely related)
   - Display lyrics from first related song with non-empty `lyrics` field
   - Related songs represent the same work by different artists or different performances

3. **Display "No lyrics available"**
   - Only shown when fallback chain is exhausted: current song AND all related songs have empty/null lyrics
   - This indicates no lyrics are available for this work in the database

**Example:** Current song is "Bohemian Rhapsody (Live at Wembley)" with no lyrics:
- Step 1: Check "Bohemian Rhapsody (Live at Wembley)" lyrics → empty
- Step 2: Check related_songs[0] "Bohemian Rhapsody (Studio)" lyrics → found! Display these lyrics
- Result: Studio version lyrics displayed for live performance (same work, different performance)

> **See:** [Architecture - Lyrics Display Behavior](architecture.md#lyrics-display-behavior) for technical implementation details

**[UI-LYRICS-030]** **(Full version only)** Lyrics editing:
- "Edit Lyrics" button visible
- Opens text editor modal/panel
- Save/Cancel buttons
- Concurrent editing uses "last write wins" strategy (no conflict resolution)
- Optional: Display warning if other users are editing simultaneously

> **See:** [API Design - Lyrics Endpoints](api_design.md#lyrics-full-version-only)
> **See:** [Multi-User Coordination - Concurrent Lyric Editing](multi_user_coordination.md#3-concurrent-lyric-editing) for concurrent editing behavior

---

## Playback Controls

### Play/Pause Button

**[UI-CTRL-010]** Display state-appropriate icon:
- **Playing state**: Show "Pause" icon (⏸)
- **Paused state**: Show "Play" icon (▶)

**[UI-CTRL-020]** Button remains active regardless of queue state:
- Empty queue + Playing state: Button shows "Pause" (can pause silence)
- Empty queue + Paused state: Button shows "Play" (ready to play when passage enqueued)

### Skip Button

**[UI-CTRL-030]** Skip to next passage in queue.

**[UI-CTRL-040]** Skip throttling (multi-user coordination):
- Skip requests within 5 seconds of previous skip are ignored
- Display temporary message: "Skip throttled, please wait"
- Button disabled for 5 seconds after successful skip

> **See:** [Multi-User Coordination - Skip Throttling](multi_user_coordination.md#1-skip-throttling) for complete throttling behavior

### Volume Control

**[UI-CTRL-050]** Volume slider:
- Range: 0-100 (percentage)
- Current volume displayed numerically: "Volume: 75%"
- Updates in real-time as user drags slider
- Updates from SSE when other users change volume

**[UI-CTRL-060]** Volume changes apply immediately (no "apply" button needed).

### Like/Dislike Buttons

**[UI-LIKE-010]** **(Full and Lite versions only)** Display Like and Dislike buttons.

**[UI-LIKE-020]** Button behavior:
- Single click: Apply weight 1.0 to current passage
- Multiple clicks (same button, within 5 minutes): Increase weight
  - 2 clicks = weight 2.0
  - 3 clicks = weight 3.0
- Opposite button click (within 5 minutes): Decrease weight (undo)
  - Like (weight 2.0) + Dislike click = Like (weight 1.0)
  - Like (weight 1.0) + Dislike click = neutral (weight 0.0)

**[UI-LIKE-030]** Visual feedback:
- Show current weight on button: "Like (2)" or just "Like"
- Highlight button when weight > 0
- Brief animation on click

**[UI-LIKE-040]** Detailed like/dislike view (expandable section):
- List of recently liked/disliked songs
- Display current weight for each
- Allow manual weight adjustment (text input, 0.0-10.0 range)
- Show timestamp of last like/dislike

> **See:** [Likes and Dislikes](like_dislike.md) for weight distribution algorithm

### Seek Control

**[UI-CTRL-070]** Progress bar is interactive (clickable/draggable):
- Click anywhere on bar to jump to that position within current passage
- Drag playhead to scrub through passage
- Display position tooltip on hover
- Valid range: 0 seconds to passage duration
- Seeking to the end (passage duration) has the same effect as skip

---

## Queue Display ("Next Up")

### Queue List

**[UI-QUEUE-010]** Display upcoming passages:
- Show minimum 3 passages when available
- List format: "Passage Title - Primary Artist"
- Truncate long titles with ellipsis
- Display passage duration: "3:45"

**[UI-QUEUE-020]** Queue empty state:
- Display message: "Queue is empty"
- Show "Add Passage" button (links to library browser)
- Display explanation of why automatic selection is not working (if applicable)

**[UI-QUEUE-025]** Automatic selection unavailable messages:

When queue is empty and automatic selection cannot enqueue passages, display the reason instead of "now playing" song information:

1. **No songs with flavor data:**
   - Message: "The library does not contain any songs with musical flavor definitions."
   - Action: "Import music files or wait for flavor data retrieval" (Full version)

2. **All songs in cooldown:**
   - Message: "All songs in the library with musical flavor definitions have been played recently. Waiting until their cooldown periods elapse."
   - Show countdown to next available song (optional)

3. **Program Director processing:**
   - Message: "The Program Director is working hard to select your next song, please be patient."
   - Show spinner/loading indicator

4. **Program Director not responding:**
   - Message: "The Program Director is not responding to my requests for something to play."
   - Action: "Check system status" or "Restart Program Director" (with appropriate permissions)

**Note:** These messages replace the "now playing" display area, not the queue display area.

### Queue Management

**[UI-QUEUE-030]** Each queue entry has a "Remove" button (❌ icon).

**[UI-QUEUE-040]** Concurrent removal handling:
- If another user removes same passage, update queue immediately
- Show brief notification: "Passage removed by another user"

> **See:** [Multi-User Coordination - Concurrent Queue Removal](multi_user_coordination.md#2-concurrent-queue-removal) for idempotent removal behavior

**[UI-QUEUE-050]** Manual enqueueing:
- "Add to Queue" button opens passage selector
- Search/filter by title, artist, album
- Can enqueue zero-song passages (automatic selection cannot)

---

## Network Status Indicators

### Internet Connection Status (Full Version Only)

**[UI-NET-010]** Display small, clear internet status indicator in library management / import UI.

**[UI-NET-020]** Status states and display:

| State | Icon | Text | Description |
|-------|------|------|-------------|
| Connected | ✓ (green) | "Online" | Internet accessible |
| Retrying | ⟳ (yellow) | "Connecting (N/20)" | Retry attempt N of 20 |
| Failed | ⚠ (red) | "Offline - Retry" | 20 retries exhausted |

**[UI-NET-030]** Position:
- Visible in library/import section (not main playback view)
- Small but clearly visible
- Does not obstruct primary UI elements

### Connection Retry Controls

**[UI-NET-040]** When status is "Failed":
- Display "Retry Connection" button
- Clicking resets counter to 20 attempts
- Show retry progress during reconnection

**[UI-NET-050]** Automatic retry triggers:
- User clicks any internet-dependent control (Import, metadata refresh)
- Automatically starts retry sequence
- Update status indicator to "Retrying (1/20)"

### Feature Availability Feedback

**[UI-NET-060]** When user attempts internet-dependent action while disconnected:
- Display modal/toast notification:
  - **Title**: "Internet Connection Required"
  - **Message**: "This feature requires internet connection. Please check your connection and retry."
  - **Button**: "Retry Connection"
  - **Button**: "Cancel"

**[UI-NET-070]** List unavailable features without internet:
- Import new music files
- Fetch MusicBrainz metadata
- Retrieve AcousticBrainz flavor data
- Download cover art

**[UI-NET-080]** Lite and Minimal versions:
- Do not display internet status indicator (no internet required)

> **See:** [Requirements - Network Error Handling](requirements.md#network-error-handling)

---

## Library Management UI (Full Version Only)

### Import View

**[UI-IMPORT-010]** Library import interface displays:
- "Select Folders" button (opens folder picker)
- List of selected folders with paths
- "Import" button to start scan
- Progress indicator during scan

**[UI-IMPORT-020]** Import progress display:
- Current file being processed
- Files processed / Total files
- Percentage complete
- Cancel button (stops import, keeps partial results)

**[UI-IMPORT-030]** Import completion:
- Summary: "Added X files, Updated Y files"
- Show any errors encountered
- "View Library" button

### Passage Editor

**[UI-EDIT-010]** Passage boundary editing interface:
- Waveform display of audio file
- Draggable markers for passage boundaries
- Play preview from marker position
- Save/Cancel buttons

**[UI-EDIT-020]** Crossfade timing editor:
- Lead-in time slider/input
- Lead-out time slider/input
- Preview crossfade button
- Reset to default button

**[UI-EDIT-030]** Metadata editor:
- User title field (overrides tag title)
- Associate with songs (search MusicBrainz)
- Album art uploader
- Lyrics text area

> **See:** [Library Management](library_management.md) for import workflows

---

## Base Probability Editor

### Probability Adjustment Interface

**[UI-PROB-010]** Base probability editing for songs, artists, and works:
- **Logarithmic slider**: Range 0.0 to 1000.0
  - Visual scale: |--0.1--|--1.0--|--10--|--100--|--1000--|
  - Slider marks at: 0.0, 0.1, 1.0, 10.0, 100.0, 1000.0
- **Numeric input**: Direct value entry (validates 0.0-1000.0)
- **Reset button**: Return to 1.0 (default)

**[UI-PROB-020]** Slider behavior:
- Logarithmic scale for intuitive adjustment
- Small values (< 1.0): Deprioritize selection
- Value 1.0: Neutral (default)
- Large values (> 1.0): Prioritize selection

**[UI-PROB-030]** Visual feedback:
- Current value displayed: "Base Probability: 2.5"
- Interpretation hint:
  - < 0.5: "Much less likely"
  - 0.5-0.9: "Less likely"
  - 0.9-1.1: "Default"
  - 1.1-2.0: "More likely"
  - > 2.0: "Much more likely"

**[UI-PROB-040]** Batch operations:
- Select multiple songs/artists
- Apply same probability adjustment to all
- "Set all to default" option

> **See:** [Program Director](program_director.md#user-configurable-parameters) for probability usage

---

## Timeslot Configuration UI

### Timeslot Editor

**[UI-TIME-010]** 24-hour timeslot schedule editor:
- Visual timeline showing all timeslots
- Color-coded timeslot blocks
- Start/end time for each timeslot
- Reference passages list per timeslot

**[UI-TIME-020]** Timeslot management:
- Add new timeslot button
- Delete timeslot button (minimum 1 timeslot required)
- Drag timeslot boundaries to adjust times
- Validation: No gaps or overlaps allowed

**[UI-TIME-030]** Reference passage assignment:
- "Add Reference Passage" button per timeslot
- Search/filter passage library
- Drag-and-drop to reorder
- Remove reference passage button
- Minimum 1 reference passage per timeslot

**[UI-TIME-040]** Temporary flavor override:
- "Set Temporary Override" button
- Duration selector: 1 hour, 2 hours, 4 hours, custom
- Reference passage selector
- Countdown timer showing time remaining
- "Cancel Override" button

> **See:** [Requirements - Musical Flavor Target](requirements.md#musical-flavor-target-by-time-of-day)

---

## Real-Time Updates (Server-Sent Events)

### SSE Connection

**[UI-SSE-010]** UI connects to `/api/events` on page load.

**[UI-SSE-020]** Connection behavior:
- Maintain persistent connection
- Automatic reconnection on disconnect (exponential backoff)
- No event replay on reconnection (fetch current state via GET /api/status)

### Event Handling

**[UI-SSE-030]** UI subscribes to and handles these events:

| Event | UI Action |
|-------|-----------|
| `passage_started` | Update now playing info, reset progress bar |
| `passage_completed` | Clear now playing if queue empty |
| `playback_state_changed` | Update play/pause button icon |
| `position_update` | Update progress bar position |
| `volume_changed` | Update volume slider position |
| `queue_changed` | Refresh queue display |
| `user_action` | Show subtle indicator: "Another user skipped" |
| `current_song_changed` | Update artwork and song info |
| `queue_empty` | Show "Queue is empty" message |

**[UI-SSE-040]** Multi-user synchronization:
- All connected clients receive same events
- UI updates reflect actions from any user
- Optimistic updates for local user actions (instant feedback)
- Server events confirm/correct optimistic updates

> **See:** [API Design - Server-Sent Events](api_design.md#server-sent-events-sse)
> **See:** [Event System](event_system.md) for complete event specifications

---

## Responsive Design

### Desktop Layout

**[UI-RESP-010]** Desktop browsers (≥ 1024px width):
- Two-column layout: Artwork left, info/controls right
- Full queue visible (scrollable if > 10 items)
- Lyrics panel below or side-by-side with artwork

### Tablet Layout

**[UI-RESP-020]** Tablet browsers (768px - 1023px width):
- Single column layout: Artwork top, info/controls below
- Collapsed queue (expandable)
- Lyrics in expandable panel

### Mobile Layout

**[UI-RESP-030]** Mobile browsers (< 768px width):
- Artwork fills width
- Compact controls
- Swipe gestures:
  - Swipe left: Skip
  - Swipe right: Show queue
  - Swipe down: Show lyrics (if available)
- Bottom navigation bar for main sections

**[UI-RESP-040]** Touch-friendly targets:
- Minimum button size: 44x44 pixels
- Adequate spacing between interactive elements
- No hover-only interactions

---

## Accessibility

### Screen Reader Support

**[UI-A11Y-010]** All interactive elements have descriptive labels:
- Play/Pause button: "Play music" / "Pause music"
- Skip button: "Skip to next passage"
- Volume slider: "Volume control, currently 75%"
- Like/Dislike buttons: "Like this passage" / "Dislike this passage"

**[UI-A11Y-020]** Dynamic content updates announced:
- New passage playing: "Now playing: [song] by [artist]"
- Queue changes: "Passage added to queue" / "Passage removed"
- Volume changes: "Volume set to 75%"

### Keyboard Navigation

**[UI-A11Y-030]** All controls accessible via keyboard:
- Tab navigation through all interactive elements
- Enter/Space to activate buttons
- Arrow keys for sliders (volume, seek)
- Keyboard shortcuts:
  - Space: Play/Pause
  - Right arrow: Skip
  - Up/Down arrows: Volume
  - L: Like
  - D: Dislike

### Visual Accessibility

**[UI-A11Y-040]** Color contrast meets WCAG AA standards:
- Text contrast ratio ≥ 4.5:1
- Interactive element contrast ratio ≥ 3:1

**[UI-A11Y-050]** Color is not the only indicator:
- Connection status uses icon + text + color
- Play/Pause uses icon shape, not just color
- Focus indicators visible on all interactive elements

---

## Performance Requirements

### Load Time

**[UI-PERF-010]** Initial page load:
- Time to interactive: < 2 seconds (desktop)
- Time to interactive: < 5 seconds (mobile)

**[UI-PERF-020]** Asset optimization:
- Minified CSS and JavaScript
- Compressed images
- Lazy loading for non-critical content

### Update Responsiveness

**[UI-PERF-030]** Real-time update latency:
- Position updates: Every 500ms
- SSE event handling: < 100ms from receipt to UI update
- User input response: < 50ms visual feedback

**[UI-PERF-040]** Smooth animations:
- 60fps for progress bar updates
- 60fps for artwork transitions
- No jank during scrolling

---

## Browser Compatibility

**[UI-COMPAT-010]** Supported browsers:
- **Desktop**: Chrome 90+, Firefox 88+, Safari 14+, Edge 90+
- **Mobile**: iOS Safari 14+, Chrome Android 90+

**[UI-COMPAT-020]** Required features:
- ES6 JavaScript
- CSS Grid and Flexbox
- Server-Sent Events (EventSource API)
- LocalStorage
- Fetch API

**[UI-COMPAT-030]** Graceful degradation:
- Basic playback controls work without JavaScript
- Progressive enhancement for advanced features

---

## Security Considerations

**[UI-SEC-010]** Client-side security:
- UUID stored in localStorage (not cookies)
- No sensitive data cached client-side
- HTTPS recommended but not required (localhost only)

**[UI-SEC-020]** Input validation:
- All user input sanitized before sending to server
- Server performs final validation (client-side is convenience only)

**[UI-SEC-030]** XSS prevention:
- All dynamic content properly escaped
- No `eval()` or `innerHTML` with user data
- Content Security Policy headers

---

## Error Handling

### Network Errors

**[UI-ERR-010]** When API request fails:
- Display toast notification: "Connection error. Retrying..."
- Retry with exponential backoff (3 attempts)
- If all retries fail: "Unable to connect. Please check your connection."

### Playback Errors

**[UI-ERR-020]** When passage playback fails:
- Display notification: "Playback error. Skipping to next passage."
- Automatically skip to next passage in queue
- Log error details for debugging

### User Input Errors

**[UI-ERR-030]** Validation errors displayed inline:
- Username too short: "Username must be at least 1 character"
- Invalid probability value: "Please enter a value between 0.0 and 1000.0"
- Empty required field: "This field is required"

---

## Testing Requirements

**[UI-TEST-010]** Unit tests for:
- Artwork selection logic
- Current song determination
- Rotation timer behavior
- Gap handling

**[UI-TEST-020]** Integration tests for:
- SSE event handling
- Multi-user synchronization
- Authentication flow
- API error handling

**[UI-TEST-030]** Visual regression tests for:
- Layout on different screen sizes
- Browser compatibility
- Accessibility (automated tools)

**[UI-TEST-040]** Manual testing:
- Multi-user scenarios (multiple browsers)
- Touch gestures on mobile devices
- Screen reader navigation
- Keyboard-only operation

----
End of document - WKMP User Interface Specification
