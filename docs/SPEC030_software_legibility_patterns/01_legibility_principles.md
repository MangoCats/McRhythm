# Legibility Principles

**Part of:** [SPEC030 - Software Legibility Patterns](00_SUMMARY.md)

---

## 1.1 Definition

**Legibility** = Direct correspondence between code and observed behavior

### Illegible Systems Exhibit

- Scattered implementation of single features across multiple files/modules
- Hidden dependencies between components
- Inability to predict runtime behavior from code inspection
- Difficulty tracing desired behavior changes to required code modifications

### Legible Systems Achieve

- One concept = one coherent functional unit with predictable behavior
- Explicit synchronization contracts between concepts
- Observable runtime state matching code structure
- Traceable causal chains from requests to effects

---

## 1.2 Three Pillars of Legibility

### Incrementality

**Definition:** Small functional increments via localized changes

**Characteristics:**
- No system-wide refactoring required for feature additions
- Clear boundaries for where changes belong
- New functionality added by creating concepts/syncs, not modifying existing

**Example:**
```rust
// Adding cooldown tracking (legible):
// Step 1: Create new Cooldown concept
// Step 2: Add RecordPlaybackForCooldown synchronization
// Step 3: Done - no changes to existing AudioPlayer or PassageSelection

// Adding cooldown tracking (illegible):
// Step 1: Modify AudioPlayer to track timestamps
// Step 2: Modify PassageSelection to check timestamps
// Step 3: Modify database access in both
// Step 4: Update controllers, models, views
// Step 5: Fix conflicts with existing features
```

### Integrity

**Definition:** Extending functionality preserves existing behavior

**Characteristics:**
- Explicit contracts prevent accidental breakage
- Synchronizations define interaction boundaries
- Concept independence prevents ripple effects

**Example:**
```rust
// Adding authentication (legible):
Synchronization {
    name: "EnforceAuthenticationOnPlayback",
    when: Event::Action("AudioPlayer", "play"),
    where: Query::IsAuthenticated(user_id),
    then: vec![Action::Allow],
    else: vec![Action::Reject(Unauthorized)],
}
// Existing playback logic unchanged - authentication enforced via sync

// Adding authentication (illegible):
// Modify AudioPlayer.play() to check auth
// Modify PassageSelection to pass user_id
// Update all call sites
// Risk: breaking existing callers that don't have user context
```

### Transparency

**Definition:** What changed at build time is visible in code structure; what executed at runtime is traceable

**Build-Time Transparency:**
- New concept = new file/module
- New synchronization = new declarative rule
- Changes localized to concept/sync boundaries

**Runtime Transparency:**
- Action traces show execution path
- Provenance edges identify producing synchronizations
- Flow tokens group causally-related actions

**Example:**
```rust
// Query: "Why did passage 42 play?"
let trace = tracer.get_flow_trace(flow_token);
// Output:
// [Web::request] ──(PlaybackRequested)──> [AudioPlayer::play(42)]

// Query: "Which sync produced this action?"
let sync = tracer.find_producing_sync(action_id);
// Output: "PlaybackRequested"

// Query: "What caused queue to refill?"
let cause = tracer.find_root_cause(refill_action_id);
// Output: [AudioPlayer::queue_depth_changed] ──(AutoSelectWhenQueueLow)──> [Timeslot::get_target] → [Selection::select] → [Player::enqueue]
```

---

## 1.3 Comparison to Traditional Architecture

| Aspect | Traditional Layered | Microservices | Legible (Concept-Based) |
|--------|---------------------|---------------|-------------------------|
| **Module Boundary** | Technical (MVC layers) | Service ownership | Functional purpose |
| **Dependencies** | Cross-layer allowed | Network boundaries | Concepts independent, sync explicit |
| **Feature Location** | Scattered across layers | Sometimes split across services | Single concept encapsulation |
| **Interaction Model** | Direct calls | HTTP/RPC | Event-driven synchronizations |
| **Traceability** | Difficult (scattered code) | Difficult (distributed traces) | Direct (action provenance) |
| **Testing** | Mock layers | Mock services | Test concepts, test syncs separately |
| **LLM Compatibility** | Poor (scattered changes) | Medium (service boundaries) | High (declarative contracts) |

### Traditional Layered Architecture (Illegible)

**Structure:**
```
Controller Layer ──> Model Layer ──> Database Layer
     ↑                  ↑                  ↑
     │                  │                  │
  Violations       Violations         Violations
  (cross-layer)    (bypass)          (direct access)
```

**Problems:**
- Controllers bypass models to access database directly
- Business logic scattered across controller, model, and database layers
- Adding feature requires touching multiple layers
- Cross-cutting concerns (auth, logging) tangled throughout

**Example Violation (RealWorld app from paper):**
```javascript
// Article controller calling User model directly (layer violation)
router.get('/articles/:slug', async (req, res) => {
    const article = await Article.findOne({slug: req.params.slug});
    const author = await User.findById(article.author);  // Cross-entity dependency
    const isFavorited = await Favorite.exists({...});     // Business logic in controller
    res.json({article, author, isFavorited});
});
```

### Microservices Architecture (Partially Legible)

**Structure:**
```
User Service    Article Service    Favorite Service
     │                │                   │
     └────────────────┴───────────────────┘
              HTTP/RPC Network
```

**Advantages:**
- Service boundaries prevent some tangling
- Deployment independence

**Remaining Problems:**
- Tangled dependencies within each service
- Distributed tracing difficult
- Functionality still scattered inside services
- Network calls obscure causality

### Legible (Concept-Based) Architecture

**Structure:**
```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│ AudioPlayer │      │  Cooldown   │      │  Timeslot   │
│  Concept    │      │  Concept    │      │  Concept    │
└──────┬──────┘      └──────┬──────┘      └──────┬──────┘
       │                    │                    │
       └────────────────────┴────────────────────┘
                  Synchronization Engine
                (Declarative Orchestration)
```

**Advantages:**
- Concepts completely independent (no direct dependencies)
- Synchronizations make interactions explicit
- Action traces provide causal provenance
- LLM-compatible declarative contracts
- Testable in isolation

**Example (WKMP):**
```rust
// Concepts:
impl AudioPlayer {
    pub async fn play(&mut self, passage_id: PassageId) -> Result<()> {
        // ONLY playback logic - no cooldown, no selection, no auth
        self.current_passage = Some(passage_id);
        self.emit_event(Event::PlaybackStarted(passage_id));
        Ok(())
    }
}

impl Cooldown {
    pub async fn record(&mut self, song_id: SongId, timestamp: i64) -> Result<()> {
        // ONLY cooldown tracking - no playback logic
        self.last_played.insert(song_id, timestamp);
        Ok(())
    }
}

// Synchronization (orchestrates independently):
Synchronization {
    name: "RecordPlaybackForCooldown",
    when: Event::PlaybackStarted(passage_id),
    where: Query::Always,
    then: vec![
        Action::Invoke("Cooldown", "record", vec!["song_id", "timestamp"]),
    ]
}
```

---

## 1.4 Why Legibility Matters

### For Human Developers

**Cognitive Load Reduction:**
- Understand one concept at a time (no scattered implementation)
- Trace behavior via action graphs (no detective work)
- Change behavior by editing declarative rules (no refactoring)

**Maintenance Efficiency:**
- Localized changes reduce merge conflicts
- Explicit contracts prevent regressions
- Observable runtime matches mental model

### For AI-Assisted Development

**LLM Code Generation:**
- Declarative synchronizations match LLM output patterns
- Concepts provide clear context boundaries (fits in LLM context window)
- Action traces enable verification ("did the LLM generate correct behavior?")

**Incremental Safety:**
- New concepts don't break existing ones (independence)
- New syncs compose with existing syncs (declarative)
- Verifiable contracts prevent "vibe coding" failures

**Research Evidence (Meng & Jackson 2024):**
> "LLM-generated synchronizations required minimal iteration—legible patterns are readable enough for both humans and AI to understand consistently."

### For Operations & Debugging

**Production Debugging:**
- Replay action sequences to reproduce issues
- Query provenance: "Which sync caused this state?"
- No instrumentation required (dev interface built-in)

**Security Auditing:**
- Complete audit trail for sensitive operations
- Trace authentication/authorization enforcement
- Verify compliance with security policies

**Performance Analysis:**
- Synchronization timing metrics
- Identify bottlenecks in orchestration
- Optimize critical paths

---

## 1.5 Legibility Anti-Patterns

### Anti-Pattern 1: God Concepts

**Symptom:** Single concept that coordinates many others

**Example:**
```rust
// BAD: Orchestrator concept
impl Orchestrator {
    async fn handle_playback(&mut self) -> Result<()> {
        let flavor = self.timeslot.get_target();
        let passage = self.selection.select(flavor).await?;
        self.player.play(passage).await?;
        self.cooldown.record(passage.song_id).await?;
        // Orchestrator becomes coupling point - defeats independence
    }
}
```

**Solution:** Use synchronizations for orchestration, not concepts

### Anti-Pattern 2: Concept-to-Concept Calls

**Symptom:** Concepts directly invoking each other's methods

**Example:**
```rust
// BAD: Direct coupling
impl PassageSelection {
    async fn select_if_queue_low(&mut self, player: &AudioPlayer) -> Result<()> {
        if player.queue_depth() < 2 {  // Direct dependency
            let passage = self.select_next().await?;
            player.enqueue(passage).await?;  // Direct call
        }
    }
}
```

**Solution:** Emit events, let synchronizations orchestrate

### Anti-Pattern 3: Stateful Synchronizations

**Symptom:** Synchronizations storing state between activations

**Example:**
```rust
// BAD: Sync with state
struct AutoSelectSync {
    last_selected_timestamp: i64,  // State in sync
}
```

**Solution:** State belongs in concepts, syncs remain stateless rules

### Anti-Pattern 4: Imperative Synchronizations

**Symptom:** Synchronizations with complex control flow

**Example:**
```rust
// BAD: Imperative logic
async fn execute_actions(&self) -> Result<()> {
    if complex_condition() {
        for item in collection {
            if another_condition(item) {
                // Nested imperative logic
            }
        }
    }
}
```

**Solution:** Complex logic belongs in concept actions, syncs remain declarative

---

## 1.6 Measuring Legibility

### Quantitative Metrics

**Concept Independence Score:**
```
Score = (concepts with 0 dependencies) / (total concepts)
Target: >95%
```

**Traceability Coverage:**
```
Score = (actions with provenance) / (total actions)
Target: 100%
```

**Synchronization Locality:**
```
Score = (syncs affecting <3 concepts) / (total syncs)
Target: >80%
```

### Qualitative Assessment

**Can a developer answer these questions in <5 minutes?**
- [ ] Where is feature X implemented? (Should be: one concept)
- [ ] What happens when event Y occurs? (Should be: check syncs for event Y)
- [ ] Why did action Z execute? (Should be: query action trace)
- [ ] How do concepts A and B interact? (Should be: find sync between them)

**Can an LLM generate correct code without iteration?**
- [ ] Add new concept (Should be: single file, no refactoring)
- [ ] Add new synchronization (Should be: declarative rule)
- [ ] Modify existing behavior (Should be: edit one sync or concept)

---

## Navigation

**Previous:** [00_SUMMARY.md](00_SUMMARY.md) - Executive summary
**Next:** [02_core_patterns.md](02_core_patterns.md) - Core structural patterns

**Back to Summary:** [00_SUMMARY.md](00_SUMMARY.md)

---

**END OF SECTION 01**
