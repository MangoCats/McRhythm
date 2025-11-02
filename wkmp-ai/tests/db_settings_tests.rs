//! Unit tests for database settings accessors
//!
//! Tests the implementation of:
//! - [APIK-DB-010] - Settings table key-value pattern
//! - [APIK-DB-020] - Generic get/set_setting()
//! - [APIK-ACID-040] - Database storage for acoustid_api_key

use wkmp_ai::db::settings::{get_acoustid_api_key, set_acoustid_api_key};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn test_get_acoustid_api_key_returns_value() {
    // tc_u_db_001
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();

    // Initialize schema
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Set key
    set_acoustid_api_key(&pool, "test-key-123".to_string())
        .await
        .unwrap();

    // Get key
    let key = get_acoustid_api_key(&pool).await.unwrap();
    assert_eq!(key, Some("test-key-123".to_string()));
}

#[tokio::test]
async fn test_set_acoustid_api_key_writes_value() {
    // tc_u_db_002
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();

    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Set key
    set_acoustid_api_key(&pool, "new-key-456".to_string())
        .await
        .unwrap();

    // Verify by direct query
    let row: (String,) = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'acoustid_api_key'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, "new-key-456");
}

#[tokio::test]
async fn test_get_acoustid_api_key_returns_none_when_missing() {
    // tc_u_db_001 (edge case)
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();

    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Get key (not set)
    let key = get_acoustid_api_key(&pool).await.unwrap();
    assert_eq!(key, None);
}

#[tokio::test]
async fn test_set_acoustid_api_key_updates_existing() {
    // tc_u_db_002 (update case)
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();

    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    // Set initial key
    set_acoustid_api_key(&pool, "old-key".to_string())
        .await
        .unwrap();

    // Update key
    set_acoustid_api_key(&pool, "new-key".to_string())
        .await
        .unwrap();

    // Verify updated
    let key = get_acoustid_api_key(&pool).await.unwrap();
    assert_eq!(key, Some("new-key".to_string()));
}
