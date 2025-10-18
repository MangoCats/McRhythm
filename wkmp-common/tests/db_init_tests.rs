//! Unit tests for database initialization and graceful degradation
//!
//! Tests the implementation of:
//! - [REQ-NF-036]: Automatic database creation with default schema
//! - [ARCH-INIT-010]: Module startup sequence
//! - [ARCH-INIT-020]: Default value initialization behavior
//! - [DEP-DB-011]: Database initialization on first run

use wkmp_common::db::init::init_database;
use std::path::PathBuf;

#[tokio::test]
async fn test_database_creation_when_missing() {
    // [REQ-NF-036]: If database does not exist, create it automatically

    let test_db = format!("/tmp/wkmp-test-db-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Initialize database
    let result = init_database(&db_path).await;

    assert!(result.is_ok(), "Database initialization failed: {:?}", result.err());

    // Verify database file was created
    assert!(db_path.exists(), "Database file was not created");

    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_database_opens_existing() {
    // [REQ-NF-036]: Should open existing database without error

    let test_db = format!("/tmp/wkmp-test-db-existing-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Create database first time
    let pool1 = init_database(&db_path).await;
    assert!(pool1.is_ok());

    // Open database second time (should succeed)
    let pool2 = init_database(&db_path).await;
    assert!(pool2.is_ok(), "Failed to open existing database: {:?}", pool2.err());

    // Cleanup
    drop(pool1);
    drop(pool2);
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_default_settings_initialized() {
    // [ARCH-INIT-020]: Default settings should be initialized

    let test_db = format!("/tmp/wkmp-test-db-settings-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Initialize database
    let pool = init_database(&db_path).await.unwrap();

    // Check that settings table exists and has default values
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Should have multiple default settings
    assert!(count > 20, "Expected 20+ default settings, got {}", count);

    // Verify specific critical settings exist
    let volume: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'volume_level'"
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(volume.is_some(), "volume_level setting not initialized");
    assert_eq!(volume.unwrap(), "0.5", "volume_level has wrong default value");

    let crossfade: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'global_crossfade_time'"
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(crossfade.is_some(), "global_crossfade_time setting not initialized");
    assert_eq!(crossfade.unwrap(), "2.0", "global_crossfade_time has wrong default value");

    // Cleanup
    drop(pool);
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_module_config_initialized() {
    // [DEP-CFG-035]: Module config table should be initialized

    let test_db = format!("/tmp/wkmp-test-db-modules-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Initialize database
    let pool = init_database(&db_path).await.unwrap();

    // Check that module_config table exists and has default modules
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM module_config")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Should have 5 modules
    assert_eq!(count, 5, "Expected 5 module configs, got {}", count);

    // Verify audio_player module exists
    let audio_player: Option<(String, i64)> = sqlx::query_as(
        "SELECT host, port FROM module_config WHERE module_name = 'audio_player'"
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(audio_player.is_some(), "audio_player module not initialized");
    let (host, port) = audio_player.unwrap();
    assert_eq!(host, "127.0.0.1", "audio_player has wrong default host");
    assert_eq!(port, 5721, "audio_player has wrong default port");

    // Cleanup
    drop(pool);
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_users_table_initialized() {
    // [ARCH-INIT-010]: Users table should be initialized with Anonymous user

    let test_db = format!("/tmp/wkmp-test-db-users-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Initialize database
    let pool = init_database(&db_path).await.unwrap();

    // Check that users table has Anonymous user
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = 'Anonymous'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(count, 1, "Anonymous user not created");

    // Verify Anonymous user properties
    let anon_user: (String, String, String, i64) = sqlx::query_as(
        "SELECT username, password_hash, password_salt, config_interface_access FROM users WHERE username = 'Anonymous'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(anon_user.0, "Anonymous");
    assert_eq!(anon_user.1, "", "Anonymous should have empty password_hash");
    assert_eq!(anon_user.2, "", "Anonymous should have empty password_salt");
    assert_eq!(anon_user.3, 1, "Anonymous should have config_interface_access enabled");

    // Cleanup
    drop(pool);
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_idempotent_initialization() {
    // [ARCH-INIT-010]: Safe to initialize multiple times

    let test_db = format!("/tmp/wkmp-test-db-idempotent-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Initialize database first time
    let pool1 = init_database(&db_path).await.unwrap();

    let count1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings")
        .fetch_one(&pool1)
        .await
        .unwrap();

    drop(pool1);

    // Initialize database second time (should not error)
    let pool2 = init_database(&db_path).await.unwrap();

    let count2: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings")
        .fetch_one(&pool2)
        .await
        .unwrap();

    // Should have same number of settings (idempotent)
    assert_eq!(count1, count2, "Settings count changed on second initialization");

    // Cleanup
    drop(pool2);
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_null_value_handling() {
    // [ARCH-INIT-020]: NULL values should be reset to defaults

    let test_db = format!("/tmp/wkmp-test-db-null-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Initialize database
    let pool = init_database(&db_path).await.unwrap();

    // Manually set a setting to NULL
    sqlx::query("UPDATE settings SET value = NULL WHERE key = 'volume_level'")
        .execute(&pool)
        .await
        .unwrap();

    // Verify it's NULL
    let value: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'volume_level'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(value.is_none(), "Value should be NULL before re-initialization");

    drop(pool);

    // Re-initialize database (should reset NULL to default)
    let pool2 = init_database(&db_path).await.unwrap();

    // Verify it's no longer NULL
    let value2: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'volume_level'"
    )
    .fetch_one(&pool2)
    .await
    .unwrap();

    assert!(value2.is_some(), "NULL value was not reset to default");
    assert_eq!(value2.unwrap(), "0.5", "NULL value was not reset to correct default");

    // Cleanup
    drop(pool2);
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_foreign_keys_enabled() {
    // Verify that foreign key constraints are enabled

    let test_db = format!("/tmp/wkmp-test-db-fk-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Initialize database
    let pool = init_database(&db_path).await.unwrap();

    // Check foreign keys setting
    let fk_enabled: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(fk_enabled, 1, "Foreign keys should be enabled");

    // Cleanup
    drop(pool);
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_busy_timeout_set() {
    // [ARCH-ERRH-070]: Busy timeout should be set

    let test_db = format!("/tmp/wkmp-test-db-timeout-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Initialize database
    let pool = init_database(&db_path).await.unwrap();

    // Check busy timeout setting
    let timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(timeout, 5000, "Busy timeout should be 5000ms");

    // Cleanup
    drop(pool);
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_specific_default_values() {
    // [ARCH-INIT-020]: Verify specific default values are correct

    let test_db = format!("/tmp/wkmp-test-db-defaults-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Initialize database
    let pool = init_database(&db_path).await.unwrap();

    // Test multiple critical settings
    let test_cases = vec![
        ("initial_play_state", "playing"),
        ("volume_level", "0.5"),
        ("global_crossfade_time", "2.0"),
        ("queue_max_size", "100"),
        ("session_timeout_seconds", "31536000"),
        ("http_base_ports", "[5720, 15720, 25720, 17200, 23400]"),
        ("backup_retention_count", "3"),
        ("relaunch_attempts", "20"),
    ];

    for (key, expected_value) in test_cases {
        let value: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(&pool)
        .await
        .unwrap();

        assert!(value.is_some(), "Setting '{}' not initialized", key);
        assert_eq!(
            value.unwrap(),
            expected_value,
            "Setting '{}' has wrong default value",
            key
        );
    }

    // Cleanup
    drop(pool);
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_all_modules_in_config() {
    // Verify all 5 modules are initialized in module_config

    let test_db = format!("/tmp/wkmp-test-db-all-modules-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Initialize database
    let pool = init_database(&db_path).await.unwrap();

    let modules = vec![
        "user_interface",
        "audio_player",
        "program_director",
        "audio_ingest",
        "lyric_editor",
    ];

    for module in modules {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM module_config WHERE module_name = ?"
        )
        .bind(module)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(exists, 1, "Module '{}' not initialized", module);
    }

    // Cleanup
    drop(pool);
    let _ = std::fs::remove_file(&db_path);
}

#[tokio::test]
async fn test_concurrent_initialization() {
    // [ARCH-INIT-010]: Multiple modules can initialize concurrently

    let test_db = format!("/tmp/wkmp-test-db-concurrent-{}.db", std::process::id());
    let db_path = PathBuf::from(&test_db);

    // Ensure database doesn't exist
    let _ = std::fs::remove_file(&db_path);

    // Spawn multiple concurrent initialization tasks
    let mut handles = vec![];

    for _ in 0..5 {
        let db_path_clone = db_path.clone();
        let handle = tokio::spawn(async move {
            init_database(&db_path_clone).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // All should succeed
    for result in &results {
        assert!(result.is_ok(), "Concurrent initialization failed: {:?}", result);
    }

    // Verify database is in consistent state
    let pool = results[0].as_ref().unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings")
        .fetch_one(pool)
        .await
        .unwrap();

    assert!(count > 20, "Settings not properly initialized after concurrent access");

    // Cleanup
    for result in results {
        drop(result);
    }
    let _ = std::fs::remove_file(&db_path);
}
