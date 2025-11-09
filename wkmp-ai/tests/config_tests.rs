//! Unit tests for configuration resolution
//!
//! Tests the implementation of:
//! - [APIK-RES-010] through [APIK-RES-050] - Multi-tier resolution
//! - [APIK-VAL-010] - Validation
//!
//! Note: Uses serial_test crate to prevent ENV variable race conditions.
//! Tests that manipulate WKMP_ACOUSTID_API_KEY are marked with #[serial]
//! to ensure they run sequentially, not in parallel.

use serial_test::serial;
use wkmp_ai::config::{resolve_acoustid_api_key, is_valid_key};
use wkmp_ai::db::settings::set_acoustid_api_key;
use wkmp_common::config::{TomlConfig, LoggingConfig};
use sqlx::sqlite::SqlitePoolOptions;

// ============================================================================
// Resolution Tests (tc_u_res_001-008)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_database_overrides_env_and_toml() {
    // tc_u_res_001: Database priority
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Setup: DB="db-key", ENV="env-key", TOML="toml-key"
    set_acoustid_api_key(&pool, "db-key".to_string())
        .await
        .unwrap();
    std::env::set_var("WKMP_ACOUSTID_API_KEY", "env-key");

    let toml_config = TomlConfig {
        root_folder: None,
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("toml-key".to_string()),
        musicbrainz_token: None,
    };

    let result = resolve_acoustid_api_key(&pool, &toml_config).await.unwrap();
    assert_eq!(result, "db-key");

    // Cleanup
    std::env::remove_var("WKMP_ACOUSTID_API_KEY");
}

#[tokio::test]
#[serial]
async fn test_env_fallback_when_database_empty() {
    // tc_u_res_002: ENV fallback

    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Setup: DB=None, ENV="env-key", TOML="toml-key"
    std::env::set_var("WKMP_ACOUSTID_API_KEY", "env-key");

    let toml_config = TomlConfig {
        root_folder: None,
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("toml-key".to_string()),
        musicbrainz_token: None,
    };

    let result = resolve_acoustid_api_key(&pool, &toml_config).await.unwrap();
    assert_eq!(result, "env-key");

    // Cleanup
    std::env::remove_var("WKMP_ACOUSTID_API_KEY");
}

#[tokio::test]
#[serial]
async fn test_toml_fallback_when_db_and_env_empty() {
    // tc_u_res_003: TOML fallback
    std::env::remove_var("WKMP_ACOUSTID_API_KEY"); // Ensure clean state

    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Setup: DB=None, ENV=None, TOML="toml-key"
    // (cleanup_env already removed it)

    let toml_config = TomlConfig {
        root_folder: None,
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("toml-key".to_string()),
        musicbrainz_token: None,
    };

    let result = resolve_acoustid_api_key(&pool, &toml_config).await.unwrap();
    assert_eq!(result, "toml-key");
}

#[tokio::test]
#[serial]
async fn test_error_when_no_key_found() {
    // tc_u_res_004: Error on no key
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Setup: DB=None, ENV=None, TOML=None
    std::env::remove_var("WKMP_ACOUSTID_API_KEY");

    let toml_config = TomlConfig {
        root_folder: None,
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: None,
        musicbrainz_token: None,
    };

    let result = resolve_acoustid_api_key(&pool, &toml_config).await;
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("AcoustID API key not configured"));
    assert!(error_msg.contains("http://localhost:5723/settings"));
    assert!(error_msg.contains("WKMP_ACOUSTID_API_KEY"));
    assert!(error_msg.contains("wkmp-ai.toml"));
}

#[tokio::test]
#[serial]
async fn test_database_ignores_env() {
    // tc_u_res_005: Database ignores ENV when present
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Setup: DB="db-key", ENV="env-key"
    set_acoustid_api_key(&pool, "db-key".to_string())
        .await
        .unwrap();
    std::env::set_var("WKMP_ACOUSTID_API_KEY", "env-key");

    let toml_config = TomlConfig {
        root_folder: None,
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: None,
        musicbrainz_token: None,
    };

    let result = resolve_acoustid_api_key(&pool, &toml_config).await.unwrap();
    assert_eq!(result, "db-key");

    // Cleanup
    std::env::remove_var("WKMP_ACOUSTID_API_KEY");
}

#[tokio::test]
#[serial]
async fn test_database_ignores_toml() {
    // tc_u_res_006: Database ignores TOML when present
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Setup: DB="db-key", TOML="toml-key"
    set_acoustid_api_key(&pool, "db-key".to_string())
        .await
        .unwrap();
    std::env::remove_var("WKMP_ACOUSTID_API_KEY");

    let toml_config = TomlConfig {
        root_folder: None,
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("toml-key".to_string()),
        musicbrainz_token: None,
    };

    let result = resolve_acoustid_api_key(&pool, &toml_config).await.unwrap();
    assert_eq!(result, "db-key");
}

#[tokio::test]
#[serial]
async fn test_env_ignores_toml() {
    // tc_u_res_007: ENV ignores TOML when present

    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Setup: DB=None, ENV="env-key", TOML="toml-key"
    std::env::set_var("WKMP_ACOUSTID_API_KEY", "env-key");

    let toml_config = TomlConfig {
        root_folder: None,
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("toml-key".to_string()),
        musicbrainz_token: None,
    };

    let result = resolve_acoustid_api_key(&pool, &toml_config).await.unwrap();
    assert_eq!(result, "env-key");

    // Cleanup
    std::env::remove_var("WKMP_ACOUSTID_API_KEY");
}

#[tokio::test]
#[serial]
async fn test_multiple_sources_warning() {
    // tc_u_res_008: Multiple sources warning logged
    // Note: This test verifies behavior, not that warning is logged
    // (tracing verification would require test subscriber)
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Setup: DB="db-key", ENV="env-key", TOML="toml-key"
    set_acoustid_api_key(&pool, "db-key".to_string())
        .await
        .unwrap();
    std::env::set_var("WKMP_ACOUSTID_API_KEY", "env-key");

    let toml_config = TomlConfig {
        root_folder: None,
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("toml-key".to_string()),
        musicbrainz_token: None,
    };

    // Should still return database key (highest priority)
    let result = resolve_acoustid_api_key(&pool, &toml_config).await.unwrap();
    assert_eq!(result, "db-key");

    // Cleanup
    std::env::remove_var("WKMP_ACOUSTID_API_KEY");
}

// ============================================================================
// Validation Tests (tc_u_val_001-003)
// ============================================================================

#[test]
fn test_empty_key_rejected() {
    // tc_u_val_001: Empty key rejected
    assert!(!is_valid_key(""));
}

#[test]
fn test_whitespace_key_rejected() {
    // tc_u_val_002: Whitespace-only key rejected
    assert!(!is_valid_key("   \t\n"));
}

#[test]
fn test_valid_key_accepted() {
    // tc_u_val_003: Valid key accepted
    assert!(is_valid_key("valid-key-123"));
}

// ============================================================================
// Write-Back Tests (tc_u_wb_001-006)
// ============================================================================

use wkmp_ai::config::{sync_settings_to_toml, migrate_key_to_database};
use tempfile::TempDir;
use std::collections::HashMap;

#[tokio::test]
async fn test_sync_settings_to_toml_creates_file() {
    // tc_u_wb_005: UI update to TOML (HashMap interface)
    let temp_dir = TempDir::new().unwrap();
    let toml_path = temp_dir.path().join("wkmp-ai.toml");

    let mut settings = HashMap::new();
    settings.insert("acoustid_api_key".to_string(), "test-key-123".to_string());

    sync_settings_to_toml(settings, &toml_path).await.unwrap();

    // Verify file created
    assert!(toml_path.exists());

    // Verify content
    let content = std::fs::read_to_string(&toml_path).unwrap();
    assert!(content.contains("acoustid_api_key"));
    assert!(content.contains("test-key-123"));
}

#[tokio::test]
async fn test_sync_settings_preserves_existing_fields() {
    // tc_u_wb_005: Verify existing fields not overwritten
    let temp_dir = TempDir::new().unwrap();
    let toml_path = temp_dir.path().join("wkmp-ai.toml");

    // Write initial TOML with root_folder
    let initial_config = TomlConfig {
        root_folder: Some(std::path::PathBuf::from("/music")),
        logging: wkmp_common::config::LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: None,
        musicbrainz_token: None,
    };
    wkmp_common::config::write_toml_config(&initial_config, &toml_path).unwrap();

    // Sync API key
    let mut settings = HashMap::new();
    settings.insert("acoustid_api_key".to_string(), "new-key".to_string());
    sync_settings_to_toml(settings, &toml_path).await.unwrap();

    // Verify both fields present
    let content = std::fs::read_to_string(&toml_path).unwrap();
    let parsed: TomlConfig = toml::from_str(&content).unwrap();
    assert_eq!(parsed.root_folder, Some(std::path::PathBuf::from("/music")));
    assert_eq!(parsed.acoustid_api_key, Some("new-key".to_string()));
}

#[tokio::test]
async fn test_migrate_key_from_env_writes_both_db_and_toml() {
    // tc_u_wb_001, tc_u_wb_002: ENV to database + TOML write-back
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    let temp_dir = TempDir::new().unwrap();
    let toml_path = temp_dir.path().join("wkmp-ai.toml");

    // Migrate from ENV source
    migrate_key_to_database(
        "env-key-123".to_string(),
        "environment",
        &pool,
        &toml_path,
    )
    .await
    .unwrap();

    // Verify database
    let db_key = wkmp_ai::db::settings::get_acoustid_api_key(&pool)
        .await
        .unwrap();
    assert_eq!(db_key, Some("env-key-123".to_string()));

    // Verify TOML (should be written for ENV source)
    assert!(toml_path.exists());
    let content = std::fs::read_to_string(&toml_path).unwrap();
    assert!(content.contains("env-key-123"));
}

#[tokio::test]
async fn test_migrate_key_from_toml_writes_only_db() {
    // tc_u_wb_003: TOML to database (no TOML write)
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    let temp_dir = TempDir::new().unwrap();
    let toml_path = temp_dir.path().join("wkmp-ai.toml");

    // Migrate from TOML source (should NOT write TOML)
    migrate_key_to_database(
        "toml-key-123".to_string(),
        "TOML",
        &pool,
        &toml_path,
    )
    .await
    .unwrap();

    // Verify database
    let db_key = wkmp_ai::db::settings::get_acoustid_api_key(&pool)
        .await
        .unwrap();
    assert_eq!(db_key, Some("toml-key-123".to_string()));

    // Verify TOML NOT written (TOML source doesn't trigger write-back)
    assert!(!toml_path.exists());
}

#[tokio::test]
async fn test_toml_write_failure_graceful_degradation() {
    // tc_u_wb_006: TOML write failure graceful degradation
    let temp_dir = TempDir::new().unwrap();
    // Create a read-only directory to force write failure
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(temp_dir.path()).unwrap().permissions();
        perms.set_mode(0o444); // Read-only
        std::fs::set_permissions(temp_dir.path(), perms).unwrap();
    }

    let toml_path = temp_dir.path().join("wkmp-ai.toml");

    let mut settings = HashMap::new();
    settings.insert("acoustid_api_key".to_string(), "key".to_string());

    // Should NOT fail (graceful degradation)
    let result = sync_settings_to_toml(settings, &toml_path).await;

    #[cfg(unix)]
    {
        // On Unix, write should fail but function returns Ok (warns only)
        assert!(result.is_ok());
    }

    #[cfg(not(unix))]
    {
        // On Windows, permissions work differently, test may pass or fail
        // Just ensure it doesn't panic
        let _ = result;
    }
}
