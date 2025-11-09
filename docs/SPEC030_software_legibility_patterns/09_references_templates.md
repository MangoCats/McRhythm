# References and Templates

**Part of:** [SPEC030 - Software Legibility Patterns](00_SUMMARY.md)

---

## 9.1 Academic Citations

### Primary Source

**Legible Software Design: Towards a Pattern Language for Code Structure**
- Authors: Christina Meng, Daniel Jackson
- Institution: MIT Computer Science and Artificial Intelligence Laboratory
- Publication: 2024
- URL: https://essenceofsoftware.com/posts/legible-software-design/

**Key Contributions:**
- Definition of software legibility
- Three pillars: incrementality, integrity, transparency
- Concept-based architecture pattern
- Synchronization as declarative orchestration
- Action traces with provenance

**Empirical Evidence:**
- Reduced lines of code by 28% in RealWorld app refactoring
- Eliminated cross-concept dependencies (100% independence)
- Improved LLM code generation success rate
- Enhanced developer comprehension in user studies

### Related Work

**Information Hiding (Parnas, 1972)**
- D.L. Parnas, "On the Criteria To Be Used in Decomposing Systems into Modules"
- Foundational work on modular design
- Legibility extends with explicit synchronization contracts

**Microservices Architecture**
- Service boundaries as coarse-grained concepts
- Legibility applies within services (finer granularity)
- Event-driven patterns align with synchronizations

**Domain-Driven Design (Evans, 2003)**
- Bounded contexts align with concept boundaries
- Ubiquitous language matches concept naming
- Aggregates similar to concepts with encapsulated state

**Aspect-Oriented Programming**
- Cross-cutting concerns handled by synchronizations
- Declarative instead of imperative weaving
- More transparent causality (action traces)

---

## 9.2 Concept Template

**File:** `<module>/src/concepts/<concept_name>.rs`

```rust
//! <Concept Name> Concept
//!
//! Purpose: <One-sentence description of user-facing functionality>
//!
//! Independence: This concept has no dependencies on other concepts.
//! All interactions mediated via event bus and synchronizations.

use wkmp_common::legibility::{Concept, EventBus, ActionTracer};
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// <Concept Name> state
///
/// All state is private. External access only via actions and queries.
pub struct <ConceptName> {
    // Private state fields:
    // - No references to other concepts
    // - No shared mutable state
    // - All domain logic encapsulated

    /// Event bus for emitting events
    event_bus: Arc<EventBus>,

    /// Action tracer for provenance
    tracer: Arc<ActionTracer>,
}

impl Concept for <ConceptName> {
    fn concept_name() -> &'static str {
        "<ConceptName>"
    }

    fn concept_uri(&self) -> String {
        format!("{}::{}", env!("CARGO_PKG_NAME"), Self::concept_name())
    }
}

impl <ConceptName> {
    /// Initialize concept with default state
    pub fn new(event_bus: Arc<EventBus>, tracer: Arc<ActionTracer>) -> Self {
        Self {
            // Initialize state
            event_bus,
            tracer,
        }
    }

    /// Initialize for testing (with mock event bus)
    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self::new(
            Arc::new(EventBus::new_for_test()),
            Arc::new(ActionTracer::new_for_test()),
        )
    }

    // ==================== ACTIONS ====================
    // Actions modify state and may emit events
    // Format: verb_noun (e.g., play_passage, enqueue, set_volume)

    /// <Action description>
    ///
    /// # Preconditions
    /// - <Condition 1>
    /// - <Condition 2>
    ///
    /// # Postconditions
    /// - <Outcome 1>
    /// - <Outcome 2>
    /// - Emits: <Event name>
    ///
    /// # Errors
    /// - <Error type>: <When it occurs>
    pub async fn action_name(&mut self, param: ParamType) -> Result<ReturnType> {
        // Record action start:
        let action_id = self.tracer.record_action(
            self.concept_uri(),
            "action_name".to_string(),
            json!({"param": param}),
            json!(null),
        );

        // Perform action logic:
        // - Update private state
        // - Validate preconditions
        // - Execute domain logic

        // Record action completion:
        self.tracer.update_action_output(action_id, json!({"result": result}));

        // Emit event:
        self.event_bus.emit(Event::<EventName> {
            // Event fields
        });

        Ok(result)
    }

    // ==================== QUERIES ====================
    // Queries read state without side effects
    // Format: get_noun, is_adjective, has_noun

    /// <Query description>
    ///
    /// Returns: <Description of return value>
    pub fn query_name(&self, filter: FilterType) -> QueryResult {
        // Read-only access to state
        // No mutations
        // No event emissions
        // Used by synchronization WHERE clauses
    }

    // ==================== INTERNAL HELPERS ====================
    // Private methods for implementation details

    fn internal_helper(&self) -> Result<()> {
        // Private implementation
        // Not exposed to synchronizations
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_action_success() {
        let mut concept = <ConceptName>::new_for_test();

        let result = concept.action_name(param).await;

        assert!(result.is_ok());
        // Verify state changes
        // Verify events emitted
    }

    #[tokio::test]
    async fn test_action_failure() {
        let mut concept = <ConceptName>::new_for_test();

        let result = concept.action_name(invalid_param).await;

        assert!(result.is_err());
        // Verify error type
    }

    #[test]
    fn test_query() {
        let concept = <ConceptName>::new_for_test();

        let result = concept.query_name(filter);

        assert_eq!(result, expected);
    }
}
```

---

## 9.3 Synchronization Template

**File:** `<module>/src/syncs/<sync_name>.rs`

```rust
//! <Synchronization Name> Synchronization
//!
//! Purpose: <One-sentence description of orchestration>
//!
//! Participants:
//! - Trigger: <Concept>::<Action or Event>
//! - Conditions: <Concept>::<Query> (if applicable)
//! - Actions: <Concept>::<Action>, <Concept>::<Action>, ...

use wkmp_common::legibility::{Synchronization, EventPattern, StateCondition, Action, Variable, Expectation};

/// <Synchronization Name>
///
/// WHEN: <Event description>
/// WHERE: <Condition description>
/// THEN: <Actions description>
pub fn <sync_name>() -> Synchronization {
    Synchronization {
        name: "<SyncName>".to_string(),

        // ==================== WHEN Clause ====================
        // Event pattern matching - what triggers this sync?

        when: EventPattern::ActionCompleted {
            concept: "<ConceptName>".to_string(),
            action: "<action_name>".to_string(),
            bindings: vec![
                // Extract variables from event:
                ("<var_name>", Variable::Free),   // Extract from event
                ("<var_name>", Variable::Bound),  // Must match bound value
            ],
        },

        // Alternative: EventType pattern
        // when: EventPattern::EventType {
        //     event_name: "<EventName>".to_string(),
        //     bindings: vec![
        //         ("<var_name>", Variable::Free),
        //     ],
        // },

        // ==================== WHERE Clause ====================
        // State conditions - when should actions execute?

        where_clause: StateCondition::Query {
            concept: "<ConceptName>".to_string(),
            query: "<query_name>".to_string(),
            parameters: vec![
                ("<param_name>", Variable::Bound),
            ],
            expectation: Expectation::Affirm,  // Must return true
        },

        // Alternative: Compound conditions
        // where_clause: StateCondition::And(vec![
        //     StateCondition::Query { ... },
        //     StateCondition::Query { ... },
        // ]),

        // Alternative: Always execute
        // where_clause: StateCondition::Always,

        // ==================== THEN Clause ====================
        // Actions to execute when triggered and conditions met

        then: vec![
            Action::Invoke {
                concept: "<ConceptName>".to_string(),
                action: "<action_name>".to_string(),
                parameters: vec![
                    ("<param_name>", Variable::Bound),  // Use bound variable
                ],
            },

            // Optional: Emit event for chaining
            Action::EmitEvent {
                event_name: "<EventName>".to_string(),
                parameters: vec![
                    ("<field_name>", Variable::Bound),
                ],
            },
        ],

        // Optional: Else clause (if WHERE fails)
        else_actions: None,
        // else_actions: Some(vec![
        //     Action::Invoke { ... },
        // ]),
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;
    use wkmp_common::legibility::SyncEngine;

    #[tokio::test]
    async fn test_sync_activates_on_event() {
        let mut engine = SyncEngine::new_for_test();
        engine.register(<sync_name>());

        // Register mock concepts:
        engine.register_concept(MockConceptA::new());
        engine.register_concept(MockConceptB::new());

        // Emit trigger event:
        engine.emit_event(Event::<TriggerEvent> {
            // Event fields
        });

        // Process events:
        engine.process_events().await;

        // Verify actions executed:
        assert!(engine.action_was_invoked("<ConceptName>", "<action_name>"));
    }

    #[tokio::test]
    async fn test_sync_respects_where_clause() {
        let mut engine = SyncEngine::new_for_test();
        engine.register(<sync_name>());

        // Set up state so WHERE clause fails:
        let mut concept = MockConcept::new();
        concept.set_state_to_reject_where_clause();
        engine.register_concept(concept);

        // Emit trigger event:
        engine.emit_event(Event::<TriggerEvent> { /* ... */ });

        // Process events:
        engine.process_events().await;

        // Verify actions NOT executed:
        assert!(!engine.action_was_invoked("<ConceptName>", "<action_name>"));
    }
}
```

---

## 9.4 Test Template

**File:** `<module>/tests/integration_<feature>.rs`

```rust
//! Integration test for <feature>
//!
//! Tests end-to-end flow through concepts and synchronizations

use wkmp_<module>::*;
use wkmp_common::legibility::*;

#[tokio::test]
async fn test_<feature>_end_to_end() {
    // ==================== ARRANGE ====================
    // Set up test environment with all concepts and syncs

    let app = TestApp::new().await;

    // Initialize concepts:
    let audio_player = AudioPlayer::new_for_test();
    let selection = PassageSelection::new_for_test();

    app.register_concept(audio_player);
    app.register_concept(selection);

    // Register synchronizations:
    app.register_sync(auto_select_when_queue_low());

    // ==================== ACT ====================
    // Trigger the feature

    let response = app.post("/api/play")
        .json(&json!({"passage_id": 42}))
        .send()
        .await;

    // ==================== ASSERT ====================
    // Verify expected outcomes

    // HTTP response:
    assert_eq!(response.status(), 200);

    // Concept state:
    let player = app.get_concept::<AudioPlayer>().await;
    assert_eq!(player.get_current_passage(), Some(PassageId::new(42)));

    // Events emitted:
    let events = app.drain_events().await;
    assert!(events.iter().any(|e| matches!(e, Event::PlaybackStarted { .. })));

    // Action trace:
    let trace = app.tracer.get_latest_flow().await;
    assert!(trace.contains_action("wkmp-ap::AudioPlayer::play_passage"));
    assert!(trace.contains_sync("AutoSelectWhenQueueLow"));

    // Provenance:
    let play_action = trace.find_action("play_passage").unwrap();
    let producing_sync = trace.find_producing_sync(play_action.id).unwrap();
    assert_eq!(producing_sync, "AutoSelectWhenQueueLow");
}

// ==================== TEST UTILITIES ====================

struct TestApp {
    // Test harness with mock infrastructure
}

impl TestApp {
    async fn new() -> Self {
        // Initialize test app
    }

    fn register_concept<C: Concept>(&mut self, concept: C) {
        // Register concept for testing
    }

    fn register_sync(&mut self, sync: Synchronization) {
        // Register synchronization
    }

    async fn get_concept<C: Concept>(&self) -> &C {
        // Retrieve concept instance
    }
}
```

---

## 9.5 Developer Interface Checklist

**For each module implementing legible patterns:**

### Concept Registration
- [ ] All concepts registered with sync engine
- [ ] Concept URIs follow `<module>::<ConceptName>` format
- [ ] Actions emit events for synchronization triggers
- [ ] Queries are side-effect free

### Synchronization Registration
- [ ] All syncs registered with sync engine
- [ ] Syncs have unique, descriptive names
- [ ] Event patterns correctly match concept actions
- [ ] WHERE clauses use queries only (no side effects)
- [ ] THEN clauses invoke concept actions via sync engine

### Action Tracing
- [ ] All concept actions record to tracer
- [ ] Action records include inputs and outputs
- [ ] Provenance edges created by sync engine
- [ ] Flow tokens propagate through sync chains

### Developer Interface Routes
- [ ] Database parameter `enable_dev_interface` added to settings table
- [ ] `/dev/` dashboard implemented
- [ ] `/dev/concepts/<name>` inspector available
- [ ] `/dev/syncs/<name>` monitor available
- [ ] `/dev/traces/<token>` viewer available
- [ ] `/dev/events/stream` SSE endpoint available
- [ ] Routes conditionally constructed based on database parameter (not build-time gating)
- [ ] Default value: `true` for dev builds, `false` for production builds

### Testing
- [ ] Unit tests for all concept actions
- [ ] Unit tests for all concept queries
- [ ] Synchronization activation tests
- [ ] Integration tests for critical flows
- [ ] Action trace verification in tests

---

## 9.6 Migration Checklist

**For each concept being extracted:**

### Preparation
- [ ] Concept identified and documented
- [ ] Dependencies mapped
- [ ] State identified (what's private)
- [ ] Actions identified (what modifies state)
- [ ] Queries identified (what reads state)

### Implementation
- [ ] Concept module created
- [ ] Private state encapsulated
- [ ] Actions implemented with event emissions
- [ ] Queries implemented (no side effects)
- [ ] Concept implements `Concept` trait
- [ ] Action tracing integrated

### Integration
- [ ] Event bus wired to concept
- [ ] Synchronizations defined for concept interactions
- [ ] Direct calls replaced with event-driven orchestration
- [ ] Legacy adapter created (if needed)

### Testing
- [ ] Unit tests written (100% action/query coverage)
- [ ] Synchronization tests written
- [ ] Integration tests verify end-to-end behavior
- [ ] Regression tests pass (legacy behavior preserved)

### Documentation
- [ ] Concept documented in module README
- [ ] Synchronizations documented
- [ ] Developer interface accessible
- [ ] Examples added to [07_wkmp_examples.md](07_wkmp_examples.md)

---

## 9.7 Quick Reference: Common Patterns

### Pattern 1: Conditional Action Based on State

```rust
Synchronization {
    name: "ConditionalAction".to_string(),
    when: EventPattern::EventType {
        event_name: "TriggerEvent".to_string(),
        bindings: vec![("data", Variable::Free)],
    },
    where_clause: StateCondition::Query {
        concept: "ConceptA".to_string(),
        query: "is_condition_met".to_string(),
        parameters: vec![],
        expectation: Expectation::Affirm,
    },
    then: vec![
        Action::Invoke {
            concept: "ConceptB".to_string(),
            action: "perform_action".to_string(),
            parameters: vec![("data", Variable::Bound)],
        },
    ],
}
```

### Pattern 2: Chained Actions

```rust
// Synchronization 1: Emit intermediate event
Synchronization {
    name: "Step1".to_string(),
    when: EventPattern::EventType {
        event_name: "StartEvent".to_string(),
        bindings: vec![("id", Variable::Free)],
    },
    where_clause: StateCondition::Always,
    then: vec![
        Action::Invoke {
            concept: "ConceptA".to_string(),
            action: "process".to_string(),
            parameters: vec![("id", Variable::Bound)],
        },
        Action::EmitEvent {
            event_name: "IntermediateEvent".to_string(),
            parameters: vec![("id", Variable::Bound)],
        },
    ],
}

// Synchronization 2: Continue chain
Synchronization {
    name: "Step2".to_string(),
    when: EventPattern::EventType {
        event_name: "IntermediateEvent".to_string(),
        bindings: vec![("id", Variable::Free)],
    },
    where_clause: StateCondition::Always,
    then: vec![
        Action::Invoke {
            concept: "ConceptB".to_string(),
            action: "finalize".to_string(),
            parameters: vec![("id", Variable::Bound)],
        },
    ],
}
```

### Pattern 3: Multi-Concept Coordination

```rust
Synchronization {
    name: "CoordinateMultipleConcepts".to_string(),
    when: EventPattern::EventType {
        event_name: "TriggerEvent".to_string(),
        bindings: vec![("id", Variable::Free)],
    },
    where_clause: StateCondition::And(vec![
        StateCondition::Query {
            concept: "ConceptA".to_string(),
            query: "is_ready".to_string(),
            parameters: vec![],
            expectation: Expectation::Affirm,
        },
        StateCondition::Query {
            concept: "ConceptB".to_string(),
            query: "is_available".to_string(),
            parameters: vec![],
            expectation: Expectation::Affirm,
        },
    ]),
    then: vec![
        Action::Invoke {
            concept: "ConceptA".to_string(),
            action: "prepare".to_string(),
            parameters: vec![("id", Variable::Bound)],
        },
        Action::Invoke {
            concept: "ConceptB".to_string(),
            action: "execute".to_string(),
            parameters: vec![("id", Variable::Bound)],
        },
        Action::Invoke {
            concept: "ConceptC".to_string(),
            action: "notify".to_string(),
            parameters: vec![("id", Variable::Bound)],
        },
    ],
}
```

---

## 9.8 Glossary

**Action:** Named operation on a concept that modifies state and may emit events

**Action Trace:** Directed acyclic graph of causally-related actions with provenance

**Concept:** Self-contained functional unit with private state and well-defined behavioral protocol

**Event Bus:** Publish-subscribe mechanism for concept communication

**Flow Token:** UUID grouping all actions triggered by single root event

**Legibility:** Direct correspondence between code structure and observed runtime behavior

**Provenance Edge:** Link between actions labeled with producing synchronization

**Query:** Read-only operation on concept state (no side effects)

**Synchronization:** Declarative rule orchestrating concept interactions (WHEN → WHERE → THEN)

**Variable Binding:** Extraction of values from events/outputs for use in subsequent actions

---

## Navigation

**Previous:** [08_migration_strategy.md](08_migration_strategy.md) - Migration strategy

**Back to Summary:** [00_SUMMARY.md](00_SUMMARY.md)

---

**END OF SECTION 09**

---

**END OF SPEC030 - SOFTWARE LEGIBILITY PATTERNS**
