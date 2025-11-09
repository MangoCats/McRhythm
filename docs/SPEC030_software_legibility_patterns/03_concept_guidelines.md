# Concept Design Guidelines

**Part of:** [SPEC030 - Software Legibility Patterns](00_SUMMARY.md)

---

## 3.1 Identifying Concepts

**Definition:** A concept is a user-facing functional unit with a clear, singular purpose

### Identification Criteria

**Ask these questions:**
1. Does this represent a distinct user-facing functionality?
2. Can it operate independently with its own state?
3. Does it provide a complete behavioral protocol for its purpose?
4. Is it reusable across different domains/contexts?
5. Can users understand its purpose without technical knowledge?

**Examples in WKMP:**

| Concept | User-Facing Purpose | Independent State | Complete Protocol |
|---------|---------------------|-------------------|-------------------|
| AudioPlayer | Playback control | Current passage, queue, volume | play, pause, stop, seek, enqueue |
| PassageSelection | Music choice | Flavor preferences, selection history | select_next, select_by_flavor |
| Cooldown | Prevent repetition | Last played timestamps | record_play, check_eligible |
| Timeslot | Time-based preferences | Schedule entries, target flavors | get_target_for_time, update_schedule |
| Authentication | User identity | Sessions, tokens, permissions | login, logout, validate_token |

### Non-Concepts (Technical Utilities)

**These are NOT concepts:**
- Database connection pool (no user-facing purpose)
- HTTP client (infrastructure)
- Logging framework (cross-cutting concern)
- Configuration loader (bootstrap utility)

**Rule:** If users cannot describe it in domain terms, it's not a concept

---

## 3.2 Concept API Design

### Action Naming Conventions

**Format:** `verb_noun` (imperative mood)

**Good:**
- `play_passage(passage_id)`
- `select_next(flavor)`
- `record_playback(song_id, timestamp)`
- `validate_token(token)`

**Bad:**
- `playback()` (ambiguous - play or stop?)
- `select()` (missing: select what?)
- `cooldown()` (noun, not action)

### Query Naming Conventions

**Format:** `get_noun` or `is_adjective` or `has_noun`

**Good:**
- `get_current_passage() -> Option<PassageId>`
- `is_playing() -> bool`
- `has_permission(user_id, resource) -> bool`
- `get_eligible_songs(timestamp) -> Vec<SongId>`

**Bad:**
- `current_passage()` (ambiguous - getter or action?)
- `playing()` (unclear return type)
- `permission()` (missing "has" or "check")

### Input/Output Contracts

**All actions and queries MUST:**
- Accept explicitly typed parameters (no `Any` or `serde_json::Value`)
- Return explicit types or `Result<T, ConceptError>`
- Document preconditions/postconditions in doc comments

**Example:**
```rust
/// Plays the specified passage with crossfading
///
/// # Preconditions
/// - `passage_id` must exist in database
/// - No other passage currently playing (or will crossfade)
///
/// # Postconditions
/// - `current_passage` set to `Some(passage_id)`
/// - `PlaybackStarted` event emitted
/// - Audio output begins within 100ms
///
/// # Errors
/// - `PassageNotFound` if passage_id invalid
/// - `AudioDeviceError` if audio subsystem fails
pub async fn play_passage(&mut self, passage_id: PassageId) -> Result<(), AudioPlayerError> {
    // Implementation
}
```

---

## 3.3 Concept Independence Rules

### Rule 1: No Direct Concept-to-Concept Calls

**Violation:**
```rust
impl PassageSelection {
    async fn auto_select(&mut self, player: &AudioPlayer) -> Result<()> {
        if player.queue_depth() < 2 {  // Direct dependency
            let passage = self.select_next().await?;
            player.enqueue(passage).await?;  // Direct call
        }
        Ok(())
    }
}
```

**Solution:**
```rust
// PassageSelection emits event:
impl PassageSelection {
    async fn select_next(&mut self, flavor: Flavor) -> Result<PassageId> {
        let passage = self.find_best_match(flavor)?;
        self.emit_event(Event::PassageSelected(passage));
        Ok(passage)
    }
}

// Synchronization orchestrates:
Synchronization {
    name: "EnqueueSelectedPassage",
    when: Event::PassageSelected(passage_id),
    where: Query::Always,
    then: vec![
        Action::Invoke("AudioPlayer", "enqueue", vec![("passage_id", Var::Bound)]),
    ]
}
```

### Rule 2: Private State Only

**Violation:**
```rust
pub struct AudioPlayer {
    pub current_passage: Option<PassageId>,  // Public field
    volume: f32,
}
```

**Solution:**
```rust
pub struct AudioPlayer {
    current_passage: Option<PassageId>,  // Private
    volume: f32,
}

impl AudioPlayer {
    // Expose via query:
    pub fn get_current_passage(&self) -> Option<PassageId> {
        self.current_passage
    }
}
```

### Rule 3: Emit Events, Don't Call Callbacks

**Violation:**
```rust
impl AudioPlayer {
    async fn on_playback_end(&mut self, callback: impl Fn()) {
        callback();  // Tight coupling to caller
    }
}
```

**Solution:**
```rust
impl AudioPlayer {
    async fn on_playback_end(&mut self) {
        self.emit_event(Event::PlaybackEnded);  // Decoupled
    }
}

// Synchronizations handle events:
Synchronization {
    name: "AutoSelectOnPlaybackEnd",
    when: Event::PlaybackEnded,
    where: Query::ConceptState("Settings", "auto_play_enabled"),
    then: vec![
        Action::Invoke("PassageSelection", "select_next"),
    ]
}
```

### Rule 4: No Shared Mutable State

**Violation:**
```rust
// Shared database connection in both concepts:
impl AudioPlayer {
    async fn play(&mut self, db: &DatabasePool) {
        // Direct database access
    }
}

impl PassageSelection {
    async fn select(&mut self, db: &DatabasePool) {
        // Same database access
    }
}
```

**Solution:**
```rust
// Each concept owns its state:
impl AudioPlayer {
    state: AudioPlayerState,  // In-memory only
}

impl PassageSelection {
    state: SelectionState,  // In-memory only
}

// Separate Persistence concept handles database:
impl Persistence {
    async fn save_playback_state(&mut self, state: PlaybackState) -> Result<()> {
        // Database operations centralized
    }
}

// Synchronization coordinates:
Synchronization {
    name: "PersistPlaybackState",
    when: Event::PlaybackStateChanged(state),
    where: Query::Always,
    then: vec![
        Action::Invoke("Persistence", "save_playback_state", vec![("state", Var::Bound)]),
    ]
}
```

---

## 3.4 URI Naming Scheme

**Purpose:** Global unique identifiers for concepts and actions enabling cross-module traceability

### Format

```
<module>::<concept>::<action>
```

**Examples:**
- `wkmp-ap::AudioPlayer::play_passage`
- `wkmp-pd::PassageSelection::select_next`
- `wkmp-ui::Authentication::validate_token`
- `wkmp-ai::MusicBrainz::identify_recording`

### Implementation

```rust
pub trait Concept {
    /// Returns the fully-qualified URI for this concept
    fn concept_uri(&self) -> String {
        format!("{}::{}", env!("CARGO_PKG_NAME"), Self::concept_name())
    }

    /// Returns the concept name (e.g., "AudioPlayer")
    fn concept_name() -> &'static str;

    /// Returns the fully-qualified URI for an action
    fn action_uri(&self, action: &str) -> String {
        format!("{}::{}", self.concept_uri(), action)
    }
}

// Usage:
impl Concept for AudioPlayer {
    fn concept_name() -> &'static str {
        "AudioPlayer"
    }
}

// Recording actions:
let uri = player.action_uri("play_passage");
// Result: "wkmp-ap::AudioPlayer::play_passage"

tracer.record_action(ActionRecord {
    concept_uri: uri,
    inputs: json!({"passage_id": 42}),
    outputs: json!({"started_at": 1699564800}),
    // ...
});
```

### Benefits

**Traceability:**
- Query: "Show all invocations of wkmp-ap::AudioPlayer::play_passage"
- Result: Complete audit trail across all flow tokens

**Cross-Module Analysis:**
- Query: "Which wkmp-pd actions triggered wkmp-ap actions?"
- Result: Provenance edges showing inter-module causality

**Debugging:**
- Query: "Trace execution path for flow_token XYZ"
- Result: Full DAG with URIs showing module boundaries

---

## 3.5 Concept Lifecycle

### Initialization

**Pattern:**
```rust
impl Concept for AudioPlayer {
    fn concept_name() -> &'static str {
        "AudioPlayer"
    }

    async fn initialize(config: AudioPlayerConfig) -> Result<Self> {
        // Load initial state (no external dependencies)
        Ok(AudioPlayer {
            current_passage: None,
            queue: VecDeque::new(),
            volume: config.default_volume,
            crossfade_engine: CrossfadeEngine::new(config.fade_curves),
        })
    }
}
```

### State Persistence (via Synchronizations)

**Pattern:**
```rust
// Concept emits events:
impl AudioPlayer {
    pub async fn set_volume(&mut self, volume: f32) -> Result<()> {
        self.volume = volume;
        self.emit_event(Event::VolumeChanged(volume));
        Ok(())
    }
}

// Synchronization triggers persistence:
Synchronization {
    name: "PersistVolumeChange",
    when: Event::VolumeChanged(volume),
    where: Query::Always,
    then: vec![
        Action::Invoke("Settings", "save_volume", vec![("volume", Var::Bound)]),
    ]
}
```

### Shutdown

**Pattern:**
```rust
impl Concept for AudioPlayer {
    async fn shutdown(&mut self) -> Result<()> {
        // Graceful cleanup:
        // 1. Stop playback
        self.stop().await?;

        // 2. Emit shutdown event
        self.emit_event(Event::AudioPlayerShuttingDown);

        // 3. Release resources
        self.crossfade_engine.shutdown().await?;

        Ok(())
    }
}
```

---

## 3.6 Testing Concepts in Isolation

### Unit Test Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_play_passage_success() {
        // Arrange: Initialize concept with known state
        let mut player = AudioPlayer::new_for_test();
        let passage_id = PassageId::new(42);

        // Act: Invoke action
        let result = player.play_passage(passage_id).await;

        // Assert: Verify state change and events
        assert!(result.is_ok());
        assert_eq!(player.get_current_passage(), Some(passage_id));

        // Verify event emitted:
        let events = player.drain_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::PlaybackStarted(42)));
    }

    #[tokio::test]
    async fn test_play_passage_not_found() {
        let mut player = AudioPlayer::new_for_test();
        let invalid_id = PassageId::new(999);

        let result = player.play_passage(invalid_id).await;

        assert!(matches!(result, Err(AudioPlayerError::PassageNotFound)));
        assert_eq!(player.get_current_passage(), None);
    }
}
```

### Mock Event Bus

```rust
pub struct MockEventBus {
    emitted_events: Vec<Event>,
}

impl EventBus for MockEventBus {
    fn emit(&mut self, event: Event) {
        self.emitted_events.push(event);
    }

    fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.emitted_events)
    }
}
```

---

## Navigation

**Previous:** [02_core_patterns.md](02_core_patterns.md) - Core structural patterns
**Next:** [04_synchronization_patterns.md](04_synchronization_patterns.md) - Synchronization patterns

**Back to Summary:** [00_SUMMARY.md](00_SUMMARY.md)

---

**END OF SECTION 03**
