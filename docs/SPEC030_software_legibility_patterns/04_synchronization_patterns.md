# Synchronization Patterns

**Part of:** [SPEC030 - Software Legibility Patterns](00_SUMMARY.md)

---

## 4.1 Event-Driven Orchestration

**Core Principle:** Concepts emit events; synchronizations listen and orchestrate actions

### Event Pattern Matching

**Basic Structure:**
```rust
pub enum EventPattern {
    /// Match specific concept action completion
    ActionCompleted {
        concept: String,
        action: String,
        bindings: Vec<(String, Variable)>,
    },

    /// Match event type regardless of source
    EventType {
        event_name: String,
        bindings: Vec<(String, Variable)>,
    },

    /// Logical combinations
    Any(Vec<EventPattern>),
    All(Vec<EventPattern>),
}

pub enum Variable {
    /// Extract value from event and bind to variable name
    Free,
    /// Must match specific bound value
    Bound,
}
```

**Example:**
```rust
// When AudioPlayer.play_passage completes, extract passage_id:
EventPattern::ActionCompleted {
    concept: "AudioPlayer".to_string(),
    action: "play_passage".to_string(),
    bindings: vec![
        ("passage_id", Variable::Free),  // Extract from event
        ("timestamp", Variable::Free),
    ],
}

// When ANY concept emits PlaybackStarted event:
EventPattern::EventType {
    event_name: "PlaybackStarted".to_string(),
    bindings: vec![
        ("passage_id", Variable::Free),
    ],
}
```

### State Conditions (WHERE Clauses)

**Query-Based Predicates:**
```rust
pub enum StateCondition {
    /// Query concept state (no side effects)
    Query {
        concept: String,
        query: String,
        parameters: Vec<(String, Variable)>,
        expectation: Expectation,
    },

    /// Logical combinations
    And(Vec<StateCondition>),
    Or(Vec<StateCondition>),
    Not(Box<StateCondition>),
}

pub enum Expectation {
    /// Query must return true
    Affirm,
    /// Query must return false
    Negate,
    /// Query result must match value
    Equals(serde_json::Value),
}
```

**Example:**
```rust
StateCondition::And(vec![
    // Check if auto-play enabled:
    StateCondition::Query {
        concept: "Settings".to_string(),
        query: "is_auto_play_enabled".to_string(),
        parameters: vec![],
        expectation: Expectation::Affirm,
    },

    // Check if queue depth < 2:
    StateCondition::Query {
        concept: "AudioPlayer".to_string(),
        query: "get_queue_depth".to_string(),
        parameters: vec![],
        expectation: Expectation::Equals(json!(1)),
    },
])
```

### Action Sequences (THEN Clauses)

**Action Types:**
```rust
pub enum Action {
    /// Invoke concept action with bound variables
    Invoke {
        concept: String,
        action: String,
        parameters: Vec<(String, Variable)>,
    },

    /// Emit new event (enables chaining)
    EmitEvent {
        event_name: String,
        parameters: Vec<(String, Variable)>,
    },

    /// Conditional branching
    If {
        condition: StateCondition,
        then: Vec<Action>,
        else_actions: Vec<Action>,
    },
}
```

**Example:**
```rust
vec![
    // Get target flavor for current time:
    Action::Invoke {
        concept: "Timeslot".to_string(),
        action: "get_target_flavor".to_string(),
        parameters: vec![("time", Variable::Bound)],
    },

    // Select passage matching flavor:
    Action::Invoke {
        concept: "PassageSelection".to_string(),
        action: "select_next".to_string(),
        parameters: vec![("flavor", Variable::Bound)],
    },

    // Enqueue selected passage:
    Action::Invoke {
        concept: "AudioPlayer".to_string(),
        action: "enqueue".to_string(),
        parameters: vec![("passage_id", Variable::Bound)],
    },
]
```

---

## 4.2 WKMP Synchronization Catalog

### Core Playback Synchronizations

#### AutoSelectWhenQueueLow
**Purpose:** Automatically refill queue when running low

```rust
Synchronization {
    name: "AutoSelectWhenQueueLow".to_string(),
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
        Action::Invoke {
            concept: "Timeslot".to_string(),
            action: "get_target_flavor".to_string(),
            parameters: vec![],
        },
        Action::Invoke {
            concept: "PassageSelection".to_string(),
            action: "select_next".to_string(),
            parameters: vec![("flavor", Variable::Bound)],
        },
        Action::Invoke {
            concept: "AudioPlayer".to_string(),
            action: "enqueue".to_string(),
            parameters: vec![("passage_id", Variable::Bound)],
        },
    ],
}
```

#### RecordPlaybackForCooldown
**Purpose:** Track song plays for cooldown enforcement

```rust
Synchronization {
    name: "RecordPlaybackForCooldown".to_string(),
    when: EventPattern::EventType {
        event_name: "PlaybackStarted".to_string(),
        bindings: vec![
            ("passage_id", Variable::Free),
            ("song_id", Variable::Free),
            ("timestamp", Variable::Free),
        ],
    },
    where_clause: StateCondition::Always,
    then: vec![
        Action::Invoke {
            concept: "Cooldown".to_string(),
            action: "record_play".to_string(),
            parameters: vec![
                ("song_id", Variable::Bound),
                ("timestamp", Variable::Bound),
            ],
        },
    ],
}
```

### Authentication Synchronizations

#### EnforceAuthenticationOnPlayback
**Purpose:** Require authentication for playback actions

```rust
Synchronization {
    name: "EnforceAuthenticationOnPlayback".to_string(),
    when: EventPattern::ActionCompleted {
        concept: "Web".to_string(),
        action: "handle_play_request".to_string(),
        bindings: vec![
            ("user_id", Variable::Free),
            ("passage_id", Variable::Free),
        ],
    },
    where_clause: StateCondition::Query {
        concept: "Authentication".to_string(),
        query: "is_authenticated".to_string(),
        parameters: vec![("user_id", Variable::Bound)],
        expectation: Expectation::Affirm,
    },
    then: vec![
        Action::Invoke {
            concept: "AudioPlayer".to_string(),
            action: "play_passage".to_string(),
            parameters: vec![("passage_id", Variable::Bound)],
        },
    ],
    else_actions: vec![
        Action::Invoke {
            concept: "Web".to_string(),
            action: "respond_unauthorized".to_string(),
            parameters: vec![],
        },
    ],
}
```

### Multi-User Coordination Synchronizations

#### BroadcastStateChangeToAllClients
**Purpose:** Notify all connected clients of state changes

```rust
Synchronization {
    name: "BroadcastStateChangeToAllClients".to_string(),
    when: EventPattern::Any(vec![
        EventPattern::EventType {
            event_name: "PlaybackStarted".to_string(),
            bindings: vec![("passage_id", Variable::Free)],
        },
        EventPattern::EventType {
            event_name: "PlaybackPaused".to_string(),
            bindings: vec![],
        },
        EventPattern::EventType {
            event_name: "PlaybackStopped".to_string(),
            bindings: vec![],
        },
    ]),
    where_clause: StateCondition::Always,
    then: vec![
        Action::Invoke {
            concept: "SSEBroadcaster".to_string(),
            action: "broadcast_event".to_string(),
            parameters: vec![("event", Variable::Bound)],
        },
    ],
}
```

---

## 4.3 Implementation Patterns

### Synchronization Engine

**Core Data Structure:**
```rust
pub struct SynchronizationEngine {
    /// Registered synchronization rules
    synchronizations: Vec<Synchronization>,

    /// Event bus for concept communication
    event_bus: Arc<Mutex<EventBus>>,

    /// Action tracer for provenance
    tracer: Arc<ActionTracer>,

    /// Query executor for WHERE clauses
    query_executor: Arc<QueryExecutor>,
}

impl SynchronizationEngine {
    pub async fn run(&self) -> Result<()> {
        let mut rx = self.event_bus.lock().await.subscribe();

        while let Ok(event) = rx.recv().await {
            // Find matching synchronizations:
            let matches = self.find_matching_syncs(&event).await;

            for sync in matches {
                // Check WHERE clause:
                if self.evaluate_condition(&sync.where_clause).await? {
                    // Execute THEN actions:
                    self.execute_actions(&sync.then, &event).await?;
                } else if let Some(else_actions) = &sync.else_actions {
                    // Execute ELSE actions:
                    self.execute_actions(else_actions, &event).await?;
                }
            }
        }

        Ok(())
    }
}
```

### Variable Binding

**Pattern:**
```rust
pub struct VariableContext {
    bindings: HashMap<String, serde_json::Value>,
}

impl VariableContext {
    /// Bind free variables from event
    pub fn bind_from_event(&mut self, event: &Event, pattern: &EventPattern) {
        match pattern {
            EventPattern::EventType { bindings, .. } => {
                for (var_name, var_type) in bindings {
                    if matches!(var_type, Variable::Free) {
                        if let Some(value) = event.get_field(var_name) {
                            self.bindings.insert(var_name.clone(), value);
                        }
                    }
                }
            }
            // ... other patterns
        }
    }

    /// Bind free variables from action outputs
    pub fn bind_from_action_output(&mut self, output: &serde_json::Value, action: &Action) {
        if let Action::Invoke { parameters, .. } = action {
            for (var_name, var_type) in parameters {
                if matches!(var_type, Variable::Free) {
                    if let Some(value) = output.get(var_name) {
                        self.bindings.insert(var_name.clone(), value.clone());
                    }
                }
            }
        }
    }

    /// Resolve bound variables to concrete values
    pub fn resolve(&self, var_name: &str) -> Option<&serde_json::Value> {
        self.bindings.get(var_name)
    }
}
```

### Query Executor

**Pattern:**
```rust
pub struct QueryExecutor {
    concepts: HashMap<String, Arc<dyn ConceptQuery>>,
}

#[async_trait]
pub trait ConceptQuery: Send + Sync {
    async fn execute_query(
        &self,
        query_name: &str,
        parameters: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value>;
}

impl QueryExecutor {
    pub async fn evaluate_condition(
        &self,
        condition: &StateCondition,
        context: &VariableContext,
    ) -> Result<bool> {
        match condition {
            StateCondition::Query { concept, query, parameters, expectation } => {
                // Resolve parameters using context:
                let resolved_params = self.resolve_parameters(parameters, context)?;

                // Execute query:
                let result = self.concepts
                    .get(concept)
                    .ok_or(SyncError::ConceptNotFound)?
                    .execute_query(query, resolved_params)
                    .await?;

                // Check expectation:
                match expectation {
                    Expectation::Affirm => Ok(result.as_bool().unwrap_or(false)),
                    Expectation::Negate => Ok(!result.as_bool().unwrap_or(true)),
                    Expectation::Equals(expected) => Ok(result == *expected),
                }
            }

            StateCondition::And(conditions) => {
                for cond in conditions {
                    if !self.evaluate_condition(cond, context).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }

            StateCondition::Or(conditions) => {
                for cond in conditions {
                    if self.evaluate_condition(cond, context).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }

            StateCondition::Not(condition) => {
                Ok(!self.evaluate_condition(condition, context).await?)
            }
        }
    }
}
```

---

## 4.4 Advanced Patterns

### Chained Synchronizations

**Pattern:** One synchronization's action emits event that triggers another synchronization

```rust
// Synchronization 1: User registers
Synchronization {
    name: "OnUserRegistration".to_string(),
    when: EventPattern::ActionCompleted {
        concept: "User".to_string(),
        action: "register".to_string(),
        bindings: vec![("user_id", Variable::Free)],
    },
    where_clause: StateCondition::Always,
    then: vec![
        Action::EmitEvent {
            event_name: "UserRegistered".to_string(),
            parameters: vec![("user_id", Variable::Bound)],
        },
    ],
}

// Synchronization 2: Create profile when user registered
Synchronization {
    name: "CreateDefaultProfile".to_string(),
    when: EventPattern::EventType {
        event_name: "UserRegistered".to_string(),
        bindings: vec![("user_id", Variable::Free)],
    },
    where_clause: StateCondition::Always,
    then: vec![
        Action::Invoke {
            concept: "Profile".to_string(),
            action: "create_default".to_string(),
            parameters: vec![("user_id", Variable::Bound)],
        },
    ],
}

// Synchronization 3: Apply settings when profile created
Synchronization {
    name: "ApplyDefaultSettings".to_string(),
    when: EventPattern::EventType {
        event_name: "ProfileCreated".to_string(),
        bindings: vec![("user_id", Variable::Free)],
    },
    where_clause: StateCondition::Always,
    then: vec![
        Action::Invoke {
            concept: "Settings".to_string(),
            action: "apply_defaults".to_string(),
            parameters: vec![("user_id", Variable::Bound)],
        },
    ],
}
```

**Result:** Single `User::register` action triggers cascading chain via events

### Conditional Synchronizations

**Pattern:** Different actions based on state

```rust
Synchronization {
    name: "SmartQueueRefill".to_string(),
    when: EventPattern::EventType {
        event_name: "QueueDepthChanged".to_string(),
        bindings: vec![("depth", Variable::Free)],
    },
    where_clause: StateCondition::Query {
        concept: "AudioPlayer".to_string(),
        query: "get_queue_depth".to_string(),
        parameters: vec![],
        expectation: Expectation::Equals(json!(1)),
    },
    then: vec![
        Action::If {
            condition: StateCondition::Query {
                concept: "Settings".to_string(),
                query: "get_selection_mode".to_string(),
                parameters: vec![],
                expectation: Expectation::Equals(json!("auto")),
            },
            then: vec![
                Action::Invoke {
                    concept: "PassageSelection".to_string(),
                    action: "select_next".to_string(),
                    parameters: vec![],
                },
            ],
            else_actions: vec![
                Action::Invoke {
                    concept: "Notification".to_string(),
                    action: "notify_user".to_string(),
                    parameters: vec![("message", Variable::Bound)],
                },
            ],
        },
    ],
}
```

### Batched Actions

**Pattern:** Group related actions to execute atomically

```rust
Synchronization {
    name: "InitializeNewUser".to_string(),
    when: EventPattern::EventType {
        event_name: "UserRegistered".to_string(),
        bindings: vec![("user_id", Variable::Free)],
    },
    where_clause: StateCondition::Always,
    then: vec![
        // All actions execute in sequence:
        Action::Invoke {
            concept: "Profile".to_string(),
            action: "create_default".to_string(),
            parameters: vec![("user_id", Variable::Bound)],
        },
        Action::Invoke {
            concept: "Settings".to_string(),
            action: "apply_defaults".to_string(),
            parameters: vec![("user_id", Variable::Bound)],
        },
        Action::Invoke {
            concept: "Library".to_string(),
            action: "create_empty".to_string(),
            parameters: vec![("user_id", Variable::Bound)],
        },
        Action::EmitEvent {
            event_name: "UserInitialized".to_string(),
            parameters: vec![("user_id", Variable::Bound)],
        },
    ],
}
```

---

## Navigation

**Previous:** [03_concept_guidelines.md](03_concept_guidelines.md) - Concept design guidelines
**Next:** [05_action_traces.md](05_action_traces.md) - Action trace architecture

**Back to Summary:** [00_SUMMARY.md](00_SUMMARY.md)

---

**END OF SECTION 04**
