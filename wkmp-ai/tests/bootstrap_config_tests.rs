//! Integration tests for two-stage database initialization
//!
//! **[AIA-INIT-010]** Verifies bootstrap configuration pattern:
//! - Stage 1: Read RESTART_REQUIRED parameters with minimal connection
//! - Stage 2: Create production pool with configuration
//!
//! **Test Coverage:**
//! - Default values when settings table empty
//! - Custom values when settings configured
//! - NULL handling for auto-detect parameters
//! - Pool configuration matches settings
//! - Startup sequence with real database

use wkmp_ai::models::WkmpAiBootstrapConfig;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tempfile::TempDir;
use std::path::PathBuf;

/// Helper: Create temporary database with settings table
async fn create_test_database() -> (TempDir, PathBuf, SqlitePool) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_bootstrap.db");

    // Create initial database with settings table
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite://{}?mode=rwc", db_path.display()))
        .await
        .expect("Failed to create database");

    // Create settings table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to create settings table");

    (temp_dir, db_path, pool)
}

#[tokio::test]
async fn test_bootstrap_with_empty_database() {
    // **[AIA-INIT-010]** Test default values when settings table empty
    let (_temp_dir, db_path, init_pool) = create_test_database().await;

    // Close initial pool
    init_pool.close().await;

    // Bootstrap should succeed with defaults
    let config = WkmpAiBootstrapConfig::from_database(&db_path)
        .await
        .expect("Bootstrap failed with empty database");

    // Verify defaults from IMPL016
    assert_eq!(config.connection_pool_size, 96, "Default pool size should be 96");
    assert_eq!(config.lock_retry_ms, 250, "Default lock retry should be 250ms");
    assert_eq!(config.max_lock_wait_ms, 5000, "Default max wait should be 5000ms");
    assert!(config.processing_thread_count() >= 1, "Thread count should be auto-detected (≥1)");

    // Verify production pool can be created with defaults
    let production_pool = config.create_pool(&db_path)
        .await
        .expect("Failed to create production pool with defaults");

    // Verify pool is functional
    let conn = production_pool.acquire().await.expect("Failed to acquire connection");
    drop(conn);

    production_pool.close().await;
}

#[tokio::test]
async fn test_bootstrap_with_custom_configuration() {
    // **[AIA-INIT-010]** Test custom values read from settings table
    let (_temp_dir, db_path, init_pool) = create_test_database().await;

    // Insert custom configuration
    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_connection_pool_size', '64')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert pool size");

    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_lock_retry_ms', '500')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert lock retry");

    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_max_lock_wait_ms', '10000')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert max wait");

    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_processing_thread_count', '8')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert thread count");

    init_pool.close().await;

    // Bootstrap should read custom values
    let config = WkmpAiBootstrapConfig::from_database(&db_path)
        .await
        .expect("Bootstrap failed with custom configuration");

    // Verify custom values
    assert_eq!(config.connection_pool_size, 64, "Custom pool size not read");
    assert_eq!(config.lock_retry_ms, 500, "Custom lock retry not read");
    assert_eq!(config.max_lock_wait_ms, 10000, "Custom max wait not read");
    assert_eq!(config.processing_thread_count(), 8, "Custom thread count not read");

    // Verify production pool created with custom configuration
    let production_pool = config.create_pool(&db_path)
        .await
        .expect("Failed to create production pool with custom config");

    production_pool.close().await;
}

#[tokio::test]
async fn test_bootstrap_with_null_thread_count() {
    // **[AIA-INIT-010]** Test NULL thread count triggers auto-detection
    let (_temp_dir, db_path, init_pool) = create_test_database().await;

    // Insert other settings but leave thread_count NULL
    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_connection_pool_size', '32')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert pool size");

    init_pool.close().await;

    // Bootstrap should auto-detect thread count
    let config = WkmpAiBootstrapConfig::from_database(&db_path)
        .await
        .expect("Bootstrap failed with NULL thread count");

    // Verify auto-detection (should be CPU cores + 1)
    let expected_min = num_cpus::get(); // At least CPU core count
    assert!(
        config.processing_thread_count() >= expected_min,
        "Auto-detected thread count ({}) should be ≥ CPU cores ({})",
        config.processing_thread_count(),
        expected_min
    );

    // Verify other custom value read correctly
    assert_eq!(config.connection_pool_size, 32, "Pool size should be custom value");
}

#[tokio::test]
async fn test_bootstrap_stage_separation() {
    // **[AIA-INIT-010]** Verify bootstrap connection closed before production pool
    let (_temp_dir, db_path, init_pool) = create_test_database().await;
    init_pool.close().await;

    // Stage 1: Bootstrap
    let config = WkmpAiBootstrapConfig::from_database(&db_path)
        .await
        .expect("Bootstrap stage 1 failed");

    // At this point, bootstrap connection should be closed
    // Verify we can create production pool (no connection conflicts)

    // Stage 2: Production
    let production_pool = config.create_pool(&db_path)
        .await
        .expect("Production stage 2 failed");

    // Verify production pool works
    let conn = production_pool.acquire().await.expect("Failed to acquire from production pool");
    drop(conn);

    production_pool.close().await;
}

#[tokio::test]
async fn test_bootstrap_interdependency_warning() {
    // **[AIA-INIT-010]** Test warning when pool size < recommended (thread_count × 8)
    let (_temp_dir, db_path, init_pool) = create_test_database().await;

    // Set pool size too small for thread count
    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_connection_pool_size', '8')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert pool size");

    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_processing_thread_count', '4')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert thread count");

    init_pool.close().await;

    // Bootstrap should succeed but log warning
    // (thread_count=4 needs pool_size≥32, but set to 8)
    let config = WkmpAiBootstrapConfig::from_database(&db_path)
        .await
        .expect("Bootstrap should succeed despite warning");

    assert_eq!(config.connection_pool_size, 8);
    assert_eq!(config.processing_thread_count(), 4);

    // Warning check: recommended = 4 × 8 = 32, actual = 8
    // (Warning logged to tracing, checked manually in test output)
}

#[tokio::test]
async fn test_bootstrap_invalid_values() {
    // **[AIA-INIT-010]** Test error handling for invalid parameter values
    let (_temp_dir, db_path, init_pool) = create_test_database().await;

    // Insert invalid value (non-numeric)
    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_connection_pool_size', 'invalid')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert invalid value");

    init_pool.close().await;

    // Bootstrap should fail with clear error
    let result = WkmpAiBootstrapConfig::from_database(&db_path).await;

    assert!(result.is_err(), "Bootstrap should fail with invalid value");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("ai_database_connection_pool_size"),
        "Error should mention parameter name, got: {}",
        error
    );
}

#[tokio::test]
async fn test_production_pool_configuration() {
    // **[AIA-INIT-010]** Verify production pool configured with bootstrap parameters
    let (_temp_dir, db_path, init_pool) = create_test_database().await;

    // Set specific configuration
    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_connection_pool_size', '16')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert pool size");

    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_lock_retry_ms', '1000')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert lock retry");

    init_pool.close().await;

    // Bootstrap
    let config = WkmpAiBootstrapConfig::from_database(&db_path)
        .await
        .expect("Bootstrap failed");

    // Create production pool
    let production_pool = config.create_pool(&db_path)
        .await
        .expect("Failed to create production pool");

    // Verify pool accepts up to configured max connections
    // (Can't directly inspect pool size, but can verify it works)
    let mut connections = Vec::new();

    // Acquire multiple connections (should succeed up to pool size)
    for i in 0..4 {
        let conn = production_pool.acquire().await
            .expect(&format!("Failed to acquire connection {}", i));
        connections.push(conn);
    }

    // Release connections
    drop(connections);
    production_pool.close().await;
}

#[tokio::test]
async fn test_full_startup_sequence_simulation() {
    // **[AIA-INIT-010]** Simulate complete wkmp-ai startup sequence
    let (_temp_dir, db_path, init_pool) = create_test_database().await;

    // Pre-populate with realistic configuration
    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_connection_pool_size', '96')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert pool size");

    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_lock_retry_ms', '250')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert lock retry");

    sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_max_lock_wait_ms', '5000')")
        .execute(&init_pool)
        .await
        .expect("Failed to insert max wait");

    // Leave thread_count NULL to test auto-detection

    init_pool.close().await;

    // Simulate main.rs startup sequence

    // Stage 1: Bootstrap
    let start = std::time::Instant::now();
    let bootstrap_config = WkmpAiBootstrapConfig::from_database(&db_path)
        .await
        .expect("Bootstrap stage failed");
    let bootstrap_duration = start.elapsed();

    println!("Bootstrap duration: {:?}", bootstrap_duration);
    assert!(bootstrap_duration.as_millis() < 100, "Bootstrap should complete in <100ms");

    // Stage 2: Production pool
    let start = std::time::Instant::now();
    let db_pool = bootstrap_config.create_pool(&db_path)
        .await
        .expect("Production pool creation failed");
    let production_duration = start.elapsed();

    println!("Production pool creation duration: {:?}", production_duration);
    assert!(production_duration.as_millis() < 200, "Production pool should create in <200ms");

    // Verify pool configuration
    assert_eq!(bootstrap_config.connection_pool_size, 96);
    assert_eq!(bootstrap_config.lock_retry_ms, 250);
    assert_eq!(bootstrap_config.max_lock_wait_ms, 5000);

    // Simulate AppState creation
    let _thread_count = bootstrap_config.processing_thread_count();
    println!("Thread count: {}", _thread_count);

    // Verify pool functional
    let conn = db_pool.acquire().await.expect("Failed to acquire connection");
    drop(conn);

    db_pool.close().await;

    // Total startup overhead should be minimal
    let total_duration = bootstrap_duration + production_duration;
    println!("Total two-stage initialization overhead: {:?}", total_duration);
    assert!(total_duration.as_millis() < 300, "Total overhead should be <300ms");
}

#[tokio::test]
async fn test_bootstrap_missing_settings_table() {
    // **[AIA-INIT-010]** Test behavior when settings table doesn't exist
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_no_table.db");

    // Create empty database (no tables)
    let init_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite://{}?mode=rwc", db_path.display()))
        .await
        .expect("Failed to create database");

    init_pool.close().await;

    // Bootstrap should fail gracefully with clear error
    let result = WkmpAiBootstrapConfig::from_database(&db_path).await;

    assert!(result.is_err(), "Bootstrap should fail when settings table missing");
    let error = result.unwrap_err();
    println!("Error with missing table: {}", error);
    // Error should be about missing table or failed query
}

#[tokio::test]
async fn test_bootstrap_concurrent_access() {
    // **[AIA-INIT-010]** Verify bootstrap doesn't block concurrent database access
    let (_temp_dir, db_path, init_pool) = create_test_database().await;
    init_pool.close().await;

    // Spawn concurrent bootstrap attempts
    let db_path_clone1 = db_path.clone();
    let db_path_clone2 = db_path.clone();

    let (result1, result2) = tokio::join!(
        WkmpAiBootstrapConfig::from_database(&db_path_clone1),
        WkmpAiBootstrapConfig::from_database(&db_path_clone2)
    );

    // Both should succeed (SQLite handles concurrent readers)
    assert!(result1.is_ok(), "First bootstrap should succeed");
    assert!(result2.is_ok(), "Second bootstrap should succeed");

    // Both should have same configuration
    let config1 = result1.unwrap();
    let config2 = result2.unwrap();

    assert_eq!(config1.connection_pool_size, config2.connection_pool_size);
    assert_eq!(config1.lock_retry_ms, config2.lock_retry_ms);
}
