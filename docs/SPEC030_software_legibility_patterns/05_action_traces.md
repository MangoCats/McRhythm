# Action Trace Architecture

**Part of:** [SPEC030 - Software Legibility Patterns](00_SUMMARY.md)

---

## 5.1 Recording Infrastructure

**Purpose:** Capture causal chains of operations with full provenance tracking

### Core Data Structures

```rust
/// Complete trace for a single flow (e.g., one HTTP request)
pub struct ActionTrace {
    /// Unique identifier grouping causally-related actions
    flow_token: Uuid,

    /// Root action that initiated this flow
    root_action_id: Uuid,

    /// All actions in this flow
    actions: HashMap<Uuid, ActionRecord>,

    /// Causal edges between actions
    provenance_edges: Vec<ProvenanceEdge>,

    /// Metadata
    created_at: i64,
    module: String,  // e.g., "wkmp-ap"
}

/// Single action within a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    /// Unique action identifier
    id: Uuid,

    /// Concept URI (e.g., "wkmp-ap::AudioPlayer")
    concept_uri: String,

    /// Action name (e.g., "play_passage")
    action: String,

    /// Input parameters (JSON)
    inputs: serde_json::Value,

    /// Output values (JSON)
    outputs: serde_json::Value,

    /// Unix timestamp (microseconds)
    timestamp: i64,

    /// Flow token grouping related actions
    flow_token: Uuid,

    /// Optional parent action (for nested calls)
    parent_id: Option<Uuid>,
}

/// Causal edge between actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEdge {
    /// Source action ID
    from_action: Uuid,

    /// Target action ID
    to_action: Uuid,

    /// Synchronization that created this edge
    synchronization: String,

    /// Timestamp when edge created
    created_at: i64,
}
```

### Tracer Interface

```rust
pub struct ActionTracer {
    /// In-memory buffer of recent traces
    traces: Arc<RwLock<LruCache<Uuid, ActionTrace>>>,

    /// Persistent storage backend
    storage: Arc<dyn TraceStorage>,

    /// Current flow token (thread-local)
    current_flow: Arc<RwLock<Option<Uuid>>>,
}

impl ActionTracer {
    /// Start a new flow (e.g., at HTTP request boundary)
    pub fn start_flow(&self, root_action: &str) -> Uuid {
        let flow_token = Uuid::new_v4();
        *self.current_flow.write().unwrap() = Some(flow_token);

        let mut traces = self.traces.write().unwrap();
        traces.put(flow_token, ActionTrace {
            flow_token,
            root_action_id: Uuid::new_v4(),
            actions: HashMap::new(),
            provenance_edges: Vec::new(),
            created_at: chrono::Utc::now().timestamp_micros(),
            module: env!("CARGO_PKG_NAME").to_string(),
        });

        flow_token
    }

    /// Record an action within current flow
    pub fn record_action(
        &self,
        concept_uri: String,
        action: String,
        inputs: serde_json::Value,
        outputs: serde_json::Value,
    ) -> Uuid {
        let action_id = Uuid::new_v4();
        let flow_token = self.current_flow.read().unwrap()
            .expect("No active flow - call start_flow first");

        let record = ActionRecord {
            id: action_id,
            concept_uri,
            action,
            inputs,
            outputs,
            timestamp: chrono::Utc::now().timestamp_micros(),
            flow_token,
            parent_id: None,
        };

        let mut traces = self.traces.write().unwrap();
        if let Some(trace) = traces.get_mut(&flow_token) {
            trace.actions.insert(action_id, record);
        }

        action_id
    }

    /// Record provenance edge (synchronization-mediated causality)
    pub fn record_provenance(
        &self,
        from_action: Uuid,
        to_action: Uuid,
        synchronization: String,
    ) {
        let flow_token = self.current_flow.read().unwrap()
            .expect("No active flow");

        let edge = ProvenanceEdge {
            from_action,
            to_action,
            synchronization,
            created_at: chrono::Utc::now().timestamp_micros(),
        };

        let mut traces = self.traces.write().unwrap();
        if let Some(trace) = traces.get_mut(&flow_token) {
            trace.provenance_edges.push(edge);
        }
    }

    /// End current flow and persist trace
    pub async fn end_flow(&self) -> Result<()> {
        let flow_token = self.current_flow.write().unwrap().take()
            .expect("No active flow");

        let trace = {
            let traces = self.traces.read().unwrap();
            traces.peek(&flow_token).cloned()
                .expect("Flow token not found")
        };

        // Persist to storage:
        self.storage.save_trace(&trace).await?;

        Ok(())
    }
}
```

---

## 5.2 Querying Action Traces

### Query Interface

```rust
#[async_trait]
pub trait TraceQuery {
    /// Get complete trace for flow token
    async fn get_trace(&self, flow_token: Uuid) -> Result<ActionTrace>;

    /// Find all actions matching criteria
    async fn find_actions(&self, filter: ActionFilter) -> Result<Vec<ActionRecord>>;

    /// Find producing synchronization for action
    async fn find_producing_sync(&self, action_id: Uuid) -> Result<Option<String>>;

    /// Get causal chain from root to action
    async fn get_causal_chain(&self, action_id: Uuid) -> Result<Vec<ActionRecord>>;

    /// Find all actions triggered by synchronization
    async fn find_by_sync(&self, sync_name: &str) -> Result<Vec<ActionRecord>>;
}

pub struct ActionFilter {
    /// Filter by concept URI (exact match or prefix)
    concept_uri: Option<String>,

    /// Filter by action name
    action: Option<String>,

    /// Filter by time range
    time_range: Option<(i64, i64)>,

    /// Filter by flow token
    flow_token: Option<Uuid>,

    /// Limit results
    limit: Option<usize>,
}
```

### Example Queries

**Query 1: "Why did passage 42 play?"**
```rust
let tracer = ActionTracer::global();

// Find the play_passage action:
let actions = tracer.find_actions(ActionFilter {
    concept_uri: Some("wkmp-ap::AudioPlayer".to_string()),
    action: Some("play_passage".to_string()),
    ..Default::default()
}).await?;

// Get the specific action with passage_id = 42:
let play_action = actions.iter()
    .find(|a| a.inputs["passage_id"] == json!(42))
    .expect("Action not found");

// Trace causal chain backwards:
let chain = tracer.get_causal_chain(play_action.id).await?;

// Output:
// [Web::handle_request]
//   ──(OnWebRequest)──> [Authentication::validate_token]
//   ──(OnAuthSuccess)──> [AudioPlayer::play_passage(42)]
```

**Query 2: "Which synchronization produced this action?"**
```rust
let producing_sync = tracer.find_producing_sync(action_id).await?;
// Output: Some("AutoSelectWhenQueueLow")
```

**Query 3: "What did AutoSelectWhenQueueLow trigger today?"**
```rust
let start_of_day = chrono::Utc::now()
    .date_naive()
    .and_hms_opt(0, 0, 0)
    .unwrap()
    .and_utc()
    .timestamp_micros();

let actions = tracer.find_by_sync("AutoSelectWhenQueueLow").await?;
let today_actions: Vec<_> = actions.iter()
    .filter(|a| a.timestamp >= start_of_day)
    .collect();

println!("AutoSelectWhenQueueLow triggered {} actions today", today_actions.len());
```

**Query 4: "Show execution path for flow token XYZ"**
```rust
let trace = tracer.get_trace(flow_token).await?;

// Build DAG for visualization:
let mut graph = petgraph::Graph::new();
let mut nodes = HashMap::new();

for (id, action) in &trace.actions {
    let node = graph.add_node(format!("{}::{}", action.concept_uri, action.action));
    nodes.insert(*id, node);
}

for edge in &trace.provenance_edges {
    let from = nodes[&edge.from_action];
    let to = nodes[&edge.to_action];
    graph.add_edge(from, to, edge.synchronization.clone());
}

// Render as DOT graph:
println!("{:?}", petgraph::dot::Dot::new(&graph));
```

---

## 5.3 Persistent Storage

### Storage Backends

**SQLite Backend (Development/Single-Node):**
```rust
pub struct SqliteTraceStorage {
    pool: SqlitePool,
}

impl SqliteTraceStorage {
    pub async fn new(db_path: &Path) -> Result<Self> {
        let pool = SqlitePool::connect(db_path.to_str().unwrap()).await?;

        // Create tables:
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS action_traces (
                flow_token TEXT PRIMARY KEY,
                root_action_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                module TEXT NOT NULL,
                trace_json TEXT NOT NULL
            )
        "#).execute(&pool).await?;

        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS action_records (
                id TEXT PRIMARY KEY,
                concept_uri TEXT NOT NULL,
                action TEXT NOT NULL,
                inputs TEXT NOT NULL,
                outputs TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                flow_token TEXT NOT NULL,
                FOREIGN KEY (flow_token) REFERENCES action_traces(flow_token)
            )
        "#).execute(&pool).await?;

        sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_action_records_concept
            ON action_records(concept_uri, action)
        "#).execute(&pool).await?;

        sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_action_records_timestamp
            ON action_records(timestamp)
        "#).execute(&pool).await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl TraceStorage for SqliteTraceStorage {
    async fn save_trace(&self, trace: &ActionTrace) -> Result<()> {
        let trace_json = serde_json::to_string(trace)?;

        sqlx::query(r#"
            INSERT INTO action_traces (flow_token, root_action_id, created_at, module, trace_json)
            VALUES (?, ?, ?, ?, ?)
        "#)
            .bind(trace.flow_token.to_string())
            .bind(trace.root_action_id.to_string())
            .bind(trace.created_at)
            .bind(&trace.module)
            .bind(trace_json)
            .execute(&self.pool)
            .await?;

        // Insert action records:
        for (id, action) in &trace.actions {
            sqlx::query(r#"
                INSERT INTO action_records
                (id, concept_uri, action, inputs, outputs, timestamp, flow_token)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            "#)
                .bind(id.to_string())
                .bind(&action.concept_uri)
                .bind(&action.action)
                .bind(serde_json::to_string(&action.inputs)?)
                .bind(serde_json::to_string(&action.outputs)?)
                .bind(action.timestamp)
                .bind(trace.flow_token.to_string())
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    async fn get_trace(&self, flow_token: Uuid) -> Result<ActionTrace> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT trace_json FROM action_traces WHERE flow_token = ?"
        )
            .bind(flow_token.to_string())
            .fetch_one(&self.pool)
            .await?;

        let trace: ActionTrace = serde_json::from_str(&row.0)?;
        Ok(trace)
    }
}
```

**In-Memory Backend (Testing):**
```rust
pub struct InMemoryTraceStorage {
    traces: Arc<RwLock<HashMap<Uuid, ActionTrace>>>,
}

#[async_trait]
impl TraceStorage for InMemoryTraceStorage {
    async fn save_trace(&self, trace: &ActionTrace) -> Result<()> {
        let mut traces = self.traces.write().unwrap();
        traces.insert(trace.flow_token, trace.clone());
        Ok(())
    }

    async fn get_trace(&self, flow_token: Uuid) -> Result<ActionTrace> {
        let traces = self.traces.read().unwrap();
        traces.get(&flow_token).cloned()
            .ok_or_else(|| anyhow::anyhow!("Trace not found"))
    }
}
```

---

## 5.4 Integration with Logging

**Structured Logging with Trace Context:**
```rust
use tracing::{info, warn, error, instrument};

impl AudioPlayer {
    #[instrument(
        skip(self),
        fields(
            flow_token = ?self.tracer.current_flow_token(),
            action = "play_passage",
        )
    )]
    pub async fn play_passage(&mut self, passage_id: PassageId) -> Result<()> {
        // Start action recording:
        let action_id = self.tracer.record_action(
            self.concept_uri(),
            "play_passage".to_string(),
            json!({"passage_id": passage_id}),
            json!(null),  // Output recorded at end
        );

        info!("Starting playback for passage {}", passage_id);

        // Perform action:
        match self.internal_play(passage_id).await {
            Ok(output) => {
                // Record successful output:
                self.tracer.update_action_output(action_id, json!({"started_at": output.timestamp}));
                info!("Playback started successfully");
                Ok(())
            }
            Err(e) => {
                // Record error:
                self.tracer.update_action_output(action_id, json!({"error": e.to_string()}));
                error!("Playback failed: {}", e);
                Err(e)
            }
        }
    }
}
```

**Benefits:**
- Trace context automatically propagated to logs (flow_token in log entries)
- Structured logs queryable via trace infrastructure
- Unified view of behavior (logs + action traces)

---

## 5.5 Visualization and Analysis

### DAG Visualization

**ASCII Representation:**
```
[Web::request] flow_token=abc123
    |
    ├──(OnWebRequest)──> [Auth::validate_token]
    |                         └─ Result: {"valid": true, "user_id": 42}
    |
    ├──(OnAuthSuccess)──> [AudioPlayer::play_passage]
    |                         └─ Input: {"passage_id": 123}
    |                         └─ Output: {"started_at": 1699564800}
    |
    └──(RecordPlaybackForCooldown)──> [Cooldown::record_play]
                                         └─ Input: {"song_id": 5, "timestamp": 1699564800}
```

**GraphViz DOT Format:**
```rust
impl ActionTrace {
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph ActionTrace {\n");
        dot.push_str("    rankdir=LR;\n");

        // Nodes:
        for (id, action) in &self.actions {
            dot.push_str(&format!(
                "    \"{}\" [label=\"{}\\n{}\\nInputs: {}\\nOutputs: {}\"];\n",
                id,
                action.concept_uri,
                action.action,
                format_json_compact(&action.inputs),
                format_json_compact(&action.outputs)
            ));
        }

        // Edges:
        for edge in &self.provenance_edges {
            dot.push_str(&format!(
                "    \"{}\" -> \"{}\" [label=\"{}\"];\n",
                edge.from_action,
                edge.to_action,
                edge.synchronization
            ));
        }

        dot.push_str("}\n");
        dot
    }
}
```

### Statistical Analysis

**Common Patterns:**
```rust
pub struct TraceAnalyzer {
    storage: Arc<dyn TraceStorage>,
}

impl TraceAnalyzer {
    /// Find most frequently triggered synchronizations
    pub async fn top_synchronizations(&self, limit: usize) -> Result<Vec<(String, usize)>> {
        let traces = self.storage.get_all_traces().await?;
        let mut counts = HashMap::new();

        for trace in traces {
            for edge in &trace.provenance_edges {
                *counts.entry(edge.synchronization.clone()).or_insert(0) += 1;
            }
        }

        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        Ok(sorted.into_iter().take(limit).collect())
    }

    /// Find longest causal chains
    pub async fn longest_chains(&self, limit: usize) -> Result<Vec<(Uuid, usize)>> {
        let traces = self.storage.get_all_traces().await?;
        let mut chains = Vec::new();

        for trace in traces {
            let depth = self.calculate_max_depth(&trace);
            chains.push((trace.flow_token, depth));
        }

        chains.sort_by_key(|(_, depth)| std::cmp::Reverse(*depth));
        Ok(chains.into_iter().take(limit).collect())
    }

    /// Calculate performance metrics for synchronizations
    pub async fn sync_performance(&self, sync_name: &str) -> Result<PerformanceMetrics> {
        let actions = self.storage.find_by_sync(sync_name).await?;
        let mut durations = Vec::new();

        for action in actions {
            if let Some(started_at) = action.outputs.get("started_at").and_then(|v| v.as_i64()) {
                let duration = started_at - action.timestamp;
                durations.push(duration);
            }
        }

        Ok(PerformanceMetrics {
            count: durations.len(),
            mean: durations.iter().sum::<i64>() / durations.len() as i64,
            median: durations[durations.len() / 2],
            p95: durations[(durations.len() as f64 * 0.95) as usize],
            p99: durations[(durations.len() as f64 * 0.99) as usize],
        })
    }
}
```

---

## Navigation

**Previous:** [04_synchronization_patterns.md](04_synchronization_patterns.md) - Synchronization patterns
**Next:** [06_dev_interface.md](06_dev_interface.md) - Visible developer interface

**Back to Summary:** [00_SUMMARY.md](00_SUMMARY.md)

---

**END OF SECTION 05**
