//! Settings database operations
//!
//! **Traceability:** [APIK-DB-010], [APIK-DB-020], [APIK-ACID-040]
//!
//! Provides get/set accessors for settings table following key-value pattern.

use sqlx::{Pool, Sqlite};
use wkmp_common::{Error, Result};

#[cfg(test)]
use sqlx::SqlitePool;

/// Get AcoustID API key from database
///
/// **Traceability:** [APIK-DB-010], [APIK-ACID-040]
///
/// **Returns:** Some(key) if exists, None if not set
pub async fn get_acoustid_api_key(db: &Pool<Sqlite>) -> Result<Option<String>> {
    get_setting::<String>(db, "acoustid_api_key").await
}

/// Set AcoustID API key in database
///
/// **Traceability:** [APIK-DB-020], [APIK-ACID-040]
pub async fn set_acoustid_api_key(db: &Pool<Sqlite>, key: String) -> Result<()> {
    set_setting(db, "acoustid_api_key", key).await
}

/// Generic setting getter (internal)
async fn get_setting<T>(db: &Pool<Sqlite>, key: &str) -> Result<Option<T>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = ?"
    )
    .bind(key)
    .fetch_optional(db)
    .await
    .map_err(Error::Database)?;

    match row {
        Some((value,)) => {
            let parsed = value.parse::<T>()
                .map_err(|e| Error::Config(format!("Parse setting failed: {}", e)))?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

/// Generic setting setter (internal)
async fn set_setting<T>(db: &Pool<Sqlite>, key: &str, value: T) -> Result<()>
where
    T: std::fmt::Display,
{
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value"
    )
    .bind(key)
    .bind(value.to_string())
    .execute(db)
    .await
    .map_err(Error::Database)?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Setup in-memory test database with settings table
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create settings table matching production schema
        sqlx::query(
            "CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_get_acoustid_api_key_exists() {
        let pool = setup_test_db().await;

        // Insert test key directly
        sqlx::query("INSERT INTO settings (key, value) VALUES ('acoustid_api_key', 'test_key_123')")
            .execute(&pool)
            .await
            .unwrap();

        // Retrieve key
        let result = get_acoustid_api_key(&pool).await.unwrap();

        assert_eq!(result, Some("test_key_123".to_string()));
    }

    #[tokio::test]
    async fn test_get_acoustid_api_key_not_exists() {
        let pool = setup_test_db().await;

        let result = get_acoustid_api_key(&pool).await.unwrap();

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_set_acoustid_api_key_insert() {
        let pool = setup_test_db().await;

        // Set new key
        set_acoustid_api_key(&pool, "new_key_456".to_string())
            .await
            .unwrap();

        // Verify key was stored
        let result = get_acoustid_api_key(&pool).await.unwrap();
        assert_eq!(result, Some("new_key_456".to_string()));
    }

    #[tokio::test]
    async fn test_set_acoustid_api_key_update() {
        let pool = setup_test_db().await;

        // Insert initial key
        set_acoustid_api_key(&pool, "old_key".to_string())
            .await
            .unwrap();

        // Update key (UPSERT)
        set_acoustid_api_key(&pool, "new_key".to_string())
            .await
            .unwrap();

        // Verify key was updated
        let result = get_acoustid_api_key(&pool).await.unwrap();
        assert_eq!(result, Some("new_key".to_string()));

        // Verify no duplicate entries
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings WHERE key = 'acoustid_api_key'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "Should have exactly one entry after update");
    }

    #[tokio::test]
    async fn test_roundtrip_set_and_get() {
        let pool = setup_test_db().await;

        let test_key = "roundtrip_test_key_789";

        // Set key
        set_acoustid_api_key(&pool, test_key.to_string())
            .await
            .unwrap();

        // Get key
        let result = get_acoustid_api_key(&pool).await.unwrap();

        assert_eq!(result, Some(test_key.to_string()));
    }
}
