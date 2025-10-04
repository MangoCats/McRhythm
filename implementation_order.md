# Implementation Order

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md) | [Database Schema](database_schema.md)

## Phase 1: Foundation (Weeks 1-3)

### 1. Database schema + migrations
- Core tables: files, passages, songs, artists, works, albums
- Musical flavor storage (AcousticBrainz high-level characterization vectors)
- Play history, queue state, settings
- Timeslot definitions for flavor targets
- Version migration system

### 2. Basic file scanner + metadata extraction
- Recursive directory scanning with change detection (Full version only)
- ID3v2/Vorbis/MP4 tag parsing
- File hash computation (SHA-256)
- Single-passage-per-file assumption initially
- Store extracted metadata in SQLite

### 3. Simple playback engine (single passage, no crossfade)
- GStreamer pipeline: file source → decodebin → audioconvert → audioresample → volume → audio sink
- Play/Pause/Stop/Seek/Volume controls
- Position reporting (500ms intervals)
- Basic error handling (skip on error)
- Audio sink auto-detection (ALSA/PulseAudio/CoreAudio/WASAPI)

### 4. Minimal REST API + basic Web UI
- Status endpoint + playback control endpoints
- Simple HTML/CSS/JS frontend with play controls
- Volume slider and seek bar
- Current passage display (from file tags)
- SSE endpoint skeleton (for future real-time updates)

## Phase 2: Core Player Features (Weeks 4-6)

### 5. Queue management
- In-memory queue with SQLite persistence
- Add/remove operations with edge case handling
- Auto-advance on completion
- State persistence (queue + volume + position)
- Manual user queue additions (including zero-song passages)

### 6. Play history tracking
- Record passage plays with timestamps
- Update last-play times for songs, artists, works
- Track completion vs skip status
- Duration played for skip detection

### 7. SSE real-time updates
- Event stream endpoint implementation
- Broadcast state changes to all connected clients
- Multi-user edge case handling:
  - Skip throttling (5-second window)
  - Concurrent queue operations
  - Lyric edit submission

### 8. Album art handling
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

### 9. Passage boundary editor UI
- Manual editing of start/fade-in/lead-in/lead-out/fade-out/end times
- Time input fields with validation (constraint: start ≤ fade-in ≤ lead-in ≤ lead-out ≤ fade-out ≤ end)
- Per-passage fade profile selection (exponential/cosine/linear)
- Waveform visualization (optional: can defer to later phases)

### 10. Dual-pipeline crossfade engine
- Secondary GStreamer pipeline for next passage
- audiomixer element for blending overlapping passages
- Volume automation using GStreamer controller API or custom audio filter
- Implement three fade profiles:
  - Exponential fade in / Logarithmic fade out
  - Cosine (S-curve) fade in/out
  - Linear fade in/out
- Lead-in/lead-out timing logic (requirements lines 282-284)
- Pause/resume with 0.5s exponential fade-in

### 11. Multi-passage file handling
- User prompt: single or multi-passage file on import
- Automatic silence detection for initial segmentation (algorithm TBD: threshold, min duration)
- UI for editing passage boundaries within a file
- Add/delete passage definitions per file
- Per-passage MusicBrainz association editing

## Phase 4: External Integration (Weeks 10-12)

### 12. Audio fingerprinting + AcoustID
- Chromaprint integration (library inclusion)
- Generate fingerprints for each passage
- Query AcoustID API with rate limiting (3 req/sec)
- Cache responses locally (indefinite storage, prune oldest when needed)
- Retrieve MusicBrainz Recording IDs

### 13. MusicBrainz integration
- Lookup Recording/Release/Artist/Work IDs from AcoustID results
- Fetch and cache metadata:
  - Canonical artist names
  - Release titles and dates
  - Genre/tags (top 10)
- Local caching with offline fallback
- Associate passages with tracks/recordings/artists/works
- Multi-song passage handling

### 14. AcousticBrainz integration
- Fetch high-level characterization data (musical flavor vectors)
- Parse and store all dimensional values:
  - Binary classifications (danceability, gender, moods)
  - Multi-dimensional characterizations (genre systems, rhythms)
- Cache results in database
- **Essentia local analysis (Full version only)**:
  - Integrate Essentia library for local computation when AcousticBrainz data unavailable
  - Background processing during import or on-demand
  - Progress indication in UI
  - Optimize for Raspberry Pi Zero2W resource constraints

### 15. Musical flavor position calculation
- For single-song passages: use song's AcousticBrainz position directly
- For multi-song passages: arithmetically average all dimensional values
- Store computed flavor position in passage database table
- Handle zero-song passages (no flavor, excluded from auto-selection)

## Phase 5: Musical Flavor Selection System (Weeks 13-16)

### 16. Distance calculation implementation
- Implement squared Euclidean distance formula:
  - Binary classifications: Σ(diff²) for all binary pairs
  - Multi-dimensional groups: Σ(diff²)/N for each group, then average all groups
  - Total distance: binary_distance + average_multidim_distance
- Unit tests with known distance calculations
- Performance optimization for 100-candidate filtering

### 17. Time-of-day flavor target system
- **Timeslot management UI**:
  - Add/remove/edit timeslots (must cover all 24 hours, no gaps/overlaps)
  - Select passages to define each timeslot's flavor
  - Validate timeslot passages contain one or more songs
- **Timeslot flavor calculation**:
  - Average position of all selected passages
- **Schedule-based target switching**:
  - Determine current timeslot flavor target based on current time
  - Passages in queue unaffected by timeslot transitions
- **Temporary flavor override**:
  - UI to select passages for temporary flavor target
  - Duration selection (e.g., 1-2 hours)
  - Override triggers immediate queue flush and reselection
  - Skip remaining time on currently playing passage
  - Refill queue based on new target

### 18. Base probability system
- Song/artist base probability storage (default 1.0, range 0.0-1000.0)
- UI with logarithmic slider + numeric input for editing
- **Passage probability calculation** (TBD: clarify multi-song/multi-artist handling):
  - Single song/artist: straightforward product
  - Multiple songs: *needs specification* (multiply all? average? min? max?)
  - Multiple artists: *needs specification*

### 19. Cooldown system
- **Song cooldowns**:
  - Minimum: 7 days (default, user-editable)
  - Ramping: 14 days (default, user-editable)
  - Linear ramp from 0.0 to 1.0 probability multiplier
- **Primary performing artist cooldowns**:
  - Minimum: 2 hours (default, user-editable)
  - Ramping: 4 hours (default, user-editable)
- **Work cooldowns** (*specification needed*):
  - Define minimum/ramping times or remove from requirements
- **Cooldown stacking**:
  - Net probability = base × song_multiplier × artist_multiplier × (work_multiplier if applicable)
- **Featured artist handling** (*specification needed*):
  - Define featured vs primary artist distinction
  - Use lowest cooldown probability among all associated artists

### 20. Flavor-based passage selection algorithm
- Filter passages to non-zero probability candidates
- Calculate squared distance from target flavor for all candidates
- Sort by distance (closest first)
- Take top 100 (or all if fewer)
- Compute final probabilities (base × cooldown multipliers)
- Weighted random selection:
  - Random number in [0, Σ(probabilities)]
  - Iterate candidates, subtract probability until reaching zero/below
  - Select first candidate that zeros the random value
- Handle edge case: no candidates available (enter Pause mode)

### 21. Automatic queue replenishment
- Monitor queue: maintain 2+ passages and 15+ minutes play time
- Trigger selection when threshold not met
- Use current timeslot flavor target (or temporary override)
- Account for anticipated play start time when selecting
- Background thread for async selection

## Phase 6: User Feedback & Refinement (Weeks 17-18)

### 22. Like/Dislike functionality
- Record likes/dislikes with timestamps
- **Effect on selection** (*specification needed*):
  - How likes affect base probability
  - How dislikes affect base probability or cooldowns
  - Passage-level vs song-level vs artist-level
  - Cumulative vs idempotent behavior
- UI buttons (already in design)
- REST API endpoints (already defined)

### 23. ListenBrainz integration
- **Specification needed for**:
  - Outbound data (plays, likes/dislikes, skips, duration)
  - Inbound data (recommendations, taste profile)
  - Effect on selection algorithm
  - Sync timing and authentication
- Implement based on finalized specification
- Network retry logic (5s delay, 20 max retries)
- Offline mode handling

### 24. Lyrics functionality
- Storage in passage table (UTF-8 text)
- Display in UI with playback
- Split-window editor:
  - Left pane: text input
  - Right pane: web search panel for copy-paste
- PUT endpoint for updates (already defined)
- Concurrent edit handling (last write wins per requirements)

## Phase 7: Platform Support & Versions (Weeks 19-21)

### 25. Platform-specific startup
- **Linux**: systemd service unit file
- **Windows**: Task Scheduler XML config
- **macOS**: launchd plist file
- Auto-start configuration UI in settings
- Service installation/uninstallation helpers

### 26. Audio sink selection & output
- GStreamer sink auto-detection and enumeration
- Manual override UI
- Platform-specific defaults:
  - Linux: PulseAudio (fallback to ALSA)
  - Windows: WASAPI
  - macOS: CoreAudio
- Bluetooth/HDMI output support testing
- Output device switching without playback interruption

### 27. Version builds (Full/Lite/Minimal)

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

## Phase 8: Polish & Optimization (Weeks 22-24)

### 28. Raspberry Pi Zero2W optimization
- Memory usage profiling and reduction
- GStreamer pipeline optimization:
  - Buffer sizes tuned for latency vs reliability
  - Dual-pipeline decode optimization (especially FLAC crossfades)
- Startup time reduction
- Thermal testing during extended playback
- SD card I/O optimization

### 29. Error handling improvements
- Network retry logic verification (5s delay, 20 max)
- UI notifications for network issues
- Graceful degradation testing:
  - Offline operation with missing external data
  - Missing files (remove from queue, continue)
  - Database errors (retry once, log, continue)
- Developer logging with tracing crate:
  - File and line number identification
  - Configurable log levels
  - Rotation and size limits

### 30. UI/UX refinements
- Queue display showing next 5 passages
- Progress bar smoothness (update every 500ms without jank)
- Responsive design for phone browsers
- Touch-friendly controls for mobile
- Keyboard shortcuts for desktop
- Loading states and spinners
- Error message display to users
- Edge case testing:
  - Concurrent users (multiple browser sessions)
  - Rapid skip clicks
  - Queue operations during playback transitions

### 31. Comprehensive testing
- **Unit tests**:
  - Cooldown calculation edge cases
  - Probability multiplication (including multi-song/artist when specified)
  - Distance formula with known inputs/outputs
  - Timeslot boundary handling
- **Integration tests**:
  - GStreamer pipeline lifecycle
  - Database migrations
  - SSE broadcasting
- **End-to-end tests**:
  - REST API full workflows
  - User scenarios (startup → play → queue → shutdown)
- **Performance tests**:
  - Selection algorithm with large libraries (10k+ passages)
  - Concurrent client connections
  - Memory leaks during extended operation
- **Platform tests**:
  - Raspberry Pi Zero2W (Lite/Minimal)
  - Linux desktop (Full)
  - Windows, macOS (Full)

## Optional/Future Enhancements

### 32. Advanced visualizations
- Waveform display in passage editor
- Musical flavor visualization (radar chart, scatter plot)
- Play history graphs and statistics

### 33. Advanced features
- Import/export preferences and probability settings
- Playlist support (manual track sequences, temporary playlists)
- Smart shuffle modes
- Social features (share flavors, import others' timeslot configs)

### 34. Mobile platforms
- Android/iOS versions using Flutter
- Native mobile UI
- Offline-first architecture
- Background playback support

## Critical Path Dependencies

**Blockers for Phase 5 (Selection System)**:
- Must clarify multi-song/multi-artist passage probability calculation
- Must define or remove Work cooldowns
- Must define featured vs primary artist distinction

**Blockers for Phase 6 (User Feedback)**:
- Must specify Like/Dislike effect on selection
- Must specify ListenBrainz integration details

**Recommended Implementation Approach**:
1. Implement phases sequentially to validate each layer
2. Deploy to Raspberry Pi Zero2W for testing at end of each phase
3. Create feature flags to disable incomplete features in early releases
4. Document missing specifications as encountered, seek clarification before proceeding
