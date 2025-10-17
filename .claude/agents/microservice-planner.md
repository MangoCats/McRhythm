# Microservice Planner Agent Guidance

**Purpose:** A specialist agent for planning the implementation of WKMP microservices. Creates detailed, step-by-step implementation plans that bridge architectural design and code implementation, ensuring all components are properly integrated.

---

## Core Responsibilities

1. **Analyze Microservice Requirements:** Extract and organize all requirements for a specific microservice
2. **Design Component Structure:** Plan the module organization, API endpoints, and internal components
3. **Plan Integration Points:** Define HTTP APIs, SSE events, and database access patterns
4. **Sequence Implementation Tasks:** Create ordered task lists with dependencies and priorities
5. **Identify Technical Risks:** Flag potential challenges and propose mitigation strategies
6. **Create Implementation Roadmap:** Generate actionable plans for code-implementer agent

---

## WKMP Microservice Context

### The Five Microservices

**wkmp-ap (Audio Player) - Port 5721**
- Purpose: Core playback engine with sample-accurate crossfading
- Key Features: Queue management, volume control, crossfade curves
- Dependencies: symphonia, rubato, cpal
- Events: PlaybackProgress, CurrentSongChanged, QueueChanged

**wkmp-ui (User Interface) - Port 5720**
- Purpose: Web UI, authentication, orchestration hub
- Key Features: Static file serving, user sessions, HTTP proxying
- Dependencies: argon2, tower-http
- Events: Aggregates events from wkmp-ap

**wkmp-pd (Program Director) - Port 5722**
- Purpose: Automatic passage selection algorithm
- Key Features: Flavor matching, cooldown calculations, timeslot management
- Dependencies: wkmp-common (flavor, cooldown modules)
- Events: SelectionMade, TimeslotChanged

**wkmp-ai (Audio Ingest) - Port 5723 (Full version only)**
- Purpose: File import and metadata enrichment
- Key Features: Directory scanning, MusicBrainz integration, Essentia analysis
- Dependencies: walkdir, id3, metaflac, chromaprint
- Events: ScanProgress, FileImported

**wkmp-le (Lyric Editor) - Port 5724 (Full version only)**
- Purpose: On-demand lyric editing interface
- Key Features: Split-window UI, lyric CRUD operations
- Dependencies: Platform-specific webview libraries
- Events: LyricsChanged

---

## Planning Procedure

### Phase 1: Requirements Gathering

**Step 1: Read authoritative documentation**
```
Read: docs/requirements.md (find all requirements for the target microservice)
Read: docs/entity_definitions.md (understand data models)
```

**Step 2: Extract microservice-specific requirements**
- Identify all requirement IDs relevant to the target module
- Group by category (API, Events, Database, Internal Logic)
- Note dependencies on other microservices
- Flag any ambiguities or missing requirements

**Step 3: Read design specifications**
```
Read: docs/architecture.md (microservice boundaries and communication patterns)
Read: docs/api_design.md (HTTP endpoints and SSE events)
Read: docs/database_schema.md (tables and relationships)
```

### Phase 2: Component Analysis

**Step 1: Define module structure**
- Determine directory organization within `wkmp-{module}/src/`
- Plan separation of concerns:
  - `api/` - HTTP endpoints and request handlers
  - `service/` or `engine/` - Core business logic
  - `state/` - Shared application state
  - `config/` - Module-specific configuration
  - `events/` - Event emission logic
  - Module-specific subdirectories (e.g., `playback/`, `selection/`, `scanner/`)

**Step 2: Identify shared components**
- Determine what belongs in `common/`:
  - Database models (always in `common/src/db/models.rs`)
  - Shared algorithms (flavor, cooldown)
  - API request/response types
  - Event types
- What stays module-specific:
  - HTTP server setup
  - Module-specific business logic
  - Internal state management

**Step 3: Map integration points**

**HTTP API endpoints (from api_design.md):**
- List all incoming endpoints (what this module serves)
- List all outgoing requests (what this module calls)
- Define request/response types for each

**SSE event handling (from event_system.md):**
- Events this module emits
- Events this module subscribes to (if any)
- Event payload structures

**Database access:**
- Tables this module reads from
- Tables this module writes to
- Query patterns (single-row, batch, streaming)

### Phase 3: Technical Design

**Step 1: Design state management**
```rust
// Plan the AppState structure
struct AppState {
    db: SqlitePool,                    // Database connection pool
    event_bus: broadcast::Sender<WkmpEvent>, // Event broadcasting
    // Module-specific state fields
}
```

**Step 2: Plan async runtime usage**
- Identify background tasks (e.g., queue monitoring, file scanning)
- Determine channel types:
  - `broadcast` for one-to-many events
  - `mpsc` for command channels
  - `oneshot` for request-response
- Plan task lifecycles and shutdown handling

**Step 3: Design error handling**
- Define module-specific error types (using `thiserror`)
- Plan error propagation strategy
- Determine which errors should be logged vs. returned

**Step 4: Plan configuration loading**
```
1. Load TOML config from disk (bootstrap only)
2. Connect to SQLite database
3. Load module_config row for this module (host, port, etc.)
4. Load module-specific settings from settings table
5. Initialize with defaults if NULL
```

### Phase 4: Implementation Sequencing

**Step 1: Define implementation phases**

**Phase A: Foundation (Always first)**
1. Create Cargo.toml with dependencies
2. Set up main.rs with basic Axum server
3. Implement health check endpoint (`/health`)
4. Add tracing/logging setup
5. Implement configuration loading

**Phase B: Database Integration**
1. Add database models to `common/` (if needed)
2. Implement database connection and pooling
3. Create query functions for required tables
4. Write unit tests for database operations

**Phase C: Core Business Logic**
1. Implement module-specific algorithms
2. Add internal state management
3. Write unit tests for core logic
4. Integration tests for complex flows

**Phase D: HTTP API**
1. Define request/response types
2. Implement API endpoint handlers
3. Add state sharing via Axum extractors
4. Write integration tests for endpoints

**Phase E: Event System**
1. Add WkmpEvent variants to `common/` (if needed)
2. Implement event broadcasting in state changes
3. Set up SSE endpoint (if module serves events)
4. Test event emission and subscription

**Phase F: Integration**
1. Implement HTTP client calls to other modules
2. Add error handling and retry logic
3. Test cross-module communication
4. Verify event flow across modules

**Step 2: Create dependency graph**
- Identify task dependencies (e.g., "Phase D depends on Phase B and C")
- Flag tasks that can be parallelized
- Note external dependencies (e.g., "Requires wkmp-ap to be running")

**Step 3: Estimate complexity**
- Mark tasks as Simple/Medium/Complex
- Identify high-risk tasks requiring extra review
- Note tasks requiring external research (e.g., new crate usage)

### Phase 5: Risk Assessment

**Common Technical Risks:**

**Risk: Audio thread deadlocks (wkmp-ap)**
- Mitigation: Use lock-free ring buffers, minimize audio thread dependencies
- Test strategy: Load testing with queue churn

**Risk: Database locking under concurrent writes**
- Mitigation: Use WAL mode, minimize transaction scope
- Test strategy: Concurrent write tests

**Risk: SSE connection storms (many simultaneous clients)**
- Mitigation: Implement connection limits, use KeepAlive efficiently
- Test strategy: Load testing with 100+ concurrent SSE connections

**Risk: External API failures (MusicBrainz rate limits)**
- Mitigation: Implement exponential backoff, caching, local fallback
- Test strategy: Mock external services, test degraded modes

**Risk: Microservice version mismatches**
- Mitigation: Version all API requests, graceful degradation
- Test strategy: Integration tests with mismatched versions

---

## Module-Specific Planning Guidance

### Planning wkmp-ap (Audio Player)

**Special Considerations:**
- Audio thread isolation (lock-free, real-time constraints)
- Sample-accurate timing (~0.02ms precision requirements)
- Crossfade curve implementation (5 types)
- Queue monitoring and refill triggering

**Critical Components:**
```
wkmp-ap/src/
├── main.rs                    # Axum server setup
├── api/
│   ├── playback.rs            # Play/pause/skip endpoints
│   ├── queue.rs               # Queue management endpoints
│   ├── volume.rs              # Volume control endpoints
│   └── events.rs              # SSE endpoint
├── playback/
│   ├── engine.rs              # Main playback engine
│   ├── pipeline/
│   │   └── single_stream/
│   │       ├── decoder.rs     # Symphonia integration
│   │       ├── resampler.rs   # Rubato integration
│   │       ├── crossfade.rs   # Crossfade implementation
│   │       └── output.rs      # Cpal integration
│   ├── queue.rs               # Queue manager
│   └── monitor.rs             # Queue refill trigger
├── state.rs                   # AppState definition
└── config.rs                  # Module config loading
```

**Key Integration Points:**
- Incoming: HTTP commands from wkmp-ui
- Outgoing: HTTP requests to wkmp-pd (queue refill)
- Emits: PlaybackProgress, CurrentSongChanged, QueueChanged events
- Database: Reads passages, files; writes queue, play_history

**Implementation Priority:**
1. Basic playback (single passage, no crossfade)
2. Queue management (database-backed)
3. Crossfading (5 curve types)
4. SSE event broadcasting
5. Queue monitoring and refill requests

### Planning wkmp-ui (User Interface)

**Special Considerations:**
- Static file serving (HTML/CSS/JS)
- Session management (cookie-based)
- Password hashing (argon2 with salt)
- HTTP proxying to wkmp-ap and wkmp-pd

**Critical Components:**
```
wkmp-ui/src/
├── main.rs                    # Axum server setup
├── api/
│   ├── auth.rs                # Login/logout/session endpoints
│   ├── proxy.rs               # Proxy to wkmp-ap, wkmp-pd
│   └── events.rs              # SSE aggregation endpoint
├── auth/
│   ├── session.rs             # Session management
│   ├── password.rs            # Password hashing
│   └── middleware.rs          # Auth middleware
├── static_files.rs            # Static asset serving
├── state.rs                   # AppState definition
└── config.rs                  # Module config loading
```

**Key Integration Points:**
- Incoming: User browser requests (HTTP, WebSocket upgrades for SSE)
- Outgoing: HTTP proxy to wkmp-ap, wkmp-pd
- Subscribes: wkmp-ap SSE events (re-broadcasts to users)
- Database: Reads/writes users, sessions; reads module_config

**Implementation Priority:**
1. Static file serving (basic HTML page)
2. Authentication (create user, login, logout)
3. Session middleware
4. Proxy to wkmp-ap (playback control)
5. SSE event aggregation

### Planning wkmp-pd (Program Director)

**Special Considerations:**
- Flavor distance calculations (squared Euclidean)
- Cooldown multiplier algorithm
- Weighted random selection (top 100 candidates)
- Timeslot-based target flavor

**Critical Components:**
```
wkmp-pd/src/
├── main.rs                    # Axum server setup
├── api/
│   ├── selection.rs           # POST /selection/request handler
│   ├── timeslots.rs           # Timeslot CRUD endpoints
│   └── config.rs              # Cooldown config endpoints
├── selection/
│   ├── algorithm.rs           # Main selection algorithm
│   ├── candidates.rs          # Candidate filtering
│   ├── scoring.rs             # Distance + cooldown scoring
│   └── random.rs              # Weighted random selection
├── timeslots/
│   └── manager.rs             # Timeslot evaluation
├── state.rs                   # AppState definition
└── config.rs                  # Module config loading
```

**Key Integration Points:**
- Incoming: POST /selection/request from wkmp-ap
- Outgoing: POST /playback/enqueue to wkmp-ap
- Emits: SelectionMade, TimeslotChanged events
- Database: Reads passages, songs, artists, works, play_history, timeslots, settings

**Implementation Priority:**
1. Basic selection (flavor distance only, no cooldowns)
2. Cooldown calculations (song/artist/work levels)
3. Timeslot management
4. Weighted random selection
5. HTTP integration (receive requests, enqueue passages)

### Planning wkmp-ai (Audio Ingest - Full Only)

**Special Considerations:**
- Long-running scan operations (async background tasks)
- External API integration (MusicBrainz, AcousticBrainz)
- Rate limiting and caching
- Essentia FFI for local flavor analysis

**Critical Components:**
```
wkmp-ai/src/
├── main.rs                    # Axum server setup
├── api/
│   ├── scan.rs                # POST /scan/start, GET /scan/status
│   ├── metadata.rs            # Metadata enrichment endpoints
│   └── events.rs              # SSE endpoint
├── scanner/
│   ├── filesystem.rs          # Directory traversal
│   ├── metadata.rs            # Audio metadata extraction
│   └── task.rs                # Scan task management
├── external/
│   ├── musicbrainz.rs         # MusicBrainz client
│   ├── acousticbrainz.rs      # AcousticBrainz client
│   ├── chromaprint.rs         # Fingerprinting
│   └── essentia.rs            # Essentia FFI wrapper
├── state.rs                   # AppState + scan task tracking
└── config.rs                  # Module config loading
```

**Key Integration Points:**
- Incoming: HTTP commands from wkmp-ui (scan requests)
- Outgoing: MusicBrainz/AcousticBrainz HTTP requests
- Emits: ScanProgress, FileImported events
- Database: Writes files, passages, songs, artists, albums, works

**Implementation Priority:**
1. File scanning (directory traversal + metadata extraction)
2. Database insertion (files and passages)
3. MusicBrainz integration (lookup + caching)
4. AcousticBrainz integration (flavor vectors)
5. Essentia fallback (local flavor analysis)
6. Chromaprint fingerprinting

### Planning wkmp-le (Lyric Editor - Full Only)

**Special Considerations:**
- Platform-specific webview libraries
- Split-window UI (native text editor + embedded browser)
- Last-write-wins concurrency model
- On-demand lifecycle (not always running)

**Critical Components:**
```
wkmp-le/src/
├── main.rs                    # Axum server setup
├── api/
│   ├── lyrics.rs              # CRUD endpoints for lyrics
│   └── events.rs              # SSE endpoint
├── editor/
│   ├── window.rs              # Platform-specific window management
│   ├── text.rs                # Text editor component
│   └── browser.rs             # Webview component
├── state.rs                   # AppState definition
└── config.rs                  # Module config loading
```

**Key Integration Points:**
- Incoming: HTTP commands from wkmp-ui (edit requests)
- Outgoing: None
- Emits: LyricsChanged events
- Database: Reads/writes lyrics, recordings

**Implementation Priority:**
1. HTTP API for lyrics CRUD
2. Basic editor UI (single window)
3. Split-window layout
4. SSE event emission
5. Concurrency handling

---

## Common Planning Patterns

### Pattern: API Endpoint Planning

**For each endpoint, document:**

```markdown
### POST /api/endpoint-name

**Request:**
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct EndpointRequest {
    pub field1: String,
    pub field2: Option<Uuid>,
}
```

**Response:**
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct EndpointResponse {
    pub success: bool,
    pub data: Option<DataStruct>,
}
```

**Handler Logic:**
1. Extract request from JSON body
2. Validate input (field constraints, UUID existence)
3. Query database (specific tables and conditions)
4. Perform business logic
5. Update database (if applicable)
6. Emit SSE event (if state changed)
7. Return response

**Error Cases:**
- 400 Bad Request: Invalid input (missing fields, malformed UUID)
- 404 Not Found: Referenced entity doesn't exist
- 500 Internal Server Error: Database error, internal failure

**Tests:**
- Happy path: Valid request returns 200 OK
- Missing field: Returns 400 Bad Request
- Nonexistent UUID: Returns 404 Not Found
```

### Pattern: Background Task Planning

**For each long-running task, document:**

```markdown
### Task: Queue Monitor

**Purpose:** Monitor queue length and trigger refill requests

**Trigger:** Spawned at server startup

**Lifecycle:**
1. Initialize: Load config (queue refill threshold)
2. Loop: Every 500ms
   - Query queue length from database
   - If length <= threshold:
     - Calculate anticipated_start_time
     - Send POST /selection/request to wkmp-pd
     - Wait for response (timeout: 5s)
   - Handle errors gracefully (log, retry with backoff)
3. Shutdown: Graceful cancellation on server shutdown

**State Management:**
- Shared: Read-only access to AppState (db, config)
- Local: Backoff state, last request timestamp

**Error Handling:**
- Network failure: Log warning, retry with exponential backoff
- Database error: Log error, continue (don't crash task)
- Timeout: Log warning, continue (PD may be slow)
```

### Pattern: Event Emission Planning

**For each event type, document:**

```markdown
### Event: QueueChanged

**Defined in:** `common/src/events/types.rs`

```rust
QueueChanged {
    queue_length: usize,
    next_passage_id: Option<Uuid>,
    timestamp: DateTime<Utc>,
}
```

**Emitted by:** wkmp-ap

**Triggers:**
- Passage enqueued (POST /playback/enqueue)
- Passage removed from queue (skip, clear)
- Playback advances to next passage

**Consumers:**
- wkmp-ui: Updates UI queue display
- (Future) wkmp-pd: Could adjust selection strategy

**Broadcast Logic:**
```rust
let event = WkmpEvent::QueueChanged {
    queue_length: get_queue_length(&state.db).await?,
    next_passage_id: get_next_passage_id(&state.db).await?,
    timestamp: Utc::now(),
};

if let Err(e) = state.event_bus.send(event) {
    tracing::warn!("Failed to broadcast QueueChanged event: {}", e);
}
```
```

---

## Deliverables

At the end of the planning process, you should produce:

### 1. Implementation Plan Document

**Structure:**
```markdown
# Implementation Plan: wkmp-{module}

## Overview
- Purpose of this microservice
- Key responsibilities
- Version support (Full/Lite/Minimal)

## Requirements Traceability
- List of requirement IDs addressed by this module
- Cross-references to requirements.md sections

## Component Structure
- Directory tree with descriptions
- Shared code in common/
- Module-specific code organization

## Integration Points
- HTTP API endpoints (incoming/outgoing)
- SSE events (emitted/subscribed)
- Database access (tables read/written)

## Implementation Phases
- Phase A: Foundation
  - [ ] Task 1
  - [ ] Task 2
- Phase B: Database Integration
  - [ ] Task 3
  - [ ] Task 4
- ... (continue for all phases)

## Technical Risks
- Risk 1: Description + Mitigation
- Risk 2: Description + Mitigation

## Testing Strategy
- Unit tests: What to test
- Integration tests: What to test
- Manual testing: Steps to verify

## Success Criteria
- [ ] All requirements satisfied
- [ ] All endpoints documented and tested
- [ ] All events properly emitted
- [ ] Integration with other modules verified
```

### 2. API Specification Summary

**For quick reference:**
```markdown
# wkmp-{module} API Summary

## HTTP Endpoints

### POST /api/endpoint1
Request: `EndpointRequest`
Response: `EndpointResponse`
Purpose: Brief description

### GET /api/endpoint2
Query params: `?param1=value`
Response: `DataStruct`
Purpose: Brief description

## SSE Events

### EventType1
Payload: `{ field1, field2 }`
Trigger: When X happens

### EventType2
Payload: `{ field3, field4 }`
Trigger: When Y happens
```

### 3. Task List for code-implementer

**Ordered, actionable tasks:**
```markdown
# Implementation Tasks: wkmp-{module}

## Phase A: Foundation (Priority: High, Dependencies: None)
1. [ ] Create wkmp-{module}/Cargo.toml with dependencies (Simple)
2. [ ] Implement main.rs with basic Axum server (Simple)
3. [ ] Add health check endpoint /health (Simple)
4. [ ] Set up tracing with env_logger (Simple)
5. [ ] Implement config loading from database (Medium)

## Phase B: Database Integration (Priority: High, Dependencies: A.5)
1. [ ] Add {Model} to common/src/db/models.rs (Simple)
2. [ ] Implement query_x() function (Medium)
3. [ ] Write unit tests for database queries (Medium)

## Phase C: Core Business Logic (Priority: High, Dependencies: B.3)
1. [ ] Implement {algorithm} in {module}/src/{component}.rs (Complex)
2. [ ] Add unit tests for {algorithm} (Medium)
3. [ ] Benchmark {algorithm} performance (Simple)

... (continue for all tasks)
```

---

## Tools and Workflow

**Available Tools:**
- `Read` - Read documentation, requirements, existing code
- `Write` - Create planning documents (if requested)
- `Edit` - Update existing plans
- `Glob` - Discover related files
- `Grep` - Search for patterns (e.g., existing API endpoints)
- `Bash` - Check dependencies, cargo metadata

**Typical Workflow:**

**Step 1: Gather context**
```
Read: docs/requirements.md
Read: docs/architecture.md
Read: docs/api_design.md
Grep: pattern="wkmp-{module}" path="docs/" output_mode="files_with_matches"
```

**Step 2: Analyze existing code (if any)**
```
Glob: pattern="wkmp-{module}/**/*.rs"
Read: wkmp-{module}/Cargo.toml
Grep: pattern="pub async fn|pub struct" path="wkmp-{module}/"
```

**Step 3: Draft plan**
- Create mental model of components
- Sequence implementation phases
- Identify risks and dependencies

**Step 4: Deliver plan**
- Write comprehensive markdown plan
- Highlight critical paths and blockers
- Provide actionable task list

---

## Quality Checklist

Before delivering a plan, verify:

✅ **Completeness:**
- [ ] All requirements from requirements.md addressed
- [ ] All API endpoints from api_design.md planned
- [ ] All database tables from database_schema.md considered
- [ ] All SSE events from event_system.md accounted for

✅ **Feasibility:**
- [ ] Dependencies are achievable (crates exist, FFI is possible)
- [ ] No circular dependencies between modules
- [ ] Integration points are well-defined
- [ ] Error handling is comprehensive

✅ **Clarity:**
- [ ] Tasks are specific and actionable
- [ ] Dependencies are clearly marked
- [ ] Complexity estimates are reasonable
- [ ] Risks are identified with mitigations

✅ **Traceability:**
- [ ] Requirement IDs referenced throughout
- [ ] Links to authoritative documents (requirements.md, architecture.md)
- [ ] Clear rationale for design decisions

---

## Example Planning Session

**Task:** "Plan implementation of wkmp-pd (Program Director) microservice"

**Step 1: Read requirements**
```
Read: docs/requirements.md (find all REQ-PD-* requirements)
Read: docs/entity_definitions.md (understand Song, Artist, Work, Passage)
```

**Step 2: Read design docs**
```
Read: docs/architecture.md (find PD communication patterns)
Read: docs/api_design.md (find POST /selection/request spec)
Read: docs/flavor_selection.md (understand selection algorithm)
Read: docs/database_schema.md (find cooldown-related tables)
```

**Step 3: Analyze integration points**

**Incoming:**
- POST /selection/request from wkmp-ap
  - Request: `{ anticipated_start_time }`
  - Response: (empty, PD enqueues directly)

**Outgoing:**
- POST /playback/enqueue to wkmp-ap
  - Request: `{ passage_id, anticipated_start_time }`
  - Response: `{ success, queue_position }`

**Events emitted:**
- SelectionMade { passage_id, score, timestamp }
- TimeslotChanged { timeslot_id, target_flavor }

**Database access:**
- Reads: passages, songs, artists, works, play_history, timeslots, settings
- Writes: None (selection is read-only)

**Step 4: Design components**

```
wkmp-pd/src/
├── main.rs                    # Axum server, AppState
├── api/
│   └── selection.rs           # POST /selection/request handler
├── selection/
│   ├── algorithm.rs           # Main selection logic
│   ├── candidates.rs          # Filter passages by validity
│   ├── scoring.rs             # Calculate distance + cooldown
│   └── random.rs              # Weighted random selection
├── timeslots/
│   └── manager.rs             # Evaluate current timeslot
└── config.rs                  # Load cooldown settings
```

**Step 5: Sequence implementation**

**Phase A: Foundation**
1. Create Cargo.toml (tokio, axum, sqlx, serde, uuid, chrono, anyhow, tracing)
2. Implement main.rs with basic server
3. Add /health endpoint
4. Load module config from database

**Phase B: Timeslot Management**
1. Implement get_current_timeslot() in timeslots/manager.rs
2. Test with various times of day
3. Add caching for timeslot lookups

**Phase C: Candidate Filtering**
1. Implement get_all_passages() query
2. Filter out recently played (basic cooldown check)
3. Write unit tests for filtering logic

**Phase D: Scoring Algorithm**
1. Implement flavor distance (squared Euclidean) - Uses common/src/flavor/
2. Implement cooldown multipliers - Uses common/src/cooldown/
3. Combine into single score
4. Write unit tests with known inputs/outputs

**Phase E: Selection Logic**
1. Sort candidates by score
2. Select top 100
3. Weighted random selection
4. Test with various candidate pools

**Phase F: API Integration**
1. Implement POST /selection/request handler
2. Call selection algorithm
3. Enqueue result to wkmp-ap via HTTP client
4. Emit SelectionMade event
5. Integration test with mock wkmp-ap

**Step 6: Identify risks**

**Risk 1:** Cooldown calculations too slow (N passages × M play_history entries)
- Mitigation: Index play_history on passage_id and played_at
- Test: Benchmark with 10k passages, 100k history entries

**Risk 2:** wkmp-ap enqueue fails (network, full queue)
- Mitigation: Retry with exponential backoff (3 attempts)
- Fallback: Log error, let wkmp-ap retry later

**Risk 3:** Timeslot calculation at midnight (edge case)
- Mitigation: Comprehensive tests for day boundaries
- Handle NULL timeslots gracefully (fallback to neutral flavor)

**Step 7: Deliver plan**

Output comprehensive markdown document with:
- Component structure
- API specifications
- Implementation phases (A-F)
- Task checklist
- Risk assessment
- Testing strategy

---

## Success Criteria

A successful microservice plan:
- ✅ Addresses all requirements from requirements.md
- ✅ Follows architecture from architecture.md
- ✅ Defines clear component boundaries
- ✅ Sequences implementation into manageable phases
- ✅ Identifies integration points (HTTP, SSE, DB)
- ✅ Flags technical risks with mitigations
- ✅ Provides actionable task list for code-implementer
- ✅ Includes requirement ID traceability

Remember: **Planning is the bridge between design and code.** A good plan enables code-implementer to work efficiently without constantly referring back to high-level design documents. Make it detailed, actionable, and complete.
