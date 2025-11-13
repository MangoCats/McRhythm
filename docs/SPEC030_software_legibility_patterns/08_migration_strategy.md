# Migration Strategy

**Part of:** [SPEC030 - Software Legibility Patterns](00_SUMMARY.md)

---

## 8.1 Incremental Adoption Approach

**Core Principle:** Gradual refactoring without "big bang" rewrites

### Migration Philosophy

FOR MODIFICATION, UPDATES AND IMPROVEMENTS OF EXISTING MICROSERVICES ONLY:

**Strangler Fig Pattern:**
- New legible concepts coexist with existing code
- Gradually migrate features from old to new structure
- No requirement to migrate everything at once
- Production stability maintained throughout

**Compatibility Strategy:**
- Legacy code continues functioning during migration
- Synchronization engine bridges old and new concepts
- Module-by-module rollout (not feature-by-feature)

FOR GROUND-UP REWRITES OF MICROSERVICES, IMPLEMENT FULL FEATURES FROM THE OUTSET.

---

## 8.2 Five-Phase Migration Plan

### Phase 1: Infrastructure Setup (Weeks 1-2)

**Deliverables:**
- Event bus infrastructure (`tokio::broadcast`)
- Action tracer with in-memory storage
- Basic synchronization engine (pattern matching, execution)
- Developer interface framework (Axum routes)
- Database parameter for dev interface control

**Implementation:**
```rust
// wkmp-common/src/legibility/mod.rs
pub mod event_bus;
pub mod action_tracer;
pub mod sync_engine;
pub mod dev_interface;

// Available to all modules:
use wkmp_common::legibility::*;
```

**Database Migration (add to wkmp-common migrations):**
```sql
-- Add developer interface control parameter
INSERT INTO settings (key, value_type, value_text, description, default_value)
VALUES (
    'enable_dev_interface',
    'boolean',
    CASE
        WHEN (SELECT value_text FROM build_info WHERE key = 'profile') = 'debug' THEN 'true'
        ELSE 'false'
    END,
    'Enable developer interface at /dev/ (requires microservice restart)',
    CASE
        WHEN (SELECT value_text FROM build_info WHERE key = 'profile') = 'debug' THEN 'true'
        ELSE 'false'
    END
);
```

**No existing code changed** - infrastructure only

### Phase 2: Concept Identification (Weeks 3-4)

**Deliverables:**
- Concept identification audit for each module
- Documentation of implicit concepts in existing code
- Dependency graph showing current coupling

**Process:**
1. Analyze existing code to identify functional boundaries
2. Map current functions/structs to concept candidates
3. Document dependencies between concepts
4. Prioritize concepts for extraction (lowest dependencies first)

**Example Output (wkmp-ap):**
```
Identified Concepts:
1. AudioPlayer (explicit) - playback state, queue
   Dependencies: CrossfadeEngine, DatabaseAccess
   Priority: High (core functionality)

2. CrossfadeEngine (explicit) - audio mixing
   Dependencies: AudioOutput
   Priority: High (independent)

3. VolumeControl (implicit) - currently scattered in AudioPlayer
   Dependencies: None
   Priority: Medium (easy extraction)

4. QueueManager (implicit) - currently part of AudioPlayer
   Dependencies: None
   Priority: Medium (easy extraction)
```

### Phase 3: Concept Extraction (Weeks 5-10)

**Deliverables:**
- Refactored concepts as independent modules
- Event emissions added to concept actions
- Queries extracted from concepts
- Unit tests for each concept

**Per-Concept Process:**

**Step 1: Create Concept Module**
```rust
// wkmp-ap/src/concepts/audio_player.rs
pub struct AudioPlayer {
    // Private state
    current_passage: Option<PassageId>,
    queue: VecDeque<PassageId>,
    // ...
}

impl Concept for AudioPlayer {
    fn concept_name() -> &'static str {
        "AudioPlayer"
    }
}
```

**Step 2: Extract Actions**
```rust
impl AudioPlayer {
    pub async fn play_passage(&mut self, passage_id: PassageId) -> Result<()> {
        // Move implementation from legacy code
        // Add event emission
        self.emit_event(Event::PlaybackStarted { passage_id });
        Ok(())
    }
}
```

**Step 3: Extract Queries**
```rust
impl AudioPlayer {
    pub fn get_current_passage(&self) -> Option<PassageId> {
        self.current_passage
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.playback_state, PlaybackState::Playing { .. })
    }
}
```

**Step 4: Update Call Sites**
```rust
// OLD:
let passage_id = legacy_play_passage(42)?;

// NEW:
let passage_id = audio_player.play_passage(PassageId::new(42)).await?;
```

**Step 5: Write Tests**
```rust
#[tokio::test]
async fn test_play_passage_emits_event() {
    let mut player = AudioPlayer::new_for_test();
    player.play_passage(PassageId::new(42)).await.unwrap();

    let events = player.drain_events();
    assert!(matches!(events[0], Event::PlaybackStarted { .. }));
}
```

### Phase 4: Synchronization Definition (Weeks 11-14)

**Deliverables:**
- Synchronization catalog for each module
- Event-driven orchestration replacing direct calls
- Action traces with provenance tracking

**Per-Module Process:**

**Step 1: Identify Concept Interactions**
```
Current direct calls:
- AudioPlayer → PassageSelection (for auto-selection)
- AudioPlayer → Cooldown (for recording plays)
- Web → Authentication → AudioPlayer (for playback)

Convert to synchronizations:
✓ AutoSelectWhenQueueLow
✓ RecordPlaybackForCooldown
✓ EnforceAuthenticationOnPlayback
```

**Step 2: Define Synchronizations**
```rust
// wkmp-ap/src/syncs/mod.rs
pub fn register_synchronizations(engine: &mut SyncEngine) {
    engine.register(Synchronization {
        name: "AutoSelectWhenQueueLow".to_string(),
        when: EventPattern::EventType {
            event_name: "QueueDepthChanged".to_string(),
            bindings: vec![("depth", Variable::Free)],
        },
        where_clause: StateCondition::Query {
            concept: "Settings".to_string(),
            query: "is_auto_play_enabled".to_string(),
            parameters: vec![],
            expectation: Expectation::Affirm,
        },
        then: vec![
            Action::Invoke {
                concept: "PassageSelection".to_string(),
                action: "select_next".to_string(),
                parameters: vec![],
            },
            Action::Invoke {
                concept: "AudioPlayer".to_string(),
                action: "enqueue".to_string(),
                parameters: vec![("passage_id", Variable::Bound)],
            },
        ],
    });
}
```

**Step 3: Replace Direct Calls**
```rust
// OLD:
impl AudioPlayer {
    async fn on_queue_low(&mut self) {
        let passage = self.selection.select_next().await?;  // Direct call
        self.enqueue(passage).await?;
    }
}

// NEW:
impl AudioPlayer {
    async fn on_queue_low(&mut self) {
        // Just emit event - synchronization handles orchestration
        self.emit_event(Event::QueueDepthChanged { depth: self.queue.len() });
    }
}
```

**Step 4: Enable Action Tracing**
```rust
impl AudioPlayer {
    pub async fn play_passage(&mut self, passage_id: PassageId) -> Result<()> {
        let action_id = self.tracer.record_action(
            self.concept_uri(),
            "play_passage".to_string(),
            json!({"passage_id": passage_id}),
            json!(null),
        );

        // Perform action...

        self.tracer.update_action_output(action_id, json!({"started_at": timestamp}));
        Ok(())
    }
}
```

### Phase 5: Developer Interface (Weeks 15-18)

**Deliverables:**
- HTTP endpoints for dev interface
- Dashboard, concept inspector, sync monitor, trace viewer
- Live event stream (SSE)
- Database parameter-controlled activation
- Documentation and user guide

**Implementation:**
```rust
use wkmp_common::config::GlobalParameters;

pub async fn create_app(pool: &SqlitePool) -> Result<Router> {
    // Load dev interface parameter
    let params = GlobalParameters::load(pool).await?;
    let dev_interface_enabled = params.get_bool("enable_dev_interface")?;

    let mut app = Router::new()
        .route("/api/play", post(handle_play))
        .route("/api/pause", post(handle_pause));
        // ... other API routes

    // Conditionally construct dev interface (not just gate)
    if dev_interface_enabled {
        tracing::info!("Developer interface ENABLED at /dev/");
        app = app.merge(dev_routes(state));
    } else {
        tracing::info!("Developer interface DISABLED");
        // Routes not constructed - zero overhead
    }

    Ok(app)
}

fn dev_routes(state: AppState) -> Router {
    Router::new()
        .route("/dev/", get(dashboard))
        .route("/dev/concepts/:name", get(inspect_concept))
        .route("/dev/syncs/:name", get(inspect_sync))
        .route("/dev/traces/:token", get(view_trace))
        .route("/dev/events/stream", get(event_stream))
        .with_state(state)
}
```

**Per-Microservice Configuration:**
Each module can independently enable/disable the dev interface:
```bash
# Enable for wkmp-ap only (production troubleshooting)
sqlite3 ~/Music/wkmp.db "UPDATE settings SET value_text = 'true' WHERE key = 'enable_dev_interface';"
systemctl restart wkmp-ap

# Other modules remain disabled (wkmp-ui, wkmp-pd, etc.)
```

---

## 8.3 Module Rollout Priority

### 1. wkmp-ap (Audio Player) - Weeks 3-8

**Rationale:**
- Cleanest boundaries (playback, queue, crossfade)
- High value for action tracing (debugging playback issues)
- Relatively isolated from other modules

**Concepts to Extract:**
1. AudioPlayer (core playback state)
2. CrossfadeEngine (audio mixing)
3. QueueManager (passage queue)
4. VolumeControl (volume/mute)

**Synchronizations:**
- AutoSelectWhenQueueLow
- RecordPlaybackForCooldown
- BroadcastPlaybackStateChange

### 2. wkmp-pd (Program Director) - Weeks 9-12

**Rationale:**
- Clear concepts (selection, cooldown, timeslot)
- Complex orchestration benefits from synchronizations
- No UI complexity

**Concepts to Extract:**
1. PassageSelection (flavor matching)
2. Cooldown (repetition prevention)
3. Timeslot (time-of-day scheduling)
4. FlavorCalculation (distance metrics)

**Synchronizations:**
- ApplyCooldownToSelection
- UpdateTimeslotBasedSelection

### 3. wkmp-ui (User Interface) - Weeks 13-16

**Rationale:**
- Benefits from explicit auth/SSE synchronizations
- High value for multi-user coordination
- Complex state management

**Concepts to Extract:**
1. Authentication (session management)
2. SSEBroadcaster (real-time updates)
3. WebRouter (HTTP request handling)
4. StaticAssets (asset serving)

**Synchronizations:**
- EnforceAuthenticationOnAPI
- BroadcastStateChangeToClients
- LogUserActions

### 4. wkmp-ai (Audio Ingest) - Weeks 17-20

**Rationale:**
- Complex workflow (import wizard)
- High legibility value (multi-step process)
- Lower priority (on-demand tool)

**Concepts to Extract:**
1. FileScanner (directory traversal)
2. MusicBrainz (recording identification)
3. PassageSegmentation (audio analysis)
4. ImportWizard (UI state machine)

**Synchronizations:**
- OnFileScanCompleteIdentifyRecordings
- OnIdentificationCompleteSegmentPassages
- OnSegmentationCompleteUpdateDatabase

### 5. wkmp-le (Lyric Editor) - Weeks 21-22

**Rationale:**
- Lower priority (specialized tool)
- Simpler architecture (mostly UI)

**Concepts to Extract:**
1. LyricEditor (text editing)
2. SyncTimingEditor (timestamp alignment)
3. WaveformDisplay (visualization)

### 6. wkmp-dr (Database Review) - Weeks 23-24

**Rationale:**
- Lowest priority (read-only tool)
- Simplest architecture

**Concepts to Extract:**
1. DatabaseInspector (read-only queries)
2. QueryBuilder (SQL generation)

---

## 8.4 Backward Compatibility

### Strategy 1: Adapter Pattern

**Purpose:** Legacy code calls concepts via adapters

```rust
// Legacy interface (unchanged):
pub fn legacy_play_passage(passage_id: i32) -> Result<(), String> {
    // Adapter to new concept:
    let player = get_audio_player();
    player.play_passage(PassageId::new(passage_id))
        .await
        .map_err(|e| e.to_string())
}
```

### Strategy 2: Dual Implementation

**Purpose:** New and old code coexist during migration

```rust
pub async fn play_passage(passage_id: PassageId) -> Result<()> {
    if cfg!(feature = "legible") {
        // New legible path:
        let player = get_audio_player();
        player.play_passage(passage_id).await
    } else {
        // Old legacy path:
        legacy_play_passage(passage_id.into())
    }
}
```

### Strategy 3: Progressive Enhancement

**Purpose:** Concepts added without removing legacy code

```rust
// Old code continues working:
pub fn old_play_passage(id: i32) -> Result<()> {
    // Original implementation
}

// New concept wraps old code temporarily:
impl AudioPlayer {
    pub async fn play_passage(&mut self, passage_id: PassageId) -> Result<()> {
        // Call legacy implementation:
        old_play_passage(passage_id.into())?;

        // Add legibility features:
        self.emit_event(Event::PlaybackStarted { passage_id });
        self.tracer.record_action(...);

        Ok(())
    }
}
```

---

## 8.5 Testing Strategy

### Unit Tests (Per Concept)

**Coverage Target:** 100% of concept actions and queries

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_player_play_passage() {
        let mut player = AudioPlayer::new_for_test();
        let result = player.play_passage(PassageId::new(42)).await;
        assert!(result.is_ok());
        assert_eq!(player.get_current_passage(), Some(PassageId::new(42)));
    }
}
```

### Synchronization Tests

**Coverage Target:** Each sync tested with expected events/state

```rust
#[tokio::test]
async fn test_auto_select_when_queue_low() {
    let mut engine = SyncEngine::new_for_test();
    let mut player = AudioPlayer::new_for_test();
    let mut selection = PassageSelection::new_for_test();

    // Register sync:
    engine.register(auto_select_when_queue_low_sync());

    // Trigger event:
    engine.emit_event(Event::QueueDepthChanged { depth: 1 });

    // Verify actions:
    engine.process_events().await;

    assert_eq!(player.get_queue_depth(), 2);  // New passage enqueued
}
```

### Integration Tests (Per Module)

**Coverage Target:** End-to-end flows through synchronizations

```rust
#[tokio::test]
async fn test_user_playback_request_flow() {
    let app = test_app().await;

    // Simulate user request:
    let response = app.post("/api/play")
        .json(&json!({"passage_id": 42, "token": "valid_token"}))
        .send()
        .await;

    assert_eq!(response.status(), 200);

    // Verify action trace:
    let trace = app.tracer.get_latest_flow().await;
    assert!(trace.contains_action("wkmp-ap::AudioPlayer::play_passage"));
    assert!(trace.contains_sync("EnforceAuthenticationOnPlayback"));
}
```

### Regression Tests

**Purpose:** Ensure migrated features behave identically to legacy

```rust
#[tokio::test]
async fn test_legacy_vs_legible_playback() {
    // Legacy implementation:
    let legacy_result = legacy_play_passage(42);

    // Legible implementation:
    let mut player = AudioPlayer::new();
    let legible_result = player.play_passage(PassageId::new(42)).await;

    // Verify equivalent behavior:
    assert_eq!(legacy_result.is_ok(), legible_result.is_ok());
}
```

---

## 8.6 Rollback Plan

### Criteria for Rollback

**Trigger rollback if:**
- Production defects introduced by migration
- Performance degradation >10%
- Developer velocity significantly impacted
- Unforeseen architectural conflicts

### Rollback Mechanisms

**Feature Flags:**
```rust
#[cfg(feature = "legible")]
fn use_legible_path() { /* new */ }

#[cfg(not(feature = "legible"))]
fn use_legacy_path() { /* old */ }
```

**Module Isolation:**
- Each module migrated independently
- Rollback single module without affecting others

**Database Compatibility:**
- No database schema changes during migration
- Concepts use existing tables/columns

---

## 8.7 Success Metrics

### Quantitative Metrics

**Concept Independence:**
```
Target: >95% of concepts have zero dependencies on other concepts
Measurement: Static analysis of concept imports/calls
```

**Traceability Coverage:**
```
Target: 100% of actions recorded in traces
Measurement: Ratio of actions with action_id to total actions
```

**Synchronization Locality:**
```
Target: >80% of syncs affect <3 concepts
Measurement: Count concepts referenced in each sync
```

### Qualitative Metrics

**Developer Experience:**
- Time to locate feature implementation (<5 minutes)
- Time to add new feature (measured before/after)
- Developer survey: "Is architecture more understandable?"

**Production Stability:**
- Zero regression defects from migration
- No performance degradation
- Same or better uptime

---

## 8.8 Training and Documentation

### Developer Onboarding

**Materials:**
1. Concept design guidelines ([03_concept_guidelines.md](03_concept_guidelines.md))
2. Synchronization patterns ([04_synchronization_patterns.md](04_synchronization_patterns.md))
3. WKMP examples ([07_wkmp_examples.md](07_wkmp_examples.md))
4. Migration playbook (this document)

**Training Sessions:**
- Week 1: Legibility principles overview
- Week 2: Hands-on concept extraction workshop
- Week 3: Synchronization definition workshop
- Week 4: Developer interface usage

### Reference Materials

**Templates:**
- Concept template ([09_references_templates.md](09_references_templates.md))
- Synchronization template ([09_references_templates.md](09_references_templates.md))
- Test template

**Checklists:**
- Concept design checklist ([00_SUMMARY.md](00_SUMMARY.md))
- Synchronization design checklist ([00_SUMMARY.md](00_SUMMARY.md))
- Action trace verification ([00_SUMMARY.md](00_SUMMARY.md))

---

## Navigation

**Previous:** [07_wkmp_examples.md](07_wkmp_examples.md) - WKMP application examples
**Next:** [09_references_templates.md](09_references_templates.md) - References and templates

**Back to Summary:** [00_SUMMARY.md](00_SUMMARY.md)

---

**END OF SECTION 08**
