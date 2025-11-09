# Core Structural Patterns

**Part of:** [SPEC030 - Software Legibility Patterns](00_SUMMARY.md)

---

## 2.1 Concepts

**Definition:** Self-contained, user-facing functional units with well-defined purposes

### Structure

```rust
pub struct ConceptName {
    // Private state - NO external dependencies on other concepts
    state: ConceptState,
}

impl ConceptName {
    // Actions: named operations with explicit inputs/outputs
    pub async fn action_name(&mut self, input: Input) -> Result<Output> {
        // Complete behavioral protocol
        // Updates only this concept's state
        // Emits events for synchronization
    }

    // Queries: read-only access to state
    pub fn query_name(&self, filter: Filter) -> QueryResult {
        // No side effects
        // Used by synchronization WHERE clauses
    }
}
```

### Key Properties

- **Independence:** Concepts do not call each other directly
- **Encapsulation:** All state private, accessed only via actions/queries
- **Completeness:** Each concept provides full behavioral protocol for its purpose
- **Naming:** Fully qualified URIs for all actions (enables global traceability)

### Examples in WKMP

- `AudioPlayer` concept: playback state, queue, crossfade engine
- `PassageSelection` concept: flavor matching, cooldown tracking
- `Authentication` concept: user sessions, token validation
- `MusicBrainz` concept: recording identification, metadata fetching

---

## 2.2 Synchronizations

**Definition:** Declarative event-driven rules orchestrating concept interactions

### Pattern

```rust
pub struct Synchronization {
    name: String,  // Unique identifier for traceability
    when: EventPattern,  // Which concept actions trigger this
    where_clause: StateCondition,  // Query-based preconditions
    then: Vec<Action>,  // Subsequent actions to invoke
}
```

### Three-Clause Structure

**WHEN** - Event pattern matching:
```rust
when: EventPattern::ActionCompleted {
    concept: "User",
    action: "register",
    bindings: vec![("user", Var::Bound), ("username", Var::Free)]
}
```

**WHERE** - State conditions (queries only, no side effects):
```rust
where_clause: StateCondition::And(vec![
    Query::ConceptState("Profile", "exists", vec![("user", Var::Bound)]).negate(),
    Query::ConceptState("Settings", "has_default_config", vec![]).affirm(),
])
```

**THEN** - Cascading actions:
```rust
then: vec![
    Action::Invoke("Profile", "create_default", vec![("user", Var::Bound)]),
    Action::Invoke("Settings", "apply_defaults", vec![("user", Var::Bound)]),
    Action::EmitEvent("UserInitialized", vec![("user", Var::Bound)]),
]
```

### Key Properties

- **Decoupling:** No concept-to-concept calls, only concept-to-sync-to-concept chains
- **Declarative:** "What" not "how" - rules not imperative code
- **Traceable:** Each sync instance tagged with name in action trace
- **Compositional:** Syncs can chain (one action triggers another sync)

---

## 2.3 Action Traces

**Definition:** Directed acyclic graphs capturing causal chains of operations

### Structure

```rust
pub struct ActionTrace {
    flow_token: Uuid,  // Groups causally-related actions
    root_action: ActionRecord,
    actions: Vec<ActionRecord>,
    provenance_edges: Vec<ProvenanceEdge>,
}

pub struct ActionRecord {
    id: Uuid,
    concept: String,  // e.g., "wkmp-ap::AudioPlayer"
    action: String,   // e.g., "play_passage"
    inputs: serde_json::Value,
    outputs: serde_json::Value,
    timestamp: i64,
    flow_token: Uuid,
}

pub struct ProvenanceEdge {
    from_action: Uuid,
    to_action: Uuid,
    synchronization: String,  // Which sync created this edge
}
```

### Visualization as Graph

```
[Web::request]
    |-- (sync: OnUserRegistration) --> [User::register]
    |-- (sync: OnUserRegistration) --> [Password::set]
    |-- (sync: CreateDefaultProfile) --> [Profile::create]
    |-- (sync: OnProfileCreated) --> [Settings::apply_defaults]
    |-- (sync: GenerateAuthToken) --> [JWT::generate]
    |-- (sync: OnUserRegistration) --> [Web::respond]
```

### Key Properties

- **Causality:** Edges show which action caused which, via which sync
- **Flow Tokens:** Group related actions (e.g., all actions for one HTTP request)
- **Named Provenance:** Every edge labeled with synchronization name
- **Queryable:** "Which sync produced this action?" "What triggered this state change?"

---

## Navigation

**Previous:** [01_legibility_principles.md](01_legibility_principles.md) - Legibility principles
**Next:** [03_concept_guidelines.md](03_concept_guidelines.md) - Concept design guidelines

**Back to Summary:** [00_SUMMARY.md](00_SUMMARY.md)

---

**NOTE:** This section contains abbreviated content from the original 1823-line document. For complete implementation details, see templates in [09_references_templates.md](09_references_templates.md).

**END OF SECTION 02**
