# Implementation Order

**📋 TIER 4 - DOWNSTREAM EXECUTION PLAN**

This document **aggregates** all specifications to define WHEN features are built. It does NOT define requirements or design.

**Update Policy:** ✅ Always update when upstream docs change | ❌ NEVER update upstream docs from this

> See [Document Hierarchy](document_hierarchy.md) for complete document relationships and update policies.

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md) | [Database Schema](database_schema.md) | [Event System](event_system.md) | [Crossfade Design](crossfade.md) | [Musical Flavor](musical_flavor.md) | [Coding Conventions](coding_conventions.md) | [Requirements Enumeration](requirements_enumeration.md)

---

## Phase 0: Documentation & Project Setup (Week 0)

### 0. Project documentation foundation
- ✅ Requirements specification complete
- ✅ Architecture design complete
- ✅ Database schema defined
- ✅ Crossfade timing specification complete
- ✅ Musical flavor system defined
- ✅ Event system architecture documented
- ✅ Coding conventions established
- ✅ Requirements enumeration scheme defined
- **Next steps:**
  - Apply requirement IDs to all requirements documents (see [Requirements Enumeration](requirements_enumeration.md))
  - Set up project repository structure following coding conventions
  - Configure Rust project with feature flags (full/lite/minimal)
  - Set up CI/CD pipeline with linting (clippy, rustfmt)

## Phase 1: Foundation (Weeks 1-3)

### 1. Database schema + migrations
- Core tables: files, passages, songs, artists, works, albums (see [Database Schema](database_schema.md))
- Musical flavor storage (AcousticBrainz high-level characterization vectors)
- Play history, queue state, settings
- Timeslot definitions for flavor targets
- Version migration system
- **Coding standards**: Follow [CO-170](coding_conventions.md#database-code) for database access patterns
- **Traceability**: Add requirement IDs to schema creation code (CO-271)

### 2. Basic file scanner + metadata extraction
- Recursive directory scanning with change detection (Full version only)
- ID3v2/Vorbis/MP4 tag parsing
- File hash computation (SHA-256)
- Single-passage-per-file assumption initially
- Store extracted metadata in SQLite
- **Module organization**: Follow [CO-030](coding_conventions.md#separation-of-concerns) for library scanner modules
- **Coding standards**: Module size limits (CO-010), function complexity (CO-020)

### 3. Event system foundation
- Implement EventBus using `tokio::broadcast` (see [Event System](event_system.md))
- Define `McRhythmEvent` enum with initial event types:
  - PlaybackEvents: PassageStarted, PassageCompleted, PlaybackStateChanged
  - Basic infrastructure for event emission and subscription
- Set up event bus in application initialization
- **Coding standards**: Follow [CO-140](coding_conventions.md#asyncawait-and-concurrency) for async organization
- **Architecture**: Implements event-driven design from [Architecture - Inter-component Communication](architecture.md#inter-component-communication)

### 4. Simple playback engine (single passage, no crossfade)
- GStreamer pipeline: file source → decodebin → audioconvert → audioresample → volume → audio sink
- Play/Pause/Stop/Seek/Volume controls
- Position reporting (500ms intervals)
- Basic error handling (skip on error)
- Audio sink auto-detection (ALSA/PulseAudio/CoreAudio/WASAPI)
- **Emit events**: PassageStarted, PassageCompleted, PlaybackStateChanged
- **Module organization**: Follow [CO-130](coding_conventions.md#gstreamer-integration) for GStreamer code
- **Error handling**: Follow [CO-160](coding_conventions.md#error-handling) for error types

### 5. Minimal REST API + basic Web UI
- Status endpoint + playback control endpoints
- Simple HTML/CSS/JS frontend with play controls
- Volume slider and seek bar
- Current passage display (from file tags)
- SSE endpoint skeleton (for future real-time updates)
- **Command channels**: Use tokio::mpsc for API → Playback Controller commands
- **Tauri integration**: Follow [CO-210](coding_conventions.md#tauri-command-handlers) for command handlers

## Phase 2: Core Player Features (Weeks 4-6)

### 6. Queue management
- In-memory queue with SQLite persistence
- Add/remove operations with edge case handling
- Auto-advance on completion
- State persistence (queue + volume + position)
- Manual user queue additions (including zero-song passages)
- **Emit events**: QueueChanged, PassageEnqueued, PassageDequeued
- **Subscribe to events**: PassageCompleted (to trigger auto-advance)
- **Module organization**: Follow [CO-120](coding_conventions.md#logical-grouping) for queue module structure

### 7. Historian component
- Subscribe to PlaybackEvents (PassageStarted, PassageCompleted)
- Record passage plays with timestamps
- Update last-play times for songs, artists, works via database triggers
- Track completion vs skip status
- Duration played for skip detection
- **Event-driven design**: Implements subscriber pattern from [Event System](event_system.md)
- **Decoupling**: Historian has no direct dependencies on Playback Controller
- **Testing**: Follow event-driven testing patterns from event_system.md

### 8. SSE real-time updates
- Event stream endpoint implementation
- Subscribe to all McRhythmEvent types for UI broadcasting
- Broadcast state changes to all connected clients
- Multi-user edge case handling: See [Multi-User Coordination](multi_user_coordination.md) for the detailed specification.
- **Architecture**: Implements REQ-CF-042 (multiple users, real-time sync)
- **Event integration**: SSE Broadcaster is primary event consumer for UI updates

### 9. Album art handling
- Extract embedded cover art from file tags
- Save as image files in same directory as audio files
  - Naming: `{filename}.cover.{ext}` (e.g., `song.mp3.cover.jpg`)
- Fetch album art from external sources (Cover Art Archive via MusicBrainz)
  - Naming: `{album_mbid}.front.{ext}` and `{album_mbid}.back.{ext}`
  - Store in audio file directory
- Resize to max 1024x1024 (preserve aspect ratio) when saving
- Store file path references in database
- Serve via REST API (read from filesystem)
- Display in UI with fallback placeholder

## Phase 3: Crossfade & Advanced Playback (Weeks 7-9)

### 10. Passage boundary editor UI
- Manual editing of start/fade-in/lead-in/lead-out/fade-out/end times
- Time input fields with validation (see [Crossfade Design - Constraints](crossfade.md#constraints))
- Per-passage fade profile selection (exponential/cosine/linear)
- Waveform visualization (optional: can defer to later phases)
- **Crossfade spec**: Implements XFD-TP-010 through XFD-CONS-010
- **UI patterns**: Follow [CO-080](coding_conventions.md#frontend-javascript-organization) for frontend organization

### 11. Dual-pipeline crossfade engine
- Secondary GStreamer pipeline for next passage
- audiomixer element for blending overlapping passages
- Volume automation using GStreamer controller API or custom audio filter
- Implement three fade profiles (see [Crossfade Design - Fade Curves](crossfade.md#fade-curves)):
  - Exponential fade in / Logarithmic fade out (XFD-CURV-010)
  - Cosine (S-curve) fade in/out (XFD-CURV-020)
  - Linear fade in/out (XFD-CURV-030)
- Lead-in/lead-out timing logic:
  - Case 1: Longer lead-in duration (XFD-BEH-C1)
  - Case 2: Shorter lead-in duration (XFD-BEH-C2)
  - Case 3: No overlap (XFD-BEH-C3)
- Pause/resume with 0.5s exponential fade-in (REQ-PB-050)
- **Architecture**: Dual pipeline design from [Architecture - Audio Engine](architecture.md#audio-engine)
- **Traceability**: Reference XFD requirement IDs in crossfade timing code (CO-273)

### 12. Multi-passage file handling
- User prompt: single or multi-passage file on import
- Automatic silence detection for initial segmentation (algorithm TBD: threshold, min duration)
- UI for editing passage boundaries within a file
- Add/delete passage definitions per file
- Per-passage MusicBrainz association editing

## Phase 4: External Integration (Weeks 10-12)

### 13. Audio fingerprinting + AcoustID
- Chromaprint integration (library inclusion)
- Generate fingerprints for each passage
- Query AcoustID API with rate limiting (3 req/sec)
- Cache responses locally (indefinite storage, prune oldest when needed)
- Retrieve MusicBrainz Recording IDs
- **Module organization**: External API client in `external/acoustid.rs` (CO-042)
- **Error handling**: Network retry logic (REQ-NET-010, CO-166)

### 14. MusicBrainz integration
- Lookup Recording/Release/Artist/Work IDs from AcoustID results
- Fetch and cache metadata:
  - Canonical artist names
  - Release titles and dates
  - Genre/tags (top 10)
- Local caching with offline fallback
- Associate passages with tracks/recordings/artists/works
- Multi-song passage handling
- **Module organization**: External API client in `external/musicbrainz.rs` (CO-042)
- **Event emission**: NetworkStatusChanged when connectivity issues occur

### 15. AcousticBrainz integration
- Fetch high-level characterization data (musical flavor vectors - see [Musical Flavor](musical_flavor.md))
- Parse and store all dimensional values:
  - Binary classifications (danceability, gender, moods - FLV-CAT-010)
  - Multi-dimensional characterizations (genre systems, rhythms - FLV-CAT-020)
- Cache results in database
- **Essentia local analysis (Full version only)**:
  - Integrate Essentia library for local computation when AcousticBrainz data unavailable
  - Background processing during import or on-demand
  - Progress indication in UI
  - Emit LibraryScanCompleted event when analysis finishes
  - Optimize for Raspberry Pi Zero2W resource constraints
- **Module organization**: External API client in `external/acousticbrainz.rs` (CO-042)

### 16. Musical flavor position calculation
- For single-song passages: use song's AcousticBrainz position directly (FLV-MAP-ONE)
- For multi-song passages: calculate a weighted average of all dimensional values, based on the duration of each song within the passage (FLV-MAP-MANY).
- Store computed flavor position in passage database table
- Handle zero-song passages (no flavor, excluded from auto-selection - FLV-MAP-ZERO)
- **Specification**: Implements [Musical Flavor - Mapping](musical_flavor.md#mapping)

## Phase 5: Musical Flavor Selection System (Weeks 13-16)

### 17. Distance calculation implementation
- Implement squared Euclidean distance formula (see [Musical Flavor - Distance Calculation](musical_flavor.md#distance-calculation)):
  - Binary classifications: Σ(diff²) for all binary pairs (FLV-DIST-BIN)
  - Multi-dimensional groups: Σ(diff²)/N for each group, then average all groups (FLV-DIST-MULTI)
  - Total distance: binary_distance + average_multidim_distance (FLV-DIST-TOT)
- Unit tests with known distance calculations
- Performance optimization for 100-candidate filtering (CO-253)
- **Module organization**: Distance calculations in `selection/flavor_distance.rs` (CO-032)

### 18. Time-of-day flavor target system
- **Timeslot management UI**:
  - Add/remove/edit timeslots (must cover all 24 hours, no gaps/overlaps)
  - Select passages to define each timeslot's flavor (FLV-USE-PREF)
  - Validate timeslot passages contain one or more songs
- **Timeslot flavor calculation**:
  - Average position of all selected passages
- **Schedule-based target switching**:
  - Determine current timeslot flavor target based on current time
  - Passages in queue unaffected by timeslot transitions (REQ-FLV-030)
  - Emit TimeslotChanged event when timeslot boundary crossed
- **Temporary flavor override**:
  - UI to select passages for temporary flavor target
  - Duration selection (e.g., 1-2 hours)
  - Emit TemporaryFlavorOverride event
  - Override triggers immediate queue flush and reselection (REQ-FLV-020)
  - Skip remaining time on currently playing passage
  - Refill queue based on new target
  - Emit TemporaryFlavorOverrideExpired when duration elapses
- **Event flows**: See [Event System - Temporary Flavor Override Flow](event_system.md#temporary-flavor-override-flow)

### 19. Base probability system
- Song/artist/work base probability storage (default 1.0, range 0.0-1000.0 - REQ-PROB-010/020/030)
- UI with logarithmic slider + numeric input for editing (REQ-PROB-041)
- **Passage probability calculation** (REQ-PROB-050):
  - Single song/artist: straightforward product
  - Multiple songs: weighted average based on duration (see requirements.md)
  - Multiple artists: weighted sum of artist probabilities (see requirements.md)
- **Module organization**: Probability calculations in `selection/probability.rs` (CO-032)

### 20. Cooldown system
- **Song cooldowns** (REQ-SEL-030):
  - Minimum: 7 days (default, user-editable - REQ-SEL-031A)
  - Ramping: 14 days (default, user-editable - REQ-SEL-032A)
  - Linear ramp from 0.0 to 1.0 probability multiplier
- **Artist cooldowns** (REQ-SEL-040):
  - Minimum: 2 hours (default, user-editable - REQ-SEL-041A)
  - Ramping: 4 hours (default, user-editable - REQ-SEL-042A)
- **Work cooldowns** (REQ-SEL-060):
  - Minimum: 3 days (default, user-editable)
  - Ramping: 7 days (default, user-editable)
- **Cooldown stacking** (REQ-SEL-070):
  - Net probability = base × song_multiplier × artist_multiplier × (work_multiplier if applicable)
- **Multi-artist cooldown handling**:
  - For songs with multiple artists, the artist cooldown multiplier is a weighted average of each artist's individual cooldown multiplier.
- **Module organization**: Cooldown calculations in `selection/cooldown.rs` (CO-032)
- **Testing**: Unit tests for cooldown edge cases (CO-093)

### 21. Flavor-based passage selection algorithm
- Filter passages to non-zero probability candidates (FLV-USE-FILT)
- Calculate squared distance from target flavor for all candidates (FLV-USE-COMP)
- Sort by distance (closest first - FLV-USE-SORT)
- Take top 100 (or all if fewer - FLV-USE-TOP)
- Compute final probabilities (base × cooldown multipliers)
- Weighted random selection (FLV-USE-RAND):
  - Random number in [0, Σ(probabilities)]
  - Iterate candidates, subtract probability until reaching zero/below
  - Select first candidate that zeros the random value
- Handle edge case: no candidates available (enter Pause mode - REQ-CF-061B2)
- **Specification**: Implements [Musical Flavor - Selection Algorithm](musical_flavor.md#usage)
- **Module organization**: Selection logic in `selection/director.rs` (CO-032)
- **Testing**: Unit tests with known probability distributions (CO-093)

### 22. Automatic queue replenishment
- Monitor queue: maintain 3+ passages and 15+ minutes play time (REQ-CF-061B1)
- Subscribe to QueueChanged events to detect low queue depth
- Trigger selection when threshold not met
- Use current timeslot flavor target (or temporary override)
- Account for anticipated play start time when selecting
- Send SelectionRequest via mpsc channel to Program Director
- Emit PassageEnqueued event when passage added
- Background task for async selection
- **Event flow**: See [Event System - Automatic Passage Selection Flow](event_system.md#automatic-passage-selection-flow)

## Phase 6: User Feedback & Refinement (Weeks 17-18)

### 23. Like/Dislike functionality
- Record likes/dislikes with timestamps in database
- Emit PassageLiked and PassageDisliked events
- **Effect on selection** (*specification needed*):
  - How likes affect base probability
  - How dislikes affect base probability or cooldowns
  - Passage-level vs song-level vs artist-level
  - Cumulative vs idempotent behavior
- UI buttons (already in design)
- REST API endpoints (already defined)
- **Event emission**: Subscribe in UI to show feedback button state updates

### 24. ListenBrainz integration
- **Specification needed for**:
  - Outbound data (plays, likes/dislikes, skips, duration)
  - Inbound data (recommendations, taste profile)
  - Effect on selection algorithm
  - Sync timing and authentication
- Subscribe to PassageCompleted events for play submission
- Implement based on finalized specification
- Network retry logic (5s delay, 20 max retries - REQ-NET-010)
- Emit NetworkStatusChanged on connectivity issues
- Offline mode handling
- **Module organization**: External API client in `external/listenbrainz.rs` (CO-042)
- **Event-driven**: ListenBrainz client is event subscriber (no coupling to playback)

### 25. Lyrics functionality
- Storage in passage table (UTF-8 text)
- Display in UI with playback
- Split-window editor:
  - Left pane: text input
  - Right pane: web search panel for copy-paste
- PUT endpoint for updates (already defined)
- Concurrent edit handling (last write wins per requirements)

## Phase 7: Platform Support & Versions (Weeks 19-21)

### 26. Platform-specific startup
- **Linux**: systemd service unit file (REQ-CF-061A1)
- **Windows**: Task Scheduler XML config (REQ-CF-061A2)
- **macOS**: launchd plist file (REQ-CF-061A3)
- Auto-start configuration UI in settings
- Service installation/uninstallation helpers
- **Platform abstraction**: Follow [Architecture - Platform Abstraction](architecture.md#platform-abstraction)

### 27. Audio sink selection & output
- GStreamer sink auto-detection and enumeration
- Manual override UI
- Platform-specific defaults (see [Architecture - Audio Output](architecture.md#audio-output)):
  - Linux: PulseAudio (fallback to ALSA)
  - Windows: WASAPI
  - macOS: CoreAudio
- Bluetooth/HDMI output support testing
- Output device switching without playback interruption
- **Module organization**: Platform detection in `platform/audio.rs` (CO-120)

### 28. Version builds (Full/Lite/Minimal)

See [Requirements - Three Versions](requirements.md#three-versions) for complete feature comparison and resource profiles.

**Rust Feature Flags:**
```toml
[features]
default = ["minimal"]
minimal = []
lite = ["minimal", "preference-editing"]
full = ["lite", "library-management", "essentia-analysis"]
```

**Conditional Compilation:**
```rust
#[cfg(feature = "full")]
mod library_scanner;

#[cfg(feature = "lite")]
mod preference_editor;

#[cfg(not(feature = "minimal"))]
fn allow_editing() { /* ... */ }
```

**Database Export/Import:**
- Full version exports complete database snapshot
- Lite/Minimal versions import read-only database
- Migration tools for version upgrades
- Platform-specific packaging per version

**Feature Flag Guidelines:**
- Follow [CO-190](coding_conventions.md#version-specific-code) for version-specific code patterns
- Use `#[cfg(feature = "...")]` annotations consistently
- Document version applicability in module comments

## Phase 8: Polish & Optimization (Weeks 22-24)

### 29. Raspberry Pi Zero2W optimization
- Memory usage profiling and reduction
- GStreamer pipeline optimization:
  - Buffer sizes tuned for latency vs reliability
  - Dual-pipeline decode optimization (especially FLAC crossfades)
- Startup time reduction
- Thermal testing during extended playback
- SD card I/O optimization
- Event bus capacity tuning (500 events for Pi - see [Event System - Performance](event_system.md#performance-considerations))
- **Performance guidelines**: Follow [CO-250](coding_conventions.md#performance-considerations) and [CO-260](coding_conventions.md#raspberry-pi-optimization)

### 30. Error handling improvements
- Network retry logic verification (5s delay, 20 max - REQ-NET-010)
- Emit NetworkStatusChanged events on connectivity changes
- UI notifications for network issues via SSE
- Graceful degradation testing:
  - Offline operation with missing external data
  - Missing files (remove from queue, continue)
  - Database errors (retry once, log, continue)
- Developer logging with tracing crate (REQ-ERR-010):
  - File and line number identification
  - Configurable log levels
  - Rotation and size limits
- **Logging standards**: Follow [CO-220](coding_conventions.md#logging-and-tracing) for structured logging
- **Error handling**: Follow [CO-160](coding_conventions.md#error-handling) for error types

### 31. UI/UX refinements
- Queue display showing next 5 passages
- Progress bar smoothness (update every 500ms without jank)
- Responsive design for phone browsers
- Touch-friendly controls for mobile
- Keyboard shortcuts for desktop
- Loading states and spinners
- Error message display to users
- Edge case testing (REQ-CF-042):
  - Concurrent users (multiple browser sessions)
  - Rapid skip clicks via UserAction event throttling
  - Queue operations during playback transitions
- **Event-driven UI**: All updates via SSE from event bus
- **Multi-user sync**: Skip throttling (5-second window) via UserAction events

### 32. Comprehensive testing
- **Unit tests** (CO-090):
  - Cooldown calculation edge cases (CO-093)
  - Probability multiplication (including multi-song/artist when specified)
  - Distance formula with known inputs/outputs (FLV-DIST-TOT)
  - Timeslot boundary handling
  - Event emission and handling (see [Event System - Testing](event_system.md#testing-event-driven-code))
- **Integration tests** (CO-232):
  - GStreamer pipeline lifecycle
  - Database migrations
  - SSE broadcasting with event bus
  - Event propagation across components
- **End-to-end tests**:
  - REST API full workflows
  - User scenarios (startup → play → queue → shutdown)
  - Multi-user concurrent sessions
- **Performance tests**:
  - Selection algorithm with large libraries (10k+ passages)
  - Concurrent client connections
  - Memory leaks during extended operation
  - Event bus throughput on Raspberry Pi
- **Platform tests**:
  - Raspberry Pi Zero2W (Lite/Minimal)
  - Linux desktop (Full)
  - Windows, macOS (Full)
- **Testing patterns**: Follow [CO-230](coding_conventions.md#test-organization) and [CO-240](coding_conventions.md#test-naming-and-structure)

## Optional/Future Enhancements

### 33. Advanced visualizations
- Waveform display in passage editor
- Musical flavor visualization (radar chart, scatter plot)
- Play history graphs and statistics
- Real-time event flow visualization (debug mode)

### 34. Advanced features
- Import/export preferences and probability settings
- Playlist support (manual track sequences, temporary playlists)
- Smart shuffle modes
- Social features (share flavors, import others' timeslot configs)
- Plugin system using event subscription pattern

### 35. Mobile platforms
- Android/iOS versions using Flutter or Tauri mobile
- Native mobile UI
- Offline-first architecture
- Background playback support
- Event synchronization across devices

## Critical Path Dependencies

**Phase 0 Prerequisites**:
- ✅ Event system architecture defined ([Event System](event_system.md))
- ✅ Coding conventions established ([Coding Conventions](coding_conventions.md))
- ✅ Requirement enumeration scheme created ([Requirements Enumeration](requirements_enumeration.md))
- 🔲 Apply requirement IDs to all specification documents
- 🔲 Set up project structure with module organization (CO-030, CO-120)

**Blockers for Phase 5 (Selection System)**:

**Blockers for Phase 6 (User Feedback)**:
- Must specify Like/Dislike effect on selection
- Must specify ListenBrainz integration details

**Event System Integration** (addresses throughout phases):
- Phase 1: EventBus foundation established
- Phase 2: Core components become event-driven (Historian, SSE, Queue Manager)
- Phase 3+: All new features use event patterns for notifications
- Benefits: Loose coupling enables easier testing, feature additions, and multi-user support

**Recommended Implementation Approach**:
1. Complete Phase 0: Apply requirement IDs and set up project structure
2. Implement phases sequentially to validate each layer
3. Deploy to Raspberry Pi Zero2W for testing at end of each phase
4. Create feature flags to disable incomplete features in early releases
5. Document missing specifications as encountered, seek clarification before proceeding
6. Follow event-driven patterns from Phase 1 onwards for consistency
7. Reference coding conventions (CO-xxx) and requirement IDs (REQ-xxx, XFD-xxx, FLV-xxx) in all code

## Documentation Maintenance

As implementation progresses:
- Update this document with actual completion dates
- Mark blockers as resolved when specifications are finalized
- Add links to implementation PRs/commits for traceability
- Document deviations from original plan with rationale
- Keep event flow diagrams updated in event_system.md as features are added
