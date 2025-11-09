//! Integration tests for database recovery and TOML backup
//!
//! Tests the implementation of:
//! - [APIK-TOML-010] Durable backup survives deletion
//! - [APIK-TEST-020] Integration tests

use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use tempfile::TempDir;
use wkmp_ai::config::{migrate_key_to_database, resolve_acoustid_api_key, sync_settings_to_toml};
use wkmp_common::config::TomlConfig;

// ============================================================================
// Recovery Tests (tc_i_recovery_001-002)
// ============================================================================

#[tokio::test]
async fn test_database_deletion_recovers_from_toml() {
    // tc_i_recovery_001: Database deletion recovers from TOML
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("wkmp.db");
    let toml_path = temp_dir.path().join("wkmp-ai.toml");

    // Step 1: Set key via migration (simulates UI save that writes DB + TOML)
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new()
        .connect(&db_url)
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    migrate_key_to_database(
        "test-key-recovery".to_string(),
        "environment",
        &pool,
        &toml_path,
    )
    .await
    .unwrap();

    // Verify database and TOML both have key
    let db_key = wkmp_ai::db::settings::get_acoustid_api_key(&pool)
        .await
        .unwrap();
    assert_eq!(db_key, Some("test-key-recovery".to_string()));
    assert!(toml_path.exists());

    // Step 2: Close database and delete it
    pool.close().await;
    drop(pool);
    // Small delay to ensure file handle released (Windows-specific)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    std::fs::remove_file(&db_path).unwrap();

    // Step 3: Recreate database (simulates restart)
    let db_url2 = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool2 = SqlitePoolOptions::new()
        .connect(&db_url2)
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool2).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool2).await.unwrap();

    // Step 4: Load TOML config and resolve key
    let toml_content = std::fs::read_to_string(&toml_path).unwrap();
    let toml_config: TomlConfig = toml::from_str(&toml_content).unwrap();

    let resolved_key = resolve_acoustid_api_key(&pool2, &toml_config).await.unwrap();
    assert_eq!(resolved_key, "test-key-recovery");

    // Step 5: Verify key migrated back to database
    // (In real startup, migration happens automatically)
    let db_key_before_migration = wkmp_ai::db::settings::get_acoustid_api_key(&pool2)
        .await
        .unwrap();
    assert_eq!(db_key_before_migration, None); // Not yet migrated

    // Simulate auto-migration
    migrate_key_to_database(resolved_key, "TOML", &pool2, &toml_path)
        .await
        .unwrap();

    let db_key_after = wkmp_ai::db::settings::get_acoustid_api_key(&pool2)
        .await
        .unwrap();
    assert_eq!(db_key_after, Some("test-key-recovery".to_string()));
}

#[tokio::test]
async fn test_database_deletion_no_toml_fails() {
    // tc_i_recovery_002: Database deletion with no TOML fails gracefully
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("wkmp.db");
    let toml_path = temp_dir.path().join("wkmp-ai.toml");

    // Create fresh database with no key
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new()
        .connect(&db_url)
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // No TOML file exists
    assert!(!toml_path.exists());

    // Empty TOML config (no key)
    let toml_config = TomlConfig {
        root_folder: None,
        logging: wkmp_common::config::LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: None,
        musicbrainz_token: None,
    };

    // Attempt to resolve key should fail with helpful error
    let result = resolve_acoustid_api_key(&pool, &toml_config).await;
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("AcoustID API key not configured"));
    assert!(error_msg.contains("http://localhost:5723/settings"));
    assert!(error_msg.contains("WKMP_ACOUSTID_API_KEY"));
    assert!(error_msg.contains("wkmp-ai.toml"));
}

// ============================================================================
// TOML Write-Back Durability Test
// ============================================================================

#[tokio::test]
async fn test_toml_write_back_survives_database_deletion() {
    // Verify that TOML write-back provides durable backup
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("wkmp.db");
    let toml_path = temp_dir.path().join("wkmp-ai.toml");

    // Step 1: Configure key via database
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new()
        .connect(&db_url)
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    wkmp_ai::db::settings::set_acoustid_api_key(&pool, "durable-key".to_string())
        .await
        .unwrap();

    // Step 2: Trigger TOML sync (simulates UI save)
    let mut settings = HashMap::new();
    settings.insert("acoustid_api_key".to_string(), "durable-key".to_string());
    sync_settings_to_toml(settings, &toml_path).await.unwrap();

    // Step 3: Verify TOML file created
    assert!(toml_path.exists());
    let toml_content = std::fs::read_to_string(&toml_path).unwrap();
    assert!(toml_content.contains("durable-key"));

    // Step 4: Delete database
    pool.close().await;
    drop(pool);
    std::fs::remove_file(&db_path).unwrap();

    // Step 5: TOML file still exists (durable backup)
    assert!(toml_path.exists());

    // Step 6: Parse TOML and verify key recoverable
    let toml_config: TomlConfig = toml::from_str(&toml_content).unwrap();
    assert_eq!(
        toml_config.acoustid_api_key,
        Some("durable-key".to_string())
    );
}
