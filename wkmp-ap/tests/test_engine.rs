//! Test harness for PlaybackEngine integration tests
//!
//! Provides TestEngine wrapper with:
//! - In-memory SQLite database for isolation
//! - Test-friendly interface for queue and chain operations
//! - State inspection helpers for verification

use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;
use sqlx::{Pool, Sqlite};
use wkmp_ap::state::SharedState;

/// Test wrapper around PlaybackEngine for integration testing
pub struct TestEngine {
    pub engine: Arc<wkmp_ap::playback::engine::PlaybackEngine>,
    pub db_pool: Pool<Sqlite>,
    pub state: Arc<SharedState>,
    _temp_dir: TempDir,
}

/// Chain state information for test verification
#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub queue_entry_id: Option<Uuid>,
    pub slot_index: usize,
    pub queue_position: Option<u32>,
    pub buffer_fill_percent: f32,
}

/// Queue entry information for test verification
#[derive(Debug, Clone)]
pub struct QueueEntryInfo {
    pub queue_entry_id: Uuid,
    pub passage_id: Option<Uuid>,
    pub play_order: i64,
}

impl TestEngine {
    /// Create new test engine with specified maximum decode streams
    pub async fn new(max_streams: usize) -> anyhow::Result<Self> {
        use std::time::Duration;

        // Create temp directory for test database
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test_wkmp.db");

        // Create database connection
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
        let db_pool = sqlx::SqlitePool::connect(&db_url).await?;

        // Enable foreign keys and WAL mode
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&db_pool)
            .await?;
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&db_pool)
            .await?;

        // Create minimal schema (same as in engine tests)
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
        .execute(&db_pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT,
                value_int INTEGER,
                value_real REAL,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&db_pool)
        .await?;

        // Insert test settings
        Self::init_test_settings(&db_pool, max_streams).await?;

        // Create shared state
        let state = Arc::new(SharedState::new());

        // Create playback engine
        let engine = Arc::new(
            wkmp_ap::playback::engine::PlaybackEngine::new(
                db_pool.clone(),
                state.clone(),
            ).await?
        );

        // Give engine components time to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(Self {
            engine,
            db_pool,
            state,
            _temp_dir: temp_dir,
        })
    }

    /// Initialize test settings in database
    async fn init_test_settings(pool: &Pool<Sqlite>, max_streams: usize) -> anyhow::Result<()> {
        // All settings use the value column (TEXT) and are parsed via FromStr
        let settings = vec![
            ("maximum_decode_streams", max_streams.to_string()),
            ("volume_level", "0.8".to_string()),
            ("minimum_buffer_threshold_ms", "1000".to_string()),
            ("position_event_interval_ms", "1000".to_string()),
            ("decoder_resume_hysteresis_samples", "44100".to_string()),
            ("ring_buffer_grace_period_ms", "500".to_string()),
            ("mixer_check_interval_ms", "10".to_string()),
            ("mixer_min_start_level_frames", "44100".to_string()),
            ("audio_buffer_size", "2208".to_string()),
            ("playout_ringbuffer_capacity", "661941".to_string()),
            ("playout_ringbuffer_headroom", "4410".to_string()),
        ];

        for (key, value) in settings {
            sqlx::query(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)"
            )
            .bind(key)
            .bind(value)
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    /// Enqueue a test audio file
    ///
    /// Note: File must exist on disk. Use create_test_audio_file() helper.
    pub async fn enqueue_file(&self, path: PathBuf) -> anyhow::Result<Uuid> {
        let queue_entry_id = self.engine.enqueue_file(path).await?;

        // Give system time to process enqueue and assign chains
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Ok(queue_entry_id)
    }

    /// Remove queue entry by ID
    pub async fn remove_queue_entry(&self, id: Uuid) -> anyhow::Result<()> {
        // Engine handles both database and in-memory removal
        let _removed = self.engine.remove_queue_entry(id).await;

        // Give system time to process removal and cleanup chains
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Ok(())
    }

    /// Start playback
    pub async fn play(&self) -> anyhow::Result<()> {
        self.engine.play().await?;

        // Give system time to start playback
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(())
    }

    /// Get buffer chain state for all active chains
    pub async fn get_buffer_chains(&self) -> Vec<ChainInfo> {
        let assignments = self.engine.test_get_chain_assignments().await;
        let queue = self.get_queue().await;

        let mut chains = Vec::new();

        for (queue_entry_id, slot_index) in assignments {
            // Find queue position for this entry
            let queue_position = queue.iter()
                .position(|e| e.queue_entry_id == queue_entry_id)
                .map(|pos| pos as u32);

            // Get buffer fill level from buffer manager
            let buffer_fill_percent = self.engine.test_get_buffer_fill_percent(queue_entry_id).await
                .unwrap_or(0.0);

            chains.push(ChainInfo {
                queue_entry_id: Some(queue_entry_id),
                slot_index,
                queue_position,
                buffer_fill_percent,
            });
        }

        // Sort by slot_index for consistent ordering
        chains.sort_by_key(|c| c.slot_index);

        chains
    }

    /// Get current queue state
    pub async fn get_queue(&self) -> Vec<QueueEntryInfo> {
        let entries = self.engine.test_get_queue_entries_from_db().await
            .unwrap_or_default();

        entries.into_iter().map(|e| {
            QueueEntryInfo {
                queue_entry_id: e.queue_entry_id,
                passage_id: e.passage_id,
                play_order: e.play_order,
            }
        }).collect()
    }

    /// Get chain index for specific queue entry
    pub async fn get_chain_index(&self, queue_entry_id: Uuid) -> Option<usize> {
        let assignments = self.engine.test_get_chain_assignments().await;
        assignments.get(&queue_entry_id).copied()
    }

    /// Get current decoder target (which buffer being filled)
    pub async fn get_decoder_target(&self) -> Option<Uuid> {
        self.engine.test_get_decoder_target().await
    }

    /// Get chain assignments generation counters
    ///
    /// Returns `(current_generation, last_observed_generation)`.
    /// When these differ, re-evaluation is pending.
    pub async fn get_generation_counter(&self) -> (u64, u64) {
        self.engine.test_get_generation_counter().await
    }

    /// Wait for generation counter to change (re-evaluation occurred)
    ///
    /// Returns `true` if generation changed within timeout, `false` if timeout.
    pub async fn wait_for_generation_change(&self, timeout_ms: u64) -> bool {
        self.engine.test_wait_for_generation_change(timeout_ms).await
    }
}

/// Create a minimal test audio file (100ms silence)
///
/// Returns path to created file in temp directory
pub fn create_test_audio_file(temp_dir: &TempDir, index: usize) -> anyhow::Result<PathBuf> {
    use std::fs::File;
    use std::io::Write;

    // Create a minimal valid MP3 file (MPEG-1 Layer 3, 44.1kHz, mono, 100ms)
    // This is a valid MP3 header + minimal data for ~100ms of silence
    let mp3_data = include_bytes!("test_assets/100ms_silence.mp3");

    let file_path = temp_dir.path().join(format!("test_audio_{}.mp3", index));
    let mut file = File::create(&file_path)?;
    file.write_all(mp3_data)?;

    Ok(file_path)
}

/// Create test audio file directly in specified directory
pub fn create_test_audio_file_in_dir(dir: &std::path::Path, index: usize) -> anyhow::Result<PathBuf> {
    use std::fs::File;
    use std::io::Write;

    let mp3_data = include_bytes!("test_assets/100ms_silence.mp3");

    let file_path = dir.join(format!("test_audio_{}.mp3", index));
    let mut file = File::create(&file_path)?;
    file.write_all(mp3_data)?;

    Ok(file_path)
}
