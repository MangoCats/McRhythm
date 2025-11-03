//! Settings database operations
//!
//! **Traceability:** [APIK-DB-010], [APIK-DB-020], [APIK-ACID-040]
//!
//! Provides get/set accessors for settings table following key-value pattern.

use sqlx::{Pool, Sqlite};
use wkmp_common::{Error, Result};

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
