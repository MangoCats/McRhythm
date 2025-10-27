//! Test server wrapper for integration tests
//!
//! Provides a programmatically controllable wkmp-ap server instance
//! with in-memory database and event monitoring capabilities.

use std::sync::Arc;
use std::time::{Duration, Instant};
use axum::Router;
use serde_json::Value;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use tokio::sync::broadcast;
use uuid::Uuid;

use wkmp_ap::api::server::AppContext;
use wkmp_ap::playback::engine::PlaybackEngine;
use wkmp_ap::state::SharedState;
use wkmp_common::events::WkmpEvent;

/// Test server instance with full API and playback engine
pub struct TestServer {
    router: Router,
    state: Arc<SharedState>,
    engine: Arc<PlaybackEngine>,
    db_pool: Pool<Sqlite>,
}

impl TestServer {
    /// Start a new test server with in-memory database
    pub async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        // Create in-memory database
        let db_pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await?;

        // Create test schema
        Self::create_test_schema(&db_pool).await?;

        // Create shared state
        let state = Arc::new(SharedState::new());

        // Create playback engine
        let engine = Arc::new(
            PlaybackEngine::new(db_pool.clone(), Arc::clone(&state)).await?
        );

        // Start engine background tasks
        engine.start().await?;

        // Get volume Arc from engine
        let volume = engine.get_volume_arc();

        // Create application context
        let ctx = AppContext {
            state: Arc::clone(&state),
            engine: Arc::clone(&engine),
            db_pool: db_pool.clone(),
            volume,
        };

        // Build router with same structure as main server
        use axum::routing::{get, post, delete};

        let router = Router::new()
            .route("/health", get(wkmp_ap::api::handlers::health))
            .route("/audio/volume", get(wkmp_ap::api::handlers::get_volume))
            .route("/audio/volume", post(wkmp_ap::api::handlers::set_volume))
            .route("/audio/devices", get(wkmp_ap::api::handlers::list_audio_devices))
            .route("/audio/device", get(wkmp_ap::api::handlers::get_audio_device))
            .route("/audio/device", post(wkmp_ap::api::handlers::set_audio_device))
            .route("/playback/enqueue", post(wkmp_ap::api::handlers::enqueue_passage))
            .route("/playback/queue/:queue_entry_id", delete(wkmp_ap::api::handlers::remove_from_queue))
            .route("/playback/queue/clear", post(wkmp_ap::api::handlers::clear_queue))
            .route("/playback/queue/reorder", post(wkmp_ap::api::handlers::reorder_queue_entry))
            .route("/playback/play", post(wkmp_ap::api::handlers::play))
            .route("/playback/pause", post(wkmp_ap::api::handlers::pause))
            .route("/playback/next", post(wkmp_ap::api::handlers::skip_next))
            .route("/playback/previous", post(wkmp_ap::api::handlers::skip_previous))
            .route("/playback/seek", post(wkmp_ap::api::handlers::seek))
            .route("/playback/queue", get(wkmp_ap::api::handlers::get_queue))
            .route("/playback/state", get(wkmp_ap::api::handlers::get_playback_state))
            .route("/playback/position", get(wkmp_ap::api::handlers::get_position))
            .route("/playback/buffer_status", get(wkmp_ap::api::handlers::get_buffer_status))
            .route("/files/browse", get(wkmp_ap::api::handlers::browse_files))
            .with_state(ctx);

        Ok(TestServer {
            router,
            state,
            engine,
            db_pool,
        })
    }

    /// Create minimal database schema for tests
    async fn create_test_schema(pool: &Pool<Sqlite>) -> Result<(), Box<dyn std::error::Error>> {
        // Settings table
        sqlx::query(
            r#"
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Queue table
        sqlx::query(
            r#"
            CREATE TABLE queue (
                guid TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                passage_guid TEXT,
                play_order INTEGER NOT NULL,
                start_time_ms INTEGER,
                end_time_ms INTEGER,
                lead_in_point_ms INTEGER,
                lead_out_point_ms INTEGER,
                fade_in_point_ms INTEGER,
                fade_out_point_ms INTEGER,
                fade_in_curve TEXT,
                fade_out_curve TEXT
            )
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Subscribe to SSE events
    pub async fn subscribe_events(&self) -> EventStream {
        let receiver = self.state.subscribe_events();
        EventStream {
            receiver,
            start_time: Instant::now(),
        }
    }

    /// Make an HTTP request to the test server
    pub async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<(axum::http::StatusCode, Option<Value>), Box<dyn std::error::Error>> {
        use axum::body::Body;
        use axum::http::{Request, Method};
        use tower::Service;

        let method = match method {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "DELETE" => Method::DELETE,
            "PUT" => Method::PUT,
            _ => return Err(format!("Unsupported method: {}", method).into()),
        };

        let mut request_builder = Request::builder()
            .method(method)
            .uri(path);

        if body.is_some() {
            request_builder = request_builder
                .header("content-type", "application/json");
        }

        let request = if let Some(json_body) = body {
            request_builder.body(Body::from(json_body.to_string()))?
        } else {
            request_builder.body(Body::empty())?
        };

        let response = self.router.clone()
            .call(request)
            .await?;

        let status = response.status();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await?;

        let json_body = if !body.is_empty() {
            Some(serde_json::from_slice(&body)?)
        } else {
            None
        };

        Ok((status, json_body))
    }

    /// Enqueue a passage for playback
    pub async fn enqueue_passage(
        &self,
        passage: PassageRequest,
    ) -> Result<Uuid, Box<dyn std::error::Error>> {
        let body = serde_json::to_value(&passage)?;
        let (status, response) = self.request("POST", "/playback/enqueue", Some(body)).await?;

        if !status.is_success() {
            return Err(format!("Enqueue failed: {:?}", response).into());
        }

        let queue_entry_id = response
            .and_then(|v| v.get("queue_entry_id").and_then(|id| id.as_str().map(String::from)))
            .ok_or("Missing queue_entry_id in response")?;

        Ok(Uuid::parse_str(&queue_entry_id)?)
    }

    /// Get current queue
    pub async fn get_queue(&self) -> Result<Vec<QueueEntry>, Box<dyn std::error::Error>> {
        let (status, response) = self.request("GET", "/playback/queue", None).await?;

        if !status.is_success() {
            return Err(format!("Get queue failed: {:?}", response).into());
        }

        let queue_array = response
            .and_then(|v| v.get("queue").cloned())
            .ok_or("Missing queue in response")?;

        Ok(serde_json::from_value(queue_array)?)
    }

    /// Start playback
    pub async fn play(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (status, _) = self.request("POST", "/playback/play", None).await?;

        if !status.is_success() {
            return Err("Play failed".into());
        }

        Ok(())
    }

    /// Pause playback
    pub async fn pause(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (status, _) = self.request("POST", "/playback/pause", None).await?;

        if !status.is_success() {
            return Err("Pause failed".into());
        }

        Ok(())
    }

    /// Skip to next passage
    pub async fn skip_next(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (status, _) = self.request("POST", "/playback/next", None).await?;

        if !status.is_success() {
            return Err("Skip failed".into());
        }

        Ok(())
    }

    /// Check server health
    pub async fn check_health(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let (status, response) = self.request("GET", "/health", None).await?;

        if !status.is_success() {
            return Err("Health check failed".into());
        }

        response.ok_or("Missing health response".into())
    }

    /// Get active buffer count (for memory leak testing)
    pub async fn get_active_buffer_count(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let (status, response) = self.request("GET", "/playback/buffer_status", None).await?;

        if !status.is_success() {
            return Err("Buffer status failed".into());
        }

        let count = response
            .and_then(|v| v.get("active_buffers").and_then(|c| c.as_u64()))
            .ok_or("Missing active_buffers in response")?;

        Ok(count as usize)
    }

    /// Get playback state
    pub async fn get_playback_state(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let (status, response) = self.request("GET", "/playback/state", None).await?;

        if !status.is_success() {
            return Err("Get playback state failed".into());
        }

        response.ok_or("Missing playback state response".into())
    }
}

/// SSE event stream wrapper
pub struct EventStream {
    pub receiver: broadcast::Receiver<WkmpEvent>,
    pub start_time: Instant,
}

impl EventStream {
    /// Wait for next event (indefinitely)
    pub async fn next(&mut self) -> Option<WkmpEvent> {
        self.receiver.recv().await.ok()
    }

    /// Wait for next event with timeout
    pub async fn next_timeout(&mut self, timeout: Duration) -> Option<WkmpEvent> {
        tokio::time::timeout(timeout, self.receiver.recv())
            .await
            .ok()
            .and_then(|r| r.ok())
    }

    /// Wait for specific event type
    pub async fn wait_for(
        &mut self,
        event_type: &str,
        timeout: Duration,
    ) -> Option<WkmpEvent> {
        let deadline = Instant::now() + timeout;

        loop {
            if Instant::now() > deadline {
                return None;
            }

            let remaining = deadline.duration_since(Instant::now());
            if let Some(event) = self.next_timeout(remaining).await {
                if event.event_type() == event_type {
                    return Some(event);
                }
            } else {
                return None;
            }
        }
    }

    /// Collect events matching type
    pub async fn take_matching(
        &mut self,
        event_type: &str,
        count: usize,
        timeout: Duration,
    ) -> Option<Vec<WkmpEvent>> {
        let mut events = Vec::new();
        let deadline = Instant::now() + timeout;

        while events.len() < count {
            if Instant::now() > deadline {
                return None;
            }

            let remaining = deadline.duration_since(Instant::now());
            if let Some(event) = self.next_timeout(remaining).await {
                if event.event_type() == event_type {
                    events.push(event);
                }
            } else {
                return None;
            }
        }

        Some(events)
    }
}

/// Passage enqueue request
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PassageRequest {
    pub file_path: String,
    pub start_time_ms: Option<u64>,
    pub end_time_ms: Option<u64>,
    pub fade_in_point_ms: Option<u64>,
    pub fade_out_point_ms: Option<u64>,
    pub lead_in_point_ms: Option<u64>,
    pub lead_out_point_ms: Option<u64>,
    pub fade_in_curve: Option<String>,
    pub fade_out_curve: Option<String>,
}

/// Queue entry response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueEntry {
    pub queue_entry_id: String,
    pub passage_id: Option<String>,
    pub file_path: String,
}

/// Builder for test passages
pub struct PassageBuilder {
    file_path: String,
    start_time_ms: Option<u64>,
    end_time_ms: Option<u64>,
    fade_in_point_ms: Option<u64>,
    fade_out_point_ms: Option<u64>,
    lead_in_point_ms: Option<u64>,
    lead_out_point_ms: Option<u64>,
    fade_in_curve: Option<String>,
    fade_out_curve: Option<String>,
}

impl PassageBuilder {
    pub fn new() -> Self {
        Self {
            file_path: String::new(),
            start_time_ms: None,
            end_time_ms: None,
            fade_in_point_ms: None,
            fade_out_point_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: None,
            fade_in_curve: None,
            fade_out_curve: None,
        }
    }

    pub fn file(mut self, path: impl Into<String>) -> Self {
        self.file_path = path.into();
        self
    }

    pub fn duration_seconds(mut self, seconds: f64) -> Self {
        self.start_time_ms = Some(0);
        self.end_time_ms = Some((seconds * 1000.0) as u64);
        self
    }

    pub fn lead_out_point_seconds(mut self, seconds: f64) -> Self {
        self.lead_out_point_ms = Some((seconds * 1000.0) as u64);
        self
    }

    pub fn fade_out_point_seconds(mut self, seconds: f64) -> Self {
        self.fade_out_point_ms = Some((seconds * 1000.0) as u64);
        self
    }

    pub fn lead_in_point_seconds(mut self, seconds: f64) -> Self {
        self.lead_in_point_ms = Some((seconds * 1000.0) as u64);
        self
    }

    pub fn fade_in_point_seconds(mut self, seconds: f64) -> Self {
        self.fade_in_point_ms = Some((seconds * 1000.0) as u64);
        self
    }

    pub fn fade_out_curve(mut self, curve: wkmp_common::FadeCurve) -> Self {
        self.fade_out_curve = Some(format!("{:?}", curve));
        self
    }

    pub fn fade_in_curve(mut self, curve: wkmp_common::FadeCurve) -> Self {
        self.fade_in_curve = Some(format!("{:?}", curve));
        self
    }

    pub fn build(self) -> PassageRequest {
        PassageRequest {
            file_path: self.file_path,
            start_time_ms: self.start_time_ms,
            end_time_ms: self.end_time_ms,
            fade_in_point_ms: self.fade_in_point_ms,
            fade_out_point_ms: self.fade_out_point_ms,
            lead_in_point_ms: self.lead_in_point_ms,
            lead_out_point_ms: self.lead_out_point_ms,
            fade_in_curve: self.fade_in_curve,
            fade_out_curve: self.fade_out_curve,
        }
    }
}
