 ## Architecture

  ### System Layers

  **1. Presentation Layer (Web UI)**
  - Tauri-based desktop application serving web interface
  - HTML/CSS/JavaScript frontend
  - Server-Sent Events (SSE) for real-time state synchronization
  - Local HTTP server on port 5720

  **2. API Layer**
  - REST endpoints for user commands (play, pause, skip, volume, queue operations)
  - SSE endpoint for pushing state updates to all connected clients
  - Request validation and command queuing for edge case handling

  **3. Business Logic Layer**
  - **Track Selector**: Probability-based track selection using cooldown algorithms
  - **Queue Manager**: Maintains playback queue with auto-replenishment logic
  - **Playback Controller**: Coordinates crossfade timing and track transitions
  - **History Tracker**: Records play events, likes/dislikes

  **4. Audio Engine Layer**
  - **Pipeline Manager**: Manages dual GStreamer pipelines for crossfading
    - Primary pipeline: Currently playing track
    - Secondary pipeline: Pre-loaded next track for seamless crossfade
  - **Volume Controller**: Implements fade profiles (exponential, cosine, linear)
  - **Audio Mixer**: Blends overlapping tracks using GStreamer `audiomixer`
  - **Format Handler**: Automatic codec detection, sample rate/format conversion

  **5. Library Management Layer**
  - **File Scanner**: Recursive directory scanning with change detection
  - **Metadata Extractor**: Tag parsing from audio files
  - **Fingerprint Generator**: Chromaprint/AcoustID integration
  - **Multi-track Editor**: Track boundary and crossfade point management

  **6. External Integration Layer**
  - **MusicBrainz Client**: Track/artist/release identification (with local cache)
  - **AcousticBrainz Client**: Musical character analysis (with fallback to local processing)
  - **ListenBrainz Client**: Listening history synchronization
  - Rate limiting and offline fallback handling

  **7. Data Layer**
  - SQLite database with connection pooling
  - Schema versioning and migration support
  - Local caching of external API responses

  ### Core Components

  **Audio Playback Engine**
  ┌─────────────────────────────────────────┐
  │   Playback Controller                   │
  │  ┌─────────────┐     ┌─────────────┐    │
  │  │  Pipeline A │────▶│             │    │
  │  │  (Current)  │     │ Audio Mixer │───▶ OS Audio Output
  │  └─────────────┘     │             │    │
  │  ┌─────────────┐     │             │    │
  │  │  Pipeline B │────▶│             │    │
  │  │  (Next)     │     └─────────────┘    │
  │  └─────────────┘                        │
  │         │                               │
  │    Volume Controller                    │
  │    (Fade Automation)                    │
  └─────────────────────────────────────────┘

  **Event Flow**
  User Input ──▶ REST API ──▶ Command Handler ──▶ State Update
                                                        │
                                                        ▼
                                                ┌──────────────┐
                                                │   Database   │
                                                └──────────────┘
                                                        │
                                                        ▼
                                                State Broadcast
                                                        │
                                                        ▼
                                            SSE ──▶ All Connected Clients

  ### Concurrency Model

  - **Main Thread**: Tauri event loop and UI coordination
  - **Audio Thread**: GStreamer pipeline execution (isolated from blocking I/O)
  - **Selector Thread**: Async background track selection when queue needs replenishment
  - **Scanner Thread**: Async file system scanning and metadata extraction
  - **API Thread Pool**: Async HTTP clients for external services

  **Inter-component Communication**:
  - `tokio` channels for async message passing between threads
  - Shared state protected by `Arc<RwLock<T>>` for playback state
  - GStreamer bus for pipeline events
  - SSE broadcaster for client updates

  ### Version Differentiation

  **Full Version**:
  - All components enabled
  - External API integration active
  - Library scanning and database building

  **Lite Version**:
  - Library scanning and preference editing enabled
  - Uses pre-built static database (read-only external data)
  - Reduced API calls (preference sync only)

  **Minimal Version**:
  - Player and selector only
  - Database and preferences are read-only
  - No library management or external API calls
  - Smallest memory footprint for Raspberry Pi Zero2W

  ### Offline Operation

  - All MusicBrainz/AcousticBrainz data cached locally in SQLite
    - cache lifetime is indefinite, infinite when sufficient storage is available
  - Automatic fallback to local AcousticBrainz algorithms when network unavailable
  - Queue continues from local library if network down
  - Graceful degradation: core playback unaffected by network status

  ### Platform Abstraction

  - GStreamer audio sink auto-detection (ALSA/PulseAudio/CoreAudio/WASAPI)
  - Tauri handles OS-specific system tray and startup integration
  - SQLite provides cross-platform persistence
  - Conditional compilation for platform-specific features
    (systemd/launchd/Task Scheduler)

