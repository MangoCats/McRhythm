//! Integration tests for concurrent access patterns
//!
//! Tests the implementation of:
//! - [APIK-ATOMIC-020] Prevent races
//! - [APIK-TEST-020] Integration tests

use std::sync::Arc;
use tempfile::TempDir;
use tokio::task::JoinSet;
use wkmp_common::config::TomlConfig;

// ============================================================================
// Concurrency Tests (tc_i_concurrent_001)
// ============================================================================

#[tokio::test]
async fn test_concurrent_toml_reads_safe() {
    // tc_i_concurrent_001: Multiple module startup TOML reads safe
    let temp_dir = TempDir::new().unwrap();
    let toml_path = temp_dir.path().join("wkmp-ai.toml");

    // Step 1: Write TOML file with API key
    let config = TomlConfig {
        root_folder: Some(std::path::PathBuf::from("/music")),
        logging: wkmp_common::config::LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("concurrent-test-key".to_string()),
    };

    wkmp_common::config::write_toml_config(&config, &toml_path)
        .unwrap();

    // Step 2: Spawn 10 concurrent tasks reading TOML
    let toml_path = Arc::new(toml_path);
    let mut join_set = JoinSet::new();

    for i in 0..10 {
        let path_clone = Arc::clone(&toml_path);
        join_set.spawn(async move {
            // Simulate concurrent startup reads
            let content = tokio::fs::read_to_string(path_clone.as_ref())
                .await
                .expect(&format!("Task {} failed to read TOML", i));

            let parsed: TomlConfig = toml::from_str(&content)
                .expect(&format!("Task {} failed to parse TOML", i));

            assert_eq!(
                parsed.acoustid_api_key,
                Some("concurrent-test-key".to_string()),
                "Task {} got incorrect key",
                i
            );

            i // Return task ID for verification
        });
    }

    // Step 3: Wait for all tasks and verify success
    let mut task_ids = Vec::new();
    while let Some(result) = join_set.join_next().await {
        let task_id = result.expect("Task panicked");
        task_ids.push(task_id);
    }

    // Step 4: Verify all 10 tasks completed successfully
    task_ids.sort();
    assert_eq!(task_ids, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[tokio::test]
async fn test_concurrent_database_reads_safe() {
    // Additional concurrency test: Multiple threads reading database
    use sqlx::sqlite::SqlitePoolOptions;

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(":memory:")
        .await
        .unwrap();

    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Set API key
    wkmp_ai::db::settings::set_acoustid_api_key(&pool, "db-concurrent-key".to_string())
        .await
        .unwrap();

    // Spawn 20 concurrent reads
    let pool = Arc::new(pool);
    let mut join_set = JoinSet::new();

    for i in 0..20 {
        let pool_clone = Arc::clone(&pool);
        join_set.spawn(async move {
            let key = wkmp_ai::db::settings::get_acoustid_api_key(&pool_clone)
                .await
                .expect(&format!("Task {} failed to read database", i));

            assert_eq!(
                key,
                Some("db-concurrent-key".to_string()),
                "Task {} got incorrect key",
                i
            );

            i
        });
    }

    // Wait for all tasks
    let mut task_ids = Vec::new();
    while let Some(result) = join_set.join_next().await {
        let task_id = result.expect("Task panicked");
        task_ids.push(task_id);
    }

    // Verify all 20 tasks completed
    task_ids.sort();
    assert_eq!(task_ids.len(), 20);
}

#[tokio::test]
async fn test_concurrent_database_writes_safe() {
    // Test concurrent writes to settings (SQLite handles serialization)
    use sqlx::sqlite::SqlitePoolOptions;

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(":memory:")
        .await
        .unwrap();

    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Spawn 10 concurrent writes with different values
    let pool = Arc::new(pool);
    let mut join_set = JoinSet::new();

    for i in 0..10 {
        let pool_clone = Arc::clone(&pool);
        join_set.spawn(async move {
            let key = format!("key-{}", i);
            wkmp_ai::db::settings::set_acoustid_api_key(&pool_clone, key.clone())
                .await
                .expect(&format!("Task {} failed to write database", i));

            i
        });
    }

    // Wait for all writes
    while let Some(result) = join_set.join_next().await {
        result.expect("Task panicked");
    }

    // Verify final state (one of the keys should be present)
    let final_key = wkmp_ai::db::settings::get_acoustid_api_key(&pool)
        .await
        .unwrap();

    assert!(final_key.is_some());
    assert!(final_key.unwrap().starts_with("key-"));
}
