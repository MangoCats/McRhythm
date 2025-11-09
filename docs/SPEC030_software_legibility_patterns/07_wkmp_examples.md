# WKMP Application Examples

**Part of:** [SPEC030 - Software Legibility Patterns](00_SUMMARY.md)

---

## 7.1 wkmp-ap (Audio Player) Concepts

### AudioPlayer Concept

**Purpose:** Core playback engine with queue management

**State:**
```rust
pub struct AudioPlayer {
    current_passage: Option<PassageId>,
    playback_state: PlaybackState,
    queue: VecDeque<PassageId>,
    volume: f32,
    crossfade_engine: Arc<CrossfadeEngine>,
    event_bus: Arc<EventBus>,
}

pub enum PlaybackState {
    Stopped,
    Playing { started_at: i64, position: Duration },
    Paused { paused_at: i64, position: Duration },
}
```

**Actions:**
```rust
impl AudioPlayer {
    pub async fn play_passage(&mut self, passage_id: PassageId) -> Result<()> {
        // Load passage metadata
        let passage = self.load_passage(passage_id).await?;

        // Start playback with crossfade
        self.crossfade_engine.start_playback(passage).await?;

        // Update state
        self.current_passage = Some(passage_id);
        self.playback_state = PlaybackState::Playing {
            started_at: chrono::Utc::now().timestamp_micros(),
            position: Duration::ZERO,
        };

        // Emit event
        self.event_bus.emit(Event::PlaybackStarted {
            passage_id,
            song_id: passage.song_id,
            timestamp: chrono::Utc::now().timestamp(),
        });

        Ok(())
    }

    pub async fn enqueue(&mut self, passage_id: PassageId) -> Result<()> {
        self.queue.push_back(passage_id);
        let depth = self.queue.len();

        self.event_bus.emit(Event::QueueDepthChanged { depth });

        Ok(())
    }

    pub async fn pause(&mut self) -> Result<()> {
        if let PlaybackState::Playing { position, .. } = self.playback_state {
            self.crossfade_engine.pause().await?;

            self.playback_state = PlaybackState::Paused {
                paused_at: chrono::Utc::now().timestamp_micros(),
                position,
            };

            self.event_bus.emit(Event::PlaybackPaused);
        }

        Ok(())
    }
}
```

**Queries:**
```rust
impl AudioPlayer {
    pub fn get_current_passage(&self) -> Option<PassageId> {
        self.current_passage
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.playback_state, PlaybackState::Playing { .. })
    }

    pub fn get_queue_depth(&self) -> usize {
        self.queue.len()
    }

    pub fn get_position(&self) -> Duration {
        match self.playback_state {
            PlaybackState::Playing { position, .. } |
            PlaybackState::Paused { position, .. } => position,
            PlaybackState::Stopped => Duration::ZERO,
        }
    }
}
```

### CrossfadeEngine Concept

**Purpose:** Sample-accurate audio crossfading

**State:**
```rust
pub struct CrossfadeEngine {
    active_fade: Option<ActiveFade>,
    fade_curves: HashMap<FadeCurveType, FadeCurve>,
    audio_output: Arc<AudioOutput>,
}

struct ActiveFade {
    outgoing_buffer: AudioBuffer,
    incoming_buffer: AudioBuffer,
    fade_curve: FadeCurveType,
    progress: f32,  // 0.0 to 1.0
}
```

**Actions:**
```rust
impl CrossfadeEngine {
    pub async fn start_crossfade(
        &mut self,
        outgoing: PassageId,
        incoming: PassageId,
        curve: FadeCurveType,
    ) -> Result<()> {
        // Pre-decode buffers
        let outgoing_buffer = self.decode_passage(outgoing).await?;
        let incoming_buffer = self.decode_passage(incoming).await?;

        self.active_fade = Some(ActiveFade {
            outgoing_buffer,
            incoming_buffer,
            fade_curve: curve,
            progress: 0.0,
        });

        Ok(())
    }

    pub async fn process_frame(&mut self, frame_size: usize) -> Result<Vec<f32>> {
        if let Some(fade) = &mut self.active_fade {
            let curve = &self.fade_curves[&fade.fade_curve];

            // Get samples from both buffers
            let outgoing = fade.outgoing_buffer.read(frame_size);
            let incoming = fade.incoming_buffer.read(frame_size);

            // Apply crossfade
            let mixed = curve.apply(outgoing, incoming, fade.progress);

            fade.progress += frame_size as f32 / fade.outgoing_buffer.total_frames() as f32;

            if fade.progress >= 1.0 {
                self.active_fade = None;
            }

            Ok(mixed)
        } else {
            // No active fade, pass through
            Ok(vec![0.0; frame_size])
        }
    }
}
```

---

## 7.2 wkmp-pd (Program Director) Concepts

### PassageSelection Concept

**Purpose:** Automatic passage selection based on musical flavor

**State:**
```rust
pub struct PassageSelection {
    db: Arc<Database>,
    selection_history: VecDeque<PassageId>,
    max_history: usize,
}
```

**Actions:**
```rust
impl PassageSelection {
    pub async fn select_next(&mut self, target_flavor: MusicalFlavor) -> Result<PassageId> {
        // Query eligible passages (not in history, passes cooldown)
        let eligible = self.db.query_eligible_passages(&self.selection_history).await?;

        // Calculate flavor distances
        let mut candidates: Vec<_> = eligible.iter()
            .map(|p| {
                let distance = calculate_flavor_distance(&target_flavor, &p.flavor);
                (p.passage_id, distance)
            })
            .collect();

        // Sort by distance (closest match first)
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Apply probabilistic selection (weighted by distance)
        let selected = self.weighted_random_selection(&candidates)?;

        // Update history
        self.selection_history.push_back(selected);
        if self.selection_history.len() > self.max_history {
            self.selection_history.pop_front();
        }

        Ok(selected)
    }

    pub async fn select_by_user(&mut self, passage_id: PassageId) -> Result<()> {
        // User-selected passages don't use flavor matching
        self.selection_history.push_back(passage_id);
        if self.selection_history.len() > self.max_history {
            self.selection_history.pop_front();
        }

        Ok(())
    }
}
```

**Queries:**
```rust
impl PassageSelection {
    pub fn get_recent_selections(&self, count: usize) -> Vec<PassageId> {
        self.selection_history.iter().rev().take(count).copied().collect()
    }
}
```

### Cooldown Concept

**Purpose:** Prevent repetition via configurable cooldown periods

**State:**
```rust
pub struct Cooldown {
    song_cooldowns: HashMap<SongId, i64>,      // Song-level
    artist_cooldowns: HashMap<ArtistId, i64>,  // Artist-level
    work_cooldowns: HashMap<WorkId, i64>,      // Work-level
    config: CooldownConfig,
}

pub struct CooldownConfig {
    song_cooldown_minutes: i64,
    artist_cooldown_minutes: i64,
    work_cooldown_minutes: i64,
}
```

**Actions:**
```rust
impl Cooldown {
    pub async fn record_play(
        &mut self,
        song_id: SongId,
        artist_id: ArtistId,
        work_id: Option<WorkId>,
        timestamp: i64,
    ) -> Result<()> {
        self.song_cooldowns.insert(song_id, timestamp);
        self.artist_cooldowns.insert(artist_id, timestamp);
        if let Some(work_id) = work_id {
            self.work_cooldowns.insert(work_id, timestamp);
        }

        Ok(())
    }

    pub async fn reset_cooldowns(&mut self) -> Result<()> {
        self.song_cooldowns.clear();
        self.artist_cooldowns.clear();
        self.work_cooldowns.clear();

        Ok(())
    }
}
```

**Queries:**
```rust
impl Cooldown {
    pub fn is_song_eligible(&self, song_id: SongId, current_time: i64) -> bool {
        if let Some(last_played) = self.song_cooldowns.get(&song_id) {
            let elapsed_minutes = (current_time - last_played) / 60;
            elapsed_minutes >= self.config.song_cooldown_minutes
        } else {
            true
        }
    }

    pub fn is_artist_eligible(&self, artist_id: ArtistId, current_time: i64) -> bool {
        if let Some(last_played) = self.artist_cooldowns.get(&artist_id) {
            let elapsed_minutes = (current_time - last_played) / 60;
            elapsed_minutes >= self.config.artist_cooldown_minutes
        } else {
            true
        }
    }

    pub fn is_work_eligible(&self, work_id: WorkId, current_time: i64) -> bool {
        if let Some(last_played) = self.work_cooldowns.get(&work_id) {
            let elapsed_minutes = (current_time - last_played) / 60;
            elapsed_minutes >= self.config.work_cooldown_minutes
        } else {
            true
        }
    }
}
```

### Timeslot Concept

**Purpose:** Time-of-day based target flavor scheduling

**State:**
```rust
pub struct Timeslot {
    schedule: Vec<TimeslotEntry>,
}

pub struct TimeslotEntry {
    time_of_day: chrono::NaiveTime,
    target_flavor: MusicalFlavor,
    name: String,  // e.g., "Morning Upbeat", "Evening Mellow"
}
```

**Actions:**
```rust
impl Timeslot {
    pub async fn update_schedule(&mut self, schedule: Vec<TimeslotEntry>) -> Result<()> {
        self.schedule = schedule;
        self.schedule.sort_by_key(|entry| entry.time_of_day);

        Ok(())
    }
}
```

**Queries:**
```rust
impl Timeslot {
    pub fn get_target_flavor(&self, time: chrono::NaiveTime) -> MusicalFlavor {
        // Find the most recent timeslot before current time
        let mut target = None;
        for entry in &self.schedule {
            if entry.time_of_day <= time {
                target = Some(&entry.target_flavor);
            } else {
                break;
            }
        }

        target.cloned().unwrap_or_else(|| {
            // If no timeslot before current time, use last timeslot (wraps from previous day)
            self.schedule.last()
                .map(|entry| entry.target_flavor.clone())
                .unwrap_or_default()
        })
    }

    pub fn get_current_timeslot_name(&self, time: chrono::NaiveTime) -> Option<String> {
        let mut name = None;
        for entry in &self.schedule {
            if entry.time_of_day <= time {
                name = Some(entry.name.clone());
            } else {
                break;
            }
        }
        name
    }
}
```

---

## 7.3 wkmp-ui (User Interface) Concepts

### Authentication Concept

**Purpose:** User session and token management

**State:**
```rust
pub struct Authentication {
    sessions: HashMap<Uuid, Session>,
    token_secret: String,
}

struct Session {
    user_id: UserId,
    created_at: i64,
    expires_at: i64,
    last_activity: i64,
}
```

**Actions:**
```rust
impl Authentication {
    pub async fn login(&mut self, username: String, password: String) -> Result<String> {
        // Validate credentials
        let user_id = self.validate_credentials(&username, &password).await?;

        // Create session
        let session_id = Uuid::new_v4();
        let now = chrono::Utc::now().timestamp();
        let session = Session {
            user_id,
            created_at: now,
            expires_at: now + 86400,  // 24 hours
            last_activity: now,
        };

        self.sessions.insert(session_id, session);

        // Generate token
        let token = self.generate_token(session_id, user_id)?;

        Ok(token)
    }

    pub async fn logout(&mut self, token: String) -> Result<()> {
        let session_id = self.parse_token(&token)?;
        self.sessions.remove(&session_id);

        Ok(())
    }

    pub async fn refresh_activity(&mut self, token: String) -> Result<()> {
        let session_id = self.parse_token(&token)?;
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.last_activity = chrono::Utc::now().timestamp();
        }

        Ok(())
    }
}
```

**Queries:**
```rust
impl Authentication {
    pub fn is_authenticated(&self, token: &str) -> bool {
        if let Ok(session_id) = self.parse_token(token) {
            if let Some(session) = self.sessions.get(&session_id) {
                let now = chrono::Utc::now().timestamp();
                return now < session.expires_at;
            }
        }
        false
    }

    pub fn get_user_id(&self, token: &str) -> Option<UserId> {
        let session_id = self.parse_token(token).ok()?;
        let session = self.sessions.get(&session_id)?;
        Some(session.user_id)
    }
}
```

### SSEBroadcaster Concept

**Purpose:** Server-sent events for multi-user coordination

**State:**
```rust
pub struct SSEBroadcaster {
    channels: HashMap<String, broadcast::Sender<String>>,
}
```

**Actions:**
```rust
impl SSEBroadcaster {
    pub async fn broadcast_event(&mut self, channel: String, event: serde_json::Value) -> Result<()> {
        let sender = self.channels.entry(channel).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            tx
        });

        let event_str = serde_json::to_string(&event)?;
        sender.send(event_str).ok();

        Ok(())
    }

    pub async fn subscribe(&mut self, channel: String) -> broadcast::Receiver<String> {
        let sender = self.channels.entry(channel).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            tx
        });

        sender.subscribe()
    }
}
```

---

## 7.4 Cross-Module Synchronizations

### wkmp-ui → wkmp-ap Playback Request

**Synchronization:**
```rust
Synchronization {
    name: "HandleWebPlaybackRequest".to_string(),
    when: EventPattern::ActionCompleted {
        concept: "wkmp-ui::Web".to_string(),
        action: "handle_play_request".to_string(),
        bindings: vec![
            ("user_id", Variable::Free),
            ("passage_id", Variable::Free),
        ],
    },
    where_clause: StateCondition::Query {
        concept: "wkmp-ui::Authentication".to_string(),
        query: "is_authenticated".to_string(),
        parameters: vec![("user_id", Variable::Bound)],
        expectation: Expectation::Affirm,
    },
    then: vec![
        // HTTP call to wkmp-ap:
        Action::Invoke {
            concept: "wkmp-ui::HttpClient".to_string(),
            action: "post".to_string(),
            parameters: vec![
                ("url", Variable::Literal(json!("http://localhost:5721/api/play"))),
                ("body", Variable::Bound),
            ],
        },
    ],
}
```

**Note:** Inter-module communication remains HTTP-based. Synchronization applies **within** each module.

### wkmp-ap → wkmp-pd Auto-Selection

**wkmp-ap emits event:**
```rust
// In AudioPlayer concept:
self.event_bus.emit(Event::QueueDepthChanged { depth: 1 });
```

**wkmp-ap synchronization (intra-module):**
```rust
Synchronization {
    name: "RequestAutoSelectionWhenQueueLow".to_string(),
    when: EventPattern::EventType {
        event_name: "QueueDepthChanged".to_string(),
        bindings: vec![("depth", Variable::Free)],
    },
    where_clause: StateCondition::And(vec![
        StateCondition::Query {
            concept: "Settings".to_string(),
            query: "is_auto_play_enabled".to_string(),
            parameters: vec![],
            expectation: Expectation::Affirm,
        },
        StateCondition::Query {
            concept: "AudioPlayer".to_string(),
            query: "get_queue_depth".to_string(),
            parameters: vec![],
            expectation: Expectation::Equals(json!(1)),
        },
    ]),
    then: vec![
        // HTTP call to wkmp-pd:
        Action::Invoke {
            concept: "HttpClient".to_string(),
            action: "get".to_string(),
            parameters: vec![
                ("url", Variable::Literal(json!("http://localhost:5722/api/select"))),
            ],
        },
        // Parse response and enqueue:
        Action::Invoke {
            concept: "AudioPlayer".to_string(),
            action: "enqueue".to_string(),
            parameters: vec![("passage_id", Variable::Bound)],
        },
    ],
}
```

---

## 7.5 Complete Example: User-Initiated Playback

**Scenario:** User clicks "Play" button in wkmp-ui for passage 42

**Flow:**

1. **wkmp-ui::Web** receives HTTP POST `/api/play` with `{"passage_id": 42, "token": "..."}`

2. **Sync: ValidateAuthBeforePlay**
   - WHEN: Web::handle_play_request
   - WHERE: Authentication::is_authenticated(token) == true
   - THEN: Proceed to step 3

3. **wkmp-ui::HttpClient** sends HTTP POST to `http://localhost:5721/api/play` with `{"passage_id": 42}`

4. **wkmp-ap::Web** receives request, emits Event::PlaybackRequested

5. **Sync: HandlePlaybackRequest**
   - WHEN: Event::PlaybackRequested
   - WHERE: Always
   - THEN: AudioPlayer::play_passage(42)

6. **wkmp-ap::AudioPlayer** starts playback, emits Event::PlaybackStarted

7. **Sync: RecordPlaybackForCooldown**
   - WHEN: Event::PlaybackStarted
   - WHERE: Always
   - THEN: Cooldown::record_play(song_id, timestamp)

8. **Sync: BroadcastPlaybackState**
   - WHEN: Event::PlaybackStarted
   - WHERE: Always
   - THEN: SSEBroadcaster::broadcast_event("playback_state", event)

9. **wkmp-ui::SSEBroadcaster** sends SSE to all connected clients

10. All web clients update UI to show "Now Playing: Passage 42"

**Action Trace:**
```
[wkmp-ui::Web::handle_play_request]
    ──(ValidateAuthBeforePlay)──> [wkmp-ui::Auth::is_authenticated]
    ──(ValidateAuthBeforePlay)──> [wkmp-ui::HttpClient::post]
                                       │
                                       └──> [HTTP to wkmp-ap]
                                               │
[wkmp-ap::Web::handle_request] <───────────────┘
    ──(HandlePlaybackRequest)──> [wkmp-ap::AudioPlayer::play_passage]
                                       │
                                       ├──(RecordPlaybackForCooldown)──> [wkmp-ap::Cooldown::record_play]
                                       │
                                       └──(BroadcastPlaybackState)──> [wkmp-ap::SSEBroadcaster::broadcast]
```

---

## Navigation

**Previous:** [06_dev_interface.md](06_dev_interface.md) - Visible developer interface
**Next:** [08_migration_strategy.md](08_migration_strategy.md) - Migration strategy

**Back to Summary:** [00_SUMMARY.md](00_SUMMARY.md)

---

**END OF SECTION 07**
