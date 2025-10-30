//! Buffer configuration tests
//!
//! **[REQ-DEBT-FUNC-002]** Buffer Configuration Management
//!
//! Tests verify:
//! - TC-FUNC-002-01: BufferManager reads custom settings from database
//! - TC-FUNC-002-02: BufferManager uses defaults when settings NULL
//! - TC-FUNC-002-03: BufferManager validates and rejects invalid settings
//! - TC-FUNC-002-04: End-to-end buffer config flow

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use wkmp_ap::playback::buffer_manager::BufferManager;
use wkmp_ap::playback::engine::PlaybackEngine;
use wkmp_ap::state::SharedState;

/// Create test database with settings table
async fn create_test_db() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    // Create settings table
    sqlx::query(
        r#"
        CREATE TABLE settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create minimal queue table (required by PlaybackEngine)
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
    .execute(&pool)
    .await
    .unwrap();

    pool
}

// ============================================================================
// TC-FUNC-002-01: BufferManager reads custom settings from database
// ============================================================================

/// **[TC-FUNC-002-01]** Verify BufferManager reads custom capacity/headroom from database
#[tokio::test]
async fn test_buffer_manager_reads_custom_settings() {
    let db = create_test_db().await;

    // Insert custom settings
    sqlx::query("INSERT INTO settings (key, value) VALUES ('playout_ringbuffer_capacity', '1000000')")
        .execute(&db)
        .await
        .unwrap();

    sqlx::query("INSERT INTO settings (key, value) VALUES ('playout_ringbuffer_headroom', '10000')")
        .execute(&db)
        .await
        .unwrap();

    // Create PlaybackEngine (loads settings via BufferManager)
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db.clone(), state)
        .await
        .expect("Engine initialization should succeed");

    // Get BufferManager and verify settings were loaded
    let buffer_manager = engine.get_buffer_manager();
    let capacity = buffer_manager.get_buffer_capacity().await;
    let headroom = buffer_manager.get_buffer_headroom().await;

    assert_eq!(capacity, 1_000_000, "Should load custom capacity");
    assert_eq!(headroom, 10_000, "Should load custom headroom");
}

// ============================================================================
// TC-FUNC-002-02: BufferManager uses defaults when settings NULL
// ============================================================================

/// **[TC-FUNC-002-02]** Verify BufferManager uses defaults when settings are NULL
#[tokio::test]
async fn test_buffer_manager_uses_defaults_when_null() {
    let db = create_test_db().await;

    // Do not insert any settings - database is empty

    // Create PlaybackEngine (should use default settings)
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db.clone(), state)
        .await
        .expect("Engine initialization should succeed");

    // Get BufferManager and verify default settings were used
    let buffer_manager = engine.get_buffer_manager();
    let capacity = buffer_manager.get_buffer_capacity().await;
    let headroom = buffer_manager.get_buffer_headroom().await;

    // Default values from settings.rs
    assert_eq!(capacity, 661_941, "Should use default capacity");
    assert_eq!(headroom, 4_410, "Should use default headroom");
}

// ============================================================================
// TC-FUNC-002-03: BufferManager validates and rejects invalid settings
// ============================================================================

/// **[TC-FUNC-002-03]** Verify validation rejects invalid buffer configurations
#[tokio::test]
async fn test_buffer_validation_rejects_invalid_configs() {
    // Test 1: Zero capacity
    let result = BufferManager::validate_buffer_config(0, 4_410);
    assert!(result.is_err(), "Should reject zero capacity");
    assert!(
        result.unwrap_err().contains("capacity must be greater than zero"),
        "Error message should mention zero capacity"
    );

    // Test 2: Zero headroom
    let result = BufferManager::validate_buffer_config(661_941, 0);
    assert!(result.is_err(), "Should reject zero headroom");
    assert!(
        result.unwrap_err().contains("headroom must be greater than zero"),
        "Error message should mention zero headroom"
    );

    // Test 3: Capacity equal to headroom
    let result = BufferManager::validate_buffer_config(10_000, 10_000);
    assert!(result.is_err(), "Should reject capacity == headroom");
    let error_msg = result.unwrap_err();
    assert!(
        error_msg.contains("capacity") && error_msg.contains("greater than headroom"),
        "Error message should mention capacity must be greater than headroom"
    );

    // Test 4: Capacity less than headroom
    let result = BufferManager::validate_buffer_config(5_000, 10_000);
    assert!(result.is_err(), "Should reject capacity < headroom");
    let error_msg = result.unwrap_err();
    assert!(
        error_msg.contains("capacity") && error_msg.contains("greater than headroom"),
        "Error message should mention capacity must be greater than headroom"
    );

    // Test 5: Valid configuration
    let result = BufferManager::validate_buffer_config(661_941, 4_410);
    assert!(result.is_ok(), "Should accept valid configuration");
}

// ============================================================================
// TC-FUNC-002-04: End-to-end buffer config flow
// ============================================================================

/// **[TC-FUNC-002-04]** Integration test: Load from DB, validate, allocate buffer
#[tokio::test]
async fn test_end_to_end_buffer_config_flow() {
    let db = create_test_db().await;

    // Insert valid custom settings
    sqlx::query("INSERT INTO settings (key, value) VALUES ('playout_ringbuffer_capacity', '500000')")
        .execute(&db)
        .await
        .unwrap();

    sqlx::query("INSERT INTO settings (key, value) VALUES ('playout_ringbuffer_headroom', '5000')")
        .execute(&db)
        .await
        .unwrap();

    // Create PlaybackEngine (loads settings via BufferManager)
    let state = Arc::new(SharedState::new());
    let engine = PlaybackEngine::new(db.clone(), state)
        .await
        .expect("Engine initialization should succeed");

    // Get BufferManager
    let buffer_manager = engine.get_buffer_manager();
    let capacity = buffer_manager.get_buffer_capacity().await;
    let headroom = buffer_manager.get_buffer_headroom().await;

    // Verify settings loaded
    assert_eq!(capacity, 500_000, "Should load custom capacity");
    assert_eq!(headroom, 5_000, "Should load custom headroom");

    // Validate the loaded configuration
    let validation_result = BufferManager::validate_buffer_config(capacity, headroom);
    assert!(
        validation_result.is_ok(),
        "Loaded configuration should pass validation"
    );

    // Allocate a buffer (tests that buffer creation works with custom settings)
    let queue_entry_id = uuid::Uuid::new_v4();
    buffer_manager.allocate_buffer(queue_entry_id).await;

    // Verify buffer was created
    let buffer_info = buffer_manager.get_buffer_info(queue_entry_id).await;

    assert!(
        buffer_info.is_some(),
        "Buffer should be allocated successfully"
    );
}
