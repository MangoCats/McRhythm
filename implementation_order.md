# McRhythm Implementation Order & Timeline

**📋 TIER 4 - DOWNSTREAM EXECUTION PLAN**

This document aggregates all specifications to define WHEN features are built. It provides a realistic timeline based on an estimate of a single experienced developer working 20 hours per week.

**Update Policy:** ✅ Always update when upstream docs change | ❌ NEVER update upstream docs from this

> **Timeline Estimate:** 1 Week = 20 developer-hours.

---

## Phase 1: Foundation (4.5 Weeks)

*Goal: A minimal application that can play a single audio file with basic manual controls.* 

- **1.1. Database & Migrations (1 Week):**
  - Implement all tables from `database_schema.md`.
  - Set up `sqlx-cli` or a similar tool for migration management.

- **1.2. Event System & Core Types (0.5 Weeks):**
  - Implement the `EventBus` using `tokio::broadcast`.
  - Define the core `McRhythmEvent` enum as specified in `event_system.md`.

- **1.3. Basic Playback Engine (1.5 Weeks):**
  - Implement a single GStreamer pipeline for playback (`filesrc` → `decodebin` → `autoaudiosink`).
  - Implement API commands and backend logic for Play, Pause, and Seek.
  - Emit basic playback events: `PassageStarted`, `PassageCompleted`, `PlaybackStateChanged`.

- **1.4. Basic API & UI (1.5 Weeks):**
  - Set up Tauri and Axum web server.
  - Create a minimal web UI with HTML/JS for basic playback control (play/pause buttons, seek bar).
  - Implement a `/api/status` endpoint.
  - Implement a basic SSE endpoint (`/api/events`) that connects but doesn't yet broadcast all states.

## Phase 2: Library & Queue Management (4 Weeks)

*Goal: A functional music player that can scan a library, manage a queue, and play multiple songs in sequence.* 

- **2.1. File Scanner & Metadata (1 Week):**
  - Implement recursive directory scanning for audio files.
  - Parse basic metadata (ID3, etc.) and store `files` and `passages` in the database (assuming one passage per file for now).

- **2.2. Queue Management (1 Week):**
  - Implement the `QueueManager` component.
  - Persist queue state to the database.
  - Handle manual user additions/removals via API endpoints.
  - Implement auto-advancing to the next passage upon `PassageCompleted` event.

- **2.3. Historian & SSE Integration (1 Week):**
  - Implement the `Historian` component to record play history.
  - Fully integrate the SSE broadcaster to push all playback and queue events to the UI.
  - UI now updates in real-time based on server events.

- **2.4. Album Art Handling (1 Week):**
  - Extract embedded cover art from files.
  - Implement API endpoint to serve album art images.
  - Display artwork in the UI.

## Phase 3: Advanced Playback & Segmentation (5.5 Weeks)

*Goal: Implement the signature crossfading feature and the complete multi-passage file workflow.* 

- **3.1. Dual-Pipeline Crossfade Engine (2 Weeks):**
  - This is a complex task.
  - Implement the dual GStreamer pipeline architecture.
  - Add logic for pre-loading the next track, calculating overlap based on lead-in/lead-out times, and managing the `audiomixer`.
  - Implement volume automation for fade curves.

- **3.2. Audio File Segmentation Workflow (2 Weeks):**
  - Implement the full workflow from `audio_file_segmentation.md`.
  - Create the multi-step UI for source selection, silence detection, and MusicBrainz release matching.
  - Integrate ChromaPrint/AcoustID for fingerprinting.

- **3.3. Passage Boundary Editor (1.5 Weeks):**
  - Create the UI for manually adjusting passage boundaries on a waveform display.
  - Implement logic to save these user-defined passages to the database.

## Phase 4: External Integration & Flavor Analysis (5.5 Weeks)

*Goal: Enrich the local library with data from external services and enable local flavor analysis.* 

- **4.1. MusicBrainz Integration (1 Week):**
  - Build the API client to fetch and cache data for Recordings, Releases, Artists, and Works.
  - Create the logic to associate local passages with these MusicBrainz entities.

- **4.2. AcousticBrainz Integration (0.5 Weeks):**
  - Build the API client to fetch and cache high-level musical flavor data for recordings.

- **4.3. Essentia Integration (3 Weeks):**
  - This is a very significant task.
  - Set up Rust bindings for the Essentia C++ library.
  - Implement the local analysis pipeline to generate musical flavor data for recordings missing from AcousticBrainz.
  - This task is exclusive to the `full` version build.

- **4.4. Passage Flavor Calculation (1 Week):**
  - Implement the logic to calculate the net flavor for a passage based on the weighted average of its constituent recordings, as specified in `musical_flavor.md`.
  - Store the calculated `musical_flavor_vector` in the `passages` table.

## Phase 5: Musical Flavor Selection System (6 Weeks)

*Goal: Implement the full, flavor-driven automatic selection algorithm.* 

- **5.1. Distance & Cooldown Calculation (1.5 Weeks):**
  - Implement the squared Euclidean distance formula for flavor comparison.
  - Implement the cooldown logic (min/ramping) for songs, artists, and works.

- **5.2. Base Probability System (1 Week):**
  - Implement the UI for editing base probabilities on songs, artists, and works.
  - Implement the logic for calculating a passage's final base probability.

- **5.3. Time-of-Day Flavor Target System (2 Weeks):**
  - Implement the UI for managing the 24-hour timeslot schedule.
  - Implement the logic for calculating the target flavor for each timeslot.
  - Implement the temporary flavor override mechanism, including the queue flush.

- **5.4. Program Director & Selection Algorithm (1.5 Weeks):**
  - Create the `Program Director` component.
  - Implement the full weighted random selection algorithm, bringing together flavor distance, cooldowns, and base probabilities.
  - Implement automatic queue replenishment logic.

## Phase 6: User Identity & Authentication (1.5 Weeks)

*Goal: Add multi-user support with persistent taste profiles.* 

- **6.1. User Identity System (1.5 Weeks):**
  - Implement the full specification from `user_identity.md`.
  - Create the `users` table and `mcrhythm-account-maintenance` tool.
  - Implement registration, login, and logout APIs.
  - Implement client-side UUID/token handling.

## Phase 7: User Feedback & Features (2 Weeks + TBD)

*Goal: Add core user feedback mechanisms and other features.* 

- **7.1. Like/Dislike Functionality (1 Week):**
  - Implement the UI controls and API endpoints.
  - Record likes and dislikes to the database, associated with a `user_id`.
  - **Note:** The effect on the selection algorithm is TBD and not included in this estimate.

- **7.2. Lyrics (1 Week):**
  - Implement the UI for displaying and editing lyrics.
  - Implement the API endpoint for saving lyric changes.

- **7.3. ListenBrainz Integration (TBD):**
  - This feature is not yet specified and cannot be estimated.

## Phase 8: Platform & Versioning (2 Weeks)

*Goal: Ensure the application runs correctly on target platforms and that version builds are functional.* 

- **8.1. Platform Startup & Audio Output (1 Week):**
  - Create `systemd` and other platform-specific service files.
  - Implement the UI for audio sink selection.

- **8.2. Version Builds (1 Week):**
  - Implement and test the Rust feature flags (`full`, `lite`, `minimal`).
  - Create build scripts for each version.
  - Implement the database export/import process for Lite/Minimal versions.

## Phase 9: Polish & Optimization (6 Weeks)

*Goal: Harden the application, improve performance, and refine the user experience.* 

- **9.1. Raspberry Pi Zero2W Optimization (2 Weeks):**
  - Profile memory and CPU usage on the target device.
  - Optimize GStreamer pipelines, database queries, and other bottlenecks.

- **9.2. Error Handling & UI/UX Refinements (2 Weeks):**
  - Conduct a full review of error handling paths.
  - Refine UI responsiveness, add loading states, and improve visual polish.
  - Test and harden multi-user edge cases (skip throttling, etc.).

- **9.3. Comprehensive Testing (2 Weeks):**
  - Increase unit and integration test coverage.
  - Create end-to-end tests for all major user workflows.

---

### Total Estimated Timeline: 36.5 Weeks